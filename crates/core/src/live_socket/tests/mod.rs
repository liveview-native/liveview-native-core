use std::time::Duration;

use super::*;
mod error;
mod navigation;
mod streaming;
mod upload;

use phoenix_channels_client::ChannelStatus;
use pretty_assertions::assert_eq;

/// serializes two documents so the formatting matches before diffing.
macro_rules! assert_doc_eq {
    ($gold:expr, $test:expr) => {{
        use crate::dom::Document;
        let gold = Document::parse($gold).expect("Gold document failed to parse");
        let test = Document::parse($test).expect("Test document failed to parse");
        assert_eq!(gold.to_string(), test.to_string());
    }};
}

pub(crate) use assert_doc_eq;

#[cfg(target_os = "android")]
const HOST: &str = "10.0.2.2:4001";

#[cfg(not(target_os = "android"))]
const HOST: &str = "127.0.0.1:4001";

#[tokio::test]
async fn join_live_view() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/hello");
    let live_socket = LiveSocket::new(url.to_string(), "swiftui".into(), Default::default())
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
    let rendered = format!("{}", join_doc);
    let expected = r#"<Group id="flash-group" />
<VStack>
    <Text>
        Hello SwiftUI!
    </Text>
</VStack>"#;
    assert_doc_eq!(expected, rendered);

    let _live_channel = live_socket
        .join_livereload_channel()
        .await
        .expect("Failed to join channel");
}

#[tokio::test]
async fn click_test() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/thermostat");

    let live_socket = LiveSocket::new(url.to_string(), "swiftui".into(), Default::default())
        .await
        .expect("Failed to get liveview socket");

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join channel");

    let join_doc = live_channel
        .join_document()
        .expect("Failed to render join payload");

    let expected = r#"<Group id="flash-group" />
<VStack>
    <Text>
        Current temperature: 70°F
    </Text>
    <Button id="button" phx-click="inc_temperature">
        +
    </Button>
</VStack>"#;

    assert_doc_eq!(expected, join_doc.to_string());

    let sender = join_doc.get_by_id("button").expect("nothing by that name");

    live_channel
        .send_event_json(protocol::event::PhxEvent::PhxClick, None, &sender)
        .await
        .expect("click failed");

    let expected = r#"<Group id="flash-group" />
<VStack>
    <Text>
        Current temperature: 70°F
    </Text>
    <Button id="button" phx-click="inc_temperature">
        +
    </Button>
</VStack>"#;

    assert_doc_eq!(expected, live_channel.document().to_string());
}

#[tokio::test]
async fn channels_keep_listening_for_diffs_on_reconnect() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/hello");

    let live_socket = LiveSocket::new(url.to_string(), "swiftui".into(), Default::default())
        .await
        .expect("Failed to get liveview socket");

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join channel");

    let live_channel = std::sync::Arc::new(live_channel);

    let chan_clone = live_channel.clone();

    let handle = tokio::spawn(async move {
        chan_clone
            .merge_diffs()
            .await
            .expect("Failed to merge diffs");
    });

    live_socket
        .socket()
        .disconnect()
        .await
        .expect("shutdown error");

    assert_eq!(
        live_channel.channel().status(),
        ChannelStatus::WaitingForSocketToConnect
    );

    assert!(!handle.is_finished());

    // reconnect
    live_socket
        .socket()
        .connect(Duration::from_millis(1000))
        .await
        .expect("shutdown error");

    assert_eq!(
        live_channel.channel().status(),
        ChannelStatus::WaitingToJoin
    );

    live_channel.rejoin().await.expect("Could not rejoin");

    // We are listening to events again
    assert_eq!(live_channel.channel().status(), ChannelStatus::Joined);
    // The merge diff event has not exited.
    assert!(!handle.is_finished());
}

// Validate that shutdown has side effects.
#[tokio::test]
async fn channels_drop_on_shutdown() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/hello");

    let live_socket = LiveSocket::new(url.to_string(), "swiftui".into(), Default::default())
        .await
        .expect("Failed to get liveview socket");

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join channel");

    let chan_clone = live_channel.channel().clone();
    let handle = tokio::spawn(async move {
        live_channel
            .merge_diffs()
            .await
            .expect("Failed to merge diffs");
    });

    live_socket
        .socket()
        .shutdown()
        .await
        .expect("shutdown error");

    assert!(handle.is_finished());
    assert_eq!(chan_clone.status(), ChannelStatus::ShutDown);
}

#[tokio::test]
async fn redirect() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/hello");
    let live_socket = LiveSocket::new(url.to_string(), "swiftui".into(), Default::default())
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
