use std::{future::Future, sync::Arc};

use futures::FutureExt;
use log::error;
use phoenix_channels_client::{
    CallError, ChannelStatus, ChannelStatuses, Event, EventPayload, Events, EventsError, Payload,
    PhoenixEvent, Socket, SocketStatus, SocketStatuses, StatusesError, WebSocketError, JSON,
};
use tokio::select;

use super::{ClientMessage, LiveViewClientState, NetworkEventHandler};
use crate::{
    client::{HistoryId, LiveChannelStatus},
    dom::ffi::{self, Document},
    error::LiveSocketError,
    live_socket::{
        navigation::{NavAction, NavOptions},
        LiveChannel,
    },
    protocol::{LiveRedirect, RedirectKind, RedirectMode},
};

pub enum ReplyAction {
    Redirected { id: HistoryId },
    DiffMerged,
    None,
}

pub struct EventLoopState {
    /// The DOM for the current view
    document: ffi::Document,
    /// The main live view channel which handles the document updates
    live_view_channel: ChannelState,
    /// The live view channel which handles asset changes on the server for reloading purposes
    live_reload: Option<ChannelState>,
    /// The current socket status stream
    socket_statuses: Arc<SocketStatuses>,
    network_handler: Option<Arc<dyn NetworkEventHandler>>,
    client_state: Arc<LiveViewClientState>,
}

struct ChannelState {
    channel: Arc<LiveChannel>,
    events: Arc<Events>,
    statuses: Arc<ChannelStatuses>,
}

impl From<Arc<LiveChannel>> for ChannelState {
    fn from(value: Arc<LiveChannel>) -> Self {
        ChannelState {
            events: value.channel.events(),
            statuses: value.channel.statuses(),
            channel: value,
        }
    }
}

impl EventLoopState {
    pub fn new(client_state: Arc<LiveViewClientState>) -> Self {
        let channel = client_state
            .liveview_channel
            .lock()
            .expect("lock poison")
            .clone();

        let live_reload = client_state
            .livereload_channel
            .lock()
            .expect("lock poison")
            .clone()
            .map(|c| c.into());

        Self {
            document: channel.document.clone(),
            live_reload,
            socket_statuses: channel.socket.statuses(),
            live_view_channel: channel.into(),
            network_handler: client_state.config.network_event_handler.clone(),
            client_state,
        }
    }

    async fn cast(&self, event: Event, payload: Payload) -> Result<(), LiveSocketError> {
        self.live_view_channel
            .channel
            .channel
            .cast(event, payload)
            .await
            .map_err(|e| LiveSocketError::Cast {
                error: format!("{e:?}"),
            })?;

        Ok(())
    }

    async fn call(&self, event: Event, payload: Payload) -> Result<Payload, CallError> {
        let timeout = self.live_view_channel.channel.timeout;
        let res = self
            .live_view_channel
            .channel
            .channel
            .call(event, payload, timeout)
            .await?;
        Ok(res)
    }

    /// Called when the owning `LiveViewClient` has been updated
    /// and has a new valid live channel - livereaload channel, and/or live socket.
    pub fn refresh_view(&mut self, socket_reconnect: bool) {
        let new_live_channel = self.client_state.liveview_channel.lock().unwrap().clone();
        self.socket_statuses = new_live_channel.socket.statuses();
        self.live_view_channel = ChannelState::from(new_live_channel.clone());
        self.document = new_live_channel.document.clone();

        if let Some(doc_change) = &self.client_state.config.patch_handler {
            self.document.arc_set_event_handler(doc_change.clone())
        }

        let new_livereload_channel = self.client_state.livereload_channel.lock().unwrap().clone();
        self.live_reload = new_livereload_channel.map(ChannelState::from);

        self.on_reload(
            self.document.clone().into(),
            self.live_view_channel.channel.clone(),
            self.live_view_channel.channel.socket.clone(),
            socket_reconnect,
        )
    }

    pub fn event_futures(
        &self,
    ) -> (
        impl Future<Output = Result<EventPayload, EventsError>> + '_,
        impl Future<Output = Result<ChannelStatus, StatusesError>> + '_,
        impl Future<Output = Result<SocketStatus, WebSocketError>> + '_,
    ) {
        let server_event = self.live_view_channel.events.event().fuse();
        let chan_status = self.live_view_channel.statuses.status().fuse();
        let maybe_reload_event = self.live_reload.as_ref().map(|l| l.events.event());

        let socket_status = self.socket_statuses.status();

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

        (server_event, chan_status, socket_status)
    }

    /// Call the user provided call back for receiving a
    pub fn on_event(&self, event: EventPayload) {
        if let Some(handler) = &self.network_handler {
            handler.handle_event(event);
        }
    }

    pub fn on_channel_status(&self, status: LiveChannelStatus) {
        if let Some(handler) = &self.network_handler {
            handler.handle_channel_status_change(status);
        }
    }

    pub fn on_socket_status(&self, status: SocketStatus) {
        if let Some(handler) = &self.network_handler {
            handler.handle_socket_status_change(status);
        }
    }

    pub fn on_reload(
        &self,
        new_document: Arc<Document>,
        new_channel: Arc<LiveChannel>,
        current_socket: Arc<Socket>,
        socket_is_new: bool,
    ) {
        if let Some(handler) = &self.network_handler {
            handler.handle_view_reloaded(new_document, new_channel, current_socket, socket_is_new);
        }
    }

    pub async fn handle_client_message(
        &self,
        message: ClientMessage,
        channel_updated: &mut bool,
        socket_updated: &mut bool,
    ) {
        match message {
            ClientMessage::Call {
                response_tx,
                event,
                payload,
            } => {
                let call_result = self.call(event, payload).await;

                match call_result {
                    Ok(reply) => {
                        if let Err(e) = self
                            .handle_reply(&reply, channel_updated, socket_updated)
                            .await
                        {
                            error!("Failure while handling server reply: {e:?}");
                        }

                        let event = EventPayload {
                            event: Event::Phoenix {
                                phoenix: PhoenixEvent::Reply,
                            },
                            payload: reply.clone(),
                        };

                        self.on_event(event);

                        let _ = response_tx.send(Ok(reply));
                    }
                    Err(e) => {
                        error!("Remote call returned error: {e:?}");
                        let _ = response_tx.send(Err(e));
                    }
                }
            }
            ClientMessage::Cast { event, payload } => {
                let _ = self.cast(event, payload).await;
            }
            ClientMessage::RefreshView { socket_reconnected } => {
                *channel_updated = true;
                *socket_updated = socket_reconnected
            }
            ClientMessage::HandleSocketReply { payload, tx } => {
                let result = self
                    .handle_reply(&payload, channel_updated, socket_updated)
                    .await;

                let event = EventPayload {
                    event: Event::Phoenix {
                        phoenix: PhoenixEvent::Reply,
                    },
                    payload,
                };

                self.on_event(event);

                let _ = tx.send(result);
            }
        }
    }

    async fn handle_redirect(
        &self,
        redirect: &JSON,
        _channel_updated: &mut bool,
        _socket_updated: &mut bool,
    ) -> Result<HistoryId, LiveSocketError> {
        let json = redirect.clone().into();
        let redirect: LiveRedirect = serde_json::from_value(json)?;
        let url = self.client_state.session_data.try_lock()?.url.clone();
        let url = url.join(&redirect.to)?;

        match redirect.mode {
            Some(RedirectMode::Patch) => {
                // TODO: get this to call handle_params
                Ok(0)
            }
            None | Some(RedirectMode::ReplaceTop) => {
                let action = match redirect.kind {
                    Some(RedirectKind::Push) | None => NavAction::Push,
                    Some(RedirectKind::Replace) => NavAction::Replace,
                };

                let opts = NavOptions {
                    action: Some(action),
                    join_params: self.live_view_channel.channel.join_params.clone().into(),
                    ..NavOptions::default()
                };

                let res = self.client_state.navigate(url.to_string(), opts).await?;
                Ok(res.history_id)
            }
        }
    }

    async fn handle_reply(
        &self,
        reply: &Payload,
        channel_updated: &mut bool,
        socket_updated: &mut bool,
    ) -> Result<ReplyAction, LiveSocketError> {
        let Payload::JSONPayload {
            json: JSON::Object { object },
        } = reply
        else {
            return Ok(ReplyAction::None);
        };

        if let Some(object) = object.get("live_redirect") {
            let id = self
                .handle_redirect(object, channel_updated, socket_updated)
                .await?;

            return Ok(ReplyAction::Redirected { id });
        }

        if let Some(object) = object.get("redirect") {
            let id = self
                .handle_redirect(object, channel_updated, socket_updated)
                .await?;
            return Ok(ReplyAction::Redirected { id });
        }

        if let Some(diff) = object.get("diff") {
            self.document
                .merge_deserialized_fragment_json(diff.clone())?;

            return Ok(ReplyAction::DiffMerged);
        };

        Ok(ReplyAction::None)
    }

    pub async fn handle_server_event(
        &self,
        event: &EventPayload,
        channel_updated: &mut bool,
        socket_updated: &mut bool,
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

                    self.document
                        .merge_deserialized_fragment_json(json.clone())?;
                }
                "assets_change" => {
                    let Some(current_entry) = self.client_state.current_history_entry() else {
                        return Ok(());
                    };

                    let opts = self
                        .client_state
                        .session_data
                        .try_lock()?
                        .connect_opts
                        .clone();

                    let join_params = self.live_view_channel.channel.join_params.clone();

                    self.client_state
                        .reconnect(current_entry.url, opts, Some(join_params))
                        .await?;

                    *socket_updated = true;
                    *channel_updated = true;
                }
                // TODO: Handle this
                "live_patch" => {
                    let Payload::JSONPayload { json, .. } = &event.payload else {
                        error!("Live patch was not json!");
                        return Ok(());
                    };

                    let json = json.clone().into();
                    let redirect: LiveRedirect = serde_json::from_value(json)?;

                    self.client_state.patch(redirect.to)?;
                }
                // TODO: Handle this
                "live_redirect" => {
                    let Payload::JSONPayload { json, .. } = &event.payload else {
                        error!("Live redirect was not json!");
                        return Ok(());
                    };

                    // respect `to` `kind` and `mode` relative to current url base
                    self.handle_redirect(json, channel_updated, socket_updated)
                        .await?;
                }
                "redirect" => {
                    let Payload::JSONPayload { json, .. } = &event.payload else {
                        error!("Live redirect was not json!");
                        return Ok(());
                    };

                    // navigate replacing top, using `to` relative to current url base
                    self.handle_redirect(json, channel_updated, socket_updated)
                        .await?;
                }
                _ => {}
            },
        };

        Ok(())
    }
}
