//! A port of **d3-shape**, from the JavaScript of the same name.
//!
//! Translated from `d3-shape/src/*.js`, version 1.3.7, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read differently,
//! the JavaScript is right.
//!
//! d3-shape draws: the lines, the areas, the arcs, the pie slices, the
//! symbols and the stacked bands, and the curves they are drawn with.
//!
//! # How a curve draws
//!
//! In the JavaScript a curve is an object with a context inside it, and it
//! is a small state machine: `areaStart`, `areaEnd`, `lineStart`,
//! `lineEnd` and `point`. The generator — the line, or the area — calls
//! those in order, and the curve writes into the context.
//!
//! The port keeps that: a curve is a struct with the JavaScript's own
//! fields, and the context is given to each of the five calls. The
//! context is anything that can be drawn into, which in practice is
//! [`crate::path::Path`].

// The JavaScript's loops here go by the place of a thing in a list, and
// the port keeps that, because the place is what the JavaScript's
// arithmetic is about.
#![allow(clippy::needless_range_loop)]

pub mod arc;
pub mod area;
pub mod curve;
pub mod line;
pub mod link;
pub mod pie;
pub mod radial;
pub mod stack;
pub mod symbol;

pub use arc::{Arc, ArcDatum};
pub use area::Area;
pub use curve::{
    curve_basis, curve_basis_closed, curve_basis_open, curve_bundle, curve_cardinal,
    curve_cardinal_closed, curve_cardinal_open, curve_catmull_rom, curve_catmull_rom_closed,
    curve_catmull_rom_open, curve_linear, curve_linear_closed, curve_monotone_x, curve_monotone_y,
    curve_natural, curve_step, curve_step_after, curve_step_before, Curve, CurveFactory,
};
pub use line::Line;
pub use link::{link_horizontal, link_radial, link_vertical, Link, LinkDatum};
pub use pie::Pie;
pub use radial::{RadialArea, RadialLine};
pub use stack::{
    stack_offset_diverging, stack_offset_expand, stack_offset_none, stack_offset_silhouette,
    stack_offset_wiggle, stack_order_appearance, stack_order_ascending, stack_order_descending,
    stack_order_inside_out, stack_order_none, stack_order_reverse, Series, SeriesPoint, Stack,
    StackDatum,
};
pub use symbol::{
    symbol_circle, symbol_cross, symbol_diamond, symbol_square, symbol_star, symbol_triangle,
    symbol_wye, Symbol, SymbolType,
};

/// Something a curve can draw into.
///
/// The JavaScript's "context" is any object with the path's methods on it,
/// and d3-shape only ever asks for six of them. The port asks for the same
/// six, and [`crate::path::Path`] is the one that answers.
pub trait Context {
    /// The JavaScript's `moveTo`.
    fn move_to(&mut self, x: f64, y: f64);
    /// The JavaScript's `lineTo`.
    fn line_to(&mut self, x: f64, y: f64);
    /// The JavaScript's `closePath`.
    fn close_path(&mut self);
    /// The JavaScript's `quadraticCurveTo`.
    fn quadratic_curve_to(&mut self, x1: f64, y1: f64, x: f64, y: f64);
    /// The JavaScript's `bezierCurveTo`.
    fn bezier_curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64);
    /// The JavaScript's `arc`.
    ///
    /// The JavaScript's d3-path throws when the radius is negative, and
    /// the port returns the error rather than panicking.
    fn arc(
        &mut self,
        x: f64,
        y: f64,
        r: f64,
        a0: f64,
        a1: f64,
        ccw: bool,
    ) -> Result<(), crate::path::PathError>;
    /// The JavaScript's `rect`.
    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64);
}

/// The point the JavaScript's `point.x` reads: the first of the pair.
pub fn point_x(p: &[f64]) -> f64 {
    p[0]
}

/// The point the JavaScript's `point.y` reads: the second of the pair.
pub fn point_y(p: &[f64]) -> f64 {
    p[1]
}
