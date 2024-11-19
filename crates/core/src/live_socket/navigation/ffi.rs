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
    Back,
    Forward,
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

#[derive(uniffi::Enum, Default, Clone)]
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
    pub fn new(
        event: NavEventType,
        to: NavHistoryEntry,
        from: Option<NavHistoryEntry>,
        info: Option<Vec<u8>>,
        state: Option<Vec<u8>>,
    ) -> Self {
        let new_url = Url::parse(&to.url).ok();
        let old_url = from.as_ref().and_then(|dest| Url::parse(&dest.url).ok());

        let same_document = old_url
            .zip(new_url)
            .is_some_and(|(old, new)| old.path() == new.path());

        NavEvent {
            event,
            same_document,
            from,
            to,
            info,
            state,
        }
    }

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

        NavEvent::new(event, new_dest, old_dest, opts.extra_event_info, opts.state)
    }

    /// Create a new nav event from the details of a [NavCtx::back] event,
    /// passing info into the event handler closure.
    pub fn new_from_back(
        new_dest: NavHistoryEntry,
        old_dest: NavHistoryEntry,
        info: Option<Vec<u8>>,
    ) -> NavEvent {
        NavEvent::new(NavEventType::Back, new_dest, Some(old_dest), info, None)
    }
}
