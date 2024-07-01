use futures::stream::TryStreamExt;

use gtk::graphene::Point;
use gtk::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use rspotify::clients::OAuthClient;
use rspotify::model::SimplifiedPlaylist;

use crate::spotconn::SpotConn;

use super::smallblock::{BlockInit, SmallBlock, SmallBlockOutput};

#[derive(Debug)]
pub struct DenseList {
    spot: SpotConn,
    dense_items: FactoryVecDeque<DenseItem>,
    cursor: Option<DynamicIndex>,
    scrollwin: gtk::ScrolledWindow,
}

/// The source of all Spotify items in the list.
/// Spotify docs sometimes refer to this as "context".
#[derive(Debug)]
pub enum Source {
    UserPlaylists,
}

#[derive(Debug)]
pub struct DenseListInit {
    pub spot: SpotConn,
    pub source: Source,
}

#[derive(Debug)]
pub enum DenseListInput {
    CursorMove(i32),
    MoveCursorTo(DynamicIndex),
}

#[derive(Debug)]
pub enum DenseListOutput {}

#[derive(Debug)]
pub enum DenseListCommandOutput {
    AddItem(BlockInit),
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
            spot: init.spot,
            dense_items,
            cursor: None,
            scrollwin: gtk::ScrolledWindow::new(),
        };
        let dense_list = model.dense_items.widget();

        let widgets = view_output!();
        model.scrollwin = widgets.scrollboxes.clone();

        let spot = model.spot.clone();
        sender.command(move |out, shutdown| {
            spot.current_user_playlists_until_shutdown(shutdown, move |sp| {
                out.send(DenseListCommandOutput::AddItem(
                    BlockInit::SimplifiedPlaylist(sp),
                ))
                .unwrap();
            })
        });

        relm4::ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            DenseListInput::MoveCursorTo(dyn_idx) => {
                let mut items = self.dense_items.guard();
                if let Some(item) = self
                    .cursor
                    .clone()
                    .and_then(|cursor| items.get_mut(cursor.current_index()))
                {
                    println!(
                        "Clearing previous cursor {}",
                        self.cursor.clone().unwrap().current_index()
                    );
                    item.has_cursor = false;
                }
                match items.get_mut(dyn_idx.current_index()) {
                    Some(item) => {
                        item.has_cursor = true;
                        self.cursor = Some(dyn_idx);
                    }
                    None => println!("cannot set cursor, message back up?"),
                }
            }
            DenseListInput::CursorMove(delta) => {
                let next_id = match self.cursor.clone() {
                    Some(cursor) => {
                        self.dense_items
                            .guard()
                            .get_mut(cursor.current_index())
                            .unwrap()
                            .has_cursor = false;
                        let idx = cursor.current_index() as i32;
                        if idx + delta < 0 {
                            println!("TODO: escaping the list");
                            return;
                        }
                        (idx + delta) as usize
                    }
                    None if delta > 0 => 1,
                    None => self.dense_items.len() - 1,
                };
                match self.dense_items.get(next_id) {
                    Some(next) => _sender
                        .input_sender()
                        .emit(DenseListInput::MoveCursorTo(next.self_idx.clone())),
                    None => {
                        println!("moving out of the list!") // TODO: send a message?
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
                let spot = self.spot.clone();
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
        println!(
            "next item point relative to list root: {} {}",
            point.x(),
            point.y()
        );
        println!("current size: {}", self.scrollwin.height());
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
            println!("Correcting adjustment from {} by {}", adj.value(), delta);
            adj.set_value(adj.value() + delta);
            self.scrollwin.set_vadjustment(Some(&adj));
        }
    }

    pub fn current_item(&self) -> Option<BlockInit> {
        let item = self.dense_items.get(self.cursor.clone()?.current_index())?;
        Some(item.sb.model().get_content().clone())
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
    type Init = BlockInit;
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
