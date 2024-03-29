use std::fmt;

use cranelift_entity::entity_impl;
use petgraph::graph::{IndexType, NodeIndex};
use smallstr::SmallString;

use super::{Attribute, AttributeName};
use crate::{InternedString, Symbol};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, uniffi::Object)]
#[repr(transparent)]
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
pub enum Node {
    /// A marker node that indicates the root of a document
    ///
    /// A document may only have a single root, and it has no attributes
    Root,
    /// A typed node that can carry attributes and may contain other nodes
    NodeElement { element: Element },
    /// A leaf node is an untyped node, typically text, and does not have any attributes or children
    Leaf { value: String },
}
impl Node {
    /// Creates a new, empty element node with the given tag name
    #[inline]
    pub fn new<T: Into<ElementName>>(tag: T) -> Self {
        Self::NodeElement { element: Element::new(tag.into()) }
    }

    /// Returns a slice of Attributes for this node, if applicable
    pub fn attributes(&self) -> &[Attribute] {
        match self {
            Self::NodeElement { element: elem } => elem.attributes(),
            _ => &[],
        }
    }

    pub(crate) fn id(&self) -> Option<SmallString<[u8; 16]>> {
        match self {
            Self::NodeElement { element: el } => el.id(),
            _ => None,
        }
    }

    /// Returns true if this node is a leaf node
    pub fn is_leaf(&self) -> bool {
        match self {
            Self::Leaf { value: _ } => true,
            _ => false,
        }
    }
}
impl From<Element> for Node {
    #[inline(always)]
    fn from(elem: Element) -> Self {
        Self::NodeElement { element: elem }
    }
}
impl From<&str> for Node {
    #[inline(always)]
    fn from(string: &str) -> Self {
        Self::Leaf { value: string.to_string() }
    }
}
impl From<String> for Node {
    #[inline(always)]
    fn from(string: String) -> Self {
        Self::Leaf { value: string }
    }
}
impl From<SmallString<[u8; 16]>> for Node {
    #[inline(always)]
    fn from(string: SmallString<[u8; 16]>) -> Self {
        Self::Leaf { value: string.to_string() }
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
    pub fn new_with_namespace<NS: Into<String>, N: Into<String>>(
        namespace: NS,
        name: N,
    ) -> Self {
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
impl Into<InternedString> for ElementName {
    fn into(self) -> InternedString {
        if self.namespace.is_none() {
            self.name.into()
        } else {
            let string = self.to_string();
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

    pub(crate) fn id(&self) -> Option<SmallString<[u8; 16]>> {
        for attr in &self.attributes {
            if attr.name.eq("id") {
                if let Some(value) = &attr.value {
                    let value = SmallString::<[u8; 16]>::from_string(value.clone());
                    return Some(value);
                }
            }
        }
        None
    }

    /// Returns a slice of AttributeRefs associated to this element
    #[inline]
    pub fn attributes(&self) -> &[Attribute] {
        self.attributes.as_slice()
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
