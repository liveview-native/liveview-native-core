// supporting code for generating fixtures and recorded scenarios.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use log::debug;
use phoenix_channels_client::{Event, Payload};
use pretty_assertions::private::CreateComparison;

struct EventLogger<T: std::fmt::Display + Send + Sync> {
    // dom::Document type is not public so we have to smuggle it in a generic
    document: T,
    log_patch_events: Arc<Mutex<bool>>,
}

impl<T: std::fmt::Display + Send + Sync> DocumentChangeHandler for EventLogger<T> {
    fn handle(
        &self,
        change_type: ChangeType,
        node_ref: std::sync::Arc<NodeRef>,
        node_data: NodeData,
        _parent: Option<std::sync::Arc<NodeRef>>,
    ) {
        if *self.log_patch_events.lock().unwrap() {
            log::info!("applied patch {change_type:#?} - {node_ref} - {node_data:#?}");
            log::info!("\n\n{}\n\n", self.document);
        }
    }
}

use super::TIME_OUT;
use crate::{
    dom::{ChangeType, Document, DocumentChangeHandler, NodeData, NodeRef},
    live_socket::{LiveChannel, LiveSocket},
};

macro_rules! json_payload {
    ($json:tt) => {{
        let val = serde_json::json!($json);
        phoenix_channels_client::Payload::JSONPayload { json: val.into() }
    }};
}
pub(super) use json_payload;

/// plays back and applies a series of messages and signals with the test
/// server. confirms that the change set between each message is what is expected.
/// All fixtures are laid out in /tests/fixtures/fixture_name.fixture
/// with expected_0.xml being the deadrender
pub struct FixturePlayback {
    log_patch_events: Arc<Mutex<bool>>,
    // if recording, write the transactions to a *.fixture directory
    recording_mode: bool,
    // path to the fixture directory containing the change set recording
    fixture_path: PathBuf,
    // the socket that does the communication
    chan: LiveChannel,
    // current step in the playback/recording process
    transaction_ct: u32,
}

fn pretty_diff(r: &str, l: &str) -> Result<(), String> {
    if r == l {
        Ok(())
    } else {
        log::error!(
            "assertion failed: `(left == right)`\
            \n\
            \n{}\
            \n",
            (r, l).create_comparison()
        );

        Err("Comparing with golden document failed".to_owned())
    }
}

fn pretty_print(payload: &Payload) -> String {
    // these should never panic
    let parsed: serde_json::Value =
        serde_json::from_str(&payload.to_string()).expect("Invalid Payload");
    serde_json::to_string_pretty(&parsed).expect("Invalid value after resiarilizing")
}

impl FixturePlayback {
    #[allow(dead_code)]
    pub async fn record(fixture_path: &str, format: &str, url_ext: &str) -> Self {
        Self::new(fixture_path, format, url_ext, true).await
    }

    #[allow(dead_code)]
    /// start fine grained logging of patch application
    pub fn start_logging_patch_events(&mut self) {
        *self.log_patch_events.lock().unwrap() = true;
    }

    #[allow(dead_code)]
    /// end fine grained logging of patch application
    pub fn stop_logging_patch_events(&mut self) {
        *self.log_patch_events.lock().unwrap() = false;
    }

    /// args: fixture file directory, format (swiftui | jetpack | html), test server url
    /// set the constructor to [FixturePlayback::record] or [FixturePlayback::playback] depending on what stage of testing you are in
    /// PROTIP: set the environment variable RECORD_ALL_FIXTURES="true" to set every fixture to record mode.
    pub async fn playback(fixture_path: &str, format: &str, url_ext: &str) -> Self {
        let record_mode = false || option_env!("RECORD_ALL_FIXTURES") == Some("true");
        Self::new(fixture_path, format, url_ext, record_mode).await
    }

    pub async fn new(
        fixture_path: &str,
        format: &str,
        url_ext: &str,
        recording_mode: bool,
    ) -> Self {
        let _ = env_logger::builder()
            .parse_default_env()
            .is_test(true)
            .try_init();

        #[cfg(target_os = "android")]
        const HOST: &str = "10.0.2.2:4001";
        #[cfg(not(target_os = "android"))]
        const HOST: &str = "127.0.0.1:4001";

        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixture_path = root.join("src/live_socket/tests/").join(fixture_path);

        if recording_mode {
            std::fs::create_dir_all(&fixture_path).expect("could not create fixture dir");
        } else {
            assert!(fixture_path.exists(), "fixture path not created yet");
        }

        let url = format!("http://{HOST}/{}", url_ext);

        let live_socket = LiveSocket::new(url.to_string(), TIME_OUT, format.to_owned())
            .await
            .expect("Failed to get liveview socket");

        let chan = live_socket
            .join_liveview_channel(None, None)
            .await
            .expect("Could not connect to channel");

        let fine_grained_log = Arc::new(Mutex::new(false));
        chan.document().set_event_handler(Box::new(EventLogger {
            document: chan.document().clone(),
            log_patch_events: fine_grained_log.clone(),
        }));

        let out = Self {
            recording_mode,
            fixture_path,
            chan,
            transaction_ct: 0,
            log_patch_events: fine_grained_log,
        };

        out
    }

    pub fn validate_json_payload(&self, pay_load: &Payload) -> Result<(), String> {
        let path = self
            .fixture_path
            .join(format!("return_payload_{}.json", self.transaction_ct));

        let rendered = pretty_print(pay_load);

        if self.recording_mode {
            std::fs::write(path, rendered)
                .map_err(|e| format!("Writing record json failed: {e:?}"))?;
        } else {
            let golden_json =
                std::fs::read_to_string(path).map_err(|e| format!("reading demo failed: {e:?}"))?;

            pretty_diff(&golden_json, &rendered)
                .map_err(|e| format!("{e} : return_payload_{}.json", self.transaction_ct))?;
        }
        Ok(())
    }

    pub fn validate_document(&self) -> Result<(), String> {
        let path = self
            .fixture_path
            .join(format!("expected_{}.xml", self.transaction_ct));

        let doc = self.chan.document();

        let rendered = doc.render();

        log::trace!(
            "rendering doc after transaction #{} \n {rendered} \n",
            self.transaction_ct
        );

        if self.recording_mode {
            std::fs::write(path, &rendered)
                .map_err(|e| format!("Writing document failed: {e:?}"))?;
        } else {
            let golden_doc =
                Document::parse_file(&path).map_err(|e| format!("Error Parsing doc: {e}"))?;

            pretty_diff(&golden_doc.to_string(), &rendered)
                .map_err(|e| format!("{e} : expected_{}.xml", self.transaction_ct))?;
        }

        Ok(())
    }

    pub async fn send_message(&mut self, event: Event, payload: Payload) -> Result<(), String> {
        let msg = if self.recording_mode {
            "RECORDING SESSION"
        } else {
            "SESSION PLAYBACK"
        };

        debug!(
            "{msg} : sending message #{}\n Event: {event:?}\n {}",
            self.transaction_ct + 1,
            pretty_print(&payload)
        );

        let return_payload = self
            .chan
            .channel()
            .call(event, payload, TIME_OUT)
            .await
            .map_err(|e| format!("Call Error: {e:?}"))?;

        let Payload::JSONPayload { json } = return_payload.clone() else {
            return Err(String::from(
                "Binary return not yet supported in this test fixture",
            ));
        };

        let val = serde_json::Value::from(json.clone());

        let obj = val
            .as_object()
            .ok_or(String::from("Return was not and object"))?;

        if *self.log_patch_events.lock().unwrap() {
            log::info!("Document Prior to changes:\n\n{}\n\n", self.chan.document());
        }

        if let Some(diff) = &obj.get("diff") {
            self.chan
                .document()
                .merge_fragment_json(&diff.to_string())
                .unwrap();
        }

        self.transaction_ct += 1;
        self.validate_json_payload(&return_payload)?;

        Ok(())
    }
}
