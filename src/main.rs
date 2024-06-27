mod library;
mod spot;
mod views;

use std::sync::OnceLock;

use gtk::gdk::Display;
use gtk::gio::{self, ActionEntry, ApplicationCommandLine, ApplicationFlags, SimpleActionGroup};
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::prelude::*;
use gtk::{prelude::*, Button, CssProvider, ListScrollFlags, PolicyType, ScrolledWindow};
use gtk::{Application, ApplicationWindow, ListView, NoSelection, SignalListItemFactory};
use librespot::core::spotify_id::SpotifyId;

use tokio::runtime::Runtime;

use crate::library::{Library, LineItem};
use crate::views::item;

const APP_ID: &str = "org.gtk_rs.Russpot";

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."))
}

fn main() {
    // Create a new application
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_startup(|_| load_css());
    app.connect_command_line(|app, arg| {
        println!("Handling command-line...");
        build_ui(app, arg);
        0
    });

    app.run_with_args(&std::env::args().collect::<Vec<_>>());
}
fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_data(include_str!("style.css"));

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(app: &Application, args: &ApplicationCommandLine) {
    println!("building ui!");
    let argv = args.arguments();
    let triplet = (argv[1].to_str(), argv[2].to_str(), argv[3].to_str());
    let (Some(username), Some(pwd), Some(track)) = triplet else {
        println!("Usage: russpot USERNAME PASSWORD TRACKID");
        return;
    };
    let username: String = username.to_string();
    let pwd: String = pwd.to_string();
    let _track: String = track.to_string();

    // Create a button with label and margins
    let button = Button::builder()
        .label("Play!")
        .width_request(100)
        .hexpand(true)
        .build();

    const playlist_id: &str = "6akHLZRrHoVKtXHgNZLfgj";

    let tracks_model = gio::ListStore::new::<LineItem>();

    let tracks_factory = SignalListItemFactory::new();
    tracks_factory.connect_setup(move |_, list_item| {
        item::new_item(list_item.downcast_ref().expect("must be a ListItem"));
    });

    let tracks_view = ListView::new(
        Some(NoSelection::new(Some(tracks_model.clone()))),
        Some(tracks_factory),
    );

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never) // Disable horizontal scrolling
        .min_content_width(360)
        .vexpand(true)
        .height_request(500)
        .child(&tracks_view)
        .build();

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    main_box.append(&scrolled_window);
    main_box.append(&button);

    // Create a window and set the title
    let window = ApplicationWindow::builder()
        .application(app)
        .title("My GTK App")
        .child(&main_box)
        .width_request(200)
        .build();

    let action_down = ActionEntry::builder("down")
        .activate(clone!(@weak tracks_view => move |_, _, _| {
            tracks_view.scroll_to(2, ListScrollFlags::FOCUS, None);
        }))
        .build();

    let nav_actions = SimpleActionGroup::new();
    nav_actions.add_action_entries([action_down]);
    window.insert_action_group("nav", Some(&nav_actions));

    app.set_accels_for_action("nav.down", &["J"]);

    // Present window
    window.present();

    // Initialize Spotify library:
    let slib = Library::new(&username, &pwd, runtime());
    // TODO: connect to library's connected signal to load playlists?

    // Connect to "clicked" signal of `button`
    let tv = tracks_view.clone();
    button.connect_clicked(move |_button| {
        println!("clicked!");
        let plist_id = SpotifyId::from_base62(playlist_id).unwrap();
        let (_plist_item, plist_store) = slib.load_playlist(plist_id);
        tv.set_model(Some(&NoSelection::new(Some(plist_store))));
    });
}
