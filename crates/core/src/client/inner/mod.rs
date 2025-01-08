mod channel_init;
mod cookie_store;
mod logging;

use std::sync::{Arc, Mutex};

use channel_init::*;
use cookie_store::PersistentCookieStore;
use log::debug;
use logging::*;
use phoenix_channels_client::{Payload, Socket, SocketStatus, JSON};
use reqwest::{redirect::Policy, Client, Url};

use super::{LiveViewClientConfiguration, LogLevel};
use crate::{
    dom::Document,
    live_socket::{
        navigation::{HistoryId, NavCtx, NavEventHandler, NavHistoryEntry, NavOptions},
        ConnectOpts, LiveChannel, LiveSocketError, SessionData,
    },
};

pub struct LiveViewClientInner {
    /// Manages navigation state, events
    config: LiveViewClientConfiguration,
    nav_ctx: Mutex<NavCtx>,
    http_client: Client,
    socket: Mutex<Arc<Socket>>,
    liveview_channel: Mutex<Arc<LiveChannel>>,
    livereload_channel: Mutex<Option<Arc<LiveChannel>>>,
    session_data: Mutex<SessionData>,
}

impl LiveViewClientInner {
    /// The first connection and initialization, fetches the dead render and opens the default channel.
    pub async fn initial_connect(
        config: LiveViewClientConfiguration,
        url: String,
    ) -> Result<Self, LiveSocketError> {
        init_log(config.log_level);
        debug!("Initializing LiveViewClient.");
        debug!("Configuration: {config:?}");

        let cookie_store = PersistentCookieStore::new(config.persistence_provider.clone());

        let http_client = Client::builder()
            .cookie_provider(Arc::new(cookie_store))
            .redirect(Policy::none())
            .build()
            .expect("Failed to build HTTP client");

        let url = Url::parse(&url)?;
        let format = config.format.to_string();

        debug!("Retrieving session data from: {url:?}");
        let session_data =
            SessionData::request(&url, &format, Default::default(), http_client.clone()).await?;

        let session_data = Mutex::new(session_data);

        let websocket_url = session_data.try_lock()?.get_live_socket_url()?;

        let cookies = session_data.try_lock()?.cookies.clone();

        debug!("Initiating Websocket connection: {websocket_url:?}");
        let socket = Socket::spawn(websocket_url, Some(cookies)).await?;
        let socket = Mutex::new(socket);

        debug!("Joining liveview Channel");
        let liveview_channel = join_liveview_channel(&config, &socket, &session_data, None).await?;

        if let Some(handler) = &config.patch_handler {
            liveview_channel
                .document
                .arc_set_event_handler(handler.clone())
        }

        let livereload_channel = if session_data.try_lock()?.has_live_reload {
            debug!("Joining liveReload Channel");
            join_livereload_channel(&config, &socket, &session_data)
                .await?
                .into()
        } else {
            None
        };

        let mut nav_ctx = NavCtx::default();
        nav_ctx.navigate(url.clone(), NavOptions::default(), false);

        if let Some(handler) = &config.navigation_handler {
            nav_ctx.set_event_handler(handler.clone());
        }

        Ok(Self {
            config,
            http_client,
            socket,
            session_data,
            nav_ctx: nav_ctx.into(),
            liveview_channel: liveview_channel.into(),
            livereload_channel: livereload_channel.into(),
        })
    }

    pub async fn reconnect(&self, url: String, opts: ConnectOpts) -> Result<(), LiveSocketError> {
        debug!("Reestablishing connection with settings: url: {url:?}, opts: {opts:?}");

        let url = Url::parse(&url)?;
        let new_session = SessionData::request(
            &url,
            &self.config.format.to_string(),
            opts,
            self.http_client.clone(),
        )
        .await?;

        let websocket_url = new_session.get_live_socket_url()?;
        let cookies = new_session.cookies.clone();

        debug!("Initiating new websocket connection: {websocket_url:?}");
        let socket = Socket::spawn(websocket_url, Some(cookies)).await?;

        let old_socket = self.socket.try_lock()?.clone();
        let _ = old_socket.disconnect().await;
        *self.socket.try_lock()? = socket;

        *self.session_data.try_lock()? = new_session;

        debug!("Rejoining liveview channel");
        let new_channel =
            join_liveview_channel(&self.config, &self.socket, &self.session_data, None).await?;

        if let Some(handler) = &self.config.patch_handler {
            new_channel.document.arc_set_event_handler(handler.clone());
        }

        let old_channel = self.liveview_channel.try_lock()?.channel.clone();
        old_channel.leave().await?;
        *self.liveview_channel.try_lock()? = new_channel;

        if self.session_data.try_lock()?.has_live_reload {
            debug!("Rejoining livereload channel");
            let new_livereload =
                join_livereload_channel(&self.config, &self.socket, &self.session_data).await?;
            *self.livereload_channel.try_lock()? = Some(new_livereload);
        }

        Ok(())
    }

    pub fn set_log_level(&self, level: LogLevel) {
        set_log_level(level)
    }
}

impl LiveViewClientInner {
    const RETRY_REASONS: &[&str] = &["stale", "unauthorized"];
    async fn try_nav(&self) -> Result<(), LiveSocketError> {
        let current = self
            .nav_ctx
            .try_lock()?
            .current()
            .ok_or(LiveSocketError::NavigationImpossible)?;

        let chan = &self.liveview_channel.try_lock()?.channel.clone();
        chan.leave().await?;

        match join_liveview_channel(
            &self.config,
            &self.socket,
            &self.session_data,
            Some(current.url.clone()),
        )
        .await
        {
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
                    .is_none_or(|reason| !Self::RETRY_REASONS.contains(&reason.as_str()))
                {
                    return Err(LiveSocketError::JoinRejection {
                        error: Payload::JSONPayload {
                            json: JSON::Object { object },
                        },
                    });
                }

                let url = Url::parse(&current.url)?;
                let format = self.config.format.to_string();

                let new_session_data = SessionData::request(
                    &url,
                    &format,
                    Default::default(),
                    self.http_client.clone(),
                )
                .await?;

                let websocket_url = new_session_data.get_live_socket_url()?;

                let new_socket =
                    Socket::spawn(websocket_url, Some(new_session_data.cookies.clone())).await?;

                let sock = self.socket.try_lock()?.clone();
                sock.shutdown()
                    .await
                    .map_err(|e| LiveSocketError::Phoenix {
                        error: format!("{e:?}"),
                    })?;

                *self.socket.try_lock()? = new_socket;
                *self.session_data.try_lock()? = new_session_data;

                let channel =
                    join_liveview_channel(&self.config, &self.socket, &self.session_data, None)
                        .await?;

                if let Some(event_handler) = &self.config.patch_handler {
                    channel
                        .document
                        .arc_set_event_handler(event_handler.clone())
                }

                *self.liveview_channel.try_lock()? = channel;
                Ok(())
            }
            Ok(channel) => {
                if let Some(event_handler) = &self.config.patch_handler {
                    channel
                        .document
                        .arc_set_event_handler(event_handler.clone())
                }

                *self.liveview_channel.try_lock()? = channel;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    async fn try_nav_outer<F>(&self, nav_action: F) -> Result<(), LiveSocketError>
    where
        F: FnOnce(&mut NavCtx) -> Option<HistoryId>,
    {
        let new_id = {
            let mut nav_ctx = self.nav_ctx.try_lock()?;
            nav_action(&mut nav_ctx)
        };

        if new_id.is_none() {
            return Err(LiveSocketError::NavigationImpossible);
        }

        self.try_nav().await
    }

    pub async fn navigate(&self, url: String, opts: NavOptions) -> Result<(), LiveSocketError> {
        let url = Url::parse(&url)?;
        self.try_nav_outer(|ctx| ctx.navigate(url, opts, true))
            .await
    }

    pub async fn reload(&self, info: Option<Vec<u8>>) -> Result<(), LiveSocketError> {
        self.try_nav_outer(|ctx| ctx.reload(info, true)).await
    }

    pub async fn back(&self, info: Option<Vec<u8>>) -> Result<(), LiveSocketError> {
        self.try_nav_outer(|ctx| ctx.back(info, true)).await
    }

    pub async fn forward(&self, info: Option<Vec<u8>>) -> Result<(), LiveSocketError> {
        self.try_nav_outer(|ctx| ctx.forward(info, true)).await
    }

    pub async fn traverse_to(
        &self,
        id: HistoryId,
        info: Option<Vec<u8>>,
    ) -> Result<(), LiveSocketError> {
        self.try_nav_outer(|ctx| ctx.traverse_to(id, info, true))
            .await
    }

    pub fn can_go_back(&self) -> bool {
        self.nav_ctx
            .try_lock()
            .map(|ctx| ctx.can_go_back())
            .unwrap_or(false)
    }

    pub fn can_go_forward(&self) -> bool {
        self.nav_ctx
            .try_lock()
            .map(|ctx| ctx.can_go_forward())
            .unwrap_or(false)
    }

    pub fn can_traverse_to(&self, id: HistoryId) -> bool {
        self.nav_ctx
            .try_lock()
            .map(|ctx| ctx.can_traverse_to(id))
            .unwrap_or(false)
    }

    pub fn get_entries(&self) -> Vec<NavHistoryEntry> {
        self.nav_ctx
            .try_lock()
            .map(|ctx| ctx.entries())
            .unwrap_or_default()
    }

    pub fn current(&self) -> Option<NavHistoryEntry> {
        self.nav_ctx.try_lock().ok().and_then(|ctx| ctx.current())
    }

    pub fn set_event_handler(
        &self,
        handler: Box<dyn NavEventHandler>,
    ) -> Result<(), LiveSocketError> {
        self.nav_ctx.try_lock()?.set_event_handler(handler.into());
        Ok(())
    }
}

// First implement the accessor methods on LiveViewClientInner
impl LiveViewClientInner {
    pub fn socket(&self) -> Result<Arc<Socket>, LiveSocketError> {
        Ok(self.socket.try_lock()?.clone())
    }

    pub fn get_phx_upload_id(&self, phx_target_name: &str) -> Result<String, LiveSocketError> {
        self.liveview_channel
            .try_lock()?
            .get_phx_upload_id(phx_target_name)
    }

    pub fn channel(&self) -> Result<Arc<LiveChannel>, LiveSocketError> {
        Ok(self.liveview_channel.try_lock()?.clone())
    }

    pub fn live_reload_channel(&self) -> Result<Option<Arc<LiveChannel>>, LiveSocketError> {
        Ok(self.livereload_channel.try_lock()?.clone())
    }

    pub fn join_url(&self) -> Result<String, LiveSocketError> {
        Ok(self.session_data.try_lock()?.url.to_string())
    }

    pub fn csrf_token(&self) -> Result<String, LiveSocketError> {
        Ok(self.session_data.try_lock()?.csrf_token.clone())
    }

    pub fn dead_render(&self) -> Result<Document, LiveSocketError> {
        Ok(self.session_data.try_lock()?.dead_render.clone())
    }

    pub fn style_urls(&self) -> Result<Vec<String>, LiveSocketError> {
        Ok(self.session_data.try_lock()?.style_urls.clone())
    }

    pub fn status(&self) -> Result<SocketStatus, LiveSocketError> {
        Ok(self.socket.try_lock()?.status())
    }
}
