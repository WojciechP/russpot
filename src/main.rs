use components::denselist::DenseList;
use components::spotitem::SpotItemModel;
use gtk::prelude::*;
use librespot::core::spotify_id::SpotifyId;
use relm4::actions::AccelsPlus;
use relm4::actions::{RelmAction, RelmActionGroup};
use relm4::{self, Component, ComponentController, Controller};
use relm4::{gtk, view, ComponentParts, ComponentSender, RelmApp, RelmWidgetExt, SimpleComponent};

use crate::components::denselist::DenseListInit;
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
        gtk::Window {
            set_title: Some("Simple app"),
            set_default_width: 300,
            set_default_height: 100,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,

                model.spot_item.widget(),

                #[name="btn"]
                gtk::Button {
                    set_label: "Increment",
                },
           },

           model.denselist.widget(),
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
                /*
                HeaderOutput::View => AppMsg::SetMode(AppMode::View),
                HeaderOutput::Edit => AppMsg::SetMode(AppMode::Edit),
                HeaderOutput::Export => AppMsg::SetMode(AppMode::Export),
                */
            });

        let denselist: Controller<DenseList> = DenseList::builder()
            .launch(DenseListInit { spot: spot.clone() })
            .forward(sender.input_sender(), |msg| match msg {});
        let model = AppModel {
            counter,
            spot_item,
            denselist,
        };

        let widgets = view_output!();

        relm4::main_application().set_accelerators_for_action::<ActionQuit>(&["<primary>Q"]);
        let a_quit: RelmAction<ActionQuit> = RelmAction::new_stateless(move |_| {
            relm4::main_application().quit();
        });
        let mut action_group = RelmActionGroup::<WindowActionGroup>::new();
        action_group.add_action(a_quit);
        action_group.register_for_main_application();

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

relm4::new_action_group!(WindowActionGroup, "win");
relm4::new_stateless_action!(ActionQuit, WindowActionGroup, "quit");

fn main() {
    let app = RelmApp::new("relm4.test.simple_manual");
    app.set_global_css(include_str!("style.css"));
    app.run::<AppModel>(0);
}
