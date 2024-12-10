use crate::dom::{
    Attribute, AttributeName, DocumentChangeHandler, Element, ElementName, NodeData, NodeRef,
};
use crate::{attr, element};
use std::{sync::Arc, time::Duration};

use crate::dom::ChangeType;

use super::*;
mod error;
mod navigation;
mod streaming;
mod upload;

use phoenix_channels_client::ChannelStatus;
use pretty_assertions::assert_eq;

use tokio::sync::mpsc::*;

/// serializes two documents so the formatting matches before diffing.
macro_rules! assert_doc_eq {
    ($gold:expr, $test:expr) => {{
        use crate::dom::Document;
        let gold = Document::parse($gold).expect("Gold document failed to parse");
        let test = Document::parse($test).expect("Test document failed to parse");
        assert_eq!(gold.to_string(), test.to_string());
    }};
}

struct Inspector {
    tx: UnboundedSender<(ChangeType, NodeData)>,
    doc: crate::dom::ffi::Document,
}

/// An extremely simple change handler that reports diffs in order
/// over an unbounded channel
impl DocumentChangeHandler for Inspector {
    fn handle(
        &self,
        change_type: ChangeType,
        _node_ref: Arc<NodeRef>,
        node_data: NodeData,
        _parent: Option<Arc<NodeRef>>,
    ) {
        let doc = self.doc.inner();

        let _test = doc
            .try_lock()
            .expect("document was locked during change handler!");

        self.tx
            .send((change_type, node_data))
            .expect("Message Never Received.");
    }
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

// channels sending events lock the dom by modifying attributes
// and class lists, this should notify the documents developer
// provided change listener
#[tokio::test]
async fn locking_dom_event_propogation() {
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

    let doc = live_channel.document();
    let (tx, mut rx) = unbounded_channel();
    live_channel.set_event_handler(Box::new(Inspector { tx, doc }));

    let sender = live_channel
        .document()
        .inner()
        .lock()
        .expect("lock poison")
        .get_by_id("button")
        .expect("nothing by that name");

    live_channel
        .send_event_json(protocol::event::PhxEvent::PhxClick, None, &sender)
        .await
        .expect("click failed");

    let mut buf = vec![];
    rx.recv_many(&mut buf, 4096).await;
    assert_eq!(
        vec![
            (
                ChangeType::Change,
                element!(
                    "Button",
                    attr!("id" = "button"),
                    attr!("phx-click" = "inc_temperature"),
                )
            ),
            (
                ChangeType::Replace,
                NodeData::leaf("Current temperature: 70°F")
            ),
            (
                ChangeType::Change,
                element!(
                    "Button",
                    attr!("id" = "button"),
                    attr!("phx-click" = "inc_temperature"),
                    attr!("data-phx-ref-src" = "5"),
                    attr!("class" = "phx-click-loading")
                )
            )
        ],
        buf
    );
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
        Current temperature: 71°F
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

#[test]
fn test_attr_macro() {
    let attr1 = attr!("class");
    assert_eq!(attr1.name.name, "class");
    assert_eq!(attr1.name.namespace, None);
    assert_eq!(attr1.value, None);

    let attr2 = attr!("class" = "container");
    assert_eq!(attr2.name.name, "class");
    assert_eq!(attr2.value, Some("container".to_string()));

    let attr3 = attr!("svg":"width");
    assert_eq!(attr3.name.namespace, Some("svg".to_string()));
    assert_eq!(attr3.name.name, "width");
    assert_eq!(attr3.value, None);

    let attr4 = attr!("svg":"width" = "100");
    assert_eq!(attr4.name.namespace, Some("svg".to_string()));
    assert_eq!(attr4.name.name, "width");
    assert_eq!(attr4.value, Some("100".to_string()));
}

#[test]
fn test_element_macro() {
    let input = element!("input", attr!("type" = "text"), attr!("id" = "username"));

    match input {
        NodeData::NodeElement { element } => {
            assert_eq!(element.name.name, "input");
            assert_eq!(element.attributes.len(), 2);

            let type_attr = &element.attributes[0];
            assert_eq!(type_attr.name.name, "type");
            assert_eq!(type_attr.value, Some("text".to_string()));

            let id_attr = &element.attributes[1];
            assert_eq!(id_attr.name.name, "id");
            assert_eq!(id_attr.value, Some("username".to_string()));
        }
        _ => panic!("Expected NodeElement"),
    }
}
