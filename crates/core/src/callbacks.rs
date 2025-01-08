/// Provides secure persistent storage for session data like cookies.
/// Implementations should handle platform-specific storage (e.g. NSUserDefaults on iOS)
/// and ensure data is stored securely as some of it may be session tokens.
#[uniffi::export(callback_interface)]
pub trait SecurePersistentStore: Send + Sync {
    /// Removes the entry for the given key
    fn remove_entry(&self, key: String);

    /// Gets the value for the given key, or None if not found
    fn get(&self, key: String) -> Option<Vec<u8>>;

    /// Sets the value for the given key
    fn set(&self, key: String, value: Vec<u8>);
}

#[uniffi::export(callback_interface)]
pub trait NavEventHandler: Send + Sync {
    /// This callback instruments events that occur when your user navigates to a
    /// new view. You can add serialized metadata to these events as a byte buffer
    /// through the [NavOptions] object.
    fn handle_event(&self, event: NavEvent) -> HandlerResponse;
}

/// Unique id in the history stack
pub type HistoryId = u64;

/// User emitted response from [NavEventHandler::handle_event].
/// Determines whether or not the default navigation action is taken.
#[derive(uniffi::Enum, Clone, Debug, PartialEq, Default)]
pub enum HandlerResponse {
    #[default]
    /// Return this to proceed as normal.
    Default,
    /// Return this to cancel the navigation before it occurs.
    PreventDefault,
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq)]
pub enum NavEventType {
    /// Pushing a new event onto the history stack
    Push,
    /// Replacing the most recent event on the history stack
    Replace,
    /// Reloading the view in place
    Reload,
    /// Skipping multiple items on the history stack, leaving them in tact.
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

impl NavHistoryEntry {
    /// Create a new navigation history entry
    pub fn new(url: String, id: HistoryId, state: Option<Vec<u8>>) -> Self {
        Self { url, id, state }
    }
}

/// An event emitted when the user navigates between views.
#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct NavEvent {
    /// The type of event being emitted.
    pub event: NavEventType,
    /// True if from and to point to the same path.
    pub same_document: bool,
    /// The previous location of the page, if there was one.
    pub from: Option<NavHistoryEntry>,
    /// Destination URL.
    pub to: NavHistoryEntry,
    /// Additional user provided metadata handed to the event handler.
    pub info: Option<Vec<u8>>,
}
