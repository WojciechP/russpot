//! Actions is a list of buttons for actions related to currently selected item:
//! play now, go to artist radio etc.

use gtk::prelude::*;
use relm4::{prelude::*, SimpleComponent};

#[derive(Debug)]
pub struct Actions {}

#[derive(Debug)]
pub enum ActionsOutput {
    PlayNow,
}

#[derive(Debug)]
pub enum ActionsInput {
    ClickedPlay,
}

#[relm4::component(pub)]
impl SimpleComponent for Actions {
    type Input = ActionsInput;
    type Output = ActionsOutput;
    type Init = ();

    view! {
        #[root]
        gtk::Box {
            gtk::Button {
                set_vexpand: false,
                set_valign: gtk::Align::Start,
                set_label: "Play now",
                connect_clicked => ActionsInput::ClickedPlay,
            },
        },
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Actions {};
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            ActionsInput::ClickedPlay => sender.output_sender().emit(ActionsOutput::PlayNow),
        }
    }
}
