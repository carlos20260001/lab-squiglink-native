//! The port of d3-path, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-path 1.0.9 under
//! Node — the same code the browser runs — and writing down what it
//! produced.

use std::fs;

use d3::js;
use d3::path::Path;
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/path.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

#[test]
fn the_paths_are_the_same() {
    let f = fixture();

    let mut p = Path::default();
    p.move_to(0.0, 0.0);
    p.line_to(1.0, 2.0);
    p.close_path();
    assert_eq!(p.to_string(), f["basic"].as_str().unwrap());

    let mut q = Path::default();
    q.move_to(0.0, 0.0);
    q.arc(1.0, 2.0, 3.0, 0.0, std::f64::consts::PI, false)
        .expect("a positive radius");
    assert_eq!(q.to_string(), f["arc"].as_str().unwrap());

    let mut s = Path::default();
    s.move_to(0.0, 0.0);
    s.quadratic_curve_to(1.0, 2.0, 3.0, 4.0);
    assert_eq!(s.to_string(), f["quadratic"].as_str().unwrap());

    let mut t = Path::default();
    t.move_to(0.0, 0.0);
    t.bezier_curve_to(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
    assert_eq!(t.to_string(), f["bezier"].as_str().unwrap());

    let mut u = Path::default();
    u.move_to(0.0, 0.0);
    u.arc_to(1.0, 1.0, 2.0, 0.0, 1.0)
        .expect("a positive radius");
    assert_eq!(u.to_string(), f["arcTo"].as_str().unwrap());

    let mut v = Path::default();
    v.rect(1.0, 2.0, 3.0, 4.0);
    assert_eq!(v.to_string(), f["rect"].as_str().unwrap());
}

#[test]
fn the_numbers_are_written_the_javascripts_way() {
    let f = fixture();
    let expected = f["numbers"].as_array().expect("a list");
    let values = [
        1.0,
        1.5,
        0.1 + 0.2,
        1e21,
        1e-7,
        1.0 / 3.0,
        -0.0,
        100.0,
        1e20,
        1e-6,
        255.0,
        1e6,
        1e-21,
        123456789012345678901234.0,
    ];
    assert_eq!(values.len(), expected.len());
    for (value, want) in values.iter().zip(expected) {
        assert_eq!(
            js::number_to_string(*value),
            want.as_str().expect("a string"),
            "the number {value}"
        );
    }
}

#[test]
fn a_path_of_large_numbers_is_the_same() {
    let f = fixture();
    let mut w = Path::default();
    w.move_to(1e21, 1e-7);
    w.line_to(-0.0, 1e20);
    assert_eq!(w.to_string(), f["bigNumbers"].as_str().unwrap());
}

#[test]
fn a_negative_radius_is_an_error_as_in_the_javascript() {
    let mut p = Path::default();
    p.move_to(0.0, 0.0);
    let error = p.arc(1.0, 2.0, -1.0, 0.0, 1.0, false).unwrap_err();
    assert_eq!(error.to_string(), "negative radius: -1");
}
