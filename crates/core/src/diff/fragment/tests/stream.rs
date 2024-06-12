use pretty_assertions::assert_eq;

use super::*;

#[test]
fn recorded_stream_test() {
    let initial = include_str!("flow-1-change-0.json");
    let root: RootDiff = serde_json::from_str(initial).expect("Failed to deserialize fragment");
    let root: Root = root.try_into().expect("Failed to convert RootDiff to Root");
    let out: String = root
        .clone()
        .try_into()
        .expect("Failed to convert Root into string");
    assert_eq!(out, include_str!("flow-1-change-0.html"));

    let diff: RootDiff = serde_json::from_str(include_str!("flow-1-change-1.json"))
        .expect("Failed to deserialize fragment");

    let root = root.merge(diff).expect("Failed to merge diff");

    let out: String = root
        .clone()
        .try_into()
        .expect("Failed to convert Root into string");
    assert_eq!(format!("{out}\n"), include_str!("flow-1-change-1.html"));

    let diff: RootDiff = serde_json::from_str(include_str!("flow-1-change-2.json"))
        .expect("Failed to deserialize fragment");

    let root = root.merge(diff).expect("Failed to merge diff");

    let out: String = root
        .clone()
        .try_into()
        .expect("Failed to convert Root into string");
    assert_eq!(format!("{out}\n"), include_str!("flow-1-change-2.html"));

    let diff: RootDiff = serde_json::from_str(include_str!("flow-1-change-3.json"))
        .expect("Failed to deserialize fragment");

    let root = root.merge(diff).expect("Failed to merge diff");

    let out: String = root
        .clone()
        .try_into()
        .expect("Failed to convert Root into string");
    assert_eq!(format!("{out}\n"), include_str!("flow-1-change-3.html"));
}
