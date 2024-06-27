mod library;
mod spot;
mod viewmodel;
mod views;

use std::sync::OnceLock;

use gtk4::gio::{self, ApplicationCommandLine, ApplicationFlags};
use gtk4::{prelude::*, Button};
use gtk4::{
    Application, ApplicationWindow, ListView, NoSelection, SignalListItemFactory,
};
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

    app.connect_command_line(|app, arg| {
        println!("Handling command-line...");
        build_ui(app, arg);
        0
    });

    app.run_with_args(&std::env::args().collect::<Vec<_>>());
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
        .height_request(40)
        .build();

    const playlist_id: &str = "4hcZPO5io5eAmd0iHW4j8a";
    let tracks_model = gio::ListStore::new::<LineItem>();

    let tracks_factory = SignalListItemFactory::new();
    tracks_factory.connect_setup(move |_, list_item| {
        item::new_item(list_item);
    });

    let tracks_view = ListView::new(
        Some(NoSelection::new(Some(tracks_model.clone()))),
        Some(tracks_factory),
    );

    let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    main_box.append(&tracks_view);
    main_box.append(&button);

    // Create a window and set the title
    let window = ApplicationWindow::builder()
        .application(app)
        .title("My GTK App")
        .child(&main_box)
        .width_request(200)
        .build();

    // Present window
    window.present();

    // Initialize Spotify library:
    let slib = Library::new(&username, &pwd, runtime());
    // TODO: connect to library's connected signal to load playlists?

    // Connect to "clicked" signal of `button`
    button.connect_clicked(move |_button| {
        println!("clicked!");
        let plist_id = SpotifyId::from_base62(playlist_id).unwrap();
        let (_plist_item, plist_store) = slib.load_playlist(plist_id);
        tracks_view.set_model(Some(&NoSelection::new(Some(plist_store))));
    });
}
