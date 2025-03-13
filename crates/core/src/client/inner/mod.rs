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
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use channel_init::*;
use cookie_store::PersistentCookieStore;
use event_loop::{ClientMessage, EventLoop};
use futures::FutureExt;
use log::{debug, warn};
use logging::*;
use navigation::NavCtx;
use phoenix_channels_client::{
    ChannelStatus, Event, EventPayload, Events, EventsError, Payload, ReconnectStrategy, Socket,
    SocketStatus, SocketStatuses, StatusesError, WebSocketError, JSON,
};
use readonly_mutex::ReadOnlyMutex;
use reqwest::{redirect::Policy, Client as HttpClient, Url};
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
};
use tokio_util::sync::CancellationToken;

use super::{ClientConnectOpts, LiveViewClientConfiguration, LogLevel};
use crate::{
    callbacks::{self, *},
    client::StrategyAdapter,
    dom::{
        ffi::{self, Document as FFIDocument},
        AttributeName, AttributeValue, Document, Selector,
    },
    error::{ConnectionError, LiveSocketError},
    live_socket::{
        navigation::{NavAction, NavActionOptions, NavOptions},
        ConnectOpts, LiveChannel, LiveFile, SessionData,
    },
    protocol::{LiveRedirect, RedirectKind},
};

pub(crate) enum LiveViewClientState {
    Disconnected,
    Connecting,
    Reconnecting {
        old_client: Option<ConnectedClient>,
    },
    /// TODO: call shutdown on all transitions
    Connected(ConnectedClient),
    /// TODO: call shutdown on all transitions
    FatalError(FatalError),
}

pub(crate) struct FatalError {
    error: Arc<LiveSocketError>,
    livereload_channel: Option<Arc<LiveChannel>>,
    channel_events: Option<Arc<Events>>,
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
        url: &Url,
        http_client: &HttpClient,
        client_opts: ClientConnectOpts,
        cookie_store: &PersistentCookieStore,
    ) -> Result<Self, LiveSocketError> {
        let opts = ConnectOpts {
            headers: client_opts.headers,
            timeout_ms: config.dead_render_timeout,
            ..ConnectOpts::default()
        };
        let format = config.format.to_string();
        let session_data = SessionData::request(url, &format, opts, http_client.clone()).await?;

        let cookies = cookie_store.get_cookie_list(url);
        let websocket_url = session_data.get_live_socket_url()?;

        log::info!("Initiating Websocket connection: {websocket_url:?} , cookies: {cookies:?}");

        let adapter = config
            .socket_reconnect_strategy
            .clone()
            .map(StrategyAdapter::from)
            .map(|s| Box::new(s) as Box<dyn ReconnectStrategy>);

        let socket = Socket::spawn(websocket_url, cookies.clone(), adapter).await?;

        let cleanup_and_return = async |err: LiveSocketError, socket: &Socket| {
            socket.shutdown().await;
            Err(err)
        };

        let ws_timeout = Duration::from_millis(config.websocket_timeout);

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
            match join_livereload_channel(&config, &session_data, cookies).await {
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
}

enum ClientStatus {
    Disconnected,
    Connecting,
    Reconnecting {
        reconnecting_client: Option<ConnectedStatus>,
    },
    /// TODO: call shutdown on all transitions
    Connected(ConnectedStatus),
    /// TODO: call shutdown on all transitions
    FatalError {
        error: Arc<LiveSocketError>,
    },
}

impl ClientStatus {
    pub fn as_connected(&self) -> Result<&ConnectedStatus, LiveSocketError> {
        match self {
            ClientStatus::Connected(out) => Ok(out),
            ClientStatus::Reconnecting { .. } => todo!(),
            ClientStatus::Disconnected => todo!(),
            ClientStatus::Connecting => todo!(),
            ClientStatus::FatalError { error } => todo!(),
        }
    }
}

pub struct LiveViewClientInner {
    /// Shared state between the background event loop and the client handle
    status: ReadOnlyMutex<ClientStatus>,
    /// user provided settings and defaults
    config: LiveViewClientConfiguration,
    /// A book keeping context for navigation events.
    nav_ctx: Mutex<NavCtx>,
    /// HTTP client used to request dead renders.
    http_client: HttpClient,
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

#[derive(Debug, Clone)]
struct NavigationSummary {
    history_id: HistoryId,
    websocket_reconnected: bool,
}

// First implement the accessor methods on LiveViewClientInner
impl LiveViewClientInner {
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

        let url = Url::parse(&url)?;

        let mut nav_ctx = NavCtx::default();
        nav_ctx.navigate(url.clone(), NavOptions::default(), false);

        let state =
            LiveViewClientState::new(&config, url, client_opts, &http_client, &cookie_store).await;

        let EventLoop {
            msg_tx,
            cancellation_token,
            status: state,
        } = EventLoop::new(state, &config);

        Ok(Self {
            status: state,
            msg_tx,
            cancellation_token,
            config,
            nav_ctx: nav_ctx.into(),
            http_client,
        })
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
    pub async fn navigate(
        &self,
        url: String,
        opts: NavOptions,
    ) -> Result<HistoryId, LiveSocketError> {
        let parsed_url = Url::parse(&url)?;
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.navigate(parsed_url, opts.clone(), true)?;

        self.msg_tx.send(ClientMessage::Navigate { url, opts });

        Ok(id)
    }

    /// This does nothing but update the navigation contex
    pub async fn patch(&self, url: String) -> Result<HistoryId, LiveSocketError> {
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.patch(url.clone(), true)?;
        Ok(id)
    }

    pub async fn reload(&self, info: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.reload(info.extra_event_info.clone(), true)?;
        let current = ctx.current().ok_or(NavigationError::NoCurrentEntry)?;

        self.msg_tx.send(ClientMessage::Navigate {
            url: current.url,
            opts: info.into(),
        });

        Ok(id)
    }

    // not for internal use
    pub async fn back(&self, info: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.back(info.extra_event_info.clone(), true)?;
        let current = ctx.current().ok_or(NavigationError::NoCurrentEntry)?;

        self.msg_tx.send(ClientMessage::Navigate {
            url: current.url,
            opts: info.into(),
        });

        Ok(id)
    }

    // not for internal use
    pub async fn forward(&self, info: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.forward(info.extra_event_info.clone(), true)?;
        let current = ctx.current().ok_or(NavigationError::NoCurrentEntry)?;

        self.msg_tx.send(ClientMessage::Navigate {
            url: current.url,
            opts: info.into(),
        });

        Ok(id)
    }

    // not for internal use
    pub async fn traverse_to(
        &self,
        id: HistoryId,
        info: NavActionOptions,
    ) -> Result<HistoryId, LiveSocketError> {
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.traverse_to(id, info.extra_event_info.clone(), true)?;
        let current = ctx.current().ok_or(NavigationError::NoCurrentEntry)?;

        self.msg_tx.send(ClientMessage::Navigate {
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
        let (response_tx, response_rx) = oneshot::channel();

        let _ = self.msg_tx.send(ClientMessage::Call {
            response_tx,
            event: Event::from_string(event_name),
            payload,
        });

        let resp = response_rx.await.map_err(|e| LiveSocketError::Call {
            error: format!("{e}"),
        })??;

        Ok(resp)
    }

    pub async fn cast(&self, event_name: String, payload: Payload) {
        let _ = self.msg_tx.send(ClientMessage::Cast {
            event: Event::from_string(event_name),
            payload,
        });
    }

    pub fn set_log_level(&self, level: LogLevel) {
        set_log_level(level)
    }
}

impl LiveViewClientState {
    /// The first connection and initialization, fetches the dead render and opens the default channel.
    pub async fn new(
        config: &LiveViewClientConfiguration,
        url: Url,
        client_opts: ClientConnectOpts,
        http_client: &HttpClient,
        cookie_store: &PersistentCookieStore,
    ) -> Self {
        match ConnectedClient::try_new(config, &url, http_client, client_opts, cookie_store).await {
            Ok(connected) => Self::Connected(connected),
            Err(LiveSocketError::ConnectionError(e)) => Self::FatalError(FatalError {
                error: LiveSocketError::ConnectionError(ConnectionError {
                    error_code: e.error_code,
                    error_text: e.error_text,
                    livereload_channel: None,
                })
                .into(),
                channel_events: e.livereload_channel.as_ref().map(|c| c.channel.events()),
                livereload_channel: e.livereload_channel,
            }),
            Err(error) => Self::FatalError(FatalError {
                error: error.into(),
                livereload_channel: None,
                channel_events: None,
            }),
        }
    }

    fn status(&self) -> ClientStatus {
        let out = match self {
            LiveViewClientState::Disconnected => ClientStatus::Disconnected,
            LiveViewClientState::Connecting => ClientStatus::Connecting,
            LiveViewClientState::Reconnecting { old_client, .. } => ClientStatus::Reconnecting {
                reconnecting_client: old_client.as_ref().map(|c| ConnectedStatus {
                    session_data: c.session_data.clone(),
                    document: c.document.clone(),
                    join_document: c.liveview_channel.join_document(),
                    join_payload: c.liveview_channel.join_payload(),
                }),
            },
            LiveViewClientState::Connected(c) => ClientStatus::Connected(ConnectedStatus {
                session_data: c.session_data.clone(),
                document: c.document.clone(),
                join_document: c.liveview_channel.join_document(),
                join_payload: c.liveview_channel.join_payload(),
            }),
            LiveViewClientState::FatalError(e) => ClientStatus::FatalError {
                error: e.error.clone(),
            },
        };

        out
    }

    pub async fn shutdown(&self) {
        match self {
            LiveViewClientState::Reconnecting {
                old_client: Some(con),
                ..
            }
            | LiveViewClientState::Connected(con) => {
                con.socket.disconnect().await;
                con.socket.shutdown().await;
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
