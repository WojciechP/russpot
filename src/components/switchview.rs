use gtk::prelude::*;
use relm4::{factory::FactoryVecDeque, prelude::*};

use crate::spotconn::SpotConn;

use super::denselist::{DenseList, DenseListInit, Source};

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
            set_vexpand: true,
                set_height_request: 400,
            #[local_ref]
            view_widgets -> gtk::Stack {
                set_vexpand: true,
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut views = FactoryVecDeque::<SwitchViewItem>::builder()
            .launch(gtk::Stack::default())
            .forward(sender.output_sender(), move |out| match out {});
        // TODO: remove hardcoded two views
        views.guard().push_back(SwitchViewItemInit {
            spot: SpotConn::new(), //TODO: accept from parent
            layout: SwitchViewItemLayout::SingleDenseList(Source::PlaylistUri(
                "spotify/playlist/1zIYbRl8ee7JIIYOPDrEJ6".to_string(),
            )),
        });
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
pub enum SwitchViewItemLayout {
    SingleDenseList(Source),
}

#[derive(Debug)]
pub struct SwitchViewItemInit {
    spot: SpotConn,
    layout: SwitchViewItemLayout,
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
    type ParentWidget = gtk::Stack;

    view! {
        #[root]
        gtk::Box::new(gtk::Orientation::Vertical, 0) {
            set_homogeneous: false,
            set_vexpand: true,
            gtk::Label {
                set_label: "Switch view item"
            },
            gtk::Button {
                gtk::Label {
                    set_label: "Switch view content"
                },
            },
            self.denselist.widget() {
            },
        }
    }

    fn init_model(init: Self::Init, index: &Self::Index, sender: FactorySender<Self>) -> Self {
        let SwitchViewItemLayout::SingleDenseList(ref source) = init.layout;
        let denselist = DenseList::builder()
            .launch(DenseListInit {
                spot: init.spot.clone(),
                source: source.clone(),
            })
            .forward(sender.output_sender(), |msg| match msg {});
        SwitchViewItem { init, denselist }
    }
}
