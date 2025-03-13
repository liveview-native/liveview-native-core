use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex},
};

use futures::{pin_mut, select, FutureExt};
use log::error;
use phoenix_channels_client::{CallError, Event, Payload, SocketStatus, JSON};
use reqwest::header::CONNECTION;
//use state::{EventLoopState, ReplyAction};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use super::{
    readonly_mutex::ReadOnlyMutex, ClientStatus, ConnectedClient, FatalError, HistoryId,
    LiveViewClientState, NavigationSummary, NetworkEventHandler,
};
use crate::{
    client::LiveViewClientConfiguration,
    error::LiveSocketError,
    live_socket::{navigation::NavOptions, ConnectOpts},
};

const MAX_REDIRECTS: u32 = 10;

/// Messages to the main background event loop
pub enum ClientMessage {
    /// Send a message and wait for the response,
    /// If it is an event it will be processed by the loop
    Call {
        response_tx: oneshot::Sender<Result<Payload, CallError>>,
        event: Event,
        payload: Payload,
    },
    /// Send a message and don't wait for a response
    Cast {
        event: Event,
        payload: Payload,
    },
    Reconnect {
        url: String,
        opts: ConnectOpts,
        join_params: Option<HashMap<String, JSON>>,
    },
    Navigate {
        url: String,
        opts: NavOptions,
    },
    Disconnect {
        response_tx: oneshot::Sender<Result<(), LiveSocketError>>,
    },
}

pub(crate) struct EventLoop {
    pub msg_tx: mpsc::UnboundedSender<ClientMessage>,
    pub cancellation_token: CancellationToken,
    pub status: ReadOnlyMutex<ClientStatus>,
}

pub struct LiveViewClientManager {
    state: LiveViewClientState,
    status: Arc<Mutex<ClientStatus>>,
    network_handler: Option<Arc<dyn NetworkEventHandler>>,
}

impl EventLoop {
    pub fn new(client_state: LiveViewClientState, config: &LiveViewClientConfiguration) -> Self {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let network_handler = config.network_event_handler.clone();
        let cancellation_token = CancellationToken::new();
        let token_clone = cancellation_token.clone();
        let status = Arc::new(Mutex::new(client_state.status()));
        let status_clone = status.clone();

        let manager = LiveViewClientManager {
            state: client_state,
            status: status_clone,
            network_handler,
        };

        tokio::spawn(async move {
            {
                //let client_state_ref = client_state.lock().expect("lock poison");
                // client_state_ref
                //     .refresh_view(Issuer::External(NavigationCall::Initialization), true)
                //     .await;
            }

            // the main event loop
            loop {
                let token_clone = token_clone.cancelled().fuse();
                match manager.state {
                    LiveViewClientState::Disconnected => {
                        let client_msg = msg_rx.recv().fuse();
                        pin_mut!(client_msg, token_clone);
                    }
                    LiveViewClientState::Connecting => {
                        let client_msg = msg_rx.recv().fuse();
                        pin_mut!(client_msg, token_clone);
                    }
                    LiveViewClientState::Reconnecting { ref old_client } => {
                        let client_msg = msg_rx.recv().fuse();

                        let (server_event, socket_status) = match old_client {
                            Some(connected) => {
                                let out = connected.event_futures();
                                (Some(out.0), Some(out.1))
                            }
                            _ => (None, None),
                        };

                        let server_event = async {
                            match server_event {
                                Some(e) => e.await,
                                None => std::future::pending().await,
                            }
                        }
                        .fuse();

                        pin_mut!(client_msg, server_event, socket_status, token_clone,);

                        select! {
                            _ = token_clone => {
                              // client_state.shutdown().await;
                              // let status = status_clone.lock().expect("lock poison");
                               return;
                            }
                            // local control flow and outbound messages
                            message = client_msg => {
                                let Some(msg) = message else {
                                    error!("All client message handlers dropped.");
                                    continue;
                                };
                                //let _ = state.handle_client_message(msg, &mut view_refresh_issuer, &mut socket_reconnected).await;
                            }
                           // networks events from the server
                            event = server_event => {
                                let Ok(payload) = event else {
                                   error!("Error retrieving event from main live channel or live_reload channel: {event:?}");
                                   continue;
                                };

                                // if let Err(e) = state.handle_server_event(&payload, &mut view_refresh_issuer, &mut socket_reconnected).await {
                                //     error!("Failure while handling server reply: {e:?}");
                                // }

                                // state.user_event_callback(payload);
                            }
                           // connectivity changes
                           // new_status = socket_status => {
                           //     match new_status {
                           //         Ok(status) => match status {
                           //          SocketStatus::NeverConnected => todo!(),
                           //          SocketStatus::Connected => todo!(),
                           //          SocketStatus::WaitingToReconnect { .. } => todo!(),
                           //          SocketStatus::Disconnected => todo!(),
                           //          SocketStatus::ShuttingDown => todo!(),
                           //          SocketStatus::ShutDown => todo!(),
                           //      },
                           //         Err(e) => error!("Error fetching liveview status: {e}"),
                           //     }
                           // }
                        }
                    }
                    LiveViewClientState::Connected(ref state) => {
                        let client_msg = msg_rx.recv().fuse();
                        let (server_event, socket_status) = state.event_futures();
                        let (server_event, socket_status) =
                            (server_event.fuse(), socket_status.fuse());

                        pin_mut!(client_msg, server_event, socket_status, token_clone);

                        select! {
                            _ = token_clone => {
                               //client_state_ref.shutdown().await;
                               return;
                            }
                            // local control flow and outbound messages
                           message = client_msg => {
                               let Some(msg) = message else {
                                   error!("All client message handlers dropped.");
                                   continue;
                               };
                               //let _ = state.handle_client_message(msg, &mut view_refresh_issuer, &mut socket_reconnected).await;
                           }
                           // networks events from the server
                           event = server_event => {
                               let Ok(payload) = event else {
                                  error!("Error retrieving event from main live channel or live_reload channel: {event:?}");
                                  continue;
                               };

                               // if let Err(e) = state.handle_server_event(&payload, &mut view_refresh_issuer, &mut socket_reconnected).await {
                               //     error!("Failure while handling server reply: {e:?}");
                               // }

                               // state.user_event_callback(payload);
                           }
                           // connectivity changes
                           new_status = socket_status => {
                               match new_status {
                                   Ok(status) => match status {
                                    SocketStatus::NeverConnected => todo!(),
                                    SocketStatus::Connected => todo!(),
                                    SocketStatus::WaitingToReconnect { until } => todo!(),
                                    SocketStatus::Disconnected => todo!(),
                                    SocketStatus::ShuttingDown => todo!(),
                                    SocketStatus::ShutDown => todo!(),
                                },
                                   Err(e) => error!("Error fetching liveview status: {e}"),
                               }
                           }
                        }
                    }

                    LiveViewClientState::FatalError(FatalError {
                        ref channel_events, ..
                    }) => {
                        let client_msg = msg_rx.recv().fuse();

                        let live_reload_proxy = async {
                            match channel_events {
                                Some(e) => e.event().await,
                                None => std::future::pending().await,
                            }
                        }
                        .fuse();

                        pin_mut!(client_msg, live_reload_proxy, token_clone);

                        select! {
                            _ = token_clone => {
                               //client_state_ref.shutdown().await;
                               return;
                            }
                            message = client_msg => {
                                // *client_state_ref.handle_client_message(message);
                            }
                            event = live_reload_proxy => {
                                let Ok(payload) = event else {
                                   error!("Error retrieving event from live_reload channel while in error state: {event:?}");
                                   continue;
                                };
                               // *client_state_ref.handle_event(event);
                            }
                        }
                    }
                }
            }
        });

        Self {
            msg_tx,
            cancellation_token,
            status: ReadOnlyMutex::new(status),
        }
    }

    // pub async fn handle_navigation_summary(
    //     &self,
    //     summary: Result<NavigationSummary, LiveSocketError>,
    //     issuer: Issuer,
    // ) -> Result<HistoryId, LiveSocketError> {
    //     match summary {
    //         Ok(res) => {
    //             self.refresh_view(res.websocket_reconnected, issuer);
    //             Ok(res.history_id)
    //         }
    //         Err(LiveSocketError::JoinRejection { error }) => {
    //             let mut result = self.handle_navigation_error(&error).await;
    //             let mut retry_count = 0;

    //             while let Err(LiveSocketError::JoinRejection { error }) = &result {
    //                 if retry_count > MAX_REDIRECTS {
    //                     return result;
    //                 }
    //                 result = self.handle_navigation_error(error).await;
    //                 retry_count += 1;
    //             }

    //             if result.is_ok() {
    //                 self.refresh_view(true, issuer);
    //             }

    //             result
    //         }
    //         Err(e) => Err(e),
    //     }
    // }

    ///// During navigation sometimes an error containing `live_redirects` can
    ///// be emitted. these errors are not forwarded to the main event loop by default
    ///// so we forward them here.
    //  async fn handle_navigation_error(
    //      &self,
    //      payload: &Payload,
    //  ) -> Result<HistoryId, LiveSocketError> {
    //      let (tx, result) = oneshot::channel();

    //      // Send navigation error payload to the event loop
    //      let _ = self.msg_tx.send(ClientMessage::HandleSocketReply {
    //          payload: payload.clone(),
    //          tx,
    //      });

    //      // Await a response, if the event loop can rectify a redirect call.
    //      let action = result.await.map_err(|_| LiveSocketError::Call {
    //          error: String::from("Response cancelled while handling navigation error"),
    //      })??;

    //      match action {
    //          ReplyAction::Redirected { summary, .. } => Ok(summary.history_id),
    //          _ => Err(LiveSocketError::JoinRejection {
    //              error: payload.clone(),
    //          }),
    //      }
    //  }
}
