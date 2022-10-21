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
    <body class="new-value" class="main" data-foo="old">
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
    <body class="new-value" class="main" data-foo="new">
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
