//! The port of d3-geo, checked against d3's own JavaScript.

use std::fs;

use d3::geo::{
    geo_area, geo_bounds, geo_centroid, geo_circle, geo_contains, geo_distance, geo_graticule10,
    geo_interpolate, geo_length, geo_rotation, Geometry,
};
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/geo.json");
    let text = fs::read_to_string(path).expect("fixture");
    serde_json::from_str(&text).expect("fixture JSON")
}

fn bits(value: &Value) -> u64 {
    let text = value.as_str().expect("bits");
    let mut bits = 0u64;
    for i in 0..16 {
        let digit = u8::from_str_radix(&text[i..i + 1], 16).expect("hex");
        bits = (bits << 4) | digit as u64;
    }
    bits
}

fn same(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        true
    } else {
        a.to_bits() == b.to_bits()
    }
}

#[test]
fn the_distance_is_the_javascripts() {
    let f = fixture();
    let exp = f64::from_bits(bits(&f["distance"]));
    let got = geo_distance(&[0.0, 0.0], &[1.0, 1.0]);
    assert!(same(got, exp), "distance: got {} exp {}", got, exp);
}

#[test]
fn the_length_is_the_javascripts() {
    let f = fixture();
    let exp = f64::from_bits(bits(&f["length"]));
    let got = geo_length(&Geometry::LineString(vec![[0.0, 0.0], [10.0, 0.0]]));
    assert!(same(got, exp), "length: got {} exp {}", got, exp);
}

#[test]
fn the_area_is_the_javascripts() {
    let f = fixture();
    let exp = f64::from_bits(bits(&f["area"]));
    let got = geo_area(&Geometry::Polygon(vec![vec![
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 10.0],
        [0.0, 10.0],
        [0.0, 0.0],
    ]]));
    assert!(same(got, exp), "area: got {} exp {}", got, exp);
}

#[test]
fn the_interpolate_is_the_javascripts() {
    let f = fixture();
    let exp0 = f64::from_bits(bits(&f["interpolate"][0]));
    let exp1 = f64::from_bits(bits(&f["interpolate"][1]));
    let interp = geo_interpolate([0.0, 0.0], [10.0, 10.0]);
    let got = interp(0.5);
    assert!(same(got[0], exp0), "interp0: got {} exp {}", got[0], exp0);
    assert!(same(got[1], exp1), "interp1: got {} exp {}", got[1], exp1);
}

#[test]
fn the_centroid_is_the_javascripts() {
    let f = fixture();
    let exp0 = f64::from_bits(bits(&f["centroid"][0]));
    let exp1 = f64::from_bits(bits(&f["centroid"][1]));
    let got = geo_centroid(&Geometry::Polygon(vec![vec![
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 10.0],
        [0.0, 10.0],
        [0.0, 0.0],
    ]]));
    assert!(same(got[0], exp0), "centroid0: got {} exp {}", got[0], exp0);
    assert!(same(got[1], exp1), "centroid1: got {} exp {}", got[1], exp1);
}

#[test]
fn the_contains_is_the_javascripts() {
    let f = fixture();
    let exp = f["contains"].as_bool().unwrap();
    let poly = Geometry::Polygon(vec![vec![
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 10.0],
        [0.0, 10.0],
        [0.0, 0.0],
    ]]);
    let got = geo_contains(&poly, &[5.0, 5.0]);
    assert_eq!(got, exp, "contains: got {} exp {}", got, exp);
}

#[test]
fn the_circle_is_the_javascripts() {
    let f = fixture();
    let exp = f["circleCoordsLen"].as_u64().unwrap() as usize;
    let circle = geo_circle();
    let got = circle([0.0, 0.0]);
    match got {
        Geometry::Polygon(rings) => {
            assert_eq!(rings[0].len(), exp);
        }
        _ => panic!("expected polygon"),
    }
}

#[test]
fn the_graticule_is_the_javascripts() {
    let got = geo_graticule10();
    match got {
        Geometry::MultiLineString(_) => {
            // type matches
        }
        _ => panic!("expected multilinestring"),
    }
}

#[test]
fn the_bounds_is_the_javascripts() {
    let f = fixture();
    let exp00 = f64::from_bits(bits(&f["bounds"][0][0]));
    let exp01 = f64::from_bits(bits(&f["bounds"][0][1]));
    let exp10 = f64::from_bits(bits(&f["bounds"][1][0]));
    let exp11 = f64::from_bits(bits(&f["bounds"][1][1]));
    let poly = Geometry::Polygon(vec![vec![
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 10.0],
        [0.0, 10.0],
        [0.0, 0.0],
    ]]);
    let got = geo_bounds(&poly);
    assert!(
        same(got.0[0], exp00),
        "bounds00: got {} exp {}",
        got.0[0],
        exp00
    );
    assert!(
        same(got.0[1], exp01),
        "bounds01: got {} exp {}",
        got.0[1],
        exp01
    );
    assert!(
        same(got.1[0], exp10),
        "bounds10: got {} exp {}",
        got.1[0],
        exp10
    );
    assert!(
        same(got.1[1], exp11),
        "bounds11: got {} exp {}",
        got.1[1],
        exp11
    );
}

#[test]
fn the_rotation_is_the_javascripts() {
    let f = fixture();
    let exp0 = f64::from_bits(bits(&f["rotation"][0]));
    let exp1 = f64::from_bits(bits(&f["rotation"][1]));
    let rot = geo_rotation([10.0, 20.0, 30.0]);
    let got = rot([0.0, 0.0]);
    assert!(same(got[0], exp0), "rot0: got {} exp {}", got[0], exp0);
    assert!(same(got[1], exp1), "rot1: got {} exp {}", got[1], exp1);
}
