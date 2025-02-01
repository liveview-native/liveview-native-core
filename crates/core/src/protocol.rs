use serde::{Deserialize, Serialize};

/// a reply
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveRedirect {
    pub kind: Option<RedirectKind>,
    pub to: String,
    pub mode: Option<RedirectMode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RedirectKind {
    Push,
    Replace,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RedirectMode {
    ReplaceTop,
    Patch,
}
