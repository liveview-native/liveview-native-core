mod diff;
mod patch;
mod traversal;

pub use diff::{diff, Morph};
pub use patch::{Patch, PatchResult};
pub use traversal::MoveTo;
