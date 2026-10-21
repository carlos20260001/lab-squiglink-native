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

/// JavaScript's `===` for numbers.
///
/// `NaN` is not equal to itself, and `-0` and `+0` are equal.
pub fn strict_equals(a: f64, b: f64) -> bool {
    a == b
}
