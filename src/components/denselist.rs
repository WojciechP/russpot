use futures::stream::TryStreamExt;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::{glib, prelude::*};
use relm4::factory::{FactoryVecDeque, FactoryVecDequeBuilder};
use relm4::prelude::*;
use rspotify::clients::OAuthClient;
use rspotify::model::SimplifiedPlaylist;

use crate::components::playlistitem::{PlaylistItem, PlaylistItemInit, PlaylistItemOutput};
use crate::spotconn::SpotConn;

#[derive(Debug)]
pub struct DenseList {
    spot: SpotConn,
    items: FactoryVecDeque<PlaylistItem>,
}

#[derive(Debug)]
pub struct DenseListInit {
    pub spot: SpotConn,
    // TODO: add an init option to choose which playlists to load?
}

#[derive(Debug)]
pub enum DenseListInput {}

#[derive(Debug)]
pub enum DenseListOutput {}

/// We allow loading items in a streaming manner: multiple messages with one item each.
#[derive(Debug)]
pub enum DenseListCommandOutput {
    ItemLoaded(SimplifiedPlaylist),
}

#[relm4::component(pub)]
impl relm4::Component for DenseList {
    type Init = DenseListInit;
    type Input = DenseListInput;
    type Output = DenseListOutput;
    type CommandOutput = DenseListCommandOutput;

    view! {
        #[root]
        gtk::ScrolledWindow {
            // set_policy:

            #[local_ref]
            list_view -> gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let items = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |output| match output {});
        let model = DenseList {
            spot: init.spot,
            items,
        };
        let list_view = model.items.widget();
        let widgets = view_output!();

        let spot = model.spot.clone();
        // TODO: this hardcodes library fetch, abstract away?
        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let rspot = spot.rspot().await;
                    let mut stream = rspot.current_user_playlists();
                    while let Some(item) = stream.try_next().await.unwrap() {
                        out.send(DenseListCommandOutput::ItemLoaded(item)).unwrap();
                    }
                })
                .drop_on_shutdown()
        });
        relm4::ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {}
    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            DenseListCommandOutput::ItemLoaded(playlist) => {
                let spot = self.spot.clone();
                println!("playlist loaded for {}, passing to a child", playlist.name);
                self.items.guard().push_back(PlaylistItemInit {
                    spot: spot,
                    simple: playlist,
                });
            }
        }
    }
}
