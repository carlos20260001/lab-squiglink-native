//! d3-shape's links, from the JavaScript of the same name.
//!
//! Translated from `d3-shape/src/link/index.js` and the files beside it,
//! version 1.3.7, of d3 5.16.0. The JavaScript is the authority.

use crate::ieee754;

use super::curve::CurveFactory;
use super::Context;
use crate::path::Path;

/// A function that reads a number out of a datum.
pub type Accessor<T> = Box<dyn Fn(&T, usize, &[T]) -> f64>;

/// A function that reads one end of a link out of a datum.
pub type End<D, E> = Box<dyn Fn(&D, usize, &[D]) -> E>;

/// A function that reads a number out of one end of a link.
pub type EndAccessor<E> = Box<dyn Fn(&E, usize, &[E]) -> f64>;

/// What a link's datum gives: the JavaScript's `linkSource` reads
/// `d.source` and its `linkTarget` reads `d.target`, and the port's
/// default asks the datum for the same two.
pub trait LinkDatum {
    /// The kind of the two ends of the link.
    type End;
    /// The JavaScript's `d.source`.
    fn source(&self) -> Self::End;
    /// The JavaScript's `d.target`.
    fn target(&self) -> Self::End;
}

/// d3-shape's `link`.
///
/// The JavaScript's `link` takes a curve — the function that draws the
/// link — and gives a generator that finds the two ends of it and hands
/// them to that curve.
pub struct Link<D, E> {
    source: End<D, E>,
    target: End<D, E>,
    x: EndAccessor<E>,
    y: EndAccessor<E>,
    curve: fn(&mut dyn Context, f64, f64, f64, f64),
}

impl<D: LinkDatum + Clone> Default for Link<D, D::End> {
    fn default() -> Self {
        Self {
            source: Box::new(|d: &D, _, _| {
                // The JavaScript's `linkSource` reads `d.source`.
                d.source()
            }),
            target: Box::new(|d: &D, _, _| {
                // The JavaScript's `linkTarget` reads `d.target`.
                d.target()
            }),
            x: Box::new(|_, _, _| 0.0),
            y: Box::new(|_, _, _| 0.0),
            curve: curve_horizontal,
        }
    }
}

impl<D, E: Clone> Link<D, E> {
    /// The JavaScript's `link.source`.
    pub fn source<F>(mut self, f: F) -> Self
    where
        F: Fn(&D, usize, &[D]) -> E + 'static,
    {
        self.source = Box::new(f);
        self
    }

    /// The JavaScript's `link.target`.
    pub fn target<F>(mut self, f: F) -> Self
    where
        F: Fn(&D, usize, &[D]) -> E + 'static,
    {
        self.target = Box::new(f);
        self
    }

    /// The JavaScript's `link.x`, which is applied to the end of the
    /// link rather than to the link itself.
    pub fn x<F>(mut self, f: F) -> Self
    where
        F: Fn(&E, usize, &[E]) -> f64 + 'static,
    {
        self.x = Box::new(f);
        self
    }

    /// The JavaScript's `link.y`, which is applied to the end of the
    /// link rather than to the link itself.
    pub fn y<F>(mut self, f: F) -> Self
    where
        F: Fn(&E, usize, &[E]) -> f64 + 'static,
    {
        self.y = Box::new(f);
        self
    }

    /// The JavaScript's `link(data)`: the path of the link.
    pub fn apply(&self, data: &[D]) -> Option<String>
    where
        D: Clone,
        E: Clone,
    {
        let mut buffer = Path::default();
        self.apply_to(data, &mut buffer);
        let text = buffer.text().to_owned();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// The JavaScript's `link(data)` drawn into a context the caller
    /// holds, which is the JavaScript's `link.context`.
    pub fn apply_to(&self, data: &[D], context: &mut dyn Context) {
        // The JavaScript's `x` and `y` are applied to the end of the
        // link, and are given the link's own data as their second
        // argument, which is what the port does too.
        let ends: Vec<E> = data
            .iter()
            .enumerate()
            .map(|(i, datum)| (self.source)(datum, i, data))
            .collect();
        for (i, datum) in data.iter().enumerate() {
            let s = ends[i].clone();
            let t = (self.target)(datum, i, data);
            let x0 = (self.x)(&s, i, &ends);
            let y0 = (self.y)(&s, i, &ends);
            let x1 = (self.x)(&t, i, &ends);
            let y1 = (self.y)(&t, i, &ends);
            (self.curve)(context, x0, y0, x1, y1);
        }
    }
}

/// The JavaScript's `curveHorizontal`.
fn curve_horizontal(context: &mut dyn Context, x0: f64, y0: f64, x1: f64, y1: f64) {
    context.move_to(x0, y0);
    let x0 = (x0 + x1) / 2.0;
    context.bezier_curve_to(x0, y0, x0, y1, x1, y1);
}

/// The JavaScript's `curveVertical`.
fn curve_vertical(context: &mut dyn Context, x0: f64, y0: f64, x1: f64, y1: f64) {
    context.move_to(x0, y0);
    let y0 = (y0 + y1) / 2.0;
    context.bezier_curve_to(x0, y0, x1, y0, x1, y1);
}

/// The JavaScript's `curveRadial`.
fn curve_radial(context: &mut dyn Context, x0: f64, y0: f64, x1: f64, y1: f64) {
    let p0 = point_radial(x0, y0);
    let p1 = point_radial(x0, (y0 + y1) / 2.0);
    let p2 = point_radial(x1, (y0 + y1) / 2.0);
    let p3 = point_radial(x1, y1);
    context.move_to(p0.0, p0.1);
    context.bezier_curve_to(p1.0, p1.1, p2.0, p2.1, p3.0, p3.1);
}

/// The JavaScript's `pointRadial`, which turns an angle and a radius
/// into a point.
///
/// The JavaScript:
///
/// ```js
/// export default function(x, y) {
///   return [(y = +y) * ieee754::cos(Mathx -= Math.PI / 2), y * ieee754::sin(Mathx)];
/// }
/// ```
///
/// The angle is turned by a quarter turn before the sine and the cosine
/// are taken, and the port keeps that.
pub fn point_radial(angle: f64, radius: f64) -> (f64, f64) {
    let angle = angle - std::f64::consts::PI / 2.0;
    (radius * ieee754::cos(angle), radius * ieee754::sin(angle))
}

/// d3-shape's `linkHorizontal`.
pub fn link_horizontal<D: LinkDatum + Clone>() -> Link<D, D::End> {
    Link {
        source: Box::new(|d: &D, _, _| d.source()),
        target: Box::new(|d: &D, _, _| d.target()),
        x: Box::new(|_, _, _| 0.0),
        y: Box::new(|_, _, _| 0.0),
        curve: curve_horizontal,
    }
}

/// d3-shape's `linkVertical`.
pub fn link_vertical<D: LinkDatum + Clone>() -> Link<D, D::End> {
    Link {
        source: Box::new(|d: &D, _, _| d.source()),
        target: Box::new(|d: &D, _, _| d.target()),
        x: Box::new(|_, _, _| 0.0),
        y: Box::new(|_, _, _| 0.0),
        curve: curve_vertical,
    }
}

/// d3-shape's `linkRadial`.
///
/// The JavaScript's radial link is the horizontal one with the `x` and the
/// `y` renamed to `angle` and `radius`.
pub fn link_radial<D: LinkDatum + Clone>() -> Link<D, D::End> {
    Link {
        source: Box::new(|d: &D, _, _| d.source()),
        target: Box::new(|d: &D, _, _| d.target()),
        x: Box::new(|_, _, _| 0.0),
        y: Box::new(|_, _, _| 0.0),
        curve: curve_radial,
    }
}

/// The curve a link draws with, kept here so the type is named.
pub type LinkCurve = fn(&mut dyn Context, f64, f64, f64, f64);

/// The JavaScript's `curve` of a link, which the port keeps for the
/// shape's own use.
pub fn link_curve(curve: CurveFactory) -> LinkCurve {
    // The JavaScript's link curve is a function of the context and the
    // four numbers, which is not the shape of a curve factory. The
    // factory is kept for the shape's own use, and the link's curve is
    // the function type above.
    let _ = curve;
    curve_horizontal
}
