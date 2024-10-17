// supporting code for generating fixtures and recorded scenarios.
use std::path::PathBuf;

use log::debug;
use phoenix_channels_client::{Event, Payload};

use super::TIME_OUT;

macro_rules! json_payload {
    ($json:tt) => {{
        let val = serde_json::json!($json);
        phoenix_channels_client::Payload::JSONPayload { json: val.into() }
    }};
}
pub(super) use json_payload;

// shared set up logic for playback tests
macro_rules! shared_setup {
    ($fixture_dir:expr, $format:expr, $url:expr, $recording:expr) => {{
        use std::path::PathBuf;

        use tests::support::FixturePlayback;

        #[cfg(target_os = "android")]
        const HOST: &str = "10.0.2.2:4001";
        #[cfg(not(target_os = "android"))]
        const HOST: &str = "127.0.0.1:4001";

        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixture_path = root.join(format!("src/live_socket/tests/{}", $fixture_dir));

        if $recording {
            std::fs::create_dir_all(&fixture_path).expect("could not create fixture dir");
        } else {
            assert!(fixture_path.exists(), "fixture path not created yet");
        }

        let url = format!("http://{HOST}/{}", $url);

        let live_socket = LiveSocket::new(url.to_string(), TIME_OUT, $format.into())
            .await
            .expect("Failed to get liveview socket");

        let channel = live_socket.join_liveview_channel(None, None).await.unwrap();

        let out = FixturePlayback::new(fixture_path, channel, $recording);

        out.set_or_check_next_document()
            .expect("first transaction failed");

        out
    }};
}

pub(crate) use shared_setup;

#[allow(unused_macros)]
/// The same as playback, except sets this fixture test to recording mode
/// you can run it individually to generate a new set of blessed fixtures.
macro_rules! record {
    ($fixture_dir:expr, $format:expr, $url:expr) => {{
        tests::support::shared_setup!($fixture_dir, $format, $url, true)
    }};
}

/// args: fixture file directory, format (swiftui | jetpack | html), test server url
/// set the macro to record! or playback! depending on what stage of testing you are in
/// PROTIP: set the environment variable RECORD_ALL_FIXTURES="true" to set every fixture to record mode.
macro_rules! playback {
    ($fixture_dir:expr, $format:expr, $url:expr) => {{
        let testing_mode = false || option_env!("RECORD_ALL_FIXTURES") == Some("true");
        tests::support::shared_setup!($fixture_dir, $format, $url, testing_mode)
    }};
}

pub(crate) use playback;
#[allow(unused_imports)]
pub(crate) use record;

use super::LiveChannel;

/// plays back and applies a series of messages and signals with the test
/// server. confirms that the change set between each message is what is expected.
/// All fixtures are laid out in /tests/fixtures/fixture_name.fixture
/// with expected_0.xml being the deadrender
pub struct FixturePlayback {
    // if recording, write the transactions to a *.fixture directory
    recording_mode: bool,
    // path to the fixture directory containing the change set recording
    fixture_path: PathBuf,
    // the socket that does the communication
    chan: LiveChannel,
    transaction_ct: u32,
}

fn pretty_print(payload: &Payload) -> String {
    // these should never panic
    let parsed: serde_json::Value =
        serde_json::from_str(&payload.to_string()).expect("Invalid Payload");
    serde_json::to_string_pretty(&parsed).expect("Invalid value after resiarilizing")
}

impl FixturePlayback {
    pub fn new(fixture_path: PathBuf, chan: LiveChannel, recording_mode: bool) -> Self {
        Self {
            recording_mode,
            fixture_path,
            chan,
            transaction_ct: 0,
        }
    }

    pub fn set_or_check_next_return_payload(&self, pay_load: &Payload) -> Result<(), String> {
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
            pretty_assertions::assert_eq!(golden_json, rendered);
        }
        Ok(())
    }

    pub fn set_or_check_next_document(&self) -> Result<String, String> {
        let path = self
            .fixture_path
            .join(format!("expected_{}.xml", self.transaction_ct));

        let doc = self.chan.document();

        let rendered = doc.render();

        if self.recording_mode {
            std::fs::write(path, &rendered)
                .map_err(|e| format!("Writing document failed: {e:?}"))?;

            Ok(rendered)
        } else {
            let golden_xml = std::fs::read_to_string(path).map_err(|e| format!("{e:?}"))?;
            pretty_assertions::assert_eq!(golden_xml, rendered);
            Ok(rendered)
        }
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

        self.chan
            .document()
            .merge_fragment_json(&obj["diff"].to_string())
            .unwrap();

        self.transaction_ct += 1;
        let rendered = self.set_or_check_next_document()?;
        self.set_or_check_next_return_payload(&return_payload)?;

        log::trace!(
            "{msg} : rendering doc after transaction #{} \n {rendered} \n",
            self.transaction_ct
        );

        Ok(())
    }
}
