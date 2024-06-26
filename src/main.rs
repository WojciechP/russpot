mod viewmodel;

use std::sync::OnceLock;

use glib::clone;
use gtk4::builders::SearchEntryBuilder;
use gtk4::gio::{self, ApplicationCommandLine, ApplicationFlags};
use gtk4::glib::ffi::G_PRIORITY_DEFAULT;
use gtk4::glib::MainContext;
use gtk4::{
    glib, Application, ApplicationWindow, Label, ListItem, ListView, NoSelection,
    SignalListItemFactory,
};
use gtk4::{prelude::*, Button};
use librespot::core::authentication::Credentials;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::core::spotify_id::SpotifyId;
use librespot::metadata::{Metadata, Playlist, Track};
use librespot::playback::audio_backend;
use librespot::playback::config::{AudioFormat, PlayerConfig};
use librespot::playback::mixer::NoOpVolume;
use librespot::playback::player::Player;
use tokio::runtime::Runtime;

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

    let tracks: Vec<SpotifyItemObject> = vec!["6PUPRb62MyZo6MRlEQZKFq", "5Y8IMaCAPl996kjC4uo9Tx"]
        .into_iter()
        .map(SpotifyItemObject::new_track)
        .collect();
    let tracks_model = gio::ListStore::new::<SpotifyItemObject>();
    tracks_model.extend_from_slice(&tracks);

    let tracks_factory = SignalListItemFactory::new();
    tracks_factory.connect_setup(move |_, list_item| {
        let label = Label::new(None);
        list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .set_child(Some(&label));
    });
    tracks_factory.connect_bind(move |_, list_item| {
        let spotify_object = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .item()
            .and_downcast::<SpotifyItemObject>()
            .expect("The item has to be SpotifyItemObject");
        // Get `Label` from `ListItem`
        let label = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .child()
            .and_downcast::<Label>()
            .expect("The child has to be a `Label`.");
        label.set_label(&spotify_object.trackid().to_string());
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

    // Start receiving tracks from the tokio async loop:
    let (sender, receiver) = async_channel::bounded::<Track>(1);

    // Connect to "clicked" signal of `button`
    button.connect_clicked(move |_button| {
        println!("clicked!");
        println!("args: {} {} {}", username, pwd, track);
        runtime().spawn(
            clone!(@strong username, @strong pwd, @strong track, @strong sender => async move{
                play_track(&username, &pwd, &track, sender ).await;
            }),
        );
    });
    // Spawn a future for receiving the tracks:
    glib::spawn_future_local(async move {
        while let Ok(response) = receiver.recv().await {
            tracks_model.append(&SpotifyItemObject::new_track(&response.name));
        }
    });
}

async fn play_track(user: &str, pwd: &str, track: &str, sender: async_channel::Sender<Track>) {
    let session_config = SessionConfig::default();
    let player_config = PlayerConfig::default();
    let audio_format = AudioFormat::default();

    let credentials = Credentials::with_password(user, pwd);

    let track = SpotifyId::from_base62(track).unwrap();

    let backend = audio_backend::find(None).unwrap();

    println!("Connecting ..");
    let (session, _) = Session::connect(session_config, credentials, None, false)
        .await
        .unwrap();

    let (mut player, _) = Player::new(
        player_config,
        session.clone(),
        Box::new(NoOpVolume),
        move || backend(None, audio_format),
    );

    player.load(track, true, 0);

    println!("Playing started, fetching playlist...");

    let playlist_uri = SpotifyId::from_uri("spotify:playlist:0q9suO2gxC523Vx2HZupcG").unwrap();
    let plist = Playlist::get(&session, playlist_uri).await.unwrap();
    println!("{:?}", plist);
    for track_id in plist.tracks {
        let plist_track = Track::get(&session, track_id).await.unwrap();
        println!("track: {} ", plist_track.name);
        sender.send(plist_track).await.expect("channel closed?");
    }

    player.await_end_of_track().await;

    println!("Done");
}
