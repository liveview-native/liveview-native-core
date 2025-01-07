use std::sync::{Arc, Mutex};

use log::{debug, trace};
use phoenix_channels_client::{Payload, Socket, Topic, JSON};

use super::LiveViewClientConfiguration;
use crate::{
    diff::fragment::{Root, RootDiff},
    dom::Document,
    live_socket::{LiveChannel, LiveSocketError, SessionData},
};

const LVN_VSN: &str = "2.0.0";
const LVN_VSN_KEY: &str = "vsn";

pub async fn join_liveview_channel(
    config: &LiveViewClientConfiguration,
    socket: &Mutex<Arc<Socket>>,
    session_data: &Mutex<SessionData>,
    redirect: Option<String>,
) -> Result<Arc<LiveChannel>, LiveSocketError> {
    let ws_timeout = std::time::Duration::from_millis(config.websocket_timeout);

    let sock = socket.try_lock()?.clone();
    sock.connect(ws_timeout).await?;

    let sent_join_payload = session_data
        .try_lock()?
        .create_join_payload(&config.join_params, redirect);

    let topic = Topic::from_string(format!("lv:{}", session_data.try_lock()?.phx_id));
    let channel = sock.channel(topic, Some(sent_join_payload)).await?;

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
