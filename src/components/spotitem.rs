use crate::spotconn::SpotConn;

use futures::stream::TryStreamExt;
use futures_util::pin_mut;
use gtk::prelude::*;
use gtk::Button;
use librespot::{core::spotify_id::SpotifyId, metadata::Track};
use relm4::{ComponentParts, ComponentSender};
use rspotify::clients::OAuthClient;
use rspotify::{prelude::*, scopes, AuthCodeSpotify, Credentials, OAuth};

#[derive(Debug)]
pub struct SpotItemModel {
    spot: SpotConn,
    id: SpotifyId,
    title: String,
}

#[derive(Debug)]
pub struct SpotifyItemInit {
    // TODO: passing SpotConn into every widget seems... troublesome.
    pub spot: SpotConn,
    pub id: SpotifyId,
}

#[derive(Debug)]
pub enum SpotifyItemInput {
    LoadTrack,
}

#[derive(Debug)]
pub enum SpotItemCommandOutput {
    TrackLoaded(Track),
}

#[relm4::component(pub)]
impl relm4::Component for SpotItemModel {
    type Input = SpotifyItemInput;
    type Output = ();
    type Init = SpotifyItemInit;
    type CommandOutput = SpotItemCommandOutput;
    view! {
        #[root]
        gtk::Box {
            gtk::Label {
                #[watch]
                set_label: &model.title,
            },
            gtk::Button {
                set_label: "load",
                connect_clicked => SpotifyItemInput::LoadTrack,
            },
        }
    }
    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SpotItemModel {
            spot: _params.spot,
            id: _params.id,
            title: "<no title>".to_string(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _: &Self::Root) {
        match msg {
            SpotifyItemInput::LoadTrack => {
                println!("Loading track: {}", self.id.to_base62().unwrap());
                let spot = self.spot.clone();
                let id = self.id.clone();
                sender.oneshot_command(async move {
                    let rspot = spot.rspot().await;
                    let stream = rspot.current_user_saved_tracks(None);
                    pin_mut!(stream);
                    println!("Items (blocking):");
                    while let Some(item) = stream.try_next().await.unwrap() {
                        println!("* {}", item.track.name);
                    }

                    let track = spot.load_track(id).await;

                    println!(
                        "Loaded track {} -> {}",
                        id.to_base62().unwrap(),
                        &track.name
                    );
                    SpotItemCommandOutput::TrackLoaded(track)
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            SpotItemCommandOutput::TrackLoaded(track) => self.title = track.name,
        }
    }
}
