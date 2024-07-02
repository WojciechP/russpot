#![allow(dead_code)]
#![allow(unused_variables)]
use gtk::gio::SimpleActionGroup;
use gtk::prelude::*;
use librespot::core::spotify_id::SpotifyId;
use log::debug;
use relm4::actions::{AccelsPlus, ActionName};
use relm4::actions::{RelmAction, RelmActionGroup};
use relm4::{self, Component, ComponentController, Controller};
use relm4::{gtk, ComponentParts, ComponentSender, RelmApp};
use spotconn::model::SpotItem;

use crate::actionbuilder::ActionBuilder;
use crate::components::actions::{Actions, ActionsOutput};
use crate::components::switchview::{SwitchView, SwitchViewInit, SwitchViewInput};
use crate::spotconn::SpotConn;

mod actionbuilder;
mod components;
mod spotconn;

struct AppModel {
    counter: u8,
    spot: SpotConn,

    actions: Controller<Actions>,
    switchview: Controller<SwitchView>,
}

#[derive(Debug, Copy, Clone)]
enum AppInput {
    PlayNow,
    SpircNow,
}

#[relm4::component]
impl relm4::SimpleComponent for AppModel {
    /// The type of the messages that this component can receive.
    type Input = AppInput;
    /// The type of the messages that this component can send.
    type Output = ();
    /// The type of data with which this component will be initialized.
    type Init = u8;

    view! {
        main_window = gtk::Window {
            set_title: Some("Russpot"),
            set_default_width: 300,
            set_default_height: 100,
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_vexpand: true,

                #[local_ref]
                switchview_widget -> gtk::Box {
                    set_width_request: 200,
                    set_vexpand: true,
                },

                #[local_ref]
                actions_widget -> gtk::Box {
                    set_width_request: 200,
                },
            },
        }
    }

    /// Initialize the UI and model.
    fn init(
        counter: Self::Init,
        window: Self::Root,
        sender: ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        // let spot_playlist_id = SpotifyId::from_base62("7EsmFgvsvdK7HXh5k5Esbt").unwrap();
        let _spot_track_id = SpotifyId::from_base62("416oYM4vj129L8BP7B0qlO").unwrap();
        let spot = SpotConn::new();

        let switchview: Controller<SwitchView> = SwitchView::builder()
            .launch(SwitchViewInit {})
            .forward(sender.input_sender(), |msg| match msg {});

        let actions_model = Actions::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                ActionsOutput::PlayNow => {
                    println!("forwarding playnow");
                    AppInput::PlayNow
                }
                ActionsOutput::SpircNow => {
                    println!("forwarding spirc");
                    AppInput::SpircNow
                }
            });
        let model = AppModel {
            spot: spot,
            counter,
            switchview,
            actions: actions_model,
        };
        let actions_widget = model.actions.widget();
        let switchview_widget = model.switchview.widget();

        let widgets = view_output!();

        let ab = ActionBuilder::new(window, "built-sag");
        let svs = &model.switchview.sender().clone();
        ab.add_emit("down", &["J"], svs, SwitchViewInput::CursorMove(1));
        ab.add_emit("up", &["K"], svs, SwitchViewInput::CursorMove(-1));
        ab.add_emit("left", &["H"], svs, SwitchViewInput::CursorMove(0)); // TODO: implement left/right
        ab.add_emit("right", &["L"], svs, SwitchViewInput::CursorMove(0)); // TODO: implement left/right
        ab.add_emit("descend", &["O"], svs, SwitchViewInput::NavDescend); // O for Open
        ab.add_emit("back", &["I"], svs, SwitchViewInput::NavBack); // I because it's on the left side of O
        ab.add_emit("play_now", &["P"], sender.input_sender(), AppInput::PlayNow);
        ab.add("quit", &["<primary>Q"], || {
            relm4::main_application().quit();
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppInput::PlayNow => {
                if let Some((ctx, offset)) = self
                    .switchview
                    .model()
                    .current_list()
                    .model()
                    .play_context()
                {
                    debug!("play now -> ctx is some");
                    let spot = self.spot.clone();
                    _sender.oneshot_command(async move {
                        spot.play_context(ctx, offset).await;
                    })
                } else {
                    debug!("playnow -> no ctx");
                }
            }
            AppInput::SpircNow => {
                let spot = self.spot.clone();
                _sender.oneshot_command(async move {
                    spot.play_on_spirc().await;
                })
            }
        }
    }
}

impl AppModel {
    fn play_now(&self, sender: ComponentSender<AppModel>, item: SpotItem) {
        let spot = self.spot.clone();
        match item {
            SpotItem::Playlist(sp) => sender.oneshot_command(async move {
                println!("starting playback of playlist {}", sp.name);
                spot.play_playlist(sp.id).await;
                ()
            }),
            SpotItem::Track(ft) => todo!("Cannot play tracks yet"),
        }
    }
}

fn main() {
    env_logger::init();
    let app = RelmApp::new("relm4.test.simple_manual");
    app.set_global_css(include_str!("style.css"));
    app.run::<AppModel>(0);
}
