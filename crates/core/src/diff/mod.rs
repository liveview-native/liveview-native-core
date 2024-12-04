pub mod fragment;
mod morph;
mod patch;
mod traversal;

pub use morph::{diff, Morph};
pub use patch::{BeforePatch, Patch, PatchResult};
pub use traversal::MoveTo;
