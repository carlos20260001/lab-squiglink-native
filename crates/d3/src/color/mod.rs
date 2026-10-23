//! A port of **d3-color**, from the JavaScript of the same name.
//!
//! Translated from `d3-color/src/color.js`, version 1.4.1, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read
//! differently, the JavaScript is right.
//!
//! The colours are the JavaScript's own: a colour is a number of
//! channels and an opacity, and the spaces between them are the
//! JavaScript's — `rgb`, `hsl`, `lab` and `hcl` — with the conversions
//! between them.
//!
//! The JavaScript's own parser is written with patterns, and the port
//! writes them out by hand, for the crate has no dependency and the
//! patterns are simple.

pub mod cubehelix;
pub mod lab;

// The JavaScript's own functions, which are the modules' own.
pub use cubehelix::cubehelix;
pub use lab::{gray, hcl, lab, lch};

use crate::arith;
use crate::js;

/// The darker a colour is made by `darker`.
pub const DARKER: f64 = 0.7;
/// The brighter a colour is made by `brighter`.
pub const BRIGHTER: f64 = 1.0 / DARKER;

/// A colour, in one of the JavaScript's own spaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Color {
    /// The JavaScript's `Rgb`.
    Rgb(Rgb),
    /// The JavaScript's `Hsl`.
    Hsl(Hsl),
    /// The JavaScript's `Lab`.
    Lab(Lab),
    /// The JavaScript's `Hcl`.
    Hcl(Hcl),
    /// The JavaScript's `Cubehelix`.
    Cubehelix(Cubehelix),
}

/// The JavaScript's `Rgb`, which is a colour of red, green and blue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    /// The red, from 0 to 255.
    pub r: f64,
    /// The green, from 0 to 255.
    pub g: f64,
    /// The blue, from 0 to 255.
    pub b: f64,
    /// The opacity, from 0 to 1.
    pub opacity: f64,
}

/// The JavaScript's `Hsl`, which is a colour of hue, saturation and
/// lightness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsl {
    /// The hue, in degrees.
    pub h: f64,
    /// The saturation, from 0 to 1.
    pub s: f64,
    /// The lightness, from 0 to 1.
    pub l: f64,
    /// The opacity, from 0 to 1.
    pub opacity: f64,
}

/// The JavaScript's `Lab`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lab {
    /// The lightness.
    pub l: f64,
    /// The green-red axis.
    pub a: f64,
    /// The blue-yellow axis.
    pub b: f64,
    /// The opacity, from 0 to 1.
    pub opacity: f64,
}

/// The JavaScript's `Hcl`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hcl {
    /// The hue, in degrees.
    pub h: f64,
    /// The chroma.
    pub c: f64,
    /// The lightness.
    pub l: f64,
    /// The opacity, from 0 to 1.
    pub opacity: f64,
}

/// The JavaScript's `Cubehelix`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cubehelix {
    /// The hue, in degrees.
    pub h: f64,
    /// The saturation.
    pub s: f64,
    /// The lightness.
    pub l: f64,
    /// The opacity, from 0 to 1.
    pub opacity: f64,
}

impl Color {
    /// The colour as an `Rgb`, which is what every colour can be made.
    pub fn rgb(&self) -> Rgb {
        match self {
            Color::Rgb(rgb) => *rgb,
            Color::Hsl(hsl) => hsl.rgb(),
            Color::Lab(lab) => lab.rgb(),
            Color::Hcl(hcl) => hcl.rgb(),
            Color::Cubehelix(cubehelix) => cubehelix.rgb(),
        }
    }

    /// The colour as an `Hsl`.
    pub fn hsl(&self) -> Hsl {
        match self {
            Color::Hsl(hsl) => *hsl,
            _ => hsl_convert(*self),
        }
    }

    /// The colour as a `Lab`.
    pub fn lab(&self) -> Lab {
        match self {
            Color::Lab(lab) => *lab,
            _ => lab::lab_convert(*self),
        }
    }

    /// The colour as an `Hcl`.
    pub fn hcl(&self) -> Hcl {
        match self {
            Color::Hcl(hcl) => *hcl,
            _ => lab::hcl_convert(*self),
        }
    }

    /// The colour as a `Cubehelix`.
    pub fn cubehelix(&self) -> Cubehelix {
        match self {
            Color::Cubehelix(cubehelix) => *cubehelix,
            _ => cubehelix::cubehelix_convert(*self),
        }
    }

    /// Whether the colour can be shown, which the JavaScript's
    /// `displayable` says.
    pub fn displayable(&self) -> bool {
        match self {
            Color::Rgb(rgb) => rgb.displayable(),
            Color::Hsl(hsl) => hsl.displayable(),
            _ => self.rgb().displayable(),
        }
    }

    /// The colour a little brighter, which the JavaScript's `brighter`
    /// gives.
    pub fn brighter(&self, k: Option<f64>) -> Color {
        let k = k.unwrap_or(BRIGHTER);
        match self {
            Color::Rgb(rgb) => Color::Rgb(rgb.brighter(Some(k))),
            Color::Hsl(hsl) => Color::Hsl(hsl.brighter(Some(k))),
            Color::Lab(lab) => Color::Lab(lab.brighter(Some(k))),
            Color::Hcl(hcl) => Color::Hcl(hcl.brighter(Some(k))),
            Color::Cubehelix(cubehelix) => Color::Cubehelix(cubehelix.brighter(Some(k))),
        }
    }

    /// The colour a little darker, which the JavaScript's `darker` gives.
    pub fn darker(&self, k: Option<f64>) -> Color {
        let k = k.unwrap_or(DARKER);
        match self {
            Color::Rgb(rgb) => Color::Rgb(rgb.darker(Some(k))),
            Color::Hsl(hsl) => Color::Hsl(hsl.darker(Some(k))),
            Color::Lab(lab) => Color::Lab(lab.darker(Some(k))),
            Color::Hcl(hcl) => Color::Hcl(hcl.darker(Some(k))),
            Color::Cubehelix(cubehelix) => Color::Cubehelix(cubehelix.darker(Some(k))),
        }
    }

    /// The colour written as the JavaScript writes it, which is the
    /// `rgb` form.
    pub fn format_rgb(&self) -> String {
        self.rgb().format_rgb()
    }

    /// The colour written as the JavaScript writes it, which is the
    /// `hsl` form.
    pub fn format_hsl(&self) -> String {
        self.hsl().format_hsl()
    }

    /// The colour written as the JavaScript writes it, which is the
    /// hexadecimal form.
    pub fn format_hex(&self) -> String {
        self.rgb().format_hex()
    }
}

impl Rgb {
    /// The colour a little brighter.
    pub fn brighter(&self, k: Option<f64>) -> Rgb {
        let k = k.unwrap_or(BRIGHTER);
        Rgb {
            r: arith::multiply(self.r, k),
            g: arith::multiply(self.g, k),
            b: arith::multiply(self.b, k),
            opacity: self.opacity,
        }
    }

    /// The colour a little darker.
    pub fn darker(&self, k: Option<f64>) -> Rgb {
        let k = k.unwrap_or(DARKER);
        Rgb {
            r: arith::multiply(self.r, k),
            g: arith::multiply(self.g, k),
            b: arith::multiply(self.b, k),
            opacity: self.opacity,
        }
    }

    /// Whether the colour can be shown.
    pub fn displayable(&self) -> bool {
        (-0.5..=255.4).contains(&self.r)
            && (-0.5..=255.4).contains(&self.g)
            && (-0.5..=255.4).contains(&self.b)
            && (0.0..=1.0).contains(&self.opacity)
    }

    /// The colour written as the JavaScript writes it, which is the
    /// hexadecimal form.
    pub fn format_hex(&self) -> String {
        format!("#{}{}{}", hex(self.r), hex(self.g), hex(self.b))
    }

    /// The colour written as the JavaScript writes it, which is the
    /// `rgb` form.
    pub fn format_rgb(&self) -> String {
        let a = if self.opacity.is_nan() {
            1.0
        } else {
            self.opacity.clamp(0.0, 1.0)
        };
        let mut out = String::new();
        if a == 1.0 {
            out.push_str("rgb(");
        } else {
            out.push_str("rgba(");
        }
        out.push_str(&js::number_to_string(channel(self.r)));
        out.push_str(", ");
        out.push_str(&js::number_to_string(channel(self.g)));
        out.push_str(", ");
        out.push_str(&js::number_to_string(channel(self.b)));
        if a != 1.0 {
            out.push_str(", ");
            out.push_str(&js::number_to_string(a));
        }
        out.push(')');
        out
    }
}

impl Hsl {
    /// The colour a little brighter.
    pub fn brighter(&self, k: Option<f64>) -> Hsl {
        let k = k.unwrap_or(BRIGHTER);
        Hsl {
            h: self.h,
            s: self.s,
            l: arith::multiply(self.l, k),
            opacity: self.opacity,
        }
    }

    /// The colour a little darker.
    pub fn darker(&self, k: Option<f64>) -> Hsl {
        let k = k.unwrap_or(DARKER);
        Hsl {
            h: self.h,
            s: self.s,
            l: arith::multiply(self.l, k),
            opacity: self.opacity,
        }
    }

    /// The colour as an `Rgb`.
    pub fn rgb(&self) -> Rgb {
        let h = arith::add(self.h % 360.0, if self.h < 0.0 { 360.0 } else { 0.0 });
        let s = if h.is_nan() || self.s.is_nan() {
            0.0
        } else {
            self.s
        };
        let l = self.l;
        let m2 = arith::add(
            l,
            arith::multiply(if l < 0.5 { l } else { arith::subtract(1.0, l) }, s),
        );
        let m1 = arith::subtract(arith::multiply(2.0, l), m2);
        Rgb {
            r: hsl2rgb(
                if h >= 240.0 {
                    arith::subtract(h, 240.0)
                } else {
                    arith::add(h, 120.0)
                },
                m1,
                m2,
            ),
            g: hsl2rgb(h, m1, m2),
            b: hsl2rgb(
                if h < 120.0 {
                    arith::add(h, 240.0)
                } else {
                    arith::subtract(h, 120.0)
                },
                m1,
                m2,
            ),
            opacity: self.opacity,
        }
    }

    /// Whether the colour can be shown.
    pub fn displayable(&self) -> bool {
        ((0.0..=1.0).contains(&self.s) || self.s.is_nan())
            && (0.0..=1.0).contains(&self.l)
            && (0.0..=1.0).contains(&self.opacity)
    }

    /// The colour written as the JavaScript writes it, which is the
    /// `hsl` form.
    pub fn format_hsl(&self) -> String {
        let a = if self.opacity.is_nan() {
            1.0
        } else {
            self.opacity.clamp(0.0, 1.0)
        };
        let mut out = String::new();
        if a == 1.0 {
            out.push_str("hsl(");
        } else {
            out.push_str("hsla(");
        }
        // The JavaScript writes `this.h || 0`, and the `||` is the
        // JavaScript's own, for a `NaN` is not true.
        out.push_str(&js::number_to_string(if js::truthy(self.h) {
            self.h
        } else {
            0.0
        }));
        out.push_str(", ");
        out.push_str(&js::number_to_string(arith::multiply(
            if js::truthy(self.s) { self.s } else { 0.0 },
            100.0,
        )));
        out.push_str("%, ");
        out.push_str(&js::number_to_string(arith::multiply(
            if js::truthy(self.l) { self.l } else { 0.0 },
            100.0,
        )));
        out.push('%');
        if a != 1.0 {
            out.push_str(", ");
            out.push_str(&js::number_to_string(a));
        }
        out.push(')');
        out
    }
}

/// The JavaScript's `color`, which reads a colour out of a string.
///
/// The JavaScript gives `null` for a string it cannot read, and the port
/// gives `None`.
pub fn color(format: &str) -> Option<Color> {
    let format = format.trim().to_lowercase();
    if let Some(rest) = format.strip_prefix('#') {
        return parse_hex(rest);
    }
    if let Some(rest) = format.strip_prefix("rgba(") {
        let rest = rest.strip_suffix(')')?;
        let parts = split_parts(rest);
        if parts.len() != 4 {
            return None;
        }
        let (r, g, b) = channels(parts[0], parts[1], parts[2])?;
        let a = parse_number(parts[3])?;
        return Some(Color::Rgb(rgba(r, g, b, a)));
    }
    if let Some(rest) = format.strip_prefix("rgb(") {
        let rest = rest.strip_suffix(')')?;
        let parts = split_parts(rest);
        if parts.len() != 3 {
            return None;
        }
        let (r, g, b) = channels(parts[0], parts[1], parts[2])?;
        return Some(Color::Rgb(Rgb {
            r,
            g,
            b,
            opacity: 1.0,
        }));
    }
    if let Some(rest) = format.strip_prefix("hsla(") {
        let rest = rest.strip_suffix(')')?;
        let parts = split_parts(rest);
        if parts.len() != 4 {
            return None;
        }
        let h = parse_number(parts[0])?;
        let s = parse_percent(parts[1])? / 100.0;
        let l = parse_percent(parts[2])? / 100.0;
        let a = parse_number(parts[3])?;
        return Some(Color::Hsl(hsla(h, s, l, a)));
    }
    if let Some(rest) = format.strip_prefix("hsl(") {
        let rest = rest.strip_suffix(')')?;
        let parts = split_parts(rest);
        if parts.len() != 3 {
            return None;
        }
        let h = parse_number(parts[0])?;
        let s = parse_percent(parts[1])? / 100.0;
        let l = parse_percent(parts[2])? / 100.0;
        return Some(Color::Hsl(Hsl {
            h,
            s,
            l,
            opacity: 1.0,
        }));
    }
    if format == "transparent" {
        return Some(Color::Rgb(Rgb {
            r: f64::NAN,
            g: f64::NAN,
            b: f64::NAN,
            opacity: 0.0,
        }));
    }
    named(&format)
}

/// The JavaScript's `rgb`, which makes a colour of red, green and blue.
pub fn rgb(r: f64, g: f64, b: f64, opacity: Option<f64>) -> Color {
    Color::Rgb(Rgb {
        r,
        g,
        b,
        opacity: opacity.unwrap_or(1.0),
    })
}

/// The JavaScript's `hsl`, which makes a colour of hue, saturation and
/// lightness.
pub fn hsl(h: f64, s: f64, l: f64, opacity: Option<f64>) -> Color {
    Color::Hsl(Hsl {
        h,
        s,
        l,
        opacity: opacity.unwrap_or(1.0),
    })
}

/// The JavaScript's `rgbConvert`, which makes an `Rgb` of any colour.
pub fn rgb_convert(color: &Color) -> Rgb {
    let rgb = color.rgb();
    Rgb {
        r: rgb.r,
        g: rgb.g,
        b: rgb.b,
        opacity: rgb.opacity,
    }
}

/// The JavaScript's `hslConvert`, which makes an `Hsl` of any colour.
pub fn hsl_convert(color: Color) -> Hsl {
    if let Color::Hsl(hsl) = color {
        return hsl;
    }
    let rgb = color.rgb();
    let r = arith::divide(rgb.r, 255.0);
    let g = arith::divide(rgb.g, 255.0);
    let b = arith::divide(rgb.b, 255.0);
    let min = js_min(js_min(r, g), b);
    let max = js_max(js_max(r, g), b);
    let mut h = f64::NAN;
    let mut s = arith::subtract(max, min);
    let l = arith::divide(arith::add(max, min), 2.0);
    if s != 0.0 {
        if r == max {
            h = arith::add(
                arith::divide(arith::subtract(g, b), s),
                if g < b { 6.0 } else { 0.0 },
            );
        } else if g == max {
            h = arith::add(arith::divide(arith::subtract(b, r), s), 2.0);
        } else {
            h = arith::add(arith::divide(arith::subtract(r, g), s), 4.0);
        }
        s = arith::divide(
            s,
            if l < 0.5 {
                arith::add(max, min)
            } else {
                arith::subtract(arith::subtract(2.0, max), min)
            },
        );
        h = arith::multiply(h, 60.0);
    } else {
        s = if l > 0.0 && l < 1.0 { 0.0 } else { h };
    }
    Hsl {
        h,
        s,
        l,
        opacity: rgb.opacity,
    }
}

/// The JavaScript's `rgba`, which makes a colour that is transparent
/// when its opacity is not above zero.
fn rgba(r: f64, g: f64, b: f64, a: f64) -> Rgb {
    if a <= 0.0 {
        return Rgb {
            r: f64::NAN,
            g: f64::NAN,
            b: f64::NAN,
            opacity: a,
        };
    }
    Rgb {
        r,
        g,
        b,
        opacity: a,
    }
}

/// The JavaScript's `hsla`, which makes a colour that is transparent
/// when its opacity is not above zero.
fn hsla(h: f64, s: f64, l: f64, a: f64) -> Hsl {
    if a <= 0.0 {
        return Hsl {
            h: f64::NAN,
            s: f64::NAN,
            l: f64::NAN,
            opacity: a,
        };
    }
    if l <= 0.0 || l >= 1.0 {
        return Hsl {
            h: f64::NAN,
            s: f64::NAN,
            l,
            opacity: a,
        };
    }
    if s <= 0.0 {
        return Hsl {
            h: f64::NAN,
            s,
            l,
            opacity: a,
        };
    }
    Hsl {
        h,
        s,
        l,
        opacity: a,
    }
}

/// The JavaScript's `hex`, which writes a channel as two hexadecimal
/// digits.
fn hex(value: f64) -> String {
    let value = value.round();
    let value = if value.is_nan() { 0.0 } else { value };
    let value = value.clamp(0.0, 255.0);
    let text = format!("{:x}", value as u64);
    if value < 16.0 {
        format!("0{}", text)
    } else {
        text
    }
}

/// The JavaScript's channel, which is a whole number from 0 to 255.
fn channel(value: f64) -> f64 {
    let value = value.round();
    let value = if value.is_nan() { 0.0 } else { value };
    value.clamp(0.0, 255.0)
}

/// The JavaScript's `hsl2rgb`, which is from the CSS Color Module.
fn hsl2rgb(h: f64, m1: f64, m2: f64) -> f64 {
    let value = if h < 60.0 {
        arith::add(
            m1,
            arith::divide(arith::multiply(arith::subtract(m2, m1), h), 60.0),
        )
    } else if h < 180.0 {
        m2
    } else if h < 240.0 {
        arith::add(
            m1,
            arith::divide(
                arith::multiply(arith::subtract(m2, m1), arith::subtract(240.0, h)),
                60.0,
            ),
        )
    } else {
        m1
    };
    arith::multiply(value, 255.0)
}

/// The JavaScript's `Math.min`, which gives a number that is not a
/// number when either of its arguments is one.
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b {
        a
    } else {
        b
    }
}

/// The JavaScript's `Math.max`, which gives a number that is not a
/// number when either of its arguments is one.
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else {
        b
    }
}

/// The hexadecimal form, which the JavaScript reads with its own
/// pattern.
fn parse_hex(digits: &str) -> Option<Color> {
    if !(3..=8).contains(&digits.len()) {
        return None;
    }
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = u64::from_str_radix(digits, 16).ok()?;
    match digits.len() {
        6 => Some(Color::Rgb(Rgb {
            r: ((n >> 16) & 0xff) as f64,
            g: ((n >> 8) & 0xff) as f64,
            b: (n & 0xff) as f64,
            opacity: 1.0,
        })),
        3 => Some(Color::Rgb(Rgb {
            r: (((n >> 8) & 0xf) | ((n >> 4) & 0xf0)) as f64,
            g: (((n >> 4) & 0xf) | (n & 0xf0)) as f64,
            b: (((n & 0xf) << 4) | (n & 0xf)) as f64,
            opacity: 1.0,
        })),
        8 => Some(Color::Rgb(rgba(
            ((n >> 24) & 0xff) as f64,
            ((n >> 16) & 0xff) as f64,
            ((n >> 8) & 0xff) as f64,
            (n & 0xff) as f64 / 0xff as f64,
        ))),
        4 => Some(Color::Rgb(rgba(
            (((n >> 12) & 0xf) | ((n >> 8) & 0xf0)) as f64,
            (((n >> 8) & 0xf) | ((n >> 4) & 0xf0)) as f64,
            (((n >> 4) & 0xf) | (n & 0xf0)) as f64,
            (((n & 0xf) << 4) | (n & 0xf)) as f64 / 0xff as f64,
        ))),
        _ => None,
    }
}

/// The JavaScript's channels, which are either whole numbers or
/// percentages of the whole.
///
/// The JavaScript has a pattern for each, and a colour that mixes the
/// two is not a colour it can read.
fn channels(r: &str, g: &str, b: &str) -> Option<(f64, f64, f64)> {
    if let (Some(r), Some(g), Some(b)) = (parse_integer(r), parse_integer(g), parse_integer(b)) {
        return Some((r, g, b));
    }
    if let (Some(r), Some(g), Some(b)) = (parse_percent(r), parse_percent(g), parse_percent(b)) {
        return Some((
            arith::divide(arith::multiply(r, 255.0), 100.0),
            arith::divide(arith::multiply(g, 255.0), 100.0),
            arith::divide(arith::multiply(b, 255.0), 100.0),
        ));
    }
    None
}

/// The JavaScript's whole number, which the JavaScript's own pattern
/// holds.
fn parse_integer(text: &str) -> Option<f64> {
    let text = text.trim();
    let bytes = text.as_bytes();
    let mut at = 0;
    if bytes[at] == b'+' || bytes[at] == b'-' {
        at += 1;
    }
    let start = at;
    while at < bytes.len() && bytes[at].is_ascii_digit() {
        at += 1;
    }
    if at == start || at != bytes.len() {
        return None;
    }
    text.parse::<f64>().ok()
}

/// The parts of a colour's arguments, which are separated by commas.
fn split_parts(text: &str) -> Vec<&str> {
    text.split(',').map(str::trim).collect()
}

/// The JavaScript's number, which is a whole number or a decimal with
/// an exponent.
fn parse_number(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let bytes = text.as_bytes();
    let mut at = 0;
    if bytes[at] == b'+' || bytes[at] == b'-' {
        at += 1;
    }
    let start = at;
    while at < bytes.len() && bytes[at].is_ascii_digit() {
        at += 1;
    }
    if at == start {
        return None;
    }
    if at < bytes.len() && bytes[at] == b'.' {
        at += 1;
        let start = at;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            at += 1;
        }
        if at == start {
            return None;
        }
    }
    if at < bytes.len() && (bytes[at] == b'e' || bytes[at] == b'E') {
        at += 1;
        if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {
            at += 1;
        }
        let start = at;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            at += 1;
        }
        if at == start {
            return None;
        }
    }
    if at != bytes.len() {
        return None;
    }
    text.parse::<f64>().ok()
}

/// The JavaScript's percentage, which is a number and a `%`, and which
/// is a part of the whole.
fn parse_percent(text: &str) -> Option<f64> {
    let text = text.trim();
    let number = text.strip_suffix('%')?;
    parse_number(number)
}

/// The colours the JavaScript names.
fn named(name: &str) -> Option<Color> {
    let n = match name {
        "aliceblue" => 0xf0f8ff,
        "antiquewhite" => 0xfaebd7,
        "aqua" => 0x00ffff,
        "aquamarine" => 0x7fffd4,
        "azure" => 0xf0ffff,
        "beige" => 0xf5f5dc,
        "bisque" => 0xffe4c4,
        "black" => 0x000000,
        "blanchedalmond" => 0xffebcd,
        "blue" => 0x0000ff,
        "blueviolet" => 0x8a2be2,
        "brown" => 0xa52a2a,
        "burlywood" => 0xdeb887,
        "cadetblue" => 0x5f9ea0,
        "chartreuse" => 0x7fff00,
        "chocolate" => 0xd2691e,
        "coral" => 0xff7f50,
        "cornflowerblue" => 0x6495ed,
        "cornsilk" => 0xfff8dc,
        "crimson" => 0xdc143c,
        "cyan" => 0x00ffff,
        "darkblue" => 0x00008b,
        "darkcyan" => 0x008b8b,
        "darkgoldenrod" => 0xb8860b,
        "darkgray" => 0xa9a9a9,
        "darkgreen" => 0x006400,
        "darkgrey" => 0xa9a9a9,
        "darkkhaki" => 0xbdb76b,
        "darkmagenta" => 0x8b008b,
        "darkolivegreen" => 0x556b2f,
        "darkorange" => 0xff8c00,
        "darkorchid" => 0x9932cc,
        "darkred" => 0x8b0000,
        "darksalmon" => 0xe9967a,
        "darkseagreen" => 0x8fbc8f,
        "darkslateblue" => 0x483d8b,
        "darkslategray" => 0x2f4f4f,
        "darkslategrey" => 0x2f4f4f,
        "darkturquoise" => 0x00ced1,
        "darkviolet" => 0x9400d3,
        "deeppink" => 0xff1493,
        "deepskyblue" => 0x00bfff,
        "dimgray" => 0x696969,
        "dimgrey" => 0x696969,
        "dodgerblue" => 0x1e90ff,
        "firebrick" => 0xb22222,
        "floralwhite" => 0xfffaf0,
        "forestgreen" => 0x228b22,
        "fuchsia" => 0xff00ff,
        "gainsboro" => 0xdcdcdc,
        "ghostwhite" => 0xf8f8ff,
        "gold" => 0xffd700,
        "goldenrod" => 0xdaa520,
        "gray" => 0x808080,
        "green" => 0x008000,
        "greenyellow" => 0xadff2f,
        "grey" => 0x808080,
        "honeydew" => 0xf0fff0,
        "hotpink" => 0xff69b4,
        "indianred" => 0xcd5c5c,
        "indigo" => 0x4b0082,
        "ivory" => 0xfffff0,
        "khaki" => 0xf0e68c,
        "lavender" => 0xe6e6fa,
        "lavenderblush" => 0xfff0f5,
        "lawngreen" => 0x7cfc00,
        "lemonchiffon" => 0xfffacd,
        "lightblue" => 0xadd8e6,
        "lightcoral" => 0xf08080,
        "lightcyan" => 0xe0ffff,
        "lightgoldenrodyellow" => 0xfafad2,
        "lightgray" => 0xd3d3d3,
        "lightgreen" => 0x90ee90,
        "lightgrey" => 0xd3d3d3,
        "lightpink" => 0xffb6c1,
        "lightsalmon" => 0xffa07a,
        "lightseagreen" => 0x20b2aa,
        "lightskyblue" => 0x87cefa,
        "lightslategray" => 0x778899,
        "lightslategrey" => 0x778899,
        "lightsteelblue" => 0xb0c4de,
        "lightyellow" => 0xffffe0,
        "lime" => 0x00ff00,
        "limegreen" => 0x32cd32,
        "linen" => 0xfaf0e6,
        "magenta" => 0xff00ff,
        "maroon" => 0x800000,
        "mediumaquamarine" => 0x66cdaa,
        "mediumblue" => 0x0000cd,
        "mediumorchid" => 0xba55d3,
        "mediumpurple" => 0x9370db,
        "mediumseagreen" => 0x3cb371,
        "mediumslateblue" => 0x7b68ee,
        "mediumspringgreen" => 0x00fa9a,
        "mediumturquoise" => 0x48d1cc,
        "mediumvioletred" => 0xc71585,
        "midnightblue" => 0x191970,
        "mintcream" => 0xf5fffa,
        "mistyrose" => 0xffe4e1,
        "moccasin" => 0xffe4b5,
        "navajowhite" => 0xffdead,
        "navy" => 0x000080,
        "oldlace" => 0xfdf5e6,
        "olive" => 0x808000,
        "olivedrab" => 0x6b8e23,
        "orange" => 0xffa500,
        "orangered" => 0xff4500,
        "orchid" => 0xda70d6,
        "palegoldenrod" => 0xeee8aa,
        "palegreen" => 0x98fb98,
        "paleturquoise" => 0xafeeee,
        "palevioletred" => 0xdb7093,
        "papayawhip" => 0xffefd5,
        "peachpuff" => 0xffdab9,
        "peru" => 0xcd853f,
        "pink" => 0xffc0cb,
        "plum" => 0xdda0dd,
        "powderblue" => 0xb0e0e6,
        "purple" => 0x800080,
        "rebeccapurple" => 0x663399,
        "red" => 0xff0000,
        "rosybrown" => 0xbc8f8f,
        "royalblue" => 0x4169e1,
        "saddlebrown" => 0x8b4513,
        "salmon" => 0xfa8072,
        "sandybrown" => 0xf4a460,
        "seagreen" => 0x2e8b57,
        "seashell" => 0xfff5ee,
        "sienna" => 0xa0522d,
        "silver" => 0xc0c0c0,
        "skyblue" => 0x87ceeb,
        "slateblue" => 0x6a5acd,
        "slategray" => 0x708090,
        "slategrey" => 0x708090,
        "snow" => 0xfffafa,
        "springgreen" => 0x00ff7f,
        "steelblue" => 0x4682b4,
        "tan" => 0xd2b48c,
        "teal" => 0x008080,
        "thistle" => 0xd8bfd8,
        "tomato" => 0xff6347,
        "turquoise" => 0x40e0d0,
        "violet" => 0xee82ee,
        "wheat" => 0xf5deb3,
        "white" => 0xffffff,
        "whitesmoke" => 0xf5f5f5,
        "yellow" => 0xffff00,
        "yellowgreen" => 0x9acd32,
        _ => return None,
    };
    Some(Color::Rgb(Rgb {
        r: ((n >> 16) & 0xff) as f64,
        g: ((n >> 8) & 0xff) as f64,
        b: (n & 0xff) as f64,
        opacity: 1.0,
    }))
}
