use std::{sync::Arc, time::Duration};


#[cfg(feature = "liveview-channels")]
use crate::dom::ffi::Document;
use crate::{
    dom::{NodeData, NodeRef},
    error::LiveSocketError,
};

#[uniffi::export(callback_interface)]
pub trait SocketReconnectStrategy: Send + Sync {
    fn sleep_duration(&self, attempt: u64) -> Duration;
}

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
    /// Changing the url of the object on the top of the stack
    Patch,
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, uniffi::Enum)]
pub enum LiveChannelStatus {
    /// [Channel] is waiting for the [Socket](crate::Socket) to
    /// [Socket::connect](crate::Socket::connect) or automatically reconnect.
    WaitingForSocketToConnect,
    /// [Socket::status](crate::Socket::status) is
    /// [SocketStatus::Connected](crate::SocketStatus::Connected) and [Channel] is waiting for
    /// [Channel::join] to be called.
    WaitingToJoin,
    /// [Channel::join] was called and awaiting response from server.
    Joining,
    /// [Channel::join] was called previously, but the [Socket](crate::Socket) was disconnected and
    /// reconnected.
    WaitingToRejoin,
    /// [Channel::join] was called and the server responded that the [Channel::topic] was joined
    /// using [Channel::payload].
    Joined,
    /// [Channel::leave] was called and awaiting response from server.
    Leaving,
    /// [Channel::leave] was called and the server responded that the [Channel::topic] was left.
    Left,
    /// [Channel::shutdown] was called, but the async task hasn't exited yet.
    ShuttingDown,
    /// The async task has exited.
    ShutDown,
}

#[repr(C)]
#[derive(Copy, Clone, uniffi::Enum)]
pub enum ChangeType {
    Change = 0,
    Add = 1,
    Remove = 2,
    Replace = 3,
}

#[derive(Copy, Clone, uniffi::Enum)]
pub enum EventType {
    Changed, // { change: ChangeType },
}

#[derive(Clone, uniffi::Enum)]
pub enum ControlFlow {
    ExitOk,
    ExitErr(String),
    ContinueListening,
}

/// Implements the change handling logic for inbound virtual dom
/// changes. Your logic for handling document patches should go here.
#[uniffi::export(callback_interface)]
pub trait DocumentChangeHandler: Send + Sync {
    /// This callback should implement your dom manipulation logic
    /// after receiving patches from LVN.
    fn handle_document_change(
        &self,
        change_type: ChangeType,
        node_ref: Arc<NodeRef>,
        node_data: NodeData,
        parent: Option<Arc<NodeRef>>,
    );
}

#[derive(Clone, Debug, uniffi::Enum)]
pub enum ClientStatus {
    Disconnected,
    Connecting,
    Reconnecting,
    Connected { new_document: Arc<Document> },
    Error { error: LiveSocketError },
}

/// Implement this if you need to instrument all replies and status
/// changes on the current live channel.
#[cfg(feature = "liveview-channels")]
#[uniffi::export(callback_interface)]
pub trait NetworkEventHandler: Send + Sync {
    /// Whenever a server sent event or reply to a user
    /// message is receiver the event payload is passed to this
    /// callback. by default the client handles diff events and will
    /// handle assets_change, live_patch, live_reload, etc, in the future
    fn on_event(&self, event: phoenix_channels_client::EventPayload);

    /// Called when the view is reloaded, provides the new document.
    /// This means that the previous livechannel has been dropped and
    /// a new livechannel has been established
    ///
    /// The socket may be the same as the previous view if the navigation
    /// API was used within the same livesession, if this is the case
    /// `socket_is_new` will be false
    ///
    /// If the socket was reconnected for any reason `socket_is_new` will be true.
    fn on_status_change(&self, status: ClientStatus);
}
