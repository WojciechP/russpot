mod library;
mod spot;
mod viewmodel;
mod views;

use std::sync::OnceLock;

use glib::clone;
use gtk4::gio::{self, ApplicationCommandLine, ApplicationFlags};
use gtk4::{
    glib, Application, ApplicationWindow, Label, ListItem, ListView, NoSelection,
    SignalListItemFactory, Widget,
};
use gtk4::{prelude::*, Button};
use librespot::core::spotify_id::SpotifyId;
use librespot::metadata::Track;
use tokio::runtime::Runtime;

use crate::library::{Library, LineItem};
use crate::viewmodel::SpotifyItemObject;

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
    let track: String = track.to_string();

    // Create a button with label and margins
    let button = Button::builder()
        .label("Play!")
        .width_request(100)
        .height_request(40)
        .build();

    let tracks_model = gio::ListStore::new::<LineItem>();

    let tracks_factory = SignalListItemFactory::new();
    tracks_factory.connect_setup(move |_, list_item| {
        let label = Label::new(None);
        list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .set_child(Some(&label));

        // Bind the LineItem properties to the widget:
        list_item
            .property_expression("item")
            .chain_property::<LineItem>("name")
            .bind(&label, "label", Widget::NONE);
    });
    /*
    tracks_factory.connect_bind(move |_, list_item| {
        let line_item = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .item()
            .and_downcast::<LineItem>()
            .expect("The item has to be LineItem");
        // Get `Label` from `ListItem`
        let label = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .child()
            .and_downcast::<Label>()
            .expect("The child has to be a `Label`.");
        label.set_label(&line_item.name().to_string());
    });
    */

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
    let (owner, slib) = Library::new(&username, &pwd, runtime());
    // TODO: connect to library's connected signal to load playlists?

    // Connect to "clicked" signal of `button`
    button.connect_clicked(move |_button| {
        let slib = slib.clone();
        println!("clicked!");
        let tids: Vec<&str> = vec!["6PUPRb62MyZo6MRlEQZKFq", "5Y8IMaCAPl996kjC4uo9Tx"];
        for tid in tids {
            println!("loading {}", tid);
            let id = SpotifyId::from_base62(tid).unwrap();
            let track = owner.load_track(id);
            tracks_model.append(&track);
        }
    });
}
