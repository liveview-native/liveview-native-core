use core::str;
use std::{collections::HashMap, time::Duration};

use log::{debug, trace};
use phoenix_channels_client::{Payload, JSON};
use reqwest::{header::LOCATION, Client, Url};
use serde::Serialize;

use crate::{
    client::{DeadRenderFetchOpts, Method},
    dom::{AttributeName, Document, ElementName, Selector},
    error::LiveSocketError,
};

const MAX_REDIRECTS: usize = 10;
const LVN_VSN: &str = "2.0.0";
const LVN_VSN_KEY: &str = "vsn";
const CSRF_KEY: &str = "_csrf_token";
const MOUNT_KEY: &str = "_mounts";
const FMT_KEY: &str = "_format";

/// Static information ascertained from the dead render when connecting.
#[derive(Clone, Debug)]
pub struct SessionData {
    pub connect_opts: DeadRenderFetchOpts,
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
        timeout: Duration,
        connect_opts: DeadRenderFetchOpts,
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

        let (dead_render, url) =
            get_dead_render(url, format, &connect_opts, timeout, client).await?;
        //TODO: remove cookies, pull it from the cookie client cookie store.

        log::trace!("dead render retrieved:\n {dead_render}");
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
                    .next_back()
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
                    .next_back()
                    .flatten()
            })
            .filter(|iframe_src| iframe_src == "/phoenix/live_reload/frame")
            .last();

        let has_live_reload = live_reload_iframe.is_some();

        let out = Self {
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

async fn get_dead_render(
    url: &Url,
    format: &str,
    options: &DeadRenderFetchOpts,
    timeout: Duration,
    client: Client,
) -> Result<(Document, Url), LiveSocketError> {
    let DeadRenderFetchOpts {
        headers,
        body,
        method,
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

    let (client, request) = builder.timeout(timeout).headers(headers).build_split();

    let mut resp = client.execute(request?).await?;

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
    }

    let status = resp.status();

    //let cookies = jar
    //    .cookies(&url)
    //    .as_ref()
    //    .and_then(|cookie_text| cookie_text.to_str().ok())
    //    .map(|text| {
    //        text.split(";")
    //            .map(str::trim)
    //            .map(String::from)
    //            .collect::<Vec<_>>()
    //    })
    //    .unwrap_or_default();

    let url = resp.url().clone();
    let resp_text = resp.text().await?;

    if !status.is_success() {
        return Err(LiveSocketError::ConnectionError(resp_text));
    }

    let dead_render = Document::parse(&resp_text)?;
    trace!("document:\n{dead_render}\n\n\n");
    Ok((dead_render, url))
}
