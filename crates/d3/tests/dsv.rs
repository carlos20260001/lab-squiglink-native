//! The port of d3-dsv, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-dsv 1.2.0 under
//! Node — the same code the browser runs — and writing down what it
//! produced. The Rust here must give the same answers.
//!
//! The dates of the fixture are read in the timezone of Greenwich, and
//! the test reads them in the same one, for a date without a timezone
//! in it is read in the timezone of the program that reads it.

use std::fs;

use d3::dsv::{self, Dsv, Row, Value};
use d3::time::{Date, TimeZone};
use serde_json::Value as Json;

fn fixture() -> Json {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/dsv.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The bits of a number, which the fixture writes down rather than the
/// number itself.
fn bits_of(json: &Json) -> u64 {
    let text = json.as_str().expect("the bits of a number");
    u64::from_str_radix(text, 16).expect("the bits of a number")
}

/// The value the fixture holds.
fn value(json: &Json) -> Value {
    if let Some(text) = json.get("text") {
        return Value::String(text.as_str().expect("a text").to_owned());
    }
    if let Some(bits) = json.get("bits") {
        return Value::Number(f64::from_bits(bits_of(bits)));
    }
    if let Some(boolean) = json.get("bool") {
        return Value::Bool(boolean.as_bool().expect("a boolean"));
    }
    if let Some(date) = json.get("date") {
        let bits = date.get("bits").expect("the bits of a date");
        return Value::Date(Date::utc(f64::from_bits(bits_of(bits))));
    }
    // The JavaScript's own null.
    Value::Null
}

/// The row the fixture holds.
fn row(json: &Json) -> Row {
    let mut row = Row::new();
    for entry in json.as_array().expect("a row") {
        let name = entry[0].as_str().expect("the name of a value");
        row.set(name, value(&entry[1]));
    }
    row
}

/// The rows the fixture holds.
fn rows(json: &Json) -> Vec<Row> {
    json.as_array().expect("the rows").iter().map(row).collect()
}

/// The row of values the fixture holds.
fn value_row(json: &Json) -> Vec<Value> {
    json.as_array().expect("a row").iter().map(value).collect()
}

/// The rows of values the fixture holds.
fn value_rows(json: &Json) -> Vec<Vec<Value>> {
    json.as_array()
        .expect("the rows")
        .iter()
        .map(value_row)
        .collect()
}

/// Whether two values are the same, which is what the JavaScript's own
/// numbers are compared on: the last bit of them is what counts, and a
/// number that is not a number is the same as itself.
fn same(got: &Value, expected: &Json) -> bool {
    match got {
        Value::Null => expected.get("null").is_some(),
        Value::Bool(got) => expected.get("bool").and_then(Json::as_bool) == Some(*got),
        Value::Number(got) => expected
            .get("bits")
            .map(|bits| bits_of(bits) == got.to_bits())
            .unwrap_or(false),
        Value::String(got) => expected.get("text").and_then(Json::as_str) == Some(got.as_str()),
        Value::Date(got) => expected
            .get("date")
            .and_then(|date| date.get("bits"))
            .map(|bits| bits_of(bits) == got.instant.to_bits())
            .unwrap_or(false),
    }
}

/// The delimiters the fixture holds, with the names they are known by.
fn delimiters(fixture: &Json) -> Vec<(&'static str, Dsv)> {
    ["csv", "tsv", "semi"]
        .iter()
        .map(|name| {
            let delimiter = fixture["delimiters"][*name]
                .as_str()
                .expect("the delimiter");
            (*name, Dsv::new(delimiter))
        })
        .collect()
}

/// The texts the fixture holds for a delimiter.
fn texts<'a>(fixture: &'a Json, name: &str) -> &'a [Json] {
    fixture["texts"][name]
        .as_array()
        .expect("the texts")
        .as_slice()
}

/// The rows of values the fixture holds for a delimiter.
fn rows_of_values(json: &Json) -> Vec<Vec<String>> {
    json.as_array()
        .expect("the rows")
        .iter()
        .map(|row| {
            row.as_array()
                .expect("a row")
                .iter()
                .map(|value| value.as_str().expect("a value").to_owned())
                .collect()
        })
        .collect()
}

/// The columns the fixture holds for a table, which are the columns
/// themselves or nothing at all.
fn columns(json: &Json) -> Option<Vec<String>> {
    match json {
        Json::Null => None,
        json => Some(
            json.as_array()
                .expect("the columns")
                .iter()
                .map(|column| column.as_str().expect("a column").to_owned())
                .collect(),
        ),
    }
}

#[test]
fn the_rows_of_the_texts_are_the_javascripts_way() {
    let fixture = fixture();
    for (name, dsv) in delimiters(&fixture) {
        let expected = fixture[name]["parseRows"].as_array().expect("the rows");
        for (text, expected) in texts(&fixture, name).iter().zip(expected.iter()) {
            let text = text.as_str().expect("a text");
            assert_eq!(
                dsv.parse_rows(text),
                rows_of_values(expected),
                "the rows of {:?} read with {}",
                text,
                name
            );
        }
    }
}

#[test]
fn the_rows_are_read_under_the_names_of_the_first_row() {
    let fixture = fixture();
    for (name, dsv) in delimiters(&fixture) {
        let expected = fixture[name]["parse"].as_array().expect("the rows");
        for (text, expected) in texts(&fixture, name).iter().zip(expected.iter()) {
            let text = text.as_str().expect("a text");
            let parsed = dsv.parse(text);
            let columns = expected["columns"]
                .as_array()
                .expect("the columns")
                .iter()
                .map(|column| column.as_str().expect("a column").to_owned())
                .collect::<Vec<String>>();
            assert_eq!(parsed.columns, columns, "the columns of {:?}", text);
            let expected = expected["rows"].as_array().expect("the rows");
            assert_eq!(parsed.rows.len(), expected.len(), "the rows of {:?}", text);
            for (got, expected) in parsed.rows.iter().zip(expected.iter()) {
                assert!(
                    same_row(got, expected),
                    "the row of {:?} read with {}",
                    text,
                    name
                );
            }
        }
    }
}

/// Whether a row is the same as the one the fixture holds, which is the
/// order of its names and the value under each of them.
fn same_row(got: &Row, expected: &Json) -> bool {
    let expected = expected.as_array().expect("a row");
    if got.len() != expected.len() {
        return false;
    }
    got.iter()
        .zip(expected.iter())
        .all(|((name, value), entry)| {
            name == entry[0].as_str().expect("the name of a value") && same(value, &entry[1])
        })
}

#[test]
fn the_values_of_the_rows_are_read_as_the_javascripts_reads_them() {
    let fixture = fixture();
    for (name, dsv) in delimiters(&fixture) {
        let expected = fixture[name]["parseWith"].as_array().expect("the rows");
        for (text, expected) in texts(&fixture, name).iter().zip(expected.iter()) {
            let text = text.as_str().expect("a text");
            let parsed = dsv.parse_with(text, |mut row, _index, _columns| {
                dsv::auto_type_in(TimeZone::Utc, &mut row);
                Some(row)
            });
            let expected = expected["rows"].as_array().expect("the rows");
            assert_eq!(parsed.rows.len(), expected.len(), "the rows of {:?}", text);
            for (got, expected) in parsed.rows.iter().zip(expected.iter()) {
                assert!(
                    same_row(got, expected),
                    "the row of {:?} read with {}",
                    text,
                    name
                );
            }
        }
    }
}

#[test]
fn a_row_may_be_left_out() {
    let fixture = fixture();
    for (name, dsv) in delimiters(&fixture) {
        let expected = fixture[name]["parseSkip"].as_array().expect("the rows");
        for (text, expected) in texts(&fixture, name).iter().zip(expected.iter()) {
            let text = text.as_str().expect("a text");
            let parsed = dsv.parse_with(
                text,
                |row, index, _columns| {
                    if index == 0 {
                        None
                    } else {
                        Some(row)
                    }
                },
            );
            let expected = expected["rows"].as_array().expect("the rows");
            assert_eq!(parsed.rows.len(), expected.len(), "the rows of {:?}", text);
            for (got, expected) in parsed.rows.iter().zip(expected.iter()) {
                assert!(
                    same_row(got, expected),
                    "the row of {:?} read with {}",
                    text,
                    name
                );
            }
        }
    }
}

#[test]
fn the_rows_of_the_texts_are_read_by_the_function() {
    let fixture = fixture();
    for (name, dsv) in delimiters(&fixture) {
        let expected = fixture[name]["parseRowsWith"].as_array().expect("the rows");
        for (text, expected) in texts(&fixture, name).iter().zip(expected.iter()) {
            let text = text.as_str().expect("a text");
            let got = dsv.parse_rows_with(text, |row, index| {
                if index % 2 == 0 {
                    None
                } else {
                    Some(row.into_iter().map(|value| value + "!").collect())
                }
            });
            assert_eq!(
                got,
                rows_of_values(expected),
                "the rows of {:?} read with {}",
                text,
                name
            );
        }
    }
}

#[test]
fn the_tables_are_written_out_as_the_javascripts_writes_them() {
    let fixture = fixture();
    let tables = fixture["tables"].as_array().expect("the tables");
    let the_columns = fixture["columns"].as_array().expect("the columns");
    for (name, dsv) in delimiters(&fixture) {
        for section in ["format", "formatBody"] {
            let expected = fixture[name][section].as_array().expect("the tables");
            let mut index = 0;
            for table in tables {
                let table = rows(table);
                for column in the_columns {
                    let column = columns(column);
                    let got = if section == "format" {
                        dsv.format(&table, column.as_deref())
                    } else {
                        dsv.format_body(&table, column.as_deref())
                    };
                    assert_eq!(
                        got,
                        expected[index].as_str().expect("the text"),
                        "the table written out with {}",
                        name
                    );
                    index += 1;
                }
            }
            assert_eq!(index, expected.len(), "the number of tables");
        }
    }
}

#[test]
fn the_rows_of_values_are_written_out() {
    let fixture = fixture();
    let table = value_rows(&fixture["valueRows"]);
    for (name, dsv) in delimiters(&fixture) {
        assert_eq!(
            dsv.format_rows(&table),
            fixture[name]["formatRows"].as_str().expect("the text"),
            "the rows written out with {}",
            name
        );

        let expected = fixture[name]["formatRow"].as_array().expect("the rows");
        assert_eq!(expected.len(), table.len(), "the number of rows");
        for (row, expected) in table.iter().zip(expected.iter()) {
            assert_eq!(
                dsv.format_row(row),
                expected.as_str().expect("the text"),
                "the row written out with {}",
                name
            );
        }
    }
}

#[test]
fn the_values_are_written_out() {
    let fixture = fixture();
    let values = fixture["values"].as_array().expect("the values");
    for (name, dsv) in delimiters(&fixture) {
        let expected = fixture[name]["formatValue"].as_array().expect("the values");
        assert_eq!(expected.len(), values.len(), "the number of values");
        for (json, expected) in values.iter().zip(expected.iter()) {
            let value = value(json);
            assert_eq!(
                dsv.format_value(&value),
                expected.as_str().expect("the text"),
                "the value written out with {}",
                name
            );
        }
    }
}

#[test]
fn the_values_are_read_as_the_javascripts_reads_them() {
    let fixture = fixture();
    let expected = fixture.get("autoType").expect("the values");
    for entry in expected.as_array().expect("the values") {
        let text = entry["text"].as_str().expect("a text");
        let mut row = Row::new();
        row.set("v", Value::String(text.to_owned()));
        dsv::auto_type_in(TimeZone::Utc, &mut row);
        let got = row.get("v").expect("the value");
        assert!(same(got, &entry["value"]), "the value of {:?}", text);
    }
}

#[test]
fn the_delimiters_that_are_not_the_ones_of_a_table_are_the_javascripts_way() {
    let fixture = fixture();
    let expected = fixture.get("odd").expect("the delimiters");
    for entry in expected.as_array().expect("the delimiters") {
        let delimiter = entry["delimiter"].as_str().expect("the delimiter");
        let dsv = Dsv::new(delimiter);
        let texts = entry["texts"].as_array().expect("the texts");
        let expected = entry["parseRows"].as_array().expect("the rows");
        for (text, expected) in texts.iter().zip(expected.iter()) {
            let text = text.as_str().expect("a text");
            assert_eq!(
                dsv.parse_rows(text),
                rows_of_values(expected),
                "the rows of {:?} read with {:?}",
                text,
                delimiter
            );
        }
        let values = entry["values"].as_array().expect("the values");
        let expected = entry["formatValue"].as_array().expect("the values");
        for (value, expected) in values.iter().zip(expected.iter()) {
            let value = Value::String(value.as_str().expect("a value").to_owned());
            assert_eq!(
                dsv.format_value(&value),
                expected.as_str().expect("the text"),
                "the value {:?} written out with {:?}",
                value.to_text(),
                delimiter
            );
        }
    }
}

#[test]
fn a_date_without_a_timezone_is_the_local_time() {
    let mut row = Row::new();
    row.set("v", Value::String("2019-01-02T03:04".to_owned()));
    dsv::auto_type(&mut row);
    let date = match row.get("v") {
        Some(Value::Date(date)) => *date,
        other => panic!("the value is a date, and it is {:?}", other),
    };
    let fields = date.fields();
    assert_eq!(fields.year, 2019, "the year");
    assert_eq!(fields.month, 0, "the month");
    assert_eq!(fields.day, 2, "the day of the month");
    assert_eq!(fields.hour, 3, "the hour");
    assert_eq!(fields.minute, 4, "the minute");
}

#[test]
fn the_values_of_the_javascripts_own_functions_are_the_javascripts_own() {
    let fixture = fixture();
    let values = fixture["values"].as_array().expect("the values");
    let expected = fixture["csv"]["formatValue"]
        .as_array()
        .expect("the values");
    for (json, expected) in values.iter().zip(expected.iter()) {
        let value = value(json);
        assert_eq!(
            dsv::csv_format_value(&value),
            expected.as_str().expect("the text"),
            "the value written out"
        );
    }
}
