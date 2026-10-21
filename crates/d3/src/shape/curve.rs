//! The curves of d3-shape, from the JavaScript of the same name.
//!
//! Translated from `d3-shape/src/curve/*.js`, version 1.3.7, of d3
//! 5.16.0. The JavaScript is the authority; where the two could be read
//! differently, the JavaScript is right.
//!
//! # The shape of a curve
//!
//! In the JavaScript a curve is an object that holds a context and a
//! handful of numbers, and it is a small state machine with five calls:
//! `areaStart`, `areaEnd`, `lineStart`, `lineEnd` and `point`. The line
//! and the area call those in order, and the curve writes into the
//! context.
//!
//! The port keeps that exactly. A curve is a struct with the JavaScript's
//! own fields, the context is given to each of the five calls, and the
//! order of the calls and the tests inside them are the JavaScript's.
//!
//! The `_line` field is worth a word, because it is the one piece of state
//! that is not a number or a flag. The JavaScript gives it `0` when an
//! area starts, `NaN` when it ends, and flips it between `0` and `1` at
//! the end of each line. It is what tells a curve whether it is drawing
//! the top of an area or a line of its own, and the port keeps it as the
//! same number.

use super::Context;
use crate::js;
use crate::js::truthy;

/// The epsilon the JavaScript compares against, from `d3-shape/src/math.js`.
const EPSILON: f64 = 1e-12;

/// A curve, which draws a line or the edge of an area.
pub trait Curve {
    /// The JavaScript's `areaStart`.
    fn area_start(&mut self, context: &mut dyn Context);
    /// The JavaScript's `areaEnd`.
    fn area_end(&mut self, context: &mut dyn Context);
    /// The JavaScript's `lineStart`.
    fn line_start(&mut self, context: &mut dyn Context);
    /// The JavaScript's `lineEnd`.
    fn line_end(&mut self, context: &mut dyn Context);
    /// The JavaScript's `point`.
    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64);
}

/// A curve that makes another curve, which is what the JavaScript's
/// `curve(context)` is.
pub type CurveFactory = fn() -> Box<dyn Curve>;

/// The JavaScript's `line` flag: zero when an area starts, `NaN` when it
/// ends, and one or zero otherwise.
fn flip_line(line: f64) -> f64 {
    1.0 - line
}

/// d3-shape's `curveLinear`.
///
/// The JavaScript:
///
/// ```js
/// point: function(x, y) {
///   x = +x, y = +y;
///   switch (this._point) {
///     case 0: this._point = 1; this._line ? this._context.lineTo(x, y) : this._context.moveTo(x, y); break;
///     case 1: this._point = 2; // proceed
///     default: this._context.lineTo(x, y); break;
///   }
/// }
/// ```
pub struct Linear {
    line: f64,
    point: u8,
}

impl Default for Linear {
    fn default() -> Self {
        Self {
            line: f64::NAN,
            point: 0,
        }
    }
}

impl Curve for Linear {
    fn area_start(&mut self, _context: &mut dyn Context) {
        self.line = 0.0;
    }

    fn area_end(&mut self, _context: &mut dyn Context) {
        self.line = f64::NAN;
    }

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        if truthy(self.line) || (!js::strict_equals(self.line, 0.0) && self.point == 1) {
            context.close_path();
        }
        self.line = flip_line(self.line);
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        match self.point {
            0 => {
                self.point = 1;
                if truthy(self.line) {
                    context.line_to(x, y);
                } else {
                    context.move_to(x, y);
                }
            }
            1 => {
                self.point = 2;
                context.line_to(x, y);
            }
            _ => context.line_to(x, y),
        }
    }
}

/// d3-shape's `curveLinearClosed`.
#[derive(Default)]
pub struct LinearClosed {
    point: u8,
}

impl Curve for LinearClosed {
    fn area_start(&mut self, _context: &mut dyn Context) {}
    fn area_end(&mut self, _context: &mut dyn Context) {}

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        if self.point != 0 {
            context.close_path();
        }
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        if self.point != 0 {
            context.line_to(x, y);
        } else {
            self.point = 1;
            context.move_to(x, y);
        }
    }
}

/// d3-shape's `curveNatural`.
///
/// The JavaScript computes the control points of a natural cubic spline
/// in `controlPoints`, and the port keeps that function as it is.
pub struct Natural {
    line: f64,
    x: Vec<f64>,
    y: Vec<f64>,
}

impl Default for Natural {
    fn default() -> Self {
        Self {
            line: f64::NAN,
            x: Vec::new(),
            y: Vec::new(),
        }
    }
}

impl Curve for Natural {
    fn area_start(&mut self, _context: &mut dyn Context) {
        self.line = 0.0;
    }

    fn area_end(&mut self, _context: &mut dyn Context) {
        self.line = f64::NAN;
    }

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.x.clear();
        self.y.clear();
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        let n = self.x.len();
        if n > 0 {
            if truthy(self.line) {
                context.line_to(self.x[0], self.y[0]);
            } else {
                context.move_to(self.x[0], self.y[0]);
            }
            if n == 2 {
                context.line_to(self.x[1], self.y[1]);
            } else {
                let px = control_points(&self.x);
                let py = control_points(&self.y);
                for i0 in 0..n.saturating_sub(1) {
                    context.bezier_curve_to(
                        px.0[i0],
                        py.0[i0],
                        px.1[i0],
                        py.1[i0],
                        self.x[i0 + 1],
                        self.y[i0 + 1],
                    );
                }
            }
        }
        if truthy(self.line) || (!js::strict_equals(self.line, 0.0) && n == 1) {
            context.close_path();
        }
        self.line = flip_line(self.line);
        self.x.clear();
        self.y.clear();
    }

    fn point(&mut self, _context: &mut dyn Context, x: f64, y: f64) {
        self.x.push(x);
        self.y.push(y);
    }
}

/// The JavaScript's `controlPoints`, which gives the two sets of control
/// points of a natural cubic spline.
///
/// The JavaScript:
///
/// ```js
/// function controlPoints(x) {
///   var i, n = x.length - 1, m, a = new Array(n), b = new Array(n), r = new Array(n);
///   a[0] = 0, b[0] = 2, r[0] = x[0] + 2 * x[1];
///   for (i = 1; i < n - 1; ++i) a[i] = 1, b[i] = 4, r[i] = 4 * x[i] + 2 * x[i + 1];
///   a[n - 1] = 2, b[n - 1] = 7, r[n - 1] = 8 * x[n - 1] + x[n];
///   for (i = 1; i < n; ++i) m = a[i] / b[i - 1], b[i] -= m, r[i] -= m * r[i - 1];
///   a[n - 1] = r[n - 1] / b[n - 1];
///   for (i = n - 2; i >= 0; --i) a[i] = (r[i] - a[i + 1]) / b[i];
///   b[n - 1] = (x[n] + a[n - 1]) / 2;
///   for (i = 0; i < n - 1; ++i) b[i] = 2 * x[i + 1] - a[i + 1];
///   return [a, b];
/// }
/// ```
pub fn control_points(x: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = x.len().saturating_sub(1);
    if n == 0 {
        return (Vec::new(), Vec::new());
    }
    let mut a = vec![0.0; n];
    let mut b = vec![0.0; n];
    let mut r = vec![0.0; n];
    a[0] = 0.0;
    b[0] = 2.0;
    r[0] = x[0] + 2.0 * x[1];
    for i in 1..n.saturating_sub(1) {
        a[i] = 1.0;
        b[i] = 4.0;
        r[i] = 4.0 * x[i] + 2.0 * x[i + 1];
    }
    a[n - 1] = 2.0;
    b[n - 1] = 7.0;
    r[n - 1] = 8.0 * x[n - 1] + x[n];
    for i in 1..n {
        let m = a[i] / b[i - 1];
        b[i] -= m;
        r[i] -= m * r[i - 1];
    }
    a[n - 1] = r[n - 1] / b[n - 1];
    for i in (0..n - 1).rev() {
        a[i] = (r[i] - a[i + 1]) / b[i];
    }
    b[n - 1] = (x[n] + a[n - 1]) / 2.0;
    for i in 0..n - 1 {
        b[i] = 2.0 * x[i + 1] - a[i + 1];
    }
    (a, b)
}

/// d3-shape's `curveCardinal`.
///
/// The JavaScript's `point`, which is shared with the open and the closed
/// cardinal, is here as [`cardinal_point`].
pub struct Cardinal {
    line: f64,
    k: f64,
    x0: f64,
    x1: f64,
    x2: f64,
    y0: f64,
    y1: f64,
    y2: f64,
    point: u8,
}

impl Cardinal {
    /// The JavaScript's `cardinal.tension`, which makes a curve with a
    /// given tension.
    pub fn new(tension: f64) -> Self {
        Self {
            line: f64::NAN,
            k: (1.0 - tension) / 6.0,
            x0: f64::NAN,
            x1: f64::NAN,
            x2: f64::NAN,
            y0: f64::NAN,
            y1: f64::NAN,
            y2: f64::NAN,
            point: 0,
        }
    }
}

impl Default for Cardinal {
    fn default() -> Self {
        Self::new(0.0)
    }
}

/// The JavaScript's cardinal `point`, shared by the three cardinal curves.
///
/// The JavaScript:
///
/// ```js
/// export function point(that, x, y) {
///   that._context.bezierCurveTo(
///     that._x1 + that._k * (that._x2 - that._x0),
///     that._y1 + that._k * (that._y2 - that._y0),
///     that._x2 + that._k * (that._x1 - x),
///     that._y2 + that._k * (that._y1 - y),
///     that._x2, that._y2
///   );
/// }
/// ```
fn cardinal_point(
    context: &mut dyn Context,
    k: f64,
    x0: f64,
    x1: f64,
    x2: f64,
    y0: f64,
    y1: f64,
    y2: f64,
    x: f64,
    y: f64,
) {
    context.bezier_curve_to(
        x1 + k * (x2 - x0),
        y1 + k * (y2 - y0),
        x2 + k * (x1 - x),
        y2 + k * (y1 - y),
        x2,
        y2,
    );
}

impl Curve for Cardinal {
    fn area_start(&mut self, _context: &mut dyn Context) {
        self.line = 0.0;
    }

    fn area_end(&mut self, _context: &mut dyn Context) {
        self.line = f64::NAN;
    }

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.x0 = f64::NAN;
        self.x1 = f64::NAN;
        self.x2 = f64::NAN;
        self.y0 = f64::NAN;
        self.y1 = f64::NAN;
        self.y2 = f64::NAN;
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        match self.point {
            2 => context.line_to(self.x2, self.y2),
            3 => cardinal_point(
                context, self.k, self.x0, self.x1, self.x2, self.y0, self.y1, self.y2, self.x1,
                self.y1,
            ),
            _ => {}
        }
        if truthy(self.line) || (!js::strict_equals(self.line, 0.0) && self.point == 1) {
            context.close_path();
        }
        self.line = flip_line(self.line);
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        match self.point {
            0 => {
                self.point = 1;
                if truthy(self.line) {
                    context.line_to(x, y);
                } else {
                    context.move_to(x, y);
                }
            }
            1 => {
                self.point = 2;
                self.x1 = x;
                self.y1 = y;
            }
            2 => {
                self.point = 3;
                cardinal_point(
                    context, self.k, self.x0, self.x1, self.x2, self.y0, self.y1, self.y2, x, y,
                );
            }
            _ => cardinal_point(
                context, self.k, self.x0, self.x1, self.x2, self.y0, self.y1, self.y2, x, y,
            ),
        }
        self.x0 = self.x1;
        self.x1 = self.x2;
        self.x2 = x;
        self.y0 = self.y1;
        self.y1 = self.y2;
        self.y2 = y;
    }
}

/// d3-shape's `curveCardinalOpen`.
pub struct CardinalOpen {
    line: f64,
    k: f64,
    x0: f64,
    x1: f64,
    x2: f64,
    y0: f64,
    y1: f64,
    y2: f64,
    point: u8,
}

impl CardinalOpen {
    /// The JavaScript's `cardinalOpen.tension`.
    pub fn new(tension: f64) -> Self {
        Self {
            line: f64::NAN,
            k: (1.0 - tension) / 6.0,
            x0: f64::NAN,
            x1: f64::NAN,
            x2: f64::NAN,
            y0: f64::NAN,
            y1: f64::NAN,
            y2: f64::NAN,
            point: 0,
        }
    }
}

impl Default for CardinalOpen {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl Curve for CardinalOpen {
    fn area_start(&mut self, _context: &mut dyn Context) {
        self.line = 0.0;
    }

    fn area_end(&mut self, _context: &mut dyn Context) {
        self.line = f64::NAN;
    }

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.x0 = f64::NAN;
        self.x1 = f64::NAN;
        self.x2 = f64::NAN;
        self.y0 = f64::NAN;
        self.y1 = f64::NAN;
        self.y2 = f64::NAN;
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        if truthy(self.line) || (!js::strict_equals(self.line, 0.0) && self.point == 3) {
            context.close_path();
        }
        self.line = flip_line(self.line);
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        match self.point {
            0 => self.point = 1,
            1 => self.point = 2,
            2 => {
                self.point = 3;
                if truthy(self.line) {
                    context.line_to(self.x2, self.y2);
                } else {
                    context.move_to(self.x2, self.y2);
                }
            }
            3 => {
                self.point = 4;
                cardinal_point(
                    context, self.k, self.x0, self.x1, self.x2, self.y0, self.y1, self.y2, x, y,
                );
            }
            _ => cardinal_point(
                context, self.k, self.x0, self.x1, self.x2, self.y0, self.y1, self.y2, x, y,
            ),
        }
        self.x0 = self.x1;
        self.x1 = self.x2;
        self.x2 = x;
        self.y0 = self.y1;
        self.y1 = self.y2;
        self.y2 = y;
    }
}

/// d3-shape's `curveCardinalClosed`.
pub struct CardinalClosed {
    k: f64,
    x0: f64,
    x1: f64,
    x2: f64,
    x3: f64,
    x4: f64,
    x5: f64,
    y0: f64,
    y1: f64,
    y2: f64,
    y3: f64,
    y4: f64,
    y5: f64,
    point: u8,
}

impl CardinalClosed {
    /// The JavaScript's `cardinalClosed.tension`.
    pub fn new(tension: f64) -> Self {
        Self {
            k: (1.0 - tension) / 6.0,
            x0: f64::NAN,
            x1: f64::NAN,
            x2: f64::NAN,
            x3: f64::NAN,
            x4: f64::NAN,
            x5: f64::NAN,
            y0: f64::NAN,
            y1: f64::NAN,
            y2: f64::NAN,
            y3: f64::NAN,
            y4: f64::NAN,
            y5: f64::NAN,
            point: 0,
        }
    }
}

impl Default for CardinalClosed {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl Curve for CardinalClosed {
    fn area_start(&mut self, _context: &mut dyn Context) {}
    fn area_end(&mut self, _context: &mut dyn Context) {}

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.x0 = f64::NAN;
        self.x1 = f64::NAN;
        self.x2 = f64::NAN;
        self.x3 = f64::NAN;
        self.x4 = f64::NAN;
        self.x5 = f64::NAN;
        self.y0 = f64::NAN;
        self.y1 = f64::NAN;
        self.y2 = f64::NAN;
        self.y3 = f64::NAN;
        self.y4 = f64::NAN;
        self.y5 = f64::NAN;
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        match self.point {
            1 => {
                context.move_to(self.x3, self.y3);
                context.close_path();
            }
            2 => {
                context.line_to(self.x3, self.y3);
                context.close_path();
            }
            3 => {
                self.point(context, self.x3, self.y3);
                self.point(context, self.x4, self.y4);
                self.point(context, self.x5, self.y5);
            }
            _ => {}
        }
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        match self.point {
            0 => {
                self.point = 1;
                self.x3 = x;
                self.y3 = y;
            }
            1 => {
                self.point = 2;
                self.x4 = x;
                self.y4 = y;
                context.move_to(self.x4, self.y4);
            }
            2 => {
                self.point = 3;
                self.x5 = x;
                self.y5 = y;
            }
            _ => cardinal_point(
                context, self.k, self.x0, self.x1, self.x2, self.y0, self.y1, self.y2, x, y,
            ),
        }
        self.x0 = self.x1;
        self.x1 = self.x2;
        self.x2 = x;
        self.y0 = self.y1;
        self.y1 = self.y2;
        self.y2 = y;
    }
}

/// d3-shape's `curveBasis`.
pub struct Basis {
    line: f64,
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    point: u8,
}

impl Default for Basis {
    fn default() -> Self {
        Self {
            line: f64::NAN,
            x0: f64::NAN,
            x1: f64::NAN,
            y0: f64::NAN,
            y1: f64::NAN,
            point: 0,
        }
    }
}

/// The JavaScript's basis `point`, shared by the three basis curves.
fn basis_point(context: &mut dyn Context, x0: f64, x1: f64, y0: f64, y1: f64, x: f64, y: f64) {
    context.bezier_curve_to(
        (2.0 * x0 + x1) / 3.0,
        (2.0 * y0 + y1) / 3.0,
        (x0 + 2.0 * x1) / 3.0,
        (y0 + 2.0 * y1) / 3.0,
        (x0 + 4.0 * x1 + x) / 6.0,
        (y0 + 4.0 * y1 + y) / 6.0,
    );
}

impl Curve for Basis {
    fn area_start(&mut self, _context: &mut dyn Context) {
        self.line = 0.0;
    }

    fn area_end(&mut self, _context: &mut dyn Context) {
        self.line = f64::NAN;
    }

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.x0 = f64::NAN;
        self.x1 = f64::NAN;
        self.y0 = f64::NAN;
        self.y1 = f64::NAN;
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        match self.point {
            3 => {
                // The JavaScript's `case 3:` falls through to `case 2:`,
                // so the point is drawn and then the line to it.
                basis_point(
                    context, self.x0, self.x1, self.y0, self.y1, self.x1, self.y1,
                );
                context.line_to(self.x1, self.y1);
            }
            2 => context.line_to(self.x1, self.y1),
            _ => {}
        }
        if truthy(self.line) || (!js::strict_equals(self.line, 0.0) && self.point == 1) {
            context.close_path();
        }
        self.line = flip_line(self.line);
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        match self.point {
            0 => {
                self.point = 1;
                if truthy(self.line) {
                    context.line_to(x, y);
                } else {
                    context.move_to(x, y);
                }
            }
            1 => self.point = 2,
            2 => {
                self.point = 3;
                context.line_to(
                    (5.0 * self.x0 + self.x1) / 6.0,
                    (5.0 * self.y0 + self.y1) / 6.0,
                );
                basis_point(context, self.x0, self.x1, self.y0, self.y1, x, y);
            }
            _ => basis_point(context, self.x0, self.x1, self.y0, self.y1, x, y),
        }
        self.x0 = self.x1;
        self.x1 = x;
        self.y0 = self.y1;
        self.y1 = y;
    }
}

/// d3-shape's `curveBasisOpen`.
pub struct BasisOpen {
    line: f64,
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    point: u8,
}

impl Default for BasisOpen {
    fn default() -> Self {
        Self {
            line: f64::NAN,
            x0: f64::NAN,
            x1: f64::NAN,
            y0: f64::NAN,
            y1: f64::NAN,
            point: 0,
        }
    }
}

impl Curve for BasisOpen {
    fn area_start(&mut self, _context: &mut dyn Context) {
        self.line = 0.0;
    }

    fn area_end(&mut self, _context: &mut dyn Context) {
        self.line = f64::NAN;
    }

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.x0 = f64::NAN;
        self.x1 = f64::NAN;
        self.y0 = f64::NAN;
        self.y1 = f64::NAN;
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        if truthy(self.line) || (!js::strict_equals(self.line, 0.0) && self.point == 3) {
            context.close_path();
        }
        self.line = flip_line(self.line);
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        match self.point {
            0 => self.point = 1,
            1 => self.point = 2,
            2 => {
                self.point = 3;
                let x0 = (self.x0 + 4.0 * self.x1 + x) / 6.0;
                let y0 = (self.y0 + 4.0 * self.y1 + y) / 6.0;
                if truthy(self.line) {
                    context.line_to(x0, y0);
                } else {
                    context.move_to(x0, y0);
                }
            }
            3 => {
                self.point = 4;
                basis_point(context, self.x0, self.x1, self.y0, self.y1, x, y);
            }
            _ => basis_point(context, self.x0, self.x1, self.y0, self.y1, x, y),
        }
        self.x0 = self.x1;
        self.x1 = x;
        self.y0 = self.y1;
        self.y1 = y;
    }
}

/// d3-shape's `curveBasisClosed`.
pub struct BasisClosed {
    x0: f64,
    x1: f64,
    x2: f64,
    x3: f64,
    x4: f64,
    y0: f64,
    y1: f64,
    y2: f64,
    y3: f64,
    y4: f64,
    point: u8,
}

impl Default for BasisClosed {
    fn default() -> Self {
        Self {
            x0: f64::NAN,
            x1: f64::NAN,
            x2: f64::NAN,
            x3: f64::NAN,
            x4: f64::NAN,
            y0: f64::NAN,
            y1: f64::NAN,
            y2: f64::NAN,
            y3: f64::NAN,
            y4: f64::NAN,
            point: 0,
        }
    }
}

impl Curve for BasisClosed {
    fn area_start(&mut self, _context: &mut dyn Context) {}
    fn area_end(&mut self, _context: &mut dyn Context) {}

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.x0 = f64::NAN;
        self.x1 = f64::NAN;
        self.x2 = f64::NAN;
        self.x3 = f64::NAN;
        self.x4 = f64::NAN;
        self.y0 = f64::NAN;
        self.y1 = f64::NAN;
        self.y2 = f64::NAN;
        self.y3 = f64::NAN;
        self.y4 = f64::NAN;
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        match self.point {
            1 => {
                context.move_to(self.x2, self.y2);
                context.close_path();
            }
            2 => {
                context.move_to(
                    (self.x2 + 2.0 * self.x3) / 3.0,
                    (self.y2 + 2.0 * self.y3) / 3.0,
                );
                context.line_to(
                    (self.x3 + 2.0 * self.x2) / 3.0,
                    (self.y3 + 2.0 * self.y2) / 3.0,
                );
                context.close_path();
            }
            3 => {
                self.point(context, self.x2, self.y2);
                self.point(context, self.x3, self.y3);
                self.point(context, self.x4, self.y4);
            }
            _ => {}
        }
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        match self.point {
            0 => {
                self.point = 1;
                self.x2 = x;
                self.y2 = y;
            }
            1 => {
                self.point = 2;
                self.x3 = x;
                self.y3 = y;
            }
            2 => {
                self.point = 3;
                self.x4 = x;
                self.y4 = y;
                context.move_to(
                    (self.x0 + 4.0 * self.x1 + x) / 6.0,
                    (self.y0 + 4.0 * self.y1 + y) / 6.0,
                );
            }
            _ => basis_point(context, self.x0, self.x1, self.y0, self.y1, x, y),
        }
        self.x0 = self.x1;
        self.x1 = x;
        self.y0 = self.y1;
        self.y1 = y;
    }
}

/// d3-shape's `curveStep`, and its `stepAfter` and `stepBefore`.
pub struct Step {
    line: f64,
    t: f64,
    x: f64,
    y: f64,
    point: u8,
}

impl Step {
    /// The JavaScript's `step`, whose `t` is a half.
    pub fn new(t: f64) -> Self {
        Self {
            line: f64::NAN,
            t,
            x: f64::NAN,
            y: f64::NAN,
            point: 0,
        }
    }
}

impl Default for Step {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl Curve for Step {
    fn area_start(&mut self, _context: &mut dyn Context) {
        self.line = 0.0;
    }

    fn area_end(&mut self, _context: &mut dyn Context) {
        self.line = f64::NAN;
    }

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.x = f64::NAN;
        self.y = f64::NAN;
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        if 0.0 < self.t && self.t < 1.0 && self.point == 2 {
            context.line_to(self.x, self.y);
        }
        if truthy(self.line) || (!js::strict_equals(self.line, 0.0) && self.point == 1) {
            context.close_path();
        }
        if !self.line.is_nan() && self.line >= 0.0 {
            self.t = 1.0 - self.t;
            self.line = flip_line(self.line);
        }
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        match self.point {
            0 => {
                self.point = 1;
                if truthy(self.line) {
                    context.line_to(x, y);
                } else {
                    context.move_to(x, y);
                }
            }
            1 => {
                self.point = 2;
                if self.t <= 0.0 {
                    context.line_to(self.x, y);
                    context.line_to(x, y);
                } else {
                    let x1 = self.x * (1.0 - self.t) + x * self.t;
                    context.line_to(x1, self.y);
                    context.line_to(x1, y);
                }
            }
            _ => {
                if self.t <= 0.0 {
                    context.line_to(self.x, y);
                    context.line_to(x, y);
                } else {
                    let x1 = self.x * (1.0 - self.t) + x * self.t;
                    context.line_to(x1, self.y);
                    context.line_to(x1, y);
                }
            }
        }
        self.x = x;
        self.y = y;
    }
}

/// d3-shape's `curveCatmullRom`.
pub struct CatmullRom {
    line: f64,
    alpha: f64,
    x0: f64,
    x1: f64,
    x2: f64,
    y0: f64,
    y1: f64,
    y2: f64,
    l01_a: f64,
    l12_a: f64,
    l23_a: f64,
    l01_2a: f64,
    l12_2a: f64,
    l23_2a: f64,
    point: u8,
}

impl CatmullRom {
    /// The JavaScript's `catmullRom.alpha`.
    ///
    /// The JavaScript's `catmullRom` with an alpha of zero is a cardinal
    /// curve, and the port keeps that: with an alpha of zero the port
    /// makes a [`Cardinal`] instead.
    pub fn new(alpha: f64) -> Self {
        Self {
            line: f64::NAN,
            alpha,
            x0: f64::NAN,
            x1: f64::NAN,
            x2: f64::NAN,
            y0: f64::NAN,
            y1: f64::NAN,
            y2: f64::NAN,
            l01_a: f64::NAN,
            l12_a: f64::NAN,
            l23_a: f64::NAN,
            l01_2a: f64::NAN,
            l12_2a: f64::NAN,
            l23_2a: f64::NAN,
            point: 0,
        }
    }
}

impl Default for CatmullRom {
    fn default() -> Self {
        Self::new(0.5)
    }
}

/// The JavaScript's Catmull-Rom `point`.
#[allow(clippy::too_many_arguments)]
fn catmull_rom_point(
    context: &mut dyn Context,
    x0: f64,
    x1: f64,
    x2: f64,
    y0: f64,
    y1: f64,
    y2: f64,
    l01_a: f64,
    l12_a: f64,
    l23_a: f64,
    l01_2a: f64,
    l12_2a: f64,
    l23_2a: f64,
    x: f64,
    y: f64,
    end_x: f64,
    end_y: f64,
) {
    // The JavaScript's `that._x1` and `that._y1`, which the first of
    // the two branches below does not change, for the JavaScript's own
    // `x1` and `y1` are copies of them and not the fields.
    let x1_before = x1;
    let y1_before = y1;
    let mut x1 = x1;
    let mut y1 = y1;
    let mut x2 = x2;
    let mut y2 = y2;
    if l01_a > EPSILON {
        let a = 2.0 * l01_2a + 3.0 * l01_a * l12_a + l12_2a;
        let n = 3.0 * l01_a * (l01_a + l12_a);
        x1 = (x1 * a - x0 * l12_2a + x2 * l01_2a) / n;
        y1 = (y1 * a - y0 * l12_2a + y2 * l01_2a) / n;
    }
    if l23_a > EPSILON {
        let b = 2.0 * l23_2a + 3.0 * l23_a * l12_a + l12_2a;
        let m = 3.0 * l23_a * (l23_a + l12_a);
        x2 = (x2 * b + x1_before * l23_2a - x * l12_2a) / m;
        y2 = (y2 * b + y1_before * l23_2a - y * l12_2a) / m;
    }
    // The JavaScript's endpoint is `that._x2, that._y2`, which is the
    // point before the one being added, and not the `x2` and `y2` above,
    // which have been moved.
    context.bezier_curve_to(x1, y1, x2, y2, end_x, end_y);
}

impl Curve for CatmullRom {
    fn area_start(&mut self, _context: &mut dyn Context) {
        self.line = 0.0;
    }

    fn area_end(&mut self, _context: &mut dyn Context) {
        self.line = f64::NAN;
    }

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.x0 = f64::NAN;
        self.x1 = f64::NAN;
        self.x2 = f64::NAN;
        self.y0 = f64::NAN;
        self.y1 = f64::NAN;
        self.y2 = f64::NAN;
        self.l01_a = f64::NAN;
        self.l12_a = f64::NAN;
        self.l23_a = f64::NAN;
        self.l01_2a = f64::NAN;
        self.l12_2a = f64::NAN;
        self.l23_2a = f64::NAN;
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        match self.point {
            2 => context.line_to(self.x2, self.y2),
            3 => self.point(context, self.x2, self.y2),
            _ => {}
        }
        if truthy(self.line) || (!js::strict_equals(self.line, 0.0) && self.point == 1) {
            context.close_path();
        }
        self.line = flip_line(self.line);
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        if self.point != 0 {
            let x23 = self.x2 - x;
            let y23 = self.y2 - y;
            self.l23_2a = (x23 * x23 + y23 * y23).powf(self.alpha);
            self.l23_a = self.l23_2a.sqrt();
        }
        match self.point {
            0 => {
                self.point = 1;
                if truthy(self.line) {
                    context.line_to(x, y);
                } else {
                    context.move_to(x, y);
                }
            }
            1 => self.point = 2,
            2 => {
                self.point = 3;
                catmull_rom_point(
                    context,
                    self.x0,
                    self.x1,
                    self.x2,
                    self.y0,
                    self.y1,
                    self.y2,
                    self.l01_a,
                    self.l12_a,
                    self.l23_a,
                    self.l01_2a,
                    self.l12_2a,
                    self.l23_2a,
                    x,
                    y,
                    self.x2,
                    self.y2,
                );
            }
            _ => catmull_rom_point(
                context,
                self.x0,
                self.x1,
                self.x2,
                self.y0,
                self.y1,
                self.y2,
                self.l01_a,
                self.l12_a,
                self.l23_a,
                self.l01_2a,
                self.l12_2a,
                self.l23_2a,
                x,
                y,
                self.x2,
                self.y2,
            ),
        }
        self.l01_a = self.l12_a;
        self.l12_a = self.l23_a;
        self.l01_2a = self.l12_2a;
        self.l12_2a = self.l23_2a;
        self.x0 = self.x1;
        self.x1 = self.x2;
        self.x2 = x;
        self.y0 = self.y1;
        self.y1 = self.y2;
        self.y2 = y;
    }
}

/// d3-shape's `curveBundle`.
pub struct Bundle {
    basis: Basis,
    beta: f64,
    x: Vec<f64>,
    y: Vec<f64>,
}

impl Bundle {
    /// The JavaScript's `bundle.beta`.
    pub fn new(beta: f64) -> Self {
        Self {
            basis: Basis::default(),
            beta,
            x: Vec::new(),
            y: Vec::new(),
        }
    }
}

impl Default for Bundle {
    fn default() -> Self {
        Self::new(0.85)
    }
}

impl Curve for Bundle {
    fn area_start(&mut self, _context: &mut dyn Context) {}
    fn area_end(&mut self, _context: &mut dyn Context) {}

    fn line_start(&mut self, context: &mut dyn Context) {
        self.x.clear();
        self.y.clear();
        self.basis.line_start(context);
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        let j = self.x.len() as isize - 1;
        if j > 0 {
            let j = j as usize;
            let x0 = self.x[0];
            let y0 = self.y[0];
            let dx = self.x[j] - x0;
            let dy = self.y[j] - y0;
            let mut i = 0usize;
            while i <= j {
                let t = i as f64 / j as f64;
                let x = self.beta * self.x[i] + (1.0 - self.beta) * (x0 + t * dx);
                let y = self.beta * self.y[i] + (1.0 - self.beta) * (y0 + t * dy);
                self.basis.point(context, x, y);
                i += 1;
            }
        }
        self.x.clear();
        self.y.clear();
        self.basis.line_end(context);
    }

    fn point(&mut self, _context: &mut dyn Context, x: f64, y: f64) {
        self.x.push(x);
        self.y.push(y);
    }
}

/// d3-shape's `curveMonotoneX`.
pub struct MonotoneX {
    line: f64,
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    t0: f64,
    point: u8,
}

impl Default for MonotoneX {
    fn default() -> Self {
        Self {
            line: f64::NAN,
            x0: f64::NAN,
            x1: f64::NAN,
            y0: f64::NAN,
            y1: f64::NAN,
            t0: f64::NAN,
            point: 0,
        }
    }
}

/// The JavaScript's `sign`.
fn sign(x: f64) -> f64 {
    if x < 0.0 {
        -1.0
    } else {
        1.0
    }
}

/// The JavaScript's `slope3`, which is the slope of the tangent.
fn slope3(x0: f64, x1: f64, y0: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let h0 = x1 - x0;
    let h1 = x2 - x1;
    // The JavaScript writes `h0 || h1 < 0 && -0`, which is zero when `h0`
    // is zero and the other is not negative, and negative zero otherwise.
    let d0 = if h0 != 0.0 {
        h0
    } else if h1 < 0.0 {
        -0.0
    } else {
        0.0
    };
    let d1 = if h1 != 0.0 {
        h1
    } else if h0 < 0.0 {
        -0.0
    } else {
        0.0
    };
    let s0 = (y1 - y0) / d0;
    let s1 = (y2 - y1) / d1;
    let p = (s0 * h1 + s1 * h0) / (h0 + h1);
    let value = (sign(s0) + sign(s1)) * s0.abs().min(s1.abs()).min(0.5 * p.abs());
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

/// The JavaScript's `slope2`, which is a one-sided slope.
fn slope2(x0: f64, x1: f64, y0: f64, y1: f64, t: f64) -> f64 {
    let h = x1 - x0;
    if h != 0.0 {
        (3.0 * (y1 - y0) / h - t) / 2.0
    } else {
        t
    }
}

/// The JavaScript's monotone `point`.
fn monotone_point(context: &mut dyn Context, x0: f64, x1: f64, y0: f64, y1: f64, t0: f64, t1: f64) {
    let dx = (x1 - x0) / 3.0;
    context.bezier_curve_to(x0 + dx, y0 + dx * t0, x1 - dx, y1 - dx * t1, x1, y1);
}

impl Curve for MonotoneX {
    fn area_start(&mut self, _context: &mut dyn Context) {
        self.line = 0.0;
    }

    fn area_end(&mut self, _context: &mut dyn Context) {
        self.line = f64::NAN;
    }

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.x0 = f64::NAN;
        self.x1 = f64::NAN;
        self.y0 = f64::NAN;
        self.y1 = f64::NAN;
        self.t0 = f64::NAN;
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        match self.point {
            2 => context.line_to(self.x1, self.y1),
            3 => {
                let t = slope2(self.x0, self.x1, self.y0, self.y1, self.t0);
                monotone_point(context, self.x0, self.x1, self.y0, self.y1, self.t0, t);
            }
            _ => {}
        }
        if truthy(self.line) || (!js::strict_equals(self.line, 0.0) && self.point == 1) {
            context.close_path();
        }
        self.line = flip_line(self.line);
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        let mut t1 = f64::NAN;
        // Ignore coincident points.
        if x == self.x1 && y == self.y1 {
            return;
        }
        match self.point {
            0 => {
                self.point = 1;
                if truthy(self.line) {
                    context.line_to(x, y);
                } else {
                    context.move_to(x, y);
                }
            }
            1 => self.point = 2,
            2 => {
                self.point = 3;
                t1 = slope3(self.x0, self.x1, self.y0, self.y1, x, y);
                let t0 = slope2(self.x0, self.x1, self.y0, self.y1, t1);
                monotone_point(context, self.x0, self.x1, self.y0, self.y1, t0, t1);
            }
            _ => {
                t1 = slope3(self.x0, self.x1, self.y0, self.y1, x, y);
                monotone_point(context, self.x0, self.x1, self.y0, self.y1, self.t0, t1);
            }
        }
        self.x0 = self.x1;
        self.x1 = x;
        self.y0 = self.y1;
        self.y1 = y;
        self.t0 = t1;
    }
}

/// d3-shape's `curveMonotoneY`.
///
/// The JavaScript's `MonotoneY` is `MonotoneX` with the two numbers
/// swapped, and the context is wrapped so that the swap happens in one
/// place. The port does the same: the curve is `MonotoneX`, and the
/// context is wrapped.
#[derive(Default)]
pub struct MonotoneY {
    inner: MonotoneX,
}

/// The JavaScript's `ReflectContext`, which swaps the two numbers of
/// every call.
struct ReflectContext<'a> {
    inner: &'a mut dyn Context,
}

impl Context for ReflectContext<'_> {
    fn move_to(&mut self, x: f64, y: f64) {
        self.inner.move_to(y, x);
    }
    fn line_to(&mut self, x: f64, y: f64) {
        self.inner.line_to(y, x);
    }
    fn close_path(&mut self) {
        self.inner.close_path();
    }
    fn quadratic_curve_to(&mut self, x1: f64, y1: f64, x: f64, y: f64) {
        self.inner.quadratic_curve_to(y1, x1, y, x);
    }
    fn bezier_curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) {
        self.inner.bezier_curve_to(y1, x1, y2, x2, y, x);
    }
    fn arc(
        &mut self,
        x: f64,
        y: f64,
        r: f64,
        a0: f64,
        a1: f64,
        ccw: bool,
    ) -> Result<(), crate::path::PathError> {
        self.inner.arc(y, x, r, a0, a1, ccw)
    }
    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.inner.rect(y, x, w, h);
    }
}

impl Curve for MonotoneY {
    fn area_start(&mut self, _context: &mut dyn Context) {
        self.inner.area_start(_context);
    }
    fn area_end(&mut self, _context: &mut dyn Context) {
        self.inner.area_end(_context);
    }
    fn line_start(&mut self, _context: &mut dyn Context) {
        self.inner.line_start(_context);
    }
    fn line_end(&mut self, context: &mut dyn Context) {
        let mut reflected = ReflectContext { inner: context };
        self.inner.line_end(&mut reflected);
    }
    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        let mut reflected = ReflectContext { inner: context };
        self.inner.point(&mut reflected, y, x);
    }
}

/// d3-shape's `curveLinear`, as a factory.
pub fn curve_linear() -> Box<dyn Curve> {
    Box::new(Linear::default())
}

/// d3-shape's `curveLinearClosed`, as a factory.
pub fn curve_linear_closed() -> Box<dyn Curve> {
    Box::new(LinearClosed::default())
}

/// d3-shape's `curveNatural`, as a factory.
pub fn curve_natural() -> Box<dyn Curve> {
    Box::new(Natural::default())
}

/// d3-shape's `curveCardinal`, as a factory.
pub fn curve_cardinal(tension: f64) -> Box<dyn Curve> {
    Box::new(Cardinal::new(tension))
}

/// d3-shape's `curveCardinalOpen`, as a factory.
pub fn curve_cardinal_open(tension: f64) -> Box<dyn Curve> {
    Box::new(CardinalOpen::new(tension))
}

/// d3-shape's `curveCardinalClosed`, as a factory.
pub fn curve_cardinal_closed(tension: f64) -> Box<dyn Curve> {
    Box::new(CardinalClosed::new(tension))
}

/// d3-shape's `curveBasis`, as a factory.
pub fn curve_basis() -> Box<dyn Curve> {
    Box::new(Basis::default())
}

/// d3-shape's `curveBasisOpen`, as a factory.
pub fn curve_basis_open() -> Box<dyn Curve> {
    Box::new(BasisOpen::default())
}

/// d3-shape's `curveBasisClosed`, as a factory.
pub fn curve_basis_closed() -> Box<dyn Curve> {
    Box::new(BasisClosed::default())
}

/// d3-shape's `curveBundle`, as a factory.
pub fn curve_bundle(beta: f64) -> Box<dyn Curve> {
    if beta == 1.0 {
        // The JavaScript's `bundle` with a beta of one is a basis curve.
        return Box::new(Basis::default());
    }
    Box::new(Bundle::new(beta))
}

/// d3-shape's `curveCatmullRom`, as a factory.
pub fn curve_catmull_rom(alpha: f64) -> Box<dyn Curve> {
    if alpha == 0.0 {
        // The JavaScript's `catmullRom` with an alpha of zero is a
        // cardinal curve.
        return Box::new(Cardinal::new(0.0));
    }
    Box::new(CatmullRom::new(alpha))
}

/// d3-shape's `curveCatmullRomOpen`, as a factory.
pub fn curve_catmull_rom_open(alpha: f64) -> Box<dyn Curve> {
    Box::new(CatmullRomOpen::new(alpha))
}

/// d3-shape's `curveCatmullRomClosed`, as a factory.
pub fn curve_catmull_rom_closed(alpha: f64) -> Box<dyn Curve> {
    Box::new(CatmullRomClosed::new(alpha))
}

/// d3-shape's `curveMonotoneX`, as a factory.
pub fn curve_monotone_x() -> Box<dyn Curve> {
    Box::new(MonotoneX::default())
}

/// d3-shape's `curveMonotoneY`, as a factory.
pub fn curve_monotone_y() -> Box<dyn Curve> {
    Box::new(MonotoneY::default())
}

/// d3-shape's `curveStep`, as a factory.
pub fn curve_step() -> Box<dyn Curve> {
    Box::new(Step::new(0.5))
}

/// d3-shape's `curveStepAfter`, as a factory.
pub fn curve_step_after() -> Box<dyn Curve> {
    Box::new(Step::new(1.0))
}

/// d3-shape's `curveStepBefore`, as a factory.
pub fn curve_step_before() -> Box<dyn Curve> {
    Box::new(Step::new(0.0))
}

/// d3-shape's `curveCatmullRomOpen`.
pub struct CatmullRomOpen {
    line: f64,
    alpha: f64,
    x0: f64,
    x1: f64,
    x2: f64,
    y0: f64,
    y1: f64,
    y2: f64,
    l01_a: f64,
    l12_a: f64,
    l23_a: f64,
    l01_2a: f64,
    l12_2a: f64,
    l23_2a: f64,
    point: u8,
}

impl CatmullRomOpen {
    /// The JavaScript's `catmullRomOpen.alpha`.
    pub fn new(alpha: f64) -> Self {
        Self {
            line: f64::NAN,
            alpha,
            x0: f64::NAN,
            x1: f64::NAN,
            x2: f64::NAN,
            y0: f64::NAN,
            y1: f64::NAN,
            y2: f64::NAN,
            l01_a: f64::NAN,
            l12_a: f64::NAN,
            l23_a: f64::NAN,
            l01_2a: f64::NAN,
            l12_2a: f64::NAN,
            l23_2a: f64::NAN,
            point: 0,
        }
    }
}

impl Default for CatmullRomOpen {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl Curve for CatmullRomOpen {
    fn area_start(&mut self, _context: &mut dyn Context) {
        self.line = 0.0;
    }

    fn area_end(&mut self, _context: &mut dyn Context) {
        self.line = f64::NAN;
    }

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.x0 = f64::NAN;
        self.x1 = f64::NAN;
        self.x2 = f64::NAN;
        self.y0 = f64::NAN;
        self.y1 = f64::NAN;
        self.y2 = f64::NAN;
        self.l01_a = f64::NAN;
        self.l12_a = f64::NAN;
        self.l23_a = f64::NAN;
        self.l01_2a = f64::NAN;
        self.l12_2a = f64::NAN;
        self.l23_2a = f64::NAN;
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        if truthy(self.line) || (!js::strict_equals(self.line, 0.0) && self.point == 3) {
            context.close_path();
        }
        self.line = flip_line(self.line);
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        if self.point != 0 {
            let x23 = self.x2 - x;
            let y23 = self.y2 - y;
            self.l23_2a = (x23 * x23 + y23 * y23).powf(self.alpha);
            self.l23_a = self.l23_2a.sqrt();
        }
        match self.point {
            0 => self.point = 1,
            1 => self.point = 2,
            2 => {
                self.point = 3;
                if truthy(self.line) {
                    context.line_to(self.x2, self.y2);
                } else {
                    context.move_to(self.x2, self.y2);
                }
            }
            3 => {
                self.point = 4;
                catmull_rom_point(
                    context,
                    self.x0,
                    self.x1,
                    self.x2,
                    self.y0,
                    self.y1,
                    self.y2,
                    self.l01_a,
                    self.l12_a,
                    self.l23_a,
                    self.l01_2a,
                    self.l12_2a,
                    self.l23_2a,
                    x,
                    y,
                    self.x2,
                    self.y2,
                );
            }
            _ => catmull_rom_point(
                context,
                self.x0,
                self.x1,
                self.x2,
                self.y0,
                self.y1,
                self.y2,
                self.l01_a,
                self.l12_a,
                self.l23_a,
                self.l01_2a,
                self.l12_2a,
                self.l23_2a,
                x,
                y,
                self.x2,
                self.y2,
            ),
        }
        self.l01_a = self.l12_a;
        self.l12_a = self.l23_a;
        self.l01_2a = self.l12_2a;
        self.l12_2a = self.l23_2a;
        self.x0 = self.x1;
        self.x1 = self.x2;
        self.x2 = x;
        self.y0 = self.y1;
        self.y1 = self.y2;
        self.y2 = y;
    }
}

/// d3-shape's `curveCatmullRomClosed`.
pub struct CatmullRomClosed {
    alpha: f64,
    x0: f64,
    x1: f64,
    x2: f64,
    x3: f64,
    x4: f64,
    x5: f64,
    y0: f64,
    y1: f64,
    y2: f64,
    y3: f64,
    y4: f64,
    y5: f64,
    l01_a: f64,
    l12_a: f64,
    l23_a: f64,
    l01_2a: f64,
    l12_2a: f64,
    l23_2a: f64,
    point: u8,
}

impl CatmullRomClosed {
    /// The JavaScript's `catmullRomClosed.alpha`.
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha,
            x0: f64::NAN,
            x1: f64::NAN,
            x2: f64::NAN,
            x3: f64::NAN,
            x4: f64::NAN,
            x5: f64::NAN,
            y0: f64::NAN,
            y1: f64::NAN,
            y2: f64::NAN,
            y3: f64::NAN,
            y4: f64::NAN,
            y5: f64::NAN,
            l01_a: f64::NAN,
            l12_a: f64::NAN,
            l23_a: f64::NAN,
            l01_2a: f64::NAN,
            l12_2a: f64::NAN,
            l23_2a: f64::NAN,
            point: 0,
        }
    }
}

impl Default for CatmullRomClosed {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl Curve for CatmullRomClosed {
    fn area_start(&mut self, _context: &mut dyn Context) {}
    fn area_end(&mut self, _context: &mut dyn Context) {}

    fn line_start(&mut self, _context: &mut dyn Context) {
        self.x0 = f64::NAN;
        self.x1 = f64::NAN;
        self.x2 = f64::NAN;
        self.x3 = f64::NAN;
        self.x4 = f64::NAN;
        self.x5 = f64::NAN;
        self.y0 = f64::NAN;
        self.y1 = f64::NAN;
        self.y2 = f64::NAN;
        self.y3 = f64::NAN;
        self.y4 = f64::NAN;
        self.y5 = f64::NAN;
        self.l01_a = f64::NAN;
        self.l12_a = f64::NAN;
        self.l23_a = f64::NAN;
        self.l01_2a = f64::NAN;
        self.l12_2a = f64::NAN;
        self.l23_2a = f64::NAN;
        self.point = 0;
    }

    fn line_end(&mut self, context: &mut dyn Context) {
        match self.point {
            1 => {
                context.move_to(self.x3, self.y3);
                context.close_path();
            }
            2 => {
                context.line_to(self.x3, self.y3);
                context.close_path();
            }
            3 => {
                self.point(context, self.x3, self.y3);
                self.point(context, self.x4, self.y4);
                self.point(context, self.x5, self.y5);
            }
            _ => {}
        }
    }

    fn point(&mut self, context: &mut dyn Context, x: f64, y: f64) {
        if self.point != 0 {
            let x23 = self.x2 - x;
            let y23 = self.y2 - y;
            self.l23_2a = (x23 * x23 + y23 * y23).powf(self.alpha);
            self.l23_a = self.l23_2a.sqrt();
        }
        match self.point {
            0 => {
                self.point = 1;
                self.x3 = x;
                self.y3 = y;
            }
            1 => {
                self.point = 2;
                self.x4 = x;
                self.y4 = y;
                context.move_to(self.x4, self.y4);
            }
            2 => {
                self.point = 3;
                self.x5 = x;
                self.y5 = y;
            }
            _ => catmull_rom_point(
                context,
                self.x0,
                self.x1,
                self.x2,
                self.y0,
                self.y1,
                self.y2,
                self.l01_a,
                self.l12_a,
                self.l23_a,
                self.l01_2a,
                self.l12_2a,
                self.l23_2a,
                x,
                y,
                self.x2,
                self.y2,
            ),
        }
        self.l01_a = self.l12_a;
        self.l12_a = self.l23_a;
        self.l01_2a = self.l12_2a;
        self.l12_2a = self.l23_2a;
        self.x0 = self.x1;
        self.x1 = self.x2;
        self.x2 = x;
        self.y0 = self.y1;
        self.y1 = self.y2;
        self.y2 = y;
    }
}
