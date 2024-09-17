use std::{collections::HashMap, sync::Arc, time::Duration};

use log::{debug, error};
use phoenix_channels_client::{url::Url, Channel, Event, Number, Payload, Socket, Topic, JSON};

use crate::{
    diff::fragment::{Root, RootDiff},
    dom::{
        ffi::{Document as FFiDocument, DocumentChangeHandler},
        AttributeName, Document, ElementName, Selector,
    },
    parser::parse,
};

mod error;
use error::{LiveSocketError, UploadError};

#[cfg(test)]
mod tests;

#[derive(uniffi::Object)]
pub struct LiveSocket {
    pub socket: Arc<Socket>,
    pub csrf_token: String,
    pub phx_id: String,
    pub phx_static: String,
    pub phx_session: String,
    pub url: Url,
    pub format: String,
    pub dead_render: Document,
    pub style_urls: Vec<String>,
    pub has_live_reload: bool,
    cookies: Vec<String>,
    timeout: Duration,
}
#[derive(uniffi::Object)]
pub struct LiveFile {
    contents: Vec<u8>,
    file_type: String,
    name: String,
    phx_id: String,
}
#[uniffi::export]
impl LiveFile {
    #[uniffi::constructor]
    pub fn new(contents: Vec<u8>, file_type: String, name: String, phx_id: String) -> Self {
        Self {
            contents,
            file_type,
            name,
            phx_id,
        }
    }
}
pub struct UploadConfig {
    chunk_size: u64,
    max_file_size: u64,
    max_entries: u64,
}

/// Defaults from https://hexdocs.pm/phoenix_live_view/Phoenix.LiveView.html#allow_upload/3
impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64_000,
            max_file_size: 8000000,
            max_entries: 1,
        }
    }
}
#[derive(uniffi::Object)]
pub struct LiveChannel {
    pub channel: Arc<Channel>,
    pub socket: Arc<Socket>,
    pub join_payload: Payload,
    document: FFiDocument,
    timeout: Duration,
}
// For non FFI functions
impl LiveChannel {
    pub fn join_document(&self) -> Result<Document, LiveSocketError> {
        let new_root = match self.join_payload {
            Payload::JSONPayload {
                json: JSON::Object { ref object },
            } => {
                if let Some(rendered) = object.get("rendered") {
                    let rendered = rendered.to_string();
                    let root: RootDiff = serde_json::from_str(rendered.as_str())?;
                    let root: Root = root.try_into()?;
                    let root: String = root.try_into()?;
                    let document = parse(&root)?;
                    Some(document)
                } else {
                    None
                }
            }
            _ => None,
        };
        let document = new_root.ok_or(LiveSocketError::NoDocumentInJoinPayload)?;
        debug!("Join payload render:\n{document}");
        Ok(document)
    }
}

#[cfg_attr(not(target_family = "wasm"), uniffi::export(async_runtime = "tokio"))]
impl LiveChannel {
    pub fn document(&self) -> FFiDocument {
        self.document.clone()
    }
    pub fn channel(&self) -> Arc<Channel> {
        self.channel.clone()
    }
    pub fn set_event_handler(&self, handler: Box<dyn DocumentChangeHandler>) {
        self.document.set_event_handler(handler);
    }

    pub async fn merge_diffs(&self) -> Result<(), LiveSocketError> {
        // TODO: This should probably take the event closure to send changes back to swift/kotlin
        let document = self.document.clone();
        let events = self.channel.events();
        loop {
            let event = events.event().await?;
            match event.event {
                Event::Phoenix { phoenix } => {
                    error!("Phoenix Event for {phoenix:?} is unimplemented");
                }
                Event::User { user } => {
                    if user == "diff" {
                        let payload = event.payload.to_string();
                        debug!("PAYLOAD: {payload}");
                        // This function merges and uses the event handler set in `set_event_handler`
                        // which will call back into the Swift/Kotlin.
                        document.merge_fragment_json(payload)?;
                    }
                }
            };
        }
    }

    pub fn join_payload(&self) -> Payload {
        self.join_payload.clone()
    }
    pub fn get_phx_ref_from_upload_join_payload(&self) -> Result<String, LiveSocketError> {
        /* Okay join response looks like: To do an upload we need the new `data-phx-upload-ref`
        {
          "rendered": {
            "0": {
              "0": " id=\"phx-F6rhEydz8YniGqoB\"",
              "1": " name=\"avatar\"",
              "2": " accept=\".jpg,.jpeg,.ico\"",
              "3": " data-phx-upload-ref=\"phx-F6rhEydz8YniGqoB\"",
              "4": " data-phx-active-refs=\"\"",
              "5": " data-phx-done-refs=\"\"",
              "6": " data-phx-preflighted-refs=\"\"",
              "7": "",
              "8": " multiple",
              "s": [
                "<input",
                " type=\"file\"",
                "",
                " data-phx-hook=\"Phoenix.LiveFileUpload\" data-phx-update=\"ignore\"",
                "",
                "",
                "",
                "",
                "",
                ">"
              ],
              "r": 1
            },
            "s": [
              "<p> THIS IS AN UPLOAD FORM </p>\n<form id=\"upload-form\" phx-submit=\"save\" phx-change=\"validate\">\n\n  ",
              "\n\n  <button type=\"submit\">Upload</button>\n</form>"
            ]
          }
        }
        */
        // As silly as it sounds, rendering this diff and parsing the dom for the
        // data-phx-upload-ref seems like the most stable way.
        let document = self.join_document()?;
        let phx_input_id = document
            .select(Selector::Attribute(AttributeName {
                namespace: None,
                name: "data-phx-upload-ref".into(),
            }))
            .last()
            .map(|node_ref| document.get(node_ref))
            .and_then(|input_div| {
                input_div
                    .attributes()
                    .iter()
                    .filter(|attr| attr.name.name == "id")
                    .map(|attr| attr.value.clone())
                    .collect::<Option<String>>()
            })
            .ok_or(LiveSocketError::NoInputRefInDocument);
        phx_input_id
    }
    pub async fn validate_upload(&self, file: &LiveFile) -> Result<Payload, LiveSocketError> {
        // Validate the inputs
        //let phx_upload_id = phx_input_id.clone().unwrap();
        let validate_event_string = format!(
            r#"{{
            "type":"form",
            "event":"validate",
            "value":"_target=avatar",
            "uploads":{{
            "{}":[{{
                    "path":"avatar",
                    "ref":"0",
                    "name":"{}",
                    "relative_path":"",
                    "type":"{}",
                    "size":{}
                }}
            ]}}
        }}"#,
            file.phx_id,
            file.name,
            file.file_type,
            file.contents.len()
        );

        let validate_event: Event = Event::User {
            user: "event".to_string(),
        };
        let validate_event_payload: Payload = Payload::json_from_serialized(validate_event_string)?;
        let validate_resp = self
            .channel
            .call(validate_event, validate_event_payload, self.timeout)
            .await;
        /* Validate "okay" response looks like:
        {
        "diff": {
            "0": {
                "2": " accept=\".jpg,.jpeg,.ico\"",
                "4": " data-phx-active-refs=\"0\"",
                "5": " data-phx-done-refs=\"\"",
                "6": " data-phx-preflighted-refs=\"\"",
                "8": " multiple"
            }
        }
                 */
        // TODO: Use the validate response.
        Ok(validate_resp?)
    }

    pub async fn upload_file(&self, file: &LiveFile) -> Result<(), LiveSocketError> {
        // Allow upload requests to upload.
        let upload_event: Event = Event::User {
            user: "allow_upload".to_string(),
        };
        let event_string = format!(
            r#"{{
            "ref":"{}",
            "entries":[
                {{
                    "name":"{}",
                    "relative_path":"",
                    "size":{},
                    "type":"{}",
                    "ref":"0"
                }}
            ]
            }}"#,
            file.phx_id,
            file.name,
            file.contents.len(),
            file.file_type
        );

        let event_payload: Payload = Payload::json_from_serialized(event_string)?;
        let allow_upload_resp = self
            .channel
            .call(upload_event, event_payload, self.timeout)
            .await?;
        debug!("allow_upload RESP: {allow_upload_resp:#?}");

        /*
        The allow upload okay response looks like:
        {
            "config":{
                "chunk_size":64000,
                "max_file_size":8000000,
                "max_entries":2
            },
            "errors":{},
            "diff":{
                "0":{
                    "2":" accept=\".jpg,.jpeg,.ico\"",
                    "4":" data-phx-active-refs=\"0\"",
                    "5":" data-phx-done-refs=\"\"",
                    "6":" data-phx-preflighted-refs=\"0\"",
                    "8":" multiple"
                }
            },
            "ref":"phx-F6rgg119TbUm66NB",
            "entries":{
                "0":"SFMyNTY.g2gDaAJhBXQAAAADdwNwaWRYdw1ub25vZGVAbm9ob3N0AACcKgAAAAAAAAAAdwNyZWZoAm0AAAAUcGh4LUY2cmdnMTE5VGJVbTY2TkJtAAAAATB3A2NpZHcDbmlsbgYARL4WE40BYgABUYA.JdLOUHO83Kp17PlDLv-_gHJVXjRWbmqf1mOaUx9yBBM"
            }
        }
                Out of this JSON we need the string from entries["0"] as this is the upload token.

        The allow upload error response looks like:
        {
            "errors":[["0", "too_large"]],
        }
                */
        let mut upload_config = UploadConfig::default();
        let upload_token = match allow_upload_resp {
            Payload::JSONPayload {
                json: JSON::Object { ref object },
            } => {
                if let Some(JSON::Object { ref object }) = object.get("config") {
                    if let Some(JSON::Numb {
                        number: Number::PosInt { pos },
                    }) = object.get("chunk_size")
                    {
                        upload_config.chunk_size = *pos;
                    }
                    if let Some(JSON::Numb {
                        number: Number::PosInt { pos },
                    }) = object.get("max_file_size")
                    {
                        upload_config.max_file_size = *pos;
                    }
                    if let Some(JSON::Numb {
                        number: Number::PosInt { pos },
                    }) = object.get("max_entries")
                    {
                        upload_config.max_entries = *pos;
                    }
                }
                if let Some(JSON::Array { array }) = object.get("error") {
                    if let Some(JSON::Array { array }) = array.first() {
                        if let Some(JSON::Str {
                            string: error_string,
                        }) = array.last()
                        {
                            error!("Upload error string: {error_string}");
                            let upload_error = match error_string.as_str() {
                                "too_large" => UploadError::FileTooLarge,
                                "not_accepted" => UploadError::FileNotAccepted,

                                other => UploadError::Other {
                                    error: other.to_string(),
                                },
                            };
                            return Err(upload_error)?;
                        }
                    }
                }
                if let Some(JSON::Object { object }) = object.get("entries") {
                    if let Some(JSON::Str { string }) = object.get("0") {
                        Some(string)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        let upload_token = upload_token.ok_or(LiveSocketError::NoUploadToken)?;
        debug!("Upload token: {upload_token:?}");

        // Given the token from the "allow_upload" event, we need to create a new channel `lvu:0`
        // with the token.
        let upload_join_payload = format!(r#"{{ "token": "{}" }}"#, upload_token);
        let upload_join_payload = Payload::json_from_serialized(upload_join_payload)?;
        let upload_channel = self
            .socket
            .channel(
                Topic::from_string("lvu:0".to_string()),
                Some(upload_join_payload),
            )
            .await?;
        let upload_join_resp = upload_channel.join(self.timeout).await;
        // The good response for a joining the upload channel is "{}"
        debug!("UPLOAD JOIN: {upload_join_resp:#?}");

        let chunk_size = upload_config.chunk_size as usize;
        let file_size = file.contents.len();
        let chunk_start_indices = (0..file_size).step_by(chunk_size);
        let chunk_end_indices = (chunk_size..file_size)
            .step_by(chunk_size)
            .chain(vec![file_size]);

        for (start_chunk, end_chunk) in chunk_start_indices.zip(chunk_end_indices) {
            debug!("Upload offsets: {start_chunk}, {end_chunk}");
            let chunk_event: Event = Event::User {
                user: "chunk".to_string(),
            };
            let chunk_payload: Payload = Payload::Binary {
                bytes: file.contents[start_chunk..end_chunk].to_vec(),
            };
            let _chunk_resp = upload_channel
                .call(chunk_event, chunk_payload, self.timeout)
                .await;

            let progress = ((end_chunk as f64 / file_size as f64) * 100.0) as i8;
            // We must inform the server we've reached 100% upload via the progress.
            let progress_event_string = format!(
                r#"{{"event":null, "ref":"{}", "entry_ref":"0", "progress":{} }}"#,
                file.phx_id, progress,
            );

            let progress_event: Event = Event::User {
                user: "progress".to_string(),
            };
            let progress_event_payload: Payload =
                Payload::json_from_serialized(progress_event_string)?;
            debug!("Progress send: {progress_event_payload:#?}");
            let progress_resp = self
                .channel
                .call(progress_event, progress_event_payload, self.timeout)
                .await;
            debug!("Progress response: {progress_resp:#?}");
        }

        // We must inform the server we've reached 100% upload via the progress.
        let progress_event_string = format!(
            r#"{{"event":null, "ref":"{}", "entry_ref":"0", "progress":100 }}"#,
            file.phx_id,
        );

        let progress_event: Event = Event::User {
            user: "progress".to_string(),
        };
        let progress_event_payload: Payload = Payload::json_from_serialized(progress_event_string)?;
        let progress_resp = self
            .channel
            .call(progress_event, progress_event_payload, self.timeout)
            .await;
        debug!("RESP: {progress_resp:#?}");

        let save_event_string = r#"{"type":"form","event":"save","value":""}"#;

        let save_event: Event = Event::User {
            user: "event".to_string(),
        };
        let save_event_payload: Payload =
            Payload::json_from_serialized(save_event_string.to_string())?;
        let save_resp = self
            .channel
            .call(save_event, save_event_payload, self.timeout)
            .await;
        debug!("RESP: {save_resp:#?}");
        upload_channel.leave().await?;
        Ok(())
    }
}

#[cfg_attr(not(target_family = "wasm"), uniffi::export(async_runtime = "tokio"))]
impl LiveSocket {
    // This is just for the jetpack client. This is an associated function constructor.
    #[uniffi::constructor]
    pub async fn connect(
        url: String,
        timeout: Duration,
        format: String,
    ) -> Result<Self, LiveSocketError> {
        Self::new(url, timeout, format).await
    }

    #[uniffi::constructor]
    pub async fn new(
        url: String,
        timeout: Duration,
        format: String,
    ) -> Result<Self, LiveSocketError> {
        // TODO: Check if params contains all of phx_id, phx_static, phx_session and csrf_token, if
        // it does maybe we don't need to do a full dead render.
        let mut url = url.parse::<Url>()?;
        url.set_query(Some(&format!("_format={format}")));
        // todo: Use the format in the query param to the deadrender.
        let resp = reqwest::get(url.clone()).await?;
        let resp_headers = resp.headers();

        let mut cookies: Vec<String> = Vec::new();
        for cookie in resp_headers.get_all("set-cookie") {
            cookies.push(cookie.to_str().expect("Cookie is not ASCII").to_string());
        }
        let resp_text = resp.text().await?;

        let dead_render = parse(&resp_text)?;
        debug!("document:\n{dead_render}\n\n\n");

        // HTML responses have
        // <meta name="csrf-token"
        // content="PBkccxQnXREEHjJhOksCJ1cVESUiRgtBYZxJSKpAEMJ0tfivopcul5Eq">
        let meta_csrf_token: Option<String> = dead_render
            .select(Selector::Tag(ElementName {
                namespace: None,
                name: "meta".into(),
            }))
            .map(|node_ref| dead_render.get(node_ref))
            // We need the node of the element with a "name" attribute that equals "csrf-token"
            .filter(|node| {
                node.attributes()
                    .iter()
                    .filter(|attr| {
                        attr.name.name == *"name" && attr.value == Some("csrf-token".to_string())
                    })
                    .last()
                    .is_some()
            })
            // We now need the "content" value
            .map(|node| {
                node.attributes()
                    .iter()
                    .filter(|attr| attr.name.name == *"content")
                    .map(|attr| attr.value.clone())
                    .last()
                    .flatten()
            })
            .last()
            .flatten();

        debug!("META CSRF TOKEN: {meta_csrf_token:#?}");

        // LiveView Native responses have:
        // <csrf-token value="CgpDGHsSYUUxHxdQDSVVc1dmchgRYhMUXlqANTR3uQBdzHmK5R9mW5wu" />
        let csrf_token = dead_render
            .select(Selector::Tag(ElementName {
                namespace: None,
                name: "csrf-token".into(),
            }))
            .last()
            .map(|node_ref| dead_render.get(node_ref))
            .and_then(|node| node.attributes().first().map(|attr| attr.value.clone()))
            .flatten()
            .or(meta_csrf_token)
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

        // NEED:
        // these from inside data-phx-main
        // data-phx-session,
        // data-phx-static
        // id
        //
        // Top level:
        // csrf-token
        // "iframe[src=\"/phoenix/live_reload/frame\"]"
        let mounts = 0;

        let websocket_scheme = match url.scheme() {
            "https" => "wss",
            "http" => "ws",
            scheme => {
                return Err(LiveSocketError::SchemeNotSupported {
                    scheme: scheme.to_string(),
                })
            }
        };
        let port = url
            .port()
            .map(|port| format!(":{port}"))
            .unwrap_or("".to_string());
        let host = url.host_str().ok_or(LiveSocketError::NoHostInURL)?;
        let websocket_url = format!(
            "{}://{}{}/live/websocket?_csrf_token={}&vsn=2.0.0&_mounts={mounts}&_format={}",
            websocket_scheme, host, port, csrf_token, format
        );
        debug!("websocket url: {websocket_url}");

        let websocket_url = websocket_url.parse::<Url>()?;
        let socket = Socket::spawn(websocket_url.clone(), Some(cookies.clone())).await?;

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

        debug!("iframe src: {live_reload_iframe:?}");

        Ok(Self {
            socket,
            csrf_token,
            phx_id,
            phx_static,
            phx_session,
            timeout,
            url,
            dead_render,
            style_urls,
            has_live_reload,
            cookies,
            format,
        })
    }

    pub fn dead_render(&self) -> FFiDocument {
        self.dead_render.clone().into()
    }

    pub fn style_urls(&self) -> Vec<String> {
        self.style_urls.clone()
    }
    pub async fn join_livereload_channel(&self) -> Result<LiveChannel, LiveSocketError> {
        let mut url = self.url.clone();
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
        url.set_path("phoenix/live_reload/socket");
        let socket = Socket::spawn(url, Some(self.cookies.clone())).await?;

        let channel = socket
            .channel(Topic::from_string("phoenix:live_reload".to_string()), None)
            .await?;
        let join_payload = channel.join(self.timeout).await?;
        let document = Document::empty();

        debug!("Join payload: {join_payload:#?}");

        Ok(LiveChannel {
            channel,
            join_payload,
            socket: self.socket.clone(),
            document: document.into(),
            timeout: self.timeout,
        })
    }

    pub async fn join_liveview_channel(
        &self,
        join_params: Option<HashMap<String, JSON>>,
        redirect: Option<String>,
    ) -> Result<LiveChannel, LiveSocketError> {
        self.socket.connect(self.timeout).await?;
        let mut collected_join_params = HashMap::from([
            (
                "_mounts".to_string(),
                JSON::Numb {
                    number: Number::PosInt { pos: 0 },
                },
            ),
            (
                "_csrf_token".to_string(),
                JSON::Str {
                    string: self.csrf_token.clone(),
                },
            ),
            (
                "_format".to_string(),
                JSON::Str {
                    string: self.format.clone(),
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
                    string: self.url.to_string(),
                },
            )
        };
        let join_payload = Payload::JSONPayload {
            json: JSON::Object {
                object: HashMap::from([
                    (
                        "static".to_string(),
                        JSON::Str {
                            string: self.phx_static.clone(),
                        },
                    ),
                    (
                        "session".to_string(),
                        JSON::Str {
                            string: self.phx_session.clone(),
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
            .socket
            .channel(
                Topic::from_string(format!("lv:{}", self.phx_id)),
                Some(join_payload),
            )
            .await?;
        let join_payload = channel.join(self.timeout).await?;

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
            socket: self.socket.clone(),
            document: document.into(),
            timeout: self.timeout,
        })
    }

    pub fn socket(&self) -> Arc<Socket> {
        self.socket.clone()
    }

    pub fn has_live_reload(&self) -> bool {
        self.has_live_reload
    }
}
