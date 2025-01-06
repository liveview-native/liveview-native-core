mod config;
mod cookie_store;
mod inner;
mod logging;

#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use config::*;
use inner::LiveViewClientInner;
use phoenix_channels_client::{SocketStatus, JSON};

use crate::{
    dom::{
        ffi::{self},
        DocumentChangeHandler,
    },
    live_socket::{
        navigation::{HistoryId, NavEventHandler, NavHistoryEntry, NavOptions},
        LiveChannel, LiveSocket, LiveSocketError,
    },
    persistence::SecurePersistentStore,
};

#[derive(uniffi::Object)]
pub struct LiveViewClientBuilder {
    config: Mutex<LiveViewClientConfiguration>,
}

#[cfg_attr(not(target_family = "wasm"), uniffi::export(async_runtime = "tokio"))]
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

        Ok(LiveViewClient {
            inner: inner.into(),
        })
    }
}

#[derive(uniffi::Object)]
pub struct LiveViewClient {
    inner: Mutex<LiveViewClientInner>,
}

#[cfg_attr(not(target_family = "wasm"), uniffi::export(async_runtime = "tokio"))]
impl LiveViewClient {
    pub fn set_log_level(&self, level: LogLevel) {
        logging::set_log_level(level)
    }

    pub async fn connect(&self, url: String) -> Result<(), LiveSocketError> {
        todo!()
    }

    pub async fn post_form(&self, form: Form, url: String) -> Result<(), LiveSocketError> {
        todo!()
    }
}

// Navigation-related functionality ported from LiveSocket
#[cfg_attr(not(target_family = "wasm"), uniffi::export(async_runtime = "tokio"))]
impl LiveViewClient {
    pub async fn navigate(&self, url: String, opts: NavOptions) -> Result<(), LiveSocketError> {
        todo!()
    }

    pub async fn reload(&self, info: Option<Vec<u8>>) -> Result<(), LiveSocketError> {
        todo!()
    }

    pub async fn back(&self, info: Option<Vec<u8>>) -> Result<(), LiveSocketError> {
        todo!()
    }

    pub async fn forward(&self, info: Option<Vec<u8>>) -> Result<(), LiveSocketError> {
        todo!()
    }

    pub async fn traverse_to(
        &self,
        id: HistoryId,
        info: Option<Vec<u8>>,
    ) -> Result<(), LiveSocketError> {
        todo!()
    }

    pub fn can_go_back(&self) -> bool {
        todo!()
    }

    pub fn can_go_forward(&self) -> bool {
        todo!()
    }

    pub fn can_traverse_to(&self, id: HistoryId) -> bool {
        todo!()
    }

    pub fn get_entries(&self) -> Vec<NavHistoryEntry> {
        todo!()
    }

    pub fn current(&self) -> Option<NavHistoryEntry> {
        todo!()
    }

    pub fn set_event_handler(&self, handler: Box<dyn NavEventHandler>) {
        todo!()
    }
}

// Connection and session management functionality ported from LiveSocket
#[cfg_attr(not(target_family = "wasm"), uniffi::export)]
impl LiveViewClient {
    pub fn socket(&self) -> Arc<LiveSocket> {
        todo!()
    }

    pub fn channel(&self) -> Arc<LiveChannel> {
        todo!()
    }

    pub fn live_reload_channel(&self) -> Option<Arc<LiveChannel>> {
        todo!()
    }

    pub fn join_url(&self) -> String {
        todo!()
    }

    pub fn csrf_token(&self) -> String {
        todo!()
    }

    pub fn dead_render(&self) -> Arc<ffi::Document> {
        todo!()
    }

    pub fn style_urls(&self) -> Vec<String> {
        todo!()
    }

    pub fn status(&self) -> SocketStatus {
        todo!()
    }
}
