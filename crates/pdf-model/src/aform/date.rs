//! The date and time half of the library: `util.printd`'s pictures, written and read.
//!
//! Adobe's *JavaScript for Acrobat API Reference* documents the picture language on its "util
//! methods" page, under `printd` — a table of place-holder strings, each with its effect and an
//! example — and documents, under `scand`, the one rule for reading a two-digit year: below 50 is
//! the twenty-first century and 50 or above the twentieth. Both are taken as written. What neither
//! page states — how a picture *reads* a string, which month names a picture writes, and what a
//! picture with no year means — is a documented choice of ADR 1578 section 6.

use std::fmt::Write as _;

use super::Refusal;

/// A moment as a picture reads and writes it: a calendar date and a time of day, no zone.
///
/// No zone because no picture in the language names one, and no clock because `CLAUDE.md`
/// principle 3 gives this crate none: a field's date is what the field says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    /// The year, in the proleptic Gregorian calendar.
    pub year: i32,
    /// The month, 1–12.
    pub month: u8,
    /// The day of the month, from 1.
    pub day: u8,
    /// The hour, 0–23.
    pub hour: u8,
    /// The minute, 0–59.
    pub minute: u8,
    /// The second, 0–59.
    pub second: u8,
}

/// The year a picture with no year in it is read against.
///
/// **The choice** (ADR 1578 section 6): a leap year, so that `2/29` typed into an `m/d` field is a
/// date. The year is never written — the picture has no place for it — so the only thing it can
/// decide is whether the twenty-ninth of February exists, and refusing a date a person can name
/// because this program chose a year without one would be refusing on a guess.
const YEAR_OF_NO_YEAR: i32 = 2000;

/// English month names, `printd`'s `mmmm` — the reference's own example of the place-holder is
/// "September", and it names no other language.
const MONTHS: [&str; 12] = [
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
];

/// English day names, `printd`'s `dddd` — the reference's example is "Wednesday".
const DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// One element of a picture.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    /// `mmmm`, `mmm`, `mm`, `m`: the month as a long name, a short name, two digits, digits.
    Month(u8),
    /// `dddd`, `ddd`: the day of the week, long and short.
    Weekday(u8),
    /// `dd`, `d`: the day of the month, two digits and digits.
    Day(u8),
    /// `yyyy`, `yy`: the year, four digits and two.
    Year(u8),
    /// `HH`, `H`: the hour on a 24-hour clock.
    Hour24(u8),
    /// `hh`, `h`: the hour on a 12-hour clock.
    Hour12(u8),
    /// `MM`, `M`: the minutes.
    Minute(u8),
    /// `ss`, `s`: the seconds.
    Second(u8),
    /// `tt`, `t`: am or pm, as two letters and one.
    Meridiem(u8),
    /// Anything else, or a character after `\`: written as it stands.
    Literal(char),
}

/// Splits a picture into its tokens, longest place-holder first.
///
/// # Errors
///
/// A [`Refusal`] for `j` and `jj`, the Japanese era place-holders, which the reference marks as
/// deprecated in favour of the XFA picture clause — a language this tree does not hold, so the
/// place-holders are refused by name rather than written as letters.
fn tokens(picture: &str) -> Result<Vec<Token>, Refusal> {
    let chars: Vec<char> = picture.chars().collect();
    let mut out = Vec::new();
    let mut at = 0_usize;
    while let Some(&c) = chars.get(at) {
        let run = chars
            .get(at..)
            .unwrap_or_default()
            .iter()
            .take_while(|next| **next == c)
            .count();
        let take = |n: usize| run.min(n);
        let (token, used) = match c {
            '\\' => match chars.get(at.saturating_add(1)) {
                Some(&escaped) => (Token::Literal(escaped), 2),
                None => (Token::Literal('\\'), 1),
            },
            'm' => {
                let n = take(4);
                (Token::Month(width(n)), n)
            }
            'd' => {
                let n = take(4);
                if n >= 3 {
                    (Token::Weekday(width(n)), n)
                } else {
                    (Token::Day(width(n)), n)
                }
            }
            'y' => {
                if run >= 4 {
                    (Token::Year(4), 4)
                } else if run >= 2 {
                    (Token::Year(2), 2)
                } else {
                    (Token::Literal('y'), 1)
                }
            }
            'H' => {
                let n = take(2);
                (Token::Hour24(width(n)), n)
            }
            'h' => {
                let n = take(2);
                (Token::Hour12(width(n)), n)
            }
            'M' => {
                let n = take(2);
                (Token::Minute(width(n)), n)
            }
            's' => {
                let n = take(2);
                (Token::Second(width(n)), n)
            }
            't' => {
                let n = take(2);
                (Token::Meridiem(width(n)), n)
            }
            'j' => {
                return Err(Refusal::new(
                    "the picture uses printd's Japanese era place-holder, which Adobe's reference \
                     deprecates in favour of the XFA picture clause this program does not read",
                ));
            }
            other => (Token::Literal(other), 1),
        };
        out.push(token);
        at = at.saturating_add(used);
    }
    Ok(out)
}

/// A run length as a token width.
fn width(n: usize) -> u8 {
    u8::try_from(n).unwrap_or(u8::MAX)
}

/// `util.printd`: a moment written in a picture.
///
/// # Errors
///
/// A [`Refusal`] where the picture uses a place-holder this program does not write (see
/// [`tokens`]).
pub fn print_date(picture: &str, moment: &DateTime) -> Result<String, Refusal> {
    let mut out = String::new();
    let twelve = match moment.hour % 12 {
        0 => 12,
        other => other,
    };
    for token in tokens(picture)? {
        match token {
            Token::Month(4) => out.push_str(month_name(moment.month)),
            Token::Month(3) => out.push_str(month_name(moment.month).get(..3).unwrap_or_default()),
            Token::Month(2) => {
                let _ = write!(out, "{:02}", moment.month);
            }
            Token::Month(_) => out.push_str(&moment.month.to_string()),
            Token::Weekday(4) => out.push_str(day_name(*moment)),
            Token::Weekday(_) => out.push_str(day_name(*moment).get(..3).unwrap_or_default()),
            Token::Day(2) => {
                let _ = write!(out, "{:02}", moment.day);
            }
            Token::Day(_) => out.push_str(&moment.day.to_string()),
            Token::Year(4) => {
                let _ = write!(out, "{:04}", moment.year);
            }
            Token::Year(_) => {
                let _ = write!(out, "{:02}", moment.year.rem_euclid(100));
            }
            Token::Hour24(2) => {
                let _ = write!(out, "{:02}", moment.hour);
            }
            Token::Hour24(_) => out.push_str(&moment.hour.to_string()),
            Token::Hour12(2) => {
                let _ = write!(out, "{twelve:02}");
            }
            Token::Hour12(_) => out.push_str(&twelve.to_string()),
            Token::Minute(2) => {
                let _ = write!(out, "{:02}", moment.minute);
            }
            Token::Minute(_) => out.push_str(&moment.minute.to_string()),
            Token::Second(2) => {
                let _ = write!(out, "{:02}", moment.second);
            }
            Token::Second(_) => out.push_str(&moment.second.to_string()),
            Token::Meridiem(2) => out.push_str(if moment.hour < 12 { "am" } else { "pm" }),
            Token::Meridiem(_) => out.push(if moment.hour < 12 { 'a' } else { 'p' }),
            Token::Literal(c) => out.push(c),
        }
    }
    Ok(out)
}

/// The long name of a month, or `""` for a number that names none.
fn month_name(month: u8) -> &'static str {
    usize::from(month)
        .checked_sub(1)
        .and_then(|index| MONTHS.get(index))
        .copied()
        .unwrap_or_default()
}

/// The long name of the day of the week a moment falls on.
fn day_name(moment: DateTime) -> &'static str {
    DAYS.get(weekday(moment.year, moment.month, moment.day))
        .copied()
        .unwrap_or_default()
}

/// The day of the week, 0 for Sunday, by Sakamoto's method over the proleptic Gregorian calendar.
fn weekday(year: i32, month: u8, day: u8) -> usize {
    const OFFSETS: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let year = if month < 3 {
        year.saturating_sub(1)
    } else {
        year
    };
    let offset = usize::from(month)
        .checked_sub(1)
        .and_then(|index| OFFSETS.get(index))
        .copied()
        .unwrap_or(0);
    let sum = year
        .saturating_add(year.div_euclid(4))
        .saturating_sub(year.div_euclid(100))
        .saturating_add(year.div_euclid(400))
        .saturating_add(offset)
        .saturating_add(i32::from(day));
    usize::try_from(sum.rem_euclid(7)).unwrap_or(0)
}

/// How many days a month has.
fn days_in(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    }
}

/// `AFParseDateEx`: a field's text read through a picture, or `None` where the text does not
/// follow it.
///
/// **The choice** (ADR 1578 section 6), since Adobe documents the function nowhere: the picture
/// is a *template* read left to right. A numeric place-holder takes digits — exactly its width
/// when its width is fixed (`mm`, `dd`, `yyyy`, `HH`, `MM`, `ss`) and the next element of the
/// picture is another place-holder with nothing between them, otherwise one or two (four for a
/// year) — so `yyyymmdd` reads `20240115`. A month-name place-holder takes a word that begins at
/// least three letters of an English month name, in either case. `tt` and `t` take `am`, `pm`,
/// `a` or `p` in either case, and may be absent, meaning the hour as written. A day-name
/// place-holder takes a day name or its first three letters and decides nothing. Every other
/// character of the picture matches any run of characters that are neither letters nor digits,
/// white space included, so `1-2-2024` reads through `m/d/yyyy`. White space around the text is
/// ignored; anything left over refuses.
///
/// Two digits under `yyyy` are a two-digit year, and `scand`'s documented horizon applies to them
/// as to `yy`: "less than 50" is the twenty-first century and 50 or more the twentieth. One or
/// three digits under a year refuse. An hour past twelve under `h` or `hh` with no marker is read
/// as a 24-hour hour. A picture with no year reads against [`YEAR_OF_NO_YEAR`]; a
/// picture with no month or day reads as the first. The moment must exist: a thirty-first of
/// April refuses.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one left-to-right walk of a picture's tokens, each arm a place-holder's reading; \
              splitting it would pass the eight partial fields between functions"
)]
pub fn parse_date(text: &str, picture: &str) -> Option<DateTime> {
    let tokens = tokens(picture).ok()?;
    let chars: Vec<char> = text.trim().chars().collect();
    let mut at = 0_usize;
    let mut year: Option<i32> = None;
    let mut month: Option<u8> = None;
    let mut day: Option<u8> = None;
    let mut hour: Option<u8> = None;
    let mut twelve = false;
    let mut meridiem: Option<bool> = None;
    let mut minute: Option<u8> = None;
    let mut second: Option<u8> = None;

    for (index, token) in tokens.iter().enumerate() {
        let next_is_field = tokens
            .get(index.saturating_add(1))
            .is_some_and(|next| !matches!(next, Token::Literal(_)));
        match token {
            Token::Literal(c) => {
                if c.is_alphanumeric() {
                    if chars.get(at) == Some(c) {
                        at = at.saturating_add(1);
                    } else {
                        return None;
                    }
                } else {
                    let run = separators(&chars, at);
                    // A separator in the picture may be matched by nothing only where the text
                    // has run out or where the picture's literal is white space.
                    if run == 0 && !c.is_whitespace() && at < chars.len() {
                        return None;
                    }
                    at = at.saturating_add(run);
                }
            }
            Token::Month(n) if *n >= 3 => {
                let (word, used) = letters(&chars, at);
                month = Some(month_from_word(&word)?);
                at = at.saturating_add(used);
            }
            Token::Weekday(_) => {
                let (word, used) = letters(&chars, at);
                if !DAYS.iter().any(|name| matches_name(&word, name)) {
                    return None;
                }
                at = at.saturating_add(used);
            }
            Token::Meridiem(_) => {
                let (word, used) = letters(&chars, at);
                if used == 0 {
                    continue;
                }
                meridiem = Some(match word.to_ascii_lowercase().as_str() {
                    "am" | "a" => false,
                    "pm" | "p" => true,
                    _ => return None,
                });
                at = at.saturating_add(used);
            }
            Token::Year(n) => {
                let fixed = next_is_field.then_some(usize::from(*n));
                let (digits, used) = number(&chars, at, fixed, 4)?;
                at = at.saturating_add(used);
                year = Some(match used {
                    4 => i32::try_from(digits).ok()?,
                    // `scand`'s date horizon, which the reference states for "a two-digit year
                    // for input" whatever the picture said.
                    2 if digits < 50 => i32::try_from(digits).ok()?.saturating_add(2000),
                    2 => i32::try_from(digits).ok()?.saturating_add(1900),
                    _ => return None,
                });
            }
            Token::Month(n)
            | Token::Day(n)
            | Token::Hour24(n)
            | Token::Hour12(n)
            | Token::Minute(n)
            | Token::Second(n) => {
                let fixed = (next_is_field && *n == 2).then_some(2);
                let (value, used) = number(&chars, at, fixed, 2)?;
                at = at.saturating_add(used);
                let value = u8::try_from(value).ok()?;
                match token {
                    Token::Month(_) => month = Some(value),
                    Token::Day(_) => day = Some(value),
                    Token::Hour24(_) => hour = Some(value),
                    Token::Hour12(_) => {
                        hour = Some(value);
                        twelve = true;
                    }
                    Token::Minute(_) => minute = Some(value),
                    _ => second = Some(value),
                }
            }
        }
    }
    if at < chars.len() {
        return None;
    }

    let year = year.unwrap_or(YEAR_OF_NO_YEAR);
    let month = month.unwrap_or(1);
    let day = day.unwrap_or(1);
    let mut hour = hour.unwrap_or(0);
    // A 12-hour place-holder with its marker present reads 1–12; without one, an hour past twelve
    // is a 24-hour hour written into it and is read as one (ADR 1578 section 6).
    if twelve && meridiem.is_some() && !(1..=12).contains(&hour) {
        return None;
    }
    match meridiem {
        Some(true) if hour < 12 => hour = hour.saturating_add(12),
        Some(false) if hour == 12 => hour = 0,
        Some(_) if hour > 12 => return None,
        _ => {}
    }
    let minute = minute.unwrap_or(0);
    let second = second.unwrap_or(0);
    let valid = (1..=12).contains(&month)
        && day >= 1
        && day <= days_in(year, month)
        && hour < 24
        && minute < 60
        && second < 60;
    valid.then_some(DateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
    })
}

/// How many characters from `at` are neither letters nor digits.
fn separators(chars: &[char], at: usize) -> usize {
    chars
        .get(at..)
        .unwrap_or_default()
        .iter()
        .take_while(|c| !c.is_alphanumeric())
        .count()
}

/// The run of letters from `at`, and its length.
fn letters(chars: &[char], at: usize) -> (String, usize) {
    let word: String = chars
        .get(at..)
        .unwrap_or_default()
        .iter()
        .take_while(|c| c.is_alphabetic())
        .collect();
    let used = word.chars().count();
    (word, used)
}

/// Digits from `at`: exactly `fixed` of them where it is given, otherwise one to `most`.
fn number(chars: &[char], at: usize, fixed: Option<usize>, most: usize) -> Option<(u32, usize)> {
    let available = chars
        .get(at..)
        .unwrap_or_default()
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .count();
    let used = match fixed {
        Some(fixed) if available >= fixed => fixed,
        Some(_) => return None,
        None if available == 0 || available > most => return None,
        None => available,
    };
    let mut value = 0_u32;
    for c in chars.get(at..at.saturating_add(used)).unwrap_or_default() {
        value = value.checked_mul(10)?.checked_add(c.to_digit(10)?)?;
    }
    Some((value, used))
}

/// The month a word names: at least its first three letters, in either case.
fn month_from_word(word: &str) -> Option<u8> {
    MONTHS
        .iter()
        .position(|name| matches_name(word, name))
        .and_then(|index| u8::try_from(index.saturating_add(1)).ok())
}

/// Whether a word is a name or a prefix of it at least three letters long.
fn matches_name(word: &str, name: &str) -> bool {
    word.chars().count() >= 3
        && name
            .to_ascii_lowercase()
            .starts_with(&word.to_ascii_lowercase())
}

/// The fourteen pictures `AFDate_Format` numbers, in the order the Interapplication reference's
/// `SetJavaScriptAction` lists them.
///
/// The reference gives them as the values `cFormat` "is one of"; a document calls the function
/// with the picture's *index* into that list at least as often as with the picture, and the index
/// is read as the position in the list as printed (ADR 1578 section 6).
pub(crate) const DATE_PICTURES: [&str; 14] = [
    "m/d",
    "m/d/yy",
    "mm/dd/yy",
    "mm/yy",
    "d-mmm",
    "d-mmm-yy",
    "dd-mmm-yy",
    "yy-mm-dd",
    "mmm-yy",
    "mmmm-yy",
    "mmm d, yyyy",
    "mmmm d, yyyy",
    "m/d/yy h:MM tt",
    "m/d/yy HH:MM",
];

/// The four pictures `AFTime_Format` numbers, the Interapplication reference's `ptf` menu.
///
/// The reference names them `24HR_MM`, `12HR_MM`, `24HR_MM_SS` and `12HR_MM_SS` and gives each an
/// example — `14:30`, `2:30 PM`, `14:30:15`, `2:30:15 PM` — and the examples are the fixtures.
/// **Its 12-hour examples write the marker in capitals**, where `printd`'s own table writes `tt`
/// as `am`: the two pages of Adobe's documentation disagree, and each function follows its own
/// page, so `AFTime_Format(1)` writes `PM` and `AFTime_FormatEx("h:MM tt")` writes `pm` (ADR 1578
/// section 6).
pub(crate) const TIME_PICTURES: [&str; 4] = ["HH:MM", "h:MM tt", "HH:MM:ss", "h:MM:ss tt"];

/// `AFTime_Format`'s text for a moment, with the menu's capital marker.
pub(crate) fn print_time_menu(index: usize, moment: DateTime) -> Option<String> {
    let picture = TIME_PICTURES.get(index)?;
    let written = print_date(picture, &moment).ok()?;
    Some(if index % 2 == 1 {
        written.replace("am", "AM").replace("pm", "PM")
    } else {
        written
    })
}

#[cfg(test)]
mod tests {
    use super::{DateTime, parse_date, print_date};

    #[test]
    fn a_fixed_width_picture_reads_digits_without_separators() {
        let read = parse_date("20240115", "yyyymmdd").expect("a date");
        assert_eq!((read.year, read.month, read.day), (2024, 1, 15));
    }

    #[test]
    fn a_date_that_does_not_exist_refuses() {
        assert_eq!(parse_date("4/31/2024", "m/d/yyyy"), None);
        assert!(parse_date("2/29", "m/d").is_some());
    }

    #[test]
    fn noon_and_midnight_on_a_twelve_hour_clock() {
        let noon = parse_date("12:00 pm", "h:MM tt").expect("noon");
        assert_eq!(noon.hour, 12);
        let midnight = parse_date("12:00 am", "h:MM tt").expect("midnight");
        assert_eq!(midnight.hour, 0);
        let written = print_date(
            "h:MM tt",
            &DateTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 0,
                minute: 5,
                second: 0,
            },
        )
        .expect("a picture");
        assert_eq!(written, "12:05 am");
    }
}
