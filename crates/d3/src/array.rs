//! A port of **d3-array**, from the JavaScript of the same name.
//!
//! Translated from `d3-array/src/*.js`, version 1.2.4, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read differently,
//! the JavaScript is right.
//!
//! d3-array is the collection of the small functions d3 is built on: the
//! statistics of a list, the way a list is searched, and the handful of
//! list operations the rest of the library needs.
//!
//! # The shape of the port
//!
//! The JavaScript's functions take a list and, often, a function that
//! reads a value out of a datum. Here the list is a slice and the function
//! is a closure, and the two are told apart by their names: a function
//! without a `_by` suffix reads the datum itself, and one with the suffix
//! takes the reader.
//!
//! Where the JavaScript returns `undefined` — for an empty list, or for a
//! list with nothing comparable in it — the port returns `None`. Where the
//! JavaScript returns `NaN`, the port returns `f64::NAN`.

use crate::js;

pub mod bisector;

// The JavaScript's index exports the bisectors from the bisector module, and
// the port does the same, so that `d3::array::bisect` is where it is in the
// JavaScript.
pub use bisector::{bisect, bisect_left, bisect_right, bisector_left, bisector_right};

/// A function that reads a value out of a datum, its place, and the list.
pub type ValueOf<'a, T> = dyn Fn(&T, usize, &[T]) -> f64 + 'a;

/// A function that reads the lowest and the highest of a list, which is
/// what the JavaScript's `extent` is.
pub type Domain<'a> = dyn Fn(&[f64]) -> (Option<f64>, Option<f64>) + 'a;

/// d3-array's `range`.
///
/// The JavaScript:
///
/// ```js
/// export default function(start, stop, step) {
///   start = +start, stop = +stop, step = (n = arguments.length) < 2 ? (stop = start, start = 0, 1) : n < 3 ? 1 : +step;
///   var i = -1,
///       n = Math.max(0, Math.ceil((stop - start) / step)) | 0,
///       range = new Array(n);
///   while (++i < n) {
///     range[i] = start + i * step;
///   }
///   return range;
/// }
/// ```
///
/// The `| 0` is JavaScript's conversion to a 32-bit signed integer, and it
/// is kept: a count that does not fit becomes zero, as it does there.
pub fn range(start: f64, stop: Option<f64>, step: Option<f64>) -> Vec<f64> {
    let (start, stop, step) = match (stop, step) {
        (None, _) => (0.0, start, 1.0),
        (Some(stop), None) => (start, stop, 1.0),
        (Some(stop), Some(step)) => (start, stop, step),
    };
    let count = (js::number(stop) - js::number(start)) / js::number(step);
    let count = if count.is_nan() { 0.0 } else { count.max(0.0) };
    let n = count.ceil();
    let n = if n > i32::MAX as f64 || n < i32::MIN as f64 {
        0
    } else {
        n as i32 as usize
    };
    let mut out = Vec::with_capacity(n);
    let mut i = 0usize;
    while i < n {
        out.push(js::number(start) + i as f64 * js::number(step));
        i += 1;
    }
    out
}

/// d3-array's `range`, with the arguments the JavaScript's `arguments`
/// object makes possible.
pub fn range_args(args: &[f64]) -> Vec<f64> {
    match args.len() {
        0 => Vec::new(),
        1 => range(args[0], None, None),
        2 => range(args[0], Some(args[1]), None),
        _ => range(args[0], Some(args[1]), Some(args[2])),
    }
}

/// d3-array's `ascending`.
///
/// The JavaScript returns `NaN` for values that cannot be compared, which
/// is `None` here.
pub fn ascending(a: f64, b: f64) -> Option<std::cmp::Ordering> {
    if a < b {
        Some(std::cmp::Ordering::Less)
    } else if a > b {
        Some(std::cmp::Ordering::Greater)
    } else if a >= b {
        Some(std::cmp::Ordering::Equal)
    } else {
        None
    }
}

/// d3-array's `descending`.
pub fn descending(a: f64, b: f64) -> Option<std::cmp::Ordering> {
    if b < a {
        Some(std::cmp::Ordering::Less)
    } else if b > a {
        Some(std::cmp::Ordering::Greater)
    } else if b >= a {
        Some(std::cmp::Ordering::Equal)
    } else {
        None
    }
}

/// d3-array's `min`, without a value function.
///
/// The JavaScript finds the first value that can be compared — that is,
/// the first that is neither `null` nor `NaN` — and compares the rest
/// against it. A list with no comparable value gives `undefined`, which is
/// `None` here.
pub fn min(values: &[f64]) -> Option<f64> {
    let mut min: Option<f64> = None;
    for &value in values {
        if value.is_nan() {
            continue;
        }
        match min {
            None => min = Some(value),
            Some(current) if current > value => min = Some(value),
            _ => {}
        }
    }
    min
}

/// d3-array's `min`, with a value function.
pub fn min_by<T>(values: &[T], valueof: &ValueOf<T>) -> Option<f64> {
    let mut min: Option<f64> = None;
    for (i, datum) in values.iter().enumerate() {
        let value = valueof(datum, i, values);
        if value.is_nan() {
            continue;
        }
        match min {
            None => min = Some(value),
            Some(current) if current > value => min = Some(value),
            _ => {}
        }
    }
    min
}

/// d3-array's `max`, without a value function.
pub fn max(values: &[f64]) -> Option<f64> {
    let mut max: Option<f64> = None;
    for &value in values {
        if value.is_nan() {
            continue;
        }
        match max {
            None => max = Some(value),
            Some(current) if current < value => max = Some(value),
            _ => {}
        }
    }
    max
}

/// d3-array's `max`, with a value function.
pub fn max_by<T>(values: &[T], valueof: &ValueOf<T>) -> Option<f64> {
    let mut max: Option<f64> = None;
    for (i, datum) in values.iter().enumerate() {
        let value = valueof(datum, i, values);
        if value.is_nan() {
            continue;
        }
        match max {
            None => max = Some(value),
            Some(current) if current < value => max = Some(value),
            _ => {}
        }
    }
    max
}

/// d3-array's `extent`, without a value function.
///
/// The JavaScript returns `[min, max]`, and either may be `undefined` when
/// the list holds no comparable value.
pub fn extent(values: &[f64]) -> (Option<f64>, Option<f64>) {
    let mut min: Option<f64> = None;
    let mut max: Option<f64> = None;
    for &value in values {
        if value.is_nan() {
            continue;
        }
        match (min, max) {
            (None, _) => {
                min = Some(value);
                max = Some(value);
            }
            (Some(current_min), Some(current_max)) => {
                if current_min > value {
                    min = Some(value);
                }
                if current_max < value {
                    max = Some(value);
                }
            }
            _ => {}
        }
    }
    (min, max)
}

/// d3-array's `extent`, with a value function.
pub fn extent_by<T>(values: &[T], valueof: &ValueOf<T>) -> (Option<f64>, Option<f64>) {
    let mut min: Option<f64> = None;
    let mut max: Option<f64> = None;
    for (i, datum) in values.iter().enumerate() {
        let value = valueof(datum, i, values);
        if value.is_nan() {
            continue;
        }
        match (min, max) {
            (None, _) => {
                min = Some(value);
                max = Some(value);
            }
            (Some(current_min), Some(current_max)) => {
                if current_min > value {
                    min = Some(value);
                }
                if current_max < value {
                    max = Some(value);
                }
            }
            _ => {}
        }
    }
    (min, max)
}

/// d3-array's `sum`, without a value function.
///
/// The JavaScript's own note is kept: zero and null are equivalent, and a
/// value that is not a number contributes nothing.
pub fn sum(values: &[f64]) -> f64 {
    let mut sum = 0.0;
    for &value in values {
        let value = js::number(value);
        if value != 0.0 {
            sum += value;
        }
    }
    sum
}

/// d3-array's `sum`, with a value function.
pub fn sum_by<T>(values: &[T], valueof: &ValueOf<T>) -> f64 {
    let mut sum = 0.0;
    for (i, datum) in values.iter().enumerate() {
        let value = js::number(valueof(datum, i, values));
        if value != 0.0 {
            sum += value;
        }
    }
    sum
}

/// d3-array's `mean`, without a value function.
///
/// The JavaScript returns `undefined` for a list with no values.
pub fn mean(values: &[f64]) -> Option<f64> {
    let mut m = values.len();
    let mut sum = 0.0;
    for &value in values {
        let value = js::number(value);
        if !value.is_nan() {
            sum += value;
        } else {
            m -= 1;
        }
    }
    if m > 0 {
        Some(sum / m as f64)
    } else {
        None
    }
}

/// d3-array's `mean`, with a value function.
pub fn mean_by<T>(values: &[T], valueof: &ValueOf<T>) -> Option<f64> {
    let mut m = values.len();
    let mut sum = 0.0;
    for (i, datum) in values.iter().enumerate() {
        let value = js::number(valueof(datum, i, values));
        if !value.is_nan() {
            sum += value;
        } else {
            m -= 1;
        }
    }
    if m > 0 {
        Some(sum / m as f64)
    } else {
        None
    }
}

/// d3-array's `median`, without a value function.
///
/// The JavaScript sorts the numbers before taking the quantile, and the
/// sort is by `ascending`, which puts the `NaN`s last. The `NaN`s are
/// dropped before the sort here, which is the same set of numbers.
pub fn median(values: &[f64]) -> Option<f64> {
    let mut numbers: Vec<f64> = values.iter().copied().filter(|v| !v.is_nan()).collect();
    numbers.sort_by(|a, b| ascending(*a, *b).unwrap_or(std::cmp::Ordering::Equal));
    quantile_sorted(&numbers, 0.5)
}

/// d3-array's `median`, with a value function.
pub fn median_by<T>(values: &[T], valueof: &ValueOf<T>) -> Option<f64> {
    let mut numbers: Vec<f64> = values
        .iter()
        .enumerate()
        .map(|(i, datum)| valueof(datum, i, values))
        .filter(|v| !v.is_nan())
        .collect();
    numbers.sort_by(|a, b| ascending(*a, *b).unwrap_or(std::cmp::Ordering::Equal));
    quantile_sorted(&numbers, 0.5)
}

/// d3-array's `quantile`, without a value function.
///
/// The JavaScript's `quantile` does **not** sort: it takes the numbers as
/// they are, and the `quantileSorted` of the later versions is not in this
/// one. The port keeps that, and [`quantile_sorted`] is here for the cases
/// where the JavaScript sorts first, as `median` does.
pub fn quantile(values: &[f64], p: f64) -> Option<f64> {
    let n = values.len();
    if n == 0 {
        return None;
    }
    let p = js::number(p);
    if p <= 0.0 || n < 2 {
        return Some(js::number(values[0]));
    }
    if p >= 1.0 {
        return Some(js::number(values[n - 1]));
    }
    let i = (n - 1) as f64 * p;
    let i0 = i.floor();
    let index = i0 as usize;
    let value0 = js::number(values[index]);
    let value1 = js::number(values[index + 1]);
    Some(value0 + (value1 - value0) * (i - i0))
}

/// d3-array's `quantile`, with a value function.
pub fn quantile_by<T>(values: &[T], p: f64, valueof: &ValueOf<T>) -> Option<f64> {
    let n = values.len();
    if n == 0 {
        return None;
    }
    let p = js::number(p);
    if p <= 0.0 || n < 2 {
        return Some(js::number(valueof(&values[0], 0, values)));
    }
    if p >= 1.0 {
        return Some(js::number(valueof(&values[n - 1], n - 1, values)));
    }
    let i = (n - 1) as f64 * p;
    let i0 = i.floor();
    let index = i0 as usize;
    let value0 = js::number(valueof(&values[index], index, values));
    let value1 = js::number(valueof(&values[index + 1], index + 1, values));
    Some(value0 + (value1 - value0) * (i - i0))
}

/// The quantile of numbers that are already in order, which is what the
/// JavaScript's `median` gets at by sorting first.
pub fn quantile_sorted(values: &[f64], p: f64) -> Option<f64> {
    let n = values.len();
    if n == 0 {
        return None;
    }
    let p = js::number(p);
    if p <= 0.0 || n < 2 {
        return Some(js::number(values[0]));
    }
    if p >= 1.0 {
        return Some(js::number(values[n - 1]));
    }
    let i = (n - 1) as f64 * p;
    let i0 = i.floor();
    let index = i0 as usize;
    let value0 = js::number(values[index]);
    let value1 = js::number(values[index + 1]);
    Some(value0 + (value1 - value0) * (i - i0))
}

/// d3-array's `variance`, without a value function.
pub fn variance(values: &[f64]) -> Option<f64> {
    let mut m = 0usize;
    let mut mean = 0.0;
    let mut sum = 0.0;
    for &value in values {
        let value = js::number(value);
        if !value.is_nan() {
            let delta = value - mean;
            m += 1;
            mean += delta / m as f64;
            sum += delta * (value - mean);
        }
    }
    if m > 1 {
        Some(sum / (m - 1) as f64)
    } else {
        None
    }
}

/// d3-array's `variance`, with a value function.
pub fn variance_by<T>(values: &[T], valueof: &ValueOf<T>) -> Option<f64> {
    let mut m = 0usize;
    let mut mean = 0.0;
    let mut sum = 0.0;
    for (i, datum) in values.iter().enumerate() {
        let value = js::number(valueof(datum, i, values));
        if !value.is_nan() {
            let delta = value - mean;
            m += 1;
            mean += delta / m as f64;
            sum += delta * (value - mean);
        }
    }
    if m > 1 {
        Some(sum / (m - 1) as f64)
    } else {
        None
    }
}

/// d3-array's `deviation`, without a value function.
pub fn deviation(values: &[f64]) -> Option<f64> {
    variance(values).map(|v| v.sqrt())
}

/// d3-array's `deviation`, with a value function.
pub fn deviation_by<T>(values: &[T], valueof: &ValueOf<T>) -> Option<f64> {
    variance_by(values, valueof).map(|v| v.sqrt())
}

/// d3-array's `merge`.
pub fn merge<T: Clone>(lists: &[Vec<T>]) -> Vec<T> {
    let mut out = Vec::new();
    for list in lists {
        out.extend(list.iter().cloned());
    }
    out
}

/// d3-array's `transpose`.
///
/// The JavaScript transposes a ragged matrix without complaint: the
/// shortest row decides how many rows the result has, and the missing
/// entries are `undefined`. The port leaves them out, and the result is
/// the same length as the shortest row.
pub fn transpose<T: Clone>(matrix: &[Vec<T>]) -> Vec<Vec<T>> {
    if matrix.is_empty() {
        return Vec::new();
    }
    let m = matrix.iter().map(|row| row.len()).min().unwrap_or(0);
    let mut out = Vec::with_capacity(m);
    for i in 0..m {
        let mut row = Vec::with_capacity(matrix.len());
        for column in matrix {
            if let Some(value) = column.get(i) {
                row.push(value.clone());
            }
        }
        out.push(row);
    }
    out
}

/// d3-array's `zip`, which is `transpose` of the arguments.
pub fn zip<T: Clone>(arrays: &[Vec<T>]) -> Vec<Vec<T>> {
    transpose(arrays)
}

/// d3-array's `pairs`.
///
/// The JavaScript's `pairs` takes an optional function that makes each
/// pair, and the port's takes one too.
pub fn pairs<T: Clone>(array: &[T]) -> Vec<(T, T)> {
    pairs_by(array, |a, b| (a.clone(), b.clone()))
}

/// d3-array's `pairs`, with the function that makes each pair.
pub fn pairs_by<T: Clone, F: Fn(&T, &T) -> (T, T)>(array: &[T], f: F) -> Vec<(T, T)> {
    if array.is_empty() {
        return Vec::new();
    }
    let n = array.len() - 1;
    let mut out = Vec::with_capacity(n);
    let mut p = &array[0];
    let mut i = 0usize;
    while i < n {
        let next = &array[i + 1];
        out.push(f(p, next));
        p = next;
        i += 1;
    }
    out
}

/// d3-array's `cross`, without a reducing function.
pub fn cross<T: Clone>(values0: &[T], values1: &[T]) -> Vec<(T, T)> {
    cross_by(values0, values1, |a, b| (a.clone(), b.clone()))
}

/// d3-array's `cross`, with the function that makes each pair.
pub fn cross_by<T: Clone, R, F: Fn(&T, &T) -> R>(values0: &[T], values1: &[T], f: F) -> Vec<R> {
    let mut out = Vec::with_capacity(values0.len() * values1.len());
    for value0 in values0 {
        for value1 in values1 {
            out.push(f(value0, value1));
        }
    }
    out
}

/// d3-array's `permute`.
pub fn permute<T: Clone>(array: &[T], indexes: &[usize]) -> Vec<T> {
    // The JavaScript builds the list with `new Array(i)`, which is a list of
    // that length with nothing in it, and then fills it from the end. The
    // port fills a list of the same length.
    let mut out: Vec<Option<T>> = vec![None; indexes.len()];
    let mut i = indexes.len();
    while i > 0 {
        i -= 1;
        out[i] = Some(array[indexes[i]].clone());
    }
    out.into_iter()
        .map(|v| v.expect("every place is filled"))
        .collect()
}

/// d3-array's `scan`.
///
/// The JavaScript:
///
/// ```js
/// export default function(values, compare) {
///   if (!(n = values.length)) return;
///   var n, i = 0, j = 0, xi, xj = values[j];
///   if (compare == null) compare = ascending;
///   while (++i < n) {
///     if (compare(xi = values[i], xj) < 0 || compare(xj, xj) !== 0) {
///       xj = xi, j = i;
///     }
///   }
///   if (compare(xj, xj) === 0) return j;
/// }
/// ```
///
/// The `compare` is a comparison of two values, and `None` stands for the
/// `NaN` the JavaScript's `ascending` gives for values that cannot be
/// compared.
pub fn scan<T>(
    values: &[T],
    compare: &dyn Fn(&T, &T) -> Option<std::cmp::Ordering>,
) -> Option<usize> {
    let n = values.len();
    if n == 0 {
        return None;
    }
    let mut j = 0usize;
    let mut xj = &values[j];
    let mut i = 0usize;
    while i + 1 < n {
        i += 1;
        let xi = &values[i];
        if compare(xi, xj) == Some(std::cmp::Ordering::Less)
            || compare(xj, xj) != Some(std::cmp::Ordering::Equal)
        {
            xj = xi;
            j = i;
        }
    }
    if compare(xj, xj) == Some(std::cmp::Ordering::Equal) {
        Some(j)
    } else {
        None
    }
}

/// d3-array's `shuffle`, which is Fisher and Yates with JavaScript's own
/// `Math.random`.
///
/// The JavaScript takes the range to shuffle, and the port takes the same
/// two bounds, with a random source, because a program that must be able
/// to draw the same shuffle twice needs to say where the numbers come
/// from.
pub fn shuffle<T>(array: &mut [T], i0: usize, i1: usize, random: &mut dyn FnMut() -> f64) {
    let mut m = i1.saturating_sub(i0);
    while m > 0 {
        m -= 1;
        let i = (random() * (m + 1) as f64) as usize;
        array.swap(m + i0, i + i0);
    }
}

/// d3-array's `ticks`.
///
/// The JavaScript:
///
/// ```js
/// export default function(start, stop, count) {
///   var reverse, i = -1, n, ticks, step;
///   stop = +stop, start = +start, count = +count;
///   if (start === stop && count > 0) return [start];
///   if (reverse = stop < start) n = start, start = stop, stop = n;
///   if ((step = tickIncrement(start, stop, count)) === 0 || !isFinite(step)) return [];
///   if (step > 0) {
///     start = Math.ceil(start / step);
///     stop = Math.floor(stop / step);
///     ticks = new Array(n = Math.ceil(stop - start + 1));
///     while (++i < n) ticks[i] = (start + i) * step;
///   } else {
///     start = Math.floor(start * step);
///     stop = Math.ceil(stop * step);
///     ticks = new Array(n = Math.ceil(start - stop + 1));
///     while (++i < n) ticks[i] = (start - i) / step;
///   }
///   if (reverse) ticks.reverse();
///   return ticks;
/// }
/// ```
pub fn ticks(start: f64, stop: f64, count: f64) -> Vec<f64> {
    let stop = js::number(stop);
    let start = js::number(start);
    let count = js::number(count);
    if start == stop && count > 0.0 {
        return vec![start];
    }
    let reverse = stop < start;
    let (start, stop) = if reverse {
        (stop, start)
    } else {
        (start, stop)
    };
    let step = tick_increment(start, stop, count);
    if step == 0.0 || !step.is_finite() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if step > 0.0 {
        let start = (start / step).ceil();
        let stop = (stop / step).floor();
        let n = (stop - start + 1.0).ceil() as usize;
        let mut i = 0usize;
        while i < n {
            out.push((start + i as f64) * step);
            i += 1;
        }
    } else {
        let start = (start * step).floor();
        let stop = (stop * step).ceil();
        let n = (start - stop + 1.0).ceil() as usize;
        let mut i = 0usize;
        while i < n {
            out.push((start - i as f64) / step);
            i += 1;
        }
    }
    if reverse {
        out.reverse();
    }
    out
}

/// d3-array's `tickIncrement`.
///
/// The JavaScript:
///
/// ```js
/// export function tickIncrement(start, stop, count) {
///   var step = (stop - start) / Math.max(0, count),
///       power = Math.floor(Math.log(step) / Math.LN10),
///       error = step / Math.pow(10, power);
///   return power >= 0
///       ? (error >= e10 ? 10 : error >= e5 ? 5 : error >= e2 ? 2 : 1) * Math.pow(10, power)
///       : -Math.pow(10, -power) / (error >= e10 ? 10 : error >= e5 ? 5 : error >= e2 ? 2 : 1);
/// }
/// ```
///
/// where `e10` is `sqrt(50)`, `e5` is `sqrt(10)` and `e2` is `sqrt(2)`.
pub fn tick_increment(start: f64, stop: f64, count: f64) -> f64 {
    let e10 = 50.0_f64.sqrt();
    let e5 = 10.0_f64.sqrt();
    let e2 = 2.0_f64.sqrt();
    let step = (js::number(stop) - js::number(start)) / js::number(count).max(0.0);
    let power = (js::number(step).ln() / std::f64::consts::LN_10).floor();
    let error = step / 10.0_f64.powf(power);
    if power >= 0.0 {
        let base = if error >= e10 {
            10.0
        } else if error >= e5 {
            5.0
        } else if error >= e2 {
            2.0
        } else {
            1.0
        };
        base * 10.0_f64.powf(power)
    } else {
        let base = if error >= e10 {
            10.0
        } else if error >= e5 {
            5.0
        } else if error >= e2 {
            2.0
        } else {
            1.0
        };
        -10.0_f64.powf(-power) / base
    }
}

/// d3-array's `tickStep`.
///
/// The JavaScript:
///
/// ```js
/// export function tickStep(start, stop, count) {
///   var step0 = Math.abs(stop - start) / Math.max(0, count),
///       step1 = Math.pow(10, Math.floor(Math.log(step0) / Math.LN10)),
///       error = step0 / step1;
///   if (error >= e10) step1 *= 10;
///   else if (error >= e5) step1 *= 5;
///   else if (error >= e2) step1 *= 2;
///   return stop < start ? -step1 : step1;
/// }
/// ```
pub fn tick_step(start: f64, stop: f64, count: f64) -> f64 {
    let e10 = 50.0_f64.sqrt();
    let e5 = 10.0_f64.sqrt();
    let e2 = 2.0_f64.sqrt();
    let stop = js::number(stop);
    let start = js::number(start);
    let step0 = (stop - start).abs() / js::number(count).max(0.0);
    let mut step1 = 10.0_f64.powf((js::number(step0).ln() / std::f64::consts::LN_10).floor());
    let error = step0 / step1;
    if error >= e10 {
        step1 *= 10.0;
    } else if error >= e5 {
        step1 *= 5.0;
    } else if error >= e2 {
        step1 *= 2.0;
    }
    if stop < start {
        -step1
    } else {
        step1
    }
}

/// d3-array's `histogram`.
///
/// The JavaScript returns the bins as an array of arrays, and each bin
/// carries its own `x0` and `x1`. The port's bin is a struct with the
/// data and the two bounds.
#[derive(Clone, Debug, PartialEq)]
pub struct Bin<T> {
    /// The data in the bin.
    pub data: Vec<T>,
    /// The lower bound of the bin.
    pub x0: f64,
    /// The upper bound of the bin.
    pub x1: f64,
}

/// d3-array's `histogram`, with the value, domain and threshold functions
/// the JavaScript's `histogram.value`, `histogram.domain` and
/// `histogram.thresholds` set.
///
/// The JavaScript's defaults are the identity, the extent, and Sturges'
/// rule.
pub fn histogram<T: Clone>(
    data: &[T],
    value: &dyn Fn(&T, usize, &[T]) -> f64,
    domain: &Domain<'_>,
    threshold: &dyn Fn(&[f64], f64, f64) -> Threshold,
) -> Vec<Bin<T>> {
    let n = data.len();
    let mut values = Vec::with_capacity(n);
    for (i, datum) in data.iter().enumerate() {
        values.push(value(datum, i, data));
    }
    let (x0, x1) = domain(&values);
    let (x0, x1) = match (x0, x1) {
        (Some(a), Some(b)) => (a, b),
        // The JavaScript's `undefined` bounds give a domain of `undefined`,
        // and every comparison against it is false, so the bins are empty.
        _ => return Vec::new(),
    };
    let mut thresholds = match threshold(&values, x0, x1) {
        Threshold::Count(count) => {
            // Convert number of thresholds into uniform thresholds.
            let tz = tick_step(x0, x1, count);
            range((x0 / tz).ceil() * tz, Some(x1), Some(tz))
        }
        Threshold::Values(values) => values,
    };
    // Remove any thresholds outside the domain.
    while thresholds.first().is_some_and(|t| *t <= x0) {
        thresholds.remove(0);
    }
    while thresholds.last().is_some_and(|t| *t > x1) {
        thresholds.pop();
    }
    let m = thresholds.len();
    let mut bins: Vec<Bin<T>> = Vec::with_capacity(m + 1);
    for i in 0..=m {
        bins.push(Bin {
            data: Vec::new(),
            x0: if i > 0 { thresholds[i - 1] } else { x0 },
            x1: if i < m { thresholds[i] } else { x1 },
        });
    }
    // Assign data to bins by value, ignoring any outside the domain.
    for (i, datum) in data.iter().enumerate() {
        let x = values[i];
        if x0 <= x && x <= x1 {
            let compare = |p: &f64, q: &f64| ascending(*p, *q);
            let index = bisector::bisector(&compare).right(&thresholds, &x, 0, m);
            bins[index].data.push(datum.clone());
        }
    }
    bins
}

/// What a histogram's threshold function gives: a number of thresholds, or
/// the thresholds themselves.
pub enum Threshold {
    /// A number of thresholds, to be spread evenly.
    Count(f64),
    /// The thresholds themselves.
    Values(Vec<f64>),
}

/// d3-array's `thresholdSturges`.
///
/// The JavaScript: `Math.ceil(Math.log(values.length) / Math.LN2) + 1`.
pub fn threshold_sturges(values: &[f64]) -> f64 {
    (js::number(values.len() as f64).ln() / std::f64::consts::LN_2).ceil() + 1.0
}

/// d3-array's `thresholdScott`.
///
/// The JavaScript: `Math.ceil((max - min) / (3.5 * deviation(values) *
/// Math.pow(values.length, -1 / 3)))`.
pub fn threshold_scott(values: &[f64], min: f64, max: f64) -> f64 {
    let deviation = deviation(values).unwrap_or(0.0);
    ((max - min) / (3.5 * deviation * (values.len() as f64).powf(-1.0 / 3.0))).ceil()
}

/// d3-array's `thresholdFreedmanDiaconis`.
///
/// The JavaScript: `Math.ceil((max - min) / (2 * (quantile(values, 0.75) -
/// quantile(values, 0.25)) * Math.pow(values.length, -1 / 3)))`.
pub fn threshold_freedman_diaconis(values: &[f64], min: f64, max: f64) -> f64 {
    let mut numbers: Vec<f64> = values.iter().copied().filter(|v| !v.is_nan()).collect();
    numbers.sort_by(|a, b| ascending(*a, *b).unwrap_or(std::cmp::Ordering::Equal));
    let q75 = quantile_sorted(&numbers, 0.75).unwrap_or(0.0);
    let q25 = quantile_sorted(&numbers, 0.25).unwrap_or(0.0);
    ((max - min) / (2.0 * (q75 - q25) * (values.len() as f64).powf(-1.0 / 3.0))).ceil()
}
