//use wasm_bindgen_test::*;
//wasm_bindgen_test_configure!(run_in_browser);
use std::time::Duration;

/*
use liveview_native_core::{
    live_socket::LiveSocket,
    diff::fragment::{
        FragmentDiff,
        FragmentMerge,
        Root,
        RootDiff,
    },
};
*/
use phoenix_channels_client::Socket;
use url::Url;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

/*
#[wasm_bindgen_test]
async fn live_socket() {
    let socket = LiveSocket::connect("http://localhost:4001".into(), std::time::Duration::from_secs(10)).await.expect("Failed to connect to server");
}
*/

fn id() -> String {
    Uuid::new_v4()
        .hyphenated()
        .encode_upper(&mut Uuid::encode_buffer())
        .to_string()
}

fn shared_secret_url(id: String) -> Url {
    Url::parse_with_params(
        "ws://127.0.0.1:9002/socket/websocket".into(),
        &[("shared_secret", "supersecret".to_string()), ("id", id)],
    )
    .unwrap()
}
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
use log::{info, Level};
use tokio::runtime::Builder;

#[wasm_bindgen(main)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug);
    let rt = Builder::new_current_thread().enable_time().build()?;
    let id = id();
    let url = shared_secret_url(id);
    info!("URL: {url}");

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async move {
            let socket = Socket::spawn(url, None).expect("Failed to spawn socket");
            socket
                .connect(CONNECT_TIMEOUT)
                .await
                .expect("Failed to connect to server");
        })
        .await;
    Ok(())
}
