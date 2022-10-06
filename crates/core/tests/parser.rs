#![feature(assert_matches)]

use std::assert_matches::assert_matches;

use liveview_native_core::dom::{AttributeValue, Node};
use liveview_native_core::parser;
use liveview_native_core::InternedString;

#[test]
fn parser_simple() {
    let result = parser::parse("<html lang=\"en\"><head><meta charset=\"utf-8\" /><meta name=\"title\" content=\"Test\" /></head><body><a href=\"about:blank\">Hello World!</a></body></html>");
    assert_matches!(result, Ok(_));
    let document = result.unwrap();
    let root = document.root();
    let html = document.children(root)[0];
    let attrs = document
        .get_attributes(html)
        .map(|a| (a.name, a.value.clone()))
        .collect::<Vec<_>>();
    let lang: InternedString = "lang".into();
    let en: AttributeValue = "en".into();
    assert_eq!(attrs, vec![(lang, en)]);
}

#[test]
fn parser_whitespace_handling() {
    let result = parser::parse(
        r#"
<html lang="en">
    <head>
        <meta charset="utf-8" />
    </head>
    <body class="new-value" class="main">
        some content
    </body>
</html>
"#,
    );
    assert_matches!(result, Ok(_));
    let document = result.unwrap();
    let root = document.root();
    let children = document.children(root);
    assert_eq!(children.len(), 1);
    let html = children[0];
    let children = document.children(html);
    assert_eq!(children.len(), 2);
    let body = children[1];
    let children = document.children(body);
    assert_eq!(children.len(), 1);
    let content = document.get(children[0]);
    assert_matches!(content, Node::Leaf(_));
    let Node::Leaf(content) = content else { unreachable!() };
    assert_eq!(content.as_str(), "some content");
}
