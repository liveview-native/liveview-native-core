use phoenix_channels_client::Socket;
use reqwest::Client;

use crate::live_socket::{navigation::NavCtx, LiveChannel};

pub enum ClientState {
    Connected {
        socket: Socket,
        liveview_channel: LiveChannel,
        livereload_channel: Option<LiveChannel>,
    },
    Disconnected,
}

pub struct LiveViewClientInner {
    http_client: Client,
    nav_ctx: NavCtx,
    state: ClientState,
}

impl LiveViewClientInner {
    pub fn new() {
        todo!()
    }

    pub fn connect() {
        todo!()
    }
}
