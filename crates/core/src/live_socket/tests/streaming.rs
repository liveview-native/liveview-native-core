use tokio::sync::mpsc::{
    channel,
    error::{TryRecvError, TrySendError},
    Sender,
};

use super::*;
use crate::dom::PatchInspector;

// As of this commit the server sends a
// stream even every 10_000 ms
// This sampling interval should catch one
const MAX_TRIES: u64 = 200;
const MS_DELAY: u64 = 100;

struct Inspector {
    tx: Sender<()>,
}

impl PatchInspector for Inspector {
    fn inspect(&self, _patches: &[crate::diff::Patch]) {
        while let Err(TrySendError::Full(_)) = self.tx.try_send(()) {}
    }
}

// Tests that streaming connects, and succeeds at parsing at least one delta.
#[tokio::test]
async fn streaming_connect() -> Result<(), String> {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/stream");

    let live_socket = LiveSocket::new(url.to_string(), TIME_OUT, "swiftui".into())
        .await
        .expect("Failed to get liveview socket");

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join the liveview channel");

    let (tx, mut rx) = channel(1);
    live_channel.set_patch_inspector(Box::new(Inspector { tx }));

    let _ = tokio::spawn(async move {
        live_channel
            .merge_diffs()
            .await
            .expect("Failed to merge diffs");
    });

    for _ in 0..MAX_TRIES {
        match rx.try_recv() {
            Ok(_) => {
                return Ok(());
            }
            Err(TryRecvError::Empty) => {
                tokio::time::sleep(Duration::from_millis(MS_DELAY)).await;
            }
            Err(_) => {
                return Err(format!("Merging Panicked"));
            }
        }
    }

    Err(format!(
        "Exceeded {MAX_TRIES} Max tries, waited {} ms",
        MAX_TRIES * MS_DELAY
    ))
}

const INITIAL_STATE: &str = "";

// asserts that streaming applies at least one diff
#[tokio::test]
async fn streaming_update() -> Result<(), String> {
    let _ = env_logger::builder()
        .parse_default_env()
        .is_test(true)
        .try_init();

    let url = format!("http://{HOST}/stream");

    let live_socket = LiveSocket::new(url.to_string(), TIME_OUT, "swiftui".into())
        .await
        .expect("Failed to get liveview socket");

    let live_channel = live_socket
        .join_liveview_channel(None, None)
        .await
        .expect("Failed to join the liveview channel");

    let (tx, mut rx) = channel(1);
    live_channel.set_patch_inspector(Box::new(Inspector { tx }));

    let _ = tokio::spawn(async move {
        live_channel
            .merge_diffs()
            .await
            .expect("Failed to merge diffs");
    });

    for _ in 0..MAX_TRIES {
        match rx.try_recv() {
            Ok(_) => {
                return Ok(());
            }
            Err(TryRecvError::Empty) => {
                tokio::time::sleep(Duration::from_millis(MS_DELAY)).await;
            }
            Err(_) => {
                return Err(format!("Merging Panicked"));
            }
        }
    }

    Err(format!(
        "Exceeded {MAX_TRIES} Max tries, waited {} ms",
        MAX_TRIES * MS_DELAY
    ))
}
