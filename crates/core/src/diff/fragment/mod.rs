use std::collections::HashMap;

mod error;
mod merge;
mod render;
mod wasm;

#[cfg(test)]
mod tests;

pub use error::*;
pub use merge::*;
use serde::{Deserialize, Serialize};

// This is the diff coming across the wire for an update to the UI. This can be
// converted directly into a Root or merged into a Root itself.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RootDiff {
    // this flag is for wasm compatibility, it currently does nothing
    #[serde(rename = "newRender", skip_serializing_if = "Option::is_none")]
    new_render: Option<bool>,
    #[serde(flatten)]
    fragment: FragmentDiff,
    #[serde(rename = "c", default = "HashMap::new")]
    components: HashMap<String, ComponentDiff>,
}

// This is the struct representation of the whole tree.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Root {
    // this flag is for wasm compatibility, it currently does nothing
    #[serde(rename = "newRender", skip_serializing_if = "Option::is_none")]
    new_render: Option<bool>,
    #[serde(flatten)]
    fragment: Fragment,
    #[serde(rename = "c", default = "HashMap::new")]
    components: HashMap<String, Component>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Component {
    #[serde(flatten)]
    children: HashMap<String, Child>,
    #[serde(rename = "s")]
    statics: ComponentStatics,
    #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
    is_root: Option<i8>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FragmentDiff {
    UpdateComprehension {
        #[serde(rename = "d")]
        dynamics: DynamicsDiff,
        #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
        templates: Templates,
        #[serde(rename = "s", skip_serializing_if = "Option::is_none")]
        statics: Option<Statics>,
        #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
        is_root: Option<i8>,
        #[serde(rename = "stream")]
        stream: Option<StreamUpdate>,
    },
    UpdateRegular {
        #[serde(flatten)]
        children: HashMap<String, ChildDiff>,
        #[serde(rename = "s", skip_serializing_if = "Option::is_none")]
        statics: Option<Statics>,
        #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
        is_root: Option<i8>,
    },
}

type Templates = Option<HashMap<String, Vec<String>>>;
type DynamicsDiff = Vec<Vec<ChildDiff>>;
type Dynamics = Vec<Vec<Child>>;
pub type StreamUpdate = Vec<StreamAttribute>;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Fragment {
    Comprehension {
        #[serde(rename = "d")]
        dynamics: Dynamics,
        #[serde(rename = "s")]
        statics: Option<Statics>,
        #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
        is_root: Option<i8>,
        #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
        templates: Templates,
        #[serde(rename = "stream", skip_serializing_if = "Option::is_none")]
        stream: Option<Stream>,
        #[serde(rename = "newRender", skip_serializing_if = "Option::is_none")]
        new_render: Option<bool>,
    },
    Regular {
        #[serde(rename = "s", skip_serializing_if = "Option::is_none")]
        statics: Option<Statics>,
        #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
        is_root: Option<i8>,
        #[serde(flatten)]
        children: HashMap<String, Child>,
        #[serde(rename = "newRender", skip_serializing_if = "Option::is_none")]
        new_render: Option<bool>,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Stream {
    // This is actually a string wrapped integer.
    id: String,
    stream_items: Vec<StreamItem>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StreamItem {
    id: String,
    index: i32,
    limit: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StreamAttribute {
    StreamID(String),
    Inserts(Vec<(String, i32, Option<i32>)>),
    DeleteIDs(Vec<String>),
    ResetStream(bool),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StreamInsert {
    StreamAt(i32),
    Limit(Option<i32>),
}

impl TryFrom<FragmentDiff> for Fragment {
    type Error = MergeError;
    fn try_from(value: FragmentDiff) -> Result<Self, MergeError> {
        match value {
            FragmentDiff::UpdateRegular {
                children,
                statics,
                is_root: reply,
            } => {
                let mut new_children: HashMap<String, Child> = HashMap::new();

                for (key, cdiff) in children.into_iter() {
                    new_children.insert(key, cdiff.try_into()?);
                }

                Ok(Self::Regular {
                    children: new_children,
                    statics,
                    is_root: reply,
                    new_render: None,
                })
            }
            FragmentDiff::UpdateComprehension {
                dynamics,
                templates,
                statics,
                stream,
                is_root: reply,
            } => {
                let dynamics: Dynamics = dynamics
                    .into_iter()
                    .map(|cdiff_vec| {
                        cdiff_vec
                            .into_iter()
                            .map(|cdiff| cdiff.try_into())
                            .collect::<Result<Vec<Child>, MergeError>>()
                    })
                    .collect::<Result<Vec<Vec<Child>>, MergeError>>()?;

                let stream = if let Some(stream_updates) = stream {
                    let stream: Stream = Stream::try_from(stream_updates)?;
                    Some(stream)
                } else {
                    None
                };

                Ok(Self::Comprehension {
                    dynamics,
                    statics,
                    templates,
                    stream,
                    is_root: reply,
                    new_render: None,
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Statics {
    String(String),
    Statics(Vec<String>),
    TemplateRef(i32),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Child {
    Fragment(Fragment),
    ComponentID(i32),
    String(OneOrManyStrings),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChildDiff {
    Fragment(FragmentDiff),
    ComponentID(i32),
    String(OneOrManyStrings),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OneOrManyStrings {
    One(String),
    Many(Vec<String>),
}

impl From<String> for OneOrManyStrings {
    fn from(value: String) -> Self {
        Self::One(value)
    }
}

impl Child {
    pub fn statics(&self) -> Option<Vec<String>> {
        match self {
            Self::Fragment(Fragment::Regular {
                statics: Some(Statics::Statics(statics)),
                ..
            }) => Some(statics.clone()),
            Self::Fragment(Fragment::Comprehension {
                statics: Some(Statics::Statics(statics)),
                ..
            }) => Some(statics.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ComponentDiff {
    ReplaceCurrent {
        #[serde(flatten)]
        children: HashMap<String, Child>,
        #[serde(rename = "s")]
        statics: ComponentStatics,
        #[serde(rename = "newRender", skip_serializing)]
        new_render: Option<bool>,
        #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
        is_root: Option<i8>,
    },
    UpdateRegular {
        #[serde(flatten)]
        children: HashMap<String, ChildDiff>,
        #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
        is_root: Option<i8>,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ComponentStatics {
    Statics(Vec<String>),
    ComponentRef(i32),
}
