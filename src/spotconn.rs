#![allow(dead_code)]
#![allow(unused_variables)]
//! Package for fetching metadata over rspotify and controlling playback over librespot.
//! TODO: Split out to separate files.
pub mod model;

use futures::TryStreamExt;
use log::{debug, error};
use std::collections::HashSet;

use std::sync::{OnceLock, RwLock};
use std::time::SystemTime;
use std::{env, sync::Arc};

use librespot::connect::spirc::Spirc;
use librespot::core::config::ConnectConfig;
use librespot::core::keymaster::Token;
use librespot::playback::config::{AudioFormat, PlayerConfig};
use librespot::playback::mixer::{MixerConfig, NoOpVolume};
use librespot::playback::player::Player;
use librespot::playback::{audio_backend, mixer};
use librespot::{
    core::{config::SessionConfig, session::Session},
    discovery::Credentials,
};
use rspotify::model::{
    AlbumId, FullTrack, Offset, PlayContextId, PlayableItem, PlaylistId, SearchResult, SearchType,
    SimplifiedPlaylist, TrackId,
};
use rspotify::{clients::BaseClient, clients::OAuthClient, AuthCodeSpotify, Token as RSToken};
use rspotify::{ClientResult, Config};
use tokio::sync::OnceCell;

use self::model::SpotItem;

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
        debug!("Connecting via LibreSpot using credentials from env vars...");
        let t0 = SystemTime::now();
        let (session, _session_credentials) =
            Session::connect(session_config, credentials, None, false)
                .await
                .expect("LibreSpot Session failed");
        debug!(
            "LibreSpot connection established in {}ms. Creating player...",
            SystemTime::now().duration_since(t0).unwrap().as_millis(),
        );

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
        debug!("Starting Spirc connection...");
        let (spirc, spirc_task) = Spirc::new(connect_config, session.clone(), player, mixer);
        tokio::spawn(spirc_task); // let spirc run in the background
        debug!(
            "Spirc connection established, total {}ms.",
            SystemTime::now().duration_since(t0).unwrap().as_millis()
        );
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
    /// Returns the global singleton instance of SpotConn.
    /// Note that the underlying connection may not exist yet,
    /// and will be established lazily on any method call.
    pub fn global() -> &'static SpotConn {
        static SPOT_CONN: OnceLock<SpotConn> = OnceLock::new();
        SPOT_CONN.get_or_init(SpotConn::new)
    }

    fn new() -> Self {
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

    /// Fetches all the user playlists and emits them via the consumer function asynchronously.
    pub async fn current_user_playlists<F>(&self, f: F)
    where
        F: Fn(SimplifiedPlaylist),
    {
        let spot = self.rspot().await;

        let mut stream = spot.current_user_playlists();
        while let Some(simple_playlist) = stream.try_next().await.unwrap() {
            f(simple_playlist);
        }
    }

    pub async fn current_user_playlists_until_shutdown<F>(
        &self,
        shutdown: relm4::ShutdownReceiver,
        f: F,
    ) where
        F: Fn(SimplifiedPlaylist),
    {
        shutdown
            .register(async { self.current_user_playlists(f).await })
            .drop_on_shutdown()
            .await
    }

    pub async fn tracks_in_playlist<F>(&self, shutdown: relm4::ShutdownReceiver, uri: String, f: F)
    where
        F: Fn(FullTrack),
    {
        shutdown
            .register(async {
                let rspot = self.rspot().await;
                let mut stream = rspot.playlist_items(
                    PlaylistId::from_uri(&uri).unwrap(),
                    None, /*fields*/
                    None, /*market*/
                );
                while let Some(item) = stream.try_next().await.unwrap() {
                    match item.track {
                        Some(PlayableItem::Track(ft)) => f(ft),
                        _ => {
                            debug!("Skipping non-track item {:?}", item,)
                        }
                    }
                }
            })
            .drop_on_shutdown()
            .await
    }

    pub async fn tracks_in_album<F>(&self, shutdown: relm4::ShutdownReceiver, uri: String, f: F)
    where
        F: Fn(FullTrack),
    {
        // Sadly, Spotify Web API only returns SimplifiedTrack items for album_track API call.
        // That is different from playlist_items call, which returns FullTrack objects.
        // The SimplifiedTrack objects don't have album data, because the assumption is that
        // the caller has the album already.
        // We could reconstruct FullTrack from SimplifiedTrack+Album, but for now
        // let's just run a second API call to re-fetch the necessary items.
        // TODO: Optimize the second call away, perhaps introduce our own Track type.
        shutdown
            .register(async {
                let rspot = self.rspot().await;
                let mut stream = rspot.album_track(AlbumId::from_uri(&uri).unwrap(), None);
                let mut track_ids: Vec<TrackId<'_>> = Vec::new();
                while let Some(item) = stream.try_next().await.unwrap() {
                    if let Some(id) = item.id {
                        track_ids.push(id);
                    }
                }
                match rspot.tracks(track_ids, None).await {
                    ClientResult::Ok(tracks) => tracks.into_iter().for_each(f),
                    ClientResult::Err(e) => {
                        error!("Failed to load tracks in {}: {:?}", uri, e);
                    }
                }
            })
            .drop_on_shutdown()
            .await
    }

    pub async fn search<F>(&self, st: SearchType, query: String, f: F)
    where
        F: Fn(SpotItem),
    {
        let spot = self.rspot().await;
        match spot.search(&query, st, None, None, None, None).await {
            ClientResult::Err(e) => {
                error!("Search failed: {:?}", e);
            }
            ClientResult::Ok(SearchResult::Tracks(tracks)) => tracks
                .items
                .into_iter()
                .for_each(|track| f(SpotItem::Track(track))),
            ClientResult::Ok(SearchResult::Playlists(playlists)) => playlists
                .items
                .into_iter()
                .for_each(|playlist| f(SpotItem::Playlist(playlist))),
            ClientResult::Ok(SearchResult::Albums(albums)) => {
                albums.items.into_iter().for_each(|a| f(SpotItem::Album(a)))
            }
            ClientResult::Ok(thing) => {
                error!("Search not implemented for {:?}", thing);
            }
        };
    }

    pub async fn play_context(&self, ctx: PlayContextId<'_>, offset: Option<Offset>) {
        self.rspot()
            .await
            .start_context_playback(
                ctx, None,   /* device id*/
                offset, /* offset */
                None,   /* Position */
            )
            .await
            .expect("failed to start context playback")
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
