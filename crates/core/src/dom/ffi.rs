use std::{
    fmt,
    sync::{Arc, Mutex},
};

pub use super::{
    attribute::Attribute,
    node::{Node, NodeData, NodeRef},
    printer::PrintOptions,
};
use crate::{
    callbacks::*,
    diff::{fragment::RenderError, PatchResult},
    parser::ParseError,
};

#[derive(Clone, uniffi::Object)]
pub struct Document {
    inner: Arc<Mutex<super::Document>>,
}

impl From<super::Document> for Document {
    fn from(doc: super::Document) -> Self {
        Self {
            inner: Arc::new(Mutex::new(doc)),
        }
    }
}

// crate local api
impl Document {
    #[cfg(feature = "liveview-channels")]
    pub(crate) fn inner(&self) -> Arc<Mutex<super::Document>> {
        self.inner.clone()
    }

    pub fn arc_set_event_handler(&self, handler: Arc<dyn DocumentChangeHandler>) {
        self.inner.lock().expect("lock poisoned!").event_callback = Some(handler);
    }
}

#[uniffi::export]
impl Document {
    #[uniffi::constructor]
    pub fn parse(input: String) -> Result<Arc<Self>, ParseError> {
        Ok(Arc::new(Self {
            inner: Arc::new(Mutex::new(super::Document::parse(input)?)),
        }))
    }

    #[uniffi::constructor]
    pub fn empty() -> Arc<Self> {
        Arc::new(Self {
            inner: Arc::new(Mutex::new(super::Document::empty())),
        })
    }

    #[uniffi::constructor]
    pub fn parse_fragment_json(input: String) -> Result<Arc<Self>, RenderError> {
        let inner = Arc::new(Mutex::new(super::Document::parse_fragment_json(input)?));
        Ok(Arc::new(Self { inner }))
    }

    pub fn set_event_handler(&self, handler: Box<dyn DocumentChangeHandler>) {
        self.inner.lock().expect("lock poisoned!").event_callback = Some(Arc::from(handler));
    }

    pub fn merge_fragment_json(&self, json: &str) -> Result<(), RenderError> {
        let json = serde_json::from_str(json)?;

        let results = self
            .inner
            .lock()
            .expect("lock poisoned!")
            .merge_fragment_json(json)?;

        let Some(handler) = self
            .inner
            .lock()
            .expect("lock poisoned")
            .event_callback
            .clone()
        else {
            return Ok(());
        };

        for patch in results.into_iter() {
            match patch {
                PatchResult::Add { node, parent, data } => {
                    handler.handle_document_change(
                        ChangeType::Add,
                        node.into(),
                        data,
                        Some(parent.into()),
                    );
                }
                PatchResult::Remove { node, parent, data } => {
                    handler.handle_document_change(
                        ChangeType::Remove,
                        node.into(),
                        data,
                        Some(parent.into()),
                    );
                }
                PatchResult::Change { node, data } => {
                    handler.handle_document_change(ChangeType::Change, node.into(), data, None);
                }
                PatchResult::Replace { node, parent, data } => {
                    handler.handle_document_change(
                        ChangeType::Replace,
                        node.into(),
                        data,
                        Some(parent.into()),
                    );
                }
            }
        }

        Ok(())
    }

    pub fn next_upload_id(&self) -> u64 {
        self.inner.lock().expect("lock poisoned!").next_upload_id()
    }

    pub fn root(&self) -> Arc<NodeRef> {
        self.inner.lock().expect("lock poisoned!").root().into()
    }

    pub fn get_parent(&self, node_ref: Arc<NodeRef>) -> Option<Arc<NodeRef>> {
        self.inner
            .lock()
            .expect("lock poisoned!")
            .parent(*node_ref)
            .map(|node_ref| node_ref.into())
    }

    pub fn children(&self, node_ref: Arc<NodeRef>) -> Vec<Arc<NodeRef>> {
        self.inner
            .lock()
            .expect("lock poisoned!")
            .children(*node_ref)
            .iter()
            .map(|node| Arc::new(*node))
            .collect()
    }

    pub fn get_attributes(&self, node_ref: Arc<NodeRef>) -> Vec<Attribute> {
        self.inner
            .lock()
            .expect("lock poisoned!")
            .attributes(*node_ref)
            .to_vec()
    }

    pub fn get(&self, node_ref: Arc<NodeRef>) -> NodeData {
        self.inner
            .lock()
            .expect("lock poisoned!")
            .get(*node_ref)
            .clone()
    }

    pub fn get_node(&self, node_ref: Arc<NodeRef>) -> Node {
        let data = self.get(node_ref.clone());
        Node::new(self, &node_ref.clone(), data)
    }

    pub fn render(&self) -> String {
        self.to_string()
    }
}
impl Document {
    pub fn print_node(
        &self,
        node: NodeRef,
        writer: &mut dyn std::fmt::Write,
        options: PrintOptions,
    ) -> fmt::Result {
        self.inner
            .lock()
            .map_err(|_| fmt::Error)?
            .print_node(node, writer, options)
    }
}

impl fmt::Display for Document {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner
            .lock()
            .map_err(|_| fmt::Error)?
            .print(f, PrintOptions::Pretty)
    }
}
