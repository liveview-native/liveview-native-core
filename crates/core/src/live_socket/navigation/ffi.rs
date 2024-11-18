type HistoryId = u64;

#[uniffi::export(callback_interface)]
pub trait NavEventHandler: Send + Sync {
    /// This callback instruments events that occur when your user navigates to a
    /// new view. You can add serialized metadata to these events as a byte buffer
    /// through the [NavOptions] object.
    fn handle_event(&self, event: NavEvent);
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq)]
pub enum NavEventType {
    Push,
    Replace,
    Reload,
    Traverse,
}

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct NavDestination {
    /// A monotonically increasing unique lookup ID.
    pub id: HistoryId,
    pub url: String,
}

/// An event emitted when the user navigates between views.
#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct NavEvent {
    pub event: NavEventType,
    pub same_document: bool,
    /// The previous location of the page, if there was one
    pub from: Option<NavDestination>,
    /// Destination URL
    pub to: NavDestination,
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
    action: NavAction,
    extra_event_info: Option<Vec<u8>>,
    state: Option<Vec<u8>>,
}
