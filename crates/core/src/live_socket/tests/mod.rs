use super::*;
mod streaming;
mod upload;

#[cfg(target_os = "android")]
const HOST: &str = "10.0.2.2";

#[cfg(not(target_os = "android"))]
const HOST: &str = "127.0.0.1";

const TIME_OUT : Duration = Duration::from_secs(2);

#[tokio::test]
async fn join_live_view() {
    let _ = env_logger::builder()
        .parse_default_env()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}:4001/upload?_lvn[format]=swiftui");
    let live_socket = LiveSocket::new(
        url.to_string(),
        TIME_OUT
    ).expect("Failed to get liveview socket");
    let live_channel = live_socket
        .join_liveview_channel()
        .await
        .expect("Failed to join channel");
    let _phx_input_id = live_channel.get_phx_ref_from_upload_join_payload();
}
