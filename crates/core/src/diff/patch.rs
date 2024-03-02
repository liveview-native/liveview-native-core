use super::traversal::MoveTo;
use crate::dom::*;

#[derive(Debug, PartialEq, Clone)]
pub enum Patch {
    InsertBefore {
        before: NodeRef,
        node: NodeData,
    },
    InsertAfter {
        after: NodeRef,
        node: NodeData,
    },
    /// Creates `node` without attaching it to a parent
    /// It is expected to be pushed on the argument stack and popped off by subsequent ops
    Create {
        node: NodeData,
    },
    /// Same as `Create`, but also makes the new node the current node
    CreateAndMoveTo {
        node: NodeData,
    },
    /// Pushes the currently selected NodeRef on the stack, intended for use in conjunction with other stack-based ops
    PushCurrent,
    /// Pushes a NodeRef on the stack, intended for use in conjunction with other stack-based ops
    Push(NodeRef),
    /// Pops a NodeRef from the stack, discarding it
    Pop,
    /// Pops two arguments off the stack; the first is the child to be attached, the second is the parent
    ///
    /// The second argument is pushed back on the stack when done, reducing the stack overall by one
    Attach,
    /// Detach node so that it can be re-attached later using Push and Attach or AppendAfter
    Detach {
        node: NodeRef,
    },
    /// Pops node off the stack and prepends relative to before
    PrependBefore {
        before: NodeRef,
    },
    /// Appends `node` using the current node as parent
    ///
    /// This is used in conjunction with `Move` to construct a subtree
    /// without modifying a Document, which is necessary when generating diffs
    Append {
        node: NodeData,
    },
    /// Pops an argument off the stack and appends it as the next sibling of `after`
    AppendAfter {
        after: NodeRef,
    },
    /// Appends `node` to `parent`
    AppendTo {
        parent: NodeRef,
        node: NodeData,
    },
    Remove {
        node: NodeRef,
    },
    Replace {
        node: NodeRef,
        replacement: NodeData,
    },
    /// Adds `attr` to the current node
    AddAttribute {
        name: AttributeName,
        value: Option<String>,
    },
    /// Adds `attr` to `node`
    AddAttributeTo {
        node: NodeRef,
        name: AttributeName,
        value: Option<String>,
    },
    UpdateAttribute {
        node: NodeRef,
        name: AttributeName,
        value: Option<String>,
    },
    RemoveAttributeByName {
        node: NodeRef,
        name: AttributeName,
    },
    /// Set attributes on node
    SetAttributes {
        node: NodeRef,
        attributes: Vec<Attribute>,
    },
    Move(MoveTo),
}

/// The result of applying a [Patch].
#[derive(Debug)]
pub enum PatchResult {
    /// The `node` has been added to the document as a child of `parent`.
    Add { node: NodeRef, parent: NodeRef },
    /// The `node` has been removed from the document, having formerly been a child of `parent`.
    Remove { node: NodeRef, parent: NodeRef },
    /// The `node` has been changed in some other way.
    Change { node: NodeRef },
    /// The `node` has been replaced
    Replace { node: NodeRef, parent: NodeRef },
}

impl Patch {
    /// Applies this patch to `doc` using `stack`.
    ///
    /// If this patch will result in a change to the underlying document, a [PatchResult]
    /// describing the change is returned, else `None`.
    pub fn apply<B>(self, doc: &mut B, stack: &mut Vec<NodeRef>) -> Option<PatchResult>
    where
        B: DocumentBuilder,
    {
        match self {
            Self::InsertBefore { before, node } => {
                let node = doc.insert_before(node, before);
                let parent = doc
                    .document()
                    .parent(node)
                    .expect("inserted node should have parent");
                Some(PatchResult::Add { node, parent })
            }
            Self::InsertAfter { after, node } => {
                let node = doc.insert_after(node, after);
                let parent = doc
                    .document()
                    .parent(node)
                    .expect("inserted node should have parent");
                Some(PatchResult::Add { node, parent })
            }
            Self::Create { node } => {
                let node = doc.push_node(node);
                stack.push(node);
                // another op will actually parent it, which will generate a PatchResult::Add
                None
            }
            Self::CreateAndMoveTo { node } => {
                let node = doc.push_node(node);
                stack.push(node);
                doc.set_insertion_point(node);
                None
            }
            Self::PushCurrent => {
                stack.push(doc.insertion_point());
                None
            }
            Self::Push(node) => {
                stack.push(node);
                None
            }
            Self::Pop => {
                stack.pop().unwrap();
                None
            }
            Self::Attach => {
                let child = stack.pop().unwrap();
                let parent = stack.pop().unwrap();
                doc.set_insertion_point(parent);
                doc.attach_node(child);
                stack.push(parent);
                Some(PatchResult::Add {
                    node: child,
                    parent,
                })
            }
            Self::Detach { node } => {
                doc.detach_node(node);
                None
            }
            Self::PrependBefore { before } => {
                let node = stack.pop().unwrap();
                let d = doc.document_mut();
                d.insert_before(node, before);
                let parent = d.parent(before).expect("inserted node should have parent");
                Some(PatchResult::Add { node, parent })
            }
            Self::Append { node } => {
                let node = doc.append(node);
                Some(PatchResult::Add {
                    node,
                    parent: doc.insertion_point(),
                })
            }
            Self::AppendTo { parent, node } => {
                let node = doc.append_child(parent, node);
                Some(PatchResult::Add { node, parent })
            }
            Self::AppendAfter { after } => {
                let node = stack.pop().unwrap();
                let d = doc.document_mut();
                d.insert_after(node, after);
                let parent = d.parent(after).expect("inserted node should have parent");
                Some(PatchResult::Add { node, parent })
            }
            Self::Remove { node } => {
                let parent = doc.document_mut().parent(node);
                doc.remove(node);
                parent.map(|parent| PatchResult::Remove { node, parent })
            }
            Self::Replace { node, replacement } => {
                let parent = doc.document_mut().parent(node)?;
                doc.replace(node, replacement);
                Some(PatchResult::Replace { node, parent })
            }
            Self::AddAttribute { name, value } => {
                doc.set_attribute(name, value);
                Some(PatchResult::Change {
                    node: doc.insertion_point(),
                })
            }
            Self::AddAttributeTo { node, name, value } => {
                let mut guard = doc.insert_guard();
                guard.set_insertion_point(node);
                guard.set_attribute(name, value);
                Some(PatchResult::Change { node })
            }
            Self::UpdateAttribute { node, name, value } => {
                let mut guard = doc.insert_guard();
                guard.set_insertion_point(node);
                guard.set_attribute(name, value);
                Some(PatchResult::Change { node })
            }
            Self::RemoveAttributeByName { node, name } => {
                let mut guard = doc.insert_guard();
                guard.set_insertion_point(node);
                guard.remove_attribute(name);
                Some(PatchResult::Change { node })
            }
            Self::SetAttributes { node, attributes } => {
                let mut guard = doc.insert_guard();
                guard.set_insertion_point(node);
                guard.replace_attributes(attributes);
                Some(PatchResult::Change { node })
            }
            Self::Move(MoveTo::Node(node)) => {
                doc.set_insertion_point(node);
                None
            }
            Self::Move(MoveTo::Parent) => {
                doc.set_insertion_point_to_parent();
                None
            }
            Self::Move(MoveTo::Child(n)) => {
                doc.set_insertion_point_to_child(n as usize);
                None
            }
            Self::Move(MoveTo::ReverseChild(n)) => {
                doc.set_insertion_point_to_child_reverse(n as usize);
                None
            }
            Self::Move(MoveTo::Sibling(n)) => {
                doc.set_insertion_point_to_sibling(n as usize);
                None
            }
            Self::Move(MoveTo::ReverseSibling(n)) => {
                doc.set_insertion_point_to_sibling_reverse(n as usize);
                None
            }
        }
    }
}
