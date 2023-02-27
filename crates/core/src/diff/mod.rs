mod diff;
mod patch;
mod traversal;

pub use self::diff::diff;
pub use self::patch::{Patch, PatchResult};
pub use self::traversal::MoveTo;
