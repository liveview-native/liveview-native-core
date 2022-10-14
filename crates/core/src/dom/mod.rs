mod attribute;
mod node;
mod printer;
mod select;

pub use self::attribute::{
    Attribute, AttributeIter, AttributeList, AttributeListPool, AttributeRef, AttributeValue,
};
pub use self::node::{Element, Node, NodeRef};
pub use self::printer::PrintOptions;
pub use self::select::{SelectionIter, Selector};

use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::fmt;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::path::Path;

use cranelift_entity::packed_option::PackedOption;
use cranelift_entity::{EntityRef, PrimaryMap, SecondaryMap};
use fixedbitset::FixedBitSet;
use fxhash::{FxBuildHasher, FxHashMap};
use petgraph::Direction;
use smallstr::SmallString;
use smallvec::SmallVec;

use self::printer::Printer;
use crate::parser;
use crate::InternedString;

/// A `Document` represents a virtual DOM, and supports common operations typically performed against them.
///
/// While I'm referring to it as a DOM because it conjures the familiar notion of an HTML document, what we're
/// really talking about here is a tree data structure composed of two types of nodes:
///
/// * _Element_ nodes, which are named, rich objects that can carry an arbitrary set of attributes, and may contain any number of child nodes.
/// * _Leaf_ nodes, which are plain content (typically text) that have no attributes and cannot have any children.
///
/// A `Document` is a tree, not a forest and not an arbitrary graph, as such, all but the root node must have a parent, and only one parent.
///
///
/// Typical operations performed against a `Document` (or DOM) include:
///
/// * Appending children to an element
/// * Inserting a node before/after another node
/// * Removing children from an element
/// * Replacing a node with another node
/// * Adding/removing/updating attributes of an element
/// * Traversing the document depth-first or breadth-first
/// * Searching for one or more nodes in the document based on various criteria such as type, content, or attributes
///
/// In addition, virtual DOMs are useful for calculating the most efficient way to update the tree given a set of
/// patches or another document reflecting the desired outcome. This can be produced as a diff, or changelist, or simply
/// applied directly.
///
/// This structure is designed with a few specific goals in mind:
///
/// * Be cache- and allocator-friendly, by which I mean that common operations generally can avoid chasing pointers,
/// and performing allocations unless absolutely necessary.
/// * Allow most operations to be performed in constant time
///
#[derive(Clone)]
pub struct Document {
    root: NodeRef,
    /// A map from node reference to node data
    nodes: PrimaryMap<NodeRef, Node>,
    /// A map from a node to its parent node, if it currently has one
    parents: SecondaryMap<NodeRef, PackedOption<NodeRef>>,
    /// A map from a node to its child nodes
    children: SecondaryMap<NodeRef, SmallVec<[NodeRef; 4]>>,
    /// A map from attribute reference to attribute data
    pub(crate) attrs: PrimaryMap<AttributeRef, Attribute>,
    /// A map from a unique id (defined in the source document) to Node, contains an entry for every
    /// node in the document which had an "id" (or equivalent) attribute set in the source document.
    /// This allows for looking up a node directly and modifying it, rather than needing to traverse the
    /// document.
    ids: BTreeMap<SmallString<[u8; 16]>, NodeRef>,
    /// The pool for lists of Attribute
    pub(crate) attribute_lists: AttributeListPool,
}
impl fmt::Debug for Document {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use petgraph::dot::{Config, Dot};
        let edge_getter = |_, edge_ref: EdgeRef| format!("{} to {}", edge_ref.from, edge_ref.to);
        let node_getter = |_, node_ref: (NodeRef, &Node)| format!("id = {}", node_ref.0);
        let dot = Dot::with_attr_getters(self, &[Config::EdgeNoLabel], &edge_getter, &node_getter);
        write!(f, "{:?}", &dot)
    }
}
impl fmt::Display for Document {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f, PrintOptions::Pretty)
    }
}
impl Default for Document {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}
impl Document {
    /// Creates a new, empty Document
    #[inline]
    pub fn empty() -> Self {
        Self::with_capacity(1)
    }

    /// Starts building a Document
    #[inline]
    pub fn build() -> Builder {
        Builder::new()
    }

    /// Creates a new empty Document, with preallocated capacity for nodes
    pub fn with_capacity(cap: usize) -> Self {
        let mut nodes = PrimaryMap::with_capacity(cap);
        let root = nodes.push(Node::Root);
        Self {
            root,
            nodes,
            parents: SecondaryMap::new(),
            children: SecondaryMap::new(),
            attrs: PrimaryMap::new(),
            ids: Default::default(),
            attribute_lists: AttributeListPool::new(),
        }
    }

    /// Parses a `Document` from a string
    pub fn parse<S: AsRef<str>>(input: S) -> Result<Self, parser::ParseError> {
        parser::parse(input.as_ref())
    }

    /// Parses a `Document` from raw bytes
    pub fn parse_bytes<B: AsRef<[u8]>>(input: B) -> Result<Self, parser::ParseError> {
        parser::parse(input.as_ref())
    }

    /// Parses a `Document` from a file at the given path
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Self, parser::ParseError> {
        parser::parse(std::fs::File::open(path)?)
    }

    /// Obtains a `DocumentBuilder` with which you can extend/modify this document
    #[inline]
    pub fn edit(&mut self) -> Editor<'_> {
        Editor::new(self)
    }

    /// Clears all data from this document, but keeps the allocated capacity, for more efficient reuse
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.root = self.nodes.push(Node::Root);
        self.parents.clear();
        self.children.clear();
        self.attrs.clear();
        self.ids.clear();
        self.attribute_lists.clear();
    }

    /// Returns true if this document is empty (contains no nodes)
    pub fn is_empty(&self) -> bool {
        self.children[self.root].is_empty()
    }

    /// Returns the root node of the document
    ///
    /// The root node can be used in insertion operations, but can not have attributes applied to it
    pub fn root(&self) -> NodeRef {
        self.root
    }

    /// Registers `node` with the identifier `id`
    ///
    /// If `id` was previously registered to a different node, that node is returned
    pub fn register_id<S: Into<SmallString<[u8; 16]>>>(
        &mut self,
        node: NodeRef,
        id: S,
    ) -> Option<NodeRef> {
        self.ids.insert(id.into(), node)
    }

    /// Returns the data associated with the given `NodeRef`
    #[inline]
    pub fn get(&self, node: NodeRef) -> &Node {
        &self.nodes[node]
    }

    /// Returns the data associated with the given `NodeRef`, mutably
    #[inline]
    pub fn get_mut(&mut self, node: NodeRef) -> &mut Node {
        &mut self.nodes[node]
    }

    /// Returns the set of attribute refs associated with `node`
    pub fn attributes(&self, node: NodeRef) -> &[AttributeRef] {
        match &self.nodes[node] {
            Node::Element(ref elem) => elem.attributes(&self.attribute_lists),
            _ => &[],
        }
    }

    /// Returns an iterator over the attributes of `node`
    pub fn get_attributes(&self, node: NodeRef) -> AttributeIter<'_> {
        AttributeIter {
            attrs: &self.attrs,
            refs: self.attributes(node),
        }
    }

    /// Returns the data associated with the given `AttributeRef`
    #[inline]
    pub fn get_attribute(&self, attr: AttributeRef) -> &Attribute {
        &self.attrs[attr]
    }

    /// Returns the first occurrence of the attribute `name` on `node`, otherwise `None`
    pub fn get_attribute_by_name<'a>(&'a self, node: NodeRef, name: &str) -> Option<&'a Attribute> {
        self.get_attributes(node)
            .find_map(|attr| if attr.name == name { Some(attr) } else { None })
    }

    /// Returns all of the occurrences of the attribute `name` on `node`
    ///
    /// Since it is possible to have duplicate instances of an attribute, this function exists to
    /// make working with such attributes more straightforward.
    pub fn get_attributes_by_name<'a>(
        &'a self,
        node: NodeRef,
        name: &str,
    ) -> SmallVec<[&'a Attribute; 2]> {
        self.get_attributes(node)
            .filter(|attr| attr.name == name)
            .collect()
    }

    /// Returns the parent of `node`, if it has one
    #[inline]
    pub fn parent(&self, node: NodeRef) -> Option<NodeRef> {
        self.parents[node].expand()
    }

    /// Returns the children of `node` as a slice
    #[inline]
    pub fn children(&self, node: NodeRef) -> &[NodeRef] {
        self.children[node].as_slice()
    }

    /// Returns the `NodeRef` associated with the given unique identifier
    pub fn get_by_id<S: AsRef<str>>(&self, id: S) -> Option<NodeRef> {
        self.ids.get(id.as_ref()).copied()
    }

    /// Returns an iterator over all nodes in this document which match `selector`
    ///
    /// The nodes are visited in depth-first order, and the iterator terminates as soon as the selection is considered fully matched
    pub fn select<'doc, 'select>(
        &'doc self,
        selector: Selector<'select>,
    ) -> SelectionIter<'doc, 'select> {
        SelectionIter::new(self, selector, self.root)
    }

    /// Returns an iterator over the portion of this document rooted at `node` which match `selector`
    ///
    /// The nodes are visited in depth-first order, and the iterator terminates as soon as the selection is considered fully matched
    pub fn select_children<'doc, 'select>(
        &'doc self,
        node: NodeRef,
        selector: Selector<'select>,
    ) -> SelectionIter<'doc, 'select> {
        SelectionIter::new(self, selector, node)
    }

    /// Attaches `doc` to this document, with `parent` as the parent of the new subtree.
    pub fn attach_document(&mut self, parent: NodeRef, mut doc: Document) {
        // Copy over attributes first, since they are referenced by nodes
        let num_attrs = doc.attrs.len();
        let mut attr_mapping = FxHashMap::<AttributeRef, AttributeRef>::with_capacity_and_hasher(
            num_attrs,
            FxBuildHasher::default(),
        );
        self.attrs.reserve(num_attrs);
        for (k, v) in doc.attrs.into_iter() {
            let new_k = self.attrs.push(v);
            attr_mapping.insert(k, new_k);
        }

        // Copy over nodes, ignoring the root element, and remapping attributes for elements
        let num_nodes = doc.nodes.len();
        let mut node_mapping = FxHashMap::<NodeRef, NodeRef>::with_capacity_and_hasher(
            num_nodes,
            FxBuildHasher::default(),
        );
        self.nodes.reserve(num_nodes);
        for (k, v) in doc.nodes.into_iter() {
            match v {
                Node::Root => continue,
                v @ Node::Leaf(_) => {
                    let new_k = self.nodes.push(v);
                    node_mapping.insert(k, new_k);
                }
                Node::Element(mut elem) => {
                    let old_attrs = elem.attributes.as_slice(&doc.attribute_lists);
                    let mut new_attrs = AttributeList::new();
                    for attr in old_attrs {
                        new_attrs.push(attr_mapping[attr], &mut self.attribute_lists);
                    }
                    elem.attributes = new_attrs;
                    let new_k = self.nodes.push(Node::Element(elem));
                    node_mapping.insert(k, new_k);
                }
            }
        }
        // Remap parents/children now that all nodes are copied over
        for (k, new_k) in node_mapping.iter() {
            if let Some(old_parent) = doc.parents[*k].expand() {
                if old_parent == doc.root {
                    self.parents[*new_k] = parent.into();
                } else {
                    if let Some(new_parent) = node_mapping.get(&old_parent) {
                        self.parents[*new_k] = (*new_parent).into();
                    }
                }
            }
            let old_children = &doc.children[*k];
            let mut children = SmallVec::<[NodeRef; 4]>::new();
            for old_child in old_children {
                children.push(node_mapping[old_child]);
            }
            self.children[*k] = children;
        }
        // Bring over id mappings from the old document
        while let Some((id, node)) = doc.ids.pop_first() {
            self.ids.insert(id, node);
        }
    }

    /// Appends `child` to the end of the list of `parent`'s children
    ///
    /// This function will panic if `child` already has a parent.
    /// To reparent an existing node, you must first detach it with `detach`.
    pub fn append_child(&mut self, parent: NodeRef, child: NodeRef) {
        assert_eq!(self.parents[child].expand(), None);

        let children = &mut self.children[parent];
        children.push(child);
        self.parents[child] = parent.into();
    }

    /// Prepends `child` to the start of the list of `parent`'s children
    ///
    /// This function will panic if `child` already has a parent.
    /// To reparent an existing node, you must first detach it with `detach`.
    pub fn prepend_child(&mut self, parent: NodeRef, child: NodeRef) {
        assert_eq!(self.parents[child].expand(), None);
        let children = &mut self.children[parent];
        children.insert(0, child);
        self.parents[child] = parent.into();
    }

    /// Inserts `node` as a sibling node of `after` in the document.
    ///
    /// As implied, `node` will follow `after` in the document, as a sibling, not a child or parent.
    ///
    /// This function will panic if:
    ///
    /// * `node` and `after` are the same node
    /// * `node` already has a parent
    /// * `after` has no parent
    ///
    /// To reparent an existing node, you must first detach it with `detach`.
    pub fn insert_after(&mut self, node: NodeRef, after: NodeRef) {
        assert_ne!(node, after);
        assert_eq!(self.parents[node].expand(), None);
        let parent = self.parents[after].expand().unwrap();
        let children = &mut self.children[parent];
        let position = children.iter().copied().position(|n| n == after).unwrap();
        // Attach `node` as a child of `parent`
        self.parents[node] = parent.into();
        // Insert `node` in the appropriate location amongst its siblings
        match position {
            // If the position of `after` is last, simply append `node` to the list of children
            n if n == children.len() - 1 => children.push(node),
            n => children.insert(n + 1, node),
        }
    }

    /// Inserts `node` as a sibling node of `before` in the document.
    ///
    /// As implied, `node` will precede `before` in the document, as a sibling, not a child or parent.
    ///
    /// This function will panic if:
    ///
    /// * `node` and `before` are the same node
    /// * `node` already has a parent
    /// * `before` has no parent
    ///
    /// To reparent an existing node, you must first detach it with `detach`.
    pub fn insert_before(&mut self, node: NodeRef, before: NodeRef) {
        assert_ne!(node, before);
        assert_eq!(self.parents[node].expand(), None);
        let parent = self.parents[before].expand().unwrap();
        let children = &mut self.children[parent];
        let position = children.iter().copied().position(|n| n == before).unwrap();
        // Attach `node` as a child of `parent`
        self.parents[node] = parent.into();
        // Insert `node` in the appropriate location amongst its siblings
        children.insert(position, node);
    }

    /// Detaches a node from the document, but preserves the subtree of the node
    ///
    /// The data associated with detached nodes remains stored in the document; see `delete` if you require that behavior.
    #[inline]
    pub fn detach(&mut self, node: NodeRef) {
        if let Some(parent) = self.parents[node].take() {
            let children = &mut self.children[parent];
            if let Some(pos) = children.iter().copied().position(|n| n == node) {
                children.remove(pos);
            }
        }
    }

    /// Deletes a node from the document, along with all of its children and associated data
    ///
    /// This operation cannot be undone; once deleted, the node tree rooted at `node` cannot be recovered.
    pub fn delete(&mut self, node: NodeRef) {
        let mut stack = VecDeque::<NodeRef>::with_capacity(4);
        stack.push_back(node);

        while let Some(node) = stack.pop_front() {
            // Detach node from its parent
            self.detach(node);

            // Remove the children from the document and add them to the stack to be visited
            //
            // We replace the existing SmallVec with a fresh one if the number of children would
            // have required a heap allocation (size > 2 machine words). This way we free up that
            // unused memory for other allocations.
            let children = &mut self.children[node];
            match children.len() {
                0 => continue,
                n if n < 4 => {
                    stack.extend(children.drain(..));
                }
                _ => {
                    stack.extend(children.drain(..));
                    let _ = mem::replace(children, SmallVec::new());
                }
            }
        }
    }

    /// Adds a node to this document, returning the corresponding NodeRef.
    ///
    /// This operation adds `node` to the document without inserting it in the tree, i.e. it is initially detached
    #[inline]
    pub fn push_node<N: Into<Node>>(&mut self, node: N) -> NodeRef {
        self.nodes.push(node.into())
    }

    /// Appends the attribute named `name` with value `value` to the list of `node`'s attributes.
    ///
    /// Returns true if `node` was an element and `attr` could be set, otherwise false.
    pub fn add_attribute<K: Into<InternedString>, V: Into<AttributeValue>>(
        &mut self,
        node: NodeRef,
        name: K,
        value: V,
    ) -> bool {
        self.append_attribute(node, Attribute::new(name.into(), value.into()))
    }

    /// Appends the attribute corresponding to `attr` to the list of `node`'s attributes.
    ///
    /// Returns true if `node` was an element and `attr` could be set, otherwise false.
    pub fn append_attribute(&mut self, node: NodeRef, attr: Attribute) -> bool {
        if let Node::Element(ref mut elem) = &mut self.nodes[node] {
            let attr = self.attrs.push(attr);
            elem.add_attribute(attr, &mut self.attribute_lists);
            true
        } else {
            false
        }
    }

    /// Replaces the value of attribute `name` with `value` on `node`.
    ///
    /// This has the effect of removing all previous individual instances of the specified attribute, giving it a single canonical value
    ///
    /// Returns true if `node` was an element and `attr` could be replaced, otherwise false.
    pub fn update_attribute<K: Into<InternedString>, V: Into<AttributeValue>>(
        &mut self,
        node: NodeRef,
        name: K,
        value: V,
    ) -> bool {
        if let Node::Element(ref mut elem) = &mut self.nodes[node] {
            let name = name.into();
            let replacement = self.attrs.push(Attribute::new(name, value.into()));
            elem.replace_attribute(replacement, &mut self.attribute_lists, |a| {
                let attr = &self.attrs[a];
                attr.namespace.is_none() && attr.name == name
            });
            true
        } else {
            false
        }
    }

    /// Removes any instance of an attribute named `name`
    pub fn remove_attribute<K: Into<InternedString>>(&mut self, node: NodeRef, name: K) {
        let name = name.into();
        self.remove_attribute_by(node, |a| a.name == name);
    }

    /// Removes any instance of an attribute named `name`, with optional namespace
    pub fn remove_attribute_by_full_name<K: Into<InternedString>>(
        &mut self,
        node: NodeRef,
        namespace: Option<InternedString>,
        name: K,
    ) {
        let name = name.into();
        self.remove_attribute_by(node, |a| a.namespace == namespace && a.name == name);
    }

    pub fn remove_attribute_by<P>(&mut self, node: NodeRef, predicate: P)
    where
        P: Fn(&Attribute) -> bool,
    {
        if let Node::Element(ref mut elem) = &mut self.nodes[node] {
            let mut matches = elem
                .attributes
                .as_slice(&mut self.attribute_lists)
                .iter()
                .copied()
                .filter(|a| predicate(&self.attrs[*a]))
                .collect::<SmallVec<[AttributeRef; 2]>>();
            for attr in matches.drain(..) {
                let pos = elem
                    .attributes
                    .as_slice(&self.attribute_lists)
                    .iter()
                    .copied()
                    .position(|a| a == attr)
                    .unwrap();
                elem.attributes.remove(pos, &mut self.attribute_lists);
            }
        }
    }

    /// Prints this document using the given writer and options
    pub fn print(&self, writer: &mut dyn fmt::Write, options: PrintOptions) -> fmt::Result {
        self.print_node(self.root, writer, options)
    }

    /// Prints a node in this document using the given writer and options
    pub fn print_node(
        &self,
        node: NodeRef,
        writer: &mut dyn fmt::Write,
        options: PrintOptions,
    ) -> fmt::Result {
        let printer = Printer::new(self, node, options);
        printer.print(writer)
    }
}

/// This trait is used to provide functionality common to construction/mutating documents
pub trait DocumentBuilder {
    fn document(&self) -> &Document;

    fn document_mut(&mut self) -> &mut Document;

    /// Returns the NodeRef of the node the builder is currently pointing at
    fn insertion_point(&self) -> NodeRef;

    /// Obtain a new, scoped builder which will restore the current insertion point when dropped
    ///
    /// This can be used to perform a sequence of traversals/mutations, while ensuring that the
    /// builder returns to the current location in the document before continuing
    fn insert_guard<'g, 'b: 'g>(&'b mut self) -> InsertionGuard<'g, Self>
    where
        Self: Sized,
    {
        InsertionGuard {
            ip: self.insertion_point(),
            builder: self,
        }
    }

    /// Move the builder to `node`, this is equivalent to `set_insertion_point_after`
    fn set_insertion_point(&mut self, node: NodeRef);

    /// Move the builder such that new nodes are appended after `node`
    fn set_insertion_point_after(&mut self, node: NodeRef);

    /// Move the builder to the parent of the current node
    ///
    /// Panics if the current node has no parent
    fn set_insertion_point_to_parent(&mut self) {
        self.set_insertion_point(self.parent().unwrap());
    }

    /// Move the builder to the `n`th child of the current node
    ///
    /// Panics if the specified index doesn't exist
    fn set_insertion_point_to_child(&mut self, n: usize) {
        self.set_insertion_point(self.children()[n]);
    }

    /// Move the builder to the `n`th from last child of the current node
    ///
    /// Panics if the specified index doesn't exist
    fn set_insertion_point_to_child_reverse(&mut self, n: usize) {
        let children = self.children();
        let child = children[children.len() - (1 + n)];
        self.set_insertion_point(child);
    }

    /// Move the builder to the `n`th sibling in the list of siblings (absolute, not relative) of the current node
    ///
    /// Panics if the specified index doesn't exist
    #[inline]
    fn set_insertion_point_to_sibling(&mut self, n: usize) {
        self.set_insertion_point_to_parent();
        self.set_insertion_point_to_child(n);
    }

    /// Move the builder to the `n`th from last sibling in the list of siblings (absolute, not relative) of the current node
    ///
    /// Panics if the specified index doesn't exist
    #[inline]
    fn set_insertion_point_to_sibling_reverse(&mut self, n: usize) {
        self.set_insertion_point_to_parent();
        self.set_insertion_point_to_child_reverse(n);
    }

    /// Returns the `Node` corresponding to the current insertion point
    #[inline]
    fn current_node(&self) -> &Node {
        self.document().get(self.insertion_point())
    }

    /// Returns the parent NodeRef of the current insertion point, if there is one
    #[inline]
    fn parent(&self) -> Option<NodeRef> {
        self.document().parent(self.insertion_point())
    }

    /// Returns the child NodeRefs of the current insertion point, if there are any
    #[inline]
    fn children(&self) -> &[NodeRef] {
        self.document().children(self.insertion_point())
    }

    /// Adds an attribute to the attribute set of the current node
    ///
    /// This function panics if `node` does not support attributes
    fn add_attribute<K: Into<InternedString>, V: Into<AttributeValue>>(
        &mut self,
        name: K,
        value: V,
    ) {
        let ip = self.insertion_point();
        assert!(self
            .document_mut()
            .add_attribute(ip, name.into(), value.into()));
    }

    /// Adds `attr` to the attribute set of the current node
    ///
    /// This function panics if `node` does not support attributes
    fn append_attribute(&mut self, attr: Attribute) {
        let ip = self.insertion_point();
        assert!(self.document_mut().append_attribute(ip, attr));
    }

    /// Updates attribute `name` with `value` on `node`
    ///
    /// This has the effect of removing all previous instances of a given attribute and giving it a single canonical value
    fn update_attribute<K: Into<InternedString>, V: Into<AttributeValue>>(
        &mut self,
        name: K,
        value: V,
    ) {
        let ip = self.insertion_point();
        assert!(self
            .document_mut()
            .update_attribute(ip, name.into(), value.into()))
    }

    /// Removes any instance of an attribute named `name`
    fn remove_attribute<K: Into<InternedString>>(&mut self, name: K) {
        let ip = self.insertion_point();
        self.document_mut().remove_attribute(ip, name.into());
    }

    /// Removes any instance of an attribute named `name`, with optional namespace
    fn remove_attribute_by_full_name<K: Into<InternedString>>(
        &mut self,
        namespace: Option<InternedString>,
        name: K,
    ) {
        let ip = self.insertion_point();
        self.document_mut()
            .remove_attribute_by_full_name(ip, namespace, name.into());
    }

    /// Creates a node, returning its NodeRef, without attaching it to the element tree
    fn push_node<N: Into<Node>>(&mut self, node: N) -> NodeRef {
        self.document_mut().push_node(node.into())
    }

    /// Makes the current node the parent of `node`, attaching it to the tree
    ///
    /// This function will panic if `node` is already attached to the tree (i.e. has a parent)
    fn attach_node(&mut self, node: NodeRef) {
        assert_eq!(self.document().parent(node), None);
        let ip = self.insertion_point();
        let doc = self.document_mut();
        doc.parents[node] = ip.into();
        doc.children[ip].push(node);
    }

    /// Merges `doc` into this document, making the current node the parent of the merged subtree
    fn attach_document(&mut self, doc: Document) {
        let ip = self.insertion_point();
        self.document_mut().attach_document(ip, doc)
    }

    /// Appends `node` as a child of the current node
    #[inline]
    fn append<N: Into<Node>>(&mut self, node: N) -> NodeRef {
        let ip = self.insertion_point();
        self.append_child(ip, node.into())
    }

    /// Appends `node` as a child of `to`
    fn append_child<N: Into<Node>>(&mut self, to: NodeRef, node: N) -> NodeRef {
        let doc = self.document_mut();
        let nr = doc.nodes.push(node.into());
        doc.append_child(to, nr);
        nr
    }

    /// Inserts `node`, returning its NodeRef, and making it the new insertion point
    #[inline]
    fn insert<N: Into<Node>>(&mut self, node: N) -> NodeRef {
        let ip = self.insertion_point();
        let nr = self.insert_after(node, ip);
        self.set_insertion_point(ip);
        nr
    }

    /// Inserts `node` as a sibling of `after`, immediately following it in the document
    fn insert_after<N: Into<Node>>(&mut self, node: N, after: NodeRef) -> NodeRef {
        let doc = self.document_mut();
        let nr = doc.nodes.push(node.into());
        doc.insert_after(nr, after);
        nr
    }

    /// Inserts `node` as a sibling of `before`, immediately preceding it in the document
    fn insert_before<N: Into<Node>>(&mut self, node: N, before: NodeRef) -> NodeRef {
        let doc = self.document_mut();
        let nr = doc.nodes.push(node.into());
        doc.insert_before(nr, before);
        nr
    }

    /// Removes `node` from the document, along with all its children
    fn remove(&mut self, node: NodeRef) {
        self.document_mut().delete(node);
    }

    /// Replaces the content of `node` with `replacement`
    fn replace<N: Into<Node>>(&mut self, node: NodeRef, replacement: N) {
        let replace = self.document_mut().get_mut(node);
        *replace = replacement.into();
    }
}

/// This is an RAII structure used to wrap a sequence of mutations/traversals in a guard
/// that when dropped, will result in the builder insertion point being reset to the point
/// at which it started
pub struct InsertionGuard<'a, T: DocumentBuilder + 'a> {
    ip: NodeRef,
    builder: &'a mut T,
}
impl<'a, T: DocumentBuilder> Drop for InsertionGuard<'a, T> {
    #[inline]
    fn drop(&mut self) {
        self.builder.set_insertion_point(self.ip);
    }
}
impl<'a, T: DocumentBuilder> Deref for InsertionGuard<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.builder
    }
}
impl<'a, T: DocumentBuilder> DerefMut for InsertionGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.builder
    }
}

/// A `Builder` is used to construct a new document
pub struct Builder {
    doc: Document,
    pos: NodeRef,
}
impl Builder {
    pub fn new() -> Self {
        let doc = Document::empty();
        let pos = doc.root();
        Self { doc, pos }
    }

    #[inline]
    pub fn finish(self) -> Document {
        self.doc
    }
}
impl DocumentBuilder for Builder {
    #[inline(always)]
    fn document(&self) -> &Document {
        &self.doc
    }

    #[inline(always)]
    fn document_mut(&mut self) -> &mut Document {
        &mut self.doc
    }

    #[inline(always)]
    fn insertion_point(&self) -> NodeRef {
        self.pos
    }

    #[inline(always)]
    fn set_insertion_point(&mut self, node: NodeRef) {
        self.pos = node;
    }

    #[inline(always)]
    fn set_insertion_point_after(&mut self, node: NodeRef) {
        self.pos = node;
    }
}

/// An `Editor` is used to extend/modify a document without taking ownership of it.
pub struct Editor<'a> {
    doc: &'a mut Document,
    pos: NodeRef,
}
impl<'a> Editor<'a> {
    pub fn new(doc: &'a mut Document) -> Self {
        let pos = doc.root();
        Self { doc, pos }
    }

    #[inline(always)]
    pub fn finish(self) {}
}
impl<'a> DocumentBuilder for Editor<'a> {
    #[inline(always)]
    fn document(&self) -> &Document {
        self.doc
    }

    #[inline(always)]
    fn document_mut(&mut self) -> &mut Document {
        self.doc
    }

    #[inline(always)]
    fn insertion_point(&self) -> NodeRef {
        self.pos
    }

    #[inline(always)]
    fn set_insertion_point(&mut self, node: NodeRef) {
        self.pos = node;
    }

    #[inline(always)]
    fn set_insertion_point_after(&mut self, node: NodeRef) {
        self.pos = node;
    }
}

/// Represents a directed edge from a parent node to a child node by combinding the
/// parent and child NodeRefs into a single value.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct EdgeId(u64);
impl EdgeId {
    /// Returns the node from which this edge originates
    #[inline]
    pub fn parent(&self) -> NodeRef {
        NodeRef::new((self.0 >> 32) as usize)
    }

    /// Returns the index of the node in `parent`'s children to which this edge points
    #[inline]
    pub fn child(&self) -> NodeRef {
        NodeRef::new((self.0 & (u32::MAX as u64)) as usize)
    }

    fn new(parent: NodeRef, child: NodeRef) -> Self {
        let parent = parent.index() as u64;
        let child = child.index() as u64;
        assert!(parent < u32::MAX as u64);
        assert!(child < u32::MAX as u64);
        Self((parent << 32) | child)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EdgeRef {
    from: NodeRef,
    to: NodeRef,
}
impl petgraph::visit::EdgeRef for EdgeRef {
    type NodeId = NodeRef;
    type EdgeId = EdgeId;
    type Weight = ();

    #[inline]
    fn source(&self) -> Self::NodeId {
        self.from
    }
    #[inline]
    fn target(&self) -> Self::NodeId {
        self.to
    }
    #[inline]
    fn weight(&self) -> &Self::Weight {
        &()
    }
    #[inline]
    fn id(&self) -> Self::EdgeId {
        EdgeId::new(self.from, self.to)
    }
}

impl petgraph::visit::GraphBase for Document {
    type NodeId = NodeRef;
    type EdgeId = EdgeId;
}
impl petgraph::visit::GraphProp for Document {
    type EdgeType = petgraph::Directed;
}
impl petgraph::visit::Data for Document {
    type NodeWeight = Node;
    type EdgeWeight = ();
}
impl petgraph::visit::NodeCount for Document {
    fn node_count(&self) -> usize {
        // NOTE: This does not reflect the number of nodes that would be produced
        // by traversing the document, it is the total number of nodes created in
        // this document so far
        self.nodes.len()
    }
}
impl<'a> petgraph::visit::IntoNodeIdentifiers for &'a Document {
    type NodeIdentifiers = NodeIdentifiers<'a>;

    #[inline]
    fn node_identifiers(self) -> Self::NodeIdentifiers {
        NodeIdentifiers::new(self)
    }
}
impl<'a> petgraph::visit::IntoNodeReferences for &'a Document {
    type NodeRef = (NodeRef, &'a Node);
    type NodeReferences = NodeReferences<'a>;

    #[inline]
    fn node_references(self) -> Self::NodeReferences {
        NodeReferences::new(self)
    }
}
impl<'a> petgraph::visit::IntoEdgeReferences for &'a Document {
    type EdgeRef = EdgeRef;
    type EdgeReferences = EdgeReferences<'a>;

    #[inline]
    fn edge_references(self) -> Self::EdgeReferences {
        EdgeReferences::new(self)
    }
}
impl petgraph::visit::NodeIndexable for Document {
    #[inline]
    fn node_bound(&self) -> usize {
        self.nodes.len()
    }
    #[inline]
    fn to_index(&self, a: Self::NodeId) -> usize {
        a.index()
    }
    #[inline]
    fn from_index(&self, i: usize) -> Self::NodeId {
        NodeRef::new(i)
    }
}
impl petgraph::data::Build for Document {
    #[inline]
    fn add_node(&mut self, weight: Self::NodeWeight) -> Self::NodeId {
        self.nodes.push(weight)
    }

    fn update_edge(
        &mut self,
        a: Self::NodeId,
        b: Self::NodeId,
        _weight: Self::EdgeWeight,
    ) -> Self::EdgeId {
        self.parents[b] = a.into();
        EdgeId::new(a, b)
    }
}
impl petgraph::data::Create for Document {
    #[inline]
    fn with_capacity(nodes: usize, _edges: usize) -> Self {
        Self::with_capacity(nodes)
    }
}
impl petgraph::data::DataMap for Document {
    #[inline]
    fn node_weight(&self, id: Self::NodeId) -> Option<&Self::NodeWeight> {
        self.nodes.get(id)
    }

    fn edge_weight(&self, id: Self::EdgeId) -> Option<&Self::EdgeWeight> {
        let child = id.child();
        match self.parents[child].expand() {
            None => None,
            Some(_) => Some(&()),
        }
    }
}

impl petgraph::data::DataMapMut for Document {
    #[inline]
    fn node_weight_mut(&mut self, id: Self::NodeId) -> Option<&mut Self::NodeWeight> {
        Some(&mut self.nodes[id])
    }

    fn edge_weight_mut(&mut self, _id: Self::EdgeId) -> Option<&mut Self::EdgeWeight> {
        None
    }
}

/// An iterator over node ids in a Document which are attached to the tree
pub struct NodeIdentifiers<'a> {
    doc: &'a Document,
    stack: VecDeque<NodeRef>,
}
impl<'a> NodeIdentifiers<'a> {
    fn new(doc: &'a Document) -> Self {
        let mut stack = VecDeque::new();
        stack.push_back(doc.root);
        Self { doc, stack }
    }
}
impl<'a> Iterator for NodeIdentifiers<'a> {
    type Item = NodeRef;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.pop_front() {
            None => None,
            Some(next) => {
                let children = self.doc.children(next);
                self.stack.extend(children.iter().copied());
                Some(next)
            }
        }
    }
}
impl<'a> core::iter::FusedIterator for NodeIdentifiers<'a> {}

/// An iterator over node ids + data in a Document which are attached to the tree
pub struct NodeReferences<'a>(NodeIdentifiers<'a>);
impl<'a> NodeReferences<'a> {
    #[inline]
    fn new(doc: &'a Document) -> Self {
        Self(NodeIdentifiers::new(doc))
    }
}
impl<'a> Iterator for NodeReferences<'a> {
    type Item = (NodeRef, &'a Node);

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            None => None,
            Some(next) => Some((next, self.0.doc.get(next))),
        }
    }
}

/// An iterator over edges between nodes in a Document
pub struct EdgeReferences<'a> {
    doc: &'a Document,
    stack: VecDeque<(NodeRef, NodeRef)>,
}
impl<'a> EdgeReferences<'a> {
    fn new(doc: &'a Document) -> Self {
        let children = doc.children(doc.root);
        let mut stack = VecDeque::with_capacity(children.len());
        stack.extend(children.iter().copied().map(|child| (doc.root, child)));
        Self { doc, stack }
    }
}
impl<'a> Iterator for EdgeReferences<'a> {
    type Item = EdgeRef;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.pop_front() {
            None => None,
            Some((from, to)) => {
                let children = self.doc.children(to);
                self.stack
                    .extend(children.iter().copied().map(|child| (to, child)));
                Some(EdgeRef { from, to })
            }
        }
    }
}

pub struct Neighbors<'a> {
    skip: NodeRef,
    parent: Option<NodeRef>,
    children: Option<&'a [NodeRef]>,
}
impl<'a> Iterator for Neighbors<'a> {
    type Item = NodeRef;

    fn next(&mut self) -> Option<NodeRef> {
        // Visit outgoing first, then incoming
        if let Some(children) = self.children.as_mut() {
            while let Some(next) = children.take_first().copied() {
                // Skip the node for which we're iterating neighbors
                if next == self.skip {
                    continue;
                }
                return Some(next);
            }
            self.children = None;
        }

        self.parent.take()
    }
}

impl<'a> petgraph::visit::IntoNeighbors for &'a Document {
    type Neighbors = Neighbors<'a>;
    fn neighbors(self, n: Self::NodeId) -> Self::Neighbors {
        // This trait produces only outgoing edges for a directed graph
        Neighbors {
            skip: n,
            parent: None,
            children: Some(self.children[n].as_slice()),
        }
    }
}
impl<'a> petgraph::visit::IntoNeighborsDirected for &'a Document {
    type NeighborsDirected = Neighbors<'a>;
    fn neighbors_directed(self, n: Self::NodeId, d: Direction) -> Self::NeighborsDirected {
        match d {
            Direction::Incoming => Neighbors {
                skip: n,
                parent: self.parents[n].expand(),
                children: None,
            },
            Direction::Outgoing => Neighbors {
                skip: n,
                parent: None,
                children: Some(self.children[n].as_slice()),
            },
        }
    }
}
impl petgraph::visit::Visitable for Document {
    type Map = fixedbitset::FixedBitSet;

    fn visit_map(&self) -> Self::Map {
        FixedBitSet::with_capacity(self.nodes.len())
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
        map.grow(self.nodes.len())
    }
}
