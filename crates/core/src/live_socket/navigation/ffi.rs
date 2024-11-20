//! # FFI Navigation Types
//!
//! Types and utilities for interacting with the navigation API for the FFI api consumers.
use reqwest::Url;

pub type HistoryId = u64;

#[uniffi::export(callback_interface)]
pub trait NavEventHandler: Send + Sync {
    /// This callback instruments events that occur when your user navigates to a
    /// new view. You can add serialized metadata to these events as a byte buffer
    /// through the [NavOptions] object.
    fn handle_event(&self, event: NavEvent) -> HandlerResponse;
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq, Default)]
pub enum HandlerResponse {
    #[default]
    Default,
    PreventDefault,
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq)]
pub enum NavEventType {
    Push,
    Replace,
    Reload,
    Traverse,
}

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct NavHistoryEntry {
    /// The target url.
    pub url: String,
    /// Unique id for this piece of nav entry state.
    pub id: HistoryId,
    /// state passed in by the user, to be passed in to the navigation event callback.
    pub state: Option<Vec<u8>>,
}

/// An event emitted when the user navigates between views.
#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct NavEvent {
    pub event: NavEventType,
    pub same_document: bool,
    /// The previous location of the page, if there was one
    pub from: Option<NavHistoryEntry>,
    /// Destination URL
    pub to: NavHistoryEntry,
    /// Additional user provided metadata handed to the event handler.
    pub info: Option<Vec<u8>>,
}

// An action taken with respect to the history stack
// when [NavCtx::navigate] is executed.
#[derive(uniffi::Enum, Default, Clone)]
pub enum NavAction {
    /// Push the navigation event onto the history stack.
    #[default]
    Push,
    /// Replace the current top of the history stack with this navigation event.
    Replace,
}

/// Options for calls to [NavCtx::navigate]
#[derive(Default, uniffi::Record)]
pub struct NavOptions {
    pub action: NavAction,
    pub extra_event_info: Option<Vec<u8>>,
    pub state: Option<Vec<u8>>,
}

impl NavEvent {
    pub fn new(
        event: NavEventType,
        to: NavHistoryEntry,
        from: Option<NavHistoryEntry>,
        info: Option<Vec<u8>>,
    ) -> Self {
        let new_url = Url::parse(&to.url).ok();
        let old_url = from.as_ref().and_then(|dest| Url::parse(&dest.url).ok());

        let same_document = old_url
            .zip(new_url)
            .is_some_and(|(old, new)| old.path() == new.path());

        NavEvent {
            event,
            same_document,
            from,
            to,
            info,
        }
    }

    pub fn new_from_reload(dest: NavHistoryEntry, info: Option<Vec<u8>>) -> NavEvent {
        NavEvent::new(NavEventType::Reload, dest.clone(), dest.into(), info)
    }

    /// Create a new nav event from the details of a [NavCtx::traverse_to] event
    pub fn new_from_traverse(
        new_dest: NavHistoryEntry,
        old_dest: Option<NavHistoryEntry>,
        info: Option<Vec<u8>>,
    ) -> NavEvent {
        NavEvent::new(NavEventType::Traverse, new_dest, old_dest, info)
    }

    /// Create a new nav event from the details of a [NavCtx::navigate] event
    pub fn new_from_navigate(
        new_dest: NavHistoryEntry,
        old_dest: Option<NavHistoryEntry>,
        opts: NavOptions,
    ) -> NavEvent {
        let event = match opts.action {
            NavAction::Push => NavEventType::Push,
            NavAction::Replace => NavEventType::Replace,
        };

        NavEvent::new(event, new_dest, old_dest, opts.extra_event_info)
    }

    /// Create a new nav event from the details of a [NavCtx::back] event,
    /// passing info into the event handler closure.
    pub fn new_from_forward(
        new_dest: NavHistoryEntry,
        old_dest: Option<NavHistoryEntry>,
        info: Option<Vec<u8>>,
    ) -> NavEvent {
        NavEvent::new(NavEventType::Push, new_dest, old_dest, info)
    }

    /// Create a new nav event from the details of a [NavCtx::back] event,
    /// passing info into the event handler closure.
    pub fn new_from_back(
        new_dest: NavHistoryEntry,
        old_dest: NavHistoryEntry,
        info: Option<Vec<u8>>,
    ) -> NavEvent {
        NavEvent::new(NavEventType::Push, new_dest, Some(old_dest), info)
    }
}

use super::{super::error::LiveSocketError, LiveSocket};

#[cfg_attr(not(target_family = "wasm"), uniffi::export(async_runtime = "tokio"))]
impl LiveSocket {
    pub async fn navigate(
        &self,
        url: String,
        opts: NavOptions,
    ) -> Result<Option<HistoryId>, LiveSocketError> {
        let url = Url::parse(&url)?;

        let mut nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        let res = nav_ctx.navigate(url, opts);

        if res.is_some() {
            //let _ = nav_ctx.current();
            //todo!("connect logic")
        }

        Ok(res)
    }

    pub async fn reload(
        &self,
        info: Option<Vec<u8>>,
    ) -> Result<Option<HistoryId>, LiveSocketError> {
        let mut nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        let res = nav_ctx.reload(info);
        if res.is_some() {
            if let Some(_current) = nav_ctx.current() {}
        }
        Ok(res)
    }

    pub async fn back(&self, info: Option<Vec<u8>>) -> Result<Option<HistoryId>, LiveSocketError> {
        let mut nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        let res = nav_ctx.back(info);
        if res.is_some() {
            if let Some(_current) = nav_ctx.current() {}
        }
        Ok(res)
    }

    pub async fn forward(
        &self,
        info: Option<Vec<u8>>,
    ) -> Result<Option<HistoryId>, LiveSocketError> {
        let mut nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        let res = nav_ctx.forward(info);
        if res.is_some() {
            if let Some(_current) = nav_ctx.current() {}
        }
        Ok(res)
    }

    pub async fn traverse_to(
        &self,
        id: HistoryId,
        info: Option<Vec<u8>>,
    ) -> Result<Option<HistoryId>, LiveSocketError> {
        let mut nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        let res = nav_ctx.traverse_to(id, info);
        if res.is_some() {
            if let Some(_current) = nav_ctx.current() {}
        }
        Ok(res)
    }

    pub fn can_go_back(&self) -> bool {
        let nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        nav_ctx.can_go_back()
    }

    pub fn can_go_forward(&self) -> bool {
        let nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        nav_ctx.can_go_forward()
    }

    pub fn can_traverse_to(&self, id: HistoryId) -> bool {
        let nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        nav_ctx.can_traverse_to(id)
    }

    pub fn get_entries(&self) -> Vec<NavHistoryEntry> {
        let nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        nav_ctx.entries()
    }

    pub fn current(&self) -> Option<NavHistoryEntry> {
        let nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        nav_ctx.current()
    }

    pub fn set_event_handler(&self, handler: Box<dyn NavEventHandler>) {
        let mut nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        nav_ctx.set_event_handler(handler.into())
    }
}
