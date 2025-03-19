use image::RgbaImage;
use tempfile::tempdir;

use super::HOST;
use crate::{
    client::{LiveViewClientConfiguration, Platform},
    error::{LiveSocketError, UploadError},
    expect_status_matches,
    live_socket::LiveFile,
    LiveViewClient,
};

fn get_image(imgx: u32, imgy: u32, suffix: String) -> Vec<u8> {
    let mut img = RgbaImage::new(imgx, imgy);
    let tile = image::load_from_memory_with_format(
        include_bytes!("../../../tests/support/tinycross.png"),
        image::ImageFormat::Png,
    )
    .expect("Failed to load example image");

    let tmp_dir = tempdir().expect("Failed to get tempdir");
    let file_path = tmp_dir.path().join(format!("image-{imgx}-{imgy}.{suffix}"));

    image::imageops::tile(&mut img, &tile);
    img.save(file_path.clone()).unwrap();

    std::fs::read(file_path).expect("Failed to get image")
}

#[tokio::test]
async fn test_single_chunk_file_upload() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/upload");
    let image_bytes = get_image(100, 100, "png".to_string());

    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;

    let client = LiveViewClient::new(config, url, Default::default())
        .await
        .expect("Failed to create client");

    let mut watcher = client.watch_status();
    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    let phx_upload_id = client
        .get_phx_upload_id("avatar")
        .expect("No ID for avatar");

    let file = LiveFile::new(
        image_bytes,
        "image/png".to_string(),
        "avatar".to_string(),
        "tile.png".to_string(),
        phx_upload_id,
    );

    client
        .upload_file(file.into())
        .await
        .expect("Failed to upload file");
}

#[tokio::test]
async fn test_multi_chunk_text_upload() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/upload");
    let text_bytes = Vec::from_iter(std::iter::repeat_n(b'a', 48_000));

    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;

    let client = LiveViewClient::new(config, url, Default::default())
        .await
        .expect("Failed to create client");

    let mut watcher = client.watch_status();
    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    let phx_upload_id = client
        .get_phx_upload_id("sample_text")
        .expect("No ID for sample_text");

    let file = LiveFile::new(
        text_bytes,
        "text/plain".to_string(),
        "sample_text".to_string(),
        "lots_of_as.txt".to_string(),
        phx_upload_id,
    );

    client
        .upload_file(file.into())
        .await
        .expect("Failed to upload file");
}

#[tokio::test]
async fn test_multi_chunk_file_upload() {
    let url = format!("http://{HOST}/upload");
    let image_bytes = get_image(2000, 2000, "png".to_string());

    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;

    let client = LiveViewClient::new(config, url, Default::default())
        .await
        .expect("Failed to create client");

    let mut watcher = client.watch_status();
    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    let phx_upload_id = client
        .get_phx_upload_id("avatar")
        .expect("No ID for avatar");

    let file = LiveFile::new(
        image_bytes,
        "image/png".to_string(),
        "avatar".to_string(),
        "tile.png".to_string(),
        phx_upload_id,
    );

    client
        .upload_file(file.into())
        .await
        .expect("Failed to upload file");
}

#[tokio::test]
async fn test_file_too_large_error() {
    let _ = env_logger::builder()
        .parse_default_env()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/upload");
    let image_bytes = get_image(2000, 2000, "tiff".to_string());

    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;

    let client = LiveViewClient::new(config, url, Default::default())
        .await
        .expect("Failed to create client");

    let mut watcher = client.watch_status();
    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    let phx_upload_id = client
        .get_phx_upload_id("avatar")
        .expect("No ID for avatar");

    let file = LiveFile::new(
        image_bytes,
        "image/png".to_string(),
        "avatar".to_string(),
        "tile.png".to_string(),
        phx_upload_id,
    );

    let error = client.upload_file(file.into()).await.err().unwrap();

    match error {
        LiveSocketError::Upload {
            error: UploadError::FileTooLarge,
        } => {}
        e => panic!("Expected FileTooLarge error, got: {e:?}"),
    }
}

#[tokio::test]
async fn test_incorrect_file_type_error() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/upload");
    let image_bytes = get_image(100, 100, "png".to_string());

    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;

    let client = LiveViewClient::new(config, url, Default::default())
        .await
        .expect("Failed to create client");

    let mut watcher = client.watch_status();
    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    let phx_upload_id = client
        .get_phx_upload_id("avatar")
        .expect("No ID for avatar");

    let file = LiveFile::new(
        image_bytes,
        "avatar".to_string(),
        "image/png".to_string(),
        "tile.png".to_string(),
        phx_upload_id,
    );

    let error = client.upload_file(file.into()).await.err().unwrap();

    match error {
        LiveSocketError::Upload {
            error: UploadError::FileNotAccepted,
        } => {}
        e => panic!("Expected FileNotAccepted error, got: {e:?}"),
    }
}
