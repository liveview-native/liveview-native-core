use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use futures::{pin_mut, select, FutureExt};
use log::{debug, error};
use phoenix_channels_client::{
    CallError, ConnectError, Event, Payload, SocketStatus, JSON,
};
use reqwest::Client;
//use state::{EventLoopState, ReplyAction};
use tokio::sync::{
        mpsc::{self, unbounded_channel},
        oneshot,
    };
use tokio_util::sync::CancellationToken;

use super::{
    cookie_store::PersistentCookieStore, readonly_mutex::ReadOnlyMutex, ClientStatus,
    ConnectedClient, DocumentChangeHandler, FatalError,
    LiveViewClientState, NetworkEventHandler,
};
use crate::{
    client::{ClientStatus as FFIClientStatus, LiveViewClientConfiguration},
    error::{ConnectionError, LiveSocketError},
    live_socket::{navigation::NavOptions, ConnectOpts},
};

const MAX_REDIRECTS: u32 = 10;

pub(crate) struct EventLoop {
    pub msg_tx: mpsc::UnboundedSender<ClientMessage>,
    pub cancellation_token: CancellationToken,
    pub status: ReadOnlyMutex<ClientStatus>,
}

impl EventLoop {
    pub async fn new(
        config: LiveViewClientConfiguration,
        url: &str,
        cookie_store: Arc<PersistentCookieStore>,
        http_client: Client,
        client_opts: crate::client::ClientConnectOpts,
    ) -> Self {
        let config = Arc::new(config);
        let url = url.to_owned();

        let client_clone = http_client.clone();
        let config_clone = config.clone();
        let cookie_store_clone = cookie_store.clone();

        let client_state = LiveViewClientState::Connecting {
            job: tokio::spawn(async move {
                ConnectedClient::try_new(
                    &config_clone,
                    &url,
                    &client_clone,
                    client_opts.clone(),
                    &cookie_store_clone,
                )
                .await
            }),
        };

        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let cancellation_token = CancellationToken::new();
        let token_clone = cancellation_token.clone();
        let status = Arc::new(Mutex::new(client_state.status()));
        let status_clone = status.clone();

        let manager = LiveViewClientManager::new(status_clone, config, http_client, cookie_store);

        tokio::spawn(async move {
            let mut current_state = Some(client_state);

            if let Some(handler) = manager.network_handler.as_ref() {
                handler.on_status_change(FFIClientStatus::Connecting);
            }

            // Run the event loop
            loop {
                let client_state = current_state.take().expect("always some");
                let tag = std::mem::discriminant(&client_state);

                let next_state = match client_state {
                    LiveViewClientState::Disconnected => {
                        manager
                            .disconnected_loop(&mut msg_rx, &token_clone, client_state)
                            .await
                    }
                    LiveViewClientState::Reconnecting { .. } => {
                        manager
                            .reconnecting_loop(&mut msg_rx, &token_clone, client_state)
                            .await
                    }
                    LiveViewClientState::Connecting { .. } => {
                        manager
                            .connecting_loop(&mut msg_rx, &token_clone, client_state)
                            .await
                    }
                    LiveViewClientState::Connected { .. } => {
                        manager
                            .connected_loop(&mut msg_rx, &token_clone, client_state)
                            .await
                    }
                    LiveViewClientState::FatalError(FatalError { .. }) => {
                        manager
                            .error_loop(&mut msg_rx, &token_clone, client_state)
                            .await
                    }
                };

                let next_state = manager.update_state(tag, next_state);
                current_state = Some(next_state);

                // If we're shutting down, break the loop
                if token_clone.is_cancelled() {
                    break;
                }
            }
        });

        Self {
            msg_tx,
            cancellation_token,
            status: ReadOnlyMutex::new(status),
        }
    }
}

/// Messages that can only be received by a connected client
pub enum ConnectedClientMessage {
    Call {
        event: Event,
        payload: Payload,
        response_tx: oneshot::Sender<Result<Payload, CallError>>,
    },
    Cast {
        event: Event,
        payload: Payload,
    },
    Navigate {
        url: String,
        opts: NavOptions,
    },
}

/// Messages to the main background event loop, can be received in any state
pub enum ClientMessage {
    /// Send a message and wait for the response,
    /// If it is an event it will be processed by the loop
    Reconnect {
        url: String,
        opts: ConnectOpts,
        join_params: Option<HashMap<String, JSON>>,
    },
    Disconnect {
        response_tx: oneshot::Sender<Result<(), LiveSocketError>>,
    },
}

pub struct LiveViewClientManager {
    status: Arc<Mutex<ClientStatus>>,
    network_handler: Option<Arc<dyn NetworkEventHandler>>,
    document_handler: Option<Arc<dyn DocumentChangeHandler>>,
    config: Arc<LiveViewClientConfiguration>,
    cookie_store: Arc<PersistentCookieStore>,
    http_client: Client,
}

impl LiveViewClientManager {
    pub fn new(
        status: Arc<Mutex<ClientStatus>>,
        config: Arc<LiveViewClientConfiguration>,
        http_client: Client,
        cookie_store: Arc<PersistentCookieStore>,
    ) -> Self {
        Self {
            status,
            network_handler: config.network_event_handler.clone(),
            document_handler: config.patch_handler.clone(),
            config,
            http_client,
            cookie_store,
        }
    }

    async fn create_reconnection_task(
        &self,
        url: String,
        opts: ConnectOpts,
        join_params: Option<HashMap<String, JSON>>,
    ) -> LiveViewClientState {
        let config = self.config.clone();
        let http_client = self.http_client.clone();
        let cookie_store = self.cookie_store.clone();

        let client_opts = crate::client::ClientConnectOpts {
            join_params,
            headers: opts.headers,
            method: opts.method,
            request_body: opts.body,
        };

        let job = tokio::spawn(async move {
            ConnectedClient::try_new(&config, &url, &http_client, client_opts, &cookie_store).await
        });

        LiveViewClientState::Connecting { job }
    }

    async fn connected_loop(
        &self,
        msg_rx: &mut mpsc::UnboundedReceiver<ClientMessage>,
        token: &CancellationToken,
        mut state: LiveViewClientState,
    ) -> Result<LiveViewClientState, LiveSocketError> {
        {
            let LiveViewClientState::Connected {
                ref mut con_msg_rx,
                ref client,
                ..
            } = state
            else {
                return Ok(state);
            };

            let token = token.cancelled().fuse();
            let client_msg = msg_rx.recv().fuse();
            let con_client_msg = con_msg_rx.recv().fuse();
            let (server_event, socket_status) = client.event_futures();
            let (server_event, socket_status) = (server_event.fuse(), socket_status.fuse());

            pin_mut!(
                client_msg,
                server_event,
                socket_status,
                token,
                con_client_msg
            );

            select! {
               _ = token => {
                  client.shutdown().await;
                  return Ok(LiveViewClientState::Disconnected);
               }
            }
        }

        Ok(state)
    }

    async fn disconnected_loop(
        &self,
        msg_rx: &mut mpsc::UnboundedReceiver<ClientMessage>,
        token: &CancellationToken,
        state: LiveViewClientState,
    ) -> Result<LiveViewClientState, LiveSocketError> {
        let LiveViewClientState::Disconnected = state else {
            return Ok(state);
        };

        let client_msg = msg_rx.recv().fuse();
        let cancelled = token.cancelled().fuse();

        pin_mut!(client_msg, cancelled);

        select! {
            _ = cancelled => { Ok(state) }
            msg = client_msg => {
              let Some(msg) = msg else {
                  token.cancel();
                  return Err(LiveSocketError::DisconnectionError)
              };
               match msg {
                   ClientMessage::Reconnect { url, opts, join_params } => {
                       debug!("Reconnection requested to URL: {}", url);
                       let client_state = self.create_reconnection_task(url, opts, join_params).await;
                       Ok(client_state)
                   },
                   ClientMessage::Disconnect { response_tx } => {
                      let _ = response_tx.send(Ok(()));
                      Ok(state)
                   },
               }
            }
        }
    }

    async fn error_loop(
        &self,
        msg_rx: &mut mpsc::UnboundedReceiver<ClientMessage>,
        token: &CancellationToken,
        state: LiveViewClientState,
    ) -> Result<LiveViewClientState, LiveSocketError> {
        {
            let LiveViewClientState::FatalError(FatalError {
                ref channel_events, ..
            }) = state
            else {
                return Ok(state);
            };

            let token = token.cancelled().fuse();
            let client_msg = msg_rx.recv().fuse();
            let live_reload_proxy = async {
                match channel_events {
                    Some(e) => e.event().await,
                    None => std::future::pending().await,
                }
            }
            .fuse();

            pin_mut!(client_msg, live_reload_proxy, token);
        }

        Ok(state)
    }

    async fn reconnecting_loop(
        &self,
        msg_rx: &mut mpsc::UnboundedReceiver<ClientMessage>,
        token: &CancellationToken,
        state: LiveViewClientState,
    ) -> Result<LiveViewClientState, LiveSocketError> {
        let LiveViewClientState::Reconnecting {
            recconecting_client: client,
        } = state
        else {
            return Ok(state);
        };

        let cancel_token = token.cancelled().fuse();
        let client_msg = msg_rx.recv().fuse();
        let sock_status = client.event_pump.socket_statuses.clone();
        let socket_status = sock_status.status().fuse();

        pin_mut!(client_msg, cancel_token, socket_status);

        select! {
            _ = cancel_token => {
                let _ = client.shutdown().await;
                Ok(LiveViewClientState::Disconnected)
            }
            msg = client_msg => {
                let Some(msg) = msg else {
                    error!("All client message handlers dropped.");
                    let _ = client.shutdown().await;
                    token.cancel();
                    return Ok(LiveViewClientState::Disconnected);
                };

                match msg {
                    ClientMessage::Reconnect { url, opts, join_params } => {
                        let _ = client.shutdown().await;
                        let client_state = self.create_reconnection_task(url, opts, join_params).await;
                        Ok(client_state)
                    }
                    ClientMessage::Disconnect { response_tx } => {
                        let _ = client.shutdown().await;
                        let _ = response_tx.send(Ok(()));
                        Ok(LiveViewClientState::Disconnected)
                    }
                }
            }
            status_result = socket_status => {

                let status = status_result?.map_err(|web_socket_error|
                    phoenix_channels_client::SocketError::Connect {
                        connect_error: ConnectError::WebSocket { web_socket_error }
                    }
                )?;

                match status {
                    SocketStatus::Connected => {
                        let (con_msg_tx, con_msg_rx) = unbounded_channel();
                        Ok(LiveViewClientState::Connected { con_msg_tx, con_msg_rx, client })
                    },
                    SocketStatus::Disconnected |
                    SocketStatus::NeverConnected |
                    SocketStatus::WaitingToReconnect { .. } => {
                        Ok(LiveViewClientState::Reconnecting { recconecting_client: client })
                    },
                    SocketStatus::ShuttingDown |
                    SocketStatus::ShutDown => {
                        token.cancel();
                        Ok(LiveViewClientState::Disconnected )
                    },
                }
            }
        }
    }

    async fn connecting_loop(
        &self,
        msg_rx: &mut mpsc::UnboundedReceiver<ClientMessage>,
        token: &CancellationToken,
        state: LiveViewClientState,
    ) -> Result<LiveViewClientState, LiveSocketError> {
        let LiveViewClientState::Connecting { job } = state else {
            return Ok(state);
        };

        let token_future = token.cancelled().fuse();
        let client_msg_future = msg_rx.recv().fuse();
        let job_fut = job.fuse();

        pin_mut!(client_msg_future, token_future, job_fut);

        select! {
            _ = token_future => {
                if let Ok(Ok(client)) = job_fut.await {
                    let _ = client.shutdown().await;
                }
                Ok(LiveViewClientState::Disconnected)
            }
            msg = client_msg_future => {
                let Some(msg) = msg else {
                    if let Ok(Ok(client)) = job_fut.await {
                        client.shutdown().await;
                    }

                    return Ok(LiveViewClientState::Disconnected);
                };

                match msg {
                    ClientMessage::Reconnect { url, opts, join_params } => {
                        debug!("Reconnection requested during connecting phase: {}", url);
                        if let Ok(Ok(client)) = job_fut.await {
                            client.shutdown().await;
                        }
                        let client_state = self.create_reconnection_task(url, opts, join_params).await;
                        Ok(client_state)
                    }
                    ClientMessage::Disconnect { response_tx } => {
                        if let Ok(Ok(client)) = job_fut.await {
                            let _ = client.socket.disconnect().await;
                            let _ = client.socket.shutdown().await;
                        }
                        let _ = response_tx.send(Ok(()));
                        Ok(LiveViewClientState::Disconnected)
                    }
                }
            }
            result = job_fut => {
                // The connection job completed
                match result {
                    Ok(Ok(client)) => {
                        let (con_msg_tx, con_msg_rx) = unbounded_channel();
                        Ok(LiveViewClientState::Connected { con_msg_tx, con_msg_rx, client })
                    }
                    Ok(Err(error)) => {
                       Err(error)
                    }
                    Err(join_error) => {
                        error!("join error! {join_error:?}");
                        Err(LiveSocketError::JoinPanic)
                    }
                }
            }
        }
    }

    /// Emits the updated state
    fn update_state(
        &self,
        tag: std::mem::Discriminant<LiveViewClientState>,
        next_state: Result<LiveViewClientState, LiveSocketError>,
    ) -> LiveViewClientState {
        match next_state {
            Ok(state) => {
                if tag == std::mem::discriminant(&state) {
                    return state;
                };

                *self.status.lock().expect("Poisoined Status") = state.status();

                match state {
                    LiveViewClientState::Disconnected => {
                        if let Some(handler) = self.network_handler.as_ref() {
                            handler.on_status_change(FFIClientStatus::Disconnected);
                        }

                        state
                    }
                    LiveViewClientState::Connecting { job } => {
                        if let Some(handler) = self.network_handler.as_ref() {
                            handler.on_status_change(FFIClientStatus::Connecting);
                        }

                        LiveViewClientState::Connecting { job }
                    }
                    LiveViewClientState::Reconnecting {
                        recconecting_client,
                    } => {
                        if let Some(handler) = self.network_handler.as_ref() {
                            handler.on_status_change(FFIClientStatus::Reconnecting);
                        }

                        LiveViewClientState::Reconnecting {
                            recconecting_client,
                        }
                    }
                    LiveViewClientState::Connected {
                        con_msg_tx,
                        con_msg_rx,
                        client,
                    } => {
                        if let Some(handler) = self.document_handler.as_ref() {
                            client.document.arc_set_event_handler(handler.clone());
                        }

                        if let Some(handler) = self.network_handler.as_ref() {
                            handler.on_status_change(FFIClientStatus::Connected {
                                new_document: client.liveview_channel.document().into(),
                            });
                        }

                        LiveViewClientState::Connected {
                            con_msg_tx,
                            con_msg_rx,
                            client,
                        }
                    }
                    LiveViewClientState::FatalError(e) => {
                        if let Some(handler) = self.network_handler.as_ref() {
                            handler.on_status_change(FFIClientStatus::Error {
                                error: e.error.clone(),
                            });
                        }

                        LiveViewClientState::FatalError(e)
                    }
                }
            }
            Err(e) => match e {
                LiveSocketError::ConnectionError(ConnectionError {
                    error_text,
                    error_code,
                    livereload_channel,
                }) => LiveViewClientState::FatalError(FatalError {
                    error: LiveSocketError::ConnectionError(ConnectionError {
                        error_text,
                        error_code,
                        livereload_channel: None,
                    }),
                    channel_events: livereload_channel.as_ref().map(|e| e.channel.events()),
                    livereload_channel,
                }),
                error => LiveViewClientState::FatalError(FatalError {
                    error,
                    livereload_channel: None,
                    channel_events: None,
                }),
            },
        }
    }
}
