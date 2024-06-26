use std::sync::OnceLock;

use glib::clone;
use gtk4::gio::{ApplicationCommandLine, ApplicationFlags};
use gtk4::{glib, Application, ApplicationWindow};
use gtk4::{prelude::*, Button};
use librespot::core::authentication::Credentials;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::core::spotify_id::SpotifyId;
use librespot::playback::audio_backend;
use librespot::playback::config::{AudioFormat, PlayerConfig};
use librespot::playback::mixer::NoOpVolume;
use librespot::playback::player::Player;
use tokio::runtime::Runtime;

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
    // Create a window and set the title
    let window = ApplicationWindow::builder()
        .application(app)
        .title("My GTK App")
        .child(&button)
        .width_request(200)
        .build();

    // Present window
    window.present();

    // Connect to "clicked" signal of `button`
    button.connect_clicked(move |button| {
        println!("clicked!");
        println!("args: {} {} {}", username, pwd, track);
        runtime().spawn(
            clone!(@strong username, @strong pwd, @strong track => async move{
                play_track(&username, &pwd, &track).await;
            }),
        );
    });
}

async fn play_track(user: &str, pwd: &str, track: &str) {
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

    let (mut player, _) = Player::new(player_config, session, Box::new(NoOpVolume), move || {
        backend(None, audio_format)
    });

    player.load(track, true, 0);

    println!("Playing...");

    player.await_end_of_track().await;

    println!("Done");
}
