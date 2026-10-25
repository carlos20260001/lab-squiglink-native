//! The timezone a date is read in.
//!
//! The JavaScript's own dates are read in the timezone of the browser
//! that is running them, and the port's dates are read in the timezone
//! that the program is given.
//!
//! There are two of them:
//!
//! - **The system's own**, on a program that runs natively. It is the
//!   timezone the browser would have been in, and it is read from the
//!   operating system: the C library on a Unix, and the operating
//!   system itself on Windows.
//! - **The browser's own**, on a program that runs in a browser. The
//!   browser is the one that knows the timezone, and it is asked for it.
//!
//! Which of the two is used is settled when the program is built, for
//! the two are the program's own kind: a program that runs in a browser
//! is one that was built for the browser.

/// A timezone, which is either the one of Greenwich or the system's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeZone {
    /// The one of Greenwich, which is the same everywhere.
    Utc,
    /// The system's own, which is the one the reader is in.
    Local,
}

impl TimeZone {
    /// The offset from Greenwich of this timezone, in seconds, at a given
    /// time.
    ///
    /// The JavaScript's own dates keep the seconds of the offset ✓, for
    /// the offset of a timezone may be a number of seconds ✓, and the
    /// whole minutes are the `getTimezoneOffset` of the JavaScript ✓.
    pub fn offset_seconds_at(&self, instant: f64) -> i32 {
        match self {
            TimeZone::Utc => 0,
            TimeZone::Local => local_offset_seconds_at(instant),
        }
    }

    /// The offset from Greenwich of this timezone, in minutes, at a given
    /// time.
    ///
    /// The time is the number of milliseconds since the epoch ✓ and the
    /// offset is the number of minutes the local time is ahead of
    /// Greenwich ✓ — which is the sign the JavaScript's own
    /// `getTimezoneOffset` has the other way about ✓, so the port's is
    /// the opposite of it ✓.
    pub fn offset_at(&self, instant: f64) -> i32 {
        match self {
            TimeZone::Utc => 0,
            TimeZone::Local => local_offset_at(instant),
        }
    }
}

/// The system's own offset from Greenwich, in minutes, at a given time.
#[cfg(unix)]
fn local_offset_at(instant: f64) -> i32 {
    unix::offset_at(instant)
}

/// The system's own offset from Greenwich, in seconds, at a given time.
#[cfg(unix)]
fn local_offset_seconds_at(instant: f64) -> i32 {
    unix::offset_seconds_at(instant)
}

/// The system's own offset from Greenwich, in minutes, at a given time.
#[cfg(windows)]
fn local_offset_at(instant: f64) -> i32 {
    windows::offset_at(instant)
}

/// The system's own offset from Greenwich, in seconds, at a given time.
#[cfg(windows)]
fn local_offset_seconds_at(instant: f64) -> i32 {
    windows::offset_at(instant) * 60
}

/// The browser's own offset from Greenwich, in minutes, at a given time.
#[cfg(target_arch = "wasm32")]
fn local_offset_at(instant: f64) -> i32 {
    browser::offset_at(instant)
}

/// The browser's own offset from Greenwich, in seconds, at a given time.
///
/// The browser's own offset is a whole number of minutes ✓ — for its
/// `getTimezoneOffset` is ✓ — and the seconds of it are not to be had ✓.
#[cfg(target_arch = "wasm32")]
fn local_offset_seconds_at(instant: f64) -> i32 {
    browser::offset_at(instant) * 60
}

/// The offset of the operating system, on a program that runs natively.
#[cfg(unix)]
mod unix {
    /// The C library's own way of reading the local time ✓.
    ///
    /// The `tm` is the C library's own, and its shape is the same on
    /// Linux and on the BSDs and on macOS ✓.
    #[repr(C)]
    struct Tm {
        seconds: i32,
        minutes: i32,
        hours: i32,
        day: i32,
        month: i32,
        year: i32,
        weekday: i32,
        day_of_year: i32,
        daylight: i32,
        /// The offset from Greenwich, in seconds ✓, which the C library
        /// fills in ✓.
        offset: i64,
        zone: *const i8,
    }

    extern "C" {
        /// The C library's own reading of the local time ✓.
        fn localtime_r(time: *const i64, result: *mut Tm) -> *mut Tm;
    }

    /// The offset from Greenwich, in minutes, at a given time ✓.
    pub fn offset_at(instant: f64) -> i32 {
        offset_seconds_at(instant) / 60
    }

    /// The offset from Greenwich, in seconds, at a given time ✓.
    pub fn offset_seconds_at(instant: f64) -> i32 {
        // The C library's own time is a whole number of seconds.
        let seconds = (instant / 1000.0).floor() as i64;
        let mut local = Tm {
            seconds: 0,
            minutes: 0,
            hours: 0,
            day: 0,
            month: 0,
            year: 0,
            weekday: 0,
            day_of_year: 0,
            daylight: 0,
            offset: 0,
            zone: std::ptr::null(),
        };
        let read = unsafe { localtime_r(&seconds, &mut local) };
        if read.is_null() {
            return 0;
        }
        // The offset is in seconds ✓, and the port keeps them ✓.
        local.offset as i32
    }
}

/// The offset of the operating system on Windows ✓.
///
/// The operating system keeps the rules of the timezone ✓ and it will
/// turn a time in Greenwich into the local time ✓ — which is what the
/// port wants ✓ — and the offset is the difference of the two ✓.
#[cfg(windows)]
mod windows {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }

    #[repr(C)]
    struct TimeZoneInformation {
        bias: i32,
        standard_name: [u16; 32],
        standard_date: SystemTime,
        standard_bias: i32,
        daylight_name: [u16; 32],
        daylight_date: SystemTime,
        daylight_bias: i32,
    }

    extern "system" {
        /// The rules of the timezone of the system ✓.
        fn GetTimeZoneInformation(time_zone_information: *mut TimeZoneInformation) -> u32;
        /// The local time of a time in Greenwich ✓.
        fn SystemTimeToTzSpecificLocalTime(
            system_time: *const SystemTime,
            time_zone_information: *const TimeZoneInformation,
            local_time: *mut SystemTime,
        ) -> i32;
    }

    /// The number of seconds between the epoch of the system and the
    /// epoch of the language ✓.
    const EPOCH: i64 = 11_644_473_600;

    /// The offset from Greenwich, in minutes, at a given time ✓.
    pub fn offset_at(instant: f64) -> i32 {
        let mut rules = TimeZoneInformation {
            bias: 0,
            standard_name: [0; 32],
            standard_date: SystemTime {
                year: 0,
                month: 0,
                day_of_week: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
                milliseconds: 0,
            },
            standard_bias: 0,
            daylight_name: [0; 32],
            daylight_date: SystemTime {
                year: 0,
                month: 0,
                day_of_week: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
                milliseconds: 0,
            },
            daylight_bias: 0,
        };
        unsafe { GetTimeZoneInformation(&mut rules) };

        // The time of the system, which counts from the epoch of the
        // system and not the epoch of the language.
        let mut ticks = (instant / 1000.0).floor() as i64 + EPOCH;
        let (seconds, milliseconds) = (ticks % 1000, 0);
        ticks -= seconds;
        let (days, time_of_day) = (ticks.div_euclid(86_400), ticks.rem_euclid(86_400));
        let (hour, minute, second) = (
            time_of_day / 3600,
            (time_of_day % 3600) / 60,
            time_of_day % 60,
        );
        let mut utc = SystemTime {
            year: 1970,
            month: 1,
            day_of_week: 4,
            day: 1,
            hour: hour as u16,
            minute: minute as u16,
            second: second as u16,
            milliseconds: milliseconds as u16,
        };
        // The day of the month of the first of January, 1970, is found
        // by counting the days from then.
        let mut year = 1970i64;
        let mut month = 1i64;
        let mut day = 1i64;
        let mut left = days;
        loop {
            let length = length_of_month(year, month);
            if left < length {
                day += left;
                break;
            }
            left -= length;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
        utc.year = year as u16;
        utc.month = month as u16;
        utc.day = day as u16;

        let mut local = utc;
        let ok = unsafe { SystemTimeToTzSpecificLocalTime(&utc, &rules, &mut local) };
        if ok == 0 {
            return 0;
        }
        // The difference of the two times is the offset ✓.
        let local_seconds =
            local.hour as i64 * 3600 + local.minute as i64 * 60 + local.second as i64;
        let utc_seconds = hour * 3600 + minute * 60 + second;
        ((local_seconds - utc_seconds) / 60) as i32
    }

    /// The number of days of a month of a year ✓.
    fn length_of_month(year: i64, month: i64) -> i64 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            _ => {
                if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                    29
                } else {
                    28
                }
            }
        }
    }
}

/// The offset of the browser, on a program that runs in a browser.
#[cfg(target_arch = "wasm32")]
mod browser {
    extern "C" {
        /// The browser's own timezone, in minutes from Greenwich.
        ///
        /// The host of the program provides this one ✓ — it is the
        /// browser's own `Date.prototype.getTimezoneOffset`, with its
        /// sign the other way about ✓.
        fn js_timezone_offset() -> f64;
    }

    /// The offset from Greenwich, in minutes, at a given time ✓.
    ///
    /// The browser's own offset is the one of the timezone the reader is
    /// in ✓ — the same for every time of the year ✓ — and so the time
    /// that is given is not looked at ✓.
    pub fn offset_at(_instant: f64) -> i32 {
        let offset = unsafe { js_timezone_offset() };
        if offset.is_nan() {
            return 0;
        }
        // The browser's offset is the other way about, and the port's is
        // the way the reader would say it.
        -(offset as i32)
    }
}
