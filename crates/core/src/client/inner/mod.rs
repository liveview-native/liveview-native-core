mod connected_client;
mod cookie_store;
mod event_loop;
mod logging;
mod navigation;

pub use navigation::NavigationError;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use cookie_store::PersistentCookieStore;
use event_loop::{ClientMessage, ConnectedClientMessage, EventLoop};
use log::{debug, warn};
use logging::*;
use navigation::NavCtx;
use phoenix_channels_client::{ChannelStatus, Event, Events, Payload, JSON};
use reqwest::{redirect::Policy, Client as HttpClient, Url};
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot, watch,
    },
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use super::{ClientConnectOpts, ClientStatuses, LiveViewClientConfiguration, LogLevel};
use crate::{
    callbacks::LiveViewClientStatus as FFIClientStatus,
    callbacks::*,
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
    status: watch::Receiver<ClientStatus>,
    /// A book keeping context for navigation events.
    nav_ctx: Arc<Mutex<NavCtx>>,
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

        let nav_ctx = Arc::new(Mutex::new(nav_ctx));

        let EventLoop {
            msg_tx,
            cancellation_token,
            status,
        } = EventLoop::new(
            config,
            &url,
            cookie_store,
            http_client,
            client_opts,
            nav_ctx.clone(),
        )
        .await;

        let out = Self {
            status,
            msg_tx,
            cancellation_token,
            nav_ctx,
        };

        Ok(out)
    }

    pub(crate) fn reconnect(
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
                remote: false,
            })
            .map_err(|_| LiveSocketError::DisconnectionError)
    }

    pub(crate) async fn disconnect(&self) -> Result<(), LiveSocketError> {
        let (response_tx, response_rx) = oneshot::channel();
        let _ = self.msg_tx.send(ClientMessage::Disconnect { response_tx });
        response_rx
            .await
            .map_err(|_| LiveSocketError::DisconnectionError)?
    }

    pub(crate) fn shutdown(&self) {
        self.cancellation_token.cancel()
    }

    pub(crate) fn status_stream(&self) -> ClientStatuses {
        ClientStatuses::from(self.status.clone())
    }

    pub async fn upload_file(&self, file: Arc<LiveFile>) -> Result<(), LiveSocketError> {
        let (response_tx, response_rx) = oneshot::channel();

        {
            let state = self.status.borrow();
            let con = state.as_connected()?;
            let _ = con
                .msg_tx
                .send(ConnectedClientMessage::UploadFile { file, response_tx });
        }

        response_rx.await.map_err(|e| LiveSocketError::Upload {
            error: crate::error::UploadError::Other {
                error: format!("{e:?}"),
            },
        })?
    }

    pub fn status(&self) -> ClientStatus {
        (*self.status.borrow()).clone()
    }

    pub fn watch_status(&self) -> watch::Receiver<ClientStatus> {
        self.status.clone()
    }

    pub fn get_phx_upload_id(&self, phx_target_name: &str) -> Result<String, LiveSocketError> {
        let state = self.status.borrow();
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
        let state = self.status.borrow();
        let con = state.as_connected()?;
        Ok(con.session_data.url.to_string())
    }

    pub fn csrf_token(&self) -> Result<String, LiveSocketError> {
        let state = self.status.borrow();
        let con = state.as_connected()?;
        Ok(con.session_data.csrf_token.clone())
    }

    /// Returns the current document state.
    pub fn document(&self) -> Result<FFIDocument, LiveSocketError> {
        let state = self.status.borrow();
        let con = state.as_connected()?;
        Ok(con.document.clone())
    }

    /// Returns the state of the document upon the initial websocket connection.
    pub fn join_document(&self) -> Result<Document, LiveSocketError> {
        let state = self.status.borrow();
        let con = state.as_connected()?;
        match &con.join_document {
            Ok(doc) => Ok(doc.clone()),
            Err(_) => Err(LiveSocketError::NoDocumentInJoinPayload),
        }
    }

    /// returns the join payload
    pub fn join_payload(&self) -> Result<Payload, LiveSocketError> {
        let state = self.status.borrow();
        let con = state.as_connected()?;
        Ok(con.join_payload.clone())
    }

    /// To establish the websocket connection, the client depends on an initial HTTP
    /// request pull an html document and extract several pieces of meta data from it.
    /// This function returns that initial document.
    pub fn dead_render(&self) -> Result<Document, LiveSocketError> {
        let state = self.status.borrow();
        let con = state.as_connected()?;
        Ok(con.session_data.dead_render.clone())
    }

    pub fn style_urls(&self) -> Result<Vec<String>, LiveSocketError> {
        let state = self.status.borrow();
        let con = state.as_connected()?;
        Ok(con.session_data.style_urls.clone())
    }

    pub fn navigate(&self, url: String, opts: NavOptions) -> Result<HistoryId, LiveSocketError> {
        let status = self.status.borrow();
        let con = status.as_connected()?;
        let parsed_url = Url::parse(&url)?;
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.navigate(parsed_url, opts.clone(), true)?;

        // todo: error handling
        let _ = con.msg_tx.send(ConnectedClientMessage::Navigate {
            url,
            opts,
            remote: false,
        });

        Ok(id)
    }

    /// This does nothing but update the navigation context
    pub fn patch(&self, url: String) -> Result<HistoryId, LiveSocketError> {
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.patch(url.clone(), true)?;
        Ok(id)
    }

    pub fn reload(&self, info: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        let status = self.status.borrow();
        let con = status.as_connected()?;
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.reload(info.extra_event_info.clone(), true)?;
        let current = ctx.current().ok_or(NavigationError::NoCurrentEntry)?;

        // todo error handling
        let _ = con.msg_tx.send(ConnectedClientMessage::Navigate {
            url: current.url,
            opts: info.into(),
            remote: false,
        });

        Ok(id)
    }

    // not for internal use
    pub fn back(&self, info: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        let status = self.status.borrow();
        let con = status.as_connected()?;
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.back(info.extra_event_info.clone(), true)?;
        let current = ctx.current().ok_or(NavigationError::NoCurrentEntry)?;

        // todo error handling
        let _ = con.msg_tx.send(ConnectedClientMessage::Navigate {
            url: current.url,
            opts: info.into(),
            remote: false,
        });

        Ok(id)
    }

    // not for internal use
    pub fn forward(&self, info: NavActionOptions) -> Result<HistoryId, LiveSocketError> {
        let status = self.status.borrow();
        let con = status.as_connected()?;
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.forward(info.extra_event_info.clone(), true)?;
        let current = ctx.current().ok_or(NavigationError::NoCurrentEntry)?;

        // todo error handling
        let _ = con.msg_tx.send(ConnectedClientMessage::Navigate {
            url: current.url,
            opts: info.into(),
            remote: false,
        });

        Ok(id)
    }

    // not for internal use
    pub fn traverse_to(
        &self,
        id: HistoryId,
        info: NavActionOptions,
    ) -> Result<HistoryId, LiveSocketError> {
        let status = self.status.borrow();
        let con = status.as_connected()?;
        let mut ctx = self.nav_ctx.lock()?;
        let id = ctx.traverse_to(id, info.extra_event_info.clone(), true)?;
        let current = ctx.current().ok_or(NavigationError::NoCurrentEntry)?;

        let _ = con.msg_tx.send(ConnectedClientMessage::Navigate {
            url: current.url,
            opts: info.into(),
            remote: false,
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
        let _ = self.status.borrow().as_connected()?;
        let (response_tx, response_rx) = oneshot::channel();

        {
            let status = self.status.borrow();
            let con = status.as_connected()?;
            let _ = con.msg_tx.send(ConnectedClientMessage::Call {
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
        let status = self.status.borrow();
        let con = status.as_connected()?;
        let _ = con.msg_tx.send(ConnectedClientMessage::Cast {
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
        job: JoinHandle<Result<connected_client::ConnectedClient, LiveSocketError>>,
    },
    /// TODO: call shutdown on all transitions
    Reconnecting {
        reconnecting_client: connected_client::ConnectedClient,
    },
    /// TODO: call shutdown on all transitions
    Connected {
        con_msg_tx: UnboundedSender<ConnectedClientMessage>,
        con_msg_rx: UnboundedReceiver<ConnectedClientMessage>,
        client: connected_client::ConnectedClient,
    },
    FatalError(FatalError),
}

#[derive(Debug, Clone)]
pub struct ConnectedStatus {
    pub session_data: SessionData,
    pub document: ffi::Document,
    pub join_document: Result<Document, LiveSocketError>,
    pub join_payload: Payload,
    pub channel_status: ChannelStatus,
    pub msg_tx: UnboundedSender<ConnectedClientMessage>,
}

#[derive(Debug, Clone)]
pub enum ClientStatus {
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

    pub fn name(&self) -> &str {
        match self {
            ClientStatus::Disconnected => "disconnected",
            ClientStatus::Connecting => "connecting",
            ClientStatus::Reconnecting => "reconnecting",
            ClientStatus::Connected(_) => "connected",
            ClientStatus::FatalError { .. } => "error",
        }
    }

    pub(crate) fn as_ffi(&self) -> FFIClientStatus {
        match self {
            Self::Disconnected => FFIClientStatus::Disconnected,
            Self::Connecting { .. } => FFIClientStatus::Connecting,
            Self::Reconnecting { .. } => FFIClientStatus::Reconnecting,
            ClientStatus::FatalError { error } => FFIClientStatus::Error {
                error: error.clone(),
            },
            Self::Connected(status) => match status.channel_status {
                ChannelStatus::Joined => FFIClientStatus::Connected {
                    channel_status: crate::callbacks::MainChannelStatus::Connected {
                        document: status.document.clone().into(),
                    },
                },
                ChannelStatus::WaitingForSocketToConnect
                | ChannelStatus::WaitingToJoin
                | ChannelStatus::Joining
                | ChannelStatus::WaitingToRejoin { .. }
                | ChannelStatus::Leaving
                | ChannelStatus::Left
                | ChannelStatus::ShuttingDown
                | ChannelStatus::ShutDown => FFIClientStatus::Connected {
                    channel_status: crate::callbacks::MainChannelStatus::Reconnecting,
                },
            },
        }
    }
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
                channel_status: client.liveview_channel.channel.status(),
            }),
            LiveViewClientState::FatalError(e) => ClientStatus::FatalError {
                error: e.error.clone(),
            },
        }
    }
}

#[derive(Clone)]
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
