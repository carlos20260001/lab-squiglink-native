//! The port of d3-time, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-time 2.3.0 under
//! Node — the same code the browser runs — and writing down what it
//! produced.

use std::fs;

use d3::time::interval::{
    time_day, time_millisecond, time_minute, time_month, time_second, time_sunday, time_week,
    time_year, utc_day, utc_hour, utc_millisecond, utc_minute, utc_month, utc_second, utc_sunday,
    utc_week, utc_year, Interval,
};
use d3::time::Date;
use serde_json::Value as Json;

fn fixture() -> Json {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/time.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The dates the intervals are given, which the fixture holds ✓.
fn dates(fixture: &Json) -> Vec<f64> {
    numbers(fixture.get("dates").expect("the dates"))
}

/// The intervals, and the names they are known by.
fn intervals() -> Vec<(&'static str, Interval)> {
    vec![
        ("timeMillisecond", time_millisecond()),
        ("timeSecond", time_second()),
        ("timeMinute", time_minute()),
        ("timeDay", time_day()),
        ("timeWeek", time_week()),
        ("timeSunday", time_sunday()),
        ("timeMonth", time_month()),
        ("timeYear", time_year()),
        ("utcMillisecond", utc_millisecond()),
        ("utcSecond", utc_second()),
        ("utcMinute", utc_minute()),
        ("utcHour", utc_hour()),
        ("utcDay", utc_day()),
        ("utcWeek", utc_week()),
        ("utcSunday", utc_sunday()),
        ("utcMonth", utc_month()),
        ("utcYear", utc_year()),
    ]
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
fn the_intervals_floor_the_dates_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("floor").expect("the floors");
    let table = table.as_object().expect("the floors");
    for (name, interval) in intervals() {
        let expected = table
            .get(name)
            .unwrap_or_else(|| panic!("the fixture holds {}", name));
        let expected = numbers(expected);
        for (instant, expected) in dates(&fixture).iter().zip(expected.iter()) {
            let mut date = Date::local(*instant);
            interval.floor(&mut date);
            assert_eq!(
                date.instant, *expected,
                "the floor of {} at {}",
                name, instant
            );
        }
    }
}

#[test]
fn the_intervals_offset_the_dates_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("offset").expect("the offsets");
    let table = table.as_object().expect("the offsets");
    for (name, interval) in intervals() {
        let expected = table
            .get(name)
            .unwrap_or_else(|| panic!("the fixture holds {}", name));
        let expected = numbers(expected);
        for (instant, expected) in dates(&fixture).iter().zip(expected.iter()) {
            let mut date = Date::local(*instant);
            interval.offset(&mut date, 3);
            assert_eq!(
                date.instant, *expected,
                "the offset of {} at {}",
                name, instant
            );
        }
    }
}

#[test]
fn the_intervals_count_the_dates_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("count").expect("the counts");
    let table = table.as_object().expect("the counts");
    let dates = dates(&fixture);
    for (name, interval) in intervals() {
        let expected = table
            .get(name)
            .unwrap_or_else(|| panic!("the fixture holds {}", name));
        let expected = expected.as_f64().expect("a number");
        let count = interval
            .count(&Date::local(dates[0]), &Date::local(dates[2]))
            .unwrap_or_else(|| panic!("{} has no count", name));
        assert_eq!(count, expected, "the count of {}", name);
    }
}

#[test]
fn the_intervals_range_over_the_dates_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("range").expect("the ranges");
    let table = table.as_object().expect("the ranges");
    let dates = dates(&fixture);
    for (name, interval) in intervals() {
        let expected = table
            .get(name)
            .unwrap_or_else(|| panic!("the fixture holds {}", name));
        let expected = numbers(expected);
        let range = interval.range(&Date::local(dates[0]), &Date::local(dates[0] + 1000.0), 1);
        assert_eq!(range.len(), expected.len(), "the range of {}", name);
        for (got, expected) in range.iter().zip(expected.iter()) {
            assert_eq!(got.instant, *expected, "the range of {}", name);
        }
    }
}

#[test]
fn the_parts_of_a_date_are_the_javascripts_way() {
    let fixture = fixture();
    let table = fixture.get("local").expect("the local parts");
    let fields = table.get("fields").expect("the fields");
    let fields = fields.as_array().expect("a list of parts");
    let offsets = table.get("offset").expect("the offsets");
    let offsets = numbers(offsets);
    for (index, instant) in dates(&fixture).iter().enumerate() {
        let date = Date::local(*instant);
        let got = date.fields();
        let expected = fields[index].as_array().expect("the parts");
        assert_eq!(got.year, expected[0].as_i64().unwrap(), "the year");
        assert_eq!(got.month, expected[1].as_u64().unwrap() as u32, "the month");
        assert_eq!(got.day, expected[2].as_u64().unwrap() as u32, "the day");
        assert_eq!(
            got.weekday,
            expected[3].as_u64().unwrap() as u32,
            "the weekday"
        );
        assert_eq!(got.hour, expected[4].as_u64().unwrap() as u32, "the hour");
        assert_eq!(
            got.minute,
            expected[5].as_u64().unwrap() as u32,
            "the minute"
        );
        assert_eq!(
            got.second,
            expected[6].as_u64().unwrap() as u32,
            "the second"
        );
        assert_eq!(
            got.millisecond,
            expected[7].as_u64().unwrap() as u32,
            "the millisecond"
        );
        // The offset of the system, which the JavaScript's own has the
        // other way about.
        assert_eq!(date.offset(), -offsets[index] as i32, "the offset");
    }
}
