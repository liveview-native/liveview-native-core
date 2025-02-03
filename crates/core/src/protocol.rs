use serde::{Deserialize, Serialize};

/// Replies can contain redirect information either in a
/// live_redirect (new channel) or a full redirect (new socket)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveRedirect {
    pub kind: Option<RedirectKind>,
    pub to: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RedirectKind {
    Push,
    Replace,
}
