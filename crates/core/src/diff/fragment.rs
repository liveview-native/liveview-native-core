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
#[derive(Debug, PartialEq, Deserialize)]
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


#[derive(Debug, PartialEq, Deserialize)]
pub struct Root {
    fragment: Fragment,
    components: HashMap<String, Component>,
}
#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Fragment {
    Regular(RegularFragment),
    Comprehension(Vec<Vec<Child>>, Statics, Option<Templates>)
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct RegularFragment {
    #[serde(flatten)]
    children: HashMap<String, Child>,
    #[serde(rename = "s")]
    statics: Statics,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Statics {
    Statics(Vec<String>),
    TemplateRef(i32),
    ComponentRef,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Child {
    Fragment(Fragment),
    ComponentID(i32),
    String(String),
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Component {
    #[serde(flatten)]
    children: HashMap<String, Child>,
    #[serde(flatten)]
    statics: ComponentStatics,
}
#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ComponentStatics {
    Statics(Vec<String>),
    ComponentRef(i32),
}


#[cfg(test)]
mod tests {
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


pub trait FragmentMerge {
    type DiffItem;
    fn merge(diff: Self::DiffItem) -> Self;
}
impl FragmentMerge for Fragment {
    type DiffItem = FragmentDiff;

    fn merge(diff: Self::DiffItem) -> Self {
        todo!()
    }
}
