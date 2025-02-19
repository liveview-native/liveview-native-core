#[cfg(feature = "liveview-channels")]
pub mod client;

#[cfg(feature = "liveview-channels")]
pub use client::inner::LiveViewClientInner as LiveViewClient;
#[cfg(feature = "liveview-channels")]
pub use client::LiveViewClientBuilder;

mod callbacks;
pub mod diff;
pub mod dom;
mod protocol;

mod error;

mod interner;
pub use self::interner::{symbols, InternedString, Symbol};

#[cfg(feature = "liveview-channels")]
phoenix_channels_client::uniffi_reexport_scaffolding!();

uniffi::setup_scaffolding!("liveview_native_core");
