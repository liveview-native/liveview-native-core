use super::*;

#[tokio::test]
async fn streaming_connect() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/simple_stream?_format=swiftui");

    let live_socket = LiveSocket::new(url.to_string(), TIME_OUT)
        .await
        .expect("Failed to get liveview socket");
    let _live_channel = live_socket
        .join_liveview_channel()
        .await
        .expect("Failed to join the liveview channel");
}
