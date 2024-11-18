use std::sync::{Arc, Mutex};

use crate::live_socket::navigation::*;
use pretty_assertions::{assert_eq, assert_ne};
use reqwest::Url;
use serde::{Deserialize, Serialize};

// Mock event handler used to validate the internal
// navigation objects state.
pub struct NavigationInspector {
    last_event: Mutex<Option<NavEvent>>,
    event_ct: Mutex<usize>,
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
        *self.event_ct.lock().expect("Lock poisoned!") += 1;
        HandlerResponse::Default
    }
}

impl NavigationInspector {
    pub fn new() -> Self {
        Self {
            last_event: None.into(),
            event_ct: 0.into(),
        }
    }

    pub fn last_event(&self) -> Option<NavEvent> {
        self.last_event.lock().expect("Lock poisoned!").clone()
    }

    pub fn event_ct(&self) -> usize {
        self.event_ct.lock().expect("Lock poisoned!").clone()
    }
}

#[test]
fn basic_internal_nav() {
    let handler = Arc::new(NavigationInspector::new());
    let mut ctx = NavCtx::new();
    ctx.set_event_handler(handler.clone());

    // sanity check
    assert_eq!(handler.event_ct(), 0);
    assert!(handler.last_event().is_none());

    // simple push nav
    let url_str = "https://www.website.com/live";
    let url = Url::parse(url_str).expect("URL failed to parse");
    ctx.navigate(url, NavOptions::default());

    assert_eq!(handler.event_ct(), 1);
    assert_eq!(
        NavEvent {
            info: None,
            state: None,
            event: NavEventType::Push,
            same_document: false,
            from: None,
            to: NavHistoryEntry {
                state: None,
                id: 1,
                url: url_str.to_string(),
            }
        },
        handler.last_event().expect("Missing Event")
    );
}
