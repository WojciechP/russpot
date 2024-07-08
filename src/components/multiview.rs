//! Multiview displays multiple lists of items.
//! For example, an Artist page is a multiview of albums, songs, and related artists.

use crate::navigation::{NavCommand, NavOutput};
use gtk::prelude::*;
use log::{debug, warn};
use relm4::{factory::FactoryVecDeque, prelude::*};
use rspotify::model::{Offset, PlayContextId};

use super::denselist_factory;

#[derive(Debug)]
pub struct Model {
    sections: FactoryVecDeque<denselist_factory::Model>,
    cur_section: usize,
}

pub struct Init {
    pub sections: Vec<denselist_factory::Init>,
}

#[derive(Debug)]
pub enum In {
    Nav(NavCommand),
    ResetSections(Vec<denselist_factory::Init>),
    NextSection,
    PrevSection,
    #[doc(hidden)]
    ForwardNavOut(NavOutput),
}

#[derive(Debug)]
pub enum Out {
    Nav(NavOutput),
}

#[relm4::component(pub)]
impl SimpleComponent for Model {
    type Init = Init;
    type Input = In;
    type Output = Out;

    view! {
        #[root]
        gtk::Box::new(gtk::Orientation::Vertical, 0) {
            gtk::Label {
                set_label: "multiview",
            },

            #[local_ref]
            sections_widget -> gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
            }

        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut sections = FactoryVecDeque::<denselist_factory::Model>::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| {
                debug!("mutliview child message: {:?}", msg);
                match msg {
                    denselist_factory::Out::Nav(nav_out) => match nav_out {
                        NavOutput::EscapedUp => In::PrevSection,
                        NavOutput::EscapedLeft => In::PrevSection,
                        NavOutput::EscapedDown => In::NextSection,
                        NavOutput::EscapedRight => In::NextSection,
                        NavOutput::CursorIsNowAt(_) => In::ForwardNavOut(nav_out),
                    },
                }
            });
        for s in init.sections {
            sections.guard().push_back(s);
        }

        let sections_widget = sections.widget();

        let widgets = view_output!();

        let model = Model {
            sections,
            cur_section: 0,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            In::Nav(nav_cmd) => self
                .sections
                .send(self.cur_section, denselist_factory::In::Nav(nav_cmd)),
            In::ResetSections(sections) => {
                self.sections.guard().clear();
                for s in sections {
                    self.sections.guard().push_back(s);
                }
                self.cur_section = 0;
            }
            In::NextSection => self.change_section(1),
            In::PrevSection => self.change_section(-1),
            In::ForwardNavOut(nav_out) => sender.output_sender().emit(Out::Nav(nav_out)),
        }
    }
}

impl Model {
    fn change_section(&mut self, delta: i32) {
        let next = self.cur_section as i32 + delta;
        if next < 0 || (next as usize) >= self.sections.len() {
            warn!("Cannot change section to {:?}, out of bounds", next);
            return;
        }

        self.sections.send(
            self.cur_section,
            denselist_factory::In::Nav(NavCommand::ClearCursor),
        );
        self.cur_section = next as usize;
        self.sections.send(
            self.cur_section,
            denselist_factory::In::Nav(if delta > 0 {
                NavCommand::Down
            } else {
                NavCommand::Up
            }),
        );
    }

    pub fn descend(&self) -> Option<denselist_factory::Init> {
        self.sections
            .get(self.cur_section)
            .and_then(|dl| dl.descend())
    }

    pub fn play_context(&self) -> Option<(PlayContextId<'static>, Option<Offset>)> {
        self.sections
            .get(self.cur_section)
            .and_then(|dl| dl.play_context())
    }
}
