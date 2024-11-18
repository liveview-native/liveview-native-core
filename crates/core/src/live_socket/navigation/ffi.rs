use reqwest::Url;

pub type HistoryId = u64;

#[uniffi::export(callback_interface)]
pub trait NavEventHandler: Send + Sync {
    /// This callback instruments events that occur when your user navigates to a
    /// new view. You can add serialized metadata to these events as a byte buffer
    /// through the [NavOptions] object.
    fn handle_event(&self, event: NavEvent) -> HandlerResponse;
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq, Default)]
pub enum HandlerResponse {
    #[default]
    Default,
    PreventDefault,
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq)]
pub enum NavEventType {
    Push,
    Replace,
    Reload,
    Traverse,
}

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct NavHistoryEntry {
    /// The target url.
    pub url: String,
    /// Unique id for this piece of nav entry state.
    pub id: HistoryId,
    /// state passed in by the user, to be passed in to the navigation event callback.
    pub state: Option<Vec<u8>>,
}

/// An event emitted when the user navigates between views.
#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct NavEvent {
    pub event: NavEventType,
    pub same_document: bool,
    /// The previous location of the page, if there was one
    pub from: Option<NavHistoryEntry>,
    /// Destination URL
    pub to: NavHistoryEntry,
    /// Additional user provided metadata handed to the event handler.
    pub info: Option<Vec<u8>>,
    /// Persistent state kept in history for as long as the even is in the history stack.
    pub state: Option<Vec<u8>>,
}

#[derive(Default, uniffi::Enum)]
pub enum NavAction {
    /// Push the navigation event onto the history stack.
    #[default]
    Push,
    /// Replace the current top of the history stack with this navigation event.
    Replace,
}

#[derive(Default, uniffi::Record)]
pub struct NavOptions {
    pub action: NavAction,
    pub extra_event_info: Option<Vec<u8>>,
    pub state: Option<Vec<u8>>,
}

impl NavEvent {
    /// Create a new nav event from the details of a [NavCtx::navigate] event
    pub fn new_from_navigate(
        new_dest: NavHistoryEntry,
        old_dest: Option<NavHistoryEntry>,
        opts: NavOptions,
    ) -> NavEvent {
        let event = match opts.action {
            NavAction::Push => NavEventType::Push,
            NavAction::Replace => NavEventType::Replace,
        };

        let new_url = Url::parse(&new_dest.url).ok();
        let old_url = old_dest
            .as_ref()
            .and_then(|dest| Url::parse(&dest.url).ok());

        let same_document = if let (Some(old_url), Some(new_url)) = (old_url, new_url) {
            old_url.path() == new_url.path()
        } else {
            false
        };

        NavEvent {
            event,
            same_document,
            from: old_dest,
            to: new_dest,
            info: opts.extra_event_info,
            state: opts.state,
        }
    }
}
