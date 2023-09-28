use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
pub struct RootDiff {
    #[serde(flatten)]
    fragment: FragmentDiff,
    #[serde(rename = "c")]
    components: Option<HashMap<String, ComponentDiff>>,
}

#[derive(Debug, PartialEq)]
pub struct Root {
    fragment: Fragment,
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
        let inner = self.fragment.render(&self.components, None)?;
        out.push_str(&inner);
        Ok(out)
    }
}

#[derive(Debug)]
pub enum RenderError {
    NoComponents,
    NoTemplates,
    TemplateNotFound(i32),
    ComponentNotFound(i32),
    ComponentHasTheWrongStatics(i32),
}

impl Fragment {
    pub fn render(&self, components: &Option<HashMap<String, Component>>, cousin_statics: Option<Vec<String>>) -> Result<String, RenderError> {
        let mut out = String::new();
        match &self {
            Fragment::Regular { children, statics } => {
                match statics {
                    Statics::Statics(statics) => {
                        assert!(statics.len() == children.len() + 1);
                        out.push_str(&statics[0]);
                        for i in 1..statics.len() {
                            let child = children.get(&(i - 1).to_string()).expect("Failed to get child");
                            let val = child.render(components, cousin_statics.clone())?;
                            out.push_str(&val);
                            out.push_str(&statics[i]);
                        }
                    }
                    Statics::TemplateRef(template_ref) => {
                        todo!();
                    }
                }
            }
            Fragment::Comprehension { dynamics, statics, templates } => {
                match (statics, cousin_statics) {
                    (None, None) => {
                        for children in dynamics.into_iter() {
                            for child in children.into_iter() {
                                let val = child.render(components, None)?;
                                out.push_str(&val);
                            }
                        }
                    }
                    (None, Some(statics)) => {
                        for children in dynamics.into_iter() {
                            assert!(statics.len() == children.len() + 1);
                            out.push_str(&statics[0]);
                            for i in 1..statics.len() {
                                let child = &children[i - 1];

                                let val = child.render(components, None)?;
                                out.push_str(&val);
                                out.push_str(&statics[i]);
                            }
                        }
                    }
                    (Some(statics), None) => {
                        match statics {
                            Statics::Statics(statics) => {
                                for children in dynamics.into_iter() {
                                    assert!(statics.len() == children.len() + 1);
                                    out.push_str(&statics[0]);
                                    for i in 1..statics.len() {
                                        let child = &children[i - 1];

                                        let val = child.render(components, None)?;
                                        out.push_str(&val);
                                        out.push_str(&statics[i]);
                                    }
                                }
                            }
                            Statics::TemplateRef(template_id) => {
                                if let Some(templates) = templates {
                                    if let Some(template) = templates.templates.get(&template_id.to_string()) {
                                    } else {
                                        return Err(RenderError::TemplateNotFound(*template_id));
                                    }
                                } else {
                                    return Err(RenderError::NoTemplates);
                                }
                            }
                        }
                    }
                    (Some(statics), Some(cousin_templates)) => {
                        panic!("Either statics or cousin statics but not both");
                    }
                }
            }
        }
        Ok(out)
    }
}
impl Child {
    pub fn render(&self, components: &Option<HashMap<String, Component>>, statics: Option<Vec<String>>) -> Result<String, RenderError> {
        match self {
            Child::Fragment(fragment) => fragment.render(components, statics),
            Child::ComponentID(cid) => {
                if let Some(inner_components) = components {
                    if let Some(component) = inner_components.get(&cid.to_string()) {
                        component.to_string_with_components(components)
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
    pub fn to_string_with_components(&self, components: &Option<HashMap<String, Component>>) -> Result<String, RenderError> {
        match &self.statics {
            ComponentStatics::Statics(statics) => {
                let mut out = String::new();
                assert!(statics.len() == self.children.len() + 1);

                out.push_str(&statics[0]);
                for i in 1..statics.len() {
                    let inner = self.children.get(&(i - 1).to_string()).expect("Failed to get child");
                    let val = inner.render(components, None)?;
                    out.push_str(&val);
                    out.push_str(&statics[i]);
                }
                Ok(out)
            }

            ComponentStatics::ComponentRef(cid) => {
                if let Some(inner_components) = components {
                    if let Some(component) = inner_components.get(&cid.to_string()) {

                        if let ComponentStatics::Statics(outer_statics) = &component.statics {
                            let mut out = String::new();
                            assert!(outer_statics.len() == self.children.len() + 1);

                            out.push_str(&outer_statics[0]);
                            for i in 1..outer_statics.len() {
                                let child = self.children.get(&(i - 1).to_string()).expect("Failed to get child");
                                let cousin = component.children.get(&(i - 1).to_string()).expect("Failed to get cousin child for statics");

                                let val = child.render(components, cousin.statics())?;
                                out.push_str(&val);
                                out.push_str(&outer_statics[i]);
                            }
                            Ok(out)
                        } else {
                            Err(RenderError::ComponentHasTheWrongStatics(*cid))
                        }

                    } else {
                        Err(RenderError::ComponentNotFound(*cid))
                    }
                } else {
                    Err(RenderError::NoComponents)
                }

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



#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum FragmentDiff {
    UpdateRegular {
        #[serde(flatten)]
        children: HashMap<String, ChildDiff>,
    },
    UpdateComprehension {
        #[serde(rename = "d")]
        dynamics: DynamicsDiff,
        #[serde(rename = "p")]
        templates: Option<Templates>,
        #[serde(rename = "s")]
        statics: Option<Statics>,
    },
    ReplaceCurrent(Fragment),
}

type DynamicsDiff = Vec<Vec<ChildDiff>>;
type Dynamics = Vec<Vec<Child>>;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Fragment {
    Regular {
        #[serde(flatten)]
        children: HashMap<String, Child>,
        #[serde(rename = "s")]
        statics: Statics,
    },
    Comprehension {
        #[serde(rename = "d")]
        dynamics: Dynamics,
        #[serde(rename = "s")]
        statics: Option<Statics>,
        #[serde(rename = "p")]
        templates: Option<Templates>,
    }
}

impl TryFrom<FragmentDiff> for Fragment {
    type Error = MergeError;
    fn try_from(value: FragmentDiff) -> Result<Self, MergeError> {
        match value {
            FragmentDiff::UpdateRegular { children } => {
                let mut new_children : HashMap<String, Child> = HashMap::new();
                for (key, cdiff) in children.into_iter() {
                    new_children.insert(key, cdiff.try_into()?);
                }
                Err(MergeError::FragmentMissingStatics)
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
pub struct Templates {
    #[serde(flatten)]
    templates: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Statics {
    Statics(Vec<String>),
    TemplateRef(i32),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Child {
    Fragment(Fragment),
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

#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ChildDiff {
    Fragment(FragmentDiff),
    ComponentID(i32),
    String(String),
}
impl TryFrom<ChildDiff> for Child {
    type Error = MergeError;

    fn try_from(value: ChildDiff) -> Result<Self, Self::Error> {
        match value {
            ChildDiff::String(s) => Ok(Child::String(s)),
            ChildDiff::ComponentID(cid) => Ok(Child::ComponentID(cid)),
            ChildDiff::Fragment(fragment_diff) => match fragment_diff {
                FragmentDiff::ReplaceCurrent(fragment) => Ok(Child::Fragment(fragment)),
                FragmentDiff::UpdateRegular {
                    children
                }=> {
                    let mut new_children : HashMap<String, Child> = HashMap::new();
                    for (key, cdiff) in children.into_iter() {
                        new_children.insert(key, cdiff.try_into()?);
                    }
                    Err(MergeError::FragmentTypeMismatch)
                },
                FragmentDiff::UpdateComprehension {
                    templates,
                    dynamics,
                    statics,
                } => {
                    let mut new_dynamics : Dynamics = Vec::new();
                    for i in dynamics {
                        let mut inner_vec : Vec<Child> = Vec::new();
                        for j in i {
                            inner_vec.push(j.try_into()?);
                        }
                        new_dynamics.push(inner_vec);
                    }

                    Ok(Child::Fragment(Fragment::Comprehension {
                        dynamics: new_dynamics,
                        statics,
                        templates: templates,
                    }))
                }
            },
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

#[derive(Debug, PartialEq, Deserialize)]
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
                Ok(Self::Comprehension {
                    dynamics: new_dynamics,
                    statics: new_statics,
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
            } => todo!() //Ok(component.fix_statics()),
        }
    }
}

impl FragmentMerge for Option<Templates> {
    type DiffItem = Option<Templates>;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        match (self, diff) {
            (None, None) => Ok(None),
            (None, Some(template)) => Ok(Some(template)),
            (Some(template), None) => Ok(Some(template)),
            (Some(mut current), Some(new)) => {
                for (key, val) in new.templates.into_iter() {
                    if let Some(curr) = current.templates.get_mut(&key) {
                        curr.extend(val);
                    } else {
                        current.templates.insert(key, val);
                    }
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
            (_, ChildDiff::Fragment(fragment_diff)) => match fragment_diff {
                FragmentDiff::ReplaceCurrent(fragment) => Ok(Self::Fragment(fragment)),
                _ => Err(MergeError::CreateChildFromUpdateFragment),
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

#[derive(Debug)]
pub enum MergeError {
    FragmentTypeMismatch,
    CreateComponentFromUpdate,
    CreateChildFromUpdateFragment,
    AddChildToExisting,
    FragmentMissingStatics,
}

#[cfg(test)]
mod test_merging {
    use super::*;
    #[test]
    fn test_replace() {
        let current = Fragment::Regular {
            children: HashMap::from([("1".into(), Child::String("a".into()))]),
            statics: Statics::Statics(vec!["b".into(), "c".into()]),
        };
        let new = Fragment::Regular {
            children: HashMap::from([("1".into(), Child::String("foo".into()))]),
            statics: Statics::Statics(vec!["bar".into(), "baz".into()]),
        };
        let diff = FragmentDiff::ReplaceCurrent(new.clone());
        let merge = current.merge(diff).expect("Failed to merge diff");
        assert_eq!(merge, new);
    }

    #[test]
    fn simple_diffing() {
        let simple_diff1 = r#"
{
  "0": "cooling",
  "1": "cooling",
  "2": "07:15:03 PM",
  "s": [
    "<div class=\"thermostat\">\n  <div class=\"bar ",
    "\">\n    <a href=\"\#\" phx-click=\"toggle-mode\">",
    "</a>\n    <span>",
    "</span>\n  </div>\n</div>\n",
  ]
  }
  "#;
    let simple_diff2 = r#"
{
  "2": "07:15:04 PM",
}"#;

    }
}
#[cfg(test)]
mod test_stringify {
    use pretty_assertions::{assert_eq, assert_ne};
    use super::*;
    use crate::dom::Document;
    #[test]
    fn fragment_render_parse() {
        let expected = Root {
            fragment: Fragment::Regular{
                children: HashMap::from([
                    ("1".into(), Child::String("foo".into())),
                    ("2".into(), Child::ComponentID(1)),
                ]),
                statics: Statics::Statics(vec!["1".into(), "2".into(), "3".into()]),
            },
            components: Some(HashMap::from([(
                "1".into(),
                Component {
                    children: HashMap::from([("1".into(), Child::String("bar".into()))]),
                    statics: ComponentStatics::Statics(vec!["4".into(), "5".into()]),
                },
            )])),
        };
        let input = "1foo24bar53";
        let document = Document::parse(input);
        println!("DOCUMENT: {document:?}");
        let document = document.expect("Failed to parse document");
        //println!("DOCUMENT: {}", document.);
    }
    #[test]
    fn simple_diff_render() {
        let simple_diff1= r#"{
  "0": "cooling",
  "1": "cooling",
  "2": "07:15:04 PM",
  "s": [
    "<div class=\"thermostat\">\n  <div class=\"bar ",
    "\">\n    <a href=\"\\#\" phx-click=\"toggle-mode\">",
    "</a>\n    <span>",
    "</span>\n  </div>\n</div>\n"
  ]
}"#;
let expected = r#"<div class="thermostat">
  <div class="bar cooling">
    <a href="\#" phx-click="toggle-mode">cooling</a>
    <span>07:15:04 PM</span>
  </div>
</div>
"#;
        let root : RootDiff = serde_json::from_str(simple_diff1).expect("Failed to deserialize fragment");
        println!("root diff: {root:#?}");
        let root : Root = root.try_into().expect("Failed to convert RootDiff to Root");
        println!("root diff: {root:#?}");
        let out : String = root.try_into().expect("Failed to convert Root into string");
        assert_eq!(out, expected);
    }

    #[test]
    fn json_to_fragment_to_string() {
        let fragment_json = r#"
{
  "0": {
    "d": [
          ["foo", {"d": [["0", 1], ["1", 2]], "s": 0}],
          ["bar", {"d": [["0", 3], ["1", 4]], "s": 0}]
    ],
    "s": ["\n  <p>\n    ", "\n    ", "\n  </p>\n"],
    "p": {"0": ["<span>", ": ", "</span>"]}
  },
  "c": {
    "1": {"0": "index_1", "1": "world", "s": ["<b>FROM ", " ", "</b>"]},
    "2": {"0": "index_2", "1": "world", "s": 1},
    "3": {"0": "index_1", "1": "world", "s": 1},
    "4": {"0": "index_2", "1": "world", "s": 3}
  },
  "s": ["<div>", "</div>"]
}
"#;
        let root : RootDiff = serde_json::from_str(fragment_json).expect("Failed to deserialize fragment");
        println!("{root:#?}");
        let root : Root = root.try_into().expect("Failed to convert RootDiff to Root");
        println!("root diff: {root:#?}");
        let out : String = root.try_into().expect("Failed to convert Root into string");

        let expected = r#"<div>
  <p>
    foo
    <span>0: <b data-phx-component="1" id="123-1-0">FROM index_1 world</b></span><span>1: <b data-phx-component="2" id="123-2-0">FROM index_2 world</b></span>
  </p>

  <p>
    bar
    <span>0: <b data-phx-component="3" id="123-3-0">FROM index_1 world</b></span><span>1: <b data-phx-component="4" id="123-4-0">FROM index_2 world</b></span>
  </p>
</div>"#;
        assert_eq!(out, expected);
    }
    #[test]
    fn fragment_with_components_with_static_component_refs() {
        let input_json = r#"
        {
            "0": {
                "0": {
                    "d": [
                        [
                            1
                        ],
                        [
                            2
                        ],
                        [
                            3
                        ]
                    ],
                    "s": [
                        "\n  ",
                        "\n"
                    ]
                },
                "s": [
                    "",
                    ""
                ]
            },
            "c": {
                "1": {
                    "0": {
                        "d": [
                            [
                                "3"
                            ],
                            [
                                "4"
                            ],
                            [
                                "5"
                            ]
                        ],
                        "s": [
                            "\n    <Text>Item ",
                            "</Text>\n"
                        ]
                    },
                    "s": [
                        "<Group>\n",
                        "\n</Group>"
                    ]
                },
                "2": {
                    "0": {
                        "d": [
                            [
                                "6"
                            ],
                            [
                                "7"
                            ],
                            [
                                "8"
                            ]
                        ]
                    },
                    "s": 1
                },
                "3": {
                    "0": {
                        "d": [
                            [
                                "9"
                            ],
                            [
                                "10"
                            ],
                            [
                                "11"
                            ]
                        ]
                    },
                    "s": 1
                }
            },
            "s": [
                "<div>",
                "</div>"
            ]
        }"#;
        let root : RootDiff = serde_json::from_str(input_json).expect("Failed to deserialize fragment");
        //println!("{root:#?}");
        let root : Root = root.try_into().expect("Failed to convert RootDiff to Root");
        println!("root diff: {root:#?}");
        let out : String = root.try_into().expect("Failed to convert Root into string");
        println!("out: {out}");
        let expected = r#"<div>
  <Group>

    <Text>Item 3</Text>

    <Text>Item 4</Text>

    <Text>Item 5</Text>

</Group>

  <Group>

    <Text>Item 6</Text>

    <Text>Item 7</Text>

    <Text>Item 8</Text>

</Group>

  <Group>

    <Text>Item 9</Text>

    <Text>Item 10</Text>

    <Text>Item 11</Text>

</Group>
</div>"#;
        assert_eq!(out, expected);
    }

    #[test]
    fn fragment_with_dynamic_component() {
        let input_json = r#"
        {
            "0": {
                "0": {
                    "d": [
                        [
                            1
                        ]
                    ],
                    "s": [
                        "\n  ",
                        "\n"
                    ]
                },
                "s": [
                    "",
                    ""
                ]
            },
            "c": {
                "1": {
                    "0": {
                        "d": [
                            [
                                "3"
                            ],
                            [
                                "4"
                            ],
                            [
                                "5"
                            ]
                        ],
                        "s": [
                            "\n    <Text>Item ",
                            "</Text>\n"
                        ]
                    },
                    "s": [
                        "<Group>\n",
                        "\n</Group>"
                    ]
                }
            },
            "s": [
                "<div>",
                "</div>"
            ]
        }"#;
        let root : RootDiff = serde_json::from_str(input_json).expect("Failed to deserialize fragment");
        //println!("{root:#?}");
        let root : Root = root.try_into().expect("Failed to convert RootDiff to Root");
        println!("root diff: {root:#?}");
        let out : String = root.try_into().expect("Failed to convert Root into string");
        println!("out: {out}");
        let expected = r#"<div>
  <Group>

    <Text>Item 3</Text>

    <Text>Item 4</Text>

    <Text>Item 5</Text>

</Group>
</div>"#;
        assert_eq!(out, expected);
    }
}
#[cfg(test)]
mod test_json_decoding {
    use super::*;

    #[test]
    fn simple() {
        let data = r#"
        {
            "1": "baz"
        }
        "#;
        let out: Result<FragmentDiff, _> = serde_json::from_str(data);
        println!("{out:?}");
        assert!(out.is_ok());
        let out = out.expect("Failed to deserialize");
        let expected = FragmentDiff::UpdateRegular {
            children: HashMap::from([(1.to_string(), ChildDiff::String("baz".into()))]),
        };
        assert_eq!(out, expected);
    }
    #[test]
    fn simple_component_diff() {
        let diffs = vec![
            r#"{"0": "index_2", "1": "world", "s": 1}"#,
            r#"{"0": "index_1", "1": "world", "s": 1}"#,
            r#"{"0": "index_2", "1": "world", "s": 3}"#,
            r#"{"0": "index_1", "1": "world", "s": ["<b>FROM ", " ", "</b>"]}"#,
        ];
        for data in &diffs {
            let out: Result<ComponentDiff, _> = serde_json::from_str(data);
            println!("{data}, {out:?}");
            assert!(out.is_ok());
            let out = out.expect("Failed to deserialize");
        }
    }


    #[test]
    fn test_decode_simple() {
        let data = r#"
        {
            "0": "foo",
            "1": "bar",
            "s": [
                "a",
                "b"
            ]
        }
        "#;
        let out: Result<FragmentDiff, _> = serde_json::from_str(data);
        println!("{out:?}");
        assert!(out.is_ok());
        let out = out.expect("Failed to deserialize");
        let expected = FragmentDiff::ReplaceCurrent(Fragment::Regular {
            children: HashMap::from([
                ("0".into(), Child::String("foo".into())),
                ("1".into(), Child::String("bar".into())),
            ]),
            statics: Statics::Statics(vec!["a".into(), "b".into()]),
        });
        assert_eq!(out, expected);
    }

    #[test]
    fn test_decode_comprehension_with_templates() {
        let data = r#"
        {
            "d": [
                ["foo", 1],
                ["bar", 1]
            ],
            "p": {
                "0": [
                    "\\n    bar ",
                    "\\n  "
                ]
            }
        }
        "#;
        let out: Result<FragmentDiff, _> = serde_json::from_str(data);
        println!("{out:#?}");
        assert!(out.is_ok());
        let out = out.expect("Failed to deserialize");
        let expected = FragmentDiff::UpdateComprehension {
            dynamics: vec![
                vec![ChildDiff::String("foo".into()), ChildDiff::ComponentID(1)],
                vec![ChildDiff::String("bar".into()), ChildDiff::ComponentID(1)],
            ],
            statics: None,
            templates: Some(Templates {
                templates: HashMap::from([(
                    "0".into(),
                    vec!["\\n    bar ".into(), "\\n  ".into()],
                )]),
            }),
        };
        assert_eq!(out, expected);
    }

    #[test]
    fn test_decode_comprehension_without_templates() {
        let data = r#"
        {
            "d": [
                ["foo", 1],
                ["bar", 1]
            ]
        }
        "#;
        let out: Result<FragmentDiff, _> = serde_json::from_str(data);
        println!("{out:?}");
        assert!(out.is_ok());
        let out = out.expect("Failed to deserialize");
        let expected = FragmentDiff::UpdateComprehension {
            dynamics: vec![
                vec![ChildDiff::String("foo".into()), ChildDiff::ComponentID(1)],
                vec![ChildDiff::String("bar".into()), ChildDiff::ComponentID(1)],
            ],
            statics: None,
            templates: None,
        };
        assert_eq!(out, expected);
    }

    #[test]
    fn test_decode_component_diff() {
        let data = r#"
        {
            "0": {
                "0": 1
            },
            "c": {
                "1": {
                    "0": {
                        "d": [
                            [
                                "0",
                                "foo"
                            ],
                            [
                                "1",
                                "bar"
                            ]
                        ]
                    }
                }
            }
        }
        "#;
        let out: Result<RootDiff, _> = serde_json::from_str(data);
        println!("{out:?}");
        assert!(out.is_ok());
        let out = out.expect("Failed to deserialize");
        let expected = RootDiff {
            fragment: FragmentDiff::UpdateRegular {
                children: HashMap::from([(
                    "0".into(),
                    ChildDiff::Fragment(FragmentDiff::UpdateRegular {
                        children: HashMap::from([("0".into(), ChildDiff::ComponentID(1))]),
                    }),
                )]),
            },
            components: Some(HashMap::from([(
                "1".into(),
                ComponentDiff::UpdateRegular {
                    children: HashMap::from([(
                        "0".into(),
                        ChildDiff::Fragment(FragmentDiff::UpdateComprehension {
                                dynamics: vec![
                                    vec![
                                        ChildDiff::String("0".into()),
                                        ChildDiff::String("foo".into()),
                                    ],
                                    vec![
                                        ChildDiff::String("1".into()),
                                        ChildDiff::String("bar".into()),
                                    ],
                                ],
                                statics: None,
                                templates: None,
                            },
                        ),
                    )]),
                },
            )])),
        };
        assert_eq!(out, expected);
    }

    #[test]
    fn test_decode_root_diff() {
        let data = r#"
        {
            "0": {
                "0": 1
            }
        }
        "#;
        let out: Result<RootDiff, _> = serde_json::from_str(data);
        println!("{out:?}");
        assert!(out.is_ok());
        let out = out.expect("Failed to deserialize");
        let expected = RootDiff {
            fragment: FragmentDiff::UpdateRegular {
                children: HashMap::from([(
                    "0".into(),
                    ChildDiff::Fragment(FragmentDiff::UpdateRegular {
                        children: HashMap::from([("0".into(), ChildDiff::ComponentID(1))]),
                    }),
                )]),
            },
            components: None,
        };
        assert_eq!(out, expected);
    }
    #[test]
    fn test_decode_component_with_dynamics_iterated() {
        let input = r#"
        {
            "0": {
                "0": {
                    "d": [
                        [
                            1
                        ],
                        [
                            2
                        ],
                        [
                            3
                        ]
                    ],
                    "s": [
                        "\n  ",
                        "\n"
                    ]
                },
                "s": [
                    "",
                    ""
                ]
            },
            "c": {
                "1": {
                    "0": {
                        "d": [
                            [
                                "1"
                            ],
                            [
                                "2"
                            ],
                            [
                                "3"
                            ]
                        ],
                        "s": [
                            "\n    <Text>Item ",
                            "</Text>\n  "
                        ]
                    },
                    "s": [
                        "<Group>\n  ",
                        "\n</Group>"
                    ]
                },
                "2": {
                    "0": {
                        "d": [
                            [
                                "1"
                            ],
                            [
                                "2"
                            ],
                            [
                                "3"
                            ]
                        ]
                    },
                    "s": 1
                },
                "3": {
                    "0": {
                        "d": [
                            [
                                "1"
                            ],
                            [
                                "2"
                            ],
                            [
                                "3"
                            ]
                        ]
                    },
                    "s": 1
                }
            },
            "s": [
                "",
                ""
            ]
        }"#;

    }
}
