use pretty_assertions::assert_eq;

use crate::dom::Document;

use super::*;
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
    let out : String = root.clone().try_into().expect("Failed to render root as string");
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

    let mut document = Document::parse_fragment_json(initial).expect("Document failed to parse fragment json");
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
    let out : String = root.clone().try_into().expect("Failed to render root as string");
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
    let new_document = Document::parse(out).expect("Failed to parse rendered dom");
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
            <Icon imageVector="filled:Add"></Icon>
            <Title>Alert Title</Title>
            <Content>
                <Text>Alert message</Text>
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

    assert_eq!(document.to_string(), document_expected);
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
    let _out : String = root.clone().try_into().expect("Failed to convert root to string");
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
    let _out : String = root.clone().try_into().expect("Failed to convert root to string");
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
    let _out : String = root.clone().try_into().expect("Failed to convert root to string");
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
    let new_diff : RootDiff = serde_json::from_str(increment).expect("Failed to deserialize diff fragment");
    let root = root.merge(new_diff).expect("Failed to merge new root in");
    let _out : String = root.clone().try_into().expect("Failed to convert root to string");
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
    let new_diff : RootDiff = serde_json::from_str(increment).expect("Failed to deserialize diff fragment");
    let root = root.merge(new_diff).expect("Failed to merge new root in");
    let out : String = root.try_into().expect("Failed to convert root to string");
    let expected = r#"<Column>
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
    assert_eq!(out, expected);

}
#[test]
fn jetpack_simple_counter() {
    let initial_json = r#"{
        "0":"0",
        "s":["<Scaffold>\n  <TopAppBar>\n    <Title><Text>Hello</Text></Title>\n  </TopAppBar>\n  <Column width=\"fill\" verticalArrangement=\"center\" horizontalAlignment=\"center\">\n    <Text style=\"headlineLarge\">Title</Text>\n    <Card shape=\"8\" padding=\"16\" width=\"140\" height=\"120\" elevation=\"{'defaultElevation': '10', 'pressedElevation': '2'}\" phx-click=\"dec\">\n      <Text padding=\"16\">Hello Jetpack!</Text>\n    </Card>\n    <Spacer height=\"8\"></Spacer>\n    <Card padding=\"16\">\n      <Text padding=\"16\">Simple card</Text>\n    </Card>\n    <Button phx-click=\"navigate\" contentPadding=\"50\" elevation=\"{'defaultElevation': '20', 'pressedElevation': '10'}\">\n      <Text>Navigate to counter</Text>\n    </Button>\n    <Button phx-click=\"redirect\"><Text>Redirect to counter</Text></Button>\n    <IconButton phx-click=\"inc\" colors=\"{'containerColor': '#FFFF0000', 'contentColor': '#FFFFFFFF'}\">\n      <Icon imageVector=\"filled:Add\"></Icon>\n    </IconButton>\n    <Row verticalAlignment=\"center\">\n      <Button phx-click=\"dec\" shape=\"circle\" size=\"60\">\n        <Text>-</Text>\n      </Button>\n      <Text>This counter: ","</Text>\n      <Button phx-click=\"inc\" shape=\"circle\" size=\"60\"><Text>+</Text></Button>\n    </Row>\n  </Column>\n</Scaffold>"]}"#;
    let root: RootDiff = serde_json::from_str(initial_json).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let increment_diff = r#"{"0": "1"}"#;
    let other_root : RootDiff = serde_json::from_str(increment_diff).expect("Failed to deserialize diff fragment");
    let new_root = root.merge(other_root).expect("Failed to merge new root in");
    let expected_root = r#"{
        "0":"1",
        "s":["<Scaffold>\n  <TopAppBar>\n    <Title><Text>Hello</Text></Title>\n  </TopAppBar>\n  <Column width=\"fill\" verticalArrangement=\"center\" horizontalAlignment=\"center\">\n    <Text style=\"headlineLarge\">Title</Text>\n    <Card shape=\"8\" padding=\"16\" width=\"140\" height=\"120\" elevation=\"{'defaultElevation': '10', 'pressedElevation': '2'}\" phx-click=\"dec\">\n      <Text padding=\"16\">Hello Jetpack!</Text>\n    </Card>\n    <Spacer height=\"8\"></Spacer>\n    <Card padding=\"16\">\n      <Text padding=\"16\">Simple card</Text>\n    </Card>\n    <Button phx-click=\"navigate\" contentPadding=\"50\" elevation=\"{'defaultElevation': '20', 'pressedElevation': '10'}\">\n      <Text>Navigate to counter</Text>\n    </Button>\n    <Button phx-click=\"redirect\"><Text>Redirect to counter</Text></Button>\n    <IconButton phx-click=\"inc\" colors=\"{'containerColor': '#FFFF0000', 'contentColor': '#FFFFFFFF'}\">\n      <Icon imageVector=\"filled:Add\"></Icon>\n    </IconButton>\n    <Row verticalAlignment=\"center\">\n      <Button phx-click=\"dec\" shape=\"circle\" size=\"60\">\n        <Text>-</Text>\n      </Button>\n      <Text>This counter: ","</Text>\n      <Button phx-click=\"inc\" shape=\"circle\" size=\"60\"><Text>+</Text></Button>\n    </Row>\n  </Column>\n</Scaffold>"]}"#;
    let expected_root: RootDiff = serde_json::from_str(expected_root).expect("Failed to deserialize fragment");
    let expected_root: Root = expected_root.try_into().expect("Failed to convert RootDiff to Root");
    assert_eq!(expected_root, new_root);

    let _out : String = new_root.try_into().expect("Failed to convert root to string");

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
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let out: String = root.try_into().expect("Failed to convert Root into string");
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
        statics: None,
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
    let expected = FragmentDiff::UpdateRegular {
        children: HashMap::from([
            ("0".into(), ChildDiff::String("foo".into())),
            ("1".into(), ChildDiff::String("bar".into())),
        ]),
        statics: Some(Statics::Statics(vec!["a".into(), "b".into()])),
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
    assert!(out.is_ok());
    let out = out.expect("Failed to deserialize");
    let expected = RootDiff {
        fragment: FragmentDiff::UpdateRegular {
            children: HashMap::from([(
                "0".into(),
                ChildDiff::Fragment(FragmentDiff::UpdateRegular {
                    children: HashMap::from([("0".into(), ChildDiff::ComponentID(1))]),
                    statics: None,
                }),
            )]),
            statics: None,
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
    assert!(out.is_ok());
    let out = out.expect("Failed to deserialize");
    let expected = RootDiff {
        fragment: FragmentDiff::UpdateRegular {
            children: HashMap::from([(
                "0".into(),
                ChildDiff::Fragment(FragmentDiff::UpdateRegular {
                    children: HashMap::from([("0".into(), ChildDiff::ComponentID(1))]),
                    statics: None,
                }),
            )]),
            statics: None,
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
