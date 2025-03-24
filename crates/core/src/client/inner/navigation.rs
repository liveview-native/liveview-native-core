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

#[derive(Debug, Clone, thiserror::Error)]
pub enum NavigationError {
    #[error("Navigation was prevented by a user handler")]
    PreventedByHandler,
    #[error("Invalid URL: {reason}")]
    InvalidUrl { reason: String },
    #[error("No current entry in navigation history")]
    NoCurrentEntry,
    #[error("Cannot navigate back: history is empty or has only one entry")]
    CannotGoBack,
    #[error("Cannot navigate forward: no forward entries available")]
    CannotGoForward,
    #[error("Cannot traverse to ID {id}: not found in history")]
    CannotTraverseToId { id: HistoryId },
    #[error("Navigation failed: {reason}")]
    Other { reason: String },
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
    pub fn navigate(
        &mut self,
        url: Url,
        opts: NavOptions,
        emit_event: bool,
    ) -> Result<HistoryId, NavigationError> {
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
            HandlerResponse::PreventDefault => return Err(NavigationError::PreventedByHandler),
        };

        match action {
            Some(NavAction::Replace) => self.replace_entry(next_dest),
            None | Some(NavAction::Push) => self.push_entry(next_dest),
        }

        // successful navigation invalidates previously coalesced state from
        // calls to `back`
        self.future.clear();
        Ok(next_id)
    }

    pub fn patch(
        &mut self,
        url_path: String,
        emit_event: bool,
    ) -> Result<HistoryId, NavigationError> {
        let old_dest = self.current().ok_or(NavigationError::NoCurrentEntry)?;
        let old_id = old_dest.id;
        let old_url = Url::parse(&old_dest.url).map_err(|e| NavigationError::InvalidUrl {
            reason: format!("{e:?}"),
        })?;
        let new_url = old_url
            .join(&url_path)
            .map_err(|_| NavigationError::InvalidUrl {
                reason: "Join Failed".into(),
            })?;

        let new_dest =
            NavHistoryEntry::new(new_url.to_string(), old_dest.id, old_dest.state.clone());

        let event = NavEvent::new(NavEventType::Patch, new_dest.clone(), old_dest.into(), None);

        match self.handle_event(event, emit_event) {
            HandlerResponse::Default => {}
            HandlerResponse::PreventDefault => return Err(NavigationError::PreventedByHandler),
        };

        self.push_entry(new_dest);

        Ok(old_id)
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
    pub fn reload(
        &mut self,
        info: Option<Vec<u8>>,
        emit_event: bool,
    ) -> Result<HistoryId, NavigationError> {
        let current = self.current().ok_or(NavigationError::NoCurrentEntry)?;
        let id = current.id;

        let event = NavEvent::new(NavEventType::Reload, current.clone(), current.into(), info);

        match self.handle_event(event, emit_event) {
            HandlerResponse::Default => {}
            HandlerResponse::PreventDefault => return Err(NavigationError::PreventedByHandler),
        };

        Ok(id)
    }

    /// Navigates back one step in the stack, returning the id of the new
    /// current entry if successful.
    /// This function fails if there is no current
    /// page or if there are no items in history and returns [None].
    pub fn back(
        &mut self,
        info: Option<Vec<u8>>,
        emit_event: bool,
    ) -> Result<HistoryId, NavigationError> {
        if !self.can_go_back() {
            log::warn!("Attempted `back` navigation without at minimum two entries.");
            return Err(NavigationError::CannotGoBack);
        }

        let previous = self.current().ok_or(NavigationError::NoCurrentEntry)?;

        let next = self.history[self.history.len() - 2].clone();

        let event = {
            let new_dest = next.clone();
            let old_dest = previous.clone();
            NavEvent::new(NavEventType::Push, new_dest, Some(old_dest), info)
        };

        match self.handle_event(event, emit_event) {
            HandlerResponse::Default => {
                let previous = self.history.pop().expect("precondition");
                let out = Ok(next.id);
                self.future.push(previous);
                out
            }
            HandlerResponse::PreventDefault => Err(NavigationError::PreventedByHandler),
        }
    }

    /// Navigate one step forward, fails if there is not at least one
    /// item in the history and future stacks.
    pub fn forward(
        &mut self,
        info: Option<Vec<u8>>,
        emit_event: bool,
    ) -> Result<HistoryId, NavigationError> {
        if !self.can_go_forward() {
            log::warn!(
                "Attempted `future` navigation with an no current location or no next entry."
            );
            return Err(NavigationError::CannotGoForward);
        }

        let next = self.future.last().cloned().expect("precondition");
        let previous = self.current();

        let event = NavEvent::new(NavEventType::Push, next, previous, info);

        match self.handle_event(event, emit_event) {
            HandlerResponse::Default => {
                let next = self.future.pop().expect("precondition");
                let out = Ok(next.id);
                self.push_entry(next);
                out
            }
            HandlerResponse::PreventDefault => Err(NavigationError::PreventedByHandler),
        }
    }

    pub fn traverse_to(
        &mut self,
        id: HistoryId,
        info: Option<Vec<u8>>,
        emit_event: bool,
    ) -> Result<HistoryId, NavigationError> {
        if !self.can_traverse_to(id) {
            log::warn!("Attempted to traverse to an untracked ID!");
            return Err(NavigationError::CannotTraverseToId { id });
        }

        let old_dest = self.current().ok_or(NavigationError::NoCurrentEntry)?;
        let in_hist = self.history.iter().position(|ent| ent.id == id);
        if let Some(entry) = in_hist {
            let new_dest = self.history[entry].clone();

            let event = NavEvent::new(NavEventType::Traverse, new_dest, old_dest.into(), info);

            match self.handle_event(event, emit_event) {
                HandlerResponse::Default => {}
                HandlerResponse::PreventDefault => return Err(NavigationError::PreventedByHandler),
            };

            // All entries except the target
            let ext = self.history.drain(entry + 1..);
            self.future.extend(ext.rev());
            return Ok(id);
        }

        let in_fut = self.future.iter().position(|ent| ent.id == id);
        if let Some(entry) = in_fut {
            let new_dest = self.future[entry].clone();

            let event = NavEvent::new(NavEventType::Traverse, new_dest, old_dest.into(), info);

            match self.handle_event(event, emit_event) {
                HandlerResponse::Default => {}
                HandlerResponse::PreventDefault => return Err(NavigationError::PreventedByHandler),
            };

            // All entries including the target, which will be at the front.
            let ext = self.future.drain(entry..);
            self.history.extend(ext.rev());
            return Ok(id);
        }

        Err(NavigationError::CannotTraverseToId { id })
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

#[cfg(test)]
mod test {
    use std::sync::Mutex;

    use super::*;

    // Mock event handler used to validate the internal
    // navigation objects state.
    pub struct NavigationInspector {
        last_event: Mutex<Option<NavEvent>>,
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

    #[test]
    fn basic_internal_nav() {
        let handler = Arc::new(NavigationInspector::new());
        let mut ctx = NavCtx::default();
        ctx.set_event_handler(handler.clone());

        // simple push nav
        let url_str = "https://www.website.com/live";
        let url = Url::parse(url_str).expect("URL failed to parse");
        ctx.navigate(url, NavOptions::default(), true)
            .expect("Nav Failed");

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
        ctx.navigate(url, NavOptions::default(), true)
            .expect("Nav Failed");

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
}
