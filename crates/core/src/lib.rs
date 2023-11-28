#![feature(allocator_api)]
#![feature(slice_take)]
#![feature(assert_matches)]
#![feature(exact_size_is_empty)]
#![feature(vec_into_raw_parts)]
#![feature(c_unwind)]

pub mod diff;
pub mod dom;
pub mod parser;

mod interner;
pub use self::interner::{symbols, InternedString, Symbol};

uniffi::include_scaffolding!("uniffi");

use crate::dom::{
    DocumentChangeHandler,
    ChangeType,
    FFiDocument as Document,
    NodeRef,
    Attribute,
    AttributeName,
    Element,
    ElementName,
    Node,
};

use crate::parser::ParseError;
use crate::diff::fragment::RenderError;
