#![feature(assert_matches)]

use std::assert_matches::assert_matches;

use liveview_native_core::diff::{self, Patch};
use liveview_native_core::dom::*;

const EMPTY: &'static [Patch] = &[];

#[test]
fn diff_patch_empty_diff_test() {
    let result = Document::parse("<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><a href=\"about:blank\">Hello World!</a></body></html>");
    assert_matches!(result, Ok(_));
    let prev = result.unwrap();

    let result = Document::parse("<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><a href=\"about:blank\">Hello World!</a></body></html>");
    assert_matches!(result, Ok(_));
    let next = result.unwrap();

    let patches = diff::diff(&prev, &next);
    assert_eq!(patches.as_slices(), (EMPTY, EMPTY));
}

#[test]
fn diff_patch_combined_test() {
    let result = Document::parse("<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><a href=\"about:blank\">Hello World!</a></body></html>");
    assert_matches!(result, Ok(_));
    let mut prev = result.unwrap();

    let result = Document::parse("<html lang=\"en\"><head><meta title=\"Hello\" /></head><body><h1>Greetings!</h1><a href=\"https://www.example.com\">Hi World!</a></body></html>");
    assert_matches!(result, Ok(_));
    let next = result.unwrap();

    let mut patches = diff::diff(&prev, &next);

    let mut editor = prev.edit();
    let mut stack = vec![];
    for patch in patches.drain(..) {
        patch.apply(&mut editor, &mut stack);
    }

    editor.finish();

    assert_eq!(prev.to_string(), next.to_string());
}

#[test]
fn diff_patch_new_child_test() {
    let result = Document::parse("<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><a href=\"about:blank\">Hello World!</a></body></html>");
    assert_matches!(result, Ok(_));
    let mut prev = result.unwrap();

    let result = Document::parse("<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><h1>Greetings!</h1><a href=\"about:blank\">Hello World!</a></body></html>");
    assert_matches!(result, Ok(_));
    let next = result.unwrap();

    let mut patches = diff::diff(&prev, &next);

    let mut editor = prev.edit();
    let mut stack = vec![];
    for patch in patches.drain(..) {
        patch.apply(&mut editor, &mut stack);
    }

    editor.finish();

    assert_eq!(prev.to_string(), next.to_string());
}

#[test]
fn diff_patch_remove_child_test() {
    let result = Document::parse("<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><h1>Greetings!</h1><a href=\"about:blank\">Hello World!</a></body></html>");
    assert_matches!(result, Ok(_));
    let mut prev = result.unwrap();

    let result = Document::parse("<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><a href=\"about:blank\">Hello World!</a></body></html>");
    assert_matches!(result, Ok(_));
    let next = result.unwrap();

    let mut patches = diff::diff(&prev, &next);

    let mut editor = prev.edit();
    let mut stack = vec![];
    for patch in patches.drain(..) {
        patch.apply(&mut editor, &mut stack);
    }

    editor.finish();

    assert_eq!(prev.to_string(), next.to_string());
}

#[test]
fn dom_swift_integration_test() {
    let mut prev = Document::parse(
        r#"
<html lang="en">
    <head>
        <meta charset="utf-8" />
    </head>
    <body class="main">
        some content
    </body>
</html>
"#,
    )
    .unwrap();

    let next = Document::parse(
        r#"
<html lang="en">
    <head>
        <meta charset="utf-8" />
        <meta name="title" content="Hello World" />
    </head>
    <body class="main">
        new content
    </body>
</html>
"#,
    )
    .unwrap();

    let mut patches = diff::diff(&prev, &next);

    dbg!(&patches);

    let mut editor = prev.edit();
    let mut stack = vec![];
    for patch in patches.drain(..) {
        patch.apply(&mut editor, &mut stack);
    }

    editor.finish();

    assert_eq!(prev.to_string(), next.to_string());
}

#[test]
fn issue3_regression_test() {
    let mut prev = Document::parse(r#"
<vstack nav-title="Cottonwood 4-5" roster-link="/room/16/roster">
    <messages-list>
        <vstack>
            <vstack id="102" alignment="leading" pad-leading="8" pad-trailing="8" pad-top="4" pad-bottom="4">
                <zstack>
                    <hstack>
                        <text font="body" font-weight="bold">
                            Test Test
                        </text>
                        <spacer />
                    </hstack>
                    <hstack>
                        <spacer />
                        <local-time datetime="2022-10-26T17:13:08Z" />
                    </hstack>
                </zstack>
                <link-text frame-alignment="leading" fix-multiline-text="">
                    A
                </link-text>
            </vstack>
        </vstack>
    </messages-list>
    <rectangle frame-height="1" fill-color="\#cccccc" />
    <hstack pad-leading="8" pad-trailing="8">
        <phx-form id="post" phx-submit="send" submit-on-enter="">
            <hstack>
                <textfield name="text" border-style="none" clear-button="while-editing" placeholder="Message" return-key-type="send" />
                <phx-submit-button after-submit="clear">
                    <text>
                        Send
                    </text>
                </phx-submit-button>
            </hstack>
        </phx-form>
    </hstack>
</vstack>
"#).unwrap();

    let next = Document::parse(r#"
<vstack nav-title="Cottonwood 4-5" roster-link="/room/16/roster">
    <messages-list>
        <vstack>
            <vstack id="102" alignment="leading" pad-leading="8" pad-trailing="8" pad-top="4" pad-bottom="4">
                <zstack>
                    <hstack>
                        <text font="body" font-weight="bold">
                            Test Test
                        </text>
                        <spacer />
                    </hstack>
                    <hstack>
                        <spacer />
                        <local-time datetime="2022-10-26T17:13:08Z" />
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
                        <spacer />
                    </hstack>
                    <hstack>
                        <spacer />
                        <local-time datetime="2022-10-26T17:13:17Z" />
                    </hstack>
                </zstack>
                <link-text frame-alignment="leading" fix-multiline-text="">
                    B
                </link-text>
            </vstack>
        </vstack>
    </messages-list>
    <rectangle frame-height="1" fill-color="\#cccccc" />
    <hstack pad-leading="8" pad-trailing="8">
        <phx-form id="post" phx-submit="send" submit-on-enter="">
            <hstack>
                <textfield name="text" border-style="none" clear-button="while-editing" placeholder="Message" return-key-type="send" />
                <phx-submit-button after-submit="clear">
                    <text>
                        Send
                    </text>
                </phx-submit-button>
            </hstack>
        </phx-form>
    </hstack>
</vstack>
"#).unwrap();

    let mut patches = diff::diff(&prev, &next);

    dbg!(&patches);

    let mut editor = prev.edit();
    let mut stack = vec![];
    for patch in patches.drain(..) {
        patch.apply(&mut editor, &mut stack);
    }

    editor.finish();

    assert_eq!(prev.to_string(), next.to_string());
}
