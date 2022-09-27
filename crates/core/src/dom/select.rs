use crate::dom::*;

use crate::InternedString;

/// Represents a selector over elements in a `Document`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector<'a> {
    /// Selects an element with the given tag name, e.g. `foo`
    Tag(InternedString),
    /// Selects an element with the given unique id, e.g. `#id`
    Id(&'a str),
    /// Selects all elements, e.g. `*`
    All,
    /// Selects elements which match both sub-selectors, e.g. `.foo.bar`
    And(Box<Selector<'a>>, Box<Selector<'a>>),
    /// Selects elements which match either sub-selector, e.g. `.foo, .bar`
    Or(Box<Selector<'a>>, Box<Selector<'a>>),
    /// Selects elements which are descendants of the first sub-selector and match the second sub-selector, e.g. `ul.foo li`
    Descendant(Box<Selector<'a>>, Box<Selector<'a>>),
    /// Selects elements which are direct children of the first sub-selector and match the second sub-selector, e.g. `ul.foo > li`
    Child(Box<Selector<'a>>, Box<Selector<'a>>),
    /// Selects elements which have an attribute with the given name, e.g. `a[href]`
    Attribute(InternedString),
    /// Selects elements which have an attribute with the given name and value, e.g. `a[href="https://example.org]`
    AttributeValue(InternedString, AttributeValue),
    /// Selects elements which have an attribute with the given name whose value is a whitespace-separated list of values containing the given string, e.g. `[attr~=value]`
    AttributeValueWhitespacedContains(InternedString, &'a str),
    /// Selects elements which have an attribute with the given name whose value is prefixed by the given string, e.g. `[attr^=value]`
    AttributeValueStartsWith(InternedString, &'a str),
    /// Selects elements which have an attribute with the given name whose value is suffixed by the given string, e.g. `[attr$=value]`
    AttributeValueEndsWith(InternedString, &'a str),
    /// Selects elements which have an attribute with the given name whose value contains the given string, e.g. `[attr*=value]`
    AttributeValueSubstring(InternedString, &'a str),
}
impl<'a> Selector<'a> {
    /// Returns true if this selection can match at most one node, which is only true when an identified
    /// node is selected or is selected using a combinator that implies exlusion. For example, selecting
    /// an identified node as a descendant/child of an arbitrary selector is guaranteed to be unique,
    /// because the selection as a whole can only produce a single result, or none.
    pub fn is_unique(&self) -> bool {
        match self {
            Self::Id(_) => true,
            Self::And(l, r) => l.is_unique() || r.is_unique(),
            Self::Descendant(_, child) => child.is_unique(),
            Self::Child(_, child) => child.is_unique(),
            _ => false,
        }
    }

    /// Checks if the given node matches this selector
    pub fn matches(&self, node: NodeRef, document: &Document) -> bool {
        let element = match &document.nodes[node] {
            Node::Element(ref elem) => elem,
            Node::Leaf(_) | Node::Root => return false,
        };

        match self {
            Self::Tag(t) => element.tag == t,
            Self::Id(id) => match document.get_by_id(*id) {
                None => false,
                Some(identified) => node == identified,
            },
            Self::All => true,
            Self::And(l, r) => l.matches(node, document) && r.matches(node, document),
            Self::Or(l, r) => l.matches(node, document) || r.matches(node, document),
            Self::Descendant(ancestor, selector) => {
                if !selector.matches(node, document) {
                    return false;
                }
                let mut parent = document.parent(node);
                while let Some(p) = parent.take() {
                    if ancestor.matches(p, document) {
                        return true;
                    }
                    parent = document.parent(p);
                }
                false
            }
            Self::Child(parent, selector) => {
                if !selector.matches(node, document) {
                    return false;
                }
                match document.parent(node) {
                    None => false,
                    Some(p) => parent.matches(p, document),
                }
            }
            Self::Attribute(name) => {
                for attr in element.attributes(&document.attribute_lists) {
                    let attr = &document.attrs[*attr];
                    if attr.name == name {
                        return true;
                    }
                }
                false
            }
            Self::AttributeValue(name, ref value) => {
                for attr in element.attributes(&document.attribute_lists) {
                    let attr = &document.attrs[*attr];
                    if attr.name == name && attr.value.eq(value) {
                        return true;
                    }
                }
                false
            }
            Self::AttributeValueWhitespacedContains(name, expected) => {
                for attr in element.attributes(&document.attribute_lists) {
                    let attr = &document.attrs[*attr];
                    if attr.name != name {
                        continue;
                    }
                    let value = match &attr.value {
                        AttributeValue::String(s) => s.as_str(),
                        AttributeValue::None => "",
                    };
                    for split in value.split_whitespace() {
                        if split == *expected {
                            return true;
                        }
                    }
                }
                false
            }
            Self::AttributeValueStartsWith(name, prefix) => {
                for attr in element.attributes(&document.attribute_lists) {
                    let attr = &document.attrs[*attr];
                    if attr.name != name {
                        continue;
                    }
                    match &attr.value {
                        AttributeValue::String(s) if s.starts_with(prefix) => return true,
                        AttributeValue::None if prefix.is_empty() => return true,
                        _ => continue,
                    }
                }
                false
            }
            Self::AttributeValueEndsWith(name, suffix) => {
                for attr in element.attributes(&document.attribute_lists) {
                    let attr = &document.attrs[*attr];
                    if attr.name != name {
                        continue;
                    }
                    match &attr.value {
                        AttributeValue::String(s) if s.ends_with(suffix) => return true,
                        AttributeValue::None if suffix.is_empty() => return true,
                        _ => continue,
                    }
                }
                false
            }
            Self::AttributeValueSubstring(name, substring) => {
                for attr in element.attributes(&document.attribute_lists) {
                    let attr = &document.attrs[*attr];
                    if attr.name != name {
                        continue;
                    }
                    match &attr.value {
                        AttributeValue::String(s) if s.contains(substring) => return true,
                        AttributeValue::None if substring.is_empty() => return true,
                        _ => continue,
                    }
                }
                false
            }
        }
    }
}

pub struct SelectionIter<'doc, 'select> {
    document: &'doc Document,
    selection: Selector<'select>,
    state: petgraph::visit::Dfs<NodeRef, fixedbitset::FixedBitSet>,
    is_unique: bool,
    is_done: bool,
}
impl<'doc, 'select> SelectionIter<'doc, 'select> {
    pub(super) fn new(
        document: &'doc Document,
        selection: Selector<'select>,
        start: NodeRef,
    ) -> Self {
        let state = petgraph::visit::Dfs::new(document, start);
        let is_unique = selection.is_unique();
        Self {
            document,
            selection,
            state,
            is_unique,
            is_done: false,
        }
    }
}
impl<'doc, 'select> Iterator for SelectionIter<'doc, 'select> {
    type Item = NodeRef;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }

        while let Some(node) = self.state.next(self.document) {
            if self.selection.matches(node, self.document) {
                if self.is_unique {
                    self.is_done = true;
                }
                return Some(node);
            }
        }

        self.is_done = true;
        None
    }
}
