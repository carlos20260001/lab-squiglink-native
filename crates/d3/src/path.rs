//! A port of **d3-path**, from the JavaScript of the same name.
//!
//! Translated from `d3-path/src/path.js`, version 1.0.9, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read differently,
//! the JavaScript is right.
//!
//! d3-path writes the text of an SVG path. Every other part of d3 that
//! draws — the shapes, the axes, the links — writes its path through it,
//! so this is where the numbers become the text a document reads.
//!
//! The path is written with the JavaScript's own number formatting, which
//! is not Rust's: `1e21` is `1e+21`, and `1e-7` is `1e-7`. The port keeps
//! that, and [`crate::js::number_to_string`] is where the rule lives.

use crate::ieee754;
use crate::js::number_to_string;

/// The two pi the JavaScript uses.
const PI: f64 = std::f64::consts::PI;
/// Twice pi.
const TAU: f64 = 2.0 * PI;
/// The epsilon the JavaScript compares against.
const EPSILON: f64 = 1e-6;
/// Twice pi, less the epsilon.
const TAU_EPSILON: f64 = TAU - EPSILON;

/// d3-path's `path`.
///
/// The JavaScript:
///
/// ```js
/// function Path() {
///   this._x0 = this._y0 = // start of current subpath
///   this._x1 = this._y1 = null; // end of current subpath
///   this._ = "";
/// }
/// ```
///
/// The port keeps the same four numbers, because they are what the
/// JavaScript's own tests of the path's shape depend on: the start of the
/// current sub-path, the end of it, and the text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    /// The start of the current sub-path.
    x0: Option<f64>,
    /// The start of the current sub-path.
    y0: Option<f64>,
    /// The end of the current sub-path.
    x1: Option<f64>,
    /// The end of the current sub-path.
    y1: Option<f64>,
    /// The text of the path.
    text: String,
}

impl Path {
    /// d3-path's `moveTo`.
    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.x0 = Some(x);
        self.x1 = Some(x);
        self.y0 = Some(y);
        self.y1 = Some(y);
        self.text
            .push_str(&format!("M{},{}", number_to_string(x), number_to_string(y)));
        self
    }

    /// d3-path's `closePath`.
    pub fn close_path(&mut self) -> &mut Self {
        if self.x1.is_some() {
            self.x1 = self.x0;
            self.y1 = self.y0;
            self.text.push('Z');
        }
        self
    }

    /// d3-path's `lineTo`.
    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.x1 = Some(x);
        self.y1 = Some(y);
        self.text
            .push_str(&format!("L{},{}", number_to_string(x), number_to_string(y)));
        self
    }

    /// d3-path's `quadraticCurveTo`.
    pub fn quadratic_curve_to(&mut self, x1: f64, y1: f64, x: f64, y: f64) -> &mut Self {
        self.x1 = Some(x);
        self.y1 = Some(y);
        self.text.push_str(&format!(
            "Q{},{},{},{}",
            number_to_string(x1),
            number_to_string(y1),
            number_to_string(x),
            number_to_string(y)
        ));
        self
    }

    /// d3-path's `bezierCurveTo`.
    pub fn bezier_curve_to(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    ) -> &mut Self {
        self.x1 = Some(x);
        self.y1 = Some(y);
        self.text.push_str(&format!(
            "C{},{},{},{},{},{}",
            number_to_string(x1),
            number_to_string(y1),
            number_to_string(x2),
            number_to_string(y2),
            number_to_string(x),
            number_to_string(y)
        ));
        self
    }

    /// d3-path's `arcTo`.
    ///
    /// The JavaScript throws when the radius is negative, and the port
    /// returns an error in that case rather than panicking.
    pub fn arc_to(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        r: f64,
    ) -> Result<&mut Self, PathError> {
        let (x1, y1, x2, y2, r) = (x1, y1, x2, y2, r);
        let (x0, y0) = match (self.x1, self.y1) {
            (Some(x), Some(y)) => (x, y),
            _ => (f64::NAN, f64::NAN),
        };
        let x21 = x2 - x1;
        let y21 = y2 - y1;
        let x01 = x0 - x1;
        let y01 = y0 - y1;
        let l01_2 = x01 * x01 + y01 * y01;

        // Is the radius negative? Error.
        if r < 0.0 {
            return Err(PathError::NegativeRadius(r));
        }

        // Is this path empty? Move to (x1,y1).
        if self.x1.is_none() {
            self.x1 = Some(x1);
            self.y1 = Some(y1);
            self.text.push_str(&format!(
                "M{},{}",
                number_to_string(x1),
                number_to_string(y1)
            ));
        }
        // Or, is (x1,y1) coincident with (x0,y0)? Do nothing.
        else if !(l01_2 > EPSILON) {
            // The JavaScript writes `!(l01_2 > epsilon)`, which is false
            // for a `NaN` as well as for a zero. The port keeps it.

            return Ok(self);
        }
        // Or, are (x0,y0), (x1,y1) and (x2,y2) collinear?
        // Equivalently, is (x1,y1) coincident with (x2,y2)?
        // Or, is the radius zero? Line to (x1,y1).
        else if !((y01 * x21 - y21 * x01).abs() > EPSILON) || r == 0.0 {
            self.x1 = Some(x1);
            self.y1 = Some(y1);
            self.text.push_str(&format!(
                "L{},{}",
                number_to_string(x1),
                number_to_string(y1)
            ));
        }
        // Otherwise, draw an arc!
        else {
            let x20 = x2 - x0;
            let y20 = y2 - y0;
            let l21_2 = x21 * x21 + y21 * y21;
            let l20_2 = x20 * x20 + y20 * y20;
            let l21 = ieee754::sqrt(l21_2);
            let l01 = ieee754::sqrt(l01_2);
            let l = r * ieee754::tan(
                (PI - ieee754::acos((l21_2 + l01_2 - l20_2) / (2.0 * l21 * l01))) / 2.0,
            );
            let t01 = l / l01;
            let t21 = l / l21;

            // If the start tangent is not coincident with (x0,y0), line to.
            if (t01 - 1.0).abs() > EPSILON {
                self.text.push_str(&format!(
                    "L{},{}",
                    number_to_string(x1 + t01 * x01),
                    number_to_string(y1 + t01 * y01)
                ));
            }
            self.x1 = Some(x1 + t21 * x21);
            self.y1 = Some(y1 + t21 * y21);
            self.text.push_str(&format!(
                "A{},{},0,0,{},{},{}",
                number_to_string(r),
                number_to_string(r),
                number_to_string((y01 * x20 > x01 * y20) as i32 as f64),
                number_to_string(self.x1.expect("just set")),
                number_to_string(self.y1.expect("just set"))
            ));
        }
        Ok(self)
    }

    /// d3-path's `arc`.
    ///
    /// The JavaScript throws when the radius is negative, and the port
    /// returns an error in that case rather than panicking.
    pub fn arc(
        &mut self,
        x: f64,
        y: f64,
        r: f64,
        a0: f64,
        a1: f64,
        ccw: bool,
    ) -> Result<&mut Self, PathError> {
        let dx = r * ieee754::cos(a0);
        let dy = r * ieee754::sin(a0);
        let x0 = x + dx;
        let y0 = y + dy;
        let cw = !ccw;
        let mut da = if ccw { a0 - a1 } else { a1 - a0 };

        // Is the radius negative? Error.
        if r < 0.0 {
            return Err(PathError::NegativeRadius(r));
        }

        // Is this path empty? Move to (x0,y0).
        if self.x1.is_none() {
            self.text.push_str(&format!(
                "M{},{}",
                number_to_string(x0),
                number_to_string(y0)
            ));
        }
        // Or, is (x0,y0) not coincident with the previous point? Line to (x0,y0).
        else if self
            .x1
            .zip(self.y1)
            .is_some_and(|(x1, y1)| (x1 - x0).abs() > EPSILON || (y1 - y0).abs() > EPSILON)
        {
            self.text.push_str(&format!(
                "L{},{}",
                number_to_string(x0),
                number_to_string(y0)
            ));
        }

        // Is this arc empty? We're done.
        if r == 0.0 {
            return Ok(self);
        }

        // Does the angle go the wrong way? Flip the direction.
        if da < 0.0 {
            da = da % TAU + TAU;
        }

        // Is this a complete circle? Draw two arcs to complete the circle.
        if da > TAU_EPSILON {
            self.text.push_str(&format!(
                "A{},{},0,1,{},{},{}A{},{},0,1,{},{},{}",
                number_to_string(r),
                number_to_string(r),
                number_to_string(cw as i32 as f64),
                number_to_string(x - dx),
                number_to_string(y - dy),
                number_to_string(r),
                number_to_string(r),
                number_to_string(cw as i32 as f64),
                number_to_string(x0),
                number_to_string(y0)
            ));
            self.x1 = Some(x0);
            self.y1 = Some(y0);
        }
        // Is this arc non-empty? Draw an arc!
        else if da > EPSILON {
            self.x1 = Some(x + r * ieee754::cos(a1));
            self.y1 = Some(y + r * ieee754::sin(a1));
            self.text.push_str(&format!(
                "A{},{},0,{},{},{},{}",
                number_to_string(r),
                number_to_string(r),
                number_to_string((da >= PI) as i32 as f64),
                number_to_string(cw as i32 as f64),
                number_to_string(self.x1.expect("just set")),
                number_to_string(self.y1.expect("just set"))
            ));
        }
        Ok(self)
    }

    /// d3-path's `rect`.
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) -> &mut Self {
        self.x0 = Some(x);
        self.x1 = Some(x);
        self.y0 = Some(y);
        self.y1 = Some(y);
        self.text.push_str(&format!(
            "M{},{}h{}v{}h{}Z",
            number_to_string(x),
            number_to_string(y),
            number_to_string(w),
            number_to_string(h),
            number_to_string(-w)
        ));
        self
    }

    /// The text of the path, which is what the JavaScript's `toString`
    /// gives. The [`Display`] implementation gives the same, and is what
    /// the port's own code uses.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl crate::shape::Context for Path {
    fn move_to(&mut self, x: f64, y: f64) {
        self.move_to(x, y);
    }
    fn line_to(&mut self, x: f64, y: f64) {
        self.line_to(x, y);
    }
    fn close_path(&mut self) {
        self.close_path();
    }
    fn quadratic_curve_to(&mut self, x1: f64, y1: f64, x: f64, y: f64) {
        self.quadratic_curve_to(x1, y1, x, y);
    }
    fn bezier_curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) {
        self.bezier_curve_to(x1, y1, x2, y2, x, y);
    }
    fn arc(
        &mut self,
        x: f64,
        y: f64,
        r: f64,
        a0: f64,
        a1: f64,
        ccw: bool,
    ) -> Result<(), PathError> {
        Path::arc(self, x, y, r, a0, a1, ccw).map(|_| ())
    }
    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        Path::rect(self, x, y, w, h);
    }
}

/// What can go wrong while a path is written.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathError {
    /// The JavaScript throws `"negative radius: " + r` for this.
    NegativeRadius(f64),
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathError::NegativeRadius(r) => write!(f, "negative radius: {r}"),
        }
    }
}

impl std::error::Error for PathError {}
