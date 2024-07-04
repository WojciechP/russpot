use gtk::prelude::*;

use log::debug;
use relm4::prelude::*;
use rspotify::model::SearchType;

use crate::{
    components::denselist::{DenseList, DenseListInit, DenseListOutput},
    spotconn::{model::SpotItem, SpotConn},
};

use super::denselist::DenseListInput;

#[derive(Debug)]
pub struct SearchPage {
    spot: SpotConn,
    searchbox: gtk::Entry,
    btn_go: gtk::Button,

    tracks: Controller<DenseList>,
}

#[derive(Debug)]
pub enum SearchPageInput {
    FocusSearchbox,
    #[doc(hidden)]
    ExecuteSearch, // run the search for current query
    CursorMoveDown,
    CursorMoveUp,
}

#[relm4::component(pub)]
impl Component for SearchPage {
    type Init = SpotConn;
    type Input = SearchPageInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box::new(gtk::Orientation::Vertical, 0) {
            set_hexpand: true,

            gtk::Box::new(gtk::Orientation::Horizontal, 0) {
                #[name="searchbox"]
                gtk::Entry {
                    connect_activate => SearchPageInput::ExecuteSearch,
                },
                #[name="btn_go"]
                gtk::Button {
                    gtk::Label {
                        set_label: "Go",
                    },
                    connect_clicked => SearchPageInput::ExecuteSearch,
                },
            },

            tracks.widget(),
        },

    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let tracks = DenseList::builder()
            .launch(DenseListInit {
                spot: init.clone(),
                source: SpotItem::UserPlaylists, // TODO: bad: should be empty, then change to search results
            })
            .forward(sender.output_sender(), |msg| match msg {
                DenseListOutput::CursorEscapedDown => {
                    todo!("implement cursor moves across sections")
                }
                DenseListOutput::CursorEscapedUp => todo!("implement cursor moves across sections"),
            });

        let widgets = view_output!();
        let model = SearchPage {
            spot: init,
            searchbox: widgets.searchbox.clone(),
            btn_go: widgets.btn_go.clone(),
            tracks,
        };
        sender.input_sender().emit(SearchPageInput::FocusSearchbox);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            SearchPageInput::FocusSearchbox => {
                self.searchbox.grab_focus();
            }
            SearchPageInput::ExecuteSearch => {
                let spot = self.spot.clone();
                let query = self.searchbox.text().to_string();

                self.tracks
                    .emit(DenseListInput::Reset(SpotItem::SearchResults {
                        st: SearchType::Track,
                        query,
                    }));
                let moved_foc = self.btn_go.grab_focus();
                debug!("Search executing; moved focus = {}", moved_foc);
            }
            SearchPageInput::CursorMoveDown => self.tracks.emit(DenseListInput::CursorMove(1)),
            SearchPageInput::CursorMoveUp => self.tracks.emit(DenseListInput::CursorMove(-1)),
        }
    }
}
