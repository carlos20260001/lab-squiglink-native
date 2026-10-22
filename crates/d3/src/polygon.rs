//! A port of **d3-polygon**, from the JavaScript of the same name.
//!
//! Translated from `d3-polygon/src/*.js`, version 1.0.6, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read differently,
//! the JavaScript is right.
//!
//! d3-polygon is the geometry of a polygon: the hull of a list of points,
//! the area a polygon covers, the point at its centre, the perimeter around
//! it, and whether a point is inside it.
//!
//! # The hull
//!
//! The hull is the monotone chain of Andrew. The points are sorted by the
//! x and then by the y, and the chain is walked twice: once over the points
//! as they are, which gives the upper side of the hull, and once over them
//! upside down, which gives the lower side. The two sides are then joined,
//! and a point that is on both of them is written down once.
//!
//! The points that are equal to one another are a part of this: the
//! JavaScript keeps one of them — which one depends on the order the
//! points came in — and the port keeps the same one.

use std::cmp::Ordering;

use crate::ieee754;

/// A point of a polygon: the x and the y.
pub type Point = [f64; 2];

/// d3-polygon's `polygonHull`.
///
/// The JavaScript:
///
/// ```js
/// export default function(points) {
///   if ((n = points.length) < 3) return null;
///   ...
/// }
/// ```
///
/// The port gives `None` where the JavaScript gives `null`, which is what
/// it gives for a list of fewer than three points.
///
/// The hull comes back in the JavaScript's own order: the upper side of
/// the hull from the right to the left, and then the lower side from the
/// left to the right.
pub fn polygon_hull(points: &[Point]) -> Option<Vec<Point>> {
    let n = points.len();
    if n < 3 {
        return None;
    }

    // The points with the place they had in the list, sorted the way the
    // JavaScript sorts them.
    let mut sorted: Vec<SortedPoint> = points
        .iter()
        .enumerate()
        .map(|(place, p)| SortedPoint {
            x: p[0],
            y: p[1],
            place,
        })
        .collect();
    sorted.sort_by(lexicographic_order);

    // The same points upside down, for the hull of these is the lower
    // side of the hull of those.
    let flipped: Vec<(f64, f64)> = sorted.iter().map(|p| (p.x, -p.y)).collect();
    let upper_points: Vec<(f64, f64)> = sorted.iter().map(|p| (p.x, p.y)).collect();

    let upper = upper_hull_indexes(&upper_points);
    let lower = upper_hull_indexes(&flipped);

    // The hull, without the points that are on both of its sides.
    let skip_left = lower[0] == upper[0];
    let skip_right = lower[lower.len() - 1] == upper[upper.len() - 1];

    let mut hull = Vec::with_capacity(upper.len() + lower.len());

    // The upper side, from the right to the left.
    for &index in upper.iter().rev() {
        hull.push(points[sorted[index].place]);
    }

    // And then the lower side, from the left to the right.
    let first = if skip_left { 1 } else { 0 };
    let last = lower.len() - usize::from(skip_right);
    for &index in lower.iter().skip(first).take(last.saturating_sub(first)) {
        hull.push(points[sorted[index].place]);
    }

    Some(hull)
}

/// d3-polygon's `polygonArea`.
///
/// The JavaScript:
///
/// ```js
/// export default function(polygon) {
///   var i = -1, n = polygon.length, a, b = polygon[n - 1], area = 0;
///   while (++i < n) {
///     a = b;
///     b = polygon[i];
///     area += a[1] * b[0] - a[0] * b[1];
///   }
///   return area / 2;
/// }
/// ```
///
/// The area keeps the JavaScript's own sign: a polygon that goes
/// anticlockwise on the screen has a positive area, and one that goes
/// clockwise has a negative one. The port does not change the sign.
pub fn polygon_area(polygon: &[Point]) -> f64 {
    let n = polygon.len();
    if n == 0 {
        return 0.0;
    }
    let mut area = 0.0;
    let mut b = polygon[n - 1];
    for &point in polygon {
        let a = b;
        b = point;
        area += a[1] * b[0] - a[0] * b[1];
    }
    area / 2.0
}

/// d3-polygon's `polygonCentroid`.
///
/// The JavaScript:
///
/// ```js
/// export default function(polygon) {
///   var i = -1, n = polygon.length, x = 0, y = 0, a, b = polygon[n - 1], c, k = 0;
///   while (++i < n) {
///     a = b;
///     b = polygon[i];
///     k += c = a[0] * b[1] - b[0] * a[1];
///     x += (a[0] + b[0]) * c;
///     y += (a[1] + b[1]) * c;
///   }
///   return k *= 3, [x / k, y / k];
/// }
/// ```
///
/// A polygon with no area has no centre either, and the JavaScript divides
/// by zero for it, so the port gives `NaN` for both of the numbers.
pub fn polygon_centroid(polygon: &[Point]) -> Point {
    let n = polygon.len();
    let mut x = 0.0;
    let mut y = 0.0;
    let mut k = 0.0;
    if n > 0 {
        let mut b = polygon[n - 1];
        for &point in polygon {
            let a = b;
            b = point;
            let c = a[0] * b[1] - b[0] * a[1];
            k += c;
            x += (a[0] + b[0]) * c;
            y += (a[1] + b[1]) * c;
        }
    }
    k *= 3.0;
    [x / k, y / k]
}

/// d3-polygon's `polygonContains`.
///
/// The JavaScript:
///
/// ```js
/// export default function(polygon, point) {
///   var n = polygon.length, p = polygon[n - 1], x = point[0], y = point[1],
///       x0 = p[0], y0 = p[1], x1, y1, inside = false;
///   for (var i = 0; i < n; ++i) {
///     p = polygon[i], x1 = p[0], y1 = p[1];
///     if (((y1 > y) !== (y0 > y)) && (x < (x0 - x1) * (y - y1) / (y0 - y1) + x1)) inside = !inside;
///     x0 = x1, y0 = y1;
///   }
///   return inside;
/// }
/// ```
///
/// The JavaScript reads the last point of the polygon before it reads any
/// other one, and for a polygon with no points in it there is none to read,
/// so it throws; the port returns [`PolygonError::EmptyPolygon`].
pub fn polygon_contains(polygon: &[Point], point: Point) -> Result<bool, PolygonError> {
    let n = polygon.len();
    if n == 0 {
        return Err(PolygonError::EmptyPolygon);
    }
    let (x, y) = (point[0], point[1]);
    let p = polygon[n - 1];
    let (mut x0, mut y0) = (p[0], p[1]);
    let mut inside = false;
    for &current in polygon {
        let (x1, y1) = (current[0], current[1]);
        if (y1 > y) != (y0 > y) && x < (x0 - x1) * (y - y1) / (y0 - y1) + x1 {
            inside = !inside;
        }
        x0 = x1;
        y0 = y1;
    }
    Ok(inside)
}

/// d3-polygon's `polygonLength`.
///
/// The JavaScript:
///
/// ```js
/// export default function(polygon) {
///   var i = -1, n = polygon.length, b = polygon[n - 1], xa, ya,
///       xb = b[0], yb = b[1], perimeter = 0;
///   while (++i < n) {
///     xa = xb;
///     ya = yb;
///     b = polygon[i];
///     xb = b[0];
///     yb = b[1];
///     xa -= xb;
///     ya -= yb;
///     perimeter += Math.sqrt(xa * xa + ya * ya);
///   }
///   return perimeter;
/// }
/// ```
///
/// The perimeter closes itself, for the last point is joined to the first
/// one as well.
///
/// The JavaScript reads the last point of the polygon before it reads any
/// other one, and for a polygon with no points in it there is none to read,
/// so it throws; the port returns [`PolygonError::EmptyPolygon`].
pub fn polygon_length(polygon: &[Point]) -> Result<f64, PolygonError> {
    let n = polygon.len();
    if n == 0 {
        return Err(PolygonError::EmptyPolygon);
    }
    let mut b = polygon[n - 1];
    let (mut xb, mut yb) = (b[0], b[1]);
    let mut perimeter = 0.0;
    for &point in polygon {
        let mut xa = xb;
        let mut ya = yb;
        b = point;
        xb = b[0];
        yb = b[1];
        xa -= xb;
        ya -= yb;
        perimeter += ieee754::sqrt(xa * xa + ya * ya);
    }
    Ok(perimeter)
}

/// What can go wrong while a polygon is measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolygonError {
    /// The JavaScript reads a point out of a polygon that has none, and
    /// throws.
    EmptyPolygon,
}

impl std::fmt::Display for PolygonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolygonError::EmptyPolygon => {
                write!(f, "cannot read properties of undefined (reading '0')")
            }
        }
    }
}

impl std::error::Error for PolygonError {}

/// A point of the hull, with the place it had in the list it came from.
struct SortedPoint {
    /// The x of the point.
    x: f64,
    /// The y of the point.
    y: f64,
    /// The place of the point in the list it came from.
    place: usize,
}

/// The order the JavaScript sorts the points of a hull in.
///
/// The JavaScript:
///
/// ```js
/// function lexicographicOrder(a, b) {
///   return a[0] - b[0] || a[1] - b[1];
/// }
/// ```
///
/// The `||` gives the y when the x makes no difference, and it gives it
/// when the x is not a number either, for `NaN` is not true. The sort
/// keeps the points that are equal in the order they came in, and the
/// port's sort does the same.
fn lexicographic_order(a: &SortedPoint, b: &SortedPoint) -> Ordering {
    let dx = a.x - b.x;
    let difference = if dx != 0.0 && !dx.is_nan() {
        dx
    } else {
        a.y - b.y
    };
    if difference.is_nan() {
        // The specification of the sort says that a comparison which
        // cannot be made is a comparison of equals.
        Ordering::Equal
    } else if difference < 0.0 {
        Ordering::Less
    } else if difference > 0.0 {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

/// The places of the points of the upper hull, from the left to the right.
///
/// The JavaScript:
///
/// ```js
/// function computeUpperHullIndexes(points) {
///   var n = points.length, indexes = [0, 1], size = 2;
///   for (var i = 2; i < n; ++i) {
///     while (size > 1 && cross(points[indexes[size - 2]], points[indexes[size - 1]], points[i]) <= 0) --size;
///     indexes[size++] = i;
///   }
///   return indexes.slice(0, size); // remove popped points
/// }
/// ```
///
/// The points are sorted, and no two of them share a y, which is what the
/// JavaScript's own note says of them.
///
/// A point that is not a turn to the left is not on the hull, and is
/// thrown away: the `cross` of three points in a line is zero, and the
/// points in a line are not on the hull either.
fn upper_hull_indexes(points: &[(f64, f64)]) -> Vec<usize> {
    let n = points.len();
    let mut indexes = vec![0usize, 1usize];

    for i in 2..n {
        while indexes.len() > 1
            && cross(
                points[indexes[indexes.len() - 2]],
                points[indexes[indexes.len() - 1]],
                points[i],
            ) <= 0.0
        {
            indexes.pop();
        }
        indexes.push(i);
    }

    indexes
}

/// The two-dimensional cross product of AB and AC.
///
/// The JavaScript:
///
/// ```js
/// export default function(a, b, c) {
///   return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
/// }
/// ```
///
/// It is positive for three points that turn anticlockwise, negative for
/// three that turn clockwise, and zero for three that are in a line.
fn cross(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}
