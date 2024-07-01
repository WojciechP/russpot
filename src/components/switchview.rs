use relm4::{factory::FactoryVecDeque, prelude::*};

use crate::spotconn::SpotConn;

use super::denselist::{DenseList, DenseListInit};

pub struct SwitchView {
    views: FactoryVecDeque<SwitchViewItem>,
}

#[derive(Debug)]
pub struct SwitchViewInit {}

#[derive(Debug)]
pub enum SwitchViewInput {
    CursorMove(i32),
}

#[derive(Debug)]
pub enum SwitchViewOutput {}

#[derive(Debug)]
pub enum SwitchViewCommandOutput {}

#[relm4::component(pub)]
impl relm4::Component for SwitchView {
    type Init = SwitchViewInit;
    type Input = SwitchViewInput;
    type Output = SwitchViewOutput;
    type CommandOutput = SwitchViewCommandOutput;

    view! {
        #[root]
        gtk::Box::new(gtk::Orientation::Horizontal, 0) {
            #[local_ref]
            view_widgets -> gtk::Box {}
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut views = FactoryVecDeque::<SwitchViewItem>::builder()
            .launch(gtk::Box::default())
            .forward(sender.output_sender(), move |out| match out {});
        views.guard().push_back(SwitchViewItemInit::UserPlaylists);
        views.guard().push_back(SwitchViewItemInit::UserPlaylists);
        let model = SwitchView { views };
        let view_widgets = model.views.widget();
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}

#[derive(Debug)]
pub struct SwitchViewItem {
    init: SwitchViewItemInit,
    denselist: Controller<DenseList>, // TODO: this should be an enum for search/home pages too
}

#[derive(Debug)]
pub enum SwitchViewItemInit {
    UserPlaylists,
}

#[derive(Debug)]
pub enum SwitchViewItemInput {}

#[derive(Debug)]
pub enum SwitchViewItemOutput {}

#[relm4::factory(pub)]
impl FactoryComponent for SwitchViewItem {
    type Init = SwitchViewItemInit;
    type Input = SwitchViewItemInput;
    type Output = SwitchViewItemOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box; // TODO: gtk::Stack?

    view! {
        #[root]
        gtk::Box::new(gtk::Orientation::Vertical, 0) {
            gtk::Label {
                set_label: "Switch view item"
            },
            gtk::Button {
                gtk::Label {
                    set_label: "Switch view content"
                },
            },
            self.denselist.widget() {}
        }
    }

    fn init_model(init: Self::Init, index: &Self::Index, sender: FactorySender<Self>) -> Self {
        let var_name = DenseListInit {
            spot: SpotConn::new(),
        };
        let denselist = DenseList::builder()
            .launch(var_name)
            .forward(sender.output_sender(), |msg| match msg {});
        SwitchViewItem { init, denselist }
    }
}
