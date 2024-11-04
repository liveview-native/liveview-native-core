use super::*;

// This is from
// https://github.com/image-rs/image/blob/4989d5f83a4a1aaaf7b1fd1f33f7b4db1d3404d3/examples/tile/main.rs
fn get_image(imgx: u32, imgy: u32, suffix: String) -> Vec<u8> {
    use image::RgbaImage;
    let mut img = RgbaImage::new(imgx, imgy);
    let tile = image::load_from_memory_with_format(
        include_bytes!("../../../tests/support/tinycross.png"),
        image::ImageFormat::Png,
    )
    .expect("Failed to load example image");

    use tempfile::tempdir;
    let tmp_dir = tempdir().expect("Failed to get tempdir");
    let file_path = tmp_dir.path().join(format!("image-{imgx}-{imgy}.{suffix}"));

    image::imageops::tile(&mut img, &tile);
    img.save(file_path.clone()).unwrap();

    // The format is deduced from the path.
    std::fs::read(file_path).expect("Failed to get image")
}

#[tokio::test]
async fn single_chunk_file() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/upload");
    let image_bytes = get_image(100, 100, "png".to_string());

    let live_socket = LiveSocket::new(url.to_string(), "swiftui".into(), Default::default())
        .await
        .expect("Failed to get liveview socket");

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join the liveview channel");

    let gh_favicon = live_channel
        .construct_upload(
            image_bytes.clone(),
            "image/png".to_string(),
            "tile.png".to_string(),
            "avatar".to_string(),
        )
        .expect("Could not construct file.");

    live_channel
        .upload_file(&gh_favicon)
        .await
        .expect("Failed to upload");
}

#[tokio::test]
async fn multi_chunk_text() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/upload");
    let text_bytes = Vec::from_iter(std::iter::repeat_n(b'a', 48_000));

    let live_socket = LiveSocket::new(url.to_string(), "swiftui".into(), Default::default())
        .await
        .expect("Failed to get liveview socket");

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join the liveview channel");

    let me = live_channel
        .construct_upload(
            text_bytes.clone(),
            "text/plain".to_string(),
            "lots_of_as.txt".to_string(),
            "sample_text".to_string(),
        )
        .expect("Could not construct file.");

    live_channel
        .upload_file(&me)
        .await
        .expect("Failed to upload");
}

#[tokio::test]
async fn multi_chunk_file() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/upload");
    let image_bytes = get_image(2000, 2000, "png".to_string());

    let live_socket = LiveSocket::new(url.to_string(), "swiftui".into(), Default::default())
        .await
        .expect("Failed to get liveview socket");

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join the liveview channel");

    let me = live_channel
        .construct_upload(
            image_bytes.clone(),
            "image/png".to_string(),
            "tile.png".to_string(),
            "avatar".to_string(),
        )
        .expect("Could not construct file.");

    live_channel
        .upload_file(&me)
        .await
        .expect("Failed to upload");
}

#[tokio::test]
async fn error_file_too_large() {
    let _ = env_logger::builder()
        .parse_default_env()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/upload");

    // For this file we want to use tiff because it's much biggger than a png.
    let image_bytes = get_image(2000, 2000, "tiff".to_string());

    let live_socket = LiveSocket::new(url.to_string(), "swiftui".into(), Default::default())
        .await
        .expect("Failed to get liveview socket");

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join the liveview channel");

    let me = live_channel
        .construct_upload(
            image_bytes.clone(),
            "image/png".to_string(),
            "tile.png".to_string(),
            "avatar".to_string(),
        )
        .expect("Could not construct file.");

    let out = live_channel
        .upload_file(&me)
        .await
        .expect_err("This file is too big and should have failed");

    match out {
        LiveSocketError::Upload {
            error: UploadError::FileTooLarge,
        } => {}
        e => {
            panic!("This should be a FileTooLarge, Error instead was: {e:?}");
        }
    }
}

#[tokio::test]
async fn error_incorrect_file_type() {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/upload");

    // For this file we want to use tiff because it's much biggger than a png.
    let image_bytes = get_image(100, 100, "png".to_string());

    let live_socket = LiveSocket::new(url.to_string(), "swiftui".into(), Default::default())
        .await
        .expect("Failed to get liveview socket");

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join the liveview channel");

    let me = live_channel
        .construct_upload(
            image_bytes.clone(),
            "image/tiff".to_string(),
            "tile.tiff".to_string(),
            "avatar".to_string(),
        )
        .expect("Could not construct file.");

    let out = live_channel
        .upload_file(&me)
        .await
        .expect_err("This should be an incorrect file error");
    // This hack is required because LiveSocketError doesn't derive from PartialEq
    if let LiveSocketError::Upload {
        error: UploadError::FileNotAccepted,
    } = out
    {
    } else {
        panic!("This should be a FileNotAccepted Error");
    }
}
