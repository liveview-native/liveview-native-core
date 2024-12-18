//! # FFI Navigation Types
//!
//! Types and utilities for interacting with the navigation API for the FFI api consumers.
use std::sync::Arc;

use phoenix_channels_client::{Payload, Socket, JSON};
use reqwest::Url;

pub type HistoryId = u64;
const RETRY_REASONS: &[&str] = &["stale", "unauthorized"];

#[uniffi::export(callback_interface)]
pub trait NavEventHandler: Send + Sync {
    /// This callback instruments events that occur when your user navigates to a
    /// new view. You can add serialized metadata to these events as a byte buffer
    /// through the [NavOptions] object.
    fn handle_event(&self, event: NavEvent) -> HandlerResponse;
}

/// User emitted response from [NavEventHandler::handle_event].
/// Determines whether or not the default navigation action is taken.
#[derive(uniffi::Enum, Clone, Debug, PartialEq, Default)]
pub enum HandlerResponse {
    #[default]
    /// Return this to proceed as normal.
    Default,
    /// Return this to cancel the navigation before it occurs.
    PreventDefault,
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq)]
pub enum NavEventType {
    /// Pushing a new event onto the history stack
    Push,
    /// Replacing the most recent event on the history stack
    Replace,
    /// Reloading the view in place
    Reload,
    /// Skipping multiple items on the history stack, leaving them in tact.
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
    /// The type of event being emitted.
    pub event: NavEventType,
    /// True if from and to point to the same path.
    pub same_document: bool,
    /// The previous location of the page, if there was one.
    pub from: Option<NavHistoryEntry>,
    /// Destination URL.
    pub to: NavHistoryEntry,
    /// Additional user provided metadata handed to the event handler.
    pub info: Option<Vec<u8>>,
}

/// An action taken with respect to the history stack
/// when [NavCtx::navigate] is executed. defaults to
/// Push behavior.
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
    #[uniffi(default = None)]
    pub action: Option<NavAction>,
    #[uniffi(default = None)]
    pub extra_event_info: Option<Vec<u8>>,
    #[uniffi(default = None)]
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
}

use super::{super::error::LiveSocketError, LiveSocket, NavCtx};
use crate::live_socket::{socket::SessionData, LiveChannel};

impl LiveSocket {
    /// Tries to navigate to the current item in the NavCtx.
    /// changing state in one fell swoop if initialilization succeeds
    async fn try_nav(
        &self,
        old_channel: Option<Arc<LiveChannel>>,
    ) -> Result<Arc<LiveChannel>, LiveSocketError> {
        let current = self
            .current()
            .ok_or(LiveSocketError::NavigationImpossible)?;

        let params = old_channel.map(|c| c.join_params.clone());

        let url = Url::parse(&current.url)?;

        match self
            .join_liveview_channel(params.clone(), Some(url.to_string()))
            .await
        {
            // A join rejection should be ameliorated by reconnecting
            Err(LiveSocketError::JoinRejection {
                error:
                    Payload::JSONPayload {
                        json: JSON::Object { object },
                    },
            }) => {
                if object
                    .get("reason")
                    .and_then(|r| match r {
                        JSON::Str { string } => Some(string),
                        _ => None,
                    })
                    .is_none_or(|reason| !RETRY_REASONS.contains(&reason.as_str()))
                {
                    return Err(LiveSocketError::JoinRejection {
                        error: Payload::JSONPayload {
                            json: JSON::Object { object },
                        },
                    });
                }

                let format = self.session_data.try_lock()?.format.clone();
                let options = self.session_data.try_lock()?.connect_opts.clone();

                let session_data = SessionData::request(&url, &format, options).await?;
                let websocket_url = session_data.get_live_socket_url()?;
                let socket =
                    Socket::spawn(websocket_url, Some(session_data.cookies.clone())).await?;

                self.socket()
                    .disconnect()
                    .await
                    .map_err(|_| LiveSocketError::DisconnectionError)?;

                *self.socket.try_lock()? = socket;
                *self.session_data.try_lock()? = session_data;
                self.join_liveview_channel(params, None)
                    .await
                    .map(Into::into)
            }
            // Just reconnect or bail
            Ok(chan) => Ok(chan.into()),
            Err(e) => Err(e),
        }
    }

    /// calls [Self::try_nav] rolling back to a previous navigation state on failure.
    async fn try_nav_outer<F>(
        &self,
        old_channel: Option<Arc<LiveChannel>>,
        nav_action: F,
    ) -> Result<Arc<LiveChannel>, LiveSocketError>
    where
        F: FnOnce(&mut NavCtx) -> Option<HistoryId>,
    {
        let new_id = {
            let mut ctx = self.navigation_ctx.lock().expect("lock poison");
            nav_action(&mut ctx)
        };

        if new_id.is_none() {
            return Err(LiveSocketError::NavigationImpossible);
        };

        match self.try_nav(old_channel).await {
            Ok(channel) => Ok(channel),
            Err(e) => Err(e),
        }
    }
}

#[cfg_attr(not(target_family = "wasm"), uniffi::export(async_runtime = "tokio"))]
impl LiveSocket {
    pub async fn navigate(
        &self,
        url: String,
        old_channel: Option<Arc<LiveChannel>>,
        opts: NavOptions,
    ) -> Result<Arc<LiveChannel>, LiveSocketError> {
        let url = Url::parse(&url)?;
        self.try_nav_outer(old_channel, |ctx| ctx.navigate(url, opts, true))
            .await
    }

    pub async fn reload(
        &self,
        old_channel: Arc<LiveChannel>,
        info: Option<Vec<u8>>,
    ) -> Result<Arc<LiveChannel>, LiveSocketError> {
        self.try_nav_outer(Some(old_channel), |ctx| ctx.reload(info, true))
            .await
    }

    pub async fn back(
        &self,
        old_channel: Option<Arc<LiveChannel>>,
        info: Option<Vec<u8>>,
    ) -> Result<Arc<LiveChannel>, LiveSocketError> {
        self.try_nav_outer(old_channel, |ctx| ctx.back(info, true))
            .await
    }

    pub async fn forward(
        &self,
        old_channel: Option<Arc<LiveChannel>>,
        info: Option<Vec<u8>>,
    ) -> Result<Arc<LiveChannel>, LiveSocketError> {
        self.try_nav_outer(old_channel, |ctx| ctx.forward(info, true))
            .await
    }

    pub async fn traverse_to(
        &self,
        id: HistoryId,
        old_channel: Option<Arc<LiveChannel>>,
        info: Option<Vec<u8>>,
    ) -> Result<Arc<LiveChannel>, LiveSocketError> {
        self.try_nav_outer(old_channel, |ctx| ctx.traverse_to(id, info, true))
            .await
    }

    /// Returns whether navigation backward in history is possible.
    pub fn can_go_back(&self) -> bool {
        let nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        nav_ctx.can_go_back()
    }

    /// Returns whether navigation forward in history is possible.
    pub fn can_go_forward(&self) -> bool {
        let nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        nav_ctx.can_go_forward()
    }

    /// Returns whether navigation to the specified history entry ID is possible.
    pub fn can_traverse_to(&self, id: HistoryId) -> bool {
        let nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        nav_ctx.can_traverse_to(id)
    }

    /// Returns a list of all history entries in traversal sequence order.
    pub fn get_entries(&self) -> Vec<NavHistoryEntry> {
        let nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        nav_ctx.entries()
    }

    /// Returns the current history entry, if one exists.
    pub fn current(&self) -> Option<NavHistoryEntry> {
        let nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        nav_ctx.current()
    }

    /// Sets the handler for navigation events.
    pub fn set_event_handler(&self, handler: Box<dyn NavEventHandler>) {
        let mut nav_ctx = self.navigation_ctx.lock().expect("lock poison");
        nav_ctx.set_event_handler(handler.into())
    }
}
