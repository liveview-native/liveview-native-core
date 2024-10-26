use std::collections::HashMap;

/// Shared protocol structs and parsing helpers,
/// In the future some of this code should be generated from a json schema
use serde::{Deserialize, Serialize};

/// an ascending nonce as assigned per upload for the live channel.
pub type UploadId = u64;

pub type ValidateResponse = CallResponse<serde_json::Value>;

/// A type mapping fragment indices (as represented by u64's) to
/// Some type
pub type IdMap<T> = HashMap<u64, T>;

/// Generic call wrapper
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum CallResponse<T> {
    Error { error: T },
    Diff { diff: T },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ValidateUpload {
    #[serde(flatten)]
    id_map: HashMap<UploadId, IdMap<IdMap<String>>>,
}
