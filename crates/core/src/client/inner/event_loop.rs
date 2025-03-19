use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{pin_mut, select, FutureExt};
use log::{debug, error};
use phoenix_channels_client::{
    CallError, ConnectError, Event, EventPayload, Payload, PhoenixEvent, SocketStatus, JSON,
};
use reqwest::{Client, Url};
use tokio::sync::{
    mpsc::{self, unbounded_channel, UnboundedSender},
    oneshot, watch,
};
use tokio_util::sync::CancellationToken;

use super::{
    cookie_store::PersistentCookieStore, navigation::NavCtx, ClientStatus, ConnectedClient,
    DocumentChangeHandler, FatalError, LiveViewClientState, NetworkEventHandler,
};
use crate::{
    client::{ClientStatus as FFIClientStatus, LiveViewClientConfiguration},
    error::{ConnectionError, LiveSocketError},
    live_socket::{
        navigation::{NavAction, NavOptions},
        ConnectOpts, LiveFile,
    },
    protocol::{LiveRedirect, RedirectKind},
};

pub(crate) struct EventLoop {
    pub msg_tx: mpsc::UnboundedSender<ClientMessage>,
    pub cancellation_token: CancellationToken,
    pub status: watch::Receiver<ClientStatus>,
}

impl EventLoop {
    pub async fn new(
        config: LiveViewClientConfiguration,
        url: &str,
        cookie_store: Arc<PersistentCookieStore>,
        http_client: Client,
        client_opts: crate::client::ClientConnectOpts,
        nav_ctx: Arc<Mutex<NavCtx>>,
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
        let msg_tx_clone = msg_tx.clone();
        let cancellation_token = CancellationToken::new();
        let token_clone = cancellation_token.clone();
        let (status_tx, status_rx) = tokio::sync::watch::channel(client_state.status());

        let manager = LiveViewClientManager::new(
            status_tx,
            config,
            http_client,
            cookie_store,
            nav_ctx,
            msg_tx_clone,
        );

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
            status: status_rx,
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
    UploadFile {
        file: Arc<LiveFile>,
        response_tx: oneshot::Sender<Result<(), LiveSocketError>>,
    },
    Cast {
        event: Event,
        payload: Payload,
    },
    Navigate {
        url: String,
        opts: NavOptions,
        /// true if the request to navigate originated from outside the server, as opposed
        /// to from the client
        remote: bool,
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
        /// True if the reconnect was triggered by the server, such as a `redirect` directive
        remote: bool,
    },
    Disconnect {
        response_tx: oneshot::Sender<Result<(), LiveSocketError>>,
    },
}

pub struct LiveViewClientManager {
    status: watch::Sender<ClientStatus>,
    network_handler: Option<Arc<dyn NetworkEventHandler>>,
    document_handler: Option<Arc<dyn DocumentChangeHandler>>,
    config: Arc<LiveViewClientConfiguration>,
    /// Navigation book keeping context shared with the client handle.
    nav_ctx: Arc<Mutex<NavCtx>>,
    cookie_store: Arc<PersistentCookieStore>,
    http_client: Client,
    receiver: UnboundedSender<ClientMessage>,
}

impl LiveViewClientManager {
    pub fn new(
        status: watch::Sender<ClientStatus>,
        config: Arc<LiveViewClientConfiguration>,
        http_client: Client,
        cookie_store: Arc<PersistentCookieStore>,
        nav_ctx: Arc<Mutex<NavCtx>>,
        receiver: UnboundedSender<ClientMessage>,
    ) -> Self {
        Self {
            status,
            network_handler: config.network_event_handler.clone(),
            document_handler: config.patch_handler.clone(),
            config,
            http_client,
            cookie_store,
            nav_ctx,
            receiver,
        }
    }

    async fn create_connection_task(
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
        state: LiveViewClientState,
    ) -> Result<LiveViewClientState, LiveSocketError> {
        let LiveViewClientState::Connected {
            mut con_msg_rx,
            con_msg_tx,
            mut client,
        } = state
        else {
            return Ok(state);
        };

        let token_fut = token.cancelled().fuse();

        pin_mut!(token_fut);

        select! {
           _ = token_fut => {
              client.shutdown().await;
              Ok(LiveViewClientState::Disconnected)
           }
           msg = msg_rx.recv().fuse() => {
            let Some(msg) = msg else {
                error!("All client message handlers dropped during error state.");
                client.shutdown().await;
                token.cancel();
                return Ok(LiveViewClientState::Disconnected);
            };

            match msg {
                ClientMessage::Reconnect { url, opts, join_params, remote } => {
                    let bail = if remote {
                        let url = Url::parse(&url)?;

                        let opts = NavOptions {
                            join_params: join_params.clone(),
                            ..Default::default()
                        };

                        self.nav_ctx.lock().expect("lock poison").navigate(url, opts, true).is_err()
                    } else {
                        false
                    };

                    if !bail {
                        client.shutdown().await;
                        let client_state = self.create_connection_task(url, opts, join_params).await;
                        Ok(client_state)
                    } else {
                        Ok(LiveViewClientState::Connected { con_msg_tx, con_msg_rx, client })
                    }
                },
                ClientMessage::Disconnect { response_tx } => {
                    // TODO: propogate error...
                    client.shutdown().await;
                    let _ = response_tx.send(Ok(()));
                    Ok(LiveViewClientState::Disconnected)
                },
            }

           }
           msg = con_msg_rx.recv().fuse() => {
            let Some(msg) = msg else {
                // should never occur
                error!("All connected client handlers dropped during error state.");
                client.shutdown().await;
                token.cancel();
                return Ok(LiveViewClientState::Disconnected);
            };

            match msg {
                ConnectedClientMessage::Call { event, payload, response_tx } => {
                    let dur = Duration::from_millis(self.config.websocket_timeout);
                    let out = client.liveview_channel.channel.call(event, payload, dur).await;

                    if let Ok(Payload::JSONPayload { json }) = &out {
                        let _ = self.handle_server_response(&client, json, &con_msg_tx);
                    }

                    let _ = response_tx.send(out);
                },
                ConnectedClientMessage::Cast { event, payload } => {
                     let _ = client.liveview_channel.channel.cast(event, payload).await;
                },
                ConnectedClientMessage::Navigate { url, opts, remote } => {
                    let bail = if remote {
                        let url = Url::parse(&url)?;
                        self.nav_ctx.lock().expect("lock poison").navigate(url, opts.clone(), true).is_err()
                    } else {
                        false
                    };

                    if !bail {
                        let nav_res = self.handle_navigation(&mut client, url, opts).await;


                        if let Err(e) = nav_res {
                            match e {
                               LiveSocketError::JoinRejection { error }  => {
                                   if let Payload::JSONPayload { json } = error {
                                       // TODO: make sure that a redirect happened
                                       let _ = self.handle_server_response(&mut client, &json, &con_msg_tx);
                                   }
                               }
                               _ => {
                                   client.shutdown().await;
                                   return Err(e);
                               }
                            }
                        } else {
                            let (con_msg_tx, con_msg_rx) = unbounded_channel();
                            let out = LiveViewClientState::Connected { con_msg_tx, con_msg_rx, client };
                            let _ = self.status.send(out.status());
                            return Ok(out);
                        };
                    }

                },
                ConnectedClientMessage::UploadFile {file, response_tx } => {
                    let e =  client.liveview_channel.upload_file(&file).await;
                    let _ = response_tx.send(e);
                }
            }
            Ok(LiveViewClientState::Connected { con_msg_tx, con_msg_rx, client })
           }
           event = client.event_futures().0.fuse() => {
               if let Ok(event) = event {
                   let message_res = self.handle_server_event(&mut client, &event, &con_msg_tx);

                   if let Err(e) = message_res {
                       client.shutdown().await;
                       return Err(e);
                   };
               }
               Ok(LiveViewClientState::Connected { con_msg_tx, con_msg_rx, client })
           }
           status = client.event_futures().1.fuse() => {

               let status = status.map_err(|web_socket_error|
                   phoenix_channels_client::SocketError::Connect {
                       connect_error: ConnectError::WebSocket { web_socket_error }
                   }
               )?;

               match status {
                   SocketStatus::Connected => {
                       Ok(LiveViewClientState::Connected { con_msg_tx, con_msg_rx, client })
                   }
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

    fn handle_server_response(
        &self,
        client: &ConnectedClient,
        response: &JSON,
        con_msg_tx: &UnboundedSender<ConnectedClientMessage>,
    ) -> Result<(), LiveSocketError> {
        if let Some(handler) = self.network_handler.as_ref() {
            let event = EventPayload {
                event: Event::Phoenix {
                    phoenix: PhoenixEvent::Reply,
                },
                payload: Payload::JSONPayload {
                    json: response.clone(),
                },
            };
            handler.on_event(event)
        }

        let JSON::Object { object } = response else {
            return Ok(());
        };

        if let Some(redirect_json) = object.get("live_redirect") {
            let json = redirect_json.clone().into();
            let redirect: LiveRedirect = serde_json::from_value(json)?;
            let base_url = client.session_data.url.clone();
            let target_url = base_url.join(&redirect.to)?;

            let action = match redirect.kind {
                Some(RedirectKind::Push) | None => NavAction::Push,
                Some(RedirectKind::Replace) => NavAction::Replace,
            };

            let opts = NavOptions {
                action: Some(action),
                join_params: client.liveview_channel.join_params.clone().into(),
                ..NavOptions::default()
            };

            debug!("`live_redirect` received in reply - server push navigation to {target_url:?}");
            let _ = con_msg_tx.send(ConnectedClientMessage::Navigate {
                url: target_url.to_string(),
                opts,
                remote: true,
            });
        }

        if let Some(redirect_json) = object.get("redirect") {
            let json = redirect_json.clone().into();
            let redirect: LiveRedirect = serde_json::from_value(json)?;

            let base_url = client.session_data.url.clone();
            let target_url = base_url.join(&redirect.to)?;

            let connect_opts = ConnectOpts {
                timeout_ms: self.config.dead_render_timeout,
                ..ConnectOpts::default()
            };

            let join_params = client.liveview_channel.join_params.clone();

            debug!("`redirect` received in reply - reconencting to {target_url:?}");
            let _ = self.receiver.send(ClientMessage::Reconnect {
                url: target_url.to_string(),
                opts: connect_opts,
                join_params: Some(join_params),
                remote: true,
            });
        }

        // Handle diffs
        if let Some(diff) = object.get("diff") {
            client
                .document
                .merge_deserialized_fragment_json(diff.clone())?;
        }

        Ok(())
    }

    async fn handle_navigation(
        &self,
        client: &mut ConnectedClient,
        url: String,
        opts: NavOptions,
    ) -> Result<(), LiveSocketError> {
        // Try to perform the navigation
        client
            .try_nav(&self.http_client, &self.config, &opts.join_params, url)
            .await?;

        // Set up handlers after successful navigation
        if let Some(handler) = self.document_handler.as_ref() {
            client.document.arc_set_event_handler(handler.clone());
        }

        if let Some(handler) = self.network_handler.as_ref() {
            handler.on_status_change(FFIClientStatus::Connected {
                new_document: client.liveview_channel.document().into(),
            });
        }

        Ok(())
    }

    fn handle_server_event(
        &self,
        client: &mut ConnectedClient,
        event: &EventPayload,
        con_msg_tx: &UnboundedSender<ConnectedClientMessage>,
    ) -> Result<(), LiveSocketError> {
        match &event.event {
            Event::Phoenix { phoenix } => {
                error!("Phoenix Event for {phoenix:?} is unimplemented");
            }
            Event::User { user } => match user.as_str() {
                "diff" => {
                    let Payload::JSONPayload { json } = &event.payload else {
                        error!("Diff was not json!");
                        return Ok(());
                    };

                    client
                        .document
                        .merge_deserialized_fragment_json(json.clone())?;
                }
                "assets_change" => {
                    let url = client.session_data.url.to_string();
                    let connect_opts = ConnectOpts {
                        timeout_ms: self.config.dead_render_timeout,
                        ..ConnectOpts::default()
                    };

                    let join_params = client.liveview_channel.join_params.clone();

                    let _ = self.receiver.send(ClientMessage::Reconnect {
                        url,
                        opts: connect_opts,
                        join_params: Some(join_params),
                        remote: true,
                    });
                }
                "live_patch" => {
                    let Payload::JSONPayload { json, .. } = &event.payload else {
                        error!("Live patch was not json!");
                        return Ok(());
                    };

                    let json_value = json.clone().into();
                    let redirect: LiveRedirect = serde_json::from_value(json_value)?;

                    let base_url = client.session_data.url.clone();
                    let url = base_url.join(&redirect.to)?;

                    client.session_data.url = url;
                }
                "live_redirect" => {
                    let Payload::JSONPayload { json, .. } = &event.payload else {
                        error!("Live redirect was not json!");
                        return Ok(());
                    };

                    let json_value = json.clone().into();
                    let redirect: LiveRedirect = serde_json::from_value(json_value)?;

                    let base_url = client.session_data.url.clone();
                    let target_url = base_url.join(&redirect.to)?;

                    let action = match redirect.kind {
                        Some(RedirectKind::Push) | None => NavAction::Push,
                        Some(RedirectKind::Replace) => NavAction::Replace,
                    };

                    let opts = NavOptions {
                        action: Some(action),
                        join_params: client.liveview_channel.join_params.clone().into(),
                        ..NavOptions::default()
                    };

                    let _ = con_msg_tx.send(ConnectedClientMessage::Navigate {
                        url: target_url.to_string(),
                        opts,
                        remote: true,
                    });
                }
                "redirect" => {
                    let Payload::JSONPayload { json, .. } = &event.payload else {
                        error!("Redirect was not json!");
                        return Ok(());
                    };

                    let json_value = json.clone().into();
                    let redirect: LiveRedirect = serde_json::from_value(json_value)?;

                    // Get the target URL
                    let base_url = client.session_data.url.clone();
                    let target_url = base_url.join(&redirect.to)?;

                    let connect_opts = ConnectOpts {
                        timeout_ms: self.config.dead_render_timeout,
                        ..ConnectOpts::default()
                    };

                    let join_params = client.liveview_channel.join_params.clone();

                    let _ = self.receiver.send(ClientMessage::Reconnect {
                        url: target_url.to_string(),
                        opts: connect_opts,
                        join_params: Some(join_params),
                        remote: true,
                    });
                }
                _ => {
                    debug!("Unhandled user event: {}", user);
                }
            },
        };

        Ok(())
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
                   ClientMessage::Reconnect { url, opts, join_params, .. } => {
                       debug!("Reconnection requested to URL: {}", url);
                       let client_state = self.create_connection_task(url, opts, join_params).await;
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
        let LiveViewClientState::FatalError(error) = state else {
            return Ok(state);
        };

        let token_future = token.cancelled().fuse();
        let client_msg = msg_rx.recv().fuse();

        // Set up the live reload event future if channel_events is present
        let live_reload_proxy = async {
            match error.channel_events.clone() {
                Some(e) => e.event().await,
                None => std::future::pending().await,
            }
        }
        .fuse();

        pin_mut!(client_msg, live_reload_proxy, token_future);

        select! {
            _ = token_future => {
                // If cancellation is requested, clean up any resources if needed
                if let Some(lr_channel) = error.livereload_channel {
                    let _ = lr_channel.channel.leave().await;
                }
                Ok(LiveViewClientState::Disconnected)
            }

            msg = client_msg => {
                let Some(msg) = msg else {
                    error!("All client message handlers dropped during error state.");
                    token.cancel();
                    return Ok(LiveViewClientState::Disconnected);
                };

                match msg {
                    ClientMessage::Reconnect { url, opts, join_params, .. } => {
                        if let Some(lr_channel) = error.livereload_channel {
                            let _ = lr_channel.channel.leave().await;
                        }

                        let client_state = self.create_connection_task(url, opts, join_params).await;
                        Ok(client_state)
                    },
                    ClientMessage::Disconnect { response_tx } => {
                        if let Some(lr_channel) = error.livereload_channel {
                            let _ = lr_channel.channel.leave().await;
                        }

                        let _ = response_tx.send(Ok(()));
                        Ok(LiveViewClientState::Disconnected)
                    },
                }
            }
            event = live_reload_proxy => {
                todo!();
                Ok(LiveViewClientState::FatalError(error.clone()))
            }
        }
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
                    ClientMessage::Reconnect { url, opts, join_params, .. } => {
                        let _ = client.shutdown().await;
                        let client_state = self.create_connection_task(url, opts, join_params).await;
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
                    ClientMessage::Reconnect { url, opts, join_params, .. } => {
                        debug!("Reconnection requested during connecting phase: {}", url);
                        if let Ok(Ok(client)) = job_fut.await {
                            client.shutdown().await;
                        }
                        let client_state = self.create_connection_task(url, opts, join_params).await;
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

                let status = state.status();
                debug!("Updating status to {}", status.name());
                let _ = self.status.send(status);

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
