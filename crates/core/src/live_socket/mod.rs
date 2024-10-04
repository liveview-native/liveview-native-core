mod channel;
mod error;
mod socket;

#[cfg(test)]
mod tests;

pub use channel::LiveChannel;
use error::{LiveSocketError, UploadError};
pub use socket::LiveSocket;

#[derive(uniffi::Object)]
pub struct LiveFile {
    contents: Vec<u8>,
    file_type: String,
    name: String,
    phx_id: String,
}
#[uniffi::export]
impl LiveFile {
    #[uniffi::constructor]
    pub fn new(contents: Vec<u8>, file_type: String, name: String, phx_id: String) -> Self {
        Self {
            contents,
            file_type,
            name,
            phx_id,
        }
    }
}
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
