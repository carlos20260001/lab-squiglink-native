//! The JavaScript's `Cubehelix` colours.
//!
//! Translated from `d3-color/src/cubehelix.js`, version 1.4.1, of d3
//! 5.16.0.

use crate::arith;
use crate::color::{Color, Cubehelix, Rgb};
use crate::ieee754;

/// The JavaScript's `A`.
const A: f64 = -0.14861;
/// The JavaScript's `B`.
const B: f64 = 1.78277;
/// The JavaScript's `C`.
const C: f64 = -0.29227;
/// The JavaScript's `D`.
const D: f64 = -0.90649;
/// The JavaScript's `E`.
const E: f64 = 1.97294;
/// The JavaScript's `ED`.
const ED: f64 = E * D;
/// The JavaScript's `EB`.
const EB: f64 = E * B;
/// The JavaScript's `BC_DA`.
const BC_DA: f64 = B * C - D * A;

impl Cubehelix {
    /// The colour a little brighter.
    pub fn brighter(&self, k: Option<f64>) -> Cubehelix {
        let k = match k {
            None => crate::color::BRIGHTER,
            Some(k) => ieee754::pow(crate::color::BRIGHTER, k),
        };
        Cubehelix {
            h: self.h,
            s: self.s,
            l: arith::multiply(self.l, k),
            opacity: self.opacity,
        }
    }

    /// The colour a little darker.
    pub fn darker(&self, k: Option<f64>) -> Cubehelix {
        let k = match k {
            None => crate::color::DARKER,
            Some(k) => ieee754::pow(crate::color::DARKER, k),
        };
        Cubehelix {
            h: self.h,
            s: self.s,
            l: arith::multiply(self.l, k),
            opacity: self.opacity,
        }
    }

    /// The colour as an `Rgb`.
    pub fn rgb(&self) -> Rgb {
        let h = if self.h.is_nan() {
            0.0
        } else {
            arith::divide(
                arith::multiply(arith::add(self.h, 120.0), std::f64::consts::PI),
                180.0,
            )
        };
        let l = self.l;
        let a = if self.s.is_nan() {
            0.0
        } else {
            arith::multiply(arith::multiply(self.s, l), arith::subtract(1.0, l))
        };
        let cosh = ieee754::cos(h);
        let sinh = ieee754::sin(h);
        Rgb {
            r: arith::multiply(
                255.0,
                arith::add(
                    l,
                    arith::multiply(
                        a,
                        arith::add(arith::multiply(A, cosh), arith::multiply(B, sinh)),
                    ),
                ),
            ),
            g: arith::multiply(
                255.0,
                arith::add(
                    l,
                    arith::multiply(
                        a,
                        arith::add(arith::multiply(C, cosh), arith::multiply(D, sinh)),
                    ),
                ),
            ),
            b: arith::multiply(
                255.0,
                arith::add(l, arith::multiply(a, arith::multiply(E, cosh))),
            ),
            opacity: self.opacity,
        }
    }
}

/// The JavaScript's `cubehelix`, which makes a colour of hue, saturation
/// and lightness in the cubehelix space.
pub fn cubehelix(h: f64, s: f64, l: f64, opacity: Option<f64>) -> Color {
    Color::Cubehelix(Cubehelix {
        h,
        s,
        l,
        opacity: opacity.unwrap_or(1.0),
    })
}

/// The JavaScript's `cubehelixConvert`, which makes a `Cubehelix` of any
/// colour.
pub fn cubehelix_convert(color: Color) -> Cubehelix {
    if let Color::Cubehelix(cubehelix) = color {
        return cubehelix;
    }
    let o = color.rgb();
    let r = arith::divide(o.r, 255.0);
    let g = arith::divide(o.g, 255.0);
    let b = arith::divide(o.b, 255.0);
    let l = arith::divide(
        arith::subtract(
            arith::add(arith::multiply(BC_DA, b), arith::multiply(ED, r)),
            arith::multiply(EB, g),
        ),
        arith::add(arith::subtract(BC_DA, EB), ED),
    );
    let bl = arith::subtract(b, l);
    let k = arith::divide(
        arith::subtract(
            arith::multiply(E, arith::subtract(g, l)),
            arith::multiply(C, bl),
        ),
        D,
    );
    let s = arith::divide(
        arith::add(arith::multiply(k, k), arith::multiply(bl, bl)).sqrt(),
        arith::multiply(arith::multiply(E, l), arith::subtract(1.0, l)),
    );
    let h = if s != 0.0 {
        arith::subtract(
            arith::divide(
                arith::multiply(ieee754::atan2(k, bl), 180.0),
                std::f64::consts::PI,
            ),
            120.0,
        )
    } else {
        f64::NAN
    };
    Cubehelix {
        h: if h < 0.0 { arith::add(h, 360.0) } else { h },
        s,
        l,
        opacity: o.opacity,
    }
}
