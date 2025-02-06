use std::sync::{Arc, Mutex};

use phoenix_channels_client::{Event, EventPayload, Payload, Socket, SocketStatus, JSON};
use pretty_assertions::assert_eq;
use serde_json::json;

use super::{json_payload, HOST};
use crate::{
    client::{
        HandlerResponse, LiveChannelStatus, LiveViewClientConfiguration, NavEvent, NavEventHandler,
        NavEventType, NavHistoryEntry, NetworkEventHandler, Platform,
    },
    dom::{self},
    live_socket::LiveChannel,
    LiveViewClient,
};

#[derive(Debug, Clone, PartialEq)]
pub enum MockMessage {
    Navigation(NavEvent),
    NetworkEvent(Event, Payload),
    ChannelStatus(LiveChannelStatus),
    SocketStatus(SocketStatus),
    ViewReload { socket_is_new: bool },
}

#[macro_export]
macro_rules! assert_any {
    ($store:expr, $predicate:expr) => {
        {
            let messages = $store.messages.lock().unwrap();
            assert!(
                messages.iter().any($predicate),
                "\nAssertion failed at {}:{}:{}\nMessage not found matching predicate.\nMessages:\n{:#?}",
                file!(),
                line!(),
                column!(),
                *messages
            );
        }
    };
}

#[derive(Default)]
pub struct MockMessageStore {
    messages: Arc<Mutex<Vec<MockMessage>>>,
}

impl MockMessageStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(&self, msg: MockMessage) {
        let mut messages = self.messages.lock().unwrap();
        messages.push(msg);
    }

    #[allow(unused)]
    pub fn dump_and_panic(&self) {
        let messages = self.messages.lock().unwrap();
        panic!(
            "Explicitly panic in test - dumping all messages \n: {:#?}",
            *messages
        );
    }

    pub fn assert_contains(&self, expected: MockMessage) {
        let messages = self.messages.lock().unwrap();
        assert!(
            messages.contains(&expected),
            "\nExpected message not found.\nExpected:\n{:#?}\n\nActual messages:\n{:#?}",
            expected,
            *messages
        );
    }

    pub fn clear(&self) {
        let mut messages = self.messages.lock().unwrap();
        messages.clear();
    }
}

pub struct MockNavEventHandler {
    message_store: Arc<MockMessageStore>,
}

impl MockNavEventHandler {
    pub fn new(message_store: Arc<MockMessageStore>) -> Self {
        Self { message_store }
    }
}

impl NavEventHandler for MockNavEventHandler {
    fn handle_event(&self, event: NavEvent) -> HandlerResponse {
        self.message_store
            .add_message(MockMessage::Navigation(event));
        HandlerResponse::Default
    }
}

// Mock implementation of NetworkEventHandler
#[cfg(feature = "liveview-channels")]
pub struct MockNetworkEventHandler {
    message_store: Arc<MockMessageStore>,
}

#[cfg(feature = "liveview-channels")]
impl MockNetworkEventHandler {
    pub fn new(message_store: Arc<MockMessageStore>) -> Self {
        Self { message_store }
    }
}

#[cfg(feature = "liveview-channels")]
impl NetworkEventHandler for MockNetworkEventHandler {
    fn handle_event(&self, event: EventPayload) {
        self.message_store
            .add_message(MockMessage::NetworkEvent(event.event, event.payload));
    }

    fn handle_channel_status_change(&self, event: LiveChannelStatus) {
        self.message_store
            .add_message(MockMessage::ChannelStatus(event));
    }

    fn handle_socket_status_change(&self, event: SocketStatus) {
        self.message_store
            .add_message(MockMessage::SocketStatus(event));
    }

    fn handle_view_reloaded(
        &self,
        _new_document: Arc<dom::ffi::Document>,
        _new_channel: Arc<LiveChannel>,
        _current_socket: Arc<Socket>,
        socket_is_new: bool,
    ) {
        self.message_store
            .add_message(MockMessage::ViewReload { socket_is_new });
    }
}

#[tokio::test]
async fn test_navigation_handler() {
    let store = Arc::new(MockMessageStore::new());
    let nav_handler = Arc::new(MockNavEventHandler::new(store.clone()));
    let net_handler = Arc::new(MockNetworkEventHandler::new(store.clone()));

    let url = format!("http://{HOST}/nav/first_page");
    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;
    config.navigation_handler = Some(nav_handler);
    config.network_event_handler = Some(net_handler);

    let client = LiveViewClient::initial_connect(config, url.clone(), Default::default())
        .await
        .expect("Failed to create client");

    let next_url = format!("http://{HOST}/nav/second_page");

    client
        .navigate(next_url.clone(), Default::default())
        .await
        .expect("Failed to navigate");

    store.assert_contains(MockMessage::Navigation(NavEvent {
        event: NavEventType::Push,
        same_document: false,
        from: Some(NavHistoryEntry::new(url, 1, None)),
        to: NavHistoryEntry::new(next_url, 2, None),
        info: None,
    }));
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    assert_any!(store, |m| { matches!(m, MockMessage::ViewReload { .. }) });
}

#[tokio::test]
async fn test_redirect_internals() {
    let store = Arc::new(MockMessageStore::new());
    let nav_handler = Arc::new(MockNavEventHandler::new(store.clone()));
    let net_handler = Arc::new(MockNetworkEventHandler::new(store.clone()));

    let url = format!("http://{HOST}/hello");
    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;
    config.navigation_handler = Some(nav_handler);
    config.network_event_handler = Some(net_handler);

    let client = LiveViewClient::initial_connect(config, url.clone(), Default::default())
        .await
        .expect("Failed to create client");

    store.clear();

    let url = format!("http://{HOST}/push_navigate?redirect_type=live_redirect");
    let redirect_url = format!("http://{HOST}/redirect_to");

    client
        .navigate(url.clone(), Default::default())
        .await
        .expect("nav failed");

    assert_eq!(client.current_history_entry().unwrap().url, redirect_url);

    // assert that it contains at least one live redirect
    assert_any!(store, |m| {
        if let MockMessage::NetworkEvent(_, Payload::JSONPayload { json }) = m {
            let real = json!({
               "live_redirect" : {
                "to" : "/redirect_to",
                "kind" : "push",
               }
            });
            return json == &JSON::from(real);
        }
        false
    });

    assert_any!(store, |m| { matches!(m, MockMessage::ViewReload { .. }) });

    store.clear();

    let url = format!("http://{HOST}/push_navigate?redirect_type=patch");
    let redirect_url = format!("http://{HOST}/push_navigate?patched=value");
    client
        .navigate(url.clone(), Default::default())
        .await
        .expect("nav failed");

    // call a patch handler, patches can only happen after mount
    let channel = client.create_channel();
    let payload = json_payload!({"type": "click", "event": "patchme", "value": {}});
    channel
        .call("event".into(), payload)
        .await
        .expect("error on click");

    // assert that the url got patched
    // and that the event landed
    assert_any!(store, |m| {
        if let MockMessage::NetworkEvent(_, Payload::JSONPayload { json }) = m {
            let real = json!({
                "kind" : "push",
                "to" : "/push_navigate?patched=value"
            });
            return json == &JSON::from(real);
        }
        false
    });

    assert_eq!(client.current_history_entry().unwrap().url, redirect_url);

    store.clear()
}
