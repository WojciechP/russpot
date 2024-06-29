use std::collections::HashSet;
use std::time::Duration;
use std::{env, sync::Arc};

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use librespot::core::keymaster::Token;
use librespot::{
    core::{config::SessionConfig, session::Session, spotify_id::SpotifyId},
    discovery::Credentials,
    metadata::{Metadata, Track},
};
use rspotify::model::{PlayContextId, PlaylistId};
use rspotify::Config;
use rspotify::{clients::OAuthClient, scopes, AuthCodeSpotify, OAuth, Token as RSToken};
use tokio::sync::OnceCell;

/// SpotConn encapsulates connection to Spotify.
/// It's a bit like Arc: it is Clone, but the cloned
/// instances are just separate handles to the same session.
/// SpotConn is thread-safe.
#[derive(Clone)]
pub struct SpotConn {
    /// raw_session is connection to librespot, supporting playback and metadata retrieval.
    raw_session: Arc<OnceCell<Session>>,
    // raw_rspot is rspotify client using Spotify Web API, supporting user library and search.
    raw_rspot: AuthCodeSpotify,
}

impl SpotConn {
    pub fn new() -> Self {
        let web_config = Config {
            token_refreshing: false,
            ..Default::default()
        };
        let web_api = AuthCodeSpotify::with_config(
            rspotify::Credentials::default(),
            rspotify::OAuth::default(),
            web_config,
        );
        SpotConn {
            raw_session: Arc::new(OnceCell::new()),
            raw_rspot: web_api,
        }
    }
    async fn session(&self) -> &Session {
        self.raw_session
            .get_or_init(|| async {
                let user =
                    env::var("RUSSPOT_USERNAME").expect("RUSSPOT_USERNAME env var must be set");
                let pwd =
                    env::var("RUSSPOT_PASSWORD").expect("RUSSPOT_PASSWORD env var must be set");
                let credentials = Credentials::with_password(user, pwd);
                let sc = SessionConfig::default();
                let (session, _) = Session::connect(sc, credentials, None, false)
                    .await
                    .expect("connecting to Spotify failed");
                session
            })
            .await
    }

    async fn get_new_token(&self) -> RSToken {
        let client_id =
            env::var("RSPOTIFY_CLIENT_ID").expect("RSPOT_CLIENT_ID env var must be set"); // TODO: hardcode the Russpot ID here?
        let scopes = "user-read-private,playlist-read-private,playlist-read-collaborative,playlist-modify-public,playlist-modify-private,user-follow-modify,user-follow-read,user-library-read,user-library-modify,user-top-read,user-read-recently-played,user-modify-playback-state";
        let url =
            format!("hm://keymaster/token/authenticated?client_id={client_id}&scope={scopes}");
        let response = self.session().await.mercury().get(url).await;
        let response = response.unwrap();
        let payload = response.payload.first().unwrap();

        let data = String::from_utf8(payload.clone()).unwrap();
        let token: Token = serde_json::from_str(&data).unwrap();
        RSToken {
            access_token: token.access_token,
            expires_in: chrono::Duration::try_seconds(token.expires_in.into()).unwrap(),
            scopes: HashSet::from_iter(token.scope),
            expires_at: None,
            refresh_token: None,
        }
    }
    pub async fn rspot(&self) -> AuthCodeSpotify {
        {
            // Check if OAuth token exists and is still valid, and refresh it if not.
            // This all happens under the token lock: we don't want other tasks
            // to reattempt the authorization in parallel.
            let locked = self.raw_rspot.token.lock();
            let mut rtok = locked.await.unwrap();
            if rtok.is_none() {
                *rtok = Some(self.get_new_token().await);
            }
        }
        self.raw_rspot.clone()
    }

    pub async fn load_track(&self, id: SpotifyId) -> Track {
        Track::get(self.session().await, id)
            .await
            .expect("could not load track")
    }

    pub async fn play_playlist<'a>(&self, id: PlaylistId<'a>) {
        let result = self
            .rspot()
            .await
            .start_context_playback(
                PlayContextId::Playlist(id.clone()),
                None, /* device id*/
                None, /* offset */
                None, /* Position */
            )
            .await;
        println!("Playback of {} started: {:?}", id, result)
    }
}

impl std::fmt::Debug for SpotConn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<SpotConn: connected={}>",
            self.raw_session.initialized()
        )
    }
}
