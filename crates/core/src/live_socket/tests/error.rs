use super::*;

#[tokio::test]
async fn dead_render_error() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/doesnt-exist");
    let live_socket_err =
        LiveSocket::new(url.to_string(), "swiftui".into(), Default::default()).await;
    assert!(live_socket_err.is_err());
    let live_socket_err = live_socket_err.err().unwrap();
    assert!(matches!(
        live_socket_err,
        LiveSocketError::ConnectionError(_)
    ));
    log::debug!("ERROR HTML: {live_socket_err}");
}
