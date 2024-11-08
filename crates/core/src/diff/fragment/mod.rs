use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

// This is the diff coming across the wire for an update to the UI. This can be
// converted directly into a Root or merged into a Root itself.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RootDiff {
    #[serde(flatten)]
    fragment: FragmentDiff,
    #[serde(rename = "c", default = "HashMap::new")]
    components: HashMap<String, ComponentDiff>,
}

// This is the struct representation of the whole tree.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Root {
    #[serde(rename = "newRender", skip_serializing_if = "Option::is_none")]
    new_render: Option<bool>,
    #[serde(flatten)]
    fragment: Fragment,
    #[serde(rename = "c", default = "HashMap::new")]
    components: HashMap<String, Component>,
}

// These are used in the wasm build.
impl Root {
    pub fn set_new_render(&mut self, new: bool) {
        self.new_render = Some(new);
    }
    pub fn is_component_only_diff(&self) -> bool {
        !self.components.is_empty() && self.fragment.is_empty()
    }
    pub fn is_new_fingerprint(&self) -> bool {
        self.fragment.is_new_fingerprint()
    }
    pub fn get_component(&self, cid: i32) -> Option<Component> {
        self.components.get(&format!("{cid}")).cloned()
    }
    pub fn component_cids(&self) -> Vec<u32> {
        let keys: Vec<u32> = self
            .components
            .keys()
            .filter_map(|key| key.parse::<u32>().ok())
            .collect();

        keys
    }
}

// This is a direct conversion from RootDiff to Root.
impl TryFrom<RootDiff> for Root {
    type Error = MergeError;
    fn try_from(value: RootDiff) -> Result<Self, MergeError> {
        let mut components: HashMap<String, Component> = HashMap::new();
        for (key, value) in value.components.into_iter() {
            components.insert(key, value.try_into()?);
        }
        Ok(Self {
            new_render: None,
            fragment: value.fragment.try_into()?,
            components,
        })
    }
}

// This is to render the Root as an XML tree in String form.
impl TryInto<String> for Root {
    type Error = RenderError;

    fn try_into(self) -> Result<String, Self::Error> {
        let mut out = String::new();
        let inner = self.fragment.render(&self.components, None, None)?;
        out.push_str(&inner);
        Ok(out)
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum RenderError {
    #[error("No components found when needed")]
    NoComponents,
    #[error("No templates found when needed")]
    NoTemplates,
    #[error("Templated ID {0} not found in templates")]
    TemplateNotFound(i32),
    #[error("Component ID {0} not found in components")]
    ComponentNotFound(i32),
    #[error("Merge Error {0}")]
    MergeError(#[from] MergeError),
    #[error("Child {0} for template")]
    ChildNotFoundForTemplate(i32),
    #[error("Child {0} not found for static")]
    ChildNotFoundForStatic(i32),
    #[error("Cousin not found for {0}")]
    CousinNotFound(i32),
    #[error("Serde Error {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("Parse Error {0}")]
    ParseError(#[from] crate::parser::ParseError),
}

impl Fragment {
    pub fn render(
        &self,
        components: &HashMap<String, Component>,
        cousin_statics: Option<Vec<String>>,
        parent_templates: Templates,
    ) -> Result<String, RenderError> {
        let mut out = String::new();
        match &self {
            Fragment::Regular {
                children, statics, ..
            } => {
                match statics {
                    None => {}
                    Some(Statics::String(_)) => {}
                    Some(Statics::Statics(statics)) => {
                        out.push_str(&statics[0]);
                        // We start at index 1 rather than zero here because
                        // templates and statics are suppose to wrap the inner
                        // contents of the children.
                        for (i, static_item) in statics.iter().enumerate().skip(1) {
                            if let Some(child) = children.get(&(i - 1).to_string()) {
                                let val = child.render(
                                    components,
                                    cousin_statics.clone(),
                                    parent_templates.clone(),
                                )?;
                                out.push_str(&val);
                            }
                            out.push_str(static_item);
                        }
                    }
                    Some(Statics::TemplateRef(template_id)) => {
                        let templates = parent_templates.ok_or(RenderError::NoTemplates)?;
                        let template = templates
                            .get(&(template_id.to_string()))
                            .ok_or(RenderError::TemplateNotFound(*template_id))?;
                        out.push_str(&template[0]);
                        // We start at index 1 rather than zero here because
                        // templates and statics are suppose to wrap the inner
                        // contents of the children.
                        for (i, template_item) in template.iter().enumerate().skip(1) {
                            let child_id = i - 1;
                            let child = children
                                .get(&child_id.to_string())
                                .ok_or(RenderError::ChildNotFoundForTemplate(child_id as i32))?;
                            let val = child.render(
                                components,
                                cousin_statics.clone(),
                                Some(templates.clone()),
                            )?;
                            out.push_str(&val);
                            out.push_str(template_item);
                        }
                    }
                }
            }
            Fragment::Comprehension {
                dynamics,
                statics,
                templates,
                ..
            } => {
                let templates: Templates = match (parent_templates, templates) {
                    (None, None) => None,
                    (None, Some(t)) => Some(t.clone()),
                    (Some(t), None) => Some(t),
                    (Some(parent), Some(child)) => Some(parent).merge(Some(child.clone()))?,
                };
                match (statics, cousin_statics) {
                    (None, None) => {
                        for children in dynamics.iter() {
                            for child in children.iter() {
                                let val = child.render(components, None, templates.clone())?;
                                out.push_str(&val);
                            }
                        }
                    }
                    (None, Some(statics)) => {
                        for children in dynamics.iter() {
                            out.push_str(&statics[0]);
                            // We start at index 1 rather than zero here because
                            // templates and statics are suppose to wrap the inner
                            // contents of the children.
                            for i in 1..statics.len() {
                                let child = &children[i - 1];

                                let val = child.render(components, None, templates.clone())?;
                                out.push_str(&val);
                                out.push_str(&statics[i]);
                            }
                        }
                    }
                    (Some(statics), None) => {
                        match statics {
                            Statics::String(_) => {}
                            Statics::Statics(statics) => {
                                for children in dynamics.iter() {
                                    out.push_str(&statics[0]);
                                    // We start at index 1 rather than zero here because
                                    // templates and statics are suppose to wrap the inner
                                    // contents of the children.
                                    for i in 1..statics.len() {
                                        let child = &children[i - 1];

                                        let val =
                                            child.render(components, None, templates.clone())?;
                                        out.push_str(&val);
                                        out.push_str(&statics[i]);
                                    }
                                }
                            }
                            Statics::TemplateRef(template_id) => {
                                if let Some(ref this_template) = templates {
                                    if let Some(template_statics) =
                                        this_template.get(&template_id.to_string())
                                    {
                                        for children in dynamics.iter() {
                                            out.push_str(&template_statics[0]);

                                            // We start at index 1 rather than zero here because
                                            // templates and statics are suppose to wrap the inner
                                            // contents of the children.
                                            for i in 1..template_statics.len() {
                                                let child = &children[i - 1];

                                                let val = child.render(
                                                    components,
                                                    None,
                                                    templates.clone(),
                                                )?;
                                                out.push_str(&val);
                                                out.push_str(&template_statics[i]);
                                            }
                                        }
                                    } else {
                                        return Err(RenderError::TemplateNotFound(*template_id));
                                    }
                                } else {
                                    return Err(RenderError::NoTemplates);
                                }
                            }
                        }
                    }
                    (Some(_statics), Some(_cousin_templates)) => {
                        panic!("Either statics or cousin statics but not both");
                    }
                }
            }
        }
        Ok(out)
    }
}

impl Child {
    pub fn render(
        &self,
        components: &HashMap<String, Component>,
        statics: Option<Vec<String>>,
        templates: Templates,
    ) -> Result<String, RenderError> {
        match self {
            Child::Fragment(fragment) => fragment.render(components, statics, templates),
            Child::ComponentID(cid) => {
                if let Some(component) = components.get(&cid.to_string()) {
                    component.render(components)
                } else {
                    Err(RenderError::ComponentNotFound(*cid))
                }
            }
            Child::String(OneOrManyStrings::One(s)) => Ok(s.clone()),
            Child::String(OneOrManyStrings::Many(s)) => Ok(s.concat()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Component {
    #[serde(flatten)]
    children: HashMap<String, Child>,
    #[serde(rename = "s")]
    statics: ComponentStatics,
}

impl Component {
    pub fn render(&self, components: &HashMap<String, Component>) -> Result<String, RenderError> {
        match &self.statics {
            ComponentStatics::Statics(statics) => {
                let mut out = String::new();

                out.push_str(&statics[0]);
                // We start at index 1 rather than zero here because
                // templates and statics are suppose to wrap the inner
                // contents of the children.
                for (i, static_item) in statics.iter().enumerate().skip(1) {
                    let inner = self
                        .children
                        .get(&(i - 1).to_string())
                        .ok_or(RenderError::ChildNotFoundForStatic((i - 1) as i32))?;
                    let val = inner.render(components, None, None)?;
                    out.push_str(&val);
                    out.push_str(static_item);
                }
                Ok(out)
            }

            ComponentStatics::ComponentRef(mut cid) => {
                let outer_statics: Vec<String>;
                let cousin_component: Component;
                loop {
                    if let Some(component) = components.get(&cid.to_string()) {
                        match &component.statics {
                            ComponentStatics::Statics(s) => {
                                outer_statics = s.to_vec();
                                cousin_component = component.clone();
                                break;
                            }
                            ComponentStatics::ComponentRef(bread_crumb_cid) => {
                                cid = *bread_crumb_cid;
                            }
                        }
                    } else {
                        return Err(RenderError::ComponentNotFound(cid));
                    }
                }
                let mut out = String::new();

                out.push_str(&outer_statics[0]);
                // We start at index 1 rather than zero here because
                // templates and statics are suppose to wrap the inner
                // contents of the children.
                for (i, outer_static_item) in outer_statics.iter().enumerate().skip(1) {
                    let child = self
                        .children
                        .get(&(i - 1).to_string())
                        .ok_or(RenderError::ChildNotFoundForStatic((i - 1) as i32))?;
                    let cousin = cousin_component
                        .children
                        .get(&(i - 1).to_string())
                        .ok_or(RenderError::CousinNotFound((i - 1) as i32))?;

                    let val = child.render(components, cousin.statics(), None)?;
                    out.push_str(&val);
                    out.push_str(outer_static_item);
                }
                Ok(out)
            }
        }
    }
    pub fn fix_statics(self) -> Self {
        match self.statics {
            ComponentStatics::ComponentRef(cid) if cid < 0 => Self {
                children: self.children,
                statics: ComponentStatics::ComponentRef(-cid),
            },
            _ => self,
        }
    }
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
        reply: Option<i8>,
        #[serde(rename = "stream")]
        stream: Option<StreamUpdate>,
    },
    UpdateRegular {
        #[serde(flatten)]
        children: HashMap<String, ChildDiff>,
        #[serde(rename = "s", skip_serializing_if = "Option::is_none")]
        statics: Option<Statics>,
        #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
        reply: Option<i8>,
    },
}

type Templates = Option<HashMap<String, Vec<String>>>;
type DynamicsDiff = Vec<Vec<ChildDiff>>;
type Dynamics = Vec<Vec<Child>>;

impl FragmentDiff {
    fn should_replace_current(&self, current: &Fragment) -> bool {
        let old_statics = match current {
            Fragment::Regular { statics, .. } => statics,
            Fragment::Comprehension { statics, .. } => statics,
        };

        match self {
            FragmentDiff::UpdateRegular { statics, .. } => {
                statics.is_some() && statics != old_statics
            }
            FragmentDiff::UpdateComprehension { statics, .. } => {
                statics.is_some() && statics != old_statics
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Fragment {
    Comprehension {
        #[serde(rename = "d")]
        dynamics: Dynamics,
        #[serde(rename = "s")]
        statics: Option<Statics>,
        #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
        reply: Option<i8>,
        #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
        templates: Templates,
        #[serde(rename = "stream", skip_serializing_if = "Option::is_none")]
        stream: Option<Stream>,
    },
    Regular {
        #[serde(rename = "s", skip_serializing_if = "Option::is_none")]
        statics: Option<Statics>,
        #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
        reply: Option<i8>,
        #[serde(flatten)]
        children: HashMap<String, Child>,
    },
}

impl Fragment {
    pub fn is_new_fingerprint(&self) -> bool {
        match self {
            Fragment::Regular { statics, .. } | Fragment::Comprehension { statics, .. } => {
                statics.is_some()
            }
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Fragment::Comprehension {
                dynamics,
                statics: None,
                reply: None,
                templates: None,
                stream: None,
            } => dynamics.is_empty(),
            _ => false,
        }
    }
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

impl TryFrom<Vec<StreamAttribute>> for Stream {
    type Error = StreamConversionError;
    fn try_from(stream_attrs: Vec<StreamAttribute>) -> Result<Self, Self::Error> {
        let mut stream: Stream = Stream {
            id: String::new(),
            stream_items: Vec::new(),
        };
        for stream_attr in &stream_attrs {
            match stream_attr {
                StreamAttribute::StreamID(id) => {
                    stream.id = id.to_string();
                }
                StreamAttribute::Inserts(inserts) => {
                    for (stream_id, index, limit) in inserts.iter() {
                        stream.stream_items.push(StreamItem {
                            id: stream_id.to_string(),
                            index: *index,
                            limit: *limit,
                        });
                    }
                }
                StreamAttribute::DeleteIDs(_delete_ids) => {
                    log::error!("Deleting Stream IDs when converting from a fragmentdiff to a fragment should not occur");
                }
                StreamAttribute::ResetStream(reset) => {
                    if *reset {
                        stream.stream_items = Vec::new();
                    }
                }
            }
        }
        Ok(stream)
    }
}

pub type StreamUpdate = Vec<StreamAttribute>;

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
                reply,
            } => {
                let mut new_children: HashMap<String, Child> = HashMap::new();

                for (key, cdiff) in children.into_iter() {
                    new_children.insert(key, cdiff.try_into()?);
                }

                Ok(Self::Regular {
                    children: new_children,
                    statics,
                    reply,
                })
            }
            FragmentDiff::UpdateComprehension {
                dynamics,
                templates,
                statics,
                stream,
                reply,
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
                    reply,
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

impl FragmentMerge for Option<Statics> {
    type DiffItem = Option<Statics>;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        Ok(diff.or(self))
    }
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

impl TryFrom<ChildDiff> for Child {
    type Error = MergeError;

    fn try_from(value: ChildDiff) -> Result<Self, Self::Error> {
        match value {
            ChildDiff::String(s) => Ok(Child::String(s)),
            ChildDiff::ComponentID(cid) => Ok(Child::ComponentID(cid)),
            ChildDiff::Fragment(fragment_diff) => Ok(Self::Fragment(fragment_diff.try_into()?)),
        }
    }
}

impl ChildDiff {
    pub fn to_new_child(self) -> Result<Child, MergeError> {
        self.try_into()
    }
}

impl TryFrom<ComponentDiff> for Component {
    type Error = MergeError;
    fn try_from(value: ComponentDiff) -> Result<Self, MergeError> {
        match value {
            ComponentDiff::UpdateRegular { .. } => Err(MergeError::CreateComponentFromUpdate),
            ComponentDiff::ReplaceCurrent {
                children, statics, ..
            } => Ok(Self { children, statics }),
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
    },
    UpdateRegular {
        #[serde(flatten)]
        children: HashMap<String, ChildDiff>,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ComponentStatics {
    Statics(Vec<String>),
    ComponentRef(i32),
}

pub trait FragmentMerge: Sized {
    type DiffItem;
    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError>;
}

impl FragmentMerge for Root {
    type DiffItem = RootDiff;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        let fragment = self.fragment.merge(diff.fragment)?;
        let components = self.components.merge(diff.components)?;

        Ok(Self {
            new_render: None,
            fragment,
            components,
        })
    }
}

impl FragmentMerge for Fragment {
    type DiffItem = FragmentDiff;

    fn merge(self, diff: FragmentDiff) -> Result<Self, MergeError> {
        if diff.should_replace_current(&self) {
            return diff.try_into();
        }

        match (self, diff) {
            (
                Fragment::Regular {
                    children: current_children,
                    statics: current_statics,
                    reply: current_reply,
                },
                FragmentDiff::UpdateRegular {
                    children: children_diffs,
                    reply: new_reply,
                    ..
                },
            ) => {
                let new_children = current_children.merge(children_diffs)?;
                let new_reply = new_reply.or(current_reply);

                Ok(Self::Regular {
                    children: new_children,
                    statics: current_statics,
                    reply: new_reply,
                })
            }
            (
                Fragment::Comprehension {
                    dynamics: mut current_dynamics,
                    statics: current_statics,
                    templates: current_templates,
                    stream: current_stream,
                    reply: current_reply,
                },
                FragmentDiff::UpdateComprehension {
                    dynamics: new_dynamics,
                    templates: new_templates,
                    statics: new_statics,
                    stream: new_stream,
                    reply: new_reply,
                },
            ) => {
                let new_reply = new_reply.or(current_reply);

                let templates = current_templates.merge(new_templates)?;
                let statics = current_statics.merge(new_statics)?;

                let new_dynamics: Vec<Vec<Child>> = new_dynamics
                    .into_iter()
                    .map(|children_children| {
                        children_children
                            .into_iter()
                            .map(|child| child.to_new_child())
                            .collect::<Result<Vec<Child>, MergeError>>()
                    })
                    .collect::<Result<Vec<Vec<Child>>, MergeError>>()?;

                let stream = match (current_stream, new_stream) {
                    (None, None) => {
                        current_dynamics = new_dynamics;
                        None
                    }
                    (None, Some(stream_attrs)) => Some(Stream::try_from(stream_attrs)?),
                    (Some(stream), None) => Some(stream),
                    (Some(mut stream), Some(stream_update)) => {
                        for stream_attr in &stream_update {
                            match stream_attr {
                                StreamAttribute::StreamID(stream_id) => {
                                    if stream.id != *stream_id {
                                        return Err(MergeError::StreamIDMismatch);
                                    }
                                }
                                StreamAttribute::Inserts(inserts) => {
                                    for (insert_id, index, _limit) in inserts.iter() {
                                        if let Some(dynamic) =
                                            new_dynamics.iter().find(|children| {
                                                children.iter().any(|child| {
                                                    Child::String(
                                                        format!(" id=\"{insert_id}\"").into(),
                                                    ) == *child
                                                })
                                            })
                                        {
                                            if *index == -1 {
                                                current_dynamics.push(dynamic.clone());
                                            }
                                        }
                                    }
                                }
                                StreamAttribute::DeleteIDs(delete_ids) => {
                                    for delete_id in delete_ids {
                                        if let Some(index) =
                                            current_dynamics.iter().position(|children| {
                                                children.iter().any(|child| {
                                                    Child::String(
                                                        format!(" id=\"{delete_id}\"").into(),
                                                    ) == *child
                                                })
                                            })
                                        {
                                            current_dynamics.remove(index);
                                        }
                                    }
                                }
                                StreamAttribute::ResetStream(reset) => {
                                    if *reset {
                                        stream.stream_items = Vec::new();
                                        current_dynamics.clone_from(&new_dynamics)
                                    }
                                }
                            }
                        }
                        Some(stream)
                    }
                };
                Ok(Self::Comprehension {
                    dynamics: current_dynamics,
                    statics,
                    templates,
                    stream,
                    reply: new_reply,
                })
            }
            _ => Err(MergeError::FragmentTypeMismatch),
        }
    }
}
impl FragmentMerge for HashMap<String, Component> {
    type DiffItem = HashMap<String, ComponentDiff>;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        let mut new_components: HashMap<String, Component> = HashMap::new();
        for (cid, comp_diff) in diff.into_iter() {
            if let Some(existing) = new_components.get_mut(&cid) {
                *existing = existing.clone().merge(comp_diff)?;
            } else {
                new_components.insert(cid.clone(), comp_diff.try_into()?);
            }
        }

        Ok(new_components)
    }
}

impl FragmentMerge for Component {
    type DiffItem = ComponentDiff;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        match diff {
            ComponentDiff::UpdateRegular {
                children: children_diffs,
                ..
            } => {
                let new_children = self.children.merge(children_diffs)?;
                Ok(Self {
                    children: new_children,
                    statics: self.statics,
                })
            }
            ComponentDiff::ReplaceCurrent {
                statics, children, ..
            } => Ok(Self { children, statics }.fix_statics()),
        }
    }
}

impl FragmentMerge for Templates {
    type DiffItem = Templates;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        match (self, diff) {
            (None, None) => Ok(None),
            (None, Some(template)) => Ok(Some(template)),
            (Some(template), None) => Ok(Some(template)),
            (Some(mut current), Some(new)) => {
                for (key, val) in new.into_iter() {
                    current.insert(key, val);
                }
                Ok(Some(current))
            }
        }
    }
}
impl FragmentMerge for Child {
    type DiffItem = ChildDiff;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        match (self, diff) {
            (Child::Fragment(current_fragment), ChildDiff::Fragment(fragment_diff)) => {
                Ok(Self::Fragment(current_fragment.merge(fragment_diff)?))
            }
            (_, ChildDiff::String(s)) => Ok(Self::String(s)),
            (_, ChildDiff::ComponentID(id)) => Ok(Self::ComponentID(id)),
            (_, ChildDiff::Fragment(fragment_diff)) => {
                Ok(Self::Fragment(fragment_diff.try_into()?))
            }
        }
    }
}

impl FragmentMerge for HashMap<String, Child> {
    type DiffItem = HashMap<String, ChildDiff>;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        let mut new_children = self;
        for (index, comp_diff) in diff.into_iter() {
            if let Some(child) = new_children.get_mut(&index) {
                *child = child.clone().merge(comp_diff)?;
            } else {
                new_children.insert(index, comp_diff.try_into()?);
            }
        }
        Ok(new_children)
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MergeError {
    #[error("Fragment type mismatch")]
    FragmentTypeMismatch,
    #[error("Create component from update")]
    CreateComponentFromUpdate,
    #[error("Create child from update fragment")]
    CreateChildFromUpdateFragment,
    #[error("Add child to existing")]
    AddChildToExisting,
    #[error("There was a id mismatch when merging a stream")]
    StreamIDMismatch,
    #[error("Stream Error {error}")]
    Stream {
        #[from]
        error: StreamConversionError,
    },
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum StreamConversionError {
    #[error("There was no stream ID for this ")]
    NoStreamID,
}
