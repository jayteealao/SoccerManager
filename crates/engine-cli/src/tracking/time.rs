//! Times of the tracking files: ISO 8601 with a fraction of a second and an offset, such as
//! `2023-05-27T13:30:12.600+00:00` or `2023-05-27T15:30:12.230+02:00`, as milliseconds since
//! the Unix epoch. The event file and the position file of one match write their times with
//! different offsets, so only the instant counts.

/// Milliseconds since the Unix epoch of an ISO 8601 time with an offset (`+hh:mm`, `-hh:mm`
/// or `Z`). The fraction may have any number of digits; past the third it is cut.
pub fn epoch_ms(text: &str) -> Result<i64, String> {
    let bad = || format!("{text} is not an ISO 8601 time with an offset");
    let b = text.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' {
        return Err(bad());
    }
    if b[16] != b':' {
        return Err(bad());
    }
    let num = |from: usize, to: usize| -> Result<i64, String> {
        text.get(from..to)
            .filter(|s| s.bytes().all(|c| c.is_ascii_digit()))
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(bad)
    };
    let (year, month, day) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (hour, minute, second) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return Err(bad());
    }
    if second > 60 {
        return Err(bad());
    }
    // The fraction: the digits after a dot, cut to milliseconds.
    let mut at = 19;
    let mut millis = 0i64;
    if b.get(at) == Some(&b'.') {
        at += 1;
        let start = at;
        while b.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == start {
            return Err(bad());
        }
        let digits = &text[start..at.min(start + 3)];
        millis = digits.parse::<i64>().map_err(|_| bad())? * 10i64.pow(3 - digits.len() as u32);
    }
    let offset_min = match b.get(at) {
        Some(b'Z') if at + 1 == b.len() => 0,
        Some(&sign @ (b'+' | b'-')) if at + 6 == b.len() && b[at + 3] == b':' => {
            let m = num(at + 1, at + 3)? * 60 + num(at + 4, at + 6)?;
            if sign == b'+' { m } else { -m }
        }
        _ => return Err(bad()),
    };
    let days = days_from_civil(year, month, day);
    let local = ((days * 24 + hour) * 60 + minute) * 60 + second;
    Ok((local - offset_min * 60) * 1000 + millis)
}

/// Days from 1970-01-01 to the civil date (proleptic Gregorian), after Howard Hinnant's
/// `days_from_civil`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_is_zero_and_a_known_day_counts_its_days() {
        assert_eq!(epoch_ms("1970-01-01T00:00:00Z"), Ok(0));
        // 2023-05-27 is day 19,504 after the epoch.
        assert_eq!(
            epoch_ms("2023-05-27T00:00:00.000+00:00"),
            Ok(19_504 * 86_400_000)
        );
    }

    /// One instant written with two offsets is one time; the fraction counts to the
    /// millisecond.
    #[test]
    fn two_offsets_of_one_instant_give_one_time() {
        let utc = epoch_ms("2023-05-27T13:30:12.600+00:00").unwrap();
        let cest = epoch_ms("2023-05-27T15:30:12.600+02:00").unwrap();
        assert_eq!(utc, cest);
        let west = epoch_ms("2023-05-27T10:00:12.6-03:30").unwrap();
        assert_eq!(west, utc);
        assert_eq!(epoch_ms("2023-05-27T13:30:12.6004+00:00").unwrap(), utc);
    }

    /// Times either side of midnight, and of a date line in another offset, follow on.
    #[test]
    fn times_across_a_date_line_follow_on() {
        let before = epoch_ms("2023-12-31T23:59:59.960+00:00").unwrap();
        let after = epoch_ms("2024-01-01T00:00:00.000+00:00").unwrap();
        assert_eq!(after - before, 40);
        // 01:00 on 1 January in +02:00 is 23:00 on 31 December in UTC.
        let east = epoch_ms("2024-01-01T01:00:00.000+02:00").unwrap();
        assert_eq!(after - east, 3_600_000);
        // A leap day.
        let feb28 = epoch_ms("2024-02-28T12:00:00Z").unwrap();
        let mar1 = epoch_ms("2024-03-01T12:00:00Z").unwrap();
        assert_eq!(mar1 - feb28, 2 * 86_400_000);
    }

    #[test]
    fn a_time_without_an_offset_or_with_a_bad_field_is_refused() {
        for text in [
            "2023-05-27T13:30:12.600",
            "2023-05-27 13:30:12Z",
            "2023-13-27T13:30:12Z",
            "2023-05-27T13:30:12.+00:00",
            "2023-05-27T13:30:12.600+0000",
            "",
        ] {
            assert!(epoch_ms(text).is_err(), "{text}");
        }
    }
}
