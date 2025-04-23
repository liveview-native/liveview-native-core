use std::{collections::HashMap, future::Future, sync::Arc, time::Duration};

use log::{debug, trace};
use phoenix_channels_client::{
    ChannelStatus, ChannelStatuses, EventPayload, Events, EventsError, Payload, ReconnectStrategy,
    Socket, SocketStatuses, Topic, JSON,
};
use reqwest::{Client, Url};
use tokio::select;

use super::LiveViewClientStatus;
use crate::{
    client::{
        inner::{connected_client, PersistentCookieStore},
        ClientConnectOpts, LiveViewClientConfiguration, StrategyAdapter,
    },
    diff::fragment::{Root, RootDiff},
    dom::{ffi, Document},
    error::LiveSocketError,
    live_socket::{ConnectOpts, LiveChannel, SessionData},
};

pub(crate) struct ConnectedClient {
    pub(crate) document: ffi::Document,
    /// The main websocket for this page
    pub(crate) socket: Arc<Socket>,
    /// The main channel which passes user interaction events
    /// and receives changes to the "dom"
    pub(crate) liveview_channel: Arc<LiveChannel>,
    /// In debug mode LiveView has a debug channel which sends
    /// asset update events, this is derived from an iframe on the page
    /// which also may be present on errored out connections.
    pub(crate) livereload_channel: Option<Arc<LiveChannel>>,
    /// Data acquired from the dead render, should only change between
    /// reconnects.
    pub(crate) session_data: SessionData,
    /// Utility to hold all the event channels
    pub(crate) event_pump: EventPump,
}

pub(crate) struct EventPump {
    pub(crate) main_events: Arc<Events>,
    pub(crate) main_channel_status: Arc<ChannelStatuses>,
    pub(crate) reload_events: Option<Arc<Events>>,
    pub(crate) socket_statuses: Arc<SocketStatuses>,
}

impl ConnectedClient {
    pub async fn try_new(
        config: &LiveViewClientConfiguration,
        url: &str,
        http_client: &Client,
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
            Err(e) => {
                return {
                    let _ = socket.shutdown().await;
                    Err(e)
                }
            }
        };

        if let Some(handler) = config.patch_handler.clone() {
            liveview_channel.document.arc_set_event_handler(handler);
        }

        let livereload_channel = if session_data.has_live_reload {
            match join_livereload_channel(config, &session_data, cookies).await {
                Ok(channel) => Some(channel),
                Err(e) => {
                    return {
                        let _ = socket.shutdown().await;
                        Err(e)
                    }
                }
            }
        } else {
            None
        };

        Ok(Self {
            document: liveview_channel.document(),
            event_pump: EventPump {
                main_events: liveview_channel.channel.events(),
                main_channel_status: liveview_channel.channel.statuses(),
                reload_events: livereload_channel.as_ref().map(|c| c.channel.events()),
                socket_statuses: socket.statuses(),
            },
            socket,
            liveview_channel,
            livereload_channel,
            session_data,
        })
    }

    pub async fn rejoin_liveview_channel(
        &mut self,
        config: &LiveViewClientConfiguration,
    ) -> Result<(), LiveSocketError> {
        let additional_params = self.liveview_channel.join_params.clone();
        let ws_timeout = Duration::from_millis(config.websocket_timeout);

        let new_channel = connected_client::join_liveview_channel(
            &self.socket,
            &self.session_data,
            &Some(additional_params),
            None,
            ws_timeout,
        )
        .await?;

        if let Some(handler) = &config.patch_handler {
            new_channel.document.arc_set_event_handler(handler.clone());
        }

        self.document = new_channel.document();
        self.liveview_channel = new_channel;
        self.event_pump = self.event_pump();
        Ok(())
    }

    /// Combined event futures from the LiveReload and main live view channel.
    pub fn event_futures(&self) -> impl Future<Output = Result<EventPayload, EventsError>> + '_ {
        let server_event = self.event_pump.main_events.event();
        let maybe_reload_event = self.event_pump.reload_events.as_ref().map(|e| e.event());

        let live_reload_proxy = async {
            match maybe_reload_event {
                Some(e) => e.await,
                None => std::future::pending().await,
            }
        };

        async {
            select! {
                r1 = live_reload_proxy => r1,
                r2 = server_event => r2,
            }
        }
    }

    pub(crate) async fn shutdown(&self) {
        let _ = self.liveview_channel.channel.leave().await;
        let _ = self.socket.disconnect().await;
        let _ = self.socket.shutdown().await;
        if let Some(livereload) = &self.livereload_channel {
            let _ = livereload.shutdown_parent_socket().await;
        }
    }

    pub(crate) fn ffi_status(&self) -> LiveViewClientStatus {
        LiveViewClientStatus::Connected {
            channel_status: self.main_channel_status(),
        }
    }

    pub(crate) fn main_channel_status(&self) -> super::MainChannelStatus {
        match self.liveview_channel.channel.status() {
            ChannelStatus::Joined => super::MainChannelStatus::Connected {
                document: self.document.clone().into(),
            },
            ChannelStatus::WaitingForSocketToConnect
            | ChannelStatus::WaitingToJoin
            | ChannelStatus::Joining
            | ChannelStatus::WaitingToRejoin { .. }
            | ChannelStatus::Leaving
            | ChannelStatus::Left
            | ChannelStatus::ShuttingDown
            | ChannelStatus::ShutDown => super::MainChannelStatus::Reconnecting,
        }
    }

    pub(crate) const RETRY_REASONS: &'static [&'static str] = &["stale", "unauthorized"];

    /// try to do the internal nav, attempting to fix
    /// recoverable errors which occur when connecting across
    /// auth state, live_sessions and respecting redirects.
    /// If the websocket needed to be refreshed this returns true
    /// otherwise it returns false.
    pub(crate) async fn try_nav(
        &mut self,
        http_client: &Client,
        config: &LiveViewClientConfiguration,
        additional_params: &Option<HashMap<String, JSON>>,
        redirect: String,
    ) -> Result<bool, LiveSocketError> {
        let ws_timeout = Duration::from_millis(config.websocket_timeout);

        self.liveview_channel.channel.leave().await?;

        match join_liveview_channel(
            &self.socket,
            &self.session_data,
            additional_params,
            Some(redirect.clone()),
            ws_timeout,
        )
        .await
        {
            Ok(new_channel) => {
                if let Some(event_handler) = &config.patch_handler {
                    new_channel
                        .document
                        .arc_set_event_handler(event_handler.clone())
                }

                self.document = new_channel.document();
                self.liveview_channel = new_channel;
                self.event_pump = self.event_pump();
                Ok(false)
            }
            Err(LiveSocketError::JoinRejection {
                error:
                    Payload::JSONPayload {
                        json: JSON::Object { object },
                    },
            }) => {
                if object
                    .get("reason")
                    .and_then(|r| match r {
                        JSON::Str { string } => Some(string),
                        _ => None,
                    })
                    .is_none_or(|reason| !Self::RETRY_REASONS.contains(&reason.as_str()))
                {
                    return Err(LiveSocketError::JoinRejection {
                        error: Payload::JSONPayload {
                            json: JSON::Object { object },
                        },
                    });
                }

                let url = Url::parse(&redirect)?;
                let format = config.format.to_string();

                let new_session_data =
                    SessionData::request(&url, &format, Default::default(), http_client.clone())
                        .await?;

                let websocket_url = new_session_data.get_live_socket_url()?;

                let adapter = config
                    .socket_reconnect_strategy
                    .clone()
                    .map(StrategyAdapter::from)
                    .map(|s| Box::new(s) as Box<dyn ReconnectStrategy>);

                let cookies = new_session_data.cookies.clone();

                let new_socket = Socket::spawn(websocket_url, Some(cookies), adapter).await?;
                new_socket.connect(ws_timeout).await?;

                self.socket
                    .shutdown()
                    .await
                    .map_err(|e| LiveSocketError::Phoenix {
                        error: format!("{e:?}"),
                    })?;

                self.socket = new_socket;
                self.session_data = new_session_data;

                let new_channel = join_liveview_channel(
                    &self.socket,
                    &self.session_data,
                    additional_params,
                    None,
                    ws_timeout,
                )
                .await?;

                if let Some(event_handler) = &config.patch_handler {
                    new_channel
                        .document
                        .arc_set_event_handler(event_handler.clone())
                }

                self.document = new_channel.document();
                self.liveview_channel = new_channel.clone();
                self.event_pump = self.event_pump();

                Ok(true)
            }
            Err(e) => Err(e),
        }
    }

    pub fn event_pump(&self) -> EventPump {
        EventPump {
            main_events: self.liveview_channel.channel.events(),
            main_channel_status: self.liveview_channel.channel.statuses(),
            reload_events: self.livereload_channel.as_ref().map(|c| c.channel.events()),
            socket_statuses: self.socket.statuses(),
        }
    }
}

const LVN_VSN: &str = "2.0.0";
const LVN_VSN_KEY: &str = "vsn";

/// TODO: Post refactor turn this into a private constructor on a LiveChannel
pub async fn join_liveview_channel(
    socket: &Arc<Socket>,
    session_data: &SessionData,
    additional_params: &Option<HashMap<String, JSON>>,
    redirect: Option<String>,
    ws_timeout: std::time::Duration,
) -> Result<Arc<LiveChannel>, LiveSocketError> {
    socket.connect(ws_timeout).await?;

    let sent_join_payload = session_data.create_join_payload(additional_params, redirect);
    let topic = Topic::from_string(format!("lv:{}", session_data.phx_id));
    let channel = socket.channel(topic, Some(sent_join_payload)).await?;

    let join_payload = channel.join(ws_timeout).await?;

    trace!("Join payload: {join_payload:#?}");
    let document = match join_payload {
        Payload::JSONPayload {
            json: JSON::Object { ref object },
        } => {
            if let Some(rendered) = object.get("rendered") {
                let rendered = rendered.to_string();
                let root: RootDiff = serde_json::from_str(rendered.as_str())?;
                trace!("root diff: {root:#?}");
                let root: Root = root.try_into()?;
                let rendered: String = root.clone().try_into()?;
                let mut document = Document::parse(&rendered)?;
                document.fragment_template = Some(root);
                Some(document)
            } else {
                None
            }
        }
        _ => None,
    }
    .ok_or(LiveSocketError::NoDocumentInJoinPayload)?;

    Ok(LiveChannel {
        channel,
        join_payload,
        join_params: additional_params.clone().unwrap_or_default(),
        socket: socket.clone(),
        document: document.into(),
        timeout: ws_timeout,
    }
    .into())
}

pub async fn join_livereload_channel(
    config: &LiveViewClientConfiguration,
    session_data: &SessionData,
    cookies: Option<Vec<String>>,
) -> Result<Arc<LiveChannel>, LiveSocketError> {
    let ws_timeout = Duration::from_millis(config.websocket_timeout);

    let mut url = session_data.url.clone();

    let websocket_scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        scheme => {
            return Err(LiveSocketError::SchemeNotSupported {
                scheme: scheme.to_string(),
            })
        }
    };
    let _ = url.set_scheme(websocket_scheme);
    url.set_path("phoenix/live_reload/socket/websocket");
    url.query_pairs_mut().append_pair(LVN_VSN_KEY, LVN_VSN);

    let adapter = config
        .socket_reconnect_strategy
        .clone()
        .map(StrategyAdapter::from)
        .map(|s| Box::new(s) as Box<dyn ReconnectStrategy>);

    let new_socket = Socket::spawn(url.clone(), cookies, adapter).await?;
    new_socket.connect(ws_timeout).await?;

    debug!("Joining live reload channel on url {url}");
    let channel = new_socket
        .channel(Topic::from_string("phoenix:live_reload".to_string()), None)
        .await?;

    debug!("Created channel for live reload socket");
    let join_payload = channel.join(ws_timeout).await?;
    let document = Document::empty();

    Ok(LiveChannel {
        channel,
        join_params: Default::default(),
        join_payload,
        socket: new_socket,
        document: document.into(),
        timeout: ws_timeout,
    }
    .into())
}
