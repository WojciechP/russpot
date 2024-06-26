use gtk4::glib::{self, Object};

glib::wrapper! {
pub struct SpotifyItemObject(ObjectSubclass<imp::SpotifyItem>);
}

impl SpotifyItemObject {
    pub fn new_track(id: &str) -> Self {
        let sio: SpotifyItemObject = Object::builder().build();
        sio.set_trackid(id);
        sio
    }
}

mod imp {

    use glib::subclass::prelude::*;
    use gtk4::glib::{
        self,
        subclass::{object::ObjectImpl, types::ObjectSubclass},
        Properties,
    };
    use gtk4::prelude::ObjectExt;
    use std::cell::RefCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type=super::SpotifyItemObject)]
    pub struct SpotifyItem {
        #[property(get, set)]
        trackid: RefCell<String>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for SpotifyItem {}

    #[glib::object_subclass]
    impl ObjectSubclass for SpotifyItem {
        const NAME: &'static str = "SpotifyItem";
        type Type = super::SpotifyItemObject;
    }
}
