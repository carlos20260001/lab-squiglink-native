//! The port of d3-scale, checked against d3's own JavaScript.

use std::fs;

use d3::scale::{
    scale_band, scale_diverging, scale_identity, scale_linear, scale_log, scale_ordinal,
    scale_point, scale_pow, scale_quantile, scale_quantize, scale_sequential, scale_sequential_log,
    scale_sequential_quantile, scale_sqrt, scale_symlog, scale_threshold, scale_time, tick_format,
    Implicit, OrdinalUnknown,
};
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/scale.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

fn bits(value: &Value) -> u64 {
    let text = value.as_str().expect("the bits of a number");
    let mut bits = 0u64;
    for i in 0..16 {
        let digit = u8::from_str_radix(&text[i..i + 1], 16).expect("a hex digit");
        bits = (bits << 4) | digit as u64;
    }
    bits
}

fn number(value: &Value) -> f64 {
    f64::from_bits(bits(value))
}

fn same(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        true
    } else {
        a.to_bits() == b.to_bits()
    }
}

#[test]
fn linear() {
    let f = fixture();
    let e = &f["linear"];
    let mut s = scale_linear();
    s.set_domain(&[0.0, 10.0]);
    s.set_range(&[0.0, 1.0]);
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    let samples = [
        0.0,
        0.25,
        0.5,
        0.75,
        1.0,
        2.0,
        5.0,
        10.0,
        -1.0,
        -0.5,
        f64::NAN,
    ];
    for (got, exp) in samples.iter().zip(exp_call.iter()) {
        assert!(
            same(s.call(*got), *exp),
            "linear call {} vs {}",
            s.call(*got),
            exp
        );
    }
    assert_eq!(
        s.domain(),
        vec![number(&e["domain"][0]), number(&e["domain"][1])]
    );
    assert_eq!(
        s.range(),
        vec![number(&e["range"][0]), number(&e["range"][1])]
    );
    for (i, v) in [0.0, 0.5, 1.0].iter().enumerate() {
        assert!(same(s.invert(*v), number(&e["invert"][i])));
    }
    let ticks: Vec<f64> = e["ticks"].as_array().unwrap().iter().map(number).collect();
    assert_eq!(s.ticks(Some(5.0)).len(), ticks.len());
    for (a, b) in s.ticks(Some(5.0)).iter().zip(ticks.iter()) {
        assert!(same(*a, *b));
    }
}

#[test]
fn linear_clamp() {
    let f = fixture();
    let e = &f["linearClamp"];
    let mut s = scale_linear();
    s.set_domain(&[0.0, 10.0]);
    s.set_range(&[0.0, 1.0]);
    s.set_clamp(true);
    let got: Vec<f64> = [-5.0, 5.0, 15.0].iter().map(|x| s.call(*x)).collect();
    let exp: Vec<f64> = e.as_array().unwrap().iter().map(number).collect();
    for (a, b) in got.iter().zip(exp.iter()) {
        assert!(same(*a, *b));
    }
}

#[test]
fn identity() {
    let f = fixture();
    let e = &f["identity"];
    let mut s = scale_identity();
    s.set_domain(&[0.0, 10.0]);
    let samples = [
        0.0,
        0.25,
        0.5,
        0.75,
        1.0,
        2.0,
        5.0,
        10.0,
        -1.0,
        -0.5,
        f64::NAN,
    ];
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    for (got, exp) in samples.iter().zip(exp_call.iter()) {
        assert!(same(s.call(*got), *exp));
    }
}

#[test]
fn log_scale() {
    let f = fixture();
    let e = &f["log"];
    let mut s = scale_log();
    s.set_domain(&[1.0, 100.0]);
    s.set_range(&[0.0, 1.0]);
    assert!(same(s.base(), number(&e["base"])));
    let samples = [1.0, 2.0, 10.0, 50.0, 100.0];
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    for (got, exp) in samples.iter().zip(exp_call.iter()) {
        eprintln!("got {} exp {}", s.call(*got), *exp);
        assert!(same(s.call(*got), *exp));
    }
}

#[test]
fn symlog_scale() {
    let f = fixture();
    let e = &f["symlog"];
    let mut s = scale_symlog();
    s.set_constant(number(&e["constant"]));
    s.set_domain(&[-10.0, 10.0]);
    s.set_range(&[0.0, 1.0]);
    assert!(same(s.constant(), number(&e["constant"])));
    let samples = [-10.0, -5.0, 0.0, 5.0, 10.0];
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    for (got, exp) in samples.iter().zip(exp_call.iter()) {
        assert!(same(s.call(*got), *exp));
    }
}

#[test]
fn pow_scale() {
    let f = fixture();
    let e = &f["pow"];
    let mut s = scale_pow();
    s.set_exponent(number(&e["exponent"]));
    s.set_domain(&[0.0, 10.0]);
    s.set_range(&[0.0, 1.0]);
    assert!(same(s.exponent(), number(&e["exponent"])));
    let samples = [0.0, 2.0, 5.0, 10.0];
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    for (got, exp) in samples.iter().zip(exp_call.iter()) {
        assert!(same(s.call(*got), *exp));
    }
    // sqrt
    let mut sq = scale_sqrt();
    sq.set_domain(&[0.0, 10.0]);
    sq.set_range(&[0.0, 1.0]);
    let exp_sqrt: Vec<f64> = e["sqrt"].as_array().unwrap().iter().map(number).collect();
    let samples_sqrt = [0.0, 4.0, 9.0];
    for (got, exp) in samples_sqrt.iter().zip(exp_sqrt.iter()) {
        assert!(same(sq.call(*got), *exp));
    }
}

#[test]
fn band_scale() {
    let f = fixture();
    let e = &f["band"];
    let mut s = scale_band();
    s.set_domain(&[0.0, 1.0, 2.0]);
    s.set_range(&[0.0, 300.0]);
    assert!(same(s.bandwidth(), number(&e["bandwidth"])));
    assert!(same(s.step(), number(&e["step"])));
    let exp_call: Vec<String> = e["call"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    // Rust returns NaN for unknown; for known returns number
    for (i, x) in [0.0, 1.0, 2.0].iter().enumerate() {
        let got = s.call(*x);
        if exp_call[i] == "undef" {
            assert!(got.is_nan());
        } else {
            assert!(same(
                got,
                number(&serde_json::from_str::<Value>(&format!("\"{}\"", exp_call[i])).unwrap())
            ));
        }
    }
}

#[test]
fn point_scale() {
    let f = fixture();
    let e = &f["point"];
    let mut s = scale_point();
    s.set_domain(&[0.0, 1.0, 2.0]);
    s.set_range(&[0.0, 300.0]);
    assert!(same(s.bandwidth(), number(&e["bandwidth"])));
    assert!(same(s.step(), number(&e["step"])));
}

#[test]
fn ordinal_scale() {
    let f = fixture();
    let e = &f["ordinal"];
    let mut s = scale_ordinal();
    s.set_domain(&[0.0, 1.0]);
    s.set_range(&[1.0, 2.0]);
    // trigger implicit
    s.call(2.0);
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    for (i, x) in [0.0, 1.0, 2.0, 3.0].iter().enumerate() {
        assert!(same(s.call(*x), exp_call[i]));
    }
    match s.unknown() {
        OrdinalUnknown::Implicit(Implicit { name }) => assert_eq!(name, "implicit"),
        _ => panic!("expected implicit"),
    }
}

#[test]
fn ordinal_unknown() {
    let f = fixture();
    let e = &f["ordinalUnknown"];
    let mut s = scale_ordinal();
    s.set_domain(&[0.0]);
    s.set_range(&[10.0]);
    s.set_unknown(OrdinalUnknown::Value(5.0));
    assert!(same(s.call(99.0), number(&e["call"])));
}

#[test]
fn quantile_scale() {
    let f = fixture();
    let e = &f["quantile"];
    let mut s = scale_quantile();
    s.set_domain(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    s.set_range(&[10.0, 20.0, 30.0]);
    let samples = [1.0, 3.5, 6.0];
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    for (got, exp) in samples.iter().zip(exp_call.iter()) {
        assert!(same(s.call(*got), *exp));
    }
    let exp_q: Vec<f64> = e["quantiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(number)
        .collect();
    let got_q = s.quantiles();
    assert_eq!(got_q.len(), exp_q.len());
    for (a, b) in got_q.iter().zip(exp_q.iter()) {
        assert!(same(*a, *b));
    }
}

#[test]
fn quantize_scale() {
    let f = fixture();
    let e = &f["quantize"];
    let mut s = scale_quantize();
    s.set_domain(&[0.0, 10.0]);
    s.set_range(&[1.0, 2.0, 3.0]);
    let samples = [0.0, 3.0, 7.0, 10.0];
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    for (got, exp) in samples.iter().zip(exp_call.iter()) {
        assert!(same(s.call(*got), *exp));
    }
    let exp_t: Vec<f64> = e["thresholds"]
        .as_array()
        .unwrap()
        .iter()
        .map(number)
        .collect();
    let got_t = s.thresholds();
    assert_eq!(got_t.len(), exp_t.len());
    for (a, b) in got_t.iter().zip(exp_t.iter()) {
        assert!(same(*a, *b));
    }
}

#[test]
fn threshold_scale() {
    let f = fixture();
    let e = &f["threshold"];
    let mut s = scale_threshold();
    s.set_domain(&[0.0, 5.0]);
    s.set_range(&[1.0, 2.0, 3.0]);
    let samples = [-1.0, 0.0, 3.0, 5.0, 10.0];
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    for (got, exp) in samples.iter().zip(exp_call.iter()) {
        assert!(same(s.call(*got), *exp));
    }
}

#[test]
fn time_scale() {
    let f = fixture();
    let e = &f["time"];
    let mut s = scale_time();
    s.set_domain(&[946684800000.0, 946771200000.0]);
    let t = 946728000000.0; // noon
    assert!(same(s.call(t), number(&e["call"])));
}

#[test]
fn sequential_scale() {
    let f = fixture();
    let e = &f["sequential"];
    let mut s = scale_sequential();
    s.set_domain(&[0.0, 10.0]);
    let samples = [0.0, 5.0, 10.0];
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    for (got, exp) in samples.iter().zip(exp_call.iter()) {
        assert!(same(s.call(*got), *exp));
    }
}

#[test]
fn sequential_log_scale() {
    let f = fixture();
    let e = &f["sequentialLog"];
    let mut s = scale_sequential_log();
    s.set_domain(&[1.0, 100.0]);
    let samples = [1.0, 10.0, 100.0];
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    for (got, exp) in samples.iter().zip(exp_call.iter()) {
        assert!(same(s.call(*got), *exp));
    }
}

#[test]
fn sequential_quantile_scale() {
    let f = fixture();
    let e = &f["sequentialQuantile"];
    let mut s = scale_sequential_quantile();
    s.set_interpolator(std::sync::Arc::new(|d: f64| d * 2.0));
    s.set_domain(&[1.0, 2.0, 3.0, 4.0]);
    let samples = [1.0, 2.5, 4.0];
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    for (got, exp) in samples.iter().zip(exp_call.iter()) {
        eprintln!("got {} exp {}", s.call(*got), *exp);
        assert!(same(s.call(*got), *exp));
    }
}

#[test]
fn diverging_scale() {
    let f = fixture();
    let e = &f["diverging"];
    let mut s = scale_diverging();
    s.set_domain(&[0.0, 0.5, 1.0]);
    let samples = [0.0, 0.25, 0.5, 0.75, 1.0];
    let exp_call: Vec<f64> = e["call"].as_array().unwrap().iter().map(number).collect();
    for (got, exp) in samples.iter().zip(exp_call.iter()) {
        assert!(same(s.call(*got), *exp));
    }
}

#[test]
#[allow(clippy::approx_constant)]
fn tick_format_fn() {
    let f = fixture();
    let e = &f["tickFormat"];
    let tf = tick_format(0.0, 10.0, Some(5.0), Some(",.1f"));
    assert_eq!(tf(3.14159), e["basic"].as_str().unwrap());
}
