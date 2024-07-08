#![allow(dead_code)]
#![allow(unused_variables)]

use gtk::prelude::*;
use librespot::core::spotify_id::SpotifyId;
use log::debug;

use relm4::{self, Component, ComponentController, Controller};
use relm4::{gtk, ComponentParts, ComponentSender, RelmApp};

use crate::actionbuilder::{AccelManager, ActionBuilder};
use crate::components::actions::{Actions, ActionsOutput};
use crate::components::switchview;
use crate::navigation::NavCommand;
use crate::spotconn::SpotConn;

mod actionbuilder;
mod components;
pub(crate) mod navigation;
mod spotconn;

struct AppModel {
    actions: Controller<Actions>,
    switchview: Controller<switchview::Model>,
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
    type Init = ();

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
        _: Self::Init,
        window: Self::Root,
        sender: ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        // let spot_playlist_id = SpotifyId::from_base62("7EsmFgvsvdK7HXh5k5Esbt").unwrap();
        let _spot_track_id = SpotifyId::from_base62("416oYM4vj129L8BP7B0qlO").unwrap();

        let switchview: Controller<switchview::Model> = switchview::Model::builder()
            .launch(switchview::Init {})
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
            switchview,
            actions: actions_model,
        };
        let actions_widget = model.actions.widget();
        let switchview_widget = model.switchview.widget();

        let widgets = view_output!();

        let ab = ActionBuilder::new(window.clone(), "global-controls");
        ab.add("quit", &["<primary>Q"], || {
            relm4::main_application().quit();
        });

        let mut am = AccelManager::new(&window, "global-navigation");
        let svs = &model.switchview.sender().clone();
        am.register_emit("down", &["J"], svs, switchview::In::Nav(NavCommand::Down));
        am.register_emit("up", &["K"], svs, switchview::In::Nav(NavCommand::Up));
        am.register_emit("left", &["H"], svs, switchview::In::Nav(NavCommand::Left));
        am.register_emit("right", &["L"], svs, switchview::In::Nav(NavCommand::Right));
        am.register_emit("descend", &["O"], svs, switchview::In::NavDescend); // O for Open
        am.register_emit("back", &["I"], svs, switchview::In::NavBack); // I because it's on the left side of O

        am.register_emit("reset-search", &["1"], svs, switchview::In::NavResetSearch);
        am.register_emit(
            "reset-playlists",
            &["2"],
            svs,
            switchview::In::NavResetPlaylists,
        );

        am.register_emit(
            "play_now",
            &["<shift>P"],
            sender.input_sender(),
            AppInput::PlayNow,
        );
        am.connect();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppInput::PlayNow => {
                if let Some((ctx, offset)) = self
                    .switchview
                    .model()
                    .current_list()
                    .and_then(|dl| dl.model().play_context())
                {
                    debug!("play now -> ctx is some");
                    _sender.oneshot_command(async move {
                        SpotConn::get().play_context(ctx, offset).await;
                    })
                } else {
                    debug!("playnow -> no ctx");
                }
            }
            AppInput::SpircNow => _sender.oneshot_command(async move {
                SpotConn::get().play_on_spirc().await;
            }),
        }
    }
}

fn main() {
    env_logger::init();
    let app = RelmApp::new("relm4.test.simple_manual");
    app.set_global_css(include_str!("style.css"));
    app.run::<AppModel>(());
}
