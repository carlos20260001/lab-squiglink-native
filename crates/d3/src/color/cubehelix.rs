//! The JavaScript's `Cubehelix` colours.
//!
//! Translated from `d3-color/src/cubehelix.js`, version 1.4.1, of d3
//! 5.16.0.

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
        let k = k.unwrap_or(crate::color::BRIGHTER);
        Cubehelix {
            h: self.h,
            s: self.s,
            l: self.l * k,
            opacity: self.opacity,
        }
    }

    /// The colour a little darker.
    pub fn darker(&self, k: Option<f64>) -> Cubehelix {
        let k = k.unwrap_or(crate::color::DARKER);
        Cubehelix {
            h: self.h,
            s: self.s,
            l: self.l * k,
            opacity: self.opacity,
        }
    }

    /// The colour as an `Rgb`.
    pub fn rgb(&self) -> Rgb {
        let h = if self.h.is_nan() {
            0.0
        } else {
            (self.h + 120.0) * std::f64::consts::PI / 180.0
        };
        let l = self.l;
        let a = if self.s.is_nan() {
            0.0
        } else {
            self.s * l * (1.0 - l)
        };
        let cosh = ieee754::cos(h);
        let sinh = ieee754::sin(h);
        Rgb {
            r: 255.0 * (l + a * (A * cosh + B * sinh)),
            g: 255.0 * (l + a * (C * cosh + D * sinh)),
            b: 255.0 * (l + a * (E * cosh)),
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
    let r = o.r / 255.0;
    let g = o.g / 255.0;
    let b = o.b / 255.0;
    let l = (BC_DA * b + ED * r - EB * g) / (BC_DA + ED - EB);
    let bl = b - l;
    let k = (E * (g - l) - C * bl) / D;
    let s = (k * k + bl * bl).sqrt() / (E * l * (1.0 - l));
    let h = if s != 0.0 {
        ieee754::atan2(k, bl) * 180.0 / std::f64::consts::PI - 120.0
    } else {
        f64::NAN
    };
    Cubehelix {
        h: if h < 0.0 { h + 360.0 } else { h },
        s,
        l,
        opacity: o.opacity,
    }
}
