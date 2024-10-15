use std::sync::{Arc, Mutex};

use tokio::sync::oneshot::{self, *};

use super::*;
use crate::dom::{ChangeType, DocumentChangeHandler, NodeData, NodeRef};

// As of this commit the server sends a
// stream even every 10_000 ms
// This sampling interval should catch one
const MAX_TRIES: u64 = 120;
const MS_DELAY: u64 = 100;

struct Inspector {
    tx: std::sync::Mutex<Option<Sender<()>>>,
}

impl DocumentChangeHandler for Inspector {
    fn handle(
        &self,
        _change_type: ChangeType,
        _node_ref: Arc<NodeRef>,
        _node_data: NodeData,
        _parent: Option<Arc<NodeRef>>,
    ) {
        let tx = self
            .tx
            .lock()
            .expect("lock poison")
            .take()
            .expect("Channel was None.");

        tx.send(()).expect("Message Never Received.");
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

    let (tx, mut rx) = oneshot::channel();
    live_channel.set_event_handler(Box::new(Inspector {
        tx: Mutex::new(Some(tx)),
    }));

    let chan_clone = live_channel.channel().clone();
    tokio::spawn(async move {
        live_channel
            .merge_diffs()
            .await
            .expect("Failed to merge diffs");
    });

    for _ in 0..MAX_TRIES {
        match rx.try_recv() {
            Ok(_) => {
                chan_clone.leave().await.expect("could not leave channel");
                return Ok(());
            }
            Err(oneshot::error::TryRecvError::Empty) => {
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
