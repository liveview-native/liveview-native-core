use std::collections::HashMap;
use serde::Deserialize;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RootDiff {
    #[serde(flatten)]
    fragment: FragmentDiff,
    #[serde(rename = "c")]
    components: Option<HashMap<String, ComponentDiff>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Root {
    #[serde(flatten)]
    fragment: Fragment,
    #[serde(rename = "c")]
    components: Option<HashMap<String, Component>>,
}

impl TryFrom<RootDiff> for Root {
    type Error = MergeError;
    fn try_from(value: RootDiff) -> Result<Self, MergeError> {
        let components = if let Some(components) = value.components {
            let mut out : HashMap<String, Component> = HashMap::new();
            for (key, value) in components.into_iter() {
                out.insert(key, value.try_into()?);
            }
            Some(out)
        } else {
            None
        };
        Ok(Self {
            fragment: value.fragment.try_into()?,
            components,
        })
    }
}

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
    pub fn render(&self, components: &Option<HashMap<String, Component>>, cousin_statics: Option<Vec<String>>, parent_templates: Templates) -> Result<String, RenderError> {
        let mut out = String::new();
        match &self {
            Fragment::Regular { children, statics } => {
                match statics {
                    Statics::Statics(statics) => {
                        out.push_str(&statics[0]);
                        // We start at index 1 rather than zero here because
                        // templates and statics are suppose to wrap the inner
                        // contents of the children.
                        for i in 1..statics.len() {
                            let child = children.get(&(i - 1).to_string()).ok_or(RenderError::ChildNotFoundForStatic((i - 1) as i32))?;
                            let val = child.render(components, cousin_statics.clone(), parent_templates.clone())?;
                            out.push_str(&val);
                            out.push_str(&statics[i]);
                        }
                    }
                    Statics::TemplateRef(template_id) => {
                        let templates = parent_templates.ok_or(RenderError::NoTemplates)?;
                        let template = templates.get(&(template_id.to_string())).ok_or(RenderError::TemplateNotFound(*template_id))?;
                        out.push_str(&template[0]);
                        // We start at index 1 rather than zero here because
                        // templates and statics are suppose to wrap the inner
                        // contents of the children.
                        for i in 1..template.len() {
                            let child_id = i - 1;
                            let child = children.get(&child_id.to_string()).ok_or(RenderError::ChildNotFoundForTemplate(child_id as i32))?;
                            let val = child.render(components, cousin_statics.clone(), Some(templates.clone()))?;
                            out.push_str(&val);
                            out.push_str(&template[i]);
                        }
                    }
                }
            }
            Fragment::Comprehension { dynamics, statics, templates } => {
                let templates : Templates = match (parent_templates, templates) {
                    (None, None) => None,
                    (None, Some(t)) => Some(t.clone()),
                    (Some(t), None) => Some(t),
                    (Some(parent), Some(child)) => {
                        Some(parent).merge(Some(child.clone()))?
                    }
                };
                match (statics, cousin_statics) {
                    (None, None) => {
                        for children in dynamics.into_iter() {
                            for child in children.into_iter() {
                                let val = child.render(components, None, templates.clone())?;
                                out.push_str(&val);
                            }
                        }
                    }
                    (None, Some(statics)) => {
                        for children in dynamics.into_iter() {
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
                            Statics::Statics(statics) => {
                                for children in dynamics.into_iter() {
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
                            Statics::TemplateRef(template_id) => {
                                if let Some(ref this_template) = templates {
                                    if let Some(ref template_statics) = this_template.get(&template_id.to_string()) {
                                        for children in dynamics.into_iter() {
                                            out.push_str(&template_statics[0]);

                                            // We start at index 1 rather than zero here because
                                            // templates and statics are suppose to wrap the inner
                                            // contents of the children.
                                            for i in 1..template_statics.len() {
                                                let child = &children[i - 1];

                                                let val = child.render(components, None, templates.clone())?;
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
    pub fn render(&self, components: &Option<HashMap<String, Component>>, statics: Option<Vec<String>>, templates: Templates) -> Result<String, RenderError> {
        match self {
            Child::Fragment(fragment) => fragment.render(components, statics, templates),
            Child::ComponentID(cid) => {
                if let Some(inner_components) = components {
                    if let Some(component) = inner_components.get(&cid.to_string()) {
                        component.render(components)
                    } else {
                        Err(RenderError::ComponentNotFound(*cid))
                    }
                } else {
                    Err(RenderError::NoComponents)
                }
            }
            Child::String(inner) => Ok(inner.to_string())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Component {
    #[serde(flatten)]
    children: HashMap<String, Child>,
    #[serde(rename = "s")]
    statics: ComponentStatics,
}

impl Component {
    pub fn render(&self, components: &Option<HashMap<String, Component>>) -> Result<String, RenderError> {
        match &self.statics {
            ComponentStatics::Statics(statics) => {
                let mut out = String::new();

                out.push_str(&statics[0]);
                // We start at index 1 rather than zero here because
                // templates and statics are suppose to wrap the inner
                // contents of the children.
                for i in 1..statics.len() {
                    let inner = self.children.get(&(i - 1).to_string()).ok_or(RenderError::ChildNotFoundForStatic((i - 1) as i32))?;
                    let val = inner.render(components, None, None)?;
                    out.push_str(&val);
                    out.push_str(&statics[i]);
                }
                Ok(out)
            }

            ComponentStatics::ComponentRef(mut cid) => {
                let outer_statics : Vec<String> ;
                let cousin_component: Component;
                loop {
                    if let Some(inner_components) = components {
                        if let Some(component) = inner_components.get(&cid.to_string()) {
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
                    } else {
                        return Err(RenderError::NoComponents);
                    }
                }
                let mut out = String::new();

                out.push_str(&outer_statics[0]);
                // We start at index 1 rather than zero here because
                // templates and statics are suppose to wrap the inner
                // contents of the children.
                for i in 1..outer_statics.len() {
                    let child = self.children.get(&(i - 1).to_string()).ok_or(RenderError::ChildNotFoundForStatic((i - 1) as i32))?;
                    let cousin = cousin_component.children.get(&(i - 1).to_string()).ok_or(RenderError::CousinNotFound((i - 1) as i32))?;

                    let val = child.render(components, cousin.statics(), None)?;
                    out.push_str(&val);
                    out.push_str(&outer_statics[i]);
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



#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum FragmentDiff {
    UpdateRegular {
        #[serde(flatten)]
        children: HashMap<String, ChildDiff>,
        #[serde(rename = "s")]
        statics: Option<Statics>,
    },
    UpdateComprehension {
        #[serde(rename = "d")]
        dynamics: DynamicsDiff,
        #[serde(rename = "p")]
        templates: Templates,
        #[serde(rename = "s")]
        statics: Option<Statics>,
    },
    ReplaceCurrent(Fragment),
}

type Templates = Option<HashMap<String, Vec<String>>>;
type DynamicsDiff = Vec<Vec<ChildDiff>>;
type Dynamics = Vec<Vec<Child>>;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Fragment {
    Regular {
        #[serde(rename = "s")]
        statics: Statics,
        #[serde(flatten)]
        children: HashMap<String, Child>,
    },
    Comprehension {
        #[serde(rename = "d")]
        dynamics: Dynamics,
        #[serde(rename = "s")]
        statics: Option<Statics>,
        #[serde(rename = "p")]
        templates: Templates,
    }
}

impl TryFrom<FragmentDiff> for Fragment {
    type Error = MergeError;
    fn try_from(value: FragmentDiff) -> Result<Self, MergeError> {
        match value {
            FragmentDiff::UpdateRegular { children, statics } => {
                let mut new_children : HashMap<String, Child> = HashMap::new();
                for (key, cdiff) in children.into_iter() {
                    new_children.insert(key, cdiff.try_into()?);
                }
                let statics = if let Some(statics) = statics {
                    statics
                } else {
                    Statics::Statics(vec!["".into(); new_children.len()])
                };
                Ok(Self::Regular {
                    children: new_children,
                    statics
                })
            },
            FragmentDiff::ReplaceCurrent(fragment) => Ok(fragment),
            FragmentDiff::UpdateComprehension {
                dynamics,
                templates,
                statics,
            } => {
                let dynamics : Dynamics = dynamics.into_iter().map(|cdiff_vec|
                    cdiff_vec.into_iter().map(|cdiff|
                        cdiff.try_into()
                    ).collect::<Result<Vec<Child>, MergeError>>()
                ).collect::<Result<Vec<Vec<Child>>, MergeError>>()?;
                Ok(Self::Comprehension {
                    dynamics, statics, templates,
                })
            }
        }
    }
}




#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Statics {
    Statics(Vec<String>),
    TemplateRef(i32),
}

impl FragmentMerge for Option<Statics> {
    type DiffItem = Option<Statics>;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        match (self, diff) {
            (None, None) => Ok(None),
            (None, Some(s)) => Ok(Some(s)),
            (Some(s), None) => Ok(Some(s)),
            // Do we merge the vec of statics?
            (Some(_current), Some(new)) => {
                Ok(Some(new))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Child {
    Fragment(Fragment),
    ComponentID(i32),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ChildDiff {
    Fragment(FragmentDiff),
    ComponentID(i32),
    String(String),
}
impl Child {
    pub fn statics(&self) -> Option<Vec<String>> {
        match self {
            Self::Fragment(Fragment::Regular{statics, ..}) => {
                match statics {
                    Statics::Statics(statics) => Some(statics.clone()),
                    _ => None,
                }
            }
            Self::Fragment(Fragment::Comprehension { statics, ..}) => {
                if let Some(Statics::Statics(statics)) = statics {
                    Some(statics.clone())
                } else {
                    None
                }
            }
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
            ComponentDiff::UpdateRegular{..} => {
                Err(MergeError::CreateComponentFromUpdate)
            }
            ComponentDiff::ReplaceCurrent {
                children,
                statics
            } => {
                Ok(Self {
                    children,
                    statics,
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ComponentDiff {
    ReplaceCurrent {
        #[serde(flatten)]
        children: HashMap<String, Child>,
        #[serde(rename = "s")]
        statics: ComponentStatics,
    },
    UpdateRegular {
        #[serde(flatten)]
        children: HashMap<String, ChildDiff>,
    }
}

impl ComponentDiff {
    pub fn to_new_component(self) -> Result<Component, MergeError> {
        self.try_into()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
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
        let components = match (self.components, diff.components) {
            (None, None) => None,
            (None, Some(component_diff)) => {
                let mut components: HashMap<String, Component> = HashMap::new();
                for (key, comp) in component_diff.into_iter() {
                    components.insert(key, comp.to_new_component()?);
                }
                Some(components)
            }
            (Some(components), None) => Some(components),
            (Some(new_components), Some(component_diff)) => {
                Some(new_components.merge(component_diff)?)
            }
        };
        Ok(Self {
            fragment,
            components,
        })
    }
}

impl FragmentMerge for Fragment {
    type DiffItem = FragmentDiff;

    fn merge(self, diff: FragmentDiff) -> Result<Self, MergeError> {
        match (self, diff) {
            (_, FragmentDiff::ReplaceCurrent(new_fragment)) => Ok(new_fragment),
            (
                Fragment::Regular {
                    children: current_children,
                    statics: current_statics,
                },
                FragmentDiff::UpdateRegular {
                    children: children_diffs,
                    ..
                },
            ) => {
                let new_children = current_children.merge(children_diffs)?;
                Ok(Self::Regular {
                    children: new_children,
                    statics: current_statics,
                })
            }
            (
                Fragment::Comprehension {
                    dynamics: _,
                    statics: current_statics,
                    templates: current_templates
                },
                FragmentDiff::UpdateComprehension {
                    dynamics: dynamic_diffs,
                    templates: new_templates,
                    statics: new_statics,
                },
            ) => {
                let templates = current_templates.merge(new_templates)?;
                let new_dynamics: Vec<Vec<Child>> = dynamic_diffs
                    .into_iter()
                    .map(|children_children| {
                        children_children
                            .into_iter()
                            .map(|child| child.to_new_child())
                            .collect::<Result<Vec<Child>, MergeError>>()
                    })
                    .collect::<Result<Vec<Vec<Child>>, MergeError>>()?;
                let statics = current_statics.merge(new_statics)?;
                Ok(Self::Comprehension {
                    dynamics: new_dynamics,
                    statics,
                    templates,
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
                new_components.insert(cid.clone(), comp_diff.to_new_component()?);
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
                statics,
                children,
            } => {
                Ok(Self {
                    children,
                    statics,
                }.fix_statics())
            }
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
            },
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
                return Err(MergeError::AddChildToExisting);
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
}
