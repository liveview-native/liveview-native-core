use crate::dom::{Document, NodeRef};

pub const PHX_REF_LOCK: &str = "data-phx-ref";
pub const PHX_REF_SRC: &str = "data-phx-ref-src";

/// Speculative action taken before a patch
#[derive(Debug)]
pub enum BeforePatch {
    /// A new node would be added to `parent`.
    Add { parent: NodeRef },
    /// The node will be removed from `parent`
    Remove { node: NodeRef },
    /// The node will be modified
    Change { node: NodeRef },
    /// The node will be replaced
    Replace { node: NodeRef },
}

/// Applications specific dom morphing hooks.
/// These functions implement application specific
#[derive(Debug, Clone, Default)]
pub struct PhxDocumentChangeHooks;

impl PhxDocumentChangeHooks {
    /// If a patch result would touch a part of the locked tree, return true.
    /// These changes are not kept in the DOM but instead in the root fragment.
    pub fn can_complete_change(&self, doc: &Document, patch: &BeforePatch) -> bool {
        match patch {
            BeforePatch::Add { parent } => !doc.get(*parent).has_attribute(PHX_REF_LOCK, None),
            BeforePatch::Change { node } => !doc.get(*node).has_attribute(PHX_REF_LOCK, None),
            BeforePatch::Remove { node } => !doc.get(*node).has_attribute(PHX_REF_LOCK, None),
            BeforePatch::Replace { node } => !doc.get(*node).has_attribute(PHX_REF_LOCK, None),
        }
    }
}
