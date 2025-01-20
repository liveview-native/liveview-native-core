use std::sync::Arc;

use futures::{channel::oneshot, pin_mut, select, FutureExt};
use log::error;
use phoenix_channels_client::{CallError, Event, EventPayload, Payload, PhoenixEvent, JSON};
use tokio::sync::mpsc;

use super::{LiveChannelEventHandler, LiveViewClientState};
use crate::{dom::ffi, error::LiveSocketError, live_socket::LiveChannel};

pub struct LiveViewClientChannel {
    /// Allows sending events back to the main event loop
    message_sender: mpsc::UnboundedSender<ClientMessage>,
}

/// RPC to the main background event loop
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
    /// Replace the current channel/
    UpdateChannel,
}

pub(crate) struct EventLoop {
    msg_tx: mpsc::UnboundedSender<ClientMessage>,
    main_background_task: tokio::task::JoinHandle<()>,
}

macro_rules! get_channel {
    ($state:expr, $field:ident) => {
        $state.$field.lock().expect("lock poison").clone()
    };
}

impl EventLoop {
    pub fn new(client_state: Arc<LiveViewClientState>) -> Self {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();

        let mut live_channel = get_channel!(client_state, liveview_channel);
        let mut live_reload_channel = get_channel!(client_state, livereload_channel);

        let patch_handler = client_state.config.patch_handler.clone();
        let live_channel_handler = client_state.config.live_channel_handler.clone();

        if let Some(handler) = &live_channel_handler {
            handler.live_channel_changed();
        }

        let main_background_task = tokio::spawn(async move {
            let mut document = Arc::new(live_channel.document.clone());

            if let Some(handler) = &patch_handler {
                handler.handle_new_document(document.clone());
            }

            let mut server_events = live_channel.channel.events();
            let mut channel_status = live_channel.channel.statuses();
            let mut live_reload_events = live_reload_channel.as_ref().map(|ch| ch.channel.events());

            loop {
                let mut channel_updated = false;

                {
                    let client_msg = msg_rx.recv().fuse();
                    let server_event = server_events.event().fuse();
                    let status = channel_status.status().fuse();

                    let live_reload_event = async {
                        match live_reload_events.as_ref() {
                            Some(events) => events.event().await,
                            // if there is no channel, pend forever
                            None => std::future::pending().await,
                        }
                    }
                    .fuse();

                    pin_mut!(server_event, status, client_msg, live_reload_event);

                    select! {
                       message = client_msg => {
                           let Some(msg) = message else {
                               error!("All client message handlers dropped.");
                               continue;
                           };

                           let _ = handle_client_message(
                               msg,
                               &document,
                               &live_channel,
                               &live_channel_handler,
                               &mut channel_updated,
                           ).await;
                       }
                       event = server_event => {
                           let Ok(payload) = event else {
                              error!("Error retrieving event from live channel: {event:?}");
                              continue;
                           };

                           if let Err(e) = handle_event(&document, &payload, &client_state, &live_channel, &mut channel_updated).await {
                               error!("Failure while handling server reply: {e:?}");
                           }

                           if let Some(handler) = &live_channel_handler {
                               handler.handle_event(payload);
                           }
                       }
                       new_status = status => {
                           if let Ok(status) = new_status {
                               if let Some(handler) = &live_channel_handler {
                                   handler.handle_status_change(status.into());
                               }
                           }
                       }
                       event = live_reload_event => {
                           if let Ok(payload) = event {
                               if let Err(e) = handle_event(&document, &payload, &client_state, &live_channel, &mut channel_updated).await {
                                   error!("Failure while handling live reload event: {e:?}");
                               }

                               if let Some(handler) = &live_channel_handler {
                                   handler.handle_event(payload);
                               }
                           }
                       }
                    };
                }

                // We cannot update these inside the loop because they are pinned.
                if channel_updated {
                    live_channel = get_channel!(client_state, liveview_channel);

                    if let Some(handler) = &live_channel_handler {
                        handler.live_channel_changed();
                    }

                    document = Arc::new(live_channel.document.clone());

                    if let Some(handler) = &patch_handler {
                        handler.handle_new_document(document.clone());
                    }

                    server_events = live_channel.channel.events();
                    channel_status = live_channel.channel.statuses();

                    live_reload_channel = get_channel!(client_state, livereload_channel);
                    live_reload_events = live_reload_channel.as_ref().map(|ch| ch.channel.events());
                }
            }
        });

        Self {
            msg_tx,
            main_background_task,
        }
    }

    pub fn replace_livechannel(&self) {
        let _ = self.msg_tx.send(ClientMessage::UpdateChannel);
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

async fn handle_client_message(
    message: ClientMessage,
    document: &ffi::Document,
    live_channel: &Arc<LiveChannel>,
    event_handler: &Option<Arc<dyn LiveChannelEventHandler>>,
    channel_updated: &mut bool,
) {
    match message {
        ClientMessage::Call {
            response_tx,
            event,
            payload,
        } => {
            let call_result = live_channel
                .channel
                .call(event, payload, live_channel.timeout)
                .await;

            match call_result {
                Ok(reply) => {
                    if let Err(e) = handle_reply(&document, &reply) {
                        error!("Failure while handling server reply: {e:?}");
                    }

                    if let Some(handler) = &event_handler {
                        let event = EventPayload {
                            event: Event::Phoenix {
                                phoenix: PhoenixEvent::Reply,
                            },
                            payload: reply.clone(),
                        };
                        handler.handle_event(event);
                    }

                    let _ = response_tx.send(Ok(reply));
                }
                Err(e) => {
                    error!("Remote call returned error: {e:?}");
                    let _ = response_tx.send(Err(e));
                }
            }
        }
        ClientMessage::Cast { event, payload } => {
            let _ = live_channel.channel.cast(event, payload).await;
        }
        ClientMessage::UpdateChannel => {
            *channel_updated = true;
        }
    }
}

/// Applies diffs in server events
async fn handle_event(
    document: &ffi::Document,
    event: &EventPayload,
    client: &LiveViewClientState,
    current_channel: &LiveChannel,
    channel_updated: &mut bool,
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

                document.merge_deserialized_fragment_json(json.clone())?;
            }
            // TODO: Handle this
            "live_patch" => {
                let Payload::JSONPayload { .. } = &event.payload else {
                    error!("Live patch was not json!");
                    return Ok(());
                };
            }
            // TODO: Handle this
            "live_redirect" => {
                let Payload::JSONPayload { .. } = &event.payload else {
                    error!("Live redirect was not json!");
                    return Ok(());
                };
            }
            // TODO: Handle this
            "redirect" => {
                let Payload::JSONPayload { .. } = &event.payload else {
                    error!("Live redirect was not json!");
                    return Ok(());
                };
            }
            "assets_change" => {
                let Some(current_entry) = client.current_history_entry() else {
                    // TODO: error
                    return Ok(());
                };
                let opts = client.session_data.try_lock()?.connect_opts.clone();
                let join_params = current_channel.join_params.clone();

                client
                    .reconnect(current_entry.url, opts, Some(join_params))
                    .await?;

                *channel_updated = true;
            }
            _ => {}
        },
    };

    Ok(())
}

/// Helper function to apply diffs from reply payloads.
fn handle_reply(document: &ffi::Document, reply: &Payload) -> Result<(), LiveSocketError> {
    let Payload::JSONPayload { json } = reply else {
        return Ok(());
    };

    let JSON::Object { object } = json else {
        return Ok(());
    };

    if let Some(diff) = object.get("diff") {
        document.merge_deserialized_fragment_json(diff.clone())?;
    };

    // TODO: handle these
    // if let Some(_) = object.get("live_patch") {}
    // if let Some(_) = object.get("live_redirect") {}
    // if let Some(_) = object.get("redirect") {}
    // if let Some(_) = object.get("assets_change") {}

    Ok(())
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
