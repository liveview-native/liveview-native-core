use std::{collections::HashMap, sync::Arc};

use phoenix_channels_client::JSON;

use crate::callbacks::*;

#[derive(uniffi::Enum, Debug, Clone, Default, Copy)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

const SWIFTUI: &str = "swiftui";
const JETPACK: &str = "jetpack";

#[derive(uniffi::Enum, Debug, Clone)]
/// Represents one of our supported platforms.
pub enum Platform {
    Swiftui,
    Jetpack,
    Other(String),
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Swiftui => f.write_str(SWIFTUI),
            Platform::Jetpack => f.write_str(JETPACK),
            Platform::Other(o) => f.write_str(o),
        }
    }
}

impl From<String> for Platform {
    fn from(value: String) -> Self {
        match value.as_str() {
            SWIFTUI => Platform::Swiftui,
            JETPACK => Platform::Jetpack,
            _ => Platform::Other(value),
        }
    }
}

impl Default for Platform {
    fn default() -> Self {
        // this could be cfg blocks but clippy complains
        if cfg!(target_vendor = "apple") {
            Platform::Swiftui
        } else if cfg!(target_os = "android") {
            Platform::Jetpack
        } else {
            Platform::Other("undefined_format".to_string())
        }
    }
}

#[derive(Clone)]
pub struct LiveViewClientConfiguration {
    /// Instruments all server side events and changes in the current LiveChannel state, including when
    /// the channel is swapped out.
    pub live_channel_handler: Option<Arc<dyn LiveChannelEventHandler>>,
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
    /// The _format argument passed on connection.
    pub format: Platform,
    /// The additional parameters passed to the live channel on join
    pub join_params: Option<HashMap<String, JSON>>,
    /// Additional headers to pass upon fetching each dead render
    pub headers: Option<HashMap<String, String>>,
}

impl Default for LiveViewClientConfiguration {
    fn default() -> Self {
        const DEAD_RENDER_TIMEOUT_MS: u64 = 30_000;
        const WEBSOCKET_TIMEOUT_MS: u64 = 5_000;

        Self {
            live_channel_handler: None,
            persistence_provider: None,
            patch_handler: None,
            navigation_handler: None,
            log_level: LogLevel::Info,
            dead_render_timeout: DEAD_RENDER_TIMEOUT_MS,
            websocket_timeout: WEBSOCKET_TIMEOUT_MS,
            format: Platform::default(),
            join_params: None,
            headers: None,
        }
    }
}

impl std::fmt::Debug for LiveViewClientConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveViewClientConfiguration")
            .field(
                "persistence_provider",
                &self.persistence_provider.is_some().then_some("..."),
            )
            .field(
                "patch_handler",
                &self.patch_handler.is_some().then_some("..."),
            )
            .field(
                "navigation_handler",
                &self.navigation_handler.is_some().then_some("..."),
            )
            .field("log_level", &self.log_level)
            .field("dead_render_timeout", &self.dead_render_timeout)
            .field("websocket_timeout", &self.websocket_timeout)
            .field("format", &self.format)
            .finish()
    }
}
