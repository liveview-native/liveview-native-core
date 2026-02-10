use pretty_assertions::assert_eq;
use serde_json::json;

use crate::dom::Document;

/// serializes two documents so the formatting matches before diffing.
macro_rules! assert_doc_eq {
    ($gold:expr, $test:expr) => {
        let gold = Document::parse($gold).expect("Gold document failed to parse");
        let test = Document::parse($test).expect("Test document failed to parse");
        assert_eq!(gold.to_string(), test.to_string());
    };
}

use super::*;
mod stream;
#[test]
fn stream_parsing() {
    let initial = r#"
        {
          "1": {
            "0": {
              "d": [
                [
                  " id=\"songs_other-486\"",
                  "song 486",
                  " phx-value-id=\"486\"",
                  " phx-value-id=\"486\""
                ]
              ],
              "s": [
                "<tr",
                ">\n      <td>",
                "</td>\n      <td><button phx-click=\"delete-song\"",
                ">delete</button></td>\n      <td><button phx-click=\"increment-song\"",
                ">increment</button></td>\n    </tr>"
              ],
              "stream": [
                "1",
                [
                  [
                    "songs_other-486",
                    -1,
                    null
                  ]
                ],
                []
              ]
            }
          }
        }
        "#;
    let root: RootDiff = serde_json::from_str(initial).expect("Failed to deserialize fragment");
    let _root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
}

#[macro_export]
macro_rules! json_struct {
    ($($token:tt)*) => {{
        use serde_json::json;
        serde_json::from_value(json!($($token)*))
            .expect("Error deserializing JSON")
    }};
}

#[test]
fn considers_links() {
    let diff1: Root = json_struct!({});

    // merge two components, one which references the other
    let diff2: RootDiff = json_struct!({
        "c": {
            "1": {
                "s": ["comp"]
            },
            "2": {
                "s": 1
            }
        }
    });

    let result = diff1.merge(diff2.clone()).expect("Merge error");

    // The reference should be resolved
    let expected: Root = json_struct!({
        "c": {
            "1": {
                "s": ["comp"]
            },
            "2": {
                "s": ["comp"]
            }
        }
    });

    assert_eq!(expected, result);
}

#[test]
fn considers_links_old_and_new() {
    // merge two components, one which references the other
    let diff1: Root = json_struct!({
        "c": {
            "1": {
                "s": ["old"]
            }
        }
    });

    let diff2: RootDiff = json_struct!({
        "c": {
            "1": {
                "s": ["new"]
            },
            "2": {
                "newRender": true,
                "s": -1
            },
            "3": {
                "newRender": true,
                "s": 1
            }
        }
    });

    let result = diff1.merge(diff2.clone()).expect("Merge error");

    let expected: Root = json_struct!({
        "c": {
            "1": {
                "s": ["new"]
            },
            "2": {
                "s": ["old"]
            },
            "3": {
                "s": ["new"]
            }
        }
    });

    assert_eq!(expected, result);
}

#[test]
fn considers_links_whole_tree() {
    let diff1: Root = json_struct!({
        "c": {
            "1": {
                "0": {"s": ["nested"]},
                "s": ["old"]
            }
        }
    });

    let diff2: RootDiff = json_struct!({
        "c": {
            "1": {
                "0": {"s": ["nested"]},
                "s": ["new"]
            },
            "2": {
                "0": {"s": ["replaced"]},
                "s": -1
            },
            "3": {
                "0": {"s": ["replaced"]},
                "s": 1
            },
            "4": {"s": -1},
            "5": {"s": 1}
        }
    });

    let result = diff1.clone().merge(diff2.clone()).expect("Merge error");

    let expected1: Root = json_struct!({
        "c": {
            "1": {
                "0": {"s": ["nested"]},
                "s": ["new"]
            },
            "2": {
                "0": {"s": ["replaced"]},
                "s": ["old"]
            },
            "3": {
                "0": {"s": ["replaced"]},
                "s": ["new"]
            },
            "4": {
                "0": {"s": ["nested"]},
                "s": ["old"]
            },
            "5": {
                "0": {"s": ["nested"]},
                "s": ["new"]
            }
        }
    });

    // These are useful when narrowing down the failure case
    assert_eq!(expected1.components.get("1"), result.components.get("1"));
    assert_eq!(expected1.components.get("2"), result.components.get("2"));
    assert_eq!(expected1.components.get("3"), result.components.get("3"));
    assert_eq!(expected1.components.get("4"), result.components.get("4"));
    assert_eq!(expected1.components.get("5"), result.components.get("5"));
    assert_eq!(expected1, result);

    let diff3: RootDiff = json_struct!({
        "c": {
            "1": {
                "0": {"s": ["newRender"]},
                "s": ["new"]
            },
            "2": {
                "0": {"s": ["replaced"]},
                "s": -1
            },
            "3": {
                "0": {"s": ["replaced"]},
                "s": 1
            },
            "4": {"s": -1},
            "5": {"s": 1}
        }
    });

    let result2 = diff1.merge(diff3.clone()).expect("Merge error");

    let expected2: Root = json_struct!({
        "c": {
            "1": {
                "0": {"s": ["newRender"]},
                "s": ["new"]
            },
            "2": {
                "0": {"s": ["replaced"]},
                "s": ["old"]
            },
            "3": {
                "0": {"s": ["replaced"]},
                "s": ["new"]
            },
            "4": {
                "0": {"s": ["nested"]},
                "s": ["old"]
            },
            "5": {
                "0": {"s": ["newRender"]},
                "s": ["new"]
            }
        }
    });
    assert_eq!(expected2, result2);
}

// these are based on the js rendered_tests from liveview
#[test]
fn simple_diff_js_mirror() {
    let simple_diff1: Root = json_struct!({
        "0": "cooling",
        "1": "cooling",
        "2": "07:15:03 PM",
        "s": [
            "<div class=\"thermostat\">\n  <div class=\"bar ",
            "\">\n    <a href=\"#\" phx-click=\"toggle-mode\">",
            "</a>\n    <span>",
            "</span>\n  </div>\n</div>\n"
        ],
    });

    let simple_diff2: RootDiff = json_struct!({
        "2": "07:15:04 PM"
    });

    let simple_result = simple_diff1.merge(simple_diff2).expect("Merge error");

    let simple_expected: Root = json_struct!({
        "0": "cooling",
        "1": "cooling",
        "2": "07:15:04 PM",
        "s": [
            "<div class=\"thermostat\">\n  <div class=\"bar ",
            "\">\n    <a href=\"#\" phx-click=\"toggle-mode\">",
            "</a>\n    <span>",
            "</span>\n  </div>\n</div>\n"
        ],
    });

    assert_eq!(simple_expected, simple_result);
}

// these are based on the js tests from live view
#[test]
fn deep_diff_js_mirror() {
    let deep_diff1: Root = json_struct!({
        "0": {
            "0": {
                "d": [["user1058", "1"], ["user99", "1"]],
                "s": ["<tr>\n<td>", " (", ")</td>\n</tr>\n"],
                "r": 1
            },
            "s": [
                "  <table>\n    <thead>\n      <tr>\n        <th>Username</th>\n        <th></th>\n      </tr>\n    </thead>\n    <tbody>\n",
                "    </tbody>\n  </table>\n"
            ],
            "r": 1
        },
        "1": {
            "d": [[
                "asdf_asdf",
            ]],
            "s": [
                "<tr>\n<td>",
                "</td>\n<td>",
            ],
            "r": 1
        }
    });

    let deep_diff2: RootDiff = json_struct!({
        "0": {
            "0": {
                "d": [["user1058", "2"]]
            }
        }
    });

    let deep_result = deep_diff1.merge(deep_diff2).expect("Merge error");

    let deep_expected: Root = json_struct!({
        "0": {
            "0": {
                "newRender": true,
                "d": [["user1058", "2"]],
                "s": ["<tr>\n<td>", " (", ")</td>\n</tr>\n"],
                "r": 1
            },
            "s": [
                "  <table>\n    <thead>\n      <tr>\n        <th>Username</th>\n        <th></th>\n      </tr>\n    </thead>\n    <tbody>\n",
                "    </tbody>\n  </table>\n"
            ],
            "newRender": true,
            "r": 1
        },
        "1": {
            "d": [[
                "asdf_asdf",
            ]],
            "s": [
                "<tr>\n<td>",
                "</td>\n<td>",
            ],
            "r": 1
        }
    });

    assert_eq!(deep_expected, deep_result);
}

#[test]
fn jetpack_show_dialog() {
    /*
       * Diffs coming from this template:
    @impl true
    @spec render(any) :: Phoenix.LiveView.Rendered.t()
    def render(%{platform_id: :jetpack} = assigns) do
      ~JETPACK"""
      <Scaffold>
        <TopAppBar>
          <Title><Text>Hello</Text></Title>
        </TopAppBar>
        <FloatingActionButton phx-click="inc">
          <Icon imageVector="filled:Add" />
        </FloatingActionButton>
        <Column width="fill" verticalArrangement="center" horizontalAlignment="center" scroll="vertical">
          <OutlinedButton phx-click="showDialog"><Text>Show Dialog</Text></OutlinedButton>
          <%= if @showDialog do %>
          <AlertDialog phx-click="hideDialog">
            <ConfirmButton>
                <TextButton  phx-click="hideDialog">
                  <Text>Confirm</Text>
                </TextButton>
            </ConfirmButton>
            <DismissButton>
              <OutlinedButton phx-click="hideDialog">
                <Text>Dismiss</Text>
              </OutlinedButton>
            </DismissButton>
            <Icon imageVector="filled:Add" />
            <Title>Alert Title</Title>
            <Content>
                <Text>Alert message</Text>
            </Content>
          </AlertDialog>
          <% end %>
          <Box size="100" contentAlignment="center">
            <BadgeBox containerColor="#FF0000FF" contentColor="#FFFF0000">
              <Badge><Text>+99</Text></Badge>
              <Icon imageVector="filled:Add" />
            </BadgeBox>
          </Box>
          <ElevatedButton phx-click="showDialog"><Text>ElevatedButton</Text></ElevatedButton>
          <FilledTonalButton phx-click="showDialog"><Text>FilledTonalButton</Text></FilledTonalButton>
          <TextButton phx-click="showDialog"><Text>TextButton</Text></TextButton>

        </Column>
      </Scaffold>
      """
    end

      */
    let initial = r#"{
    "0":"",
    "s":[
      "<Scaffold>\n  <TopAppBar>\n    <Title><Text>Hello</Text></Title>\n  </TopAppBar>\n  <FloatingActionButton phx-click=\"inc\">\n    <Icon imageVector=\"filled:Add\"></Icon>\n  </FloatingActionButton>\n  <Column width=\"fill\" verticalArrangement=\"center\" horizontalAlignment=\"center\" scroll=\"vertical\">\n    <OutlinedButton phx-click=\"showDialog\"><Text>Show Dialog</Text></OutlinedButton>\n",
      "\n    <Box size=\"100\" contentAlignment=\"center\">\n      <BadgeBox containerColor=\"\\#FF0000FF\" contentColor=\"\\#FFFF0000\">\n        <Badge><Text>+99</Text></Badge>\n        <Icon imageVector=\"filled:Add\"></Icon>\n      </BadgeBox>\n    </Box>\n    <ElevatedButton phx-click=\"showDialog\"><Text>ElevatedButton</Text></ElevatedButton>\n    <FilledTonalButton phx-click=\"showDialog\"><Text>FilledTonalButton</Text></FilledTonalButton>\n    <TextButton phx-click=\"showDialog\"><Text>TextButton</Text></TextButton>\n\n  </Column>\n</Scaffold>"
      ]
      }
    "#;
    let root: RootDiff = serde_json::from_str(initial).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let out: String = root
        .clone()
        .try_into()
        .expect("Failed to render root as string");
    let expected = r#"<Scaffold>
  <TopAppBar>
    <Title><Text>Hello</Text></Title>
  </TopAppBar>
  <FloatingActionButton phx-click="inc">
    <Icon imageVector="filled:Add"></Icon>
  </FloatingActionButton>
  <Column width="fill" verticalArrangement="center" horizontalAlignment="center" scroll="vertical">
    <OutlinedButton phx-click="showDialog"><Text>Show Dialog</Text></OutlinedButton>

    <Box size="100" contentAlignment="center">
      <BadgeBox containerColor="\#FF0000FF" contentColor="\#FFFF0000">
        <Badge><Text>+99</Text></Badge>
        <Icon imageVector="filled:Add"></Icon>
      </BadgeBox>
    </Box>
    <ElevatedButton phx-click="showDialog"><Text>ElevatedButton</Text></ElevatedButton>
    <FilledTonalButton phx-click="showDialog"><Text>FilledTonalButton</Text></FilledTonalButton>
    <TextButton phx-click="showDialog"><Text>TextButton</Text></TextButton>

  </Column>
</Scaffold>"#;
    assert_eq!(expected, out);

    let mut document = crate::dom::Document::parse_fragment_json(initial.to_owned())
        .expect("Document failed to parse fragment json");
    // This is the same as above with minor styling changes.
    let document_expected = r#"<Scaffold>
    <TopAppBar>
        <Title>
            <Text>
                Hello
            </Text>
        </Title>
    </TopAppBar>
    <FloatingActionButton phx-click="inc">
        <Icon imageVector="filled:Add" />
    </FloatingActionButton>
    <Column width="fill" verticalArrangement="center" horizontalAlignment="center" scroll="vertical">
        <OutlinedButton phx-click="showDialog">
            <Text>
                Show Dialog
            </Text>
        </OutlinedButton>
        <Box size="100" contentAlignment="center">
            <BadgeBox containerColor="\#FF0000FF" contentColor="\#FFFF0000">
                <Badge>
                    <Text>
                        +99
                    </Text>
                </Badge>
                <Icon imageVector="filled:Add" />
            </BadgeBox>
        </Box>
        <ElevatedButton phx-click="showDialog">
            <Text>
                ElevatedButton
            </Text>
        </ElevatedButton>
        <FilledTonalButton phx-click="showDialog">
            <Text>
                FilledTonalButton
            </Text>
        </FilledTonalButton>
        <TextButton phx-click="showDialog">
            <Text>
                TextButton
            </Text>
        </TextButton>
    </Column>
</Scaffold>"#;
    assert_eq!(document_expected, document.to_string());

    let increment = r#"{
    "0":{
        "s":["\n    <AlertDialog phx-click=\"hideDialog\">\n      <ConfirmButton>\n          <TextButton phx-click=\"hideDialog\">\n            <Text>Confirm</Text>\n          </TextButton>\n      </ConfirmButton>\n      <DismissButton>\n        <OutlinedButton phx-click=\"hideDialog\">\n          <Text>Dismiss</Text>\n        </OutlinedButton>\n      </DismissButton>\n      <Icon imageVector=\"filled:Add\"></Icon>\n      <Title>Alert Title</Title>\n      <Content>\n          <Text>Alert message</Text>\n      </Content>\n    </AlertDialog>\n"
        ]
    }
}
"#;
    let diff: RootDiff = serde_json::from_str(increment).expect("Failed to deserialize fragment");

    let root = root.merge(diff).expect("Failed to merge diff");
    let out: String = root
        .clone()
        .try_into()
        .expect("Failed to render root as string");
    let expected = r#"<Scaffold>
  <TopAppBar>
    <Title><Text>Hello</Text></Title>
  </TopAppBar>
  <FloatingActionButton phx-click="inc">
    <Icon imageVector="filled:Add"></Icon>
  </FloatingActionButton>
  <Column width="fill" verticalArrangement="center" horizontalAlignment="center" scroll="vertical">
    <OutlinedButton phx-click="showDialog"><Text>Show Dialog</Text></OutlinedButton>

    <AlertDialog phx-click="hideDialog">
      <ConfirmButton>
          <TextButton phx-click="hideDialog">
            <Text>Confirm</Text>
          </TextButton>
      </ConfirmButton>
      <DismissButton>
        <OutlinedButton phx-click="hideDialog">
          <Text>Dismiss</Text>
        </OutlinedButton>
      </DismissButton>
      <Icon imageVector="filled:Add"></Icon>
      <Title>Alert Title</Title>
      <Content>
          <Text>Alert message</Text>
      </Content>
    </AlertDialog>

    <Box size="100" contentAlignment="center">
      <BadgeBox containerColor="\#FF0000FF" contentColor="\#FFFF0000">
        <Badge><Text>+99</Text></Badge>
        <Icon imageVector="filled:Add"></Icon>
      </BadgeBox>
    </Box>
    <ElevatedButton phx-click="showDialog"><Text>ElevatedButton</Text></ElevatedButton>
    <FilledTonalButton phx-click="showDialog"><Text>FilledTonalButton</Text></FilledTonalButton>
    <TextButton phx-click="showDialog"><Text>TextButton</Text></TextButton>

  </Column>
</Scaffold>"#;
    assert_eq!(out, expected);
    let new_document = crate::dom::Document::parse(out).expect("Failed to parse rendered dom");
    let patches = crate::diff::diff(&document, &new_document);
    if patches.is_empty() {
        return;
    }

    let mut editor = document.edit();
    let mut stack = vec![];
    for patch in patches.into_iter() {
        let _ = patch.apply(&mut editor, &mut stack);
    }
    editor.finish();
    //document.merge_fragment(diff.clone()).expect("Failed to merge in diff with document");
    let document_expected = r#"
<Scaffold>
    <TopAppBar>
        <Title>
            <Text>
                Hello
            </Text>
        </Title>
    </TopAppBar>
    <FloatingActionButton phx-click="inc">
        <Icon imageVector="filled:Add" />
    </FloatingActionButton>
    <Column width="fill" verticalArrangement="center" horizontalAlignment="center" scroll="vertical">
        <OutlinedButton phx-click="showDialog">
            <Text>
                Show Dialog
            </Text>
        </OutlinedButton>
        <AlertDialog phx-click="hideDialog">
            <ConfirmButton>
                <TextButton phx-click="hideDialog">
                    <Text>
                        Confirm
                    </Text>
                </TextButton>
            </ConfirmButton>
            <DismissButton>
                <OutlinedButton phx-click="hideDialog">
                    <Text>
                        Dismiss
                    </Text>
                </OutlinedButton>
            </DismissButton>
            <Icon imageVector="filled:Add" />
            <Title>
                Alert Title
            </Title>
            <Content>
                <Text>
                    Alert message
                </Text>
            </Content>
        </AlertDialog>
        <Box size="100" contentAlignment="center">
            <BadgeBox containerColor="\#FF0000FF" contentColor="\#FFFF0000">
                <Badge>
                    <Text>
                        +99
                    </Text>
                </Badge>
                <Icon imageVector="filled:Add" />
            </BadgeBox>
        </Box>
        <ElevatedButton phx-click="showDialog">
            <Text>
                ElevatedButton
            </Text>
        </ElevatedButton>
        <FilledTonalButton phx-click="showDialog">
            <Text>
                FilledTonalButton
            </Text>
        </FilledTonalButton>
        <TextButton phx-click="showDialog">
            <Text>
                TextButton
            </Text>
        </TextButton>
    </Column>
</Scaffold>"#;

    assert_doc_eq!(document.to_string(), document_expected);
}

#[test]
fn jetpack_more_edge_cases() {
    let initial = r#"{
  "0":"0",
  "1":"0",
  "2":"",
  "s":[
    "<Column>\n  <Button phx-click=\"inc\">\n    <Text>Button</Text>\n  </Button>\n  <Text>Static Text </Text>\n  <Text>Counter 1: ",
    " </Text>\n  <Text>Counter 2: ",
    " </Text>\n  ",
    "\n</Column>"
    ]
}"#;
    let root: RootDiff = serde_json::from_str(initial).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let increment = r#"{
  "0":"1",
  "1":"1",
  "2":{
      "0":{
          "s":[
              "\n      <Text fontWeight=\"W600\" fontSize=\"24\">Item ",
              "!!!</Text>\n      ","\n      ",
              "\n    "
          ],
          "p":{
              "0":[
                  "\n        <Text color=\" #FFFF0000\">Number = ",
                  " + 3 is even</Text>\n      "
              ],
              "1":[
                  "\n        <Text>Number + 4 = ",
                  " is odd</Text>\n      "
              ]
          },
          "d":[[
            "1",
            {"0":"1","s":0},
            {"0":"5","s":1}]]
        },
        "1":"101",
        "s":[
          "\n    ",
          "\n    <Text>Number + 100 is ",
          "</Text>\n  "
        ]
  }
}"#;
    let diff: RootDiff = serde_json::from_str(increment).expect("Failed to deserialize fragment");
    let root = root.merge(diff).expect("Failed to merge diff into root");
    let _out: String = root
        .clone()
        .try_into()
        .expect("Failed to convert root to string");
    let increment = r#"{
  "0":"2",
  "1":"2",
  "2":{
    "0":{
      "p":{
        "0":[
          "\n        <Text color=\" #FFFF0000\">Number = ",
          " + 3 is even</Text>\n      "
        ],
        "1":[
          "\n        <Text>Number + 4 = ",
          " is odd</Text>\n      "
        ],
        "2":[
          "\n        <Text color=\" #FF0000FF\">Number = ",
          " + 3 is odd</Text>\n      "
        ],
        "3":[
          "\n        <Text>Number + 4 = ",
          " is even</Text>\n      "
        ]
      },
      "d":[
        [
          "1",
          {"0":"1","s":0},
          {"0":"5","s":1}
        ],[
          "2",
          {"0":"2","s":2},
          {"0":"6","s":3}
        ]
      ]
    },
    "1":"102"
  }
}"#;
    let diff: RootDiff = serde_json::from_str(increment).expect("Failed to deserialize fragment");
    let root = root.merge(diff).expect("Failed to merge diff into root");
    let _out: String = root
        .clone()
        .try_into()
        .expect("Failed to convert root to string");
}
#[test]
fn expands_shared_static_from_cids() {
    let root: Root = json_struct!({});
    let mount_diff: RootDiff = json_struct!({
        "0": "",
        "1": "",
        "2": {
            "0": "new post",
            "1": "",
            "2": {
                "d": [[1], [2]],
                "s": ["", ""]
            },
            "s": ["h1", "h2", "h3", "h4"]
        },
        "c": {
            "1": {
                "0": "1008",
                "1": "chris_mccord",
                "2": "My post",
                "3": "1",
                "4": "0",
                "5": "1",
                "6": "0",
                "7": "edit",
                "8": "delete",
                "s": ["s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9"]
            },
            "2": {
                "0": "1007",
                "1": "chris_mccord",
                "2": "My post",
                "3": "2",
                "4": "0",
                "5": "2",
                "6": "0",
                "7": "edit",
                "8": "delete",
                "s": 1
            }
        },
        "s": ["f1", "f2", "f3", "f4"],
        "title": "Listing Posts"
    });

    let root = root.merge(mount_diff).expect("merge failed");

    let component1_static = root.components.get("1").expect("C1 Missing");
    let component2_static = root.components.get("2").expect("C2 Missing");
    assert!(matches!(
        component1_static.statics,
        ComponentStatics::Statics(_)
    ));
    assert_eq!(component1_static.statics, component2_static.statics);

    let update_diff: RootDiff = json_struct!({
        "2": {
            "2": {
                "d": [[3]]
            }
        },
        "c": {
            "3": {
                "0": "1009",
                "1": "chris_mccord",
                "2": "newnewnewnewnewnewnewnew",
                "3": "3",
                "4": "0",
                "5": "3",
                "6": "0",
                "7": "edit",
                "8": "delete",
                "s": -2
            }
        }
    });

    let root = root.merge(update_diff).expect("Merge error");
    let _ = root.components.get("2").expect("C2 Post Merge Missing");
    let _ = root.components.get("3").expect("C3 Post Merge Missing");

    let Component { statics, .. } = root.components.get("1").expect("C1 Post Merge Missing");
    assert!(matches!(statics, ComponentStatics::Statics(_)));

    assert_eq!(
        Some(statics.clone()),
        root.components.get("2").map(|c| c.statics.clone())
    );

    assert_eq!(
        Some(statics.clone()),
        root.components.get("3").map(|c| c.statics.clone())
    );
}

#[test]
fn reuses_statics() {
    let static_reuse_diff: RootDiff = json_struct!({
        "0": {
            "d": [
                ["foo", {"d": [["0", 1], ["1", 2]], "s": 0}],
                ["bar", {"d": [["0", 3], ["1", 4]], "s": 0}]
            ],
            "s": ["\n  <p>\n    ", "\n    ", "\n  </p>\n"],
            "r": 1,
            "p": {"0": ["<span>", ": ", "</span>"]}
        },
        "c": {
            "1": {"0": "index_1", "1": "world", "s": ["<b>FROM ", " ", "</b>"], "r": 1},
            "2": {"0": "index_2", "1": "world", "s": 1, "r": 1},
            "3": {"0": "index_1", "1": "world", "s": 1, "r": 1},
            "4": {"0": "index_2", "1": "world", "s": 3, "r": 1}
        },
        "s": ["<div>", "</div>"],
        "r": 1
    });
    let root: Root = static_reuse_diff.try_into().expect("conversion failed");
    let doc: String = root.try_into().expect("render failed");

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

    assert_doc_eq!(doc, expected);
}

#[test]
fn jetpack_complex() {
    /*

    The incremental diffs for this test came from this template:
    @impl true
    def render(%{platform_id: :jetpack} = assigns) do
      ~JETPACK"""
      <Column>
        <Button phx-click="inc">
          <Text>Increment</Text>
        </Button>
        <Button phx-click="dec">
          <Text>Decrement</Text>
        </Button>
        <Text>Static Text </Text>
        <Text>Counter 1: <%= @val %> </Text>
        <Text>Counter 2: <%= @val %> </Text>
        <%= if @val > 0 do %>
          <%= for x <- 1..@val do %>
            <Text fontWeight="W600" fontSize="24">Item <%= x %>!!!</Text>
            <%= if rem(x+3,2) == 0 do %>
              <Text color="#FFFF0000">Number = <%= x %> + 3 is even</Text>
            <% else %>
              <Text color="#FF0000FF">Number = <%= x %> + 3 is odd</Text>
            <% end %>
            <%= if rem(x+4,2) == 0 do %>
              <Text>Number + 4 = <%= x+4 %> is even</Text>
            <% else %>
              <Text>Number + 4 = <%= x+4 %> is odd</Text>
            <% end %>
          <% end %>
          <Text>Number + 100 is <%= @val+100 %></Text>
        <% end %>
      </Column>
      """
    end
       */
    let initial = r#"{
  "0":"0",
  "1":"0",
  "2":"",
  "s":[
    "<Column>\n  <Button phx-click=\"inc\">\n    <Text>Increment</Text>\n  </Button>\n  <Button phx-click=\"dec\">\n    <Text>Decrement</Text>\n  </Button>\n  <Text>Static Text </Text>\n  <Text>Counter 1: ",
    " </Text>\n  <Text>Counter 2: ",
    " </Text>\n",
    "\n</Column>"
  ]
}
"#;
    let root: RootDiff = serde_json::from_str(initial).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let _out: String = root
        .clone()
        .try_into()
        .expect("Failed to convert root to string");
    let increment = r#"{
  "0":"1",
  "1":"1",
  "2":{
    "0":{
      "s":[
        "\n      <Text fontWeight=\"W600\" fontSize=\"24\">Item ",
        "!!!</Text>\n",
        "\n",
        "\n"
      ],
      "p":{
         "0":[
           "\n        <Text color=\" #FFFF0000\">Number = ",
           " + 3 is even</Text>\n"
         ],
         "1":[
           "\n        <Text>Number + 4 = ",
           " is odd</Text>\n"
           ]
      },
      "d":[["1",{"0":"1","s":0},{"0":"5","s":1}]]
    },
    "1":"101",
    "s":[
      "\n",
      "\n    <Text>Number + 100 is ","</Text>\n"
    ]
  }
}
    "#;
    let new_diff: RootDiff =
        serde_json::from_str(increment).expect("Failed to deserialize diff fragment");
    let root = root.merge(new_diff).expect("Failed to merge new root in");
    let _out: String = root
        .clone()
        .try_into()
        .expect("Failed to convert root to string");
    let increment = r#"{
  "0":"2",
  "1":"2",
  "2":{
    "0":{
      "p":{
        "0":[
          "\n        <Text color=\" #FFFF0000\">Number = ",
          " + 3 is even</Text>\n"
        ],
        "1":[
          "\n        <Text>Number + 4 = ",
          " is odd</Text>\n"
        ],
        "2":[
          "\n        <Text color=\" #FF0000FF\">Number = ",
          " + 3 is odd</Text>\n"
        ],
        "3":[
          "\n        <Text>Number + 4 = ",
          " is even</Text>\n"
        ]
      },
      "d":[
        ["1",{"0":"1","s":0},{"0":"5","s":1}],
        ["2",{"0":"2","s":2},{"0":"6","s":3}]
      ]
    },
    "1":"102"
  }
}"#;
    let new_diff: RootDiff =
        serde_json::from_str(increment).expect("Failed to deserialize diff fragment");
    let root = root.merge(new_diff).expect("Failed to merge new root in");
    let out: String = root.try_into().expect("Failed to convert root to string");
    let expected = r#"
  <Column>
  <Button phx-click="inc">
    <Text>Increment</Text>
  </Button>
  <Button phx-click="dec">
    <Text>Decrement</Text>
  </Button>
  <Text>Static Text </Text>
  <Text>Counter 1: 2 </Text>
  <Text>Counter 2: 2 </Text>
        <Text fontWeight="W600" fontSize="24">Item 1!!!</Text>
        <Text color=" #FFFF0000">Number = 1 + 3 is even</Text>
        <Text>Number + 4 = 5 is odd</Text>
        <Text fontWeight="W600" fontSize="24">Item 2!!!</Text>
        <Text color=" #FF0000FF">Number = 2 + 3 is odd</Text>
        <Text>Number + 4 = 6 is even</Text>
    <Text>Number + 100 is 102</Text>
</Column>"#;
    assert_doc_eq!(out, expected);
}
#[test]
fn jetpack_simple_counter() {
    let initial_json = r#"{
        "0":"0",
        "s":["<Scaffold>\n  <TopAppBar>\n    <Title><Text>Hello</Text></Title>\n  </TopAppBar>\n  <Column width=\"fill\" verticalArrangement=\"center\" horizontalAlignment=\"center\">\n    <Text style=\"headlineLarge\">Title</Text>\n    <Card shape=\"8\" padding=\"16\" width=\"140\" height=\"120\" elevation=\"{'defaultElevation': '10', 'pressedElevation': '2'}\" phx-click=\"dec\">\n      <Text padding=\"16\">Hello Jetpack!</Text>\n    </Card>\n    <Spacer height=\"8\"></Spacer>\n    <Card padding=\"16\">\n      <Text padding=\"16\">Simple card</Text>\n    </Card>\n    <Button phx-click=\"navigate\" contentPadding=\"50\" elevation=\"{'defaultElevation': '20', 'pressedElevation': '10'}\">\n      <Text>Navigate to counter</Text>\n    </Button>\n    <Button phx-click=\"redirect\"><Text>Redirect to counter</Text></Button>\n    <IconButton phx-click=\"inc\" colors=\"{'containerColor': '#FFFF0000', 'contentColor': '#FFFFFFFF'}\">\n      <Icon imageVector=\"filled:Add\"></Icon>\n    </IconButton>\n    <Row verticalAlignment=\"center\">\n      <Button phx-click=\"dec\" shape=\"circle\" size=\"60\">\n        <Text>-</Text>\n      </Button>\n      <Text>This counter: ","</Text>\n      <Button phx-click=\"inc\" shape=\"circle\" size=\"60\"><Text>+</Text></Button>\n    </Row>\n  </Column>\n</Scaffold>"]}"#;
    let root: RootDiff =
        serde_json::from_str(initial_json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let increment_diff = r#"{"0": "1"}"#;
    let other_root: RootDiff =
        serde_json::from_str(increment_diff).expect("Failed to deserialize diff fragment");
    let new_root = root.merge(other_root).expect("Failed to merge new root in");
    let expected_root = r#"{
        "0":"1",
        "s":["<Scaffold>\n  <TopAppBar>\n    <Title><Text>Hello</Text></Title>\n  </TopAppBar>\n  <Column width=\"fill\" verticalArrangement=\"center\" horizontalAlignment=\"center\">\n    <Text style=\"headlineLarge\">Title</Text>\n    <Card shape=\"8\" padding=\"16\" width=\"140\" height=\"120\" elevation=\"{'defaultElevation': '10', 'pressedElevation': '2'}\" phx-click=\"dec\">\n      <Text padding=\"16\">Hello Jetpack!</Text>\n    </Card>\n    <Spacer height=\"8\"></Spacer>\n    <Card padding=\"16\">\n      <Text padding=\"16\">Simple card</Text>\n    </Card>\n    <Button phx-click=\"navigate\" contentPadding=\"50\" elevation=\"{'defaultElevation': '20', 'pressedElevation': '10'}\">\n      <Text>Navigate to counter</Text>\n    </Button>\n    <Button phx-click=\"redirect\"><Text>Redirect to counter</Text></Button>\n    <IconButton phx-click=\"inc\" colors=\"{'containerColor': '#FFFF0000', 'contentColor': '#FFFFFFFF'}\">\n      <Icon imageVector=\"filled:Add\"></Icon>\n    </IconButton>\n    <Row verticalAlignment=\"center\">\n      <Button phx-click=\"dec\" shape=\"circle\" size=\"60\">\n        <Text>-</Text>\n      </Button>\n      <Text>This counter: ","</Text>\n      <Button phx-click=\"inc\" shape=\"circle\" size=\"60\"><Text>+</Text></Button>\n    </Row>\n  </Column>\n</Scaffold>"]}"#;
    let expected_root: RootDiff =
        serde_json::from_str(expected_root).expect("Failed to deserialize fragment");
    let expected_root: Root = expected_root
        .try_into()
        .expect("Failed to convert RootDiff to Root");
    assert_eq!(expected_root, new_root);

    let _out: String = new_root
        .try_into()
        .expect("Failed to convert root to string");
}

// asserts that diffs with a new set of statics replace the previous fragment
#[test]
fn test_replace() {
    let current = Fragment::Regular {
        children: HashMap::from([("1".into(), Child::String("a".to_owned().into()))]),
        statics: Statics::Statics(vec!["b".into(), "c".into()]).into(),
        is_root: None,
        templates: None,
        new_render: None,

    };

    let diff = FragmentDiff::UpdateRegular {
        children: HashMap::from([("1".into(), ChildDiff::String("foo".to_owned().into()))]),
        templates: None,
        statics: Statics::Statics(vec!["bar".into(), "baz".into()]).into(),
        is_root: None,
        event: None,
    };

    let new = Fragment::Regular {
        statics: Statics::Statics(vec!["bar".into(), "baz".into()]).into(),
        is_root: None,
        children: HashMap::from([("1".into(), Child::String("foo".to_owned().into()))]),
        templates: None,
        new_render: None,

    };

    assert_eq!(
        Fragment::try_from(diff.clone()).expect("diff not equal to frag"),
        new
    );

    let merge = current.merge(diff).expect("Failed to merge diff");
    assert_eq!(merge, new);
}

#[test]
fn test_mutate() {
    let current = Fragment::Regular {
        children: HashMap::from([("1".into(), Child::String("a".to_owned().into()))]),
        statics: Statics::Statics(vec!["b".into(), "c".into()]).into(),
        is_root: None,
        templates: None,
        new_render: None,

    };

    let diff = FragmentDiff::UpdateRegular {
        children: HashMap::from([("1".into(), ChildDiff::String("foo".to_owned().into()))]),
        templates: None,
        statics: None,
        is_root: None,
        event: None,
    };

    let new = Fragment::Regular {
        children: HashMap::from([("1".into(), Child::String("foo".to_owned().into()))]),
        statics: Statics::Statics(vec!["b".into(), "c".into()]).into(),
        is_root: None,
        templates: None,
        new_render: None,

    };

    let merge = current.merge(diff).expect("Failed to merge diff");
    assert_eq!(merge, new);
}

#[test]
fn fragment_render_parse() {
    let root = Root {
        fragment: Fragment::Regular {
            children: HashMap::from([
                ("0".into(), Child::String("foo".to_owned().into())),
                ("1".into(), Child::ComponentID(1)),
            ]),
            statics: Statics::Statics(vec!["1".into(), "2".into(), "3".into()]).into(),
            is_root: None,
            templates: None,
            new_render: None,
    
        },
        components: HashMap::from([(
            "1".into(),
            Component {
                children: HashMap::from([("0".into(), Child::String("bar".to_owned().into()))]),
                statics: ComponentStatics::Statics(vec!["4".into(), "5".into()]),
                is_root: None,
            },
        )]),
        new_render: None,
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
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
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
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let simple_diff2 = r#"{"2": "07:15:04 PM"}"#;
    let root_diff: RootDiff =
        serde_json::from_str(simple_diff2).expect("Failed to deserialize fragment");
    let root = root
        .merge(root_diff)
        .expect("Failed to merge diff into root");
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
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
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
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let out: String = root.try_into().expect("Failed to convert Root into string");
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
    assert_doc_eq!(out, expected);
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
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let out: String = root.try_into().expect("Failed to convert Root into string");
    let expected = r#"<div>
  <Group>
    <Text>Item 3</Text>
    <Text>Item 4</Text>
    <Text>Item 5</Text>
  </Group>
</div>"#;
    assert_doc_eq!(out, expected);
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
        children: HashMap::from([(1.to_string(), ChildDiff::String("baz".to_owned().into()))]),
        templates: None,
        statics: None,
        is_root: None,
        event: None,
    };
    assert_eq!(out, expected);
}

#[test]
fn simple_component_diff() {
    let diffs = vec![
        r#"{"0": "index_2", "1": "world", "s": 1}"#,
        r#"{"0": "index_1", "1": "world", "s": 1}"#,
        r#"{"0": "index_2", "1": "world", "s": 3}"#,
        r#"{"0": "index_2", "1": {"s": "new"}, "s": ["str"]}"#,
        r#"{"0": "index_1", "1": "world", "s": ["<b>FROM ", " ", "</b>"]}"#,
    ];
    for data in &diffs {
        let out: Result<ComponentDiff, _> = serde_json::from_str(data);
        assert!(out.is_ok());
    }
}

// reproduces a test in the swift xcframework specific tests
#[test]
fn swift_bug_repro() {
    let initial_json = json!({
        "s" : [
            "",
            ""
        ],
        "0" : {
            "0" : "",
            "s" : [
                "<VStack>\n  ",
                "\n  <Button phx-click=\"inc_temperature\"> Increment Temperature </Button>\n  <Button phx-click=\"dec_temperature\"> Decrement Temperature </Button>\n</VStack>"
            ],
            "r" : 1
        }
    });
    let root: Root = serde_json::from_value(initial_json).expect("Root");
    let expected = r#"<VStack>
    <Button phx-click="inc_temperature"> Increment Temperature </Button>
    <Button phx-click="dec_temperature"> Decrement Temperature </Button>
</VStack>
"#;

    let out: String = root.clone().try_into().expect("bad root");
    assert_doc_eq!(expected, out);

    let first_increment = json!(
    {
        "0" : {
            "0" : {
                "s" : [
                    "<Text> Temperature: ",
                    " </Text>"
                ],
                "d" : [
                    ["Increment"]
                ]
            }
        }
    });

    let diff = serde_json::from_value(first_increment).expect("invalid diff");

    let root = root.merge(diff).expect("merge failed");

    let expected = r#"<VStack>
    <Text>
        Temperature: Increment
    </Text>
    <Button phx-click="inc_temperature"> Increment Temperature </Button>
    <Button phx-click="dec_temperature"> Decrement Temperature </Button>
</VStack>"#;

    let out: String = root.clone().try_into().expect("bad root");
    assert_doc_eq!(expected, out);
    let second_increment = json!(
    {
        "0" : {
            "0" : {
                "d" : []
            }
        }
    });

    let diff = serde_json::from_value(second_increment).expect("invalid diff");
    let root = root.merge(diff).expect("merge failed");

    let third_increment = json!({ "0" : {
        "0" : { "d" : [ ["Increment"] ]  }
        }
    });

    let diff = serde_json::from_value(third_increment).expect("invalid diff");
    let _root = root.merge(diff).expect("merge failed");
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
    let expected = FragmentDiff::UpdateRegular {
        children: HashMap::from([
            ("0".into(), ChildDiff::String("foo".to_owned().into())),
            ("1".into(), ChildDiff::String("bar".to_owned().into())),
        ]),
        templates: None,
        statics: Some(Statics::Statics(vec!["a".into(), "b".into()])),
        is_root: None,
        event: None,
    };
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
    assert!(out.is_ok());
    let out = out.expect("Failed to deserialize");
    let expected = FragmentDiff::UpdateComprehension {
        dynamics: vec![
            vec![
                ChildDiff::String("foo".to_owned().into()),
                ChildDiff::ComponentID(1),
            ],
            vec![
                ChildDiff::String("bar".to_owned().into()),
                ChildDiff::ComponentID(1),
            ],
        ],
        statics: None,
        templates: Some(HashMap::from([(
            "0".into(),
            vec!["\\n    bar ".into(), "\\n  ".into()],
        )])),
        stream: None,
        is_root: None,
        event: None,
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
            vec![
                ChildDiff::String("foo".to_owned().into()),
                ChildDiff::ComponentID(1),
            ],
            vec![
                ChildDiff::String("bar".to_owned().into()),
                ChildDiff::ComponentID(1),
            ],
        ],
        statics: None,
        templates: None,
        stream: None,
        is_root: None,
        event: None,
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
    assert!(out.is_ok());
    let out = out.expect("Failed to deserialize");
    let expected = RootDiff {
        fragment: FragmentDiff::UpdateRegular {
            children: HashMap::from([(
                "0".into(),
                ChildDiff::Fragment(FragmentDiff::UpdateRegular {
                    children: HashMap::from([("0".into(), ChildDiff::ComponentID(1))]),
                    templates: None,
                    statics: None,
                    is_root: None,
                    event: None,
                }),
            )]),
            templates: None,
            statics: None,
            is_root: None,
            event: None,
        },
        new_render: None,
        components: HashMap::from([(
            "1".into(),
            ComponentDiff::UpdateRegular {
                is_root: None,
                children: HashMap::from([(
                    "0".into(),
                    ChildDiff::Fragment(FragmentDiff::UpdateComprehension {
                        dynamics: vec![
                            vec![
                                ChildDiff::String("0".to_owned().into()),
                                ChildDiff::String("foo".to_owned().into()),
                            ],
                            vec![
                                ChildDiff::String("1".to_owned().into()),
                                ChildDiff::String("bar".to_owned().into()),
                            ],
                        ],
                        statics: None,
                        templates: None,
                        stream: None,
                        is_root: None,
                        event: None,
                    }),
                )]),
            },
        )]),
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
    assert!(out.is_ok());
    let out = out.expect("Failed to deserialize");
    let expected = RootDiff {
        fragment: FragmentDiff::UpdateRegular {
            children: HashMap::from([(
                "0".into(),
                ChildDiff::Fragment(FragmentDiff::UpdateRegular {
                    children: HashMap::from([("0".into(), ChildDiff::ComponentID(1))]),
                    templates: None,
                    statics: None,
                    is_root: None,
                    event: None,
                }),
            )]),
            templates: None,
            statics: None,
            is_root: None,
            event: None,
        },
        components: HashMap::new(),
        new_render: None,
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

/// Tests for Regular fragments with templates (LiveView 1.0+ format)
/// This format uses "p" for shared templates even in Regular (non-Comprehension) fragments,
/// with "s" as an integer template reference.
#[test]
fn regular_fragment_with_templates_simple() {
    // This is the format LiveView 1.0+ sends for templates even without comprehensions
    // "p" contains template parts, "s" at root level references template index
    let json = r#"{"0":{"s":0,"r":1},"p":{"0":["<Text>Hello, Jetpack!</Text>"],"1":["",""]},"s":1}"#;

    let root: RootDiff = serde_json::from_str(json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let out: String = root.try_into().expect("Failed to convert Root into string");

    // Template "1" is ["",""], so root renders empty
    // Child "0" uses template "0" which is ["<Text>Hello, Jetpack!</Text>"]
    assert_eq!(out, "<Text>Hello, Jetpack!</Text>");
}

#[test]
fn regular_fragment_with_template_ref_and_children() {
    // Regular fragment with template reference and dynamic children
    let json = r#"{
        "0": "World",
        "p": {"0": ["<Text>Hello, ", "!</Text>"]},
        "s": 0
    }"#;

    let root: RootDiff = serde_json::from_str(json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let out: String = root.try_into().expect("Failed to convert Root into string");

    assert_eq!(out, "<Text>Hello, World!</Text>");
}

#[test]
fn regular_fragment_with_nested_template_refs() {
    // Nested regular fragments with template references
    let json = r#"{
        "0": {
            "0": "inner",
            "s": 0
        },
        "p": {
            "0": ["<Inner>", "</Inner>"],
            "1": ["<Outer>", "</Outer>"]
        },
        "s": 1
    }"#;

    let root: RootDiff = serde_json::from_str(json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let out: String = root.try_into().expect("Failed to convert Root into string");

    assert_eq!(out, "<Outer><Inner>inner</Inner></Outer>");
}

#[test]
fn regular_fragment_merge_preserves_templates() {
    // Initial render with templates
    let initial_json = r#"{
        "0": "first",
        "p": {"0": ["<Text>", "</Text>"]},
        "s": 0
    }"#;

    let root: RootDiff = serde_json::from_str(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");

    // Diff that only updates the dynamic value (no templates in diff)
    let diff_json = r#"{"0": "second"}"#;
    let diff: RootDiff = serde_json::from_str(diff_json).expect("Failed to deserialize diff");

    let merged = root.merge(diff).expect("Failed to merge");
    let out: String = merged.try_into().expect("Failed to convert");

    // Templates should be preserved from initial render
    assert_eq!(out, "<Text>second</Text>");
}

#[test]
fn keyed_comprehension_basic() {
    // Basic keyed comprehension with template reference
    let json = r#"{
        "0": {
            "k": {
                "0": {"0": "Item 1"},
                "1": {"0": "Item 2"},
                "kc": 2
            },
            "s": 0
        },
        "p": {"0": ["<Text>", "</Text>"]},
        "s": ["<Column>", "</Column>"]
    }"#;

    let root: RootDiff = serde_json::from_str(json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let html: String = root.try_into().expect("Failed to render");

    assert_doc_eq!(html, "<Column><Text>Item 1</Text><Text>Item 2</Text></Column>");
}

#[test]
fn keyed_comprehension_single_item() {
    // Single keyed item
    let json = r#"{
        "k": {"0": {"0": "Hello"}, "kc": 1},
        "p": {"0": ["<Text>", "</Text>"]},
        "s": 0
    }"#;

    let root: RootDiff = serde_json::from_str(json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let html: String = root.try_into().expect("Failed to render");

    assert_eq!(html, "<Text>Hello</Text>");
}

#[test]
fn keyed_comprehension_multiple_dynamics() {
    // Keyed item with multiple dynamic values
    let json = r#"{
        "k": {
            "0": {"0": "Alice", "1": "30"},
            "kc": 1
        },
        "p": {"0": ["<Row><Text>", "</Text><Text>Age: ", "</Text></Row>"]},
        "s": 0
    }"#;

    let root: RootDiff = serde_json::from_str(json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let html: String = root.try_into().expect("Failed to render");

    assert_doc_eq!(html, "<Row><Text>Alice</Text><Text>Age: 30</Text></Row>");
}

#[test]
fn keyed_comprehension_empty() {
    // Empty keyed comprehension
    let json = r#"{
        "k": {"kc": 0},
        "p": {"0": ["<Text>", "</Text>"]},
        "s": 0
    }"#;

    let root: RootDiff = serde_json::from_str(json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let html: String = root.try_into().expect("Failed to render");

    assert_eq!(html, "");
}

#[test]
fn keyed_comprehension_nested_in_regular() {
    // Keyed comprehension nested inside a regular fragment
    let json = r#"{
        "0": {
            "0": {
                "k": {
                    "0": {"0": "Route 1"},
                    "1": {"0": "Route 2"},
                    "kc": 2
                },
                "s": 0
            },
            "1": "",
            "s": 1
        },
        "p": {
            "0": ["<Text>", "</Text>"],
            "1": ["<Column><LazyColumn>", "</LazyColumn>", "</Column>"],
            "2": ["", ""]
        },
        "s": 2
    }"#;

    let root: RootDiff = serde_json::from_str(json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let html: String = root.try_into().expect("Failed to render");

    assert!(html.contains("Route 1"));
    assert!(html.contains("Route 2"));
}

#[test]
fn keyed_comprehension_routes_list() {
    // Real-world route list case
    let json = r#"{
        "0": {
            "0": {
                "k": {
                    "0": {
                        "0": "601",
                        "1": "Amsterdam Driver 1",
                        "2": "Preparing",
                        "3": "false",
                        "4": " phx-value-route_id=\"abc\"",
                        "5": "true"
                    },
                    "kc": 1
                },
                "s": 0
            },
            "1": "",
            "s": 1
        },
        "p": {
            "0": ["<Row><Text>", "</Text><Text>", "</Text><Text>", "</Text><Switch checked=\"", "\"", " enabled=\"", "\"/></Row>"],
            "1": ["<Column><LazyColumn>", "</LazyColumn>", "</Column>"],
            "2": ["", ""]
        },
        "s": 2
    }"#;

    let root: RootDiff = serde_json::from_str(json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let html: String = root.try_into().expect("Failed to render");

    assert!(html.contains("601"));
    assert!(html.contains("Amsterdam Driver 1"));
    assert!(html.contains("Preparing"));
}

#[test]
fn keyed_comprehension_with_nested_statics() {
    // Keyed items with their own nested statics, wrapped in an outer fragment
    let json = r#"{
        "0": {
            "k": {
                "0": {"0": "A", "s": ["<Item>", "</Item>"]},
                "1": {"0": "B", "s": ["<Item>", "</Item>"]},
                "kc": 2
            }
        },
        "s": ["<List>", "</List>"]
    }"#;

    let root: RootDiff = serde_json::from_str(json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let html: String = root.try_into().expect("Failed to render");

    assert_doc_eq!(html, "<List><Item>A</Item><Item>B</Item></List>");
}

#[test]
fn keyed_comprehension_parsing() {
    // Just test that we can parse a keyed comprehension correctly
    let json = r#"{
        "k": {
            "0": {"0": "value1"},
            "1": {"0": "value2"},
            "kc": 2
        },
        "s": 0,
        "p": {"0": ["<X>", "</X>"]}
    }"#;

    let diff: FragmentDiff = serde_json::from_str(json).expect("Failed to deserialize");

    match diff {
        FragmentDiff::UpdateKeyedComprehension { keyed, .. } => {
            assert_eq!(keyed.key_count, 2);
            assert_eq!(keyed.items.len(), 2);
        }
        _ => panic!("Expected UpdateKeyedComprehension variant"),
    }
}

#[test]
fn keyed_comprehension_as_child() {
    // Keyed comprehension appearing as a child of a regular fragment
    let json = r#"{
        "0": "Header",
        "1": {
            "k": {
                "0": {"0": "Item A"},
                "1": {"0": "Item B"},
                "kc": 2
            },
            "s": 0
        },
        "p": {"0": ["<Li>", "</Li>"]},
        "s": ["<Container><H1>", "</H1><List>", "</List></Container>"]
    }"#;

    let root: RootDiff = serde_json::from_str(json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let html: String = root.try_into().expect("Failed to render");

    assert!(html.contains("Header"));
    assert!(html.contains("Item A"));
    assert!(html.contains("Item B"));
}

#[test]
fn keyed_comprehension_partial_update() {
    // Test that partial diffs to keyed items preserve unchanged fields
    // This simulates the toggle scenario where only one field changes
    let initial_json = r#"{
        "0": {
            "k": {
                "0": {"0": "601", "1": "Driver 1", "2": "Ready", "3": "false"},
                "kc": 1
            },
            "s": 0
        },
        "p": {"0": ["<Row><Text>", "</Text><Text>", "</Text><Text>", "</Text><Switch checked=\"", "\"/></Row>"]},
        "s": ["<Column>", "</Column>"]
    }"#;

    let root: RootDiff = serde_json::from_str(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");
    let html: String = root.clone().try_into().expect("Failed to render");

    assert!(html.contains("601"));
    assert!(html.contains("Driver 1"));
    assert!(html.contains("Ready"));
    assert!(html.contains("checked=\"false\""));

    // Apply partial diff - only field "3" changes from "false" to "true"
    let diff_json = r#"{
        "0": {
            "k": {
                "0": {"3": "true"},
                "kc": 1
            }
        }
    }"#;

    let diff: RootDiff = serde_json::from_str(diff_json).expect("Failed to deserialize diff");
    let merged = root.merge(diff).expect("Failed to merge");
    let html: String = merged.try_into().expect("Failed to render");

    // All original fields should still be present
    assert!(html.contains("601"), "Route ID should be preserved");
    assert!(html.contains("Driver 1"), "Driver name should be preserved");
    assert!(html.contains("Ready"), "Status should be preserved");
    // And the changed field should be updated
    assert!(html.contains("checked=\"true\""), "Checked should be updated to true");
}

#[test]
fn keyed_comprehension_partial_update_multiple_items() {
    // Test that when we have multiple items and only one gets a diff,
    // the other items are preserved completely
    let initial_json = r#"{
        "k": {
            "0": {"0": "Item A", "1": "off"},
            "1": {"0": "Item B", "1": "off"},
            "2": {"0": "Item C", "1": "off"},
            "kc": 3
        },
        "p": {"0": ["<Row><Text>", "</Text><Switch checked=\"", "\"/></Row>"]},
        "s": 0
    }"#;

    let root: RootDiff = serde_json::from_str(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");
    let html: String = root.clone().try_into().expect("Failed to render");

    assert!(html.contains("Item A"));
    assert!(html.contains("Item B"));
    assert!(html.contains("Item C"));

    // Only update item 1's toggle - items 0 and 2 should remain unchanged
    let diff_json = r#"{
        "k": {
            "1": {"1": "on"},
            "kc": 3
        }
    }"#;

    let diff: RootDiff = serde_json::from_str(diff_json).expect("Failed to deserialize diff");
    let merged = root.merge(diff).expect("Failed to merge");
    let html: String = merged.try_into().expect("Failed to render");

    // All items should still be present
    assert!(html.contains("Item A"), "Item A should be preserved");
    assert!(html.contains("Item B"), "Item B should be preserved");
    assert!(html.contains("Item C"), "Item C should be preserved");

    // Item B's field should be updated, but text preserved
    assert!(html.contains("checked=\"on\""), "Item B should be toggled on");
}

#[test]
fn keyed_comprehension_sequential_toggles() {
    // Test that toggling back and forth works correctly
    let initial_json = r#"{
        "k": {
            "0": {"0": "Route 1", "1": "false"},
            "kc": 1
        },
        "p": {"0": ["<Row><Text>", "</Text><Switch checked=\"", "\"/></Row>"]},
        "s": 0
    }"#;

    let root: RootDiff = serde_json::from_str(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");

    // First toggle: false -> true
    let diff1 = r#"{"k": {"0": {"1": "true"}, "kc": 1}}"#;
    let diff1: RootDiff = serde_json::from_str(diff1).expect("deserialize");
    let root = root.merge(diff1).expect("merge");
    let html: String = root.clone().try_into().expect("render");
    assert!(html.contains("Route 1"), "Name preserved after first toggle");
    assert!(html.contains("checked=\"true\""), "First toggle worked");

    // Second toggle: true -> false
    let diff2 = r#"{"k": {"0": {"1": "false"}, "kc": 1}}"#;
    let diff2: RootDiff = serde_json::from_str(diff2).expect("deserialize");
    let root = root.merge(diff2).expect("merge");
    let html: String = root.clone().try_into().expect("render");
    assert!(html.contains("Route 1"), "Name preserved after second toggle");
    assert!(html.contains("checked=\"false\""), "Second toggle worked");

    // Third toggle: false -> true again
    let diff3 = r#"{"k": {"0": {"1": "true"}, "kc": 1}}"#;
    let diff3: RootDiff = serde_json::from_str(diff3).expect("deserialize");
    let root = root.merge(diff3).expect("merge");
    let html: String = root.try_into().expect("render");
    assert!(html.contains("Route 1"), "Name preserved after third toggle");
    assert!(html.contains("checked=\"true\""), "Third toggle worked");
}

#[test]
fn keyed_comprehension_add_item_with_partial_update() {
    // Test adding a new item while also updating an existing one
    let initial_json = r#"{
        "k": {
            "0": {"0": "Item 1", "1": "active"},
            "kc": 1
        },
        "p": {"0": ["<Row><Text>", "</Text><Status>", "</Status></Row>"]},
        "s": 0
    }"#;

    let root: RootDiff = serde_json::from_str(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");

    // Add new item 1, and update item 0's status
    let diff_json = r#"{
        "k": {
            "0": {"1": "inactive"},
            "1": {"0": "Item 2", "1": "active"},
            "kc": 2
        }
    }"#;

    let diff: RootDiff = serde_json::from_str(diff_json).expect("Failed to deserialize diff");
    let merged = root.merge(diff).expect("Failed to merge");
    let html: String = merged.try_into().expect("Failed to render");

    // Both items should be present with correct values
    assert!(html.contains("Item 1"), "Item 1 name should be preserved");
    assert!(html.contains("Item 2"), "Item 2 should be added");
    assert!(html.contains("<Status>inactive</Status>"), "Item 1 status should be updated");
    assert!(html.contains("<Status>active</Status>"), "Item 2 status should be active");
}

#[test]
fn keyed_comprehension_remove_item_partial_update_remaining() {
    // Test removing an item while updating the remaining one
    let initial_json = r#"{
        "k": {
            "0": {"0": "Item A", "1": "value1"},
            "1": {"0": "Item B", "1": "value2"},
            "kc": 2
        },
        "p": {"0": ["<Row><Text>", "</Text><Data>", "</Data></Row>"]},
        "s": 0
    }"#;

    let root: RootDiff = serde_json::from_str(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");

    // Remove item 1, update item 0
    let diff_json = r#"{
        "k": {
            "0": {"1": "updated_value"},
            "kc": 1
        }
    }"#;

    let diff: RootDiff = serde_json::from_str(diff_json).expect("Failed to deserialize diff");
    let merged = root.merge(diff).expect("Failed to merge");
    let html: String = merged.try_into().expect("Failed to render");

    // Only item A should remain with updated value
    assert!(html.contains("Item A"), "Item A name should be preserved");
    assert!(html.contains("<Data>updated_value</Data>"), "Item A data should be updated");
    // Item B should be gone (kc reduced to 1)
    assert!(!html.contains("Item B"), "Item B should be removed");
}

#[test]
fn keyed_comprehension_deeply_nested_partial_update() {
    // Test partial update in a deeply nested structure
    let initial_json = r#"{
        "0": {
            "0": {
                "k": {
                    "0": {"0": "route-1", "1": "Driver A", "2": "Pending", "3": "false", "4": "5"},
                    "1": {"0": "route-2", "1": "Driver B", "2": "Active", "3": "true", "4": "10"},
                    "kc": 2
                },
                "s": 0
            },
            "s": 1
        },
        "p": {
            "0": ["<Row id=\"", "\"><Text>", "</Text><Badge>", "</Badge><Switch checked=\"", "\"/><Count>", "</Count></Row>"],
            "1": ["<Column>", "</Column>"]
        },
        "s": ["<Root>", "</Root>"]
    }"#;

    let root: RootDiff = serde_json::from_str(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");
    let html: String = root.clone().try_into().expect("Failed to render");

    // Verify initial state
    assert!(html.contains("Driver A"));
    assert!(html.contains("Driver B"));
    assert!(html.contains("Pending"));
    assert!(html.contains("Active"));

    // Partial update: change status and count for route-1 only
    let diff_json = r#"{
        "0": {
            "0": {
                "k": {
                    "0": {"2": "Complete", "4": "0"},
                    "kc": 2
                }
            }
        }
    }"#;

    let diff: RootDiff = serde_json::from_str(diff_json).expect("Failed to deserialize diff");
    let merged = root.merge(diff).expect("Failed to merge");
    let html: String = merged.try_into().expect("Failed to render");

    // Route 1 should be updated
    assert!(html.contains("route-1"), "route-1 id preserved");
    assert!(html.contains("Driver A"), "Driver A preserved");
    assert!(html.contains("Complete"), "Status updated to Complete");
    assert!(html.contains("<Count>0</Count>"), "Count updated to 0");

    // Route 2 should be completely unchanged
    assert!(html.contains("route-2"), "route-2 id preserved");
    assert!(html.contains("Driver B"), "Driver B preserved");
    assert!(html.contains("Active"), "Active status preserved");
    assert!(html.contains("<Count>10</Count>"), "Count 10 preserved");
}

/// Test that toggling a conditional multiple times doesn't cause duplication.
/// This catches bugs where stale state accumulates across transitions.
#[test]
fn conditional_toggles_multiple_times_without_duplication() {
    // Initial: empty list, conditional shown
    let initial_json = json!({
        "0": {"d": [], "s": ["<Item>", "</Item>"]},
        "1": {"s": ["<Empty/>"]},
        "s": ["<Root>", "", "", "</Root>"]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");

    // Toggle 1: show list, hide empty state
    let diff1 = json!({"0": {"d": [["A"]]}, "1": ""});
    let diff1: RootDiff = serde_json::from_value(diff1).expect("deserialize diff1");
    let merged1 = root.merge(diff1).expect("merge diff1");
    let html1: String = merged1.clone().try_into().expect("render");
    assert!(html1.contains("A"), "Toggle 1: Should have item A. Got: {}", html1);
    assert!(!html1.contains("<Empty/>"), "Toggle 1: Should not have Empty. Got: {}", html1);

    // Toggle 2: hide list, show empty state
    let diff2 = json!({"0": {"d": []}, "1": {"s": ["<Empty/>"]}});
    let diff2: RootDiff = serde_json::from_value(diff2).expect("deserialize diff2");
    let merged2 = merged1.merge(diff2).expect("merge diff2");
    let html2: String = merged2.clone().try_into().expect("render");
    assert!(!html2.contains("A"), "Toggle 2: Item A should be gone. Got: {}", html2);
    assert!(html2.contains("<Empty/>"), "Toggle 2: Should have Empty. Got: {}", html2);
    assert_eq!(html2.matches("<Empty/>").count(), 1, "Toggle 2: Should have exactly one Empty. Got: {}", html2);

    // Toggle 3: show list again with different item
    let diff3 = json!({"0": {"d": [["B"]]}, "1": ""});
    let diff3: RootDiff = serde_json::from_value(diff3).expect("deserialize diff3");
    let merged3 = merged2.merge(diff3).expect("merge diff3");
    let html3: String = merged3.try_into().expect("render");

    // Final state should have exactly one "B", no "A", no "<Empty/>"
    assert_eq!(html3.matches("<Empty/>").count(), 0, "Toggle 3: Should have no Empty. Got: {}", html3);
    assert_eq!(html3.matches("B").count(), 1, "Toggle 3: Should have exactly one B. Got: {}", html3);
    assert_eq!(html3.matches("A").count(), 0, "Toggle 3: Should have no A. Got: {}", html3);
}

/// Test conditional with keyed comprehension sibling.
/// This is closer to a real-world routes scenario.
#[test]
fn conditional_with_keyed_comprehension_sibling() {
    // Initial: no routes, empty state shown
    let initial_json = json!({
        "0": {
            "0": {
                "k": {"kc": 0},  // Empty keyed comprehension
                "s": 0
            },
            "1": {"s": ["<Box>No routes available</Box>"]},  // Empty state shown
            "s": 1
        },
        "p": {
            "0": ["<Row><Text>", "</Text></Row>"],  // Route item template
            "1": ["<Column><LazyColumn>", "</LazyColumn>", "</Column>"]  // Container
        },
        "s": ["", ""]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");
    let html: String = root.clone().try_into().expect("Failed to render");

    assert!(html.contains("No routes available"), "Initial: Should show empty state. Got: {}", html);

    // Diff: add a route, hide empty state
    let diff_json = json!({
        "0": {
            "0": {
                "k": {
                    "0": {"0": "Route ABC"},
                    "kc": 1
                }
            },
            "1": ""  // Hide empty state
        }
    });

    let diff: RootDiff = serde_json::from_value(diff_json).expect("Failed to deserialize diff");
    let merged = root.merge(diff).expect("Failed to merge");
    let html: String = merged.clone().try_into().expect("Failed to render");

    assert!(!html.contains("No routes available"), "After adding route: Empty state should be hidden. Got: {}", html);
    assert!(html.contains("Route ABC"), "After adding route: Should show the route. Got: {}", html);

    // Diff: remove all routes, show empty state again
    let diff2_json = json!({
        "0": {
            "0": {
                "k": {"kc": 0}  // Empty the keyed comprehension
            },
            "1": {"s": ["<Box>No routes available</Box>"]}  // Show empty state
        }
    });

    let diff2: RootDiff = serde_json::from_value(diff2_json).expect("Failed to deserialize diff2");
    let merged2 = merged.merge(diff2).expect("Failed to merge diff2");
    let html2: String = merged2.try_into().expect("Failed to render");

    assert!(html2.contains("No routes available"), "After removing routes: Empty state should reappear. Got: {}", html2);
    assert!(!html2.contains("Route ABC"), "After removing routes: Route should be gone. Got: {}", html2);
    // Ensure no duplication
    assert_eq!(html2.matches("No routes available").count(), 1,
        "Should have exactly one empty state message. Got: {}", html2);
}

/// Test that TemplateRef rendering handles missing children gracefully.
///
/// When using a template reference (s: 0 instead of inline s: [...]),
/// if a child is missing (e.g., hidden by `:if` conditional becoming false),
/// the renderer should handle it like inline statics do - by just rendering
/// nothing for that slot rather than erroring with ChildNotFoundForTemplate.
///
/// This bug manifests as "Child N for template" errors when `:if` conditions
/// toggle elements on/off.
#[test]
fn template_ref_handles_missing_children_gracefully() {
    // Initial render with all children present
    // Template "0" expects 2 children: ["<Row>", "</Row><Text>", "</Text>"]
    let initial_json = json!({
        "0": "visible content",
        "1": "other content",
        "p": {"0": ["<Row>", "</Row><Text>", "</Text>"]},
        "s": 0
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");
    let html: String = root.clone().try_into().expect("Failed to render");

    assert!(html.contains("visible content"), "Initial: Should have visible content");
    assert!(html.contains("other content"), "Initial: Should have other content");

    // Now apply a diff that hides child 0 (simulating :if={false})
    // Note: child 1 stays, but child 0 is gone from the children map entirely
    // (This is how Phoenix sends the diff when :if changes to false)
    let diff_json = json!({
        "0": ""  // Child 0 hidden - becomes empty string
    });

    let diff: RootDiff = serde_json::from_value(diff_json).expect("Failed to deserialize diff");
    let merged = root.merge(diff).expect("Failed to merge");

    // This should NOT fail with ChildNotFoundForTemplate
    // Instead, it should render with child 0 as empty
    let html: String = merged.try_into().expect("Should render successfully even with hidden child");

    assert!(!html.contains("visible content"), "After diff: visible content should be hidden");
    assert!(html.contains("other content"), "After diff: other content should remain");
}

/// Test that when a child becomes hidden (empty string), the template
/// renders correctly without that child's content.
#[test]
fn template_ref_with_child_becoming_empty_string() {
    // Template has 3 slots: static1 + child0 + static2 + child1 + static3
    let initial_json = json!({
        "0": {
            "0": "row-content",
            "1": "progress-bar",
            "s": 0
        },
        "p": {
            "0": ["<Card><Row>", "</Row><LinearProgressIndicator progress=\"", "\"/></Card>"]
        },
        "s": ["<Container>", "</Container>"]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");
    let html: String = root.clone().try_into().expect("Failed to render");

    assert!(html.contains("row-content"), "Initial: Should have row content");
    assert!(html.contains("progress-bar"), "Initial: Should have progress bar");

    // Hide the Row by setting child 0 to empty string
    let diff_json = json!({
        "0": {
            "0": ""  // Row becomes hidden
        }
    });

    let diff: RootDiff = serde_json::from_value(diff_json).expect("Failed to deserialize diff");
    let merged = root.merge(diff).expect("Failed to merge");
    let html: String = merged.try_into().expect("Should render without error");

    assert!(!html.contains("row-content"), "After diff: row content should be hidden");
    assert!(html.contains("progress-bar"), "After diff: progress bar should remain");
}

/// Test 2: Sibling Template Independence
/// Two siblings both use template 0
/// Diff updates one sibling's template 0
/// Other sibling should keep original template 0
#[test]
fn phoenix_template_scoping_siblings() {
    // Two siblings both using template 0 initially
    let initial_json = json!({
        "0": {
            "0": "Sibling A content",
            "s": 0
        },
        "1": {
            "0": "Sibling B content",
            "s": 0
        },
        "p": {
            "0": ["<Box>", "</Box>"]
        },
        "s": ["<Root>", "", "</Root>"]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");
    let html: String = root.clone().try_into().expect("Failed to render");

    assert!(html.contains("<Box>Sibling A content</Box>"), "Initial: Should have Sibling A in Box. Got: {}", html);
    assert!(html.contains("<Box>Sibling B content</Box>"), "Initial: Should have Sibling B in Box. Got: {}", html);

    // Sibling A gets updated with a new template structure
    // But Sibling B should keep using the original template 0
    let diff_json = json!({
        "0": {
            "0": "Updated A",
            "s": 1  // Sibling A now uses template 1
        },
        "p": {
            "1": ["<Strong>", "</Strong>"]  // Template 1 for sibling A
        }
    });

    let diff: RootDiff = serde_json::from_value(diff_json).expect("Failed to deserialize diff");
    let merged = root.merge(diff).expect("Failed to merge");
    let html: String = merged.try_into().expect("Failed to render after merge");

    // Sibling A should use the new Strong template
    assert!(html.contains("<Strong>Updated A</Strong>"), "After merge: Sibling A should use Strong. Got: {}", html);

    // Sibling B should STILL use the original Box template
    assert!(html.contains("<Box>Sibling B content</Box>"), "After merge: Sibling B should still use original Box. Got: {}", html);
}

/// Regression test for the route-change template overwrite crash.
///
/// When a diff overwrites a template index (e.g. `p.0` changes from Card statics to Column
/// statics), existing keyed comprehension items that held `TemplateRef(0)` must still render
/// with the *old* (Card) statics, not the new (Column) statics.
///
/// Without `expand_statics`, TemplateRef(0) remained unresolved in keyed comp items.
/// A later diff that replaced `p.0` with different statics caused a
/// `ChildNotFoundForTemplate` crash because the fragment's children didn't match the
/// new template's slot count.
#[test]
fn keyed_comp_items_survive_template_overwrite() {
    // Step 1: Initial render — Card-like template with 3 slots in p.0
    // The keyed comp items use s:0 (TemplateRef to p.0)
    // Each keyed item has children 0, 1, 2 matching the 3-slot Card template.
    let initial_json = json!({
        "0": {
            "k": {
                "0": {"0": "Card Title", "1": "Card Body", "2": "Card Footer"},
                "1": {"0": "Card Title 2", "1": "Card Body 2", "2": "Card Footer 2"},
                "kc": 2
            },
            "s": 0
        },
        "p": {
            "0": ["<Card><Title>", "</Title><Body>", "</Body><Footer>", "</Footer></Card>"]
        },
        "s": ["<Root>", "</Root>"]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");
    let html: String = root.clone().try_into().expect("Failed to render initial");

    // Verify initial render
    assert!(html.contains("<Card>"), "Initial: should have Card. Got: {}", html);
    assert!(html.contains("Card Title"), "Initial: should have Card Title. Got: {}", html);
    assert!(html.contains("Card Footer 2"), "Initial: should have Card Footer 2. Got: {}", html);

    // After expand_statics, TemplateRef(0) should have been resolved to inline statics.
    // Verify that by checking the fragment structure:
    match &root.fragment {
        Fragment::Regular { children, .. } => {
            if let Some(Child::Fragment(Fragment::KeyedComprehension { statics, .. })) =
                children.get("0")
            {
                // The statics should now be Statics::Statics (resolved), not TemplateRef
                assert!(
                    matches!(statics, Some(Statics::Statics(_))),
                    "Keyed comp statics should be resolved from TemplateRef to Statics. Got: {:?}",
                    statics
                );
            } else {
                panic!("Expected child 0 to be a KeyedComprehension fragment");
            }
        }
        _ => panic!("Expected Regular fragment at root"),
    }

    // Step 2: Merge a diff that overwrites p.0 with a Column template (only 1 slot).
    // The keyed comp items are NOT updated in this diff — they should keep the old
    // resolved Card statics. Without expand_statics, they'd still hold TemplateRef(0)
    // which now points to the Column template, causing ChildNotFoundForTemplate.
    let diff_json = json!({
        "p": {
            "0": ["<Column>", "</Column>"]
        }
    });

    let diff: RootDiff = serde_json::from_value(diff_json).expect("Failed to deserialize diff");
    let merged = root.merge(diff).expect("Failed to merge");

    // This is the critical assertion: rendering should succeed, not crash
    let html: String = merged.try_into().expect(
        "Render after template overwrite should succeed - keyed items should use old resolved statics"
    );

    // The keyed comp items should still render with their Card statics
    assert!(html.contains("<Card>"), "After overwrite: Card template should be preserved in keyed items. Got: {}", html);
    assert!(html.contains("Card Title"), "After overwrite: Card Title should be preserved. Got: {}", html);
    assert!(html.contains("Card Footer 2"), "After overwrite: Card Footer 2 should be preserved. Got: {}", html);
}

/// Regression test: after a template overwrite at the root, new children within
/// keyed comp items that have their own TemplateRef must resolve against the
/// root's *current* templates, not stale inherited copies.
///
/// Scenario:
/// 1. Initial render: ROOT has p.0 = Card template (2 slots).
///    Keyed comp uses inline statics. Each item has a child fragment with s:0.
/// 2. Diff overwrites p.0 → Strong template (1 slot), and adds a new item "2"
///    whose child fragment also has s:0.
/// 3. After merge, new item's child TemplateRef(0) should resolve to Strong,
///    NOT stale Card template.
#[test]
fn template_overwrite_preserves_existing_items_but_resolves_new_refs() {
    // Step 1: Initial render
    // ROOT has p.0 = Card template. Keyed comp uses inline statics.
    // Each keyed item has child "0" which is a fragment with s:0 (TemplateRef to p.0).
    let initial_json = json!({
        "0": {
            "k": {
                "0": {
                    "0": {
                        "0": "Card Title A",
                        "1": "Card Body A",
                        "s": 0
                    }
                },
                "1": {
                    "0": {
                        "0": "Card Title B",
                        "1": "Card Body B",
                        "s": 0
                    }
                },
                "kc": 2
            },
            "s": ["<Item>", "</Item>"]
        },
        "p": {
            "0": ["<Card><Title>", "</Title><Body>", "</Body></Card>"]
        },
        "s": ["<Root>", "</Root>"]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");
    let html: String = root.clone().try_into().expect("Failed to render initial");

    // Verify initial render has Card templates
    assert!(html.contains("<Card>"), "Initial should have Card. Got: {}", html);
    assert!(html.contains("Card Title A"), "Initial should have Card Title A. Got: {}", html);
    assert!(html.contains("Card Body B"), "Initial should have Card Body B. Got: {}", html);

    // Step 2: Merge diff that:
    //   - Overwrites p.0 → Strong template (1 slot)
    //   - Keeps existing items 0, 1 (moved from old positions)
    //   - Adds new item "2" with a child fragment that has s:0
    let diff_json = json!({
        "0": {
            "k": {
                "0": 0,
                "1": 1,
                "2": {
                    "0": {
                        "0": "New Strong Content",
                        "s": 0
                    }
                },
                "kc": 3
            }
        },
        "p": {
            "0": ["<Strong>", "</Strong>"]
        }
    });

    let diff: RootDiff = serde_json::from_value(diff_json).expect("Failed to deserialize diff");
    let merged = root.merge(diff).expect("Failed to merge");

    // This is the critical assertion: rendering should succeed.
    // The new item "2"'s child has s:0, which should resolve to Strong (1 slot).
    // If expand_statics used stale templates (Card, 2 slots), the child would
    // resolve against the old Card template instead of the new Strong template.
    let html: String = merged.try_into().expect(
        "Render after template overwrite should succeed - new item's child should use current Strong template"
    );

    // Existing items should still render with their already-resolved Card statics
    // (their TemplateRefs were resolved during initial expand_statics)
    assert!(html.contains("<Card>"), "After merge: existing items should still use Card. Got: {}", html);
    assert!(html.contains("Card Title A"), "After merge: Card Title A should be preserved. Got: {}", html);

    // New item's child should render with Strong template (current p.0)
    assert!(html.contains("<Strong>New Strong Content</Strong>"), "After merge: new item's child should use Strong (current template). Got: {}", html);
}

/// Regression test: deeply nested TemplateRefs must resolve against the root's
/// *current* templates, passed down through the tree — not stale inherited copies.
///
/// Scenario:
/// 1. ROOT has p: {"0": tmpl_X, "1": tmpl_Y}. Nested child at depth 3 has s:1
/// 2. Merge diff updating p.1 → tmpl_Z, and adding new nested fragment with s:1
/// 3. The new s:1 ref should resolve to tmpl_Z, not stale tmpl_Y
#[test]
fn nested_template_ref_uses_root_templates_not_stale_inherited() {
    // Step 1: Initial render — ROOT has two templates, deeply nested child uses s:1
    let initial_json = json!({
        "0": {
            "0": {
                "0": "deep value Y",
                "s": 1
            },
            "s": ["<Middle>", "</Middle>"]
        },
        "p": {
            "0": ["<Box>", "</Box>"],
            "1": ["<Text>", "</Text>"]
        },
        "s": ["<Root>", "</Root>"]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("Failed to deserialize");
    let root: Root = root.try_into().expect("Failed to convert");
    let html: String = root.clone().try_into().expect("Failed to render initial");

    // The deeply nested s:1 should have resolved to tmpl_Y = ["<Text>", "</Text>"]
    assert!(html.contains("<Text>deep value Y</Text>"), "Initial: nested child should use Text template. Got: {}", html);

    // Step 2: Merge diff that:
    //   - Updates p.1 → tmpl_Z = ["<Strong>", "</Strong>"]
    //   - Adds a new nested child with s:1
    let diff_json = json!({
        "0": {
            "0": {
                "0": "deep value Z",
                "s": 1
            }
        },
        "p": {
            "1": ["<Strong>", "</Strong>"]
        }
    });

    let diff: RootDiff = serde_json::from_value(diff_json).expect("Failed to deserialize diff");
    let merged = root.merge(diff).expect("Failed to merge");

    let html: String = merged.try_into().expect(
        "Render after template update should succeed"
    );

    // The nested s:1 ref should resolve to the NEW tmpl_Z = Strong, not stale tmpl_Y = Text
    assert!(html.contains("<Strong>deep value Z</Strong>"), "After merge: nested s:1 should resolve to Strong (new template), not Text (stale). Got: {}", html);
}

// expand_statics regression tests — all 6 fail if expand_statics is removed.
// They exercise multi-step diff sequences that mirror real Phoenix sessions.

/// Structural test: after Root creation, TemplateRefs must be resolved
/// to Statics::Statics and templates must be None (deleted).
#[test]
fn expand_statics_resolves_refs_and_deletes_templates() {
    let initial_json = json!({
        "0": {
            "0": "content",
            "s": 0
        },
        "p": {
            "0": ["<Box>", "</Box>"]
        },
        "s": ["<Root>", "</Root>"]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("parse");
    let root: Root = root.try_into().expect("convert");

    match &root.fragment {
        Fragment::Regular { children, templates, .. } => {
            // Templates must be None — expand_statics deletes them after resolution
            assert_eq!(*templates, None, "templates should be None after expand_statics");

            // Child's statics must be resolved from TemplateRef(0) to Statics::Statics
            match children.get("0") {
                Some(Child::Fragment(Fragment::Regular { statics, templates: child_templates, .. })) => {
                    assert!(
                        matches!(statics, Some(Statics::Statics(v)) if *v == vec!["<Box>".to_string(), "</Box>".to_string()]),
                        "Child statics should be resolved to [\"<Box>\", \"</Box>\"]. Got: {:?}",
                        statics
                    );
                    assert_eq!(
                        *child_templates, None,
                        "Child templates should also be None"
                    );
                }
                other => panic!("Expected Regular fragment child, got: {:?}", other),
            }
        }
        other => panic!("Expected Regular fragment, got: {:?}", other),
    }
}

/// A diff that sends only NEW template keys (not all) works because
/// expand_statics already resolved old refs and deleted old templates.
/// Without expand_statics: TemplateNotFound(0) on render.
#[test]
fn partial_template_diff_after_expand_statics_deletion() {
    // Initial: two siblings both using template 0
    let initial_json = json!({
        "0": {
            "0": "Sibling A",
            "s": 0
        },
        "1": {
            "0": "Sibling B",
            "s": 0
        },
        "p": {
            "0": ["<Box>", "</Box>"]
        },
        "s": ["<Root>", "", "</Root>"]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("parse");
    let root: Root = root.try_into().expect("convert");

    // Verify initial render
    let html: String = root.clone().try_into().expect("render initial");
    assert!(html.contains("<Box>Sibling A</Box>"), "Initial A: {}", html);
    assert!(html.contains("<Box>Sibling B</Box>"), "Initial B: {}", html);

    // Diff: sibling A switches to template 1 (Strong).
    // Only template 1 is sent — template 0 is NOT included.
    // This works because expand_statics already resolved sibling B's statics
    // to Statics::Statics(["<Box>", "</Box>"]) and deleted all templates.
    let diff_json = json!({
        "0": {
            "0": "Updated A",
            "s": 1
        },
        "p": {
            "1": ["<Strong>", "</Strong>"]
        }
    });

    let diff: RootDiff = serde_json::from_value(diff_json).expect("parse diff");
    let merged = root.merge(diff).expect("merge");
    let html: String = merged.try_into().expect("render after partial template diff");

    assert!(html.contains("<Strong>Updated A</Strong>"), "A should use Strong: {}", html);
    assert!(html.contains("<Box>Sibling B</Box>"), "B should keep Box: {}", html);
}

/// Three-step sequence: render → content update → template change.
/// Without expand_statics, step 3 crashes (3-slot items vs 1-slot Column).
#[test]
fn three_step_sequence_dynamic_update_then_template_change() {
    // Step 1: Initial render with Card template (3 slots)
    let initial_json = json!({
        "0": {
            "k": {
                "0": {"0": "Title A", "1": "Body A", "2": "Footer A"},
                "1": {"0": "Title B", "1": "Body B", "2": "Footer B"},
                "kc": 2
            },
            "s": 0
        },
        "p": {
            "0": ["<Card><Title>", "</Title><Body>", "</Body><Footer>", "</Footer></Card>"]
        },
        "s": ["<Root>", "</Root>"]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("parse");
    let root: Root = root.try_into().expect("convert");
    let html: String = root.clone().try_into().expect("render step 1");
    assert!(html.contains("<Card><Title>Title A</Title>"), "Step 1: {}", html);

    // Step 2: Dynamic content update — no template change, just content
    let diff2 = json!({
        "0": {
            "k": {
                "0": {"0": "Title A UPDATED", "1": "Body A", "2": "Footer A"},
                "1": 1,
                "kc": 2
            }
        }
    });

    let diff: RootDiff = serde_json::from_value(diff2).expect("parse diff 2");
    let merged = root.merge(diff).expect("merge step 2");
    let html: String = merged.clone().try_into().expect("render step 2");
    assert!(html.contains("<Card><Title>Title A UPDATED</Title>"), "Step 2 updated: {}", html);
    assert!(html.contains("<Card><Title>Title B</Title>"), "Step 2 preserved: {}", html);

    // Step 3: Template change — p.0 becomes Column (1 slot).
    // Existing items have 3 children each. Without expand_statics, their
    // TemplateRef(0) would now resolve to Column (1 slot) → crash.
    let diff3 = json!({
        "p": {
            "0": ["<Column>", "</Column>"]
        }
    });

    let diff: RootDiff = serde_json::from_value(diff3).expect("parse diff 3");
    let merged2 = merged.merge(diff).expect("merge step 3");

    // Critical: this must not crash. Items keep their resolved Card statics.
    let html: String = merged2.try_into().expect("render step 3");
    assert!(html.contains("<Card>"), "Step 3: items should still render with Card: {}", html);
    assert!(html.contains("Title A UPDATED"), "Step 3: content should be preserved: {}", html);
}

/// Old keyed items' children keep resolved Card statics (frozen);
/// new items' children get the current Badge template.
/// Without expand_statics, all items switch to Badge (wrong for old items).
#[test]
fn new_keyed_items_get_current_template_old_items_keep_resolved() {
    // Initial: keyed comp with inline statics, each item has a child fragment
    // with TemplateRef(0) pointing to Card template (2 slots)
    let initial_json = json!({
        "0": {
            "k": {
                "0": {
                    "0": {
                        "0": "Card A",
                        "1": "Detail A",
                        "s": 0
                    }
                },
                "1": {
                    "0": {
                        "0": "Card B",
                        "1": "Detail B",
                        "s": 0
                    }
                },
                "kc": 2
            },
            "s": ["<Item>", "</Item>"]
        },
        "p": {
            "0": ["<Card><Name>", "</Name><Detail>", "</Detail></Card>"]
        },
        "s": ["<Root>", "</Root>"]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("parse");
    let root: Root = root.try_into().expect("convert");
    let html: String = root.clone().try_into().expect("render initial");
    assert!(html.contains("<Card><Name>Card A</Name>"), "Initial: {}", html);

    // Diff: template 0 changes to Badge (1 slot), and a new item "2" arrives
    // with a child fragment that has TemplateRef(0) → should resolve to Badge.
    // Old items 0,1 keep old positions (children already resolved to Card).
    let diff_json = json!({
        "0": {
            "k": {
                "0": 0,
                "1": 1,
                "2": {
                    "0": {
                        "0": "New Badge",
                        "s": 0
                    }
                },
                "kc": 3
            }
        },
        "p": {
            "0": ["<Badge>", "</Badge>"]
        }
    });

    let diff: RootDiff = serde_json::from_value(diff_json).expect("parse diff");
    let merged = root.merge(diff).expect("merge");
    let html: String = merged.try_into().expect("render after template change");

    // Old items' children should still use Card (resolved before template change)
    assert!(html.contains("<Card>"), "Old items should keep Card: {}", html);
    assert!(html.contains("Card A"), "Old item A content preserved: {}", html);
    assert!(html.contains("Card B"), "Old item B content preserved: {}", html);

    // New item's child should use Badge (current template 0)
    assert!(html.contains("<Badge>New Badge</Badge>"), "New item should use Badge: {}", html);
}

/// Templates are None after each expand_statics pass, so partial diffs
/// with only new template keys don't accumulate stale entries.
/// Verifies structural state (templates=None, statics resolved) after each step.
#[test]
fn template_deletion_prevents_stale_key_accumulation() {
    // Initial: two children using templates 0 and 1, plus a third using inline statics
    let initial_json = json!({
        "0": {
            "0": "in box",
            "s": 0
        },
        "1": {
            "0": "in text",
            "s": 1
        },
        "2": "plain string",
        "p": {
            "0": ["<Box>", "</Box>"],
            "1": ["<Text>", "</Text>"]
        },
        "s": ["<Root>", "", "", "</Root>"]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("parse");
    let root: Root = root.try_into().expect("convert");

    // After initial convert, templates should be None (deleted by expand_statics)
    match &root.fragment {
        Fragment::Regular { templates, .. } => {
            assert_eq!(*templates, None, "Templates should be deleted after expand_statics");
        }
        _ => panic!("Expected Regular fragment"),
    }

    // Verify initial render
    let html: String = root.clone().try_into().expect("render initial");
    assert!(html.contains("<Box>in box</Box>"), "Initial Box: {}", html);
    assert!(html.contains("<Text>in text</Text>"), "Initial Text: {}", html);

    // Diff: updates child 2 to a fragment using template 2, provides ONLY template 2.
    // This is a partial template diff — it doesn't resend templates 0 and 1.
    // This works because expand_statics already resolved children 0,1 and deleted templates.
    let diff_json = json!({
        "2": {
            "0": "in badge",
            "s": 2
        },
        "p": {
            "2": ["<Badge>", "</Badge>"]
        }
    });

    let diff: RootDiff = serde_json::from_value(diff_json).expect("parse diff");
    let merged = root.merge(diff).expect("merge");

    // After merge + expand_statics, templates should be None again
    match &merged.fragment {
        Fragment::Regular { templates, children, .. } => {
            assert_eq!(*templates, None, "Templates should be None after second expand_statics");

            // Children 0 and 1 should still have their resolved statics
            for (key, expected_tag) in [("0", "<Box>"), ("1", "<Text>")] {
                match children.get(key) {
                    Some(Child::Fragment(Fragment::Regular { statics, .. })) => {
                        if let Some(Statics::Statics(s)) = statics {
                            assert!(
                                s[0] == expected_tag,
                                "Child {} should have statics starting with {}. Got: {:?}",
                                key, expected_tag, s
                            );
                        } else {
                            panic!("Child {} statics should be resolved. Got: {:?}", key, statics);
                        }
                    }
                    other => panic!("Child {} should be Regular fragment. Got: {:?}", key, other),
                }
            }

            // Child 2 should now be a fragment with resolved Badge statics
            match children.get("2") {
                Some(Child::Fragment(Fragment::Regular { statics, .. })) => {
                    assert!(
                        matches!(statics, Some(Statics::Statics(s)) if s[0] == "<Badge>"),
                        "Child 2 should have Badge statics. Got: {:?}",
                        statics
                    );
                }
                other => panic!("Child 2 should be Regular fragment. Got: {:?}", other),
            }
        }
        _ => panic!("Expected Regular fragment"),
    }

    // Render should produce all three
    let html: String = merged.try_into().expect("render");
    assert!(html.contains("<Box>in box</Box>"), "Box: {}", html);
    assert!(html.contains("<Text>in text</Text>"), "Text: {}", html);
    assert!(html.contains("<Badge>in badge</Badge>"), "Badge: {}", html);
}

/// Deep child resolved to Emphasis stays Emphasis when parent's template 0
/// changes to Strikethrough. Without expand_statics, it switches.
#[test]
fn nested_template_inheritance_survives_parent_template_change() {
    // Initial: parent has template 0 = Emphasis, deeply nested child uses it
    let initial_json = json!({
        "0": {
            "0": {
                "0": "deep content",
                "s": 0
            },
            "s": ["<Middle>", "</Middle>"]
        },
        "p": {
            "0": ["<Emphasis>", "</Emphasis>"]
        },
        "s": ["<Root>", "</Root>"]
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("parse");
    let root: Root = root.try_into().expect("convert");
    let html: String = root.clone().try_into().expect("render initial");
    assert!(html.contains("<Emphasis>deep content</Emphasis>"), "Initial: {}", html);

    // Diff: parent's template 0 changes to Strikethrough
    // Deep child already resolved to Emphasis — should be immune
    let diff_json = json!({
        "p": {
            "0": ["<Strikethrough>", "</Strikethrough>"]
        }
    });

    let diff: RootDiff = serde_json::from_value(diff_json).expect("parse diff");
    let merged = root.merge(diff).expect("merge");
    let html: String = merged.try_into().expect("render after template change");

    // Deep child keeps Emphasis (already resolved), NOT Strikethrough
    assert!(
        html.contains("<Emphasis>deep content</Emphasis>"),
        "Deep child should keep resolved Emphasis, not switch to Strikethrough: {}", html
    );
}

// ====================================================================
// Phoenix LiveView conformance tests
// Ported from: phoenix_live_view/assets/test/rendered_test.ts
// These tests use the exact JSON payloads from Phoenix's JS test suite
// to verify our template resolution and merge behavior matches Phoenix.
// ====================================================================

/// Port of Phoenix rendered_test.ts: toString("stringifies a diff")
/// Tests basic rendering with statics and dynamic children after a merge.
#[test]
fn phoenix_conformance_simple_render() {
    // simpleDiff1 from rendered_test.ts
    let initial_json = json!({
        "0": "cooling",
        "1": "cooling",
        "2": "07:15:03 PM",
        "s": [
            "<div class=\"thermostat\">\n  <div class=\"bar ",
            "\">\n    <a href=\"#\" phx-click=\"toggle-mode\">",
            "</a>\n    <span>",
            "</span>\n  </div>\n</div>\n"
        ],
        "r": 1
    });

    // simpleDiff2 from rendered_test.ts
    let diff_json = json!({
        "2": "07:15:04 PM"
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("parse initial");
    let root: Root = root.try_into().expect("convert initial");
    let diff: RootDiff = serde_json::from_value(diff_json).expect("parse diff");
    let merged = root.merge(diff).expect("merge");
    let html: String = merged.try_into().expect("render");

    // Phoenix expected output (minus data-phx-id attribute which our code doesn't generate)
    let expected = concat!(
        "<div class=\"thermostat\">\n",
        "  <div class=\"bar cooling\">\n",
        "    <a href=\"#\" phx-click=\"toggle-mode\">cooling</a>\n",
        "    <span>07:15:04 PM</span>\n",
        "  </div>\n",
        "</div>\n",
    );
    assert_eq!(html, expected);
}

/// Port of Phoenix rendered_test.ts:
/// toString("reuses static in components and comprehensions")
///
/// This is the key template conformance test. It exercises:
/// - Template refs (TEMPLATES / "p" key) with TemplateRef statics (s: 0)
/// - Nested keyed comprehensions
/// - Component references (ComponentRef statics)
/// - Template inheritance from parent to child fragments
#[test]
fn phoenix_conformance_static_reuse_with_templates_and_components() {
    // staticReuseDiff from rendered_test.ts
    let json_data = json!({
        "0": {
            "k": {
                "kc": 2,
                "0": {
                    "0": "foo",
                    "1": {
                        "k": {
                            "kc": 2,
                            "0": { "0": "0", "1": 1 },
                            "1": { "0": "1", "1": 2 }
                        },
                        "s": 0
                    }
                },
                "1": {
                    "0": "bar",
                    "1": {
                        "k": {
                            "kc": 2,
                            "0": { "0": "0", "1": 3 },
                            "1": { "0": "1", "1": 4 }
                        },
                        "s": 0
                    }
                }
            },
            "s": ["\n  <p>\n    ", "\n    ", "\n  </p>\n"],
            "r": 1,
            "p": { "0": ["<span>", ": ", "</span>"] }
        },
        "c": {
            "1": {
                "0": "index_1",
                "1": "world",
                "s": ["<b>FROM ", " ", "</b>"],
                "r": 1
            },
            "2": { "0": "index_2", "1": "world", "s": 1, "r": 1 },
            "3": { "0": "index_1", "1": "world", "s": 1, "r": 1 },
            "4": { "0": "index_2", "1": "world", "s": 3, "r": 1 }
        },
        "s": ["<div>", "</div>"],
        "r": 1
    });

    let root: RootDiff = serde_json::from_value(json_data).expect("parse");
    let root: Root = root.try_into().expect("convert");
    let html: String = root.try_into().expect("render");

    // Phoenix expected output (minus data-phx-* attributes)
    let expected = concat!(
        "<div>",
        "\n  <p>\n    foo\n    ",
        "<span>0: <b>FROM index_1 world</b></span>",
        "<span>1: <b>FROM index_2 world</b></span>",
        "\n  </p>\n",
        "\n  <p>\n    bar\n    ",
        "<span>0: <b>FROM index_1 world</b></span>",
        "<span>1: <b>FROM index_2 world</b></span>",
        "\n  </p>\n",
        "</div>",
    );
    assert_eq!(html, expected);
}

/// Port of Phoenix rendered_test.ts:
/// mergeDiff("merges the latter diff if it contains a `static` key")
///
/// When a diff includes statics, the entire fragment is replaced (not merged).
#[test]
fn phoenix_conformance_merge_replaces_on_new_static() {
    let initial_json = json!({ "0": ["a"], "1": ["b"] });
    let diff_json = json!({ "0": ["c"], "s": ["c"] });

    let root: RootDiff = serde_json::from_value(initial_json).expect("parse initial");
    let root: Root = root.try_into().expect("convert");
    let diff: RootDiff = serde_json::from_value(diff_json).expect("parse diff");
    let merged = root.merge(diff).expect("merge");

    // After replacement, statics=["c"] with 0 dynamic slots -> renders "c"
    // Child "1" from the original should be gone (full replacement)
    let html: String = merged.try_into().expect("render");
    assert_eq!(html, "c");
}

/// Port of Phoenix rendered_test.ts:
/// mergeDiff("merges the latter diff if it contains a `static` key even when nested")
#[test]
fn phoenix_conformance_merge_replaces_on_new_static_nested() {
    let initial_json = json!({ "0": { "0": ["a"], "1": ["b"] } });
    let diff_json = json!({ "0": { "0": ["c"], "s": ["c"] } });

    let root: RootDiff = serde_json::from_value(initial_json).expect("parse initial");
    let root: Root = root.try_into().expect("convert");
    let diff: RootDiff = serde_json::from_value(diff_json).expect("parse diff");
    let merged = root.merge(diff).expect("merge");

    // Outer fragment has no statics so we verify the inner structure
    match &merged.fragment {
        Fragment::Regular { children, statics, .. } => {
            assert!(statics.is_none(), "Outer fragment should have no statics");
            match children.get("0") {
                Some(Child::Fragment(Fragment::Regular {
                    statics: inner_statics,
                    children: inner_children,
                    ..
                })) => {
                    assert_eq!(
                        *inner_statics,
                        Some(Statics::Statics(vec!["c".to_string()]))
                    );
                    // Child "1" from original should be gone after replacement
                    assert!(
                        inner_children.get("1").is_none(),
                        "Child 1 should not exist after replacement"
                    );
                }
                other => panic!("Expected inner Regular fragment, got: {:?}", other),
            }
        }
        other => panic!("Expected Regular fragment, got: {:?}", other),
    }
}

/// Port of Phoenix rendered_test.ts:
/// mergeDiff("replaces a string when a map is returned")
#[test]
fn phoenix_conformance_merge_string_to_map() {
    let initial_json = json!({ "0": { "0": "<button>Press Me</button>", "s": "" } });
    let diff_json = json!({ "0": { "0": { "0": "val", "s": "" }, "s": "" } });

    let root: RootDiff = serde_json::from_value(initial_json).expect("parse initial");
    let root: Root = root.try_into().expect("convert");

    // Verify initial: child 0's child 0 is a string
    match &root.fragment {
        Fragment::Regular { children, .. } => {
            if let Some(Child::Fragment(Fragment::Regular { children: inner, .. })) =
                children.get("0")
            {
                assert!(
                    matches!(inner.get("0"), Some(Child::String(_))),
                    "Initial child 0.0 should be a string"
                );
            }
        }
        _ => panic!("Expected Regular fragment"),
    }

    let diff: RootDiff = serde_json::from_value(diff_json).expect("parse diff");
    let merged = root.merge(diff).expect("merge");

    // After merge: child 0's child 0 should now be a fragment (was a string)
    match &merged.fragment {
        Fragment::Regular { children, .. } => {
            if let Some(Child::Fragment(Fragment::Regular { children: inner, .. })) =
                children.get("0")
            {
                assert!(
                    matches!(inner.get("0"), Some(Child::Fragment(_))),
                    "After merge, child 0.0 should be a fragment (was string)"
                );
            } else {
                panic!("Expected fragment for child 0");
            }
        }
        _ => panic!("Expected Regular fragment"),
    }
}

/// Port of Phoenix rendered_test.ts:
/// mergeDiff("replaces a map when a string is returned")
#[test]
fn phoenix_conformance_merge_map_to_string() {
    let initial_json = json!({ "0": { "0": { "0": "val", "s": "" }, "s": "" } });
    let diff_json = json!({ "0": { "0": "<button>Press Me</button>", "s": "" } });

    let root: RootDiff = serde_json::from_value(initial_json).expect("parse initial");
    let root: Root = root.try_into().expect("convert");

    // Verify initial: child 0's child 0 is a fragment
    match &root.fragment {
        Fragment::Regular { children, .. } => {
            if let Some(Child::Fragment(Fragment::Regular { children: inner, .. })) =
                children.get("0")
            {
                assert!(
                    matches!(inner.get("0"), Some(Child::Fragment(_))),
                    "Initial child 0.0 should be a fragment"
                );
            }
        }
        _ => panic!("Expected Regular fragment"),
    }

    let diff: RootDiff = serde_json::from_value(diff_json).expect("parse diff");
    let merged = root.merge(diff).expect("merge");

    // After merge: child 0's child 0 should now be a string (was a fragment)
    match &merged.fragment {
        Fragment::Regular { children, .. } => {
            if let Some(Child::Fragment(Fragment::Regular { children: inner, .. })) =
                children.get("0")
            {
                assert!(
                    matches!(inner.get("0"), Some(Child::String(_))),
                    "After merge, child 0.0 should be a string (was fragment)"
                );
            } else {
                panic!("Expected fragment for child 0");
            }
        }
        _ => panic!("Expected Regular fragment"),
    }
}

/// Port of Phoenix rendered_test.ts:
/// mergeDiff("recursively merges two diffs") — deep part
///
/// Tests keyed comprehension merge with partial updates.
/// deepDiff1 has no root statics (not renderable), so we verify structure.
#[test]
fn phoenix_conformance_deep_diff_keyed_merge() {
    // deepDiff1 from rendered_test.ts
    let initial_json = json!({
        "0": {
            "0": {
                "k": {
                    "0": { "0": "user1058", "1": "1" },
                    "1": { "0": "user99", "1": "1" },
                    "kc": 2
                },
                "s": [
                    "        <tr>\n          <td>",
                    " (",
                    ")</td>\n        </tr>\n"
                ],
                "r": 1
            },
            "s": [
                "  <table>\n    <thead>\n      <tr>\n        <th>Username</th>\n        <th></th>\n      </tr>\n    </thead>\n    <tbody>\n",
                "    </tbody>\n  </table>\n"
            ],
            "r": 1
        },
        "1": {
            "k": {
                "0": {
                    "0": "asdf_asdf",
                    "1": "asdf@asdf.com",
                    "2": "123-456-7890",
                    "3": "<a href=\"/users/1\">Show</a>",
                    "4": "<a href=\"/users/1/edit\">Edit</a>",
                    "5": "<a href=\"#\" phx-click=\"delete_user\" phx-value=\"1\">Delete</a>"
                },
                "kc": 1
            },
            "s": [
                "    <tr>\n      <td>",
                "</td>\n      <td>",
                "</td>\n      <td>",
                "</td>\n\n      <td>\n",
                "        ",
                "\n",
                "      </td>\n    </tr>\n"
            ],
            "r": 1
        }
    });

    // deepDiff2 from rendered_test.ts
    let diff_json = json!({
        "0": {
            "0": {
                "k": {
                    "0": { "0": "user1058", "1": "2" },
                    "kc": 1
                }
            }
        }
    });

    let root: RootDiff = serde_json::from_value(initial_json).expect("parse initial");
    let root: Root = root.try_into().expect("convert");
    let diff: RootDiff = serde_json::from_value(diff_json).expect("parse diff");
    let merged = root.merge(diff).expect("merge");

    // Verify the inner keyed comp was updated:
    // - key_count should be 1 (was 2)
    // - item "0" should have "1" = "2" (was "1")
    match &merged.fragment {
        Fragment::Regular { children, .. } => {
            let child_0 = children.get("0").expect("child 0");
            if let Child::Fragment(Fragment::Regular { children: inner, .. }) = child_0 {
                let child_0_0 = inner.get("0").expect("child 0.0");
                if let Child::Fragment(Fragment::KeyedComprehension { keyed, .. }) = child_0_0 {
                    assert_eq!(keyed.key_count, 1, "key_count should be 1 after merge");
                    let item_0 = keyed.items.get("0").expect("keyed item 0");
                    if let KeyedItem::Fragment(frag) = item_0 {
                        if let Fragment::Regular { children: item_children, .. } = frag.as_ref() {
                            let val = item_children.get("1").expect("child 1 of item 0");
                            assert_eq!(
                                *val,
                                Child::String(OneOrManyStrings::One("2".to_string())),
                                "item 0's child 1 should be '2' after merge"
                            );
                        } else {
                            panic!("Expected Regular fragment for keyed item");
                        }
                    } else {
                        panic!("Expected Fragment keyed item");
                    }
                } else {
                    panic!("Expected KeyedComprehension for child 0.0");
                }
            } else {
                panic!("Expected Fragment for child 0");
            }
        }
        _ => panic!("Expected Regular fragment"),
    }
}

