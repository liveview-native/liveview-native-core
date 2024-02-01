use phoenix_channels_client::{
    ChannelError,
    PhoenixError,
    SpawnError,
    URLParseError,
    SocketError,
    SocketChannelError,
    StatusesError,
    ConnectError,
    ChannelJoinError,
    JSONDeserializationError,
    CallError,
    LeaveError,
};
use crate::{
    parser::ParseError,
    diff::fragment::{RenderError, MergeError},
};

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum LiveSocketError {
    #[error("Phoenix Socket Error - {error}")]
    Phoenix {
        error: String,
    },
    #[error("Reqwest Error - {error}")]
    Request {
        error: String,
    },
    #[error("Parse Error - {error}")]
    Parse {
        error: ParseError,
    },
    #[error("JSON Deserialization - {error}")]
    JSONDeserialization {
        error: String,
    },
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

    #[error("Liveview Scheme not supported! {scheme}")]
    SchemeNotSupported { scheme: String },

    #[error(transparent)]
    Upload {
        error: UploadError,
    },

    #[error("Failed to get document out of the join payload.")]
    NoDocumentInJoinPayload,

    #[error(transparent)]
    DocumentMerge {
        error: MergeError,
    },

    #[error(transparent)]
    DocumentRender {
        error: RenderError,
    },

    #[error("Failed to find the data-phx-upload-ref in the join payload.")]
    NoInputRefInDocument,

    #[error("Failed to find the data-phx-upload-ref in the join payload.")]
    Serde {
        error: String,
    },
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum UploadError {

    #[error("File exceeds maximum filesize.")]
    FileTooLarge,

    #[error("File was not accepted. Perhaps this file type is invalid.")]
    FileNotAccepted,

    #[error("There was another issue with uploading {error}")]
    Other { error: String},
}
impl From<UploadError> for LiveSocketError {
    fn from(value: UploadError) -> Self {
        Self::Upload {
            error: value,
        }
    }
}

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
        Self::JSONDeserialization { error: value.to_string() }
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
impl From<ParseError> for LiveSocketError {
    fn from(error: ParseError) -> Self {
        Self::Parse { error }
    }
}

impl From<serde_json::Error> for LiveSocketError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde {
            error: value.to_string(),
        }
    }
}

impl From<RenderError> for LiveSocketError {
    fn from(error: RenderError) -> Self {
        Self::DocumentRender {
            error
        }
    }
}
impl From<MergeError> for LiveSocketError {
    fn from(error: MergeError) -> Self {
        Self::DocumentMerge {
            error
        }
    }
}
