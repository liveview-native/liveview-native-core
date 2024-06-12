mod morph;
pub mod fragment;
mod patch;
mod traversal;

pub use morph::{diff, Morph};
pub use patch::{Patch, PatchResult};
pub use traversal::MoveTo;
