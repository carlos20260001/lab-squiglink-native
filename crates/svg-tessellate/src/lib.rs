//! Tessellate SVG path data into egui shapes (polylines), without pixmaps.
//!
//! Parses an SVG path data string with `svgtypes` and samples curves into
//! polyline points, returning `egui::Shape`s that egui tessellates as vectors.

use egui::{Color32, Pos2, Shape, Stroke};
use svgtypes::PathParser;

/// Options for tessellation.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Number of segments per cubic bezier curve.
    pub curve_segments: usize,
    /// Number of segments per elliptical arc.
    pub arc_segments: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self { curve_segments: 12, arc_segments: 24 }
    }
}

/// Tessellate an SVG path data string into egui shapes.
pub fn tessellate(path_data: &str, stroke: Option<Stroke>, fill: Option<Color32>, options: Options) -> Vec<Shape> {
    let mut subpaths: Vec<Vec<Pos2>> = Vec::new();
    let mut current: Vec<Pos2> = Vec::new();
    let _start = Pos2::ZERO;
    let mut cx = 0.0_f64;
    let mut cy = 0.0_f64;
    let mut sx = 0.0_f64;
    let mut sy = 0.0_f64;

    for seg in PathParser::from(path_data) {
        let seg = match seg { Ok(s) => s, Err(_) => continue };
        match seg {
            svgtypes::PathSegment::MoveTo { abs, x, y } => {
                if !current.is_empty() { subpaths.push(std::mem::take(&mut current)); }
                if abs { cx = x; cy = y; } else { cx += x; cy += y; }
                sx = cx; sy = cy;
                current.push(Pos2::new(cx as f32, cy as f32));
            }
            svgtypes::PathSegment::LineTo { abs, x, y } => {
                if abs { cx = x; cy = y; } else { cx += x; cy += y; }
                current.push(Pos2::new(cx as f32, cy as f32));
            }
            svgtypes::PathSegment::HorizontalLineTo { abs, x } => {
                if abs { cx = x; } else { cx += x; }
                current.push(Pos2::new(cx as f32, cy as f32));
            }
            svgtypes::PathSegment::VerticalLineTo { abs, y } => {
                if abs { cy = y; } else { cy += y; }
                current.push(Pos2::new(cx as f32, cy as f32));
            }
            svgtypes::PathSegment::CurveTo { abs, x1, y1, x2, y2, x, y } => {
                let (x1, y1, x2, y2, x, y) = if abs {
                    (x1, y1, x2, y2, x, y)
                } else {
                    (cx + x1, cy + y1, cx + x2, cy + y2, cx + x, cy + y)
                };
                let p0 = (cx, cy);
                let p1 = (x1, y1);
                let p2 = (x2, y2);
                let p3 = (x, y);
                for i in 1..=options.curve_segments {
                    let t = i as f64 / options.curve_segments as f64;
                    let pt = cubic(p0, p1, p2, p3, t);
                    current.push(Pos2::new(pt.0 as f32, pt.1 as f32));
                }
                cx = x; cy = y;
            }
            svgtypes::PathSegment::SmoothCurveTo { abs, x2, y2, x, y } => {
                // approximate: treat as cubic with reflected control point
                let (x2, y2, x, y) = if abs { (x2, y2, x, y) } else { (cx + x2, cy + y2, cx + x, cy + y) };
                // reflect last control point (unknown; use current point)
                let x1 = cx; let y1 = cy;
                let p0 = (cx, cy);
                let p1 = (x1, y1);
                let p2 = (x2, y2);
                let p3 = (x, y);
                for i in 1..=options.curve_segments {
                    let t = i as f64 / options.curve_segments as f64;
                    let pt = cubic(p0, p1, p2, p3, t);
                    current.push(Pos2::new(pt.0 as f32, pt.1 as f32));
                }
                cx = x; cy = y;
            }
            svgtypes::PathSegment::Quadratic { abs, x1, y1, x, y } => {
                let (x1, y1, x, y) = if abs { (x1, y1, x, y) } else { (cx + x1, cy + y1, cx + x, cy + y) };
                let p0 = (cx, cy);
                let p1 = (x1, y1);
                let p2 = (x, y);
                for i in 1..=options.curve_segments {
                    let t = i as f64 / options.curve_segments as f64;
                    let pt = quadratic(p0, p1, p2, t);
                    current.push(Pos2::new(pt.0 as f32, pt.1 as f32));
                }
                cx = x; cy = y;
            }
            svgtypes::PathSegment::SmoothQuadratic { abs, x, y } => {
                let (x, y) = if abs { (x, y) } else { (cx + x, cy + y) };
                // reflect control point
                let x1 = cx; let y1 = cy;
                let p0 = (cx, cy);
                let p1 = (x1, y1);
                let p2 = (x, y);
                for i in 1..=options.curve_segments {
                    let t = i as f64 / options.curve_segments as f64;
                    let pt = quadratic(p0, p1, p2, t);
                    current.push(Pos2::new(pt.0 as f32, pt.1 as f32));
                }
                cx = x; cy = y;
            }
            svgtypes::PathSegment::EllipticalArc { abs, rx, ry, x_axis_rotation, large_arc, sweep, x, y } => {
                let (x2, y2) = if abs { (x, y) } else { (cx + x, cy + y) };
                // Proper SVG elliptical arc sampling (W3C algorithm).
                let mut rx = rx.abs();
                let mut ry = ry.abs();
                if rx == 0.0 || ry == 0.0 {
                    // Degenerate: treat as line.
                    current.push(Pos2::new(x2 as f32, y2 as f32));
                    cx = x2; cy = y2;
                    continue;
                }
                let phi = x_axis_rotation.to_radians();
                let (sin_phi, cos_phi) = (phi.sin(), phi.cos());
                let dx2 = (cx - x2) / 2.0;
                let dy2 = (cy - y2) / 2.0;
                let x1p = cos_phi * dx2 + sin_phi * dy2;
                let y1p = -sin_phi * dx2 + cos_phi * dy2;
                let x1p_sq = x1p * x1p;
                let y1p_sq = y1p * y1p;
                let rx_sq = rx * rx;
                let ry_sq = ry * ry;
                let lambda = x1p_sq / rx_sq + y1p_sq / ry_sq;
                if lambda > 1.0 {
                    let s = lambda.sqrt();
                    rx *= s;
                    ry *= s;
                }
                let rx_sq = rx * rx;
                let ry_sq = ry * ry;
                let factor = ((rx_sq * ry_sq - rx_sq * y1p_sq - ry_sq * x1p_sq)
                    / (rx_sq * y1p_sq + ry_sq * x1p_sq))
                    .max(0.0)
                    .sqrt();
                let sign = if large_arc == sweep { -1.0 } else { 1.0 };
                let cxp = sign * factor * (rx * y1p / ry);
                let cyp = sign * factor * (-(ry * x1p) / rx);
                let ccx = cos_phi * cxp - sin_phi * cyp + (cx + x2) / 2.0;
                let ccy = sin_phi * cxp + cos_phi * cyp + (cy + y2) / 2.0;
                // Start and end angles.
                let ux = (x1p - cxp) / rx;
                let uy = (y1p - cyp) / ry;
                let vx = (-x1p - cxp) / rx;
                let vy = (-y1p - cyp) / ry;
                let theta1 = uy.atan2(ux);
                let mut dtheta = (vy.atan2(vx) - theta1).rem_euclid(std::f64::consts::TAU);
                if !sweep && dtheta > 0.0 {
                    dtheta -= std::f64::consts::TAU;
                }
                if sweep && dtheta < 0.0 {
                    dtheta += std::f64::consts::TAU;
                }
                let steps = options.arc_segments;
                for i in 1..=steps {
                    let t = theta1 + dtheta * (i as f64 / steps as f64);
                    let px = ccx + rx * t.cos() * cos_phi - ry * t.sin() * sin_phi;
                    let py = ccy + rx * t.cos() * sin_phi + ry * t.sin() * cos_phi;
                    current.push(Pos2::new(px as f32, py as f32));
                }
                cx = x2; cy = y2;
            }
            svgtypes::PathSegment::ClosePath { .. } => {
                if !current.is_empty() {
                    // close by pushing start
                    if let Some(first) = current.first().copied() {
                        current.push(first);
                    }
                    subpaths.push(std::mem::take(&mut current));
                }
                cx = sx; cy = sy;
                current.push(Pos2::new(cx as f32, cy as f32));
            }
        }
    }
    if !current.is_empty() { subpaths.push(current); }

    let mut shapes = Vec::new();
    for sub in subpaths {
        if sub.len() < 2 { continue; }
        if let Some(stroke) = stroke {
            shapes.push(Shape::line(sub.clone(), stroke));
        }
        if let Some(fill) = fill {
            // egui PathShape fill requires closed; only fill if first==last
            if sub.first() == sub.last() {
                shapes.push(Shape::Path(egui::epaint::PathShape {
                    points: sub,
                    closed: true,
                    fill,
                    stroke: egui::epaint::PathStroke::NONE,
                }));
            }
        }
    }
    shapes
}

fn cubic(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    let x = u * u * u * p0.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * p3.0;
    let y = u * u * u * p0.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * p3.1;
    (x, y)
}

fn quadratic(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    (u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0,
     u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1)
}
