//! The ECMAScript's own numbers, and the writing of them.
//!
//! The specification writes a number in three ways — `toFixed`,
//! `toExponential` and `toPrecision` — and each of them rounds the
//! number in the same way: the number is the exact value of the double,
//! and a half is rounded away from zero.
//!
//! Rust's own formatting rounds a half to even, so `(2.5).toFixed(0)` is
//! `"2"` in Rust and `"3"` in JavaScript. The port cannot use Rust's
//! formatting for these three, and writes them out here.
//!
//! The rule for a half is found without writing the number out: a double
//! is an odd integer times a power of two, and the number of digits after
//! the point in its exact writing is the number of that power. A half is
//! reached at a given number of digits when the exact writing has one
//! digit more than the number asked for, and no more.

use crate::js;

/// The exact value of a number, as an odd integer and a power of two.
///
/// The integer is odd, for the powers of two are taken out of it, and the
/// power is then the one the number has.
fn exact(value: f64) -> (u128, i32) {
    let bits = value.to_bits();
    let mut exponent = ((bits >> 52) & 0x7ff) as i32;
    let mut mantissa = (bits & 0x000f_ffff_ffff_ffff) as u128;
    if exponent == 0 {
        // A subnormal, which has no bit of its own for the exponent.
        exponent = -1074;
    } else {
        mantissa |= 1 << 52;
        exponent -= 1075;
    }
    // The powers of two are taken out of the mantissa, so that the number
    // of digits after the point is the exponent's.
    let mut trailing = 0;
    while mantissa != 0 && mantissa.is_multiple_of(2) {
        mantissa /= 2;
        trailing += 1;
    }
    (mantissa, exponent + trailing)
}

/// The number of digits after the point in the exact writing of a
/// number.
fn fractional_digits(exponent: i32) -> i32 {
    if exponent < 0 {
        -exponent
    } else {
        0
    }
}

/// Whether a number is exactly a half at a given number of digits after
/// the point.
///
/// The exact writing of the number has one digit more than the number
/// asked for, and the digit is a five: the writing of a double ends
/// after so many digits, and the number is a half exactly when the
/// writing stops there.
fn is_half_at(_mantissa: u128, exponent: i32, digits: usize) -> bool {
    if exponent >= 0 {
        // A whole number is never a half.
        return false;
    }
    fractional_digits(exponent) as usize == digits + 1
}

/// Whether a number is exactly a half at a given number of significant
/// digits.
///
/// The exact writing of the number has one significant digit more than
/// the number asked for, and that digit is a five.
fn is_half_at_significant(mantissa: u128, exponent: i32, digits: usize) -> bool {
    let d = fractional_digits(exponent);
    if d > 22 {
        // The writing is too long to be a half at so many digits.
        return false;
    }
    // The exact writing is the number itself, and the number is the
    // mantissa over ten to the power of the exponent, or the mantissa
    // times two to the power of it.
    let mut whole = mantissa;
    if exponent >= 0 {
        match mantissa.checked_shl(exponent as u32) {
            Some(shifted) => whole = shifted,
            None => return false,
        }
    } else {
        for _ in 0..d {
            match whole.checked_mul(5) {
                Some(multiplied) => whole = multiplied,
                None => return false,
            }
        }
    }
    // The writing has one digit more than the number asked for, and
    // that digit is a five.
    whole >= 10u128.pow(digits as u32) && whole < 10u128.pow(digits as u32 + 1) && whole % 10 == 5
}

/// The JavaScript's `toFixed`, which writes a number with a given number
/// of digits after the point.
///
/// The number is the whole number the JavaScript writes, and the digits
/// after the point are the ones the JavaScript writes.
pub fn to_fixed(value: f64, digits: usize) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value < 0.0 {
            "-Infinity".to_owned()
        } else {
            "Infinity".to_owned()
        };
    }
    let negative = value < 0.0;
    let value = value.abs();
    if value >= 1e21 {
        // The JavaScript writes a number this large in full, with its
        // sign.
        return js::number_to_string(if negative { -value } else { value });
    }
    let (mantissa, exponent) = exact(value);
    let text = if is_half_at(mantissa, exponent, digits) {
        // The number is a half, and the JavaScript rounds it away from
        // zero.
        round_away(&format!("{:.*}", digits + 1, value), digits)
    } else {
        format!("{:.*}", digits, value)
    };
    if negative {
        format!("-{}", text)
    } else {
        text
    }
}

/// The JavaScript's `toExponential`, which writes a number in the
/// exponential form with a given number of digits after the point.
pub fn to_exponential(value: f64, digits: usize) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value < 0.0 {
            "-Infinity".to_owned()
        } else {
            "Infinity".to_owned()
        };
    }
    let negative = value < 0.0;
    let value = value.abs();
    if value == 0.0 {
        let text = format!("{:.*e}", digits, 0.0);
        return fix_exponent(&text, 0);
    }
    let (_mantissa, exact_exponent) = exact(value);
    // The exponent the number has, which the shortest writing gives.
    let own = exponent_of(value);
    let (front, written) = if is_half_at_exponential(exact_exponent, digits, own) {
        // The number is a half, and the JavaScript rounds it away from
        // zero.
        let text = format!("{:.*e}", digits + 1, value);
        let (front, back) = text.split_once('e').expect("the exponent");
        let back: i32 = back.parse().expect("the exponent");
        (round_away(front, digits), back)
    } else {
        let text = format!("{:.*e}", digits, value);
        let (front, back) = text.split_once('e').expect("the exponent");
        (front.to_owned(), back.parse().expect("the exponent"))
    };
    // The mantissa may have carried into a whole number, and the
    // exponent is then one less.
    // The mantissa may have carried into a whole number, and the
    // exponent is then one greater.
    let (front, written) = normalise(&front, written);
    let text = fix_exponent(&format!("{}e{}", front, written), written);
    if negative {
        format!("-{}", text)
    } else {
        text
    }
}

/// Whether a number is exactly a half at a given number of digits after
/// the point of its mantissa.
fn is_half_at_exponential(exponent: i32, digits: usize, own: i32) -> bool {
    if exponent >= 0 {
        return false;
    }
    // The mantissa is the number divided by ten to the power of its own
    // exponent, and the point of the exact writing moves with it.
    fractional_digits(exponent) as i64 + own as i64 == digits as i64 + 1
}

/// The JavaScript's `toPrecision`, which writes a number with a given
/// number of significant digits.
pub fn to_precision(value: f64, digits: usize) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value < 0.0 {
            "-Infinity".to_owned()
        } else {
            "Infinity".to_owned()
        };
    }
    if digits == 0 {
        return "0".to_owned();
    }
    let negative = value < 0.0;
    let value = value.abs();
    if value == 0.0 {
        let text = if digits == 1 {
            "0".to_owned()
        } else {
            format!("0.{}", "0".repeat(digits - 1))
        };
        return if negative { format!("-{}", text) } else { text };
    }
    let (mantissa, exponent) = exact(value);
    // The digits of the number, in the exponential form.
    let (front, written) = if is_half_at_significant(mantissa, exponent, digits) {
        // The number is a half, and the JavaScript rounds it away from
        // zero.
        let text = format!("{:.*e}", digits, value);
        let (front, back) = text.split_once('e').expect("the exponent");
        let back: i32 = back.parse().expect("the exponent");
        (round_away(front, digits - 1), back)
    } else {
        let text = format!("{:.*e}", digits - 1, value);
        let (front, back) = text.split_once('e').expect("the exponent");
        (front.to_owned(), back.parse().expect("the exponent"))
    };
    // The mantissa may have carried into a whole number, and the
    // exponent is then one greater.
    let (front, written) = normalise(&front, written);
    // The exponent the number has after it is rounded, which the
    // JavaScript's own form is chosen by.
    let own = written;
    // The number is written in full when its exponent is small enough,
    // and in the exponential form when it is not.
    let text = if own < -6 || own >= digits as i32 {
        fix_exponent(&format!("{}e{}", front, own), own)
    } else {
        // The number is written with the digits after the point, and
        // the point is after so many digits of it.
        let written: String = front.chars().filter(|c| *c != '.').collect();
        let before = 1 + own;
        let (whole, after) = if before <= 0 {
            // The point is before the number.
            let mut after = written;
            for _ in 0..(-before) {
                after.insert(0, '0');
            }
            ("0".to_owned(), after)
        } else if before as usize >= written.len() {
            // The point is after the number.
            let mut whole = written;
            while whole.len() < before as usize {
                whole.push('0');
            }
            (whole, String::new())
        } else {
            let before = before as usize;
            (written[..before].to_owned(), written[before..].to_owned())
        };
        if after.is_empty() {
            whole
        } else {
            format!("{}.{}", whole, after)
        }
    };
    if negative {
        format!("-{}", text)
    } else {
        text
    }
}

/// The JavaScript's `toString(radix)`, which writes a whole number in
/// another base.
///
/// The number is a whole number, for the JavaScript's own `toString`
/// writes the whole part of it, and the formats that use this round the
/// number first.
pub fn to_radix(value: f64, radix: u32) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value < 0.0 {
            "-Infinity".to_owned()
        } else {
            "Infinity".to_owned()
        };
    }
    if radix == 10 {
        // The JavaScript gives the number to its own `ToString`.
        return js::number_to_string(value);
    }
    let negative = value < 0.0;
    let value = value.abs();
    let whole = value.trunc();
    if whole >= 1e21 {
        // The JavaScript writes a number this large in full.
        return js::number_to_string(value);
    }
    // The digits, which the JavaScript takes by dividing the number by
    // the base, and which are written from the last one to the first.
    //
    // The number is divided by the base while the quotient is large
    // enough for the division to lose the last of it, and the digits
    // that are lost are written as zeros.
    let mut integer = whole;
    let mut digits = Vec::new();
    loop {
        let remainder = integer % radix as f64;
        digits.push(DIGITS[remainder as usize]);
        integer = (integer - remainder) / radix as f64;
        if integer <= 0.0 {
            break;
        }
    }
    digits.reverse();
    let text = String::from_utf8(digits).expect("the digits");
    if negative {
        format!("-{}", text)
    } else {
        text
    }
}

/// The digits the JavaScript writes a number in another base with.
const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// The exponent the number has, which the shortest writing of it gives.
fn exponent_of(value: f64) -> i32 {
    let shortest = format!("{:e}", value);
    shortest
        .split_once('e')
        .and_then(|(_, exponent)| exponent.parse::<i32>().ok())
        .unwrap_or(0)
}

/// A mantissa that has carried into a whole number, which is written as
/// a mantissa again and with an exponent one greater.
fn normalise(front: &str, written: i32) -> (String, i32) {
    let (whole, after) = match front.split_once('.') {
        Some((whole, after)) => (whole, Some(after)),
        None => (front, None),
    };
    if whole != "10" {
        return (front.to_owned(), written);
    }
    let front = match after {
        Some(after) => format!("1.{}", after),
        None => "1".to_owned(),
    };
    (front, written + 1)
}

/// The exponent of the exponential form, which the JavaScript writes
/// with a sign and with at least two digits.
fn fix_exponent(text: &str, exponent: i32) -> String {
    let (front, _) = text.split_once('e').unwrap_or((text, "0"));
    let sign = if exponent < 0 { "-" } else { "+" };
    format!("{}e{}{}", front, sign, exponent.abs())
}

/// A number that is a half, rounded away from zero.
///
/// The text is the number with one digit more than the number asked for,
/// and the digits after the point are the ones to be rounded.
fn round_away(text: &str, digits: usize) -> String {
    let (integer, fraction) = text.split_once('.').unwrap_or((text, ""));
    let mut fraction: Vec<u8> = fraction.as_bytes().iter().copied().take(digits).collect();
    let mut index = fraction.len();
    loop {
        if index == 0 {
            // The carry goes into the whole number.
            let mut integer: Vec<u8> = integer.as_bytes().to_vec();
            let mut at = integer.len();
            loop {
                if at == 0 {
                    integer.insert(0, b'1');
                    break;
                }
                at -= 1;
                if integer[at] == b'9' {
                    integer[at] = b'0';
                    continue;
                }
                integer[at] += 1;
                break;
            }
            if digits == 0 {
                return String::from_utf8(integer).expect("the whole number");
            }
            return format!(
                "{}.{}",
                String::from_utf8(integer).expect("the whole number"),
                String::from_utf8(fraction).expect("the digits")
            );
        }
        index -= 1;
        if fraction[index] == b'9' {
            fraction[index] = b'0';
            continue;
        }
        fraction[index] += 1;
        break;
    }
    if digits == 0 {
        return integer.to_owned();
    }
    format!(
        "{}.{}",
        integer,
        String::from_utf8(fraction).expect("the digits")
    )
}
