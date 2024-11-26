use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use super::navigation::{NavCtx, NavOptions};

use log::debug;
use phoenix_channels_client::{url::Url, Number, Payload, Socket, Topic, JSON};
use reqwest::Method as ReqMethod;

pub use super::{LiveChannel, LiveSocketError};

use crate::{
    diff::fragment::{Root, RootDiff},
    dom::{ffi::Document as FFiDocument, AttributeName, Document, ElementName, Selector},
    parser::parse,
};

#[macro_export]
macro_rules! lock {
    ($mutex:expr) => {
        $mutex.lock().expect("Failed to acquire lock")
    };
    ($mutex:expr, $msg:expr) => {
        $mutex.lock().expect($msg)
    };
}

const LVN_VSN: &str = "2.0.0";
const LVN_VSN_KEY: &str = "vsn";
const CSRF_KEY: &str = "_csrf_token";
const MOUNT_KEY: &str = "_mounts";
const FMT_KEY: &str = "_format";

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
#[repr(u8)]
pub enum Method {
    Get = 0,
    Options,
    Post,
    Put,
    Delete,
    Head,
    Trace,
    Connect,
    Patch,
}

impl From<Method> for ReqMethod {
    fn from(val: Method) -> ReqMethod {
        match val {
            Method::Options => ReqMethod::OPTIONS,
            Method::Get => ReqMethod::GET,
            Method::Post => ReqMethod::POST,
            Method::Put => ReqMethod::PUT,
            Method::Delete => ReqMethod::DELETE,
            Method::Head => ReqMethod::HEAD,
            Method::Trace => ReqMethod::TRACE,
            Method::Connect => ReqMethod::CONNECT,
            Method::Patch => ReqMethod::PATCH,
        }
    }
}

// If you change this also change the
// default below in the proc macro
const DEFAULT_TIMEOUT: u64 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConnectOpts {
    #[uniffi(default = None)]
    pub headers: Option<HashMap<String, String>>,
    #[uniffi(default = None)]
    pub body: Option<String>,
    #[uniffi(default = None)]
    pub method: Option<Method>,
    #[uniffi(default = 30_000)]
    pub timeout_ms: u64,
}

impl Default for ConnectOpts {
    fn default() -> Self {
        Self {
            headers: None,
            body: None,
            method: None,
            timeout_ms: DEFAULT_TIMEOUT,
        }
    }
}

/// Static information ascertained from the dead render when connecting.
#[derive(Clone, Debug)]
pub struct SessionData {
    pub connect_opts: ConnectOpts,
    /// Cross site request forgery, security token, sent with dead render.
    pub csrf_token: String,
    /// The id of the phoenix channel to join.
    pub phx_id: String,
    pub phx_static: String,
    pub phx_session: String,
    pub url: Url,
    /// One of `swift`, `kotlin` or `html` indicating the developer platform.
    pub format: String,
    /// An html page that on the web would be used to bootstrap the web socket connection.
    pub dead_render: Document,
    pub style_urls: Vec<String>,
    /// Whether or not the dead render contains a live reload iframe for development mode.
    pub has_live_reload: bool,
    /// A list of cookies sent over with the dead render.
    pub cookies: Vec<String>,
}

impl SessionData {
    pub async fn request(
        url: &Url,
        format: &String,
        connect_opts: ConnectOpts,
    ) -> Result<Self, LiveSocketError> {
        // NEED:
        // these from inside data-phx-main
        // data-phx-session,
        // data-phx-static
        // id
        //
        // Top level:
        // csrf-token
        // "iframe[src=\"/phoenix/live_reload/frame\"]"
        let (dead_render, cookies) =
            LiveSocket::get_dead_render(url, format, &connect_opts).await?;

        let csrf_token = dead_render
            .get_csrf_token()
            .ok_or(LiveSocketError::CSFRTokenMissing)?;

        let mut phx_id: Option<String> = None;
        let mut phx_static: Option<String> = None;
        let mut phx_session: Option<String> = None;

        let main_div_attributes = dead_render
            .select(Selector::Attribute(AttributeName {
                name: "data-phx-main".into(),
                namespace: None,
            }))
            .last();

        debug!("MAIN DIV: {main_div_attributes:?}");

        let main_div_attributes = dead_render
            .select(Selector::Attribute(AttributeName {
                namespace: None,
                name: "data-phx-main".into(),
            }))
            .last()
            .map(|node_ref| dead_render.get(node_ref))
            .map(|main_div| main_div.attributes())
            .ok_or(LiveSocketError::PhoenixMainMissing)?;

        for attr in main_div_attributes {
            if attr.name.name == "id" {
                phx_id.clone_from(&attr.value)
            } else if attr.name.name == "data-phx-session" {
                phx_session.clone_from(&attr.value)
            } else if attr.name.name == "data-phx-static" {
                phx_static.clone_from(&attr.value)
            }
        }
        let phx_id = phx_id.ok_or(LiveSocketError::PhoenixIDMissing)?;
        let phx_static = phx_static.ok_or(LiveSocketError::PhoenixStaticMissing)?;
        let phx_session = phx_session.ok_or(LiveSocketError::PhoenixSessionMissing)?;
        debug!("phx_id = {phx_id:?}, session = {phx_session:?}, static = {phx_static:?}");

        // A Style looks like:
        // <Style url="/assets/app.swiftui.styles" />
        let style_urls: Vec<String> = dead_render
            .select(Selector::Tag(ElementName {
                namespace: None,
                name: "Style".into(),
            }))
            .map(|node_ref| dead_render.get(node_ref))
            .filter_map(|node| {
                node.attributes()
                    .iter()
                    .filter(|attr| attr.name.name == "url")
                    .map(|attr| attr.value.clone())
                    .last()
                    .flatten()
            })
            .collect();

        // The iframe portion looks like:
        // <iframe hidden height="0" width="0" src="/phoenix/live_reload/frame"></iframe>
        let live_reload_iframe: Option<String> = dead_render
            .select(Selector::Tag(ElementName {
                namespace: None,
                name: "iframe".into(),
            }))
            .map(|node_ref| dead_render.get(node_ref))
            .filter_map(|node| {
                node.attributes()
                    .iter()
                    .filter(|attr| attr.name.name == "src")
                    .map(|attr| attr.value.clone())
                    .last()
                    .flatten()
            })
            .filter(|iframe_src| iframe_src == "/phoenix/live_reload/frame")
            .last();

        let has_live_reload = live_reload_iframe.is_some();

        let out = Self {
            connect_opts,
            url: url.clone(),
            format: format.to_string(),
            csrf_token,
            phx_id,
            phx_static,
            phx_session,
            dead_render,
            style_urls,
            has_live_reload,
            cookies,
        };

        debug!("Session data successfully acquired {out:?}");

        Ok(out)
    }

    /// reconstruct the live socket url from the session data
    pub fn get_live_socket_url(&self) -> Result<Url, LiveSocketError> {
        let websocket_scheme = match self.url.scheme() {
            "https" => "wss",
            "http" => "ws",
            scheme => {
                return Err(LiveSocketError::SchemeNotSupported {
                    scheme: scheme.to_string(),
                })
            }
        };

        let port = self.url.port().map(|p| format!(":{p}")).unwrap_or_default();
        let host = self.url.host_str().ok_or(LiveSocketError::NoHostInURL)?;

        let mut websocket_url = Url::parse(&format!("{websocket_scheme}://{host}{port}"))?;

        websocket_url
            .query_pairs_mut()
            .append_pair(LVN_VSN_KEY, LVN_VSN)
            .append_pair(CSRF_KEY, &self.csrf_token)
            .append_pair(MOUNT_KEY, "0")
            .append_pair(FMT_KEY, &self.format);

        websocket_url.set_path("/live/websocket");

        debug!("websocket url: {websocket_url}");

        Ok(websocket_url)
    }
}

#[derive(uniffi::Object)]
pub struct LiveSocket {
    pub socket: Mutex<Arc<Socket>>,
    pub session_data: Mutex<SessionData>,
    pub(super) navigation_ctx: Mutex<NavCtx>,
}

// non uniffi bindings.
impl LiveSocket {
    /// Gets the 'dead render', a static html page containing metadata about how to
    /// connect to a websocket and initialize the live view session.
    async fn get_dead_render(
        url: &Url,
        format: &str,
        options: &ConnectOpts,
    ) -> Result<(Document, Vec<String>), LiveSocketError> {
        let ConnectOpts {
            headers,
            body,
            method,
            timeout_ms,
        } = options;

        let method = method.clone().unwrap_or(Method::Get).into();

        // TODO: Check if params contains all of phx_id, phx_static, phx_session and csrf_token, if
        // it does maybe we don't need to do a full dead render.
        let mut url = url.clone();
        url.query_pairs_mut().append_pair(FMT_KEY, format);

        let headers = (&headers.clone().unwrap_or_default())
            .try_into()
            .map_err(|e| LiveSocketError::InvalidHeader {
                error: format!("{e:?}"),
            })?;

        let client = reqwest::Client::default();
        let req = reqwest::Request::new(method, url.clone());
        let builder = reqwest::RequestBuilder::from_parts(client, req);

        let builder = if let Some(body) = body {
            builder.body(body.clone())
        } else {
            builder
        };

        let timeout = Duration::from_millis(*timeout_ms);
        let (client, request) = builder.timeout(timeout).headers(headers).build_split();

        let resp = client.execute(request?).await?;
        let status = resp.status();
        let resp_headers = resp.headers();

        let cookies = resp_headers
            .get_all("set-cookie")
            .iter()
            .map(|cookie| cookie.to_str().expect("Cookie is not ASCII").to_string())
            .collect();

        let resp_text = resp.text().await?;
        if !status.is_success() {
            return Err(LiveSocketError::ConnectionError(resp_text));
        }

        let dead_render = parse(&resp_text)?;
        debug!("document:\n{dead_render}\n\n\n");
        Ok((dead_render, cookies))
    }
}

#[cfg_attr(not(target_family = "wasm"), uniffi::export(async_runtime = "tokio"))]
impl LiveSocket {
    // This is just for the jetpack client. This is an associated function constructor.
    #[uniffi::constructor]
    pub async fn connect(
        url: String,
        format: String,
        options: Option<ConnectOpts>,
    ) -> Result<Self, LiveSocketError> {
        Self::new(url, format, options).await
    }

    #[uniffi::constructor]
    pub async fn new(
        url: String,
        format: String,
        options: Option<ConnectOpts>,
    ) -> Result<Self, LiveSocketError> {
        let url = Url::parse(&url)?;
        let options = options.unwrap_or_default();

        // Make HTTP request to get initial dead render, an HTML document with
        // metadata needed to set up the liveview websocket connection.
        let session_data = SessionData::request(&url, &format, options).await?;
        let websocket_url = session_data.get_live_socket_url()?;

        let socket = Socket::spawn(websocket_url, Some(session_data.cookies.clone()))
            .await?
            .into();

        let navigation_ctx = Mutex::new(NavCtx::default());

        navigation_ctx.lock().expect("Lock Poisoned!").navigate(
            url.clone(),
            NavOptions::default(),
            false,
        );

        Ok(Self {
            socket,
            session_data: session_data.into(),
            navigation_ctx,
        })
    }

    pub fn dead_render(&self) -> FFiDocument {
        lock!(self.session_data).dead_render.clone().into()
    }

    pub fn style_urls(&self) -> Vec<String> {
        self.session_data
            .lock()
            .expect("lock poisoined")
            .style_urls
            .clone()
    }

    pub async fn join_livereload_channel(&self) -> Result<LiveChannel, LiveSocketError> {
        let mut url = lock!(self.session_data).url.clone();

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

        let cookies = lock!(self.session_data).cookies.clone();

        let socket = Socket::spawn(url.clone(), Some(cookies)).await?;
        socket.connect(self.timeout()).await?;

        debug!("Joining live reload channel on url {url}");
        let channel = socket
            .channel(Topic::from_string("phoenix:live_reload".to_string()), None)
            .await?;
        debug!("Created channel for live reload socket");
        let join_payload = channel.join(self.timeout()).await?;
        let document = Document::empty();

        Ok(LiveChannel {
            channel,
            join_payload,
            socket: self.socket(),
            document: document.into(),
            timeout: self.timeout(),
        })
    }

    pub async fn join_liveview_channel(
        &self,
        join_params: Option<HashMap<String, JSON>>,
        redirect: Option<String>,
    ) -> Result<LiveChannel, LiveSocketError> {
        self.socket().connect(self.timeout()).await?;
        let session_data = lock!(self.session_data).clone();

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
                    string: session_data.csrf_token,
                },
            ),
            (
                FMT_KEY.to_string(),
                JSON::Str {
                    string: session_data.format,
                },
            ),
        ]);
        if let Some(join_params) = join_params {
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
                            string: session_data.phx_static,
                        },
                    ),
                    (
                        "session".to_string(),
                        JSON::Str {
                            string: session_data.phx_session,
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

        let channel = self
            .socket()
            .channel(
                Topic::from_string(format!("lv:{}", session_data.phx_id)),
                Some(join_payload),
            )
            .await?;

        let join_payload = channel.join(self.timeout()).await?;

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
            socket: self.socket(),
            document: document.into(),
            timeout: self.timeout(),
        })
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_millis(lock!(self.session_data).connect_opts.timeout_ms)
    }

    pub fn socket(&self) -> Arc<Socket> {
        lock!(self.socket).clone()
    }

    pub fn has_live_reload(&self) -> bool {
        lock!(self.session_data).has_live_reload
    }
}
