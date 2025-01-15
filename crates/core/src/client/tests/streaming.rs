use std::{sync::Arc, time::Duration};

use tokio::{
    sync::mpsc::{error, *},
    time,
};

use super::*;
use crate::{
    client::{ChangeType, DocumentChangeHandler},
    dom::{self, NodeData, NodeRef},
};

const MAX_TRIES: u64 = 10;
const MS_DELAY: u64 = 1500;

struct Inspector {
    tx: UnboundedSender<(ChangeType, NodeData)>,
}

impl Inspector {
    pub fn new() -> (Self, UnboundedReceiver<(ChangeType, NodeData)>) {
        let (tx, rx) = unbounded_channel();
        let out = Self { tx };
        (out, rx)
    }
}

/// An extremely simple change handler that reports diffs in order
/// over an unbounded channel
impl DocumentChangeHandler for Inspector {
    fn handle_document_change(
        &self,
        change_type: ChangeType,
        _node_ref: Arc<NodeRef>,
        node_data: NodeData,
        _parent: Option<Arc<NodeRef>>,
    ) {
        self.tx
            .send((change_type, node_data))
            .expect("Message Never Received.");
    }

    fn handle_new_document(&self, document: std::sync::Arc<dom::ffi::Document>) {
        let _ = document;
    }
}

// Tests that streaming connects, and succeeds at parsing at least one delta.
#[tokio::test]
async fn streaming_connect() -> Result<(), String> {
    let url = format!("http://{HOST}/stream");

    let (inspector, mut rx) = Inspector::new();

    let mut config = LiveViewClientConfiguration::default();
    config.patch_handler = Some(Arc::new(inspector));
    config.format = Platform::Swiftui;

    let _client = LiveViewClient::initial_connect(config, url, Default::default())
        .await
        .expect("Failed to create client");

    for _ in 0..MAX_TRIES {
        match rx.try_recv() {
            Ok(_) => {
                return Ok(());
            }
            Err(error::TryRecvError::Empty) => {
                time::sleep(Duration::from_millis(MS_DELAY)).await;
            }
            Err(_) => {
                return Err(String::from("Merging Panicked"));
            }
        }
    }

    Err(format!(
        "Exceeded {MAX_TRIES} Max tries, waited {} ms",
        MAX_TRIES * MS_DELAY
    ))
}
