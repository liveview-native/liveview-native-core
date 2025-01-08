use std::sync::Arc;

use reqwest::Url;

use crate::{
    callbacks::*,
    live_socket::navigation::{NavAction, NavOptions},
};

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

/// The internal navigation context.
/// handles the history state of the visited views.
#[derive(Debug, Clone, Default)]
pub struct NavCtx {
    /// Previously visited views
    history: Vec<NavHistoryEntry>,
    /// Views that are "forward" in history
    future: Vec<NavHistoryEntry>,
    /// monotonically increasing ID for `NavHistoryEntry`
    id_source: HistoryId,
    /// user provided callback
    navigation_event_handler: HandlerInternal,
}

impl NavCtx {
    /// Navigate to `url` with behavior and metadata specified in `opts`.
    /// Returns the current history ID if changed
    pub fn navigate(&mut self, url: Url, opts: NavOptions, emit_event: bool) -> Option<HistoryId> {
        let action = opts.action.clone();
        let next_dest = self.speculative_next_dest(&url, opts.state.clone());
        let next_id = next_dest.id;

        let event = {
            let new_dest = next_dest.clone();
            let old_dest = self.current();
            let event = match opts.action {
                Some(NavAction::Replace) => NavEventType::Replace,
                _ => NavEventType::Push,
            };

            NavEvent::new(event, new_dest, old_dest, opts.extra_event_info)
        };

        match self.handle_event(event, emit_event) {
            HandlerResponse::Default => {}
            HandlerResponse::PreventDefault => return None,
        };

        match action {
            Some(NavAction::Replace) => self.replace_entry(next_dest),
            None | Some(NavAction::Push) => self.push_entry(next_dest),
        }

        // successful navigation invalidates previously coalesced state from
        // calls to `back`
        self.future.clear();
        Some(next_id)
    }

    // Returns true if the navigator can go back one entry.
    pub fn can_go_back(&self) -> bool {
        self.history.len() >= 2
    }

    // Returns true if the navigator can go forward one entry.
    pub fn can_go_forward(&self) -> bool {
        !self.future.is_empty()
    }

    // Returns true if the `id` is tracked in the navigation context.
    pub fn can_traverse_to(&self, id: HistoryId) -> bool {
        let hist = self.history.iter().find(|ent| ent.id == id);
        let fut = self.future.iter().find(|ent| ent.id == id);
        hist.or(fut).is_some()
    }

    // Returns all of the tracked history entries by cloning them.
    // They are in traversal sequence order, with no guarantees about
    // the position of the current entry.
    pub fn entries(&self) -> Vec<NavHistoryEntry> {
        self.history
            .iter()
            .chain(self.future.iter().rev())
            .cloned()
            .collect()
    }

    /// Calls the handler for reload events
    pub fn reload(&mut self, info: Option<Vec<u8>>, emit_event: bool) -> Option<HistoryId> {
        let current = self.current()?;
        let id = current.id;

        let event = NavEvent::new(NavEventType::Reload, current.clone(), current.into(), info);

        match self.handle_event(event, emit_event) {
            HandlerResponse::Default => {}
            HandlerResponse::PreventDefault => return None,
        };

        Some(id)
    }

    /// Navigates back one step in the stack, returning the id of the new
    /// current entry if successful.
    /// This function fails if there is no current
    /// page or if there are no items in history and returns [None].
    pub fn back(&mut self, info: Option<Vec<u8>>, emit_event: bool) -> Option<HistoryId> {
        if !self.can_go_back() {
            log::warn!("Attempted `back` navigation without at minimum two entries.");
            return None;
        }

        let previous = self.current()?;

        let next = self.history[self.history.len() - 2].clone();

        let event = {
            let new_dest = next.clone();
            let old_dest = previous.clone();
            NavEvent::new(NavEventType::Push, new_dest, Some(old_dest), info)
        };

        match self.handle_event(event, emit_event) {
            HandlerResponse::Default => {
                let previous = self.history.pop()?;
                let out = Some(next.id);
                self.future.push(previous);
                out
            }
            HandlerResponse::PreventDefault => None,
        }
    }

    /// Navigate one step forward, fails if there is not at least one
    /// item in the history and future stacks.
    pub fn forward(&mut self, info: Option<Vec<u8>>, emit_event: bool) -> Option<HistoryId> {
        if !self.can_go_forward() {
            log::warn!(
                "Attempted `future` navigation with an no current location or no next entry."
            );
            return None;
        }

        let next = self.future.last().cloned()?;
        let previous = self.current();

        let event = NavEvent::new(NavEventType::Push, next, previous, info);

        match self.handle_event(event, emit_event) {
            HandlerResponse::Default => {
                let next = self.future.pop()?;
                let out = Some(next.id);
                self.push_entry(next);
                out
            }
            HandlerResponse::PreventDefault => None,
        }
    }

    pub fn traverse_to(
        &mut self,
        id: HistoryId,
        info: Option<Vec<u8>>,
        emit_event: bool,
    ) -> Option<HistoryId> {
        if !self.can_traverse_to(id) {
            log::warn!("Attempted to traverse to an untracked ID!");
            return None;
        }

        let old_dest = self.current()?;
        let in_hist = self.history.iter().position(|ent| ent.id == id);
        if let Some(entry) = in_hist {
            let new_dest = self.history[entry].clone();

            let event = NavEvent::new(NavEventType::Traverse, new_dest, old_dest.into(), info);

            match self.handle_event(event, emit_event) {
                HandlerResponse::Default => {}
                HandlerResponse::PreventDefault => return None,
            };

            // All entries except the target
            let ext = self.history.drain(entry + 1..);
            self.future.extend(ext.rev());
            return Some(id);
        }

        let in_fut = self.future.iter().position(|ent| ent.id == id);
        if let Some(entry) = in_fut {
            let new_dest = self.future[entry].clone();

            let event = NavEvent::new(NavEventType::Traverse, new_dest, old_dest.into(), info);

            match self.handle_event(event, emit_event) {
                HandlerResponse::Default => {}
                HandlerResponse::PreventDefault => return None,
            };

            // All entries including the target, which will be at the front.
            let ext = self.future.drain(entry..);
            self.history.extend(ext.rev());
            return Some(id);
        }

        None
    }

    /// Returns the current history entry and state
    pub fn current(&self) -> Option<NavHistoryEntry> {
        self.history.last().cloned()
    }

    fn replace_entry(&mut self, history_entry: NavHistoryEntry) {
        if let Some(last) = self.history.last_mut() {
            self.id_source += 1;

            *last = history_entry
        } else {
            self.push_entry(history_entry)
        }
    }

    fn push_entry(&mut self, history_entry: NavHistoryEntry) {
        self.id_source += 1;
        self.history.push(history_entry);
    }

    pub fn set_event_handler(&mut self, handler: Arc<dyn NavEventHandler>) {
        self.navigation_event_handler.0 = Some(handler)
    }

    pub fn handle_event(&mut self, event: NavEvent, emit_event: bool) -> HandlerResponse {
        if !emit_event {
            return HandlerResponse::Default;
        }

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
