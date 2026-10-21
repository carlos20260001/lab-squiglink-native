//! d3-shape's radial line and area, from the JavaScript of the same name.
//!
//! Translated from `d3-shape/src/lineRadial.js`, `d3-shape/src/areaRadial.js`
//! and `d3-shape/src/curve/radial.js`, version 1.3.7, of d3 5.16.0. The
//! JavaScript is the authority.

use crate::ieee754;

use super::area::Area;
use super::curve::{Curve, CurveFactory};
use super::line::Line;
use super::Context;

/// d3-shape's `curveRadial`.
///
/// The JavaScript:
///
/// ```js
/// Radial.prototype = {
///   ...
///   point: function(a, r) {
///     this._curve.point(r * ieee754::sin(Matha), r * -ieee754::cos(Matha));
///   }
/// };
/// ```
///
/// The radial curve is the curve it wraps, with the angle and the radius
/// turned into a point first.
pub struct Radial {
    curve: Box<dyn Curve>,
}

impl Radial {
    /// The JavaScript's `curveRadial(curve)`.
    pub fn new(curve: Box<dyn Curve>) -> Self {
        Self { curve }
    }
}

impl Curve for Radial {
    fn area_start(&mut self, context: &mut dyn Context) {
        self.curve.area_start(context);
    }
    fn area_end(&mut self, context: &mut dyn Context) {
        self.curve.area_end(context);
    }
    fn line_start(&mut self, context: &mut dyn Context) {
        self.curve.line_start(context);
    }
    fn line_end(&mut self, context: &mut dyn Context) {
        self.curve.line_end(context);
    }
    fn point(&mut self, context: &mut dyn Context, a: f64, r: f64) {
        self.curve
            .point(context, r * ieee754::sin(a), r * -ieee754::cos(a));
    }
}

/// d3-shape's `curveRadialLinear`, which is the radial curve of the
/// linear one.
pub fn curve_radial_linear() -> Box<dyn Curve> {
    Box::new(Radial::new(super::curve::curve_linear()))
}

/// d3-shape's radial line.
///
/// The JavaScript's `lineRadial` is the line with the `x` and the `y`
/// renamed to `angle` and `radius`, and the curve wrapped in the radial
/// one.
pub struct RadialLine<T> {
    inner: Line<T>,
}

impl<T: AsRef<[f64]>> Default for RadialLine<T> {
    fn default() -> Self {
        Self { inner: Line::new() }
    }
}

impl<T> RadialLine<T> {
    /// The JavaScript's `lineRadial.angle`, which is the line's `x`.
    pub fn angle<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.inner = self.inner.x(f);
        self
    }

    /// The JavaScript's `lineRadial.radius`, which is the line's `y`.
    pub fn radius<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.inner = self.inner.y(f);
        self
    }

    /// The JavaScript's `lineRadial.defined`.
    pub fn defined<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> bool + 'static,
    {
        self.inner = self.inner.defined(f);
        self
    }

    /// The JavaScript's `lineRadial.curve`, which wraps the curve in the
    /// radial one.
    pub fn curve(mut self, curve: CurveFactory) -> Self {
        self.inner = self.inner.curve(curve);
        self
    }

    /// The JavaScript's `lineRadial(data)`.
    pub fn apply(&self, data: &[T]) -> Option<String> {
        self.inner.apply(data)
    }
}

/// d3-shape's radial area.
///
/// The JavaScript's `areaRadial` is the area with the edges renamed to
/// angles and radii.
pub struct RadialArea<T> {
    inner: Area<T>,
}

impl<T: AsRef<[f64]>> Default for RadialArea<T> {
    fn default() -> Self {
        Self {
            inner: Area::default(),
        }
    }
}

impl<T> RadialArea<T> {
    /// The JavaScript's `areaRadial.angle`, which is the area's `x`.
    pub fn angle<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.inner = self.inner.x0(f);
        self
    }

    /// The JavaScript's `areaRadial.startAngle`, which is the area's
    /// `x0`.
    pub fn start_angle<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.inner = self.inner.x0(f);
        self
    }

    /// The JavaScript's `areaRadial.endAngle`, which is the area's `x1`.
    pub fn end_angle<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.inner = self.inner.x1(f);
        self
    }

    /// The JavaScript's `areaRadial.radius`, which is the area's `y`.
    pub fn radius<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.inner = self.inner.y0(f);
        self
    }

    /// The JavaScript's `areaRadial.innerRadius`, which is the area's
    /// `y0`.
    pub fn inner_radius<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.inner = self.inner.y0(f);
        self
    }

    /// The JavaScript's `areaRadial.outerRadius`, which is the area's
    /// `y1`.
    pub fn outer_radius<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.inner = self.inner.y1(f);
        self
    }

    /// The JavaScript's `areaRadial.defined`.
    pub fn defined<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> bool + 'static,
    {
        self.inner = self.inner.defined(f);
        self
    }

    /// The JavaScript's `areaRadial.curve`.
    pub fn curve(mut self, curve: CurveFactory) -> Self {
        self.inner = self.inner.curve(curve);
        self
    }

    /// The JavaScript's `areaRadial(data)`.
    pub fn apply(&self, data: &[T]) -> Option<String> {
        self.inner.apply(data)
    }
}
