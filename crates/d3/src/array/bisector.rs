//! d3-array's `bisector`, from the JavaScript of the same name.
//!
//! Translated from `d3-array/src/bisector.js`, version 1.2.4, of d3
//! 5.16.0. The JavaScript is the authority.

use super::ascending;

/// A comparison of two values, which gives `None` for the `NaN` that the
/// JavaScript's `ascending` gives for values that cannot be compared.
pub type Compare<'a, T> = dyn Fn(&T, &T) -> Option<std::cmp::Ordering> + 'a;

/// d3-array's `bisector`.
///
/// The JavaScript:
///
/// ```js
/// export default function(compare) {
///   if (compare.length === 1) compare = ascendingComparator(compare);
///   return {
///     left: function(a, x, lo, hi) { ... },
///     right: function(a, x, lo, hi) { ... }
///   };
/// }
/// ```
///
/// The `compare.length === 1` test is JavaScript's way of telling a
/// comparison of two values from a function that reads a value out of a
/// datum. Here the two are separate functions, so the port takes the
/// comparison of two values.
pub fn bisector<'a, T>(compare: &'a Compare<'a, T>) -> Bisector<'a, T> {
    Bisector { compare }
}

/// A bisector, with the two searches the JavaScript's gives.
pub struct Bisector<'a, T> {
    compare: &'a Compare<'a, T>,
}

impl<'a, T> Bisector<'a, T> {
    /// The JavaScript's `left`: the index of the first element that is not
    /// less than the value.
    pub fn left(&self, a: &[T], x: &T, lo: usize, hi: usize) -> usize {
        let mut lo = lo;
        let mut hi = hi;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if (self.compare)(&a[mid], x) == Some(std::cmp::Ordering::Less) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// The JavaScript's `right`: the index of the first element that is
    /// greater than the value.
    pub fn right(&self, a: &[T], x: &T, lo: usize, hi: usize) -> usize {
        let mut lo = lo;
        let mut hi = hi;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if (self.compare)(&a[mid], x) == Some(std::cmp::Ordering::Greater) {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        lo
    }
}

/// d3-array's `bisectLeft`, over numbers in order.
pub fn bisect_left(a: &[f64], x: f64) -> usize {
    let compare = |p: &f64, q: &f64| ascending(*p, *q);
    let bisector = bisector(&compare);
    bisector.left(a, &x, 0, a.len())
}

/// d3-array's `bisectRight`, over numbers in order.
pub fn bisect_right(a: &[f64], x: f64) -> usize {
    let compare = |p: &f64, q: &f64| ascending(*p, *q);
    let bisector = bisector(&compare);
    bisector.right(a, &x, 0, a.len())
}

/// d3-array's `bisect`, which is `bisectRight`.
pub fn bisect(a: &[f64], x: f64) -> usize {
    bisect_right(a, x)
}

/// d3-array's `bisectorLeft`, over numbers in order.
pub fn bisector_left(a: &[f64], x: f64) -> usize {
    bisect_left(a, x)
}

/// d3-array's `bisectorRight`, over numbers in order.
pub fn bisector_right(a: &[f64], x: f64) -> usize {
    bisect_right(a, x)
}
