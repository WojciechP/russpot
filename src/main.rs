use components::denselist::DenseList;
use components::spotitem::SpotItemModel;
use gtk::prelude::*;
use librespot::core::spotify_id::SpotifyId;
use relm4::actions::{AccelsPlus, ActionName, ActionablePlus};
use relm4::actions::{RelmAction, RelmActionGroup};
use relm4::{self, Component, ComponentController, Controller};
use relm4::{gtk, view, ComponentParts, ComponentSender, RelmApp, RelmWidgetExt, SimpleComponent};

use crate::components::denselist::{DenseListInit, DenseListInput};
use crate::components::spotitem::SpotifyItemInit;
use crate::spotconn::SpotConn;

mod components;
mod spotconn;

struct AppModel {
    counter: u8,
    spot_item: Controller<SpotItemModel>,

    denselist: Controller<DenseList>,
}

#[derive(Debug)]
enum AppInput {
    Increment,
    Decrement,
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
                set_orientation: gtk::Orientation::Vertical,

                #[name="btn"]
                gtk::MenuButton {
                    set_menu_model: Some(&menu_model),
                },


                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_homogeneous: false,
                     #[local_ref]
                     denselist_widget -> gtk::ScrolledWindow{
                         set_vexpand: true,
                     },
                }
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
        let spot_track_id = SpotifyId::from_base62("416oYM4vj129L8BP7B0qlO").unwrap();
        let spot = SpotConn::new();
        let spot_item: Controller<SpotItemModel> = SpotItemModel::builder()
            .launch(SpotifyItemInit {
                spot: spot.clone(),
                id: spot_track_id,
            })
            .forward(sender.input_sender(), |msg| match msg {
                _ => panic!("child should not have msgs yet"),
            });

        let denselist: Controller<DenseList> = DenseList::builder()
            .launch(DenseListInit { spot: spot.clone() })
            .forward(sender.input_sender(), |msg| match msg {});
        let model = AppModel {
            counter,
            spot_item,
            denselist,
        };

        let menu_model = gtk::gio::Menu::new();
        menu_model.append(Some("Down"), Some(&ActionDown::action_name()));
        menu_model.append(Some("Up"), Some(&ActionUp::action_name()));

        let denselist_widget = model.denselist.widget();
        let widgets = view_output!();

        let mut app = relm4::main_application();

        app.set_accelerators_for_action::<ActionQuit>(&["<primary>Q"]);
        let denselist_sender = model.denselist.sender().clone();
        let a_quit: RelmAction<ActionQuit> = RelmAction::new_stateless(move |_| {
            println!("quit");
            relm4::main_application().quit();
            denselist_sender.emit(DenseListInput::CursorMove(1));
        });
        let denselist_sender = model.denselist.sender().clone();

        app.set_accelerators_for_action::<ActionDown>(&["J"]);
        let denselist_sender = model.denselist.sender().clone();
        let a_down: RelmAction<ActionDown> = RelmAction::new_stateless(move |_| {
            //         app.quit();
            println!("actin down");
            denselist_sender
                .send(DenseListInput::CursorMove(1))
                .unwrap();
        });
        app.set_accelerators_for_action::<ActionUp>(&["K"]);
        let denselist_sender = model.denselist.sender().clone();
        let a_up: RelmAction<ActionUp> = RelmAction::new_stateless(move |_| {
            println!("actin up");
            denselist_sender
                .send(DenseListInput::CursorMove(-1))
                .unwrap();
        });
        let mut action_group = RelmActionGroup::<BozoActionGroup>::new();
        action_group.add_action(a_down);
        action_group.add_action(a_up);
        action_group.add_action(a_quit);
        action_group.register_for_widget(widgets.main_window.clone());
        println!("ag: {:?}\n", relm4::main_application().list_actions());

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppInput::Increment => {
                self.counter = self.counter.wrapping_add(1);
            }
            AppInput::Decrement => {
                self.counter = self.counter.wrapping_sub(1);
            }
        }
    }
}

relm4::new_action_group!(BozoActionGroup, "bozo");
relm4::new_stateless_action!(ActionQuit, BozoActionGroup, "quitquitquit");
relm4::new_stateless_action!(ActionDown, BozoActionGroup, "down");
relm4::new_stateless_action!(ActionUp, BozoActionGroup, "up");

fn main() {
    let app = RelmApp::new("relm4.test.simple_manual");
    app.set_global_css(include_str!("style.css"));
    app.run::<AppModel>(0);
}
