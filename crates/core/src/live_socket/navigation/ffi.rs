//! # FFI Navigation Types
//!
//! Types and utilities for interacting with the navigation API for the FFI api consumers.
use std::collections::HashMap;

use phoenix_channels_client::{Payload, Socket, JSON};
#[cfg(not(test))]
use reqwest::cookie::Jar;
use reqwest::{redirect::Policy, Url};

use crate::callbacks::*;

const RETRY_REASONS: &[&str] = &["stale", "unauthorized"];

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
    /// Additional params to be passed upon joining the liveview channel.
    #[uniffi(default = None)]
    pub join_params: Option<HashMap<String, JSON>>,
    /// see [NavAction], defaults to [NavAction::Push].
    #[uniffi(default = None)]
    pub action: Option<NavAction>,
    /// Ephemeral extra information to be pushed to the even handler.
    #[uniffi(default = None)]
    pub extra_event_info: Option<Vec<u8>>,
    /// Persistent state, intended to be deserialized for user specific purposes when
    /// revisiting a given view.
    #[uniffi(default = None)]
    pub state: Option<Vec<u8>>,
}

#[derive(Default, uniffi::Record)]
pub struct NavActionOptions {
    /// Additional params to be passed upon joining the liveview channel.
    #[uniffi(default = None)]
    pub join_params: Option<HashMap<String, JSON>>,
    /// Ephemeral extra information to be pushed to the even handler.
    #[uniffi(default = None)]
    pub extra_event_info: Option<Vec<u8>>,
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

use super::{LiveSocket, NavCtx};
#[cfg(not(test))]
use crate::live_socket::socket::COOKIE_JAR;
#[cfg(test)]
use crate::live_socket::socket::TEST_COOKIE_JAR;
use crate::{
    error::LiveSocketError,
    live_socket::{socket::SessionData, LiveChannel},
};

impl LiveSocket {
    /// Tries to navigate to the current item in the NavCtx.
    /// changing state in one fell swoop if initialilization succeeds
    async fn try_nav(
        &self,
        join_params: Option<HashMap<String, JSON>>,
    ) -> Result<LiveChannel, LiveSocketError> {
        let current = self
            .current()
            .ok_or(LiveSocketError::NavigationImpossible)?;

        let url = Url::parse(&current.url)?;

        match self
            .join_liveview_channel(join_params.clone(), url.to_string().into())
            .await
        {
            // A join rejection should be ameliorated by reconnecting
            Err(LiveSocketError::JoinRejection {
                error:
                    Payload::JSONPayload {
                        json: JSON::Object { object },
                    },
            }) => {
                // retry on { "reason" : "stale" } and unauthorized
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

                //TODO: punt the to an argument. move this on to the LiveViewClient
                #[cfg(not(test))]
                let jar = COOKIE_JAR.get_or_init(|| Jar::default().into());

                #[cfg(test)]
                let jar = TEST_COOKIE_JAR.with(|inner| inner.clone());

                let client = reqwest::Client::builder()
                    .use_rustls_tls()
                    // Use platform certs maintained by OS
                    .tls_built_in_native_certs(true)
                    .cookie_provider(jar.clone())
                    .redirect(Policy::none())
                    .build()?;

                let session_data = SessionData::request(&url, &format, options, client).await?;
                let websocket_url = session_data.get_live_socket_url()?;
                let socket =
                    Socket::spawn(websocket_url, Some(session_data.cookies.clone())).await?;

                self.socket()
                    .shutdown()
                    .await
                    .map_err(|_| LiveSocketError::DisconnectionError)?;

                *self.socket.try_lock()? = socket;
                *self.session_data.try_lock()? = session_data;
                self.join_liveview_channel(join_params, None).await
            }
            // Just reconnect or bail
            Ok(chan) => Ok(chan),
            Err(e) => Err(e),
        }
    }

    /// calls [Self::try_nav] rolling back to a previous navigation state on failure.
    async fn try_nav_outer<F>(
        &self,
        join_params: Option<HashMap<String, JSON>>,
        nav_action: F,
    ) -> Result<LiveChannel, LiveSocketError>
    where
        F: FnOnce(&mut NavCtx) -> Option<HistoryId>,
    {
        // Tries to complete the nav action, updating state,
        // this may be cancelled by the user or by the navigation
        // being impossiblem, such as back navigation on an empty stack.
        let new_id = {
            let mut ctx = self.navigation_ctx.lock().expect("lock poison");
            nav_action(&mut ctx)
        };

        if new_id.is_none() {
            return Err(LiveSocketError::NavigationImpossible);
        };

        // actually return the update liveview channel
        match self.try_nav(join_params).await {
            Ok(channel) => Ok(channel),
            Err(e) => Err(e),
        }
    }
}

#[cfg_attr(not(target_family = "wasm"), uniffi::export(async_runtime = "tokio"))]
impl LiveSocket {
    /// Navigates the socket to a new URL, reusing the previous channel's connection parameters, closing it safely,
    /// and emitting a new [LiveChannel]
    pub async fn navigate(
        &self,
        url: String,
        join_params: Option<HashMap<String, JSON>>,
        opts: NavOptions,
    ) -> Result<LiveChannel, LiveSocketError> {
        let url = Url::parse(&url)?;
        self.try_nav_outer(join_params, |ctx| ctx.navigate(url, opts, true))
            .await
    }

    /// Reload the current channel.
    pub async fn reload(
        &self,
        join_params: Option<HashMap<String, JSON>>,
        info: Option<Vec<u8>>,
    ) -> Result<LiveChannel, LiveSocketError> {
        self.try_nav_outer(join_params, |ctx| ctx.reload(info, true))
            .await
    }

    /// Navigates the socket to the previous entry in the stack.
    pub async fn back(
        &self,
        join_params: Option<HashMap<String, JSON>>,
        info: Option<Vec<u8>>,
    ) -> Result<LiveChannel, LiveSocketError> {
        self.try_nav_outer(join_params, |ctx| ctx.back(info, true))
            .await
    }

    /// Navigates the socket to the next entry in the stack. Reuses the previous channel's connection parameters, closes it safely,
    /// and emits a new [LiveChannel]
    pub async fn forward(
        &self,
        join_params: Option<HashMap<String, JSON>>,
        info: Option<Vec<u8>>,
    ) -> Result<LiveChannel, LiveSocketError> {
        self.try_nav_outer(join_params, |ctx| ctx.forward(info, true))
            .await
    }

    /// Navigates the socket to the specified entry in the stack, preserving the stack. Resuses the previous channel's connection parameters, closes it safely,
    /// and emits a new [LiveChannel]
    pub async fn traverse_to(
        &self,
        id: HistoryId,
        join_params: Option<HashMap<String, JSON>>,
        info: Option<Vec<u8>>,
    ) -> Result<LiveChannel, LiveSocketError> {
        self.try_nav_outer(join_params, |ctx| ctx.traverse_to(id, info, true))
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
