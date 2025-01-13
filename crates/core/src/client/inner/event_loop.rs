use std::sync::Arc;

use futures::{channel::oneshot, pin_mut, select, FutureExt};
use log::error;
use phoenix_channels_client::{CallError, Event, EventPayload, Payload, PhoenixEvent, JSON};
use tokio::sync::mpsc;

use super::LiveViewClientState;
use crate::{dom::ffi, error::LiveSocketError};

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

impl EventLoop {
    pub fn new(client_state: Arc<LiveViewClientState>) -> Self {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();

        let mut live_channel = client_state
            .liveview_channel
            .lock()
            .expect("lock poison")
            .clone();

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

            loop {
                let mut channel_updated = false;

                {
                    let client_msg = msg_rx.recv().fuse();
                    let server_event = server_events.event().fuse();
                    let status = channel_status.status().fuse();

                    pin_mut!(server_event, status, client_msg);

                    select! {
                       message = client_msg => {
                           let Some(msg) = message else {
                               error!("All client message handlers dropped.");
                               continue;
                           };

                            match msg {
                                ClientMessage::Call { response_tx, event, payload } => {
                                    let call_result = live_channel.channel.call(event, payload, live_channel.timeout).await;

                                    if let Ok(reply) = call_result {
                                        if let Err(e) = handle_reply(&document, &reply) {
                                           error!("Failure while handling server reply: {e:?}");
                                        }

                                        if let Some(handler) = &live_channel_handler {
                                            let event = EventPayload {
                                                    event: Event::Phoenix {
                                                        phoenix: PhoenixEvent::Reply,
                                                    },
                                                    payload: reply.clone(),
                                            };
                                           handler.handle_event(event);
                                        }

                                        let _ = response_tx.send(Ok(reply));
                                    } else {
                                        error!("Remote call returned error: {call_result:?}");
                                        let _ = response_tx.send(call_result);
                                    }
                                },
                                ClientMessage::Cast { event, payload } => {
                                    // TODO: warn or error here
                                    let _ = live_channel.channel.cast(event, payload).await;
                                },
                                ClientMessage::UpdateChannel => {
                                    live_channel = client_state
                                        .liveview_channel
                                        .lock()
                                        .expect("lock poison")
                                        .clone();

                                    if let Some(handler) = &live_channel_handler {
                                        handler.live_channel_changed();
                                        handler.handle_status_change(live_channel.channel.status().into());
                                    }

                                    channel_updated = true;
                                },
                            }
                       }
                       event = server_event => {
                           let Ok(payload) = event else {
                              error!("Error retrieving event from live channel: {event:?}");
                              continue;
                           };

                           if let Err(e) = handle_event(&document, &payload) {
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
                    };
                }

                // We cannot update these inside the loop because they are pinned.
                if channel_updated {
                    document = Arc::new(live_channel.document.clone());

                    if let Some(handler) = &patch_handler {
                        handler.handle_new_document(document.clone());
                    }

                    server_events = live_channel.channel.events();
                    channel_status = live_channel.channel.statuses();
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

/// Applies diffs in server events
fn handle_event(document: &ffi::Document, event: &EventPayload) -> Result<(), LiveSocketError> {
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
            // TODO: Handle these
            "live_patch" => {}
            "live_redirect" => {}
            "redirect" => {}
            "assets_change" => {}
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
    pub async fn call(&self, event: Event, payload: Payload) -> Result<Payload, LiveSocketError> {
        let (response_tx, response_rx) = oneshot::channel();

        let _ = self.message_sender.send(ClientMessage::Call {
            response_tx,
            event,
            payload,
        });

        let resp = response_rx.await.map_err(|_| LiveSocketError::Call)??;

        Ok(resp)
    }

    pub async fn cast(&self, event: Event, payload: Payload) {
        let _ = self
            .message_sender
            .send(ClientMessage::Cast { event, payload });
    }
}
