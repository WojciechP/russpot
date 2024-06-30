use gtk::gdk_pixbuf::Pixbuf;
use gtk::{gio, glib, prelude::*};
use relm4::prelude::*;
use rspotify::model::SimplifiedPlaylist;

use crate::spotconn::SpotConn;

#[derive(Debug)]
#[tracker::track]
pub struct PlaylistItem {
    #[do_not_track]
    spot: SpotConn,
    #[do_not_track]
    pub self_idx: DynamicIndex,

    simple: SimplifiedPlaylist,
    image: Option<Pixbuf>,
    pub has_cursor: bool,
}

#[derive(Debug)]
pub struct PlaylistItemInit {
    pub spot: SpotConn,
    pub simple: SimplifiedPlaylist,
}

#[derive(Debug)]
pub enum PlaylistItemInput {
    PlayNow,
    SetCursorByClick,
}

#[derive(Debug)]
pub enum PlaylistItemOutput {
    CaptcuredCursorByClick(DynamicIndex),
}

#[derive(Debug)]
pub enum PlaylistItemCommandOutput {
    ImageLoaded(glib::Bytes),
    PlaylistStarted,
}

#[relm4::factory(pub)]
impl FactoryComponent for PlaylistItem {
    type Init = PlaylistItemInit;
    type Input = PlaylistItemInput;
    type Output = PlaylistItemOutput;
    type CommandOutput = PlaylistItemCommandOutput;
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Button {
            set_css_classes: &["playlistitem"],
            #[watch]
            set_class_active: ("has-cursor", self.has_cursor),
            connect_clicked => PlaylistItemInput::SetCursorByClick,
            gtk::Box{
                set_orientation: gtk::Orientation::Horizontal,
                #[name="image"]
                gtk::Image {
                    set_width_request: 36,
                    set_height_request: 36,
                    #[watch]
                    set_from_pixbuf: self.image.as_ref(),
                },
                gtk::Box {
                    set_css_classes: &["textpart"],
                    set_orientation: gtk::Orientation::Vertical,

                    gtk::Label {
                        set_css_classes: &["name"],
                        set_label: &self.simple.name,
                        set_xalign: 0.0,
                    },
                    gtk::Label {
                        set_css_classes: &["user"],
                        set_label: &self.simple.owner.display_name.clone().unwrap_or_default(),
                        set_xalign: 0.0,
                    },
                    gtk::Label{
                        #[track = "self.changed(PlaylistItem::has_cursor())"]
                        set_label: &format!("cursor={}", self.has_cursor),
                    },
                },
            },
        }
    }

    fn init_model(
        init: Self::Init,
        _index: &DynamicIndex,
        sender: relm4::FactorySender<Self>,
    ) -> Self {
        let model = PlaylistItem {
            spot: init.spot,
            simple: init.simple,
            image: None::<Pixbuf>,
            has_cursor: false,
            self_idx: _index.clone(),
            tracker: 0,
        };
        if let Some(img) = model.simple.images.first() {
            let img_url = img.url.clone();
            sender.oneshot_command(async {
                let result = reqwest::get(img_url).await.unwrap();
                let bytes = result.bytes().await.unwrap().to_vec();
                let bytes = glib::Bytes::from(&bytes.to_vec());
                PlaylistItemCommandOutput::ImageLoaded(bytes)
            });
        }
        model
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        self.reset();
        match msg {
            PlaylistItemInput::PlayNow => {
                println!("Play now: {}", self.simple.name);
                let rspot = self.spot.clone();
                let id = self.simple.id.clone();
                sender.oneshot_command(async move {
                    rspot.play_playlist(id).await;
                    PlaylistItemCommandOutput::PlaylistStarted
                })
            }
            PlaylistItemInput::SetCursorByClick => {
                sender
                    .output_sender()
                    .emit(PlaylistItemOutput::CaptcuredCursorByClick(
                        self.self_idx.clone(),
                    ));
            }
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, sender: FactorySender<Self>) {
        match message {
            PlaylistItemCommandOutput::ImageLoaded(bytes) => {
                let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
                let pixbuf = Pixbuf::from_stream(&stream, gtk::gio::Cancellable::NONE).unwrap();
                self.set_image(Some(pixbuf));
            }
            PlaylistItemCommandOutput::PlaylistStarted => {
                println!("started playing playlist {}", self.simple.name);
            }
        }
    }
}
