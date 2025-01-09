use core::str;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use log::{debug, trace};
use phoenix_channels_client::{url::Url, Payload, Socket, SocketStatus, Topic, JSON};
use reqwest::{
    cookie::{CookieStore, Jar},
    header::{HeaderMap, LOCATION, SET_COOKIE},
    redirect::Policy,
    Client, Method as ReqMethod,
};
use serde::Serialize;

use super::navigation::{NavCtx, NavOptions};
pub use super::LiveChannel;
use crate::{
    diff::fragment::{Root, RootDiff},
    dom::{ffi::Document as FFiDocument, AttributeName, Document, ElementName, Selector},
    error::LiveSocketError,
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

#[cfg(not(test))]
use std::sync::OnceLock;
#[cfg(not(test))]
pub static COOKIE_JAR: OnceLock<Arc<Jar>> = OnceLock::new();

// Each test runs in a separate thread and should make requests
// as if it is an isolated session.
#[cfg(test)]
thread_local! {
    pub static TEST_COOKIE_JAR: Arc<Jar> = Arc::default();
}

const MAX_REDIRECTS: usize = 10;
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
    pub join_headers: HashMap<String, Vec<String>>,
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

//TODO: Move this into the protocol module when it exists
/// The expected structure of a json payload send upon joining a liveview channel
#[derive(Serialize)]
struct JoinRequestPayload {
    #[serde(rename = "static")]
    static_token: String,
    session: String,
    #[serde(flatten)]
    url_or_redirect: UrlOrRedirect,
    params: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum UrlOrRedirect {
    Url { url: String },
    Redirect { redirect: String },
}

impl SessionData {
    pub async fn request(
        url: &Url,
        format: &String,
        connect_opts: ConnectOpts,
        client: Client,
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

        let (dead_render, cookies, url, header_map) =
            LiveSocket::get_dead_render(url, format, &connect_opts, client).await?;
        //TODO: remove cookies, pull it from the cookie client cookie store.

        let csrf_token = dead_render
            .get_csrf_token()
            .ok_or(LiveSocketError::CSRFTokenMissing)?;

        let mut phx_id: Option<String> = None;
        let mut phx_static: Option<String> = None;
        let mut phx_session: Option<String> = None;

        let main_div_attributes = dead_render
            .select(Selector::Attribute(AttributeName {
                name: "data-phx-main".into(),
                namespace: None,
            }))
            .last();

        trace!("main div attributes: {main_div_attributes:?}");

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
        trace!("phx_id = {phx_id:?}, session = {phx_session:?}, static = {phx_static:?}");

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

        let mut join_headers = HashMap::new();

        for key in header_map.keys() {
            let entries = header_map
                .get_all(key)
                .iter()
                .filter_map(|value| Some(value.to_str().ok()?.to_string()))
                .collect();

            join_headers.insert(key.to_string(), entries);
        }

        let out = Self {
            join_headers,
            connect_opts,
            url,
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

        debug!("Session data successfully acquired");
        debug!("{out:?}");

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

    pub fn create_join_payload(
        &self,
        additional_params: &Option<HashMap<String, JSON>>,
        redirect: Option<String>,
    ) -> Payload {
        let mut params = HashMap::new();
        params.insert(MOUNT_KEY.to_string(), serde_json::json!(0));
        params.insert(CSRF_KEY.to_string(), serde_json::json!(self.csrf_token));
        params.insert(FMT_KEY.to_string(), serde_json::json!(self.format));

        if let Some(join_params) = additional_params {
            params.extend(
                join_params
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone().into())),
            );
        }

        let payload = JoinRequestPayload {
            static_token: self.phx_static.clone(),
            session: self.phx_session.clone(),
            url_or_redirect: redirect
                .map(|r| UrlOrRedirect::Redirect { redirect: r })
                .unwrap_or_else(|| UrlOrRedirect::Url {
                    url: self.url.to_string(),
                }),
            params,
        };

        let json = serde_json::to_value(payload).expect("Serde Error");
        Payload::JSONPayload { json: json.into() }
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
        client: Client,
    ) -> Result<(Document, Vec<String>, Url, HeaderMap), LiveSocketError> {
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
        if url.query_pairs().all(|(name, _)| name != FMT_KEY) {
            url.query_pairs_mut().append_pair(FMT_KEY, format);
        }

        let headers = (&headers.clone().unwrap_or_default())
            .try_into()
            .map_err(|e| LiveSocketError::InvalidHeader {
                error: format!("{e:?}"),
            })?;

        let req = reqwest::Request::new(method, url.clone());
        let builder = reqwest::RequestBuilder::from_parts(client, req);

        let builder = if let Some(body) = body {
            builder.body(body.clone())
        } else {
            builder
        };

        let timeout = Duration::from_millis(*timeout_ms);
        let (client, request) = builder.timeout(timeout).headers(headers).build_split();

        let mut resp = client.execute(request?).await?;
        let mut headers = resp.headers().clone();

        for _ in 0..MAX_REDIRECTS {
            if !resp.status().is_redirection() {
                log::debug!("{resp:?}");
                break;
            }
            log::debug!("-- REDIRECTING -- ");
            log::debug!("{resp:?}");

            let mut location = resp
                .headers()
                .get(LOCATION)
                .and_then(|loc| str::from_utf8(loc.as_bytes()).ok())
                .and_then(|loc| url.join(loc).ok())
                .ok_or_else(|| LiveSocketError::Request {
                    error: "No valid redirect location in 300 response".into(),
                })?;

            if location.query_pairs().all(|(name, _)| name != FMT_KEY) {
                location.query_pairs_mut().append_pair(FMT_KEY, format);
            }

            resp = client.get(location).send().await?;

            // TODO: Remove this when persistent state is managed by core
            let cookies = resp.headers().get_all(SET_COOKIE);

            for cookie in cookies {
                if headers.try_append(SET_COOKIE, cookie.clone()).is_err() {
                    log::error!("Could not collect set cookie headers");
                }
            }
        }

        let status = resp.status();

        #[cfg(not(test))]
        let jar = COOKIE_JAR.get_or_init(|| Jar::default().into());

        #[cfg(test)]
        let jar = TEST_COOKIE_JAR.with(|inner| inner.clone());

        let cookies = jar
            .cookies(&url)
            .as_ref()
            .and_then(|cookie_text| cookie_text.to_str().ok())
            .map(|text| {
                text.split(";")
                    .map(str::trim)
                    .map(String::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let url = resp.url().clone();
        let resp_text = resp.text().await?;

        if !status.is_success() {
            return Err(LiveSocketError::ConnectionError(resp_text));
        }

        let dead_render = Document::parse(&resp_text)?;
        trace!("document:\n{dead_render}\n\n\n");
        Ok((dead_render, cookies, url, headers))
    }
}
/// Stores a cookie for the duration of the application run.
#[uniffi::export]
pub fn store_session_cookie(cookie: String, url: String) -> Result<(), LiveSocketError> {
    let url = Url::parse(&url)?;

    #[cfg(not(test))]
    let jar = COOKIE_JAR.get_or_init(|| Jar::default().into());

    #[cfg(test)]
    let jar = TEST_COOKIE_JAR.with(|inner| inner.clone());

    jar.add_cookie_str(&cookie, &url);

    Ok(())
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

        #[cfg(not(test))]
        let jar = COOKIE_JAR.get_or_init(|| Jar::default().into());

        #[cfg(test)]
        let jar = TEST_COOKIE_JAR.with(|inner| inner.clone());

        let client = reqwest::Client::builder()
            .cookie_provider(jar.clone())
            .redirect(Policy::none())
            .build()?;

        // Make HTTP request to get initial dead render, an HTML document with
        // metadata needed to set up the liveview websocket connection.
        let session_data = SessionData::request(&url, &format, options, client).await?;
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

    /// Returns the url of the final dead render
    pub fn join_url(&self) -> String {
        lock!(self.session_data).url.to_string().clone()
    }

    /// Returns the headers of the final dead render response
    pub fn join_headers(&self) -> HashMap<String, Vec<String>> {
        lock!(self.session_data).join_headers.clone()
    }

    pub fn csrf_token(&self) -> String {
        lock!(self.session_data).csrf_token.clone()
    }

    pub fn cookies(&self) -> Vec<String> {
        lock!(self.session_data).cookies.clone()
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
            join_params: Default::default(),
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

        let join_payload = session_data.create_join_payload(&join_params, redirect);

        let channel = self
            .socket()
            .channel(
                Topic::from_string(format!("lv:{}", session_data.phx_id)),
                Some(join_payload),
            )
            .await?;

        let join_payload = channel.join(self.timeout()).await?;

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
            join_params: join_params.unwrap_or_default(),
            socket: self.socket(),
            document: document.into(),
            timeout: self.timeout(),
        })
    }

    /// Returns the connection timeout duration for each connection attempt
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(lock!(self.session_data).connect_opts.timeout_ms)
    }

    /// Returns the socket status
    pub fn status(&self) -> SocketStatus {
        self.socket().status()
    }

    pub fn socket(&self) -> Arc<Socket> {
        lock!(self.socket).clone()
    }

    pub fn has_live_reload(&self) -> bool {
        lock!(self.session_data).has_live_reload
    }
}
