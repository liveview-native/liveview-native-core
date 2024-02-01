use std::{
    fmt,
    sync::{
        Arc,
        RwLock,
    }
};
pub use super::{
    attribute::Attribute,
    node::{Node, NodeRef},
    printer::PrintOptions,
    DocumentChangeHandler,
};
use crate::parser::ParseError;
use crate::diff::fragment::RenderError;


#[derive(uniffi::Object)]
pub struct Document {
    inner: Arc<RwLock<super::Document>>,
}

#[uniffi::export]
impl Document {
    #[uniffi::constructor]
    pub fn parse(
        input: String,
    ) -> Result<Arc<Self>, ParseError> {
        Ok(Arc::new(Self {
            inner: Arc::new(RwLock::new(super::Document::parse(input)?)),
        }))
    }

    #[uniffi::constructor]
    pub fn empty() -> Arc<Self> {
        Arc::new(Self {
            inner: Arc::new(RwLock::new(super::Document::empty())),
        })
    }

    #[uniffi::constructor]
    pub fn parse_fragment_json(
        input: String,
    ) -> Result<Arc<Self>, RenderError> {
        let inner = Arc::new(RwLock::new(super::Document::parse_fragment_json(input)?));
        Ok(Arc::new(Self {
            inner
        }))
    }

    pub fn merge_fragment_json(
        &self,
        json: String,
        handler: Arc<dyn DocumentChangeHandler>
    ) -> Result<(), RenderError> {
        if let Ok(mut inner) = self.inner.write() {
            Ok(inner.merge_fragment_json(json, handler)?)
        } else {
            unimplemented!("The error case for when we cannot get the lock for the Document has not been finished yet");
        }
    }

    pub fn root(&self) -> Arc<NodeRef> {
        self.inner.read().expect("Failed to get lock").root().into()
    }

    pub fn get_parent(&self, node_ref: Arc<NodeRef>) -> Option<Arc<NodeRef>> {
        self.inner.read().expect("Failed to get lock").parent(*node_ref).map(|node_ref| node_ref.into())
    }

    pub fn children(&self, node_ref: Arc<NodeRef>) -> Vec<Arc<NodeRef>> {
        self.inner.read().expect("Failed to get lock").children(*node_ref).iter().map(|node| Arc::new(*node)).collect()
    }

    pub fn get_attributes(&self, node_ref: Arc<NodeRef>) -> Vec<Attribute> {
        self.inner.read().expect("Failed to get lock").attributes(*node_ref).to_vec()
    }
    pub fn get(&self, node_ref: Arc<NodeRef>) -> Node {
        self.inner.read().expect("Failed to get lock").get(*node_ref).clone()
    }
    pub fn render(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Document {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Ok(inner) = self.inner.read() {
            inner.print(f, PrintOptions::Pretty)
        } else {
            todo!()
        }
    }
}

