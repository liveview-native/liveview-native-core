use liveview_native_core::{
    diff::{self, Patch},
    dom::{ParseError, *},
};
use paste::paste;
use text_diff::print_diff;
use thiserror::Error;

#[derive(Error, Debug)]
enum Error {
    #[error("Document transformed incorrectly")]
    IncorrectTransformation,
    #[error(transparent)]
    ParseError(#[from] ParseError),
}

fn check_transformation(from: &str, to: &str) -> Result<(), Error> {
    let mut prev = Document::parse(from)?;
    let next = Document::parse(to)?;

    let mut patches = diff::diff(&prev, &next);

    let mut editor = prev.edit();
    let mut stack = vec![];
    for patch in patches.drain(..) {
        patch.apply(&mut editor, &mut stack);
    }

    editor.finish();

    let prev = prev.to_string();
    let next = next.to_string();

    if prev.ne(&next) {
        print_diff(prev.as_str(), next.as_str(), "\n");
        return Err(Error::IncorrectTransformation);
    }

    Ok(())
}

fn check_diff(from: &str, to: &str, patches: &[Patch]) -> Result<(), Error> {
    let mut prev = Document::parse(from)?;
    let next = Document::parse(to)?;

    let diff = diff::diff(&prev, &next);

    let mut editor = prev.edit();
    let mut stack = vec![];

    for patch in patches.iter().cloned() {
        patch.apply(&mut editor, &mut stack);
    }

    editor.finish();

    let prev = prev.to_string();

    let next = next.to_string();

    if prev.ne(&next) {
        print_diff(prev.as_str(), next.as_str(), "\n");
        return Err(Error::IncorrectTransformation);
    }

    assert_eq!(diff, Vec::from_iter(patches.iter().cloned()));

    Ok(())
}

macro_rules! test_fixture {
    ($name:literal) => {
        paste! {
            #[test]
            fn [<diff_ $name:snake>]() -> Result<(), Error> {
                check_transformation(
                    include_str!(concat!("fixtures/", $name, "/from.html")),
                    include_str!(concat!("fixtures/", $name, "/to.html"))
                )
            }
        }
    };
}

#[test]
fn diff_patch_empty_diff_test() -> Result<(), Error> {
    check_diff(
        "<html lang=\"en\"><head><meta charset=\"utf-8\"/></head><body><a href=\"about:blank\">Hello World!</a></body></html>",
        "<html lang=\"en\"><head><meta charset=\"utf-8\"/></head><body><a href=\"about:blank\">Hello World!</a></body></html>",
        &[]
    )
}

#[test]
fn diff_patch_combined_test() -> Result<(), Error> {
    check_transformation(
        "<html lang=\"en\"><head><meta charset=\"utf-8\"/></head><body><a href=\"about:blank\">Hello World!</a></body></html>",
        "<html lang=\"en\"><head><meta title=\"Hello\"/></head><body><h1>Greetings!</h1><a href=\"https://www.example.com\">Hi World!</a></body></html>"
    )
}

#[test]
fn diff_patch_new_child_test() -> Result<(), Error> {
    check_transformation(
        "<html lang=\"en\"><head><meta charset=\"utf-8\"/></head><body><a href=\"about:blank\">Hello World!</a></body></html>",
        "<html lang=\"en\"><head><meta charset=\"utf-8\"/></head><body><h1>Greetings!</h1><a href=\"about:blank\">Hello World!</a></body></html>"
    )
}

#[test]
fn diff_patch_remove_child_test() -> Result<(), Error> {
    check_transformation(
        "<html lang=\"en\"><head><meta charset=\"utf-8\"/></head><body><h1>Greetings!</h1><a href=\"about:blank\">Hello World!</a></body></html>",
        "<html lang=\"en\"><head><meta charset=\"utf-8\"/></head><body><a href=\"about:blank\">Hello World!</a></body></html>"
    )
}

#[test]
fn dom_swift_integration_test() -> Result<(), Error> {
    check_transformation(
        r#"
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
            </head>
            <body class="main">
                some content
            </body>
        </html>
        "#,
        r#"
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="title" content="Hello World"/>
            </head>
            <body class="main">
                new content
            </body>
        </html>
        "#,
    )
}

#[test]
fn issue3_regression_test() -> Result<(), Error> {
    check_transformation(
        r#"
<vstack nav-title="Cottonwood 4-5" roster-link="/room/16/roster">
    <messages-list>
        <vstack>
            <vstack id="102" alignment="leading" pad-leading="8" pad-trailing="8" pad-top="4" pad-bottom="4">
                <zstack>
                    <hstack>
                        <text font="body" font-weight="bold">
                            Test Test
                        </text>
                        <spacer/>
                    </hstack>
                    <hstack>
                        <spacer/>
                        <local-time datetime="2022-10-26T17:13:08Z"/>
                    </hstack>
                </zstack>
                <link-text frame-alignment="leading" fix-multiline-text="">
                    A
                </link-text>
            </vstack>
        </vstack>
    </messages-list>
    <rectangle frame-height="1" fill-color="\#cccccc"/>
    <hstack pad-leading="8" pad-trailing="8">
        <phx-form id="post" phx-submit="send" submit-on-enter="">
            <hstack>
                <textfield name="text" border-style="none" clear-button="while-editing" placeholder="Message" return-key-type="send"/>
                <phx-submit-button after-submit="clear">
                    <text>
                        Send
                    </text>
                </phx-submit-button>
            </hstack>
        </phx-form>
    </hstack>
</vstack>
"#,
        r#"
<vstack nav-title="Cottonwood 4-5" roster-link="/room/16/roster">
    <messages-list>
        <vstack>
            <vstack id="102" alignment="leading" pad-leading="8" pad-trailing="8" pad-top="4" pad-bottom="4">
                <zstack>
                    <hstack>
                        <text font="body" font-weight="bold">
                            Test Test
                        </text>
                        <spacer/>
                    </hstack>
                    <hstack>
                        <spacer/>
                        <local-time datetime="2022-10-26T17:13:08Z"/>
                    </hstack>
                </zstack>
                <link-text frame-alignment="leading" fix-multiline-text="">
                    A
                </link-text>
            </vstack>
            <vstack id="103" alignment="leading" pad-leading="8" pad-trailing="8" pad-top="4" pad-bottom="4">
                <zstack>
                    <hstack>
                        <text font="body" font-weight="bold">
                            Test Test
                        </text>
                        <spacer/>
                    </hstack>
                    <hstack>
                        <spacer/>
                        <local-time datetime="2022-10-26T17:13:17Z"/>
                    </hstack>
                </zstack>
                <link-text frame-alignment="leading" fix-multiline-text="">
                    B
                </link-text>
            </vstack>
        </vstack>
    </messages-list>
    <rectangle frame-height="1" fill-color="\#cccccc"/>
    <hstack pad-leading="8" pad-trailing="8">
        <phx-form id="post" phx-submit="send" submit-on-enter="">
            <hstack>
                <textfield name="text" border-style="none" clear-button="while-editing" placeholder="Message" return-key-type="send"/>
                <phx-submit-button after-submit="clear">
                    <text>
                        Send
                    </text>
                </phx-submit-button>
            </hstack>
        </phx-form>
    </hstack>
</vstack>
"#,
    )
}

#[test]
fn diff_add_child_oob() -> Result<(), Error> {
    check_transformation("<a></a>", "<a><b></b></a>")
}

#[test]
fn diff_remove_node() -> Result<(), Error> {
    check_diff(
        "<a/><b/>",
        "<b/>",
        &[Patch::Remove {
            node: NodeRef::from_u32(1),
        }],
    )
}

#[test]
fn diff_live_form() -> Result<(), Error> {
    check_transformation(
        r#"<VStack modifiers="">
        <VStack>
          <LiveForm id="login" phx-submit="login">
            <TextField name="email" modifiers="">
              Email
            </TextField>
            <LiveSubmitButton modifiers="">
              <Text>Enter</Text>
            </LiveSubmitButton>
          </LiveForm>
        </VStack>
    </VStack>"#,
        r#"<VStack modifiers="">
        <VStack>
          <Text>Success! Check your email for magic link</Text>
        </VStack>
    </VStack>"#,
    )
}

test_fixture!("attr-value-empty-string");
test_fixture!("change-tagname");
test_fixture!("change-tagname-ids");
test_fixture!("change-types");
test_fixture!("data-table");
test_fixture!("data-table2");
test_fixture!("equal");
// test_fixture!("id-change-tag-name");
test_fixture!("ids-nested");
test_fixture!("ids-nested-2");
test_fixture!("ids-nested-3");
test_fixture!("ids-nested-4");
test_fixture!("ids-nested-5");
test_fixture!("ids-nested-6");
test_fixture!("ids-nested-7");
test_fixture!("ids-prepend");
test_fixture!("input-element");
test_fixture!("input-element-disabled");
test_fixture!("input-element-enabled");
test_fixture!("large");
test_fixture!("lengthen");
test_fixture!("one");
test_fixture!("reverse");
test_fixture!("reverse-ids");
test_fixture!("select-element");
test_fixture!("select-element-default");
test_fixture!("shorten");
test_fixture!("simple");
test_fixture!("simple-ids");
test_fixture!("simple-text-el");
test_fixture!("svg");
test_fixture!("svg-append");
test_fixture!("svg-append-new");
test_fixture!("svg-no-default-namespace");
test_fixture!("svg-xlink");
test_fixture!("tag-to-text");
test_fixture!("text-to-tag");
test_fixture!("text-to-text");
// TODO: https://github.com/liveview-native/liveview-native-core/issues/58
//test_fixture!("textarea");
test_fixture!("todomvc");
test_fixture!("todomvc2");
test_fixture!("two");
