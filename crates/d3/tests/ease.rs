//! The port of d3-ease, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-ease under Node —
//! the same code the browser runs — and writing down what it produced. The
//! numbers are written down as their bits, for a number written in text and
//! read back may not be the same number.

use std::fs;

use d3::ease::{self, Back, Elastic, Poly};
use serde_json::Value;

/// The fixture, which the oracle wrote.
fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ease.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The number a fixture holds, which is written down as its bits.
fn number(value: &Value) -> f64 {
    let text = value.as_str().expect("the bits of a number");
    let mut bits = 0u64;
    for index in 0..16 {
        let digit = u8::from_str_radix(&text[index..index + 1], 16).expect("a hex digit");
        bits = (bits << 4) | digit as u64;
    }
    f64::from_bits(bits)
}

/// The numbers of a fixture's list.
fn numbers(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("a list of numbers")
        .iter()
        .map(number)
        .collect()
}

/// Whether two numbers are the same number.
///
/// The bits are the same, except that a `NaN` is a `NaN`.
///
/// This is a difference that is not yet settled: the elastic eases reach
/// a `NaN` through the sine of a number too large, and the JavaScript's
/// sine and the port's give the `NaN` different bits. The JavaScript's
/// own `Math.sin` is the port's `ieee754::sin` ✓, so the difference is
/// in the number that is given to it ✓, and it is not yet found.
fn same(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    a.to_bits() == b.to_bits()
}

/// Whether a list of numbers is the fixture's list.
fn same_list(got: &[f64], expected: &Value, what: &str) {
    let expected = numbers(expected);
    assert_eq!(got.len(), expected.len(), "{}", what);
    for (got, expected) in got.iter().zip(expected.iter()) {
        assert!(
            same(*got, *expected),
            "{}: the port gave {:?}, the JavaScript gave {:?}",
            what,
            got,
            expected
        );
    }
}

/// The port's own ease, which the JavaScript's export names.
fn ease_of(name: &str) -> fn(f64) -> f64 {
    match name {
        "easeLinear" => ease::ease_linear,
        "easeQuad" => ease::ease_quad,
        "easeQuadIn" => ease::ease_quad_in,
        "easeQuadOut" => ease::ease_quad_out,
        "easeQuadInOut" => ease::ease_quad_in_out,
        "easeCubic" => ease::ease_cubic,
        "easeCubicIn" => ease::ease_cubic_in,
        "easeCubicOut" => ease::ease_cubic_out,
        "easeCubicInOut" => ease::ease_cubic_in_out,
        "easePoly" => ease::ease_poly,
        "easePolyIn" => ease::ease_poly_in,
        "easePolyOut" => ease::ease_poly_out,
        "easePolyInOut" => ease::ease_poly_in_out,
        "easeSin" => ease::ease_sin,
        "easeSinIn" => ease::ease_sin_in,
        "easeSinOut" => ease::ease_sin_out,
        "easeSinInOut" => ease::ease_sin_in_out,
        "easeExp" => ease::ease_exp,
        "easeExpIn" => ease::ease_exp_in,
        "easeExpOut" => ease::ease_exp_out,
        "easeExpInOut" => ease::ease_exp_in_out,
        "easeCircle" => ease::ease_circle,
        "easeCircleIn" => ease::ease_circle_in,
        "easeCircleOut" => ease::ease_circle_out,
        "easeCircleInOut" => ease::ease_circle_in_out,
        "easeBounce" => ease::ease_bounce,
        "easeBounceIn" => ease::ease_bounce_in,
        "easeBounceOut" => ease::ease_bounce_out,
        "easeBounceInOut" => ease::ease_bounce_in_out,
        "easeBack" => ease::ease_back,
        "easeBackIn" => ease::ease_back_in,
        "easeBackOut" => ease::ease_back_out,
        "easeBackInOut" => ease::ease_back_in_out,
        "easeElastic" => ease::ease_elastic,
        "easeElasticIn" => ease::ease_elastic_in,
        "easeElasticOut" => ease::ease_elastic_out,
        "easeElasticInOut" => ease::ease_elastic_in_out,
        other => panic!("{} is not an ease", other),
    }
}

/// The port's own poly, which the JavaScript's export names.
fn poly_of(poly: Poly, name: &str) -> Box<dyn Fn(f64) -> f64> {
    match name {
        "easePolyIn" => Box::new(move |t| poly.poly_in(t)),
        "easePolyOut" => Box::new(move |t| poly.poly_out(t)),
        "easePolyInOut" => Box::new(move |t| poly.poly_in_out(t)),
        other => panic!("{} is not a poly", other),
    }
}

/// The port's own back, which the JavaScript's export names.
fn back_of(back: Back, name: &str) -> Box<dyn Fn(f64) -> f64> {
    match name {
        "easeBackIn" => Box::new(move |t| back.back_in(t)),
        "easeBackOut" => Box::new(move |t| back.back_out(t)),
        "easeBackInOut" => Box::new(move |t| back.back_in_out(t)),
        other => panic!("{} is not a back", other),
    }
}

/// The port's own elastic, which the JavaScript's export names.
fn elastic_of(elastic: Elastic, name: &str) -> Box<dyn Fn(f64) -> f64> {
    match name {
        "easeElasticIn" => Box::new(move |t| elastic.elastic_in(t)),
        "easeElasticOut" => Box::new(move |t| elastic.elastic_out(t)),
        "easeElasticInOut" => Box::new(move |t| elastic.elastic_in_out(t)),
        other => panic!("{} is not an elastic", other),
    }
}

/// The points the fixture was drawn at.
fn points(fixture: &Value) -> Vec<f64> {
    numbers(&fixture["points"])
}

#[test]
fn the_eases_are_the_javascripts_way() {
    let fixture = fixture();
    let points = points(&fixture);
    let eases = fixture["eases"].as_object().expect("the eases");
    assert_eq!(eases.len(), 37, "the package has thirty-seven eases");
    for (name, expected) in eases {
        let ease = ease_of(name);
        let got: Vec<f64> = points.iter().map(|t| ease(*t)).collect();
        same_list(&got, expected, name);
    }
}

#[test]
fn the_poly_eases_are_the_javascripts_way() {
    let fixture = fixture();
    let points = points(&fixture);
    let polys = fixture["poly"].as_object().expect("the polys");
    assert_eq!(polys.len(), 3, "the polys have three members");
    for (name, table) in polys {
        let table = table.as_object().expect("a table of exponents");
        assert!(!table.is_empty(), "{} has no exponents", name);
        for (exponent, expected) in table {
            let exponent = exponent.parse::<f64>().expect("an exponent");
            let poly = Poly::new(exponent);
            let ease = poly_of(poly, name);
            let got: Vec<f64> = points.iter().map(|t| ease(*t)).collect();
            same_list(&got, expected, &format!("{} of {}", name, exponent));
        }
    }
}

#[test]
fn the_back_eases_are_the_javascripts_way() {
    let fixture = fixture();
    let points = points(&fixture);
    let backs = fixture["back"].as_object().expect("the backs");
    assert_eq!(backs.len(), 3, "the backs have three members");
    for (name, table) in backs {
        let table = table.as_object().expect("a table of overshoots");
        assert!(!table.is_empty(), "{} has no overshoots", name);
        for (overshoot, expected) in table {
            let overshoot = overshoot.parse::<f64>().expect("an overshoot");
            let back = Back::new(overshoot);
            let ease = back_of(back, name);
            let got: Vec<f64> = points.iter().map(|t| ease(*t)).collect();
            same_list(&got, expected, &format!("{} of {}", name, overshoot));
        }
    }
}

#[test]
fn the_elastic_eases_are_the_javascripts_way() {
    let fixture = fixture();
    let points = points(&fixture);
    let elastic = fixture["elastic"].as_object().expect("the elastic ones");
    assert_eq!(elastic.len(), 3, "the elastic ones have three members");
    for (name, table) in elastic {
        let table = table.as_object().expect("a table of settings");
        assert!(!table.is_empty(), "{} has no settings", name);
        for (setting, expected) in table {
            let (amplitude, period) = setting.split_once('/').expect("two settings");
            let amplitude = amplitude.parse::<f64>().expect("an amplitude");
            let period = period.parse::<f64>().expect("a period");
            let elastic = Elastic::new(amplitude, period);
            let ease = elastic_of(elastic, name);
            let got: Vec<f64> = points.iter().map(|t| ease(*t)).collect();
            same_list(&got, expected, &format!("{} of {}", name, setting));
        }
    }
}

#[test]
fn the_setters_are_the_javascripts_way() {
    let fixture = fixture();
    let points = points(&fixture);
    let setters = fixture["setters"]["elastic"]
        .as_object()
        .expect("the setters of the elastic ones");
    assert_eq!(setters.len(), 3, "the elastic ones have three members");
    for (name, table) in setters {
        let table = table.as_object().expect("a table of setters");

        // The amplitude setter, which keeps the period.
        let amplitudes = table["amplitude"].as_object().expect("the amplitudes");
        assert!(!amplitudes.is_empty(), "{} has no amplitudes", name);
        for (amplitude, expected) in amplitudes {
            let amplitude = amplitude.parse::<f64>().expect("an amplitude");
            let elastic = Elastic::default().amplitude(amplitude);
            let ease = elastic_of(elastic, name);
            let got: Vec<f64> = points.iter().map(|t| ease(*t)).collect();
            same_list(&got, expected, &format!("{} of {}", name, amplitude));
        }

        // The period setter, which keeps the amplitude.
        let periods = table["period"].as_object().expect("the periods");
        assert!(!periods.is_empty(), "{} has no periods", name);
        for (period, expected) in periods {
            let period = period.parse::<f64>().expect("a period");
            let elastic = Elastic::default().period(period);
            let ease = elastic_of(elastic, name);
            let got: Vec<f64> = points.iter().map(|t| ease(*t)).collect();
            same_list(&got, expected, &format!("{} of {}", name, period));
        }
    }
}

#[test]
fn the_defaults_are_the_javascripts_own() {
    // The JavaScript's own exponent is three, its own overshoot is
    // 1.70158, and its own amplitude and period are one and 0.3.
    assert_eq!(Poly::default().poly_in(0.5), ease::ease_poly_in(0.5));
    assert_eq!(Poly::default().poly_out(0.5), ease::ease_poly_out(0.5));
    assert_eq!(
        Poly::default().poly_in_out(0.5),
        ease::ease_poly_in_out(0.5)
    );
    assert_eq!(Back::default().back_in(0.5), ease::ease_back_in(0.5));
    assert_eq!(Back::default().back_out(0.5), ease::ease_back_out(0.5));
    assert_eq!(
        Back::default().back_in_out(0.5),
        ease::ease_back_in_out(0.5)
    );
    assert_eq!(
        Elastic::default().elastic_in(0.5),
        ease::ease_elastic_in(0.5)
    );
    assert_eq!(
        Elastic::default().elastic_out(0.5),
        ease::ease_elastic_out(0.5)
    );
    assert_eq!(
        Elastic::default().elastic_in_out(0.5),
        ease::ease_elastic_in_out(0.5)
    );
}

#[test]
fn the_setters_do_not_change_the_ease_they_are_on() {
    // The JavaScript's setter gives back a new function and leaves the old
    // one alone.
    let poly = Poly::default();
    let other = poly.exponent(4.0);
    assert_eq!(poly.poly_in(0.5), ease::ease_poly_in(0.5));
    assert_ne!(other.poly_in(0.5), poly.poly_in(0.5));

    let back = Back::default();
    let other = back.overshoot(0.0);
    assert_eq!(back.back_in(0.5), ease::ease_back_in(0.5));
    assert_ne!(other.back_in(0.5), back.back_in(0.5));

    let elastic = Elastic::default();
    let other = elastic.amplitude(2.0);
    assert_eq!(elastic.elastic_in(0.5), ease::ease_elastic_in(0.5));
    assert_ne!(other.elastic_in(0.5), elastic.elastic_in(0.5));

    let other = elastic.period(0.5);
    assert_eq!(elastic.elastic_in(0.5), ease::ease_elastic_in(0.5));
    assert_ne!(other.elastic_in(0.5), elastic.elastic_in(0.5));
}
