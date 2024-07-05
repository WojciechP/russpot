//! Small block component, for example a song or a playlist in the library view.
//! This has an image on the left, and some text on the right.

use gtk::{gdk_pixbuf::Pixbuf, glib, prelude::*};
use relm4::{prelude::*, Component, ComponentParts};
use std::fmt::Debug;

use crate::spotconn::model::SpotItem;

/// Model holds the state for the displayed component.
pub struct Model {
    /// The underlying data. Only set at initialization.
    init: SpotItem,
    /// The image associated with the entry (usually album art).
    /// Loaded asynchronously after initialization.
    pixbuf: Option<Pixbuf>,
}

impl Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<SmallBlock for {:?} >", self.init)
    }
}

impl Model {
    /// Returns the underlying Spotify item.
    pub fn get_content(&self) -> &SpotItem {
        &self.init
    }
}

#[derive(Debug)]
pub enum In {
    Clicked,
}

#[derive(Debug)]
pub enum Out {
    Clicked,
}

#[derive(Debug)]
pub enum CmdOut {
    ImageLoaded(glib::Bytes),
}

#[relm4::component(pub)]
#[allow(deprecated)]
impl Component for Model {
    type Init = SpotItem;
    type Input = In;
    type Output = Out;
    type CommandOutput = CmdOut;

    view! {
        #[root]
        gtk::Button {
            set_css_classes: &["smallblock"],
            connect_clicked => In::Clicked,
            gtk::Box{
                set_orientation: gtk::Orientation::Horizontal,
                #[name="image"]
                gtk::Image {
                    set_width_request: 36,
                    set_height_request: 36,
                    #[watch]
                    set_from_pixbuf: model.pixbuf.as_ref(),
                },
                gtk::Box {
                    set_css_classes: &["textpart"],
                    set_orientation: gtk::Orientation::Vertical,

                    gtk::Label {
                        set_css_classes: &["name"],
                        set_label: &model.init.name(),
                        set_xalign: 0.0,
                    },
                    gtk::Label {
                        set_css_classes: &["user"],
                        set_label: &model.second_line(),
                        set_xalign: 0.0,
                    },
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Model { init, pixbuf: None };
        let widgets = view_output!();

        if let Some(img_url) = model.init.img_url().map(str::to_string) {
            sender.oneshot_command(async move {
                let result = reqwest::get(img_url).await.unwrap();
                let bytes = result.bytes().await.unwrap().to_vec();
                let bytes = glib::Bytes::from(&bytes.to_vec());
                CmdOut::ImageLoaded(bytes)
            });
        }
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: In, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            In::Clicked => sender.output_sender().emit(Out::Clicked),
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            CmdOut::ImageLoaded(bytes) => {
                let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
                let pixbuf = Pixbuf::from_stream(&stream, gtk::gio::Cancellable::NONE).unwrap();
                self.pixbuf = Some(pixbuf);
            }
        }
    }
}

impl Model {
    fn second_line(&self) -> String {
        match &self.init {
            SpotItem::Album(a) => format!("Album by {}", self.init.artist()),
            SpotItem::Playlist(p) => format!("Playlist by {}", self.init.artist()),
            other => other.artist(),
        }
    }
}
