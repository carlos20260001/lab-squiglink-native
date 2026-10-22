//! A port of **d3-dsv**, from the JavaScript of the same name.
//!
//! Translated from `d3-dsv/src/*.js`, version 1.2.0, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read
//! differently, the JavaScript is right.
//!
//! # What the package does
//!
//! A text of values that are separated by a delimiter — the
//! comma-separated values of a table, and the tab-separated ones — is
//! read into rows, and written out of them.
//!
//! The reading is a state machine of the JavaScript's own: a quote
//! opens a field, a doubled quote is one quote, and a field that is
//! not closed runs to the end of the text. The writing quotes a value
//! when it holds the delimiter, a quote, a newline or a carriage
//! return.
//!
//! The names of the columns are the first row of the text, and the rows
//! that follow are read under them.
//!
//! # The shape of the port
//!
//! The JavaScript's `dsv` is a function that takes a delimiter and
//! gives an object of seven functions; the port's is a [`Dsv`], which
//! holds the delimiter and has the same seven methods. The JavaScript's
//! `csvParse` and `tsvParse` and the rest are the port's
//! [`csv_parse`] and [`tsv_parse`] and the rest.
//!
//! A row of a table is a [`Row`], which is the JavaScript's own object
//! written out: the values under their names, in the order the object
//! keeps them in. A value is a [`Value`], which is the JavaScript's own
//! set of types written out — a string, a number, a boolean, nothing
//! at all, and a date.
//!
//! # Where the port differs
//!
//! - The JavaScript's rows are objects, and the port's are a [`Row`].
//!   The name `__proto__` is not a name of a row, for the JavaScript's
//!   objects set their prototype with it and have no key of their own
//!   there. The JavaScript's `format` gives the prototype of a row for
//!   a column of that name, which it writes as `[object Object]`; the
//!   port has no prototype, and writes an empty value.
//! - The JavaScript reads a date that has no timezone in it in the
//!   timezone of the browser that reads it, and the port reads it in
//!   the timezone it is given: the system's own for [`auto_type`], and
//!   the one that is given for [`auto_type_in`]. The JavaScript has a
//!   flag of its own for the timezone, which says whether the dates are
//!   to be read in the language's other way; the port reads them the
//!   same way in both, for the two ways of the JavaScript give the same
//!   time.
//! - The JavaScript counts the characters of a text in the code units
//!   of UTF-16, and the port counts them in characters. The two are the
//!   same for every text, for the characters the port counts are the
//!   characters the JavaScript counts, and the fields that hold a
//!   character outside the Basic Multilingual Plane are the same as
//!   well.
//! - The JavaScript's `autoType` fails on a value that is not a string,
//!   for it takes the spaces off it, and the port leaves such a value
//!   as it is.

use crate::js;
use crate::time::interval::Fields;
use crate::time::{Date, TimeZone};

/// The quote, which opens a field.
const QUOTE: char = '"';

/// The newline, which ends a line.
const NEWLINE: char = '\n';

/// The carriage return, which ends a line too.
const RETURN: char = '\r';

/// The limit of the range of a date, in milliseconds since the epoch.
const TIME_LIMIT: f64 = 8.64e15;

/// A value of a row of a table.
///
/// The JavaScript's values are of many kinds, and the port writes the
/// ones that a table can hold out: a string, a number, a boolean,
/// nothing at all, and a date.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A string.
    String(String),
    /// A number.
    Number(f64),
    /// A boolean.
    Bool(bool),
    /// The JavaScript's `null` and `undefined`.
    Null,
    /// A date, which is a number of milliseconds since the epoch.
    Date(Date),
}

impl Value {
    /// The value as a string, which is what the JavaScript's own
    /// `String(value)` gives.
    ///
    /// The JavaScript writes a date in the form of the specification,
    /// which is [`format_date`], and a number in the form of the
    /// specification, which is [`crate::js::number_to_string`].
    pub fn to_text(&self) -> String {
        match self {
            Value::String(text) => text.clone(),
            Value::Number(value) => js::number_to_string(*value),
            Value::Bool(value) => value.to_string(),
            Value::Null => String::new(),
            Value::Date(date) => format_date(date),
        }
    }
}

/// A row of a table: the values, under their names.
///
/// The JavaScript's row is an object, and an object keeps its keys in
/// its own order: the keys that are numbers first, in their numeric
/// order, and then the rest, in the order they were put in. The port
/// keeps that order, for the columns of a table are found from it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    /// The keys that are numbers, and their values, in their own order.
    numbers: Vec<(String, Value)>,
    /// The other keys, and their values, in the order they were put in.
    names: Vec<(String, Value)>,
}

impl Row {
    /// A row with nothing in it.
    pub fn new() -> Self {
        Row::default()
    }

    /// The number of values in the row.
    pub fn len(&self) -> usize {
        self.numbers.len() + self.names.len()
    }

    /// Whether the row has no values in it.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The value under a name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        let numbers = &self.numbers;
        let names = &self.names;
        match array_index(name) {
            Some(_) => numbers
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value),
            None => names
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value),
        }
    }

    /// The value is put under a name.
    ///
    /// A name that is there already keeps its place, for an object
    /// keeps the place of a key that is given a value again.
    ///
    /// The name `__proto__` is not a name of the row, for the
    /// JavaScript's own objects set their prototype with it and have
    /// no key of their own there — and the JavaScript's rows are
    /// objects, so a column of that name is lost.
    pub fn set(&mut self, name: &str, value: Value) {
        if name == "__proto__" {
            return;
        }
        match array_index(name) {
            Some(index) => {
                match self.numbers.iter_mut().find(|(key, _)| key == name) {
                    Some(entry) => entry.1 = value,
                    None => {
                        // The numbers are kept in their own order.
                        let at = self
                            .numbers
                            .iter()
                            .position(|(key, _)| array_index(key).unwrap_or(0) > index)
                            .unwrap_or(self.numbers.len());
                        self.numbers.insert(at, (name.to_owned(), value));
                    }
                }
            }
            None => match self.names.iter_mut().find(|(key, _)| key == name) {
                Some(entry) => entry.1 = value,
                None => self.names.push((name.to_owned(), value)),
            },
        }
    }

    /// The names of the row, in the JavaScript's own order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.numbers
            .iter()
            .map(|(name, _)| name.as_str())
            .chain(self.names.iter().map(|(name, _)| name.as_str()))
    }

    /// The values of the row and their names, in the JavaScript's own
    /// order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.numbers
            .iter()
            .map(|(name, value)| (name.as_str(), value))
            .chain(
                self.names
                    .iter()
                    .map(|(name, value)| (name.as_str(), value)),
            )
    }
}

/// The rows of a table, and the columns they have.
///
/// The JavaScript's own is an array of rows with a `columns` property
/// on it, and the port writes it out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rows {
    /// The rows of the table.
    pub rows: Vec<Row>,
    /// The columns of the table, which are the names of the first row
    /// of the text — with the repeats, for the names of the rows
    /// themselves have the repeats taken out.
    pub columns: Vec<String>,
}

/// The delimiter-separated values of a text, read and written.
#[derive(Clone, Debug)]
pub struct Dsv {
    /// The delimiter, which is the JavaScript's own string.
    delimiter: String,
    /// The first character of the delimiter, which is the one the parser
    /// looks for.
    first: Option<char>,
    /// The characters that make a value be quoted.
    quotes: Vec<(char, char)>,
    /// The characters that must be there all at once for a value to be
    /// quoted.
    after: String,
}

impl Dsv {
    /// The JavaScript's `dsv`, which the package exports as `dsvFormat`.
    ///
    /// The delimiter is a string in the JavaScript, and only its first
    /// character is the delimiter that the parser looks for; the rest
    /// of it is in the expression that says whether a value is to be
    /// quoted.
    pub fn new(delimiter: &str) -> Self {
        let (quotes, after) = quote_characters(delimiter);
        Dsv {
            delimiter: delimiter.to_owned(),
            first: delimiter.chars().next(),
            quotes,
            after,
        }
    }

    /// The JavaScript's `parse`, which reads the rows of a text under
    /// the names of its first row.
    pub fn parse(&self, text: &str) -> Rows {
        let mut rows = Vec::new();
        let mut columns = Vec::new();
        let mut header = true;
        for values in self.parse_rows(text) {
            if header {
                header = false;
                columns = values;
                continue;
            }
            rows.push(self.object_row(&columns, &values));
        }
        Rows { rows, columns }
    }

    /// The JavaScript's `parse`, with the function that reads a row.
    ///
    /// The function is given the row under the names of the columns,
    /// the number of the row among the rows of the table, and the
    /// columns themselves. A row of nothing at all is left out of the
    /// table, which is what the JavaScript's own does with a function
    /// that gives nothing back.
    pub fn parse_with(
        &self,
        text: &str,
        mut f: impl FnMut(Row, usize, &[String]) -> Option<Row>,
    ) -> Rows {
        let mut rows = Vec::new();
        let mut columns = Vec::new();
        let mut header = true;
        let mut index = 0;
        for values in self.parse_rows(text) {
            if header {
                header = false;
                columns = values;
                continue;
            }
            let object = self.object_row(&columns, &values);
            if let Some(row) = f(object, index, &columns) {
                rows.push(row);
            }
            index += 1;
        }
        Rows { rows, columns }
    }

    /// The JavaScript's `parseRows`, which reads the rows of a text.
    pub fn parse_rows(&self, text: &str) -> Vec<Vec<String>> {
        self.parse_rows_with(text, |row, _index| Some(row))
    }

    /// The JavaScript's `parseRows`, with the function that reads a row.
    ///
    /// The function is given the values of the row and the number of the
    /// row among the rows of the text, which counts the first row as
    /// well. A row of nothing at all is left out of the text's rows.
    pub fn parse_rows_with(
        &self,
        text: &str,
        mut f: impl FnMut(Vec<String>, usize) -> Option<Vec<String>>,
    ) -> Vec<Vec<String>> {
        let characters: Vec<char> = text.chars().collect();
        let mut parser = Parser::new(&characters, self.first);
        let mut rows = Vec::new();
        let mut index = 0;
        loop {
            let mut token = parser.token();
            if matches!(token, Token::Eof) {
                break;
            }
            let mut row = Vec::new();
            while !matches!(token, Token::Eol | Token::Eof) {
                if let Token::Text(text) = token {
                    row.push(text);
                }
                token = parser.token();
            }
            if let Some(row) = f(row, index) {
                rows.push(row);
            }
            index += 1;
        }
        rows
    }

    /// The JavaScript's `format`, which writes a table out.
    ///
    /// The columns are the ones that are given, and the ones that are
    /// found from the rows themselves when none are given. The names of
    /// the columns are the first line of the text, as the JavaScript's
    /// own writes them.
    ///
    /// A column that a row does not have is written as an empty value,
    /// which is what the JavaScript's own writes for a column that is
    /// not there — save for the column named `__proto__`, which the
    /// JavaScript's own gives the prototype of the row.
    pub fn format(&self, rows: &[Row], columns: Option<&[String]>) -> String {
        let columns = match columns {
            Some(columns) => columns.to_vec(),
            None => infer_columns(rows),
        };
        let header: Vec<String> = columns
            .iter()
            .map(|column| self.format_value(&Value::String(column.clone())))
            .collect();
        let mut lines = vec![header.join(&self.delimiter)];
        lines.extend(self.preformat_body(rows, &columns));
        lines.join("\n")
    }

    /// The JavaScript's `formatBody`, which writes a table out without
    /// the names of its columns.
    pub fn format_body(&self, rows: &[Row], columns: Option<&[String]>) -> String {
        let columns = match columns {
            Some(columns) => columns.to_vec(),
            None => infer_columns(rows),
        };
        self.preformat_body(rows, &columns).join("\n")
    }

    /// The JavaScript's `formatRows`, which writes rows of values out.
    pub fn format_rows(&self, rows: &[Vec<Value>]) -> String {
        rows.iter()
            .map(|row| self.format_row(row))
            .collect::<Vec<String>>()
            .join("\n")
    }

    /// The JavaScript's `formatRow`, which writes a row of values out.
    pub fn format_row(&self, row: &[Value]) -> String {
        row.iter()
            .map(|value| self.format_value(value))
            .collect::<Vec<String>>()
            .join(&self.delimiter)
    }

    /// The JavaScript's `formatValue`, which writes a value out.
    ///
    /// A value is quoted when it holds a character that must be quoted:
    /// the delimiter, a quote, a newline or a carriage return.
    pub fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Null => String::new(),
            Value::Date(date) => format_date(date),
            _ => {
                let text = value.to_text();
                if self.must_be_quoted(&text) {
                    format!("\"{}\"", text.replace('"', "\"\""))
                } else {
                    text
                }
            }
        }
    }

    /// Whether a value holds a character that must be quoted.
    fn must_be_quoted(&self, text: &str) -> bool {
        let in_class = text.chars().any(|c| {
            self.quotes
                .iter()
                .any(|(first, last)| *first <= c && c <= *last)
        });
        // The characters that are after the class of the JavaScript's
        // own expression must be there all at once.
        in_class || (!self.after.is_empty() && text.contains(&self.after))
    }

    /// The rows of a table, without the names of the columns.
    fn preformat_body(&self, rows: &[Row], columns: &[String]) -> Vec<String> {
        rows.iter()
            .map(|row| {
                columns
                    .iter()
                    .map(|column| {
                        // A column that is not there is nothing at all,
                        // which is written as an empty value.
                        let value = row.get(column).unwrap_or(&Value::Null);
                        self.format_value(value)
                    })
                    .collect::<Vec<String>>()
                    .join(&self.delimiter)
            })
            .collect()
    }

    /// The row of an object, which the JavaScript builds with an object
    /// literal.
    ///
    /// The literal is `{"name": d[0] || "", ...}`: a value that is
    /// empty is an empty string, and a column named `__proto__` is not
    /// a column of the object at all, for the literal sets the
    /// prototype of the object with it.
    fn object_row(&self, columns: &[String], values: &[String]) -> Row {
        let mut row = Row::new();
        for (index, column) in columns.iter().enumerate() {
            let value = values.get(index).map(String::as_str).unwrap_or("");
            row.set(column, Value::String(value.to_owned()));
        }
        row
    }
}

/// The JavaScript's `dsv`, which the package exports as `dsvFormat`.
pub fn dsv(delimiter: &str) -> Dsv {
    Dsv::new(delimiter)
}

/// The comma-separated values, which the JavaScript's own is the
/// delimiter of a comma.
fn csv() -> Dsv {
    Dsv::new(",")
}

/// The tab-separated values, which the JavaScript's own is the delimiter
/// of a tab.
fn tsv() -> Dsv {
    Dsv::new("\t")
}

/// The JavaScript's `csvParse`.
pub fn csv_parse(text: &str) -> Rows {
    csv().parse(text)
}

/// The JavaScript's `csvParse`, with the function that reads a row.
pub fn csv_parse_with(text: &str, f: impl FnMut(Row, usize, &[String]) -> Option<Row>) -> Rows {
    csv().parse_with(text, f)
}

/// The JavaScript's `csvParseRows`.
pub fn csv_parse_rows(text: &str) -> Vec<Vec<String>> {
    csv().parse_rows(text)
}

/// The JavaScript's `csvParseRows`, with the function that reads a row.
pub fn csv_parse_rows_with(
    text: &str,
    f: impl FnMut(Vec<String>, usize) -> Option<Vec<String>>,
) -> Vec<Vec<String>> {
    csv().parse_rows_with(text, f)
}

/// The JavaScript's `csvFormat`.
pub fn csv_format(rows: &[Row], columns: Option<&[String]>) -> String {
    csv().format(rows, columns)
}

/// The JavaScript's `csvFormatBody`.
pub fn csv_format_body(rows: &[Row], columns: Option<&[String]>) -> String {
    csv().format_body(rows, columns)
}

/// The JavaScript's `csvFormatRows`.
pub fn csv_format_rows(rows: &[Vec<Value>]) -> String {
    csv().format_rows(rows)
}

/// The JavaScript's `csvFormatRow`.
pub fn csv_format_row(row: &[Value]) -> String {
    csv().format_row(row)
}

/// The JavaScript's `csvFormatValue`.
pub fn csv_format_value(value: &Value) -> String {
    csv().format_value(value)
}

/// The JavaScript's `tsvParse`.
pub fn tsv_parse(text: &str) -> Rows {
    tsv().parse(text)
}

/// The JavaScript's `tsvParse`, with the function that reads a row.
pub fn tsv_parse_with(text: &str, f: impl FnMut(Row, usize, &[String]) -> Option<Row>) -> Rows {
    tsv().parse_with(text, f)
}

/// The JavaScript's `tsvParseRows`.
pub fn tsv_parse_rows(text: &str) -> Vec<Vec<String>> {
    tsv().parse_rows(text)
}

/// The JavaScript's `tsvParseRows`, with the function that reads a row.
pub fn tsv_parse_rows_with(
    text: &str,
    f: impl FnMut(Vec<String>, usize) -> Option<Vec<String>>,
) -> Vec<Vec<String>> {
    tsv().parse_rows_with(text, f)
}

/// The JavaScript's `tsvFormat`.
pub fn tsv_format(rows: &[Row], columns: Option<&[String]>) -> String {
    tsv().format(rows, columns)
}

/// The JavaScript's `tsvFormatBody`.
pub fn tsv_format_body(rows: &[Row], columns: Option<&[String]>) -> String {
    tsv().format_body(rows, columns)
}

/// The JavaScript's `tsvFormatRows`.
pub fn tsv_format_rows(rows: &[Vec<Value>]) -> String {
    tsv().format_rows(rows)
}

/// The JavaScript's `tsvFormatRow`.
pub fn tsv_format_row(row: &[Value]) -> String {
    tsv().format_row(row)
}

/// The JavaScript's `tsvFormatValue`.
pub fn tsv_format_value(value: &Value) -> String {
    tsv().format_value(value)
}

/// The JavaScript's `autoType`, which reads the values of a row.
///
/// The values of the row are read as they are written: a value that is
/// empty is nothing at all, `true` and `false` are booleans, `NaN` is
/// a number that is not a number, a value that spells a number is that
/// number, and a value that spells a date is that date. A value that is
/// none of these is left as it is.
pub fn auto_type(row: &mut Row) {
    auto_type_in(TimeZone::Local, row)
}

/// The JavaScript's `autoType`, with the timezone the dates are read in.
///
/// The JavaScript reads a date that has no timezone in it in the
/// timezone of the program that reads it, and the port's is the
/// timezone it is given — the system's own for [`auto_type`].
pub fn auto_type_in(timezone: TimeZone, row: &mut Row) {
    let names: Vec<String> = row.names().map(|name| name.to_owned()).collect();
    for name in names {
        let text = match row.get(&name) {
            Some(Value::String(text)) => text.clone(),
            // The JavaScript's own would fail on a value that is not a
            // string, for it takes the spaces off it.
            _ => continue,
        };
        let trimmed = js_trim(&text);
        let value = if trimmed.is_empty() {
            Value::Null
        } else if trimmed == "true" {
            Value::Bool(true)
        } else if trimmed == "false" {
            Value::Bool(false)
        } else if trimmed == "NaN" {
            Value::Number(f64::NAN)
        } else {
            let number = js_to_number(trimmed);
            if !number.is_nan() {
                Value::Number(number)
            } else if let Some(date) = parse_date(trimmed, timezone) {
                Value::Date(date)
            } else {
                // The value is left as it is.
                continue;
            }
        };
        row.set(&name, value);
    }
}

/// The columns of the rows, which are found from the rows themselves.
///
/// The names are in the order the rows have them in, and a name that is
/// there already is not named twice.
fn infer_columns(rows: &[Row]) -> Vec<String> {
    let mut columns: Vec<String> = Vec::new();
    for row in rows {
        for name in row.names() {
            if !columns.iter().any(|column| column == name) {
                columns.push(name.to_owned());
            }
        }
    }
    columns
}

/// The characters that make a value be quoted, which the JavaScript
/// finds with an expression it builds out of the delimiter, and the
/// characters that must be there all at once for it as well.
///
/// The expression is `["` + the delimiter + `\n\r]`, and the regular
/// expression reads it as a class of characters: the quote, the
/// delimiter's own characters, the newline and the carriage return. A
/// `-` between two of them makes a range of characters, and a `\`
/// takes the character after it as it is. A `]` ends the class early,
/// and what is after it is a sequence of characters that must be there
/// all at once — as the JavaScript's own expression does.
fn quote_characters(delimiter: &str) -> (Vec<(char, char)>, String) {
    let pattern: Vec<char> = format!("[\"{delimiter}\n\r]").chars().collect();
    let mut characters: Vec<(char, char)> = Vec::new();
    let mut after = String::new();
    let mut i = 1;
    let mut in_class = true;
    while i < pattern.len() {
        let c = pattern[i];
        // A backslash takes the character after it as it is.
        let (literal, escaped) = if c == '\\' {
            i += 1;
            (pattern.get(i).copied().unwrap_or('\\'), true)
        } else {
            (c, false)
        };
        i += 1;
        if !in_class {
            after.push(literal);
            continue;
        }
        if literal == ']' && !escaped {
            // The class ends here.
            in_class = false;
            continue;
        }
        // A range, when a `-` is between two characters.
        if pattern.get(i) == Some(&'-') && pattern.get(i + 1).is_some_and(|c| *c != ']') {
            i += 1;
            let last = if pattern[i] == '\\' {
                i += 1;
                pattern.get(i).copied().unwrap_or('\\')
            } else {
                pattern[i]
            };
            i += 1;
            characters.push((literal, last));
        } else {
            characters.push((literal, literal));
        }
    }
    (characters, after)
}

/// Whether a name is one of the JavaScript's own array indexes, which
/// are the keys that an object keeps in their numeric order.
fn array_index(name: &str) -> Option<u32> {
    if name.is_empty() || name.len() > 10 {
        return None;
    }
    if name.len() > 1 && name.starts_with('0') {
        return None;
    }
    if !name.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let value: u64 = name.parse().ok()?;
    // The last array index is one less than the largest number a name
    // may have, for the length of an array is that number.
    if value >= u32::MAX as u64 {
        return None;
    }
    Some(value as u32)
}

/// The JavaScript's `trim`, which takes the spaces off both ends.
///
/// The JavaScript's own spaces are the white space of the language and
/// the line endings of it, and the byte order mark is one of them —
/// which Rust's own `trim` does not take off, for the byte order mark
/// is not white space in Unicode.
fn js_trim(text: &str) -> &str {
    text.trim_matches(is_javascript_space)
}

/// Whether a character is one of the JavaScript's own spaces.
fn is_javascript_space(c: char) -> bool {
    matches!(
        c,
        '\u{9}'
            | '\u{a}'
            | '\u{b}'
            | '\u{c}'
            | '\u{d}'
            | '\u{20}'
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// The JavaScript's `+x` for a string, which is the number the string
/// spells.
///
/// A string that spells no number is a number that is not a number,
/// and the numbers that are written in another base are read in it.
fn js_to_number(text: &str) -> f64 {
    let text = js_trim(text);
    if text.is_empty() {
        return 0.0;
    }
    if text == "Infinity" || text == "+Infinity" {
        return f64::INFINITY;
    }
    if text == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    if let Some(digits) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        return from_radix(digits, 16);
    }
    if let Some(digits) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
        return from_radix(digits, 8);
    }
    if let Some(digits) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
        return from_radix(digits, 2);
    }
    if is_decimal(text) {
        // The number is read as the language reads it, which rounds it
        // to the nearest one.
        return text.parse().unwrap_or(f64::NAN);
    }
    f64::NAN
}

/// The number that the digits spell in another base.
fn from_radix(digits: &str, radix: u32) -> f64 {
    if digits.is_empty() {
        return f64::NAN;
    }
    let mut value: u128 = 0;
    for c in digits.chars() {
        let digit = match c.to_digit(radix) {
            Some(digit) => digit,
            None => return f64::NAN,
        };
        match value
            .checked_mul(radix as u128)
            .and_then(|value| value.checked_add(digit as u128))
        {
            Some(next) => value = next,
            None => {
                // The number is too large for the count, and is read in
                // the number's own way, which rounds it as it goes —
                // where the JavaScript rounds it once, at the end.
                return digits.chars().fold(0.0, |value, c| {
                    value * radix as f64 + c.to_digit(radix).unwrap_or(0) as f64
                });
            }
        }
    }
    value as f64
}

/// Whether the text is a number the language writes.
fn is_decimal(text: &str) -> bool {
    let characters: Vec<char> = text.chars().collect();
    let mut i = 0;
    if matches!(characters.get(i), Some('+') | Some('-')) {
        i += 1;
    }
    let mut whole = 0;
    while matches!(characters.get(i), Some(c) if c.is_ascii_digit()) {
        i += 1;
        whole += 1;
    }
    let mut fraction = 0;
    if characters.get(i) == Some(&'.') {
        i += 1;
        while matches!(characters.get(i), Some(c) if c.is_ascii_digit()) {
            i += 1;
            fraction += 1;
        }
    }
    if whole == 0 && fraction == 0 {
        return false;
    }
    if matches!(characters.get(i), Some('e') | Some('E')) {
        i += 1;
        if matches!(characters.get(i), Some('+') | Some('-')) {
            i += 1;
        }
        let mut exponent = 0;
        while matches!(characters.get(i), Some(c) if c.is_ascii_digit()) {
            i += 1;
            exponent += 1;
        }
        if exponent == 0 {
            return false;
        }
    }
    i == characters.len()
}

/// The date of a text, which is the JavaScript's own `new Date(text)`.
///
/// The JavaScript reads a date of the format the specification gives —
/// the year, the month, the day, the hour, the minute, the second and
/// the millisecond, and an offset from Greenwich — and a date without
/// a time in it is read in Greenwich, and a date with a time in it and
/// no offset is read in the timezone of the program that reads it.
///
/// The JavaScript has a flag of its own for the timezone, which says
/// whether the dates are to be read in the language's other way; the
/// port reads them the same way in both, for the two ways of the
/// JavaScript give the same time.
fn parse_date(text: &str, timezone: TimeZone) -> Option<Date> {
    let parts = date_parts(text)?;

    // The parts that are out of their own ranges make a date that is
    // not a date.
    let invalid = parts.month < 1
        || parts.month > 12
        || parts.day < 1
        || parts.day > 31
        || parts.hour > 24
        || parts.minute > 59
        || parts.second > 59
        || (parts.hour == 24 && (parts.minute != 0 || parts.second != 0))
        || parts
            .offset
            .is_some_and(|offset| offset.unsigned_abs() > 1439);

    if invalid {
        return Some(Date {
            instant: f64::NAN,
            timezone,
        });
    }

    let fields = Fields {
        year: parts.year,
        month: parts.month - 1,
        day: parts.day,
        weekday: 0,
        hour: parts.hour,
        minute: parts.minute,
        second: parts.second,
        millisecond: parts.millisecond,
    };

    // The date in Greenwich, which is the way a date without a time in
    // it is read.
    let mut utc = Date::utc(0.0);
    utc.set_fields(fields);

    let instant = match parts.offset {
        // The offset is taken off the time of Greenwich.
        Some(offset) => utc.instant - offset as f64 * 60_000.0,
        None => {
            if parts.time {
                // A time without an offset is the local time.
                let mut local = Date::new(0.0, timezone);
                local.set_fields(fields);
                local.instant
            } else {
                utc.instant
            }
        }
    };

    // The range of a date is limited.
    if instant.abs() > TIME_LIMIT {
        return Some(Date {
            instant: f64::NAN,
            timezone,
        });
    }
    Some(Date { instant, timezone })
}

/// The parts of a date, read out of the text the JavaScript's format
/// gives.
struct DateParts {
    /// The year.
    year: i64,
    /// The month, from one.
    month: u32,
    /// The day of the month, from one.
    day: u32,
    /// The hour.
    hour: u32,
    /// The minute.
    minute: u32,
    /// The second.
    second: u32,
    /// The millisecond.
    millisecond: u32,
    /// The offset from Greenwich, in minutes, if the text gives one.
    offset: Option<i32>,
    /// Whether the text has a time in it.
    time: bool,
}

/// The parts of a date, read out of the text the JavaScript's format
/// gives, which is the year, the month, the day, the hour, the minute,
/// the second, the millisecond and an offset from Greenwich.
///
/// The year may have a sign in front of it, and then it has six digits.
fn date_parts(text: &str) -> Option<DateParts> {
    let characters: Vec<char> = text.chars().collect();
    let mut i = 0;

    let year = match characters.get(i) {
        Some('+') | Some('-') => {
            let sign = if characters[i] == '-' { -1 } else { 1 };
            i += 1;
            let high = digits(&characters, &mut i, 2)?;
            let low = digits(&characters, &mut i, 4)?;
            sign * (high * 10000 + low)
        }
        _ => digits(&characters, &mut i, 4)?,
    };

    let mut month = 1;
    let mut day = 1;
    if characters.get(i) == Some(&'-') {
        i += 1;
        month = digits(&characters, &mut i, 2)? as u32;
        if characters.get(i) == Some(&'-') {
            i += 1;
            day = digits(&characters, &mut i, 2)? as u32;
        }
    }

    let mut hour = 0;
    let mut minute = 0;
    let mut second = 0;
    let mut millisecond = 0;
    let mut offset = None;
    let mut time = false;
    if characters.get(i) == Some(&'T') {
        i += 1;
        time = true;
        hour = digits(&characters, &mut i, 2)? as u32;
        if characters.get(i) != Some(&':') {
            return None;
        }
        i += 1;
        minute = digits(&characters, &mut i, 2)? as u32;
        if characters.get(i) == Some(&':') {
            i += 1;
            second = digits(&characters, &mut i, 2)? as u32;
            if characters.get(i) == Some(&'.') {
                i += 1;
                millisecond = digits(&characters, &mut i, 3)? as u32;
            }
        }
        if characters.get(i) == Some(&'Z') {
            i += 1;
            offset = Some(0);
        } else if matches!(characters.get(i), Some('+') | Some('-')) {
            let sign = if characters[i] == '-' { -1 } else { 1 };
            i += 1;
            let hours = digits(&characters, &mut i, 2)? as i32;
            if characters.get(i) != Some(&':') {
                return None;
            }
            i += 1;
            let minutes = digits(&characters, &mut i, 2)? as i32;
            offset = Some(sign * (hours * 60 + minutes));
        }
    }

    if i != characters.len() {
        return None;
    }

    Some(DateParts {
        year,
        month,
        day,
        hour,
        minute,
        second,
        millisecond,
        offset,
        time,
    })
}

/// The digits of the text, which must be there and must be digits.
fn digits(characters: &[char], i: &mut usize, count: usize) -> Option<i64> {
    let mut value = 0i64;
    for _ in 0..count {
        let c = *characters.get(*i)?;
        if !c.is_ascii_digit() {
            return None;
        }
        value = value * 10 + c.to_digit(10)? as i64;
        *i += 1;
    }
    Some(value)
}

/// The JavaScript's `formatDate`, which writes a date out.
///
/// The date is written in the form of the specification, and the time of
/// it is left out when it is the midnight of the day.
fn format_date(date: &Date) -> String {
    if date.instant.is_nan() {
        return "Invalid Date".to_owned();
    }
    // The JavaScript's own dates are whole numbers of milliseconds, for
    // the number is made one when the date is made.
    let fields = Date::new(date.instant.trunc(), TimeZone::Utc).fields();
    let mut text = format_year(fields.year);
    text.push('-');
    text.push_str(&pad(fields.month as i64 + 1, 2));
    text.push('-');
    text.push_str(&pad(fields.day as i64, 2));
    if fields.millisecond != 0 {
        text.push('T');
        text.push_str(&pad(fields.hour as i64, 2));
        text.push(':');
        text.push_str(&pad(fields.minute as i64, 2));
        text.push(':');
        text.push_str(&pad(fields.second as i64, 2));
        text.push('.');
        text.push_str(&pad(fields.millisecond as i64, 3));
        text.push('Z');
    } else if fields.second != 0 {
        text.push('T');
        text.push_str(&pad(fields.hour as i64, 2));
        text.push(':');
        text.push_str(&pad(fields.minute as i64, 2));
        text.push(':');
        text.push_str(&pad(fields.second as i64, 2));
        text.push('Z');
    } else if fields.minute != 0 || fields.hour != 0 {
        text.push('T');
        text.push_str(&pad(fields.hour as i64, 2));
        text.push(':');
        text.push_str(&pad(fields.minute as i64, 2));
        text.push('Z');
    }
    text
}

/// The year of a date, which is written with its sign when it is out
/// of the range of the years that are written with four digits.
fn format_year(year: i64) -> String {
    if year < 0 {
        format!("-{}", pad(-year, 6))
    } else if year > 9999 {
        format!("+{}", pad(year, 6))
    } else {
        pad(year, 4)
    }
}

/// The number, written with zeros in front of it, to the width that is
/// asked for.
fn pad(value: i64, width: usize) -> String {
    let text = value.to_string();
    if text.len() < width {
        format!("{}{}", "0".repeat(width - text.len()), text)
    } else {
        text
    }
}

/// A token of the parser, which is the JavaScript's own.
enum Token {
    /// The value of a field.
    Text(String),
    /// The end of a line.
    Eol,
    /// The end of the text.
    Eof,
}

/// The parser of a text, which is the JavaScript's own state machine.
struct Parser<'a> {
    /// The characters of the text.
    text: &'a [char],
    /// The number of characters of the text.
    len: usize,
    /// The number of characters, with the newline at the end taken off.
    n: usize,
    /// The character the parser is at.
    i: usize,
    /// Whether the token is followed by the end of the text.
    eof: bool,
    /// Whether the token is followed by the end of a line.
    eol: bool,
    /// The delimiter, which is the first character of the JavaScript's own.
    delimiter: Option<char>,
}

impl<'a> Parser<'a> {
    /// The parser of a text.
    fn new(text: &'a [char], delimiter: Option<char>) -> Self {
        // The newline at the end of the text is taken off, for a text
        // that ends with one has no row after it.
        let mut n = text.len();
        if n > 0 && text[n - 1] == NEWLINE {
            n -= 1;
        }
        if n > 0 && text[n - 1] == RETURN {
            n -= 1;
        }
        Parser {
            text,
            len: text.len(),
            n,
            i: 0,
            // The end of the text is looked for before the newline at
            // the end of it is taken off.
            eof: text.is_empty(),
            eol: false,
            delimiter,
        }
    }

    /// The next token of the text, which is the JavaScript's own.
    fn token(&mut self) -> Token {
        if self.eof {
            return Token::Eof;
        }
        if self.eol {
            self.eol = false;
            return Token::Eol;
        }

        // The character the token begins at.
        let j = self.i;

        // A quote opens a field, and a doubled quote is one quote.
        if self.text.get(j) == Some(&QUOTE) {
            // while (I++ < N && text.charCodeAt(I) !== QUOTE || text.charCodeAt(++I) === QUOTE);
            loop {
                let within = self.i < self.n;
                self.i += 1;
                if within && self.text.get(self.i) != Some(&QUOTE) {
                    // The first part of the expression is true, and the
                    // second is not looked at.
                    continue;
                }
                self.i += 1;
                if self.text.get(self.i) == Some(&QUOTE) {
                    // A doubled quote is one quote, and the field goes
                    // on.
                    continue;
                }
                break;
            }
            // The character after the quote that ends the field.
            let i = self.i;
            if i >= self.n {
                self.eof = true;
            } else {
                let c = self.text[self.i];
                self.i += 1;
                if c == NEWLINE {
                    self.eol = true;
                } else if c == RETURN {
                    self.eol = true;
                    if self.text.get(self.i) == Some(&NEWLINE) {
                        self.i += 1;
                    }
                }
            }
            // The value is what is between the quotes, with the doubled
            // quotes made single. The slice is of the whole text, and
            // not of the text with the newline at the end taken off, for
            // the JavaScript's own slice is of the whole text as well.
            let start = (j + 1).min(self.len);
            let end = i.saturating_sub(1).min(self.len).max(start);
            let value: String = self.text[start..end].iter().collect();
            return Token::Text(value.replace("\"\"", "\""));
        }

        // The value runs to the delimiter or to the end of the line.
        while self.i < self.n {
            let i = self.i;
            self.i += 1;
            let c = self.text[i];
            if c == NEWLINE {
                self.eol = true;
            } else if c == RETURN {
                self.eol = true;
                if self.text.get(self.i) == Some(&NEWLINE) {
                    self.i += 1;
                }
            } else if Some(c) != self.delimiter {
                continue;
            }
            return Token::Text(self.text[j..i].iter().collect());
        }

        // The value runs to the end of the text.
        self.eof = true;
        Token::Text(self.text[j..self.n].iter().collect())
    }
}
