//! d3-shape's `arc`, from the JavaScript of the same name.
//!
//! Translated from `d3-shape/src/arc.js`, version 1.3.7, of d3 5.16.0.
//! The JavaScript is the authority.

use crate::ieee754;

use super::Context;
use crate::path::Path;

/// The epsilon the JavaScript compares against.
const EPSILON: f64 = 1e-12;
/// Twice pi.
const TAU: f64 = std::f64::consts::PI * 2.0;
/// Half pi.
const HALF_PI: f64 = std::f64::consts::PI / 2.0;

/// A function that reads a number out of a datum.
pub type Accessor<T> = Box<dyn Fn(&T, usize, &[T]) -> f64>;

/// A function that reads a number out of a datum, without its place.
pub type Plain<T> = Box<dyn Fn(&T) -> f64>;

/// The JavaScript's `arc`, which draws a circular sector, a ring, or a
/// part of one.
///
/// The JavaScript:
///
/// ```js
/// function arc() {
///   var buffer, r,
///       r0 = +innerRadius.apply(this, arguments),
///       r1 = +outerRadius.apply(this, arguments),
///       a0 = startAngle.apply(this, arguments) - halfPi,
///       a1 = endAngle.apply(this, arguments) - halfPi,
///       da = abs(a1 - a0),
///       cw = a1 > a0;
///   ...
/// }
/// ```
///
/// The JavaScript's `arc.centroid` gives the middle of the arc.
pub struct Arc<T> {
    inner_radius: Accessor<T>,
    outer_radius: Accessor<T>,
    corner_radius: Plain<T>,
    pad_radius: Option<Plain<T>>,
    start_angle: Accessor<T>,
    end_angle: Accessor<T>,
    pad_angle: Plain<T>,
}

/// What an arc's datum gives: the JavaScript's `arcInnerRadius`,
/// `arcOuterRadius`, `arcStartAngle`, `arcEndAngle` and `arcPadAngle` read
/// these off the datum, and the port's default does the same.
pub trait ArcDatum {
    /// The JavaScript's `d.innerRadius`.
    fn inner_radius(&self) -> f64;
    /// The JavaScript's `d.outerRadius`.
    fn outer_radius(&self) -> f64;
    /// The JavaScript's `d.startAngle`.
    fn start_angle(&self) -> f64;
    /// The JavaScript's `d.endAngle`.
    fn end_angle(&self) -> f64;
    /// The JavaScript's `d.padAngle`, which is optional.
    fn pad_angle(&self) -> Option<f64> {
        None
    }
}

impl<T: ArcDatum> Default for Arc<T> {
    fn default() -> Self {
        Self {
            inner_radius: Box::new(|d: &T, _, _| d.inner_radius()),
            outer_radius: Box::new(|d: &T, _, _| d.outer_radius()),
            corner_radius: Box::new(|_| 0.0),
            pad_radius: None,
            start_angle: Box::new(|d: &T, _, _| d.start_angle()),
            end_angle: Box::new(|d: &T, _, _| d.end_angle()),
            pad_angle: Box::new(|d: &T| d.pad_angle().unwrap_or(0.0)),
        }
    }
}

impl<T> Arc<T> {
    /// The JavaScript's `arc.innerRadius`.
    pub fn inner_radius<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.inner_radius = Box::new(f);
        self
    }

    /// The JavaScript's `arc.outerRadius`.
    pub fn outer_radius<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.outer_radius = Box::new(f);
        self
    }

    /// The JavaScript's `arc.cornerRadius`.
    pub fn corner_radius<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> f64 + 'static,
    {
        self.corner_radius = Box::new(f);
        self
    }

    /// The JavaScript's `arc.padRadius`.
    pub fn pad_radius<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> f64 + 'static,
    {
        self.pad_radius = Some(Box::new(f));
        self
    }

    /// The JavaScript's `arc.startAngle`.
    pub fn start_angle<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.start_angle = Box::new(f);
        self
    }

    /// The JavaScript's `arc.endAngle`.
    pub fn end_angle<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, usize, &[T]) -> f64 + 'static,
    {
        self.end_angle = Box::new(f);
        self
    }

    /// The JavaScript's `arc.padAngle`.
    pub fn pad_angle<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> f64 + 'static,
    {
        self.pad_angle = Box::new(f);
        self
    }

    /// The JavaScript's `arc.centroid`: the middle of the arc.
    pub fn centroid(&self, datum: &T, index: usize, data: &[T]) -> (f64, f64) {
        let r = ((self.inner_radius)(datum, index, data) + (self.outer_radius)(datum, index, data))
            / 2.0;
        let a = ((self.start_angle)(datum, index, data) + (self.end_angle)(datum, index, data))
            / 2.0
            - std::f64::consts::PI / 2.0;
        (ieee754::cos(a) * r, ieee754::sin(a) * r)
    }

    /// The JavaScript's `arc(datum)`: the path of the arc.
    pub fn apply(&self, datum: &T, index: usize, data: &[T]) -> Option<String> {
        let mut buffer = Path::default();
        self.apply_to(datum, index, data, &mut buffer);
        let text = buffer.to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// The JavaScript's `arc(datum)` drawn into a context the caller
    /// holds.
    pub fn apply_to(&self, datum: &T, index: usize, data: &[T], context: &mut dyn Context) {
        let r0 = (self.inner_radius)(datum, index, data);
        let r1 = (self.outer_radius)(datum, index, data);
        let a0 = (self.start_angle)(datum, index, data) - HALF_PI;
        let a1 = (self.end_angle)(datum, index, data) - HALF_PI;
        let da = (a1 - a0).abs();
        let cw = a1 > a0;

        // Ensure that the outer radius is always larger than the inner radius.
        let (r0, r1) = if r1 < r0 { (r1, r0) } else { (r0, r1) };

        // Is it a point?
        if !(r1 > EPSILON) {
            context.move_to(0.0, 0.0);
        }
        // Or is it a circle or annulus?
        else if da > TAU - EPSILON {
            context.move_to(r1 * ieee754::cos(a0), r1 * ieee754::sin(a0));
            let _ = context.arc(0.0, 0.0, r1, a0, a1, !cw);
            if r0 > EPSILON {
                context.move_to(r0 * ieee754::cos(a1), r0 * ieee754::sin(a1));
                let _ = context.arc(0.0, 0.0, r0, a1, a0, cw);
            }
        }
        // Or is it a circular or annular sector?
        else {
            let mut a01 = a0;
            let mut a11 = a1;
            let mut a00 = a0;
            let mut a10 = a1;
            let mut da0 = da;
            let mut da1 = da;
            let ap = (self.pad_angle)(datum) / 2.0;
            let rp = if ap > EPSILON {
                match &self.pad_radius {
                    Some(f) => (f)(datum),
                    None => (r0 * r0 + r1 * r1).sqrt(),
                }
            } else {
                0.0
            };
            let rc = ((r1 - r0).abs() / 2.0).min((self.corner_radius)(datum));
            let mut rc0 = rc;
            let mut rc1 = rc;

            // Apply padding? Note that since r1 >= r0, da1 >= da0.
            if rp > EPSILON {
                let mut p0 = ieee754::asin(rp / r0 * ieee754::sin(ap));
                let mut p1 = ieee754::asin(rp / r1 * ieee754::sin(ap));
                da0 -= p0 * 2.0;
                if da0 > EPSILON {
                    if !cw {
                        p0 = -p0;
                    }
                    a00 += p0;
                    a10 -= p0;
                } else {
                    da0 = 0.0;
                    a00 = (a0 + a1) / 2.0;
                    a10 = a00;
                }
                da1 -= p1 * 2.0;
                if da1 > EPSILON {
                    if !cw {
                        p1 = -p1;
                    }
                    a01 += p1;
                    a11 -= p1;
                } else {
                    da1 = 0.0;
                    a01 = (a0 + a1) / 2.0;
                    a11 = a01;
                }
            }

            let x01 = r1 * ieee754::cos(a01);
            let y01 = r1 * ieee754::sin(a01);
            let x10 = r0 * ieee754::cos(a10);
            let y10 = r0 * ieee754::sin(a10);

            // Apply rounded corners?
            // The JavaScript declares these with `var` inside the block,
            // so they are in scope for the rest of the function, and the
            // port lifts them out.
            let x11 = r1 * ieee754::cos(a11);
            let y11 = r1 * ieee754::sin(a11);
            let x00 = r0 * ieee754::cos(a00);
            let y00 = r0 * ieee754::sin(a00);
            if rc > EPSILON {
                // Restrict the corner radius according to the sector angle.
                if da < std::f64::consts::PI {
                    if let Some(oc) = intersect(x01, y01, x00, y00, x11, y11, x10, y10) {
                        let ax = x01 - oc.0;
                        let ay = y01 - oc.1;
                        let bx = x11 - oc.0;
                        let by = y11 - oc.1;
                        let kc = 1.0
                            / ieee754::sin(
                                ieee754::acos(
                                    (ax * bx + ay * by)
                                        / ((ax * ax + ay * ay).sqrt() * (bx * bx + by * by).sqrt()),
                                ) / 2.0,
                            );
                        let lc = (oc.0 * oc.0 + oc.1 * oc.1).sqrt();
                        rc0 = rc.min((r0 - lc) / (kc - 1.0));
                        rc1 = rc.min((r1 - lc) / (kc + 1.0));
                    }
                }
            }

            // Is the sector collapsed to a line?
            if !(da1 > EPSILON) {
                context.move_to(x01, y01);
            }
            // Does the sector's outer ring have rounded corners?
            else if rc1 > EPSILON {
                let t0 = corner_tangents(
                    ieee754::cos(a00) * r0,
                    ieee754::sin(a00) * r0,
                    x01,
                    y01,
                    r1,
                    rc1,
                    cw,
                );
                let t1 = corner_tangents(x11, y11, x10, y10, r1, rc1, cw);

                context.move_to(t0.cx + t0.x01, t0.cy + t0.y01);

                // Have the corners merged?
                if rc1 < rc {
                    let _ = context.arc(
                        t0.cx,
                        t0.cy,
                        rc1,
                        ieee754::atan2(t0.y01, t0.x01),
                        ieee754::atan2(t1.y01, t1.x01),
                        !cw,
                    );
                }
                // Otherwise, draw the two corners and the ring.
                else {
                    let _ = context.arc(
                        t0.cx,
                        t0.cy,
                        rc1,
                        ieee754::atan2(t0.y01, t0.x01),
                        ieee754::atan2(t0.y11, t0.x11),
                        !cw,
                    );
                    let _ = context.arc(
                        0.0,
                        0.0,
                        r1,
                        ieee754::atan2(t0.cy + t0.y11, t0.cx + t0.x11),
                        ieee754::atan2(t1.cy + t1.y11, t1.cx + t1.x11),
                        !cw,
                    );
                    let _ = context.arc(
                        t1.cx,
                        t1.cy,
                        rc1,
                        ieee754::atan2(t1.y11, t1.x11),
                        ieee754::atan2(t1.y01, t1.x01),
                        !cw,
                    );
                }
            }
            // Or is the outer ring just a circular arc?
            else {
                context.move_to(x01, y01);
                let _ = context.arc(0.0, 0.0, r1, a01, a11, !cw);
            }

            // Is there no inner ring, and it's a circular sector?
            // Or perhaps it's an annular sector collapsed due to padding?
            if !(r0 > EPSILON) || !(da0 > EPSILON) {
                context.line_to(x10, y10);
            }
            // Does the sector's inner ring (or point) have rounded corners?
            else if rc0 > EPSILON {
                let t0 = corner_tangents(x10, y10, x11, y11, r0, -rc0, cw);
                let t1 = corner_tangents(x01, y01, x00, y00, r0, -rc0, cw);

                context.line_to(t0.cx + t0.x01, t0.cy + t0.y01);

                // Have the corners merged?
                if rc0 < rc {
                    let _ = context.arc(
                        t0.cx,
                        t0.cy,
                        rc0,
                        ieee754::atan2(t0.y01, t0.x01),
                        ieee754::atan2(t1.y01, t1.x01),
                        !cw,
                    );
                }
                // Otherwise, draw the two corners and the ring.
                else {
                    let _ = context.arc(
                        t0.cx,
                        t0.cy,
                        rc0,
                        ieee754::atan2(t0.y01, t0.x01),
                        ieee754::atan2(t0.y11, t0.x11),
                        !cw,
                    );
                    let _ = context.arc(
                        0.0,
                        0.0,
                        r0,
                        ieee754::atan2(t0.cy + t0.y11, t0.cx + t0.x11),
                        ieee754::atan2(t1.cy + t1.y11, t1.cx + t1.x11),
                        cw,
                    );
                    let _ = context.arc(
                        t1.cx,
                        t1.cy,
                        rc0,
                        ieee754::atan2(t1.y11, t1.x11),
                        ieee754::atan2(t1.y01, t1.x01),
                        !cw,
                    );
                }
            }
            // Or is the inner ring just a circular arc?
            else {
                let _ = context.arc(0.0, 0.0, r0, a10, a00, cw);
            }
        }
        context.close_path();
    }
}

/// The JavaScript's `intersect`, which finds where two lines cross.
fn intersect(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
) -> Option<(f64, f64)> {
    let x10 = x1 - x0;
    let y10 = y1 - y0;
    let x32 = x3 - x2;
    let y32 = y3 - y2;
    let mut t = y32 * x10 - x32 * y10;
    if t * t < EPSILON {
        return None;
    }
    t = (x32 * (y0 - y2) - y32 * (x0 - x2)) / t;
    Some((x0 + t * x10, y0 + t * y10))
}

/// The JavaScript's `cornerTangents`, which finds where a rounded corner
/// begins and ends.
struct CornerTangents {
    cx: f64,
    cy: f64,
    x01: f64,
    y01: f64,
    x11: f64,
    y11: f64,
}

/// The JavaScript's `cornerTangents`.
fn corner_tangents(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    r1: f64,
    rc: f64,
    cw: bool,
) -> CornerTangents {
    let x01 = x0 - x1;
    let y01 = y0 - y1;
    let lo = (if cw { rc } else { -rc }) / (x01 * x01 + y01 * y01).sqrt();
    let ox = lo * y01;
    let oy = -lo * x01;
    let x11 = x0 + ox;
    let y11 = y0 + oy;
    let x10 = x1 + ox;
    let y10 = y1 + oy;
    let x00 = (x11 + x10) / 2.0;
    let y00 = (y11 + y10) / 2.0;
    let dx = x10 - x11;
    let dy = y10 - y11;
    let d2 = dx * dx + dy * dy;
    let r = r1 - rc;
    let big_d = x11 * y10 - x10 * y11;
    let d = (if dy < 0.0 { -1.0 } else { 1.0 }) * (r * r * d2 - big_d * big_d).max(0.0).sqrt();
    let mut cx0 = (big_d * dy - dx * d) / d2;
    let mut cy0 = (-big_d * dx - dy * d) / d2;
    let cx1 = (big_d * dy + dx * d) / d2;
    let cy1 = (-big_d * dx + dy * d) / d2;
    let dx0 = cx0 - x00;
    let dy0 = cy0 - y00;
    let dx1 = cx1 - x00;
    let dy1 = cy1 - y00;

    // Pick the closer of the two intersection points.
    if dx0 * dx0 + dy0 * dy0 > dx1 * dx1 + dy1 * dy1 {
        cx0 = cx1;
        cy0 = cy1;
    }

    CornerTangents {
        cx: cx0,
        cy: cy0,
        x01: -ox,
        y01: -oy,
        x11: cx0 * (r1 / r - 1.0),
        y11: cy0 * (r1 / r - 1.0),
    }
}
