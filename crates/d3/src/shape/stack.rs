//! d3-shape's `stack`, from the JavaScript of the same name.
//!
//! Translated from `d3-shape/src/stack.js`, `d3-shape/src/offset/*.js` and
//! `d3-shape/src/order/*.js`, version 1.3.7, of d3 5.16.0. The JavaScript
//! is the authority.

use crate::arith;
use crate::js;

/// A band of a stack: the space one key takes in one place.
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesPoint<T> {
    /// The lower bound of the band.
    pub y0: f64,
    /// The upper bound of the band.
    pub y1: f64,
    /// The datum the band was made from.
    pub data: T,
}

/// A key of a stack, with the band it takes in every place.
#[derive(Clone, Debug, PartialEq)]
pub struct Series<T> {
    /// The key.
    pub key: String,
    /// The band, one place for each datum.
    pub points: Vec<SeriesPoint<T>>,
    /// The key's place among the keys, in the order they are drawn.
    pub index: usize,
}

/// A function that reads the value of a datum for a key.
pub type ValueOf<T> = Box<dyn Fn(&T, &str, usize, &[T]) -> f64>;

/// A function that says in what order the keys are drawn.
pub type Order<T> = Box<dyn Fn(&[Series<T>]) -> Vec<usize>>;

/// A function that moves the bands of the stack.
pub type Offset<T> = Box<dyn Fn(&mut [Series<T>], &[usize])>;

/// d3-shape's `stack`.
///
/// The JavaScript:
///
/// ```js
/// function stack(data) {
///   var kz = keys.apply(this, arguments), i, m = data.length, n = kz.length,
///       sz = new Array(n), oz;
///   for (i = 0; i < n; ++i) {
///     for (var ki = kz[i], si = sz[i] = new Array(m), j = 0, sij; j < m; ++j) {
///       si[j] = sij = [0, +value(data[j], ki, j, data)];
///       sij.data = data[j];
///     }
///     si.key = ki;
///   }
///   for (i = 0, oz = order(sz); i < n; ++i) { sz[oz[i]].index = i; }
///   offset(sz, oz);
///   return sz;
/// }
/// ```
///
/// The JavaScript's `stack.keys` takes either a function or a list, and
/// the port's takes a list.
pub struct Stack<T> {
    keys: Vec<String>,
    order: Order<T>,
    offset: Offset<T>,
    value: ValueOf<T>,
}

/// What a stack's datum gives: the JavaScript's `stackValue` reads
/// `d[key]`, and the port's default asks the datum for its value for a
/// key.
pub trait StackDatum {
    /// The value of the datum for a key.
    fn value_for(&self, key: &str) -> f64;
}

impl<T: StackDatum> Default for Stack<T> {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            order: Box::new(|series: &[Series<T>]| stack_order_none(series)),
            offset: Box::new(|series: &mut [Series<T>], order: &[usize]| {
                stack_offset_none(series, order)
            }),
            value: Box::new(|d: &T, key: &str, _, _| d.value_for(key)),
        }
    }
}

impl<T> Stack<T> {
    /// The JavaScript's `stack.keys`.
    pub fn keys(mut self, keys: Vec<String>) -> Self {
        self.keys = keys;
        self
    }

    /// The JavaScript's `stack.value`.
    pub fn value<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, &str, usize, &[T]) -> f64 + 'static,
    {
        self.value = Box::new(f);
        self
    }

    /// The JavaScript's `stack.order`.
    pub fn order<F>(mut self, f: F) -> Self
    where
        F: Fn(&[Series<T>]) -> Vec<usize> + 'static,
    {
        self.order = Box::new(f);
        self
    }

    /// The JavaScript's `stack.offset`.
    pub fn offset<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut [Series<T>], &[usize]) + 'static,
    {
        self.offset = Box::new(f);
        self
    }

    /// The JavaScript's `stack(data)`.
    pub fn apply(&self, data: &[T]) -> Vec<Series<T>>
    where
        T: Clone,
    {
        let m = data.len();
        let n = self.keys.len();
        let mut series: Vec<Series<T>> = Vec::with_capacity(n);
        for key in &self.keys {
            let mut points = Vec::with_capacity(m);
            for (j, datum) in data.iter().enumerate() {
                points.push(SeriesPoint {
                    y0: 0.0,
                    y1: (self.value)(datum, key, j, data),
                    data: datum.clone(),
                });
            }
            series.push(Series {
                key: key.clone(),
                points,
                index: 0,
            });
        }
        let order = (self.order)(&series);
        for (i, &o) in order.iter().enumerate() {
            series[o].index = i;
        }
        (self.offset)(&mut series, &order);
        series
    }
}

/// d3-shape's `stackOffsetNone`.
///
/// The JavaScript:
///
/// ```js
/// export default function(series, order) {
///   if (!((n = series.length) > 1)) return;
///   for (var i = 1, j, s0, s1 = series[order[0]], n, m = s1.length; i < n; ++i) {
///     s0 = s1, s1 = series[order[i]];
///     for (j = 0; j < m; ++j) {
///       s1[j][1] += s1[j][0] = isNaN(s0[j][1]) ? s0[j][0] : s0[j][1];
///     }
///   }
/// }
/// ```
pub fn stack_offset_none<T>(series: &mut [Series<T>], order: &[usize]) {
    let n = series.len();
    if n <= 1 {
        return;
    }
    let m = series[order[0]].points.len();
    let mut s0 = order[0];
    for i in 1..n {
        let s1 = order[i];
        for j in 0..m {
            let previous = series[s0].points[j].y1;
            let base = if previous.is_nan() {
                series[s0].points[j].y0
            } else {
                previous
            };
            series[s1].points[j].y0 = base;
            series[s1].points[j].y1 = arith::add(series[s1].points[j].y1, base);
        }
        s0 = s1;
    }
}

/// d3-shape's `stackOffsetDiverging`.
pub fn stack_offset_diverging<T>(series: &mut [Series<T>], order: &[usize]) {
    let n = series.len();
    if n == 0 {
        return;
    }
    let m = series[order[0]].points.len();
    for j in 0..m {
        let mut yp = 0.0;
        let mut yn = 0.0;
        for &index in order.iter() {
            let dy = arith::subtract(series[index].points[j].y1, series[index].points[j].y0);
            if dy > 0.0 {
                series[index].points[j].y0 = yp;
                yp = arith::add(yp, dy);
                series[index].points[j].y1 = yp;
            } else if dy < 0.0 {
                series[index].points[j].y1 = yn;
                yn = arith::add(yn, dy);
                series[index].points[j].y0 = yn;
            } else {
                series[index].points[j].y0 = 0.0;
                series[index].points[j].y1 = dy;
            }
        }
    }
}

/// d3-shape's `stackOffsetExpand`.
pub fn stack_offset_expand<T>(series: &mut [Series<T>], order: &[usize]) {
    let n = series.len();
    if n == 0 {
        return;
    }
    let m = series[0].points.len();
    for j in 0..m {
        let mut y = 0.0;
        for i in 0..n {
            let value = series[i].points[j].y1;
            y = arith::add(y, if js::truthy(value) { value } else { 0.0 });
        }
        if js::truthy(y) {
            for i in 0..n {
                series[i].points[j].y1 = arith::divide(series[i].points[j].y1, y);
            }
        }
    }
    stack_offset_none(series, order);
}

/// d3-shape's `stackOffsetSilhouette`.
pub fn stack_offset_silhouette<T>(series: &mut [Series<T>], order: &[usize]) {
    let n = series.len();
    if n == 0 {
        return;
    }
    let s0 = order[0];
    let m = series[s0].points.len();
    for j in 0..m {
        let mut y = 0.0;
        for i in 0..n {
            let value = series[i].points[j].y1;
            y = arith::add(y, if js::truthy(value) { value } else { 0.0 });
        }
        series[s0].points[j].y0 = arith::divide(-y, 2.0);
        series[s0].points[j].y1 = arith::add(series[s0].points[j].y1, series[s0].points[j].y0);
    }
    stack_offset_none(series, order);
}

/// d3-shape's `stackOffsetWiggle`.
pub fn stack_offset_wiggle<T>(series: &mut [Series<T>], order: &[usize]) {
    let n = series.len();
    if n == 0 {
        return;
    }
    let s0 = order[0];
    let m = series[s0].points.len();
    if m == 0 {
        return;
    }
    let mut y = 0.0;
    for j in 1..m {
        let mut s1 = 0.0;
        let mut s2 = 0.0;
        for (i, &index) in order.iter().enumerate() {
            let raw0 = series[index].points[j].y1;
            let raw1 = series[index].points[j - 1].y1;
            let sij0 = if js::truthy(raw0) { raw0 } else { 0.0 };
            let sij1 = if js::truthy(raw1) { raw1 } else { 0.0 };
            let mut s3 = arith::divide(arith::subtract(sij0, sij1), 2.0);
            for k in 0..i {
                let sk = order[k];
                let raw0 = series[sk].points[j].y1;
                let raw1 = series[sk].points[j - 1].y1;
                let skj0 = if js::truthy(raw0) { raw0 } else { 0.0 };
                let skj1 = if js::truthy(raw1) { raw1 } else { 0.0 };
                s3 = arith::add(s3, arith::subtract(skj0, skj1));
            }
            s1 = arith::add(s1, sij0);
            s2 = arith::add(s2, arith::multiply(s3, sij0));
        }
        series[s0].points[j - 1].y0 = y;
        series[s0].points[j - 1].y1 = arith::add(series[s0].points[j - 1].y1, y);
        if js::truthy(s1) {
            y = arith::subtract(y, arith::divide(s2, s1));
        }
    }
    series[s0].points[m - 1].y0 = y;
    series[s0].points[m - 1].y1 = arith::add(series[s0].points[m - 1].y1, y);
    stack_offset_none(series, order);
}

/// d3-shape's `stackOrderNone`.
pub fn stack_order_none<T>(series: &[Series<T>]) -> Vec<usize> {
    // The JavaScript's `while (--n >= 0) o[n] = n;` fills the list from
    // the end, and gives the order the keys are in.
    (0..series.len()).collect()
}

/// d3-shape's `stackOrderReverse`.
pub fn stack_order_reverse<T>(series: &[Series<T>]) -> Vec<usize> {
    let mut order = stack_order_none(series);
    order.reverse();
    order
}

/// d3-shape's `stackOrderAscending`.
pub fn stack_order_ascending<T>(series: &[Series<T>]) -> Vec<usize> {
    let sums: Vec<f64> = series.iter().map(stack_sum).collect();
    let mut order = stack_order_none(series);
    order.sort_by(|&a, &b| {
        let difference = arith::subtract(sums[a], sums[b]);
        if difference.is_nan() || difference == 0.0 {
            std::cmp::Ordering::Equal
        } else if difference < 0.0 {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });
    order
}

/// The JavaScript's `sum`, which adds a series' values.
fn stack_sum<T>(series: &Series<T>) -> f64 {
    let mut s = 0.0;
    for point in &series.points {
        let v = point.y1;
        if js::truthy(v) {
            s = arith::add(s, v);
        }
    }
    s
}

/// d3-shape's `stackOrderDescending`.
pub fn stack_order_descending<T>(series: &[Series<T>]) -> Vec<usize> {
    let mut order = stack_order_ascending(series);
    order.reverse();
    order
}

/// d3-shape's `stackOrderAppearance`.
pub fn stack_order_appearance<T>(series: &[Series<T>]) -> Vec<usize> {
    let peaks: Vec<usize> = series.iter().map(stack_peak).collect();
    let mut order = stack_order_none(series);
    order.sort_by(|&a, &b| peaks[a].cmp(&peaks[b]));
    order
}

/// The JavaScript's `peak`, which finds the place of a series' largest
/// value.
fn stack_peak<T>(series: &Series<T>) -> usize {
    let mut j = 0usize;
    let mut vj = f64::NEG_INFINITY;
    for (i, point) in series.points.iter().enumerate() {
        let vi = point.y1;
        if vi > vj {
            vj = vi;
            j = i;
        }
    }
    j
}

/// d3-shape's `stackOrderInsideOut`.
pub fn stack_order_inside_out<T>(series: &[Series<T>]) -> Vec<usize> {
    let n = series.len();
    let sums: Vec<f64> = series.iter().map(stack_sum).collect();
    let order = stack_order_appearance(series);
    let mut top = 0.0;
    let mut bottom = 0.0;
    let mut tops = Vec::new();
    let mut bottoms = Vec::new();
    for i in 0..n {
        let j = order[i];
        if top < bottom {
            top = arith::add(top, sums[j]);
            tops.push(j);
        } else {
            bottom = arith::add(bottom, sums[j]);
            bottoms.push(j);
        }
    }
    bottoms.reverse();
    bottoms.extend(tops);
    bottoms
}
