use color_eyre::Result;
use liveview_native_core::callbacks::{Issuer, LiveChannelStatus, NetworkEventHandler};
use liveview_native_core::client::config::{LiveViewClientConfiguration, Platform};
use liveview_native_core::dom::ffi::Document;
use liveview_native_core::live_socket::LiveChannel;
use liveview_native_core::LiveViewClient;
use phoenix_channels_client::{Socket, SocketStatus};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Position},
    style::{Color, Style},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    DefaultTerminal, Frame,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(target_os = "android")]
const HOST: &str = "10.0.2.2:4001";

#[cfg(not(target_os = "android"))]
const HOST: &str = "127.0.0.1:4001";

enum View {
    Client,
    Network,
}

enum InputMode {
    Normal,
    Insert,
}

enum CursorDir {
    Left,
    Right,
}

struct Store {
    message_store: Mutex<Vec<String>>,
}

impl Store {
    fn new() -> Self {
        Self {
            message_store: Mutex::new(Vec::new()),
        }
    }

    fn push(&self, message: String) {
        let mut store = self.message_store.lock().unwrap();
        store.push(message);
    }
}

impl NetworkEventHandler for Store {
    fn handle_event(&self, _: phoenix_channels_client::EventPayload) {
        self.push("event".to_string());
    }

    fn handle_channel_status_change(&self, _: LiveChannelStatus) {
        self.push("status change".to_string());
    }

    fn handle_socket_status_change(&self, _: SocketStatus) {
        self.push("socket status change".to_string());
    }

    fn handle_view_reloaded(
        &self,
        _: Issuer,
        _: Arc<Document>,
        _: Arc<LiveChannel>,
        _: Arc<Socket>,
        _: bool,
    ) {
        self.push("reload".to_string());
    }
}

struct State {
    input: String,
    scroll: usize,
    cursor: usize,
    view: View,
    mode: InputMode,
    scrollbar_state: ScrollbarState,
    store: Arc<Store>,
    client: LiveViewClient,
}

impl State {
    fn new(client: LiveViewClient, store: Arc<Store>) -> Self {
        Self {
            input: String::new(),
            scroll: 0,
            cursor: 0,
            view: View::Client,
            mode: InputMode::Normal,
            scrollbar_state: Default::default(),
            store,
            client,
        }
    }

    fn move_cursor(&mut self, dir: CursorDir) {
        let new_pos = match dir {
            CursorDir::Left => self.cursor.saturating_sub(1),
            CursorDir::Right => self.cursor.saturating_add(1),
        };

        self.cursor = new_pos.clamp(0, self.input.chars().count());
    }

    async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();

        loop {
            terminal.draw(|frame| self.draw(frame))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match self.mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('1') => {
                                self.view = View::Client;
                                self.scroll = 0;
                            }
                            KeyCode::Char('2') => {
                                self.view = View::Network;
                                self.scroll = 0;
                            }
                            KeyCode::Char('e') => self.mode = InputMode::Insert,
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Up | KeyCode::PageUp => {
                                self.scroll = self.scroll.saturating_sub(1);
                                self.scrollbar_state = self.scrollbar_state.position(self.scroll)
                            }
                            KeyCode::Down | KeyCode::PageDown => {
                                self.scroll = self.scroll.saturating_add(1);
                                self.scrollbar_state = self.scrollbar_state.position(self.scroll)
                            }
                            _ => {}
                        },
                        InputMode::Insert if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Enter => {
                                let ch = self.client.channel().unwrap().channel();
                                let input = self.input.clone();
                                tokio::spawn(async move {
                                    ch.call(
                                        phoenix_channels_client::Event::from_string("event".into()),
                                        phoenix_channels_client::Payload::JSONPayload {
                                            json: serde_json::json!(input).into(),
                                        },
                                        Duration::new(5, 0),
                                    )
                                    .await
                                });
                                self.input.clear();
                                self.cursor = 0;
                            }
                            KeyCode::Char(c) => {
                                self.input.insert(
                                    self.input
                                        .char_indices()
                                        .map(|(i, _)| i)
                                        .nth(self.cursor)
                                        .unwrap_or(self.input.len()),
                                    c,
                                );
                                self.move_cursor(CursorDir::Right);
                            }
                            KeyCode::Backspace => {
                                if self.cursor != 0 {
                                    let pos = self.cursor;
                                    let (pre, post) = (
                                        self.input.chars().take(pos - 1),
                                        self.input.chars().skip(pos),
                                    );

                                    self.input = pre.chain(post).collect();
                                    self.move_cursor(CursorDir::Left);
                                }
                            }
                            KeyCode::Left => self.move_cursor(CursorDir::Left),
                            KeyCode::Right => self.move_cursor(CursorDir::Right),
                            KeyCode::Esc => self.mode = InputMode::Normal,
                            _ => {}
                        },
                        InputMode::Insert => {}
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]);

        let [info_area, input_area] = vertical.areas(frame.area());

        match self.view {
            View::Client => {
                let info_text = format!("{:#?}", self.client);

                let info = Paragraph::new(info_text.as_str())
                    .scroll((self.scroll as u16, 0))
                    .block(Block::bordered());

                self.scrollbar_state = self
                    .scrollbar_state
                    .content_length(info_text.as_bytes().iter().filter(|&&c| c == b'\n').count());
                frame.render_widget(info, info_area);
            }
            View::Network => {
                let store = self.store.message_store.lock().unwrap();

                let info = Paragraph::new(ratatui::text::Text::from_iter(store.clone()))
                    .scroll((self.scroll as u16, 0))
                    .block(Block::bordered());
                self.scrollbar_state = self.scrollbar_state.content_length(store.len());
                frame.render_widget(info, info_area);
            }
        }

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            info_area,
            &mut self.scrollbar_state,
        );

        match self.mode {
            InputMode::Normal => {}
            InputMode::Insert => frame.set_cursor_position(Position::new(
                input_area.x + self.cursor as u16 + 1,
                input_area.y + 1,
            )),
        }

        let input = Paragraph::new(self.input.as_str())
            .style(match self.mode {
                InputMode::Normal => Style::default(),
                InputMode::Insert => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered());
        frame.render_widget(input, input_area);
    }
}

#[tokio::main]
async fn main() {
    let _ = env_logger::builder().parse_default_env().try_init();

    let store = Arc::new(Store::new());
    let url = format!("http://{HOST}/stream");

    let config = LiveViewClientConfiguration {
        format: Platform::Swiftui,
        network_event_handler: Some(store.clone()),
        ..Default::default()
    };

    let client = LiveViewClient::initial_connect(config, url, Default::default())
        .await
        .expect("Failed to create client");

    color_eyre::install().expect("Failed to initialize colors");

    State::new(client, store)
        .run(ratatui::init())
        .await
        .expect("Failure in terminal context");
    ratatui::restore();
}
