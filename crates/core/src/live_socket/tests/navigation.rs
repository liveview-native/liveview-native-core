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
    ctx.navigate(url, NavOptions::default());

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
    ctx.back(None);

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

// TODO:
// - [ ] Test `replace` navigation.
// - [ ] Test state and info passing.
