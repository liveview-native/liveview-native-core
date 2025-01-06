use std::sync::Arc;

use log::debug;
use phoenix_channels_client::Socket;
use reqwest::{cookie::CookieStore, header::SET_COOKIE, redirect::Policy, Client, Url};

use super::LiveViewClientConfiguration;
use crate::{
    client::{cookie_store::PersistentCookieStore, logging::init_log},
    live_socket::{
        navigation::{NavCtx, NavOptions},
        LiveChannel, LiveSocketError, SessionData,
    },
};

pub struct LiveViewClientInner {
    /// Manages navigation state, events
    config: LiveViewClientConfiguration,
    nav_ctx: NavCtx,
    http_client: Client,
    socket: Arc<Socket>,
    liveview_channel: LiveChannel,
    livereload_channel: Option<LiveChannel>,
    session_state: SessionData,
}

impl LiveViewClientInner {
    pub async fn initial_connect(
        config: LiveViewClientConfiguration,
        url: String,
    ) -> Result<Self, LiveSocketError> {
        init_log(config.log_level);
        debug!("Initializing LiveViewClient.");
        debug!("Configuration: {config:?}");

        let url = Url::parse(&url)?;
        let format = config.format.to_string();

        let session_data = SessionData::request(&url, &format, Default::default()).await?;
        let websocket_url = session_data.get_live_socket_url()?;

        let socket = Socket::spawn(websocket_url, Some(session_data.cookies.clone())).await?;

        let store = PersistentCookieStore::new(config.persistence_provider.clone());

        let mut nav_ctx = NavCtx::default();
        nav_ctx.navigate(url.clone(), NavOptions::default(), false);

        let http_client = Client::builder()
            .cookie_provider(Arc::new(store))
            .redirect(Policy::none())
            .build()
            .expect("Failed to build HTTP client");

        Ok(Self {
            config,
            http_client,
            nav_ctx,
            socket,
            liveview_channel: todo!(),
            livereload_channel: todo!(),
            session_state: todo!(),
        })
    }
}
