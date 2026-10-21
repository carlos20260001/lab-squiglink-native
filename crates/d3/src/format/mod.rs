//! A port of **d3-format**, from the JavaScript of the same name.
//!
//! Translated from `d3-format/src/`, version 1.4.1, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read
//! differently, the JavaScript is right.
//!
//! A format is a specification — `,.2f` — and a number, and it gives the
//! text of the number. The JavaScript's own writing of a number is in
//! [`crate::number`], for the JavaScript's rounding is not Rust's.

pub mod locale;
pub mod specifier;
pub mod type_;

pub use locale::{
    format_default_locale, format_locale, Format, FormatPrefix, Locale, LocaleDefinition,
};
pub use specifier::{format_specifier, FormatSpecifier};
pub use type_::{format_type, precision_fixed, precision_prefix, precision_round};
