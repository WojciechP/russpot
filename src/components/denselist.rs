use gtk::graphene::Point;
use gtk::prelude::*;
use log::{debug, error, warn};
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use rspotify::{model::Offset, prelude::*};

use crate::spotconn::{model::SpotItem, SpotConn};

use super::smallblock::{SmallBlock, SmallBlockOutput};

#[derive(Debug, Clone)]
pub struct DenseListInit {
    pub spot: SpotConn,
    pub source: SpotItem,
}

#[derive(Debug)]
pub struct DenseList {
    init: DenseListInit,
    dense_items: FactoryVecDeque<DenseItem>,
    cursor: Option<DynamicIndex>,
    scrollwin: gtk::ScrolledWindow,
}

impl DenseList {
    fn item_under_cursor(&self) -> Option<SpotItem> {
        let item = self
            .dense_items
            .get(self.cursor.as_ref()?.current_index())?;
        Some(item.sb.model().get_content().clone())
    }

    /// Obtains an init struct for descending into currently selected item.
    /// Returns None if no item is selected, or the selected item
    /// cannot be descended into (it's a track).
    pub fn descend(&self) -> Option<DenseListInit> {
        match self.item_under_cursor()? {
            // Tracks cannot be descended into, anything else can:
            SpotItem::Track(_) => None,
            item => Some(DenseListInit {
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
pub enum DenseListInput {
    CursorMove(i32),
    MoveCursorTo(DynamicIndex),
    DestroyCursor,
    Reset(SpotItem),
}

#[derive(Debug)]
pub enum DenseListOutput {
    CursorEscapedDown,
    CursorEscapedUp,
}

#[derive(Debug)]
pub enum DenseListCommandOutput {
    AddItem(SpotItem),
}

#[relm4::component(pub)]
impl relm4::Component for DenseList {
    type Init = DenseListInit;
    type Input = DenseListInput;
    type Output = DenseListOutput;
    type CommandOutput = DenseListCommandOutput;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_hexpand: true,
            set_vexpand: true,
            #[name="scrollboxes"]
            gtk::ScrolledWindow {
                set_hexpand: true,
                #[local_ref]
                dense_list -> gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                }
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
        let dense_items = FactoryVecDeque::<DenseItem>::builder()
            .launch(gtk::Box::default())
            // Forward DenseItem output messages to our own input, so that we can handle cursor movements:
            .forward(sender.input_sender(), move |out| match out {
                DenseItemOutput::Clicked(idx) => DenseListInput::MoveCursorTo(idx),
            });

        let mut model = DenseList {
            init,
            dense_items,
            cursor: None,
            scrollwin: gtk::ScrolledWindow::new(),
        };
        let dense_list = model.dense_items.widget();

        let widgets = view_output!();
        model.scrollwin = widgets.scrollboxes.clone();

        DenseList::init_data_loading(&model.init.spot, &model.init.source, &sender);

        relm4::ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            DenseListInput::DestroyCursor => {
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
            DenseListInput::Reset(source) => {
                self.init.source = source;
                self.dense_items.guard().clear();
                DenseList::init_data_loading(&self.init.spot, &self.init.source, &sender);
            }
            DenseListInput::MoveCursorTo(dyn_idx) => {
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
                    }
                    None => error!("cannot set cursor, message back up?"),
                }
                drop(items); // release guard on items to be able to call ensure_visible
                if let Some(next) = move_focus_to {
                    self.ensure_visible(&next);
                }
            }
            DenseListInput::CursorMove(delta) => {
                let next_id = match self.cursor.clone() {
                    Some(cursor) => {
                        // TODO: remove the next line, has_cursor=false is handled in MoveCursorTo
                        self.dense_items
                            .guard()
                            .get_mut(cursor.current_index())
                            .unwrap()
                            .has_cursor = false;
                        let idx = cursor.current_index() as i32;
                        if idx + delta < 0 {
                            warn!("cursor up out of the list");
                            sender
                                .output_sender()
                                .emit(DenseListOutput::CursorEscapedUp);
                            return;
                        }
                        if (idx + delta) as usize >= self.dense_items.len() {
                            warn!("cursor down out of the list");
                            sender
                                .output_sender()
                                .emit(DenseListOutput::CursorEscapedDown);
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
                        .emit(DenseListInput::MoveCursorTo(next.self_idx.clone())),
                    None => {
                        error!("should never happen: next_id is within range, yet no child.")
                    }
                }
            }
        }
    }
    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            DenseListCommandOutput::AddItem(item) => {
                let spot = self.init.spot.clone();
                self.dense_items.guard().push_back(item);
            }
        }
    }
}

impl DenseList {
    /// Scrolls the list to make sure that the passed child is visible.
    fn ensure_visible(&self, widget: &gtk::Button) {
        let point = widget
            .compute_point(&self.scrollwin, &Point::new(0.0, 0.0))
            .unwrap();
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
        }
    }

    pub fn current_item(&self) -> Option<SpotItem> {
        let item = self.dense_items.get(self.cursor.clone()?.current_index())?;
        Some(item.sb.model().get_content().clone())
    }

    fn init_data_loading(spot: &SpotConn, source: &SpotItem, sender: &ComponentSender<DenseList>) {
        let spot = spot.clone();
        debug!("Initializing data load for source {:?}", source);
        match source.clone() {
            SpotItem::UserPlaylists => sender.command(move |out, shutdown| {
                spot.current_user_playlists_until_shutdown(shutdown, move |sp| {
                    out.emit(DenseListCommandOutput::AddItem(SpotItem::Playlist(sp)))
                })
            }),

            SpotItem::Playlist(sp) => sender.command(move |out, shutdown| {
                spot.tracks_in_playlist(shutdown, sp.id.uri(), move |ft| {
                    out.emit(DenseListCommandOutput::AddItem(SpotItem::Track(ft)))
                })
            }),
            SpotItem::Track(_) => {
                panic!("a single track should never be rendered as a list");
            }
            SpotItem::SearchResults { st, query } => sender.command(move |out, shutdown| {
                spot.search(st, query, move |item| {
                    out.emit(DenseListCommandOutput::AddItem(item))
                })
            }),
        }
    }
}

/// A FactoryComponent for DenseList. This is just a factory-enabled wrapper around SmallBlock.
#[derive(Debug)]
struct DenseItem {
    sb: Controller<SmallBlock>,
    has_cursor: bool,
    self_idx: DynamicIndex,
}

#[derive(Debug)]
enum DenseItemInput {}

#[derive(Debug)]
enum DenseItemOutput {
    Clicked(DynamicIndex),
}

#[relm4::factory]
impl FactoryComponent for DenseItem {
    type Init = SpotItem;
    type Input = DenseItemInput;
    type Output = DenseItemOutput;
    type ParentWidget = gtk::Box;
    type CommandOutput = ();

    view! {
        // Ideally we would put the SmallBlock widget here directly.
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
        let sb = SmallBlock::builder().launch(init);
        let cloned_index = index.clone();
        // Forward SmallBlock output messages directly as DenseItem output messages:
        let sb = sb.forward(sender.output_sender(), move |msg| match msg {
            SmallBlockOutput::Clicked => DenseItemOutput::Clicked(cloned_index.clone()),
        });
        DenseItem {
            sb,
            has_cursor: false,
            self_idx: index.clone(),
        }
    }
}
