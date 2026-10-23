//! The port of d3-format, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-format 1.4.1
//! under Node — the same code the browser runs — and writing down what it
//! produced.

use std::fs;

use d3::format::{format_default_locale, format_locale, format_specifier, LocaleDefinition};
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/format.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

#[test]
fn the_numbers_are_written_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("format").expect("the formats");
    let table = table.as_object().expect("a table of formats");
    assert!(!table.is_empty());
    let locale = format_default_locale();
    let values = values(&fixture);
    for (specifier, expected) in table {
        let format = locale.format(specifier);
        match expected {
            Value::Null => {
                // The JavaScript cannot read the specification, and the
                // port cannot either.
                assert!(format.is_none(), "{} was read", specifier);
            }
            expected => {
                let format = format.unwrap_or_else(|| panic!("{} was not read", specifier));
                let expected = expected.as_array().expect("a list of numbers");
                for (got, expected) in expected.iter().enumerate() {
                    let value = values[got];
                    assert_eq!(
                        format.format(value),
                        expected.as_str().expect("a number"),
                        "the format of {} for {}",
                        specifier,
                        value
                    );
                }
            }
        }
    }
}

/// The values the oracle wrote the formats for, which the fixture
/// holds as their bits.
fn values(fixture: &Value) -> Vec<f64> {
    fixture
        .get("values")
        .expect("the values")
        .as_array()
        .expect("a list of values")
        .iter()
        .map(|value| {
            let text = value.as_str().expect("the bits of a number");
            let mut bits = 0u64;
            for i in 0..16 {
                let digit = u8::from_str_radix(&text[i..i + 1], 16).expect("a hex digit");
                bits = (bits << 4) | digit as u64;
            }
            f64::from_bits(bits)
        })
        .collect()
}

#[test]
fn the_specifications_are_read_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("specifier").expect("the specifications");
    let table = table.as_object().expect("a table of specifications");
    assert!(!table.is_empty());
    for (specifier, expected) in table {
        let read = format_specifier(specifier);
        match expected {
            Value::Null => {
                assert!(read.is_none(), "{} was read", specifier);
            }
            expected => {
                let read = read.unwrap_or_else(|| panic!("{} was not read", specifier));
                let expected = expected.as_object().expect("a specification");
                assert_eq!(read.fill, expected["fill"].as_str().unwrap());
                assert_eq!(read.align, expected["align"].as_str().unwrap());
                assert_eq!(read.sign, expected["sign"].as_str().unwrap());
                assert_eq!(read.symbol, expected["symbol"].as_str().unwrap());
                assert_eq!(read.zero, expected["zero"].as_bool().unwrap());
                assert_eq!(
                    read.width,
                    expected["width"].as_u64().map(|width| width as usize)
                );
                assert_eq!(read.comma, expected["comma"].as_bool().unwrap());
                assert_eq!(
                    read.precision,
                    expected["precision"]
                        .as_u64()
                        .map(|precision| precision as usize)
                );
                assert_eq!(read.trim, expected["trim"].as_bool().unwrap());
                assert_eq!(read.kind, expected["type"].as_str().unwrap());
            }
        }
    }
}

#[test]
fn the_prefixed_numbers_are_written_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("prefix").expect("the prefixed numbers");
    let table = table.as_object().expect("a table of numbers");
    assert!(!table.is_empty());
    let locale = format_default_locale();
    for (key, expected) in table {
        let (specifier, value) = key.rsplit_once('/').expect("a format and a value");
        let value: f64 = value.parse().expect("a number");
        let format = locale
            .format_prefix(specifier, value)
            .unwrap_or_else(|| panic!("{} was not read", specifier));
        assert_eq!(
            format.format(value),
            expected.as_str().expect("a number"),
            "the prefixed format of {} for {}",
            specifier,
            value
        );
    }
}

#[test]
fn the_counts_of_the_digits_are_the_javascripts_way() {
    let fixture = fixture();
    let expected = fixture.get("precision").expect("the counts of the digits");
    let expected = expected.as_object().expect("the counts of the digits");
    let values = values(&fixture);
    let step = 0.1;
    let fixed = expected["fixed"].as_array().expect("a list of numbers");
    let prefix = expected["prefix"].as_array().expect("a list of numbers");
    let round = expected["round"].as_array().expect("a list of numbers");
    for (index, value) in values.iter().enumerate() {
        let expected_fixed = fixed[index].as_f64().unwrap_or(f64::NAN);
        let expected_prefix = prefix[index].as_f64().unwrap_or(f64::NAN);
        let expected_round = round[index].as_f64().unwrap_or(f64::NAN);
        let got_fixed = d3::format::precision_fixed(step);
        let got_prefix = d3::format::precision_prefix(step, *value);
        let got_round = d3::format::precision_round(step, *value);
        assert!(
            (got_fixed.is_nan() && expected_fixed.is_nan()) || got_fixed == expected_fixed,
            "the fixed count for {}",
            value
        );
        assert!(
            (got_prefix.is_nan() && expected_prefix.is_nan()) || got_prefix == expected_prefix,
            "the prefix count for {}",
            value
        );
        assert!(
            (got_round.is_nan() && expected_round.is_nan()) || got_round == expected_round,
            "the round count for {}",
            value
        );
    }
}

#[test]
fn the_locale_is_the_javascripts_way() {
    let fixture = fixture();
    let expected = fixture.get("locale").expect("the locale");
    let expected = expected.as_object().expect("the locale");
    let definition = LocaleDefinition {
        decimal: Some(",".to_owned()),
        thousands: Some(".".to_owned()),
        grouping: Some(vec![3, 2]),
        currency: Some(["€".to_owned(), String::new()]),
        percent: Some("%".to_owned()),
        minus: Some("−".to_owned()),
        numerals: None,
        nan: None,
    };
    let locale = format_locale(&definition);
    let format = locale.format(",.2f").expect("the format");
    let values = values(&fixture);
    let list = expected["format"].as_array().expect("a list of numbers");
    for (index, expected) in list.iter().enumerate() {
        assert_eq!(
            format.format(values[index]),
            expected.as_str().expect("a number"),
            "the number {}",
            values[index]
        );
    }
    let prefix = locale.format_prefix(".2f", 1234.5).expect("the format");
    assert_eq!(
        prefix.format(1234.5),
        expected["prefix"].as_str().expect("a number")
    );
}
