//! The ECMAScript's own writing of a number, checked against Node.
//!
//! The fixture this test reads was produced by running Node, which is V8,
//! and writing down what it produced.

use std::fs;

use d3::number::{to_exponential, to_fixed, to_precision, to_radix};
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/number.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The number and the digits of a key.
fn parts(text: &str) -> (f64, usize) {
    let (value, digits) = text.rsplit_once('/').expect("a number and a count");
    (
        value.parse::<f64>().expect("a number"),
        digits.parse::<usize>().expect("a count"),
    )
}

#[test]
fn the_fixed_numbers_are_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("fixed").expect("the fixed numbers");
    let table = table.as_object().expect("a table of numbers");
    assert!(!table.is_empty());
    for (key, expected) in table {
        let (value, digits) = parts(key);
        assert_eq!(
            to_fixed(value, digits),
            expected.as_str().expect("a number"),
            "the fixed form of {}",
            key
        );
    }
}

#[test]
fn the_exponential_numbers_are_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("exponential").expect("the exponential numbers");
    let table = table.as_object().expect("a table of numbers");
    assert!(!table.is_empty());
    for (key, expected) in table {
        let (value, digits) = parts(key);
        assert_eq!(
            to_exponential(value, digits),
            expected.as_str().expect("a number"),
            "the exponential form of {}",
            key
        );
    }
}

#[test]
fn the_numbers_in_another_base_are_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("radix").expect("the numbers in another base");
    let table = table.as_object().expect("a table of numbers");
    assert!(!table.is_empty());
    for (key, expected) in table {
        let (value, radix) = key.rsplit_once('/').expect("a number and a base");
        let value = value.parse::<f64>().expect("a number");
        let radix = radix.parse::<u32>().expect("a base");
        // The formats that use this round the number first.
        assert_eq!(
            to_radix(d3::js::round(value), radix),
            expected.as_str().expect("a number"),
            "the number {} in the base {}",
            value,
            radix
        );
    }
}

#[test]
fn the_significant_numbers_are_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("precision").expect("the significant numbers");
    let table = table.as_object().expect("a table of numbers");
    assert!(!table.is_empty());
    for (key, expected) in table {
        let (value, digits) = parts(key);
        assert_eq!(
            to_precision(value, digits),
            expected.as_str().expect("a number"),
            "the significant form of {}",
            key
        );
    }
}
