mod config;
mod inner;

#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use config::*;
use futures::future::try_join_all;
use inner::LiveViewClientInner;
use phoenix_channels_client::{Socket, SocketStatus, JSON};
use reqwest::header::CONTENT_TYPE;

use crate::{
    callbacks::*,
    dom::{
        ffi::{self},
        DocumentChangeHandler,
    },
    error::LiveSocketError,
    live_socket::{navigation::NavOptions, ConnectOpts, LiveChannel, Method},
};

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

    // Setters
    pub fn set_persistence_provider(&self, provider: Box<dyn SecurePersistentStore>) {
        let mut config = self.config.lock().unwrap();
        config.persistence_provider = Some(provider.into());
    }

    pub fn set_channel_join_params(&self, join_params: HashMap<String, JSON>) {
        let mut config = self.config.lock().unwrap();
        config.join_params = join_params.into();
    }

    pub fn set_patch_handler(&self, handler: Box<dyn DocumentChangeHandler>) {
        let mut config = self.config.lock().unwrap();
        config.patch_handler = Some(handler.into());
    }

    pub fn set_navigation_handler(&self, handler: Box<dyn NavEventHandler>) {
        let mut config = self.config.lock().unwrap();
        config.navigation_handler = Some(handler.into());
    }

    pub fn set_log_level(&self, level: LogLevel) {
        let mut config = self.config.lock().unwrap();
        config.log_level = level;
    }

    pub fn set_dead_render_timeout(&self, timeout: u64) {
        let mut config = self.config.lock().unwrap();
        config.dead_render_timeout = timeout;
    }

    pub fn set_websocket_timeout(&self, timeout: u64) {
        let mut config = self.config.lock().unwrap();
        config.websocket_timeout = timeout;
    }

    pub fn set_format(&self, format: Platform) {
        let mut config = self.config.lock().unwrap();
        config.format = format;
    }

    pub fn channel_join_params(&self) -> Option<HashMap<String, JSON>> {
        let config = self.config.lock().unwrap();
        config.join_params.clone()
    }

    pub fn log_level(&self) -> LogLevel {
        let config = self.config.lock().unwrap();
        config.log_level
    }

    pub fn dead_render_timeout(&self) -> u64 {
        let config = self.config.lock().unwrap();
        config.dead_render_timeout
    }

    pub fn websocket_timeout(&self) -> u64 {
        let config = self.config.lock().unwrap();
        config.websocket_timeout
    }

    pub fn format(&self) -> Platform {
        let config = self.config.lock().unwrap();
        config.format.clone()
    }

    pub async fn connect(&self, url: String) -> Result<LiveViewClient, LiveSocketError> {
        let config = self.config.lock().unwrap().clone();
        let inner = LiveViewClientInner::initial_connect(config, url).await?;

        Ok(LiveViewClient { inner })
    }
}

#[derive(uniffi::Object)]
pub struct LiveViewClient {
    inner: LiveViewClientInner,
}

#[uniffi::export(async_runtime = "tokio")]
impl LiveViewClient {
    pub fn set_log_level(&self, level: LogLevel) {
        self.inner.set_log_level(level)
    }

    pub async fn reconnect(&self, url: String) -> Result<(), LiveSocketError> {
        self.inner.reconnect(url, Default::default()).await?;
        Ok(())
    }

    pub async fn post_form(&self, form: Form, url: String) -> Result<(), LiveSocketError> {
        let form_data = serde_urlencoded::to_string(form.fields)?;

        let mut headers = HashMap::new();
        headers.insert(
            CONTENT_TYPE.to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );

        let opts = ConnectOpts {
            headers: Some(headers),
            body: Some(form_data),
            method: Some(Method::Post),
            timeout_ms: 30_000, // Actually unused
        };

        self.inner.reconnect(url, opts).await?;

        let chan = self.inner.channel()?;
        let futs = form.files.iter().map(|file| chan.upload_file(file));

        try_join_all(futs).await?;

        Ok(())
    }
}

// Navigation-related functionality ported from LiveSocket
#[uniffi::export(async_runtime = "tokio")]
impl LiveViewClient {
    pub async fn navigate(&self, url: String, opts: NavOptions) -> Result<(), LiveSocketError> {
        self.inner.navigate(url, opts).await
    }

    pub async fn reload(&self, info: Option<Vec<u8>>) -> Result<(), LiveSocketError> {
        self.inner.reload(info).await
    }

    pub async fn back(&self, info: Option<Vec<u8>>) -> Result<(), LiveSocketError> {
        self.inner.back(info).await
    }

    pub async fn forward(&self, info: Option<Vec<u8>>) -> Result<(), LiveSocketError> {
        self.inner.forward(info).await
    }

    pub async fn traverse_to(
        &self,
        id: HistoryId,
        info: Option<Vec<u8>>,
    ) -> Result<(), LiveSocketError> {
        self.inner.traverse_to(id, info).await
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

    pub fn get_entries(&self) -> Vec<NavHistoryEntry> {
        self.inner.get_entries()
    }

    pub fn current(&self) -> Option<NavHistoryEntry> {
        self.inner.current()
    }

    pub fn set_event_handler(
        &self,
        handler: Box<dyn NavEventHandler>,
    ) -> Result<(), LiveSocketError> {
        self.inner.set_event_handler(handler)
    }
}

#[cfg_attr(not(target_family = "wasm"), uniffi::export)]
impl LiveViewClient {
    // TODO: socket and channel should probably be arcswaps, if they can be,
    // This will still leave the user with a stupid amount of dangling state when they navigate...
    pub fn socket(&self) -> Result<Arc<Socket>, LiveSocketError> {
        self.inner.socket()
    }

    pub fn channel(&self) -> Result<Arc<LiveChannel>, LiveSocketError> {
        self.inner.channel()
    }

    pub fn get_phx_upload_id(&self, phx_target_name: &str) -> Result<String, LiveSocketError> {
        self.inner.get_phx_upload_id(phx_target_name)
    }

    pub fn live_reload_channel(&self) -> Result<Option<Arc<LiveChannel>>, LiveSocketError> {
        self.inner.live_reload_channel()
    }

    pub fn join_url(&self) -> Result<String, LiveSocketError> {
        self.inner.join_url()
    }

    pub fn csrf_token(&self) -> Result<String, LiveSocketError> {
        self.inner.csrf_token()
    }

    pub fn dead_render(&self) -> Result<Arc<ffi::Document>, LiveSocketError> {
        Ok(Arc::new(self.inner.dead_render()?.into()))
    }

    pub fn style_urls(&self) -> Result<Vec<String>, LiveSocketError> {
        self.inner.style_urls()
    }

    pub fn status(&self) -> Result<SocketStatus, LiveSocketError> {
        self.inner.status()
    }
}
