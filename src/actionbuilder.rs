use std::{cell::RefCell, collections::HashMap};

use gtk::prelude::*;

use relm4::Sender;

/// A helper for building keyboard-accelerated stateless actions.
/// The relm4 way of creating actions is quite baroque, with creating
/// a new type for each action. Let's skip that, use gtk-rs directly.
/// For accelerators without modifier keys, use AccelManager instead.
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

/// We would like to use single-letter accelerators for vim-style navigation,
/// but that conflicts with text input entries for search. Therefore
/// we have to register and un-register the accelerators depending on whether
/// a text entry is focused. I wish this was easier, but see
/// [https://stackoverflow.com/questions/22782726/how-to-disable-accelerators-when-typing-text-in-gtk].
///
/// Usage:
/// ```
/// let am = AccelManager::new(window, "somegroup")
/// am.register_emit("someaction", &["J"], sender, msg);
/// // more register_emit calls here
/// am.connect();
/// ```
///     
pub struct AccelManager {
    accels_for_action: HashMap<String, Vec<String>>,
    builder: ActionBuilder,
    window: gtk::Window,
    is_text_focused: bool,
}

impl AccelManager {
    pub fn new(window: &gtk::Window, group_name: &str) -> Self {
        AccelManager {
            accels_for_action: HashMap::new(),
            builder: ActionBuilder::new(window.clone(), group_name),
            window: window.clone(),
            is_text_focused: false,
        }
    }
    pub fn connect(self) {
        let window = self.window.clone();
        let w2 = window.clone();
        // On every focus change, we need to mutate the is_text_focused field.
        // We cannot pass a mutable borrow to a closure, though, so let's
        // use a RefCell instead.
        let mutself = RefCell::new(self);
        window.connect_focus_widget_notify(move |_| {
            mutself
                .borrow_mut()
                .on_focus_change(GtkWindowExt::focus(&w2))
        });
    }
    fn on_focus_change(&mut self, target: Option<gtk::Widget>) {
        let is_text = target.is_some_and(|t| t.downcast::<gtk::Text>().is_ok());
        match (self.is_text_focused, is_text) {
            (false, true) => {
                self.remove_accels();
                self.is_text_focused = true;
            }
            (true, false) => {
                self.register_accels();
                self.is_text_focused = false;
            }
            _ => {}
        }
    }

    fn register_accels(&self) {
        for (act, accels) in &self.accels_for_action {
            let accels: Vec<&str> = accels.iter().map(|s| &**s).collect();
            relm4::main_application().set_accels_for_action(act, &accels[..]);
        }
    }
    fn remove_accels(&self) {
        for act in self.accels_for_action.keys() {
            relm4::main_application().set_accels_for_action(act, &[]);
        }
    }

    pub fn register_emit<A: Clone + 'static>(
        &mut self,
        name: &str,
        accels: &[&str],
        sender: &Sender<A>,
        msg: A,
    ) {
        self.builder.add_emit(name, accels, sender, msg);
        let accels: Vec<String> = accels.to_vec().iter().map(|&s| s.into()).collect();
        self.accels_for_action
            .insert(format!("{}.{}", self.builder.name, name), accels);
    }
}
