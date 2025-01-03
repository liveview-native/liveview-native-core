mod inner;

use std::sync::{Arc, Mutex};

use inner::LiveViewClientInner;

use crate::{
    dom::DocumentChangeHandler,
    live_socket::navigation::{NavCtx, NavEventHandler},
    persistence::SecurePersistentStore,
};

#[derive(uniffi::Enum, Debug, Clone)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(uniffi::Object)]
pub struct LiveViewClient {
    inner: Mutex<LiveViewClientInner>,
    config: LiveViewClientConfiguration,
}

#[derive(uniffi::Object, Clone)]
pub struct LiveViewClientConfiguration {
    /// Provides a way to store persistent state between sessions. Used for cookies and potentially persistent settings.
    pub persistence_provider: Option<Arc<dyn SecurePersistentStore>>,
    /// Instruments the patches provided by `diff` events.
    pub patch_handler: Option<Arc<dyn DocumentChangeHandler>>,
    /// An event handler for application navigation events, this is meant for client developer use
    /// If you are looking to expose navigation event handling to the user, see the api endpoints with the
    /// `app` prefix.
    pub navigation_handler: Option<Arc<dyn NavEventHandler>>,
    /// Initial log level - defaults to [LogLevel::Info]
    pub log_level: LogLevel,
    /// Timeout when connecting to a new view.
    pub dead_render_timeout: u64,
    /// Timeout when sending messages to the server via websocket
    pub websocket_timeout: u64,
    /// The number of log lines kept in memory for the console.
    pub max_log_lines: u64,
}

#[cfg_attr(not(target_family = "wasm"), uniffi::export(async_runtime = "tokio"))]
impl LiveViewClient {
    #[uniffi::constructor]
    pub async fn new() -> Self {
        todo!()
    }

    pub async fn connect(&self) {}

    pub async fn post_form(&self) {}
}
