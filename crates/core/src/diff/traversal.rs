use crate::dom::NodeRef;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveTo {
    Node(NodeRef),

    /// Move from the current node up to its parent.
    Parent,

    /// Move to the current node's n^th child.
    Child(u32),

    /// Move to the current node's n^th from last child.
    ReverseChild(u32),

    /// Move to the n^th sibling. Not relative from the current
    /// location. Absolute indexed within all of the current siblings.
    Sibling(u32),

    /// Move to the n^th from last sibling. Not relative from the current
    /// location. Absolute indexed within all of the current siblings.
    ReverseSibling(u32),
}
