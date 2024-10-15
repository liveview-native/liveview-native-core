use std::{sync::Arc, time::Duration};

use log::{debug, error};
use phoenix_channels_client::{Channel, Event, Number, Payload, Socket, Topic, JSON};

use super::{LiveFile, LiveSocketError, UploadConfig, UploadError};
use crate::{
    diff::fragment::{Root, RootDiff},
    dom::{
        ffi::{Document as FFiDocument, DocumentChangeHandler},
        AttributeName, Document, Selector,
    },
    parser::parse,
};

#[derive(uniffi::Object)]
pub enum DiffResult {}

#[derive(uniffi::Object)]
pub struct LiveChannel {
    pub channel: Arc<Channel>,
    pub socket: Arc<Socket>,
    pub join_payload: Payload,
    pub document: FFiDocument,
    pub timeout: Duration,
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

    // Blocks indefinitely, processing changes to the document using the user provided callback
    // In `set_even_handler`
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
