mod ffi;

use super::socket::LiveSocket;
pub use ffi::*;
use reqwest::Url;
use std::sync::Arc;

#[derive(Clone, Default)]
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

#[derive(Debug, Clone, Default)]
pub struct NavCtx {
    // Previously visited views
    history: Vec<NavHistoryEntry>,
    // Views that are "forward" in history
    future: Vec<NavHistoryEntry>,
    /// monotonically increasing ID for `NavHistoryEntry`
    id_source: HistoryId,
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
    /// Navigate to `url` with behavior and metadata specified in `opts`.
    pub fn navigate(&mut self, url: Url, opts: NavOptions) {
        let action = opts.action.clone();
        let next_dest = self.speculative_next_dest(&url, opts.state.clone());
        let event = NavEvent::new_from_navigate(next_dest.clone(), self.current_dest.clone(), opts);

        match self.handle_event(event) {
            HandlerResponse::Default => {}
            HandlerResponse::PreventDefault => return,
        };

        match action {
            NavAction::Push => self.push_entry(next_dest),
            NavAction::Replace => self.replace_entry(next_dest),
        }

        self.future.clear();
    }

    /// Navigates back one step in the stack, returning the id of the new
    /// current entry if successful.
    /// This function fails if there is no current
    /// page or if there are no items in history and returns [None].
    pub fn back(&mut self, info: Option<Vec<u8>>) -> Option<HistoryId> {
        let Some(previous) = self.current_dest.clone() else {
            log::warn!("Attempted `back` navigation with no current state.");
            return None;
        };

        let Some(next) = self.history.pop() else {
            log::warn!("Attempted `back` navigation with an emprt history.");
            return None;
        };

        let event = NavEvent::new_from_back(next.clone(), previous.clone(), info);

        match self.handle_event(event) {
            HandlerResponse::Default => {
                let out = Some(next.id);
                self.push_entry(next);
                self.future.push(previous);
                out
            }
            HandlerResponse::PreventDefault => None,
        }
    }

    fn replace_entry(&mut self, history_entry: NavHistoryEntry) {
        if let Some(last) = self.history.last_mut() {
            self.id_source += 1;
            self.current_dest = Some(history_entry.clone());
            *last = history_entry
        } else {
            self.push_entry(history_entry)
        }
    }

    fn push_entry(&mut self, history_entry: NavHistoryEntry) {
        self.id_source += 1;
        let old = self.current_dest.replace(history_entry);
        self.history.extend(old);
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

    /// create a new destination if one would be added to history, this includes
    /// the next unique ID that would be issued.
    fn speculative_next_dest(&self, url: &Url, state: Option<Vec<u8>>) -> NavHistoryEntry {
        NavHistoryEntry {
            id: self.id_source + 1,
            url: url.to_string(),
            state,
        }
    }
}

impl LiveSocket {}
