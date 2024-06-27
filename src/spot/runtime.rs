use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use librespot::{
    core::{config::SessionConfig, session::Session, spotify_id::SpotifyId},
    discovery::Credentials,
    metadata::{Metadata, Track},
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer::NoOpVolume,
        player::Player,
    },
};

// Note that Session is Clone, but Player is not.
struct Spot {
    session: Session,
    player: Player,
    tracks: HashMap<String, Option<Track>>, // None if metadata not loaded yet
}

#[derive(Clone)]
pub struct Runtime {
    spot: Arc<Mutex<Option<Spot>>>,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            spot: Arc::new(Mutex::new(None)),
        }
    }
    pub async fn run(&mut self, user: &str, pwd: &str) {
        let session_config = SessionConfig::default();
        let player_config = PlayerConfig::default();
        let audio_format = AudioFormat::default();

        let credentials = Credentials::with_password(user, pwd);

        let backend = audio_backend::find(None).unwrap();

        println!("Connecting ..");
        let (session, _) = Session::connect(session_config, credentials, None, false)
            .await
            .unwrap();

        let (player, _) = Player::new(
            player_config,
            session.clone(),
            Box::new(NoOpVolume),
            move || backend(None, audio_format),
        );
        let mut spot = self.spot.lock().unwrap();
        *spot = Some(Spot {
            session: session,
            player: player,
            tracks: HashMap::new(),
        });
    }

    fn session(&self) -> Session {
        self.spot.lock().unwrap().as_ref().unwrap().session.clone()
    }

    pub async fn load_track_metadata(&mut self, b16id: &str) {
        let session = self.session();
        let id = SpotifyId::from_base62(b16id).unwrap();
        let track = Track::get(&session, id).await;
        match track {
            Ok(track) => {
                let mut spot = self.spot.lock().unwrap();
                spot.as_mut()
                    .unwrap()
                    .tracks
                    .insert(b16id.into(), Some(track));
            }
            Err(e) => {
                println!("Failed to load track {}: {:?}", b16id, e);
            }
        }
    }

    pub fn play_track(&mut self, track: &str, sender: async_channel::Sender<Track>) {
        let track = SpotifyId::from_base62(track).unwrap();
        self.spot
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .player
            .load(track, true, 0);
    }
}
