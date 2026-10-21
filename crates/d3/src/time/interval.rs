//! The JavaScript's own calendar, and the intervals of it.
//!
//! Translated from `d3-time/src/`, version 2.3.0, of d3 5.16.0.
//!
//! The JavaScript's dates are read in the timezone of the browser, and
//! the port's dates are read in the timezone the program is given — see
//! [`crate::time::zone`].

use std::sync::Arc;

use crate::time::Date;

/// The number of milliseconds in a second ✓.
pub const DURATION_SECOND: f64 = 1e3;
/// The number of milliseconds in a minute ✓.
pub const DURATION_MINUTE: f64 = 6e4;
/// The number of milliseconds in an hour ✓.
pub const DURATION_HOUR: f64 = 36e5;
/// The number of milliseconds in a day ✓.
pub const DURATION_DAY: f64 = 864e5;
/// The number of milliseconds in a week ✓.
pub const DURATION_WEEK: f64 = 6048e5;

/// The parts of a date, as the calendar has them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fields {
    /// The year ✓.
    pub year: i64,
    /// The month, from zero ✓.
    pub month: u32,
    /// The day of the month, from one ✓.
    pub day: u32,
    /// The day of the week, from zero ✓ and with the Sunday first ✓.
    pub weekday: u32,
    /// The hour ✓.
    pub hour: u32,
    /// The minute ✓.
    pub minute: u32,
    /// The second ✓.
    pub second: u32,
    /// The millisecond ✓.
    pub millisecond: u32,
}

/// The number of days since the epoch of a date in the calendar ✓.
///
/// The count is from Howard Hinnant's algorithms ✓, which are the ones
/// the calendar itself is built on ✓.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    // The month of the count is from one, and the JavaScript's is from
    // zero ✓.
    let month = month as i64 + 1;
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day = day as i64;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146097 + day_of_era - 719468
}

/// The date in the calendar of a number of days since the epoch ✓.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let day_of_era = days - era * 146097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    // The month of the JavaScript's own counts from zero ✓.
    let month = (if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    }) as u32
        - 1;
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

/// The number of days of a month of a year ✓.
pub fn length_of_month(year: i64, month: u32) -> u32 {
    match month {
        0 | 2 | 4 | 6 | 7 | 9 | 11 => 31,
        3 | 5 | 8 | 10 => 30,
        _ => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
    }
}

impl Date {
    /// The parts of the date, as the calendar has them ✓.
    ///
    /// The parts are the ones of the local time ✓, for the JavaScript's
    /// own getters are the local ones ✓.
    pub fn fields(&self) -> Fields {
        let local = self.local_instant();
        let days = (local / 86_400_000.0).floor() as i64;
        let time = local - (days as f64) * 86_400_000.0;
        let (year, month, day) = civil_from_days(days);
        Fields {
            year,
            month,
            day,
            // The day of the week, with the Thursday of the epoch first
            // and the Sunday fourth ✓.
            weekday: (days + 4).rem_euclid(7) as u32,
            hour: (time / 3_600_000.0).floor() as u32,
            minute: ((time % 3_600_000.0) / 60_000.0).floor() as u32,
            second: ((time % 60_000.0) / 1_000.0).floor() as u32,
            millisecond: (time % 1_000.0).floor() as u32,
        }
    }

    /// The date is set to the parts of the calendar ✓.
    ///
    /// The JavaScript's own setters take the local time ✓, and the
    /// offset of the timezone is found from the local time itself ✓ —
    /// for the offset of a timezone is a part of the time it belongs to
    /// ✓ — and the offset is found again from the local time it gives ✓,
    /// for the offset changes with the time of the year ✓.
    pub fn set_fields(&mut self, fields: Fields) {
        let days = days_from_civil(fields.year, fields.month, fields.day);
        let time = fields.hour as f64 * 3_600_000.0
            + fields.minute as f64 * 60_000.0
            + fields.second as f64 * 1_000.0
            + fields.millisecond as f64;
        let mut instant = days as f64 * 86_400_000.0 + time;
        for _ in 0..2 {
            let offset = self.timezone.offset_at(instant) as f64 * 60_000.0;
            instant = days as f64 * 86_400_000.0 + time - offset;
        }
        self.instant = instant;
    }

    /// The hour of the local time is put at zero ✓.
    pub fn set_hours_zero(&mut self) {
        let mut fields = self.fields();
        fields.hour = 0;
        fields.minute = 0;
        fields.second = 0;
        fields.millisecond = 0;
        self.set_fields(fields);
    }

    /// The day of the month of the local time is put ✓.
    ///
    /// The JavaScript's own: a day that is not in the month rolls over
    /// into the month beside it ✓, for the day of the month is a number
    /// and not a place in the calendar ✓.
    pub fn set_date(&mut self, day: i64) {
        let mut fields = self.fields();
        let length = length_of_month(fields.year, fields.month) as i64;
        let mut day = day;
        let mut month = fields.month as i64;
        let mut year = fields.year;
        if day < 1 {
            // The month before ✓.
            month -= 1;
            if month < 0 {
                month = 11;
                year -= 1;
            }
            day += length_of_month(year, month as u32) as i64;
        } else if day > length {
            // The month after ✓.
            day -= length;
            month += 1;
            if month > 11 {
                month = 0;
                year += 1;
            }
        }
        fields.day = day as u32;
        fields.month = month as u32;
        fields.year = year;
        self.set_fields(fields);
    }

    /// The month of the local time is put ✓.
    ///
    /// The JavaScript's own: a month that is not in the year rolls over
    /// into the year beside it ✓.
    pub fn set_month(&mut self, month: i64) {
        let mut fields = self.fields();
        let mut year = fields.year;
        let mut month = month;
        if month < 0 {
            year -= 1;
            month += 12;
        } else if month > 11 {
            year += 1;
            month -= 12;
        }
        fields.month = month as u32;
        fields.year = year;
        self.set_fields(fields);
    }

    /// The year of the local time is put ✓.
    pub fn set_full_year(&mut self, year: i64) {
        let mut fields = self.fields();
        fields.year = year;
        self.set_fields(fields);
    }
}

/// The function that puts a date at the start of its interval ✓.
type Floor = Arc<dyn Fn(&mut Date) + Send + Sync>;
/// The function that moves a date by a number of intervals ✓.
type Offset = Arc<dyn Fn(&mut Date, i32) + Send + Sync>;
/// The function that counts the intervals between two dates ✓.
type Count = Arc<dyn Fn(&Date, &Date) -> f64 + Send + Sync>;
/// The function that gives the number of the interval a date is in ✓.
type Field = Arc<dyn Fn(&Date) -> i32 + Send + Sync>;

/// An interval, which is the JavaScript's own.
#[derive(Clone)]
pub struct Interval {
    floor: Floor,
    offset: Offset,
    count: Option<Count>,
    field: Option<Field>,
}

impl Interval {
    /// The interval, which is the JavaScript's own ✓.
    pub fn new(
        floor: impl Fn(&mut Date) + Send + Sync + 'static,
        offset: impl Fn(&mut Date, i32) + Send + Sync + 'static,
        count: Option<impl Fn(&Date, &Date) -> f64 + Send + Sync + 'static>,
        field: Option<impl Fn(&Date) -> i32 + Send + Sync + 'static>,
    ) -> Self {
        Interval {
            floor: Arc::new(floor),
            offset: Arc::new(offset),
            count: count.map(|count| Arc::new(count) as Count),
            field: field.map(|field| Arc::new(field) as Field),
        }
    }

    /// The date is put at the start of its interval ✓.
    pub fn floor(&self, date: &mut Date) {
        (self.floor)(date);
    }

    /// The date is put at the end of its interval ✓.
    ///
    /// The JavaScript's own: the date is moved back a millisecond
    /// before it is put at the start of its interval ✓, for a date that
    /// is already at the start of one is left there ✓.
    pub fn ceil(&self, date: &mut Date) {
        date.instant -= 1.0;
        (self.floor)(date);
        (self.offset)(date, 1);
        (self.floor)(date);
    }

    /// The date is put at the start or the end of its interval ✓,
    /// whichever is nearer ✓.
    pub fn round(&self, date: &mut Date) {
        let mut before = *date;
        (self.floor)(&mut before);
        let mut after = *date;
        (self.floor)(&mut after);
        (self.offset)(&mut after, 1);
        (self.floor)(&mut after);
        if date.instant - before.instant < after.instant - date.instant {
            *date = before;
        } else {
            *date = after;
        }
    }

    /// The date is moved by a number of intervals ✓.
    pub fn offset(&self, date: &mut Date, step: i32) {
        (self.offset)(date, step);
    }

    /// The dates from one to another, one in each interval ✓.
    pub fn range(&self, start: &Date, stop: &Date, step: i32) -> Vec<Date> {
        let mut range = Vec::new();
        let mut start = *start;
        self.ceil(&mut start);
        if !(start.instant < stop.instant) || !(step > 0) {
            return range;
        }
        let mut previous = start;
        loop {
            range.push(previous);
            (self.offset)(&mut start, step);
            (self.floor)(&mut start);
            if !(previous.instant < start.instant && start.instant < stop.instant) {
                break;
            }
            previous = start;
        }
        range
    }

    /// The number of the intervals between two dates ✓.
    pub fn count(&self, start: &Date, end: &Date) -> Option<f64> {
        let count = self.count.as_ref()?;
        let mut start = *start;
        let mut end = *end;
        (self.floor)(&mut start);
        (self.floor)(&mut end);
        Some(count(&start, &end).floor())
    }

    /// The interval of the dates that pass a test ✓.
    ///
    /// The JavaScript's own: the date is put at the start of its
    /// interval, and if it does not pass the test it is moved back a
    /// millisecond at a time until it does ✓.
    pub fn filter(&self, test: impl Fn(&Date) -> bool + Send + Sync + 'static) -> Interval {
        let floor = self.floor.clone();
        let offset = self.offset.clone();
        let test = Arc::new(test);
        let first = test.clone();
        let second = test.clone();
        Interval::new(
            move |date: &mut Date| {
                while !first(date) {
                    let instant = date.instant - 1.0;
                    *date = Date::new(instant, date.timezone);
                    floor(date);
                }
            },
            move |date: &mut Date, step: i32| {
                if step < 0 {
                    let mut step = step;
                    while {
                        step += 1;
                        step <= 0
                    } {
                        while {
                            offset(date, -1);
                            !second(date)
                        } {}
                    }
                } else {
                    let mut step = step;
                    while {
                        step -= 1;
                        step >= 0
                    } {
                        while {
                            offset(date, 1);
                            !second(date)
                        } {}
                    }
                }
            },
            None::<fn(&Date, &Date) -> f64>,
            None::<fn(&Date) -> i32>,
        )
    }

    /// The interval of every so many ✓.
    pub fn every(&self, step: i32) -> Option<Interval> {
        if !(step > 0) {
            return None;
        }
        if step <= 1 {
            return Some(self.filter(|_date| true));
        }
        match &self.field {
            Some(field) => {
                let field = field.clone();
                Some(self.filter(move |date| field(date) % step == 0))
            }
            None => {
                let floor = self.floor.clone();
                let offset = self.offset.clone();
                let interval = Interval::new(
                    move |date: &mut Date| floor(date),
                    move |date: &mut Date, step: i32| offset(date, step),
                    None::<fn(&Date, &Date) -> f64>,
                    None::<fn(&Date) -> i32>,
                );
                let counted = interval.clone();
                Some(interval.filter(move |date| {
                    counted.count(&Date::utc(0.0), date).unwrap_or(0.0) % step as f64 == 0.0
                }))
            }
        }
    }
}

/// The interval of a millisecond ✓.
pub fn time_millisecond() -> Interval {
    Interval::new(
        |date: &mut Date| {
            // The JavaScript's own: the fraction of the millisecond is
            // taken away ✓.
            let instant = date.instant;
            date.instant = instant - instant.rem_euclid(1.0);
        },
        |date: &mut Date, step: i32| {
            let mut fields = date.fields();
            fields.millisecond = (fields.millisecond as i64 + step as i64) as u32;
            date.set_fields(fields);
        },
        Some(|start: &Date, end: &Date| (end.instant - start.instant) / 1.0),
        None::<fn(&Date) -> i32>,
    )
}

/// The interval of a second ✓.
pub fn time_second() -> Interval {
    Interval::new(
        |date: &mut Date| {
            let mut fields = date.fields();
            fields.millisecond = 0;
            date.set_fields(fields);
        },
        |date: &mut Date, step: i32| {
            let mut fields = date.fields();
            fields.second = (fields.second as i64 + step as i64) as u32;
            date.set_fields(fields);
        },
        Some(|start: &Date, end: &Date| (end.instant - start.instant) / DURATION_SECOND),
        None::<fn(&Date) -> i32>,
    )
}

/// The interval of a minute ✓.
pub fn time_minute() -> Interval {
    Interval::new(
        |date: &mut Date| {
            let mut fields = date.fields();
            fields.second = 0;
            fields.millisecond = 0;
            date.set_fields(fields);
        },
        |date: &mut Date, step: i32| {
            let mut fields = date.fields();
            fields.minute = (fields.minute as i64 + step as i64) as u32;
            date.set_fields(fields);
        },
        Some(|start: &Date, end: &Date| (end.instant - start.instant) / DURATION_MINUTE),
        None::<fn(&Date) -> i32>,
    )
}

/// The interval of an hour ✓.
pub fn time_hour() -> Interval {
    Interval::new(
        |date: &mut Date| {
            let mut fields = date.fields();
            fields.minute = 0;
            fields.second = 0;
            fields.millisecond = 0;
            date.set_fields(fields);
        },
        |date: &mut Date, step: i32| {
            let mut fields = date.fields();
            fields.hour = (fields.hour as i64 + step as i64) as u32;
            date.set_fields(fields);
        },
        Some(|start: &Date, end: &Date| (end.instant - start.instant) / DURATION_HOUR),
        None::<fn(&Date) -> i32>,
    )
}

/// The interval of a day ✓.
pub fn time_day() -> Interval {
    Interval::new(
        |date: &mut Date| date.set_hours_zero(),
        |date: &mut Date, step: i32| {
            let mut fields = date.fields();
            fields.day = (fields.day as i64 + step as i64) as u32;
            date.set_fields(fields);
        },
        Some(|start: &Date, end: &Date| {
            (end.instant - start.instant - (end.offset() - start.offset()) as f64 * DURATION_MINUTE)
                / DURATION_DAY
        }),
        Some(|date: &Date| date.fields().day as i32 - 1),
    )
}

/// The interval of a week, which begins on the Sunday ✓.
pub fn time_week() -> Interval {
    time_weekday(0)
}

/// The interval of a week, which begins on the Sunday ✓.
pub fn time_sunday() -> Interval {
    time_weekday(0)
}

/// The interval of a week, which begins on the Monday ✓.
pub fn time_monday() -> Interval {
    time_weekday(1)
}

/// The interval of a week, which begins on the Tuesday ✓.
pub fn time_tuesday() -> Interval {
    time_weekday(2)
}

/// The interval of a week, which begins on the Wednesday ✓.
pub fn time_wednesday() -> Interval {
    time_weekday(3)
}

/// The interval of a week, which begins on the Thursday ✓.
pub fn time_thursday() -> Interval {
    time_weekday(4)
}

/// The interval of a week, which begins on the Friday ✓.
pub fn time_friday() -> Interval {
    time_weekday(5)
}

/// The interval of a week, which begins on the Saturday ✓.
pub fn time_saturday() -> Interval {
    time_weekday(6)
}

/// The interval of a week, which begins on the day of the week given ✓.
pub fn time_weekday(weekday: u32) -> Interval {
    Interval::new(
        move |date: &mut Date| {
            let fields = date.fields();
            date.set_date(fields.day as i64 - (fields.weekday + 7 - weekday) as i64 % 7);
            date.set_hours_zero();
        },
        move |date: &mut Date, step: i32| {
            let mut fields = date.fields();
            fields.day = (fields.day as i64 + step as i64 * 7) as u32;
            date.set_fields(fields);
        },
        Some(|start: &Date, end: &Date| {
            (end.instant - start.instant - (end.offset() - start.offset()) as f64 * DURATION_MINUTE)
                / DURATION_WEEK
        }),
        None::<fn(&Date) -> i32>,
    )
}

/// The interval of a month ✓.
pub fn time_month() -> Interval {
    Interval::new(
        |date: &mut Date| {
            date.set_date(1);
            date.set_hours_zero();
        },
        |date: &mut Date, step: i32| {
            let fields = date.fields();
            date.set_month(fields.month as i64 + step as i64);
        },
        Some(|start: &Date, end: &Date| {
            let a = start.fields();
            let b = end.fields();
            (b.month as i64 - a.month as i64 + (b.year - a.year) * 12) as f64
        }),
        Some(|date: &Date| date.fields().month as i32),
    )
}

/// The interval of a year ✓.
pub fn time_year() -> Interval {
    Interval::new(
        |date: &mut Date| {
            date.set_month(0);
            date.set_date(1);
            date.set_hours_zero();
        },
        |date: &mut Date, step: i32| {
            let mut fields = date.fields();
            fields.year += step as i64;
            date.set_fields(fields);
        },
        Some(|start: &Date, end: &Date| (end.fields().year - start.fields().year) as f64),
        Some(|date: &Date| date.fields().year as i32),
    )
}

/// The interval of a millisecond, in Greenwich ✓.
pub fn utc_millisecond() -> Interval {
    utc_of(time_millisecond())
}

/// The interval of a second, in Greenwich ✓.
pub fn utc_second() -> Interval {
    utc_of(time_second())
}

/// The interval of a minute, in Greenwich ✓.
pub fn utc_minute() -> Interval {
    utc_of(time_minute())
}

/// The interval of an hour, in Greenwich ✓.
pub fn utc_hour() -> Interval {
    utc_of(time_hour())
}

/// The interval of a day, in Greenwich ✓.
pub fn utc_day() -> Interval {
    utc_of(time_day())
}

/// The interval of a week, in Greenwich ✓.
pub fn utc_week() -> Interval {
    utc_of(time_week())
}

/// The interval of a week, in Greenwich, which begins on the Sunday ✓.
pub fn utc_sunday() -> Interval {
    utc_of(time_weekday(0))
}

/// The interval of a week, in Greenwich, which begins on the Monday ✓.
pub fn utc_monday() -> Interval {
    utc_of(time_weekday(1))
}

/// The interval of a week, in Greenwich, which begins on the Tuesday ✓.
pub fn utc_tuesday() -> Interval {
    utc_of(time_weekday(2))
}

/// The interval of a week, in Greenwich, which begins on the Wednesday ✓.
pub fn utc_wednesday() -> Interval {
    utc_of(time_weekday(3))
}

/// The interval of a week, in Greenwich, which begins on the Thursday ✓.
pub fn utc_thursday() -> Interval {
    utc_of(time_weekday(4))
}

/// The interval of a week, in Greenwich, which begins on the Friday ✓.
pub fn utc_friday() -> Interval {
    utc_of(time_weekday(5))
}

/// The interval of a week, in Greenwich, which begins on the Saturday ✓.
pub fn utc_saturday() -> Interval {
    utc_of(time_weekday(6))
}

/// The interval of a month, in Greenwich ✓.
pub fn utc_month() -> Interval {
    utc_of(time_month())
}

/// The interval of a year, in Greenwich ✓.
pub fn utc_year() -> Interval {
    utc_of(time_year())
}

/// An interval in Greenwich, which is the local one read in Greenwich ✓.
fn utc_of(interval: Interval) -> Interval {
    let floor = interval.floor;
    let offset = interval.offset;
    let count = interval.count;
    let field = interval.field;
    Interval::new(
        move |date: &mut Date| {
            *date = Date::utc(date.instant);
            floor(date);
        },
        move |date: &mut Date, step: i32| {
            *date = Date::utc(date.instant);
            offset(date, step);
        },
        count.map(|count| {
            move |start: &Date, end: &Date| {
                let start = Date::utc(start.instant);
                let end = Date::utc(end.instant);
                count(&start, &end)
            }
        }),
        field.map(|field| {
            move |date: &Date| {
                let date = Date::utc(date.instant);
                field(&date)
            }
        }),
    )
}
