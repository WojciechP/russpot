#![allow(dead_code)]
#![allow(unused_variables)]

use gtk::prelude::*;
use log::debug;
use relm4::{factory::FactoryVecDeque, prelude::*};

use super::denselist;
use super::searchpage;
use crate::spotconn::{model::SpotItem, SpotConn};

pub struct Model {
    spot: SpotConn,
    views: FactoryVecDeque<Child>,
    gtk_stack: gtk::Stack,
}

#[derive(Debug)]
pub struct Init {}

#[derive(Debug, Clone, Copy)]
pub enum In {
    /// Move cursor down (+1) or up (-1).
    CursorMove(i32),
    /// Descend into selected playlist or album.
    NavDescend,
    /// Move back up to the previews view.
    NavBack,
    /// Reset the view to saved playlists.
    NavResetPlaylists,
    /// Reset the view to search page.
    NavResetSearch,
}

#[derive(Debug)]
pub enum Out {}

#[derive(Debug)]
pub enum CmdOut {}

#[relm4::component(pub)]
impl relm4::Component for Model {
    type Init = Init;
    type Input = In;
    type Output = Out;
    type CommandOutput = CmdOut;

    view! {
        #[root]
        gtk::Box::new(gtk::Orientation::Horizontal, 0) {
            set_vexpand: true,
            set_height_request: 400,
            #[local_ref]
            view_widgets -> gtk::Stack {
                set_vexpand: true,
            }
        }
    }

    fn post_view() {
        view_widgets.set_visible_child(&view_widgets.last_child().unwrap());
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let views = FactoryVecDeque::<Child>::builder()
            .launch(gtk::Stack::new())
            .forward(sender.output_sender(), move |out| match out {});
        let mut model = Model {
            spot: SpotConn::new(), //TODO: accept from parent
            views,
            gtk_stack: gtk::Stack::default(),
        };
        let view_widgets = model.views.widget();
        let widgets = view_output!();
        model.gtk_stack = view_widgets.clone();
        sender.input_sender().emit(In::NavResetPlaylists);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            In::CursorMove(delta) => {
                self.views
                    .guard()
                    .back()
                    .expect("page stack cannot be empty")
                    .denselist
                    .cursor_move(delta);
            }
            In::NavDescend => {
                let mut pages = self.views.guard();
                let maybe_dli = {
                    let last_page = pages.back().expect("page stack cannot be empty");
                    last_page.denselist.descend()
                };
                if let Some(dli) = maybe_dli {
                    debug!("descending into {:?}", dli);
                    pages.push_back(ChildInit {
                        spot: dli.spot.clone(),
                        layout: ChildLayout::SingleDenseList(dli.source),
                    });
                } else {
                    debug!("cannot descend");
                }
            }
            In::NavBack => {
                let mut pages = self.views.guard();
                if pages.len() == 1 {
                    debug!("cannot go back up: already at the root");
                } else {
                    pages.pop_back();
                }
            }
            In::NavResetPlaylists => {
                let mut pages = self.views.guard();
                pages.clear();
                debug!("NavResetPlaylists");
                pages.push_back(ChildInit {
                    spot: self.spot.clone(),
                    layout: ChildLayout::SingleDenseList(SpotItem::UserPlaylists),
                });
            }
            In::NavResetSearch => {
                let mut pages = self.views.guard();
                pages.clear();
                debug!("NavResetSearch");
                pages.push_back(ChildInit {
                    spot: self.spot.clone(),
                    layout: ChildLayout::SearchPage,
                });
            }
        }
    }
}

impl Model {
    fn last_page_widget(&self) -> gtk::Widget {
        self.gtk_stack.last_child().unwrap()
    }

    pub fn current_list(&self) -> Option<&Controller<denselist::Model>> {
        self.views.back().and_then(|item| match &item.denselist {
            ChildContent::DenseList(ctrl) => Some(ctrl),
            _ => None,
        })
    }
}

#[derive(Debug)]
pub enum ChildContent {
    DenseList(Controller<denselist::Model>),
    SearchPage(Controller<searchpage::Model>),
}

impl ChildContent {
    fn descend(&self) -> Option<denselist::Init> {
        match self {
            ChildContent::DenseList(dl) => dl.model().descend(),
            ChildContent::SearchPage(sp) => None, // TODO: implement descend for search
        }
    }

    fn cursor_move(&self, delta: i32) {
        match self {
            ChildContent::DenseList(dl) => dl.emit(denselist::In::CursorMove(delta)),
            ChildContent::SearchPage(sp) => {
                if delta > 0 {
                    sp.emit(searchpage::In::CursorMoveDown)
                } else {
                    sp.emit(searchpage::In::CursorMoveUp)
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Child {
    init: ChildInit,
    denselist: ChildContent,
}

#[derive(Debug)]
pub enum ChildLayout {
    SingleDenseList(SpotItem),
    SearchPage,
}

#[derive(Debug)]
pub struct ChildInit {
    spot: SpotConn,
    layout: ChildLayout,
}

#[derive(Debug)]
pub enum ChildIn {
    MoveCursorDown,
    MoveCursorUp,
}

#[derive(Debug)]
pub enum ChildOut {}

#[relm4::factory(pub)]
impl FactoryComponent for Child {
    type Init = ChildInit;
    type Input = ChildIn;
    type Output = ChildOut;
    type CommandOutput = ();
    type ParentWidget = gtk::Stack;

    view! {
        #[root]
        gtk::Box::new(gtk::Orientation::Vertical, 0) {
            set_homogeneous: false,
            set_vexpand: true,
            gtk::Label {
                set_label: "Switch view item"
            },
            self.child_widget() {
            },
        }
    }

    fn init_model(init: Self::Init, index: &Self::Index, sender: FactorySender<Self>) -> Self {
        match init.layout {
            ChildLayout::SingleDenseList(ref source) => {
                let denselist = denselist::Model::builder()
                    .launch(denselist::Init {
                        spot: init.spot.clone(),
                        source: source.clone(),
                    })
                    .forward(sender.input_sender(), |msg| match msg {
                        // When the cursor tries to escape from a single dense list,
                        // we just send it straight back to keep it within bounds.
                        denselist::Out::CursorEscapedUp => ChildIn::MoveCursorDown,
                        denselist::Out::CursorEscapedDown => ChildIn::MoveCursorUp,
                    });
                Child {
                    init,
                    denselist: ChildContent::DenseList(denselist),
                }
            }
            ChildLayout::SearchPage => {
                let sp = searchpage::Model::builder()
                    .launch(init.spot.clone())
                    .forward(sender.output_sender(), |msg| match msg {
                        () => todo!(),
                    });
                Child {
                    init,
                    denselist: ChildContent::SearchPage(sp),
                }
            }
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            ChildIn::MoveCursorDown => self.denselist.cursor_move(1),
            ChildIn::MoveCursorUp => self.denselist.cursor_move(-1),
        }
    }
}

impl Child {
    fn child_widget(&self) -> gtk::Widget {
        match &self.denselist {
            ChildContent::DenseList(dl) => dl.widget().clone().into(),
            ChildContent::SearchPage(sp) => sp.widget().clone().into(),
        }
    }
}
