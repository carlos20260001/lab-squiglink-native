//! The JavaScript's colour interpolators.
//!
//! Translated from `d3-interpolate/src/color.js` and the colour
//! interpolators beside it, version 1.4.0, of d3 5.16.0.

use crate::color::{Color, Cubehelix, Hcl, Hsl, Lab, Rgb};
use crate::ieee754;
use crate::interpolate::{interpolate_number, interpolate_number_array};

/// A function that draws between two numbers.
pub type Draw = Box<dyn Fn(f64) -> f64 + 'static>;

/// A function that draws between two numbers.
pub type Between = fn(f64, f64) -> Draw;

/// A function that draws between two numbers in the power of a third.
pub type Gamma = fn(f64) -> Box<dyn Fn(f64, f64) -> Draw + 'static>;

/// The JavaScript's `linear`, which draws between two numbers.
fn linear(a: f64, d: f64) -> impl Fn(f64) -> f64 {
    move |t| a + t * d
}

/// The JavaScript's `exponential`, which draws between two numbers in
/// the power of a third.
fn exponential(a: f64, b: f64, y: f64) -> impl Fn(f64) -> f64 {
    let a = ieee754::pow(a, y);
    let b = ieee754::pow(b, y) - a;
    let y = 1.0 / y;
    move |t| ieee754::pow(a + t * b, y)
}

/// The JavaScript's `hue`, which draws between two hues the short way
/// round.
pub fn hue(a: f64, b: f64) -> Draw {
    let d = b - a;
    if d != 0.0 {
        let d = if d > 180.0 || d < -180.0 {
            d - 360.0 * js_round(d / 360.0)
        } else {
            d
        };
        Box::new(linear(a, d))
    } else {
        let value = if a.is_nan() { b } else { a };
        Box::new(move |_| value)
    }
}

/// The JavaScript's `nogamma`, which draws between two numbers.
pub fn nogamma(a: f64, b: f64) -> Draw {
    let d = b - a;
    if d != 0.0 {
        Box::new(linear(a, d))
    } else {
        let value = if a.is_nan() { b } else { a };
        Box::new(move |_| value)
    }
}

/// The JavaScript's `gamma`, which draws between two numbers in the
/// power of a third.
pub fn gamma(y: f64) -> Box<dyn Fn(f64, f64) -> Draw + 'static> {
    if y == 1.0 {
        return Box::new(|a, b| nogamma(a, b));
    }
    Box::new(move |a, b| {
        if b - a != 0.0 {
            Box::new(exponential(a, b, y))
        } else {
            let value = if a.is_nan() { b } else { a };
            Box::new(move |_| value)
        }
    })
}

/// The JavaScript's `rgb`, which draws between two colours.
///
/// The JavaScript's colours are written out as they are drawn, and the
/// port gives the text of the colour ✓.
pub fn interpolate_rgb(start: &Color, end: &Color) -> impl Fn(f64) -> String {
    let start = start.rgb();
    let end = end.rgb();
    let r = nogamma(start.r, end.r);
    let g = nogamma(start.g, end.g);
    let b = nogamma(start.b, end.b);
    let opacity = nogamma(start.opacity, end.opacity);
    move |t| {
        Rgb {
            r: r(t),
            g: g(t),
            b: b(t),
            opacity: opacity(t),
        }
        .format_rgb()
    }
}

/// The JavaScript's `rgb` with a gamma, which is the `rgb.gamma` of the
/// JavaScript.
pub fn interpolate_rgb_gamma(y: f64, start: &Color, end: &Color) -> impl Fn(f64) -> String {
    let start = start.rgb();
    let end = end.rgb();
    // The JavaScript calls this one `color`, which is the name of the
    // module it is in; the port calls it `colour`.
    let colour = gamma(y);
    let r = colour(start.r, end.r);
    let g = colour(start.g, end.g);
    let b = colour(start.b, end.b);
    let opacity = nogamma(start.opacity, end.opacity);
    move |t| {
        Rgb {
            r: r(t),
            g: g(t),
            b: b(t),
            opacity: opacity(t),
        }
        .format_rgb()
    }
}

/// The JavaScript's `hsl`, which draws between two colours in the
/// JavaScript's own hue, saturation and lightness.
pub fn interpolate_hsl(start: &Color, end: &Color) -> impl Fn(f64) -> String {
    interpolate_hsl_with(hue, start, end)
}

/// The JavaScript's `hslLong`, which draws the long way round.
pub fn interpolate_hsl_long(start: &Color, end: &Color) -> impl Fn(f64) -> String {
    interpolate_hsl_with(nogamma, start, end)
}

/// The JavaScript's `hsl` with a hue of its own.
fn interpolate_hsl_with(hue: Between, start: &Color, end: &Color) -> impl Fn(f64) -> String {
    let start = start.hsl();
    let end = end.hsl();
    let h = hue(start.h, end.h);
    let s = nogamma(start.s, end.s);
    let l = nogamma(start.l, end.l);
    let opacity = nogamma(start.opacity, end.opacity);
    move |t| {
        // The JavaScript writes the colour with its own `toString`,
        // which is the `rgb` form for every colour.
        Hsl {
            h: h(t),
            s: s(t),
            l: l(t),
            opacity: opacity(t),
        }
        .rgb()
        .format_rgb()
    }
}

/// The JavaScript's `lab`, which draws between two colours in the
/// JavaScript's own lightness and axes.
pub fn interpolate_lab(start: &Color, end: &Color) -> impl Fn(f64) -> String {
    let start = start.lab();
    let end = end.lab();
    let l = nogamma(start.l, end.l);
    let a = nogamma(start.a, end.a);
    let b = nogamma(start.b, end.b);
    let opacity = nogamma(start.opacity, end.opacity);
    move |t| {
        Lab {
            l: l(t),
            a: a(t),
            b: b(t),
            opacity: opacity(t),
        }
        .rgb()
        .format_rgb()
    }
}

/// The JavaScript's `hcl`, which draws between two colours in the
/// JavaScript's own hue, chroma and lightness.
pub fn interpolate_hcl(start: &Color, end: &Color) -> impl Fn(f64) -> String {
    interpolate_hcl_with(hue, start, end)
}

/// The JavaScript's `hclLong`, which draws the long way round.
pub fn interpolate_hcl_long(start: &Color, end: &Color) -> impl Fn(f64) -> String {
    interpolate_hcl_with(nogamma, start, end)
}

/// The JavaScript's `hcl` with a hue of its own.
fn interpolate_hcl_with(hue: Between, start: &Color, end: &Color) -> impl Fn(f64) -> String {
    let start = start.hcl();
    let end = end.hcl();
    let h = hue(start.h, end.h);
    let c = nogamma(start.c, end.c);
    let l = nogamma(start.l, end.l);
    let opacity = nogamma(start.opacity, end.opacity);
    move |t| {
        Hcl {
            h: h(t),
            c: c(t),
            l: l(t),
            opacity: opacity(t),
        }
        .rgb()
        .format_rgb()
    }
}

/// The JavaScript's `cubehelix`, which draws between two colours in the
/// JavaScript's own cubehelix.
pub fn interpolate_cubehelix(start: &Color, end: &Color) -> impl Fn(f64) -> String {
    interpolate_cubehelix_with(hue, 1.0, start, end)
}

/// The JavaScript's `cubehelixLong`, which draws the long way round.
pub fn interpolate_cubehelix_long(start: &Color, end: &Color) -> impl Fn(f64) -> String {
    interpolate_cubehelix_with(nogamma, 1.0, start, end)
}

/// The JavaScript's `cubehelix` with a gamma of its own.
pub fn interpolate_cubehelix_gamma(y: f64, start: &Color, end: &Color) -> impl Fn(f64) -> String {
    interpolate_cubehelix_with(hue, y, start, end)
}

/// The JavaScript's `cubehelix` with a hue and a gamma of its own.
fn interpolate_cubehelix_with(
    hue: Between,
    y: f64,
    start: &Color,
    end: &Color,
) -> impl Fn(f64) -> String {
    let start = start.cubehelix();
    let end = end.cubehelix();
    let h = hue(start.h, end.h);
    let s = nogamma(start.s, end.s);
    let l = nogamma(start.l, end.l);
    let opacity = nogamma(start.opacity, end.opacity);
    move |t| {
        Cubehelix {
            h: h(t),
            s: s(t),
            l: l(ieee754::pow(t, y)),
            opacity: opacity(t),
        }
        .rgb()
        .format_rgb()
    }
}

/// The JavaScript's `rgbSpline`, which draws between a list of colours
/// along a curve.
fn rgb_spline(spline: fn(&[f64]) -> Draw, colours: &[Color]) -> impl Fn(f64) -> String {
    let n = colours.len();
    let mut r = Vec::with_capacity(n);
    let mut g = Vec::with_capacity(n);
    let mut b = Vec::with_capacity(n);
    for colour in colours {
        let colour = colour.rgb();
        r.push(if colour.r == 0.0 { 0.0 } else { colour.r });
        g.push(if colour.g == 0.0 { 0.0 } else { colour.g });
        b.push(if colour.b == 0.0 { 0.0 } else { colour.b });
    }
    let r = spline(&r);
    let g = spline(&g);
    let b = spline(&b);
    move |t| {
        Rgb {
            r: r(t),
            g: g(t),
            b: b(t),
            opacity: 1.0,
        }
        .format_rgb()
    }
}

/// The JavaScript's `rgbBasis`, which draws between a list of colours
/// along the JavaScript's own basis curve.
pub fn interpolate_rgb_basis(colours: &[Color]) -> impl Fn(f64) -> String {
    rgb_spline(crate::interpolate::curve::basis, colours)
}

/// The JavaScript's `rgbBasisClosed`, which draws between a list of
/// colours along the JavaScript's own closed basis curve.
pub fn interpolate_rgb_basis_closed(colours: &[Color]) -> impl Fn(f64) -> String {
    rgb_spline(crate::interpolate::curve::basis_closed, colours)
}

/// The JavaScript's own rounding of a number, which is the JavaScript's
/// `Math.round`.
fn js_round(value: f64) -> f64 {
    crate::js::round(value)
}

/// The numbers of a list of colours, which the JavaScript's `rgbSpline`
/// reads.
pub fn colour_numbers(colours: &[Color]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut r = Vec::with_capacity(colours.len());
    let mut g = Vec::with_capacity(colours.len());
    let mut b = Vec::with_capacity(colours.len());
    for colour in colours {
        let colour = colour.rgb();
        r.push(colour.r);
        g.push(colour.g);
        b.push(colour.b);
    }
    (r, g, b)
}

/// The JavaScript's `interpolateNumberArray`, which the port's own
/// number array uses.
pub fn numbers(a: &[f64], b: &[f64]) -> impl Fn(f64) -> Vec<f64> {
    interpolate_number_array(a, b)
}

/// The JavaScript's `interpolateNumber`, which the port's own numbers
/// use.
pub fn numbers_between(a: f64, b: f64) -> impl Fn(f64) -> f64 {
    interpolate_number(a, b)
}
