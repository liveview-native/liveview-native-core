use super::*;

#[tokio::test]
async fn join_live_view() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/doesnt-exist");
    let live_socket_err =
        LiveSocket::new(url.to_string(), "swiftui".into(), Default::default()).await;
    assert!(live_socket_err.is_err());
    let live_socket_err = live_socket_err.err().unwrap();
    assert!(
        matches!(live_socket_err, LiveSocketError::ConnectionError{ error_html: ref _error_html})
    );
    println!("ERROR HTML: {live_socket_err}");
}
