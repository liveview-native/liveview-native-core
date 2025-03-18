mod channel_init;
mod cookie_store;
mod event_loop;
mod logging;
mod navigation;
mod readonly_mutex;

pub use navigation::NavigationError;

use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};

use channel_init::*;
use cookie_store::PersistentCookieStore;
use event_loop::{ClientMessage, ConnectedClientMessage, EventLoop};
use futures::FutureExt;
use log::{debug, warn};
use logging::*;
use navigation::NavCtx;
use phoenix_channels_client::{
    Event, EventPayload, Events, EventsError, Payload, ReconnectStrategy, Socket,
    SocketStatus, SocketStatuses, WebSocketError, JSON,
};
use readonly_mutex::ReadOnlyMutex;
use reqwest::{redirect::Policy, Client as HttpClient, Url};
use tokio::{
    select,
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use super::{ClientConnectOpts, LiveViewClientConfiguration, LogLevel, Status};
use crate::{
    callbacks::{*},
    client::StrategyAdapter,
    dom::{
        ffi::{self, Document as FFIDocument},
        AttributeName, AttributeValue, Document, Selector,
    },
    error::{ConnectionError, LiveSocketError},
    live_socket::{
        navigation::{NavActionOptions, NavOptions},
        ConnectOpts, LiveChannel, LiveFile, SessionData,
    },
};

pub struct LiveViewClientInner {
    /// Shared state between the background event loop and the client handle
    status: ReadOnlyMutex<ClientStatus>,
    /// A book keeping context for navigation events.
    nav_ctx: Mutex<NavCtx>,
    /// A channel to send user action messages to the background listener
    msg_tx: mpsc::UnboundedSender<ClientMessage>,
    /// A token which causes the backed to attempt a graceful shutdown - freeing network resources if
    /// a graceful disconnect is impossible
    cancellation_token: CancellationToken,
}

impl Drop for LiveViewClientInner {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
    }
}

// First implement the accessor methods on LiveViewClientInner
impl LiveViewClientInner {
    /// Create a new LiveViewClient, this will only fail if you cannot create an HTTP client.
    pub async fn new(
        config: LiveViewClientConfiguration,
        url: String,
        client_opts: ClientConnectOpts,
    ) -> Result<Self, LiveSocketError> {
        init_log(config.log_level);
        debug!("Initializing LiveViewClient.");
        debug!("LiveViewCore Version: {}", env!("CARGO_PKG_VERSION"));

        if config.network_event_handler.is_none() {
            warn!("Network event handler is not set: You will not be able to instrument events such as view reloads and server push events.")
        }

        if config.navigation_handler.is_none() {
            warn!("Navigation handler is not set: you will not be able to instrument internal and external calls to `navigate`, `traverse`, `back` and `forward`.")
        }

        debug!("Configuration: {config:?}");

        let cookie_store: Arc<_> =
            PersistentCookieStore::new(config.persistence_provider.clone()).into();

        let http_client = HttpClient::builder()
            .cookie_provider(cookie_store.clone())
            .redirect(Policy::none())
            .build()?;

        let mut nav_ctx = NavCtx::default();

        // this failure will be reproduced inside the event loop
        // if the URL fails to parse
        if let Ok(url) = Url::parse(&url) {
            // the first navigate to a valid url should be infallible unless
            // prevented by a user, in which case we will just disconnect internally
            let _ = nav_ctx.navigate(url, NavOptions::default(), false);
        }

        if let Some(nav_handler) = config.navigation_handler.clone() {
            nav_ctx.set_event_handler(nav_handler);
        }

        let EventLoop {
            msg_tx,
            cancellation_token,
            status,
        } = EventLoop::new(config, &url, cookie_store, http_client, client_opts).await;

        let out = Self {
            status,
            msg_tx,
            cancellation_token,
            nav_ctx: nav_ctx.into(),
        };

        Ok(out)
    }

    pub(crate) async fn reconnect(
        &self,
        url: String,
        opts: ConnectOpts,
        join_params: Option<HashMap<String, JSON>>,
    ) -> Result<(), LiveSocketError> {
        self.msg_tx
            .send(ClientMessage::Reconnect {
                url,
                opts,
                join_params,
            })
            .map_err(|_| LiveSocketError::DisconnectionError)
    }

    pub(crate) async fn disconnect(&self) -> Result<(), LiveSocketError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.msg_tx.send(ClientMessage::Disconnect { response_tx });
        response_rx
            .await
            .map_err(|_| LiveSocketError::DisconnectionError)?
    }

    pub(crate) fn shutdown(&self) {
        self.cancellation_token.cancel()
    }

    pub async fn upload_file(&self, file: Arc<LiveFile>) -> Result<(), LiveSocketError> {
        todo!();
        Ok(())
    }

    pub fn status(&self) -> Status {
        match &*self.status.read() {
            ClientStatus::Disconnected => Status::Disconnected,
            ClientStatus::Connecting => Status::Connecting,
            ClientStatus::Reconnecting => Status::Reconnecting,
            ClientStatus::Connected(_) => Status::Connected,
            ClientStatus::FatalError { error } => Status::FatalError {
                error: error.clone(),
            },
        }
    }

    pub fn get_phx_upload_id(&self, phx_target_name: &str) -> Result<String, LiveSocketError> {
        let state = self.status.read();
        let con = state.as_connected()?;

        let node_ref = con
            .document
            .inner()
            .lock()
            .expect("lock poison!")
            .select(Selector::And(
                Box::new(Selector::Attribute(AttributeName {
                    namespace: None,
                    name: "data-phx-upload-ref".into(),
                })),
                Box::new(Selector::AttributeValue(
                    AttributeName {
                        namespace: None,
                        name: "name".into(),
                    },
                    AttributeValue::String(phx_target_name.into()),
                )),
            ))
            .nth(0);

        let upload_id = node_ref
            .map(|node_ref| con.document.get(node_ref.into()))
            .and_then(|input_div| {
                input_div
                    .attributes()
                    .iter()
                    .filter(|attr| attr.name.name == "id")
                    .map(|attr| attr.value.clone())
                    .collect::<Option<String>>()
            })
            .ok_or(LiveSocketError::NoInputRefInDocument)?;

        Ok(upload_id)
    }

    pub fn join_url(&self) -> Result<String, LiveSocketError> {
        let state = self.status.read();
        let con = state.as_connected()?;
        Ok(con.session_data.url.to_string())
    }

    pub fn csrf_token(&self) -> Result<String, LiveSocketError> {
        let state = self.status.read();
        let con = state.as_connected()?;
        Ok(con.session_data.csrf_token.clone())
    }

    /// Returns the current document state.
    pub fn document(&self) -> Result<FFIDocument, LiveSocketError> {
        let state = self.status.read();
        let con = state.as_connected()?;
        Ok(con.document.clone())
    }

    /// Returns the state of the document upon the initial websocket connection.
    pub fn join_document(&self) -> Result<Document, LiveSocketError> {
        let state = self.status.read();
        let con = state.as_connected()?;
        match &con.join_document {
            Ok(doc) => Ok(doc.clone()),
            Err(_) => Err(LiveSocketError::NoDocumentInJoinPayload),
        }
    }

    /// returns the join payload
    pub fn join_payload(&self) -> Result<Payload, LiveSocketError> {
        let state = self.status.read();
        let con = state.as_connected()?;
        Ok(con.join_payload.clone())
    }

    /// To establish the websocket connection, the client depends on an initial HTTP
    /// request pull an html document and extract several pieces of meta data from it.
    /// This function returns that initial document.
    pub fn dead_render(&self) -> Result<Document, LiveSocketError> {
        let state = self.status.read();
        let con = state.as_connected()?;
        Ok(con.session_data.dead_render.clone())
    }

    pub fn style_urls(&self) -> Result<Vec<String>, LiveSocketError> {
        let state = self.status.read();
        let con = state.as_connected()?;
        Ok(con.session_data.style_urls.clone())
    }

    // not for internal use
    pub fn navigate(&self, url: String, opts: NavOptions) -> Result<HistoryId, LiveSocketError> {
        let status = self.status.read();
        let con = status.as_connected()?;
        let parsed_url = Url::parse(&url)?;
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.navigate(parsed_url, opts.clone(), true)?;

        con.msg_tx
            .send(ConnectedClientMessage::Navigate { url, opts });

        Ok(id)
    }

    /// This does nothing but update the navigation contex
    pub fn patch(&self, url: String) -> Result<HistoryId, LiveSocketError> {
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.patch(url.clone(), true)?;
        Ok(id)
    }

    pub fn reload(&self, info: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        let status = self.status.read();
        let con = status.as_connected()?;
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.reload(info.extra_event_info.clone(), true)?;
        let current = ctx.current().ok_or(NavigationError::NoCurrentEntry)?;

        con.msg_tx.send(ConnectedClientMessage::Navigate {
            url: current.url,
            opts: info.into(),
        });

        Ok(id)
    }

    // not for internal use
    pub fn back(&self, info: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        let status = self.status.read();
        let con = status.as_connected()?;
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.back(info.extra_event_info.clone(), true)?;
        let current = ctx.current().ok_or(NavigationError::NoCurrentEntry)?;

        con.msg_tx.send(ConnectedClientMessage::Navigate {
            url: current.url,
            opts: info.into(),
        });

        Ok(id)
    }

    // not for internal use
    pub fn forward(&self, info: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        let status = self.status.read();
        let con = status.as_connected()?;
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.forward(info.extra_event_info.clone(), true)?;
        let current = ctx.current().ok_or(NavigationError::NoCurrentEntry)?;

        con.msg_tx.send(ConnectedClientMessage::Navigate {
            url: current.url,
            opts: info.into(),
        });

        Ok(id)
    }

    // not for internal use
    pub fn traverse_to(
        &self,
        id: HistoryId,
        info: NavActionOptions,
    ) -> Result<HistoryId, LiveSocketError> {
        let status = self.status.read();
        let con = status.as_connected()?;
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.traverse_to(id, info.extra_event_info.clone(), true)?;
        let current = ctx.current().ok_or(NavigationError::NoCurrentEntry)?;

        con.msg_tx.send(ConnectedClientMessage::Navigate {
            url: current.url,
            opts: info.into(),
        });

        Ok(id)
    }

    pub fn can_go_back(&self) -> bool {
        self.nav_ctx.lock().expect("Lock Poison").can_go_back()
    }

    pub fn can_go_forward(&self) -> bool {
        self.nav_ctx.lock().expect("Lock Poison").can_go_forward()
    }

    pub fn can_traverse_to(&self, id: HistoryId) -> bool {
        self.nav_ctx
            .lock()
            .expect("Lock Poison")
            .can_traverse_to(id)
    }

    pub fn get_entries(&self) -> Vec<NavHistoryEntry> {
        self.nav_ctx.lock().expect("Lock Poison").entries()
    }

    pub fn current_history_entry(&self) -> Option<NavHistoryEntry> {
        self.nav_ctx.lock().expect("Lock Poison").current()
    }

    pub async fn call(
        &self,
        event_name: String,
        payload: Payload,
    ) -> Result<Payload, LiveSocketError> {
        let _ = self.status.read().as_connected()?;
        let (response_tx, response_rx) = oneshot::channel();

        {
            let status = self.status.read();
            let con = status.as_connected()?;
            con.msg_tx.send(ConnectedClientMessage::Call {
                response_tx,
                event: Event::from_string(event_name),
                payload,
            });
        }

        let resp = response_rx.await.map_err(|e| LiveSocketError::Call {
            error: format!("{e}"),
        })??;

        Ok(resp)
    }

    pub async fn cast(&self, event_name: String, payload: Payload) -> Result<(), LiveSocketError> {
        let status = self.status.read();
        let con = status.as_connected()?;
        con.msg_tx.send(ConnectedClientMessage::Cast {
            event: Event::from_string(event_name),
            payload,
        });

        Ok(())
    }

    pub fn set_log_level(&self, level: LogLevel) {
        set_log_level(level)
    }
}

pub(crate) enum LiveViewClientState {
    Disconnected,
    Connecting {
        job: JoinHandle<Result<ConnectedClient, LiveSocketError>>,
    },
    /// TODO: call shutdown on all transitions
    Reconnecting {
        recconecting_client: ConnectedClient,
    },
    /// TODO: call shutdown on all transitions
    Connected {
        con_msg_tx: UnboundedSender<ConnectedClientMessage>,
        con_msg_rx: UnboundedReceiver<ConnectedClientMessage>,
        client: ConnectedClient,
    },
    FatalError(FatalError),
}

pub enum ConnectingClient {
    Job {
        job: JoinHandle<Result<ConnectedClient, LiveSocketError>>,
    },
    Interrupted {
        client: ConnectedClient,
    },
}

pub(crate) struct FatalError {
    error: LiveSocketError,
    livereload_channel: Option<Arc<LiveChannel>>,
    channel_events: Option<Arc<Events>>,
}

impl From<LiveSocketError> for FatalError {
    fn from(value: LiveSocketError) -> Self {
        match value {
            LiveSocketError::ConnectionError(ConnectionError {
                error_text,
                error_code,
                livereload_channel,
            }) => Self {
                error: LiveSocketError::ConnectionError(ConnectionError {
                    error_text,
                    error_code,
                    livereload_channel: None,
                }),
                channel_events: livereload_channel.clone().map(|e| e.channel.events()),
                livereload_channel,
            },
            e => Self {
                error: e,
                livereload_channel: None,
                channel_events: None,
            },
        }
    }
}

// TODO: remove arcs if possible, helps
// readers reason about lifetimes
pub(crate) struct ConnectedClient {
    document: ffi::Document,
    /// The main websocket for this page
    socket: Arc<Socket>,
    /// The main channel which passes user interaction events
    /// and receives changes to the "dom"
    liveview_channel: Arc<LiveChannel>,
    /// In debug mode LiveView has a debug channel which sends
    /// asset update events, this is derived from an iframe on the page
    /// which also may be present on errored out connections.
    livereload_channel: Option<Arc<LiveChannel>>,
    /// Data acquired from the dead render, should only change between
    /// reconnects.
    session_data: SessionData,
    /// Utility to hold all the event channels
    event_pump: EventPump,
}

struct EventPump {
    main_events: Arc<Events>,
    reload_events: Option<Arc<Events>>,
    socket_statuses: Arc<SocketStatuses>,
}

pub enum ReplyAction {
    Redirected { summary: NavigationSummary },
    DiffMerged,
    None,
}

impl ConnectedClient {
    pub async fn try_new(
        config: &LiveViewClientConfiguration,
        url: &str,
        http_client: &HttpClient,
        client_opts: ClientConnectOpts,
        cookie_store: &PersistentCookieStore,
    ) -> Result<Self, LiveSocketError> {
        log::info!("Starting new client connection: {url:?}");

        let url = Url::parse(url)?;

        let opts = ConnectOpts {
            headers: client_opts.headers,
            timeout_ms: config.dead_render_timeout,
            ..ConnectOpts::default()
        };
        let format = config.format.to_string();

        let session_data = SessionData::request(&url, &format, opts, http_client.clone()).await?;

        let cookies = cookie_store.get_cookie_list(&url);
        let websocket_url = session_data.get_live_socket_url()?;

        log::info!("Initiating Websocket connection: {websocket_url:?} , cookies: {cookies:?}");

        let adapter = config
            .socket_reconnect_strategy
            .clone()
            .map(StrategyAdapter::from)
            .map(|s| Box::new(s) as Box<dyn ReconnectStrategy>);

        let socket = Socket::spawn(websocket_url, cookies.clone(), adapter).await?;

        let ws_timeout = Duration::from_millis(config.websocket_timeout);

        socket.connect(ws_timeout).await?;

        let cleanup_and_return = async |err: LiveSocketError, socket: &Socket| {
            let _ = socket.shutdown().await;
            Err(err)
        };

        debug!("Joining liveview Channel");

        let liveview_channel = match join_liveview_channel(
            &socket,
            &session_data,
            &client_opts.join_params,
            None,
            ws_timeout,
        )
        .await
        {
            Ok(channel) => channel,
            Err(e) => return cleanup_and_return(e, &socket).await,
        };

        if let Some(handler) = config.patch_handler.clone() {
            liveview_channel.document.arc_set_event_handler(handler);
        }

        let livereload_channel = if session_data.has_live_reload {
            match join_livereload_channel(config, &session_data, cookies).await {
                Ok(channel) => Some(channel),
                Err(e) => return cleanup_and_return(e, &socket).await,
            }
        } else {
            None
        };

        Ok(Self {
            document: liveview_channel.document(),
            event_pump: EventPump {
                main_events: liveview_channel.channel.events(),
                reload_events: livereload_channel.as_ref().map(|c| c.channel.events()),
                socket_statuses: socket.statuses(),
            },
            socket,
            liveview_channel,
            livereload_channel,
            session_data,
        })
    }

    pub fn event_futures(
        &self,
    ) -> (
        impl Future<Output = Result<EventPayload, EventsError>> + '_,
        impl Future<Output = Result<SocketStatus, WebSocketError>> + '_,
    ) {
        let server_event = self.event_pump.main_events.event().fuse();
        let maybe_reload_event = self.event_pump.reload_events.as_ref().map(|e| e.event());
        let socket_status = self.event_pump.socket_statuses.status();

        let socket_status = async {
            match socket_status.await {
                Ok(res) => res,
                Err(_) => std::future::pending().await,
            }
        }
        .fuse();

        let live_reload_proxy = async {
            match maybe_reload_event {
                Some(e) => e.await,
                None => std::future::pending().await,
            }
        }
        .fuse();

        let server_event = async {
            select! {
                r1 = live_reload_proxy => r1,
                r2 = server_event => r2,
            }
        }
        .fuse();

        (server_event, socket_status)
    }

    async fn shutdown(&self) {
        let _ = self.socket.disconnect().await;
        let _ = self.socket.shutdown().await;
        if let Some(livereload) = &self.livereload_channel {
            let _ = livereload.shutdown_parent_socket().await;
        }
    }

    // async fn handle_reply(&self, reply: &Payload) -> Result<ReplyAction, LiveSocketError> {
    //     let Payload::JSONPayload {
    //         json: JSON::Object { object },
    //     } = reply
    //     else {
    //         return Ok(ReplyAction::None);
    //     };

    //     if let Some(object) = object.get("live_redirect") {
    //         let summary = self.handle_redirect(object).await?;
    //         return Ok(ReplyAction::Redirected {
    //             summary,
    //             issuer: Issuer::LiveRedirect,
    //         });
    //     }

    //     if let Some(object) = object.get("redirect") {
    //         let summary = self.handle_redirect(object).await?;
    //         return Ok(ReplyAction::Redirected {
    //             summary,
    //             issuer: Issuer::Redirect,
    //         });
    //     }

    //     if let Some(diff) = object.get("diff") {
    //         self.document
    //             .merge_deserialized_fragment_json(diff.clone())?;

    //         return Ok(ReplyAction::DiffMerged);
    //     };

    //     Ok(ReplyAction::None)
    // }

    // async fn handle_redirect(&self, redirect: &JSON) -> Result<NavigationSummary, LiveSocketError> {
    //     let json = redirect.clone().into();
    //     let redirect: LiveRedirect = serde_json::from_value(json)?;
    //     let url = self.session_data.url.clone();
    //     let url = url.join(&redirect.to)?;

    //     let action = match redirect.kind {
    //         Some(RedirectKind::Push) | None => NavAction::Push,
    //         Some(RedirectKind::Replace) => NavAction::Replace,
    //     };

    //     let opts = NavOptions {
    //         action: Some(action),
    //         // TODO: this might contain user provided join params which should be omitted
    //         join_params: self.liveview_channel.join_params.clone().into(),
    //         ..NavOptions::default()
    //     };

    //     self.client_state.navigate(url.to_string(), opts).await
    // }
}

struct ConnectedStatus {
    pub session_data: SessionData,
    pub document: ffi::Document,
    pub join_document: Result<Document, LiveSocketError>,
    pub join_payload: Payload,
    pub msg_tx: UnboundedSender<ConnectedClientMessage>,
}

enum ClientStatus {
    Disconnected,
    Connecting,
    Reconnecting,
    /// TODO: call shutdown on all transitions
    Connected(ConnectedStatus),
    /// TODO: call shutdown on all transitions
    FatalError {
        error: LiveSocketError,
    },
}

impl ClientStatus {
    pub fn as_connected(&self) -> Result<&ConnectedStatus, LiveSocketError> {
        match self {
            ClientStatus::Connected(out) => Ok(out),
            ClientStatus::Reconnecting { .. }
            | ClientStatus::Disconnected
            | ClientStatus::Connecting => Err(LiveSocketError::ClientNotConnected),
            ClientStatus::FatalError { error } => Err(error.clone()),
        }
    }
}

#[derive(Debug, Clone)]
struct NavigationSummary {
    history_id: HistoryId,
    websocket_reconnected: bool,
}

impl LiveViewClientState {
    fn status(&self) -> ClientStatus {
        

        match self {
            LiveViewClientState::Disconnected => ClientStatus::Disconnected,
            LiveViewClientState::Connecting { .. } => ClientStatus::Connecting,
            LiveViewClientState::Reconnecting { .. } => ClientStatus::Reconnecting,
            LiveViewClientState::Connected {
                client, con_msg_tx, ..
            } => ClientStatus::Connected(ConnectedStatus {
                session_data: client.session_data.clone(),
                document: client.document.clone(),
                join_document: client.liveview_channel.join_document(),
                join_payload: client.liveview_channel.join_payload(),
                msg_tx: con_msg_tx.clone(),
            }),
            LiveViewClientState::FatalError(e) => ClientStatus::FatalError {
                error: e.error.clone(),
            },
        }
    }

    pub async fn shutdown(self) {
        match self {
            LiveViewClientState::Reconnecting {
                recconecting_client: con,
            }
            | LiveViewClientState::Connected { client: con, .. } => {
                con.shutdown().await;
            }
            LiveViewClientState::Connecting { job } => {
                tokio::spawn(async move {
                    let Ok(Ok(con)) = job.await else {
                        return;
                    };
                    con.shutdown().await;
                });
            }
            LiveViewClientState::FatalError(error) => {
                if let Some(e) = error.livereload_channel.as_ref() {
                    e.shutdown_parent_socket().await;
                }
            }
            _ => {}
        }
    }

    // Reconnect the websocket at the given url
    // TODO: this should just be an entirely new connection pushed through a ClientMessage
    //pub async fn reconnect(
    //    &self,
    //    url: String,
    //    opts: ConnectOpts,
    //    join_params: Option<HashMap<String, JSON>>,
    //) -> Result<(), LiveSocketError> {
    //    debug!("Reestablishing connection with settings: url: {url:?}, opts: {opts:?}");

    //    let url = Url::parse(&url)?;
    //    let new_session = SessionData::request(
    //        &url,
    //        &self.config.format.to_string(),
    //        opts,
    //        self.http_client.clone(),
    //    )
    //    .await?;

    //    let websocket_url = new_session.get_live_socket_url()?;

    //    let cookies = self.cookie_store.get_cookie_list(&url);

    //    debug!("Initiating new websocket connection: {websocket_url:?}");

    //    let adapter = self
    //        .config
    //        .socket_reconnect_strategy
    //        .clone()
    //        .map(StrategyAdapter::from)
    //        .map(|s| Box::new(s) as Box<dyn ReconnectStrategy>);

    //    let socket = Socket::spawn(websocket_url, cookies.clone(), adapter).await?;

    //    let old_socket = self.socket.lock()?.clone();
    //    let _ = old_socket.shutdown().await;

    //    *self.socket.lock()? = socket;

    //    *self.session_data.lock()? = new_session;
    //    let ws_timeout = Duration::from_millis(self.config.websocket_timeout);
    //    debug!("Rejoining liveview channel");
    //    let new_channel = join_liveview_channel(
    //        &self.socket,
    //        &self.session_data,
    //        &join_params,
    //        None,
    //        ws_timeout,
    //    )
    //    .await?;

    //    if let Some(handler) = &self.config.patch_handler {
    //        new_channel.document.arc_set_event_handler(handler.clone());
    //    }

    //    *self.liveview_channel.lock()? = new_channel;
    //    let has_reload = self.session_data.lock()?.has_live_reload;

    //    if has_reload {
    //        debug!("Rejoining livereload channel");
    //        let new_livereload =
    //            join_livereload_channel(&self.config, &self.session_data, cookies).await?;
    //        let old = self.livereload_channel.lock()?.take();

    //        if let Some(channel) = old {
    //            let _ = channel.socket.shutdown().await;
    //        }

    //        *self.livereload_channel.lock()? = Some(new_livereload);
    //    }

    //    Ok(())
    //}
}

// impl LiveViewClientState {
//     const RETRY_REASONS: &[&str] = &["stale", "unauthorized"];
//     /// try to do the internal nav, attempting to fix
//     /// recoverable errors which occur when connecting across
//     /// auth state, live_sessions and respecting redirects.
//     /// If the websocket needed to be refreshed this returns true
//     /// otherwise it returns false.
//     async fn try_nav(
//         &self,
//         additional_params: &Option<HashMap<String, JSON>>,
//     ) -> Result<bool, LiveSocketError> {
//         let current = self
//             .nav_ctx
//             .lock()?
//             .current()
//             .ok_or(LiveSocketError::NavigationImpossible)?;
//
//         let ws_timeout = Duration::from_millis(self.config.websocket_timeout);
//
//         let chan = self.liveview_channel.lock()?.channel();
//         chan.leave().await?;
//
//         match join_liveview_channel(
//             &self.socket,
//             &self.session_data,
//             additional_params,
//             Some(current.url.clone()),
//             ws_timeout,
//         )
//         .await
//         {
//             Err(LiveSocketError::JoinRejection {
//                 error:
//                     Payload::JSONPayload {
//                         json: JSON::Object { object },
//                     },
//             }) => {
//                 if object
//                     .get("reason")
//                     .and_then(|r| match r {
//                         JSON::Str { string } => Some(string),
//                         _ => None,
//                     })
//                     .is_none_or(|reason| !Self::RETRY_REASONS.contains(&reason.as_str()))
//                 {
//                     return Err(LiveSocketError::JoinRejection {
//                         error: Payload::JSONPayload {
//                             json: JSON::Object { object },
//                         },
//                     });
//                 }
//
//                 let url = Url::parse(&current.url)?;
//                 let format = self.config.format.to_string();
//
//                 let new_session_data = SessionData::request(
//                     &url,
//                     &format,
//                     Default::default(),
//                     self.http_client.clone(),
//                 )
//                 .await?;
//
//                 let websocket_url = new_session_data.get_live_socket_url()?;
//
//                 let adapter = self
//                     .config
//                     .socket_reconnect_strategy
//                     .clone()
//                     .map(StrategyAdapter::from)
//                     .map(|s| Box::new(s) as Box<dyn ReconnectStrategy>);
//
//                 let new_socket = Socket::spawn(
//                     websocket_url,
//                     Some(new_session_data.cookies.clone()),
//                     adapter,
//                 )
//                 .await?;
//
//                 let sock = self.socket.lock()?.clone();
//                 sock.shutdown()
//                     .await
//                     .map_err(|e| LiveSocketError::Phoenix {
//                         error: format!("{e:?}"),
//                     })?;
//
//                 *self.socket.lock()? = new_socket;
//                 *self.session_data.lock()? = new_session_data;
//
//                 let channel = join_liveview_channel(
//                     &self.socket,
//                     &self.session_data,
//                     additional_params,
//                     None,
//                     ws_timeout,
//                 )
//                 .await?;
//
//                 if let Some(event_handler) = &self.config.patch_handler {
//                     channel
//                         .document
//                         .arc_set_event_handler(event_handler.clone())
//                 }
//
//                 let chan = &self.liveview_channel.lock()?.channel.clone();
//                 chan.leave().await?;
//
//                 *self.liveview_channel.lock()? = channel;
//                 Ok(true)
//             }
//             Ok(channel) => {
//                 let chan = &self.liveview_channel.lock()?.channel.clone();
//                 chan.leave().await?;
//
//                 if let Some(event_handler) = &self.config.patch_handler {
//                     channel
//                         .document
//                         .arc_set_event_handler(event_handler.clone())
//                 }
//
//                 *self.liveview_channel.lock()? = channel;
//                 Ok(false)
//             }
//             Err(e) => Err(e),
//         }
//     }
//
//     async fn try_nav_outer<F>(
//         &self,
//         additional_params: &Option<HashMap<String, JSON>>,
//         nav_action: F,
//     ) -> Result<NavigationSummary, LiveSocketError>
//     where
//         F: FnOnce(&mut NavCtx) -> Option<HistoryId>,
//     {
//         // try the navigation action, if it's impossible the returned
//         // history id will be None.
//         let new_id = {
//             let mut nav_ctx = self.nav_ctx.lock()?;
//             nav_action(&mut nav_ctx)
//         };
//
//         match new_id {
//             Some(id) => {
//                 // actually do the navigation, updating everything in one fell swoop
//                 let websocket_reconnected = self.try_nav(additional_params).await?;
//                 Ok(NavigationSummary {
//                     history_id: id,
//                     websocket_reconnected,
//                 })
//             }
//             None => Err(LiveSocketError::NavigationImpossible),
//         }
//     }
//
//     async fn navigate(
//         &self,
//         url: String,
//         opts: NavOptions,
//     ) -> Result<NavigationSummary, LiveSocketError> {
//         let url = Url::parse(&url)?;
//         self.try_nav_outer(&opts.join_params.clone(), |ctx| {
//             ctx.navigate(url, opts, true)
//         })
//         .await
//     }
//
//     async fn reload(&self, opts: NavActionOptions) -> Result<NavigationSummary, LiveSocketError> {
//         self.try_nav_outer(&opts.join_params, |ctx| {
//             ctx.reload(opts.extra_event_info, true)
//         })
//         .await
//     }
//
//     async fn back(&self, opts: NavActionOptions) -> Result<NavigationSummary, LiveSocketError> {
//         self.try_nav_outer(&opts.join_params, |ctx| {
//             ctx.back(opts.extra_event_info, true)
//         })
//         .await
//     }
//
//     async fn forward(&self, opts: NavActionOptions) -> Result<NavigationSummary, LiveSocketError> {
//         self.try_nav_outer(&opts.join_params, |ctx| {
//             ctx.forward(opts.extra_event_info, true)
//         })
//         .await
//     }
//
//     fn patch(&self, url_path: String) -> Result<NavigationSummary, LiveSocketError> {
//         let id = self.nav_ctx.lock()?.patch(url_path, true);
//
//         Ok(NavigationSummary {
//             history_id: id.unwrap_or(0),
//             websocket_reconnected: false,
//         })
//     }
//
//     async fn traverse_to(
//         &self,
//         id: HistoryId,
//         opts: NavActionOptions,
//     ) -> Result<NavigationSummary, LiveSocketError> {
//         self.try_nav_outer(&opts.join_params, |ctx| {
//             ctx.traverse_to(id, opts.extra_event_info, true)
//         })
//         .await
//     }
//
//     fn can_go_back(&self) -> bool {
//         self.nav_ctx
//             .try_lock()
//             .map(|ctx| ctx.can_go_back())
//             .unwrap_or(false)
//     }
//
//     fn can_go_forward(&self) -> bool {
//         self.nav_ctx
//             .try_lock()
//             .map(|ctx| ctx.can_go_forward())
//             .unwrap_or(false)
//     }
//
//     fn can_traverse_to(&self, id: HistoryId) -> bool {
//         self.nav_ctx
//             .try_lock()
//             .map(|ctx| ctx.can_traverse_to(id))
//             .unwrap_or(false)
//     }
//
//     fn get_history_entries(&self) -> Vec<NavHistoryEntry> {
//         self.nav_ctx
//             .try_lock()
//             .map(|ctx| ctx.entries())
//             .unwrap_or_default()
//     }
//
//     fn current_history_entry(&self) -> Option<NavHistoryEntry> {
//         self.nav_ctx.try_lock().ok().and_then(|ctx| ctx.current())
//     }
// }
