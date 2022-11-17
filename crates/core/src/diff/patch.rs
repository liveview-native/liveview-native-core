use crate::dom::*;

use super::traversal::MoveTo;

#[derive(Debug, PartialEq)]
pub enum Patch {
    InsertBefore {
        before: NodeRef,
        node: Node,
    },
    InsertAfter {
        after: NodeRef,
        node: Node,
    },
    /// Creates `node` without attaching it to a parent
    /// It is expected to be pushed on the argument stack and popped off by subsequent ops
    Create {
        node: Node,
    },
    /// Same as `Create`, but also makes the new node the current node
    CreateAndMoveTo {
        node: Node,
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
    /// Appends `node` using the current node as parent
    ///
    /// This is used in conjunction with `Move` to construct a subtree
    /// without modifying a Document, which is necessary when generating diffs
    Append {
        node: Node,
    },
    /// Pops an argument off the stack and appends it as the immediate sibling of `after`
    AppendAfter {
        after: NodeRef,
    },
    /// Appends `node` to `parent`
    AppendTo {
        parent: NodeRef,
        node: Node,
    },
    Remove {
        node: NodeRef,
    },
    Replace {
        node: NodeRef,
        replacement: Node,
    },
    /// Adds `attr` to the current node
    AddAttribute {
        name: AttributeName,
        value: AttributeValue,
    },
    /// Adds `attr` to `node`
    AddAttributeTo {
        node: NodeRef,
        name: AttributeName,
        value: AttributeValue,
    },
    UpdateAttribute {
        node: NodeRef,
        name: AttributeName,
        value: AttributeValue,
    },
    RemoveAttributeByName {
        node: NodeRef,
        name: AttributeName,
    },
    Move(MoveTo),
}

impl Patch {
    /// Applies this patch to `doc` using `stack`.
    ///
    /// If this patch will result in a change to the underlying document, the affected [NodeRef] is returned, else `None`.
    ///
    /// For modifications, the affected node is the node to which the change applies, but for additions/removals, the affected
    /// node is the parent to which the child was added/removed. This can be used to associate callbacks to nodes in the document
    /// and be able to properly handle changes to them.
    pub fn apply<B>(self, doc: &mut B, stack: &mut Vec<NodeRef>) -> Option<NodeRef>
    where
        B: DocumentBuilder,
    {
        match self {
            Self::InsertBefore { before, node } => Some(doc.insert_before(node, before)),
            Self::InsertAfter { after, node } => Some(doc.insert_after(node, after)),
            Self::Create { node } => {
                let node = doc.push_node(node);
                stack.push(node);
                Some(node)
            }
            Self::CreateAndMoveTo { node } => {
                let node = doc.push_node(node);
                stack.push(node);
                doc.set_insertion_point(node);
                Some(node)
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
                Some(parent)
            }
            Self::Append { node } => Some(doc.append(node)),
            Self::AppendTo { parent, node } => {
                doc.append_child(parent, node);
                Some(parent)
            }
            Self::AppendAfter { after } => {
                let node = stack.pop().unwrap();
                let d = doc.document_mut();
                d.insert_after(node, after);
                d.parent(after)
            }
            Self::Remove { node } => {
                let parent = doc.document_mut().parent(node);
                doc.remove(node);
                parent
            }
            Self::Replace { node, replacement } => {
                doc.replace(node, replacement);
                Some(node)
            }
            Self::AddAttribute { name, value } => {
                doc.set_attribute(name, value);
                Some(doc.insertion_point())
            }
            Self::AddAttributeTo { node, name, value } => {
                let mut guard = doc.insert_guard();
                guard.set_insertion_point(node);
                guard.set_attribute(name, value);
                Some(node)
            }
            Self::UpdateAttribute { node, name, value } => {
                let mut guard = doc.insert_guard();
                guard.set_insertion_point(node);
                guard.set_attribute(name, value);
                Some(node)
            }
            Self::RemoveAttributeByName { node, name } => {
                let mut guard = doc.insert_guard();
                guard.set_insertion_point(node);
                guard.remove_attribute(name);
                Some(node)
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
