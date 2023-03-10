use phoenix_channels_client as phoenix_channels;
use url::Url;

use super::*;

pub struct Config {
    pub(super) callbacks: Callbacks,
    pub(super) url: Url,
    pub(super) platform: Option<String>,
    pub(super) mode: NavigationMode,
    pub(super) auto_reconnect: bool,
    pub(super) live_reload: bool,
    pub(super) live_params: HashMap<String, String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            callbacks: Callbacks::default(),
            url: Url::parse("http://localhost:4000").unwrap(),
            platform: None,
            mode: NavigationMode::default(),
            auto_reconnect: true,
            live_reload: true,
            live_params: HashMap::default(),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn use_tls(&self) -> bool {
        match self.url.scheme() {
            "https" => true,
            _ => false,
        }
    }

    pub fn liveview_socket_config(&self) -> phoenix_channels::Config {
        let mut url = self.url.clone();
        url.set_scheme(if self.use_tls() { "wss" } else { "ws" })
            .unwrap();
        url.set_path("live/websocket");
        url.query_pairs_mut().extend_pairs(self.live_params.iter());
        if let Some(platform) = self.platform.as_ref() {
            url.query_pairs_mut().append_pair("_platform", platform);
        }
        phoenix_channels::Config::new(url).unwrap()
    }

    pub fn livereload_socket_config(&self) -> phoenix_channels::Config {
        let mut url = self.url.clone();
        url.set_scheme(if self.use_tls() { "wss" } else { "ws" })
            .unwrap();
        url.set_path("phoenix/live_reload/socket");
        phoenix_channels::Config::new(url).unwrap()
    }

    pub fn with_url(&mut self, url: Url) -> &mut Self {
        assert!(
            url.scheme().starts_with("http"),
            "invalid url, expected scheme to be http(s), got {}",
            url.scheme()
        );
        self.url = url;
        self
    }

    pub fn with_platform(&mut self, platform: String) -> &mut Self {
        self.platform = Some(platform);
        self
    }

    pub fn with_live_reload(&mut self, enabled: bool) -> &mut Self {
        self.live_reload = enabled;
        self
    }

    /// Set the callback to be invoked when the document associated with this client is loaded for the first time
    pub fn on_load<F>(&mut self, callback: F) -> &mut Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.callbacks.loaded.replace(Box::new(callback));
        self
    }

    /// Set the callback to be invoked when the document associated with this client is changed, requiring a re-render
    pub fn on_change<F>(&mut self, callback: F) -> &mut Self
    where
        F: Fn(NodeRef) + Send + Sync + 'static,
    {
        self.callbacks.changed.replace(Box::new(callback));
        self
    }

    /// Set the callback to be invoked when this client succesfully connects to a LiveView server
    pub fn on_connect<F>(&mut self, callback: F) -> &mut Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.callbacks.connected.replace(Box::new(callback));
        self
    }

    /// Set the callback to be invoked when this client is disconnected from a previously connected LiveView server
    pub fn on_disconnect<F>(&mut self, callback: F) -> &mut Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.callbacks.disconnected.replace(Box::new(callback));
        self
    }
}

#[derive(Default)]
pub(super) struct Callbacks {
    loaded: Option<Box<dyn Fn() + Send + Sync>>,
    changed: Option<Box<dyn Fn(NodeRef) + Send + Sync>>,
    connected: Option<Box<dyn Fn() + Send + Sync>>,
    disconnected: Option<Box<dyn Fn() + Send + Sync>>,
}
impl Callbacks {
    #[inline]
    pub fn loaded(&self) {
        if let Some(loaded) = self.loaded.as_ref() {
            loaded()
        }
    }

    #[inline]
    pub fn changed(&self, node: NodeRef) {
        if let Some(changed) = self.changed.as_ref() {
            changed(node)
        }
    }

    #[inline]
    pub fn connected(&self) {
        if let Some(connected) = self.connected.as_ref() {
            connected()
        }
    }

    #[inline]
    pub fn disconnected(&self) {
        if let Some(disconnected) = self.disconnected.as_ref() {
            disconnected()
        }
    }
}
