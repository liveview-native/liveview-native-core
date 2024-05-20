use std::{
    fmt,
    sync::{
        Arc,
    },
    cell::SyncUnsafeCell,
};
pub use super::{
    attribute::Attribute,
    node::{NodeData, NodeRef, Node},
    printer::PrintOptions,
    DocumentChangeHandler,
};
use crate::parser::ParseError;
use crate::diff::fragment::RenderError;


#[derive(Clone, uniffi::Object)]
pub struct Document {
    inner: Arc<SyncUnsafeCell<super::Document>>,
}

impl From<super::Document> for Document {
    fn from(doc: super::Document) -> Self {
        Self {
            inner: Arc::new(SyncUnsafeCell::new(doc))
        }
    }
}

#[uniffi::export]
impl Document {
    #[uniffi::constructor]
    pub fn parse(
        input: String,
    ) -> Result<Arc<Self>, ParseError> {
        Ok(Arc::new(Self {
            inner: Arc::new(SyncUnsafeCell::new(super::Document::parse(input)?)),
        }))
    }

    #[uniffi::constructor]
    pub fn empty() -> Arc<Self> {
        Arc::new(Self {
            inner: Arc::new(SyncUnsafeCell::new(super::Document::empty())),
        })
    }

    #[uniffi::constructor]
    pub fn parse_fragment_json(
        input: String,
    ) -> Result<Arc<Self>, RenderError> {
        let inner = Arc::new(SyncUnsafeCell::new(super::Document::parse_fragment_json(input)?));
        Ok(Arc::new(Self {
            inner
        }))
    }
    pub fn set_event_handler(
        &self,
        handler: Box<dyn DocumentChangeHandler>
    ) {
        self.inner_mut().event_callback = Some(Arc::from(handler));
    }

    pub fn merge_fragment_json(
        &self,
        json: String,
    ) -> Result<(), RenderError> {
        self.inner_mut().merge_fragment_json(json)
    }

    pub fn root(&self) -> Arc<NodeRef> {
        self.inner().root().into()
    }

    pub fn get_parent(&self, node_ref: Arc<NodeRef>) -> Option<Arc<NodeRef>> {
        self.inner().parent(*node_ref).map(|node_ref| node_ref.into())
    }

    pub fn children(&self, node_ref: Arc<NodeRef>) -> Vec<Arc<NodeRef>> {
        self.inner().children(*node_ref).iter().map(|node| Arc::new(*node)).collect()
    }

    pub fn get_attributes(&self, node_ref: Arc<NodeRef>) -> Vec<Attribute> {
        self.inner().attributes(*node_ref).to_vec()
    }
    pub fn get(&self, node_ref: Arc<NodeRef>) -> NodeData {
        self.inner().get(*node_ref).clone()
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
    fn inner(&self) -> &super::Document {
        unsafe {&*self.inner.get()}
    }
    #[allow(clippy::mut_from_ref)]
    fn inner_mut(&self) -> &mut super::Document {
        unsafe { &mut *self.inner.get() }
    }
    pub fn print_node(
        &self,
        node: NodeRef,
        writer: &mut dyn std::fmt::Write,
        options: PrintOptions,
    ) -> fmt::Result {
        self.inner().print_node(node, writer, options)
    }
}

impl fmt::Display for Document {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        self.inner().print(f, PrintOptions::Pretty)
    }
}

