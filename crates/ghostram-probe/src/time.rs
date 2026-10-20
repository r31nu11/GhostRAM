//! UTC RFC 3339 timestamp formatting without a calendar dependency.

use std::time::{SystemTime, UNIX_EPOCH};

/// Formats a `SystemTime` as a UTC RFC 3339 timestamp with whole-second
/// precision.
#[must_use]
pub fn format_utc(timestamp: SystemTime) -> String {
    let seconds = match timestamp.duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
        Err(error) => -i64::try_from(error.duration().as_secs()).unwrap_or(i64::MAX),
    };
    format_unix_seconds(seconds)
}

/// Returns the current time as a UTC RFC 3339 timestamp.
#[must_use]
pub fn now_rfc3339() -> String {
    format_utc(SystemTime::now())
}

fn format_unix_seconds(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

// Howard Hinnant's civil-from-days algorithm, with days relative to 1970-01-01.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::format_utc;

    #[test]
    fn formats_the_epoch() {
        assert_eq!(format_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn formats_a_known_utc_timestamp() {
        let timestamp = UNIX_EPOCH + Duration::from_secs(946_684_800);
        assert_eq!(format_utc(timestamp), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn formats_a_leap_day() {
        let timestamp = UNIX_EPOCH + Duration::from_secs(1_709_208_896);
        assert_eq!(format_utc(timestamp), "2024-02-29T12:14:56Z");
    }
}
