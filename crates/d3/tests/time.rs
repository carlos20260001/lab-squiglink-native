//! The timezone the dates are read in.

use d3::time::zone::TimeZone;

#[test]
fn the_utc_is_the_same_everywhere() {
    assert_eq!(TimeZone::Utc.offset_at(0.0), 0);
    assert_eq!(TimeZone::Utc.offset_at(1_700_000_000_000.0), 0);
    assert_eq!(TimeZone::Utc.offset_at(-1_700_000_000_000.0), 0);
}

#[test]
fn the_local_offset_is_a_timezone_of_the_world() {
    // The offset is in minutes, and no timezone of the world is more
    // than fourteen hours from Greenwich.
    for instant in [0.0, 1_700_000_000_000.0, -1_700_000_000_000.0] {
        let offset = TimeZone::Local.offset_at(instant);
        assert!(
            offset.abs() <= 14 * 60,
            "the offset at {} is {}",
            instant,
            offset
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_local_offset_is_the_operating_system_s_own() {
    // The C library's own way of reading the local time, which is the
    // one the port reads the offset with, and the two must agree.
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
        offset: i64,
        zone: *const i8,
    }
    extern "C" {
        fn localtime_r(time: *const i64, result: *mut Tm) -> *mut Tm;
    }

    for instant in [0.0f64, 1_700_000_000_000.0] {
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
        assert!(!read.is_null());
        assert_eq!(
            TimeZone::Local.offset_at(instant),
            (local.offset as i32) / 60,
            "the offset at {}",
            instant
        );
    }
}
