#![feature(allocator_api)]
#![feature(slice_take)]
#![feature(assert_matches)]
#![feature(exact_size_is_empty)]
#![feature(vec_into_raw_parts)]
#![feature(c_unwind)]


pub mod diff;
pub mod dom;
pub mod parser;
pub mod live_socket;

mod interner;
pub use self::interner::{symbols, InternedString, Symbol};

phoenix_channels_client::uniffi_reexport_scaffolding!();
uniffi::setup_scaffolding!("liveview_native_core");
