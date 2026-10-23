//! d3-shape's `pie`, from the JavaScript of the same name.
//!
//! Translated from `d3-shape/src/pie.js`, version 1.3.7, of d3 5.16.0.
//! The JavaScript is the authority.

use crate::arith;
use crate::js;

/// Twice pi, which is a whole turn.
const TAU: f64 = std::f64::consts::PI * 2.0;

/// A slice of a pie.
#[derive(Clone, Debug, PartialEq)]
pub struct ArcDatum<T> {
    /// The datum the slice was made from.
    pub data: T,
    /// The slice's place among the slices, in the order they were drawn.
    pub index: usize,
    /// The datum's value, which is what the slice's size came from.
    pub value: f64,
    /// Where the slice starts, in radians.
    pub start_angle: f64,
    /// Where the slice ends, in radians.
    pub end_angle: f64,
    /// The angle the slice was padded by.
    pub pad_angle: f64,
}

/// A function that reads the value of a datum.
pub type ValueOf<T> = Box<dyn Fn(&T, usize, &[T]) -> f64>;

/// A function that compares two values, which is how the JavaScript's
/// `sortValues` orders the slices.
pub type CompareValues = Box<dyn Fn(f64, f64) -> f64>;

/// A function that compares two data, which is how the JavaScript's
/// `sort` orders the slices.
pub type CompareData<T> = Box<dyn Fn(&T, &T) -> f64>;

/// A function that reads an angle of the pie, which the JavaScript's
/// `startAngle`, `endAngle` and `padAngle` are.
pub type Angle = Box<dyn Fn(&[f64]) -> f64>;

/// d3-shape's `pie`.
///
/// The JavaScript:
///
/// ```js
/// function pie(data) {
///   var i, n = data.length, j, k, sum = 0,
///       index = new Array(n), arcs = new Array(n),
///       a0 = +startAngle.apply(this, arguments),
///       da = Math.min(tau, Math.max(-tau, endAngle.apply(this, arguments) - a0)),
///       a1, p = Math.min(Math.abs(da) / n, padAngle.apply(this, arguments)),
///       pa = p * (da < 0 ? -1 : 1), v;
///   for (i = 0; i < n; ++i) {
///     if ((v = arcs[index[i] = i] = +value(data[i], i, data)) > 0) sum += v;
///   }
///   if (sortValues != null) index.sort(function(i, j) { return sortValues(arcs[i], arcs[j]); });
///   else if (sort != null) index.sort(function(i, j) { return sort(data[i], data[j]); });
///   for (i = 0, k = sum ? (da - n * pa) / sum : 0; i < n; ++i, a0 = a1) {
///     j = index[i], v = arcs[j], a1 = a0 + (v > 0 ? v * k : 0) + pa, arcs[j] = {...};
///   }
///   return arcs;
/// }
/// ```
///
/// The JavaScript's `pie.sortValues` and `pie.sort` are two ways of
/// ordering the slices, and only one of them may be set at a time: setting
/// one clears the other. The port keeps that.
pub struct Pie<T> {
    value: ValueOf<T>,
    sort_values: Option<CompareValues>,
    sort: Option<CompareData<T>>,
    start_angle: Angle,
    end_angle: Angle,
    pad_angle: Angle,
}

impl Pie<f64> {
    /// The JavaScript's `pie`, with the value function the JavaScript's
    /// `identity` stands for: the datum is its own value.
    ///
    /// The JavaScript's `identity` works on any datum and is turned into
    /// a number by the `+` that follows it. In Rust the datum is a
    /// number, and this is the pie that reads it.
    pub fn of_numbers() -> Self {
        Self {
            value: Box::new(|d: &f64, _, _| *d),
            sort_values: Some(Box::new(|a: f64, b: f64| {
                // The JavaScript's `descending`.
                if b < a {
                    -1.0
                } else if b > a {
                    1.0
                } else if b >= a {
                    0.0
                } else {
                    f64::NAN
                }
            })),
            sort: None,
            start_angle: Box::new(|_| 0.0),
            end_angle: Box::new(|_| TAU),
            pad_angle: Box::new(|_| 0.0),
        }
    }
}

impl<T> Pie<T> {
    /// The JavaScript's `pie.value`.
    pub fn value<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.value = Box::new(f);
        self
    }

    /// The JavaScript's `pie.sortValues`, which orders the slices by the
    /// value they were made from.
    pub fn sort_values<F>(mut self, f: F) -> Self
    where
        F: Fn(f64, f64) -> f64 + 'static,
    {
        self.sort_values = Some(Box::new(f));
        self.sort = None;
        self
    }

    /// The JavaScript's `pie.sort`, which orders the slices by the data
    /// they were made from.
    pub fn sort<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, &T) -> f64 + 'static,
    {
        self.sort = Some(Box::new(f));
        self.sort_values = None;
        self
    }

    /// The JavaScript's `pie.startAngle`.
    pub fn start_angle<F>(mut self, f: F) -> Self
    where
        F: Fn(&[f64]) -> f64 + 'static,
    {
        self.start_angle = Box::new(f);
        self
    }

    /// The JavaScript's `pie.endAngle`.
    pub fn end_angle<F>(mut self, f: F) -> Self
    where
        F: Fn(&[f64]) -> f64 + 'static,
    {
        self.end_angle = Box::new(f);
        self
    }

    /// The JavaScript's `pie.padAngle`.
    pub fn pad_angle<F>(mut self, f: F) -> Self
    where
        F: Fn(&[f64]) -> f64 + 'static,
    {
        self.pad_angle = Box::new(f);
        self
    }

    /// The JavaScript's `pie(data)`: the slices of the pie.
    pub fn apply(&self, data: &[T]) -> Vec<ArcDatum<T>>
    where
        T: Clone,
    {
        // The JavaScript's angles are told the data, and the port's are
        // told the values, which is what the angles are made of.
        let values: Vec<f64> = (0..data.len())
            .map(|i| (self.value)(&data[i], i, data))
            .collect();
        let n = data.len();
        let mut sum = 0.0;
        let mut index: Vec<usize> = (0..n).collect();
        for v in &values {
            if *v > 0.0 {
                sum = arith::add(sum, *v);
            }
        }
        let a0 = (self.start_angle)(&values);
        let da = js_min(
            TAU,
            js_max(-TAU, arith::subtract((self.end_angle)(&values), a0)),
        );
        let p = js_min(arith::divide(da.abs(), n as f64), (self.pad_angle)(&values));
        let pa = arith::multiply(p, if da < 0.0 { -1.0 } else { 1.0 });

        // Optionally sort the arcs by previously-computed values or by data.
        if let Some(sort_values) = &self.sort_values {
            index.sort_by(|&i, &j| {
                // The JavaScript's `index.sort(function(i, j) { return
                // sortValues(arcs[i], arcs[j]); })`, and the port's is
                // the same call.
                let order = sort_values(values[i], values[j]);
                if order < 0.0 {
                    std::cmp::Ordering::Less
                } else if order > 0.0 {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
        } else if let Some(sort) = &self.sort {
            index.sort_by(|&i, &j| {
                let order = sort(&data[i], &data[j]);
                if order < 0.0 {
                    std::cmp::Ordering::Less
                } else if order > 0.0 {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
        }

        // Compute the arcs! They are stored in the original data's order:
        // the JavaScript writes each arc into the place of the datum it
        // came from, and says which order it is drawn in.
        let mut arcs: Vec<Option<ArcDatum<T>>> = vec![None; n];
        let mut a0 = a0;
        let k = if js::truthy(sum) {
            arith::divide(arith::subtract(da, arith::multiply(n as f64, pa)), sum)
        } else {
            0.0
        };
        for (i, &j) in index.iter().enumerate() {
            let v = values[j];
            let a1 = arith::add(
                arith::add(a0, if v > 0.0 { arith::multiply(v, k) } else { 0.0 }),
                pa,
            );
            arcs[j] = Some(ArcDatum {
                data: data[j].clone(),
                index: i,
                value: v,
                start_angle: a0,
                end_angle: a1,
                pad_angle: p,
            });
            a0 = a1;
        }
        arcs.into_iter()
            .map(|arc| arc.expect("every datum has an arc"))
            .collect()
    }
}

/// The JavaScript's `Math.min`, which gives a number that is not a
/// number when either of its arguments is one.
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b {
        a
    } else {
        b
    }
}

/// The JavaScript's `Math.max`, which gives a number that is not a
/// number when either of its arguments is one.
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else {
        b
    }
}
