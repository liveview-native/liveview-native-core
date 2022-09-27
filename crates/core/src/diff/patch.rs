use crate::dom::*;
use crate::InternedString;

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
        attr: Attribute,
    },
    /// Adds `attr` to `node`
    AddAttributeTo {
        node: NodeRef,
        attr: Attribute,
    },
    UpdateAttribute {
        node: NodeRef,
        name: InternedString,
        value: AttributeValue,
    },
    RemoveAttributeByName {
        node: NodeRef,
        namespace: Option<InternedString>,
        name: InternedString,
    },
    Move(MoveTo),
}

impl Patch {
    pub fn is_mutation(&self) -> bool {
        match self {
            Self::Move(_) => false,
            _ => true,
        }
    }

    pub fn apply<B>(self, doc: &mut B, stack: &mut Vec<NodeRef>)
    where
        B: DocumentBuilder,
    {
        match self {
            Self::InsertBefore { before, node } => {
                doc.insert_before(node, before);
            }
            Self::InsertAfter { after, node } => {
                doc.insert_after(node, after);
            }
            Self::Create { node } => {
                stack.push(doc.push_node(node));
            }
            Self::CreateAndMoveTo { node } => {
                let node = doc.push_node(node);
                stack.push(node);
                doc.set_insertion_point(node);
            }
            Self::Push(node) => {
                stack.push(node);
            }
            Self::Pop => {
                stack.pop().unwrap();
            }
            Self::Attach => {
                let child = stack.pop().unwrap();
                let parent = stack.pop().unwrap();
                doc.set_insertion_point(parent);
                doc.attach_node(child);
                stack.push(parent);
            }
            Self::Append { node } => {
                doc.append(node);
            }
            Self::AppendTo { parent, node } => {
                doc.append_child(parent, node);
            }
            Self::Remove { node } => {
                doc.remove(node);
            }
            Self::Replace { node, replacement } => {
                doc.replace(node, replacement)
            }
            Self::AddAttribute { attr } => {
                doc.append_attribute(attr);
            }
            Self::AddAttributeTo { node, attr } => {
                let mut guard = doc.insert_guard();
                guard.set_insertion_point(node);
                guard.append_attribute(attr);
            }
            Self::UpdateAttribute { node, name, value } => {
                let mut guard = doc.insert_guard();
                guard.set_insertion_point(node);
                guard.update_attribute(name, value);
            }
            Self::RemoveAttributeByName {
                node,
                namespace,
                name,
            } => {
                let mut guard = doc.insert_guard();
                guard.set_insertion_point(node);
                guard.remove_attribute_by_full_name(namespace, name);
            }
            Self::Move(MoveTo::Node(node)) => {
                doc.set_insertion_point(node);
            }
            Self::Move(MoveTo::Parent) => {
                doc.set_insertion_point_to_parent();
            }
            Self::Move(MoveTo::Child(n)) => {
                doc.set_insertion_point_to_child(n as usize);
            }
            Self::Move(MoveTo::ReverseChild(n)) => {
                doc.set_insertion_point_to_child_reverse(n as usize);
            }
            Self::Move(MoveTo::Sibling(n)) => {
                doc.set_insertion_point_to_sibling(n as usize);
            }
            Self::Move(MoveTo::ReverseSibling(n)) => {
                doc.set_insertion_point_to_sibling_reverse(n as usize);
            }
        }
    }
}
