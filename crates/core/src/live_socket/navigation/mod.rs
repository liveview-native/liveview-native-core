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
pub struct NavHistoryEntry {
    /// The target url.
    pub url: Url,
    /// Unique id for this piece of nav entry state.
    pub id: usize,
    /// state passed in by the user, to be passed in to the navigation event callback.
    pub state: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct NavCtx {
    history: Vec<NavHistoryEntry>,
    current_id: usize,
    navigation_event_handler: HandlerInternal,
}

impl NavHistoryEntry {
    pub fn new(url: Url, id: usize, state: Option<Vec<u8>>) -> Self {
        Self { url, id, state }
    }
}

impl NavCtx {
    pub fn new() -> Self {
        Self {
            history: vec![],
            current_id: 0,
            navigation_event_handler: HandlerInternal(None),
        }
    }

    /// Navigate to `url` with behavior and metadata specified in `opts`.
    pub fn navigate(&mut self, url: Url, opts: NavOptions) {
        let entry = NavHistoryEntry::new(url, self.current_id, None);
        self.current_id += 1;
        self.history.push(entry)
    }

    pub fn set_event_handler(&mut self, handler: Arc<dyn NavEventHandler>) {
        self.navigation_event_handler.0 = Some(handler)
    }

    pub fn handle_event(&mut self, event: NavEvent) {
        if let Some(handler) = self.navigation_event_handler.0.as_ref() {
            handler.handle_event(event);
        }
    }
}

impl LiveSocket {}
