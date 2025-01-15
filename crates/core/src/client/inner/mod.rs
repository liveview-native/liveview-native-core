mod channel_init;
mod cookie_store;
mod event_loop;
mod logging;
mod navigation;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use channel_init::*;
use cookie_store::PersistentCookieStore;
use event_loop::EventLoop;
pub(crate) use event_loop::LiveViewClientChannel;
use futures::future::try_join_all;
use log::debug;
use logging::*;
use navigation::NavCtx;
use phoenix_channels_client::{Payload, Socket, SocketStatus, JSON};
use reqwest::{redirect::Policy, Client, Url};

use super::{ClientConnectOpts, LiveViewClientConfiguration, LogLevel};
use crate::{
    callbacks::*,
    dom::{ffi::Document as FFIDocument, Document},
    error::LiveSocketError,
    live_socket::{
        navigation::{NavActionOptions, NavOptions},
        ConnectOpts, LiveChannel, LiveFile, SessionData,
    },
};

pub(crate) struct LiveViewClientState {
    /// Manages navigation state, events
    config: LiveViewClientConfiguration,
    /// A book keeping context for navigation events.
    nav_ctx: Mutex<NavCtx>,
    /// HTTP client used to request dead renders.
    http_client: Client,
    socket: Mutex<Arc<Socket>>,
    /// The main channel which passes user interaction events
    /// and receives changes to the "dom"
    liveview_channel: Mutex<Arc<LiveChannel>>,
    /// In debug mode LiveView has a debug channel which sends
    /// asset update events, this is derived from an iframe on the page
    /// which also may be present on errored out connections.
    livereload_channel: Mutex<Option<Arc<LiveChannel>>>,
    /// Data acquired from the dead render, should only change between
    /// reconnects.
    session_data: Mutex<SessionData>,
    /// Responsible for holding cookies and serializing them to disk
    cookie_store: Arc<PersistentCookieStore>,
}

pub struct LiveViewClientInner {
    state: Arc<LiveViewClientState>,
    event_loop: EventLoop,
}

// First implement the accessor methods on LiveViewClientInner
impl LiveViewClientInner {
    pub async fn initial_connect(
        config: LiveViewClientConfiguration,
        url: String,
        client_opts: ClientConnectOpts,
    ) -> Result<Self, LiveSocketError> {
        let state = LiveViewClientState::initial_connect(config, url, client_opts).await?;
        let state = Arc::new(state);
        let event_loop = EventLoop::new(state.clone());
        Ok(Self { state, event_loop })
    }

    pub(crate) async fn reconnect(
        &self,
        url: String,
        opts: ConnectOpts,
        join_params: Option<HashMap<String, JSON>>,
    ) -> Result<(), LiveSocketError> {
        self.state.reconnect(url, opts, join_params).await?;
        self.event_loop.replace_livechannel();
        Ok(())
    }

    pub async fn upload_files(&self, files: Vec<Arc<LiveFile>>) -> Result<(), LiveSocketError> {
        let chan = self.channel()?;
        let futs = files.iter().map(|file| chan.upload_file(file));
        try_join_all(futs).await?;

        Ok(())
    }

    pub fn get_phx_upload_id(&self, phx_target_name: &str) -> Result<String, LiveSocketError> {
        self.state
            .liveview_channel
            .try_lock()?
            .get_phx_upload_id(phx_target_name)
    }

    pub fn channel(&self) -> Result<Arc<LiveChannel>, LiveSocketError> {
        Ok(self.state.liveview_channel.try_lock()?.clone())
    }

    // TODO: a live reload channel is distinct from a live channel in a couple important
    // ways, it should be it's own struct.
    pub fn live_reload_channel(&self) -> Result<Option<Arc<LiveChannel>>, LiveSocketError> {
        Ok(self.state.livereload_channel.try_lock()?.clone())
    }

    pub fn join_url(&self) -> Result<String, LiveSocketError> {
        Ok(self.state.session_data.try_lock()?.url.to_string())
    }

    pub fn csrf_token(&self) -> Result<String, LiveSocketError> {
        Ok(self.state.session_data.try_lock()?.csrf_token.clone())
    }

    /// Returns the current document state.
    pub fn document(&self) -> Result<FFIDocument, LiveSocketError> {
        Ok(self.state.liveview_channel.try_lock()?.document())
    }

    /// Returns the state of the document upon the initial websocket connection.
    pub fn join_document(&self) -> Result<Document, LiveSocketError> {
        self.state.liveview_channel.try_lock()?.join_document()
    }

    /// To establish the websocket connection, the client depends on an initial HTTP
    /// request pull an html document and extract several pieces of meta data from it.
    /// This function returns that initial document.
    pub fn dead_render(&self) -> Result<Document, LiveSocketError> {
        Ok(self.state.session_data.try_lock()?.dead_render.clone())
    }

    pub fn style_urls(&self) -> Result<Vec<String>, LiveSocketError> {
        Ok(self.state.session_data.try_lock()?.style_urls.clone())
    }

    pub fn status(&self) -> Result<SocketStatus, LiveSocketError> {
        Ok(self.state.socket.try_lock()?.status())
    }

    pub async fn navigate(
        &self,
        url: String,
        opts: NavOptions,
    ) -> Result<HistoryId, LiveSocketError> {
        let res = self.state.navigate(url, opts).await?;
        self.event_loop.replace_livechannel();
        Ok(res)
    }

    pub async fn reload(&self, info: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        let res = self.state.reload(info).await?;
        self.event_loop.replace_livechannel();
        Ok(res)
    }

    pub async fn back(&self, info: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        let res = self.state.back(info).await?;
        self.event_loop.replace_livechannel();
        Ok(res)
    }

    pub async fn forward(&self, info: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        let res = self.state.forward(info).await?;
        self.event_loop.replace_livechannel();
        Ok(res)
    }

    pub async fn traverse_to(
        &self,
        id: HistoryId,
        info: NavActionOptions,
    ) -> Result<HistoryId, LiveSocketError> {
        let res = self.state.traverse_to(id, info).await?;
        self.event_loop.replace_livechannel();
        Ok(res)
    }

    pub fn can_go_back(&self) -> bool {
        self.state.can_go_back()
    }

    pub fn can_go_forward(&self) -> bool {
        self.state.can_go_forward()
    }

    pub fn can_traverse_to(&self, id: HistoryId) -> bool {
        self.state.can_traverse_to(id)
    }

    pub fn get_entries(&self) -> Vec<NavHistoryEntry> {
        self.state.get_entries()
    }

    pub fn current(&self) -> Option<NavHistoryEntry> {
        self.state.current()
    }

    pub fn create_channel(&self) -> LiveViewClientChannel {
        self.event_loop.create_handle()
    }

    pub fn set_log_level(&self, level: LogLevel) {
        set_log_level(level)
    }
}

impl LiveViewClientState {
    /// The first connection and initialization, fetches the dead render and opens the default channel.
    pub async fn initial_connect(
        config: LiveViewClientConfiguration,
        url: String,
        client_opts: ClientConnectOpts,
    ) -> Result<Self, LiveSocketError> {
        init_log(config.log_level);
        debug!("Initializing LiveViewClient.");
        debug!("Configuration: {config:?}");

        let cookie_store: Arc<_> =
            PersistentCookieStore::new(config.persistence_provider.clone()).into();

        let http_client = Client::builder()
            .cookie_provider(cookie_store.clone())
            .redirect(Policy::none())
            .build()
            .expect("Failed to build HTTP client");

        let url = Url::parse(&url)?;
        let format = config.format.to_string();

        let mut opts = ConnectOpts::default();
        opts.headers = client_opts.headers;

        debug!("Retrieving session data from: {url:?}");
        let session_data = SessionData::request(&url, &format, opts, http_client.clone()).await?;

        let cookies = cookie_store.get_cookie_list(&url);

        let websocket_url = session_data.get_live_socket_url()?;

        let session_data = Mutex::new(session_data);

        log::info!("Initiating Websocket connection: {websocket_url:?} , cookies: {cookies:?}");
        let socket = Socket::spawn(websocket_url, cookies.clone()).await?;
        let socket = Mutex::new(socket);

        let ws_timeout = Duration::from_millis(config.websocket_timeout);
        debug!("Joining liveview Channel");
        let liveview_channel = join_liveview_channel(
            &socket,
            &session_data,
            &client_opts.join_params,
            None,
            ws_timeout,
        )
        .await?;

        if let Some(handler) = &config.patch_handler {
            liveview_channel
                .document
                .arc_set_event_handler(handler.clone())
        }

        let livereload_channel = if session_data.try_lock()?.has_live_reload {
            debug!("Joining liveReload Channel");
            join_livereload_channel(&config, &socket, &session_data, cookies)
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
            cookie_store,
        })
    }

    /// Reconnect the websocket at the given url
    pub async fn reconnect(
        &self,
        url: String,
        opts: ConnectOpts,
        join_params: Option<HashMap<String, JSON>>,
    ) -> Result<(), LiveSocketError> {
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

        let cookies = self.cookie_store.get_cookie_list(&url);

        debug!("Initiating new websocket connection: {websocket_url:?}");
        let socket = Socket::spawn(websocket_url, cookies.clone()).await?;

        let old_socket = self.socket.try_lock()?.clone();
        let _ = old_socket.disconnect().await;
        *self.socket.try_lock()? = socket;

        *self.session_data.try_lock()? = new_session;
        let ws_timeout = Duration::from_millis(self.config.websocket_timeout);
        debug!("Rejoining liveview channel");
        let new_channel = join_liveview_channel(
            &self.socket,
            &self.session_data,
            &join_params,
            None,
            ws_timeout,
        )
        .await?;

        if let Some(handler) = &self.config.patch_handler {
            new_channel.document.arc_set_event_handler(handler.clone());
        }

        *self.liveview_channel.try_lock()? = new_channel;
        let has_reload = self.session_data.try_lock()?.has_live_reload;

        if has_reload {
            debug!("Rejoining livereload channel");
            let new_livereload =
                join_livereload_channel(&self.config, &self.socket, &self.session_data, cookies)
                    .await?;
            *self.livereload_channel.try_lock()? = Some(new_livereload);
        }

        Ok(())
    }
}

impl LiveViewClientState {
    const RETRY_REASONS: &[&str] = &["stale", "unauthorized"];
    async fn try_nav(
        &self,
        additional_params: &Option<HashMap<String, JSON>>,
    ) -> Result<(), LiveSocketError> {
        let current = self
            .nav_ctx
            .try_lock()?
            .current()
            .ok_or(LiveSocketError::NavigationImpossible)?;

        let ws_timeout = Duration::from_millis(self.config.websocket_timeout);
        match join_liveview_channel(
            &self.socket,
            &self.session_data,
            additional_params,
            Some(current.url.clone()),
            ws_timeout,
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

                let channel = join_liveview_channel(
                    &self.socket,
                    &self.session_data,
                    additional_params,
                    None,
                    ws_timeout,
                )
                .await?;

                if let Some(event_handler) = &self.config.patch_handler {
                    channel
                        .document
                        .arc_set_event_handler(event_handler.clone())
                }

                let chan = &self.liveview_channel.try_lock()?.channel.clone();
                chan.leave().await?;

                *self.liveview_channel.try_lock()? = channel;
                Ok(())
            }
            Ok(channel) => {
                let chan = &self.liveview_channel.try_lock()?.channel.clone();
                chan.leave().await?;

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

    async fn try_nav_outer<F>(
        &self,
        additional_params: &Option<HashMap<String, JSON>>,
        nav_action: F,
    ) -> Result<HistoryId, LiveSocketError>
    where
        F: FnOnce(&mut NavCtx) -> Option<HistoryId>,
    {
        let new_id = {
            let mut nav_ctx = self.nav_ctx.try_lock()?;
            nav_action(&mut nav_ctx)
        };

        match new_id {
            Some(id) => {
                self.try_nav(additional_params).await?;
                Ok(id)
            }
            None => Err(LiveSocketError::NavigationImpossible),
        }
    }

    pub async fn navigate(
        &self,
        url: String,
        mut opts: NavOptions,
    ) -> Result<HistoryId, LiveSocketError> {
        let url = Url::parse(&url)?;
        self.try_nav_outer(&opts.join_params.take(), |ctx| {
            ctx.navigate(url, opts, true)
        })
        .await
    }

    pub async fn reload(&self, opts: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        self.try_nav_outer(&opts.join_params, |ctx| {
            ctx.reload(opts.extra_event_info, true)
        })
        .await
    }
    pub async fn back(&self, opts: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        self.try_nav_outer(&opts.join_params, |ctx| {
            ctx.back(opts.extra_event_info, true)
        })
        .await
    }

    pub async fn forward(&self, opts: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        self.try_nav_outer(&opts.join_params, |ctx| {
            ctx.forward(opts.extra_event_info, true)
        })
        .await
    }

    pub async fn traverse_to(
        &self,
        id: HistoryId,
        opts: NavActionOptions,
    ) -> Result<HistoryId, LiveSocketError> {
        self.try_nav_outer(&opts.join_params, |ctx| {
            ctx.traverse_to(id, opts.extra_event_info, true)
        })
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
}
