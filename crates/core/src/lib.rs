#![feature(allocator_api)]
#![feature(slice_take)]
#![feature(assert_matches)]
#![feature(exact_size_is_empty)]
#![feature(vec_into_raw_parts)]
#![feature(c_unwind)]

pub mod diff;
pub mod dom;
pub mod ffi;
mod interner;
pub mod parser;

pub use self::interner::{symbols, InternedString, Symbol};

uniffi::include_scaffolding!("lib");

use crate::ffi::{
    ChangeType,
    NodeType,
};

use crate::dom::{
    DocumentChangeHandler,
    FFiDocument as Document,
    NodeRef,
    Attribute,
    Element,
    Node,
};

use crate::parser::ParseError;
use crate::diff::fragment::RenderError;

