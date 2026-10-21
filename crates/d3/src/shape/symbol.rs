//! d3-shape's symbols, from the JavaScript of the same name.
//!
//! Translated from `d3-shape/src/symbol.js` and `d3-shape/src/symbol/*.js`,
//! version 1.3.7, of d3 5.16.0. The JavaScript is the authority.

use crate::ieee754;

use super::Context;
use crate::path::Path;

/// Twice pi.
const TAU: f64 = std::f64::consts::PI * 2.0;

/// The shape of a symbol, which draws itself.
pub trait SymbolType {
    /// The JavaScript's `draw`.
    fn draw(&self, context: &mut dyn Context, size: f64);
}

/// d3-shape's `symbolCircle`.
pub struct SymbolCircle;

impl SymbolType for SymbolCircle {
    fn draw(&self, context: &mut dyn Context, size: f64) {
        let r = (size / std::f64::consts::PI).sqrt();
        context.move_to(r, 0.0);
        // The radius is a square root, so it is never negative and
        // the error cannot happen.
        let _ = context.arc(0.0, 0.0, r, 0.0, TAU, false);
    }
}

/// d3-shape's `symbolCross`.
pub struct SymbolCross;

impl SymbolType for SymbolCross {
    fn draw(&self, context: &mut dyn Context, size: f64) {
        let r = (size / 5.0).sqrt() / 2.0;
        context.move_to(-3.0 * r, -r);
        context.line_to(-r, -r);
        context.line_to(-r, -3.0 * r);
        context.line_to(r, -3.0 * r);
        context.line_to(r, -r);
        context.line_to(3.0 * r, -r);
        context.line_to(3.0 * r, r);
        context.line_to(r, r);
        context.line_to(r, 3.0 * r);
        context.line_to(-r, 3.0 * r);
        context.line_to(-r, r);
        context.line_to(-3.0 * r, r);
        context.close_path();
    }
}

/// d3-shape's `symbolDiamond`.
pub struct SymbolDiamond;

impl SymbolType for SymbolDiamond {
    fn draw(&self, context: &mut dyn Context, size: f64) {
        let tan30 = (1.0f64 / 3.0).sqrt();
        let tan30_2 = tan30 * 2.0;
        let y = (size / tan30_2).sqrt();
        let x = y * tan30;
        context.move_to(0.0, -y);
        context.line_to(x, 0.0);
        context.line_to(0.0, y);
        context.line_to(-x, 0.0);
        context.close_path();
    }
}

/// d3-shape's `symbolSquare`.
pub struct SymbolSquare;

impl SymbolType for SymbolSquare {
    fn draw(&self, context: &mut dyn Context, size: f64) {
        let w = size.sqrt();
        let x = -w / 2.0;
        context.rect(x, x, w, w);
    }
}

/// d3-shape's `symbolStar`.
pub struct SymbolStar;

impl SymbolType for SymbolStar {
    fn draw(&self, context: &mut dyn Context, size: f64) {
        // The JavaScript's own constant, written out as it is there. The
        // extra digits are the JavaScript's, and the port keeps them.
        #[allow(clippy::excessive_precision)]
        let ka = 0.89081309152928522810;
        let kr = ieee754::sin(std::f64::consts::PI / 10.0)
            / ieee754::sin(7.0 * std::f64::consts::PI / 10.0);
        let kx = ieee754::sin(TAU / 10.0) * kr;
        let ky = -ieee754::cos(TAU / 10.0) * kr;
        let r = (size * ka).sqrt();
        let x = kx * r;
        let y = ky * r;
        context.move_to(0.0, -r);
        context.line_to(x, y);
        for i in 1..5 {
            let a = TAU * i as f64 / 5.0;
            let c = ieee754::cos(a);
            let s = ieee754::sin(a);
            context.line_to(s * r, -c * r);
            context.line_to(c * x - s * y, s * x + c * y);
        }
        context.close_path();
    }
}

/// d3-shape's `symbolTriangle`.
pub struct SymbolTriangle;

impl SymbolType for SymbolTriangle {
    fn draw(&self, context: &mut dyn Context, size: f64) {
        let sqrt3 = 3.0f64.sqrt();
        let y = -(size / (sqrt3 * 3.0)).sqrt();
        context.move_to(0.0, y * 2.0);
        context.line_to(-sqrt3 * y, -y);
        context.line_to(sqrt3 * y, -y);
        context.close_path();
    }
}

/// d3-shape's `symbolWye`.
pub struct SymbolWye;

impl SymbolType for SymbolWye {
    fn draw(&self, context: &mut dyn Context, size: f64) {
        let c = -0.5;
        let s = 3.0f64.sqrt() / 2.0;
        let k = 1.0 / 12.0f64.sqrt();
        let a = (k / 2.0 + 1.0) * 3.0;
        let r = (size / a).sqrt();
        let x0 = r / 2.0;
        let y0 = r * k;
        let x1 = x0;
        let y1 = r * k + r;
        let x2 = -x1;
        let y2 = y1;
        context.move_to(x0, y0);
        context.line_to(x1, y1);
        context.line_to(x2, y2);
        context.line_to(c * x0 - s * y0, s * x0 + c * y0);
        context.line_to(c * x1 - s * y1, s * x1 + c * y1);
        context.line_to(c * x2 - s * y2, s * x2 + c * y2);
        context.line_to(c * x0 + s * y0, c * y0 - s * x0);
        context.line_to(c * x1 + s * y1, c * y1 - s * x1);
        context.line_to(c * x2 + s * y2, c * y2 - s * x2);
        context.close_path();
    }
}

/// d3-shape's `symbol`.
///
/// The JavaScript's `symbol.type` takes either a function or a symbol,
/// and the port's takes a symbol or a function that gives one.
pub struct Symbol {
    kind: Box<dyn Fn() -> Box<dyn SymbolType>>,
    size: Box<dyn Fn() -> f64>,
}

impl Default for Symbol {
    fn default() -> Self {
        Self {
            kind: Box::new(|| Box::new(SymbolCircle)),
            size: Box::new(|| 64.0),
        }
    }
}

impl Symbol {
    /// The JavaScript's `symbol.type`.
    pub fn kind<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Box<dyn SymbolType> + 'static,
    {
        self.kind = Box::new(f);
        self
    }

    /// The JavaScript's `symbol.size`.
    pub fn size<F>(mut self, f: F) -> Self
    where
        F: Fn() -> f64 + 'static,
    {
        self.size = Box::new(f);
        self
    }

    /// The JavaScript's `symbol()`: the path of the symbol.
    pub fn apply(&self) -> Option<String> {
        let mut buffer = Path::default();
        (self.kind)().draw(&mut buffer, (self.size)());
        let text = buffer.text().to_owned();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// The JavaScript's `symbol()` drawn into a context the caller holds.
    pub fn apply_to(&self, context: &mut dyn Context) {
        (self.kind)().draw(context, (self.size)());
    }
}

/// d3-shape's `symbolCircle`.
pub fn symbol_circle() -> Box<dyn SymbolType> {
    Box::new(SymbolCircle)
}

/// d3-shape's `symbolCross`.
pub fn symbol_cross() -> Box<dyn SymbolType> {
    Box::new(SymbolCross)
}

/// d3-shape's `symbolDiamond`.
pub fn symbol_diamond() -> Box<dyn SymbolType> {
    Box::new(SymbolDiamond)
}

/// d3-shape's `symbolSquare`.
pub fn symbol_square() -> Box<dyn SymbolType> {
    Box::new(SymbolSquare)
}

/// d3-shape's `symbolStar`.
pub fn symbol_star() -> Box<dyn SymbolType> {
    Box::new(SymbolStar)
}

/// d3-shape's `symbolTriangle`.
pub fn symbol_triangle() -> Box<dyn SymbolType> {
    Box::new(SymbolTriangle)
}

/// d3-shape's `symbolWye`.
pub fn symbol_wye() -> Box<dyn SymbolType> {
    Box::new(SymbolWye)
}
