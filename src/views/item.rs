use gtk4::glib::{self, Object};
use librespot::core::spotify_id::SpotifyId;

glib::wrapper! {
pub struct RSItem(ObjectSubclass<imp::RSItem>);
}

pub const RSK_TRACK: &str = "track";
pub const RSK_PLAYLIST: &str = "playlist";

impl RSItem {
    pub fn new_track(id: &SpotifyId) -> Self {
        let sio: RSItem = Object::builder().build();
        sio.set_id_b16(id.to_base16().unwrap());
        sio.set_kind(RSK_TRACK);
        sio
    }
    pub fn new_playlist(id: &SpotifyId) -> Self {
        let p: RSItem = Object::builder().build();
        p.set_kind(RSK_PLAYLIST);
        p.set_id_b16(id.to_base16().unwrap());
        p
    }
}

mod imp {
    use gtk4::glib::{
        self,
        subclass::{object::ObjectImpl, prelude::*, types::ObjectSubclass},
        Properties,
    };
    use gtk4::prelude::ObjectExt;
    use std::cell::RefCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type=super::RSItem)]
    pub struct RSItem {
        // kind is one of "artist", "track".
        #[property(get, set)]
        kind: RefCell<String>,

        // id_b16 is base16-encoded Spotify id,
        // suitable for SpotifyId::parse.
        #[property(get, set)]
        id_b16: RefCell<String>,

        // name is the primary name of the item: song title, artist name, etc.
        #[property(get, set)]
        name: RefCell<String>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for RSItem {}

    #[glib::object_subclass]
    impl ObjectSubclass for RSItem {
        const NAME: &'static str = "RSItem";
        type Type = super::RSItem;
    }
}
