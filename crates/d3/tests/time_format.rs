//! The port of d3-time-format, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-time-format
//! 2.3.0 under Node — the same code the browser runs — and writing down
//! what it produced. The Rust here must give the same answers.

use std::fs;
use std::sync::{Mutex, MutexGuard};

use d3::time::Date;
use d3::time_format::{
    default_definition, iso_format, iso_parse, time_format, time_format_default_locale,
    time_format_locale, time_parse, utc_format, utc_parse, LocaleDefinition,
};
use serde_json::Value as Json;

/// The lock the tests that read the default locale are held under ✓.
///
/// The default locale is the JavaScript's own, and the JavaScript's own
/// changes it ✓, and the change is seen by everything that reads it ✓.
static DEFAULT: Mutex<()> = Mutex::new(());

/// The lock the default locale is held under ✓.
///
/// The lock is taken even when a test has failed, for the other tests
/// are of the port and not of the lock ✓.
fn default() -> MutexGuard<'static, ()> {
    DEFAULT.lock().unwrap_or_else(|held| held.into_inner())
}

fn fixture() -> Json {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/time_format.json"
    );
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The dates the formats and the parses are given, which the fixture
/// holds ✓.
fn dates(fixture: &Json, name: &str) -> Vec<f64> {
    fixture
        .get(name)
        .expect("the dates")
        .as_array()
        .expect("a list of dates")
        .iter()
        .map(|date| date.as_f64().expect("a number"))
        .collect()
}

/// The locale of one's own, which the oracle's own holds ✓.
fn definition() -> LocaleDefinition {
    LocaleDefinition {
        date_time: "%x %X".to_owned(),
        date: "%d.%m.%Y".to_owned(),
        time: "%H:%M".to_owned(),
        periods: vec!["am".to_owned(), "pm".to_owned()],
        days: [
            "dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi",
        ]
        .iter()
        .map(|day| (*day).to_owned())
        .collect(),
        short_days: ["di", "lu", "ma", "me", "je", "ve", "sa"]
            .iter()
            .map(|day| (*day).to_owned())
            .collect(),
        months: [
            "janvier",
            "février",
            "mars",
            "avril",
            "mai",
            "juin",
            "juillet",
            "août",
            "septembre",
            "octobre",
            "novembre",
            "décembre",
        ]
        .iter()
        .map(|month| (*month).to_owned())
        .collect(),
        short_months: [
            "jan", "fév", "mar", "avr", "mai", "jui", "jul", "aoû", "sep", "oct", "nov", "déc",
        ]
        .iter()
        .map(|month| (*month).to_owned())
        .collect(),
    }
}

/// The strings of a list of the fixture's own ✓.
fn strings(value: &Json) -> Vec<String> {
    value
        .as_array()
        .expect("a list of strings")
        .iter()
        .map(|value| value.as_str().expect("a string").to_owned())
        .collect()
}

/// The date a parse gives, which the fixture holds ✓.
///
/// The fixture writes `null` for a text that is not a date, and a word
/// for a date that is not a date at all ✓.
fn same_date(date: Option<Date>, value: &Json) -> bool {
    match value {
        Json::Null => date.is_none(),
        Json::Number(instant) => match date {
            Some(date) => date.instant == instant.as_f64().expect("a number"),
            None => false,
        },
        Json::String(word) => {
            word == "Invalid"
                && match date {
                    Some(date) => date.instant.is_nan(),
                    None => false,
                }
        }
        _ => false,
    }
}

#[test]
fn the_formats_write_the_dates_the_javascripts_way() {
    let _held = default();
    let fixture = fixture();
    let table = fixture.get("format").expect("the formats");
    let table = table.as_object().expect("the formats");
    for (specifier, expected) in table {
        let format = time_format(specifier);
        assert_eq!(format.specifier(), specifier, "the specification");
        let expected = strings(expected);
        for (instant, expected) in dates(&fixture, "dates").iter().zip(expected.iter()) {
            assert_eq!(
                format.format(&Date::local(*instant)),
                *expected,
                "the format of {} at {}",
                specifier,
                instant
            );
        }
    }
}

#[test]
fn the_utc_formats_write_the_dates_the_javascripts_way() {
    let _held = default();
    let fixture = fixture();
    let table = fixture.get("utcFormat").expect("the formats");
    let table = table.as_object().expect("the formats");
    for (specifier, expected) in table {
        let format = utc_format(specifier);
        let expected = strings(expected);
        let instants: Vec<f64> = dates(&fixture, "dates")
            .into_iter()
            .chain(dates(&fixture, "oldDates"))
            .collect();
        for (instant, expected) in instants.iter().zip(expected.iter()) {
            assert_eq!(
                format.format(&Date::utc(*instant)),
                *expected,
                "the format of {} at {}",
                specifier,
                instant
            );
        }
    }
}

#[test]
fn the_parses_read_the_dates_the_javascripts_way() {
    let _held = default();
    let fixture = fixture();
    let table = fixture.get("parse").expect("the parses");
    let table = table.as_object().expect("the parses");
    for (specifier, expected) in table {
        let parse = time_parse(specifier);
        let expected = expected.as_object().expect("the dates");
        for (text, expected) in expected {
            assert!(
                same_date(parse.parse(text), expected),
                "the parse of {} at {:?}",
                specifier,
                text
            );
        }
    }
}

#[test]
fn the_utc_parses_read_the_dates_the_javascripts_way() {
    let _held = default();
    let fixture = fixture();
    let table = fixture.get("utcParse").expect("the parses");
    let table = table.as_object().expect("the parses");
    for (specifier, expected) in table {
        let parse = utc_parse(specifier);
        let expected = expected.as_object().expect("the dates");
        for (text, expected) in expected {
            assert!(
                same_date(parse.parse(text), expected),
                "the parse of {} at {:?}",
                specifier,
                text
            );
        }
    }
}

#[test]
fn the_locale_of_ones_own_writes_the_dates_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("locale").expect("the locale");
    let locale = time_format_locale(&definition());
    let specifier = "%A %d %B %Y %X %x %c %p";
    let expected = strings(table.get("format").expect("the formats"));
    let format = locale.format(specifier);
    for (instant, expected) in dates(&fixture, "dates").iter().zip(expected.iter()) {
        assert_eq!(
            format.format(&Date::local(*instant)),
            *expected,
            "the format at {}",
            instant
        );
    }
    let expected = strings(table.get("utcFormat").expect("the formats"));
    let format = locale.utc_format(specifier);
    let instants: Vec<f64> = dates(&fixture, "dates")
        .into_iter()
        .chain(dates(&fixture, "oldDates"))
        .collect();
    for (instant, expected) in instants.iter().zip(expected.iter()) {
        assert_eq!(
            format.format(&Date::utc(*instant)),
            *expected,
            "the format at {}",
            instant
        );
    }
}

#[test]
fn the_locale_of_ones_own_reads_the_dates_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("locale").expect("the locale");
    let locale = time_format_locale(&definition());
    for (name, key) in [("parse", "parse"), ("utcParse", "utcParse")] {
        let table = table.get(key).expect("the parses");
        let table = table.as_object().expect("the parses");
        for (specifier, expected) in table {
            let parse = if name == "parse" {
                locale.parse(specifier)
            } else {
                locale.utc_parse(specifier)
            };
            let expected = expected.as_object().expect("the dates");
            for (text, expected) in expected {
                assert!(
                    same_date(parse.parse(text), expected),
                    "the parse of {} at {:?}",
                    specifier,
                    text
                );
            }
        }
    }
}

#[test]
fn the_default_locale_is_the_one_the_javascripts_way() {
    let _held = default();
    let fixture = fixture();
    let table = fixture.get("default").expect("the default locale");

    // The locale is changed, and the change is seen by everything that
    // reads the default ✓. The locale is put back before anything is
    // compared, for the other tests are reading it as well ✓.
    let locale = time_format_default_locale(&definition());
    let specifier = "%A %d %B %Y";
    let format = time_format(specifier);
    let instants: Vec<f64> = dates(&fixture, "dates")
        .into_iter()
        .chain(dates(&fixture, "oldDates"))
        .collect();
    let local: Vec<(f64, String)> = dates(&fixture, "dates")
        .iter()
        .map(|instant| (*instant, format.format(&Date::local(*instant))))
        .collect();
    let utc_format = utc_format(specifier);
    let utc: Vec<(f64, String)> = instants
        .iter()
        .map(|instant| (*instant, utc_format.format(&Date::utc(*instant))))
        .collect();
    let parse = time_parse("%Y %d.%m.%Y");
    let parsed: Vec<(String, Json)> = table
        .get("parse")
        .expect("the parses")
        .as_object()
        .expect("the dates")
        .iter()
        .map(|(text, date)| (text.clone(), date.clone()))
        .collect();
    let utc_parse = utc_parse("%Y %d.%m.%Y");
    let utc_parsed: Vec<(String, Json)> = table
        .get("utcParse")
        .expect("the parses")
        .as_object()
        .expect("the dates")
        .iter()
        .map(|(text, date)| (text.clone(), date.clone()))
        .collect();
    time_format_default_locale(&default_definition());

    // The formats are the ones of the locale that was given ✓.
    let expected = strings(table.get("format").expect("the formats"));
    for ((instant, got), expected) in local.iter().zip(expected.iter()) {
        assert_eq!(got, expected, "the format at {}", instant);
    }
    let expected = strings(table.get("utcFormat").expect("the formats"));
    for ((instant, got), expected) in utc.iter().zip(expected.iter()) {
        assert_eq!(got, expected, "the format at {}", instant);
    }

    // The parses are the ones of the locale that was given ✓.
    for (text, expected) in parsed {
        assert!(
            same_date(parse.parse(&text), &expected),
            "the parse at {:?}",
            text
        );
    }
    for (text, expected) in utc_parsed {
        assert!(
            same_date(utc_parse.parse(&text), &expected),
            "the parse at {:?}",
            text
        );
    }

    // The locale of one's own is the one that was given ✓.
    assert_eq!(locale, time_format_locale(&definition()));
}

#[test]
fn a_date_that_is_not_a_date_is_written_the_javascripts_way() {
    let _held = default();
    let fixture = fixture();
    let table = fixture.get("invalid").expect("the date that is not a date");
    for (name, key) in [("format", "format"), ("utcFormat", "utcFormat")] {
        let table = table.get(key).expect("the formats");
        let table = table.as_object().expect("the formats");
        for (specifier, expected) in table {
            let format = if name == "format" {
                time_format(specifier)
            } else {
                utc_format(specifier)
            };
            let expected = expected.as_str().expect("a string");
            assert_eq!(
                format.format(&Date::utc(f64::NAN)),
                expected,
                "the format of {}",
                specifier
            );
        }
    }
}

#[test]
fn the_iso_form_writes_the_dates_the_javascripts_way() {
    let fixture = fixture();
    let expected = strings(
        fixture
            .get("iso")
            .expect("the ISO form")
            .get("format")
            .expect("the formats"),
    );
    let instants: Vec<f64> = dates(&fixture, "dates")
        .into_iter()
        .chain(dates(&fixture, "oldDates"))
        .collect();
    for (instant, expected) in instants.iter().zip(expected.iter()) {
        match iso_format(&Date::utc(*instant)) {
            Ok(text) => assert_eq!(text, *expected, "the ISO form at {}", instant),
            Err(_) => assert_eq!(*expected, "Invalid", "the ISO form at {}", instant),
        }
    }
}

#[test]
fn the_iso_form_reads_the_dates_the_javascripts_way() {
    let fixture = fixture();
    let expected = fixture
        .get("iso")
        .expect("the ISO form")
        .get("parse")
        .expect("the parses")
        .as_object()
        .expect("the dates");
    for (text, expected) in expected {
        assert!(
            same_date(iso_parse(text), expected),
            "the ISO form of {:?}",
            text
        );
    }
}
