use std::{fmt, sync::Arc};

use cranelift_entity::entity_impl;
use petgraph::graph::{IndexType, NodeIndex};
use smallstr::SmallString;

use super::{ffi::Document as FFiDocument, Attribute, AttributeName};
use crate::{InternedString, Symbol};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, uniffi::Object)]
pub struct NodeRef(pub(crate) u32);

entity_impl!(NodeRef, "node");
impl Default for NodeRef {
    #[inline]
    fn default() -> Self {
        use cranelift_entity::packed_option::ReservedValue;

        Self::reserved_value()
    }
}

#[uniffi::export]
impl NodeRef {
    // Kotlin uses ref but is a signed integer.
    pub fn r#ref(&self) -> i32 {
        self.0 as i32
    }
}

impl From<NodeIndex<Self>> for NodeRef {
    #[inline(always)]
    fn from(id: NodeIndex<Self>) -> Self {
        Self(id.index() as u32)
    }
}
unsafe impl IndexType for NodeRef {
    #[inline(always)]
    fn new(x: usize) -> Self {
        Self(x as u32)
    }

    #[inline(always)]
    fn index(&self) -> usize {
        self.0 as usize
    }

    #[inline(always)]
    fn max() -> Self {
        Self(u32::MAX)
    }
}

/// This enum represents the valid node types of a `Document` tree
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum NodeData {
    /// A marker node that indicates the root of a document
    ///
    /// A document may only have a single root, and it has no attributes
    Root,
    /// A typed node that can carry attributes and may contain other nodes
    NodeElement { element: Element },
    /// A leaf node is an untyped node, typically text, and does not have any attributes or children
    Leaf { value: String },
}

#[derive(Clone, uniffi::Object)]
pub struct Node {
    document: FFiDocument,
    pub id: NodeRef,
    pub data: NodeData,
}
impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.document
            .print_node(self.id, f, crate::dom::PrintOptions::Pretty)
    }
}

#[uniffi::export]
impl Node {
    #[uniffi::constructor]
    pub fn new(document: &FFiDocument, id: &NodeRef, data: NodeData) -> Self {
        Self {
            document: document.clone(),
            id: *id,
            data,
        }
    }
    pub fn get_children(&self) -> Vec<Arc<Node>> {
        self.document
            .children(self.id.into())
            .iter()
            .map(|node_ref| self.document.get_node(node_ref.clone()).into())
            .collect()
    }

    pub fn get_depth_first_children(&self) -> Vec<Arc<Node>> {
        let mut out: Vec<Arc<Node>> = Vec::new();
        //out.push(self.clone().into());
        for child in self.get_children() {
            out.push(child.clone());
            let depth = child.get_depth_first_children();
            out.extend(depth);
        }
        out
    }
    pub fn document(&self) -> FFiDocument {
        self.document.clone()
    }
    pub fn id(&self) -> NodeRef {
        self.id
    }

    pub fn data(&self) -> NodeData {
        self.data.clone()
    }

    pub fn attributes(&self) -> Vec<Attribute> {
        self.data.attributes()
    }

    pub fn get_attribute(&self, name: AttributeName) -> Option<Attribute> {
        let attrs = match &self.data {
            NodeData::NodeElement { element } => &element.attributes,
            _ => return None,
        };

        attrs.iter().find(|attr| attr.name == name).cloned()
    }

    pub fn display(&self) -> String {
        format!("{self}")
    }
}
impl NodeData {
    pub fn has_attribute(&self, name: &str, namespace: Option<&str>) -> bool {
        let attrs = match &self {
            NodeData::NodeElement { element } => &element.attributes,
            _ => return false,
        };

        attrs
            .iter()
            .any(|attr| attr.name.name == name && attr.name.namespace.as_deref() == namespace)
    }

    /// Returns a slice of Attributes for this node, if applicable
    pub fn attributes(&self) -> Vec<Attribute> {
        match self {
            Self::NodeElement { element: elem } => elem.attributes.clone(),
            _ => vec![],
        }
    }

    pub fn id(&self) -> Option<String> {
        match self {
            Self::NodeElement { element: el } => el.id(),
            _ => None,
        }
    }

    /// Returns true if this node is a leaf node
    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf { value: _ })
    }
}
impl NodeData {
    /// Creates a new, empty element node with the given tag name
    #[inline]
    pub fn new<T: Into<ElementName>>(tag: T) -> Self {
        Self::NodeElement {
            element: Element::new(tag.into()),
        }
    }
}

impl From<Element> for NodeData {
    #[inline(always)]
    fn from(elem: Element) -> Self {
        Self::NodeElement { element: elem }
    }
}
impl From<&str> for NodeData {
    #[inline(always)]
    fn from(string: &str) -> Self {
        Self::Leaf {
            value: string.to_string(),
        }
    }
}
impl From<String> for NodeData {
    #[inline(always)]
    fn from(string: String) -> Self {
        Self::Leaf { value: string }
    }
}
impl From<SmallString<[u8; 16]>> for NodeData {
    #[inline(always)]
    fn from(string: SmallString<[u8; 16]>) -> Self {
        Self::Leaf {
            value: string.to_string(),
        }
    }
}

/// Represents the fully-qualified name of an element
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, uniffi::Record)]
pub struct ElementName {
    pub namespace: Option<String>,
    pub name: String,
}
impl fmt::Display for ElementName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref ns) = self.namespace {
            write!(f, "{}:{}", ns, &self.name)
        } else {
            write!(f, "{}", &self.name)
        }
    }
}
impl ElementName {
    #[inline]
    pub fn new<N: Into<String>>(name: N) -> Self {
        Self {
            namespace: None,
            name: name.into(),
        }
    }

    #[inline]
    pub fn new_with_namespace<NS: Into<String>, N: Into<String>>(namespace: NS, name: N) -> Self {
        Self {
            namespace: Some(namespace.into()),
            name: name.into(),
        }
    }
}
impl From<&str> for ElementName {
    fn from(s: &str) -> Self {
        match s.split_once(':') {
            None => Self::new(s),
            Some((ns, name)) => Self::new_with_namespace(ns, name),
        }
    }
}
impl From<InternedString> for ElementName {
    #[inline]
    fn from(s: InternedString) -> Self {
        Self::from(s.as_str())
    }
}
impl From<Symbol> for ElementName {
    #[inline]
    fn from(s: Symbol) -> Self {
        Self::from(InternedString::from(s))
    }
}
impl From<ElementName> for InternedString {
    fn from(val: ElementName) -> Self {
        if val.namespace.is_none() {
            val.name.into()
        } else {
            let string = val.to_string();
            string.into()
        }
    }
}
impl PartialEq<InternedString> for ElementName {
    fn eq(&self, other: &InternedString) -> bool {
        let name = Self::from(other.as_str());
        self.eq(&name)
    }
}

/// An `Element` is a typed node in a document, with the ability to carry attributes and contain other nodes.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Element {
    pub name: ElementName,
    pub attributes: Vec<Attribute>,
}
impl Element {
    /// Creates a new element whose type is given by `tag`, without any attributes or children
    ///
    /// The default namespace for all elements is None.
    pub fn new(name: ElementName) -> Self {
        Self {
            name,
            attributes: Vec::new(),
        }
    }

    pub(crate) fn id(&self) -> Option<String> {
        for attr in &self.attributes {
            if attr.name.eq("id") {
                return attr.value.clone();
            }
        }
        None
    }

    /// Returns a slice of AttributeRefs associated to this element
    #[inline]
    pub fn attributes(&self) -> Vec<Attribute> {
        self.attributes.clone()
    }

    /// Sets the attribute named `name` on this element.
    ///
    /// If the attribute is already associated with this element, the value is replaced.
    #[inline]
    pub fn set_attribute(&mut self, name: AttributeName, value: Option<String>) {
        for attribute in self.attributes.iter_mut() {
            if attribute.name == name {
                attribute.value = value;
                return;
            }
        }
        self.attributes.push(Attribute { name, value });
    }

    /// Removes the attribute represented by `attr` from this element's attribute list
    pub fn remove_attribute(&mut self, name: &AttributeName) {
        if let Some(pos) = self.attributes.iter().position(|attr| attr.name.eq(name)) {
            self.attributes.remove(pos);
        }
    }
}
