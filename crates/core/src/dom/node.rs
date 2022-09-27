use cranelift_entity::entity_impl;
use petgraph::graph::{IndexType, NodeIndex};
use smallstr::SmallString;
use smallvec::SmallVec;

use crate::InternedString;

use super::{AttributeList, AttributeListPool, AttributeRef};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct NodeRef(u32);
entity_impl!(NodeRef, "node");
impl Default for NodeRef {
    #[inline]
    fn default() -> Self {
        use cranelift_entity::packed_option::ReservedValue;

        Self::reserved_value()
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
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// A marker node that indicates the root of a document
    ///
    /// A document may only have a single root, and it has no attributes
    Root,
    /// A typed node that can carry attributes and may contain other nodes
    Element(Element),
    /// A leaf node is an untyped node, typically text, and does not have any attributes or children
    Leaf(SmallString<[u8; 16]>),
}
impl Node {
    /// Creates a new, empty element node with the given tag name
    #[inline]
    pub fn new<T: Into<InternedString>>(tag: T) -> Self {
        Self::Element(Element::new(tag.into()))
    }

    /// Creates a new, empty element node with the given tag name and namespace
    pub fn with_namespace<T: Into<InternedString>, NS: Into<InternedString>>(
        tag: T,
        namespace: NS,
    ) -> Self {
        let mut element = Element::new(tag.into());
        element.set_namespace(namespace.into());
        Self::Element(element)
    }

    /// Returns a slice of AttributeRefs for this node, if applicable
    pub fn attributes<'a>(&self, pool: &'a AttributeListPool) -> &'a [AttributeRef] {
        match self {
            Self::Element(elem) => elem.attributes(pool),
            _ => &[],
        }
    }

    /// Returns true if this node is a leaf node
    pub fn is_leaf(&self) -> bool {
        match self {
            Self::Leaf(_) => true,
            _ => false,
        }
    }
}
impl From<Element> for Node {
    #[inline(always)]
    fn from(elem: Element) -> Self {
        Self::Element(elem)
    }
}
impl From<&str> for Node {
    #[inline(always)]
    fn from(string: &str) -> Self {
        Self::Leaf(SmallString::from_str(string))
    }
}
impl From<String> for Node {
    #[inline(always)]
    fn from(string: String) -> Self {
        Self::Leaf(SmallString::from_string(string))
    }
}
impl From<SmallString<[u8; 16]>> for Node {
    #[inline(always)]
    fn from(string: SmallString<[u8; 16]>) -> Self {
        Self::Leaf(string)
    }
}

/// An `Element` is a typed node in a document, with the ability to carry attributes and contain other nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub namespace: Option<InternedString>,
    pub tag: InternedString,
    pub attributes: AttributeList,
}
impl Element {
    /// Creates a new element whose type is given by `tag`, without any attributes or children
    ///
    /// The default namespace for all elements is None.
    pub fn new(tag: InternedString) -> Self {
        Self {
            namespace: None,
            tag,
            attributes: AttributeList::new(),
        }
    }

    /// Sets the namespace of this element, e.g. `http://www.w3.org/2000/svg`
    pub fn set_namespace(&mut self, namespace: InternedString) {
        self.namespace.replace(namespace);
    }

    /// Returns a slice of AttributeRefs for this element
    #[inline]
    pub fn attributes<'a>(&self, pool: &'a AttributeListPool) -> &'a [AttributeRef] {
        self.attributes.as_slice(pool)
    }

    /// Appends the attribute represented by `attr` to this element's attribute list
    #[inline]
    pub fn add_attribute(&mut self, attr: AttributeRef, pool: &mut AttributeListPool) {
        if self.attributes.as_slice(pool).contains(&attr) {
            return;
        }
        self.attributes.push(attr, pool);
    }

    /// Replaces all attributes for which `predicate` returns true, with `replacement`, removing duplicates.
    ///
    /// Preserves the relative position of the first matching attribute, but all duplicates after are removed
    #[inline(never)]
    pub fn replace_attribute<P>(
        &mut self,
        replacement: AttributeRef,
        pool: &mut AttributeListPool,
        predicate: P,
    ) where
        P: Fn(AttributeRef) -> bool,
    {
        // Contains all duplicate matches
        let mut duplicates = SmallVec::<[AttributeRef; 4]>::new();
        // Contains the index to truncate to if all duplicates are at the end of the list contiguously
        let mut truncate_to = None;

        // Find all duplicate matches, and perform the replacement on the first match
        {
            let attributes = self.attributes.as_mut_slice(pool);
            if attributes.is_empty() {
                return;
            }
            let mut first_duplicate = None;
            let mut replaced = false;
            let mut contiguous = true;
            for (i, attr) in attributes.iter_mut().enumerate() {
                if predicate(*attr) {
                    if !replaced {
                        replaced = true;
                        *attr = replacement;
                    } else {
                        if first_duplicate.is_none() {
                            first_duplicate.replace(i);
                        }
                        duplicates.push(*attr);
                    }
                } else {
                    // If we encounter a non-matching attribute after the first duplicate, the duplicates are not contiguous
                    if first_duplicate.is_some() {
                        contiguous = false;
                    }
                }
            }

            if contiguous && first_duplicate.is_some() {
                // If all of the duplicates are contiguous (meaning there were no non-matching attributes after the first duplicate),
                // then we can use truncation to remove the dupes, by simply adding 1 to the first dupe index to get the new effective length
                truncate_to = first_duplicate.map(|i| i + 1);
            }
        };

        // If there are no duplicates, we're done
        if duplicates.is_empty() {
            return;
        }

        // If there were duplicates but we can truncate, then prefer that
        if let Some(new_len) = truncate_to {
            self.attributes.truncate(new_len, pool);
            return;
        }

        // Otherwise we need to go through and remove each duplicate individually
        while let Some(duplicate) = duplicates.pop() {
            self.remove_attribute(duplicate, pool);
        }
    }

    /// Removes the attribute represented by `attr` from this element's attribute list
    pub fn remove_attribute(&mut self, attr: AttributeRef, pool: &mut AttributeListPool) {
        if let Some(pos) = self
            .attributes
            .as_slice(pool)
            .iter()
            .copied()
            .position(|a| a == attr)
        {
            self.attributes.remove(pos, pool);
        }
    }
}
