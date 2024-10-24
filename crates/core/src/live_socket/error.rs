use phoenix_channels_client::{
    CallError, ChannelError, ChannelJoinError, ConnectError, EventsError, JSONDeserializationError,
    LeaveError, PhoenixError, SocketChannelError, SocketError, SpawnError, StatusesError,
    URLParseError,
};

use crate::{
    diff::fragment::{MergeError, RenderError},
    parser::ParseError,
};

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum LiveSocketError {
    #[error("Invalid Header - {error}")]
    InvalidHeader { error: String },
    #[error("Invalid Method - {error}")]
    InvalidMethod { error: String },
    #[error("Phoenix Socket Error - {error}")]
    Phoenix { error: String },
    #[error("Reqwest Error - {error}")]
    Request { error: String },
    #[error("Parse Error - {error}")]
    Parse {
        #[from]
        error: ParseError,
    },
    #[error("JSON Deserialization - {error}")]
    JSONDeserialization { error: String },
    #[error("CSFR Token Missing from DOM!")]
    CSFRTokenMissing,

    #[error("Phoenix ID Missing from DOM!")]
    PhoenixIDMissing,

    #[error("Phoenix Session Missing from DOM!")]
    PhoenixSessionMissing,

    #[error("Phoenix Static Missing from DOM!")]
    PhoenixStaticMissing,

    #[error("Phoenix Main Missing from DOM!")]
    PhoenixMainMissing,

    #[error("Failed to get the host from the URL!")]
    NoHostInURL,

    #[error("Failed to retrieve an upload token.")]
    NoUploadToken,

    #[error("Failed to find live reload url from deadrender.")]
    NoLiveReloadURL,

    #[error("Liveview Scheme not supported! {scheme}")]
    SchemeNotSupported { scheme: String },

    #[error(transparent)]
    Upload {
        #[from]
        error: UploadError,
    },

    #[error("Failed to get document out of the join payload.")]
    NoDocumentInJoinPayload,

    #[error(transparent)]
    DocumentMerge {
        #[from]
        error: MergeError,
    },

    #[error(transparent)]
    DocumentRender {
        #[from]
        error: RenderError,
    },

    #[error("Failed to find the data-phx-upload-ref in the join payload.")]
    NoInputRefInDocument,

    #[error("Failed to find the data-phx-upload-ref in the join payload.")]
    Serde { error: String },

    #[error("There was an error with retrieving the events from the channel.")]
    Events { error: String },
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum UploadError {
    #[error("File exceeds maximum filesize.")]
    FileTooLarge,

    #[error("File was not accepted. Perhaps this file type is invalid.")]
    FileNotAccepted,

    #[error("There was another issue with uploading {error}")]
    Other { error: String },
}

// These are all manually implemented and turned into a string because uniffi doesn't support
// exported error types in the generations.
impl From<PhoenixError> for LiveSocketError {
    fn from(value: PhoenixError) -> Self {
        Self::Phoenix {
            error: value.to_string(),
        }
    }
}
impl From<ConnectError> for LiveSocketError {
    fn from(value: ConnectError) -> Self {
        Self::from(PhoenixError::from(value))
    }
}
impl From<JSONDeserializationError> for LiveSocketError {
    fn from(value: JSONDeserializationError) -> Self {
        Self::JSONDeserialization {
            error: value.to_string(),
        }
    }
}
impl From<URLParseError> for LiveSocketError {
    fn from(value: URLParseError) -> Self {
        Self::from(PhoenixError::from(value))
    }
}
impl From<phoenix_channels_client::url::ParseError> for LiveSocketError {
    fn from(value: phoenix_channels_client::url::ParseError) -> Self {
        Self::from(PhoenixError::from(value))
    }
}

impl From<SocketError> for LiveSocketError {
    fn from(value: SocketError) -> Self {
        Self::from(PhoenixError::from(value))
    }
}
impl From<CallError> for LiveSocketError {
    fn from(value: CallError) -> Self {
        Self::from(PhoenixError::from(value))
    }
}

impl From<SocketChannelError> for LiveSocketError {
    fn from(value: SocketChannelError) -> Self {
        Self::from(PhoenixError::from(SocketError::from(value)))
    }
}

impl From<LeaveError> for LiveSocketError {
    fn from(value: LeaveError) -> Self {
        Self::from(PhoenixError::from(ChannelError::from(value)))
    }
}
impl From<ChannelError> for LiveSocketError {
    fn from(value: ChannelError) -> Self {
        Self::from(PhoenixError::from(value))
    }
}

impl From<ChannelJoinError> for LiveSocketError {
    fn from(value: ChannelJoinError) -> Self {
        Self::from(PhoenixError::from(ChannelError::from(value)))
    }
}
impl From<StatusesError> for LiveSocketError {
    fn from(value: StatusesError) -> Self {
        Self::from(PhoenixError::from(value))
    }
}
impl From<SpawnError> for LiveSocketError {
    fn from(value: SpawnError) -> Self {
        Self::from(PhoenixError::from(value))
    }
}
impl From<reqwest::Error> for LiveSocketError {
    fn from(value: reqwest::Error) -> Self {
        Self::Request {
            error: value.to_string(),
        }
    }
}
impl From<serde_json::Error> for LiveSocketError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde {
            error: value.to_string(),
        }
    }
}
impl From<EventsError> for LiveSocketError {
    fn from(error: EventsError) -> Self {
        Self::Events {
            error: error.to_string(),
        }
    }
}
