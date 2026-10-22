//! The ES date-time string grammar behind `Date.parse`.

/// Whether `Date.parse(text)` is not NaN, for the ES date-time string format only.
///
/// V8 additionally accepts legacy forms (`"Aug 25 2026"`, `"2026-8-25"`, a space separator,
/// lowercase `t`/`z`, `+0530` offsets, surrounding whitespace). This function rejects those, so
/// it may reject strings `Date.parse` accepts.
///
/// For every string with an explicit offset (`Z` or `+hh:mm`) it never accepts a string
/// `Date.parse` rejects. A date-time with no offset is local time to V8, which applies the local
/// zone before its +/-8.64e15 ms range check; this function range-checks it as UTC. The two agree
/// under `TZ=UTC` (the basis of the golden corpus) and away from the range bounds; within 14
/// hours of either bound (about `+275760-09-13T00:00:00` and `-271821-04-20T00:00:00`) a
/// non-UTC zone can make V8 reject a string this function accepts.
pub(crate) fn is_es_iso_timestamp(text: &str) -> bool {
    parse_es_iso(text).is_some()
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn digits(&mut self, count: usize) -> Option<i64> {
        let slice = self.bytes.get(self.at..self.at + count)?;
        if !slice.iter().all(u8::is_ascii_digit) {
            return None;
        }
        self.at += count;
        Some(
            slice
                .iter()
                .fold(0, |acc, d| acc * 10 + i64::from(d - b'0')),
        )
    }
}

fn parse_es_iso(text: &str) -> Option<()> {
    let mut cursor = Cursor {
        bytes: text.as_bytes(),
        at: 0,
    };
    let year = match cursor.peek()? {
        sign @ (b'+' | b'-') => {
            cursor.at += 1;
            let magnitude = cursor.digits(6)?;
            if sign == b'-' {
                if magnitude == 0 {
                    return None;
                }
                -magnitude
            } else {
                magnitude
            }
        }
        _ => cursor.digits(4)?,
    };
    let (mut month, mut day) = (1, 1);
    if cursor.eat(b'-') {
        month = cursor.digits(2)?;
        if !(1..=12).contains(&month) {
            return None;
        }
        if cursor.eat(b'-') {
            day = cursor.digits(2)?;
            if !(1..=31).contains(&day) {
                return None;
            }
        }
    }
    let (mut hour, mut minute, mut second, mut fraction_zero, mut millisecond) = (0, 0, 0, true, 0);
    let mut offset_minutes = 0;
    if cursor.eat(b'T') {
        hour = cursor.digits(2)?;
        if !cursor.eat(b':') {
            return None;
        }
        minute = cursor.digits(2)?;
        if cursor.eat(b':') {
            second = cursor.digits(2)?;
            if cursor.eat(b'.') {
                let start = cursor.at;
                // Digits past the third are read but truncated, as in V8.
                while let Some(digit) = cursor.peek().filter(u8::is_ascii_digit) {
                    if digit != b'0' {
                        fraction_zero = false;
                    }
                    if cursor.at - start < 3 {
                        millisecond += i64::from(digit - b'0') * [100, 10, 1][cursor.at - start];
                    }
                    cursor.at += 1;
                }
                if cursor.at == start {
                    return None;
                }
            }
        }
        if hour > 24 || minute > 59 || second > 59 {
            return None;
        }
        if hour == 24 && (minute != 0 || second != 0 || !fraction_zero) {
            return None;
        }
        if !cursor.eat(b'Z')
            && let Some(sign @ (b'+' | b'-')) = cursor.peek()
        {
            cursor.at += 1;
            let offset_hour = cursor.digits(2)?;
            if !cursor.eat(b':') {
                return None;
            }
            let offset_minute = cursor.digits(2)?;
            if offset_hour > 23 || offset_minute > 59 {
                return None;
            }
            let total = offset_hour * 60 + offset_minute;
            offset_minutes = if sign == b'-' { -total } else { total };
        }
    }
    if cursor.at != text.len() {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let millis = (days * 86_400 + hour * 3_600 + minute * 60 + second - offset_minutes * 60)
        * 1_000
        + millisecond;
    (millis.abs() <= 8_640_000_000_000_000).then_some(())
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Hinnant's algorithm). A day past the
/// month's end overflows into the next month, as in `Date.UTC`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let shifted_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn es_iso_boundaries() {
        assert!(is_es_iso_timestamp("2026-08-25T22:01:00.000Z"));
        assert!(is_es_iso_timestamp("2026-08-25T24:00:00Z"));
        assert!(!is_es_iso_timestamp("2026-08-25T24:00:01Z"));
        assert!(!is_es_iso_timestamp("2026-13-01"));
        assert!(is_es_iso_timestamp("+275760-09-13T00:00:00.000Z"));
        assert!(!is_es_iso_timestamp("+275760-09-13T00:00:00.001Z"));
        assert!(!is_es_iso_timestamp("-000000-01-01T00:00:00Z"));
    }
}
