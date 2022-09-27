#![feature(assert_matches)]

use std::assert_matches::assert_matches;

use liveview_native_core::dom::AttributeValue;
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
