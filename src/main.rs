use components::smallblock::BlockInit;
use gtk::prelude::*;
use librespot::core::spotify_id::SpotifyId;
use relm4::actions::{AccelsPlus, ActionName};
use relm4::actions::{RelmAction, RelmActionGroup};
use relm4::{self, Component, ComponentController, Controller};
use relm4::{gtk, ComponentParts, ComponentSender, RelmApp};

use crate::components::actions::{Actions, ActionsOutput};
use crate::components::switchview::{SwitchView, SwitchViewInit, SwitchViewInput};
use crate::spotconn::SpotConn;

mod components;
mod spotconn;

struct AppModel {
    counter: u8,
    spot: SpotConn,

    actions: Controller<Actions>,
    switchview: Controller<SwitchView>,
}

#[derive(Debug)]
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
                #[name="btn"]
                gtk::MenuButton {
                    set_menu_model: Some(&menu_model),
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

        let menu_model = gtk::gio::Menu::new();
        menu_model.append(Some("Down"), Some(&ActionDown::action_name()));
        menu_model.append(Some("Up"), Some(&ActionUp::action_name()));

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

        let app = relm4::main_application();

        app.set_accelerators_for_action::<ActionQuit>(&["<primary>Q"]);
        let denselist_sender = model.switchview.sender().clone();
        let a_quit: RelmAction<ActionQuit> = RelmAction::new_stateless(move |_| {
            println!("quit");
            relm4::main_application().quit();
            denselist_sender.emit(SwitchViewInput::CursorMove(1));
        });
        let _denselist_sender = model.switchview.sender().clone();

        app.set_accelerators_for_action::<ActionDown>(&["J"]);
        let denselist_sender = model.switchview.sender().clone();
        let a_down: RelmAction<ActionDown> = RelmAction::new_stateless(move |_| {
            //         app.quit();
            println!("actin down");
            denselist_sender
                .send(SwitchViewInput::CursorMove(1))
                .unwrap();
        });

        app.set_accelerators_for_action::<ActionUp>(&["K"]);
        let denselist_sender = model.switchview.sender().clone();
        let a_up: RelmAction<ActionUp> = RelmAction::new_stateless(move |_| {
            println!("actin up");
            denselist_sender
                .send(SwitchViewInput::CursorMove(-1))
                .unwrap();
        });

        app.set_accelerators_for_action::<ActionDescend>(&["O"]); // O for Open
        let denselist_sender = model.switchview.sender().clone();
        let a_descend: RelmAction<ActionDescend> = RelmAction::new_stateless(move |_| {
            println!("actin descend");
            denselist_sender.send(SwitchViewInput::NavDescend).unwrap();
        });
        app.set_accelerators_for_action::<ActionBack>(&["I"]); // I because it's on the left side of O
        let denselist_sender = model.switchview.sender().clone();
        let a_back: RelmAction<ActionBack> = RelmAction::new_stateless(move |_| {
            println!("actin back");
            denselist_sender.send(SwitchViewInput::NavBack).unwrap();
        });

        app.set_accelerators_for_action::<ActionPlayNow>(&["<shift>P"]);
        let denselist_sender = model.switchview.sender().clone();
        let a_play: RelmAction<ActionPlayNow> = RelmAction::new_stateless(move |_| {
            println!("actin play - not implemented");
        });

        let mut action_group = RelmActionGroup::<BozoActionGroup>::new();
        action_group.add_action(a_down);
        action_group.add_action(a_up);
        action_group.add_action(a_quit);
        action_group.add_action(a_descend);
        action_group.add_action(a_back);
        action_group.add_action(a_play);
        action_group.register_for_widget(widgets.main_window.clone());
        println!("ag: {:?}\n", relm4::main_application().list_actions());

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppInput::PlayNow => {
                todo!("forward PlayNow to switchview?")
            }
            /*
            AppInput::PlayNow => match self.denselist.model().current_item() {
                Some(item) => {
                    self.play_now(_sender, item);
                }
                None => {
                    println!("cannot play, cursor is empty")
                }
            },
            */
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
    fn play_now(&self, sender: ComponentSender<AppModel>, item: BlockInit) {
        let spot = self.spot.clone();
        match item {
            BlockInit::SimplifiedPlaylist(sp) => sender.oneshot_command(async move {
                println!("starting playback of playlist {}", sp.name);
                spot.play_playlist(sp.id).await;
                ()
            }),
            BlockInit::FullTrack(tr) => todo!("Cannot play tracks yet"),
        }
    }
}

relm4::new_action_group!(BozoActionGroup, "bozo");
relm4::new_stateless_action!(ActionQuit, BozoActionGroup, "quitquitquit");
relm4::new_stateless_action!(ActionDown, BozoActionGroup, "down");
relm4::new_stateless_action!(ActionUp, BozoActionGroup, "up");
relm4::new_stateless_action!(ActionDescend, BozoActionGroup, "descend");
relm4::new_stateless_action!(ActionBack, BozoActionGroup, "back");
relm4::new_stateless_action!(ActionPlayNow, BozoActionGroup, "play_now");

fn main() {
    env_logger::init();
    let app = RelmApp::new("relm4.test.simple_manual");
    app.set_global_css(include_str!("style.css"));
    app.run::<AppModel>(0);
}
