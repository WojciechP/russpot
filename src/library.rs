use gtk4::gio;
use gtk4::glib::prelude::*;
use gtk4::glib::subclass::prelude::*;
use gtk4::glib::Properties;
use gtk4::glib::{self, Object};
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::core::spotify_id::SpotifyId;
use librespot::discovery::Credentials;
use librespot::metadata::{Metadata, Track};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::runtime::Runtime;
use tokio::sync::{OnceCell, SetError};

pub const LIK_TRACK: &str = "track";
pub const LIK_PLAYLIST: &str = "playlist";

#[derive(Default, Clone)]
struct Author {
    name: String,
    nick: String,
}

mod imp {

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::LineItem)]
    pub struct LineItem {
        #[property(name = "author-name", get, set, type = String, member = name)]
        #[property(name = "author-nick", get, set, type = String, member = nick)]
        author: RefCell<Author>,

        // Must be one of LIK_ values
        #[property(get, set)]
        kind: RefCell<String>,

        #[property(get, set)]
        id_b16: RefCell<String>,

        // The main human-readable name (track title, artist name, ...).
        #[property(get, set)]
        name: RefCell<String>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for LineItem {}

    #[glib::object_subclass]
    impl ObjectSubclass for LineItem {
        const NAME: &'static str = "RusspotLineItem";
        type Type = super::LineItem;
    }
}

glib::wrapper! {
    pub struct LineItem(ObjectSubclass<imp::LineItem>);
}

impl LineItem {
    pub fn new_track(id: &SpotifyId) -> Self {
        let it: Self = Object::builder().build();
        it.set_kind(LIK_TRACK);
        it.set_id_b16(id.to_base62().unwrap());
        it
    }

    fn update_track_metadata(&self, tm: &Track) {
        self.set_name(tm.name.clone());
    }
}

// The GTK-friendly interface for accessing Spotify music library.
// All the public functions here are non-async and should be invoked
// from the UI thread. They return GObjects immediately, and the
// contents of the GObjects are filled in asynchronously later.
#[derive(Clone)]
pub struct Library {
    spot_session: Arc<OnceLock<Session>>,
    runtime: &'static Runtime,

    // track_sender is used to send track metadata from any thread back into UI,
    // where they are inserted into tracks hashmap.
    track_sender: async_channel::Sender<Track>,
}

// LibraryOwner is the UI-thread side of the bridge. It owns all the track data.
#[derive(Clone)]
pub struct LibraryOwner {
    lib: Library,
    // tracks is a repository of LineItem GObjects in the UI thread.
    tracks: Rc<RefCell<HashMap<String, LineItem>>>,
}

impl Library {
    pub fn new(user: &str, pwd: &str, runtime: &'static Runtime) -> (LibraryOwner, Self) {
        let (track_sender, track_receiver) = async_channel::bounded::<Track>(10);
        let l = Library {
            spot_session: Arc::new(OnceLock::new()),
            runtime: runtime,
            track_sender: track_sender,
        };
        let owner = LibraryOwner {
            lib: l,
            tracks: Rc::new(RefCell::new(HashMap::new())),
        };
        let credentials = Credentials::with_password(user, pwd);
        let lib = owner.lib.clone();
        let cloned = lib.clone();
        owner
            .lib
            .runtime
            .spawn(async move { cloned.connect_session(credentials).await });

        // Spawn a UI future for updating the tracks in the hashmap:
        let cloned_owner = owner.clone();
        glib::spawn_future_local(async move {
            while let Ok(track) = track_receiver.recv().await {
                cloned_owner.update_track_entry(&track);
            }
        });

        (owner, lib)
    }

    async fn connect_session(&self, creds: Credentials) -> Result<(), Session> {
        println!("Connecting ..");
        let session_config = SessionConfig::default();
        let (session, _) = Session::connect(session_config, creds, None, false)
            .await
            .unwrap();
        let result = self.spot_session.set(session);
        println!("Session is now ready, ok={}", result.is_ok());
        // TODO: we should probably start loading library here, or emit a GTK signal.
        result
    }
    fn trigger_track_loading(&self, id: SpotifyId) {
        let session = self.spot_session.get().unwrap().clone(); // TODO - panics before session connected.
        let sender = self.track_sender.clone();
        self.runtime.spawn(async move {
            println!("loading track...");
            let track = Track::get(&session, id).await;
            println!("track loaded:");
            sender
                .send(track.unwrap())
                .await
                .expect("track channel closed?");
        });
    }

    /*
    pub fn load_tracks_from_playlist(&self, id: &SpotifyId) -> gio::ListStore<LineItem> {
        // TODO: cache the LineItems for reuse later
        let store = gio::ListStore::new::<LineItem>();
        self.runtime.spawn(async move {
            println!("loading playlist ...");
            let plist = Playlist::get(&session, playlist_uri).await.unwrap();
            println!("{:?}", plist);
            for track_id in plist.tracks {
                let plist_track = Track::get(&session, track_id).await.unwrap();
                println!("track: {} ", plist_track.name);
                sender.send(plist_track).await.expect("channel closed?");
            }
        });
        store
    }
    */
}

impl LibraryOwner {
    fn unique_track(&self, id: &SpotifyId) -> LineItem {
        let id_str = id.to_base62().unwrap();
        let mut tracks = self.tracks.borrow_mut();
        match tracks.get(&id_str) {
            Some(item) => item.clone(),
            None => {
                let it = LineItem::new_track(&id);
                tracks.insert(id_str.clone(), it.clone());
                println!(
                    "Inserted {}. Now got {}.",
                    &id_str,
                    tracks.keys().cloned().collect::<Vec<String>>().join(",")
                );
                it
            }
        }
    }

    pub fn load_track(&self, id: SpotifyId) -> LineItem {
        let it = self.unique_track(&id);
        self.lib.trigger_track_loading(id);
        it
    }

    fn update_track_entry(&self, track: &Track) {
        let id_str = track.id.to_base62().unwrap();
        let tracks = self.tracks.borrow();
        match tracks.get(&id_str) {
            Some(item) => item.update_track_metadata(track),
            None => println!(
                "Track {} (id {}) is not in the cache. Got: {}",
                track.name,
                id_str,
                tracks.keys().cloned().collect::<Vec<String>>().join(",")
            ),
        }
    }
}
