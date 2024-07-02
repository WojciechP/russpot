//! Small block component, for example a song or a playlist in the library view.
//! This has an image on the left, and some text on the right.

use gtk::{gdk_pixbuf::Pixbuf, glib, prelude::*};
use relm4::{prelude::*, Component, ComponentParts};
use std::fmt::Debug;

use crate::spotconn::model::SpotItem;

/// SmallBlock holds the state for the displayed component.
pub struct SmallBlock {
    /// The underlying data. Only set at initialization.
    init: SpotItem,
    /// The image associated with the entry (usually album art).
    /// Loaded asynchronously after initialization.
    pixbuf: Option<Pixbuf>,
}

impl Debug for SmallBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<SmallBlock for {:?} >", self.init)
    }
}

impl SmallBlock {
    /// Returns the underlying Spotify item.
    pub fn get_content(&self) -> &SpotItem {
        &self.init
    }
}

#[derive(Debug)]
pub enum SmallBlockInput {
    Clicked,
}

#[derive(Debug)]
pub enum SmallBlockOutput {
    Clicked,
}

#[derive(Debug)]
pub enum SmallBlockCommandOutput {
    ImageLoaded(glib::Bytes),
}

#[relm4::component(pub)]
#[allow(deprecated)]
impl Component for SmallBlock {
    type Init = SpotItem;
    type Input = SmallBlockInput;
    type Output = SmallBlockOutput;
    type CommandOutput = SmallBlockCommandOutput;

    view! {
        #[root]
        gtk::Button {
            set_css_classes: &["smallblock"],
            connect_clicked => SmallBlockInput::Clicked,
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
                        set_label: model.init.name(),
                        set_xalign: 0.0,
                    },
                    gtk::Label {
                        set_css_classes: &["user"],
                        set_label: &model.init.artist(),
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
        let model = SmallBlock { init, pixbuf: None };
        let widgets = view_output!();

        if let Some(img_url) = model.init.img_url().map(str::to_string) {
            sender.oneshot_command(async move {
                let result = reqwest::get(img_url).await.unwrap();
                let bytes = result.bytes().await.unwrap().to_vec();
                let bytes = glib::Bytes::from(&bytes.to_vec());
                SmallBlockCommandOutput::ImageLoaded(bytes)
            });
        }
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: SmallBlockInput, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            SmallBlockInput::Clicked => sender.output_sender().emit(SmallBlockOutput::Clicked),
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            SmallBlockCommandOutput::ImageLoaded(bytes) => {
                let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
                let pixbuf = Pixbuf::from_stream(&stream, gtk::gio::Cancellable::NONE).unwrap();
                self.pixbuf = Some(pixbuf);
            }
        }
    }
}
