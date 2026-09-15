use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds in a civil day. A leap second is the clock's business, not the
/// calendar's.
const DAY: i64 = 86_400;

/// Today where the person is, as `YYYY-MM-DD`, for dating a session with the
/// tape.
///
/// The local day rather than UTC's: it is the date the person writes on the
/// measuring card, and a fitting at ten at night three hours west of
/// Greenwich happens today there, not tomorrow. Where the local offset cannot
/// be read, UTC stands in, which is a day off at most.
pub fn today() -> String {
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_secs()).unwrap_or(i64::MAX),
        Err(before) => i64::try_from(before.duration().as_secs()).map_or(i64::MIN, |secs| -secs),
    };
    civil_day(now, local_offset(now).unwrap_or(0))
}

/// The day `seconds` after the Unix epoch falls on, for a clock `offset`
/// seconds east of UTC.
fn civil_day(seconds: i64, offset: i64) -> String {
    let (year, month, day) = civil_from_days(seconds.saturating_add(offset).div_euclid(DAY));
    format!("{year:04}-{month:02}-{day:02}")
}

/// Year, month and day in the proleptic Gregorian calendar, from days since
/// the Unix epoch.
///
/// Howard Hinnant's `civil_from_days`. Counting the year from March puts the
/// leap day last, and whole 400-year eras make every step an integer division.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let march_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * march_month + 2) / 5 + 1;
    let month = if march_month < 10 {
        march_month + 3
    } else {
        march_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month as u32, day as u32)
}

/// How far east of UTC the local clock stands at `seconds`, by the C
/// library's reading of the time zone.
#[cfg(unix)]
#[allow(
    clippy::useless_conversion,
    clippy::unnecessary_fallible_conversions,
    reason = "time_t and c_long are 32 bits on some unix targets and 64 on others"
)]
fn local_offset(seconds: i64) -> Option<i64> {
    let time = libc::time_t::try_from(seconds).ok()?;
    let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: `time` is a live value to read and `local` is writable space
    // the size of a `tm`; localtime_r keeps neither pointer past the call.
    let filled = unsafe { libc::localtime_r(&raw const time, local.as_mut_ptr()) };
    if filled.is_null() {
        return None;
    }
    // SAFETY: a non-null return is localtime_r's word that it wrote `local`.
    let local = unsafe { local.assume_init() };
    Some(i64::from(local.tm_gmtoff))
}

#[cfg(not(unix))]
fn local_offset(_seconds: i64) -> Option<i64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-09-15, midnight UTC.
    const SEPT_15: i64 = 20_711 * DAY;

    #[test]
    fn the_epoch_and_the_second_before_it_are_two_days() {
        assert_eq!(civil_day(0, 0), "1970-01-01");
        assert_eq!(civil_day(-1, 0), "1969-12-31");
        assert_eq!(civil_day(DAY - 1, 0), "1970-01-01");
        assert_eq!(civil_day(DAY, 0), "1970-01-02");
    }

    #[test]
    fn a_known_day_and_the_leap_rules_land_where_the_calendar_says() {
        assert_eq!(civil_day(SEPT_15, 0), "2026-09-15");
        assert_eq!(civil_day(11_016 * DAY, 0), "2000-02-29");
        assert_eq!(civil_day(47_540 * DAY, 0), "2100-02-28");
        assert_eq!(civil_day(47_541 * DAY, 0), "2100-03-01");
        assert_eq!(civil_day(-25_508 * DAY, 0), "1900-03-01");
    }

    #[test]
    fn the_local_offset_moves_the_day_across_midnight_both_ways() {
        let late_utc = SEPT_15 + 23 * 3_600 + 30 * 60;
        assert_eq!(civil_day(late_utc, 0), "2026-09-15");
        assert_eq!(civil_day(late_utc, 2 * 3_600), "2026-09-16");
        let early_utc = SEPT_15 + DAY + 3_600;
        assert_eq!(civil_day(early_utc, 0), "2026-09-16");
        assert_eq!(civil_day(early_utc, -3 * 3_600), "2026-09-15");
    }

    /// Eight centuries walked one day at a time with the month lengths and the
    /// leap rule, which shares nothing with the arithmetic under test.
    #[test]
    fn every_day_of_eight_centuries_agrees_with_counting_them_one_by_one() {
        let length = |year: i64, month: u32| match month {
            2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        let (mut year, mut month, mut day) = (1600, 1, 1);
        for days in -135_140..=157_419 {
            assert_eq!(civil_from_days(days), (year, month, day), "day {days}");
            day += 1;
            if day > length(year, month) {
                day = 1;
                month += 1;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
            }
        }
        assert_eq!((year, month, day), (2401, 1, 1));
    }

    #[test]
    fn today_is_written_year_month_day() {
        let day = today();
        let bytes = day.as_bytes();
        assert_eq!(bytes.len(), 10, "{day}");
        assert_eq!((bytes[4], bytes[7]), (b'-', b'-'), "{day}");
        assert!(
            day.replace('-', "").bytes().all(|b| b.is_ascii_digit()),
            "{day}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_local_clock_is_read_and_is_a_real_time_zone_away() {
        let offset = local_offset(SEPT_15).expect("the C library reads the zone");
        assert!(offset.abs() <= 14 * 3_600, "{offset}");
    }
}
