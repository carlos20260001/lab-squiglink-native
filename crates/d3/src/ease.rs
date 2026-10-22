//! A port of **d3-ease**, from the JavaScript of the same name.
//!
//! Translated from `d3-ease/src/*.js`, version 1.0.7, of d3 5.16.0. The
//! JavaScript is the authority; where the two could be read differently, the
//! JavaScript is right.
//!
//! d3-ease is the set of eases that d3's transitions are drawn with: each
//! of them takes a number, which is how far the transition has gone, and
//! gives back the number it is at.
//!
//! # The shape of the port
//!
//! The JavaScript's eases are plain functions, `t -> number`, and most of
//! them are all the package has. Those are here as plain functions too,
//! named for the JavaScript's own exports: `easeCubicIn` is
//! [`ease_cubic_in`], and `easeBounce` is [`ease_bounce`]. Where the
//! JavaScript's export is another name for one of the family's members —
//! `easeCubic` is `cubicInOut` — the port says so.
//!
//! Three of the families have a setting of their own: the polys have an
//! exponent, the backs an overshoot, and the elastic ones an amplitude and
//! a period. The JavaScript hangs the setting on the function itself, as
//! `easePoly.exponent(4)`, and the setter gives back a *new* function. Here
//! the setting is held by a small struct, and the setter is a method on it
//! that gives back another of them: [`Poly::exponent`], [`Back::overshoot`],
//! [`Elastic::amplitude`] and [`Elastic::period`].
//!
//! # The numbers
//!
//! The JavaScript's `Math.sin`, `Math.cos`, `Math.asin` and `Math.pow` are
//! V8's own, and are not the platform's: the port uses the crate's own
//! [`crate::ieee754`], which is the same code, so that the eases agree with
//! the JavaScript to the last bit.

use crate::arith;
use crate::ieee754;
use crate::js;

/// The pi the JavaScript uses.
const PI: f64 = std::f64::consts::PI;
/// Half of pi, which the JavaScript's `sin` uses.
const HALF_PI: f64 = PI / 2.0;
/// Twice pi, which the JavaScript's `elastic` uses.
const TAU: f64 = 2.0 * PI;

// The bounce's own numbers, from `d3-ease/src/bounce.js`.
const B1: f64 = 4.0 / 11.0;
const B2: f64 = 6.0 / 11.0;
const B3: f64 = 8.0 / 11.0;
const B4: f64 = 3.0 / 4.0;
const B5: f64 = 9.0 / 11.0;
const B6: f64 = 10.0 / 11.0;
const B7: f64 = 15.0 / 16.0;
const B8: f64 = 21.0 / 22.0;
const B9: f64 = 63.0 / 64.0;
const B0: f64 = 1.0 / B1 / B1;

// The poly's own exponent, from `d3-ease/src/poly.js`.
const EXPONENT: f64 = 3.0;

// The back's own overshoot, from `d3-ease/src/back.js`.
const OVERSHOOT: f64 = 1.70158;

// The elastic's own amplitude and period, from `d3-ease/src/elastic.js`.
const AMPLITUDE: f64 = 1.0;
const PERIOD: f64 = 0.3;

/// d3-ease's `easeLinear`, which is the JavaScript's `linear`.
///
/// The JavaScript:
///
/// ```js
/// export function linear(t) {
///   return +t;
/// }
/// ```
///
/// The JavaScript's `+t` is the number itself, and the port's
/// [`js::number`] is the same ✓.
pub fn ease_linear(t: f64) -> f64 {
    js::number(t)
}

/// d3-ease's `easeQuad`, which is the JavaScript's `quadInOut`.
///
/// The JavaScript:
///
/// ```js
/// export function quadInOut(t) {
///   return ((t *= 2) <= 1 ? t * t : --t * (2 - t) + 1) / 2;
/// }
/// ```
pub fn ease_quad(t: f64) -> f64 {
    ease_quad_in_out(t)
}

/// d3-ease's `easeQuadIn`, which is the JavaScript's `quadIn`.
///
/// The JavaScript:
///
/// ```js
/// export function quadIn(t) {
///   return t * t;
/// }
/// ```
pub fn ease_quad_in(t: f64) -> f64 {
    t * t
}

/// d3-ease's `easeQuadOut`, which is the JavaScript's `quadOut`.
///
/// The JavaScript:
///
/// ```js
/// export function quadOut(t) {
///   return t * (2 - t);
/// }
/// ```
pub fn ease_quad_out(t: f64) -> f64 {
    t * (2.0 - t)
}

/// d3-ease's `easeQuadInOut`, which is the JavaScript's `quadInOut`.
///
/// The JavaScript:
///
/// ```js
/// export function quadInOut(t) {
///   return ((t *= 2) <= 1 ? t * t : --t * (2 - t) + 1) / 2;
/// }
/// ```
///
/// The JavaScript's `--t` takes one off the doubled number, and the port's
/// does the same ✓.
pub fn ease_quad_in_out(t: f64) -> f64 {
    let mut t = t * 2.0;
    let value = if t <= 1.0 {
        t * t
    } else {
        t -= 1.0;
        t * (2.0 - t) + 1.0
    };
    value / 2.0
}

/// d3-ease's `easeCubic`, which is the JavaScript's `cubicInOut`.
///
/// The JavaScript:
///
/// ```js
/// export function cubicInOut(t) {
///   return ((t *= 2) <= 1 ? t * t * t : (t -= 2) * t * t + 2) / 2;
/// }
/// ```
pub fn ease_cubic(t: f64) -> f64 {
    ease_cubic_in_out(t)
}

/// d3-ease's `easeCubicIn`, which is the JavaScript's `cubicIn`.
///
/// The JavaScript:
///
/// ```js
/// export function cubicIn(t) {
///   return t * t * t;
/// }
/// ```
pub fn ease_cubic_in(t: f64) -> f64 {
    t * t * t
}

/// d3-ease's `easeCubicOut`, which is the JavaScript's `cubicOut`.
///
/// The JavaScript:
///
/// ```js
/// export function cubicOut(t) {
///   return --t * t * t + 1;
/// }
/// ```
pub fn ease_cubic_out(t: f64) -> f64 {
    let t = t - 1.0;
    t * t * t + 1.0
}

/// d3-ease's `easeCubicInOut`, which is the JavaScript's `cubicInOut`.
///
/// The JavaScript:
///
/// ```js
/// export function cubicInOut(t) {
///   return ((t *= 2) <= 1 ? t * t * t : (t -= 2) * t * t + 2) / 2;
/// }
/// ```
pub fn ease_cubic_in_out(t: f64) -> f64 {
    let mut t = t * 2.0;
    let value = if t <= 1.0 {
        t * t * t
    } else {
        t -= 2.0;
        t * t * t + 2.0
    };
    value / 2.0
}

/// d3-ease's `easePoly`, which is the JavaScript's `polyInOut` with the
/// JavaScript's own exponent of three.
///
/// The JavaScript:
///
/// ```js
/// export var polyInOut = (function custom(e) {
///   e = +e;
///
///   function polyInOut(t) {
///     return ((t *= 2) <= 1 ? Math.pow(t, e) : 2 - Math.pow(2 - t, e)) / 2;
///   }
///
///   polyInOut.exponent = custom;
///
///   return polyInOut;
/// })(exponent);
/// ```
///
/// The exponent is held by [`Poly`], and this is that struct with the
/// JavaScript's own exponent ✓.
pub fn ease_poly(t: f64) -> f64 {
    Poly::default().poly_in_out(t)
}

/// d3-ease's `easePolyIn`, which is the JavaScript's `polyIn` with the
/// JavaScript's own exponent of three.
///
/// See [`Poly`], which holds the exponent.
pub fn ease_poly_in(t: f64) -> f64 {
    Poly::default().poly_in(t)
}

/// d3-ease's `easePolyOut`, which is the JavaScript's `polyOut` with the
/// JavaScript's own exponent of three.
///
/// See [`Poly`], which holds the exponent.
pub fn ease_poly_out(t: f64) -> f64 {
    Poly::default().poly_out(t)
}

/// d3-ease's `easePolyInOut`, which is the JavaScript's `polyInOut` with the
/// JavaScript's own exponent of three.
///
/// See [`Poly`], which holds the exponent.
pub fn ease_poly_in_out(t: f64) -> f64 {
    Poly::default().poly_in_out(t)
}

/// d3-ease's `easeSin`, which is the JavaScript's `sinInOut`.
///
/// The JavaScript:
///
/// ```js
/// export function sinInOut(t) {
///   return (1 - Math.cos(pi * t)) / 2;
/// }
/// ```
pub fn ease_sin(t: f64) -> f64 {
    ease_sin_in_out(t)
}

/// d3-ease's `easeSinIn`, which is the JavaScript's `sinIn`.
///
/// The JavaScript:
///
/// ```js
/// export function sinIn(t) {
///   return (+t === 1) ? 1 : 1 - Math.cos(t * halfPi);
/// }
/// ```
///
/// The JavaScript's `===` is not true of a `NaN`, and the port's is not
/// either ✓.
pub fn ease_sin_in(t: f64) -> f64 {
    if js::strict_equals(js::number(t), 1.0) {
        1.0
    } else {
        1.0 - ieee754::cos(t * HALF_PI)
    }
}

/// d3-ease's `easeSinOut`, which is the JavaScript's `sinOut`.
///
/// The JavaScript:
///
/// ```js
/// export function sinOut(t) {
///   return Math.sin(t * halfPi);
/// }
/// ```
pub fn ease_sin_out(t: f64) -> f64 {
    ieee754::sin(t * HALF_PI)
}

/// d3-ease's `easeSinInOut`, which is the JavaScript's `sinInOut`.
///
/// The JavaScript:
///
/// ```js
/// export function sinInOut(t) {
///   return (1 - Math.cos(pi * t)) / 2;
/// }
/// ```
pub fn ease_sin_in_out(t: f64) -> f64 {
    (1.0 - ieee754::cos(PI * t)) / 2.0
}

/// d3-ease's `easeExp`, which is the JavaScript's `expInOut`.
///
/// The JavaScript:
///
/// ```js
/// export function expInOut(t) {
///   return ((t *= 2) <= 1 ? tpmt(1 - t) : 2 - tpmt(t - 1)) / 2;
/// }
/// ```
pub fn ease_exp(t: f64) -> f64 {
    ease_exp_in_out(t)
}

/// d3-ease's `easeExpIn`, which is the JavaScript's `expIn`.
///
/// The JavaScript:
///
/// ```js
/// export function expIn(t) {
///   return tpmt(1 - +t);
/// }
/// ```
pub fn ease_exp_in(t: f64) -> f64 {
    tpmt(1.0 - js::number(t))
}

/// d3-ease's `easeExpOut`, which is the JavaScript's `expOut`.
///
/// The JavaScript:
///
/// ```js
/// export function expOut(t) {
///   return 1 - tpmt(t);
/// }
/// ```
pub fn ease_exp_out(t: f64) -> f64 {
    1.0 - tpmt(t)
}

/// d3-ease's `easeExpInOut`, which is the JavaScript's `expInOut`.
///
/// The JavaScript:
///
/// ```js
/// export function expInOut(t) {
///   return ((t *= 2) <= 1 ? tpmt(1 - t) : 2 - tpmt(t - 1)) / 2;
/// }
/// ```
pub fn ease_exp_in_out(t: f64) -> f64 {
    let t = t * 2.0;
    let value = if t <= 1.0 {
        tpmt(1.0 - t)
    } else {
        2.0 - tpmt(t - 1.0)
    };
    value / 2.0
}

/// d3-ease's `easeCircle`, which is the JavaScript's `circleInOut`.
///
/// The JavaScript:
///
/// ```js
/// export function circleInOut(t) {
///   return ((t *= 2) <= 1 ? 1 - Math.sqrt(1 - t * t) : Math.sqrt(1 - (t -= 2) * t) + 1) / 2;
/// }
/// ```
pub fn ease_circle(t: f64) -> f64 {
    ease_circle_in_out(t)
}

/// d3-ease's `easeCircleIn`, which is the JavaScript's `circleIn`.
///
/// The JavaScript:
///
/// ```js
/// export function circleIn(t) {
///   return 1 - Math.sqrt(1 - t * t);
/// }
/// ```
pub fn ease_circle_in(t: f64) -> f64 {
    1.0 - (1.0 - t * t).sqrt()
}

/// d3-ease's `easeCircleOut`, which is the JavaScript's `circleOut`.
///
/// The JavaScript:
///
/// ```js
/// export function circleOut(t) {
///   return Math.sqrt(1 - --t * t);
/// }
/// ```
pub fn ease_circle_out(t: f64) -> f64 {
    let t = t - 1.0;
    (1.0 - t * t).sqrt()
}

/// d3-ease's `easeCircleInOut`, which is the JavaScript's `circleInOut`.
///
/// The JavaScript:
///
/// ```js
/// export function circleInOut(t) {
///   return ((t *= 2) <= 1 ? 1 - Math.sqrt(1 - t * t) : Math.sqrt(1 - (t -= 2) * t) + 1) / 2;
/// }
/// ```
pub fn ease_circle_in_out(t: f64) -> f64 {
    let mut t = t * 2.0;
    let value = if t <= 1.0 {
        1.0 - (1.0 - t * t).sqrt()
    } else {
        t -= 2.0;
        (1.0 - t * t).sqrt() + 1.0
    };
    value / 2.0
}

/// d3-ease's `easeBounce`, which is the JavaScript's `bounceOut`.
///
/// The JavaScript's export is the *out* of the family, and not the in and
/// out as the other families' are:
///
/// ```js
/// export {
///   bounceOut as easeBounce,
///   ...
/// }
/// ```
pub fn ease_bounce(t: f64) -> f64 {
    ease_bounce_out(t)
}

/// d3-ease's `easeBounceIn`, which is the JavaScript's `bounceIn`.
///
/// The JavaScript:
///
/// ```js
/// export function bounceIn(t) {
///   return 1 - bounceOut(1 - t);
/// }
/// ```
pub fn ease_bounce_in(t: f64) -> f64 {
    1.0 - ease_bounce_out(1.0 - t)
}

/// d3-ease's `easeBounceOut`, which is the JavaScript's `bounceOut`.
///
/// The JavaScript:
///
/// ```js
/// export function bounceOut(t) {
///   return (t = +t) < b1 ? b0 * t * t : t < b3 ? b0 * (t -= b2) * t + b4 : t < b6 ? b0 * (t -= b5) * t + b7 : b0 * (t -= b8) * t + b9;
/// }
/// ```
///
/// The JavaScript takes one off the number inside each of the branches, and
/// the port's does the same ✓.
pub fn ease_bounce_out(t: f64) -> f64 {
    let mut t = js::number(t);
    if t < B1 {
        B0 * t * t
    } else if t < B3 {
        t -= B2;
        B0 * t * t + B4
    } else if t < B6 {
        t -= B5;
        B0 * t * t + B7
    } else {
        t -= B8;
        B0 * t * t + B9
    }
}

/// d3-ease's `easeBounceInOut`, which is the JavaScript's `bounceInOut`.
///
/// The JavaScript:
///
/// ```js
/// export function bounceInOut(t) {
///   return ((t *= 2) <= 1 ? 1 - bounceOut(1 - t) : bounceOut(t - 1) + 1) / 2;
/// }
/// ```
pub fn ease_bounce_in_out(t: f64) -> f64 {
    let t = t * 2.0;
    let value = if t <= 1.0 {
        1.0 - ease_bounce_out(1.0 - t)
    } else {
        ease_bounce_out(t - 1.0) + 1.0
    };
    value / 2.0
}

/// d3-ease's `easeBack`, which is the JavaScript's `backInOut` with the
/// JavaScript's own overshoot.
///
/// The JavaScript:
///
/// ```js
/// export var backInOut = (function custom(s) {
///   s = +s;
///
///   function backInOut(t) {
///     return ((t *= 2) < 1 ? t * t * ((s + 1) * t - s) : (t -= 2) * t * ((s + 1) * t + s) + 2) / 2;
///   }
///
///   backInOut.overshoot = custom;
///
///   return backInOut;
/// })(overshoot);
/// ```
///
/// The overshoot is held by [`Back`], and this is that struct with the
/// JavaScript's own overshoot ✓.
pub fn ease_back(t: f64) -> f64 {
    Back::default().back_in_out(t)
}

/// d3-ease's `easeBackIn`, which is the JavaScript's `backIn` with the
/// JavaScript's own overshoot.
///
/// See [`Back`], which holds the overshoot.
pub fn ease_back_in(t: f64) -> f64 {
    Back::default().back_in(t)
}

/// d3-ease's `easeBackOut`, which is the JavaScript's `backOut` with the
/// JavaScript's own overshoot.
///
/// See [`Back`], which holds the overshoot.
pub fn ease_back_out(t: f64) -> f64 {
    Back::default().back_out(t)
}

/// d3-ease's `easeBackInOut`, which is the JavaScript's `backInOut` with the
/// JavaScript's own overshoot.
///
/// See [`Back`], which holds the overshoot.
pub fn ease_back_in_out(t: f64) -> f64 {
    Back::default().back_in_out(t)
}

/// d3-ease's `easeElastic`, which is the JavaScript's `elasticOut` with the
/// JavaScript's own amplitude and period.
///
/// The JavaScript:
///
/// ```js
/// export var elasticOut = (function custom(a, p) {
///   var s = Math.asin(1 / (a = Math.max(1, a))) * (p /= tau);
///
///   function elasticOut(t) {
///     return 1 - a * tpmt(t = +t) * Math.sin((t + s) / p);
///   }
///
///   elasticOut.amplitude = function(a) { return custom(a, p * tau); };
///   elasticOut.period = function(p) { return custom(a, p); };
///
///   return elasticOut;
/// })(amplitude, period);
/// ```
///
/// The JavaScript's export is the *out* of the family, and not the in and
/// out as the other families' are. The amplitude and the period are held
/// by [`Elastic`] ✓.
pub fn ease_elastic(t: f64) -> f64 {
    Elastic::default().elastic_out(t)
}

/// d3-ease's `easeElasticIn`, which is the JavaScript's `elasticIn` with
/// the JavaScript's own amplitude and period.
///
/// See [`Elastic`], which holds the amplitude and the period.
pub fn ease_elastic_in(t: f64) -> f64 {
    Elastic::default().elastic_in(t)
}

/// d3-ease's `easeElasticOut`, which is the JavaScript's `elasticOut` with
/// the JavaScript's own amplitude and period.
///
/// See [`Elastic`], which holds the amplitude and the period.
pub fn ease_elastic_out(t: f64) -> f64 {
    Elastic::default().elastic_out(t)
}

/// d3-ease's `easeElasticInOut`, which is the JavaScript's `elasticInOut`
/// with the JavaScript's own amplitude and period.
///
/// See [`Elastic`], which holds the amplitude and the period.
pub fn ease_elastic_in_out(t: f64) -> f64 {
    Elastic::default().elastic_in_out(t)
}

/// The JavaScript's `tpmt`, which is two power minus ten times t scaled to
/// `[0, 1]`.
///
/// The JavaScript:
///
/// ```js
/// export function tpmt(x) {
///   return (Math.pow(2, -10 * x) - 0.0009765625) * 1.0009775171065494;
/// }
/// ```
fn tpmt(x: f64) -> f64 {
    // The JavaScript's own arithmetic, which gives the second of two
    // numbers that are not numbers ✓, and the machine's does not ✓.
    arith::multiply(
        arith::subtract(ieee754::pow(2.0, -10.0 * x), 0.0009765625),
        1.0009775171065494,
    )
}

/// d3-ease's `easePoly`, `easePolyIn`, `easePolyOut` and `easePolyInOut`,
/// which have an exponent.
///
/// The JavaScript hangs the exponent on each of the three functions, as
/// `polyIn.exponent`, and the setter gives back a new function:
///
/// ```js
/// export var polyIn = (function custom(e) {
///   e = +e;
///
///   function polyIn(t) {
///     return Math.pow(t, e);
///   }
///
///   polyIn.exponent = custom;
///
///   return polyIn;
/// })(exponent);
/// ```
///
/// The port holds the exponent here instead, and the setter is
/// [`Poly::exponent`], which gives back another of these ✓.
#[derive(Clone, Copy, Debug)]
pub struct Poly {
    exponent: f64,
}

impl Poly {
    /// The JavaScript's `polyIn` and the rest, with an exponent.
    ///
    /// The JavaScript's own exponent is three, which is what
    /// [`Poly::default`] gives.
    pub fn new(exponent: f64) -> Self {
        Self {
            exponent: js::number(exponent),
        }
    }

    /// The JavaScript's `polyIn.exponent`, which gives back another set of
    /// eases with a different exponent.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// polyIn.exponent = custom;
    /// ```
    ///
    /// The JavaScript's own ease is not changed by it, and the port's is
    /// not either ✓.
    pub fn exponent(&self, exponent: f64) -> Self {
        Self::new(exponent)
    }

    /// The JavaScript's `polyIn`.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// function polyIn(t) {
    ///   return Math.pow(t, e);
    /// }
    /// ```
    pub fn poly_in(&self, t: f64) -> f64 {
        ieee754::pow(t, self.exponent)
    }

    /// The JavaScript's `polyOut`.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// function polyOut(t) {
    ///   return 1 - Math.pow(1 - t, e);
    /// }
    /// ```
    pub fn poly_out(&self, t: f64) -> f64 {
        1.0 - ieee754::pow(1.0 - t, self.exponent)
    }

    /// The JavaScript's `polyInOut`.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// function polyInOut(t) {
    ///   return ((t *= 2) <= 1 ? Math.pow(t, e) : 2 - Math.pow(2 - t, e)) / 2;
    /// }
    /// ```
    pub fn poly_in_out(&self, t: f64) -> f64 {
        let t = t * 2.0;
        let value = if t <= 1.0 {
            ieee754::pow(t, self.exponent)
        } else {
            2.0 - ieee754::pow(2.0 - t, self.exponent)
        };
        value / 2.0
    }
}

impl Default for Poly {
    /// The JavaScript's own exponent, which is three.
    fn default() -> Self {
        Self::new(EXPONENT)
    }
}

/// d3-ease's `easeBack`, `easeBackIn`, `easeBackOut` and `easeBackInOut`,
/// which have an overshoot.
///
/// The JavaScript hangs the overshoot on each of the three functions, as
/// `backIn.overshoot`, and the setter gives back a new function:
///
/// ```js
/// export var backIn = (function custom(s) {
///   s = +s;
///
///   function backIn(t) {
///     return (t = +t) * t * (s * (t - 1) + t);
///   }
///
///   backIn.overshoot = custom;
///
///   return backIn;
/// })(overshoot);
/// ```
///
/// The port holds the overshoot here instead, and the setter is
/// [`Back::overshoot`], which gives back another of these ✓.
#[derive(Clone, Copy, Debug)]
pub struct Back {
    overshoot: f64,
}

impl Back {
    /// The JavaScript's `backIn` and the rest, with an overshoot.
    ///
    /// The JavaScript's own overshoot is 1.70158, which is what
    /// [`Back::default`] gives.
    pub fn new(overshoot: f64) -> Self {
        Self {
            overshoot: js::number(overshoot),
        }
    }

    /// The JavaScript's `backIn.overshoot`, which gives back another set of
    /// eases with a different overshoot.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// backIn.overshoot = custom;
    /// ```
    ///
    /// The JavaScript's own ease is not changed by it, and the port's is
    /// not either ✓.
    pub fn overshoot(&self, overshoot: f64) -> Self {
        Self::new(overshoot)
    }

    /// The JavaScript's `backIn`.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// function backIn(t) {
    ///   return (t = +t) * t * (s * (t - 1) + t);
    /// }
    /// ```
    pub fn back_in(&self, t: f64) -> f64 {
        let t = js::number(t);
        t * t * (self.overshoot * (t - 1.0) + t)
    }

    /// The JavaScript's `backOut`.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// function backOut(t) {
    ///   return --t * t * ((t + 1) * s + t) + 1;
    /// }
    /// ```
    pub fn back_out(&self, t: f64) -> f64 {
        let t = t - 1.0;
        t * t * ((t + 1.0) * self.overshoot + t) + 1.0
    }

    /// The JavaScript's `backInOut`.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// function backInOut(t) {
    ///   return ((t *= 2) < 1 ? t * t * ((s + 1) * t - s) : (t -= 2) * t * ((s + 1) * t + s) + 2) / 2;
    /// }
    /// ```
    pub fn back_in_out(&self, t: f64) -> f64 {
        let mut t = t * 2.0;
        let value = if t < 1.0 {
            t * t * ((self.overshoot + 1.0) * t - self.overshoot)
        } else {
            t -= 2.0;
            t * t * ((self.overshoot + 1.0) * t + self.overshoot) + 2.0
        };
        value / 2.0
    }
}

impl Default for Back {
    /// The JavaScript's own overshoot, which is 1.70158.
    fn default() -> Self {
        Self::new(OVERSHOOT)
    }
}

/// d3-ease's `easeElastic`, `easeElasticIn`, `easeElasticOut` and
/// `easeElasticInOut`, which have an amplitude and a period.
///
/// The JavaScript hangs the two settings on each of the three functions, as
/// `elasticIn.amplitude` and `elasticIn.period`, and each of the setters
/// gives back a new function:
///
/// ```js
/// export var elasticIn = (function custom(a, p) {
///   var s = Math.asin(1 / (a = Math.max(1, a))) * (p /= tau);
///
///   function elasticIn(t) {
///     return a * tpmt(-(--t)) * Math.sin((s - t) / p);
///   }
///
///   elasticIn.amplitude = function(a) { return custom(a, p * tau); };
///   elasticIn.period = function(p) { return custom(a, p); };
///
///   return elasticIn;
/// })(amplitude, period);
/// ```
///
/// The port holds the two settings here instead, and the setters are
/// [`Elastic::amplitude`] and [`Elastic::period`], which give back another
/// of these ✓.
#[derive(Clone, Copy, Debug)]
pub struct Elastic {
    amplitude: f64,
    period: f64,
    phase: f64,
}

impl Elastic {
    /// The JavaScript's `custom`, which is the constructor of all three of
    /// the elastic eases.
    ///
    /// The JavaScript's amplitude is at least one, and its period is
    /// measured in halves of a turn, because the JavaScript divides it by
    /// twice pi as it takes it.
    fn custom(amplitude: f64, period: f64) -> Self {
        let amplitude = at_least_one(amplitude);
        let period = period / TAU;
        let phase = ieee754::asin(1.0 / amplitude) * period;
        Self {
            amplitude,
            period,
            phase,
        }
    }

    /// The JavaScript's `elasticIn` and the rest, with an amplitude and a
    /// period.
    ///
    /// The JavaScript's own amplitude is one and its own period is 0.3,
    /// which is what [`Elastic::default`] gives.
    pub fn new(amplitude: f64, period: f64) -> Self {
        Self::custom(amplitude, period)
    }

    /// The JavaScript's `elasticIn.amplitude`, which gives back another of
    /// the eases with a different amplitude.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// elasticIn.amplitude = function(a) { return custom(a, p * tau); };
    /// ```
    ///
    /// The JavaScript's period is measured in halves of a turn by the time
    /// the setter is written, so it multiplies it by twice pi to give it
    /// back, and the port's does the same ✓.
    pub fn amplitude(&self, amplitude: f64) -> Self {
        Self::custom(amplitude, self.period * TAU)
    }

    /// The JavaScript's `elasticIn.period`, which gives back another of the
    /// eases with a different period.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// elasticIn.period = function(p) { return custom(a, p); };
    /// ```
    pub fn period(&self, period: f64) -> Self {
        Self::custom(self.amplitude, period)
    }

    /// The JavaScript's `elasticIn`.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// function elasticIn(t) {
    ///   return a * tpmt(-(--t)) * Math.sin((s - t) / p);
    /// }
    /// ```
    pub fn elastic_in(&self, mut t: f64) -> f64 {
        t -= 1.0;
        arith::multiply(
            arith::multiply(self.amplitude, tpmt(-t)),
            ieee754::sin((self.phase - t) / self.period),
        )
    }

    /// The JavaScript's `elasticOut`.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// function elasticOut(t) {
    ///   return 1 - a * tpmt(t = +t) * Math.sin((t + s) / p);
    /// }
    /// ```
    pub fn elastic_out(&self, t: f64) -> f64 {
        let t = js::number(t);
        arith::subtract(
            1.0,
            arith::multiply(
                arith::multiply(self.amplitude, tpmt(t)),
                ieee754::sin((t + self.phase) / self.period),
            ),
        )
    }

    /// The JavaScript's `elasticInOut`.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// function elasticInOut(t) {
    ///   return ((t = t * 2 - 1) < 0
    ///       ? a * tpmt(-t) * Math.sin((s - t) / p)
    ///       : 2 - a * tpmt(t) * Math.sin((s + t) / p)) / 2;
    /// }
    /// ```
    pub fn elastic_in_out(&self, t: f64) -> f64 {
        let t = t * 2.0 - 1.0;
        let value = if t < 0.0 {
            arith::multiply(
                arith::multiply(self.amplitude, tpmt(-t)),
                ieee754::sin((self.phase - t) / self.period),
            )
        } else {
            arith::subtract(
                2.0,
                arith::multiply(
                    arith::multiply(self.amplitude, tpmt(t)),
                    ieee754::sin((self.phase + t) / self.period),
                ),
            )
        };
        value / 2.0
    }
}

impl Default for Elastic {
    /// The JavaScript's own amplitude, which is one, and its own period,
    /// which is 0.3.
    fn default() -> Self {
        Self::new(AMPLITUDE, PERIOD)
    }
}

/// JavaScript's `Math.max(1, a)`, which is a `NaN` when `a` is one, where
/// Rust's own `max` is the other one.
fn at_least_one(a: f64) -> f64 {
    if a.is_nan() {
        return f64::NAN;
    }
    if a > 1.0 {
        a
    } else {
        1.0
    }
}
