mod ffi;

use super::socket::LiveSocket;
pub use ffi::*;
use reqwest::Url;
use std::sync::Arc;

#[derive(Clone)]
struct HandlerInternal(pub Option<Arc<dyn NavEventHandler>>);

impl std::fmt::Debug for HandlerInternal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_some() {
            write!(f, "Handler Active")?;
        } else {
            write!(f, "No Handler Present")?;
        };
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct NavCtx {
    history: Vec<NavHistoryEntry>,
    current_id: HistoryId,
    navigation_event_handler: HandlerInternal,
    current_dest: Option<NavHistoryEntry>,
}

impl NavHistoryEntry {
    pub fn new(url: Url, id: HistoryId, state: Option<Vec<u8>>) -> Self {
        Self {
            url: url.to_string(),
            id,
            state,
        }
    }
}

impl NavCtx {
    pub fn new() -> Self {
        Self {
            history: vec![],
            current_id: 0,
            navigation_event_handler: HandlerInternal(None),
            current_dest: None,
        }
    }

    /// Navigate to `url` with behavior and metadata specified in `opts`.
    pub fn navigate(&mut self, url: Url, opts: NavOptions) {
        let next_dest = self.speculative_next_dest(&url, opts.state.clone());
        let event = NavEvent::new_from_navigate(next_dest.clone(), self.current_dest.clone(), opts);

        match self.handle_event(event) {
            HandlerResponse::Default => {}
            HandlerResponse::PreventDefault => return,
        };

        self.current_id += 1;
        self.history.push(next_dest)
    }

    pub fn set_event_handler(&mut self, handler: Arc<dyn NavEventHandler>) {
        self.navigation_event_handler.0 = Some(handler)
    }

    pub fn handle_event(&mut self, event: NavEvent) -> HandlerResponse {
        if let Some(handler) = self.navigation_event_handler.0.as_ref() {
            handler.handle_event(event)
        } else {
            HandlerResponse::Default
        }
    }

    fn speculative_next_dest(&self, url: &Url, state: Option<Vec<u8>>) -> NavHistoryEntry {
        NavHistoryEntry {
            id: self.current_id + 1,
            url: url.to_string(),
            state,
        }
    }
}

impl LiveSocket {}
