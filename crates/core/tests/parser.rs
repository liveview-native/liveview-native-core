use liveview_native_core::{
    dom::{AttributeName, NodeData},
    parser, InternedString,
};

#[test]
fn parser_simple() {
    let result = parser::parse("<html lang=\"en\"><head><meta charset=\"utf-8\"/><meta name=\"title\" content=\"Test\"/></head><body><a href=\"about:blank\">Hello World!</a></body></html>");
    assert!(result.is_ok());
    let document = result.unwrap();
    let root = document.root();
    let html = document.children(root)[0];
    let attrs = document
        .attributes(html)
        .iter()
        .map(|a| (a.name.clone(), a.value.clone()))
        .collect::<Vec<_>>();
    let lang: AttributeName = "lang".into();
    let en = Some("en".to_string());
    assert_eq!(attrs, vec![(lang, en)]);
}

#[test]
fn parser_whitespace_handling() {
    let result = parser::parse(
        r#"
<html lang="en">
    <head>
        <meta charset="utf-8"/>
    </head>
    <body class="new-value" class="main">
        some content
    </body>
</html>
"#,
    );
    assert!(result.is_ok());
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
    assert!(matches!(content, NodeData::Leaf { .. }));
    let NodeData::Leaf { value: content } = content else {
        unreachable!()
    };
    assert_eq!(content.as_str(), "some content");
}

#[test]
fn parser_preserve_upcase() {
    let result = parser::parse("<Component id=5><SubComponent id=7><a href=\"about:blank\">Hello World!</a></SubComponent></Component>");
    assert!(result.is_ok());
    let document = result.unwrap();
    let root = document.root();
    let component = document.children(root)[0];
    let element = document.get(component);
    let NodeData::NodeElement { element } = element else {
        panic!("expected element");
    };
    let expected_name: InternedString = "Component".into();
    assert_eq!(element.name, expected_name);
}
