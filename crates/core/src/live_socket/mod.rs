mod channel;
pub mod navigation;
mod socket;

pub use channel::{LiveChannel, LiveFile};
pub use socket::{ConnectOpts, LiveSocket, Method, SessionData};

pub struct UploadConfig {
    chunk_size: u64,
    max_file_size: u64,
    max_entries: u64,
}

/// Defaults from https://hexdocs.pm/phoenix_live_view/Phoenix.LiveView.html#allow_upload/3
impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64_000,
            max_file_size: 8000000,
            max_entries: 1,
        }
    }
}
