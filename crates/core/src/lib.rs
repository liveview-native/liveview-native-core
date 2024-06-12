#![feature(slice_take)]
#![feature(assert_matches)]
#![feature(sync_unsafe_cell)]

pub mod diff;
pub mod dom;
pub mod parser;

#[cfg(feature = "liveview-channels")]
pub mod live_socket;

mod interner;
pub use self::interner::{symbols, InternedString, Symbol};

#[cfg(feature = "liveview-channels")]
phoenix_channels_client::uniffi_reexport_scaffolding!();

uniffi::setup_scaffolding!("liveview_native_core");
