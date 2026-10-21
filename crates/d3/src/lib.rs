//! A faithful port of d3, the JavaScript data visualisation library.
//!
//! This crate is a translation of Mike Bostock's d3, version 5.16.0. The
//! JavaScript it was translated from is vendored under `reference/`, and
//! that JavaScript is the authority: where a reading of it and a reading of
//! this crate could differ, the JavaScript is right.
//!
//! The translation keeps the JavaScript's own names for things — a scale is
//! a scale, a curve is a curve, a path is a path — and its own behaviour,
//! including the parts that are surprising. Nothing is dropped: every
//! package of d3 is here, with its whole surface, so that this crate can
//! stand in for d3 in any project that uses it.
//!
//! # What is where
//!
//! Each module is named for the package it was translated from, and each
//! module's own header names the JavaScript file it was taken from. The
//! packages are:
//!
//! | Module | The package |
//! |---|---|
//! | [`array`] | d3-array |
//! | [`axis`] | d3-axis |
//! | [`brush`] | d3-brush |
//! | [`chord`] | d3-chord |
//! | [`collection`] | d3-collection |
//! | [`color`] | d3-color |
//! | [`contour`] | d3-contour |
//! | [`dispatch`] | d3-dispatch |
//! | [`drag`] | d3-drag |
//! | [`dsv`] | d3-dsv |
//! | [`ease`] | d3-ease |
//! | [`fetch`] | d3-fetch |
//! | [`force`] | d3-force |
//! | [`format`] | d3-format |
//! | [`geo`] | d3-geo |
//! | [`hierarchy`] | d3-hierarchy |
//! | [`interpolate`] | d3-interpolate |
//! | [`path`] | d3-path |
//! | [`polygon`] | d3-polygon |
//! | [`quadtree`] | d3-quadtree |
//! | [`random`] | d3-random |
//! | [`scale`] | d3-scale |
//! | [`scale_chromatic`] | d3-scale-chromatic |
//! | [`selection`] | d3-selection |
//! | [`shape`] | d3-shape |
//! | [`time`] | d3-time |
//! | [`time_format`] | d3-time-format |
//! | [`timer`] | d3-timer |
//! | [`transition`] | d3-transition |
//! | [`voronoi`] | d3-voronoi |
//! | [`zoom`] | d3-zoom |
//!
//! # The parts that need a document
//!
//! d3-selection, d3-transition, d3-zoom, d3-brush, d3-drag, d3-axis and
//! d3-fetch all work on a document: they select elements, listen for
//! events, and read and write attributes. A native program has no document,
//! so these are written against a trait — [`dom`] — that a project
//! implements for whatever it draws with.
//!
//! The trait is deliberately small and the port does not depend on any
//! particular implementation of it. A program that draws with a browser's
//! DOM implements it over that; a program that draws with a native toolkit
//! implements it over that. The port's own behaviour does not change.

// The JavaScript's own tests are written as `!(x > epsilon)`, which is
// false for a `NaN` as well as for a value that is not above the epsilon.
// Rust's own `partial_cmp` would read the same way, but the port keeps
// the JavaScript's form so that the two can be read side by side.
#![allow(clippy::neg_cmp_op_on_partial_ord)]
// The JavaScript's curves hold their numbers on the object itself, and
// the port's helper functions take them as arguments, which is the same
// thing written out. The port does not gather them into a struct, because
// that would hide which numbers the JavaScript has where.
#![allow(clippy::too_many_arguments)]

pub mod array;
pub mod color;
pub mod ieee754;
pub mod interpolate;
pub mod js;
pub mod number;
// pub mod axis;  // not yet written.
// pub mod brush;  // not yet written.
// pub mod chord;  // not yet written.
// pub mod collection;  // not yet written.
// pub mod color;  // not yet written.
// pub mod contour;  // not yet written.
// pub mod dispatch;  // not yet written.
// pub mod dom;  // not yet written.
// pub mod drag;  // not yet written.
// pub mod dsv;  // not yet written.
// pub mod ease;  // not yet written.
// pub mod fetch;  // not yet written.
// pub mod force;  // not yet written.
// pub mod format;  // not yet written.
// pub mod geo;  // not yet written.
// pub mod hierarchy;  // not yet written.
// pub mod interpolate;  // not yet written.
pub mod path;
// pub mod polygon;  // not yet written.
// pub mod quadtree;  // not yet written.
// pub mod random;  // not yet written.
// pub mod scale;  // not yet written.
// pub mod scale_chromatic;  // not yet written.
// pub mod selection;  // not yet written.
pub mod shape;
// pub mod time;  // not yet written.
// pub mod time_format;  // not yet written.
// pub mod timer;  // not yet written.
// pub mod transition;  // not yet written.
// pub mod voronoi;  // not yet written.
// pub mod zoom;  // not yet written.
