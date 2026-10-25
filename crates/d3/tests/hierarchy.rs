//! The port of d3-hierarchy, checked against d3's own JavaScript.
use d3::hierarchy::{hierarchy, HierarchyData};
use serde_json::Value;
use std::fs;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/hierarchy.json");
    let text = fs::read_to_string(path).expect("fixture");
    serde_json::from_str(&text).expect("fixture JSON")
}

#[derive(Clone, Debug, PartialEq)]
struct Data {
    name: String,
    value: f64,
    children: Option<Vec<Data>>,
}

impl HierarchyData for Data {
    fn children(&self) -> Option<Vec<Self>> {
        self.children.clone()
    }
    fn value(&self) -> Option<f64> {
        Some(self.value)
    }
}

#[test]
fn the_hierarchy_is_the_javascripts() {
    let f = fixture();
    let data = Data {
        name: "root".to_owned(),
        value: 0.0,
        children: Some(vec![
            Data {
                name: "a".to_owned(),
                value: 1.0,
                children: Some(vec![Data {
                    name: "a1".to_owned(),
                    value: 1.0,
                    children: None,
                }]),
            },
            Data {
                name: "b".to_owned(),
                value: 2.0,
                children: Some(vec![Data {
                    name: "b1".to_owned(),
                    value: 2.0,
                    children: None,
                }]),
            },
        ]),
    };
    let root = hierarchy(data);
    assert_eq!(
        d3::hierarchy::HierarchyNode::<Data>::descendants(&root).len(),
        f["descendantsCount"].as_u64().unwrap() as usize
    );
    assert_eq!(
        d3::hierarchy::HierarchyNode::<Data>::leaves(&root).len(),
        f["leavesCount"].as_u64().unwrap() as usize
    );
}
