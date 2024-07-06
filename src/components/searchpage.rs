use gtk::prelude::*;


use relm4::prelude::*;
use rspotify::model::SearchType;

use crate::{
    components::denselist,
    navigation::{NavCommand, NavOutput},
    spotconn::{model::SpotItem, SpotConn},
};

#[derive(Debug)]
pub struct Model {
    spot: SpotConn,
    searchbox: gtk::Entry,
    btn_go: gtk::Button,

    albums: Controller<denselist::Model>,
    tracks: Controller<denselist::Model>,
}

#[derive(Debug)]
pub enum In {
    FocusSearchbox,
    #[doc(hidden)]
    ExecuteSearch, // run the search for current query
    Nav(NavCommand),
}

#[derive(Debug)]
pub enum Out {
    Nav(NavOutput),
}

#[relm4::component(pub)]
impl Component for Model {
    type Init = SpotConn;
    type Input = In;
    type Output = Out;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box::new(gtk::Orientation::Vertical, 0) {
            set_hexpand: true,

            gtk::Box::new(gtk::Orientation::Horizontal, 0) {
                #[name="searchbox"]
                gtk::Entry {
                    connect_activate => In::ExecuteSearch,
                },
                #[name="btn_go"]
                gtk::Button {
                    gtk::Label {
                        set_label: "Go",
                    },
                    connect_clicked => In::ExecuteSearch,
                },
            },

            albums.widget(),
            tracks.widget(),
        },

    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let tracks = denselist::Model::builder()
            .launch(denselist::Init {
                spot: init.clone(),
                source: SpotItem::UserPlaylists, // TODO: bad: should be empty, then change to search results
            })
            .forward(sender.output_sender(), |msg| match msg {
                denselist::Out::Nav(NavOutput::CursorIsNowAt(item)) => {
                    Out::Nav(NavOutput::CursorIsNowAt(item))
                }
                denselist::Out::Nav(_) => {
                    todo!("implement cursor moves across sections");
                }
            });
        let albums = denselist::Model::builder()
            .launch(denselist::Init {
                spot: init.clone(),
                source: SpotItem::UserPlaylists, // TODO: bad: should be empty, then change to search results
            })
            .forward(sender.output_sender(), |msg| match msg {
                denselist::Out::Nav(NavOutput::CursorIsNowAt(item)) => {
                    Out::Nav(NavOutput::CursorIsNowAt(item))
                }
                denselist::Out::Nav(_) => {
                    todo!("implement cursor moves across sections");
                }
            });

        let widgets = view_output!();
        let model = Model {
            spot: init,
            searchbox: widgets.searchbox.clone(),
            btn_go: widgets.btn_go.clone(),
            albums,
            tracks,
        };
        sender.input_sender().emit(In::FocusSearchbox);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            In::FocusSearchbox => {
                self.searchbox.grab_focus();
            }
            In::ExecuteSearch => {
                let spot = self.spot.clone();
                let query = self.searchbox.text().to_string();

                self.tracks
                    .emit(denselist::In::Reset(SpotItem::SearchResults {
                        st: SearchType::Track,
                        query: query.clone(),
                    }));
                self.albums
                    .emit(denselist::In::Reset(SpotItem::SearchResults {
                        st: SearchType::Album,
                        query,
                    }));
                self.btn_go.grab_focus();
            }
            // TODO: moves across multiple lists
            In::Nav(nav_cmd) => self.albums.emit(denselist::In::Nav(nav_cmd)),
        }
    }
}

impl Model {
    pub fn descend(&self) -> Option<denselist::Init> {
        self.albums
            .model()
            .descend()
            .or(self.tracks.model().descend())
    }
}
