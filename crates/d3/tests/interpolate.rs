//! The port of d3-interpolate, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-interpolate 1.4.0
//! under Node — the same code the browser runs — and writing down what it
//! produced. The numbers are written down as their bits, for a number
//! written in text and read back may not be the same number.

use std::collections::BTreeMap;
use std::fs;

use d3::color::{color, Color};
use d3::interpolate::{
    color as colour, curve, interpolate, interpolate_date, interpolate_discrete, interpolate_hue,
    interpolate_number, interpolate_number_array, interpolate_object, interpolate_round,
    interpolate_string, piecewise, quantize, transform, zoom, Value,
};
use serde_json::Value as Json;

/// The number a fixture holds, which is written down as its bits.
fn bits(value: &Json) -> u64 {
    let text = value.as_str().expect("the bits of a number");
    let mut bits = 0u64;
    for i in 0..16 {
        let digit = u8::from_str_radix(&text[i..i + 1], 16).expect("a hex digit");
        bits = (bits << 4) | digit as u64;
    }
    bits
}

/// The number a fixture holds, which is written down as its bits.
fn number(value: &Json) -> f64 {
    f64::from_bits(bits(value))
}

fn fixture() -> Json {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/interpolate.json"
    );
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The numbers of a fixture's list.
fn numbers(value: &Json) -> Vec<f64> {
    value
        .as_array()
        .expect("a list of numbers")
        .iter()
        .map(number)
        .collect()
}

/// The table of the fixture, under a name.
fn table<'a>(fixture: &'a Json, name: &str) -> &'a serde_json::Map<String, Json> {
    fixture
        .get(name)
        .unwrap_or_else(|| panic!("the fixture holds {}", name))
        .as_object()
        .expect("a table of values")
}

#[test]
fn the_numbers_are_drawn_the_javascripts_way() {
    let fixture = fixture();
    for (name, function) in [("number", 0), ("round", 1), ("date", 2)] {
        let table = table(&fixture, name);
        assert!(!table.is_empty());
        for (key, expected) in table {
            let (a, b) = key.split_once('/').expect("two numbers");
            let a = a.parse::<f64>().expect("a number");
            let b = b.parse::<f64>().expect("a number");
            let expected = numbers(expected);
            let got: Vec<f64> = expected
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    let t = point(index, expected.len());
                    match function {
                        0 => interpolate_number(a, b)(t),
                        1 => interpolate_round(a, b)(t),
                        _ => interpolate_date(a, b)(t),
                    }
                })
                .collect();
            for (got, expected) in got.iter().zip(expected.iter()) {
                assert_eq!(
                    got.to_bits(),
                    expected.to_bits(),
                    "{} of {} gave {:?}, the JavaScript gave {:?}",
                    name,
                    key,
                    got,
                    expected
                );
            }
        }
    }
}

/// The points the fixture was drawn at.
fn point(index: usize, count: usize) -> f64 {
    const POINTS: [f64; 7] = [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0];
    assert_eq!(count, POINTS.len());
    POINTS[index]
}

#[test]
fn a_number_that_is_not_a_number_is_drawn_the_javascripts_way() {
    let fixture = fixture();
    let nan_table = table(&fixture, "numberNaN");
    assert!(!nan_table.is_empty());
    for (key, expected) in nan_table {
        let (a, b) = key.split_once('/').expect("two numbers");
        let a = js_number(a);
        let b = js_number(b);
        let got = interpolate_number(a, b)(0.5);
        assert_eq!(got.to_bits(), bits(expected), "{}", key);
    }
    let hue_table = table(&fixture, "hueNaN");
    assert!(!hue_table.is_empty());
    for (key, expected) in hue_table {
        let (a, b) = key.split_once('/').expect("two numbers");
        let a = js_number(a);
        let b = js_number(b);
        let got = interpolate_hue(a, b)(0.5);
        assert_eq!(got.to_bits(), bits(expected), "{}", key);
    }
}

/// A number written the JavaScript's way, which is `NaN` or `Infinity`
/// as well as a decimal.
fn js_number(text: &str) -> f64 {
    match text {
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        other => other.parse::<f64>().expect("a number"),
    }
}

#[test]
fn the_lists_are_drawn_the_javascripts_way() {
    let fixture = fixture();
    let table = table(&fixture, "numberArray");
    for (key, expected) in table {
        let (a, b) = key.split_once('/').expect("two lists");
        let a = list(a);
        let b = list(b);
        let expected = expected.as_array().expect("a list of lists");
        let count = expected.len();
        for (index, expected) in expected.iter().enumerate() {
            let t = point(index, count);
            let got = interpolate_number_array(&a, &b)(t);
            let expected = numbers(expected);
            assert_eq!(got.len(), expected.len());
            for (got, expected) in got.iter().zip(expected.iter()) {
                assert_eq!(got.to_bits(), expected.to_bits());
            }
        }
    }
}

/// A list of numbers, which the fixture's key holds.
fn list(text: &str) -> Vec<f64> {
    text.trim_matches(['[', ']'])
        .split(',')
        .filter(|part| !part.trim().is_empty())
        .map(|part| part.trim().parse::<f64>().expect("a number"))
        .collect()
}

#[test]
fn the_objects_are_drawn_the_javascripts_way() {
    let fixture = fixture();
    let table = table(&fixture, "object");
    for (key, expected) in table {
        let (a, b) = key.split_once('/').expect("two objects");
        let a = object(a);
        let b = object(b);
        let expected = expected.as_array().expect("a list of objects");
        let count = expected.len();
        let got: Vec<BTreeMap<String, Value>> = expected
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let t = point(index, count);
                interpolate_object(&a, &b)(t)
            })
            .collect();
        for (got, expected) in got.iter().zip(expected.iter()) {
            let expected = expected.as_object().expect("an object");
            for (key, value) in expected {
                let got = got.get(key).expect("the key");
                match got {
                    Value::Number(number) => {
                        assert_eq!(number.to_bits(), bits(value), "the key {}", key)
                    }
                    other => panic!("the key {} is {:?}", key, other),
                }
            }
        }
    }
}

/// An object, which the fixture's key holds.
fn object(text: &str) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for part in text.trim_matches(['{', '}']).split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (key, number) = part.split_once(':').expect("a key and a number");
        out.insert(
            key.trim().trim_matches('"').to_owned(),
            Value::Number(number.trim().parse::<f64>().expect("a number")),
        );
    }
    out
}

#[test]
fn the_strings_are_drawn_the_javascripts_way() {
    let fixture = fixture();
    let table = table(&fixture, "string");
    for (key, expected) in table {
        let (a, b) = key.split_once('/').expect("two strings");
        let expected = expected.as_array().expect("a list of strings");
        let count = expected.len();
        let mut got = interpolate_string(a, b);
        for (index, expected) in expected.iter().enumerate() {
            let t = point(index, count);
            assert_eq!(
                &got(t),
                expected.as_str().expect("a string"),
                "the string of {}",
                key
            );
        }
    }
}

#[test]
fn the_colours_are_drawn_the_javascripts_way() {
    let fixture = fixture();
    let table = table(&fixture, "rgb");
    for (key, expected) in table {
        let (a, b) = key.split_once('/').expect("two colours");
        let a = color(a).expect("the colour");
        let b = color(b).expect("the colour");
        let expected = expected.as_array().expect("a list of colours");
        let count = expected.len();
        let got = colour::interpolate_rgb(&a, &b);
        for (index, expected) in expected.iter().enumerate() {
            let t = point(index, count);
            assert_eq!(
                got(t),
                expected.as_str().expect("a colour"),
                "the colour of {}",
                key
            );
        }
    }
}

#[test]
fn the_colour_spaces_are_drawn_the_javascripts_way() {
    let fixture = fixture();
    let table = table(&fixture, "colours");
    for (key, expected) in table {
        let mut parts = key.split('/');
        let name = parts.next().expect("the name");
        let a = parts.next().expect("the colour");
        let b = parts.next().expect("the colour");
        let a = color(a).expect("the colour");
        let b = color(b).expect("the colour");
        let expected = expected.as_array().expect("a list of colours");
        let count = expected.len();
        let got: Box<dyn Fn(f64) -> String> = match name {
            "hsl" => Box::new(colour::interpolate_hsl(&a, &b)),
            "hslLong" => Box::new(colour::interpolate_hsl_long(&a, &b)),
            "lab" => Box::new(colour::interpolate_lab(&a, &b)),
            "hcl" => Box::new(colour::interpolate_hcl(&a, &b)),
            "hclLong" => Box::new(colour::interpolate_hcl_long(&a, &b)),
            "cubehelix" => Box::new(colour::interpolate_cubehelix(&a, &b)),
            "cubehelixLong" => Box::new(colour::interpolate_cubehelix_long(&a, &b)),
            other => panic!("{} is not a colour space", other),
        };
        for (index, expected) in expected.iter().enumerate() {
            let t = point(index, count);
            assert_eq!(
                got(t),
                expected.as_str().expect("a colour"),
                "the {} of {}",
                name,
                key
            );
        }
    }
}

#[test]
fn the_colour_curves_are_drawn_the_javascripts_way() {
    let fixture = fixture();
    let table = table(&fixture, "colourCurves");
    for (name, expected) in table {
        let colours = ["red", "green", "blue"]
            .iter()
            .map(|text| color(text).expect("the colour"))
            .collect::<Vec<Color>>();
        let expected = expected.as_array().expect("a list of colours");
        let count = expected.len();
        let got: Box<dyn Fn(f64) -> String> = match name.as_str() {
            "rgbBasis" => Box::new(colour::interpolate_rgb_basis(&colours)),
            "rgbBasisClosed" => Box::new(colour::interpolate_rgb_basis_closed(&colours)),
            other => panic!("{} is not a curve", other),
        };
        for (index, expected) in expected.iter().enumerate() {
            let t = point(index, count);
            assert_eq!(
                got(t),
                expected.as_str().expect("a colour"),
                "the {} curve",
                name
            );
        }
    }
}

#[test]
fn the_hue_is_drawn_the_javascripts_way() {
    let fixture = fixture();
    let table = table(&fixture, "hue");
    for (key, expected) in table {
        let (a, b) = key.split_once('/').expect("two numbers");
        let a = a.parse::<f64>().expect("a number");
        let b = b.parse::<f64>().expect("a number");
        let expected = numbers(expected);
        let count = expected.len();
        let got: Vec<f64> = expected
            .iter()
            .enumerate()
            .map(|(index, _)| interpolate_hue(a, b)(point(index, count)))
            .collect();
        for (got, expected) in got.iter().zip(expected.iter()) {
            assert_eq!(got.to_bits(), expected.to_bits());
        }
    }
}

#[test]
fn the_curves_are_drawn_the_javascripts_way() {
    let fixture = fixture();
    let table = table(&fixture, "curves");
    for (key, expected) in table {
        let (name, values) = key.split_once('/').expect("a name and a list");
        let values = list(values);
        let expected = numbers(expected);
        let count = expected.len();
        let got: Vec<f64> = expected
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let t = point(index, count);
                match name {
                    "basis" => curve::basis(&values)(t),
                    "basisClosed" => curve::basis_closed(&values)(t),
                    other => panic!("{} is not a curve", other),
                }
            })
            .collect();
        for (got, expected) in got.iter().zip(expected.iter()) {
            assert_eq!(got.to_bits(), expected.to_bits());
        }
    }
}

#[test]
fn the_discrete_and_the_piecewise_are_the_javascripts_way() {
    let fixture = fixture();
    let discrete = fixture
        .get("discrete")
        .expect("the discrete")
        .as_array()
        .expect("a list of values");
    let range = ["a", "b", "c"]
        .iter()
        .map(|text| Value::String(text.to_string()))
        .collect::<Vec<Value>>();
    let got = interpolate_discrete(&range);
    for (index, expected) in discrete.iter().enumerate() {
        let t = point(index, discrete.len());
        assert_eq!(
            got(t),
            Value::String(expected.as_str().expect("a value").to_owned())
        );
    }

    let pieces = fixture
        .get("piecewise")
        .expect("the piecewise")
        .as_array()
        .expect("a list of numbers");
    let got = piecewise(
        |a, b| Box::new(interpolate_number(a, b)),
        &[0.0, 10.0, 20.0],
    );
    for (index, expected) in pieces.iter().enumerate() {
        let t = point(index, pieces.len());
        assert_eq!(got(t).to_bits(), bits(expected));
    }

    let quantized = fixture
        .get("quantize")
        .expect("the quantize")
        .as_array()
        .expect("a list of numbers");
    let got = quantize(interpolate_number(0.0, 10.0), quantized.len());
    for (got, expected) in got.iter().zip(quantized.iter()) {
        assert_eq!(got.to_bits(), bits(expected));
    }
}

#[test]
fn the_zoom_is_drawn_the_javascripts_way() {
    let fixture = fixture();
    let table = table(&fixture, "zoom");
    for (key, expected) in table {
        let (a, b) = key.split_once('/').expect("two zooms");
        let a = zoom_of(a);
        let b = zoom_of(b);
        let expected = expected.as_array().expect("a list of zooms");
        let count = expected.len();
        let got = zoom::interpolate_zoom(a, b);
        for (index, expected) in expected.iter().enumerate() {
            let t = point(index, count);
            let got = got(t);
            let expected = expected.as_array().expect("a zoom");
            for (index, value) in expected.iter().enumerate() {
                assert_eq!(got[index].to_bits(), bits(value), "the zoom of {}", key);
            }
        }
    }
}

/// A zoom, which the fixture's key holds.
fn zoom_of(text: &str) -> [f64; 3] {
    let values = list(text);
    [values[0], values[1], values[2]]
}

#[test]
fn the_value_is_drawn_the_javascripts_way() {
    let fixture = fixture();
    let table = table(&fixture, "value");
    for (key, expected) in table {
        let (a, b) = key.split_once('/').expect("two values");
        let a = value_of(a);
        let b = value_of(b);
        let expected = expected.as_array().expect("a list of values");
        let count = expected.len();
        let mut got = interpolate(&a, &b);
        for (index, expected) in expected.iter().enumerate() {
            let t = point(index, count);
            let expected = expected.as_object().expect("a value");
            let kind = expected["kind"].as_str().expect("a kind");
            let got = got(t);
            match kind {
                "number" => match got {
                    Value::Number(number) => assert_eq!(
                        number.to_bits(),
                        bits(&expected["value"]),
                        "the value of {}",
                        key
                    ),
                    other => panic!("the value of {} is {:?}", key, other),
                },
                "string" => match got {
                    Value::String(text) => assert_eq!(
                        text,
                        expected["value"].as_str().expect("a string"),
                        "the value of {}",
                        key
                    ),
                    other => panic!("the value of {} is {:?}", key, other),
                },
                "null" => assert!(matches!(got, Value::Null), "the value of {}", key),
                "boolean" => match got {
                    Value::Bool(value) => {
                        assert_eq!(value, expected["value"].as_bool().expect("a boolean"))
                    }
                    other => panic!("the value of {} is {:?}", key, other),
                },
                "array" => match got {
                    Value::Array(values) => {
                        let expected = expected["value"].as_array().expect("a list");
                        assert_eq!(values.len(), expected.len());
                        for (value, expected) in values.iter().zip(expected.iter()) {
                            let expected = expected.as_object().expect("a value");
                            assert_eq!(expected["kind"].as_str().unwrap(), "number");
                            match value {
                                Value::Number(number) => {
                                    assert_eq!(number.to_bits(), bits(&expected["value"]))
                                }
                                other => panic!("a value is {:?}", other),
                            }
                        }
                    }
                    other => panic!("the value of {} is {:?}", key, other),
                },
                "object" => match got {
                    Value::Object(values) => {
                        let expected = expected["value"].as_object().expect("an object");
                        assert_eq!(values.len(), expected.len());
                        for (key, expected) in expected {
                            let expected = expected.as_object().expect("a value");
                            assert_eq!(expected["kind"].as_str().unwrap(), "number");
                            match values.get(key) {
                                Some(Value::Number(number)) => {
                                    assert_eq!(number.to_bits(), bits(&expected["value"]))
                                }
                                other => panic!("the value of {} is {:?}", key, other),
                            }
                        }
                    }
                    other => panic!("the value of {} is {:?}", key, other),
                },
                other => panic!("{} is not a kind", other),
            }
        }
    }
}

#[test]
fn the_matrix_is_taken_apart_the_javascripts_way() {
    let fixture = fixture();
    let table = table(&fixture, "decompose");
    assert!(!table.is_empty());
    for (key, expected) in table {
        let matrix: Vec<f64> = key
            .split('/')
            .map(|part| part.parse::<f64>().expect("a number"))
            .collect();
        let got = transform::decompose(
            matrix[0], matrix[1], matrix[2], matrix[3], matrix[4], matrix[5],
        );
        let expected = expected.as_object().expect("a transform");
        assert_eq!(got.translate_x.to_bits(), bits(&expected["translateX"]));
        assert_eq!(got.translate_y.to_bits(), bits(&expected["translateY"]));
        assert_eq!(got.rotate.to_bits(), bits(&expected["rotate"]));
        assert_eq!(got.skew_x.to_bits(), bits(&expected["skewX"]));
        assert_eq!(got.scale_x.to_bits(), bits(&expected["scaleX"]));
        assert_eq!(got.scale_y.to_bits(), bits(&expected["scaleY"]));
    }
}

/// A value, which the fixture's key holds.
fn value_of(text: &str) -> Value {
    match text {
        "null" => Value::Null,
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => {
            if text.starts_with('"') {
                return Value::String(text.trim_matches('"').to_owned());
            }
            if text.starts_with('[') {
                let mut values = Vec::new();
                for part in text.trim_matches(['[', ']']).split(',') {
                    let part = part.trim();
                    if part.is_empty() {
                        continue;
                    }
                    values.push(value_of(part));
                }
                return Value::Array(values);
            }
            if text.starts_with('{') {
                let mut values = BTreeMap::new();
                for part in text.trim_matches(['{', '}']).split(',') {
                    let part = part.trim();
                    if part.is_empty() {
                        continue;
                    }
                    let (key, number) = part.split_once(':').expect("a key and a number");
                    values.insert(
                        key.trim().trim_matches('"').to_owned(),
                        Value::Number(number.trim().parse::<f64>().expect("a number")),
                    );
                }
                return Value::Object(values);
            }
            Value::Number(text.parse::<f64>().expect("a number"))
        }
    }
}
