
use super::*;
fn get_phx_input_from_resp(join_response: Payload) -> String {
    let new_root = match join_response {
        Payload::JSONPayload {
            json: JSON::Object {
                ref object
            },
        } => {
            if let Some(rendered) = object.get("rendered") {
                let rendered = rendered.to_string();
                use crate::diff::fragment::{
                    Root,
                    RootDiff,
                };
                let root: RootDiff = serde_json::from_str(rendered.as_str()).expect("Failed to deserialize fragment");
                let root : Root = root.try_into().expect("Failed to convert rootdiff into root");
                let root : String = root.try_into().expect("Failed to convert root into string");
                let document = parse(&root).expect("Failed to parse document");
                Some(document)
            } else {
                None
            }
        },
        _ => {
            None
        }
    };
    let document = new_root.expect("Failed to get new document");

    let phx_input_id = document.select(Selector::Attribute(AttributeName {
        namespace: None,
        name: "data-phx-upload-ref".into(),
    },
    ))
        .last()
        .map(|node_ref| document.get(node_ref))
        .map(|input_div| {
            input_div
                .attributes()
                .iter()
                .filter(|attr| attr.name.name == "id")
                .map(|attr| attr.value.clone())
                .collect::<Option<String>>()
        }).flatten()
    .expect("Failed to get input_id");
    phx_input_id
}

// This is from
// https://github.com/image-rs/image/blob/4989d5f83a4a1aaaf7b1fd1f33f7b4db1d3404d3/examples/tile/main.rs
fn get_image(imgx: u32, imgy: u32, suffix: String) -> Vec<u8> {
    use image::RgbaImage;
    let mut img = RgbaImage::new(imgx, imgy);
    let tile = image::load_from_memory_with_format(
        include_bytes!("../../tests/support/tinycross.png"),
        image::ImageFormat::Png
    ).expect("Failed to load example image");

    use tempfile::tempdir;
    let tmp_dir = tempdir().expect("Failed to get tempdir");
    let file_path = tmp_dir.path().join(format!("image-{imgx}-{imgy}.{suffix}"));

    image::imageops::tile(&mut img, &tile);
    img.save(file_path.clone()).unwrap();

    // The format is deduced from the path.
    std::fs::read(file_path).expect("Failed to get image")
}

#[cfg(target_os = "android")]
const HOST: &str = "10.0.2.2";

#[cfg(not(target_os = "android"))]
const HOST: &str = "127.0.0.1";

#[tokio::test]
async fn join_live_view() {
    let _ = env_logger::builder()
        .parse_default_env()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();
    let timeout = std::time::Duration::from_secs(2);;

    let url = format!("http://{HOST}:4000/upload?_lvn[format]=swiftui");
    let timeout = Duration::from_secs(2);
    let live_socket = LiveSocket::new(url.to_string(), timeout).expect("Failed to get liveview socket");
    let live_channel = live_socket.join_liveview_channel().await.expect("Failed to join channel");
    let join_response = live_channel.join_payload;
    let _phx_input_id = get_phx_input_from_resp(join_response);
}

#[tokio::test]
async fn single_chunk_file() {
    let _ = env_logger::builder()
        .parse_default_env()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}:4000/upload?_lvn[format]=swiftui");
    let image_bytes = get_image(100, 100, "png".to_string());
    let timeout = Duration::from_secs(2);
    let live_socket = LiveSocket::new(url.to_string(), timeout).expect("Failed to get liveview socket");
    let live_channel = live_socket.join_liveview_channel().await.expect("Failed to join the liveview channel");
    let join_response = live_channel.join_payload.clone();
    let phx_input_id = get_phx_input_from_resp(join_response);


    let gh_favicon = LiveFile {
        contents: image_bytes.clone(),
        phx_id: phx_input_id.clone(),
        file_type: "png".to_string(),
        name: "tile.png".to_string(),
    };
    let _ = live_channel.validate_upload(&gh_favicon).await.expect("Failed to validate upload");
    live_channel.upload_file(&gh_favicon).await.expect("Failed to upload");
}

#[tokio::test]
async fn multi_chunk_file() {
    let _ = env_logger::builder()
        .parse_default_env()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}:4000/upload?_lvn[format]=swiftui");
    let image_bytes = get_image(2000, 2000, "png".to_string());

    let timeout = Duration::from_secs(2);
    let live_socket = LiveSocket::new(url.to_string(), timeout).expect("Failed to get liveview socket");
    let live_channel = live_socket.join_liveview_channel().await.expect("Failed to join the liveview channel");
    let join_response = live_channel.join_payload.clone();
    let phx_input_id = get_phx_input_from_resp(join_response);

    let me = LiveFile {
        contents: image_bytes.clone(),
        phx_id: phx_input_id,
        file_type: "png".to_string(),
        name: "tile.png".to_string(),
    };
    let _ = live_channel.validate_upload(&me).await.expect("Failed to validate upload");
    live_channel.upload_file(&me).await.expect("Failed to upload");
}

#[tokio::test]
async fn error_file_too_large() {
    let _ = env_logger::builder()
        .parse_default_env()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}:4000/upload?_lvn[format]=swiftui");

    // For this file we want to use tiff because it's much biggger than a png.
    let image_bytes = get_image(2000, 2000, "tiff".to_string());

    let timeout = Duration::from_secs(2);
    let live_socket = LiveSocket::new(url.to_string(), timeout).expect("Failed to get liveview socket");
    let live_channel = live_socket.join_liveview_channel().await.expect("Failed to join the liveview channel");
    let join_response = live_channel.join_payload.clone();
    let phx_input_id = get_phx_input_from_resp(join_response);

    let me = LiveFile {
        contents: image_bytes.clone(),
        phx_id: phx_input_id,
        file_type: "png".to_string(),
        name: "tile.png".to_string(),
    };
    let _ = live_channel.validate_upload(&me).await.expect("Failed to validate upload");
    let out = live_channel.upload_file(&me).await.expect_err("This file is too big and should have failed");

    // This hack is required because LiveSocketError doesn't derive from PartialEq
    if let LiveSocketError::Upload{error: UploadError::FileTooLarge} = out {
    } else {
        panic!("This should be a FileTooLarge Error");
    }
}

#[tokio::test]
async fn error_incorrect_file_type() {
    let _ = env_logger::builder()
        .parse_default_env()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}:4000/upload?_lvn[format]=swiftui");

    // For this file we want to use tiff because it's much biggger than a png.
    let image_bytes = get_image(200, 200, "tiff".to_string());

    let timeout = Duration::from_secs(2);
    let live_socket = LiveSocket::new(url.to_string(), timeout).expect("Failed to get liveview socket");
    let live_channel = live_socket.join_liveview_channel().await.expect("Failed to join the liveview channel");
    let join_response = live_channel.join_payload.clone();
    let phx_input_id = get_phx_input_from_resp(join_response);

    assert_eq!(live_channel.channel.status(), ChannelStatus::Joined);
    let me = LiveFile {
        contents: image_bytes.clone(),
        phx_id: phx_input_id,
        file_type: "tiff".to_string(),
        name: "tile.tiff".to_string(),
    };
    let _ = live_channel.validate_upload(&me).await.expect("Failed to validate upload");
    let out = live_channel.upload_file(&me).await.expect_err("This should b ean incorrect file error");
    // This hack is required because LiveSocketError doesn't derive from PartialEq
    if let LiveSocketError::Upload{error: UploadError::FileNotAccepted} = out {
    } else {
        panic!("This should be a FileNotAccepted Error");
    }
}

