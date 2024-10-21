#![feature(assert_matches)]

use liveview_native_core::dom::*;

#[test]
fn dom_builder_example() {
    // Start building a new document
    let mut builder = Document::build();

    // Start creating a new document rooted at the given node
    let html = builder.append(NodeData::new("html"));
    builder.set_insertion_point(html);

    // Apply an attribute to the html node
    builder.set_attribute("lang", "en".to_string());

    // Insert a new node and move the insertion point to that node
    let head = builder.append(NodeData::new("head"));
    builder.set_insertion_point(head);

    // Use of insertion guards
    {
        let mut guard = builder.insert_guard();
        let meta = guard.append(NodeData::new("meta"));
        guard.set_insertion_point(meta);
        guard.set_attribute("charset", "utf-8".to_string());
    }

    assert_eq!(builder.insertion_point(), head);

    // Insert a node after another node, regardless of where the builder is currently positioned
    let body = builder.insert_after(NodeData::new("body"), head);
    builder.set_insertion_point(body);
    builder.set_attribute("class", "main".to_string());

    // Move the insertion point back up a level (to "html" in this example)
    builder.set_insertion_point_to_parent();
    assert_eq!(builder.insertion_point(), html);

    // Update an attribute of a node
    builder.set_insertion_point(body);
    builder.set_attribute("class", "new-value".to_string());

    // Append a leaf node to the body
    builder.append("some content");

    // Get the final document
    let doc = builder.finish();

    // Print
    let mut buffer = String::with_capacity(4 * 1024);
    let expected = r#"<html lang="en">
    <head>
        <meta charset="utf-8" />
    </head>
    <body class="new-value">
        some content
    </body>
</html>"#;
    assert!(doc.print(&mut buffer, PrintOptions::Pretty).is_ok());
    assert_eq!(buffer.as_str(), expected);
}

#[test]
fn iframe_closing_tag_roundtrip() {
    let orig_body = r#"<arb>
    <iframe>
    </iframe>
</arb>"#;

    let min_body = r#"<arb>
    <iframe/ >
</arb>"#;

    let doc = Document::parse(orig_body).expect("invalid document");
    pretty_assertions::assert_eq!(min_body, doc.to_string());
}

/*
 * TODO: https://github.com/liveview-native/liveview-native-core/issues/58
#[test]
fn dom_selection() {
    let document = Document::parse(
        r#"<html lang="en">
<head>
  <meta charset="utf-8"/>
  <link rel="stylesheet" href="foo.css"/>
</head>
<body class="main">
  <section id="foo">
    <p>First</p>
  </section>
  <section id="bar">
    <p class="class1 class2">Second</p>
  </section>
</body>
</html>"#,
    )
    .unwrap();
    // 1. Select all (but the root node)
    assert_eq!(document.select(Selector::All).count(), 9);

    // 2. Select by tag
    assert_eq!(document.select(Selector::Tag("section".into())).count(), 2);
    // 3. Select by id
    //assert_eq!(document.select(Selector::Id("foo".into()).count()), 1);
    // 4. Select both
    let is_meta = Box::new(Selector::Tag("meta".into()));
    let has_charset = Box::new(Selector::Attribute("charset".into()));
    assert_eq!(
        document
            .select(Selector::And(is_meta.clone(), has_charset))
            .count(),
        1
    );
    // 5. Select either
    let is_link = Box::new(Selector::Tag("link".into()));
    assert_eq!(document.select(Selector::Or(is_meta, is_link)).count(), 2);
    // 6. Select descendant
    let is_html = Box::new(Selector::Tag("html".into()));
    let is_section = Box::new(Selector::Tag("section".into()));
    assert_eq!(
        document
            .select(Selector::Descendant(is_html, is_section.clone()))
            .count(),
        2
    );
    // 7. Select child
    let is_body = Box::new(Selector::Tag("body".into()));
    assert_eq!(
        document
            .select(Selector::Child(is_body, is_section.clone()))
            .count(),
        2
    );
    // 8. Select by attribute name
    assert_eq!(
        document
            .select(Selector::Attribute("charset".into()))
            .count(),
        1
    );
    // 9. Select by attribute name and value
    assert_eq!(
        document
            .select(Selector::AttributeValue("charset".into(), "utf-8".into()))
            .count(),
        1
    );
    assert_eq!(
        document
            .select(Selector::AttributeValue("charset".into(), "nope".into()))
            .count(),
        0
    );
    // 10. Select by attribute name and value content (whitespace-separated values)
    assert_eq!(
        document
            .select(Selector::AttributeValueWhitespacedContains(
                "class".into(),
                "class2"
            ))
            .count(),
        1
    );
    // 11. Select by attribute name and value content (prefix)
    assert_eq!(
        document
            .select(Selector::AttributeValueStartsWith("class".into(), "ma"))
            .count(),
        1
    );
    // 11. Select by attribute name and value content (suffix)
    assert_eq!(
        document
            .select(Selector::AttributeValueEndsWith("class".into(), "in"))
            .count(),
        1
    );
    // 12. Select by attribute name and value content (contains)
    assert_eq!(
        document
            .select(Selector::AttributeValueSubstring("class".into(), "ai"))
            .count(),
        1
    );
}
*/
