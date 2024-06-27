use gtk::gio;
use gtk::glib::prelude::*;
use gtk::glib::subclass::prelude::*;
use gtk::glib::Properties;
use gtk::glib::{self, Object};
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::core::spotify_id::SpotifyId;
use librespot::discovery::Credentials;
use librespot::metadata::{Metadata, Playlist, Track};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use tokio::runtime::Runtime;

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

    pub fn new_playlist(id: &SpotifyId) -> Self {
        let it: Self = Object::builder().build();
        it.set_kind(LIK_PLAYLIST);
        it.set_id_b16(id.to_base62().unwrap());
        it
    }

    fn update_playlist_metadata(&self, pm: &PlaylistWithId) {
        self.set_name(pm.playlist.name.clone());
    }
}

// PlaylistWithId adds the base62-encoded playlist id, because Playlist type does not have it.
struct PlaylistWithId {
    playlist: Playlist,
    id_b62: String,
}

#[derive(Clone)]
pub struct GPlaylist {
    id_b62: String,

    playlist: Option<Playlist>,
    line_item: LineItem,
    tracks: gio::ListStore,
}

// GTK-friendly repository for Spotify music metadata.
// All functions are synchronous and return GObjects immediately.
// The returned GObjects may be empty, though, and the details are filled in later.
// Under the hood, Library uses tokio runtime to fetch the data asynchronously
// and then update the relevant GObject properties.
// Note that since GObjects are not thread safe, neither is Library (it is not Send).
#[derive(Clone)]
pub struct Library {
    lib: LibraryWorker,
    // tracks is a repository of LineItem GObjects in the UI thread.
    tracks: Rc<RefCell<HashMap<String, LineItem>>>,
    playlists: Rc<RefCell<HashMap<String, GPlaylist>>>,
}

impl Library {
    pub fn new(user: &str, pwd: &str, runtime: &'static Runtime) -> Self {
        let (track_sender, track_receiver) = async_channel::bounded::<Track>(10);
        let (playlist_sender, playlist_receiver) = async_channel::bounded::<PlaylistWithId>(10);
        let l = LibraryWorker {
            spot_session: Arc::new(OnceLock::new()),
            runtime: runtime,
            track_sender: track_sender,
            playlist_sender: playlist_sender,
        };
        let owner = Library {
            lib: l,
            tracks: Rc::new(RefCell::new(HashMap::new())),
            playlists: Rc::new(RefCell::new(HashMap::new())),
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
        // Spawn a UI future for updating the playlsts in the hashmap:
        let cloned_owner = owner.clone();
        glib::spawn_future_local(async move {
            while let Ok(playlist) = playlist_receiver.recv().await {
                cloned_owner.update_playlist_entry(&playlist);
            }
        });

        owner
    }

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
        let item = self.unique_track(&track.id);
        item.update_track_metadata(track)
    }

    fn unique_playlist(&self, id: &SpotifyId) -> GPlaylist {
        let id_str = id.to_base62().unwrap();
        let mut playlists = self.playlists.borrow_mut();
        match playlists.get(&id_str) {
            Some(item) => item.clone(),
            None => {
                let it = LineItem::new_playlist(&id);
                let gp = GPlaylist {
                    playlist: None,
                    id_b62: id_str.clone(),
                    line_item: it,
                    tracks: gio::ListStore::new::<LineItem>(),
                };
                playlists.insert(id_str.clone(), gp.clone());
                println!(
                    "Inserted playlist {}. Now got {}.",
                    &id_str,
                    playlists.keys().cloned().collect::<Vec<String>>().join(",")
                );
                gp
            }
        }
    }

    pub fn load_playlist(&self, id: SpotifyId) -> (LineItem, gio::ListStore) {
        let it = self.unique_playlist(&id);
        self.lib.trigger_playlist_loading(id);
        (it.line_item, it.tracks)
    }

    fn update_playlist_entry(&self, playlist: &PlaylistWithId) {
        let gp = self.unique_playlist(&SpotifyId::from_base62(&playlist.id_b62).unwrap());
        gp.line_item.update_playlist_metadata(playlist);
        gp.tracks.remove_all();
        for tid in &playlist.playlist.tracks {
            gp.tracks.append(&self.unique_track(tid));
            self.lib.trigger_track_loading(*tid);
        }
    }
}

// Connection to Spotify using tokio runtime with the capacity to send data back to GTK side.
// LibraryWorker does not touch any GObjects directly, because they
// can only be accessed from the UI thread, not from tokio.
// Track and playlist metadata is instead passed back to the UI thread
// over channels.
// All methods with names like `trigger_X` spawn async operations and return immediately.
// The results are visible via changes to GObjects later on.
#[derive(Clone)]
struct LibraryWorker {
    spot_session: Arc<OnceLock<Session>>,
    runtime: &'static Runtime,

    // track_sender is used to send track metadata from any thread back into UI,
    // where they are inserted into tracks hashmap.
    track_sender: async_channel::Sender<Track>,
    playlist_sender: async_channel::Sender<PlaylistWithId>,
}

impl LibraryWorker {
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
            let track = Track::get(&session, id).await;
            sender
                .send(track.unwrap())
                .await
                .expect("track channel closed?");
        });
    }

    fn trigger_playlist_loading(&self, id: SpotifyId) {
        let session = self.spot_session.get().unwrap().clone(); // TODO - panics before session connected.
        let sender = self.playlist_sender.clone();
        let _cloned_self = self.clone();
        self.runtime.spawn(async move {
            println!("loading playlist ...");
            let plist = Playlist::get(&session, id).await.unwrap();
            println!("{:?}", plist);
            sender
                .send(PlaylistWithId {
                    id_b62: id.to_base62().unwrap(),
                    playlist: plist.clone(),
                })
                .await
                .expect("playlist channel closed?");
        });
    }
}
