use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashSet;

use std::sync::RwLock;
use std::{env, sync::Arc};

use librespot::connect::spirc::Spirc;
use librespot::core::config::ConnectConfig;
use librespot::core::keymaster::Token;
use librespot::playback::config::{AudioFormat, PlayerConfig};
use librespot::playback::mixer::softmixer::SoftMixer;
use librespot::playback::mixer::{MixerConfig, NoOpVolume};
use librespot::playback::player::Player;
use librespot::playback::{audio_backend, mixer};
use librespot::{
    core::{config::SessionConfig, session::Session, spotify_id::SpotifyId},
    discovery::Credentials,
    metadata::{Metadata, Track},
};
use rspotify::http::HttpError;
use rspotify::model::{PlayContextId, PlaylistId};
use rspotify::{clients::OAuthClient, AuthCodeSpotify, Token as RSToken};
use rspotify::{ClientError, Config};
use tokio::sync::OnceCell;

struct LibreSpotConn {
    session: Session,
    player: RwLock<Option<Player>>, // RwLock because starting playback is considered a mutation.
    spirc: Option<Spirc>,
}

impl LibreSpotConn {
    async fn new() -> LibreSpotConn {
        let user = env::var("RUSSPOT_USERNAME").expect("RUSSPOT_USERNAME env var must be set");
        let pwd = env::var("RUSSPOT_PASSWORD").expect("RUSSPOT_PASSWORD env var must be set");
        let credentials = Credentials::with_password(user, pwd);
        let session_config = SessionConfig::default();
        let (session, _session_credentials) =
            Session::connect(session_config, credentials, None, false)
                .await
                .expect("LibreSpot Session failed");

        let player_config = PlayerConfig::default();
        let audio_format = AudioFormat::default();
        let mut connect_config = ConnectConfig::default();
        connect_config.name = "russpot".to_owned();
        let backend = audio_backend::find(None).unwrap();
        let (player, _player_event_channel) = Player::new(
            player_config,
            session.clone(),
            Box::new(NoOpVolume),
            move || backend(None, audio_format),
        );
        let mixer_factory = mixer::find(None).expect("mixer factory not found");
        let mixer = mixer_factory(MixerConfig::default());
        eprintln!("Starting Spirc connection...\n\n");
        let (spirc, spirc_task) = Spirc::new(connect_config, session.clone(), player, mixer);
        tokio::spawn(spirc_task); // let spirc run in the background
        eprintln!("LibreSpot (Spirc) connected!\n\n");
        LibreSpotConn {
            session,
            player: RwLock::new(None),
            spirc: Some(spirc),
        }
    }
}

/// SpotConn encapsulates connection to Spotify.
/// It's a bit like Arc: it is Clone, but the cloned
/// instances are just separate handles to the same session.
/// SpotConn is thread-safe.
#[derive(Clone)]
pub struct SpotConn {
    /// raw_librespot is connection to librespot, supporting playback and metadata retrieval.
    raw_librespot: Arc<OnceCell<LibreSpotConn>>,
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
            raw_librespot: Arc::new(OnceCell::new()),
            raw_rspot: web_api,
        }
    }
    async fn librespot(&self) -> &LibreSpotConn {
        self.raw_librespot
            .get_or_init(|| async { LibreSpotConn::new().await })
            .await
    }
    async fn session(&self) -> &Session {
        &self.librespot().await.session
    }

    async fn get_new_token(&self) -> RSToken {
        let client_id =
            env::var("RSPOTIFY_CLIENT_ID").expect("RSPOT_CLIENT_ID env var must be set"); // TODO: hardcode the Russpot ID here?
        let scopes = "user-read-private,playlist-read-private,playlist-read-collaborative,playlist-modify-public,playlist-modify-private,user-follow-modify,user-follow-read,user-library-read,user-library-modify,user-top-read,user-read-recently-played,user-modify-playback-state,user-read-playback-state";
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
        match result {
            Ok(_) => println!("playback ok {} started.", id),
            Err(ClientError::Http(http)) => {
                if let HttpError::StatusCode(sc) = *http {
                    let text = sc.text().await;
                    println!("could not start: \n{}", text.unwrap());
                } else {
                    println!("boo: {:?}", http);
                }
            }
            other => println!("unknown playback error: {:?}", other),
        }
    }

    pub async fn play_on_spirc(&self) {
        let librespot = self.librespot().await;
        let spirc = librespot.spirc.as_ref();
        if spirc.is_none() {
            return;
        }
        // librespot.spirc.as_ref().unwrap().play();
        // println!("spirc play!");

        let spot = self.rspot().await;
        match spot.device().await {
            Ok(devices) => {
                println!("Available devides: {:?}", devices);
                for dev in devices {
                    if dev.name.contains("russpot") {
                        println!("enabling russpot device {}", dev.id.clone().unwrap());
                        match spot.transfer_playback(dev.id.as_ref().unwrap(), None).await {
                            Ok(_) => println!("playback transferred"),
                            Err(e) => println!("could not transfer: {:?}", e),
                        }
                    }
                }
            }
            Err(e) => println!("Could not list devices: {}", e),
        }
    }
}

impl std::fmt::Debug for SpotConn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<SpotConn: connected={}>",
            self.raw_librespot.initialized()
        )
    }
}
