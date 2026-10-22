//! The JavaScript's own arithmetic.
//!
//! The JavaScript runs on V8, and V8's arithmetic and Rust's do not
//! always give the same number when a number that is not a number is
//! one of the operands.
//!
//! The difference is in the *bits* of the number, not in its value: a
//! number that is not a number is a number that is not a number either
//! way, but the bits of it are what a program can see. V8's own
//! multiplication of two such numbers gives the second of them, and
//! Rust's gives the first of them when the first is a quiet one — so the
//! same expression gives a different number by its bits on the two.
//!
//! The port therefore does its arithmetic here, where the JavaScript's
//! own way of it is written out once.

/// The JavaScript's multiplication, which gives the second of two
/// numbers that are not numbers.
///
/// The JavaScript's own `a * b` is the machine's ✗, and the machine
/// gives the first of the two ✗ — and V8's ✗ gives the second ✗, and
/// the port's has to give the second ✗ as well ✗.
pub fn multiply(a: f64, b: f64) -> f64 {
    propagate(a * b, a, b)
}

/// The JavaScript's addition, which gives the second of two numbers
/// that are not numbers.
pub fn add(a: f64, b: f64) -> f64 {
    propagate(a + b, a, b)
}

/// The JavaScript's subtraction, which gives the second of two numbers
/// that are not numbers.
pub fn subtract(a: f64, b: f64) -> f64 {
    propagate(a - b, a, b)
}

/// The JavaScript's division, which gives the second of two numbers
/// that are not numbers.
pub fn divide(a: f64, b: f64) -> f64 {
    propagate(a / b, a, b)
}

/// A number that is not a number, with the second of the two that were
/// multiplied, and the second of the two that were added.
fn propagate(result: f64, a: f64, b: f64) -> f64 {
    if !result.is_nan() {
        return result;
    }
    if b.is_nan() {
        return quiet(b);
    }
    if a.is_nan() {
        return quiet(a);
    }
    result
}

/// A number that is not a number, with the bit that says it is quiet.
///
/// The machine quiets a number that is not a number when it makes one ✗
/// — for a number that is not quiet raises an error when it is used ✗ —
/// and so the port does ✗.
fn quiet(value: f64) -> f64 {
    f64::from_bits(value.to_bits() | 0x0008_0000_0000_0000)
}
