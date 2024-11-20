use std::sync::{Arc, Mutex};

use crate::live_socket::navigation::*;
use pretty_assertions::assert_eq;
use reqwest::Url;
use serde::{Deserialize, Serialize};

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
    fn empty() -> Self {
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
    ctx.navigate(url, NavOptions::default());

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
    ctx.navigate(url, NavOptions::default());

    // second page
    let url_str = "https://www.website.com/second";
    let url = Url::parse(url_str).expect("URL failed to parse");
    ctx.navigate(url, NavOptions::default()).expect("Failed.");

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

    //roll back one view
    ctx.back(None).expect("Failed Back.");

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
    let mut ctx = NavCtx::default();
    let url = Url::parse("https://example.com").expect("parse");
    let state = vec![1, 2, 3];

    let id = ctx
        .navigate(
            url.clone(),
            NavOptions {
                state: Some(state.clone()),
                ..Default::default()
            },
        )
        .expect("nav");

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
        .navigate(first.clone(), NavOptions::default())
        .expect("nav first");
    let id2 = ctx
        .navigate(second.clone(), NavOptions::default())
        .expect("nav second");
    let id3 = ctx
        .navigate(third.clone(), NavOptions::default())
        .expect("nav third");

    assert_eq!(ctx.current().expect("current").url, third.to_string());

    let prev_id = ctx.back(None).expect("back");
    assert_eq!(prev_id, id2);
    assert_eq!(ctx.current().expect("current").url, second.to_string());
    assert_eq!(ctx.entries().len(), 3);

    let next_id = ctx.forward(None).expect("forward");
    assert_eq!(next_id, id3);
    assert_eq!(ctx.current().expect("current").url, third.to_string());
    assert_eq!(ctx.entries().len(), 3);

    ctx.traverse_to(id1, None).expect("Failed to traverse");
    assert_eq!(ctx.current().expect("current").url, first.to_string());
    assert_eq!(ctx.entries().len(), 3);

    ctx.traverse_to(id3, None).expect("Failed to traverse");
    assert_eq!(ctx.current().expect("current").url, third.to_string());
    assert_eq!(ctx.entries().len(), 3);
}
