//! The port of d3-array, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-array 1.2.4 under
//! Node — the same code the browser runs — and writing down what it
//! produced. The Rust here must give the same answers.

use std::fs;

use d3::array;
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/array.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The number a fixture holds.
fn number(value: &Value) -> f64 {
    value.as_f64().expect("a number")
}

/// The list a fixture holds.
fn numbers(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("a list")
        .iter()
        .map(number)
        .collect()
}

/// Whether two numbers are the same to the last bit, which is what the
/// JavaScript's numbers are compared on.
fn same(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    a == b && a.is_sign_negative() == b.is_sign_negative()
}

/// Whether a list of numbers is the same as the fixture's.
fn same_list(a: &[f64], value: &Value) -> bool {
    let expected = numbers(value);
    a.len() == expected.len() && a.iter().zip(expected.iter()).all(|(x, y)| same(*x, *y))
}

#[test]
fn the_port_gives_what_the_javascript_gives() {
    let f = fixture();

    // range
    assert!(same_list(
        &array::range(0.0, Some(10.0), Some(2.0)),
        &f["range"]
    ));
    assert!(same_list(
        &array::range(10.0, Some(0.0), Some(-2.0)),
        &f["rangeNeg"]
    ));
    assert!(same_list(
        &array::range(0.0, Some(1.0), Some(0.25)),
        &f["rangeFloat"]
    ));
    assert!(same_list(&array::range(5.0, None, None), &f["rangeOne"]));

    // min, max, extent
    assert!(same(
        array::min(&[3.0, 1.0, 2.0]).unwrap(),
        number(&f["min"])
    ));
    assert_eq!(array::min(&[]), None);
    assert!(f["minEmpty"].is_null());
    assert!(same(
        array::min(&[3.0, f64::NAN, 2.0]).unwrap(),
        number(&f["minNaN"])
    ));
    assert!(same(
        array::max(&[3.0, 1.0, 2.0]).unwrap(),
        number(&f["max"])
    ));
    assert_eq!(array::max(&[]), None);
    let (min, max) = array::extent(&[3.0, 1.0, 2.0]);
    assert!(same(min.unwrap(), number(&f["extent"][0])));
    assert!(same(max.unwrap(), number(&f["extent"][1])));
    let (min, max) = array::extent(&[]);
    assert_eq!(min, None);
    assert_eq!(max, None);
    assert!(f["extentEmpty"][0].is_null());

    // sum, mean, median, quantile
    assert!(same(array::sum(&[1.0, 2.0, 3.0]), number(&f["sum"])));
    assert!(same(array::sum(&[]), number(&f["sumEmpty"])));
    assert!(same(
        array::mean(&[1.0, 2.0, 3.0]).unwrap(),
        number(&f["mean"])
    ));
    assert_eq!(array::mean(&[]), None);
    assert!(f["meanEmpty"].is_null());
    assert!(same(
        array::median(&[1.0, 3.0, 2.0]).unwrap(),
        number(&f["median"])
    ));
    assert!(same(
        array::median(&[1.0, 2.0, 3.0, 4.0]).unwrap(),
        number(&f["medianEven"])
    ));
    assert!(same(
        array::quantile(&[1.0, 2.0, 3.0, 4.0], 0.5).unwrap(),
        number(&f["quantile"])
    ));
    // The JavaScript's quantile does not sort, and the port must not
    // either.
    assert!(same(
        array::quantile(&[4.0, 1.0, 3.0, 2.0], 0.5).unwrap(),
        number(&f["quantileUnsorted"])
    ));

    // variance, deviation
    assert!(same(
        array::variance(&[1.0, 2.0, 3.0, 4.0]).unwrap(),
        number(&f["variance"])
    ));
    assert!(same(
        array::deviation(&[1.0, 2.0, 3.0, 4.0]).unwrap(),
        number(&f["deviation"])
    ));

    // merge
    assert!(same_list(
        &array::merge(&[vec![1.0, 2.0], vec![3.0], vec![4.0, 5.0]]),
        &f["merge"]
    ));

    // transpose
    for (rows, key) in [
        (vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]], "transpose"),
        (vec![vec![1.0, 2.0, 3.0], vec![4.0]], "transposeRagged"),
    ] {
        let got = array::transpose(&rows);
        let expected = f[key].as_array().expect("a list");
        assert_eq!(got.len(), expected.len());
        for (row, want) in got.iter().zip(expected) {
            assert!(same_list(row, want));
        }
    }

    // pairs
    let got = array::pairs(&[1.0, 2.0, 3.0, 4.0]);
    let expected = f["pairs"].as_array().expect("a list");
    assert_eq!(got.len(), expected.len());
    for (pair, want) in got.iter().zip(expected) {
        let want = want.as_array().expect("a pair");
        assert!(same(pair.0, number(&want[0])));
        assert!(same(pair.1, number(&want[1])));
    }

    // cross, of two lists of the same kind
    let got = array::cross(&[1.0, 2.0], &[10.0, 20.0]);
    let expected = f["cross"].as_array().expect("a list");
    assert_eq!(got.len(), expected.len());
    for (pair, want) in got.iter().zip(expected) {
        let want = want.as_array().expect("a pair");
        assert!(same(pair.0, number(&want[0])));
        assert!(same(pair.1, number(&want[1])));
    }

    // zip, of two lists of the same kind
    let got = array::zip(&[vec![1.0, 2.0, 3.0], vec![10.0, 20.0, 30.0]]);
    let expected = f["zip"].as_array().expect("a list");
    assert_eq!(got.len(), expected.len());
    for (row, want) in got.iter().zip(expected) {
        let want = want.as_array().expect("a row");
        assert_eq!(row.len(), want.len());
        assert!(same(row[0], number(&want[0])));
        assert!(same(row[1], number(&want[1])));
    }

    // ticks
    assert!(same_list(&array::ticks(0.0, 10.0, 5.0), &f["ticks"]));
    assert!(same_list(&array::ticks(10.0, 0.0, 5.0), &f["ticksNeg"]));
    assert!(same(
        array::tick_increment(0.0, 10.0, 5.0),
        number(&f["tickIncrement"])
    ));
    assert!(same(
        array::tick_step(0.0, 10.0, 5.0),
        number(&f["tickStep"])
    ));
    assert!(same(
        array::tick_step(10.0, 0.0, 5.0),
        number(&f["tickStepNeg"])
    ));

    // bisect
    assert_eq!(
        array::bisect(&[1.0, 2.0, 3.0, 4.0], 3.0),
        f["bisect"].as_u64().unwrap() as usize
    );
    assert_eq!(
        array::bisect_right(&[1.0, 2.0, 3.0, 4.0], 3.0),
        f["bisectRight"].as_u64().unwrap() as usize
    );
    assert_eq!(
        array::bisect_left(&[1.0, 2.0, 3.0, 4.0], 3.0),
        f["bisectLeft"].as_u64().unwrap() as usize
    );
    assert_eq!(
        array::bisector_left(&[1.0, 2.0, 3.0, 4.0], 3.0),
        f["bisectorLeft"].as_u64().unwrap() as usize
    );
    assert_eq!(
        array::bisector_right(&[1.0, 2.0, 3.0, 4.0], 3.0),
        f["bisectorRight"].as_u64().unwrap() as usize
    );

    // ascending, descending
    let ascending = f["ascending"].as_array().expect("a list");
    assert_eq!(array::ascending(1.0, 2.0), Some(std::cmp::Ordering::Less));
    assert_eq!(
        array::ascending(2.0, 1.0),
        Some(std::cmp::Ordering::Greater)
    );
    assert_eq!(array::ascending(1.0, 1.0), Some(std::cmp::Ordering::Equal));
    assert!(same(ascending[0].as_f64().unwrap(), -1.0));
    assert!(same(ascending[1].as_f64().unwrap(), 1.0));
    assert!(same(ascending[2].as_f64().unwrap(), 0.0));
    let descending = f["descending"].as_array().expect("a list");
    assert!(same(descending[0].as_f64().unwrap(), 1.0));
    assert!(same(descending[1].as_f64().unwrap(), -1.0));
    assert!(same(descending[2].as_f64().unwrap(), 0.0));

    // scan, permute
    assert_eq!(
        array::scan(&[1.0, 3.0, 2.0], &|a: &f64, b: &f64| array::ascending(
            *a, *b
        )),
        Some(f["scan"].as_u64().unwrap() as usize)
    );
    assert_eq!(
        array::scan::<f64>(&[], &|a: &f64, b: &f64| array::ascending(*a, *b)),
        None
    );
    let got = array::permute(
        &["a".to_owned(), "b".to_owned(), "c".to_owned()],
        &[2, 0, 1],
    );
    let expected = f["permute"].as_array().expect("a list");
    assert_eq!(got.len(), expected.len());
    for (value, want) in got.iter().zip(expected) {
        assert_eq!(value, want.as_str().expect("a string"));
    }

    // the thresholds
    let values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    assert!(same(
        array::threshold_sturges(&values),
        number(&f["sturges"])
    ));
    assert!(same(
        array::threshold_scott(&values, 1.0, 8.0),
        number(&f["scott"])
    ));
    assert!(same(
        array::threshold_freedman_diaconis(&values, 1.0, 8.0),
        number(&f["fd"])
    ));

    // histogram
    let data = [
        1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0,
    ];
    let bins = array::histogram(
        &data,
        &|d: &f64, _, _| *d,
        &|values: &[f64]| array::extent(values),
        &|values: &[f64], min: f64, max: f64| {
            array::Threshold::Count(array::threshold_sturges(values).min(max - min))
        },
    );
    let expected = f["histogram"].as_array().expect("a list");
    assert_eq!(bins.len(), expected.len());
    for (bin, want) in bins.iter().zip(expected) {
        assert!(same(bin.x0, number(&want["x0"])));
        assert!(same(bin.x1, number(&want["x1"])));
        assert_eq!(bin.data.len(), want["n"].as_u64().unwrap() as usize);
    }
}
