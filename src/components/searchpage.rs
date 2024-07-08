use gtk::prelude::*;

use relm4::prelude::*;
use rspotify::model::{Offset, PlayContextId, SearchType};

use crate::{
    components::{denselist, multiview},
    navigation::{NavCommand, NavOutput},
    spotconn::model::SpotItem,
};

#[derive(Debug)]
pub struct Model {
    searchbox: gtk::Entry,
    btn_go: gtk::Button,

    multiview: Controller<multiview::Model>,
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
    type Init = ();
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

            multiview.widget(),
        },

    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let multiview = multiview::Model::builder()
            .launch(multiview::Init { sections: vec![] })
            .forward(sender.output_sender(), |msg| match msg {
                multiview::Out::Nav(nav_out) => Out::Nav(nav_out),
            });

        let widgets = view_output!();
        let model = Model {
            searchbox: widgets.searchbox.clone(),
            btn_go: widgets.btn_go.clone(),
            multiview,
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
                let query = self.searchbox.text().to_string();

                let sections = vec![
                    denselist::Init {
                        source: SpotItem::SearchResults {
                            st: SearchType::Album,
                            query: query.clone(),
                        },
                    },
                    denselist::Init {
                        source: SpotItem::SearchResults {
                            st: SearchType::Track,
                            query: query.clone(),
                        },
                    },
                ];
                self.multiview.emit(multiview::In::ResetSections(sections));
                self.btn_go.grab_focus();
            }
            // TODO: moves across multiple lists
            In::Nav(nav_cmd) => self.multiview.emit(multiview::In::Nav(nav_cmd)),
        }
    }
}

impl Model {
    pub fn descend(&self) -> Option<denselist::Init> {
        self.multiview.model().descend()
    }

    pub fn play_context(&self) -> Option<(PlayContextId<'static>, Option<Offset>)> {
        self.multiview.model().play_context()
    }
}
