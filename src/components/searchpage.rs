use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub struct SearchPage {
    searchbox: gtk::Entry,
}

#[derive(Debug)]
pub enum SearchPageInput {
    FocusSearchbox,
}

#[relm4::component(pub)]
impl Component for SearchPage {
    type Init = ();
    type Input = SearchPageInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
    gtk::Box::new(gtk::Orientation::Vertical, 0) {
        set_hexpand: true,
        #[name="searchbox"]
        gtk::Entry {
        },

        gtk::Button {
            gtk::Label {
                set_label: "Search page",
                }
            },
        },
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = SearchPage {
            searchbox: widgets.searchbox.clone(),
        };
        sender.input_sender().emit(SearchPageInput::FocusSearchbox);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            SearchPageInput::FocusSearchbox => self.searchbox.grab_focus(),
        };
    }
}
