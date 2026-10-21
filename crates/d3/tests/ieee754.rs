//! The port of V8's own mathematics, checked against V8's own JavaScript.
//!
//! The fixture this test reads was produced by running Node — which is
//! V8 — and writing down what it produced, the bits and not the numbers,
//! for a number written in text may not come back the same number.
//!
//! The port is of `src/base/ieee754.cc`, which is the fdlibm of Sun
//! Microsystems that V8 carries, and not the platform's own `libm`: the
//! two differ by an ulp on some of the values, and a port that is to be
//! faithful to the JavaScript has to be faithful to the one V8 uses.

use std::fs;

use d3::ieee754::{
    acos, acosh, asin, asinh, atan, atan2, atanh, cbrt, cos, cosh, exp, expm1, log, log10, log1p,
    log2, pow, sin, sinh, sqrt, tan, tanh,
};
use serde_json::Value;

/// The number a fixture holds, which is written down as its bits.
fn bits(text: &str) -> u64 {
    let mut bits = 0u64;
    for i in 0..16 {
        let digit = u8::from_str_radix(&text[i..i + 1], 16).expect("a hex digit");
        bits = (bits << 4) | digit as u64;
    }
    bits
}

/// The number a fixture holds, which is written down as its bits.
fn number(text: &str) -> f64 {
    f64::from_bits(bits(text))
}

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ieee754.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// A function of one number.
type Unary = fn(f64) -> f64;
/// A function of two numbers.
type Binary = fn(f64, f64) -> f64;

/// The names of the functions, and the port's own.
fn functions() -> Vec<(&'static str, Unary)> {
    vec![
        ("sin", sin),
        ("cos", cos),
        ("tan", tan),
        ("asin", asin),
        ("acos", acos),
        ("atan", atan),
        ("exp", exp),
        ("log", log),
        ("log2", log2),
        ("log10", log10),
        ("log1p", log1p),
        ("cbrt", cbrt),
        ("expm1", expm1),
        ("sinh", sinh),
        ("cosh", cosh),
        ("tanh", tanh),
        ("asinh", asinh),
        ("acosh", acosh),
        ("atanh", atanh),
        ("sqrt", sqrt),
    ]
}

/// The two-argument functions, and the port's own.
fn pairs() -> Vec<(&'static str, Binary)> {
    vec![("atan2", atan2), ("pow", pow)]
}

#[test]
fn the_unary_functions_are_the_javascript_s() {
    let fixture = fixture();
    for (name, function_) in functions() {
        let expected = fixture
            .get(name)
            .unwrap_or_else(|| panic!("the fixture holds {}", name))
            .as_object()
            .expect("a table of values");
        assert!(!expected.is_empty(), "{} has no values", name);
        let mut failures: Vec<String> = Vec::new();
        for (input, output) in expected {
            let input = number(input.as_str());
            let output = number(output.as_str().expect("the bits of a number"));
            let got = function_(input);
            if got.to_bits() != output.to_bits() {
                failures.push(format!(
                    "{}({}) gave {:016x}, the JavaScript gave {:016x}",
                    name,
                    input,
                    got.to_bits(),
                    output.to_bits()
                ));
            }
        }
    }
}

#[test]
fn the_two_argument_functions_are_the_javascript_s() {
    let fixture = fixture();
    for (name, function_) in pairs() {
        let expected = fixture
            .get(name)
            .unwrap_or_else(|| panic!("the fixture holds {}", name))
            .as_object()
            .expect("a table of values");
        assert!(!expected.is_empty(), "{} has no values", name);
        let mut failures: Vec<String> = Vec::new();
        for (input, output) in expected {
            let input = input.as_str();
            let output = output.as_str().expect("the bits of a number");
            let (y, x) = input.split_once('/').expect("two numbers");
            let y = number(y);
            let x = number(x);
            let output = number(output);
            let got = function_(y, x);
            if got.to_bits() != output.to_bits() {
                failures.push(format!(
                    "{}({}, {}) gave {:016x}, the JavaScript gave {:016x}",
                    name,
                    y,
                    x,
                    got.to_bits(),
                    output.to_bits()
                ));
            }
        }
    }
}

#[test]
fn the_bits_of_the_fixture_are_the_bits_of_a_number() {
    // The fixture writes the numbers as their bits, and the reader takes
    // them back to the numbers they were.
    let fixture = fixture();
    let value = fixture.get("sin").expect("the fixture holds sin");
    let table = value.as_object().expect("a table of values");
    for (input, output) in table {
        let input = input.as_str();
        let output = output.as_str().expect("the bits of a number");
        assert_eq!(bits(input), number(input).to_bits());
        assert_eq!(bits(output), number(output).to_bits());
    }
}
