use crate::dom::{Document, NodeRef};

pub const PHX_REF_LOCK: &str = "data-phx-ref";
pub const PHX_REF_SRC: &str = "data-phx-ref-src";

/// Speculative action taken before a patch
#[derive(Debug)]
pub enum BeforePatch {
    /// A new node would be added to `parent`.
    WouldAdd { parent: NodeRef },
    /// The node will be removed from `parent`
    WouldRemove { node: NodeRef },
    /// The node will be modified
    WouldChange { node: NodeRef },
    /// The node will be replaced
    WouldReplace { node: NodeRef },
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
            BeforePatch::WouldAdd { parent } => !doc.get(*parent).has_attribute(PHX_REF_LOCK, None),
            BeforePatch::WouldChange { node } => !doc.get(*node).has_attribute(PHX_REF_LOCK, None),
            BeforePatch::WouldRemove { node } => !doc.get(*node).has_attribute(PHX_REF_LOCK, None),
            BeforePatch::WouldReplace { node } => !doc.get(*node).has_attribute(PHX_REF_LOCK, None),
        }
    }
}
