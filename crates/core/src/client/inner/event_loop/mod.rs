mod state;

use std::sync::Arc;

use futures::{channel::oneshot, pin_mut, select, FutureExt};
use log::error;
use phoenix_channels_client::{CallError, Event, Payload};
use state::{EventLoopState, ReplyAction};
use tokio::sync::mpsc;

use super::{HistoryId, LiveViewClientState, NavigationSummary, NetworkEventHandler};
use crate::error::LiveSocketError;

const MAX_REDIRECTS: u32 = 10;

pub struct LiveViewClientChannel {
    /// Allows sending events back to the main event loop
    message_sender: mpsc::UnboundedSender<ClientMessage>,
}

impl LiveViewClientChannel {
    pub async fn call(
        &self,
        event_name: String,
        payload: Payload,
    ) -> Result<Payload, LiveSocketError> {
        let (response_tx, response_rx) = oneshot::channel();

        let _ = self.message_sender.send(ClientMessage::Call {
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
        let _ = self.message_sender.send(ClientMessage::Cast {
            event: Event::from_string(event_name),
            payload,
        });
    }
}

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
    Cast { event: Event, payload: Payload },
    /// Replace the current channel
    RefreshView { socket_reconnected: bool },
    /// For internal use, error events are not broadcast
    /// from the socket when you attempt to connect to a channel.
    /// So we reinject them into the event loop with these messages
    HandleSocketReply {
        payload: Payload,
        tx: oneshot::Sender<Result<ReplyAction, LiveSocketError>>,
    },
}

pub(crate) struct EventLoop {
    msg_tx: mpsc::UnboundedSender<ClientMessage>,
    main_background_task: tokio::task::JoinHandle<()>,
}

impl EventLoop {
    pub fn new(client_state: Arc<LiveViewClientState>) -> Self {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
        let mut state = EventLoopState::new(client_state);

        state.refresh_view(true);

        let main_background_task = tokio::spawn(async move {
            // the main event loop
            loop {
                let mut view_refresh_needed = false;
                let mut socket_reconnected = false;

                {
                    let client_msg = msg_rx.recv().fuse();
                    let (server_event, chan_status, socket_status) = state.event_futures();
                    let (server_event, chan_status, socket_status) = (
                        server_event.fuse(),
                        chan_status.fuse(),
                        socket_status.fuse(),
                    );

                    pin_mut!(client_msg, server_event, chan_status, socket_status);

                    select! {
                        // local control flow and outbound messages
                       message = client_msg => {
                           let Some(msg) = message else {
                               error!("All client message handlers dropped.");
                               continue;
                           };
                           let _ = state.handle_client_message(msg, &mut view_refresh_needed, &mut socket_reconnected).await;
                       }
                       // networks events from the server
                       event = server_event => {
                           let Ok(payload) = event else {
                              error!("Error retrieving event from main live channel or live_reload channel: {event:?}");
                              continue;
                           };

                           if let Err(e) = state.handle_server_event(&payload, &mut view_refresh_needed, &mut socket_reconnected).await {
                               error!("Failure while handling server reply: {e:?}");
                           }

                           state.user_event_callback(payload);
                       }
                       // connectivity changes
                       new_status = chan_status => {
                           match new_status {
                               Ok(status) => state.user_channel_callback(status.into()),
                               Err(e) => error!("Error fetching liveview status: {e}"),
                           }
                       }
                       new_status = socket_status => {
                           match new_status {
                               Ok(status) => state.user_socket_callback(status),
                               Err(e) => error!("Error fetching liveview status: {e}"),
                           }
                       }
                    }
                }

                if view_refresh_needed {
                    state.refresh_view(socket_reconnected);
                }
            }
        });

        Self {
            msg_tx,
            main_background_task,
        }
    }

    /// This must be called after any function which successfully
    /// changes the underlying live channel, if `socket_reconnected` is true
    /// listeners to refresh events will also be notified. `socket_reconnected` is
    /// the equivalent of a full liveview reload like a `live_redirect` or an web
    /// page reload.
    pub fn refresh_view(&self, socket_reconnected: bool) {
        let _ = self
            .msg_tx
            .send(ClientMessage::RefreshView { socket_reconnected });
    }

    pub async fn handle_navigation_summary(
        &self,
        summary: Result<NavigationSummary, LiveSocketError>,
    ) -> Result<HistoryId, LiveSocketError> {
        match summary {
            Ok(res) => {
                self.refresh_view(res.websocket_reconnected);
                Ok(res.history_id)
            }
            Err(LiveSocketError::JoinRejection { error }) => {
                let mut result = self.handle_navigation_error(&error).await;
                let mut retry_count = 0;

                while let Err(LiveSocketError::JoinRejection { error }) = &result {
                    if retry_count > MAX_REDIRECTS {
                        return result;
                    }
                    result = self.handle_navigation_error(error).await;
                    retry_count += 1;
                }

                result
            }
            Err(e) => Err(e),
        }
    }

    /// During navigation sometimes an error containing `live_redirects` can
    /// be emitted. these errors are not forwarded to the main event loop by default
    /// so we forward them here.
    async fn handle_navigation_error(
        &self,
        payload: &Payload,
    ) -> Result<HistoryId, LiveSocketError> {
        let (tx, result) = oneshot::channel();

        let _ = self.msg_tx.send(ClientMessage::HandleSocketReply {
            payload: payload.clone(),
            tx,
        });

        let action = result.await.map_err(|_| LiveSocketError::Call {
            error: String::from("Response cancelled while handling navigation error"),
        })??;

        match action {
            ReplyAction::Redirected { id } => Ok(id),
            _ => Err(LiveSocketError::JoinRejection {
                error: payload.clone(),
            }),
        }
    }

    pub fn create_handle(&self) -> LiveViewClientChannel {
        let msg_tx = self.msg_tx.clone();
        LiveViewClientChannel {
            message_sender: msg_tx,
        }
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        self.main_background_task.abort();
    }
}
