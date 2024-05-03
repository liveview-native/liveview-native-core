use super::*;

#[tokio::test]
async fn streaming_connect() {
    let _ = env_logger::builder()
        .parse_default_env()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/simple_stream?_lvn[format]=swiftui");

    let live_socket = LiveSocket::new(
        url.to_string(), TIME_OUT
    ).expect("Failed to get liveview socket");
    let _live_channel = live_socket
        .join_liveview_channel()
        .await
        .expect("Failed to join the liveview channel");
    //live_channel.merge_diffs().await.expect("Failed to merge diffs");

}
