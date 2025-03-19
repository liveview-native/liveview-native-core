use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use phoenix_channels_client::{EventPayload, Payload, JSON};
use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::time::{sleep, timeout};

use super::{json_payload, HOST};
use crate::{
    client::{
        ClientStatus, HandlerResponse, LiveViewClientConfiguration, NavEvent, NavEventHandler,
        NavEventType, NavHistoryEntry, NetworkEventHandler, Platform,
    },
    expect_status_matches, LiveViewClient,
};

#[derive(Debug, Clone)]
pub enum MockMessage {
    Navigation(NavEvent),
    NetworkEvent(Payload),
    ViewReload(ClientStatus),
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

    pub async fn wait_for<F>(
        &self,
        predicate: F,
        timeout_duration: Duration,
    ) -> Result<MockMessage, &'static str>
    where
        F: Fn(&MockMessage) -> bool + Send + 'static,
    {
        let poll_future = async {
            loop {
                let result = {
                    let messages = self.messages.lock().unwrap();
                    messages.iter().find(|m| predicate(m)).cloned()
                };

                if let Some(message) = result {
                    return message;
                }

                sleep(Duration::from_millis(10)).await;
            }
        };

        match timeout(timeout_duration, poll_future).await {
            Ok(message) => Ok(message),
            Err(_) => Err("Timeout"),
        }
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
    fn on_event(&self, event: EventPayload) {
        self.message_store
            .add_message(MockMessage::NetworkEvent(event.payload));
    }

    fn on_status_change(&self, status: ClientStatus) {
        self.message_store
            .add_message(MockMessage::ViewReload(status));
    }
}

#[tokio::test]
async fn test_navigation_handler() {
    let store = Arc::new(MockMessageStore::new());
    let nav_handler = Arc::new(MockNavEventHandler::new(store.clone()));
    let net_handler = Arc::new(MockNetworkEventHandler::new(store.clone()));

    let url = format!("http://{HOST}/nav/first_page");
    let mut config = LiveViewClientConfiguration::default();
    config.dead_render_timeout = 1000;
    config.format = Platform::Swiftui;
    config.navigation_handler = Some(nav_handler);
    config.network_event_handler = Some(net_handler);

    let client = LiveViewClient::new(config, url.clone(), Default::default())
        .await
        .expect("Failed to create client");

    let next_url = format!("http://{HOST}/nav/second_page");

    store
        .wait_for(
            |e| matches!(e, MockMessage::ViewReload(ClientStatus::Connecting)),
            Duration::from_secs(2),
        )
        .await
        .expect("didn't connect");

    store
        .wait_for(
            |e| matches!(e, MockMessage::ViewReload(ClientStatus::Connected { .. })),
            Duration::from_secs(2),
        )
        .await
        .expect("didn't connect");

    store.clear();

    client
        .navigate(next_url.clone(), Default::default())
        .expect("Failed to navigate");

    store
        .wait_for(
            move |e| {
                let expected = NavEvent {
                    event: NavEventType::Push,
                    same_document: false,
                    from: Some(NavHistoryEntry::new(url.clone(), 1, None)),
                    to: NavHistoryEntry::new(next_url.clone(), 2, None),
                    info: None,
                };

                let MockMessage::Navigation(event) = e else {
                    return false;
                };
                *event == expected
            },
            Duration::from_secs(2),
        )
        .await
        .expect("didn't connect");

    store
        .wait_for(
            |e| matches!(e, MockMessage::ViewReload(ClientStatus::Connected { .. })),
            Duration::from_secs(2),
        )
        .await
        .expect("didn't connect");
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

    let client = LiveViewClient::new(config, url.clone(), Default::default())
        .await
        .expect("Failed to create client");

    store.clear();

    let url = format!("http://{HOST}/push_navigate?redirect_type=live_redirect");
    let redirect_url = format!("http://{HOST}/redirect_to");

    let mut watcher = client.watch_status();
    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    client
        .navigate(url.clone(), Default::default())
        .expect("nav failed");

    timeout(Duration::from_secs(3), watcher.changed())
        .await
        .expect("Timeout")
        .expect("Failed");

    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    assert_eq!(client.current_history_entry().unwrap().url, redirect_url);

    // assert that it contains at least one live redirect
    assert_any!(store, |m| {
        if let MockMessage::NetworkEvent(Payload::JSONPayload { json }) = m {
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
        .expect("nav failed");

    timeout(Duration::from_secs(3), watcher.changed())
        .await
        .expect("no update")
        .expect("timeout");

    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    // call a patch handler, patches can only happen after mount
    let payload = json_payload!({"type": "click", "event": "patchme", "value": {}});

    client
        .call("event".into(), payload)
        .await
        .expect("error on click");

    // assert that the url got patched
    // and that the event landed
    assert_any!(store, |m| {
        if let MockMessage::NetworkEvent(Payload::JSONPayload { json }) = m {
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
