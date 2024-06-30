use std::time::Duration;

use futures::stream::TryStreamExt;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::graphene::Point;
use gtk::{glib, prelude::*};
use relm4::factory::{FactoryVecDeque, FactoryVecDequeBuilder};
use relm4::prelude::*;
use rspotify::clients::OAuthClient;
use rspotify::model::SimplifiedPlaylist;

use crate::components::playlistitem::{PlaylistItem, PlaylistItemInit, PlaylistItemOutput};
use crate::spotconn::SpotConn;

#[derive(Debug)]
pub struct DenseList {
    spot: SpotConn,
    items: FactoryVecDeque<PlaylistItem>,
    cursor: Option<DynamicIndex>,
    scrollwin: gtk::ScrolledWindow,
}

#[derive(Debug)]
pub struct DenseListInit {
    pub spot: SpotConn,
    // TODO: add an init option to choose which playlists to load?
}

#[derive(Debug)]
pub enum DenseListInput {
    CursorMove(i32),
    MoveCursorTo(DynamicIndex),
}

#[derive(Debug)]
pub enum DenseListOutput {}

/// We allow loading items in a streaming manner: multiple messages with one item each.
#[derive(Debug)]
pub enum DenseListCommandOutput {
    ItemLoaded(SimplifiedPlaylist),
}

#[relm4::component(pub)]
impl relm4::Component for DenseList {
    type Init = DenseListInit;
    type Input = DenseListInput;
    type Output = DenseListOutput;
    type CommandOutput = DenseListCommandOutput;

    view! {
        #[root]
        #[name="scrollwin"]
        gtk::ScrolledWindow {
            // set_policy:

            #[local_ref]
            list_view -> gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let items = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |output| match output {
                PlaylistItemOutput::CaptcuredCursorByClick(dyn_idx) => {
                    DenseListInput::MoveCursorTo(dyn_idx)
                }
            });
        let mut model = DenseList {
            spot: init.spot,
            items,
            cursor: None,
            scrollwin: gtk::ScrolledWindow::new(),
        };
        let list_view = model.items.widget();
        let widgets = view_output!();
        model.scrollwin = widgets.scrollwin.clone();

        let spot = model.spot.clone();
        // TODO: this hardcodes library fetch, abstract away?
        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let rspot = spot.rspot().await;
                    let mut stream = rspot.current_user_playlists();
                    while let Some(item) = stream.try_next().await.unwrap() {
                        out.send(DenseListCommandOutput::ItemLoaded(item)).unwrap();
                    }
                })
                .drop_on_shutdown()
        });
        relm4::ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        println!("Received msg: {:?}", msg);
        match msg {
            DenseListInput::MoveCursorTo(dyn_idx) => {
                let mut items = self.items.guard();
                if let Some(item) = self
                    .cursor
                    .clone()
                    .and_then(|cursor| items.get_mut(cursor.current_index()))
                {
                    println!(
                        "Clearing previous cursor {}",
                        self.cursor.clone().unwrap().current_index()
                    );
                    item.set_has_cursor(false);
                }
                match items.get_mut(dyn_idx.current_index()) {
                    Some(item) => {
                        item.set_has_cursor(true);
                        self.cursor = Some(dyn_idx);
                    }
                    None => println!("cannot set cursor, message back up?"),
                }
            }
            DenseListInput::CursorMove(delta) => {
                let mut next: Option<&mut PlaylistItem> = None;
                let mut items = self.items.guard();
                match self.cursor.clone() {
                    Some(cursor) => {
                        items
                            .get_mut(cursor.current_index())
                            .unwrap()
                            .set_has_cursor(false);
                        next = items.get_mut(((cursor.current_index() as i32) + delta) as usize);
                    }
                    None if delta > 0 => {
                        next = items.get_mut(0);
                    }
                    None => {
                        let idx = items.len() - 1;
                        next = items.get_mut(idx);
                    }
                }
                if let Some(next) = next {
                    next.set_has_cursor(true);
                    self.cursor = Some(next.self_idx.clone());
                    let point = next
                        .widget_root
                        .borrow()
                        .as_ref()
                        .unwrap()
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
                } else {
                    println!("should move focus out in direction {}", delta);
                    self.cursor = None;
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
            DenseListCommandOutput::ItemLoaded(playlist) => {
                let spot = self.spot.clone();
                self.items.guard().push_back(PlaylistItemInit {
                    spot: spot,
                    simple: playlist,
                });
            }
        }
    }
}
