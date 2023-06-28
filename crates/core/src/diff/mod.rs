mod diff;
mod morph;
mod patch;
mod traversal;

pub use self::morph::diff;
pub use self::patch::{Patch, PatchResult};
pub use self::traversal::MoveTo;
