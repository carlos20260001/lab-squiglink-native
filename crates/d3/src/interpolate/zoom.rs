//! The JavaScript's `interpolateZoom`.
//!
//! Translated from `d3-interpolate/src/zoom.js`, version 1.4.0, of d3
//! 5.16.0.

use crate::ieee754;

/// The JavaScript's `interpolateZoom`, which draws between two zooms.
///
/// A zoom is a centre and a width, and the drawing between them is the
/// JavaScript's own.
pub fn interpolate_zoom(p0: [f64; 3], p1: [f64; 3]) -> impl Fn(f64) -> [f64; 3] {
    const RHO: f64 = std::f64::consts::SQRT_2;
    const RHO2: f64 = 2.0;
    const RHO4: f64 = 4.0;
    const EPSILON2: f64 = 1e-12;

    let ux0 = p0[0];
    let uy0 = p0[1];
    let w0 = p0[2];
    let ux1 = p1[0];
    let uy1 = p1[1];
    let w1 = p1[2];
    let dx = ux1 - ux0;
    let dy = uy1 - uy0;
    let d2 = dx * dx + dy * dy;

    // The two cases, which the JavaScript has as two closures.
    // The first is the case where the two points are the same.
    let same = d2 < EPSILON2;
    let (s, r0, d1) = if same {
        (ieee754::log(w1 / w0) / RHO, f64::NAN, f64::NAN)
    } else {
        // The general case.
        let d1 = d2.sqrt();
        let b0 = (w1 * w1 - w0 * w0 + RHO4 * d2) / (2.0 * w0 * RHO2 * d1);
        let b1 = (w1 * w1 - w0 * w0 - RHO4 * d2) / (2.0 * w1 * RHO2 * d1);
        let r0 = ieee754::log((b0 * b0 + 1.0).sqrt() - b0);
        let r1 = ieee754::log((b1 * b1 + 1.0).sqrt() - b1);
        ((r1 - r0) / RHO, r0, d1)
    };

    move |t| {
        if same {
            // The case where the two points are the same.
            [ux0 + t * dx, uy0 + t * dy, w0 * ieee754::exp(RHO * t * s)]
        } else {
            // The general case.
            let s = t * s;
            let coshr0 = cosh(r0);
            let u = w0 / (RHO2 * d1) * (coshr0 * tanh(RHO * s + r0) - sinh(r0));
            [ux0 + u * dx, uy0 + u * dy, w0 * coshr0 / cosh(RHO * s + r0)]
        }
    }
}

/// The JavaScript's `cosh`.
fn cosh(x: f64) -> f64 {
    let x = ieee754::exp(x);
    (x + 1.0 / x) / 2.0
}

/// The JavaScript's `sinh`.
fn sinh(x: f64) -> f64 {
    let x = ieee754::exp(x);
    (x - 1.0 / x) / 2.0
}

/// The JavaScript's `tanh`.
fn tanh(x: f64) -> f64 {
    let x = ieee754::exp(2.0 * x);
    (x - 1.0) / (x + 1.0)
}
