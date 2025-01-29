mod config;
pub(crate) mod inner;

#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use config::*;
use futures::future::try_join_all;
use inner::LiveViewClientInner;
use phoenix_channels_client::{Payload, SocketStatus, JSON};
use reqwest::header::CONTENT_TYPE;

use crate::{
    callbacks::*,
    dom::ffi::{self},
    error::LiveSocketError,
    live_socket::{
        navigation::{NavActionOptions, NavOptions},
        ConnectOpts, LiveChannel, LiveFile, Method,
    },
};

const CSRF_HEADER: &str = "x-csrf-token";

/// A configuration interface for building a [LiveViewClient].
/// Options on this object will used for all http and websocket connections
/// through out the current session.
///
/// Additionally provides the [LiveViewClient] with callbacks and essential functionality,
/// without proper configuration your client may not function properly.
/// See [LiveViewClientBuilder::set_persistence_provider]
#[derive(uniffi::Object, Default)]
pub struct LiveViewClientBuilder {
    config: Mutex<LiveViewClientConfiguration>,
}

#[uniffi::export(async_runtime = "tokio")]
impl LiveViewClientBuilder {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            config: Default::default(),
        }
    }

    /// Provides the [LiveViewClient] with a way to store Cookies, and potentially other
    /// user session data like settings.
    pub fn set_persistence_provider(&self, provider: Box<dyn SecurePersistentStore>) {
        let mut config = self.config.lock().unwrap();
        config.persistence_provider = Some(provider.into());
    }

    /// Provides the [LiveViewClient] with a way to store Cookies, and potentially other
    /// user session data like settings.
    pub fn set_live_channel_event_handler(&self, handler: Box<dyn NetworkEventHandler>) {
        let mut config = self.config.lock().unwrap();
        config.live_channel_handler = Some(handler.into());
    }

    /// The [DocumentChangeHandler] here will be called whenever a diff event
    /// applies a change to the document that is being observed. By default,
    /// no events will be emitted
    pub fn set_patch_handler(&self, handler: Box<dyn DocumentChangeHandler>) {
        let mut config = self.config.lock().unwrap();
        config.patch_handler = Some(handler.into());
    }

    /// This is an endpoint intended for client developers to instrument navigation and
    /// store view state. By default it permits all navigation.
    pub fn set_navigation_handler(&self, handler: Box<dyn NavEventHandler>) {
        let mut config = self.config.lock().unwrap();
        config.navigation_handler = Some(handler.into());
    }

    /// Set the log filter level.
    ///
    /// By Default the log filter is set to [LogLevel::Info]
    pub fn set_log_level(&self, level: LogLevel) {
        let mut config = self.config.lock().unwrap();
        config.log_level = level;
    }

    /// Set the time out for establishign the initial http connection in milliseconds.
    ///
    /// By default the timeout is 30 seconds.
    pub fn set_dead_render_timeout_ms(&self, timeout: u64) {
        let mut config = self.config.lock().unwrap();
        config.dead_render_timeout = timeout;
    }

    /// Set the time out for awaiting responses from the websocket in milliseconds.
    ///
    /// By default the timeout is 5 seconds.
    pub fn set_websocket_timeout_ms(&self, timeout: u64) {
        let mut config = self.config.lock().unwrap();
        config.websocket_timeout = timeout;
    }

    /// Sets the '_format' arg set upon fetching the dead render and upon
    /// establishing the websocket connection.
    ///
    /// On android this defaults to [Platform::Jetpack], on apple vendored
    /// devices this defaults to [Platform::Swiftui], everywhere else it
    /// defaults to "unknown", which will cause a connection failure.
    pub fn set_format(&self, format: Platform) {
        let mut config = self.config.lock().unwrap();
        config.format = format;
    }

    /// Returns the current log level setting.
    pub fn log_level(&self) -> LogLevel {
        let config = self.config.lock().unwrap();
        config.log_level
    }

    /// Returns the current dead render timeout in milliseconds.
    pub fn dead_render_timeout(&self) -> u64 {
        let config = self.config.lock().unwrap();
        config.dead_render_timeout
    }

    /// Returns the current websocket timeout in milliseconds.
    pub fn websocket_timeout(&self) -> u64 {
        let config = self.config.lock().unwrap();
        config.websocket_timeout
    }

    /// Returns the current platform format setting.
    pub fn format(&self) -> Platform {
        let config = self.config.lock().unwrap();
        config.format.clone()
    }

    /// Attempt to establish a new, connected [LiveViewClient] with the param set above
    pub async fn connect(
        &self,
        url: String,
        opts: ClientConnectOpts,
    ) -> Result<LiveViewClient, LiveSocketError> {
        let config = self.config.lock().unwrap().clone();
        let inner = LiveViewClientInner::initial_connect(config, url, opts).await?;

        Ok(LiveViewClient { inner })
    }
}

#[derive(uniffi::Object)]
pub struct LiveViewClient {
    inner: LiveViewClientInner,
}

#[uniffi::export(async_runtime = "tokio")]
impl LiveViewClient {
    pub async fn reconnect(
        &self,
        url: String,
        client_opts: ClientConnectOpts,
    ) -> Result<(), LiveSocketError> {
        let opts = ConnectOpts {
            headers: client_opts.headers,
            body: client_opts.request_body,
            method: client_opts.method,
            ..Default::default()
        };

        self.inner
            .reconnect(url, opts, client_opts.join_params)
            .await?;

        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), LiveSocketError> {
        self.inner.disconnect().await?;
        Ok(())
    }

    pub async fn upload_files(&self, files: Vec<Arc<LiveFile>>) -> Result<(), LiveSocketError> {
        let chan = self.inner.channel()?;
        let futs = files.iter().map(|file| chan.upload_file(file));
        try_join_all(futs).await?;

        Ok(())
    }

    pub async fn post_form(
        &self,
        url: String,
        form: HashMap<String, String>,
        join_params: Option<HashMap<String, JSON>>,
        mut headers: Option<HashMap<String, String>>,
    ) -> Result<(), LiveSocketError> {
        let form_data = serde_urlencoded::to_string(form)?;

        let header_map = headers.get_or_insert_default();
        header_map.insert(
            CONTENT_TYPE.to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );
        header_map.insert(CSRF_HEADER.to_string(), self.csrf_token()?);

        let opts = ConnectOpts {
            headers,
            body: Some(form_data.into_bytes()),
            method: Some(Method::Post),
            timeout_ms: 30_000, // Actually unused, should remove at one point
        };

        self.inner.reconnect(url, opts, join_params).await?;
        Ok(())
    }

    pub fn set_log_level(&self, level: LogLevel) {
        self.inner.set_log_level(level)
    }

    pub fn channel(&self) -> LiveViewClientChannel {
        let inner = self.inner.create_channel();
        LiveViewClientChannel { inner }
    }
}

// Navigation-related functionality ported from LiveSocket
#[uniffi::export(async_runtime = "tokio")]
impl LiveViewClient {
    /// Navigate to `url` with behavior and metadata specified in `opts`.
    pub async fn navigate(
        &self,
        url: String,
        opts: NavOptions,
    ) -> Result<HistoryId, LiveSocketError> {
        self.inner.navigate(url, opts).await
    }

    /// Dispose of the current channel and remount the view. Replaces the current view
    /// event data with the bytes in `info`
    pub async fn reload(&self, opts: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        self.inner.reload(opts).await
    }

    /// Navigates back one step in the history stack.
    /// This function fails if there are no items in history.
    pub async fn back(&self, opts: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        self.inner.back(opts).await
    }

    /// Navigates back one step in the history stack.
    /// This function fails if there are no items ahead of this one in history.
    pub async fn forward(&self, opts: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        self.inner.forward(opts).await
    }

    /// Navigates to the entry with `id`. Retaining the state of the current history stack.
    /// This function fails if the entry has been removed.
    pub async fn traverse_to(
        &self,
        id: HistoryId,
        opts: NavActionOptions,
    ) -> Result<HistoryId, LiveSocketError> {
        self.inner.traverse_to(id, opts).await
    }

    pub fn can_go_back(&self) -> bool {
        self.inner.can_go_back()
    }

    pub fn can_go_forward(&self) -> bool {
        self.inner.can_go_forward()
    }

    pub fn can_traverse_to(&self, id: HistoryId) -> bool {
        self.inner.can_traverse_to(id)
    }

    /// Returns a list of all History entries currently tracked by the
    /// navigation context. There are no guarantees about the position of the
    /// current element in this list.
    pub fn get_entries(&self) -> Vec<NavHistoryEntry> {
        self.inner.get_entries()
    }

    /// Returns the current history entry, Should virtually never return a nullish
    /// value unless a connection error has occurred and not been properly recovered from.
    pub fn current(&self) -> Option<NavHistoryEntry> {
        self.inner.current()
    }
}

#[cfg_attr(not(target_family = "wasm"), uniffi::export)]
impl LiveViewClient {
    pub fn get_phx_upload_id(&self, phx_target_name: &str) -> Result<String, LiveSocketError> {
        self.inner.get_phx_upload_id(phx_target_name)
    }

    // TODO: the live reload channel should not be a user concern. It can appear in
    // an error page, as well as a successful dead render, the client config should have a callback handler
    // which any given live reload channel can use
    pub fn live_reload_channel(&self) -> Result<Option<Arc<LiveChannel>>, LiveSocketError> {
        self.inner.live_reload_channel()
    }

    pub fn join_url(&self) -> Result<String, LiveSocketError> {
        self.inner.join_url()
    }

    pub fn join_payload(&self) -> Result<Payload, LiveSocketError> {
        self.inner.join_payload()
    }

    pub fn csrf_token(&self) -> Result<String, LiveSocketError> {
        self.inner.csrf_token()
    }

    pub fn dead_render(&self) -> Result<Arc<ffi::Document>, LiveSocketError> {
        Ok(Arc::new(self.inner.dead_render()?.into()))
    }

    pub fn document(&self) -> Result<ffi::Document, LiveSocketError> {
        self.inner.document()
    }

    pub fn style_urls(&self) -> Result<Vec<String>, LiveSocketError> {
        self.inner.style_urls()
    }

    pub fn status(&self) -> Result<SocketStatus, LiveSocketError> {
        self.inner.status()
    }
}

#[derive(uniffi::Object)]
/// A thin message sending interface that will
/// send messages through the current websocket.
pub struct LiveViewClientChannel {
    inner: inner::LiveViewClientChannel,
}

#[uniffi::export(async_runtime = "tokio")]
impl LiveViewClientChannel {
    pub async fn call(
        &self,
        event_name: String,
        payload: Payload,
    ) -> Result<Payload, LiveSocketError> {
        self.inner.call(event_name, payload).await
    }

    pub async fn cast(&self, event_name: String, payload: Payload) {
        self.inner.cast(event_name, payload).await
    }
}
