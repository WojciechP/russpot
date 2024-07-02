use gtk::prelude::*;
use relm4::Sender;

/// A helper for building keyboard-accelerated stateless actions.
/// The relm4 way of creating actions is quite baroque, with creating
/// a new type for each action. Let's skip that, use gtk-rs directly.
pub struct ActionBuilder {
    name: String,
    group: gtk::gio::SimpleActionGroup,
}

impl ActionBuilder {
    pub fn new(widget: impl IsA<gtk::Widget>, action_group_name: &str) -> Self {
        let b = ActionBuilder {
            name: action_group_name.to_owned(),
            group: gtk::gio::SimpleActionGroup::new(),
        };
        widget.insert_action_group(action_group_name, Some(&b.group));
        b
    }

    pub fn add<F>(&self, name: &str, accels: &[&str], f: F)
    where
        F: Fn() + 'static,
    {
        let sa = gtk::gio::ActionEntry::<gtk::gio::SimpleActionGroup>::builder(name)
            .activate(move |_, _, _| {
                f();
            })
            .build();
        self.group.add_action_entries([sa]);
        let compound_name = format!("{}.{}", self.name, name);
        relm4::main_application().set_accels_for_action(&compound_name, accels);
    }

    /// Registers an action that emits a single message to a given relm4 sender.
    pub fn add_emit<A: Clone + 'static>(
        &self,
        name: &str,
        accels: &[&str],
        sender: &Sender<A>,
        msg: A,
    ) {
        let sender = sender.clone();
        self.add(name, accels, move || {
            sender.emit(msg.clone());
        });
    }
}
