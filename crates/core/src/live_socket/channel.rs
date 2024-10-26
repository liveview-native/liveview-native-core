use std::{sync::Arc, time::Duration};

use log::{debug, error};
use phoenix_channels_client::{Channel, Event, Number, Payload, Socket, Topic, JSON};

use super::{protocol, LiveSocketError, UploadConfig, UploadError};
use crate::{
    diff::fragment::{Root, RootDiff},
    dom::{
        ffi::{Document as FFiDocument, DocumentChangeHandler},
        AttributeName, AttributeValue, Document, Selector,
    },
    parser::parse,
};

#[derive(uniffi::Object)]
pub struct LiveChannel {
    pub channel: Arc<Channel>,
    pub socket: Arc<Socket>,
    pub join_payload: Payload,
    pub document: FFiDocument,
    pub timeout: Duration,
    /// unique file upload ID for this view
    pub file_upload_id: Option<String>,
}

#[derive(uniffi::Object)]
pub struct LiveFile {
    contents: Vec<u8>,
    mime_type: String,
    path: String,
    phx_target_name: String,
    upload_id: String,
    ref_id: u64,
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

    pub fn construct_upload(
        &self,
        contents: Vec<u8>,
        mime_type: String,
        file_path: String,
        phx_target_name: String,
    ) -> Result<LiveFile, LiveSocketError> {
        // this is not great but we have to mimic constructing
        // this ad hoc object to send to the server
        // https://github.com/phoenixframework/phoenix_live_view/blob/b59bede3fcec6995f1d5876a520af8badc4bb7fb/priv/static/phoenix_live_view.js#L1315
        let ref_id = self.document().next_upload_id();

        // find the upload with target equal to phx_target_name
        // retrieve the security token
        let node_ref = self
            .document()
            .inner()
            .lock()
            .expect("lock poison!")
            .select(Selector::And(
                Box::new(Selector::Attribute(AttributeName {
                    namespace: None,
                    name: "data-phx-upload-ref".into(),
                })),
                Box::new(Selector::AttributeValue(
                    AttributeName {
                        namespace: None,
                        name: "name".into(),
                    },
                    AttributeValue::String(phx_target_name.clone().into()),
                )),
            ))
            .nth(0);

        let upload_id = node_ref
            .map(|node_ref| self.document().get(node_ref.into()))
            .and_then(|input_div| {
                input_div
                    .attributes()
                    .iter()
                    .filter(|attr| attr.name.name == "id")
                    .map(|attr| attr.value.clone())
                    .collect::<Option<String>>()
            })
            .ok_or(LiveSocketError::NoInputRefInDocument)?;

        Ok(LiveFile {
            contents,
            mime_type,
            path: file_path,
            phx_target_name,
            upload_id,
            ref_id,
        })
    }

    // Blocks indefinitely, processing changes to the document using the user provided callback
    // In `set_event_handler`
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
                        document.merge_fragment_json(&payload)?;
                    }
                }
            };
        }
    }

    pub fn join_payload(&self) -> Payload {
        self.join_payload.clone()
    }

    pub async fn validate_upload(&self, file: &LiveFile) -> Result<Payload, LiveSocketError> {
        // TODO: move this into protocol
        let validate_event_payload = serde_json::json!(
        {
          "type" : "form",
          "event" : "validate",
          "value" : format!("_target={}", file.phx_target_name),
          "uploads" : {
              file.upload_id.clone() : [{
                  "relative_path" : "", // only needed with multiple uploads, fully qualified path
                  "path" : file.phx_target_name, // poorly, poorly named field. refers to an atom
                  "ref" :  file.ref_id.to_string(), // uniformly increasing id issued by current client
                  "name" : file.path, // the actual file name - potentially just the root
                  "type" : file.mime_type, // www3 mime string
                  "size" : file.contents.len(), // byte length
              }]
          },
        });

        let _mime_type: mime::Mime =
            file.mime_type
                .parse()
                .map_err(|e| LiveSocketError::MimeType {
                    error: format!("{e}"),
                })?;

        let validate_event: Event = Event::User {
            user: "event".to_string(),
        };

        let validate_event_payload: Payload = Payload::JSONPayload {
            json: JSON::from(validate_event_payload),
        };

        let validate_resp = self
            .channel
            .call(validate_event, validate_event_payload, self.timeout)
            .await?;

        let Payload::JSONPayload { json } = &validate_resp else {
            return Err(LiveSocketError::PayloadNotJson);
        };

        let value = serde_json::Value::from(json.clone());
        let _s: protocol::ValidateResponse = serde_json::from_value(value)?;

        // TODO: move this into protocol
        /* Validate "okay" response looks like:
        {
        "diff": {
            "0": { // The file upload id
                "2": " accept=\".jpg,.jpeg,.ico\"",
                "4": " data-phx-active-refs=\"0\"",
                "5": " data-phx-done-refs=\"\"",
                "6": " data-phx-preflighted-refs=\"\"",
                "8": " multiple"
            }
        }
        */

        // TODO: Use the validate response.
        Ok(validate_resp)
    }

    pub async fn upload_file(&self, file: &LiveFile) -> Result<(), LiveSocketError> {
        // Allow upload requests to upload.
        let upload_event: Event = Event::User {
            user: "allow_upload".to_string(),
        };

        let upload_id = self
            .file_upload_id
            .as_ref()
            .ok_or(LiveSocketError::NoInputRefInDocument)?;

        // TODO: move this into protocol
        let event_string = format!(
            r#"{{
            "ref":"{}",
            "entries":[
                {{
                    "name":"{}",
                    "relative_path":"",
                    "size":{},
                    "type":"{}",
                    "ref":"{}"
                }}
            ]
            }}"#,
            upload_id,
            file.path,
            file.contents.len(),
            file.mime_type,
            file.ref_id
        );

        let event_payload: Payload = Payload::json_from_serialized(event_string)?;
        // TODO: Create a configurable, introspectable upload interface so control
        // timeouts
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
        // TODO: move this into protocol
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
                    // TODO: search for the actual ID from the live file
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
                    if let Some(JSON::Str { string }) = object.get(&file.ref_id.to_string()) {
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
                Topic::from_string(format!("lvu:{}", file.ref_id)),
                Some(upload_join_payload),
            )
            .await?;

        let upload_join_resp = upload_channel.join(self.timeout).await?;
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

            // TODO: zero copy
            let chunk_payload: Payload = Payload::Binary {
                bytes: file.contents[start_chunk..end_chunk].to_vec(),
            };

            let _chunk_resp = upload_channel
                .call(chunk_event, chunk_payload, self.timeout)
                .await?;

            debug!("Chunk upload resp: {_chunk_resp}");

            let progress = ((end_chunk as f64 / file_size as f64) * 100.0) as i8;

            if progress < 100 {
                // We must inform the server we've reached 100% upload via the progress.
                // TODO: move this into protocol
                let progress_event_string = format!(
                    r#"{{"event":null, "ref":"{}", "entry_ref":"{}", "progress":{} }}"#,
                    upload_id, file.ref_id, progress,
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
                    .await?;

                debug!("Progress response: {progress_resp:#?}");
            }
        }

        // TODO: move this into protocol
        let progress_event_string = format!(
            r#"{{"event":null, "ref":"{}", "entry_ref":"{}", "progress": 100 }}"#,
            upload_id, file.ref_id,
        );

        let progress_event: Event = Event::User {
            user: "progress".to_string(),
        };

        let progress_event_payload: Payload = Payload::json_from_serialized(progress_event_string)?;
        let progress_resp = self
            .channel
            .call(progress_event, progress_event_payload, self.timeout)
            .await?;

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
            .await?;

        debug!("RESP: {save_resp:#?}");
        upload_channel.leave().await?;
        Ok(())
    }
}
