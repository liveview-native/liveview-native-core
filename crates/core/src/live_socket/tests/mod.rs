use std::time::Duration;

use super::*;
mod streaming;
mod support;
mod upload;

#[cfg(target_os = "android")]
const HOST: &str = "10.0.2.2:4001";

#[cfg(not(target_os = "android"))]
const HOST: &str = "127.0.0.1:4001";

const TIME_OUT: Duration = std::time::Duration::from_secs(10);

use phoenix_channels_client::Event;
use pretty_assertions::assert_eq;
use support::{json_payload, FixturePlayback};

#[derive(Debug)]
struct AnyPanic;

// A fairly hacky way to provide stack traces in tests which
// return an error.
impl<E: std::fmt::Display> From<E> for AnyPanic {
    #[track_caller]
    fn from(err: E) -> Self {
        panic!("{}", err);
    }
}

// records a session with the test server, writing the returned schema to disk
// for later verification.
// PROTIP: set the environment variable RECORD_ALL_FIXTURES="true" to set every fixture to record mode.
#[tokio::test]
async fn thermostat_playback() -> Result<(), AnyPanic> {
    // args: fixture file directory, format (swiftui | jetpack | html), test server url
    // set the macro to record! or playback! depending on what stage of testing you are in
    let mut playback =
        FixturePlayback::playback("fixtures/test_1.fixture", "swiftui", "thermostat").await;

    // validate initial state
    playback.validate_document()?;

    // click the increment temperature button
    let payload = json_payload!({"type": "click", "event": "inc_temperature", "value": {}});
    let user_event = Event::from_string("event".to_owned());

    playback.send_message(user_event, payload).await?;
    playback.validate_document()?;

    Ok(())
}

#[tokio::test]
async fn android_show_dialog_playback() -> Result<(), AnyPanic> {
    let mut playback =
        FixturePlayback::playback("fixtures/test_2.fixture", "jetpack", "android_bug").await;

    playback.validate_document()?;

    // Click show dialog
    let user_event = Event::from_string("event".to_owned());
    let payload = json_payload!({"type": "click", "event": "showDialog", "value": {}});

    //playback.start_logging_patch_events();
    playback.send_message(user_event, payload).await?;
    playback.validate_document()?;
    //playback.stop_logging_patch_events();

    // Click close dialog
    let user_event = Event::from_string("event".to_owned());
    let payload = json_payload!({"type": "click", "event": "hideDialog", "value": {}});

    playback.send_message(user_event, payload).await?;
    playback.validate_document()?;

    Ok(())
}

#[tokio::test]
async fn join_live_view() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/hello");
    let live_socket = LiveSocket::new(url.to_string(), TIME_OUT, "swiftui".into())
        .await
        .expect("Failed to get liveview socket");

    let style_urls = live_socket.style_urls();
    let expected_style_urls = vec!["/assets/app.swiftui.styles".to_string()];
    assert_eq!(style_urls, expected_style_urls);

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join channel");
    let join_doc = live_channel
        .join_document()
        .expect("Failed to render join payload");
    let rendered = format!("{}", join_doc.to_string());
    let expected = r#"<Group id="flash-group" />
<VStack>
    <Text>
        Hello SwiftUI!
    </Text>
</VStack>"#;
    assert_eq!(expected, rendered);
    let _phx_input_id = live_channel.get_phx_ref_from_upload_join_payload();

    let _live_channel = live_socket
        .join_livereload_channel()
        .await
        .expect("Failed to join channel");
}

#[tokio::test]
async fn redirect() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/hello");
    let live_socket = LiveSocket::new(url.to_string(), TIME_OUT, "swiftui".into())
        .await
        .expect("Failed to get liveview socket");

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join channel");

    //live_channel.channel().shutdown().await.expect("Failed to leave live channel");
    //
    // Leave should be: ["4","13","lv:phx-F_azBZxXhBqPjAAm","phx_leave",{}]
    live_channel
        .channel()
        .leave()
        .await
        .expect("Failed to leave live channel");
    let redirect = format!("http://{HOST}/upload");
    let _live_channel = live_socket
        .join_liveview_channel(None, Some(redirect))
        .await
        .expect("Failed to join channel");
}
