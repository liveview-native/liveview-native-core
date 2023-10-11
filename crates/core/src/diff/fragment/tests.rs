use pretty_assertions::assert_eq;

use super::*;

#[test]
fn test_from_jetpack_ui() {
    let initial_json = r#"{
        "0":"0",
        "s":["\u003cScaffold\u003e\n  \u003cTopAppBar\u003e\n    \u003cTitle\u003e\u003cText\u003eHello\u003c/Text\u003e\u003c/Title\u003e\n  \u003c/TopAppBar\u003e\n  \u003cColumn width\u003d\"fill\" verticalArrangement\u003d\"center\" horizontalAlignment\u003d\"center\"\u003e\n    \u003cText style\u003d\"headlineLarge\"\u003eTitle\u003c/Text\u003e\n    \u003cCard shape\u003d\"8\" padding\u003d\"16\" width\u003d\"140\" height\u003d\"120\" elevation\u003d\"{\u0027defaultElevation\u0027: \u002710\u0027, \u0027pressedElevation\u0027: \u00272\u0027}\" phx-click\u003d\"dec\"\u003e\n      \u003cText padding\u003d\"16\"\u003eHello Jetpack!\u003c/Text\u003e\n    \u003c/Card\u003e\n    \u003cSpacer height\u003d\"8\"\u003e\u003c/Spacer\u003e\n    \u003cCard padding\u003d\"16\"\u003e\n      \u003cText padding\u003d\"16\"\u003eSimple card\u003c/Text\u003e\n    \u003c/Card\u003e\n    \u003cButton phx-click\u003d\"navigate\" contentPadding\u003d\"50\" elevation\u003d\"{\u0027defaultElevation\u0027: \u002720\u0027, \u0027pressedElevation\u0027: \u002710\u0027}\"\u003e\n      \u003cText\u003eNavigate to counter\u003c/Text\u003e\n    \u003c/Button\u003e\n    \u003cButton phx-click\u003d\"redirect\"\u003e\u003cText\u003eRedirect to counter\u003c/Text\u003e\u003c/Button\u003e\n    \u003cIconButton phx-click\u003d\"inc\" colors\u003d\"{\u0027containerColor\u0027: \u0027#FFFF0000\u0027, \u0027contentColor\u0027: \u0027#FFFFFFFF\u0027}\"\u003e\n      \u003cIcon imageVector\u003d\"filled:Add\"\u003e\u003c/Icon\u003e\n    \u003c/IconButton\u003e\n    \u003cRow verticalAlignment\u003d\"center\"\u003e\n      \u003cButton phx-click\u003d\"dec\" shape\u003d\"circle\" size\u003d\"60\"\u003e\n        \u003cText\u003e-\u003c/Text\u003e\n      \u003c/Button\u003e\n      \u003cText\u003eThis counter: ","\u003c/Text\u003e\n      \u003cButton phx-click\u003d\"inc\" shape\u003d\"circle\" size\u003d\"60\"\u003e\u003cText\u003e+\u003c/Text\u003e\u003c/Button\u003e\n    \u003c/Row\u003e\n  \u003c/Column\u003e\n\u003c/Scaffold\u003e"]}"#;
    let root: RootDiff = serde_json::from_str(initial_json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let increment_diff = r#"{"0": "1"}"#;
    let other_root : RootDiff = serde_json::from_str(increment_diff).expect("Failed to deserialize diff fragment");
    let new_root = root.merge(other_root).expect("Failed to merge new root in");
    let expected_root = r#"{
        "0":"1",
        "s":["\u003cScaffold\u003e\n  \u003cTopAppBar\u003e\n    \u003cTitle\u003e\u003cText\u003eHello\u003c/Text\u003e\u003c/Title\u003e\n  \u003c/TopAppBar\u003e\n  \u003cColumn width\u003d\"fill\" verticalArrangement\u003d\"center\" horizontalAlignment\u003d\"center\"\u003e\n    \u003cText style\u003d\"headlineLarge\"\u003eTitle\u003c/Text\u003e\n    \u003cCard shape\u003d\"8\" padding\u003d\"16\" width\u003d\"140\" height\u003d\"120\" elevation\u003d\"{\u0027defaultElevation\u0027: \u002710\u0027, \u0027pressedElevation\u0027: \u00272\u0027}\" phx-click\u003d\"dec\"\u003e\n      \u003cText padding\u003d\"16\"\u003eHello Jetpack!\u003c/Text\u003e\n    \u003c/Card\u003e\n    \u003cSpacer height\u003d\"8\"\u003e\u003c/Spacer\u003e\n    \u003cCard padding\u003d\"16\"\u003e\n      \u003cText padding\u003d\"16\"\u003eSimple card\u003c/Text\u003e\n    \u003c/Card\u003e\n    \u003cButton phx-click\u003d\"navigate\" contentPadding\u003d\"50\" elevation\u003d\"{\u0027defaultElevation\u0027: \u002720\u0027, \u0027pressedElevation\u0027: \u002710\u0027}\"\u003e\n      \u003cText\u003eNavigate to counter\u003c/Text\u003e\n    \u003c/Button\u003e\n    \u003cButton phx-click\u003d\"redirect\"\u003e\u003cText\u003eRedirect to counter\u003c/Text\u003e\u003c/Button\u003e\n    \u003cIconButton phx-click\u003d\"inc\" colors\u003d\"{\u0027containerColor\u0027: \u0027#FFFF0000\u0027, \u0027contentColor\u0027: \u0027#FFFFFFFF\u0027}\"\u003e\n      \u003cIcon imageVector\u003d\"filled:Add\"\u003e\u003c/Icon\u003e\n    \u003c/IconButton\u003e\n    \u003cRow verticalAlignment\u003d\"center\"\u003e\n      \u003cButton phx-click\u003d\"dec\" shape\u003d\"circle\" size\u003d\"60\"\u003e\n        \u003cText\u003e-\u003c/Text\u003e\n      \u003c/Button\u003e\n      \u003cText\u003eThis counter: ","\u003c/Text\u003e\n      \u003cButton phx-click\u003d\"inc\" shape\u003d\"circle\" size\u003d\"60\"\u003e\u003cText\u003e+\u003c/Text\u003e\u003c/Button\u003e\n    \u003c/Row\u003e\n  \u003c/Column\u003e\n\u003c/Scaffold\u003e"]}"#;
    let expected_root: RootDiff = serde_json::from_str(expected_root).expect("Failed to deserialize fragment");
    let expected_root: Root = expected_root.try_into().expect("Failed to convert RootDiff to Root");
    assert_eq!(expected_root, new_root);

    let out : String = new_root.try_into().expect("Failed to convert root to string");
    println!("diff: {out}");

    }
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
fn fragment_render_parse() {
    let root = Root {
        fragment: Fragment::Regular {
            children: HashMap::from([
                ("0".into(), Child::String("foo".into())),
                ("1".into(), Child::ComponentID(1)),
            ]),
            statics: Statics::Statics(vec!["1".into(), "2".into(), "3".into()]),
        },
        components: Some(HashMap::from([(
            "1".into(),
            Component {
                children: HashMap::from([("0".into(), Child::String("bar".into()))]),
                statics: ComponentStatics::Statics(vec!["4".into(), "5".into()]),
            },
        )])),
    };
    let expected = "1foo24bar53";
    let out: String = root.try_into().expect("Failed to render root");
    assert_eq!(out, expected);
}

#[test]
fn simple_diff_render() {
    let simple_diff1 = r#"{
  "0": "cooling",
  "1": "cooling",
  "2": "07:15:03 PM",
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
    <span>07:15:03 PM</span>
  </div>
</div>
"#;
    let root: RootDiff =
        serde_json::from_str(simple_diff1).expect("Failed to deserialize fragment");
    println!("root diff: {root:#?}");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    println!("root diff: {root:#?}");
    let out: String = root.try_into().expect("Failed to convert Root into string");
    assert_eq!(out, expected);
}

#[test]
fn simple_diff_merge_and_render() {
    let simple_diff1 = r#"{
  "0": "cooling",
  "1": "cooling",
  "2": "07:15:03 PM",
  "s": [
    "<div class=\"thermostat\">\n  <div class=\"bar ",
    "\">\n    <a href=\"\\#\" phx-click=\"toggle-mode\">",
    "</a>\n    <span>",
    "</span>\n  </div>\n</div>\n"
  ]
}"#;
    let root: RootDiff =
        serde_json::from_str(simple_diff1).expect("Failed to deserialize fragment");
    println!("root diff: {root:#?}");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let simple_diff2 = r#"{"2": "07:15:04 PM"}"#;
    let root_diff: RootDiff =
        serde_json::from_str(simple_diff2).expect("Failed to deserialize fragment");
    let root = root
        .merge(root_diff)
        .expect("Failed to merge diff into root");
    println!("root diff: {root:#?}");
    let out: String = root.try_into().expect("Failed to convert Root into string");
    let expected = r#"<div class="thermostat">
  <div class="bar cooling">
    <a href="\#" phx-click="toggle-mode">cooling</a>
    <span>07:15:04 PM</span>
  </div>
</div>
"#;
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
    let root: RootDiff =
        serde_json::from_str(fragment_json).expect("Failed to deserialize fragment");
    println!("{root:#?}");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    println!("root diff: {root:#?}");
    let out: String = root.try_into().expect("Failed to convert Root into string");

    let expected = r#"<div>
  <p>
    foo
    <span>0: <b>FROM index_1 world</b></span><span>1: <b>FROM index_2 world</b></span>
  </p>

  <p>
    bar
    <span>0: <b>FROM index_1 world</b></span><span>1: <b>FROM index_2 world</b></span>
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
                        "\n  </Group>"
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
    let root: RootDiff = serde_json::from_str(input_json).expect("Failed to deserialize fragment");
    //println!("{root:#?}");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    println!("root diff: {root:#?}");
    let out: String = root.try_into().expect("Failed to convert Root into string");
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
                        "\n  </Group>"
                    ]
                }
            },
            "s": [
                "<div>",
                "</div>"
            ]
        }"#;
    let root: RootDiff = serde_json::from_str(input_json).expect("Failed to deserialize fragment");
    //println!("{root:#?}");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    println!("root diff: {root:#?}");
    let out: String = root.try_into().expect("Failed to convert Root into string");
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
#[test]
fn deep_diff_merging() {
    let deep_diff1 = r#"{
  "0": {
    "0": {
      "d": [["user1058", "1"], ["user99", "1"]],
      "s": ["        <tr>\n          <td>", " (", ")</td>\n        </tr>\n"]
    },
    "s": [
      "  <table>\n    <thead>\n      <tr>\n        <th>Username</th>\n        <th></th>\n      </tr>\n    </thead>\n    <tbody>\n",
      "    </tbody>\n  </table>\n"
    ]
  },
  "1": {
    "d": [
      [
        "asdf_asdf",
        "asdf@asdf.com",
        "123-456-7890",
        "<a href=\"/users/1\">Show</a>",
        "<a href=\"/users/1/edit\">Edit</a>",
        "<a href=\"\\#\" phx-click=\"delete_user\" phx-value=\"1\">Delete</a>"
      ]
    ],
    "s": [
      "    <tr>\n      <td>",
      "</td>\n      <td>",
      "</td>\n      <td>",
      "</td>\n\n      <td>\n",
      "        ",
      "\n",
      "      </td>\n    </tr>\n"
    ]
  }
}"#;
    let root: RootDiff = serde_json::from_str(deep_diff1).expect("Failed to deserialize fragment");
    println!("root - {root:#?}");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");

    let deep_diff2 = r#"{
  "0": {
    "0": {
      "d": [["user1058", "2"]]
    }
  }
}"#;
    let root_diff: RootDiff =
        serde_json::from_str(deep_diff2).expect("Failed to deserialize fragment");
    let root = root.merge(root_diff).expect("Failed to merge root");
    let deep_diff_result = r#" {
  "0": {
    "0": {
      "d": [["user1058", "2"]],
      "s": ["        <tr>\n          <td>", " (", ")</td>\n        </tr>\n"]
    },
    "s": [
      "  <table>\n    <thead>\n      <tr>\n        <th>Username</th>\n        <th></th>\n      </tr>\n    </thead>\n    <tbody>\n",
      "    </tbody>\n  </table>\n"
    ]
  },
  "1": {
    "d": [
      [
        "asdf_asdf",
        "asdf@asdf.com",
        "123-456-7890",
        "<a href=\"/users/1\">Show</a>",
        "<a href=\"/users/1/edit\">Edit</a>",
        "<a href=\"\\#\" phx-click=\"delete_user\" phx-value=\"1\">Delete</a>"
      ]
    ],
    "s": [
      "    <tr>\n      <td>",
      "</td>\n      <td>",
      "</td>\n      <td>",
      "</td>\n\n      <td>\n",
      "        ",
      "\n",
      "      </td>\n    </tr>\n"
    ]
  }
}"#;
    let expected_root: RootDiff =
        serde_json::from_str(deep_diff_result).expect("Failed to deserialize fragment");
    let expected_root: Root = expected_root
        .try_into()
        .expect("Failed to convert RootDiff to Root");
    assert_eq!(root, expected_root);
}

#[test]
fn simple() {
    let data = r#"
        {
            "1": "baz"
        }
        "#;
    let out: Result<FragmentDiff, _> = serde_json::from_str(data);
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
        assert!(out.is_ok());
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
        templates: Some(HashMap::from([(
            "0".into(),
            vec!["\\n    bar ".into(), "\\n  ".into()],
        )])),
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
                    }),
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
    let _root: RootDiff = serde_json::from_str(input).expect("Failed to deserialize fragment");
}
