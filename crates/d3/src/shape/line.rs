//! d3-shape's `line`, from the JavaScript of the same name.
//!
//! Translated from `d3-shape/src/line.js`, version 1.3.7, of d3 5.16.0.
//! The JavaScript is the authority.

use super::curve::{Curve, CurveFactory, Linear};
use super::{point_x, point_y, Context};
use crate::path::Path;

/// A function that reads a number out of a datum, and its place.
pub type Accessor<T> = Box<dyn Fn(&T, usize, &[T]) -> f64>;

/// A function that says whether a datum is drawn at all.
pub type Defined<T> = Box<dyn Fn(&T, usize, &[T]) -> bool>;

/// d3-shape's `line`.
///
/// The JavaScript:
///
/// ```js
/// function line(data) {
///   var i, n = data.length, d, defined0 = false, buffer;
///   if (context == null) output = curve(buffer = path());
///   for (i = 0; i <= n; ++i) {
///     if (!(i < n && defined(d = data[i], i, data)) === defined0) {
///       if (defined0 = !defined0) output.lineStart();
///       else output.lineEnd();
///     }
///     if (defined0) output.point(+x(d, i, data), +y(d, i, data));
///   }
///   if (buffer) return output = null, buffer + "" || null;
/// }
/// ```
///
/// The JavaScript's `line.x` takes either a function or a number, and
/// makes a constant function of a number. The port's `x` takes a closure,
/// and [`constant`] is there for the number.
pub struct Line<T> {
    x: Accessor<T>,
    y: Accessor<T>,
    defined: Defined<T>,
    curve: CurveFactory,
}

/// The JavaScript's `point.x` and `point.y` read a pair, and the port's
/// default does the same, so a line of pairs is made with `Line::new`.
impl<T: AsRef<[f64]>> Default for Line<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: AsRef<[f64]>> Line<T> {
    /// A line that reads the horizontal of a point.
    pub fn new() -> Self {
        Self {
            x: Box::new(|p: &T, _, _| point_x(p.as_ref())),
            y: Box::new(|p: &T, _, _| point_y(p.as_ref())),
            defined: Box::new(|_, _, _| true),
            curve: super::curve::curve_linear,
        }
    }
}

/// The JavaScript's `constant`, which makes a function of no arguments
/// that gives a value.
pub fn constant<T: 'static>(value: f64) -> impl Fn(&T, usize, &[T]) -> f64 {
    move |_, _, _| value
}

impl<T> Line<T> {
    /// The JavaScript's `line.x`.
    pub fn x<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.x = Box::new(f);
        self
    }

    /// The JavaScript's `line.y`.
    pub fn y<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.y = Box::new(f);
        self
    }

    /// The JavaScript's `line.defined`, which says whether a point is
    /// drawn at all.
    pub fn defined<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> bool + 'static,
    {
        self.defined = Box::new(f);
        self
    }

    /// The JavaScript's `line.curve`.
    pub fn curve(mut self, curve: CurveFactory) -> Self {
        self.curve = curve;
        self
    }

    /// The JavaScript's `line(data)`: the path of the line.
    ///
    /// The JavaScript returns `null` when the data is empty, and the port
    /// does the same.
    pub fn apply(&self, data: &[T]) -> Option<String> {
        let mut buffer = Path::default();
        let mut output = (self.curve)();
        let n = data.len();
        let mut defined0 = false;
        for i in 0..=n {
            let defined = i < n && (self.defined)(&data[i], i, data);
            if defined != defined0 {
                defined0 = !defined0;
                if defined0 {
                    output.line_start(&mut buffer);
                } else {
                    output.line_end(&mut buffer);
                }
            }
            if defined0 {
                let d = &data[i];
                let x = (self.x)(d, i, data);
                let y = (self.y)(d, i, data);
                output.point(&mut buffer, x, y);
            }
        }
        let text = buffer.text().to_owned();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// The JavaScript's `line(data)` drawn into a context the caller
    /// holds, which is the JavaScript's `line.context`.
    pub fn apply_to(&self, data: &[T], context: &mut dyn Context) {
        let mut output = (self.curve)();
        let n = data.len();
        let mut defined0 = false;
        for i in 0..=n {
            let defined = i < n && (self.defined)(&data[i], i, data);
            if defined != defined0 {
                defined0 = !defined0;
                if defined0 {
                    output.line_start(context);
                } else {
                    output.line_end(context);
                }
            }
            if defined0 {
                let d = &data[i];
                let x = (self.x)(d, i, data);
                let y = (self.y)(d, i, data);
                output.point(context, x, y);
            }
        }
    }
}

/// The JavaScript's `curveLinear`, kept here because the line's default
/// curve is it.
pub fn default_curve() -> Box<dyn Curve> {
    Box::new(Linear::default())
}
