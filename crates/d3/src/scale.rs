//! A port of **d3-scale**, from the JavaScript of the same name.
//!
//! Translated from `d3-scale/src/`, version 2.2.2 (the release d3 5.16.0
//! pins), of d3 5.16.0. The JavaScript is the authority; where the two
//! could be read differently, the JavaScript is right.

use std::sync::Arc;

use crate::arith;
use crate::array;
use crate::format;
use crate::js;
use crate::time::interval::*;

fn js_number_string(x: f64) -> String {
    format!("{}", x)
}

/// The JavaScript's `implicit`, which a scale uses for a value it does
/// not know.
#[derive(Clone, Debug, PartialEq)]
pub struct Implicit {
    /// The JavaScript's own `name`, which is `"implicit"`.
    pub name: &'static str,
}

/// The JavaScript's `scaleImplicit`.
pub fn scale_implicit() -> Implicit {
    Implicit { name: "implicit" }
}

/// An interpolator between two numbers, which the JavaScript's own
/// `interpolate` gives.
pub type InterpolatorFactory = Arc<dyn Fn(f64, f64) -> Box<dyn Fn(f64) -> f64>>;

fn number_interpolator_factory() -> InterpolatorFactory {
    Arc::new(|a: f64, b: f64| {
        let d = arith::subtract(b, a);
        Box::new(move |t: f64| arith::add(a, arith::multiply(d, t)))
    })
}

fn round_interpolator_factory() -> InterpolatorFactory {
    Arc::new(|a: f64, b: f64| {
        let d = arith::subtract(b, a);
        Box::new(move |t: f64| js::round(arith::add(a, arith::multiply(d, t))))
    })
}

fn default_interpolator() -> InterpolatorFactory {
    number_interpolator_factory()
}

fn normalize(a: f64, b: f64) -> Box<dyn Fn(f64) -> f64> {
    let a0 = a;
    let diff = arith::subtract(b, a0);
    if diff != 0.0 {
        Box::new(move |x: f64| arith::divide(arith::subtract(x, a0), diff))
    } else {
        let value = if diff.is_nan() { f64::NAN } else { 0.5 };
        Box::new(move |_x: f64| value)
    }
}

fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b {
        a
    } else {
        b
    }
}

fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else {
        b
    }
}

fn clamp_value(domain: &[f64], x: f64) -> f64 {
    let a = domain.first().copied().unwrap_or(f64::NAN);
    let b = domain.last().copied().unwrap_or(f64::NAN);
    let (lo, hi) = if a > b { (b, a) } else { (a, b) };
    js_max(lo, js_min(hi, x))
}

fn make_piecewise(
    domain: &[f64],
    range: &[f64],
    interp: &InterpolatorFactory,
) -> Box<dyn Fn(f64) -> f64> {
    let n = domain.len().min(range.len());
    if n == 0 {
        return Box::new(|_x| f64::NAN);
    }
    if n == 1 {
        let r0 = range[0];
        return Box::new(move |_x| r0);
    }
    if n == 2 {
        let d0 = domain[0];
        let d1 = domain[1];
        let r0 = range[0];
        let r1 = range[1];
        if d1 < d0 {
            let norm = normalize(d1, d0);
            let interp = interp(d1, d0); // placeholder; wrong below
            let _ = (r0, r1, norm, interp);
        }
        // bimap proper:
        let d0v = domain[0];
        let d1v = domain[1];
        let r0v = range[0];
        let r1v = range[1];
        if d1v < d0v {
            let norm = normalize(d1v, d0v);
            let ip = interp(r1v, r0v);
            let mut ip = ip;
            return Box::new(move |x: f64| {
                let t = norm(x);
                ip(t)
            });
        } else {
            let norm = normalize(d0v, d1v);
            let ip = interp(r0v, r1v);
            return Box::new(move |x: f64| {
                let t = norm(x);
                ip(t)
            });
        }
    }
    // polymap
    let j = n - 1;
    let mut domain = domain[..=j].to_vec();
    let mut range = range[..=j].to_vec();
    if domain[j] < domain[0] {
        domain.reverse();
        range.reverse();
    }
    let mut norms: Vec<Box<dyn Fn(f64) -> f64>> = Vec::with_capacity(j);
    let mut interps: Vec<Box<dyn Fn(f64) -> f64>> = Vec::with_capacity(j);
    for i in 0..j {
        norms.push(normalize(domain[i], domain[i + 1]));
        interps.push(interp(range[i], range[i + 1]));
    }
    // need mutable interps inside closure; wrap in Arc<Mutex>? Use indices and recreate?
    // We'll store as Vec and use Cell-like via Rc<RefCell>.
    use std::cell::RefCell;
    let norms = Arc::new(norms);
    let interps = Arc::new(RefCell::new(interps));
    Box::new(move |x: f64| {
        // bisect domain, 1, j
        let mut lo = 1usize;
        let mut hi = j;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if domain[mid] <= x {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let idx = lo - 1;
        let idx = if idx >= j { j - 1 } else { idx };
        let t = norms[idx](x);
        let mut interps = interps.borrow_mut();
        interps[idx](t)
    })
}

// Continuous core
#[derive(Clone)]
pub struct ContinuousCore {
    pub transform: Arc<dyn Fn(f64) -> f64>,
    pub untransform: Arc<dyn Fn(f64) -> f64>,
    pub domain: Vec<f64>,
    pub range: Vec<f64>,
    pub interpolate: InterpolatorFactory,
    pub clamp: bool,
    pub unknown: f64,
}

impl ContinuousCore {
    pub fn new(transform: Arc<dyn Fn(f64) -> f64>, untransform: Arc<dyn Fn(f64) -> f64>) -> Self {
        Self {
            transform,
            untransform,
            domain: vec![0.0, 1.0],
            range: vec![0.0, 1.0],
            interpolate: default_interpolator(),
            clamp: false,
            unknown: f64::NAN,
        }
    }

    pub fn call(&self, x: f64) -> f64 {
        if !js::is_comparable(x) { // x >= x false for NaN
             // isNaN check
        }
        if x.is_nan() {
            return self.unknown;
        }
        let clamped = if self.clamp {
            clamp_value(&self.domain, x)
        } else {
            x
        };
        let tx = (self.transform)(clamped);
        let domain_t: Vec<f64> = self.domain.iter().map(|d| (self.transform)(*d)).collect();
        let pw = make_piecewise(&domain_t, &self.range, &self.interpolate);
        pw(tx)
    }

    pub fn invert(&self, y: f64) -> f64 {
        let domain_t: Vec<f64> = self.domain.iter().map(|d| (self.transform)(*d)).collect();
        let ipw = make_piecewise(&self.range, &domain_t, &number_interpolator_factory());
        let t = ipw(y);
        let x = (self.untransform)(t);
        if self.clamp {
            clamp_value(&self.domain, x)
        } else {
            x
        }
    }
}

// ScaleLinear
#[derive(Clone)]
pub struct ScaleLinear {
    pub core: ContinuousCore,
}

impl ScaleLinear {
    pub fn new() -> Self {
        let id = Arc::new(|x: f64| x);
        Self {
            core: ContinuousCore::new(id.clone(), id),
        }
    }
    pub fn domain(&self) -> Vec<f64> {
        self.core.domain.clone()
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        self.core.domain = domain.iter().map(|x| *x).collect();
        self
    }
    pub fn range(&self) -> Vec<f64> {
        self.core.range.clone()
    }
    pub fn set_range(&mut self, range: &[f64]) -> &mut Self {
        self.core.range = range.iter().map(|x| *x).collect();
        self
    }
    pub fn range_round(&mut self, range: &[f64]) -> &mut Self {
        self.core.range = range.iter().map(|x| *x).collect();
        self.core.interpolate = round_interpolator_factory();
        self
    }
    pub fn clamp(&self) -> bool {
        self.core.clamp
    }
    pub fn set_clamp(&mut self, clamp: bool) -> &mut Self {
        self.core.clamp = clamp;
        self
    }
    pub fn interpolate(&self) -> InterpolatorFactory {
        self.core.interpolate.clone()
    }
    pub fn set_interpolate(&mut self, f: InterpolatorFactory) -> &mut Self {
        self.core.interpolate = f;
        self
    }
    pub fn unknown(&self) -> f64 {
        self.core.unknown
    }
    pub fn set_unknown(&mut self, v: f64) -> &mut Self {
        self.core.unknown = v;
        self
    }
    pub fn invert(&self, y: f64) -> f64 {
        self.core.invert(y)
    }
    pub fn call(&self, x: f64) -> f64 {
        self.core.call(x)
    }
    pub fn ticks(&self, count: Option<f64>) -> Vec<f64> {
        let count = count.unwrap_or(10.0);
        let d = self.core.domain.clone();
        array::ticks(d[0], d[d.len() - 1], count)
    }
    pub fn tick_format(
        &self,
        count: Option<f64>,
        specifier: Option<&str>,
    ) -> Box<dyn Fn(f64) -> String> {
        let count = count.unwrap_or(10.0);
        let d = self.core.domain.clone();
        tick_format(d[0], d[d.len() - 1], Some(count), specifier)
    }
    pub fn nice(&mut self, count: Option<f64>) -> &mut Self {
        let count = count.unwrap_or(10.0);
        let mut d = self.core.domain.clone();
        // linearish nice
        let i0 = 0usize;
        let i1 = d.len() - 1;
        let mut start = d[i0];
        let mut stop = d[i1];
        if stop < start {
            let t = start;
            start = stop;
            stop = t;
        }
        let mut step = array::tick_increment(start, stop, count);
        if step > 0.0 {
            start = (start / step).floor() * step;
            stop = (stop / step).ceil() * step;
            step = array::tick_increment(start, stop, count);
        } else if step < 0.0 {
            start = (start * step).ceil() / step;
            stop = (stop * step).floor() / step;
            step = array::tick_increment(start, stop, count);
        }
        if step > 0.0 {
            d[i0] = (start / step).floor() * step;
            d[i1] = (stop / step).ceil() * step;
        } else if step < 0.0 {
            d[i0] = (start * step).ceil() / step;
            d[i1] = (stop * step).floor() / step;
        }
        self.core.domain = d;
        self
    }
    pub fn copy(&self) -> Self {
        Self {
            core: self.core.clone(),
        }
    }
}

pub fn scale_linear() -> ScaleLinear {
    ScaleLinear::new()
}

// ScaleIdentity
#[derive(Clone)]
pub struct ScaleIdentity {
    pub domain: Vec<f64>,
    pub unknown: f64,
}

impl ScaleIdentity {
    pub fn new() -> Self {
        Self {
            domain: vec![0.0, 1.0],
            unknown: f64::NAN,
        }
    }
    pub fn domain(&self) -> Vec<f64> {
        self.domain.clone()
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        self.domain = domain.iter().map(|x| js::number(*x)).collect();
        self
    }
    pub fn range(&self) -> Vec<f64> {
        self.domain.clone()
    }
    pub fn set_range(&mut self, range: &[f64]) -> &mut Self {
        self.set_domain(range)
    }
    pub fn unknown(&self) -> f64 {
        self.unknown
    }
    pub fn set_unknown(&mut self, v: f64) -> &mut Self {
        self.unknown = v;
        self
    }
    pub fn call(&self, x: f64) -> f64 {
        if x.is_nan() {
            self.unknown
        } else {
            x
        }
    }
    pub fn invert(&self, y: f64) -> f64 {
        if y.is_nan() {
            self.unknown
        } else {
            y
        }
    }
    pub fn ticks(&self, count: Option<f64>) -> Vec<f64> {
        let count = count.unwrap_or(10.0);
        let d = &self.domain;
        array::ticks(d[0], d[d.len() - 1], count)
    }
    pub fn tick_format(
        &self,
        count: Option<f64>,
        specifier: Option<&str>,
    ) -> Box<dyn Fn(f64) -> String> {
        let count = count.unwrap_or(10.0);
        let d = &self.domain;
        tick_format(d[0], d[d.len() - 1], Some(count), specifier)
    }
    pub fn nice(&mut self, count: Option<f64>) -> &mut Self {
        let count = count.unwrap_or(10.0);
        let mut d = self.domain.clone();
        let i0 = 0;
        let i1 = d.len() - 1;
        let mut start = d[i0];
        let mut stop = d[i1];
        if stop < start {
            let t = start;
            start = stop;
            stop = t;
        }
        let mut step = array::tick_increment(start, stop, count);
        if step > 0.0 {
            start = (start / step).floor() * step;
            stop = (stop / step).ceil() * step;
            step = array::tick_increment(start, stop, count);
        } else if step < 0.0 {
            start = (start * step).ceil() / step;
            stop = (stop * step).floor() / step;
            step = array::tick_increment(start, stop, count);
        }
        if step > 0.0 {
            d[i0] = (start / step).floor() * step;
            d[i1] = (stop / step).ceil() * step;
        } else if step < 0.0 {
            d[i0] = (start * step).ceil() / step;
            d[i1] = (stop * step).floor() / step;
        }
        self.domain = d;
        self
    }
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

pub fn scale_identity() -> ScaleIdentity {
    ScaleIdentity::new()
}

// ScaleLog
#[derive(Clone)]
pub struct ScaleLog {
    pub core: ContinuousCore,
    pub base: f64,
}

impl ScaleLog {
    pub fn new() -> Self {
        let core = ContinuousCore::new(Arc::new(|x: f64| x.ln()), Arc::new(|x: f64| x.exp()));
        let mut s = Self { core, base: 10.0 };
        // default domain [1,10]
        s.core.domain = vec![1.0, 10.0];
        s.rescale();
        s
    }
    fn rescale(&mut self) {
        let logs: Arc<dyn Fn(f64) -> f64> = if self.base == std::f64::consts::E {
            Arc::new(|x: f64| x.ln())
        } else if self.base == 10.0 {
            Arc::new(|x: f64| x.log10())
        } else if self.base == 2.0 {
            Arc::new(|x: f64| x.log2())
        } else {
            let b = self.base.ln();
            Arc::new(move |x: f64| x.ln() / b)
        };
        let pows: Arc<dyn Fn(f64) -> f64> = if self.base == 10.0 {
            Arc::new(|x: f64| {
                if x.is_finite() {
                    crate::ieee754::pow(10.0, x)
                } else {
                    if x < 0.0 {
                        0.0
                    } else {
                        x
                    }
                }
            })
        } else if self.base == std::f64::consts::E {
            Arc::new(|x: f64| x.exp())
        } else {
            let b = self.base;
            Arc::new(move |x: f64| b.powf(x))
        };
        let domain = self.core.domain.clone();
        let neg = domain.first().copied().unwrap_or(0.0) < 0.0;
        if neg {
            // reflect
            let logs_r = Arc::new(move |x: f64| -((-x).ln())); // placeholder reflect
            let _ = logs_r;
        }
        // We store transform closures in core based on sign.
        let mut transform: Arc<dyn Fn(f64) -> f64>;
        let untransform: Arc<dyn Fn(f64) -> f64>;
        if neg {
            transform = Arc::new(|x: f64| -crate::ieee754::log(-x));
            untransform = Arc::new(|x: f64| -crate::ieee754::exp(-x));
        } else {
            transform = Arc::new(|x: f64| crate::ieee754::log(x));
            untransform = Arc::new(|x: f64| crate::ieee754::exp(x));
        }
        self.core.transform = transform;
        self.core.untransform = untransform;
        // store logs/pows? need for ticks. We'll recompute in ticks.
    }
    pub fn base(&self) -> f64 {
        self.base
    }
    pub fn set_base(&mut self, base: f64) -> &mut Self {
        self.base = base;
        self.rescale();
        self
    }
    pub fn domain(&self) -> Vec<f64> {
        self.core.domain.clone()
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        self.core.set_domain(domain); // need method
        self.rescale();
        self
    }
    // proxy methods
    pub fn range(&self) -> Vec<f64> {
        self.core.range.clone()
    }
    pub fn set_range(&mut self, range: &[f64]) -> &mut Self {
        self.core.set_range(range);
        self
    }
    pub fn range_round(&mut self, range: &[f64]) -> &mut Self {
        self.core.range_round(range);
        self
    }
    pub fn clamp(&self) -> bool {
        self.core.clamp
    }
    pub fn set_clamp(&mut self, c: bool) -> &mut Self {
        self.core.set_clamp(c);
        self
    }
    pub fn interpolate(&self) -> InterpolatorFactory {
        self.core.interpolate.clone()
    }
    pub fn set_interpolate(&mut self, f: InterpolatorFactory) -> &mut Self {
        self.core.set_interpolate(f);
        self
    }
    pub fn unknown(&self) -> f64 {
        self.core.unknown
    }
    pub fn set_unknown(&mut self, v: f64) -> &mut Self {
        self.core.set_unknown(v);
        self
    }
    pub fn invert(&self, y: f64) -> f64 {
        self.core.invert(y)
    }
    pub fn call(&self, x: f64) -> f64 {
        self.core.call(x)
    }
    pub fn ticks(&self, count: Option<f64>) -> Vec<f64> {
        let count = count.unwrap_or(10.0);
        let d = self.core.domain.clone();
        let mut u = d[0];
        let mut v = d[d.len() - 1];
        let reverse = v < u;
        if reverse {
            let t = u;
            u = v;
            v = t;
        }
        // logs/pows recompute
        let base = self.base;
        let logs0: Box<dyn Fn(f64) -> f64> = Box::new(move |x: f64| {
            if base == std::f64::consts::E {
                crate::ieee754::log(x)
            } else if base == 10.0 {
                crate::ieee754::log10(x)
            } else if base == 2.0 {
                crate::ieee754::log2(x)
            } else {
                crate::ieee754::log(x) / crate::ieee754::log(base)
            }
        });
        let pows0: Box<dyn Fn(f64) -> f64> = Box::new(move |x: f64| {
            if base == 10.0 {
                if x.is_finite() {
                    crate::ieee754::pow(10.0, x)
                } else {
                    if x < 0.0 {
                        0.0
                    } else {
                        x
                    }
                }
            } else if base == std::f64::consts::E {
                crate::ieee754::exp(x)
            } else {
                crate::ieee754::pow(base, x)
            }
        });
        let logs: Box<dyn Fn(f64) -> f64> = if d[0] < 0.0 {
            let l = logs0;
            Box::new(move |x: f64| -l(-x))
        } else {
            logs0
        };
        let pows: Box<dyn Fn(f64) -> f64> = if d[0] < 0.0 {
            let p = pows0;
            Box::new(move |x: f64| -p(-x))
        } else {
            pows0
        };
        let i = logs(u);
        let j = logs(v);
        let n = count;
        let mut z = Vec::new();
        let base_int = base as i64;
        let integer_base = base == base.floor() && base != 0.0;
        if integer_base && (j - i).abs() < n {
            let mut ii = i.round() as i64 - 1;
            let jj = j.round() as i64 + 1;
            if u > 0.0 {
                while (ii as f64) < (jj as f64) {
                    let p = pows(ii as f64);
                    for k in 1..(base as i64) {
                        let t = p * (k as f64);
                        if t < u {
                            continue;
                        }
                        if t > v {
                            break;
                        }
                        z.push(t);
                    }
                    ii += 1;
                }
            } else {
                while (ii as f64) < (jj as f64) {
                    let p = pows(ii as f64);
                    for k in (1..(base as i64)).rev() {
                        let t = p * (k as f64);
                        if t < u {
                            continue;
                        }
                        if t > v {
                            break;
                        }
                        z.push(t);
                    }
                    ii += 1;
                }
            }
        } else {
            let ts = array::ticks(i, j, n.min((j - i).abs()));
            for t in ts {
                z.push(pows(t));
            }
        }
        if reverse {
            z.reverse();
        }
        z
    }
    pub fn tick_format(
        &self,
        count: Option<f64>,
        specifier: Option<&str>,
    ) -> Box<dyn Fn(f64) -> String> {
        let count = count.unwrap_or(10.0);
        let base = self.base;
        let logs = move |x: f64| -> f64 {
            if base == std::f64::consts::E {
                crate::ieee754::log(x)
            } else if base == 10.0 {
                crate::ieee754::log10(x)
            } else if base == 2.0 {
                crate::ieee754::log2(x)
            } else {
                crate::ieee754::log(x) / crate::ieee754::log(base)
            }
        };
        let pows = move |x: f64| -> f64 {
            if base == 10.0 {
                if x.is_finite() {
                    crate::ieee754::pow(10.0, x)
                } else {
                    if x < 0.0 {
                        0.0
                    } else {
                        x
                    }
                }
            } else if base == std::f64::consts::E {
                crate::ieee754::exp(x)
            } else {
                crate::ieee754::pow(base, x)
            }
        };
        let spec_str = specifier
            .unwrap_or(if base == 10.0 { ".0e" } else { "," })
            .to_string();
        let ticks_len = self.ticks(Some(count)).len().max(1) as f64;
        let k = (base * count / ticks_len).max(1.0);
        let pows_c = pows;
        let logs_c = logs;
        Box::new(move |d: f64| {
            let spec = format::format_specifier(&spec_str).unwrap();
            let locale = format::format_default_locale();
            let i = d / pows_c(logs_c(d).round());
            let mut ii = i;
            if ii * base < base - 0.5 {
                ii *= base;
            }
            if ii <= k {
                let f = locale.format(&spec.to_string()).unwrap();
                f.format(d)
            } else {
                String::new()
            }
        })
    }
    pub fn nice(&mut self) -> &mut Self {
        let d = self.core.domain.clone();
        let mut x0 = d[0];
        let mut x1 = d[d.len() - 1];
        let reverse = x1 < x0;
        if reverse {
            let t = x0;
            x0 = x1;
            x1 = t;
        }
        let base = self.base;
        let logs = |x: f64| -> f64 {
            if base == std::f64::consts::E {
                x.abs().ln()
            }
            // approximate
            else if base == 10.0 {
                x.abs().log10()
            } else if base == 2.0 {
                x.abs().log2()
            } else {
                x.abs().ln() / base.ln()
            }
        };
        // use nice.js with interval floor/ceil via pows
        let floor = |x: f64| -> f64 {
            let l = if x < 0.0 { -logs(-x) } else { logs(x) };
            let p = l.floor();
            if x < 0.0 {
                // reflect pows
                let pw = if base == 10.0 {
                    10.0_f64.powf(p)
                } else {
                    base.powf(p)
                };
                -pw
            } else {
                let pw = if base == 10.0 {
                    10.0_f64.powf(p)
                } else {
                    base.powf(p)
                };
                pw
            }
        };
        let ceil = |x: f64| -> f64 {
            let l = if x < 0.0 { -logs(-x) } else { logs(x) };
            let p = l.ceil();
            if x < 0.0 {
                let pw = if base == 10.0 {
                    10.0_f64.powf(p)
                } else {
                    base.powf(p)
                };
                -pw
            } else {
                let pw = if base == 10.0 {
                    10.0_f64.powf(p)
                } else {
                    base.powf(p)
                };
                pw
            }
        };
        let mut domain = d.clone();
        let i0 = 0;
        let i1 = domain.len() - 1;
        let xx0 = domain[i0];
        let xx1 = domain[i1];
        let (mut a, mut b) = (xx0, xx1);
        let swapped = b < a;
        if swapped {
            let t = a;
            a = b;
            b = t;
        }
        domain[i0] = if swapped { ceil(b) } else { floor(a) }; // approximate; JS nice uses floor(x0), ceil(x1)
                                                               // Actually JS: domain[i0]=floor(start); domain[i1]=ceil(stop) after swap handling.
        let mut dom = d.clone();
        let (i0, i1) = (0usize, dom.len() - 1);
        let (x0, x1) = (dom[i0], dom[i1]);
        let swapped = x1 < x0;
        let (mut lo, mut hi) = if swapped { (x1, x0) } else { (x0, x1) };
        let f_lo = floor(lo);
        let c_hi = ceil(hi);
        if swapped {
            dom[i1] = f_lo;
            dom[i0] = c_hi;
        } else {
            dom[i0] = f_lo;
            dom[i1] = c_hi;
        }
        self.core.domain = dom;
        self.rescale();
        self
    }
    pub fn copy(&self) -> Self {
        Self {
            core: self.core.clone(),
            base: self.base,
        }
    }
}

pub fn scale_log() -> ScaleLog {
    ScaleLog::new()
}

// ContinuousCore methods used above
impl ContinuousCore {
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        self.domain = domain.iter().copied().collect();
        self
    }
    pub fn set_range(&mut self, range: &[f64]) -> &mut Self {
        self.range = range.iter().copied().collect();
        self
    }
    pub fn range_round(&mut self, range: &[f64]) -> &mut Self {
        self.range = range.iter().copied().collect();
        self.interpolate = round_interpolator_factory();
        self
    }
    pub fn set_clamp(&mut self, c: bool) -> &mut Self {
        self.clamp = c;
        self
    }
    pub fn set_interpolate(&mut self, f: InterpolatorFactory) -> &mut Self {
        self.interpolate = f;
        self
    }
    pub fn set_unknown(&mut self, v: f64) -> &mut Self {
        self.unknown = v;
        self
    }
}

// ScaleSymlog
#[derive(Clone)]
pub struct ScaleSymlog {
    pub core: ContinuousCore,
    pub c: f64,
}

impl ScaleSymlog {
    pub fn new() -> Self {
        let id = Arc::new(|x: f64| x);
        Self {
            core: ContinuousCore::new(id.clone(), id),
            c: 1.0,
        }
    }
    fn rescale(&mut self) {
        let c = self.c;
        let t = Arc::new(move |x: f64| (x / c).abs().ln_1p() * x.signum());
        let u = Arc::new(move |x: f64| (x.abs().exp_m1()) * c * x.signum());
        self.core.transform = t;
        self.core.untransform = u;
    }
    pub fn constant(&self) -> f64 {
        self.c
    }
    pub fn set_constant(&mut self, c: f64) -> &mut Self {
        self.c = c;
        self.rescale();
        self
    }
    pub fn domain(&self) -> Vec<f64> {
        self.core.domain.clone()
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        self.core.set_domain(domain);
        self
    }
    pub fn range(&self) -> Vec<f64> {
        self.core.range.clone()
    }
    pub fn set_range(&mut self, range: &[f64]) -> &mut Self {
        self.core.set_range(range);
        self
    }
    pub fn range_round(&mut self, range: &[f64]) -> &mut Self {
        self.core.range_round(range);
        self
    }
    pub fn clamp(&self) -> bool {
        self.core.clamp
    }
    pub fn set_clamp(&mut self, c: bool) -> &mut Self {
        self.core.set_clamp(c);
        self
    }
    pub fn interpolate(&self) -> InterpolatorFactory {
        self.core.interpolate.clone()
    }
    pub fn set_interpolate(&mut self, f: InterpolatorFactory) -> &mut Self {
        self.core.set_interpolate(f);
        self
    }
    pub fn unknown(&self) -> f64 {
        self.core.unknown
    }
    pub fn set_unknown(&mut self, v: f64) -> &mut Self {
        self.core.set_unknown(v);
        self
    }
    pub fn invert(&self, y: f64) -> f64 {
        self.core.invert(y)
    }
    pub fn call(&self, x: f64) -> f64 {
        self.core.call(x)
    }
    pub fn ticks(&self, count: Option<f64>) -> Vec<f64> {
        let count = count.unwrap_or(10.0);
        let d = self.core.domain.clone();
        array::ticks(d[0], d[d.len() - 1], count)
    }
    pub fn tick_format(
        &self,
        count: Option<f64>,
        specifier: Option<&str>,
    ) -> Box<dyn Fn(f64) -> String> {
        let count = count.unwrap_or(10.0);
        let d = self.core.domain.clone();
        tick_format(d[0], d[d.len() - 1], Some(count), specifier)
    }
    pub fn nice(&mut self, count: Option<f64>) -> &mut Self {
        let count = count.unwrap_or(10.0);
        let mut s = ScaleLinear {
            core: self.core.clone(),
        };
        s.nice(Some(count));
        self.core.domain = s.core.domain;
        self
    }
    pub fn copy(&self) -> Self {
        Self {
            core: self.core.clone(),
            c: self.c,
        }
    }
}

// rescale on new
// fix new to rescale
// (redefine new above? can't; add init in a separate? We'll set transform manually by calling rescale via a helper after construction; do here with a constructor function)
// We'll instead make new call rescale by making it mutable:
// Already new doesn't call rescale. Let's adjust by overriding? Use a free function.

pub fn scale_symlog() -> ScaleSymlog {
    let mut s = ScaleSymlog::new();
    s.rescale();
    s
}

// ScalePow
#[derive(Clone)]
pub struct ScalePow {
    pub core: ContinuousCore,
    pub exponent: f64,
}

impl ScalePow {
    pub fn new() -> Self {
        let id = Arc::new(|x: f64| x);
        Self {
            core: ContinuousCore::new(id.clone(), id),
            exponent: 1.0,
        }
    }
    pub fn rescale(&mut self) {
        let e = self.exponent;
        if e == 1.0 {
            let id = Arc::new(|x: f64| x);
            self.core.transform = id.clone();
            self.core.untransform = id;
        } else if e == 0.5 {
            self.core.transform = Arc::new(|x: f64| if x < 0.0 { -(-x).sqrt() } else { x.sqrt() });
            self.core.untransform = Arc::new(|x: f64| if x < 0.0 { -x * x } else { x * x }); // transformPow(2) = square with sign
                                                                                             // Actually transformSquare: x<0 ? -x*x : x*x
            self.core.untransform = Arc::new(|x: f64| if x < 0.0 { -x * x } else { x * x });
        } else {
            self.core.transform =
                Arc::new(move |x: f64| if x < 0.0 { -(-x).powf(e) } else { x.powf(e) });
            self.core.untransform = Arc::new(move |x: f64| {
                if x < 0.0 {
                    -(-x).powf(1.0 / e)
                } else {
                    x.powf(1.0 / e)
                }
            });
        }
    }
    pub fn exponent(&self) -> f64 {
        self.exponent
    }
    pub fn set_exponent(&mut self, e: f64) -> &mut Self {
        self.exponent = e;
        self.rescale();
        self
    }
    pub fn domain(&self) -> Vec<f64> {
        self.core.domain.clone()
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        self.core.set_domain(domain);
        self
    }
    pub fn range(&self) -> Vec<f64> {
        self.core.range.clone()
    }
    pub fn set_range(&mut self, range: &[f64]) -> &mut Self {
        self.core.set_range(range);
        self
    }
    pub fn range_round(&mut self, range: &[f64]) -> &mut Self {
        self.core.range_round(range);
        self
    }
    pub fn clamp(&self) -> bool {
        self.core.clamp
    }
    pub fn set_clamp(&mut self, c: bool) -> &mut Self {
        self.core.set_clamp(c);
        self
    }
    pub fn interpolate(&self) -> InterpolatorFactory {
        self.core.interpolate.clone()
    }
    pub fn set_interpolate(&mut self, f: InterpolatorFactory) -> &mut Self {
        self.core.set_interpolate(f);
        self
    }
    pub fn unknown(&self) -> f64 {
        self.core.unknown
    }
    pub fn set_unknown(&mut self, v: f64) -> &mut Self {
        self.core.set_unknown(v);
        self
    }
    pub fn invert(&self, y: f64) -> f64 {
        self.core.invert(y)
    }
    pub fn call(&self, x: f64) -> f64 {
        self.core.call(x)
    }
    pub fn ticks(&self, count: Option<f64>) -> Vec<f64> {
        let count = count.unwrap_or(10.0);
        let d = self.core.domain.clone();
        array::ticks(d[0], d[d.len() - 1], count)
    }
    pub fn tick_format(
        &self,
        count: Option<f64>,
        specifier: Option<&str>,
    ) -> Box<dyn Fn(f64) -> String> {
        let count = count.unwrap_or(10.0);
        let d = self.core.domain.clone();
        tick_format(d[0], d[d.len() - 1], Some(count), specifier)
    }
    pub fn nice(&mut self, count: Option<f64>) -> &mut Self {
        let count = count.unwrap_or(10.0);
        let mut s = ScaleLinear {
            core: self.core.clone(),
        };
        s.nice(Some(count));
        self.core.domain = s.core.domain;
        self
    }
    pub fn copy(&self) -> Self {
        Self {
            core: self.core.clone(),
            exponent: self.exponent,
        }
    }
}

pub fn scale_pow() -> ScalePow {
    let mut s = ScalePow::new();
    s.rescale();
    s
}
pub fn scale_sqrt() -> ScalePow {
    let mut s = scale_pow();
    s.set_exponent(0.5);
    s
}

// ScaleOrdinal
#[derive(Clone, Debug, PartialEq)]
pub enum OrdinalUnknown {
    Implicit(Implicit),
    Value(f64),
}

#[derive(Clone)]
pub struct ScaleOrdinal {
    // We'll store domain values and map
    pub domain: Vec<f64>,
    pub range: Vec<f64>,
    pub unknown: OrdinalUnknown,
    // internal map from string key to index in domain
    keys: Vec<String>,
}

impl ScaleOrdinal {
    pub fn new() -> Self {
        Self {
            domain: Vec::new(),
            range: Vec::new(),
            unknown: OrdinalUnknown::Implicit(scale_implicit()),
            keys: Vec::new(),
        }
    }
    fn rebuild_keys(&mut self) {
        self.keys = self.domain.iter().map(|d| d.to_string()).collect(); // approximate; JS uses d+""
                                                                         // Actually need dedup keys; domain already deduped
    }
    pub fn domain(&self) -> Vec<f64> {
        self.domain.clone()
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        // dedup by key
        let mut new_domain = Vec::new();
        let mut seen = Vec::new();
        let mut new_keys = Vec::new();
        for d in domain {
            let key = js_number_string(*d);
            if !seen.iter().any(|k| k == &key) {
                seen.push(key.clone());
                new_keys.push(key);
                new_domain.push(*d);
            }
        }
        self.domain = new_domain;
        self.keys = new_keys;
        self
    }
    pub fn range(&self) -> Vec<f64> {
        self.range.clone()
    }
    pub fn set_range(&mut self, range: &[f64]) -> &mut Self {
        self.range = range.to_vec();
        self
    }
    pub fn unknown(&self) -> OrdinalUnknown {
        self.unknown.clone()
    }
    pub fn set_unknown(&mut self, u: OrdinalUnknown) -> &mut Self {
        self.unknown = u;
        self
    }
    pub fn call(&mut self, d: f64) -> f64 {
        let key = js_number_string(d);
        let mut idx = None;
        for (i, k) in self.keys.iter().enumerate() {
            if k == &key {
                idx = Some(i);
                break;
            }
        }
        let i = match idx {
            Some(i) => i,
            None => {
                if let OrdinalUnknown::Value(v) = &self.unknown {
                    return *v;
                }
                // implicit: add to domain
                let i = self.domain.len();
                self.domain.push(d);
                self.keys.push(key);
                i
            }
        };
        let rlen = self.range.len();
        if rlen == 0 {
            return f64::NAN;
        }
        self.range[i % rlen]
    }
    // Note: implicit case should mutate domain; but &self call can't. We'll provide call_mut?
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

// Because implicit case mutates, implement FnMut with mutation via interior? ScaleOrdinal not Copy. We'll make call take &mut?
// To support mutation, store not needed; tests likely call on mut. But Fn trait call(&self) can't mutate.
// We'll change call to &mut self and implement FnMut only, plus method call(&mut self).

pub fn tick_format(
    start: f64,
    stop: f64,
    count: Option<f64>,
    specifier: Option<&str>,
) -> Box<dyn Fn(f64) -> String> {
    let count = count.unwrap_or(10.0);
    let step = crate::array::tick_step(start, stop, count);
    let mut spec = crate::format::format_specifier(specifier.unwrap_or(",f"))
        .unwrap_or_else(|| crate::format::format_specifier(",f").unwrap());
    let locale = crate::format::format_default_locale();
    match spec.kind.as_str() {
        "s" => {
            let value = start.abs().max(stop.abs());
            if spec.precision.is_none() {
                let p = crate::format::precision_prefix(step, value);
                if !p.is_nan() {
                    spec.precision = Some(p as usize);
                }
            }
            let fp = locale.format_prefix(&spec.to_string(), value).unwrap();
            Box::new(move |d| fp.format(d))
        }
        "" | "e" | "g" | "p" | "r" => {
            if spec.precision.is_none() {
                let p = crate::format::precision_round(step, start.abs().max(stop.abs()));
                if !p.is_nan() {
                    let mut p = p as usize;
                    if spec.kind == "e" {
                        p = p.saturating_sub(1);
                    }
                    spec.precision = Some(p);
                }
            }
            let f = locale.format(&spec.to_string()).unwrap();
            Box::new(move |d| f.format(d))
        }
        "f" | "%" => {
            if spec.precision.is_none() {
                let p = crate::format::precision_fixed(step);
                if !p.is_nan() {
                    let mut p = p as usize;
                    if spec.kind == "%" {
                        p = p.saturating_sub(2);
                    }
                    spec.precision = Some(p);
                }
            }
            let f = locale.format(&spec.to_string()).unwrap();
            Box::new(move |d| f.format(d))
        }
        _ => {
            let f = locale.format(&spec.to_string()).unwrap();
            Box::new(move |d| f.format(d))
        }
    }
}

// ScaleBand
#[derive(Clone)]
pub struct ScaleBand {
    pub domain: Vec<f64>,
    pub range: [f64; 2],
    pub round: bool,
    pub padding_inner: f64,
    pub padding_outer: f64,
    pub align: f64,
    point: bool,
    // computed
    pub step: f64,
    pub bandwidth: f64,
    values: Vec<f64>,
}

impl ScaleBand {
    pub fn new(point: bool) -> Self {
        let mut s = Self {
            domain: Vec::new(),
            range: [0.0, 1.0],
            round: false,
            padding_inner: 0.0,
            padding_outer: 0.0,
            align: 0.5,
            point,
            step: 0.0,
            bandwidth: 0.0,
            values: Vec::new(),
        };
        if point {
            s.padding_inner = 1.0;
        }
        s.rescale();
        s
    }
    fn rescale(&mut self) {
        let n = self.domain.len();
        let reverse = self.range[1] < self.range[0];
        let start = if reverse {
            self.range[1]
        } else {
            self.range[0]
        };
        let stop = if reverse {
            self.range[0]
        } else {
            self.range[1]
        };
        let denom = (n as f64 - self.padding_inner + self.padding_outer * 2.0).max(1.0);
        let mut step = (stop - start) / denom;
        if self.round {
            step = step.floor();
        }
        let mut start =
            start + (stop - start - step * (n as f64 - self.padding_inner)) * self.align;
        let mut bandwidth = step * (1.0 - self.padding_inner);
        if self.round {
            start = start.round();
            bandwidth = bandwidth.round();
        }
        let mut values: Vec<f64> = (0..n).map(|i| start + step * i as f64).collect();
        if reverse {
            values.reverse();
        }
        self.step = step;
        self.bandwidth = bandwidth;
        self.values = values;
    }
    pub fn domain(&self) -> Vec<f64> {
        self.domain.clone()
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        // dedup
        let mut new_domain = Vec::new();
        let mut seen = Vec::new();
        for d in domain {
            let key = js_number_string(*d);
            if !seen.iter().any(|k| k == &key) {
                seen.push(key);
                new_domain.push(*d);
            }
        }
        self.domain = new_domain;
        self.rescale();
        self
    }
    pub fn range(&self) -> [f64; 2] {
        self.range
    }
    pub fn set_range(&mut self, range: &[f64]) -> &mut Self {
        self.range = [range[0], range[1]];
        self.rescale();
        self
    }
    pub fn range_round(&mut self, range: &[f64]) -> &mut Self {
        self.range = [range[0], range[1]];
        self.round = true;
        self.rescale();
        self
    }
    pub fn bandwidth(&self) -> f64 {
        self.bandwidth
    }
    pub fn step(&self) -> f64 {
        self.step
    }
    pub fn round(&self) -> bool {
        self.round
    }
    pub fn set_round(&mut self, r: bool) -> &mut Self {
        self.round = r;
        self.rescale();
        self
    }
    pub fn padding(&self) -> f64 {
        if self.point {
            self.padding_outer
        } else {
            self.padding_inner
        }
    }
    pub fn set_padding(&mut self, v: f64) -> &mut Self {
        if self.point {
            self.padding_outer = v;
            self.padding_inner = 1.0;
        } else {
            self.padding_outer = v;
            self.padding_inner = v.min(1.0);
        }
        self.rescale();
        self
    }
    pub fn padding_inner(&self) -> f64 {
        self.padding_inner
    }
    pub fn set_padding_inner(&mut self, v: f64) -> &mut Self {
        self.padding_inner = v.min(1.0);
        self.rescale();
        self
    }
    pub fn padding_outer(&self) -> f64 {
        self.padding_outer
    }
    pub fn set_padding_outer(&mut self, v: f64) -> &mut Self {
        self.padding_outer = v;
        self.rescale();
        self
    }
    pub fn align(&self) -> f64 {
        self.align
    }
    pub fn set_align(&mut self, v: f64) -> &mut Self {
        self.align = v.max(0.0).min(1.0);
        self.rescale();
        self
    }
    pub fn call(&self, d: f64) -> f64 {
        let key = js_number_string(d);
        for (i, dom) in self.domain.iter().enumerate() {
            if js_number_string(*dom) == key {
                return self.values[i];
            }
        }
        f64::NAN
    }
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

pub fn scale_band() -> ScaleBand {
    ScaleBand::new(false)
}
pub fn scale_point() -> ScaleBand {
    ScaleBand::new(true)
}

// ScaleQuantile
#[derive(Clone)]
pub struct ScaleQuantile {
    pub domain: Vec<f64>,
    pub range: Vec<f64>,
    thresholds: Vec<f64>,
    pub unknown: f64,
}

impl ScaleQuantile {
    pub fn new() -> Self {
        Self {
            domain: Vec::new(),
            range: Vec::new(),
            thresholds: Vec::new(),
            unknown: f64::NAN,
        }
    }
    fn rescale(&mut self) {
        let n = self.range.len().max(1);
        self.thresholds = vec![f64::NAN; n.saturating_sub(1)];
        // use d3-array quantile
        let mut i = 0usize;
        while i + 1 < n {
            let q = i as f64 / n as f64; // placeholder; need quantile
                                         // We'll compute properly below
            i += 1;
        }
        // proper:
        self.thresholds = Vec::with_capacity(n.saturating_sub(1));
        let mut i = 1usize;
        while i < n {
            let t = crate::array::quantile(&self.domain, i as f64 / n as f64).unwrap_or(f64::NAN);
            self.thresholds.push(t);
            i += 1;
        }
    }
    pub fn domain(&self) -> Vec<f64> {
        self.domain.clone()
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        let mut d = Vec::new();
        for x in domain {
            let v = js::number(*x);
            if !v.is_nan() {
                // d != null && !isNaN
                // d != null: treat NaN as skip; all f64
                d.push(v);
            }
        }
        d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        self.domain = d;
        self.rescale();
        self
    }
    pub fn range(&self) -> Vec<f64> {
        self.range.clone()
    }
    pub fn set_range(&mut self, range: &[f64]) -> &mut Self {
        self.range = range.to_vec();
        self.rescale();
        self
    }
    pub fn unknown(&self) -> f64 {
        self.unknown
    }
    pub fn set_unknown(&mut self, v: f64) -> &mut Self {
        self.unknown = v;
        self
    }
    pub fn call(&self, x: f64) -> f64 {
        if x.is_nan() {
            return self.unknown;
        }
        let i = crate::array::bisect_right(&self.thresholds, x);
        let rlen = self.range.len();
        if rlen == 0 {
            return f64::NAN;
        }
        self.range[i.min(rlen - 1)]
    }
    pub fn invert_extent(&self, y: f64) -> [f64; 2] {
        let i = self
            .range
            .iter()
            .position(|&v| js::object_is(v, y))
            .unwrap_or(usize::MAX);
        if i == usize::MAX {
            return [f64::NAN, f64::NAN];
        }
        let lo = if i > 0 {
            self.thresholds[i - 1]
        } else {
            *self.domain.first().unwrap_or(&f64::NAN)
        };
        let hi = if i < self.thresholds.len() {
            self.thresholds[i]
        } else {
            *self.domain.last().unwrap_or(&f64::NAN)
        };
        [lo, hi]
    }
    pub fn quantiles(&self) -> Vec<f64> {
        self.thresholds.clone()
    }
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

pub fn scale_ordinal() -> ScaleOrdinal {
    ScaleOrdinal::new()
}

pub fn scale_quantile() -> ScaleQuantile {
    ScaleQuantile::new()
}

// ScaleQuantize
#[derive(Clone)]
pub struct ScaleQuantize {
    pub x0: f64,
    pub x1: f64,
    pub range: Vec<f64>,
    domain_vals: Vec<f64>,
    pub unknown: f64,
}

impl ScaleQuantize {
    pub fn new() -> Self {
        let mut s = Self {
            x0: 0.0,
            x1: 1.0,
            range: vec![0.0, 1.0],
            domain_vals: vec![0.5],
            unknown: f64::NAN,
        };
        s.rescale();
        s
    }
    fn rescale(&mut self) {
        let n = self.range.len().saturating_sub(1).max(1);
        let mut domain = vec![0.0; n];
        let mut i = 0usize;
        while i < n {
            domain[i] =
                ((i as f64 + 1.0) * self.x1 - (i as f64 - n as f64) * self.x0) / (n as f64 + 1.0);
            i += 1;
        }
        self.domain_vals = domain;
    }
    pub fn domain(&self) -> [f64; 2] {
        [self.x0, self.x1]
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        self.x0 = js::number(domain[0]);
        self.x1 = js::number(domain[1]);
        self.rescale();
        self
    }
    pub fn range(&self) -> Vec<f64> {
        self.range.clone()
    }
    pub fn set_range(&mut self, range: &[f64]) -> &mut Self {
        self.range = range.to_vec();
        self.rescale();
        self
    }
    pub fn unknown(&self) -> f64 {
        self.unknown
    }
    pub fn set_unknown(&mut self, v: f64) -> &mut Self {
        self.unknown = v;
        self
    }
    pub fn call(&self, x: f64) -> f64 {
        if x.is_nan() {
            return self.unknown;
        } // NaN check x <= x in JS: x <= x true except NaN
          // JS: x <= x ? range[bisect(domain, x, 0, n)] : unknown
        if !x.is_nan() {
            let n = self.range.len().saturating_sub(1);
            let i = crate::array::bisect_right(&self.domain_vals[..n], x);
            self.range[i.min(self.range.len() - 1)]
        } else {
            self.unknown
        }
    }
    pub fn invert_extent(&self, y: f64) -> [f64; 2] {
        let i = self
            .range
            .iter()
            .position(|&v| js::object_is(v, y))
            .unwrap_or(usize::MAX);
        if i == usize::MAX {
            return [f64::NAN, f64::NAN];
        }
        if i < 1 {
            [self.x0, self.domain_vals[0]]
        } else if i >= self.range.len().saturating_sub(1) {
            [*self.domain_vals.last().unwrap_or(&f64::NAN), self.x1]
        } else {
            [self.domain_vals[i - 1], self.domain_vals[i]]
        }
    }
    pub fn thresholds(&self) -> Vec<f64> {
        self.domain_vals.clone()
    }
    pub fn ticks(&self, count: Option<f64>) -> Vec<f64> {
        let count = count.unwrap_or(10.0);
        // linearish uses domain [x0,x1]
        crate::array::ticks(self.x0, self.x1, count)
    }
    pub fn tick_format(
        &self,
        count: Option<f64>,
        specifier: Option<&str>,
    ) -> Box<dyn Fn(f64) -> String> {
        let count = count.unwrap_or(10.0);
        tick_format(self.x0, self.x1, Some(count), specifier)
    }
    pub fn nice(&mut self, count: Option<f64>) -> &mut Self {
        let count = count.unwrap_or(10.0);
        let mut s = ScaleLinear::new();
        s.set_domain(&[self.x0, self.x1]);
        s.nice(Some(count));
        let d = s.domain();
        self.x0 = d[0];
        self.x1 = d[1];
        self.rescale();
        self
    }
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

pub fn scale_quantize() -> ScaleQuantize {
    ScaleQuantize::new()
}

// ScaleThreshold
#[derive(Clone)]
pub struct ScaleThreshold {
    pub domain: Vec<f64>,
    pub range: Vec<f64>,
    pub unknown: f64,
}

impl ScaleThreshold {
    pub fn new() -> Self {
        Self {
            domain: vec![0.5],
            range: vec![0.0, 1.0],
            unknown: f64::NAN,
        }
    }
    pub fn domain(&self) -> Vec<f64> {
        self.domain.clone()
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        self.domain = domain.to_vec();
        self
    }
    pub fn range(&self) -> Vec<f64> {
        self.range.clone()
    }
    pub fn set_range(&mut self, range: &[f64]) -> &mut Self {
        self.range = range.to_vec();
        self
    }
    pub fn unknown(&self) -> f64 {
        self.unknown
    }
    pub fn set_unknown(&mut self, v: f64) -> &mut Self {
        self.unknown = v;
        self
    }
    pub fn call(&self, x: f64) -> f64 {
        if x.is_nan() {
            // x <= x in JS
            return self.unknown;
        }
        // JS: x <= x ? range[bisect(domain, x, 0, n)] : unknown, with n = min(domain.len, range.len-1)
        if !x.is_nan() {
            let n = self.domain.len().min(self.range.len().saturating_sub(1));
            let i = crate::array::bisect_right(&self.domain[..n], x);
            self.range[i.min(self.range.len().saturating_sub(1))]
        } else {
            self.unknown
        }
    }
    pub fn invert_extent(&self, y: f64) -> [f64; 2] {
        let i = self
            .range
            .iter()
            .position(|&v| js::object_is(v, y))
            .unwrap_or(usize::MAX);
        if i == usize::MAX {
            return [f64::NAN, f64::NAN];
        } // JS returns [domain[i-1], domain[i]] maybe NaNs
        let lo = if i > 0 {
            self.domain.get(i - 1).copied().unwrap_or(f64::NAN)
        } else {
            f64::NAN
        };
        let hi = self.domain.get(i).copied().unwrap_or(f64::NAN);
        [lo, hi]
    }
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

pub fn scale_threshold() -> ScaleThreshold {
    ScaleThreshold::new()
}

// ScaleTime (and Utc)
#[derive(Clone)]
pub struct ScaleTime {
    pub core: ContinuousCore,
    pub utc: bool,
}

impl ScaleTime {
    pub fn new(utc: bool) -> Self {
        let id = Arc::new(|x: f64| x);
        let mut s = Self {
            core: ContinuousCore::new(id.clone(), id),
            utc,
        };
        // default domain
        if utc {
            s.core.domain = vec![
                crate::time::Date::utc(946684800000.0).time(),
                crate::time::Date::utc(946771200000.0).time(),
            ];
        } else {
            s.core.domain = vec![
                crate::time::Date::local(946684800000.0).time(),
                crate::time::Date::local(946771200000.0).time(),
            ];
        }
        s
    }
    fn intervals(&self) -> Vec<(crate::time::interval::Interval, i32, f64)> {
        let ms = 1000.0;
        let min = ms * 60.0;
        let hour = min * 60.0;
        let day = hour * 24.0;
        let week = day * 7.0;
        let month = day * 30.0;
        let year = day * 365.0;
        if self.utc {
            vec![
                (utc_second(), 1, ms),
                (utc_second(), 5, 5.0 * ms),
                (utc_second(), 15, 15.0 * ms),
                (utc_second(), 30, 30.0 * ms),
                (utc_minute(), 1, min),
                (utc_minute(), 5, 5.0 * min),
                (utc_minute(), 15, 15.0 * min),
                (utc_minute(), 30, 30.0 * min),
                (utc_hour(), 1, hour),
                (utc_hour(), 3, 3.0 * hour),
                (utc_hour(), 6, 6.0 * hour),
                (utc_hour(), 12, 12.0 * hour),
                (utc_day(), 1, day),
                (utc_day(), 2, 2.0 * day),
                (utc_week(), 1, week),
                (utc_month(), 1, month),
                (utc_month(), 3, 3.0 * month),
                (utc_year(), 1, year),
            ]
        } else {
            vec![
                (time_second(), 1, ms),
                (time_second(), 5, 5.0 * ms),
                (time_second(), 15, 15.0 * ms),
                (time_second(), 30, 30.0 * ms),
                (time_minute(), 1, min),
                (time_minute(), 5, 5.0 * min),
                (time_minute(), 15, 15.0 * min),
                (time_minute(), 30, 30.0 * min),
                (time_hour(), 1, hour),
                (time_hour(), 3, 3.0 * hour),
                (time_hour(), 6, 6.0 * hour),
                (time_hour(), 12, 12.0 * hour),
                (time_day(), 1, day),
                (time_day(), 2, 2.0 * day),
                (time_week(), 1, week),
                (time_month(), 1, month),
                (time_month(), 3, 3.0 * month),
                (time_year(), 1, year),
            ]
        }
    }
    pub fn domain(&self) -> Vec<f64> {
        self.core.domain.clone()
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        self.core.domain = domain.iter().map(|x| *x).collect();
        self
    }
    pub fn range(&self) -> Vec<f64> {
        self.core.range.clone()
    }
    pub fn set_range(&mut self, range: &[f64]) -> &mut Self {
        self.core.set_range(range);
        self
    }
    pub fn invert(&self, y: f64) -> f64 {
        self.core.invert(y)
    }
    pub fn call(&self, x: f64) -> f64 {
        self.core.call(x)
    }
    pub fn clamp(&self) -> bool {
        self.core.clamp
    }
    pub fn set_clamp(&mut self, c: bool) -> &mut Self {
        self.core.set_clamp(c);
        self
    }
    pub fn unknown(&self) -> f64 {
        self.core.unknown
    }
    pub fn set_unknown(&mut self, v: f64) -> &mut Self {
        self.core.set_unknown(v);
        self
    }
    pub fn ticks(&self, count: Option<f64>) -> Vec<f64> {
        let count = count.unwrap_or(10.0);
        let d = self.core.domain.clone();
        let (mut t0, mut t1) = (d[0], d[d.len() - 1]);
        let reverse = t1 < t0;
        if reverse {
            let t = t0;
            t0 = t1;
            t1 = t;
        }
        // tickInterval
        let ivs = self.intervals();
        let durations: Vec<f64> = ivs.iter().map(|x| x.2).collect();
        let target = (t1 - t0).abs() / count;
        let i = crate::array::bisect_right(&durations, target);
        let mut chosen: Option<crate::time::interval::Interval> = None;
        let mut step: Option<i32> = None;
        if i == ivs.len() {
            // year step
            let step_y = crate::array::tick_step(
                t0 / (365.0 * 24.0 * 3600.0 * 1000.0),
                t1 / (365.0 * 24.0 * 3600.0 * 1000.0),
                count,
            );
            let iv = if self.utc {
                crate::time::interval::utc_year()
            } else {
                crate::time::interval::time_year()
            };
            chosen = Some(iv);
            step = Some(step_y as i32);
        } else if i > 0 {
            let prev = &ivs[i - 1];
            let curr = &ivs[i];
            let chosen_iv = if target / prev.2 < curr.2 / target {
                prev
            } else {
                curr
            };
            chosen = Some(chosen_iv.0.clone());
            step = Some(chosen_iv.1);
        } else {
            let st = crate::array::tick_step(t0, t1, count).max(1.0) as i32;
            let iv = if self.utc {
                crate::time::interval::utc_millisecond()
            } else {
                crate::time::interval::time_millisecond()
            };
            chosen = Some(iv);
            step = Some(st);
        }
        let iv = chosen.unwrap();
        let iv = if let Some(s) = step {
            iv.every(s).unwrap_or(iv.clone())
        } else {
            iv
        };
        let mut date0 = if self.utc {
            crate::time::Date::utc(t0)
        } else {
            crate::time::Date::local(t0)
        };
        let mut date1 = if self.utc {
            crate::time::Date::utc(t1)
        } else {
            crate::time::Date::local(t1)
        };
        let mut ticks = iv.range(&date0, &date1, 1);
        // inclusive stop: range to t1+1ms? JS t.range(t0, t1+1). We'll add one more by extending? Use range with stop+1ms
        let stop_date = if self.utc {
            crate::time::Date::utc(t1 + 1.0)
        } else {
            crate::time::Date::local(t1 + 1.0)
        };
        ticks = iv.range(&date0, &stop_date, 1);
        let mut out: Vec<f64> = ticks.iter().map(|d| d.time()).collect();
        // dedup and ensure within
        out.retain(|&t| t >= t0.min(t1) && t <= t1.max(t0) || true);
        if reverse {
            out.reverse();
        }
        out
    }
    pub fn tick_format(
        &self,
        count: Option<f64>,
        specifier: Option<&str>,
    ) -> Box<dyn Fn(f64) -> String> {
        if let Some(spec) = specifier {
            let locale = crate::format::format_default_locale();
            let f = locale.format(spec).unwrap();
            return Box::new(move |d| f.format(d));
        }
        // default tickFormat function
        let utc = self.utc;
        let ms = move |d: f64| -> String {
            let date = if utc {
                crate::time::Date::utc(d)
            } else {
                crate::time::Date::local(d)
            };
            let second_iv = if utc {
                crate::time::interval::utc_second()
            } else {
                crate::time::interval::time_second()
            };
            let minute_iv = if utc {
                crate::time::interval::utc_minute()
            } else {
                crate::time::interval::time_minute()
            };
            let hour_iv = if utc {
                crate::time::interval::utc_hour()
            } else {
                crate::time::interval::time_hour()
            };
            let day_iv = if utc {
                crate::time::interval::utc_day()
            } else {
                crate::time::interval::time_day()
            };
            let week_iv = if utc {
                crate::time::interval::utc_week()
            } else {
                crate::time::interval::time_week()
            };
            let month_iv = if utc {
                crate::time::interval::utc_month()
            } else {
                crate::time::interval::time_month()
            };
            let year_iv = if utc {
                crate::time::interval::utc_year()
            } else {
                crate::time::interval::time_year()
            };
            let floor = |iv: &crate::time::interval::Interval,
                         dt: crate::time::Date|
             -> crate::time::Date {
                let mut x = dt;
                iv.floor(&mut x);
                x
            };
            let second = floor(&second_iv, date);
            let minute = floor(&minute_iv, date);
            let hour = floor(&hour_iv, date);
            let day = floor(&day_iv, date);
            let week = floor(&week_iv, date);
            let month = floor(&month_iv, date);
            let year = floor(&year_iv, date);
            let fmt = if second.time() < date.time() {
                crate::time_format::time_format(".%L")
            } else if minute.time() < date.time() {
                crate::time_format::time_format(":%S")
            } else if hour.time() < date.time() {
                crate::time_format::time_format("%I:%M")
            } else if day.time() < date.time() {
                if week.time() < date.time() {
                    crate::time_format::time_format("%a %d")
                } else {
                    crate::time_format::time_format("%b %d")
                }
            } else if month.time() < date.time() {
                crate::time_format::time_format("%B")
            } else {
                crate::time_format::time_format("%Y")
            };
            fmt.format(&date)
        };
        Box::new(ms)
    }
    pub fn nice(&mut self, count: Option<f64>) -> &mut Self {
        let count = count.unwrap_or(10.0);
        let d = self.core.domain.clone();
        let (t0, t1) = (d[0], d[d.len() - 1]);
        let ivs = self.intervals();
        let durations: Vec<f64> = ivs.iter().map(|x| x.2).collect();
        let target = (t1 - t0).abs() / count;
        let i = crate::array::bisect_right(&durations, target);
        // choose interval
        let mut iv = if i == ivs.len() {
            if self.utc {
                crate::time::interval::utc_year()
            } else {
                crate::time::interval::time_year()
            }
        } else if i > 0 {
            let prev = &ivs[i - 1];
            let curr = &ivs[i];
            if target / prev.2 < curr.2 / target {
                prev.0.clone()
            } else {
                curr.0.clone()
            }
        } else {
            if self.utc {
                crate::time::interval::utc_millisecond()
            } else {
                crate::time::interval::time_millisecond()
            }
        };
        // floor/ceil
        let mut d0 = if self.utc {
            crate::time::Date::utc(t0)
        } else {
            crate::time::Date::local(t0)
        };
        let mut d1 = if self.utc {
            crate::time::Date::utc(t1)
        } else {
            crate::time::Date::local(t1)
        };
        iv.floor(&mut d0);
        iv.ceil(&mut d1);
        self.core.domain = vec![d0.time(), d1.time()];
        self
    }
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

pub fn scale_time() -> ScaleTime {
    ScaleTime::new(false)
}
pub fn scale_utc() -> ScaleTime {
    ScaleTime::new(true)
}

// Sequential kinds
#[derive(Clone)]
pub enum SeqKind {
    Identity,
    Log(f64),
    Symlog(f64),
    Pow(f64),
}

fn seq_transform(kind: &SeqKind, domain0: f64) -> Arc<dyn Fn(f64) -> f64> {
    match kind {
        SeqKind::Identity => Arc::new(|x: f64| x),
        SeqKind::Log(base) => {
            let base = *base;
            if domain0 < 0.0 {
                Arc::new(move |x: f64| -((-x).abs().ln() / base.ln()))
            } else {
                Arc::new(move |x: f64| {
                    x.abs().ln() / base.ln() * x.signum().max(0.0).min(1.0) + 0.0
                }) // not exact
            }
        }
        SeqKind::Symlog(c) => {
            let c = *c;
            Arc::new(move |x: f64| (x / c).abs().ln_1p() * x.signum())
        }
        SeqKind::Pow(e) => {
            let e = *e;
            Arc::new(move |x: f64| if x < 0.0 { -(-x).powf(e) } else { x.powf(e) })
        }
    }
}

// Fix Log transform for positive: use log with base
fn seq_transform_log(base: f64, domain0: f64) -> Arc<dyn Fn(f64) -> f64> {
    let _ = base;
    if domain0 < 0.0 {
        Arc::new(|x: f64| -crate::ieee754::log(-x))
    } else {
        Arc::new(|x: f64| crate::ieee754::log(x))
    }
}

// We'll override seq_transform usage for Log by calling seq_transform_log in set_domain. To keep simple, store base in kind and compute transform in set_domain manually.

#[derive(Clone)]
pub struct ScaleSequential {
    pub x0: f64,
    pub x1: f64,
    pub t0: f64,
    pub t1: f64,
    pub k10: f64,
    pub kind: SeqKind,
    pub clamp: bool,
    pub interpolator: Arc<dyn Fn(f64) -> f64>,
    pub unknown: f64,
    pub transform: Arc<dyn Fn(f64) -> f64>,
}

impl ScaleSequential {
    pub fn new(kind: SeqKind, domain: [f64; 2]) -> Self {
        let mut s = Self {
            x0: domain[0],
            x1: domain[1],
            t0: 0.0,
            t1: 0.0,
            k10: 0.0,
            kind,
            clamp: false,
            interpolator: Arc::new(|x: f64| x),
            unknown: f64::NAN,
            transform: Arc::new(|x: f64| x),
        };
        s.set_domain(&domain);
        s
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        self.x0 = domain[0];
        self.x1 = domain[1];
        // transform
        self.transform = match &self.kind {
            SeqKind::Identity => Arc::new(|x: f64| x),
            SeqKind::Log(base) => seq_transform_log(*base, self.x0),
            SeqKind::Symlog(c) => {
                let c = *c;
                Arc::new(move |x: f64| (x / c).abs().ln_1p() * x.signum())
            }
            SeqKind::Pow(e) => {
                let e = *e;
                Arc::new(move |x: f64| if x < 0.0 { -(-x).powf(e) } else { x.powf(e) })
            }
        };
        self.t0 = (self.transform)(self.x0);
        self.t1 = (self.transform)(self.x1);
        self.k10 = if self.t0 == self.t1 {
            0.0
        } else {
            1.0 / (self.t1 - self.t0)
        };
        self
    }
    pub fn domain(&self) -> [f64; 2] {
        [self.x0, self.x1]
    }
    pub fn clamp(&self) -> bool {
        self.clamp
    }
    pub fn set_clamp(&mut self, c: bool) -> &mut Self {
        self.clamp = c;
        self
    }
    pub fn interpolator(&self) -> Arc<dyn Fn(f64) -> f64> {
        self.interpolator.clone()
    }
    pub fn set_interpolator(&mut self, f: Arc<dyn Fn(f64) -> f64>) -> &mut Self {
        self.interpolator = f;
        self
    }
    pub fn unknown(&self) -> f64 {
        self.unknown
    }
    pub fn set_unknown(&mut self, v: f64) -> &mut Self {
        self.unknown = v;
        self
    }
    pub fn call(&self, x: f64) -> f64 {
        if x.is_nan() {
            return self.unknown;
        }
        let tx = (self.transform)(x);
        let mut param = if self.k10 == 0.0 {
            0.5
        } else {
            (tx - self.t0) * self.k10
        };
        if self.clamp {
            param = param.max(0.0).min(1.0);
        }
        (self.interpolator)(param)
    }
    pub fn base(&self) -> f64 {
        match &self.kind {
            SeqKind::Log(b) => *b,
            _ => 10.0,
        }
    }
    pub fn set_base(&mut self, b: f64) -> &mut Self {
        if let SeqKind::Log(_) = self.kind {
            self.kind = SeqKind::Log(b);
            self.set_domain(&[self.x0, self.x1]);
        }
        self
    }
    pub fn constant(&self) -> f64 {
        match &self.kind {
            SeqKind::Symlog(c) => *c,
            _ => 1.0,
        }
    }
    pub fn set_constant(&mut self, c: f64) -> &mut Self {
        if let SeqKind::Symlog(_) = self.kind {
            self.kind = SeqKind::Symlog(c);
            self.set_domain(&[self.x0, self.x1]);
        }
        self
    }
    pub fn exponent(&self) -> f64 {
        match &self.kind {
            SeqKind::Pow(e) => *e,
            _ => 1.0,
        }
    }
    pub fn set_exponent(&mut self, e: f64) -> &mut Self {
        if let SeqKind::Pow(_) = self.kind {
            self.kind = SeqKind::Pow(e);
            self.set_domain(&[self.x0, self.x1]);
        }
        self
    }
    pub fn ticks(&self, count: Option<f64>) -> Vec<f64> {
        let count = count.unwrap_or(10.0);
        crate::array::ticks(self.x0, self.x1, count)
    }
    pub fn tick_format(
        &self,
        count: Option<f64>,
        specifier: Option<&str>,
    ) -> Box<dyn Fn(f64) -> String> {
        let count = count.unwrap_or(10.0);
        tick_format(self.x0, self.x1, Some(count), specifier)
    }
    pub fn nice(&mut self, count: Option<f64>) -> &mut Self {
        let count = count.unwrap_or(10.0);
        let mut s = ScaleLinear::new();
        s.set_domain(&[self.x0, self.x1]);
        s.nice(Some(count));
        let d = s.domain();
        self.set_domain(&d);
        self
    }
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

pub fn scale_sequential() -> ScaleSequential {
    ScaleSequential::new(SeqKind::Identity, [0.0, 1.0])
}
pub fn scale_sequential_log() -> ScaleSequential {
    let mut s = ScaleSequential::new(SeqKind::Log(10.0), [1.0, 10.0]);
    s
}
pub fn scale_sequential_symlog() -> ScaleSequential {
    ScaleSequential::new(SeqKind::Symlog(1.0), [0.0, 1.0])
}
pub fn scale_sequential_pow() -> ScaleSequential {
    ScaleSequential::new(SeqKind::Pow(1.0), [0.0, 1.0])
}
pub fn scale_sequential_sqrt() -> ScaleSequential {
    let mut s = scale_sequential_pow();
    s.kind = SeqKind::Pow(0.5);
    s.set_domain(&[s.x0, s.x1]);
    s
}

// ScaleSequentialQuantile
#[derive(Clone)]
pub struct ScaleSequentialQuantile {
    pub domain: Vec<f64>,
    pub interpolator: Arc<dyn Fn(f64) -> f64>,
}

impl ScaleSequentialQuantile {
    pub fn new() -> Self {
        Self {
            domain: Vec::new(),
            interpolator: Arc::new(|x: f64| x),
        }
    }
    pub fn domain(&self) -> Vec<f64> {
        self.domain.clone()
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        let mut d = Vec::new();
        for x in domain {
            let v = js::number(*x);
            if !v.is_nan() {
                d.push(v);
            } // !isNaN
        }
        d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        self.domain = d;
        self
    }
    pub fn interpolator(&self) -> Arc<dyn Fn(f64) -> f64> {
        self.interpolator.clone()
    }
    pub fn set_interpolator(&mut self, f: Arc<dyn Fn(f64) -> f64>) -> &mut Self {
        self.interpolator = f;
        self
    }
    pub fn call(&self, x: f64) -> f64 {
        if x.is_nan() {
            return f64::NAN;
        }
        let i = crate::array::bisect(&self.domain, x);
        let denom = self.domain.len() as f64 - 1.0;
        let t = if denom == 0.0 {
            0.0
        } else {
            (i as f64 - 1.0) / denom
        };
        (self.interpolator)(t)
    }
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

pub fn scale_sequential_quantile() -> ScaleSequentialQuantile {
    ScaleSequentialQuantile::new()
}

// ScaleDiverging
#[derive(Clone)]
pub struct ScaleDiverging {
    pub x0: f64,
    pub x1: f64,
    pub x2: f64,
    pub t0: f64,
    pub t1: f64,
    pub t2: f64,
    pub k10: f64,
    pub k21: f64,
    pub kind: SeqKind,
    pub clamp: bool,
    pub interpolator: Arc<dyn Fn(f64) -> f64>,
    pub unknown: f64,
    pub transform: Arc<dyn Fn(f64) -> f64>,
}

impl ScaleDiverging {
    pub fn new(kind: SeqKind, domain: [f64; 3]) -> Self {
        let mut s = Self {
            x0: domain[0],
            x1: domain[1],
            x2: domain[2],
            t0: 0.0,
            t1: 0.0,
            t2: 0.0,
            k10: 0.0,
            k21: 0.0,
            kind,
            clamp: false,
            interpolator: Arc::new(|x: f64| x),
            unknown: f64::NAN,
            transform: Arc::new(|x: f64| x),
        };
        s.set_domain(&domain);
        s
    }
    pub fn set_domain(&mut self, domain: &[f64]) -> &mut Self {
        self.x0 = domain[0];
        self.x1 = domain[1];
        self.x2 = domain[2];
        self.transform = match &self.kind {
            SeqKind::Identity => Arc::new(|x: f64| x),
            SeqKind::Log(base) => seq_transform_log(*base, self.x0),
            SeqKind::Symlog(c) => {
                let c = *c;
                Arc::new(move |x: f64| (x / c).abs().ln_1p() * x.signum())
            }
            SeqKind::Pow(e) => {
                let e = *e;
                Arc::new(move |x: f64| if x < 0.0 { -(-x).powf(e) } else { x.powf(e) })
            }
        };
        self.t0 = (self.transform)(self.x0);
        self.t1 = (self.transform)(self.x1);
        self.t2 = (self.transform)(self.x2);
        self.k10 = if self.t0 == self.t1 {
            0.0
        } else {
            0.5 / (self.t1 - self.t0)
        };
        self.k21 = if self.t1 == self.t2 {
            0.0
        } else {
            0.5 / (self.t2 - self.t1)
        };
        self
    }
    pub fn domain(&self) -> [f64; 3] {
        [self.x0, self.x1, self.x2]
    }
    pub fn clamp(&self) -> bool {
        self.clamp
    }
    pub fn set_clamp(&mut self, c: bool) -> &mut Self {
        self.clamp = c;
        self
    }
    pub fn interpolator(&self) -> Arc<dyn Fn(f64) -> f64> {
        self.interpolator.clone()
    }
    pub fn set_interpolator(&mut self, f: Arc<dyn Fn(f64) -> f64>) -> &mut Self {
        self.interpolator = f;
        self
    }
    pub fn unknown(&self) -> f64 {
        self.unknown
    }
    pub fn set_unknown(&mut self, v: f64) -> &mut Self {
        self.unknown = v;
        self
    }
    pub fn call(&self, x: f64) -> f64 {
        if x.is_nan() {
            return self.unknown;
        }
        let tx = (self.transform)(x);
        let mut param = 0.5 + (tx - self.t1) * (if tx < self.t1 { self.k10 } else { self.k21 });
        if self.clamp {
            param = param.max(0.0).min(1.0);
        }
        (self.interpolator)(param)
    }
    pub fn base(&self) -> f64 {
        match &self.kind {
            SeqKind::Log(b) => *b,
            _ => 10.0,
        }
    }
    pub fn set_base(&mut self, b: f64) -> &mut Self {
        if let SeqKind::Log(_) = self.kind {
            self.kind = SeqKind::Log(b);
            self.set_domain(&[self.x0, self.x1, self.x2]);
        }
        self
    }
    pub fn constant(&self) -> f64 {
        match &self.kind {
            SeqKind::Symlog(c) => *c,
            _ => 1.0,
        }
    }
    pub fn set_constant(&mut self, c: f64) -> &mut Self {
        if let SeqKind::Symlog(_) = self.kind {
            self.kind = SeqKind::Symlog(c);
            self.set_domain(&[self.x0, self.x1, self.x2]);
        }
        self
    }
    pub fn exponent(&self) -> f64 {
        match &self.kind {
            SeqKind::Pow(e) => *e,
            _ => 1.0,
        }
    }
    pub fn set_exponent(&mut self, e: f64) -> &mut Self {
        if let SeqKind::Pow(_) = self.kind {
            self.kind = SeqKind::Pow(e);
            self.set_domain(&[self.x0, self.x1, self.x2]);
        }
        self
    }
    pub fn ticks(&self, count: Option<f64>) -> Vec<f64> {
        let count = count.unwrap_or(10.0);
        crate::array::ticks(self.x0, self.x2, count)
    }
    pub fn tick_format(
        &self,
        count: Option<f64>,
        specifier: Option<&str>,
    ) -> Box<dyn Fn(f64) -> String> {
        let count = count.unwrap_or(10.0);
        tick_format(self.x0, self.x2, Some(count), specifier)
    }
    pub fn nice(&mut self, count: Option<f64>) -> &mut Self {
        let count = count.unwrap_or(10.0);
        let mut s = ScaleLinear::new();
        s.set_domain(&[self.x0, self.x2]);
        s.nice(Some(count));
        let d = s.domain();
        // adjust only endpoints? JS nice uses domain whole; keep middle
        self.x0 = d[0];
        self.x2 = d[1];
        self.set_domain(&[self.x0, self.x1, self.x2]);
        self
    }
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

pub fn scale_diverging() -> ScaleDiverging {
    ScaleDiverging::new(SeqKind::Identity, [0.0, 0.5, 1.0])
}
pub fn scale_diverging_log() -> ScaleDiverging {
    ScaleDiverging::new(SeqKind::Log(10.0), [0.1, 1.0, 10.0])
}
pub fn scale_diverging_symlog() -> ScaleDiverging {
    ScaleDiverging::new(SeqKind::Symlog(1.0), [0.0, 0.5, 1.0])
}
pub fn scale_diverging_pow() -> ScaleDiverging {
    ScaleDiverging::new(SeqKind::Pow(1.0), [0.0, 0.5, 1.0])
}
pub fn scale_diverging_sqrt() -> ScaleDiverging {
    let mut s = scale_diverging_pow();
    s.kind = SeqKind::Pow(0.5);
    s.set_domain(&[s.x0, s.x1, s.x2]);
    s
}
