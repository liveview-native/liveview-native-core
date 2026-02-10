use std::collections::HashMap;

mod error;
mod merge;
mod render;
mod wasm;

#[cfg(test)]
mod tests;

pub use error::*;
pub use merge::*;
use serde::{de::DeserializeOwned, de::Deserializer, de::MapAccess, de::Visitor, Deserialize, Serialize, ser::Serializer};
use serde_json::Value;

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

impl RootDiff {
    pub fn events<T: DeserializeOwned>(&self) -> Result<Option<T>, serde_json::Error> {
        match &self.fragment {
            FragmentDiff::UpdateComprehension {
                event: Some(event), ..
            } => {
                let out = serde_json::from_value(event.clone())?;
                Ok(Some(out))
            }
            FragmentDiff::UpdateRegular {
                event: Some(event), ..
            } => {
                let out = serde_json::from_value(event.clone())?;
                Ok(Some(out))
            }
            _ => Ok(None),
        }
    }
}

// This is the struct representation a complete interpolation tree.
// It is not a type we expect over the wire. It is a patchable
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
        #[serde(rename = "e")]
        event: Option<Value>,
    },
    /// Keyed comprehension diff - uses "k" for keyed items
    UpdateKeyedComprehension {
        #[serde(rename = "k")]
        keyed: KeyedItemsDiff,
        #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
        templates: Templates,
        #[serde(rename = "s", skip_serializing_if = "Option::is_none")]
        statics: Option<Statics>,
        #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
        is_root: Option<i8>,
        #[serde(rename = "e")]
        event: Option<Value>,
    },
    UpdateRegular {
        #[serde(flatten)]
        children: HashMap<String, ChildDiff>,
        #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
        templates: Templates,
        #[serde(rename = "s", skip_serializing_if = "Option::is_none")]
        statics: Option<Statics>,
        #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
        is_root: Option<i8>,
        #[serde(rename = "e")]
        event: Option<Value>,
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
    /// Keyed comprehension - uses "k" for keyed items instead of "d" for dynamics
    /// This is new in LiveView 1.1+ and enables efficient diffing of lists with keys
    KeyedComprehension {
        #[serde(rename = "k")]
        keyed: KeyedItems,
        #[serde(rename = "s")]
        statics: Option<Statics>,
        #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
        is_root: Option<i8>,
        #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
        templates: Templates,
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
        #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
        templates: Templates,
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

/// Keyed items container for keyed comprehensions.
/// The "k" key in the wire protocol contains both the keyed items and a "kc" (key count) field.
/// Items are keyed by their string index ("0", "1", etc.) and kc indicates how many items there are.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyedItems {
    pub items: HashMap<String, KeyedItem>,
    pub key_count: i32,
}

/// A keyed item can be:
/// - A full fragment (new or changed item)
/// - An integer indicating the item was moved from that old position unchanged
/// - A tuple [old_pos, diff] indicating moved with changes
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum KeyedItem {
    /// Item moved from old position, apply diff to get new state
    MovedWithDiff(i32, Box<FragmentDiff>),
    /// Item moved from old position with no changes
    MovedFrom(i32),
    /// New or fully replaced item
    Fragment(Box<Fragment>),
}

/// Diff variant for keyed items
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum KeyedItemDiff {
    /// Item moved from old position, apply diff to get new state
    MovedWithDiff(i32, Box<FragmentDiff>),
    /// Item moved from old position with no changes
    MovedFrom(i32),
    /// New or fully replaced item as a diff
    FragmentDiff(Box<FragmentDiff>),
}

impl<'de> Deserialize<'de> for KeyedItems {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyedItemsVisitor;

        impl<'de> Visitor<'de> for KeyedItemsVisitor {
            type Value = KeyedItems;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map with keyed items and a kc (key count) field")
            }

            fn visit_map<M>(self, mut map: M) -> Result<KeyedItems, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut items = HashMap::new();
                let mut key_count = None;

                while let Some(key) = map.next_key::<String>()? {
                    if key == "kc" {
                        key_count = Some(map.next_value::<i32>()?);
                    } else {
                        let value = map.next_value::<KeyedItem>()?;
                        items.insert(key, value);
                    }
                }

                let key_count = key_count.unwrap_or(items.len() as i32);

                Ok(KeyedItems { items, key_count })
            }
        }

        deserializer.deserialize_map(KeyedItemsVisitor)
    }
}

impl Serialize for KeyedItems {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.items.len() + 1))?;
        for (k, v) in &self.items {
            map.serialize_entry(k, v)?;
        }
        map.serialize_entry("kc", &self.key_count)?;
        map.end()
    }
}

/// Keyed items diff container - similar to KeyedItems but for diffs
#[derive(Debug, Clone, PartialEq)]
pub struct KeyedItemsDiff {
    pub items: HashMap<String, KeyedItemDiff>,
    pub key_count: i32,
}

impl<'de> Deserialize<'de> for KeyedItemsDiff {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyedItemsDiffVisitor;

        impl<'de> Visitor<'de> for KeyedItemsDiffVisitor {
            type Value = KeyedItemsDiff;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map with keyed item diffs and a kc (key count) field")
            }

            fn visit_map<M>(self, mut map: M) -> Result<KeyedItemsDiff, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut items = HashMap::new();
                let mut key_count = None;

                while let Some(key) = map.next_key::<String>()? {
                    if key == "kc" {
                        key_count = Some(map.next_value::<i32>()?);
                    } else {
                        let value = map.next_value::<KeyedItemDiff>()?;
                        items.insert(key, value);
                    }
                }

                let key_count = key_count.unwrap_or(items.len() as i32);

                Ok(KeyedItemsDiff { items, key_count })
            }
        }

        deserializer.deserialize_map(KeyedItemsDiffVisitor)
    }
}

impl Serialize for KeyedItemsDiff {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.items.len() + 1))?;
        for (k, v) in &self.items {
            map.serialize_entry(k, v)?;
        }
        map.serialize_entry("kc", &self.key_count)?;
        map.end()
    }
}

impl TryFrom<FragmentDiff> for Fragment {
    type Error = MergeError;
    fn try_from(value: FragmentDiff) -> Result<Self, MergeError> {
        match value {
            FragmentDiff::UpdateRegular {
                children,
                templates,
                statics,
                is_root: reply,
                ..
            } => {
                let mut new_children: HashMap<String, Child> = HashMap::new();

                for (key, cdiff) in children.into_iter() {
                    new_children.insert(key, cdiff.try_into()?);
                }

                Ok(Self::Regular {
                    children: new_children,
                    statics,
                    is_root: reply,
                    templates,
                    new_render: None,
                })
            }
            FragmentDiff::UpdateComprehension {
                dynamics,
                templates,
                statics,
                stream,
                is_root: reply,
                ..
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
            FragmentDiff::UpdateKeyedComprehension {
                keyed,
                templates,
                statics,
                is_root: reply,
                ..
            } => {
                // Convert KeyedItemsDiff to KeyedItems
                let mut items = HashMap::new();
                for (key, item_diff) in keyed.items {
                    let item = keyed_item_diff_to_keyed_item(item_diff)?;
                    items.insert(key, item);
                }

                Ok(Self::KeyedComprehension {
                    keyed: KeyedItems {
                        items,
                        key_count: keyed.key_count,
                    },
                    statics,
                    templates,
                    is_root: reply,
                    new_render: None,
                })
            }
        }
    }
}

/// Convert a KeyedItemDiff to a KeyedItem
fn keyed_item_diff_to_keyed_item(diff: KeyedItemDiff) -> Result<KeyedItem, MergeError> {
    match diff {
        KeyedItemDiff::MovedFrom(pos) => Ok(KeyedItem::MovedFrom(pos)),
        KeyedItemDiff::MovedWithDiff(pos, fragment_diff) => {
            Ok(KeyedItem::MovedWithDiff(pos, fragment_diff))
        }
        KeyedItemDiff::FragmentDiff(fragment_diff) => {
            let fragment: Fragment = (*fragment_diff).try_into()?;
            Ok(KeyedItem::Fragment(Box::new(fragment)))
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
            Self::Fragment(Fragment::KeyedComprehension {
                statics: Some(Statics::Statics(statics)),
                ..
            }) => Some(statics.clone()),
            _ => None,
        }
    }

}

impl Fragment {
    /// Resolve all TemplateRef values in-place and delete templates afterward.
    /// This mirrors the side effects Phoenix JS performs during `toString()`:
    /// resolve refs via `templateStatic`, then `delete rendered[TEMPLATES]`.
    pub fn expand_statics(&mut self) {
        self.expand_statics_with_parent(&None);
    }

    /// Own templates REPLACE parent (not merge). Templates are deleted after
    /// extraction. KeyedComprehension uses parent templates for own resolution
    /// (falls back to own when at root level).
    fn expand_statics_with_parent(&mut self, parent_templates: &Templates) {
        match self {
            Fragment::Regular {
                statics,
                templates,
                children,
                ..
            } => {
                // Own replaces parent (Phoenix toOutputBuffer)
                let effective = if templates.is_some() {
                    templates.clone()
                } else {
                    parent_templates.clone()
                };

                if let Some(Statics::TemplateRef(id)) = statics {
                    if let Some(ref tmpl) = effective {
                        if let Some(resolved) = tmpl.get(&id.to_string()) {
                            *statics = Some(Statics::Statics(resolved.clone()));
                        }
                    }
                }

                *templates = None;

                for child in children.values_mut() {
                    if let Child::Fragment(frag) = child {
                        frag.expand_statics_with_parent(&effective);
                    }
                }
            }
            Fragment::Comprehension {
                statics,
                templates,
                dynamics,
                ..
            } => {
                let effective = if templates.is_some() {
                    templates.clone()
                } else {
                    parent_templates.clone()
                };

                if let Some(Statics::TemplateRef(id)) = statics {
                    if let Some(ref tmpl) = effective {
                        if let Some(resolved) = tmpl.get(&id.to_string()) {
                            *statics = Some(Statics::Statics(resolved.clone()));
                        }
                    }
                }

                *templates = None;

                for row in dynamics.iter_mut() {
                    for child in row.iter_mut() {
                        if let Child::Fragment(frag) = child {
                            frag.expand_statics_with_parent(&effective);
                        }
                    }
                }
            }
            Fragment::KeyedComprehension {
                statics,
                templates,
                keyed,
                ..
            } => {
                // Phoenix comprehensionToBuffer: parent priority, fall back to own
                let effective = if parent_templates.is_some() {
                    parent_templates.clone()
                } else {
                    templates.clone()
                };

                if let Some(Statics::TemplateRef(id)) = statics {
                    if let Some(ref tmpl) = effective {
                        if let Some(resolved) = tmpl.get(&id.to_string()) {
                            *statics = Some(Statics::Statics(resolved.clone()));
                        }
                    }
                }

                *templates = None;

                for item in keyed.items.values_mut() {
                    if let KeyedItem::Fragment(frag) = item {
                        frag.expand_statics_with_parent(&effective);
                    }
                }
            }
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
