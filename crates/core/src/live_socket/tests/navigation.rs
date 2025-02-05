use std::sync::{Arc, Mutex};

use pretty_assertions::assert_eq;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::assert_doc_eq;
use crate::{
    callbacks::*,
    live_socket::{
        navigation::{NavCtx, NavOptions},
        LiveSocket,
    },
};

// Mock event handler used to validate the internal
// navigation objects state.
pub struct NavigationInspector {
    last_event: Mutex<Option<NavEvent>>,
}

#[derive(Serialize, Deserialize)]
pub struct EventMetadata {
    prevent_default: bool,
}

#[derive(Serialize, Deserialize)]
pub struct HistoryState {
    name: String,
}

impl NavEventHandler for NavigationInspector {
    fn handle_event(&self, event: NavEvent) -> HandlerResponse {
        *self.last_event.lock().expect("Lock poisoned!") = Some(event);
        HandlerResponse::Default
    }
}

impl NavigationInspector {
    pub fn new() -> Self {
        Self {
            last_event: None.into(),
        }
    }

    pub fn last_event(&self) -> Option<NavEvent> {
        self.last_event.lock().expect("Lock poisoned!").clone()
    }
}

impl NavEvent {
    // utility function so I can sugar out boiler plate code in tests.
    pub fn empty() -> Self {
        Self {
            to: NavHistoryEntry {
                url: String::new(),
                id: 0,
                state: None,
            },
            event: NavEventType::Push,
            same_document: false,
            from: None,
            info: None,
        }
    }
}

#[test]
fn basic_internal_nav() {
    let handler = Arc::new(NavigationInspector::new());
    let mut ctx = NavCtx::default();
    ctx.set_event_handler(handler.clone());

    // simple push nav
    let url_str = "https://www.website.com/live";
    let url = Url::parse(url_str).expect("URL failed to parse");
    ctx.navigate(url, NavOptions::default(), true);

    assert_eq!(
        NavEvent {
            event: NavEventType::Push,
            to: NavHistoryEntry {
                state: None,
                id: 1,
                url: url_str.to_string(),
            },
            ..NavEvent::empty()
        },
        handler.last_event().expect("Missing Event")
    );
}

#[test]
fn basic_internal_navigate_back() {
    let handler = Arc::new(NavigationInspector::new());
    let mut ctx = NavCtx::default();
    ctx.set_event_handler(handler.clone());

    // initial page
    let first_url_str = "https://www.website.com/first";
    let url = Url::parse(first_url_str).expect("URL failed to parse");
    ctx.navigate(url, NavOptions::default(), true);

    // second page
    let url_str = "https://www.website.com/second";
    let url = Url::parse(url_str).expect("URL failed to parse");
    ctx.navigate(url, NavOptions::default(), true)
        .expect("Failed.");

    assert_eq!(
        NavEvent {
            to: NavHistoryEntry {
                state: None,
                id: 2,
                url: url_str.to_string(),
            },
            from: NavHistoryEntry {
                state: None,
                id: 1,
                url: first_url_str.to_string(),
            }
            .into(),
            ..NavEvent::empty()
        },
        handler.last_event().expect("Missing Event")
    );

    //go back one view
    ctx.back(None, true).expect("Failed Back.");

    assert_eq!(
        NavEvent {
            to: NavHistoryEntry {
                state: None,
                id: 1,
                url: first_url_str.to_string(),
            },
            from: NavHistoryEntry {
                state: None,
                id: 2,
                url: url_str.to_string(),
            }
            .into(),
            ..NavEvent::empty()
        },
        handler.last_event().expect("Missing Event")
    );
}

#[test]
fn test_navigation_with_state() {
    let handler = Arc::new(NavigationInspector::new());
    let mut ctx = NavCtx::default();
    ctx.set_event_handler(handler.clone());

    let url = Url::parse("https://example.com").expect("parse");
    let state = vec![1, 2, 3];
    let info = vec![4, 5, 6];

    let opts = NavOptions {
        state: Some(state.clone()),
        extra_event_info: Some(info.clone()),
        ..Default::default()
    };

    let id = ctx.navigate(url.clone(), opts, true).expect("nav");

    let last_ev = handler.last_event().expect("no event.");
    assert_eq!(last_ev.info, Some(info));

    let current = ctx.current().expect("current");
    assert_eq!(current.id, id);
    assert_eq!(current.state, Some(state));
}

#[test]
fn test_navigation_stack() {
    let mut ctx = NavCtx::default();
    let first = Url::parse("https://example.com/first").expect("parse first");
    let second = Url::parse("https://example.com/second").expect("parse second");
    let third = Url::parse("https://example.com/third").expect("parse third");

    let id1 = ctx
        .navigate(first.clone(), NavOptions::default(), true)
        .expect("nav first");
    let id2 = ctx
        .navigate(second.clone(), NavOptions::default(), true)
        .expect("nav second");
    let id3 = ctx
        .navigate(third.clone(), NavOptions::default(), true)
        .expect("nav third");

    assert_eq!(ctx.current().expect("current").url, third.to_string());

    let prev_id = ctx.back(None, true).expect("back");
    assert_eq!(prev_id, id2);
    assert_eq!(ctx.current().expect("current").url, second.to_string());
    assert_eq!(ctx.entries().len(), 3);

    let next_id = ctx.forward(None, true).expect("forward");
    assert_eq!(next_id, id3);
    assert_eq!(ctx.current().expect("current").url, third.to_string());
    assert_eq!(ctx.entries().len(), 3);

    ctx.traverse_to(id1, None, true)
        .expect("Failed to traverse");
    assert_eq!(ctx.current().expect("current").url, first.to_string());
    assert_eq!(ctx.entries().len(), 3);

    ctx.traverse_to(id3, None, true)
        .expect("Failed to traverse");
    assert_eq!(ctx.current().expect("current").url, third.to_string());
    assert_eq!(ctx.entries().len(), 3);
}

#[cfg(target_os = "android")]
const HOST: &str = "10.0.2.2:4001";

#[cfg(not(target_os = "android"))]
const HOST: &str = "127.0.0.1:4001";

#[test]
fn test_navigation_rollback_forward() {
    let mut ctx = NavCtx::default();
    let first = Url::parse("https://example.com/first").expect("parse first");
    let second = Url::parse("https://example.com/second").expect("parse second");

    let id1 = ctx
        .navigate(first.clone(), NavOptions::default(), true)
        .expect("nav first");

    let id2 = ctx
        .navigate(second.clone(), NavOptions::default(), true)
        .expect("nav second");

    ctx.back(None, true).expect("back");
    assert_eq!(ctx.current().expect("current").id, id1);

    ctx.forward(None, true).expect("forward");
    assert_eq!(ctx.current().expect("current").id, id2);
}

#[tokio::test]
async fn basic_nav_flow() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let first = "first_page";
    let second = "second_page";
    let url = format!("http://{HOST}/nav/{first}");

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
        first_page
    </Text>
    <NavigationLink id="Next" destination="/nav/next">
        <Text>
            NEXT
        </Text>
    </NavigationLink>
</VStack>
"#;

    assert_doc_eq!(expected, join_doc.to_string());

    let url = format!("http://{HOST}/nav/{second}");
    let live_channel = live_socket
        .navigate(url, None, Default::default())
        .await
        .expect("navigate");

    let join_doc = live_channel
        .join_document()
        .expect("Failed to render join payload");

    let expected = r#"
<Group id="flash-group" />
<VStack>
    <Text>
       second_page
    </Text>
    <NavigationLink id="Next" destination="/nav/next">
        <Text>
            NEXT
        </Text>
    </NavigationLink>
</VStack>
"#;

    assert_doc_eq!(expected, join_doc.to_string());
}
