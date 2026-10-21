//! The JavaScript's own locales, and the formats they give.
//!
//! Translated from `d3-format/src/locale.js` and `d3-format/src/defaultLocale.js`,
//! version 1.4.1, of d3 5.16.0.

use crate::format::specifier::{format_specifier, FormatSpecifier};
use crate::format::type_::{format_group, format_numerals, format_trim, format_type};

/// The prefixes the JavaScript writes a number with.
pub const PREFIXES: [&str; 17] = [
    "y", "z", "a", "f", "p", "n", "µ", "m", "", "k", "M", "G", "T", "P", "E", "Z", "Y",
];

/// A locale, which the JavaScript's own takes as an object.
#[derive(Clone, Debug, PartialEq)]
pub struct LocaleDefinition {
    /// The decimal point.
    pub decimal: Option<String>,
    /// The thousands separator.
    pub thousands: Option<String>,
    /// The sizes of the groups of the digits.
    pub grouping: Option<Vec<usize>>,
    /// The currency, which is its prefix and its suffix.
    pub currency: Option<[String; 2]>,
    /// The percent.
    pub percent: Option<String>,
    /// The minus.
    pub minus: Option<String>,
    /// The digits of another set.
    pub numerals: Option<Vec<String>>,
    /// The name of the number that is not a number.
    pub nan: Option<String>,
}

/// The locale the JavaScript has by default.
pub fn default_definition() -> LocaleDefinition {
    LocaleDefinition {
        decimal: Some(".".to_owned()),
        thousands: Some(",".to_owned()),
        grouping: Some(vec![3]),
        currency: Some(["$".to_owned(), String::new()]),
        percent: Some("%".to_owned()),
        minus: Some("-".to_owned()),
        numerals: None,
        nan: Some("NaN".to_owned()),
    }
}

/// A locale, which gives the formats.
#[derive(Clone, Debug)]
pub struct Locale {
    /// The decimal point.
    pub decimal: String,
    /// The thousands separator.
    pub thousands: String,
    /// The sizes of the groups of the digits.
    pub grouping: Vec<usize>,
    /// The currency, which is its prefix and its suffix.
    pub currency: [String; 2],
    /// The percent.
    pub percent: String,
    /// The minus.
    pub minus: String,
    /// The digits of another set.
    pub numerals: Option<Vec<String>>,
    /// The name of the number that is not a number.
    pub nan: String,
}

impl Locale {
    /// The format of a specification, which writes a number.
    pub fn format(&self, specifier: &str) -> Option<Format> {
        format_specifier(specifier).map(|specifier| self.new_format(specifier))
    }

    /// The format of a specification, with a prefix.
    pub fn format_prefix(&self, specifier: &str, value: f64) -> Option<FormatPrefix> {
        let mut specifier = format_specifier(specifier)?;
        specifier.kind = "f".to_owned();
        let format = self.new_format(specifier);
        // The exponent of the prefix, which the JavaScript takes from
        // the number the format is made for.
        let shortest = format!("{:e}", value.abs());
        let exponent = shortest
            .split_once('e')
            .and_then(|(_, exponent)| exponent.parse::<i32>().ok())
            .unwrap_or(0);
        let e = (exponent as f64 / 3.0).floor().clamp(-8.0, 8.0) as i32 * 3;
        Some(FormatPrefix {
            format,
            scale: 10f64.powi(-e),
            prefix: PREFIXES[(8 + e / 3) as usize].to_owned(),
        })
    }

    /// The JavaScript's `newFormat`, which makes the format of a
    /// specification.
    fn new_format(&self, specifier: FormatSpecifier) -> Format {
        let mut fill = specifier.fill.clone();
        let mut align = specifier.align.clone();
        let sign = specifier.sign.clone();
        let symbol = specifier.symbol.clone();
        let _ = &symbol;
        let mut zero = specifier.zero;
        let width = specifier.width;
        let mut comma = specifier.comma;
        let mut precision = specifier.precision;
        let mut trim = specifier.trim;
        let mut kind = specifier.kind.clone();

        // The "n" kind is an alias for ",g".
        if kind == "n" {
            comma = true;
            kind = "g".to_owned();
        } else if !is_known_kind(&kind) {
            // The empty kind, and any kind that is not one of them, is
            // an alias for ".12~g".
            if precision.is_none() {
                precision = Some(12);
            }
            trim = true;
            kind = "g".to_owned();
        }

        // The padding of a zero goes after the sign and before the
        // digits.
        if zero || (fill == "0" && align == "=") {
            zero = true;
            fill = "0".to_owned();
            align = "=".to_owned();
        }

        // The prefix and the suffix.
        let prefix = if symbol == "$" {
            self.currency[0].clone()
        } else if symbol == "#" && matches!(kind.as_str(), "b" | "o" | "x" | "X") {
            format!("0{}", kind.to_lowercase())
        } else {
            String::new()
        };
        let suffix = if symbol == "$" {
            self.currency[1].clone()
        } else if matches!(kind.as_str(), "%" | "p") {
            self.percent.clone()
        } else {
            String::new()
        };

        // The count of the digits, which is given or is the default, and
        // is held within what the kind can do.
        let precision = match precision {
            None => 6,
            Some(precision) => {
                if matches!(kind.as_str(), "g" | "p" | "r" | "s") {
                    precision.clamp(1, 21)
                } else {
                    precision.clamp(0, 20)
                }
            }
        };
        // The kinds that have a part after the digits.
        let maybe_suffix = matches!(kind.as_str(), "d" | "e" | "f" | "g" | "p" | "r" | "s" | "%");

        Format {
            fill,
            align,
            sign,
            zero,
            width,
            comma,
            precision,
            trim,
            kind,
            prefix,
            suffix,
            maybe_suffix,
            decimal: self.decimal.clone(),
            minus: self.minus.clone(),
            nan: self.nan.clone(),
            group: format_group(&self.grouping, &self.thousands),
            numerals: match &self.numerals {
                Some(numerals) => Some(format_numerals(numerals)),
                None => None,
            },
            prefix_exponent: 0,
        }
    }
}

/// The grouping of the digits of a number.
type Group = Box<dyn Fn(&str, f64) -> String>;

/// A format, which writes a number.
pub struct Format {
    fill: String,
    align: String,
    sign: String,
    zero: bool,
    width: Option<usize>,
    comma: bool,
    precision: usize,
    trim: bool,
    kind: String,
    prefix: String,
    suffix: String,
    maybe_suffix: bool,
    decimal: String,
    minus: String,
    nan: String,
    group: Group,
    numerals: Option<Box<dyn Fn(String) -> String>>,
    prefix_exponent: i32,
}

impl Format {
    /// The number, as the JavaScript writes it.
    pub fn format(&self, value: f64) -> String {
        let mut value_prefix = self.prefix.clone();
        let mut value_suffix = self.suffix.clone();

        let value = if self.kind == "c" {
            // The kind writes the number itself, and the value is the
            // text of it.
            value_suffix = format!(
                "{}{}",
                format_type(&self.kind, value, self.precision),
                value_suffix
            );
            String::new()
        } else {
            // The sign of the number: a negative zero is not less than
            // zero, but one divided by it is.
            let mut negative = value < 0.0 || 1.0 / value < 0.0;
            let mut text = if value.is_nan() {
                self.nan.clone()
            } else {
                format_type(&self.kind, value.abs(), self.precision)
            };
            if self.trim {
                text = format_trim(&text);
            }
            // A negative number that rounds to zero has no sign, unless
            // a sign was asked for. The JavaScript looks at the number
            // it has written, and not at the number it was given.
            if negative && text.parse::<f64>().unwrap_or(f64::NAN) == 0.0 && self.sign != "+" {
                negative = false;
            }
            let sign = if negative {
                if self.sign == "(" {
                    self.sign.clone()
                } else {
                    self.minus.clone()
                }
            } else if self.sign == "-" || self.sign == "(" {
                String::new()
            } else {
                self.sign.clone()
            };
            value_prefix = format!("{}{}", sign, value_prefix);
            // The "s" kind writes the prefix of the SI, and the
            // JavaScript keeps the exponent of it in a variable of its
            // own, which the kind itself sets.
            let prefix_exponent = if self.kind == "s" {
                match crate::format::type_::format_decimal_parts(value.abs(), Some(self.precision))
                {
                    Some((_, exponent)) => crate::format::type_::prefix_exponent(exponent),
                    None => 0,
                }
            } else {
                self.prefix_exponent
            };
            value_suffix = format!(
                "{}{}{}",
                if self.kind == "s" {
                    PREFIXES[(8 + prefix_exponent / 3) as usize].to_owned()
                } else {
                    String::new()
                },
                value_suffix,
                if negative && self.sign == "(" {
                    ")"
                } else {
                    ""
                }
            );
            // The number is broken into the part that can be grouped and
            // the part that cannot.
            if self.maybe_suffix {
                let characters: Vec<char> = text.chars().collect();
                for (index, character) in characters.iter().enumerate() {
                    if !character.is_ascii_digit() {
                        let rest: String = characters[index..].iter().collect();
                        value_suffix = format!(
                            "{}{}",
                            if *character == '.' {
                                format!("{}{}", self.decimal, &rest[1..])
                            } else {
                                rest
                            },
                            value_suffix
                        );
                        text = characters[..index].iter().collect();
                        break;
                    }
                }
            }
            text
        };

        // The grouping is done before the padding, unless the padding is
        // with zeros.
        let mut value = value;
        if self.comma && !self.zero {
            value = (self.group)(&value, f64::INFINITY);
        }

        // The padding.
        let length =
            value_prefix.chars().count() + value.chars().count() + value_suffix.chars().count();
        let width = self.width.unwrap_or(0);
        let padding: String = if length < width {
            self.fill.repeat(width - length)
        } else {
            String::new()
        };

        // With zeros the grouping is done after the padding.
        if self.comma && self.zero {
            let padded = format!("{}{}", padding, value);
            let limit = if padding.is_empty() {
                f64::INFINITY
            } else {
                (width - value_suffix.chars().count()) as f64
            };
            value = (self.group)(&padded, limit);
        }

        // The number is put together as the way of padding asks.
        let text = match self.align.as_str() {
            "<" => format!("{}{}{}{}", value_prefix, value, value_suffix, padding),
            "=" => format!("{}{}{}{}", value_prefix, padding, value, value_suffix),
            "^" => {
                let half = padding.chars().count() / 2;
                let (left, right) = padding.split_at(
                    padding
                        .char_indices()
                        .nth(half)
                        .map(|(index, _)| index)
                        .unwrap_or(padding.len()),
                );
                format!("{}{}{}{}{}", left, value_prefix, value, value_suffix, right)
            }
            _ => format!("{}{}{}{}", padding, value_prefix, value, value_suffix),
        };

        match &self.numerals {
            Some(numerals) => numerals(text),
            None => text,
        }
    }
}

/// A format with a prefix, which writes a number with the prefix of the
/// SI.
pub struct FormatPrefix {
    format: Format,
    scale: f64,
    prefix: String,
}

impl FormatPrefix {
    /// The number, as the JavaScript writes it, with the prefix.
    pub fn format(&self, value: f64) -> String {
        format!("{}{}", self.format.format(self.scale * value), self.prefix)
    }
}

/// Whether a kind is one of the JavaScript's own.
fn is_known_kind(kind: &str) -> bool {
    matches!(
        kind,
        "%" | "b" | "c" | "d" | "e" | "f" | "g" | "o" | "p" | "r" | "s" | "X" | "x"
    )
}

/// The JavaScript's `formatLocale`, which makes a locale.
pub fn format_locale(definition: &LocaleDefinition) -> Locale {
    let grouping = match (&definition.grouping, &definition.thousands) {
        (Some(grouping), Some(_)) => grouping.clone(),
        _ => vec![0],
    };
    let currency = definition.currency.clone().unwrap_or_default();
    Locale {
        decimal: definition.decimal.clone().unwrap_or_else(|| ".".to_owned()),
        thousands: definition.thousands.clone().unwrap_or_default(),
        grouping,
        currency,
        percent: definition.percent.clone().unwrap_or_else(|| "%".to_owned()),
        minus: definition.minus.clone().unwrap_or_else(|| "-".to_owned()),
        numerals: definition.numerals.clone(),
        nan: definition.nan.clone().unwrap_or_else(|| "NaN".to_owned()),
    }
}

/// The JavaScript's `defaultLocale`, which is the locale it has by
/// default.
pub fn format_default_locale() -> Locale {
    format_locale(&default_definition())
}
