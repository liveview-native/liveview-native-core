use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use log::debug;
use phoenix_channels_client::{Number, Payload, PhoenixError, Socket, SocketStatus, Topic, JSON};
use reqwest::{redirect::Policy, Client, Url};

use super::LiveViewClientConfiguration;
use crate::{
    client::{cookie_store::PersistentCookieStore, logging::init_log},
    diff::fragment::{Root, RootDiff},
    dom::Document,
    live_socket::{
        navigation::{HistoryId, NavCtx, NavEventHandler, NavHistoryEntry, NavOptions},
        LiveChannel, LiveSocketError, SessionData,
    },
};

pub struct LiveViewClientInner {
    /// Manages navigation state, events
    config: LiveViewClientConfiguration,
    nav_ctx: Mutex<NavCtx>,
    http_client: Client,
    socket: Mutex<Arc<Socket>>,
    liveview_channel: Mutex<Arc<LiveChannel>>,
    livereload_channel: Mutex<Option<Arc<LiveChannel>>>,
    session_data: Mutex<SessionData>,
}

impl LiveViewClientInner {
    pub async fn initial_connect(
        config: LiveViewClientConfiguration,
        url: String,
    ) -> Result<Self, LiveSocketError> {
        init_log(config.log_level);
        debug!("Initializing LiveViewClient.");
        debug!("Configuration: {config:?}");

        let cookie_store = PersistentCookieStore::new(config.persistence_provider.clone());
        let http_client = Client::builder()
            .cookie_provider(Arc::new(cookie_store))
            .redirect(Policy::none())
            .build()
            .expect("Failed to build HTTP client");

        let url = Url::parse(&url)?;
        let format = config.format.to_string();

        debug!("Retrieving session data from: {url:?}");
        let session_data =
            SessionData::request(&url, &format, Default::default(), http_client.clone()).await?;

        let session_data = Mutex::new(session_data);

        let websocket_url = session_data.try_lock()?.get_live_socket_url()?;

        let cookies = session_data.try_lock()?.cookies.clone();

        debug!("Initiating Websocket connection: {websocket_url:?}");
        let socket = Socket::spawn(websocket_url, Some(cookies)).await?;

        let socket = Mutex::new(socket);

        debug!("Joining liveview Channel");
        let liveview_channel = join_liveview_channel(&config, &socket, &session_data, None).await?;

        if let Some(handler) = &config.patch_handler {
            liveview_channel
                .document
                .arc_set_event_handler(handler.clone())
        }

        let livereload_channel = if session_data.try_lock()?.has_live_reload {
            debug!("Joining liveReload Channel");
            join_livereload_channel(&config, &socket, &session_data)
                .await?
                .into()
        } else {
            None
        };

        let mut nav_ctx = NavCtx::default();
        nav_ctx.navigate(url.clone(), NavOptions::default(), false);

        if let Some(handler) = &config.navigation_handler {
            nav_ctx.set_event_handler(handler.clone());
        }

        Ok(Self {
            config,
            http_client,
            nav_ctx: nav_ctx.into(),
            socket: socket.into(),
            liveview_channel: liveview_channel.into(),
            livereload_channel: livereload_channel.into(),
            session_data: session_data.into(),
        })
    }
}
/// Navigation related api.

impl LiveViewClientInner {
    const RETRY_REASONS: &[&str] = &["stale", "unauthorized"];
    async fn try_nav(&self) -> Result<(), LiveSocketError> {
        let current = self
            .nav_ctx
            .try_lock()?
            .current()
            .ok_or(LiveSocketError::NavigationImpossible)?;

        let chan = &self.liveview_channel.try_lock()?.channel.clone();
        chan.leave().await?;

        match join_liveview_channel(
            &self.config,
            &self.socket,
            &self.session_data,
            Some(current.url.clone()),
        )
        .await
        {
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

                let url = Url::parse(&current.url)?;
                let format = self.config.format.to_string();

                let new_session_data = SessionData::request(
                    &url,
                    &format,
                    Default::default(),
                    self.http_client.clone(),
                )
                .await?;

                let websocket_url = new_session_data.get_live_socket_url()?;

                let new_socket =
                    Socket::spawn(websocket_url, Some(new_session_data.cookies.clone())).await?;

                let sock = self.socket.try_lock()?.clone();
                sock.shutdown()
                    .await
                    .map_err(|e| LiveSocketError::Phoenix {
                        error: format!("{e:?}"),
                    })?;

                *self.socket.try_lock()? = new_socket;
                *self.session_data.try_lock()? = new_session_data;

                let channel =
                    join_liveview_channel(&self.config, &self.socket, &self.session_data, None)
                        .await?;

                if let Some(event_handler) = &self.config.patch_handler {
                    channel
                        .document
                        .arc_set_event_handler(event_handler.clone())
                }

                *self.liveview_channel.try_lock()? = channel;
                Ok(())
            }
            Ok(channel) => {
                if let Some(event_handler) = &self.config.patch_handler {
                    channel
                        .document
                        .arc_set_event_handler(event_handler.clone())
                }

                *self.liveview_channel.try_lock()? = channel;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    async fn try_nav_outer<F>(&self, nav_action: F) -> Result<(), LiveSocketError>
    where
        F: FnOnce(&mut NavCtx) -> Option<HistoryId>,
    {
        let new_id = {
            let mut nav_ctx = self.nav_ctx.try_lock()?;
            nav_action(&mut nav_ctx)
        };

        if new_id.is_none() {
            return Err(LiveSocketError::NavigationImpossible);
        }

        self.try_nav().await
    }

    pub async fn navigate(&self, url: String, opts: NavOptions) -> Result<(), LiveSocketError> {
        let url = Url::parse(&url)?;
        self.try_nav_outer(|ctx| ctx.navigate(url, opts, true))
            .await
    }

    pub async fn reload(&self, info: Option<Vec<u8>>) -> Result<(), LiveSocketError> {
        self.try_nav_outer(|ctx| ctx.reload(info, true)).await
    }

    pub async fn back(&self, info: Option<Vec<u8>>) -> Result<(), LiveSocketError> {
        self.try_nav_outer(|ctx| ctx.back(info, true)).await
    }

    pub async fn forward(&self, info: Option<Vec<u8>>) -> Result<(), LiveSocketError> {
        self.try_nav_outer(|ctx| ctx.forward(info, true)).await
    }

    pub async fn traverse_to(
        &self,
        id: HistoryId,
        info: Option<Vec<u8>>,
    ) -> Result<(), LiveSocketError> {
        self.try_nav_outer(|ctx| ctx.traverse_to(id, info, true))
            .await
    }

    pub fn can_go_back(&self) -> bool {
        self.nav_ctx
            .try_lock()
            .map(|ctx| ctx.can_go_back())
            .unwrap_or(false)
    }

    pub fn can_go_forward(&self) -> bool {
        self.nav_ctx
            .try_lock()
            .map(|ctx| ctx.can_go_forward())
            .unwrap_or(false)
    }

    pub fn can_traverse_to(&self, id: HistoryId) -> bool {
        self.nav_ctx
            .try_lock()
            .map(|ctx| ctx.can_traverse_to(id))
            .unwrap_or(false)
    }

    pub fn get_entries(&self) -> Vec<NavHistoryEntry> {
        self.nav_ctx
            .try_lock()
            .map(|ctx| ctx.entries())
            .unwrap_or_default()
    }

    pub fn current(&self) -> Option<NavHistoryEntry> {
        self.nav_ctx.try_lock().ok().and_then(|ctx| ctx.current())
    }

    pub fn set_event_handler(
        &self,
        handler: Box<dyn NavEventHandler>,
    ) -> Result<(), LiveSocketError> {
        self.nav_ctx.try_lock()?.set_event_handler(handler.into());
        Ok(())
    }
}

// First implement the accessor methods on LiveViewClientInner
impl LiveViewClientInner {
    pub fn socket(&self) -> Result<Arc<Socket>, LiveSocketError> {
        Ok(self.socket.try_lock()?.clone())
    }

    pub fn channel(&self) -> Result<Arc<LiveChannel>, LiveSocketError> {
        Ok(self.liveview_channel.try_lock()?.clone())
    }

    pub fn live_reload_channel(&self) -> Result<Option<Arc<LiveChannel>>, LiveSocketError> {
        Ok(self.livereload_channel.try_lock()?.clone())
    }

    pub fn join_url(&self) -> Result<String, LiveSocketError> {
        Ok(self.session_data.try_lock()?.url.to_string())
    }

    pub fn csrf_token(&self) -> Result<String, LiveSocketError> {
        Ok(self.session_data.try_lock()?.csrf_token.clone())
    }

    pub fn dead_render(&self) -> Result<Document, LiveSocketError> {
        Ok(self.session_data.try_lock()?.dead_render.clone())
    }

    pub fn style_urls(&self) -> Result<Vec<String>, LiveSocketError> {
        Ok(self.session_data.try_lock()?.style_urls.clone())
    }

    pub fn status(&self) -> Result<SocketStatus, LiveSocketError> {
        Ok(self.socket.try_lock()?.status())
    }
}

/// TODO: Clean up things below this line
const CSRF_KEY: &str = "_csrf_token";
const MOUNT_KEY: &str = "_mounts";
const FMT_KEY: &str = "_format";

pub async fn join_liveview_channel(
    config: &LiveViewClientConfiguration,
    socket: &Mutex<Arc<Socket>>,
    session_data: &Mutex<SessionData>,
    redirect: Option<String>,
) -> Result<Arc<LiveChannel>, LiveSocketError> {
    let ws_timeout = std::time::Duration::from_millis(config.websocket_timeout);

    let sock = socket.try_lock()?.clone();
    sock.connect(ws_timeout).await?;

    let mut collected_join_params = HashMap::from([
        (
            MOUNT_KEY.to_string(),
            JSON::Numb {
                number: Number::PosInt { pos: 0 },
            },
        ),
        (
            CSRF_KEY.to_string(),
            JSON::Str {
                string: session_data.try_lock()?.csrf_token.clone(),
            },
        ),
        (
            FMT_KEY.to_string(),
            JSON::Str {
                string: session_data.try_lock()?.format.clone(),
            },
        ),
    ]);
    if let Some(join_params) = config.join_params.clone() {
        for (key, value) in &join_params {
            collected_join_params.insert(key.clone(), value.clone());
        }
    }
    let redirect_or_url: (String, JSON) = if let Some(redirect) = redirect {
        ("redirect".to_string(), JSON::Str { string: redirect })
    } else {
        (
            "url".to_string(),
            JSON::Str {
                string: session_data.try_lock()?.url.to_string(),
            },
        )
    };
    let join_payload = Payload::JSONPayload {
        json: JSON::Object {
            object: HashMap::from([
                (
                    "static".to_string(),
                    JSON::Str {
                        string: session_data.try_lock()?.phx_static.clone(),
                    },
                ),
                (
                    "session".to_string(),
                    JSON::Str {
                        string: session_data.try_lock()?.phx_session.clone(),
                    },
                ),
                // TODO: Add redirect key. Swift code:
                // (redirect ? "redirect": "url"): self.url.absoluteString,
                redirect_or_url,
                (
                    "params".to_string(),
                    // TODO: Merge join_params with this simple object.
                    JSON::Object {
                        object: collected_join_params,
                    },
                ),
            ]),
        },
    };

    let channel = sock
        .channel(
            Topic::from_string(format!("lv:{}", session_data.try_lock()?.phx_id)),
            Some(join_payload),
        )
        .await?;

    let join_payload = channel.join(ws_timeout).await?;

    debug!("Join payload: {join_payload:#?}");
    let document = match join_payload {
        Payload::JSONPayload {
            json: JSON::Object { ref object },
        } => {
            if let Some(rendered) = object.get("rendered") {
                let rendered = rendered.to_string();
                let root: RootDiff = serde_json::from_str(rendered.as_str())?;
                debug!("root diff: {root:#?}");
                let root: Root = root.try_into()?;
                let rendered: String = root.clone().try_into()?;
                let mut document = crate::parser::parse(&rendered)?;
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
        join_params: config.join_params.clone().unwrap_or_default(),
        socket: socket.try_lock()?.clone(),
        document: document.into(),
        timeout: ws_timeout,
    }
    .into())
}

const LVN_VSN: &str = "2.0.0";
const LVN_VSN_KEY: &str = "vsn";

pub async fn join_livereload_channel(
    config: &LiveViewClientConfiguration,
    socket: &Mutex<Arc<Socket>>,
    session_data: &Mutex<SessionData>,
) -> Result<Arc<LiveChannel>, LiveSocketError> {
    let ws_timeout = std::time::Duration::from_millis(config.websocket_timeout);

    let mut url = session_data.try_lock()?.url.clone();

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

    // TODO: get these out of the client
    let cookies = session_data.try_lock()?.cookies.clone();

    // TODO: Reuse the socket from before? why are we mixing sockets here?
    let new_socket = Socket::spawn(url.clone(), Some(cookies)).await?;
    new_socket.connect(ws_timeout).await?;

    debug!("Joining live reload channel on url {url}");
    let socket = socket.try_lock()?.clone();
    let channel = socket
        .channel(Topic::from_string("phoenix:live_reload".to_string()), None)
        .await?;

    debug!("Created channel for live reload socket");
    let join_payload = channel.join(ws_timeout).await?;
    let document = Document::empty();

    Ok(LiveChannel {
        channel,
        join_params: Default::default(),
        join_payload,
        socket, // TODO: this field is prone to memory leakage
        document: document.into(),
        timeout: ws_timeout,
    }
    .into())
}
