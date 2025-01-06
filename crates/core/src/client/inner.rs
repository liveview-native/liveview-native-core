use std::{collections::HashMap, sync::Arc};

use log::debug;
use phoenix_channels_client::{Number, Payload, Socket, Topic, JSON};
use reqwest::{redirect::Policy, Client, Url};

use super::LiveViewClientConfiguration;
use crate::{
    client::{cookie_store::PersistentCookieStore, logging::init_log},
    diff::fragment::{Root, RootDiff},
    dom::Document,
    live_socket::{
        navigation::{NavCtx, NavOptions},
        LiveChannel, LiveSocketError, SessionData,
    },
};

pub struct LiveViewClientInner {
    /// Manages navigation state, events
    config: LiveViewClientConfiguration,
    nav_ctx: NavCtx,
    http_client: Client,
    socket: Arc<Socket>,
    liveview_channel: LiveChannel,
    livereload_channel: Option<LiveChannel>,
    session_data: SessionData,
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

        let websocket_url = session_data.get_live_socket_url()?;

        debug!("Initiating Websocket connection: {websocket_url:?}");
        let socket = Socket::spawn(websocket_url, Some(session_data.cookies.clone())).await?;

        debug!("Joining liveview Channel");
        let liveview_channel =
            join_liveview_channel(&config, &socket.clone(), &session_data, None).await?;

        if let Some(handler) = &config.patch_handler {
            liveview_channel
                .document
                .arc_set_event_handler(handler.clone())
        }

        let livereload_channel = if session_data.has_live_reload {
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
            nav_ctx,
            socket,
            liveview_channel,
            livereload_channel,
            session_data,
        })
    }
}

/// TODO: Clean up things below this line

const CSRF_KEY: &str = "_csrf_token";
const MOUNT_KEY: &str = "_mounts";
const FMT_KEY: &str = "_format";

pub async fn join_liveview_channel(
    config: &LiveViewClientConfiguration,
    socket: &Arc<Socket>,
    session_data: &SessionData,
    redirect: Option<String>,
) -> Result<LiveChannel, LiveSocketError> {
    let ws_timeout = std::time::Duration::from_millis(config.websocket_timeout);
    socket.connect(ws_timeout).await?;

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
                string: session_data.csrf_token.clone(),
            },
        ),
        (
            FMT_KEY.to_string(),
            JSON::Str {
                string: session_data.format.clone(),
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
                string: session_data.url.to_string(),
            },
        )
    };
    let join_payload = Payload::JSONPayload {
        json: JSON::Object {
            object: HashMap::from([
                (
                    "static".to_string(),
                    JSON::Str {
                        string: session_data.phx_static.clone(),
                    },
                ),
                (
                    "session".to_string(),
                    JSON::Str {
                        string: session_data.phx_session.clone(),
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

    let channel = socket
        .channel(
            Topic::from_string(format!("lv:{}", session_data.phx_id)),
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
        socket: socket.clone(),
        document: document.into(),
        timeout: ws_timeout,
    })
}

const LVN_VSN: &str = "2.0.0";
const LVN_VSN_KEY: &str = "vsn";

pub async fn join_livereload_channel(
    config: &LiveViewClientConfiguration,
    socket: &Arc<Socket>,
    session_data: &SessionData,
) -> Result<LiveChannel, LiveSocketError> {
    let ws_timeout = std::time::Duration::from_millis(config.websocket_timeout);

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

    // TODO: get these out of the client
    let cookies = session_data.cookies.clone();

    // TODO: Reuse the socket from before? why are we mixing sockets here?
    let new_socket = Socket::spawn(url.clone(), Some(cookies)).await?;
    new_socket.connect(ws_timeout).await?;

    debug!("Joining live reload channel on url {url}");
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
        socket: socket.clone(),
        document: document.into(),
        timeout: ws_timeout,
    })
}
