//! XFA 3.3's canonical formats: how a date, a time, a date-time and a number are written in the
//! data, whatever locale a person typed them in.
//!
//! Chapter 24, *Canonical Format Reference* (pages 1045–1048), is the definition, and chapter 4's
//! *About the Canonical Format Used in the Data and Form DOMs* (page 156) is why it matters here:
//! the Data DOM, and so the `datasets` packet a save writes, holds every such value in this form.
//! Each form has a recogniser — Rule 2A (page 164) accepts a value already canonical as it is — and
//! a writer, which [`super::picture`] calls once a picture clause has taken a value apart. XFA 3.3
//! is cited by chapter and printed page and paraphrased (RFC 0011's preamble).

use std::fmt::Write as _;

/// A Gregorian date as chapter 24 writes it: the year, and the month and day where stated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Date {
    pub(crate) year: u16,
    pub(crate) month: Option<u8>,
    pub(crate) day: Option<u8>,
}

impl Date {
    /// The date in the hyphenated form of page 1045, `YYYY[-MM[-DD]]`: the form the chapter's own
    /// worked examples write (page 164's "2004-01-12"), and the one a reader of the packet can
    /// read without counting digits. The unhyphenated form is as canonical; the choice is a
    /// spelling, not a value.
    pub(crate) fn write(self) -> String {
        match (self.month, self.day) {
            (Some(month), Some(day)) => format!("{:04}-{month:02}-{day:02}", self.year),
            (Some(month), None) => format!("{:04}-{month:02}", self.year),
            _ => format!("{:04}", self.year),
        }
    }
}

/// A time of day as chapter 24 writes it, to the precision the value stated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Time {
    pub(crate) hour: u8,
    pub(crate) minute: Option<u8>,
    pub(crate) second: Option<u8>,
    pub(crate) millisecond: Option<u16>,
}

impl Time {
    /// The time in the colon form of page 1046, `HH[:MM[:SS[.FFF]]]`, with no zone: a picture
    /// clause this module reads states none, and page 1046 reads an omitted zone as a local time
    /// in a zone nobody stated.
    pub(crate) fn write(self) -> String {
        let mut out = format!("{:02}", self.hour);
        if let Some(minute) = self.minute {
            let _ = write!(out, ":{minute:02}");
            if let Some(second) = self.second {
                let _ = write!(out, ":{second:02}");
                if let Some(millisecond) = self.millisecond {
                    let _ = write!(out, ".{millisecond:03}");
                }
            }
        }
        out
    }
}

/// The number of days in a Gregorian month, the only calendar XFA 3.3 supports (page 157).
pub(crate) fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Whether a Gregorian year has a 29 February.
pub(crate) fn leap(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// The day of the week, 1 for Sunday to 7 for Saturday — page 1167's numbering of `E` — by
/// Zeller's congruence on the Gregorian calendar.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a u16 year, a u8 month and a u8 day widened to i64 keep every term under 2^20"
)]
pub(crate) fn weekday(year: u16, month: u8, day: u8) -> u8 {
    // Zeller counts January and February as the thirteenth and fourteenth months of the year
    // before.
    let (zeller_year, zeller_month) = if month < 3 {
        (i64::from(year) - 1, i64::from(month) + 12)
    } else {
        (i64::from(year), i64::from(month))
    };
    let (of_century, century) = (zeller_year.rem_euclid(100), zeller_year.div_euclid(100));
    // Zeller's answer is 0 for Saturday, 1 for Sunday; page 1167 wants Sunday 1 and Saturday 7.
    let answer = (i64::from(day)
        + (13 * (zeller_month + 1)) / 5
        + of_century
        + of_century / 4
        + century / 4
        + 5 * century)
        .rem_euclid(7);
    let sunday_first = (answer + 6).rem_euclid(7) + 1;
    u8::try_from(sunday_first).unwrap_or(1)
}

/// The month and day of a year's `day`th day (1-based), where the year has one.
pub(crate) fn from_day_of_year(year: u16, day: u16) -> Option<(u8, u8)> {
    let mut left = day;
    for month in 1..=12 {
        let length = u16::from(days_in_month(year, month));
        if left >= 1 && left <= length {
            return Some((month, u8::try_from(left).ok()?));
        }
        left = left.checked_sub(length)?;
    }
    None
}

fn digits(text: &str, count: usize) -> Option<u16> {
    (text.len() == count && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

/// Whether `text` is one of page 1045's date forms, `YYYY[MM[DD]]` or `YYYY[-MM[-DD]]`, naming a
/// month and a day that exist.
pub(crate) fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = if text.contains('-') {
        text.split('-').collect()
    } else if text.is_ascii() {
        [0..4, 4..6, 6..8]
            .into_iter()
            .filter_map(|range| text.get(range))
            .filter(|part| !part.is_empty())
            .collect()
    } else {
        return false;
    };
    if !text.contains('-') && parts.iter().map(|p| p.len()).sum::<usize>() != text.len() {
        return false;
    }
    let mut parts = parts.into_iter();
    let Some(year) = parts.next().and_then(|p| digits(p, 4)) else {
        return false;
    };
    let month = match parts.next() {
        None => return true,
        Some(part) => match digits(part, 2) {
            Some(month @ 1..=12) => month,
            _ => return false,
        },
    };
    let day = match parts.next() {
        None => return true,
        Some(part) => digits(part, 2),
    };
    let (Ok(month), Some(day)) = (u8::try_from(month), day) else {
        return false;
    };
    parts.next().is_none() && day >= 1 && day <= u16::from(days_in_month(year, month))
}

/// Whether `text` is one of page 1046's time forms: `HH[MM[SS[.FFF][zone]]]` or the same with
/// colons, the zone `Z`, `+HH[MM]` or `-HH[MM]` (colons again in the colon form).
pub(crate) fn is_time(text: &str) -> bool {
    let colons = text.contains(':');
    let (clock, zone) = match text.find(['Z', '+', '-']) {
        Some(at) => (&text[..at], Some(&text[at..])),
        None => (text, None),
    };
    let (clock, fraction) = match clock.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (clock, None),
    };
    let fields: Vec<&str> = if colons {
        clock.split(':').collect()
    } else if clock.is_ascii() && clock.len() % 2 == 0 {
        (0..clock.len() / 2)
            .filter_map(|i| clock.get(i.saturating_mul(2)..i.saturating_mul(2).saturating_add(2)))
            .collect()
    } else {
        return false;
    };
    let limits = [23, 59, 59];
    if fields.is_empty() || fields.len() > 3 {
        return false;
    }
    for (field, limit) in fields.iter().zip(limits) {
        if digits(field, 2).is_none_or(|value| value > limit) {
            return false;
        }
    }
    // Page 1046 hangs the fraction and the zone on the seconds.
    if (fraction.is_some() || zone.is_some()) && fields.len() < 3 {
        return false;
    }
    if fraction.is_some_and(|f| digits(f, 3).is_none()) {
        return false;
    }
    match zone {
        None | Some("Z") => true,
        Some(offset) => {
            let offset = &offset[1..];
            let (hours, minutes) = if colons {
                match offset.split_once(':') {
                    Some((h, m)) => (h, Some(m)),
                    None => (offset, None),
                }
            } else {
                match (offset.get(..2), offset.get(2..)) {
                    (Some(h), Some("")) => (h, None),
                    (Some(h), Some(m)) => (h, Some(m)),
                    _ => return false,
                }
            };
            digits(hours, 2).is_some_and(|h| h <= 23)
                && minutes.is_none_or(|m| digits(m, 2).is_some_and(|m| m <= 59))
        }
    }
}

/// Whether `text` is page 1047's date-time: a date, `T`, a time.
pub(crate) fn is_date_time(text: &str) -> bool {
    text.split_once('T')
        .is_some_and(|(date, time)| is_date(date) && is_time(time))
}

/// Whether `text` is page 1047's number literal: an optional sign, digits with an optional
/// decimal point among or after them, and an optional exponent — `1`, `+1`, `-1.e-3`,
/// `1234.E+10`, `+.1234` are the page's own examples.
///
/// Page 1047 prints the grammar twice, and neither printing survives the conversion to text as a
/// well-formed expression; the examples are what this reads it from, so a lone `.` or a bare
/// exponent are not numbers.
pub(crate) fn is_number(text: &str) -> bool {
    let text = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        // `e` and `E` are one byte, so the exponent starts one byte on.
        Some(at) => (&text[..at], text.get(at.saturating_add(1)..)),
        None => (text, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let all_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(whole) || !all_digits(fraction) || (whole.is_empty() && fraction.is_empty()) {
        return false;
    }
    match exponent {
        None => true,
        Some(exponent) => {
            let exponent = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
            !exponent.is_empty() && all_digits(exponent)
        }
    }
}

/// A number from its parts as a picture clause took them apart: the sign, the integer digits and
/// the fraction digits as typed, written as page 1047's literal. Leading zeros go, as page 1182's
/// `z999` reads `0150` as `150`; the fraction is kept to the digits stated, as page 1182 reads
/// `99V99`'s `1050` as `10.50`; an empty integer part is written `0`, a choice among page 1047's
/// equal spellings.
pub(crate) fn number(negative: bool, whole: &str, fraction: Option<&str>) -> String {
    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    let zero = whole == "0" && fraction.is_none_or(|f| f.bytes().all(|b| b == b'0'));
    let mut out = String::new();
    if negative && !zero {
        out.push('-');
    }
    out.push_str(whole);
    if let Some(fraction) = fraction.filter(|f| !f.is_empty()) {
        out.push('.');
        out.push_str(fraction);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Date, Time, is_date, is_date_time, is_number, is_time, number, weekday};

    /// Page 1045's EXAMPLE 24.1 dates, page 1047's EXAMPLE 24.2 times and page 1047's numbers are
    /// each canonical; a month or a day that does not exist is not.
    #[test]
    fn the_chapters_own_examples_are_canonical() {
        for date in [
            "19970716",
            "199707",
            "1997",
            "1997-07-16",
            "1997-07",
            "2004-02-29",
        ] {
            assert!(is_date(date), "{date}");
        }
        for not in ["1997-7-16", "1997-13", "2003-02-29", "97", "1997/07/16", ""] {
            assert!(!is_date(not), "{not}");
        }
        for time in [
            "202045.321Z",
            "192045.321-0100",
            "192045.321-01",
            "192045.321",
            "19:20:45",
            "1920",
            "19",
        ] {
            assert!(is_time(time), "{time}");
        }
        for not in ["24", "19:60", "7:05", "1920Z"] {
            assert!(!is_time(not), "{not}");
        }
        assert!(is_date_time("1997-07-16T20:20:45.432Z"));
        for literal in [
            "1",
            "+1",
            "1234",
            "-1234",
            "1E100",
            "1234e-4",
            "-1.e-3",
            "1234.E+10",
            "2.1",
            "+.1234",
            "-.12e2",
        ] {
            assert!(is_number(literal), "{literal}");
        }
        for not in [".", "", "1,234", "1e", "deux", "NaN"] {
            assert!(!is_number(not), "{not}");
        }
    }

    /// The writers' spellings, and page 1167's weekday numbering with 1 for Sunday.
    #[test]
    fn the_writers_spell_the_hyphen_and_colon_forms() {
        let date = Date {
            year: 2004,
            month: Some(1),
            day: Some(12),
        };
        assert_eq!(date.write(), "2004-01-12");
        let time = Time {
            hour: 9,
            minute: Some(5),
            second: None,
            millisecond: None,
        };
        assert_eq!(time.write(), "09:05");
        assert_eq!(number(false, "0150", None), "150");
        assert_eq!(number(true, "1234", Some("00")), "-1234.00");
        assert_eq!(number(true, "", Some("0")), "0.0");
        // 22 July 2004 was a Thursday, page 1172's `E` example answering 5.
        assert_eq!(weekday(2004, 7, 22), 5);
        assert_eq!(weekday(2000, 1, 1), 7);
    }
}
