use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Deserialize)]
pub struct RootDiff {
    #[serde(flatten)]
    fragment: FragmentDiff,
    #[serde(rename = "c")]
    components: Option<HashMap<String, ComponentDiff>>
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Root {
    fragment: Fragment,
    components: Option<HashMap<String, Component>>
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum FragmentDiff {
    UpdateRegular(UpdateRegular),
    ReplaceCurrent(Fragment),
    UpdateComprehension(UpdateComprehension),
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct UpdateRegular {
    #[serde(flatten)]
    children: HashMap<String, ChildDiff>
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct UpdateComprehension {
    #[serde(rename = "d")]
    dynamics: Vec<Vec<ChildDiff>>,
    #[serde(rename = "p")]
    templates: Option<Templates>
}


#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ComponentDiff {
    UpdateRegular(UpdateRegular),
    ReplaceCurrent(Component),
}

impl ComponentDiff {
    pub fn to_new_component(self) -> Result<Component, MergeError> {
        match self {
            ComponentDiff::UpdateRegular(_) => {
                Err(MergeError::CreateComponentFromUpdate)
            }
            ComponentDiff::ReplaceCurrent(component) => {
                Ok(component.fix_statics())
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Templates {
    #[serde(flatten)]
    templates: HashMap<String, Vec<String>>,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ChildDiff {
    String(String),
    ComponentID(i32),
    Fragment(FragmentDiff),
}

impl ChildDiff {
    pub fn to_new_child(self) -> Result<Child, MergeError> {
        match self {
            ChildDiff::String(s) => {
                Ok(Child::String(s))
            }
            ChildDiff::ComponentID(cid) => {
                Ok(Child::ComponentID(cid))
            }
            ChildDiff::Fragment(fragment_diff) => {
                match fragment_diff {
                    FragmentDiff::ReplaceCurrent(fragment) => {
                        Ok(Child::Fragment(fragment))
                    }
                    _ => {
                        Err(MergeError::CreateChildFromUpdateFragment)
                    }
                }
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Fragment {
    Regular(RegularFragment),
    Comprehension(Vec<Vec<Child>>, Statics, Option<Templates>)
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RegularFragment {
    #[serde(flatten)]
    children: HashMap<String, Child>,
    #[serde(rename = "s")]
    statics: Statics,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Statics {
    Statics(Vec<String>),
    TemplateRef(i32),
    ComponentRef,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Child {
    Fragment(Fragment),
    ComponentID(i32),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Component {
    #[serde(flatten)]
    children: HashMap<String, Child>,
    #[serde(flatten)]
    statics: ComponentStatics,
}

impl Component {
    pub fn fix_statics(self) -> Self {
        match self.statics {
            ComponentStatics::ComponentRef(cid) if cid < 0 => {
                Self {
                    children: self.children,
                    statics: ComponentStatics::ComponentRef(-cid),
                }
            }
            _ => self,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ComponentStatics {
    Statics(Vec<String>),
    ComponentRef(i32),
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
        let out : Result<FragmentDiff, _> = serde_json::from_str(data);
        println!("{out:?}");
        assert!(out.is_ok());
        let out = out.expect("Failed to deserialize");
        let expected = FragmentDiff::UpdateRegular(
            UpdateRegular { children: HashMap::from([
                (1.to_string(), ChildDiff::String("baz".into())),
            ]),
            }
        );
        assert_eq!(out, expected);
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
        let out : Result<FragmentDiff, _> = serde_json::from_str(data);
        println!("{out:?}");
        assert!(out.is_ok());
        let out = out.expect("Failed to deserialize");
        let expected = FragmentDiff::ReplaceCurrent(
            Fragment::Regular(
                RegularFragment {
                    children: HashMap::from([
                        ("0".into(), Child::String("foo".into())),
                        ("1".into(), Child::String("bar".into())),
                    ]),
                    statics: Statics::Statics(
                        vec![
                        "a".into(),
                        "b".into(),
                        ]
                    )
                }
            )
        );
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
        let out : Result<UpdateComprehension, _> = serde_json::from_str(data);
        println!("{out:?}");
        assert!(out.is_ok());
        let out : Result<FragmentDiff, _> = serde_json::from_str(data);
        println!("{out:?}");
        assert!(out.is_ok());
        let out = out.expect("Failed to deserialize");
        let expected = FragmentDiff::UpdateComprehension(
            UpdateComprehension {
                dynamics: vec![
                    vec![ChildDiff::String("foo".into()), ChildDiff::ComponentID(1)],
                    vec![ChildDiff::String("bar".into()), ChildDiff::ComponentID(1)],
                ],
                templates: Some(
                    Templates {
                        templates: HashMap::from([
                            ("0".into(), vec!["\\n    bar ".into(), "\\n  ".into()]),
                        ]),
                    },
                ),
            }
        );
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
        let out : Result<FragmentDiff, _> = serde_json::from_str(data);
        println!("{out:?}");
        assert!(out.is_ok());
        let out = out.expect("Failed to deserialize");
        let expected = FragmentDiff::UpdateComprehension(
            UpdateComprehension {
                dynamics: vec![
                    vec![ChildDiff::String("foo".into()), ChildDiff::ComponentID(1)],
                    vec![ChildDiff::String("bar".into()), ChildDiff::ComponentID(1)],
                ],
                templates: None,
            }
        );
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
        let out : Result<RootDiff, _> = serde_json::from_str(data);
        println!("{out:?}");
        assert!(out.is_ok());
        let out = out.expect("Failed to deserialize");
        let expected = RootDiff {
            fragment: FragmentDiff::UpdateRegular(UpdateRegular {
                children: HashMap::from([
                    ("0".into(), ChildDiff::Fragment(FragmentDiff::UpdateRegular(
                                UpdateRegular {
                                    children: HashMap::from([
                                                  ("0".into(), ChildDiff::ComponentID(1))
                                    ]),
                                }
                    )))
                ]),
            }),
            components: Some(
                            HashMap::from([
                                (
                                    "1".into(),
                                    ComponentDiff::UpdateRegular(
                                        UpdateRegular {
                                            children: HashMap::from([(
                                                "0".into(),
                                                ChildDiff::Fragment(FragmentDiff::UpdateComprehension(
                                                        UpdateComprehension {
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
                                                            templates: None,
                                                        }
                                                ))
                                            )]),
                                        }
                                    )
                                ),
                            ]),
                        ),
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
        let out : Result<RootDiff, _> = serde_json::from_str(data);
        println!("{out:?}");
        assert!(out.is_ok());
        let out = out.expect("Failed to deserialize");
        let expected = RootDiff {
            fragment: FragmentDiff::UpdateRegular(UpdateRegular {
                children: HashMap::from([
                    ("0".into(), ChildDiff::Fragment(FragmentDiff::UpdateRegular(
                                UpdateRegular {
                                    children: HashMap::from([
                                                  ("0".into(), ChildDiff::ComponentID(1))
                                    ]),
                                }
                    )))
                ]),
            }),
            components: None,
        };
        assert_eq!(out, expected);
    }
}


pub trait FragmentMerge : Sized {
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
            (_, FragmentDiff::ReplaceCurrent(new_fragment)) => {
                Ok(new_fragment)
            }
            (Fragment::Regular(RegularFragment {children: current_children, statics: current_statics }), FragmentDiff::UpdateRegular(UpdateRegular { children: children_diffs})) => {
                let new_children = current_children.merge(children_diffs)?;
                Ok(Self::Regular(RegularFragment {
                    children: new_children,
                    statics: current_statics,
                }))
            }
            (Fragment::Comprehension(_, current_statics, current_templates), FragmentDiff::UpdateComprehension(UpdateComprehension{ dynamics: dynamic_diffs, templates: new_templates})) => {
                let templates = current_templates.merge(new_templates)?;
                let new_dynamics : Vec<Vec<Child>> = dynamic_diffs.into_iter().map(|children_children| {
                    children_children.into_iter().map(|child| child.to_new_child()).collect::<Result<Vec<Child>, MergeError>>()
                }).collect::<Result<Vec<Vec<Child>>, MergeError>>()?;
                Ok(Self::Comprehension(new_dynamics, current_statics, templates))
            }

            _ => {
                Err(MergeError::FragmentTypeMismatch)
            }
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
            ComponentDiff::UpdateRegular(UpdateRegular{ children: children_diffs }) => {
                let new_children = self.children.merge(children_diffs)?;
                Ok(Self {
                    children: new_children,
                    statics: self.statics,
                })
            }
            ComponentDiff::ReplaceCurrent(component) => {
                Ok(component.fix_statics())
            }
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
            },
            (_, ChildDiff::String(s)) => {
                Ok(Self::String(s))
            }
            (_, ChildDiff::ComponentID(id)) => {
                Ok(Self::ComponentID(id))
            },
            (_, ChildDiff::Fragment(fragment_diff)) => {
                match fragment_diff {
                    FragmentDiff::ReplaceCurrent(fragment) => {
                        Ok(Self::Fragment(fragment))
                    }
                    _ => {
                        Err(MergeError::CreateChildFromUpdateFragment)
                    }
                }
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
}

#[cfg(test)]
mod test_merging {
    use super::*;
    #[test]
    fn test_replace() {
        let current = Fragment::Regular(RegularFragment {
            children: HashMap::from([
                          ("1".into(), Child::String("a".into())),
            ]),
            statics: Statics::Statics(
                vec![
                "b".into(),
                "c".into(),
                ],
            ),
        });
        let new = Fragment::Regular(RegularFragment {
            children: HashMap::from([
                          ("1".into(), Child::String("foo".into())),
            ]),
            statics: Statics::Statics(
                vec![
                "bar".into(),
                "baz".into(),
                ],
            ),
        });
        let diff = FragmentDiff::ReplaceCurrent(new.clone());
        let merge = current.merge(diff).expect("Failed to merge diff");
        assert_eq!(merge, new);


    }
}
