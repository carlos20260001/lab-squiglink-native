//! The port of d3-polygon, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-polygon 1.0.6
//! under Node — the same code the browser runs — and writing down what it
//! produced. The Rust here must give the same answers.

use std::fs;

use d3::polygon::{
    polygon_area, polygon_centroid, polygon_contains, polygon_hull, polygon_length, Point,
    PolygonError,
};
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/polygon.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The number a fixture holds, which is written down as its bits.
///
/// The fixture writes the numbers as their bits, because a number written
/// in text and read back may not be the same number: the reader may round
/// it to the nearest one, and the nearest one is not always the one that
/// was written. The bits also hold the numbers that JSON cannot hold — the
/// `NaN` of a centroid with no area, and the sign of a zero.
fn bits(value: &Value) -> u64 {
    let text = value.as_str().expect("the bits of a number");
    // The bits are written down as they are in the number, most
    // significant first, and the port reads them the same way.
    let mut bits = 0u64;
    for i in 0..16 {
        let digit = u8::from_str_radix(&text[i..i + 1], 16).expect("a hex digit");
        bits = (bits << 4) | digit as u64;
    }
    bits
}

/// Whether two numbers are the same.
///
/// Two that are not numbers are the same when both of them are not, for
/// the sign of a `NaN` is the machine's own and the JavaScript's is not
/// the port's to keep.
fn same_number(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        true
    } else {
        a.to_bits() == b.to_bits()
    }
}

/// Whether a point is the point a fixture holds.
fn same_point(point: Point, value: &Value) -> bool {
    let list = value.as_array().expect("a point");
    list.len() == 2
        && same_number(point[0], f64::from_bits(bits(&list[0])))
        && same_number(point[1], f64::from_bits(bits(&list[1])))
}

/// Whether a list of points is the list a fixture holds.
fn same_points(points: &[Point], value: &Value) -> bool {
    let list = value.as_array().expect("a list of points");
    points.len() == list.len()
        && points
            .iter()
            .zip(list.iter())
            .all(|(point, value)| same_point(*point, value))
}

/// Whether the hull the port gives is the hull the fixture holds.
fn same_hull(hull: Option<Vec<Point>>, value: &Value) -> bool {
    match (hull, value) {
        (None, Value::Null) => true,
        (Some(hull), list) => same_points(&hull, list),
        _ => false,
    }
}

#[test]
fn the_hulls_are_the_same() {
    let f = fixture();
    let hulls = &f["hull"];

    // A list of fewer than three points has no hull at all.
    assert_eq!(polygon_hull(&[]), None);
    assert_eq!(polygon_hull(&[[0.0, 1.0]]), None);
    assert_eq!(polygon_hull(&[[0.0, 1.0], [1.0, 0.0]]), None);
    assert!(same_hull(None, &hulls["empty"]));
    assert!(same_hull(None, &hulls["one"]));
    assert!(same_hull(None, &hulls["two"]));

    // A triangle, and a triangle with a point inside it.
    assert!(same_hull(
        polygon_hull(&[[200.0, 200.0], [760.0, 300.0], [500.0, 500.0]]),
        &hulls["triangle"]
    ));
    assert!(same_hull(
        polygon_hull(&[
            [200.0, 200.0],
            [760.0, 300.0],
            [500.0, 500.0],
            [400.0, 400.0]
        ]),
        &hulls["triangleWithInner"]
    ));

    // The points that share an ordinate, and the ones whose upper and
    // lower sides overlap.
    assert!(same_hull(
        polygon_hull(&[[-10.0, -10.0], [10.0, 10.0], [10.0, -10.0], [-10.0, 10.0]]),
        &hulls["duplicateOrdinates"]
    ));
    assert!(same_hull(
        polygon_hull(&[
            [0.0, -10.0],
            [0.0, 10.0],
            [0.0, 0.0],
            [10.0, 0.0],
            [-10.0, 0.0]
        ]),
        &hulls["overlapping"]
    ));

    // The three hulls of the JavaScript's own tests, which are taken from
    // the problem 681 of the Online Judge.
    assert!(same_hull(
        polygon_hull(&[
            [60.0, 20.0],
            [250.0, 140.0],
            [180.0, 170.0],
            [79.0, 140.0],
            [50.0, 60.0],
            [60.0, 20.0]
        ]),
        &hulls["uvaFirst"]
    ));
    assert!(same_hull(
        polygon_hull(&[
            [50.0, 60.0],
            [60.0, 20.0],
            [70.0, 45.0],
            [100.0, 70.0],
            [125.0, 90.0],
            [200.0, 113.0],
            [250.0, 140.0],
            [180.0, 170.0],
            [105.0, 140.0],
            [79.0, 140.0],
            [60.0, 85.0],
            [50.0, 60.0]
        ]),
        &hulls["uvaSecond"]
    ));
    assert!(same_hull(
        polygon_hull(&[
            [30.0, 30.0],
            [50.0, 60.0],
            [60.0, 20.0],
            [70.0, 45.0],
            [86.0, 39.0],
            [112.0, 60.0],
            [200.0, 113.0],
            [250.0, 50.0],
            [300.0, 200.0],
            [130.0, 240.0],
            [76.0, 150.0],
            [47.0, 76.0],
            [36.0, 40.0],
            [33.0, 35.0],
            [30.0, 30.0]
        ]),
        &hulls["uvaThird"]
    ));

    // The points that are equal to one another. The JavaScript keeps one
    // of them, and which one it keeps depends on the order they came in.
    assert!(same_hull(
        polygon_hull(&[[0.0, 0.0], [0.0, 0.0], [1.0, 1.0]]),
        &hulls["equalPoints"]
    ));
    assert!(same_hull(
        polygon_hull(&[[1.0, 1.0], [1.0, 1.0], [1.0, 1.0]]),
        &hulls["equalPointsThree"]
    ));
    assert!(same_hull(
        polygon_hull(&[[0.0, 0.0], [0.0, 0.0], [0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]),
        &hulls["equalPointsMixed"]
    ));
    assert!(same_hull(
        polygon_hull(&[[0.0, 0.0], [1.0, 1.0], [0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]),
        &hulls["equalPointsApart"]
    ));

    // The points that are in a line, which are not on the hull, for only
    // the ends of the line are.
    assert!(same_hull(
        polygon_hull(&[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]),
        &hulls["collinear"]
    ));
    assert!(same_hull(
        polygon_hull(&[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0], [3.0, 3.0], [4.0, 4.0]]),
        &hulls["collinearWithOne"]
    ));

    // The points that share an x, which the JavaScript sorts by the y.
    assert!(same_hull(
        polygon_hull(&[[0.0, 0.0], [0.0, 1.0], [0.0, 2.0], [1.0, 1.0]]),
        &hulls["sameX"]
    ));
    assert!(same_hull(
        polygon_hull(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
        &hulls["sameXAndY"]
    ));

    // The negative zero, which the JavaScript's own coercion keeps.
    assert!(same_hull(
        polygon_hull(&[[-0.0, 0.0], [0.0, 0.0], [1.0, 1.0]]),
        &hulls["signedZero"]
    ));

    // A point that is not a number, which the JavaScript's own sort puts
    // where its comparison cannot say.
    assert!(same_hull(
        polygon_hull(&[[0.0, 0.0], [f64::NAN, 1.0], [1.0, 1.0], [2.0, 2.0]]),
        &hulls["withNaN"]
    ));

    // A ring of points, which is the hull of itself.
    let ring: Vec<Point> = (0..16)
        .map(|i| {
            let a = (i as f64 / 16.0) * 2.0 * std::f64::consts::PI;
            [a.cos(), a.sin()]
        })
        .collect();
    assert!(same_hull(polygon_hull(&ring), &hulls["ring"]));
}

#[test]
fn the_areas_are_the_same() {
    let f = fixture();
    let areas = &f["area"];

    let same = |value: f64, key: &str| {
        assert!(
            same_number(value, f64::from_bits(bits(&areas[key]))),
            "the area of {key}"
        );
    };

    // A polygon with no points in it has no area.
    same(polygon_area(&[]), "empty");
    same(polygon_area(&[[0.0, 0.0]]), "one");
    same(polygon_area(&[[0.0, 0.0], [1.0, 0.0]]), "two");

    // The area is negative for a polygon that goes anticlockwise.
    same(
        polygon_area(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
        "counterclockwise",
    );
    same(
        polygon_area(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]),
        "clockwise",
    );
    same(
        polygon_area(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]),
        "openCounterclockwise",
    );
    same(
        polygon_area(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        "openClockwise",
    );
    same(
        polygon_area(&[[1.0, 1.0], [3.0, 2.0], [2.0, 3.0]]),
        "triangle",
    );

    // A point that is not a number makes the area not a number too.
    same(
        polygon_area(&[[0.0, 0.0], [f64::NAN, 1.0], [1.0, 1.0]]),
        "withNaN",
    );

    // A polygon so large that the sum of its parts is not the whole of it.
    let large = large_polygon();
    same(polygon_area(&large), "large");
}

#[test]
fn the_centroids_are_the_same() {
    let f = fixture();
    let centroids = &f["centroid"];

    let same = |value: Point, key: &str| {
        assert!(same_point(value, &centroids[key]), "the centroid of {key}");
    };

    // A polygon with no points in it has no centre either.
    same(polygon_centroid(&[]), "empty");

    same(
        polygon_centroid(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
        "counterclockwise",
    );
    same(
        polygon_centroid(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]),
        "clockwise",
    );
    same(
        polygon_centroid(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]),
        "openCounterclockwise",
    );
    same(
        polygon_centroid(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        "openClockwise",
    );
    same(
        polygon_centroid(&[[1.0, 1.0], [3.0, 2.0], [2.0, 3.0]]),
        "triangle",
    );

    // A polygon with no area has no centre either.
    same(
        polygon_centroid(&[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]),
        "collinear",
    );

    // A polygon so large that the sum of its parts is not the whole of it.
    same(polygon_centroid(&large_polygon()), "large");
}

#[test]
fn the_containments_are_the_same() {
    let f = fixture();
    let contains = &f["contains"];

    let counterclockwise = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    let clockwise = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]];

    let same = |value: bool, key: &str| {
        assert_eq!(
            value,
            contains[key].as_bool().expect("a truth"),
            "for {key}"
        );
    };

    same(
        polygon_contains(&counterclockwise, [0.5, 0.5]).expect("a polygon"),
        "inside",
    );
    same(
        polygon_contains(&counterclockwise, [1.5, 0.5]).expect("a polygon"),
        "outsideRight",
    );
    same(
        polygon_contains(&counterclockwise, [-0.5, 0.5]).expect("a polygon"),
        "outsideLeft",
    );
    same(
        polygon_contains(&counterclockwise, [0.5, 1.5]).expect("a polygon"),
        "outsideTop",
    );
    same(
        polygon_contains(&counterclockwise, [0.5, -0.5]).expect("a polygon"),
        "outsideBottom",
    );
    same(
        polygon_contains(&counterclockwise, [0.0, 0.0]).expect("a polygon"),
        "onAVertex",
    );
    same(
        polygon_contains(&counterclockwise, [0.5, 0.0]).expect("a polygon"),
        "onAnEdge",
    );
    same(
        polygon_contains(&clockwise, [0.5, 0.5]).expect("a polygon"),
        "clockwiseInside",
    );
    same(
        polygon_contains(&[[1.0, 1.0], [3.0, 2.0], [2.0, 3.0]], [1.5, 1.5]).expect("a polygon"),
        "triangleInside",
    );
    same(
        polygon_contains(&[[1.0, 1.0], [3.0, 2.0], [2.0, 3.0]], [0.5, 0.5]).expect("a polygon"),
        "triangleOutside",
    );
}

#[test]
fn the_perimeters_are_the_same() {
    let f = fixture();
    let lengths = &f["length"];

    let same = |value: f64, key: &str| {
        assert!(
            same_number(value, f64::from_bits(bits(&lengths[key]))),
            "the perimeter of {key}"
        );
    };

    same(polygon_length(&[[0.0, 0.0]]).expect("a polygon"), "one");
    same(
        polygon_length(&[[0.0, 0.0], [3.0, 4.0]]).expect("a polygon"),
        "two",
    );
    same(
        polygon_length(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]])
            .expect("a polygon"),
        "counterclockwise",
    );
    same(
        polygon_length(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]])
            .expect("a polygon"),
        "clockwise",
    );
    same(
        polygon_length(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]).expect("a polygon"),
        "openCounterclockwise",
    );
    same(
        polygon_length(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]).expect("a polygon"),
        "openClockwise",
    );
    same(
        polygon_length(&[[1.0, 1.0], [3.0, 2.0], [2.0, 3.0]]).expect("a polygon"),
        "triangle",
    );
}

#[test]
fn a_polygon_with_no_points_is_an_error_as_in_the_javascript() {
    let f = fixture();

    // The JavaScript reads the last point of the polygon before it reads
    // any other one, and throws when there is none.
    let error = polygon_length(&[]).unwrap_err();
    assert_eq!(error, PolygonError::EmptyPolygon);
    assert_eq!(
        error.to_string(),
        "cannot read properties of undefined (reading '0')"
    );
    assert_eq!(
        polygon_contains(&[], [0.0, 0.0]).unwrap_err(),
        PolygonError::EmptyPolygon
    );

    // And the fixture holds what the JavaScript threw.
    let thrown = &f["length"]["empty"];
    assert_eq!(
        thrown["error"].as_str(),
        Some("TypeError: Cannot read properties of undefined (reading '0')")
    );
}

/// A polygon so large that the sum of its parts is not the whole of it.
fn large_polygon() -> Vec<Point> {
    let stop = 1e8;
    let step = 1e4;
    let mut points = Vec::new();
    let mut value = 0.0;
    while value < stop {
        points.push([0.0, value]);
        value += step;
    }
    value = 0.0;
    while value < stop {
        points.push([value, stop]);
        value += step;
    }
    value = stop - step;
    while value >= 0.0 {
        points.push([stop, value]);
        value -= step;
    }
    value = stop - step;
    while value >= 0.0 {
        points.push([value, 0.0]);
        value -= step;
    }
    points
}
