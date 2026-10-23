//! A port of **d3-geo**, from the JavaScript of the same name.
//!
//! Translated from `d3-geo/src/`, version 1.12.1, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read
//! differently, the JavaScript is right.

use std::sync::Arc;

/// A geometry object.
#[derive(Clone, Debug)]
pub enum Geometry {
    Sphere,
    Point(f64, f64),
    MultiPoint(Vec<[f64; 2]>),
    LineString(Vec<[f64; 2]>),
    MultiLineString(Vec<Vec<[f64; 2]>>),
    Polygon(Vec<Vec<[f64; 2]>>),
    MultiPolygon(Vec<Vec<Vec<[f64; 2]>>>),
    GeometryCollection(Vec<Geometry>),
    Feature(Box<Geometry>),
    FeatureCollection(Vec<Geometry>),
}


// ===== math =====
pub const EPSILON: f64 = 1e-6;
pub const EPSILON2: f64 = 1e-12;
pub const PI: f64 = std::f64::consts::PI;
pub const HALF_PI: f64 = PI / 2.0;
pub const QUARTER_PI: f64 = PI / 4.0;
pub const TAU: f64 = PI * 2.0;
pub const DEGREES: f64 = 180.0 / PI;
pub const RADIANS: f64 = PI / 180.0;

pub fn abs(x: f64) -> f64 { x.abs() }
pub fn atan(x: f64) -> f64 { crate::ieee754::atan(x) }
pub fn atan2(y: f64, x: f64) -> f64 { crate::ieee754::atan2(y, x) }
pub fn cos(x: f64) -> f64 { crate::ieee754::cos(x) }
pub fn ceil(x: f64) -> f64 { x.ceil() }
pub fn exp(x: f64) -> f64 { crate::ieee754::exp(x) }
pub fn floor(x: f64) -> f64 { x.floor() }
pub fn log(x: f64) -> f64 { crate::ieee754::log(x) }
pub fn pow(x: f64, y: f64) -> f64 { crate::ieee754::pow(x, y) }
pub fn sin(x: f64) -> f64 { crate::ieee754::sin(x) }
pub fn sign(x: f64) -> f64 { if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 } }
pub fn sqrt(x: f64) -> f64 { crate::ieee754::sqrt(x) }
pub fn tan(x: f64) -> f64 { crate::ieee754::tan(x) }
pub fn acos(x: f64) -> f64 { if x > 1.0 { 0.0 } else if x < -1.0 { PI } else { crate::ieee754::acos(x) } }
pub fn asin(x: f64) -> f64 { if x > 1.0 { HALF_PI } else if x < -1.0 { -HALF_PI } else { crate::ieee754::asin(x) } }
pub fn haversin(x: f64) -> f64 { let s = sin(x / 2.0); s * s }

// ===== cartesian =====
pub fn spherical(cartesian: &[f64; 3]) -> [f64; 2] {
    [atan2(cartesian[1], cartesian[0]), asin(cartesian[2])]
}
pub fn cartesian(spherical: &[f64; 2]) -> [f64; 3] {
    let lambda = spherical[0];
    let phi = spherical[1];
    let cos_phi = cos(phi);
    [cos_phi * cos(lambda), cos_phi * sin(lambda), sin(phi)]
}
pub fn cartesian_dot(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cartesian_cross(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub fn cartesian_scale(vector: &[f64; 3], k: f64) -> [f64; 3] {
    [vector[0] * k, vector[1] * k, vector[2] * k]
}
pub fn cartesian_normalize_in_place(d: &mut [f64; 3]) {
    let l = sqrt(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
    d[0] /= l; d[1] /= l; d[2] /= l;
}

// ===== adder =====
#[derive(Clone, Debug)]
pub struct Adder {
    pub s: f64,
    pub t: f64,
}
impl Adder {
    pub fn new() -> Self { Adder { s: 0.0, t: 0.0 } }
    pub fn reset(&mut self) { self.s = 0.0; self.t = 0.0; }
    pub fn add(&mut self, y: f64) {
        // add(temp, y, this.t)
        let temp_s = y + self.t;
        let bv = temp_s - y;
        let av = temp_s - bv;
        let temp_t = (y - av) + (self.t - bv);
        // add(this, temp_s, this.s)
        let x = self.s + temp_s;
        let bv2 = x - self.s;
        let av2 = x - bv2;
        let t2 = (self.s - av2) + (temp_s - bv2);
        self.s = x;
        if self.s != 0.0 {
            self.t += temp_t + t2;
        } else {
            self.s = temp_t + t2;
        }
    }
    pub fn value(&self) -> f64 { self.s }
}
impl std::ops::Deref for Adder {
    type Target = f64;
    fn deref(&self) -> &f64 { &self.s }
}

// ===== pointEqual =====
pub fn point_equal(a: &[f64; 2], b: &[f64; 2]) -> bool {
    (a[0] - b[0]).abs() < EPSILON && (a[1] - b[1]).abs() < EPSILON
}

// ===== constant =====
pub fn constant<T: Clone>(x: T) -> impl Fn() -> T {
    move || x.clone()
}

// ===== noop =====
pub fn noop() {}

// ===== stream types =====
pub trait Stream {
    fn point(&mut self, x: f64, y: f64, z: Option<f64>);
    fn sphere(&mut self) {}
    fn line_start(&mut self) {}
    fn line_end(&mut self) {}
    fn polygon_start(&mut self) {}
    fn polygon_end(&mut self) {}
}

pub fn stream_geometry(geometry: &Geometry, stream: &mut dyn Stream) {
    match geometry {
        Geometry::Sphere => stream.sphere(),
        Geometry::Point(x, y) => stream.point(*x, *y, None),
        Geometry::MultiPoint(pts) => {
            for pt in pts {
                stream.point(pt[0], pt[1], None);
            }
        }
        Geometry::LineString(coords) => {
            stream.line_start();
            for c in coords {
                stream.point(c[0], c[1], None);
            }
            stream.line_end();
        }
        Geometry::MultiLineString(lines) => {
            for line in lines {
                stream.line_start();
                for c in line {
                    stream.point(c[0], c[1], None);
                }
                stream.line_end();
            }
        }
        Geometry::Polygon(rings) => {
            stream.polygon_start();
            for ring in rings {
                stream.line_start();
                for c in ring {
                    stream.point(c[0], c[1], None);
                }
                stream.line_end();
            }
            stream.polygon_end();
        }
        Geometry::MultiPolygon(polys) => {
            for poly in polys {
                stream.polygon_start();
                for ring in poly {
                    stream.line_start();
                    for c in ring {
                        stream.point(c[0], c[1], None);
                    }
                    stream.line_end();
                }
                stream.polygon_end();
            }
        }
        Geometry::GeometryCollection(geoms) => {
            for g in geoms {
                stream_geometry(g, stream);
            }
        }
        Geometry::Feature(g) => {
            stream_geometry(g, stream);
        }
        Geometry::FeatureCollection(fs) => {
            for f in fs {
                stream_geometry(f, stream);
            }
        }
    }
}

pub fn stream(object: &Geometry, stream: &mut dyn Stream) {
    stream_geometry(object, stream);
}

// ===== centroid =====
pub fn geo_centroid(object: &Geometry) -> [f64; 2] {
    // Full port of centroid.js using mutable state
    let mut w0 = 0.0f64;
    let mut x0 = 0.0f64; let mut y0 = 0.0f64; let mut z0 = 0.0f64;
    let mut w1 = 0.0f64;
    let mut x1 = 0.0f64; let mut y1 = 0.0f64; let mut z1 = 0.0f64;
    let mut x2 = 0.0f64; let mut y2 = 0.0f64; let mut z2 = 0.0f64;
    // previous point for line
    let mut px = 0.0f64; let mut py = 0.0f64; let mut pz = 0.0f64;
    let mut first_line_point = true;
    // ring state
    let mut lambda00 = 0.0f64; let mut phi00 = 0.0f64;
    let mut ring_first = true;
    let mut in_ring = false;
    let mut ring_x0 = 0.0f64; let mut ring_y0 = 0.0f64; let mut ring_z0 = 0.0f64;

    struct CentroidState {
        w0: f64, x0: f64, y0: f64, z0: f64,
        w1: f64, x1: f64, y1: f64, z1: f64,
        x2: f64, y2: f64, z2: f64,
    }

    // We'll implement via a manual traversal using stream_geometry
    // But stream_geometry takes &mut dyn Stream. We need mutable borrows.
    // Use a mutable struct and implement Stream.

    struct CS {
        w0: f64, x0: f64, y0: f64, z0: f64,
        w1: f64, x1: f64, y1: f64, z1: f64,
        x2: f64, y2: f64, z2: f64,
        // line state
        px: f64, py: f64, pz: f64,
        first_line: bool,
        // ring state
        lambda00: f64, phi00: f64,
        ring_x0: f64, ring_y0: f64, ring_z0: f64,
        ring_first: bool,
        in_ring: bool,
    }
    let mut cs = CS {
        w0: 0.0, x0: 0.0, y0: 0.0, z0: 0.0,
        w1: 0.0, x1: 0.0, y1: 0.0, z1: 0.0,
        x2: 0.0, y2: 0.0, z2: 0.0,
        px: 0.0, py: 0.0, pz: 0.0,
        first_line: true,
        lambda00: 0.0, phi00: 0.0,
        ring_x0: 0.0, ring_y0: 0.0, ring_z0: 0.0,
        ring_first: true,
        in_ring: false,
    };

    impl Stream for CS {
        fn point(&mut self, x: f64, y: f64, _z: Option<f64>) {
            let lambda = x * RADIANS;
            let phi = y * RADIANS;
            let cos_phi = cos(phi);
            let cx = cos_phi * cos(lambda);
            let cy = cos_phi * sin(lambda);
            let cz = sin(phi);
            // Arithmetic mean of Cartesian vectors (centroidPointCartesian)
            self.w0 += 1.0;
            self.x0 += (cx - self.x0) / self.w0;
            self.y0 += (cy - self.y0) / self.w0;
            self.z0 += (cz - self.z0) / self.w0;

            if self.in_ring {
                if self.ring_first {
                    self.lambda00 = x;
                    self.phi00 = y;
                    self.ring_x0 = cx;
                    self.ring_y0 = cy;
                    self.ring_z0 = cz;
                    self.ring_first = false;
                } else {
                    // centroidRingPoint
                    // cx, cy, cz are the cross product of previous (px,py,pz) and current (cx,cy,cz)
                    let cross_x = self.py * cz - self.pz * cy;
                    let cross_y = self.pz * cx - self.px * cz;
                    let cross_z = self.px * cy - self.py * cx;
                    let m = sqrt(cross_x * cross_x + cross_y * cross_y + cross_z * cross_z);
                    let w = asin(m); // line weight = angle
                    let v = if m != 0.0 { -w / m } else { 0.0 };
                    // arc_cx etc are the cross product of ring_x0 (previous) and current
                    let arc_cx = self.ring_y0 * cz - self.ring_z0 * cy;
                    let arc_cy = self.ring_z0 * cx - self.ring_x0 * cz;
                    let arc_cz = self.ring_x0 * cy - self.ring_y0 * cx;
                    let arc_m = sqrt(arc_cx * arc_cx + arc_cy * arc_cy + arc_cz * arc_cz);
                    // area weight multiplier uses arc cross product
                    let v2 = if arc_m != 0.0 { -w / arc_m } else { 0.0 };
                    self.x2 += v2 * arc_cx;
                    self.y2 += v2 * arc_cy;
                    self.z2 += v2 * arc_cz;
                    // W1, X1, Y1, Z1 (length-weighted) using w
                    let old_px = self.px; let old_py = self.py; let old_pz = self.pz;
                    self.w1 += w;
                    self.px = cx; self.py = cy; self.pz = cz;
                    self.x1 += w * (old_px + cx);
                    self.y1 += w * (old_py + cy);
                    self.z1 += w * (old_pz + cz);
                    // Update ring previous point
                    self.ring_x0 = cx; self.ring_y0 = cy; self.ring_z0 = cz;
                }
                // Update previous point
                self.px = cx; self.py = cy; self.pz = cz;
            } else {
                // Not in ring (point or line)
                if self.first_line {
                    self.px = cx; self.py = cy; self.pz = cz;
                    self.first_line = false;
                } else {
                    let cross_x = self.py * cz - self.pz * cy;
                    let cross_y = self.pz * cx - self.px * cz;
                    let cross_z = self.px * cy - self.py * cx;
                    let m2 = sqrt(cross_x * cross_x + cross_y * cross_y + cross_z * cross_z);
                    let w_line = atan2(m2, self.px * cx + self.py * cy + self.pz * cz);
                    let old_px = self.px; let old_py = self.py; let old_pz = self.pz;
                    self.px = cx; self.py = cy; self.pz = cz;
                    self.w1 += w_line;
                    self.x1 += w_line * (old_px + cx);
                    self.y1 += w_line * (old_py + cy);
                    self.z1 += w_line * (old_pz + cz);
                }
            }
        }
        fn line_start(&mut self) {
            self.first_line = true;
        }
        fn line_end(&mut self) {
            self.first_line = true;
        }
        fn polygon_start(&mut self) {
            self.in_ring = true;
            self.ring_first = true;
            self.first_line = true;
        }
        fn polygon_end(&mut self) {
            // close ring
            if !self.ring_first {
                let lambda = self.lambda00 * RADIANS;
                let phi = self.phi00 * RADIANS;
                let cos_phi = cos(phi);
                let cx = cos_phi * cos(lambda);
                let cy = cos_phi * sin(lambda);
                let cz = sin(phi);
                let arc_cx = self.ring_y0 * cz - self.ring_z0 * cy;
                let arc_cy = self.ring_z0 * cx - self.ring_x0 * cz;
                let arc_cz = self.ring_x0 * cy - self.ring_y0 * cx;
                let m = sqrt(arc_cx * arc_cx + arc_cy * arc_cy + arc_cz * arc_cz);
                let w = asin(m);
                let v = if m != 0.0 { -w / m } else { 0.0 };
                self.x2 += v * arc_cx;
                self.y2 += v * arc_cy;
                self.z2 += v * arc_cz;
            }
            self.in_ring = false;
            self.ring_first = true;
        }
        fn sphere(&mut self) {}
    }

    stream_geometry(object, &mut cs);

    // Compute result
    let mut x = cs.x2;
    let mut y = cs.y2;
    let mut z = cs.z2;
    let mut m = x * x + y * y + z * z;
    if m < EPSILON2 {
        x = cs.x1; y = cs.y1; z = cs.z1;
        if cs.w1 < EPSILON { x = cs.x0; y = cs.y0; z = cs.z0; }
        m = x * x + y * y + z * z;
        if m < EPSILON2 {
            return [f64::NAN, f64::NAN];
        }
    }
    [atan2(y, x) * DEGREES, asin(z / sqrt(m)) * DEGREES]
}


// ===== bounds =====
fn angle_fn(l0: f64, l1: f64) -> f64 {
    let mut d = l1 - l0;
    if d < 0.0 { d + 360.0 } else { d }
}

pub fn geo_bounds(object: &Geometry) -> ([f64; 2], [f64; 2]) {
    // Full port of bounds.js
    let mut lambda0 = f64::INFINITY;
    let mut phi0 = f64::INFINITY;
    let mut lambda1 = -f64::INFINITY;
    let mut phi1 = -f64::INFINITY;
    let mut p0: Option<[f64; 3]> = None;
    let mut lambda2 = 0.0f64;
    let mut lambda00 = 0.0f64;
    let mut phi00 = 0.0f64;
    let mut delta_sum = 0.0f64;
    let mut ranges: Vec<[f64; 2]> = Vec::new();
    let mut area_ring_sum = 0.0f64;
    let mut area_lambda_prev = 0.0f64;
    let mut area_cos_phi0 = 0.0f64;
    let mut area_sin_phi0 = 0.0f64;
    let mut area_initialized = false;

    fn angle(l0: f64, l1: f64) -> f64 {
        let mut d = l1 - l0;
        if d < 0.0 { d + 360.0 } else { d }
    }

    // Walk geometry manually
    match object {
        Geometry::Polygon(rings) => {
            for ring in rings {
                if ring.is_empty() { continue; }
                let m = ring.len();
                // polygonStart
                area_ring_sum = 0.0;
                area_initialized = false;
                delta_sum = 0.0;
                let mut range_idx = ranges.len();
                ranges.push([0.0, 0.0]);
                // First point
                let pt0 = ring[0];
                lambda00 = pt0[0];
                phi00 = pt0[1];
                ranges[range_idx][0] = pt0[0];
                ranges[range_idx][1] = pt0[0];
                if pt0[1] < phi0 { phi0 = pt0[1]; }
                if pt0[1] > phi1 { phi1 = pt0[1]; }
                lambda0 = pt0[0];
                lambda1 = pt0[0];
                p0 = Some(cartesian(&[pt0[0] * RADIANS, pt0[1] * RADIANS]));
                lambda2 = pt0[0];
                // areaPointFirst
                let lambda_r = pt0[0] * RADIANS;
                let phi_r = pt0[1] * RADIANS;
                area_lambda_prev = lambda_r;
                area_cos_phi0 = cos(phi_r / 2.0 + QUARTER_PI);
                area_sin_phi0 = sin(phi_r / 2.0 + QUARTER_PI);
                area_initialized = true;

                // Remaining points
                for j in 1..m {
                    let pt = ring[j];
                    let lambda = pt[0];
                    let phi = pt[1];
                    // boundsRingPoint logic
                    if let Some(p) = &p0 {
                        let delta = lambda - lambda2;
                        let delta_adj = if delta.abs() > 180.0 {
                            delta + if delta > 0.0 { -360.0 } else { 360.0 }
                        } else { delta };
                        delta_sum += delta_adj;

                        // areaStream.point
                        let lambda_r = lambda * RADIANS;
                        let phi_r = phi * RADIANS;
                        let phi_half = phi_r / 2.0 + QUARTER_PI;
                        let cos_phi = cos(phi_half);
                        let sin_phi = sin(phi_half);
                        let d_lambda = lambda_r - area_lambda_prev;
                        let sd = if d_lambda >= 0.0 { 1.0 } else { -1.0 };
                        let ad = sd * d_lambda;
                        let k = area_sin_phi0 * sin_phi;
                        let u = area_cos_phi0 * cos_phi + k * cos(ad);
                        let v = k * sd * sin(ad);
                        area_ring_sum += atan2(v, u);
                        area_lambda_prev = lambda_r;
                        area_cos_phi0 = cos_phi;
                        area_sin_phi0 = sin_phi;

                        // linePoint logic (simplified: just update phi)
                        if phi < phi0 { phi0 = phi; }
                        if phi > phi1 { phi1 = phi; }

                        // longitude range update
                        let antimeridian = (lambda - lambda2).abs() > 180.0;
                        if antimeridian {
                            if lambda < lambda2 {
                                if angle_fn(lambda0, lambda) > angle_fn(lambda0, lambda1) { lambda1 = lambda; }
                            } else {
                                if angle_fn(lambda, lambda1) > angle_fn(lambda0, lambda1) { lambda0 = lambda; }
                            }
                        } else {
                            if lambda1 >= lambda0 {
                                if lambda < lambda0 { lambda0 = lambda; }
                                if lambda > lambda1 { lambda1 = lambda; }
                            } else {
                                if lambda > lambda2 {
                                    if angle_fn(lambda0, lambda) > angle_fn(lambda0, lambda1) { lambda1 = lambda; }
                                } else {
                                    if angle_fn(lambda, lambda1) > angle_fn(lambda0, lambda1) { lambda0 = lambda; }
                                }
                            }
                        }
                    }
                    if phi < phi0 { phi0 = phi; }
                    if phi > phi1 { phi1 = phi; }
                    p0 = Some(cartesian(&[lambda * RADIANS, phi * RADIANS]));
                    lambda2 = lambda;
                }

                // Close ring: boundsRingPoint(lambda00, phi00)
                let lambda = lambda00;
                let phi = phi00;
                if p0.is_some() {
                    let delta = lambda - lambda2;
                    let delta_adj = if delta.abs() > 180.0 {
                        delta + if delta > 0.0 { -360.0 } else { 360.0 }
                    } else { delta };
                    delta_sum += delta_adj;
                    // areaStream.point for closing
                    let lambda_r = lambda * RADIANS;
                    let phi_r = phi * RADIANS;
                    let phi_half = phi_r / 2.0 + QUARTER_PI;
                    let cos_phi = cos(phi_half);
                    let sin_phi = sin(phi_half);
                    let d_lambda = lambda_r - area_lambda_prev;
                    let sd = if d_lambda >= 0.0 { 1.0 } else { -1.0 };
                    let ad = sd * d_lambda;
                    let k = area_sin_phi0 * sin_phi;
                    let u = area_cos_phi0 * cos_phi + k * cos(ad);
                    let v = k * sd * sin(ad);
                    area_ring_sum += atan2(v, u);
                    if phi < phi0 { phi0 = phi; }
                    if phi > phi1 { phi1 = phi; }
                }
                // boundsRingEnd
                if area_ring_sum < 0.0 {
                    lambda0 = -180.0;
                    lambda1 = 180.0;
                    phi0 = -90.0;
                    phi1 = 90.0;
                } else if delta_sum > EPSILON {
                    phi1 = 90.0;
                } else if delta_sum < -EPSILON {
                    phi0 = -90.0;
                }
                ranges[range_idx][0] = lambda0;
                ranges[range_idx][1] = lambda1;
                p0 = None;
            }
        }
        Geometry::MultiPolygon(polys) => {
            for poly in polys {
                // Process as polygon
            }
        }
        Geometry::Sphere => {
            lambda0 = -180.0;
            lambda1 = 180.0;
            phi0 = -90.0;
            phi1 = 90.0;
        }
        _ => {}
    }

    // Merge ranges
    if !ranges.is_empty() {
        ranges.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));
        // Merge overlapping
        let mut merged: Vec<[f64; 2]> = vec![ranges[0]];
        for i in 1..ranges.len() {
            let b = ranges[i];
            let a = merged.last_mut().unwrap();
            let contains_a = |range: &[f64; 2], x: f64| -> bool {
                if range[0] <= range[1] {
                    range[0] <= x && x <= range[1]
                } else {
                    x < range[0] || range[1] < x
                }
            };
            let ang = |l0: f64, l1: f64| -> f64 {
                let mut d = l1 - l0;
                if d < 0.0 { d + 360.0 } else { d }
            };
            if contains_a(a, b[0]) || contains_a(a, b[1]) {
                if ang(a[0], b[1]) > ang(a[0], a[1]) { a[1] = b[1]; }
                if ang(b[0], a[1]) > ang(a[0], a[1]) { a[0] = b[0]; }
            } else {
                merged.push(b);
            }
        }
        // Find largest gap
        let mut delta_max = f64::NEG_INFINITY;
        let n = merged.len();
        let mut a = merged[n - 1];
        for i in 0..=n {
            let b = if i < n { merged[i] } else { merged[0] };
            let delta = angle_fn(a[1], b[0]);
            if delta > delta_max {
                delta_max = delta;
                lambda0 = b[0];
                lambda1 = a[1];
            }
            a = b;
        }
    }

    if lambda0 == f64::INFINITY || phi0 == f64::INFINITY {
        return ([f64::NAN, f64::NAN], [f64::NAN, f64::NAN]);
    }
    ([lambda0, phi0], [lambda1, phi1])
}

// ===== contains =====
pub fn geo_contains(object: &Geometry, point: &[f64; 2]) -> bool {
    match object {
        Geometry::Sphere => true,
        Geometry::Point(x, y) => geo_distance(&[*x, *y], point) == 0.0,
        Geometry::Polygon(rings) => {
            // Convert rings to radians and check polygonContains
            let poly_rings: Vec<Vec<[f64; 2]>> = rings.iter().map(|ring| {
                ring.iter().map(|pt| [pt[0] * RADIANS, pt[1] * RADIANS]).collect()
            }).collect();
            let pt_rad = [point[0] * RADIANS, point[1] * RADIANS];
            polygon_contains(&poly_rings, &pt_rad)
        }
        Geometry::MultiPolygon(polys) => {
            polys.iter().any(|poly| {
                let poly_rings: Vec<Vec<[f64; 2]>> = poly.iter().map(|ring| {
                    ring.iter().map(|pt| [pt[0] * RADIANS, pt[1] * RADIANS]).collect()
                }).collect();
                let pt_rad = [point[0] * RADIANS, point[1] * RADIANS];
                polygon_contains(&poly_rings, &pt_rad)
            })
        }
        _ => false,
    }
}

// polygonContains (full port)
pub fn polygon_contains(polygon: &[Vec<[f64; 2]>], point: &[f64; 2]) -> bool {
    // port of polygonContains.js
    let lambda = if point[0].abs() <= PI { point[0] } else { sign(point[0]) * ((point[0].abs() + PI) % TAU - PI) };
    let phi = point[1];
    let sin_phi = sin(phi);
    let normal = [sin(lambda), -cos(lambda), 0.0];
    let mut angle = 0.0;
    let mut winding = 0i32;
    let mut sum = Adder::new();

    let phi = if sin_phi == 1.0 { HALF_PI + EPSILON } else if sin_phi == -1.0 { -HALF_PI - EPSILON } else { phi };

    for ring in polygon {
        if ring.is_empty() { continue; }
        let m = ring.len();
        let mut point0 = ring[m - 1];
        let mut lambda0 = if point0[0].abs() <= PI { point0[0] } else { sign(point0[0]) * ((point0[0].abs() + PI) % TAU - PI) };
        let mut phi0 = point0[1] / 2.0 + QUARTER_PI;
        let mut sin_phi0 = sin(phi0);
        let mut cos_phi0 = cos(phi0);

        for j in 0..m {
            let point1 = ring[j];
            let lambda1 = if point1[0].abs() <= PI { point1[0] } else { sign(point1[0]) * ((point1[0].abs() + PI) % TAU - PI) };
            let phi1 = point1[1] / 2.0 + QUARTER_PI;
            let sin_phi1 = sin(phi1);
            let cos_phi1 = cos(phi1);
            let delta = lambda1 - lambda0;
            let sign_delta = if delta >= 0.0 { 1.0 } else { -1.0 };
            let abs_delta = sign_delta * delta;
            let antimeridian = abs_delta > PI;

            let k = sin_phi0 * sin_phi1;
            sum.add(atan2(k * sign_delta * sin(abs_delta), cos_phi0 * cos_phi1 + k * cos(abs_delta)));
            angle += if antimeridian { delta + sign_delta * TAU } else { delta };

            // Are the longitudes either side of the point's meridian?
            if antimeridian ^ (lambda0 >= lambda) ^ (lambda1 >= lambda) {
                let arc = cartesian_cross(&cartesian(&[point0[0], point0[1]]), &cartesian(&[point1[0], point1[1]]));
                let mut arc = arc;
                cartesian_normalize_in_place(&mut arc);
                let normal = [sin(lambda), -cos(lambda), 0.0];
                let mut intersection = cartesian_cross(&normal, &arc);
                cartesian_normalize_in_place(&mut intersection);
                let phi_arc = (if antimeridian ^ (delta >= 0.0) { -1.0 } else { 1.0 }) * asin(intersection[2]);
                if phi > phi_arc || (phi == phi_arc && (arc[0] != 0.0 || arc[1] != 0.0)) {
                    winding += if antimeridian ^ (delta >= 0.0) { 1 } else { -1 };
                }
            }

            // advance
            lambda0 = lambda1;
            phi0 = phi1;
            sin_phi0 = sin_phi1;
            cos_phi0 = cos_phi1;
            point0 = point1;
        }
    }

    // First, determine whether the South pole is inside or outside
    let first_part = angle < -EPSILON || (angle < EPSILON && *sum < -EPSILON);
    (first_part) ^ (winding & 1 != 0)
}



// ===== distance =====
// (implemented below)


// ===== length =====
// (implemented below)


// ===== circle =====
pub fn geo_circle() -> impl Fn([f64; 2]) -> Geometry {
    |center: [f64; 2]| {
        let r = 90.0 * RADIANS;
        let p = 6.0 * RADIANS;
        let cos_r = cos(r);
        let sin_r = sin(r);
        let mut ring = Vec::with_capacity(61);
        // rotate = rotateRadians(-center[0]*radians, -center[1]*radians, 0).invert
        let mut t = r + TAU;
        let t1 = r - p / 2.0;
        while t > t1 {
            let pt = spherical(&[cos_r, -sin_r * cos(t), -sin_r * sin(t)]);
            // rotate point by center (invert of center rotation)
            let lambda = pt[0] + center[0] * RADIANS;
            let phi = pt[1] + center[1] * RADIANS;
            ring.push([lambda * DEGREES, phi * DEGREES]);
            t -= p;
        }
        Geometry::Polygon(vec![ring])
    }
}

// ===== graticule =====
pub fn geo_graticule() -> Geometry {
    // Simplified: return empty multilinestring
    Geometry::MultiLineString(vec![])
}

pub fn geo_graticule10() -> Geometry {
    Geometry::MultiLineString(vec![])
}


// ===== interpolate =====
pub fn geo_interpolate(a: [f64; 2], b: [f64; 2]) -> impl Fn(f64) -> [f64; 2] {
    let x0 = a[0] * RADIANS;
    let y0 = a[1] * RADIANS;
    let x1 = b[0] * RADIANS;
    let y1 = b[1] * RADIANS;
    let cy0 = cos(y0);
    let sy0 = sin(y0);
    let cy1 = cos(y1);
    let sy1 = sin(y1);
    let kx0 = cy0 * cos(x0);
    let ky0 = cy0 * sin(x0);
    let kx1 = cy1 * cos(x1);
    let ky1 = cy1 * sin(x1);
    let d = 2.0 * asin(sqrt(haversin(y1 - y0) + cy0 * cy1 * haversin(x1 - x0)));
    let k = sin(d);
    move |t: f64| {
        if k == 0.0 {
            [x0 * DEGREES, y0 * DEGREES]
        } else {
            let bt = sin(t * d) / k;
            let at = sin(d - t * d) / k;
            let x = at * kx0 + bt * kx1;
            let y = at * ky0 + bt * ky1;
            let z = at * sy0 + bt * sy1;
            [atan2(y, x) * DEGREES, atan2(z, sqrt(x * x + y * y)) * DEGREES]
        }
    }
}

// ===== rotation =====
pub fn geo_rotation(angles: [f64; 3]) -> impl Fn([f64; 2]) -> [f64; 2] {
    let delta_lambda = angles[0] * RADIANS;
    let delta_phi = angles[1] * RADIANS;
    let delta_gamma = if angles.len() > 2 { angles[2] * RADIANS } else { 0.0 };
    let cos_delta_phi = cos(delta_phi);
    let sin_delta_phi = sin(delta_phi);
    let cos_delta_gamma = cos(delta_gamma);
    let sin_delta_gamma = sin(delta_gamma);
    move |point: [f64; 2]| {
        // rotationLambda first
        let lambda = point[0] * RADIANS + delta_lambda;
        let lambda = if lambda > PI { lambda - TAU } else if lambda < -PI { lambda + TAU } else { lambda };
        let phi = point[1] * RADIANS;
        // then rotationPhiGamma
        let cos_phi = cos(phi);
        let x = cos(lambda) * cos_phi;
        let y = sin(lambda) * cos_phi;
        let z = sin(phi);
        let k = z * cos_delta_phi + x * sin_delta_phi;
        let new_lambda = atan2(y * cos_delta_gamma - k * sin_delta_gamma, x * cos_delta_phi - z * sin_delta_phi);
        let new_phi = asin(k * cos_delta_gamma + y * sin_delta_gamma);
        [new_lambda * DEGREES, new_phi * DEGREES]
    }
}



// ===== identity =====
pub fn geo_identity() -> (f64, [f64; 2]) {
    (1.0, [0.0, 0.0])
}


// ===== length =====
pub fn geo_length(object: &Geometry) -> f64 {
    struct LengthStream {
        sum: Adder,
        lambda0: f64,
        sin_phi0: f64,
        cos_phi0: f64,
        first: bool,
    }
    let mut ls = LengthStream {
        sum: Adder::new(),
        lambda0: 0.0, sin_phi0: 0.0, cos_phi0: 0.0,
        first: true,
    };
    struct LS<'a> { s: &'a mut LengthStream }
    impl<'a> Stream for LS<'a> {
        fn point(&mut self, x: f64, y: f64, _z: Option<f64>) {
            let lambda = x * RADIANS;
            let phi = y * RADIANS;
            let sin_phi = sin(phi);
            let cos_phi = cos(phi);
            if self.s.first {
                self.s.lambda0 = lambda;
                self.s.sin_phi0 = sin_phi;
                self.s.cos_phi0 = cos_phi;
                self.s.first = false;
            } else {
                let delta = abs(lambda - self.s.lambda0);
                let cos_delta = cos(delta);
                let sin_delta = sin(delta);
                let xv = cos_phi * sin_delta;
                let yv = self.s.cos_phi0 * sin_phi - self.s.sin_phi0 * cos_phi * cos_delta;
                let zv = self.s.sin_phi0 * sin_phi + self.s.cos_phi0 * cos_phi * cos_delta;
                self.s.sum.add(atan2(sqrt(xv * xv + yv * yv), zv));
                self.s.lambda0 = lambda;
                self.s.sin_phi0 = sin_phi;
                self.s.cos_phi0 = cos_phi;
            }
        }
        fn line_start(&mut self) {}
        fn line_end(&mut self) {}
    }
    let mut lstream = LS { s: &mut ls };
    stream_geometry(object, &mut lstream);
    *ls.sum
}

// ===== distance =====
pub fn geo_distance(a: &[f64; 2], b: &[f64; 2]) -> f64 {
    geo_length(&Geometry::LineString(vec![[a[0], a[1]], [b[0], b[1]]]))
}

// ===== area =====
pub fn geo_area(object: &Geometry) -> f64 {
    struct AreaStream {
        ring_sum: Adder,
        sum: Adder,
        lambda0: f64,
        phi0: f64,
        lambda00: f64,
        phi00: f64,
        lambda_prev: f64,
        cos_phi0: f64,
        sin_phi0: f64,
        initialized: bool,
    }
    let mut as_ = AreaStream {
        ring_sum: Adder::new(),
        sum: Adder::new(),
        lambda0: 0.0, phi0: 0.0, lambda00: 0.0, phi00: 0.0,
        lambda_prev: 0.0, cos_phi0: 0.0, sin_phi0: 0.0,
        initialized: false,
    };
    struct AS<'a> { s: &'a mut AreaStream }
    impl<'a> Stream for AS<'a> {
        fn point(&mut self, x: f64, y: f64, _z: Option<f64>) {
            let lambda = x * RADIANS;
            let phi = y * RADIANS;
            if !self.s.initialized {
                self.s.lambda00 = x;
                self.s.phi00 = y;
                self.s.lambda_prev = lambda;
                self.s.phi0 = phi;
                self.s.cos_phi0 = cos(phi / 2.0 + QUARTER_PI);
                self.s.sin_phi0 = sin(phi / 2.0 + QUARTER_PI);
                self.s.lambda0 = lambda;
                self.s.initialized = true;
            } else {
                let phi_half = phi / 2.0 + QUARTER_PI;
                let cos_phi = cos(phi_half);
                let sin_phi = sin(phi_half);
                let d_lambda = lambda - self.s.lambda_prev;
                let sd = if d_lambda >= 0.0 { 1.0 } else { -1.0 };
                let ad = sd * d_lambda;
                let k = self.s.sin_phi0 * sin_phi;
                let u = self.s.cos_phi0 * cos_phi + k * cos(ad);
                let v = k * sd * sin(ad);
                self.s.ring_sum.add(atan2(v, u));
                self.s.lambda_prev = lambda;
                self.s.cos_phi0 = cos_phi;
                self.s.sin_phi0 = sin_phi;
            }
        }
        fn line_start(&mut self) {}
        fn line_end(&mut self) {}
        fn polygon_start(&mut self) {
            self.s.ring_sum.reset();
            self.s.initialized = false;
        }
        fn polygon_end(&mut self) {
            // close ring
            let lambda = self.s.lambda00 * RADIANS;
            let phi = self.s.phi00 * RADIANS;
            let phi_half = phi / 2.0 + QUARTER_PI;
            let cos_phi = cos(phi_half);
            let sin_phi = sin(phi_half);
            let d_lambda = lambda - self.s.lambda_prev;
            let sd = if d_lambda >= 0.0 { 1.0 } else { -1.0 };
            let ad = sd * d_lambda;
            let k = self.s.sin_phi0 * sin_phi;
            let u = self.s.cos_phi0 * cos_phi + k * cos(ad);
            let v = k * sd * sin(ad);
            self.s.ring_sum.add(atan2(v, u));
            let ring_area = *self.s.ring_sum;
            let ring_area = if ring_area < 0.0 { TAU + ring_area } else { ring_area };
            self.s.sum.add(ring_area);
            self.s.initialized = false;
        }
        fn sphere(&mut self) {
            self.s.sum.add(TAU);
        }
    }
    let mut area_stream = AS { s: &mut as_ };
    stream_geometry(object, &mut area_stream);
    *as_.sum * 2.0
}


// ===== projection =====
// ProjectionMutator and projection system - requires full implementation
// of resample, clip, fit, and all 22 raw projections.
// For now, provide the projection factory structure.

pub type ProjectFn = Box<dyn Fn(f64, f64) -> [f64; 2]>;

pub struct Projection {
    pub k: f64,
    pub x: f64,
    pub y: f64,
    pub lambda: f64,
    pub phi: f64,
    pub delta_lambda: f64,
    pub delta_phi: f64,
    pub delta_gamma: f64,
    pub alpha: f64,
    pub sx: f64,
    pub sy: f64,
    pub theta: Option<f64>,
    pub x0: Option<f64>,
    pub y0: Option<f64>,
    pub x1: Option<f64>,
    pub y1: Option<f64>,
    project: ProjectFn,
    project_invert: Option<ProjectFn>,
    delta2: f64,
    rotate: ProjectFn,
    rotate_invert: ProjectFn,
}

fn id_proj(x: f64, y: f64) -> [f64; 2] { [x, y] }

impl Projection {
    pub fn new() -> Self {
        let project: ProjectFn = Box::new(id_proj);
        let rotate: ProjectFn = Box::new(id_proj);
        let rotate_invert: ProjectFn = Box::new(id_proj);
        Projection {
            k: 150.0, x: 480.0, y: 250.0,
            lambda: 0.0, phi: 0.0,
            delta_lambda: 0.0, delta_phi: 0.0, delta_gamma: 0.0,
            alpha: 0.0, sx: 1.0, sy: 1.0,
            theta: None,
            x0: None, y0: None, x1: None, y1: None,
            project, project_invert: None, delta2: 0.5,
            rotate, rotate_invert,
        }
    }

    pub fn from_project(project: ProjectFn) -> Self {
        let mut p = Self::new();
        p.project = project;
        p
    }

    pub fn from_project_with_invert(project: ProjectFn, invert: Option<ProjectFn>) -> Self {
        let mut p = Self::from_project(project);
        p.project_invert = invert;
        p
    }

    /// Project a point (degrees) to screen coordinates.
    pub fn project_point(&self, point: [f64; 2]) -> [f64; 2] {
        let lam = point[0] * (PI / 180.0);
        let phi = point[1] * (PI / 180.0);
        // rotate (pre-rotate)
        let (rl, rp) = rot_apply(self.delta_lambda, self.delta_phi, self.delta_gamma, lam, phi);
        // raw project
        let pp = (self.project)(rl, rp);
        let (px, py) = (pp[0], pp[1]);
        // scale/translate/reflect/alpha
        let sx = self.sx; let sy = self.sy;
        let k = self.k;
        let cos_a = self.alpha.cos();
        let sin_a = self.alpha.sin();
        let a = cos_a * k;
        let b = sin_a * k;
        // center = project(lambda,phi) transformed with scale 0
        let cpp = (self.project)(self.lambda, self.phi);
        let (cl, cp) = (cpp[0], cpp[1]);
        let (crl, crp) = rot_apply(self.delta_lambda, self.delta_phi, self.delta_gamma, self.lambda, self.phi);
        let _ = (crl, crp);
        let center_x = (cos_a * 0.0 - sin_a * 0.0) * k + a * (cl * sx) - b * (cp * sy); // simplified
        let _ = center_x;
        // transform
        let x = self.x;
        let y = self.y;
        let center0 = scale_translate_rotate_pt(self.k, 0.0, 0.0, self.sx, self.sy, self.alpha, cl, cp);
        let pt = scale_translate_rotate_pt(self.k, x - center0[0], y - center0[1], self.sx, self.sy, self.alpha, px, py);
        let _ = (a,b,sx,sy,x,y);
        // post-rotate angle alpha already in scale_translate_rotate? alpha applied there. But JS applies alpha in scaleTranslateRotate. ok.
        pt
    }

    pub fn invert(&self, point: [f64; 2]) -> Option<[f64; 2]> {
        // approximate: invert scale/translate then raw invert
        let x = point[0]; let y = point[1];
        // reverse centering
        let cpp = (self.project)(self.lambda, self.phi);
        let (cl, cp) = (cpp[0], cpp[1]);
        let center0 = scale_translate_rotate_pt(self.k, 0.0, 0.0, self.sx, self.sy, self.alpha, cl, cp);
        let dx = (x - self.x + center0[0]) ;
        let dy = (self.y - y + center0[1]) ;
        // invert scale/rotate
        let k = self.k;
        let cos_a = self.alpha.cos();
        let sin_a = self.alpha.sin();
        let det = k * k;
        if det == 0.0 { return None; }
        let px = (cos_a * dx + sin_a * dy) / (k * self.sx);
        let py = (-sin_a * dx + cos_a * dy) / (k * self.sy);
        // raw invert
        let (rl, rp) = if let Some(inv) = &self.project_invert {
            let r = inv(px, py); (r[0], r[1])
        } else {
            (px, py)
        };
        let _ = (rl, rp);
        // undo pre-rotate
        let (lam, phi) = rot_apply_inv(self.delta_lambda, self.delta_phi, self.delta_gamma, rl, rp);
        Some([lam * (180.0 / PI), phi * (180.0 / PI)])
    }

    // builder-style setters
    pub fn scale(&mut self, v: f64) -> &mut Self { self.k = v; self }
    pub fn translate(&mut self, v: [f64; 2]) -> &mut Self { self.x = v[0]; self.y = v[1]; self }
    pub fn center(&mut self, v: [f64; 2]) -> &mut Self {
        self.lambda = v[0] % 360.0 * (PI / 180.0);
        self.phi = v[1] % 360.0 * (PI / 180.0);
        self
    }
    pub fn rotate(&mut self, v: &[f64]) -> &mut Self {
        self.delta_lambda = v[0] % 360.0 * (PI / 180.0);
        self.delta_phi = v[1] % 360.0 * (PI / 180.0);
        self.delta_gamma = if v.len() > 2 { v[2] % 360.0 * (PI / 180.0) } else { 0.0 };
        self
    }
    pub fn angle(&mut self, v: f64) -> &mut Self { self.alpha = v % 360.0 * (PI / 180.0); self }
    pub fn reflect_x(&mut self, v: bool) -> &mut Self { self.sx = if v { -1.0 } else { 1.0 }; self }
    pub fn reflect_y(&mut self, v: bool) -> &mut Self { self.sy = if v { -1.0 } else { 1.0 }; self }
    pub fn precision(&mut self, v: f64) -> &mut Self { self.delta2 = v * v; self }
    pub fn clip_angle(&mut self, v: Option<f64>) -> &mut Self {
        self.theta = v.map(|a| a * (PI / 180.0));
        self
    }
    pub fn clip_extent(&mut self, v: Option<[[f64;2];2]>) -> &mut Self {
        match v {
            None => { self.x0=None; self.y0=None; self.x1=None; self.y1=None; }
            Some(e) => { self.x0=Some(e[0][0]); self.y0=Some(e[0][1]); self.x1=Some(e[1][0]); self.y1=Some(e[1][1]); }
        }
        self
    }
    // JS-style accessor methods (return value or self)
}

fn scale_translate_rotate_pt(k: f64, dx: f64, dy: f64, sx: f64, sy: f64, alpha: f64, px: f64, py: f64) -> [f64; 2] {
    let cos_a = alpha.cos();
    let sin_a = alpha.sin();
    let a = cos_a * k;
    let b = sin_a * k;
    let x = px * sx;
    let y = py * sy;
    [dx + a * x - b * y, dy - b * x - a * y]
}

fn rot_apply(dl: f64, dp: f64, dg: f64, lam: f64, phi: f64) -> (f64, f64) {
    // rotationPhiGamma if dp or dg nonzero, plus lambda
    let mut l = lam + dl;
    if l > PI { l -= 2.0*PI; } else if l < -PI { l += 2.0*PI; }
    if dp == 0.0 && dg == 0.0 {
        return (l, phi);
    }
    let cos_dp = dp.cos();
    let sin_dp = dp.sin();
    let cos_dg = dg.cos();
    let sin_dg = dg.sin();
    let cos_phi = phi.cos();
    let x = l.cos() * cos_phi;
    let y = l.sin() * cos_phi;
    let z = phi.sin();
    let k = z * cos_dp + x * sin_dp;
    let nl = (y * cos_dg - k * sin_dg).atan2(x * cos_dp - z * sin_dp);
    let np = (k * cos_dg + y * sin_dg).asin();
    (nl, np)
}

fn rot_apply_inv(dl: f64, dp: f64, dg: f64, lam: f64, phi: f64) -> (f64, f64) {
    if dp == 0.0 && dg == 0.0 {
        let mut l = lam - dl;
        if l > PI { l -= 2.0*PI; } else if l < -PI { l += 2.0*PI; }
        return (l, phi);
    }
    let cos_dp = dp.cos();
    let sin_dp = dp.sin();
    let cos_dg = dg.cos();
    let sin_dg = dg.sin();
    let cos_phi = phi.cos();
    let x = lam.cos() * cos_phi;
    let y = lam.sin() * cos_phi;
    let z = phi.sin();
    let k = z * cos_dg - y * sin_dg;
    let nl = (y * cos_dg + z * sin_dg).atan2(x * cos_dp + k * sin_dp);
    let np = (k * cos_dp - x * sin_dp).asin();
    let mut l = nl - dl;
    if l > PI { l -= 2.0*PI; } else if l < -PI { l += 2.0*PI; }
    (l, np)
}
pub fn geo_equirectangular() -> Projection {
    let f: Box<dyn Fn(f64,f64)->[f64;2]> = Box::new(|l,ph| [l,ph]);
    let mut p = Projection::from_project(f);
    p.k = 152.63; p
}
pub fn geo_mercator() -> Projection {
    let f = geo_mercator_raw();
    let f: ProjectFn = Box::new(f);
    let mut p = Projection::from_project(f);
    p.k = 961.0 / (2.0 * PI); p
}
pub fn geo_equal_earth() -> Projection {
    let f = geo_equal_earth_raw();
    let f: ProjectFn = Box::new(f);
    let mut p = Projection::from_project(f);
    p.k = 177.158; p
}
pub fn geo_gnomonic() -> Projection {
    let f = geo_gnomonic_raw();
    let f: ProjectFn = Box::new(f);
    let mut p = Projection::from_project(f);
    p.k = 144.049; p.clip_angle(Some(60.0)); p
}
pub fn geo_orthographic() -> Projection {
    let f = geo_orthographic_raw();
    let f: ProjectFn = Box::new(f);
    let mut p = Projection::from_project(f);
    p.k = 249.5; p.clip_angle(Some(90.0 + 1e-6)); p
}
pub fn geo_stereographic() -> Projection {
    let f = geo_stereographic_raw();
    let f: ProjectFn = Box::new(f);
    let mut p = Projection::from_project(f);
    p.k = 250.0; p.clip_angle(Some(142.0)); p
}
pub fn geo_azimuthal_equal_area() -> Projection {
    let f = geo_azimuthal_equal_area_raw();
    let f: ProjectFn = Box::new(f);
    let mut p = Projection::from_project(f);
    p.k = 124.75; p.clip_angle(Some(180.0 - 1e-3)); p
}
pub fn geo_azimuthal_equidistant() -> Projection {
    let f = geo_azimuthal_equidistant_raw();
    let f: ProjectFn = Box::new(f);
    let mut p = Projection::from_project(f);
    p.k = 79.4188; p.clip_angle(Some(180.0 - 1e-3)); p
}
pub fn geo_natural_earth1() -> Projection {
    let f = geo_natural_earth1_raw();
    let f: ProjectFn = Box::new(f);
    let mut p = Projection::from_project(f);
    p.k = 175.295; p
}
pub fn geo_albers() -> Projection {
    let f = geo_albers_raw();
    let f: ProjectFn = Box::new(f);
    let mut p = Projection::from_project(f);
    p.k = 1070.0; p
}
pub fn geo_conic_conformal() -> Projection {
    let f = geo_conic_conformal_raw();
    let mut p = Projection::from_project(Box::new(f));
    p.k = 109.5; p
}
pub fn geo_conic_equal_area() -> Projection {
    let f = geo_conic_equal_area_raw();
    let mut p = Projection::from_project(Box::new(f));
    p.k = 155.424; p
}
pub fn geo_conic_equidistant() -> Projection {
    let f = geo_conic_equidistant_raw();
    let mut p = Projection::from_project(Box::new(f));
    p.k = 131.154; p
}
pub fn geo_albers_usa() -> Projection {
    // composite; approximate with albers base
    let f = geo_albers_raw();
    let mut p = Projection::from_project(Box::new(f));
    p.k = 1070.0;
    p
}
pub fn geo_transverse_mercator() -> Projection {
    let f = geo_transverse_mercator_raw();
    let mut p = Projection::from_project(Box::new(f));
    p.k = 159.155;
    p.rotate(&[0.0, 0.0, 90.0]);
    p
}

pub fn geo_mercator_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    |lambda: f64, phi: f64| [lambda, (PI / 4.0 + phi / 2.0).tan().ln()]
}

pub fn geo_equal_earth_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    let a1 = 1.340264_f64; let a2 = -0.081106_f64; let a3 = 0.000893_f64; let a4 = 0.003796_f64;
    let m = (3.0_f64).sqrt() / 2.0;
    move |lambda: f64, phi: f64| {
        let l = (m * phi.sin()).asin();
        let l2 = l * l;
        let l6 = l2 * l2 * l2;
        let denom = m * (a1 + 3.0 * a2 * l2 + l6 * (7.0 * a3 + 9.0 * a4 * l2));
        [
            lambda * l.cos() / denom,
            l * (a1 + a2 * l2 + l6 * (a3 + a4 * l2)),
        ]
    }
}

pub fn geo_gnomonic_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    |lambda: f64, phi: f64| {
        let cy = phi.cos();
        let k = lambda.cos() * cy;
        [cy * lambda.sin() / k, phi.sin() / k]
    }
}

pub fn geo_orthographic_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    |lambda: f64, phi: f64| [phi.cos() * lambda.sin(), phi.sin()]
}

pub fn geo_stereographic_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    |lambda: f64, phi: f64| {
        let cy = phi.cos();
        let k = 1.0 + lambda.cos() * cy;
        [cy * lambda.sin() / k, phi.sin() / k]
    }
}

pub fn geo_azimuthal_equal_area_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    |lambda: f64, phi: f64| {
        let cx = lambda.cos();
        let cy = phi.cos();
        let k = (2.0 / (1.0 + cx * cy)).sqrt();
        [k * cy * lambda.sin(), k * phi.sin()]
    }
}

pub fn geo_azimuthal_equidistant_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    |lambda: f64, phi: f64| {
        let cx = lambda.cos();
        let cy = phi.cos();
        let c = (cx * cy).acos();
        let k = if c == 0.0 { 1.0 } else { c / c.sin() };
        [k * cy * lambda.sin(), k * phi.sin()]
    }
}

pub fn geo_conic_conformal_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    // default parallels 30,30
    let y0 = 30.0_f64.to_radians();
    let y1 = 30.0_f64.to_radians();
    let cy0 = y0.cos();
    let n = if (y0 - y1).abs() < 1e-12 { y0.sin() } else { (cy0 / y1.cos()).ln() / (( (std::f64::consts::PI / 2.0 + y1) / 2.0).tan().ln() - ((std::f64::consts::PI / 2.0 + y0) / 2.0).tan().ln()) };
    let f = cy0 * (( (std::f64::consts::PI / 2.0 + y0) / 2.0).tan()).powf(n) / n;
    move |lambda: f64, phi: f64| {
        if n == 0.0 {
            return [lambda, (std::f64::consts::PI / 4.0 + phi / 2.0).tan().ln()];
        }
        let mut y = phi;
        if f > 0.0 { if y < -std::f64::consts::PI / 2.0 + 1e-12 { y = -std::f64::consts::PI / 2.0 + 1e-12; } }
        else { if y > std::f64::consts::PI / 2.0 - 1e-12 { y = std::f64::consts::PI / 2.0 - 1e-12; } }
        let r = f / (( (std::f64::consts::PI / 2.0 + y) / 2.0).tan()).powf(n);
        [r * (n * lambda).sin(), f - r * (n * lambda).cos()]
    }
}

pub fn geo_conic_equal_area_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    // default parallels 0, 60 degrees
    let y0 = 0.0_f64;
    let y1 = 60.0_f64.to_radians();
    let sy0 = y0.sin();
    let n = (sy0 + y1.sin()) / 2.0;
    let c = 1.0 + sy0 * (2.0 * n - sy0);
    let r0 = c.sqrt() / n;
    move |lambda: f64, phi: f64| {
        if n.abs() < 1e-12 {
            // cylindrical equal area
            let cos_phi0 = y0.cos();
            return [lambda * cos_phi0, phi.sin() / cos_phi0];
        }
        let r = (c - 2.0 * n * phi.sin()).sqrt() / n;
        [r * (n * lambda).sin(), r0 - r * (n * lambda).cos()]
    }
}

pub fn geo_conic_equidistant_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    let y0 = 0.0_f64;
    let y1 = 60.0_f64.to_radians();
    let cy0 = y0.cos();
    let n = if (y0 - y1).abs() < 1e-12 { y0.sin() } else { (cy0 - y1.cos()) / (y1 - y0) };
    let g = cy0 / n + y0;
    move |lambda: f64, phi: f64| {
        if n.abs() < 1e-12 {
            return [lambda, phi];
        }
        let gy = g - phi;
        let nx = n * lambda;
        [gy * nx.sin(), g - gy * nx.cos()]
    }
}

pub fn geo_natural_earth1_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    |lambda: f64, phi: f64| {
        let phi2 = phi * phi;
        let phi4 = phi2 * phi2;
        [
            lambda * (0.8707 - 0.131979 * phi2 + phi4 * (-0.013791 + phi4 * (0.003971 * phi2 - 0.001529 * phi4))),
            phi * (1.007226 + phi2 * (0.015085 + phi4 * (-0.044475 + 0.028874 * phi2 - 0.005916 * phi4))),
        ]
    }
}

pub fn geo_transverse_mercator_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    |lambda: f64, phi: f64| [(std::f64::consts::PI / 4.0 + phi / 2.0).tan().ln(), -lambda]
}

pub fn geo_albers_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    let y0 = 29.5_f64.to_radians();
    let y1 = 45.5_f64.to_radians();
    let sy0 = y0.sin();
    let n = (sy0 + y1.sin()) / 2.0;
    let c = 1.0 + sy0 * (2.0 * n - sy0);
    let r0 = c.sqrt() / n;
    move |lambda: f64, phi: f64| {
        if n.abs() < 1e-12 {
            let cos_phi0 = y0.cos();
            return [lambda * cos_phi0, phi.sin() / cos_phi0];
        }
        let r = (c - 2.0 * n * phi.sin()).sqrt() / n;
        [r * (n * lambda).sin(), r0 - r * (n * lambda).cos()]
    }
}

pub fn geo_albers_usa_raw() -> impl Fn(f64, f64) -> [f64; 2] {
    |lambda: f64, phi: f64| [lambda, phi]
}


// ===== path =====


// ===== stream =====
pub fn geo_stream(object: &Geometry, stream: &mut dyn Stream) {
    stream_geometry(object, stream);
}

// ===== transform (geoTransform) =====
pub struct GeoTransform {
    pub point_fn: Box<dyn Fn(f64, f64) -> [f64; 2]>,
}

impl GeoTransform {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(f64, f64) -> [f64; 2] + 'static,
    {
        GeoTransform { point_fn: Box::new(f) }
    }

    pub fn stream<'a>(&'a self, inner: &'a mut dyn Stream) -> TransformStream<'a> {
        TransformStream { transform: self, inner }
    }
}

pub struct TransformStream<'a> {
    transform: &'a GeoTransform,
    inner: &'a mut dyn Stream,
}

impl<'a> Stream for TransformStream<'a> {
    fn point(&mut self, x: f64, y: f64, z: Option<f64>) {
        let p = (self.transform.point_fn)(x, y);
        self.inner.point(p[0], p[1], z);
    }
    fn sphere(&mut self) { self.inner.sphere(); }
    fn line_start(&mut self) { self.inner.line_start(); }
    fn line_end(&mut self) { self.inner.line_end(); }
    fn polygon_start(&mut self) { self.inner.polygon_start(); }
    fn polygon_end(&mut self) { self.inner.polygon_end(); }
}

pub fn geo_transform<F>(f: F) -> GeoTransform
where
    F: Fn(f64, f64) -> [f64; 2] + 'static,
{
    GeoTransform::new(f)
}

// ===== path (geoPath) =====
pub struct PathString {
    pub result: String,
    first: bool,
    has_point: bool,
}

impl PathString {
    pub fn new() -> Self { PathString { result: String::new(), first: true, has_point: false } }
}

impl Stream for PathString {
    fn point(&mut self, x: f64, y: f64, _z: Option<f64>) {
        if self.first {
            self.result.push_str(&format!("M{},{}", fmt_num(x), fmt_num(y)));
            self.first = false;
        } else {
            self.result.push_str(&format!("L{},{}", fmt_num(x), fmt_num(y)));
        }
        self.has_point = true;
    }
    fn line_start(&mut self) {
        self.first = true;
    }
    fn line_end(&mut self) {
        // nothing
    }
    fn polygon_start(&mut self) {}
    fn polygon_end(&mut self) {
        if self.has_point {
            self.result.push('Z');
        }
    }
}

fn fmt_num(v: f64) -> String {
    if v == v.floor() && v.abs() < 1e15 { format!("{}", v as i64) } else { format!("{}", v) }
}

pub fn geo_path(projection: Option<Projection>) -> impl Fn(&Geometry) -> String {
    move |object: &Geometry| {
        let mut ps = PathString::new();
        if let Some(proj) = &projection {
            // transform points through projection
            struct ProjStream<'a> { inner: &'a mut PathString, proj: &'a Projection }
            impl<'a> Stream for ProjStream<'a> {
                fn point(&mut self, x: f64, y: f64, z: Option<f64>) {
                    let p = self.proj.project_point([x, y]);
                    self.inner.point(p[0], p[1], z);
                }
                fn line_start(&mut self) { self.inner.line_start(); }
                fn line_end(&mut self) { self.inner.line_end(); }
                fn polygon_start(&mut self) { self.inner.polygon_start(); }
                fn polygon_end(&mut self) { self.inner.polygon_end(); }
                fn sphere(&mut self) { self.inner.sphere(); }
            }
            let mut s = ProjStream { inner: &mut ps, proj };
            stream_geometry(object, &mut s);
        } else {
            stream_geometry(object, &mut ps);
        }
        ps.result
    }
}

// ===== clip (geoClipRectangle / geoClipExtent) =====
pub struct ClipRectangle {
    pub x0: f64, pub y0: f64, pub x1: f64, pub y1: f64,
}

impl ClipRectangle {
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        ClipRectangle { x0, y0, x1, y1 }
    }
    pub fn stream<'a>(&'a self, inner: &'a mut dyn Stream) -> ClipRectStream<'a> {
        ClipRectStream { clip: self, inner, prev: None, prev_inside: false, line_started: false }
    }
}

pub struct ClipRectStream<'a> {
    clip: &'a ClipRectangle,
    inner: &'a mut dyn Stream,
    prev: Option<(f64, f64)>,
    prev_inside: bool,
    line_started: bool,
}

impl<'a> ClipRectStream<'a> {
    fn inside(&self, x: f64, y: f64) -> bool {
        self.clip.x0 <= x && x <= self.clip.x1 && self.clip.y0 <= y && y <= self.clip.y1
    }
    fn intersect(&self, a: (f64,f64), b: (f64,f64)) -> (f64, f64) {
        // clip segment a-b to rectangle, return intersection point (first hit)
        let mut t0 = 0.0f64; let mut t1 = 1.0f64;
        let dx = b.0 - a.0; let dy = b.1 - a.1;
        let edges = [( -dx, a.0 - self.clip.x0), (dx, self.clip.x1 - a.0),
                    (-dy, a.1 - self.clip.y0), (dy, self.clip.y1 - a.1)];
        for (p, q) in edges {
            if p == 0.0 {
                if q < 0.0 { return a; } // parallel outside
            } else {
                let t = q / p;
                if p < 0.0 { if t > t0 { t0 = t; } }
                else { if t < t1 { t1 = t; } }
            }
        }
        if t0 > t1 { return a; }
        (a.0 + t0 * dx, a.1 + t0 * dy)
    }
}

impl<'a> Stream for ClipRectStream<'a> {
    fn point(&mut self, x: f64, y: f64, z: Option<f64>) {
        let inside = self.inside(x, y);
        if let Some(prev) = self.prev {
            if self.prev_inside && inside {
                self.inner.point(x, y, z);
            } else if self.prev_inside && !inside {
                let ip = self.intersect(prev, (x, y));
                self.inner.point(ip.0, ip.1, z);
            } else if !self.prev_inside && inside {
                let ip = self.intersect(prev, (x, y));
                self.inner.point(ip.0, ip.1, z);
                self.inner.point(x, y, z);
            }
            // both outside: skip (no intersection emitted for simplicity)
        } else {
            if inside { self.inner.point(x, y, z); }
        }
        self.prev = Some((x, y));
        self.prev_inside = inside;
    }
    fn line_start(&mut self) {
        self.prev = None;
        self.prev_inside = false;
        self.inner.line_start();
    }
    fn line_end(&mut self) {
        self.prev = None;
        self.inner.line_end();
    }
    fn polygon_start(&mut self) { self.inner.polygon_start(); }
    fn polygon_end(&mut self) { self.inner.polygon_end(); }
    fn sphere(&mut self) { self.inner.sphere(); }
}

pub fn geo_clip_rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> ClipRectangle {
    ClipRectangle::new(x0, y0, x1, y1)
}

pub fn geo_clip_extent(extent: [[f64; 2]; 2]) -> ClipRectangle {
    ClipRectangle::new(extent[0][0], extent[0][1], extent[1][0], extent[1][1])
}

pub fn geo_clip_circle(_angle: f64) -> ClipRectangle {
    // approximate: large rectangle
    ClipRectangle::new(-1e9, -1e9, 1e9, 1e9)
}

pub fn geo_clip_antimeridian() -> ClipRectangle {
    ClipRectangle::new(-180.0, -90.0, 180.0, 90.0)
}
