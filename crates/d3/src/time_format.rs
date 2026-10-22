//! A port of **d3-time-format**, from the JavaScript of the same name.
//!
//! Translated from `d3-time-format/src/`, version 2.3.0, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read
//! differently, the JavaScript is right.
//!
//! A format is a specification — `%Y-%m-%d` — and a date, and it gives
//! the text of the date. A parse is a specification and a text, and it
//! gives the date. The JavaScript's own dates are read in the timezone
//! of the browser that is running them; the port's dates are read in the
//! timezone they carry, which is the system's own for a program that
//! runs natively — see [`crate::time::zone`].
//!
//! The JavaScript's own module holds a locale, and
//! [`time_format_default_locale`] changes it for everything that reads
//! the module's own formats and parses ✓; the port holds it in the same
//! way ✓, under a lock ✓, and the formats and the parses made before
//! the change keep the locale they were made with ✓.
//!
//! The JavaScript's own [`iso_parse`] reads the texts of the ISO form
//! with the JavaScript's own `new Date` ✓, which falls back upon a
//! reader of its own for the texts that are not in that form ✓ — the
//! port reads the ISO form itself ✓, and gives `None` for the others ✓.

use std::sync::{OnceLock, RwLock};

use crate::js::number_to_string;
use crate::time::interval::{
    length_of_month, time_day, time_monday, time_sunday, time_thursday, time_year, Fields,
};
use crate::time::{Date, TimeZone};

/// The number of milliseconds in a day ✓.
const DURATION_DAY: f64 = 86_400_000.0;

/// The number of milliseconds the JavaScript's own dates can hold ✓.
///
/// A date beyond it is not a date at all: the JavaScript's own `Date`
/// gives `NaN` for it, and the port's date gives the same ✓.
const MAX_TIME: f64 = 8.64e15;

/// The number of milliseconds in an hour ✓.
const DURATION_HOUR: f64 = 3_600_000.0;

/// The number of milliseconds in a minute ✓.
const DURATION_MINUTE: f64 = 60_000.0;

/// The number of milliseconds in a second ✓.
const DURATION_SECOND: f64 = 1_000.0;

/// A locale, which the JavaScript's own takes as an object ✓.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocaleDefinition {
    /// The date and the time, which the `%c` directive writes ✓.
    pub date_time: String,
    /// The date, which the `%x` directive writes ✓.
    pub date: String,
    /// The time, which the `%X` directive writes ✓.
    pub time: String,
    /// The periods of the day, which are the morning and the evening ✓.
    pub periods: Vec<String>,
    /// The days of the week, with the Sunday first ✓.
    pub days: Vec<String>,
    /// The days of the week in their short names, with the Sunday
    /// first ✓.
    pub short_days: Vec<String>,
    /// The months of the year, with the January first ✓.
    pub months: Vec<String>,
    /// The months of the year in their short names, with the January
    /// first ✓.
    pub short_months: Vec<String>,
}

/// The locale the JavaScript has by default ✓.
pub fn default_definition() -> LocaleDefinition {
    LocaleDefinition {
        date_time: "%x, %X".to_owned(),
        date: "%-m/%-d/%Y".to_owned(),
        time: "%-I:%M:%S %p".to_owned(),
        periods: vec!["AM".to_owned(), "PM".to_owned()],
        days: [
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ]
        .iter()
        .map(|day| (*day).to_owned())
        .collect(),
        short_days: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
            .iter()
            .map(|day| (*day).to_owned())
            .collect(),
        months: [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ]
        .iter()
        .map(|month| (*month).to_owned())
        .collect(),
        short_months: [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ]
        .iter()
        .map(|month| (*month).to_owned())
        .collect(),
    }
}

/// A locale, which gives the formats ✓.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locale {
    /// The names and the specifications of the locale ✓.
    definition: LocaleDefinition,
}

impl Locale {
    /// The format of a specification, which writes a date ✓.
    pub fn format(&self, specifier: &str) -> Format {
        Format {
            specifier: specifier.to_owned(),
            locale: self.clone(),
            utc: false,
        }
    }

    /// The parse of a specification, which reads a date ✓.
    pub fn parse(&self, specifier: &str) -> Parse {
        Parse {
            specifier: specifier.to_owned(),
            locale: self.clone(),
            utc: false,
        }
    }

    /// The format of a specification, which writes a date in Greenwich ✓.
    pub fn utc_format(&self, specifier: &str) -> Format {
        Format {
            specifier: specifier.to_owned(),
            locale: self.clone(),
            utc: true,
        }
    }

    /// The parse of a specification, which reads a date in Greenwich ✓.
    pub fn utc_parse(&self, specifier: &str) -> Parse {
        Parse {
            specifier: specifier.to_owned(),
            locale: self.clone(),
            utc: true,
        }
    }
}

/// A locale of one's own ✓.
///
/// This is the JavaScript's `timeFormatLocale` ✓.
pub fn time_format_locale(definition: &LocaleDefinition) -> Locale {
    Locale {
        definition: definition.clone(),
    }
}

/// The default locale, which the JavaScript's own changes ✓.
static DEFAULT: OnceLock<RwLock<Locale>> = OnceLock::new();

/// The default locale, and the lock it is held under ✓.
fn default_locale() -> &'static RwLock<Locale> {
    DEFAULT.get_or_init(|| {
        RwLock::new(Locale {
            definition: default_definition(),
        })
    })
}

/// The locale the JavaScript has by default, which
/// [`time_format_default_locale`] changes ✓.
///
/// This is the JavaScript's `timeFormatDefaultLocale` ✓. The locale it
/// sets is the one the module's own `timeFormat`, `timeParse`,
/// `utcFormat` and `utcParse` use afterwards ✓, and the formats and
/// the parses made before it keep the locale they were made with ✓.
pub fn time_format_default_locale(definition: &LocaleDefinition) -> Locale {
    let locale = time_format_locale(definition);
    let mut held = default_locale().write().expect("the default locale's lock");
    *held = locale.clone();
    locale
}

/// The format of a specification, in the default locale ✓.
///
/// This is the JavaScript's `timeFormat` ✓.
pub fn time_format(specifier: &str) -> Format {
    default_locale()
        .read()
        .expect("the default locale's lock")
        .format(specifier)
}

/// The parse of a specification, in the default locale ✓.
///
/// This is the JavaScript's `timeParse` ✓.
pub fn time_parse(specifier: &str) -> Parse {
    default_locale()
        .read()
        .expect("the default locale's lock")
        .parse(specifier)
}

/// The format of a specification, in the default locale and in Greenwich ✓.
///
/// This is the JavaScript's `utcFormat` ✓.
pub fn utc_format(specifier: &str) -> Format {
    default_locale()
        .read()
        .expect("the default locale's lock")
        .utc_format(specifier)
}

/// The parse of a specification, in the default locale and in Greenwich ✓.
///
/// This is the JavaScript's `utcParse` ✓.
pub fn utc_parse(specifier: &str) -> Parse {
    default_locale()
        .read()
        .expect("the default locale's lock")
        .utc_parse(specifier)
}

/// A format, which writes a date ✓.
#[derive(Clone, Debug)]
pub struct Format {
    /// The specification the format was made for ✓.
    specifier: String,
    /// The locale the names are written in ✓.
    locale: Locale,
    /// Whether the date is written in Greenwich ✓.
    utc: bool,
}

impl Format {
    /// The date, as the JavaScript writes it ✓.
    pub fn format(&self, date: &Date) -> String {
        self.write(&self.specifier, date)
    }

    /// The number of milliseconds since the epoch, as the JavaScript
    /// writes it ✓.
    ///
    /// The JavaScript's own formats take a number as well as a date,
    /// and make a date of it ✓.
    pub fn format_instant(&self, instant: f64) -> String {
        self.format(&Date::local(instant))
    }

    /// The specification the format was made for ✓.
    ///
    /// The JavaScript's own `toString` gives the specification back ✓.
    pub fn specifier(&self) -> &str {
        &self.specifier
    }

    /// The date as the format reads it ✓.
    ///
    /// The UTC formats read the date in Greenwich, and the others read
    /// it in the timezone the date carries ✓.
    fn reading(&self, date: &Date) -> Date {
        if self.utc {
            Date::utc(date.instant)
        } else {
            *date
        }
    }

    /// The text of a specification, which is the JavaScript's own
    /// `newFormat` ✓.
    fn write(&self, specifier: &str, date: &Date) -> String {
        let characters: Vec<char> = specifier.chars().collect();
        let mut out = String::new();
        let mut i = 0usize;
        // Where the text that is written as it is begins ✓.
        let mut j = 0usize;
        while i < characters.len() {
            if characters[i] != '%' {
                i += 1;
                continue;
            }
            // The text of the specification, which may be past its end
            // when the percent is the last of it ✓.
            out.extend(characters[j..i.min(characters.len())].iter());
            i += 1;
            // The character after the percent, which may be the fill the
            // number is written with ✓.
            let mut directive = characters.get(i).copied();
            let fill = match directive {
                Some('-') => {
                    i += 1;
                    directive = characters.get(i).copied();
                    String::new()
                }
                Some('_') => {
                    i += 1;
                    directive = characters.get(i).copied();
                    " ".to_owned()
                }
                Some('0') => {
                    i += 1;
                    directive = characters.get(i).copied();
                    "0".to_owned()
                }
                // The day of the month is written with a space, and not
                // with a zero ✓.
                Some('e') => " ".to_owned(),
                _ => "0".to_owned(),
            };
            // A directive the JavaScript does not know is written as it
            // is ✓.
            if let Some(directive) = directive {
                match self.directive(directive, date, &fill) {
                    Some(text) => out.push_str(&text),
                    None => out.push(directive),
                }
            }
            i += 1;
            j = i;
        }
        out.extend(characters[j.min(characters.len())..].iter());
        out
    }

    /// The text of a directive, which is the JavaScript's own `formats`
    /// ✓.
    fn directive(&self, directive: char, date: &Date, fill: &str) -> Option<String> {
        let reading = self.reading(date);
        let parts = Parts::of(&reading);
        Some(match directive {
            'a' => name(&self.locale.definition.short_days, parts.weekday),
            'A' => name(&self.locale.definition.days, parts.weekday),
            'b' => name(&self.locale.definition.short_months, parts.month),
            'B' => name(&self.locale.definition.months, parts.month),
            'c' => self.write(&self.locale.definition.date_time, date),
            'd' | 'e' => pad(parts.day, fill, 2),
            'f' => format!("{}000", pad(parts.millisecond, fill, 3)),
            'g' => pad(self.iso_year(date) % 100.0, fill, 2),
            'G' => pad(self.iso_year(date) % 10_000.0, fill, 4),
            'H' => pad(parts.hour, fill, 2),
            'I' => pad(or(parts.hour % 12.0, 12.0), fill, 2),
            'j' => pad(self.day_of_year(date), fill, 3),
            'L' => pad(parts.millisecond, fill, 3),
            'm' => pad(parts.month + 1.0, fill, 2),
            'M' => pad(parts.minute, fill, 2),
            'p' => name(
                &self.locale.definition.periods,
                if parts.hour >= 12.0 { 1.0 } else { 0.0 },
            ),
            'q' => number_to_string(1.0 + (parts.month / 3.0).trunc() as i64 as f64),
            'Q' => number_to_string(reading.instant),
            's' => number_to_string((reading.instant / 1_000.0).floor()),
            'S' => pad(parts.second, fill, 2),
            'u' => number_to_string(if parts.weekday == 0.0 {
                7.0
            } else {
                parts.weekday
            }),
            'U' => pad(self.week_number(date, false), fill, 2),
            'V' => pad(self.iso_week(date), fill, 2),
            'w' => number_to_string(parts.weekday),
            'W' => pad(self.week_number(date, true), fill, 2),
            'x' => self.write(&self.locale.definition.date, date),
            'X' => self.write(&self.locale.definition.time, date),
            'y' => pad(parts.year % 100.0, fill, 2),
            'Y' => pad(parts.year % 10_000.0, fill, 4),
            // The zone of Greenwich is always the same ✓.
            'Z' => {
                if self.utc {
                    "+0000".to_owned()
                } else {
                    zone(&reading)
                }
            }
            '%' => "%".to_owned(),
            _ => return None,
        })
    }

    /// The day of the year of the date ✓.
    ///
    /// The JavaScript counts the days between the start of the year and
    /// the date, with the change of the offset taken out ✓; the number
    /// of the day of the local calendar is the same number ✓.
    fn day_of_year(&self, date: &Date) -> f64 {
        let reading = self.reading(date);
        if reading.instant.is_nan() {
            return f64::NAN;
        }
        let mut year = reading;
        time_year().floor(&mut year);
        (day_number(&reading) - day_number(&year) + 1) as f64
    }

    /// The week of the year of the date ✓.
    ///
    /// The week is counted from the Sunday, or from the Monday ✓. The
    /// count begins at the Sunday, or the Monday, of the week that the
    /// start of the year is in ✓ — and the start of the year is a
    /// millisecond before the week that is counted from, for a year that
    /// begins on the day itself is in the week before ✓.
    fn week_number(&self, date: &Date, monday: bool) -> f64 {
        let reading = self.reading(date);
        if reading.instant.is_nan() {
            return f64::NAN;
        }
        let mut year = reading;
        time_year().floor(&mut year);
        year.instant -= 1.0;
        let mut start = year;
        floor_to_week(&mut start, monday);
        let mut end = reading;
        floor_to_week(&mut end, monday);
        ((day_number(&end) - day_number(&start)) / 7) as f64
    }

    /// The Thursday of the week the date is in ✓.
    ///
    /// This is the JavaScript's own `dISO` ✓: a date that is on the
    /// Thursday or after it belongs to the week of that Thursday, and a
    /// date before it belongs to the week of the Thursday that follows
    /// ✓.
    fn iso_day(&self, date: &Date) -> Date {
        let mut reading = self.reading(date);
        if reading.instant.is_nan() {
            // A date that is not a date has no Thursday ✓.
            return reading;
        }
        let weekday = reading.fields().weekday;
        if weekday >= 4 || weekday == 0 {
            time_thursday().floor(&mut reading);
        } else {
            time_thursday().ceil(&mut reading);
        }
        reading
    }

    /// The week of the year of the ISO ✓.
    ///
    /// The JavaScript counts the Thursdays between the start of the year
    /// and the Thursday of the week, and adds one when the year begins
    /// on a Thursday ✓, for the week of the first Thursday is the first
    /// week of the year ✓.
    fn iso_week(&self, date: &Date) -> f64 {
        let thursday = self.iso_day(date);
        if thursday.instant.is_nan() {
            return f64::NAN;
        }
        let mut year = thursday;
        time_year().floor(&mut year);
        let mut start = year;
        time_thursday().floor(&mut start);
        let count = (day_number(&thursday) - day_number(&start)) / 7;
        (count + i64::from(year.fields().weekday == 4)) as f64
    }

    /// The year of the week the date is in ✓.
    ///
    /// This is the year of the Thursday of the week, for the week of a
    /// date that is at the end of a year may be a week of the year
    /// beside it ✓.
    fn iso_year(&self, date: &Date) -> f64 {
        let thursday = self.iso_day(date);
        if thursday.instant.is_nan() {
            return f64::NAN;
        }
        thursday.fields().year as f64
    }
}

/// The number of the day of the date, which the local calendar counts ✓.
fn day_number(date: &Date) -> i64 {
    (date.local_instant() / DURATION_DAY).floor() as i64
}

/// The date put at the start of its week, which begins on the Sunday or
/// on the Monday ✓.
fn floor_to_week(date: &mut Date, monday: bool) {
    if monday {
        time_monday().floor(date);
    } else {
        time_sunday().floor(date);
    }
}

/// The name at a place in a list ✓.
///
/// The JavaScript writes nothing at all when the list is too short ✓,
/// for the name is not there and the text of the format joins it as
/// nothing ✓.
fn name(list: &[String], index: f64) -> String {
    if index.is_nan() || index < 0.0 || index >= list.len() as f64 {
        return String::new();
    }
    list[index as usize].clone()
}

/// The JavaScript's `x || y` ✓, which takes the second when the first
/// is zero or is not a number ✓.
fn or(value: f64, other: f64) -> f64 {
    if value == 0.0 || value.is_nan() {
        other
    } else {
        value
    }
}

/// The number, written with a fill ✓.
///
/// This is the JavaScript's own `pad` ✓: the sign of the number is
/// written before the fill, and the fill is of the digits and not of
/// the sign ✓.
fn pad(value: f64, fill: &str, width: usize) -> String {
    let negative = value < 0.0;
    let digits = number_to_string(if negative { -value } else { value });
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    let length = digits.chars().count();
    if length < width {
        out.push_str(&fill.repeat(width - length));
    }
    out.push_str(&digits);
    out
}

/// The offset from Greenwich of the date, as the JavaScript writes it ✓.
fn zone(date: &Date) -> String {
    if date.instant.is_nan() {
        // The offset of a date that is not a date is not a number, and
        // the hours of it are written as a zero ✓.
        return format!("+{}{}", pad(0.0, "0", 2), pad(f64::NAN, "0", 2));
    }
    let offset = date.offset();
    let (sign, offset) = if offset < 0 {
        ("-", -offset)
    } else {
        ("+", offset)
    };
    format!(
        "{}{}{}",
        sign,
        pad((offset / 60) as f64, "0", 2),
        pad((offset % 60) as f64, "0", 2)
    )
}

/// The parts of a date, as the JavaScript's own getters give them ✓.
///
/// A date that is not a date at all gives `NaN` for every part of it,
/// as the JavaScript's own getters do ✓.
#[derive(Clone, Copy, Debug)]
struct Parts {
    /// The year ✓.
    year: f64,
    /// The month, from zero ✓.
    month: f64,
    /// The day of the month, from one ✓.
    day: f64,
    /// The day of the week, from zero, with the Sunday first ✓.
    weekday: f64,
    /// The hour ✓.
    hour: f64,
    /// The minute ✓.
    minute: f64,
    /// The second ✓.
    second: f64,
    /// The millisecond ✓.
    millisecond: f64,
}

impl Parts {
    /// The parts of the date ✓.
    fn of(date: &Date) -> Parts {
        if date.instant.is_nan() {
            return Parts {
                year: f64::NAN,
                month: f64::NAN,
                day: f64::NAN,
                weekday: f64::NAN,
                hour: f64::NAN,
                minute: f64::NAN,
                second: f64::NAN,
                millisecond: f64::NAN,
            };
        }
        let fields = date.fields();
        Parts {
            year: fields.year as f64,
            month: fields.month as f64,
            day: fields.day as f64,
            weekday: fields.weekday as f64,
            hour: fields.hour as f64,
            minute: fields.minute as f64,
            second: fields.second as f64,
            millisecond: fields.millisecond as f64,
        }
    }
}

/// The parts of a date, as the parse reads them ✓.
#[derive(Clone, Copy, Debug, Default)]
struct Parsed {
    /// The year ✓.
    year: Option<f64>,
    /// The month, from zero ✓.
    month: Option<f64>,
    /// The day of the month, from one ✓.
    day: Option<f64>,
    /// The hour ✓.
    hour: Option<f64>,
    /// The minute ✓.
    minute: Option<f64>,
    /// The second ✓.
    second: Option<f64>,
    /// The millisecond ✓.
    millisecond: Option<f64>,
    /// The period, which is zero for the morning and one for the
    /// evening ✓.
    period: Option<f64>,
    /// The quarter, which is the month the quarter begins in ✓.
    quarter: Option<f64>,
    /// The number of milliseconds since the epoch ✓.
    timestamp: Option<f64>,
    /// The number of seconds since the epoch ✓.
    timestamp_seconds: Option<f64>,
    /// The day of the week, from one, with the Monday first ✓.
    weekday_monday: Option<f64>,
    /// The week of the year, with the Sunday first ✓.
    week_sunday: Option<f64>,
    /// The week of the year, of the ISO ✓.
    week_iso: Option<f64>,
    /// The day of the week, from zero, with the Sunday first ✓.
    weekday_sunday: Option<f64>,
    /// The week of the year, with the Monday first ✓.
    week_monday: Option<f64>,
    /// The offset from Greenwich, in minutes, the other way about ✓.
    zone: Option<f64>,
}

/// A parse, which reads a date ✓.
#[derive(Clone, Debug)]
pub struct Parse {
    /// The specification the parse was made for ✓.
    specifier: String,
    /// The locale the names are read in ✓.
    locale: Locale,
    /// Whether the date is read in Greenwich ✓.
    utc: bool,
}

impl Parse {
    /// The date of the text, as the JavaScript reads it ✓.
    ///
    /// A text the specification does not fit is not a date ✓, and the
    /// JavaScript gives `null` for it ✓.
    pub fn parse(&self, string: &str) -> Option<Date> {
        let mut parts = Parsed::default();
        let characters: Vec<char> = string.chars().collect();
        let end = self.parse_specifier(&mut parts, &self.specifier, &characters, 0)?;
        if end != characters.len() {
            return None;
        }

        // A UNIX timestamp is the date itself ✓.
        if let Some(timestamp) = parts.timestamp {
            return Some(clip(Date::local(timestamp)));
        }
        if let Some(seconds) = parts.timestamp_seconds {
            let millisecond = parts.millisecond.unwrap_or(0.0);
            return Some(clip(Date::local(seconds * 1_000.0 + millisecond)));
        }

        // The UTC parse never reads the local timezone ✓.
        if self.utc && parts.zone.is_none() {
            parts.zone = Some(0.0);
        }

        // The period of the day is the morning or the evening ✓.
        if let Some(period) = parts.period {
            parts.hour = Some(parts.hour.unwrap_or(0.0) % 12.0 + period * 12.0);
        }

        // A month that was not given is the month of the quarter, or
        // the January ✓.
        if parts.month.is_none() {
            parts.month = Some(parts.quarter.unwrap_or(0.0));
        }

        // The week of the ISO is turned into a day of the year ✓.
        if let Some(week) = parts.week_iso {
            if !(1.0..=53.0).contains(&week) {
                return None;
            }
            if parts.weekday_sunday.is_none() {
                parts.weekday_sunday = Some(1.0);
            }
            let mut monday = self.first_of_year(&parts);
            let weekday = monday.fields().weekday;
            if weekday > 4 || weekday == 0 {
                time_monday().ceil(&mut monday);
            } else {
                time_monday().floor(&mut monday);
            }
            time_day().offset(&mut monday, (week as i32 - 1) * 7);
            let fields = monday.fields();
            parts.year = Some(fields.year as f64);
            parts.month = Some(fields.month as f64);
            parts.day = Some(fields.day as f64 + (parts.weekday_sunday.unwrap_or(0.0) + 6.0) % 7.0);
        } else if parts.week_monday.is_some() || parts.week_sunday.is_some() {
            // The week of the year is turned into a day of the year ✓.
            if parts.weekday_sunday.is_none() {
                parts.weekday_sunday = Some(if let Some(weekday) = parts.weekday_monday {
                    weekday % 7.0
                } else if parts.week_monday.is_some() {
                    1.0
                } else {
                    0.0
                });
            }
            let weekday = self.first_of_year(&parts).fields().weekday as f64;
            parts.month = Some(0.0);
            parts.day = Some(if let Some(week) = parts.week_monday {
                (parts.weekday_sunday.unwrap_or(0.0) + 6.0) % 7.0 + week * 7.0
                    - (weekday + 5.0) % 7.0
            } else {
                parts.weekday_sunday.unwrap_or(0.0) + parts.week_sunday.unwrap_or(0.0) * 7.0
                    - (weekday + 6.0) % 7.0
            });
        }

        // A timezone means that the parts are the time in Greenwich,
        // and are moved by the offset ✓.
        if let Some(zone) = parts.zone {
            parts.hour = Some(parts.hour.unwrap_or(0.0) + (zone / 100.0).trunc());
            parts.minute = Some(parts.minute.unwrap_or(0.0) + zone % 100.0);
            return Some(make_date(&parts, TimeZone::Utc));
        }

        // Otherwise the parts are the local time ✓.
        Some(make_date(&parts, TimeZone::Local))
    }

    /// The specification the parse was made for ✓.
    ///
    /// The JavaScript's own `toString` gives the specification back ✓.
    pub fn specifier(&self) -> &str {
        &self.specifier
    }

    /// The first of the year of the parts, which the week of the year
    /// is counted from ✓.
    fn first_of_year(&self, parts: &Parsed) -> Date {
        let first = Parsed {
            year: parts.year,
            month: Some(0.0),
            day: Some(1.0),
            ..Parsed::default()
        };
        make_date(
            &first,
            if parts.zone.is_some() {
                TimeZone::Utc
            } else {
                TimeZone::Local
            },
        )
    }

    /// The place in the text the specification reaches, which is the
    /// JavaScript's own `parseSpecifier` ✓.
    fn parse_specifier(
        &self,
        parts: &mut Parsed,
        specifier: &str,
        string: &[char],
        mut j: usize,
    ) -> Option<usize> {
        let characters: Vec<char> = specifier.chars().collect();
        let mut i = 0usize;
        while i < characters.len() {
            if j >= string.len() {
                return None;
            }
            let c = characters[i];
            i += 1;
            if c != '%' {
                // A character of the specification is a character of the
                // text ✓.
                if string.get(j) != Some(&c) {
                    return None;
                }
                j += 1;
                continue;
            }
            // The directive, which may be written with a fill ✓.
            let mut directive = characters.get(i).copied();
            i += 1;
            if matches!(directive, Some('-') | Some('_') | Some('0')) {
                directive = characters.get(i).copied();
                i += 1;
            }
            let directive = directive?;
            j = self.parse_directive(parts, directive, string, j)?;
        }
        Some(j)
    }

    /// The place in the text a directive reaches, which is the
    /// JavaScript's own `parses` ✓.
    fn parse_directive(
        &self,
        parts: &mut Parsed,
        directive: char,
        string: &[char],
        j: usize,
    ) -> Option<usize> {
        let text = || -> String { string[j..].iter().collect() };
        Some(match directive {
            'a' => {
                let (index, length) = match_name(&self.locale.definition.short_days, &text())?;
                parts.weekday_sunday = Some(index as f64);
                j + length
            }
            'A' => {
                let (index, length) = match_name(&self.locale.definition.days, &text())?;
                parts.weekday_sunday = Some(index as f64);
                j + length
            }
            'b' => {
                let (index, length) = match_name(&self.locale.definition.short_months, &text())?;
                parts.month = Some(index as f64);
                j + length
            }
            'B' => {
                let (index, length) = match_name(&self.locale.definition.months, &text())?;
                parts.month = Some(index as f64);
                j + length
            }
            'c' => self.parse_specifier(parts, &self.locale.definition.date_time, string, j)?,
            'd' | 'e' => {
                let (value, length) = match_number(&window(string, j, 2))?;
                parts.day = Some(value);
                j + length
            }
            'f' => {
                let (value, length) = match_number(&window(string, j, 6))?;
                parts.millisecond = Some((value / 1_000.0).floor());
                j + length
            }
            'g' | 'y' => {
                let (value, length) = match_number(&window(string, j, 2))?;
                parts.year = Some(value + if value > 68.0 { 1900.0 } else { 2000.0 });
                j + length
            }
            'G' | 'Y' => {
                let (value, length) = match_number(&window(string, j, 4))?;
                parts.year = Some(value);
                j + length
            }
            'H' | 'I' => {
                let (value, length) = match_number(&window(string, j, 2))?;
                parts.hour = Some(value);
                j + length
            }
            'j' => {
                let (value, length) = match_number(&window(string, j, 3))?;
                parts.month = Some(0.0);
                parts.day = Some(value);
                j + length
            }
            'L' => {
                let (value, length) = match_number(&window(string, j, 3))?;
                parts.millisecond = Some(value);
                j + length
            }
            'm' => {
                let (value, length) = match_number(&window(string, j, 2))?;
                parts.month = Some(value - 1.0);
                j + length
            }
            'M' => {
                let (value, length) = match_number(&window(string, j, 2))?;
                parts.minute = Some(value);
                j + length
            }
            'p' => {
                let (index, length) = match_name(&self.locale.definition.periods, &text())?;
                parts.period = Some(index as f64);
                j + length
            }
            'q' => {
                let (value, length) = match_number(&window(string, j, 1))?;
                parts.quarter = Some(value * 3.0 - 3.0);
                j + length
            }
            'Q' => {
                let (value, length) = match_number(&text())?;
                parts.timestamp = Some(value);
                j + length
            }
            's' => {
                let (value, length) = match_number(&text())?;
                parts.timestamp_seconds = Some(value);
                j + length
            }
            'S' => {
                let (value, length) = match_number(&window(string, j, 2))?;
                parts.second = Some(value);
                j + length
            }
            'u' => {
                let (value, length) = match_number(&window(string, j, 1))?;
                parts.weekday_monday = Some(value);
                j + length
            }
            'U' => {
                let (value, length) = match_number(&window(string, j, 2))?;
                parts.week_sunday = Some(value);
                j + length
            }
            'V' => {
                let (value, length) = match_number(&window(string, j, 2))?;
                parts.week_iso = Some(value);
                j + length
            }
            'w' => {
                let (value, length) = match_number(&window(string, j, 1))?;
                parts.weekday_sunday = Some(value);
                j + length
            }
            'W' => {
                let (value, length) = match_number(&window(string, j, 2))?;
                parts.week_monday = Some(value);
                j + length
            }
            'x' => self.parse_specifier(parts, &self.locale.definition.date, string, j)?,
            'X' => self.parse_specifier(parts, &self.locale.definition.time, string, j)?,
            'Z' => {
                let (value, length) = match_zone(&window(string, j, 6))?;
                parts.zone = Some(value as f64);
                j + length
            }
            '%' => {
                if string.get(j) != Some(&'%') {
                    return None;
                }
                j + 1
            }
            _ => return None,
        })
    }
}

/// The text of the string from the place, for as many characters as the
/// window holds ✓.
fn window(string: &[char], j: usize, length: usize) -> String {
    string.iter().skip(j).take(length).collect::<String>()
}

/// The number the JavaScript's own `numberRe` reads at the front of a
/// string ✓.
///
/// The regular expression reads the spaces before the number ✓, and the
/// number is the digits that follow them ✓.
fn match_number(text: &str) -> Option<(f64, usize)> {
    let mut start = 0;
    for c in text.chars() {
        if is_space(c) {
            start += c.len_utf8();
        } else {
            break;
        }
    }
    let mut end = start;
    for c in text[start..].chars() {
        if c.is_ascii_digit() {
            end += 1;
        } else {
            break;
        }
    }
    if end == start {
        return None;
    }
    let value = text[..end]
        .trim_start_matches(is_space)
        .parse::<f64>()
        .ok()?;
    Some((value, end))
}

/// The name the JavaScript's own `formatRe` reads at the front of a
/// string ✓.
///
/// The regular expression does not tell the case ✓, and it takes the
/// first of the names that the string begins with ✓.
fn match_name(names: &[String], text: &str) -> Option<(usize, usize)> {
    for (index, name) in names.iter().enumerate() {
        if begins_with(text, name) {
            return Some((index, name.chars().count()));
        }
    }
    None
}

/// Whether the text begins with the name, whatever their case ✓.
fn begins_with(text: &str, name: &str) -> bool {
    let mut text = text.chars();
    for c in name.chars() {
        match text.next() {
            None => return false,
            Some(t) => {
                let t = t.to_lowercase().next().unwrap_or(t);
                let c = c.to_lowercase().next().unwrap_or(c);
                if t != c {
                    return false;
                }
            }
        }
    }
    true
}

/// The offset the JavaScript's own zone pattern reads at the front of a
/// string ✓.
///
/// The pattern is a "Z", which is the offset from Greenwich of nothing,
/// or an offset such as "+05:30" ✓. The regular expression may find
/// the offset anywhere in the six characters it is given ✓.
fn match_zone(text: &str) -> Option<(i32, usize)> {
    let characters: Vec<char> = text.chars().collect();
    if characters.first() == Some(&'Z') {
        return Some((0, 1));
    }
    for start in 0..characters.len() {
        let sign = match characters.get(start) {
            Some('+') => 1,
            Some('-') => -1,
            _ => continue,
        };
        let hours = match characters.get(start + 1..start + 3) {
            Some(hours) if hours.len() == 2 && hours.iter().all(|c| c.is_ascii_digit()) => {
                hours.iter().collect::<String>().parse::<i32>().unwrap_or(0)
            }
            _ => continue,
        };
        let mut length = 3;
        let mut minutes = 0;
        // The minutes, which may be written with a colon or without ✓.
        let rest = &characters[(start + 3).min(characters.len())..];
        let with_colon = rest.first() == Some(&':');
        let minute_text = if with_colon {
            rest.get(1..3)
        } else {
            rest.get(0..2)
        };
        if let Some(minute_text) = minute_text {
            if minute_text.len() == 2 && minute_text.iter().all(|c| c.is_ascii_digit()) {
                minutes = minute_text
                    .iter()
                    .collect::<String>()
                    .parse::<i32>()
                    .unwrap_or(0);
                length += if with_colon { 3 } else { 2 };
            }
        }
        // The offset is written as the hours and the minutes are one
        // number, and the JavaScript's own splits it again ✓.
        let zone = sign * (hours * 100 + minutes);
        return Some((-zone, length));
    }
    None
}

/// Whether a character is one of the JavaScript's own spaces ✓.
///
/// The JavaScript's `\s` is a list of its own, and it is not Rust's
/// `char::is_whitespace`: the JavaScript has the byte-order mark in it,
/// and Rust does not ✓.
fn is_space(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// The date of the parts, in the timezone the parse reads them in ✓.
fn make_date(parts: &Parsed, timezone: TimeZone) -> Date {
    let year = parts.year.unwrap_or(1900.0);
    if (0.0..100.0).contains(&year) {
        // The JavaScript's own constructor makes the year before, and
        // puts the year in afterwards, for a year under a hundred is a
        // year of the calendar and not a year of nineteen hundred ✓.
        let mut date = make_date_in(parts, year - 1.0, timezone);
        date.set_full_year(year as i64);
        return date;
    }
    make_date_in(parts, year, timezone)
}

/// The date of the parts, in the timezone the parse reads them in, in
/// the year given ✓.
fn make_date_in(parts: &Parsed, year: f64, timezone: TimeZone) -> Date {
    let (year, month, day) =
        in_calendar(year, parts.month.unwrap_or(0.0), parts.day.unwrap_or(1.0));
    // The time of the day may be out of the day, for the timezone may
    // have taken an hour from it ✓.
    let time = parts.hour.unwrap_or(0.0) * DURATION_HOUR
        + parts.minute.unwrap_or(0.0) * DURATION_MINUTE
        + parts.second.unwrap_or(0.0) * DURATION_SECOND
        + parts.millisecond.unwrap_or(0.0);
    let days = (time / DURATION_DAY).floor();
    let time = time - days * DURATION_DAY;
    let (year, month, day) = in_calendar(year as f64, month as f64, day as f64 + days);
    let fields = Fields {
        year,
        month,
        day,
        // The day of the week is found from the date itself ✓.
        weekday: 0,
        hour: (time / DURATION_HOUR).floor() as u32,
        minute: ((time % DURATION_HOUR) / DURATION_MINUTE).floor() as u32,
        second: ((time % DURATION_MINUTE) / DURATION_SECOND).floor() as u32,
        millisecond: (time % DURATION_SECOND).floor() as u32,
    };
    let mut date = Date::new(0.0, timezone);
    date.set_fields(fields);
    clip(date)
}

/// The year, the month and the day of a date, as the JavaScript's own
/// constructor puts them in the calendar ✓.
///
/// A month that is not in the year rolls over into the year beside it,
/// and a day that is not in the month rolls over into the month beside
/// it ✓.
fn in_calendar(year: f64, month: f64, day: f64) -> (i64, u32, u32) {
    let mut year = year + (month / 12.0).floor();
    let mut month = month - 12.0 * (month / 12.0).floor();
    let mut day = day;
    while day < 1.0 {
        month -= 1.0;
        if month < 0.0 {
            month = 11.0;
            year -= 1.0;
        }
        day += length_of_month(year as i64, month as u32) as f64;
    }
    while day > length_of_month(year as i64, month as u32) as f64 {
        day -= length_of_month(year as i64, month as u32) as f64;
        month += 1.0;
        if month > 11.0 {
            month = 0.0;
            year += 1.0;
        }
    }
    (year as i64, month as u32, day as u32)
}

/// The date, which the JavaScript's own clips ✓.
///
/// A date beyond the range of the calendar is not a date at all ✓.
fn clip(date: Date) -> Date {
    if date.instant.abs() > MAX_TIME {
        Date::new(f64::NAN, date.timezone)
    } else {
        date
    }
}

/// The error the JavaScript throws when a date cannot be written ✓.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsoError {
    /// The date is beyond the range of the calendar ✓.
    OutOfRange,
}

impl std::fmt::Display for IsoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsoError::OutOfRange => write!(f, "Invalid time value"),
        }
    }
}

impl std::error::Error for IsoError {}

/// The date in the ISO form, which is the JavaScript's own `toISOString`
/// ✓.
///
/// The JavaScript throws when the date is beyond the range of the
/// calendar ✓, and the port gives the error ✓.
pub fn iso_format(date: &Date) -> Result<String, IsoError> {
    if !(-MAX_TIME..=MAX_TIME).contains(&date.instant) {
        return Err(IsoError::OutOfRange);
    }
    let fields = Date::utc(date.instant).fields();
    let year = fields.year;
    let year = if (0..=9999).contains(&year) {
        format!("{:04}", year)
    } else if year > 9999 {
        format!("+{:06}", year)
    } else {
        format!("-{:06}", -year)
    };
    Ok(format!(
        "{}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year,
        fields.month + 1,
        fields.day,
        fields.hour,
        fields.minute,
        fields.second,
        fields.millisecond
    ))
}

/// The date of a text in the ISO form, which is the JavaScript's own
/// `new Date` ✓.
///
/// A text that is not in the ISO form is not a date the JavaScript's own
/// specification gives, and the port gives `None` ✓.
pub fn iso_parse(string: &str) -> Option<Date> {
    let characters: Vec<char> = string.chars().collect();
    let mut i = 0usize;

    // The year, which may be written with a sign and with six digits ✓.
    let year = match characters.get(i) {
        Some('+') | Some('-') => {
            let sign = if characters[i] == '+' { 1.0 } else { -1.0 };
            i += 1;
            let digits = digits(&characters, i, 6)?;
            i += 6;
            sign * digits
        }
        _ => {
            let digits = digits(&characters, i, 4)?;
            i += 4;
            digits
        }
    };

    // The month and the day, which may be left out ✓.
    let mut month = 1.0;
    let mut day = 1.0;
    if characters.get(i) == Some(&'-') {
        month = digits(&characters, i + 1, 2)?;
        i += 3;
        if characters.get(i) == Some(&'-') {
            day = digits(&characters, i + 1, 2)?;
            i += 3;
        }
    }
    if !(1.0..=12.0).contains(&month) || !(1.0..=31.0).contains(&day) {
        return None;
    }

    // The time, which may be left out ✓.
    let mut hour = 0.0;
    let mut minute = 0.0;
    let mut second = 0.0;
    let mut millisecond = 0.0;
    let mut time = false;
    if matches!(characters.get(i), Some('T') | Some('t')) {
        time = true;
        i += 1;
        hour = digits(&characters, i, 2)?;
        i += 2;
        if characters.get(i) != Some(&':') {
            return None;
        }
        i += 1;
        minute = digits(&characters, i, 2)?;
        i += 2;
        if characters.get(i) == Some(&':') {
            i += 1;
            second = digits(&characters, i, 2)?;
            i += 2;
            if characters.get(i) == Some(&'.') {
                i += 1;
                let start = i;
                while matches!(characters.get(i), Some(c) if c.is_ascii_digit()) {
                    i += 1;
                }
                if i == start {
                    return None;
                }
                // The fraction is kept to the millisecond ✓.
                let fraction: String = characters[start..i].iter().collect();
                let fraction = format!("{:0<3}", fraction);
                millisecond = fraction[..3].parse::<f64>().ok()?;
            }
        }
    }
    if !(0.0..=24.0).contains(&hour)
        || !(0.0..=59.0).contains(&minute)
        || !(0.0..=59.0).contains(&second)
    {
        return None;
    }
    // The hour of twenty-four is the midnight of the day after, and
    // only when the rest of the time is zero ✓.
    if hour == 24.0 && (minute != 0.0 || second != 0.0 || millisecond != 0.0) {
        return None;
    }

    // The offset from Greenwich, which may be left out ✓.
    let mut offset = None;
    match characters.get(i) {
        Some('Z') | Some('z') => {
            offset = Some(0.0);
            i += 1;
        }
        Some('+') | Some('-') => {
            let sign = if characters[i] == '+' { 1.0 } else { -1.0 };
            i += 1;
            let hours = digits(&characters, i, 2)?;
            i += 2;
            // The minutes, which may be written with a colon or without ✓.
            let minutes = if characters.get(i) == Some(&':') {
                i += 1;
                digits(&characters, i, 2)?
            } else {
                digits(&characters, i, 2)?
            };
            i += 2;
            offset = Some(sign * (hours * 60.0 + minutes));
        }
        _ => {}
    }
    if i != characters.len() {
        return None;
    }

    let mut parts = Parsed {
        year: Some(year),
        month: Some(month - 1.0),
        day: Some(day),
        hour: Some(hour),
        minute: Some(minute),
        second: Some(second),
        millisecond: Some(millisecond),
        ..Parsed::default()
    };
    // A date without a time is read in Greenwich, and a date with a
    // time and without an offset is read in the local timezone ✓.
    let timezone = match offset {
        Some(offset) => {
            parts.hour = Some(parts.hour.unwrap_or(0.0) - (offset / 60.0).trunc());
            parts.minute = Some(parts.minute.unwrap_or(0.0) - offset % 60.0);
            TimeZone::Utc
        }
        None => {
            if time {
                TimeZone::Local
            } else {
                TimeZone::Utc
            }
        }
    };
    Some(make_date(&parts, timezone))
}

/// The number of the digits of the string, which may be left out ✓.
fn digits(characters: &[char], i: usize, length: usize) -> Option<f64> {
    let digits = characters.get(i..i + length)?;
    if !digits.iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    digits.iter().collect::<String>().parse::<f64>().ok()
}
