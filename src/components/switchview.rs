#![allow(dead_code)]
#![allow(unused_variables)]

use gtk::prelude::*;
use log::{debug, error};
use relm4::{factory::FactoryVecDeque, prelude::*};

use super::denselist::{DenseList, DenseListInit, DenseListInput};
use crate::spotconn::{model::SpotItem, SpotConn};

pub struct SwitchView {
    spot: SpotConn,
    views: FactoryVecDeque<SwitchViewItem>,
    gtk_stack: gtk::Stack,
}

#[derive(Debug)]
pub struct SwitchViewInit {}

#[derive(Debug, Clone, Copy)]
pub enum SwitchViewInput {
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
pub enum SwitchViewOutput {}

#[derive(Debug)]
pub enum SwitchViewCommandOutput {}

#[relm4::component(pub)]
impl relm4::Component for SwitchView {
    type Init = SwitchViewInit;
    type Input = SwitchViewInput;
    type Output = SwitchViewOutput;
    type CommandOutput = SwitchViewCommandOutput;

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
        let views = FactoryVecDeque::<SwitchViewItem>::builder()
            .launch(gtk::Stack::new())
            .forward(sender.output_sender(), move |out| match out {});
        let mut model = SwitchView {
            spot: SpotConn::new(), //TODO: accept from parent
            views,
            gtk_stack: gtk::Stack::default(),
        };
        let view_widgets = model.views.widget();
        let widgets = view_output!();
        model.gtk_stack = view_widgets.clone();
        sender
            .input_sender()
            .emit(SwitchViewInput::NavResetPlaylists);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            SwitchViewInput::CursorMove(delta) => {
                self.views
                    .guard()
                    .back()
                    .expect("page stack cannot be empty")
                    .denselist
                    .emit(DenseListInput::CursorMove(delta));
            }
            SwitchViewInput::NavDescend => {
                let mut pages = self.views.guard();
                let maybe_dli = {
                    let last_page = pages.back().expect("page stack cannot be empty");
                    last_page.denselist.model().descend()
                };
                if let Some(dli) = maybe_dli {
                    debug!("descending into {:?}", dli);
                    pages.push_back(SwitchViewItemInit {
                        spot: dli.spot.clone(),
                        layout: SwitchViewItemLayout::SingleDenseList(dli.source),
                    });
                } else {
                    debug!("cannot descend");
                }
            }
            SwitchViewInput::NavBack => {
                let mut pages = self.views.guard();
                if pages.len() == 1 {
                    debug!("cannot go back up: already at the root");
                } else {
                    pages.pop_back();
                }
            }
            SwitchViewInput::NavResetPlaylists => {
                let mut pages = self.views.guard();
                pages.clear();
                pages.push_back(SwitchViewItemInit {
                    spot: self.spot.clone(),
                    layout: SwitchViewItemLayout::SingleDenseList(SpotItem::UserPlaylists),
                });
            }
            SwitchViewInput::NavResetSearch => {
                error!("Search is not implemented yet");
            }
        }
    }
}

impl SwitchView {
    fn last_page_widget(&self) -> gtk::Widget {
        self.gtk_stack.last_child().unwrap()
    }

    pub fn current_list(&self) -> &Controller<DenseList> {
        &self.views.back().unwrap().denselist
    }
}

#[derive(Debug)]
pub struct SwitchViewItem {
    init: SwitchViewItemInit,
    denselist: Controller<DenseList>, // TODO: this should be an enum for search/home pages too
}

#[derive(Debug)]
pub enum SwitchViewItemLayout {
    SingleDenseList(SpotItem),
}

#[derive(Debug)]
pub struct SwitchViewItemInit {
    spot: SpotConn,
    layout: SwitchViewItemLayout,
}

#[derive(Debug)]
pub enum SwitchViewItemInput {}

#[derive(Debug)]
pub enum SwitchViewItemOutput {}

#[relm4::factory(pub)]
impl FactoryComponent for SwitchViewItem {
    type Init = SwitchViewItemInit;
    type Input = SwitchViewItemInput;
    type Output = SwitchViewItemOutput;
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
            gtk::Button {
                gtk::Label {
                    set_label: "Switch view content"
                },
            },
            self.denselist.widget() {
            },
        }
    }

    fn init_model(init: Self::Init, index: &Self::Index, sender: FactorySender<Self>) -> Self {
        let SwitchViewItemLayout::SingleDenseList(ref source) = init.layout;
        let denselist = DenseList::builder()
            .launch(DenseListInit {
                spot: init.spot.clone(),
                source: source.clone(),
            })
            .forward(sender.output_sender(), |msg| match msg {});
        SwitchViewItem { init, denselist }
    }
}
