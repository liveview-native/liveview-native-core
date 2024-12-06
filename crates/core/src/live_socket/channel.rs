use futures::{future::FutureExt, pin_mut, select};

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use super::{
    dom_locking::{self, PHX_REF_LOCK, PHX_REF_SRC},
    LiveSocketError, UploadConfig, UploadError,
};
use crate::{
    diff::fragment::{Root, RootDiff},
    dom::{
        ffi::{Document as FFiDocument, DocumentChangeHandler},
        Attribute, AttributeName, AttributeValue, Document, NodeRef, Selector,
    },
    parser::parse,
};
use log::{debug, error};
use phoenix_channels_client::{Channel, Event, Number, Payload, Socket, Topic, JSON};

#[derive(uniffi::Object)]
pub struct LiveChannel {
    pub current_lock_id: Mutex<u64>, // Atomics not supported in wasm
    pub channel: Arc<Channel>,
    pub socket: Arc<Socket>,
    pub join_payload: Payload,
    pub document: FFiDocument,
    pub timeout: Duration,
}

#[derive(uniffi::Object)]
pub struct LiveFile {
    contents: Vec<u8>,
    mime_type: String,
    name: String,
    relative_path: String,
    phx_upload_id: String,
}

#[uniffi::export]
impl LiveFile {
    /// constructs a new `LiveFile` representing a file ready for preflight and upload.
    ///
    /// * `contents` binary contents of the file
    /// * `mime_type` Compliant mime type string of the data `image/png`, `tiff` etc.
    /// * `name` Name of the field in the input, technically, the same as the `allow_upload` atom
    /// * `relative_path` name of the file, as a relative path
    /// * `phx_upload_id` upload id acquire by passing name to [LiveChannel::get_phx_upload_id]
    #[uniffi::constructor]
    pub fn new(
        contents: Vec<u8>,
        mime_type: String,
        name: String,
        relative_path: String,
        phx_upload_id: String,
    ) -> Self {
        Self {
            contents,
            mime_type,
            name,
            relative_path,
            phx_upload_id,
        }
    }
}

// For non FFI functions
impl LiveChannel {
    /// Retrieves the initial document received upon joining the channel.
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

    pub fn next_id(&self) -> u64 {
        let mut id = self.current_lock_id.lock().expect("lock_poison");
        *id += 1;
        *id
    }

    pub fn unlock_node(&self, node: NodeRef, loading_class: Option<&str>) {
        self.document
            .inner()
            .lock()
            .expect("lock poison")
            .remove_attributes_by(node, |attr| {
                attr.name.name != dom_locking::PHX_REF_LOCK
                    && attr.name.name != dom_locking::PHX_REF_SRC
            });

        if let Some(loading_class) = loading_class {
            self.document
                .inner()
                .lock()
                .expect("lock poison")
                .remove_classes_by(node, |class| class != loading_class);
        }
    }

    pub fn lock_node(&self, node: NodeRef, loading_class: Option<&str>) {
        let lock = Attribute::new(PHX_REF_LOCK, Some(self.next_id().to_string()));

        self.document
            .inner()
            .lock()
            .expect("lock poison")
            .add_attribute(node, lock);

        let el_lock = Attribute::new(PHX_REF_SRC, Some(node.0.to_string()));

        self.document
            .inner()
            .lock()
            .expect("lock poison")
            .add_attribute(node, el_lock);

        if let Some(attr) = loading_class {
            self.document
                .inner()
                .lock()
                .expect("lock poison")
                .extend_class_list(node, &[attr]);
        }
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

    pub fn get_phx_upload_id(&self, phx_target_name: &str) -> Result<String, LiveSocketError> {
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
                    AttributeValue::String(phx_target_name.into()),
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

        Ok(upload_id)
    }

    /// Blocks indefinitely, processing changes to the document using the user provided callback
    /// In `set_event_handler`
    pub async fn merge_diffs(&self) -> Result<(), LiveSocketError> {
        // TODO: This should probably take the event closure to send changes back to swift/kotlin
        let document = self.document.clone();
        let events = self.channel.events();
        let statuses = self.channel.statuses();
        loop {
            let event = events.event().fuse();
            let status = statuses.status().fuse();

            pin_mut!(event, status);

            select! {
               e = event => {
                   let e = e?;
                   match e.event {
                       Event::Phoenix { phoenix } => {
                           error!("Phoenix Event for {phoenix:?} is unimplemented");
                       }
                       Event::User { user } => {
                           if user == "diff" {
                               let Payload::JSONPayload { json } = e.payload else {
                                   error!("Diff was not json!");
                                   continue;
                               };

                               debug!("PAYLOAD: {json:?}");
                               // This function merges and uses the event handler set in `set_event_handler`
                               // which will call back into the Swift/Kotlin.
                               document.merge_fragment_json_unserialized(json)?;
                           }
                       }
                   };
               }
               s = status => {
                   match s? {
                    phoenix_channels_client::ChannelStatus::Left => return Ok(()),
                    phoenix_channels_client::ChannelStatus::ShutDown => return Ok(()),
                    _ => {},
                  }
               }
            };
        }
    }

    /// Rejoin the channel
    pub async fn rejoin(&self) -> Result<(), LiveSocketError> {
        self.channel().join(self.timeout).await?;
        Ok(())
    }

    pub fn join_payload(&self) -> Payload {
        self.join_payload.clone()
    }

    pub async fn upload_file(&self, file: &LiveFile) -> Result<(), LiveSocketError> {
        // this is not great but we have to mimic constructing
        // this ad hoc object to send to the server
        // https://github.com/phoenixframework/phoenix_live_view/blob/b59bede3fcec6995f1d5876a520af8badc4bb7fb/priv/static/phoenix_live_view.js#L1315
        let ref_id = self.document().next_upload_id();

        // Allow upload requests to upload.
        let upload_event: Event = Event::User {
            user: "allow_upload".to_string(),
        };

        // TODO: move this into protocol
        let event_string = format!(
            r#"{{
            "ref":"{}",
            "entries":[
                {{
                    "name":"{}",
                    "relative_path":"{}",
                    "size":{},
                    "type":"{}",
                    "ref":"{}"
                }}
            ]
            }}"#,
            file.phx_upload_id,
            file.name,
            file.relative_path,
            file.contents.len(),
            file.mime_type,
            ref_id
        );

        let event_payload: Payload = Payload::json_from_serialized(event_string)?;

        // TODO: Create a configurable, introspectable upload interface to control
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
                    if let Some(JSON::Str { string }) = object.get(&ref_id.to_string()) {
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
                Topic::from_string(format!("lvu:{}", ref_id)),
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
                    file.phx_upload_id, ref_id, progress,
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
            file.phx_upload_id, ref_id,
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
