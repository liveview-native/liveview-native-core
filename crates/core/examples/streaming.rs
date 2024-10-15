use std::time::Duration;

use liveview_native_core::live_socket::LiveSocket;

#[cfg(target_os = "android")]
const HOST: &str = "10.0.2.2:4001";

#[cfg(not(target_os = "android"))]
const HOST: &str = "127.0.0.1:4001";

const TIME_OUT: Duration = Duration::from_secs(2);

#[tokio::main]
async fn main() {
    let _ = env_logger::builder().parse_default_env().try_init();

    let url = format!("http://{HOST}/stream");

    let live_socket = LiveSocket::new(url.to_string(), TIME_OUT, "swiftui".into())
        .await
        .expect("Failed to get liveview socket");

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join the liveview channel");

    live_channel
        .merge_diffs()
        .await
        .expect("Failed to merge diffs");
}
