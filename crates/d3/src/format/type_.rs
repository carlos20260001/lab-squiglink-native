//! The JavaScript's own writing of a number in each kind of format.
//!
//! Translated from `d3-format/src/formatTypes.js` and the helpers beside
//! it, version 1.4.1, of d3 5.16.0.

use crate::js;
use crate::number::{to_exponential, to_fixed, to_precision, to_radix};

/// The JavaScript's `formatTypes`, which writes a number in the kind of
/// format asked for.
pub fn format_type(kind: &str, value: f64, precision: usize) -> String {
    match kind {
        "%" => to_fixed(value * 100.0, precision),
        "b" => to_radix(js::round(value), 2),
        "c" => js::number_to_string(value),
        "d" => format_decimal(value),
        "e" => to_exponential(value, precision),
        "f" => to_fixed(value, precision),
        "g" => to_precision(value, precision),
        "o" => to_radix(js::round(value), 8),
        "p" => format_rounded(value * 100.0, precision),
        "r" => format_rounded(value, precision),
        "s" => format_prefix_auto(value, precision),
        "X" => to_radix(js::round(value), 16).to_uppercase(),
        "x" => to_radix(js::round(value), 16),
        _ => js::number_to_string(value),
    }
}

/// The JavaScript's `formatDecimal`, which writes a whole number.
pub fn format_decimal(value: f64) -> String {
    let value = js::round(value);
    if value.abs() >= 1e21 {
        // The JavaScript writes a number this large in full, and the
        // grouping of it is taken away.
        return whole_digits(value);
    }
    js::number_to_string(value)
}

/// The whole digits of a number, which the JavaScript writes in full.
pub fn whole_digits(value: f64) -> String {
    let negative = value < 0.0;
    let whole = value.abs().trunc();
    let mut digits = Vec::new();
    let mut whole = whole;
    while whole >= 1.0 {
        let digit = (whole % 10.0) as u32;
        digits.push((b'0' + digit as u8) as char);
        whole = (whole / 10.0).trunc();
    }
    if digits.is_empty() {
        digits.push('0');
    }
    digits.reverse();
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.extend(digits);
    out
}

/// The JavaScript's `formatDecimalParts`, which gives the decimal
/// coefficient and the exponent of a number.
///
/// The number is positive ✓ and the count of the significant digits is
/// between one and twenty-one ✓.
pub fn format_decimal_parts(value: f64, precision: Option<usize>) -> Option<(String, i32)> {
    if !value.is_finite() || value == 0.0 {
        return None;
    }
    let text = match precision {
        Some(digits) if digits > 0 => to_exponential(value, digits - 1),
        _ => to_exponential(value, 16),
    };
    let (front, back) = text.split_once('e')?;
    let exponent: i32 = back.parse().ok()?;
    let coefficient = if front.len() > 1 {
        format!("{}{}", &front[..1], &front[2..])
    } else {
        front.to_owned()
    };
    Some((coefficient, exponent))
}

/// The JavaScript's `formatRounded`, which writes a number with a given
/// number of the significant digits.
pub fn format_rounded(value: f64, precision: usize) -> String {
    let (coefficient, exponent) = match format_decimal_parts(value, Some(precision)) {
        Some(parts) => parts,
        None => return js::number_to_string(value),
    };
    if exponent < 0 {
        format!("0.{}{}", "0".repeat((-exponent - 1) as usize), coefficient)
    } else if coefficient.len() as i32 > exponent + 1 {
        format!(
            "{}.{}",
            &coefficient[..(exponent + 1) as usize],
            &coefficient[(exponent + 1) as usize..]
        )
    } else {
        format!(
            "{}{}",
            coefficient,
            "0".repeat((exponent - coefficient.len() as i32 + 1) as usize)
        )
    }
}

/// The JavaScript's `formatPrefixAuto`, which writes a number with an SI
/// prefix, and the prefix itself.
pub fn format_prefix_auto(value: f64, precision: usize) -> String {
    let (coefficient, exponent) = match format_decimal_parts(value, Some(precision)) {
        Some(parts) => parts,
        None => return crate::number::to_precision(value, precision),
    };
    let prefix_exponent = prefix_exponent(exponent);
    let i = exponent - prefix_exponent + 1;
    let n = coefficient.len() as i32;
    if i == n {
        coefficient
    } else if i > n {
        format!("{}{}", coefficient, "0".repeat((i - n) as usize))
    } else if i > 0 {
        format!(
            "{}.{}",
            &coefficient[..i as usize],
            &coefficient[i as usize..]
        )
    } else {
        // The number is below one, and the digits of it are written out
        // again.
        let rest = match format_decimal_parts(value, Some(precision + (i - 1).max(0) as usize)) {
            Some((coefficient, _)) => coefficient,
            None => String::new(),
        };
        format!("0.{}{}", "0".repeat((-i) as usize), rest)
    }
}

/// The JavaScript's `prefixExponent`, which is the exponent of the SI
/// prefix.
///
/// The exponent is a multiple of three ✓ and is held between minus
/// eight and eight ✓.
pub fn prefix_exponent(exponent: i32) -> i32 {
    (exponent as f64 / 3.0).floor().clamp(-8.0, 8.0) as i32 * 3
}

/// The JavaScript's `formatTrim`, which takes the insignificant zeros off
/// a written number.
pub fn format_trim(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let mut i = 1;
    let mut i0: i32 = -1;
    let mut i1: i32 = -1;
    while i < characters.len() {
        match characters[i] {
            '.' => {
                i0 = i as i32;
                i1 = i as i32;
            }
            '0' => {
                if i0 == 0 {
                    i0 = i as i32;
                }
                i1 = i as i32;
            }
            other => {
                if !other.is_ascii_digit() {
                    break;
                }
                if i0 > 0 {
                    i0 = 0;
                }
            }
        }
        i += 1;
    }
    if i0 > 0 && i1 >= 0 {
        let mut out: String = characters[..i0 as usize].iter().collect();
        out.extend(&characters[(i1 + 1) as usize..]);
        out
    } else {
        text.to_owned()
    }
}

/// The grouping of the digits of a number.
pub type Group = Box<dyn Fn(&str, f64) -> String>;

/// The JavaScript's `exponent`, which is the exponent of a number.
pub fn exponent(value: f64) -> f64 {
    match format_decimal_parts(value.abs(), None) {
        Some((_, exponent)) => exponent as f64,
        None => f64::NAN,
    }
}

/// The JavaScript's `precisionFixed`, which is the count of the digits
/// after the point that a step needs.
pub fn precision_fixed(step: f64) -> f64 {
    let value = -exponent(step.abs());
    if value.is_nan() {
        f64::NAN
    } else if value < 0.0 {
        0.0
    } else {
        value
    }
}

/// The JavaScript's `precisionPrefix`, which is the count of the digits
/// a prefix needs.
pub fn precision_prefix(step: f64, value: f64) -> f64 {
    let prefix = (exponent(value) / 3.0).floor().clamp(-8.0, 8.0);
    let value = prefix * 3.0 - exponent(step.abs());
    if value.is_nan() {
        f64::NAN
    } else if value < 0.0 {
        0.0
    } else {
        value
    }
}

/// The JavaScript's `precisionRound`, which is the count of the digits
/// that a step and a number need.
pub fn precision_round(step: f64, max: f64) -> f64 {
    let step = step.abs();
    let max = max.abs() - step;
    let value = exponent(max) - exponent(step);
    let value = if value.is_nan() {
        f64::NAN
    } else if value < 0.0 {
        0.0
    } else {
        value
    };
    value + 1.0
}

/// The JavaScript's `formatGroup`, which groups the digits of a number.
pub fn format_group(grouping: &[usize], thousands: &str) -> Group {
    let grouping = grouping.to_vec();
    let thousands = thousands.to_owned();
    Box::new(move |value: &str, width: f64| {
        // The JavaScript takes the groups from the end of the number,
        // and the group that is left is the one that is too wide.
        let mut at = value.len() as i64;
        let mut out: Vec<String> = Vec::new();
        let mut index = 0;
        let mut group = grouping.first().copied().unwrap_or(0) as i64;
        let mut length = 0i64;
        while at > 0 && group > 0 {
            if length + group + 1 > width as i64 {
                group = (width as i64 - length).max(1);
            }
            at -= group;
            let start = (at.max(0) as usize).min(value.len());
            let end = ((at + group).max(0) as usize).min(value.len());
            out.push(value[start..end].to_owned());
            length += group + 1;
            if length > width as i64 {
                break;
            }
            index = (index + 1) % grouping.len();
            group = grouping[index] as i64;
        }
        out.reverse();
        out.join(&thousands)
    })
}

/// The JavaScript's `formatNumerals`, which writes a number in another
/// set of digits.
pub fn format_numerals(numerals: &[String]) -> Box<dyn Fn(String) -> String> {
    let numerals = numerals.to_vec();
    Box::new(move |value: String| {
        value
            .chars()
            .map(|character| match character.to_digit(10) {
                Some(digit) => numerals
                    .get(digit as usize)
                    .cloned()
                    .unwrap_or_else(|| character.to_string()),
                None => character.to_string(),
            })
            .collect()
    })
}
