//! The JavaScript's `Lab` and `Hcl` colours.
//!
//! Translated from `d3-color/src/lab.js`, version 1.4.1, of d3 5.16.0.

use crate::arith;
use crate::color::{Color, Hcl, Lab, Rgb};
use crate::ieee754;

/// The JavaScript's `K`, which is how much a Lab colour brightens.
const K: f64 = 18.0;
/// The JavaScript's `Xn`.
const XN: f64 = 0.96422;
/// The JavaScript's `Yn`.
const YN: f64 = 1.0;
/// The JavaScript's `Zn`.
const ZN: f64 = 0.82521;
/// The JavaScript's `t0`.
const T0: f64 = 4.0 / 29.0;
/// The JavaScript's `t1`.
const T1: f64 = 6.0 / 29.0;
/// The JavaScript's `t2`.
const T2: f64 = 3.0 * T1 * T1;
/// The JavaScript's `t3`.
const T3: f64 = T1 * T1 * T1;

impl Lab {
    /// The colour a little brighter.
    pub fn brighter(&self, k: Option<f64>) -> Lab {
        Lab {
            l: arith::add(self.l, arith::multiply(K, k.unwrap_or(1.0))),
            a: self.a,
            b: self.b,
            opacity: self.opacity,
        }
    }

    /// The colour a little darker.
    pub fn darker(&self, k: Option<f64>) -> Lab {
        Lab {
            l: arith::subtract(self.l, arith::multiply(K, k.unwrap_or(1.0))),
            a: self.a,
            b: self.b,
            opacity: self.opacity,
        }
    }

    /// The colour as an `Rgb`.
    pub fn rgb(&self) -> Rgb {
        let y = arith::divide(arith::add(self.l, 16.0), 116.0);
        let x = if self.a.is_nan() {
            y
        } else {
            arith::add(y, arith::divide(self.a, 500.0))
        };
        let z = if self.b.is_nan() {
            y
        } else {
            arith::subtract(y, arith::divide(self.b, 200.0))
        };
        let x = arith::multiply(XN, lab2xyz(x));
        let y = arith::multiply(YN, lab2xyz(y));
        let z = arith::multiply(ZN, lab2xyz(z));
        Rgb {
            r: lrgb2rgb(arith::subtract(
                arith::subtract(arith::multiply(3.1338561, x), arith::multiply(1.6168667, y)),
                arith::multiply(0.4906146, z),
            )),
            g: lrgb2rgb(arith::add(
                arith::add(
                    arith::multiply(-0.9787684, x),
                    arith::multiply(1.9161415, y),
                ),
                arith::multiply(0.0334540, z),
            )),
            b: lrgb2rgb(arith::add(
                arith::subtract(arith::multiply(0.0719453, x), arith::multiply(0.2289914, y)),
                arith::multiply(1.4052427, z),
            )),
            opacity: self.opacity,
        }
    }
}

impl Hcl {
    /// The colour a little brighter.
    pub fn brighter(&self, k: Option<f64>) -> Hcl {
        Hcl {
            h: self.h,
            c: self.c,
            l: arith::add(self.l, arith::multiply(K, k.unwrap_or(1.0))),
            opacity: self.opacity,
        }
    }

    /// The colour a little darker.
    pub fn darker(&self, k: Option<f64>) -> Hcl {
        Hcl {
            h: self.h,
            c: self.c,
            l: arith::subtract(self.l, arith::multiply(K, k.unwrap_or(1.0))),
            opacity: self.opacity,
        }
    }

    /// The colour as an `Rgb`.
    pub fn rgb(&self) -> Rgb {
        hcl2lab(*self).rgb()
    }
}

/// The JavaScript's `lab`, which makes a colour of lightness and two
/// axes.
pub fn lab(l: f64, a: f64, b: f64, opacity: Option<f64>) -> Color {
    Color::Lab(Lab {
        l,
        a,
        b,
        opacity: opacity.unwrap_or(1.0),
    })
}

/// The JavaScript's `labConvert`, which makes a `Lab` of any colour.
pub fn lab_convert(color: Color) -> Lab {
    if let Color::Lab(lab) = color {
        return lab;
    }
    if let Color::Hcl(hcl) = color {
        return hcl2lab(hcl);
    }
    let o = color.rgb();
    let r = rgb2lrgb(o.r);
    let g = rgb2lrgb(o.g);
    let b = rgb2lrgb(o.b);
    let y = xyz2lab(arith::divide(
        arith::add(
            arith::add(arith::multiply(0.2225045, r), arith::multiply(0.7168786, g)),
            arith::multiply(0.0606169, b),
        ),
        YN,
    ));
    let (x, z) = if r == g && g == b {
        (y, y)
    } else {
        (
            xyz2lab(arith::divide(
                arith::add(
                    arith::add(arith::multiply(0.4360747, r), arith::multiply(0.3850649, g)),
                    arith::multiply(0.1430804, b),
                ),
                XN,
            )),
            xyz2lab(arith::divide(
                arith::add(
                    arith::add(arith::multiply(0.0139322, r), arith::multiply(0.0971045, g)),
                    arith::multiply(0.7141733, b),
                ),
                ZN,
            )),
        )
    };
    Lab {
        l: arith::subtract(arith::multiply(116.0, y), 16.0),
        a: arith::multiply(500.0, arith::subtract(x, y)),
        b: arith::multiply(200.0, arith::subtract(y, z)),
        opacity: o.opacity,
    }
}

/// The JavaScript's `gray`, which makes a colour of no colour at all.
pub fn gray(l: f64, opacity: Option<f64>) -> Color {
    Color::Lab(Lab {
        l,
        a: 0.0,
        b: 0.0,
        opacity: opacity.unwrap_or(1.0),
    })
}

/// The JavaScript's `hcl`, which makes a colour of hue, chroma and
/// lightness.
pub fn hcl(h: f64, c: f64, l: f64, opacity: Option<f64>) -> Color {
    Color::Hcl(Hcl {
        h,
        c,
        l,
        opacity: opacity.unwrap_or(1.0),
    })
}

/// The JavaScript's `lch`, which makes a colour of lightness, chroma and
/// hue.
pub fn lch(l: f64, c: f64, h: f64, opacity: Option<f64>) -> Color {
    Color::Hcl(Hcl {
        h,
        c,
        l,
        opacity: opacity.unwrap_or(1.0),
    })
}

/// The JavaScript's `hclConvert`, which makes an `Hcl` of any colour.
pub fn hcl_convert(color: Color) -> Hcl {
    if let Color::Hcl(hcl) = color {
        return hcl;
    }
    let o = lab_convert(color);
    if o.a == 0.0 && o.b == 0.0 {
        return Hcl {
            h: f64::NAN,
            c: if o.l > 0.0 && o.l < 100.0 {
                0.0
            } else {
                f64::NAN
            },
            l: o.l,
            opacity: o.opacity,
        };
    }
    let h = arith::divide(
        arith::multiply(ieee754::atan2(o.b, o.a), 180.0),
        std::f64::consts::PI,
    );
    Hcl {
        h: if h < 0.0 { arith::add(h, 360.0) } else { h },
        c: arith::add(arith::multiply(o.a, o.a), arith::multiply(o.b, o.b)).sqrt(),
        l: o.l,
        opacity: o.opacity,
    }
}

/// The JavaScript's `hcl2lab`.
fn hcl2lab(o: Hcl) -> Lab {
    if o.h.is_nan() {
        return Lab {
            l: o.l,
            a: 0.0,
            b: 0.0,
            opacity: o.opacity,
        };
    }
    let h = arith::divide(arith::multiply(o.h, std::f64::consts::PI), 180.0);
    Lab {
        l: o.l,
        a: arith::multiply(ieee754::cos(h), o.c),
        b: arith::multiply(ieee754::sin(h), o.c),
        opacity: o.opacity,
    }
}

/// The JavaScript's `xyz2lab`.
fn xyz2lab(t: f64) -> f64 {
    if t > T3 {
        ieee754::pow(t, arith::divide(1.0, 3.0))
    } else {
        arith::add(arith::divide(t, T2), T0)
    }
}

/// The JavaScript's `lab2xyz`.
fn lab2xyz(t: f64) -> f64 {
    if t > T1 {
        arith::multiply(arith::multiply(t, t), t)
    } else {
        arith::multiply(T2, arith::subtract(t, T0))
    }
}

/// The JavaScript's `lrgb2rgb`.
fn lrgb2rgb(x: f64) -> f64 {
    if x <= 0.0031308 {
        arith::multiply(255.0, arith::multiply(12.92, x))
    } else {
        arith::multiply(
            255.0,
            arith::subtract(
                arith::multiply(1.055, ieee754::pow(x, arith::divide(1.0, 2.4))),
                0.055,
            ),
        )
    }
}

/// The JavaScript's `rgb2lrgb`.
fn rgb2lrgb(x: f64) -> f64 {
    let x = arith::divide(x, 255.0);
    if x <= 0.04045 {
        arith::divide(x, 12.92)
    } else {
        ieee754::pow(arith::divide(arith::add(x, 0.055), 1.055), 2.4)
    }
}
