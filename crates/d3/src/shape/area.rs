//! d3-shape's `area`, from the JavaScript of the same name.
//!
//! Translated from `d3-shape/src/area.js`, version 1.3.7, of d3 5.16.0.
//! The JavaScript is the authority.

use super::curve::CurveFactory;
use super::line::{Accessor, Defined};
use super::Context;
use crate::path::Path;

/// d3-shape's `area`.
///
/// The JavaScript:
///
/// ```js
/// function area(data) {
///   var i, j, k, n = data.length, d, defined0 = false, buffer,
///       x0z = new Array(n), y0z = new Array(n);
///   if (context == null) output = curve(buffer = path());
///   for (i = 0; i <= n; ++i) {
///     if (!(i < n && defined(d = data[i], i, data)) === defined0) {
///       if (defined0 = !defined0) {
///         j = i;
///         output.areaStart();
///         output.lineStart();
///       } else {
///         output.lineEnd();
///         output.lineStart();
///         for (k = i - 1; k >= j; --k) {
///           output.point(x0z[k], y0z[k]);
///         }
///         output.lineEnd();
///         output.areaEnd();
///       }
///     }
///     if (defined0) {
///       x0z[i] = +x0(d, i, data), y0z[i] = +y0(d, i, data);
///       output.point(x1 ? +x1(d, i, data) : x0z[i], y1 ? +y1(d, i, data) : y0z[i]);
///     }
///   }
///   if (buffer) return output = null, buffer + "" || null;
/// }
/// ```
///
/// The JavaScript's `area.x` sets both the inner and the outer edge, and
/// `area.x0` and `area.x1` set them one at a time. The port has the same
/// three, and `x` clears the outer one, as the JavaScript does.
pub struct Area<T> {
    x0: Accessor<T>,
    x1: Option<Accessor<T>>,
    y0: Accessor<T>,
    y1: Option<Accessor<T>>,
    defined: Defined<T>,
    curve: CurveFactory,
}

impl<T: AsRef<[f64]>> Default for Area<T> {
    fn default() -> Self {
        Self {
            x0: Box::new(|p: &T, _, _| super::point_x(p.as_ref())),
            x1: None,
            y0: Box::new(|_, _, _| 0.0),
            y1: None,
            defined: Box::new(|_, _, _| true),
            curve: super::curve::curve_linear,
        }
    }
}

impl<T> Area<T> {
    /// The JavaScript's `area.x`, which sets the inner edge and clears
    /// the outer one.
    pub fn x<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.x0 = Box::new(f);
        self.x1 = None;
        self
    }

    /// The JavaScript's `area.x0`.
    pub fn x0<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.x0 = Box::new(f);
        self
    }

    /// The JavaScript's `area.x1`.
    pub fn x1<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.x1 = Some(Box::new(f));
        self
    }

    /// The JavaScript's `area.y`, which sets the inner edge and clears
    /// the outer one.
    pub fn y<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.y0 = Box::new(f);
        self.y1 = None;
        self
    }

    /// The JavaScript's `area.y0`.
    pub fn y0<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.y0 = Box::new(f);
        self
    }

    /// The JavaScript's `area.y1`.
    pub fn y1<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.y1 = Some(Box::new(f));
        self
    }

    /// The JavaScript's `area.defined`.
    pub fn defined<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> bool + 'static,
    {
        self.defined = Box::new(f);
        self
    }

    /// The JavaScript's `area.curve`.
    pub fn curve(mut self, curve: CurveFactory) -> Self {
        self.curve = curve;
        self
    }

    /// The JavaScript's `area(data)`: the path of the area.
    pub fn apply(&self, data: &[T]) -> Option<String> {
        let mut buffer = Path::default();
        let mut output = (self.curve)();
        let n = data.len();
        let mut defined0 = false;
        let mut j = 0usize;
        let mut x0z = vec![0.0; n];
        let mut y0z = vec![0.0; n];
        for i in 0..=n {
            let defined = i < n && (self.defined)(&data[i], i, data);
            if defined != defined0 {
                defined0 = !defined0;
                if defined0 {
                    j = i;
                    output.area_start(&mut buffer);
                    output.line_start(&mut buffer);
                } else {
                    output.line_end(&mut buffer);
                    output.line_start(&mut buffer);
                    let mut k = i as isize - 1;
                    while k >= j as isize {
                        output.point(&mut buffer, x0z[k as usize], y0z[k as usize]);
                        k -= 1;
                    }
                    output.line_end(&mut buffer);
                    output.area_end(&mut buffer);
                }
            }
            if defined0 {
                let d = &data[i];
                x0z[i] = (self.x0)(d, i, data);
                y0z[i] = (self.y0)(d, i, data);
                let x = match &self.x1 {
                    Some(f) => f(d, i, data),
                    None => x0z[i],
                };
                let y = match &self.y1 {
                    Some(f) => f(d, i, data),
                    None => y0z[i],
                };
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

    /// The JavaScript's `area(data)` drawn into a context the caller
    /// holds, which is the JavaScript's `area.context`.
    pub fn apply_to(&self, data: &[T], context: &mut dyn Context) {
        let mut output = (self.curve)();
        let n = data.len();
        let mut defined0 = false;
        let mut j = 0usize;
        let mut x0z = vec![0.0; n];
        let mut y0z = vec![0.0; n];
        for i in 0..=n {
            let defined = i < n && (self.defined)(&data[i], i, data);
            if defined != defined0 {
                defined0 = !defined0;
                if defined0 {
                    j = i;
                    output.area_start(context);
                    output.line_start(context);
                } else {
                    output.line_end(context);
                    output.line_start(context);
                    let mut k = i as isize - 1;
                    while k >= j as isize {
                        output.point(context, x0z[k as usize], y0z[k as usize]);
                        k -= 1;
                    }
                    output.line_end(context);
                    output.area_end(context);
                }
            }
            if defined0 {
                let d = &data[i];
                x0z[i] = (self.x0)(d, i, data);
                y0z[i] = (self.y0)(d, i, data);
                let x = match &self.x1 {
                    Some(f) => f(d, i, data),
                    None => x0z[i],
                };
                let y = match &self.y1 {
                    Some(f) => f(d, i, data),
                    None => y0z[i],
                };
                output.point(context, x, y);
            }
        }
    }
}
