#![allow(dead_code)]
#![allow(unused_variables)]

use gtk::prelude::*;
use log::debug;
use relm4::{factory::FactoryVecDeque, prelude::*};
use rspotify::model::Offset;
use rspotify::model::PlayContextId;

use super::denselist_factory as denselist;
use super::multiview;
use super::searchpage;
use crate::navigation::NavCommand;
use crate::navigation::NavOutput;
use crate::spotconn::model::SpotItem;

pub struct Model {
    views: FactoryVecDeque<Child>,
    gtk_stack: gtk::Stack,
    scrollwin: gtk::ScrolledWindow,
}

#[derive(Debug)]
pub struct Init {}

#[derive(Debug, Clone, Copy)]
pub enum In {
    Nav(NavCommand),
    #[doc(hidden)]
    EnsureCurrentVisible,
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
            #[name="scrollwin"]
            gtk::ScrolledWindow {
                #[local_ref]
                view_widgets -> gtk::Stack {
                    set_vexpand: true,
                }
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
            .forward(sender.input_sender(), move |out| match out {
                ChildOut::Nav(NavOutput::CursorIsNowAt(_)) => In::EnsureCurrentVisible,
                // When the cursor attemts to leave a child view, bounce it back:
                ChildOut::Nav(nav_out) => In::Nav(match nav_out {
                    NavOutput::EscapedUp => NavCommand::Down,
                    NavOutput::EscapedDown => NavCommand::Up,
                    NavOutput::EscapedLeft => NavCommand::Right,
                    NavOutput::EscapedRight => NavCommand::Left,
                    NavOutput::CursorIsNowAt(_) => unreachable!("NowOutput::CursorIsNowAt is handled in the parent match-case statement already"),
                }),
            });
        let mut model = Model {
            views,
            gtk_stack: gtk::Stack::default(),
            scrollwin: gtk::ScrolledWindow::new(),
        };
        let view_widgets = model.views.widget();
        let widgets = view_output!();
        model.gtk_stack = view_widgets.clone();
        sender.input_sender().emit(In::NavResetPlaylists);
        model.scrollwin = widgets.scrollwin.clone();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            In::EnsureCurrentVisible => {
                self.ensure_current_visible();
            }
            In::Nav(nav_cmd) => {
                self.views
                    .guard()
                    .back()
                    .expect("page stack cannot be empty")
                    .emit_nav(nav_cmd);
                // Scroll the view so that the newly selected item is still visible.
                // We're using a message here, instead of direct function call,
                // so that the cursor has time to move before the calculation happens.
                sender.input_sender().emit(In::EnsureCurrentVisible);
            }
            In::NavDescend => {
                let mut pages = self.views.guard();
                let maybe_dli = {
                    let last_page = pages.back().expect("page stack cannot be empty");
                    last_page.child.descend()
                };
                if let Some(dli) = maybe_dli {
                    debug!("descending into {:?}", dli);
                    pages.push_back(ChildInit {
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
                    layout: ChildLayout::SingleDenseList(SpotItem::UserPlaylists),
                });
            }
            In::NavResetSearch => {
                let mut pages = self.views.guard();
                pages.clear();
                debug!("NavResetSearch");
                pages.push_back(ChildInit {
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

    /// Scrolls the list to make sure that the passed child is visible.
    /// Returns the delta in pixels.
    /// Returns None if no scrollng was performed.
    fn ensure_current_visible(&self) -> Option<f64> {
        let widget = self.views.back()?.child_widget();

        let point = widget.compute_point(&self.scrollwin, &gtk::graphene::Point::new(0.0, 0.0))?;
        let mut delta: f64 = 0.0;

        let height = self.scrollwin.height() as f64;
        let point_y = point.y() as f64;
        if point_y < 0.0 {
            delta = point_y - 20.0; // 20 margin
        }
        if point_y + 40.0 > height {
            delta = point_y + 40.0 - height;
        }
        if delta != 0.0 {
            let adj = self.scrollwin.vadjustment();
            debug!("Correcting adjustment from {} by {}", adj.value(), delta);
            adj.set_value(adj.value() + delta);
            self.scrollwin.set_vadjustment(Some(&adj));
            Some(delta)
        } else {
            None
        }
    }

    pub fn play_context(&self) -> Option<(PlayContextId<'static>, Option<Offset>)> {
        self.views.back()?.child.play_context()
    }
}

#[derive(Debug)]
pub enum ChildContent {
    MultiView(Controller<multiview::Model>),
    SearchPage(Controller<searchpage::Model>),
}

impl ChildContent {
    fn descend(&self) -> Option<denselist::Init> {
        match self {
            ChildContent::MultiView(mv) => mv.model().descend(),
            ChildContent::SearchPage(sp) => sp.model().descend(),
        }
    }

    pub fn play_context(&self) -> Option<(PlayContextId<'static>, Option<Offset>)> {
        match self {
            ChildContent::MultiView(mv) => mv.model().play_context(),
            ChildContent::SearchPage(sp) => sp.model().play_context(),
        }
    }
}

#[derive(Debug)]
pub struct Child {
    init: ChildInit,
    child: ChildContent,
}

#[derive(Debug)]
pub enum ChildLayout {
    SingleDenseList(SpotItem),
    SearchPage,
}

#[derive(Debug)]
pub struct ChildInit {
    layout: ChildLayout,
}

#[derive(Debug)]
pub enum ChildIn {
    Nav(NavCommand),
}

impl From<NavCommand> for ChildIn {
    fn from(nc: NavCommand) -> Self {
        ChildIn::Nav(nc)
    }
}

#[derive(Debug)]
pub enum ChildOut {
    Nav(NavOutput),
}

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
                let mv = multiview::Model::builder()
                    .launch(multiview::Init {
                        sections: vec![denselist::Init {
                            source: source.clone(),
                        }],
                    })
                    .forward(sender.output_sender(), |msg| match msg {
                        multiview::Out::Nav(nav_out) => ChildOut::Nav(nav_out),
                    });
                Child {
                    init,
                    child: ChildContent::MultiView(mv),
                }
            }
            ChildLayout::SearchPage => {
                let sp = searchpage::Model::builder().launch(()).forward(
                    sender.output_sender(),
                    |msg| match msg {
                        searchpage::Out::Nav(nav_out) => ChildOut::Nav(nav_out),
                    },
                );
                Child {
                    init,
                    child: ChildContent::SearchPage(sp),
                }
            }
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            ChildIn::Nav(nav_cmd) => self.emit_nav(nav_cmd),
        }
    }
}

impl Child {
    fn emit_nav(&self, nav_cmd: NavCommand) {
        match &self.child {
            ChildContent::MultiView(mv) => mv.emit(multiview::In::Nav(nav_cmd)),
            ChildContent::SearchPage(sp) => sp.emit(searchpage::In::Nav(nav_cmd)),
        }
    }

    fn child_widget(&self) -> gtk::Widget {
        match &self.child {
            ChildContent::MultiView(mv) => mv.widget().clone().into(),
            ChildContent::SearchPage(sp) => sp.widget().clone().into(),
        }
    }
}
