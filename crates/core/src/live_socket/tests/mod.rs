use std::{collections::HashMap, sync::Arc, time::Duration};

use super::*;
use crate::dom::{
    ChangeType, ControlFlow, DocumentChangeHandler, LiveChannelStatus, NodeData, NodeRef,
};
mod error;
mod navigation;
mod streaming;
mod upload;

#[cfg(target_os = "android")]
const HOST: &str = "10.0.2.2:4001";

#[cfg(not(target_os = "android"))]
const HOST: &str = "127.0.0.1:4001";

use phoenix_channels_client::ChannelStatus;
use pretty_assertions::assert_eq;

macro_rules! assert_doc_eq {
    ($gold:expr, $test:expr) => {{
        use crate::dom::Document;
        let gold = Document::parse($gold).expect("Gold document failed to parse");
        let test = Document::parse($test).expect("Test document failed to parse");
        assert_eq!(gold.to_string(), test.to_string());
    }};
}

pub(crate) use assert_doc_eq;
use socket::{brian_get_dead_render, ConnectOpts};
use tokio::sync::mpsc::*;

struct Inspector {
    tx: UnboundedSender<(ChangeType, NodeData)>,
    doc: crate::dom::ffi::Document,
}

impl Inspector {
    pub fn new(
        doc: crate::dom::ffi::Document,
    ) -> (Self, UnboundedReceiver<(ChangeType, NodeData)>) {
        let (tx, rx) = unbounded_channel();
        let out = Self { tx, doc };
        (out, rx)
    }
}

/// An extremely simple change handler that reports diffs in order
/// over an unbounded channel
impl DocumentChangeHandler for Inspector {
    fn handle_document_change(
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

    fn handle_channel_status(&self, channel_status: LiveChannelStatus) -> ControlFlow {
        match channel_status {
            LiveChannelStatus::Left | LiveChannelStatus::ShutDown => ControlFlow::ExitOk,
            _ => ControlFlow::ContinueListening,
        }
    }
}

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
async fn get_dead_render() {
    let connect_url = "http://127.0.0.1:4001/hello".into();
    let mut opts = ConnectOpts::default();
    opts.headers = HashMap::from([("Content-Type".into(), "text/swiftui".into())]).into();
    let _ = brian_get_dead_render(connect_url, opts)
        .await
        .expect("Bad result");
}

#[tokio::test]
async fn join_redirect() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/redirect_from");

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

    let expected = r#"
<Group id="flash-group" />
<VStack>
    <Text>
        Redirected!
    </Text>
</VStack>"#;
    assert_doc_eq!(expected, join_doc.to_string());

    let _live_channel = live_socket
        .join_livereload_channel()
        .await
        .expect("Failed to join channel");
}

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
    let expected = r#"
<Group id="flash-group" />
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
async fn channel_redirect() {
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
