
use gtk::prelude::*;
use log::{debug, error, warn};
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use rspotify::{model::Offset, prelude::*};

use crate::{
    navigation::{NavCommand, NavOutput},
    spotconn::{
        model::{format_search_type, SpotItem},
        SpotConn,
    },
};

use super::smallblock;

#[derive(Debug, Clone)]
pub struct Init {
    pub spot: SpotConn,
    pub source: SpotItem,
}

#[derive(Debug)]
pub struct Model {
    init: Init,
    dense_items: FactoryVecDeque<ChildItem>,
    cursor: Option<DynamicIndex>,
}

impl Model {
    fn item_under_cursor(&self) -> Option<SpotItem> {
        let item = self
            .dense_items
            .get(self.cursor.as_ref()?.current_index())?;
        Some(item.sb.model().get_content().clone())
    }

    /// Obtains an init struct for descending into currently selected item.
    /// Returns None if no item is selected, or the selected item
    /// cannot be descended into (it's a track).
    pub fn descend(&self) -> Option<Init> {
        debug!("Attempting a descent into {:?}", self.item_under_cursor());
        match self.item_under_cursor()? {
            // Tracks cannot be descended into, anything else can:
            SpotItem::Track(_) => None,
            item => Some(Init {
                spot: self.init.spot.clone(),
                source: item.clone(),
            }),
        }
    }

    // Reutrns an rspotify PlayContext for the collection, including the cursor position.
    pub fn play_context(&self) -> Option<(PlayContextId<'static>, Option<Offset>)> {
        let item = self.item_under_cursor();
        match (self.init.source.context_id(), item) {
            (None, None) => {
                debug!(
                    "play_context: None, because no item is selected and list is {:?}",
                    self.init.source
                );
                None
            }
            (None, Some(item)) => match item.context_id() {
                Some(ctx) => {
                    debug!(
                        "play_context: for item, because list is {:?}",
                        self.init.source
                    );
                    Some((ctx.clone_static(), None))
                }
                None => {
                    debug!("play_context: None, because item is {:?}", item);
                    None
                }
            },
            (Some(ctx), item) => {
                debug!("Play context for list {:?}", self.init.source);
                Some((
                    ctx.clone_static(),
                    item.and_then(|it| it.uri()).map(Offset::Uri),
                ))
            }
        }
    }
}

#[derive(Debug)]
pub enum In {
    Nav(NavCommand),
    MoveCursorTo(DynamicIndex),
    Reset(SpotItem),
}

#[derive(Debug)]
pub enum Out {
    Nav(NavOutput),
}

#[derive(Debug)]
pub enum CmdOut {
    AddItem(SpotItem),
}

#[relm4::component(pub)]
impl relm4::Component for Model {
    type Init = Init;
    type Input = In;
    type Output = Out;
    type CommandOutput = CmdOut;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_hexpand: true,
            set_vexpand: true,

            gtk::Label {
                set_label: &model.list_title(),
            },

            #[local_ref]
            dense_list -> gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        // Old-style PlaylistItem children:
        // New-style DenseItem children
        let dense_items = FactoryVecDeque::<ChildItem>::builder()
            .launch(gtk::Box::default())
            // Forward DenseItem output messages to our own input, so that we can handle cursor movements:
            .forward(sender.input_sender(), move |out| match out {
                ChildOut::Clicked(idx) => In::MoveCursorTo(idx),
            });

        let model = Model {
            init,
            dense_items,
            cursor: None,
        };
        let dense_list = model.dense_items.widget();

        let widgets = view_output!();

        Model::init_data_loading(&model.init.spot, &model.init.source, &sender);

        relm4::ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            In::Nav(NavCommand::ClearCursor) => {
                let mut items = self.dense_items.guard();
                if let Some(item) = self
                    .cursor
                    .clone()
                    .and_then(|cursor| items.get_mut(cursor.current_index()))
                {
                    item.has_cursor = false;
                }
                self.cursor = None;
            }
            In::Reset(source) => {
                self.init.source = source;
                self.dense_items.guard().clear();
                Model::init_data_loading(&self.init.spot, &self.init.source, &sender);
            }
            In::MoveCursorTo(dyn_idx) => {
                let mut items = self.dense_items.guard();
                let mut move_focus_to: Option<gtk::Button> = None;
                if let Some(item) = self
                    .cursor
                    .clone()
                    .and_then(|cursor| items.get_mut(cursor.current_index()))
                {
                    item.has_cursor = false;
                }
                match items.get_mut(dyn_idx.current_index()) {
                    Some(item) => {
                        item.has_cursor = true;
                        self.cursor = Some(dyn_idx);
                        move_focus_to = Some(item.sb.widget().clone());
                        sender
                            .output_sender()
                            .emit(Out::Nav(NavOutput::CursorIsNowAt(
                                item.sb.model().get_content().clone(),
                            )));
                    }
                    None => error!("cannot set cursor, message back up?"),
                }
            }
            // TODO: distinguish between directions
            In::Nav(NavCommand::Up) => self.move_cursor(-1, &sender),
            In::Nav(NavCommand::Left) => self.move_cursor(-1, &sender),
            In::Nav(NavCommand::Down) => self.move_cursor(1, &sender),
            In::Nav(NavCommand::Right) => self.move_cursor(1, &sender),
        }
    }
    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CmdOut::AddItem(item) => {
                let spot = self.init.spot.clone();
                self.dense_items.guard().push_back(item);
            }
        }
    }
}

impl Model {
    fn list_title(&self) -> String {
        match self.init.source {
            SpotItem::SearchResults { ref st, ref query } => {
                format!("{} matching {}", format_search_type(st), query)
            }
            _ => "".to_string(),
        }
    }

    pub fn current_item(&self) -> Option<SpotItem> {
        let item = self.dense_items.get(self.cursor.clone()?.current_index())?;
        Some(item.sb.model().get_content().clone())
    }

    pub fn current_widget(&self) -> Option<gtk::Widget> {
        self.dense_items
            .get(self.cursor.clone()?.current_index())
            .map(|child| child.sb.widget().clone().into())
    }

    fn init_data_loading(spot: &SpotConn, source: &SpotItem, sender: &ComponentSender<Model>) {
        let spot = spot.clone();
        debug!("Initializing data load for source {:?}", source);
        match source.clone() {
            SpotItem::UserPlaylists => sender.command(move |out, shutdown| {
                spot.current_user_playlists_until_shutdown(shutdown, move |sp| {
                    out.emit(CmdOut::AddItem(SpotItem::Playlist(sp)))
                })
            }),

            SpotItem::Playlist(sp) => sender.command(move |out, shutdown| {
                spot.tracks_in_playlist(shutdown, sp.id.uri(), move |ft| {
                    out.emit(CmdOut::AddItem(SpotItem::Track(ft)))
                })
            }),
            SpotItem::Track(_) => {
                panic!("a single track should never be rendered as a list");
            }
            SpotItem::Album(a) => match source.uri() {
                Some(uri) => sender.command(move |out, shutdown| {
                    spot.tracks_in_album(shutdown, uri, move |ft| {
                        out.emit(CmdOut::AddItem(SpotItem::Track(ft)))
                    })
                }),
                None => {
                    error!(
                        "Cannot fetch tracks for album {}, as it does not have an URI.",
                        a.name
                    );
                }
            },
            SpotItem::SearchResults { st, query } => sender.command(move |out, shutdown| {
                spot.search(st, query, move |item| out.emit(CmdOut::AddItem(item)))
            }),
        }
    }

    fn move_cursor(&mut self, delta: i32, sender: &ComponentSender<Self>) {
        let next_id = match self.cursor.clone() {
            Some(cursor) => {
                // TODO: remove the next line, has_cursor=false is handled in MoveCursorTo
                self.dense_items
                    .guard()
                    .get_mut(cursor.current_index())
                    .unwrap()
                    .has_cursor = false;
                self.cursor = None;
                let idx = cursor.current_index() as i32;
                if idx + delta < 0 {
                    warn!("cursor up out of the list");
                    sender.output_sender().emit(Out::Nav(NavOutput::EscapedUp));
                    return;
                }
                if (idx + delta) as usize >= self.dense_items.len() {
                    warn!("cursor down out of the list");
                    sender
                        .output_sender()
                        .emit(Out::Nav(NavOutput::EscapedDown));
                    return;
                }
                (idx + delta) as usize
            }
            None if delta > 0 => 0,
            None => self.dense_items.len() - 1,
        };
        match self.dense_items.get(next_id) {
            Some(next) => sender
                .input_sender()
                .emit(In::MoveCursorTo(next.self_idx.clone())),
            None => {
                error!("should never happen: next_id is within range, yet no child.")
            }
        }
    }
}

/// A FactoryComponent for DenseList. This is just a factory-enabled wrapper around smallblock::Model.
#[derive(Debug)]
struct ChildItem {
    sb: Controller<smallblock::Model>,
    has_cursor: bool,
    self_idx: DynamicIndex,
}

#[derive(Debug)]
enum ChildIn {}

#[derive(Debug)]
enum ChildOut {
    Clicked(DynamicIndex),
}

#[relm4::factory]
impl FactoryComponent for ChildItem {
    type Init = SpotItem;
    type Input = ChildIn;
    type Output = ChildOut;
    type ParentWidget = gtk::Box;
    type CommandOutput = ();

    view! {
        // Ideally we would put the smallblock::Model widget here directly.
        // This doesn't work, though: relm4 expects one top-level component first,
        // so let's use a box. On the pro side, we can attach cursor-related
        // CSS classes here.
        #[root]
        gtk::Box {
            set_css_classes: &["dense-item"],
            #[watch]
            set_class_active: ("has-cursor", self.has_cursor),
            set_orientation: gtk::Orientation::Vertical,
            self.sb.widget() {}
        }
    }

    fn init_model(init: Self::Init, index: &Self::Index, sender: FactorySender<Self>) -> Self {
        let sb = smallblock::Model::builder().launch(init);
        let cloned_index = index.clone();
        // Forward smallblock::Model output messages directly as DenseItem output messages:
        let sb = sb.forward(sender.output_sender(), move |msg| match msg {
            smallblock::Out::Clicked => ChildOut::Clicked(cloned_index.clone()),
        });
        ChildItem {
            sb,
            has_cursor: false,
            self_idx: index.clone(),
        }
    }
}
