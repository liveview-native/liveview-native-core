use std::sync::Arc;
use std::time::Duration;
use url::Url;

use phoenix_channels_client::{ConnectError, Socket, SpawnError};

pub struct LiveSocket {
    pub socket: Arc<Socket>
}
impl LiveSocket {
    pub async fn spawn(url: Url) -> Result<Self, SpawnError> {
        Ok(Self {
            socket: Socket::spawn(url).await?
        })
    }

    pub async fn connect(&self, timeout: Duration) -> Result<(), ConnectError> {
        self.socket.connect(timeout).await
    }
}
