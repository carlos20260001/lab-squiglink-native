//! The JavaScript's own dates, and the timezone they are read in.
//!
//! The JavaScript's `Date` is a number of milliseconds since the epoch,
//! and it is read in the timezone of the browser that is running it. The
//! port's date is the same number, and it is read in the timezone that
//! the program is given — the system's own for a program that runs
//! natively, and the browser's own for a program that runs in a browser.

pub mod interval;
pub mod zone;

pub use interval::Interval;
pub use zone::TimeZone;

/// A date, which is the JavaScript's own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Date {
    /// The number of milliseconds since the epoch.
    pub instant: f64,
    /// The timezone the date is read in.
    pub timezone: TimeZone,
}

impl Date {
    /// The date of a number of milliseconds since the epoch, in the
    /// timezone of Greenwich.
    pub fn utc(instant: f64) -> Self {
        Date {
            instant,
            timezone: TimeZone::Utc,
        }
    }

    /// The date of a number of milliseconds since the epoch, in the
    /// system's own timezone.
    pub fn local(instant: f64) -> Self {
        Date {
            instant,
            timezone: TimeZone::Local,
        }
    }

    /// The date of a number of milliseconds since the epoch, in a
    /// timezone of the program's own.
    pub fn new(instant: f64, timezone: TimeZone) -> Self {
        Date { instant, timezone }
    }

    /// The number of milliseconds since the epoch ✓.
    pub fn time(&self) -> f64 {
        self.instant
    }

    /// The offset from Greenwich of the date's timezone, in minutes ✓.
    pub fn offset(&self) -> i32 {
        self.timezone.offset_at(self.instant)
    }

    /// The local time of the date, in milliseconds since the epoch ✓.
    pub fn local_instant(&self) -> f64 {
        self.instant + (self.offset() as f64) * 60_000.0
    }
}
