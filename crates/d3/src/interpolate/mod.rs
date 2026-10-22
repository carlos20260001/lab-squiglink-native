//! A port of **d3-interpolate**, from the JavaScript of the same name.
//!
//! Translated from `d3-interpolate/src/`, version 1.4.0, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read
//! differently, the JavaScript is right.
//!
//! An interpolator is a function of a number from 0 to 1, which gives
//! the value between two: the number, the colour, the string, the date,
//! the list, the object.
//!
//! The JavaScript's own `interpolate` works out which of them to use
//! from the type of the second value, and the port does the same with
//! [`Value`], which is the JavaScript's own set of types written out.

pub mod color;
pub mod curve;
pub mod transform;
pub mod zoom;

use std::collections::BTreeMap;

use crate::arith;
use crate::color::Color;
use crate::js;

/// A value the JavaScript can hold, which an interpolator can work
/// between.
///
/// The JavaScript's types are: `null` and `undefined`, `boolean`,
/// `number`, `string`, an array and an object — and a colour and a date,
/// which are objects with their own reading. The port writes them out,
/// for Rust has no such a type as the JavaScript's `object`.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// The JavaScript's `null` and `undefined`.
    Null,
    /// The JavaScript's `boolean`.
    Bool(bool),
    /// The JavaScript's `number`.
    Number(f64),
    /// The JavaScript's `string`.
    String(String),
    /// The JavaScript's colour, which is an object of its own.
    Color(Color),
    /// The JavaScript's `Date`, which is a number of milliseconds.
    Date(f64),
    /// The JavaScript's array of numbers, which is read differently from
    /// any other array.
    NumberArray(Vec<f64>),
    /// The JavaScript's array of anything.
    Array(Vec<Value>),
    /// The JavaScript's object, which is a map of names to values.
    Object(BTreeMap<String, Value>),
}

impl Value {
    /// The value as a number, which the JavaScript's `+` gives.
    pub fn as_number(&self) -> f64 {
        match self {
            Value::Number(value) => *value,
            Value::Bool(value) => {
                if *value {
                    1.0
                } else {
                    0.0
                }
            }
            Value::Null => 0.0,
            Value::String(value) => {
                if value.trim().is_empty() {
                    0.0
                } else {
                    value.trim().parse::<f64>().unwrap_or(f64::NAN)
                }
            }
            Value::Date(value) => *value,
            _ => f64::NAN,
        }
    }
}

/// The JavaScript's `interpolate`, which works out what to draw between
/// two values.
///
/// The JavaScript looks at the type of the second value: a number is
/// drawn between as a number, a string that reads as a colour is drawn
/// between as a colour, and so on.
pub fn interpolate(a: &Value, b: &Value) -> Box<dyn FnMut(f64) -> Value> {
    match b {
        Value::Null => constant(b.clone()),
        Value::Bool(_) => constant(b.clone()),
        Value::Number(_) => {
            let a = a.as_number();
            let b = b.as_number();
            Box::new(move |t| Value::Number(interpolate_number(a, b)(t)))
        }
        Value::String(text) => match crate::color::color(text) {
            Some(colour) => match colour_of(a) {
                Some(a) => {
                    let inner = color::interpolate_rgb(&a, &colour);
                    Box::new(move |t| Value::String(inner(t)))
                }
                None => {
                    let a = text_of(a);
                    let b = text.clone();
                    let mut inner = interpolate_string(&a, &b);
                    Box::new(move |t| Value::String(inner(t)))
                }
            },
            None => {
                let a = text_of(a);
                let mut inner = interpolate_string(&a, text);
                Box::new(move |t| Value::String(inner(t)))
            }
        },
        Value::Color(colour) => {
            let a = colour_of(a).unwrap_or_else(transparent);
            let inner = color::interpolate_rgb(&a, colour);
            Box::new(move |t| Value::String(inner(t)))
        }
        Value::Date(_) => {
            let a = a.as_number();
            let b = b.as_number();
            Box::new(move |t| Value::Date(interpolate_date(a, b)(t)))
        }
        Value::NumberArray(_) => {
            let a = numbers_of(a);
            let b = numbers_of(b);
            let inner = interpolate_number_array(&a, &b);
            Box::new(move |t| Value::NumberArray(inner(t)))
        }
        Value::Array(_) => {
            let a = values_of(a);
            let b = values_of(b);
            let mut inner = interpolate_array(&a, &b);
            Box::new(move |t| Value::Array(inner(t)))
        }
        Value::Object(_) => {
            let a = object_of(a);
            let b = object_of(b);
            let mut inner = interpolate_object(&a, &b);
            Box::new(move |t| Value::Object(inner(t)))
        }
    }
}

/// The JavaScript's `constant`, which gives the same value whatever the
/// number.
pub fn constant(value: Value) -> Box<dyn FnMut(f64) -> Value> {
    Box::new(move |_| value.clone())
}

/// The JavaScript's `number`, which draws between two numbers.
pub fn interpolate_number(a: f64, b: f64) -> impl Fn(f64) -> f64 {
    move |t| {
        arith::add(
            arith::multiply(a, arith::subtract(1.0, t)),
            arith::multiply(b, t),
        )
    }
}

/// The JavaScript's `round`, which draws between two numbers and rounds
/// the result to a whole number.
pub fn interpolate_round(a: f64, b: f64) -> impl Fn(f64) -> f64 {
    move |t| js::round(interpolate_number(a, b)(t))
}

/// The JavaScript's `date`, which draws between two dates.
///
/// The JavaScript's date is a whole number of milliseconds, and the
/// `setTime` of it takes the whole part of the number, and not the
/// number itself.
pub fn interpolate_date(a: f64, b: f64) -> impl Fn(f64) -> f64 {
    move |t| interpolate_number(a, b)(t).trunc()
}

/// The JavaScript's `numberArray`, which draws between two lists of
/// numbers.
pub fn interpolate_number_array(a: &[f64], b: &[f64]) -> impl Fn(f64) -> Vec<f64> {
    let n = a.len().min(b.len());
    let a = a.to_vec();
    let b = b.to_vec();
    move |t| {
        let mut c = b.clone();
        for i in 0..n {
            c[i] = a[i] * (1.0 - t) + b[i] * t;
        }
        c
    }
}

/// The JavaScript's `array`, which draws between two lists of anything.
pub fn interpolate_array(a: &[Value], b: &[Value]) -> impl FnMut(f64) -> Vec<Value> {
    let na = a.len().min(b.len());
    let na = if a.is_empty() { 0 } else { na };
    let mut x = Vec::with_capacity(na);
    for i in 0..na {
        x.push(interpolate(&a[i], &b[i]));
    }
    let mut c = b.to_vec();
    move |t| {
        for i in 0..na {
            c[i] = x[i](t);
        }
        c.clone()
    }
}

/// The JavaScript's `object`, which draws between two objects.
pub fn interpolate_object(
    a: &BTreeMap<String, Value>,
    b: &BTreeMap<String, Value>,
) -> impl FnMut(f64) -> BTreeMap<String, Value> {
    let mut keys = Vec::new();
    let mut i: Vec<Box<dyn FnMut(f64) -> Value>> = Vec::new();
    let mut c = BTreeMap::new();
    for (key, value) in b {
        match a.get(key) {
            Some(other) => {
                keys.push(key.clone());
                i.push(interpolate(other, value));
            }
            None => {
                c.insert(key.clone(), value.clone());
            }
        }
    }
    let mut i = i;
    move |t| {
        let mut c = c.clone();
        for (index, key) in keys.iter().enumerate() {
            c.insert(key.clone(), i[index](t));
        }
        c
    }
}

/// The JavaScript's `discrete`, which draws between a list of values and
/// gives the one the number falls on.
pub fn interpolate_discrete(range: &[Value]) -> impl Fn(f64) -> Value {
    let n = range.len();
    let range = range.to_vec();
    move |t| {
        let index = (t * n as f64).floor() as i64;
        let index = index.clamp(0, n as i64 - 1) as usize;
        range[index].clone()
    }
}

/// A function that draws between two numbers.
pub type Draw = Box<dyn Fn(f64) -> f64 + 'static>;

/// The JavaScript's `piecewise`, which draws between a list of values
/// with a list of interpolators.
pub fn piecewise(interpolate: fn(f64, f64) -> Draw, values: &[f64]) -> impl Fn(f64) -> f64 {
    let n = values.len().saturating_sub(1);
    let mut i = Vec::with_capacity(n);
    let mut v = values.first().copied().unwrap_or(f64::NAN);
    for index in 0..n {
        let next = values[index + 1];
        i.push(interpolate(v, next));
        v = next;
    }
    move |t| {
        // The JavaScript multiplies the number by the number of the
        // intervals, and gives what is left of it to the interval.
        let scaled = t * n as f64;
        let index = (scaled.floor() as i64).clamp(0, n as i64 - 1) as usize;
        i[index](scaled - index as f64)
    }
}

/// The JavaScript's `quantize`, which draws between two values and
/// gives one of a list of samples.
pub fn quantize(interpolator: impl Fn(f64) -> f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| interpolator(i as f64 / (n - 1) as f64))
        .collect()
}

/// The JavaScript's `hue`, which draws between two hues the short way
/// round.
pub fn interpolate_hue(a: f64, b: f64) -> impl Fn(f64) -> f64 {
    let inner = color::hue(a, b);
    move |t| {
        let x = inner(t);
        x - 360.0 * (x / 360.0).floor()
    }
}

/// A number that is drawn between two numbers.
type Number = Box<dyn FnMut(f64) -> f64 + 'static>;

/// A part of a string, which is either text or a number that is drawn.
enum Part {
    /// The text, which is kept as it is.
    Text(String),
    /// The number, which is the index of the interpolator that draws it.
    Number(usize),
}

/// The JavaScript's `string`, which draws between two strings, and
/// between the numbers in them.
pub fn interpolate_string(a: &str, b: &str) -> Box<dyn FnMut(f64) -> String> {
    let (parts, mut numbers) = scan_string(a, b);
    if parts.len() < 2 {
        if let Some(function) = numbers.into_iter().next() {
            let mut function = function;
            return Box::new(move |t| function(t).to_string());
        }
        let b = b.to_owned();
        return Box::new(move |_| b.clone());
    }
    Box::new(move |t| {
        let mut out = String::new();
        for part in &parts {
            match part {
                Part::Text(text) => out.push_str(text),
                Part::Number(index) => out.push_str(&numbers[*index](t).to_string()),
            }
        }
        out
    })
}

/// The colour of a value, which the dispatcher reads.
fn colour_of(value: &Value) -> Option<Color> {
    match value {
        Value::Color(colour) => Some(*colour),
        Value::String(text) => crate::color::color(text),
        _ => None,
    }
}

/// The colour of nothing, which the JavaScript's `rgbConvert` gives.
fn transparent() -> Color {
    Color::Rgb(crate::color::Rgb {
        r: f64::NAN,
        g: f64::NAN,
        b: f64::NAN,
        opacity: 1.0,
    })
}

/// The text of a value, which the JavaScript's `+ ""` gives.
fn text_of(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => crate::js::number_to_string(*number),
        other => format!("{:?}", other),
    }
}

/// The numbers of a value, which the JavaScript's `numberArray` reads.
fn numbers_of(value: &Value) -> Vec<f64> {
    match value {
        Value::NumberArray(values) => values.clone(),
        Value::Array(values) => values.iter().map(Value::as_number).collect(),
        _ => Vec::new(),
    }
}

/// The values of a value, which the JavaScript's `array` reads.
fn values_of(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(values) => values.clone(),
        Value::NumberArray(values) => values.iter().map(|v| Value::Number(*v)).collect(),
        _ => Vec::new(),
    }
}

/// The object of a value, which the JavaScript's `object` reads.
fn object_of(value: &Value) -> BTreeMap<String, Value> {
    match value {
        Value::Object(object) => object.clone(),
        _ => BTreeMap::new(),
    }
}

/// The JavaScript's own number in a string, and the string's own parts.
///
/// The JavaScript's pattern is
/// `[-+]?(?:\d+\.?\d*|\.?\d+)(?:[eE][-+]?\d+)?`, and the port finds the
/// numbers by hand.
fn scan_string(a: &str, b: &str) -> (Vec<Part>, Vec<Number>) {
    let a_numbers = numbers_in(a);
    let b_numbers = numbers_in(b);
    let mut parts: Vec<Part> = Vec::new();
    let mut numbers: Vec<Box<dyn FnMut(f64) -> f64>> = Vec::new();
    let mut bi = 0usize;

    for (am, bm) in a_numbers.iter().zip(b_numbers.iter()) {
        // The text that comes before the number in b.
        if bm.0 > bi {
            let text = b[bi..bm.0].to_owned();
            match parts.last_mut() {
                Some(Part::Text(part)) => part.push_str(&text),
                _ => parts.push(Part::Text(text)),
            }
        }
        if am.1 == bm.1 {
            // The numbers are the same, and the text is kept as it is.
            match parts.last_mut() {
                Some(Part::Text(part)) => part.push_str(&bm.1),
                _ => parts.push(Part::Text(bm.1.clone())),
            }
        } else {
            // The numbers are drawn between.
            numbers.push(Box::new(interpolate_number(
                am.1.parse::<f64>().unwrap_or(f64::NAN),
                bm.1.parse::<f64>().unwrap_or(f64::NAN),
            )));
            parts.push(Part::Number(numbers.len() - 1));
        }
        bi = bm.2;
    }
    // What is left of b.
    if bi < b.len() {
        let text = b[bi..].to_owned();
        match parts.last_mut() {
            Some(Part::Text(part)) => part.push_str(&text),
            _ => parts.push(Part::Text(text)),
        }
    }
    (parts, numbers)
}

/// The numbers in a string, and where they are.
fn numbers_in(text: &str) -> Vec<(usize, String, usize)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let start = at;
        if bytes[at] == b'+' || bytes[at] == b'-' {
            at += 1;
        }
        let digits = at;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            at += 1;
        }
        let had_digits = at > digits;
        let mut had_point = false;
        if at < bytes.len() && bytes[at] == b'.' {
            at += 1;
            had_point = true;
            let after = at;
            while at < bytes.len() && bytes[at].is_ascii_digit() {
                at += 1;
            }
            if at == after && !had_digits {
                // A point that is not a number.
                at = start;
                continue;
            }
        }
        if !had_digits && !had_point {
            at = start + 1;
            continue;
        }
        // The exponent.
        if at < bytes.len() && (bytes[at] == b'e' || bytes[at] == b'E') {
            let mark = at;
            at += 1;
            if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {
                at += 1;
            }
            let exponent = at;
            while at < bytes.len() && bytes[at].is_ascii_digit() {
                at += 1;
            }
            if at == exponent {
                at = mark;
            }
        }
        out.push((start, text[start..at].to_owned(), at));
    }
    out
}
