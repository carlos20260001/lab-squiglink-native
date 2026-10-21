//! The JavaScript number semantics the port depends on.
//!
//! The JavaScript this crate was translated from has a number type that is
//! not quite IEEE 754 binary64: it is binary64 with three exceptions, and
//! the port has to keep them, because the JavaScript's own results depend on
//! them.
//!
//! The exceptions are:
//!
//! - `NaN` is a value of its own, and it is not equal to itself. The port
//!   carries it as `f64::NAN`, which behaves the same way.
//! - `-0` and `+0` are the same number for every purpose except
//!   `1 / x`, and the port keeps the sign where it is visible.
//! - Integer values outside the safe range lose their precision, as they do
//!   in JavaScript, because the port's numbers are the same `f64`.
//!
//! JavaScript's `+x` conversion, which turns a value into a number, is
//! [`number`]. It is not the same as Rust's `as f64`, because it maps
//! `null` to `NaN` rather than to zero.

/// JavaScript's `+x`: the value as a number.
///
/// `null` becomes `NaN`, and everything else becomes its number. A string
/// that does not spell a number becomes `NaN`, which the port leaves to the
/// caller's own conversion.
pub fn number(x: f64) -> f64 {
    if x.is_nan() {
        f64::NAN
    } else {
        x
    }
}

/// Whether a value is `NaN`, in the sense JavaScript means it.
///
/// This is the same as `f64::is_nan`, and is named separately because the
/// JavaScript writes it `isNaN`.
pub fn is_nan(x: f64) -> bool {
    x.is_nan()
}

/// JavaScript's `x >= x`, which is false only for `NaN`.
///
/// This is the test the JavaScript uses to find "the first comparable
/// value", and it is what Rust's `partial_cmp` returns `None` for.
#[allow(clippy::eq_op)]
pub fn is_comparable(x: f64) -> bool {
    // The JavaScript writes this `x >= x`, which is false for `NaN` and
    // true for everything else. The comparison looks odd on purpose: it is
    // how the JavaScript asks whether a value can be compared at all.
    x >= x
}

/// JavaScript's `Object.is`, which distinguishes `-0` from `+0`.
///
/// `Object.is(NaN, NaN)` is true, and `Object.is(0, -0)` is false, unlike
/// `===`.
pub fn object_is(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a == 0.0 && b == 0.0 {
        return a.is_sign_negative() == b.is_sign_negative();
    }
    a == b
}

/// Whether a number is truthy in JavaScript.
///
/// The JavaScript's curves keep a flag of `0`, `1` or `NaN`, and they ask
/// whether it is true in order to decide what to draw. In JavaScript
/// `NaN` is falsy, and `0` is falsy, and every other number is truthy —
/// which is not the same as Rust's `!= 0.0`, where `NaN` is truthy.
pub fn truthy(value: f64) -> bool {
    value != 0.0 && !value.is_nan()
}

/// JavaScript's `===` for numbers.
///
/// `NaN` is not equal to itself, and `-0` and `+0` are equal.
pub fn strict_equals(a: f64, b: f64) -> bool {
    a == b
}

/// JavaScript's `String(x)` for a number.
///
/// The rules are the ones the specification gives for
/// `Number::toString`, and Rust's own `{}` does not follow them: Rust
/// writes `1e21` as a thousand digits and `1e-7` in full, where
/// JavaScript writes the first as `1e+21` and the second as `1e-7`. The
/// paths d3 writes are made of these strings, so the port has to write
/// them the same way.
///
/// The rule, from the specification: let `n`, `k` and `s` be such that
/// `k >= 1`, `10^(k-1) <= s < 10^k`, `s * 10^(n-k)` is the number, and `k`
/// is as small as possible — that is, `s` is the number's shortest round
/// trip digits. Then:
///
/// - if `k <= n <= 21`, the digits of `s` and then `n-k` zeros;
/// - if `0 < n <= 21`, the digits of `s` with a point after `n` of them;
/// - if `-6 < n <= 0`, `0.`, then `-n` zeros, then the digits of `s`;
/// - otherwise the digits of `s` with a point in them, then `e`, the sign,
///   and `|n-1|`.
pub fn number_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value == 0.0 {
        // JavaScript writes both zeros as `0`.
        return "0".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 {
            "Infinity".to_owned()
        } else {
            "-Infinity".to_owned()
        };
    }
    let negative = value < 0.0;
    let value = value.abs();

    // The shortest round trip digits, and the exponent they sit at. Rust's
    // `{:e}` gives the same digits as JavaScript's `toExponential`, in the
    // same form.
    let exponential = format!("{:e}", value);
    let (mantissa, exponent) = exponential
        .split_once('e')
        .expect("Rust's exponential form always has an exponent");
    let exponent: i32 = exponent.parse().expect("Rust's exponent is a number");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exponent + 1;

    let text = if k <= n && n <= 21 {
        let mut out = digits;
        out.push_str(&"0".repeat((n - k) as usize));
        out
    } else if 0 < n && n <= 21 {
        let split = n as usize;
        format!("{}.{}", &digits[..split], &digits[split..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{}", "0".repeat((-n) as usize), digits)
    } else {
        let sign = if n > 1 { "+" } else { "-" };
        let magnitude = (n - 1).abs();
        if k == 1 {
            format!("{}e{}{}", digits, sign, magnitude)
        } else {
            format!("{}.{}e{}{}", &digits[..1], &digits[1..], sign, magnitude)
        }
    };

    if negative {
        format!("-{text}")
    } else {
        text
    }
}
