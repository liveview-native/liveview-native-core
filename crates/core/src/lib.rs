#![feature(slice_take)]
#![feature(assert_matches)]
#![feature(once_cell)]
#![feature(map_first_last)]
#![feature(exact_size_is_empty)]

pub mod diff;
pub mod dom;
mod interner;
pub mod parser;

pub use self::interner::{symbols, InternedString, Symbol};
