//! The port of d3-collection, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-collection 1.0.7
//! under Node — the same code the browser runs — and writing down what it
//! produced.

use std::fs;

use d3::collection::nest::{Entry, Nest, Node};
use d3::collection::{map, set};
use serde_json::Value as Json;

fn fixture() -> Json {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/collection.json"
    );
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The strings of a list.
fn strings(value: &Json) -> Vec<String> {
    value
        .as_array()
        .expect("a list of strings")
        .iter()
        .map(|value| value.as_str().expect("a string").to_owned())
        .collect()
}

/// The numbers of a list.
fn numbers(value: &Json) -> Vec<f64> {
    value
        .as_array()
        .expect("a list of numbers")
        .iter()
        .map(|value| value.as_f64().expect("a number"))
        .collect()
}

#[test]
fn the_map_is_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("map").expect("the maps");
    let table = table.as_object().expect("the maps");

    let from_array = map(&[1.0, 2.0, 3.0], None::<fn(&f64, usize) -> String>);
    let expected = table.get("array").expect("the map of an array");
    assert_eq!(from_array.keys(), strings(&expected["keys"]));
    assert_eq!(
        from_array
            .values()
            .into_iter()
            .copied()
            .collect::<Vec<f64>>(),
        numbers(&expected["values"])
    );
    assert_eq!(
        from_array.size(),
        expected["size"].as_u64().unwrap() as usize
    );
    assert_eq!(from_array.empty(), expected["empty"].as_bool().unwrap());
    assert_eq!(from_array.has("0"), expected["has"][0].as_bool().unwrap());
    assert_eq!(from_array.has("9"), expected["has"][1].as_bool().unwrap());

    let by_key = map(
        &[1.0, 2.0, 3.0],
        Some(|_value: &f64, index: usize| format!("k{}", index)),
    );
    let expected = table.get("byKey").expect("the map by a key");
    assert_eq!(by_key.keys(), strings(&expected["keys"]));
    assert_eq!(
        by_key.values().into_iter().copied().collect::<Vec<f64>>(),
        numbers(&expected["values"])
    );
    let entries = by_key.entries();
    let expected = expected["entries"].as_array().expect("the entries");
    assert_eq!(entries.len(), expected.len());
    for (entry, expected) in entries.iter().zip(expected) {
        let expected = expected.as_array().expect("an entry");
        assert_eq!(entry.0, expected[0].as_str().unwrap());
        assert_eq!(*entry.1, expected[1].as_f64().unwrap());
    }

    // The map that is changed and emptied.
    let mut changed = map(&[1.0, 2.0, 3.0], None::<fn(&f64, usize) -> String>);
    changed.set("0", 10.0);
    let expected = table.get("set").expect("the changed map");
    assert_eq!(changed.keys(), strings(&expected["keys"]));
    assert_eq!(
        changed.values().into_iter().copied().collect::<Vec<f64>>(),
        numbers(&expected["values"])
    );
    changed.remove("0");
    let expected = table.get("removed").expect("the map without a key");
    assert_eq!(changed.keys(), strings(&expected["keys"]));
    changed.clear();
    let expected = table.get("cleared").expect("the emptied map");
    assert_eq!(changed.size(), expected["size"].as_u64().unwrap() as usize);
    assert_eq!(changed.empty(), expected["empty"].as_bool().unwrap());
}

#[test]
fn the_set_is_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("set").expect("the sets");
    let table = table.as_object().expect("the sets");

    let mut made = set(
        &["a".to_owned(), "b".to_owned(), "a".to_owned()],
        None::<fn(&String, usize) -> String>,
    );
    let expected = table.get("values").expect("the values");
    assert_eq!(made.values(), strings(expected));
    assert_eq!(made.size(), table["size"].as_u64().unwrap() as usize);
    assert_eq!(made.has("a"), table["has"][0].as_bool().unwrap());
    assert_eq!(made.has("z"), table["has"][1].as_bool().unwrap());

    let by_key = set(
        &[1.0, 2.0, 3.0],
        Some(|value: &f64, _| format!("k{}", value)),
    );
    let expected = table.get("byKey").expect("the set by a key");
    assert_eq!(by_key.values(), strings(expected));

    made.remove("a");
    let expected = table.get("removed").expect("the set without a value");
    assert_eq!(made.values(), strings(expected));
    made.clear();
    let expected = table.get("cleared").expect("the emptied set");
    assert_eq!(made.size(), expected["size"].as_u64().unwrap() as usize);
    assert_eq!(made.empty(), expected["empty"].as_bool().unwrap());
}

/// A row of the nest, which the JavaScript's own has.
#[derive(Clone, Debug, PartialEq)]
struct Row {
    year: i32,
    month: &'static str,
    value: i32,
}

/// The key of a row by its year.
fn year(value: &Row) -> String {
    value.year.to_string()
}

/// The key of a row by its month.
fn month(value: &Row) -> String {
    value.month.to_owned()
}

#[test]
fn the_nest_is_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("nest").expect("the nests");
    let table = table.as_object().expect("the nests");

    let rows = vec![
        Row {
            year: 2020,
            month: "jan",
            value: 1,
        },
        Row {
            year: 2020,
            month: "feb",
            value: 2,
        },
        Row {
            year: 2021,
            month: "jan",
            value: 3,
        },
    ];
    let nest = Nest::new().key(year).key(month);

    // The map of the nest, which is the JavaScript's own.
    let map = nest.map(&rows);
    let expected = table.get("map").expect("the map of the nest");
    let expected: Vec<(String, Vec<(String, usize)>)> =
        serde_json::from_str(expected.as_str().unwrap()).expect("the map");
    let entries = map.entries();
    assert_eq!(entries.len(), expected.len());
    for (entry, expected) in entries.iter().zip(expected) {
        assert_eq!(entry.0, expected.0);
        let node = match entry.1 {
            Node::Nest(values) => values,
            Node::List(values) => {
                assert_eq!(values.len(), 0);
                continue;
            }
        };
        assert_eq!(node.len(), expected.1.len());
        for (value, expected) in node.iter().zip(expected.1.iter()) {
            assert_eq!(value.0, expected.0);
            match &value.1 {
                Node::List(values) => assert_eq!(values.len(), expected.1),
                Node::Nest(_) => panic!("the value is a nest"),
            }
        }
    }

    // The object of the nest, which is the JavaScript's own.
    let object = nest.object(&rows);
    let expected = table.get("object").expect("the object of the nest");
    let expected: Vec<(String, Vec<(String, usize)>)> =
        serde_json::from_str(expected.as_str().unwrap()).expect("the object");
    assert_eq!(object.len(), expected.len());
    for (entry, expected) in object.iter().zip(expected) {
        assert_eq!(entry.0, expected.0);
        let node = match &entry.1 {
            Node::Nest(values) => values,
            Node::List(values) => {
                assert_eq!(values.len(), 0);
                continue;
            }
        };
        assert_eq!(node.len(), expected.1.len());
        for (value, expected) in node.iter().zip(expected.1.iter()) {
            assert_eq!(value.0, expected.0);
            match &value.1 {
                Node::List(values) => assert_eq!(values.len(), expected.1),
                Node::Nest(_) => panic!("the value is a nest"),
            }
        }
    }

    // The entries of the nest, which are the JavaScript's own.
    let entries = nest.entries(&rows);
    let expected = table.get("entries").expect("the entries of the nest");
    let expected: Vec<(String, Vec<(String, usize)>)> =
        serde_json::from_str(expected.as_str().unwrap()).expect("the entries");
    assert_eq!(entries.len(), expected.len());
    for (entry, expected) in entries.iter().zip(expected) {
        assert_eq!(entry.key(), expected.0);
        let values = match entry {
            Entry::Level(_, values) => values,
            Entry::Leaf(_, leaves) => {
                assert_eq!(leaves.len(), expected.1.len());
                continue;
            }
        };
        assert_eq!(values.len(), expected.1.len());
        for (value, expected) in values.iter().zip(expected.1.iter()) {
            assert_eq!(value.key(), expected.0);
            match value {
                Entry::Leaf(_, leaves) => assert_eq!(leaves.len(), expected.1),
                Entry::Level(_, _) => panic!("the value is a level"),
            }
        }
    }
}
