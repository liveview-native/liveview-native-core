use std::{sync::Arc, collections::HashMap, time::Duration};
use phoenix_channels_client::{
    Socket,
    Topic,
    Payload,
    Channel,
    JSON,
    url::Url, Number, Event,
};
use log::{
    debug,
    error,
};
use crate::{
    dom::{
        Selector, AttributeName, ElementName,
    },
    parser::parse,
    diff::fragment::{
        FragmentMerge,
        Root,
        RootDiff,
    },
};

mod error;
use error::{
    LiveSocketError,
    UploadError
};

#[cfg(test)]
mod tests;



#[derive(uniffi::Object)]
pub struct LiveSocket {
    pub socket: Arc<Socket>,
    pub csrf_token: String,
    pub phx_id: String,
    pub phx_static: String,
    pub phx_session: String,
    timeout: Duration,
}
#[derive(uniffi::Object)]
pub struct LiveChannel {
    pub channel: Arc<Channel>,
    pub socket: Arc<Socket>,
    pub join_payload: Payload,
    document: Root,
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

#[uniffi::export(async_runtime = "tokio")]
impl LiveChannel {
    pub async fn merge_diffs(&self) -> Result<(), LiveSocketError> {
        // TODO: This should probably take the event closure to send changes back to swift/kotlin
        let mut document = self.document.clone();
        let events = self.channel.events();
        loop {
            let event = events.event().await?;
            if let Event::User { user: user_event } = event.event {
                if user_event == "diff" {
                    let payload = event.payload.to_string();
                    debug!("PAYLOAD: {payload}");
                    let diff : RootDiff = serde_json::from_str(payload.as_str())?;
                    debug!("diff: {diff:#?}");
                    document = document.merge(diff)?;
                }
            }
        }
    }
    pub fn join_payload(&self) -> Payload {
        self.join_payload.clone()
    }
    pub fn get_phx_ref_from_upload_join_payload(&self) -> Result<String, LiveSocketError> {
        let new_root = match self.join_payload {
            Payload::JSONPayload {
                json: JSON::Object {
                    ref object
                },
            } => {
                if let Some(rendered) = object.get("rendered") {
                    let rendered = rendered.to_string();
                    let root : RootDiff = serde_json::from_str(rendered.as_str())?;
                    let root : Root = root.try_into()?;
                    let root : String = root.try_into()?;
                    let document = parse(&root)?;
                    Some(document)
                } else {
                    None
                }
            },
            _ => {
                None
            }
        };
        let document = new_root.ok_or(LiveSocketError::NoDocumentInJoinPayload)?;
        debug!("Join payload render: {document}");

        let phx_input_id = document.select(Selector::Attribute(AttributeName {
            namespace: None,
            name: "data-phx-upload-ref".into(),
        },
        ))
            .last()
            .map(|node_ref| document.get(node_ref))
            .map(|input_div| {
                input_div
                    .attributes()
                    .iter()
                    .filter(|attr| attr.name.name == "id")
                    .map(|attr| attr.value.clone())
                    .collect::<Option<String>>()
            }).flatten()
        .ok_or(LiveSocketError::NoInputRefInDocument);
        phx_input_id
    }
    pub async fn validate_upload(&self, file: &LiveFile) -> Result<Payload, LiveSocketError> {
        // Validate the inputs
        //let phx_upload_id = phx_input_id.clone().unwrap();
        let validate_event_string = format!(r#"{{
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
        }}"#, file.phx_id, file.name, file.file_type, file.contents.len());

        let validate_event: Event = Event::User {
            user: "event".to_string(),
        };
        let validate_event_payload: Payload = Payload::json_from_serialized(validate_event_string)?;
        let validate_resp = self.channel.call(
            validate_event,
            validate_event_payload,
            self.timeout
        ).await;
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
        let upload_event : Event = Event::User {
            user: "allow_upload".to_string(),
        };
        let event_string = format!(r#"{{
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
        let allow_upload_resp = self.channel.call(upload_event, event_payload, self.timeout).await?;
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
                json: JSON::Object {
                    ref object
                },
            } => {
                if let Some(JSON::Object{ ref object }) = object.get("config") {
                    if let Some(JSON::Numb { number: Number::PosInt{ pos }}) = object.get("chunk_size") {
                        upload_config.chunk_size = *pos;
                    }
                    if let Some(JSON::Numb { number: Number::PosInt{ pos }}) = object.get("max_file_size") {
                        upload_config.max_file_size = *pos;
                    }
                    if let Some(JSON::Numb { number: Number::PosInt{ pos }}) = object.get("max_entries") {
                        upload_config.max_entries = *pos;
                    }
                }
                if let Some(JSON::Array{ array }) = object.get("error") {
                    if let Some(JSON::Array { array } ) = array.first() {
                        if let Some(JSON::Str{ string: error_string}) = array.last() {
                            error!("Upload error string: {error_string}");
                            let upload_error = match error_string.as_str() {
                                "too_large" => {
                                    UploadError::FileTooLarge
                                }
                                "not_accepted" => {
                                    UploadError::FileNotAccepted
                                }

                                other => {
                                    UploadError::Other{ error: other.to_string()}
                                }
                            };
                            return Err(upload_error)?;
                        }
                    }
                }
                if let Some(entries) = object.get("entries") {
                    match entries {
                        JSON::Object { object } => {
                            if let Some(JSON::Str { string }) = object.get("0") {
                                Some(string)
                            } else {
                                None
                            }
                        },
                        _ => {
                            None
                        }
                    }
                } else {
                    None
                }
            },
            _ => {
                None
            },
        };

        let upload_token = upload_token.ok_or(LiveSocketError::NoUploadToken)?;
        debug!("Upload token: {upload_token:?}");

        // Given the token from the "allow_upload" event, we need to create a new channel `lvu:0`
        // with the token.
        let upload_join_payload = format!(r#"{{ "token": "{}" }}"#, upload_token);
        let upload_join_payload = Payload::json_from_serialized(upload_join_payload)?;
        let upload_channel = self.socket.channel(Topic::from_string("lvu:0".to_string()), Some(upload_join_payload)).await?;
        let upload_join_resp = upload_channel.join(self.timeout).await;
        // The good response for a joining the upload channel is "{}"
        debug!("UPLOAD JOIN: {upload_join_resp:#?}");

        let chunk_size = upload_config.chunk_size as usize;
        let file_size = file.contents.len();
        let chunk_start_indices = (0..file_size).step_by(chunk_size);
        let chunk_end_indices = (chunk_size..file_size).step_by(chunk_size).chain(vec![file_size]);

        for (start_chunk, end_chunk) in chunk_start_indices.zip(chunk_end_indices) {
            debug!("Upload offsets: {start_chunk}, {end_chunk}");
            let chunk_event: Event = Event::User {
                user: "chunk".to_string(),
            };
            let chunk_payload: Payload = Payload::Binary {
                bytes: file.contents[start_chunk..end_chunk].to_vec(),
            };
            let _chunk_resp = upload_channel.call(chunk_event, chunk_payload, self.timeout).await;


            let progress = ((end_chunk as f64/ file_size as f64) * 100.0) as i8;
            // We must inform the server we've reached 100% upload via the progress.
            let progress_event_string = format!(
                r#"{{"event":null, "ref":"{}", "entry_ref":"0", "progress":{} }}"#,
                file.phx_id,
                progress,
            );


            let progress_event: Event = Event::User {
                user: "progress".to_string(),
            };
            let progress_event_payload: Payload = Payload::json_from_serialized(progress_event_string)?;
            let progress_resp = self.channel.call(progress_event, progress_event_payload, self.timeout).await;
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
        let progress_resp = self.channel.call(progress_event, progress_event_payload, self.timeout).await;
        debug!("RESP: {progress_resp:#?}");

        // TODO: Use chunk size and max_file_size

        let save_event_string = r#"{"type":"form","event":"save","value":""}"#;

        let save_event: Event = Event::User {
            user: "event".to_string(),
        };
        let save_event_payload: Payload = Payload::json_from_serialized(save_event_string.to_string())?;
        let save_resp = self.channel.call(save_event, save_event_payload, self.timeout).await;
        debug!("RESP: {save_resp:#?}");
        upload_channel.leave().await?;
        Ok(())
    }

}

#[uniffi::export(async_runtime = "tokio")]
impl LiveSocket {

    #[uniffi::constructor]
    pub fn new(url: String, timeout: Duration) -> Result<Self, LiveSocketError> {
        let url = url.parse::<Url>()?;
        let resp = futures::executor::block_on(async_compat::Compat::new(async {
            reqwest::get(url.clone()).await
        }))?;
        let resp_headers = resp.headers();
        let mut cookies: Vec<String> = Vec::new();
        for cookie in resp_headers.get_all("set-cookie") {
            cookies.push(cookie.to_str().expect("Cookie is not ASCII").to_string());
        }

        let resp_text = futures::executor::block_on(async_compat::Compat::new(async {
            resp.text().await
        }))?;

        let document = parse(&resp_text)?;
        debug!("document: {document}\n\n\n");

        let csrf_token = document.select(Selector::Tag(ElementName {
                namespace: None,
                name: "csrf-token".into(),
            },
        ))
            .last()
            .map(|node_ref| document.get(node_ref))
            .map(|node| node.attributes().first().map(|attr| attr.value.clone()))
            .flatten()
            .flatten()
            .ok_or(LiveSocketError::CSFRTokenMissing)?;

        let mut phx_id: Option<String> = None;
        let mut phx_static: Option<String> = None;
        let mut phx_session: Option<String> = None;
        let main_div_attributes = document.select(Selector::Attribute(AttributeName {
                namespace: None,
                name: "data-phx-main".into(),
            },
        ))
            .last();
        debug!("MAIN DIV: {main_div_attributes:?}");
        let main_div_attributes = document.select(Selector::Attribute(AttributeName {
                namespace: None,
                name: "data-phx-main".into(),
            },
        ))
            .last()
            .map(|node_ref| document.get(node_ref)).map(|main_div| main_div.attributes())
            .ok_or(LiveSocketError::PhoenixMainMissing)?;

        for attr in main_div_attributes {
            if attr.name.name == "id" {
                phx_id = attr.value.clone();
            } else if attr.name.name == "data-phx-session" {
                phx_session = attr.value.clone();
            } else if attr.name.name == "data-phx-static" {
                phx_static = attr.value.clone();
            }
        }
        let phx_id = phx_id.ok_or(LiveSocketError::PhoenixIDMissing)?;
        let phx_static = phx_static.ok_or(LiveSocketError::PhoenixStaticMissing)?;
        let phx_session = phx_session.ok_or(LiveSocketError::PhoenixSessionMissing)?;
        debug!("phx_id = {phx_id:?}, session = {phx_session:?}, static = {phx_static:?}");

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

        let websocket_scheme = match url.scheme()  {
            "https" => "wss",
            "http" => "ws",
            scheme => return Err(LiveSocketError::SchemeNotSupported{scheme: scheme.to_string()}),
        };
        let port = url.port().map(|port| format!(":{port}")).unwrap_or("".to_string());
        let host = url.host_str().ok_or(LiveSocketError::NoHostInURL)?;
        let websocket_url = format!(
            "{}://{}{}/live/websocket?_csrf_token={}&vsn=2.0.0&_mount={mounts}",
            websocket_scheme,
            host,
            port,
            csrf_token,
        );
        debug!("websocket url: {websocket_url}");

        let websocket_url = websocket_url.parse::<Url>()?;
        let socket = futures::executor::block_on(async_compat::Compat::new(async {
            Socket::spawn(websocket_url.clone(), Some(cookies))
        }))?;

        Ok(Self {
            socket,
            csrf_token,
            phx_id,
            phx_static,
            phx_session,
            timeout,
        })
    }

    pub async fn join_liveview_channel(&self) -> Result<LiveChannel, LiveSocketError> {
        let _ = self.socket.connect(self.timeout).await?;
        let join_payload = Payload::JSONPayload {
            json: JSON::Object {
                object: HashMap::from([
                            ("static".to_string(),  JSON::Str { string: self.phx_static.clone()}),
                            ("session".to_string(), JSON::Str { string: self.phx_session.clone()}),
                            ("params".to_string(),  JSON::Object {
                                object: HashMap::from([
                                            ("_mounts".to_string(), JSON::Numb { number: Number::PosInt { pos: 0 }}),
                                            ("_csrf_token".to_string(), JSON::Str { string: self.csrf_token.clone() }),
                                ])
                            })
                ]),
            }
        };

        let channel = self.socket.channel(Topic::from_string(format!("lv:{}", self.phx_id)), Some(join_payload)).await?;
        let join_payload = channel.join(self.timeout).await?;
        let root = match join_payload {
            Payload::JSONPayload {
                json: JSON::Object {
                    ref object
                },
            } => {
                if let Some(rendered) = object.get("rendered") {
                    let rendered = rendered.to_string();
                    let root : RootDiff = serde_json::from_str(rendered.as_str())?;
                    let root : Root = root.try_into()?;
                    Some(root)
                } else {
                    None
                }
            },
            _ => {
                None
            }
        }.ok_or(LiveSocketError::NoDocumentInJoinPayload)?;
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
        Ok(LiveChannel {
            channel: channel,
            join_payload,
            socket: self.socket.clone(),
            document: root,
            timeout: self.timeout,
        })
    }

    pub fn socket(&self) -> Arc<Socket> {
        self.socket.clone()
    }

}

