//! The port of d3-quadtree, checked against d3's own JavaScript.

use std::fs;

use d3::quadtree::{quadtree, Point};
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quadtree.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

fn bits(value: &Value) -> u64 {
    let text = value.as_str().expect("the bits of a number");
    let mut bits = 0u64;
    for i in 0..16 {
        let digit = u8::from_str_radix(&text[i..i + 1], 16).expect("a hex digit");
        bits = (bits << 4) | digit as u64;
    }
    bits
}

fn same_number(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        true
    } else {
        a.to_bits() == b.to_bits()
    }
}

fn same_point(a: [f64; 2], b: &Value) -> bool {
    let list = b.as_array().expect("a point");
    list.len() == 2
        && same_number(a[0], f64::from_bits(bits(&list[0])))
        && same_number(a[1], f64::from_bits(bits(&list[1])))
}

#[test]
fn the_size_is_the_javascripts() {
    let f = fixture();
    let pts: Vec<[f64; 2]> = vec![
        [0.0, 0.0],
        [1.0, 1.0],
        [2.0, 2.0],
        [0.5, 0.5],
        [3.0, 3.0],
        [1.0, 0.0],
        [0.0, 1.0],
    ];
    let q = quadtree(&pts);
    assert_eq!(q.size(), f["size"].as_u64().unwrap() as usize);
}

#[test]
fn the_data_is_the_javascripts() {
    let f = fixture();
    let pts: Vec<[f64; 2]> = vec![
        [0.0, 0.0],
        [1.0, 1.0],
        [2.0, 2.0],
        [0.5, 0.5],
        [3.0, 3.0],
        [1.0, 0.0],
        [0.0, 1.0],
    ];
    let q = quadtree(&pts);
    let data = q.data();
    let expected = f["data"].as_array().unwrap();
    assert_eq!(data.len(), expected.len());
    for (d, e) in data.iter().zip(expected.iter()) {
        assert!(same_point(**d, e));
    }
}

#[test]
fn the_extent_is_the_javascripts() {
    let f = fixture();
    let pts: Vec<[f64; 2]> = vec![
        [0.0, 0.0],
        [1.0, 1.0],
        [2.0, 2.0],
        [0.5, 0.5],
        [3.0, 3.0],
        [1.0, 0.0],
        [0.0, 1.0],
    ];
    let q = quadtree(&pts);
    let ext = q.extent().unwrap();
    let e = f["extent"].as_array().unwrap();
    // extent is [[x0,y0],[x1,y1]]
    let e0 = e[0].as_array().unwrap();
    let e1 = e[1].as_array().unwrap();
    assert!(same_number(ext.0 .0, f64::from_bits(bits(&e0[0]))));
    assert!(same_number(ext.0 .1, f64::from_bits(bits(&e0[1]))));
    assert!(same_number(ext.1 .0, f64::from_bits(bits(&e1[0]))));
    assert!(same_number(ext.1 .1, f64::from_bits(bits(&e1[1]))));
}

#[test]
fn the_find_is_the_javascripts() {
    let f = fixture();
    let pts: Vec<[f64; 2]> = vec![
        [0.0, 0.0],
        [1.0, 1.0],
        [2.0, 2.0],
        [0.5, 0.5],
        [3.0, 3.0],
        [1.0, 0.0],
        [0.0, 1.0],
    ];
    let q = quadtree(&pts);
    // find(0,0)
    let r = q.find(0.0, 0.0, None).copied();
    if let Some(pt) = r {
        assert!(same_point(pt, &f["find00"]));
    } else {
        assert!(f["find00"].is_null());
    }
    let r2 = q.find(0.6, 0.6, Some(0.5));
    if let Some(pt) = r2 {
        assert!(same_point(*pt, &f["findNear"]));
    } else {
        assert!(f["findNear"].is_null());
    }
}

#[test]
fn the_cover_is_the_javascripts() {
    let f = fixture();
    let mut q = d3::quadtree::Quadtree::<[f64; 2]>::new();
    q.cover(5.0, 5.0).cover(-1.0, -1.0);
    let ext = q.extent().unwrap();
    let e = f["coverExtent"].as_array().unwrap();
    let e0 = e[0].as_array().unwrap();
    let e1 = e[1].as_array().unwrap();
    assert!(same_number(ext.0 .0, f64::from_bits(bits(&e0[0]))));
    assert!(same_number(ext.0 .1, f64::from_bits(bits(&e0[1]))));
    assert!(same_number(ext.1 .0, f64::from_bits(bits(&e1[0]))));
    assert!(same_number(ext.1 .1, f64::from_bits(bits(&e1[1]))));
}

#[test]
fn the_remove_is_the_javascripts() {
    let f = fixture();
    let pts2: Vec<[f64; 2]> = vec![
        [0.0, 0.0],
        [1.0, 1.0],
        [2.0, 2.0],
        [0.5, 0.5],
        [3.0, 3.0],
        [1.0, 0.0],
        [0.0, 1.0],
    ];
    let mut q = quadtree(&pts2);
    q.remove(&pts2[1]);
    assert_eq!(q.size(), f["sizeAfterRemove"].as_u64().unwrap() as usize);
    let data = q.data();
    let expected = f["dataAfterRemove"].as_array().unwrap();
    assert_eq!(data.len(), expected.len());
    for (d, e) in data.iter().zip(expected.iter()) {
        assert!(same_point(**d, e));
    }
}

#[test]
fn the_visit_count_is_the_javascripts() {
    let f = fixture();
    let pts: Vec<[f64; 2]> = vec![
        [0.0, 0.0],
        [1.0, 1.0],
        [2.0, 2.0],
        [0.5, 0.5],
        [3.0, 3.0],
        [1.0, 0.0],
        [0.0, 1.0],
    ];
    let q = quadtree(&pts);
    let mut count = 0usize;
    q.visit(|_node, _x0, _y0, _x1, _y1| {
        count += 1;
        false
    });
    assert_eq!(count, f["visitedCount"].as_u64().unwrap() as usize);
}

#[test]
fn the_copy_is_the_javascripts() {
    let f = fixture();
    let pts: Vec<[f64; 2]> = vec![
        [0.0, 0.0],
        [1.0, 1.0],
        [2.0, 2.0],
        [0.5, 0.5],
        [3.0, 3.0],
        [1.0, 0.0],
        [0.0, 1.0],
    ];
    let q = quadtree(&pts);
    let qc = q.copy();
    assert_eq!(qc.size(), f["copySize"].as_u64().unwrap() as usize);
    assert_eq!(qc.size() == q.size(), f["copyEqual"].as_bool().unwrap());
}

#[test]
fn the_accessors_are_the_javascripts() {
    let f = fixture();
    #[derive(Clone)]
    struct P {
        x: f64,
        y: f64,
    }
    impl Point for P {
        fn x(&self) -> f64 {
            self.x
        }
        fn y(&self) -> f64 {
            self.y
        }
    }
    let mut q = d3::quadtree::Quadtree::<P>::with_accessors(|d: &P| d.x, |d: &P| d.y);
    q.add_all(&[P { x: 1.0, y: 2.0 }, P { x: 3.0, y: 4.0 }]);
    let data = q.data();
    let expected = f["accData"].as_array().unwrap();
    assert_eq!(data.len(), expected.len());
    for (d, e) in data.iter().zip(expected.iter()) {
        let list = e.as_array().unwrap();
        assert!(same_number(d.x, f64::from_bits(bits(&list[0]))));
        assert!(same_number(d.y, f64::from_bits(bits(&list[1]))));
    }
}
