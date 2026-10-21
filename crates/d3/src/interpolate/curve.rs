//! The JavaScript's curves, which the interpolators draw along.
//!
//! Translated from `d3-interpolate/src/basis.js` and
//! `d3-interpolate/src/basisClosed.js`, version 1.4.0, of d3 5.16.0.

/// The JavaScript's `basis`, which draws along a curve through a list of
/// numbers.
pub fn basis(values: &[f64]) -> Box<dyn Fn(f64) -> f64> {
    let n = values.len().saturating_sub(1);
    let values = values.to_vec();
    Box::new(move |mut t| {
        let i = if t <= 0.0 {
            t = 0.0;
            0
        } else if t >= 1.0 {
            t = 1.0;
            n.saturating_sub(1)
        } else {
            (t * n as f64).floor() as usize
        };
        let v1 = values.get(i).copied().unwrap_or(f64::NAN);
        let v2 = values.get(i + 1).copied().unwrap_or(f64::NAN);
        let v0 = if i > 0 { values[i - 1] } else { 2.0 * v1 - v2 };
        let v3 = if i < n.saturating_sub(1) {
            values.get(i + 2).copied().unwrap_or(f64::NAN)
        } else {
            2.0 * v2 - v1
        };
        basis_at((t - i as f64 / n as f64) * n as f64, v0, v1, v2, v3)
    })
}

/// The JavaScript's `basisClosed`, which draws along a curve that comes
/// back to where it started.
pub fn basis_closed(values: &[f64]) -> Box<dyn Fn(f64) -> f64> {
    let n = values.len();
    let values = values.to_vec();
    Box::new(move |mut t| {
        t %= 1.0;
        if t < 0.0 {
            t += 1.0;
        }
        let i = (t * n as f64).floor() as usize;
        let v0 = values[(i + n - 1) % n];
        let v1 = values[i % n];
        let v2 = values[(i + 1) % n];
        let v3 = values[(i + 2) % n];
        basis_at((t - i as f64 / n as f64) * n as f64, v0, v1, v2, v3)
    })
}

/// The JavaScript's `basis`, which is the curve at a point of it.
fn basis_at(t1: f64, v0: f64, v1: f64, v2: f64, v3: f64) -> f64 {
    let t2 = t1 * t1;
    let t3 = t2 * t1;
    ((1.0 - 3.0 * t1 + 3.0 * t2 - t3) * v0
        + (4.0 - 6.0 * t2 + 3.0 * t3) * v1
        + (1.0 + 3.0 * t1 + 3.0 * t2 - 3.0 * t3) * v2
        + t3 * v3)
        / 6.0
}
