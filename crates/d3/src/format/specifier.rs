//! The JavaScript's own reading of a format's specification.
//!
//! Translated from `d3-format/src/formatSpecifier.js`, version 1.4.1, of
//! d3 5.16.0.
//!
//! The specification is
//! `[[fill]align][sign][symbol][0][width][,][.precision][~][type]`, and
//! the JavaScript reads it with a pattern. The port reads it by hand, for
//! the crate has no dependency and the pattern is simple.

/// A format's specification, which the JavaScript reads into one.
#[derive(Clone, Debug, PartialEq)]
pub struct FormatSpecifier {
    /// The character the number is padded with.
    pub fill: String,
    /// The way the number is padded: `<` to the left, `=` about the
    /// sign, and `^` in the middle.
    pub align: String,
    /// The sign: `-` for a negative number only, `+` for every number,
    /// `(` for a negative number in brackets, and ` ` for a space.
    pub sign: String,
    /// The symbol: `$` for the currency and `#` for the base.
    pub symbol: String,
    /// Whether the number is padded with zeros.
    pub zero: bool,
    /// The width of the whole.
    pub width: Option<usize>,
    /// Whether the digits are grouped.
    pub comma: bool,
    /// The number of the digits.
    pub precision: Option<usize>,
    /// Whether the insignificant zeros are trimmed.
    pub trim: bool,
    /// The kind of the number.
    pub kind: String,
}

impl FormatSpecifier {
    /// The specification as the JavaScript writes it back.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.fill);
        out.push_str(&self.align);
        out.push_str(&self.sign);
        out.push_str(&self.symbol);
        if self.zero {
            out.push('0');
        }
        if let Some(width) = self.width {
            out.push_str(&width.max(1).to_string());
        }
        if self.comma {
            out.push(',');
        }
        if let Some(precision) = self.precision {
            out.push('.');
            out.push_str(&precision.to_string());
        }
        if self.trim {
            out.push('~');
        }
        out.push_str(&self.kind);
        out
    }
}

/// The JavaScript's `formatSpecifier`, which reads a specification.
///
/// The JavaScript throws for a specification it cannot read, and the port
/// gives `None`.
pub fn format_specifier(specifier: &str) -> Option<FormatSpecifier> {
    let mut rest = specifier;
    let mut fill = None;
    let mut align = None;

    // The fill and the align, which are a character and one of the three
    // ways of padding.
    let mut characters = rest.chars();
    let first = characters.next();
    let second = characters.next();
    if let (Some(first), Some(second)) = (first, second) {
        if is_align(second) {
            fill = Some(first);
            align = Some(second);
            rest = &rest[first.len_utf8() + second.len_utf8()..];
        }
    }
    if align.is_none() {
        if let Some(first) = rest.chars().next() {
            if is_align(first) {
                align = Some(first);
                rest = &rest[first.len_utf8()..];
            }
        }
    }

    // The sign.
    let mut sign = None;
    if let Some(character) = rest.chars().next() {
        if matches!(character, '+' | '-' | '(' | ' ') {
            sign = Some(character);
            rest = &rest[character.len_utf8()..];
        }
    }

    // The symbol.
    let mut symbol = None;
    if let Some(character) = rest.chars().next() {
        if matches!(character, '$' | '#') {
            symbol = Some(character);
            rest = &rest[character.len_utf8()..];
        }
    }

    // The zero, and the width.
    let mut zero = false;
    if rest.starts_with('0') {
        zero = true;
        rest = &rest[1..];
    }
    let (width, tail) = take_digits(rest);
    rest = tail;

    // The comma.
    let mut comma = false;
    if rest.starts_with(',') {
        comma = true;
        rest = &rest[1..];
    }

    // The precision.
    let mut precision = None;
    if rest.starts_with('.') {
        let (digits, tail) = take_digits(&rest[1..]);
        precision = digits;
        rest = tail;
    }

    // The trim.
    let mut trim = false;
    if rest.starts_with('~') {
        trim = true;
        rest = &rest[1..];
    }

    // The kind, which is a letter or a percent.
    let mut kind = String::new();
    if let Some(character) = rest.chars().next() {
        if character.is_ascii_alphabetic() || character == '%' {
            kind.push(character);
            rest = &rest[character.len_utf8()..];
        }
    }
    if !rest.is_empty() {
        // The JavaScript's pattern reads the whole of the string.
        return None;
    }

    Some(FormatSpecifier {
        fill: fill
            .map(|character| character.to_string())
            .unwrap_or_else(|| " ".to_owned()),
        align: align
            .map(|character| character.to_string())
            .unwrap_or_else(|| ">".to_owned()),
        sign: sign
            .map(|character| character.to_string())
            .unwrap_or_else(|| "-".to_owned()),
        symbol: symbol
            .map(|character| character.to_string())
            .unwrap_or_default(),
        zero,
        width,
        comma,
        precision,
        trim,
        kind,
    })
}

/// Whether a character is one of the three ways of padding.
fn is_align(character: char) -> bool {
    matches!(character, '<' | '=' | '>' | '^')
}

/// The digits of a string, and what is left of it.
fn take_digits(text: &str) -> (Option<usize>, &str) {
    let end = text
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(text.len());
    if end == 0 {
        return (None, text);
    }
    (text[..end].parse::<usize>().ok(), &text[end..])
}
