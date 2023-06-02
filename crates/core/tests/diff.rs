use std::collections::VecDeque;

use liveview_native_core::diff::{self, Patch};
use liveview_native_core::dom::*;
use liveview_native_core::parser::ParseError;

macro_rules! assert_transformation {
    ($from:expr, $to:expr) => {{
        let mut prev = Document::parse($from)?;
        let next = Document::parse($to)?;

        let mut patches = diff::diff(&prev, &next);

        let mut editor = prev.edit();
        let mut stack = vec![];
        for patch in patches.drain(..) {
            patch.apply(&mut editor, &mut stack);
        }

        editor.finish();

        assert_eq!(prev.to_string(), next.to_string());

        Ok(())
    }};
    ($from:expr, $to:expr, $patches:expr) => {{
        let mut prev = Document::parse($from)?;
        let next = Document::parse($to)?;

        let diff = diff::diff(&prev, &next);

        let mut patches: VecDeque<Patch> = VecDeque::from($patches);

        let mut editor = prev.edit();
        let mut stack = vec![];
        for patch in patches.drain(..) {
            patch.apply(&mut editor, &mut stack);
        }

        editor.finish();

        assert_eq!(prev.to_string(), next.to_string());

        assert_eq!(diff, VecDeque::from($patches));

        Ok(())
    }};
}

#[test]
fn diff_patch_empty_diff_test() -> Result<(), ParseError> {
    assert_transformation!(
        "<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><a href=\"about:blank\">Hello World!</a></body></html>",
        "<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><a href=\"about:blank\">Hello World!</a></body></html>",
        []
    )
}

#[test]
fn diff_patch_combined_test() -> Result<(), ParseError> {
    assert_transformation!(
        "<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><a href=\"about:blank\">Hello World!</a></body></html>",
        "<html lang=\"en\"><head><meta title=\"Hello\" /></head><body><h1>Greetings!</h1><a href=\"https://www.example.com\">Hi World!</a></body></html>"
    )
}

#[test]
fn diff_patch_new_child_test() -> Result<(), ParseError> {
    assert_transformation!(
        "<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><a href=\"about:blank\">Hello World!</a></body></html>",
        "<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><h1>Greetings!</h1><a href=\"about:blank\">Hello World!</a></body></html>"
    )
}

#[test]
fn diff_patch_remove_child_test() -> Result<(), ParseError> {
    assert_transformation!(
        "<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><h1>Greetings!</h1><a href=\"about:blank\">Hello World!</a></body></html>",
        "<html lang=\"en\"><head><meta charset=\"utf-8\" /></head><body><a href=\"about:blank\">Hello World!</a></body></html>"
    )
}

#[test]
fn dom_swift_integration_test() -> Result<(), ParseError> {
    assert_transformation!(
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
        "#
    )
}

#[test]
fn issue3_regression_test() -> Result<(), ParseError> {
    assert_transformation!(
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
"#
    )
}

#[test]
fn diff_add_child_oob() -> Result<(), ParseError> {
    assert_transformation!("<a></a>", "<a><b></b></a>")
}
