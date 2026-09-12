//! ISO 8601's date and date-and-time-of-day forms, as the working draft this project holds
//! defines them, read for `Lexical::Date`.
//!
//! # The text, and what it is not
//!
//! The ground is **ISO/WD 8601-1**, ISO/TC 154/WG 5 N0038 of 2016-02-16 — *Data elements and
//! interchange formats — Information interchange — Representation of dates and times — Part 1:
//! Basic rules* — held at `doc/iso-tc154-wg5_n0038_iso_wd_8601-1_2016-02-16.pdf` since
//! `doc/questions/A53`. It is a working draft and its cover says so: not an International
//! Standard, and not to be referred to as one. So every citation below is to the WD's own clause
//! numbers, and the published ISO 8601-1:2016 is a different document — the purchase to make on
//! the day a difference between the two matters. The cover permits reproduction to participants
//! in the standards process and to nobody else, so **nothing here quotes it**: the grammar is
//! stated in this crate's words, clause by clause, and `doc/third-party-data.md` carries the row.
//!
//! # Why a grammar rather than a list
//!
//! `TechNote 0010` A020 resolves that an XMP `Date` is validated as ISO 8601 and nothing more
//! (`crate::clarification`). Until session 993 the check was the XMP Specification's six
//! profiles of ISO 8601, and `doc/questions/Q53` recorded the gap: a date in any other ISO 8601
//! form was refused. The six profiles remain what an XMP packet is *written* in — `pdf_model::xmp`
//! emits them and this module does not change that — and what moves is what the validator
//! *admits*. ADR 1013 has the decision.
//!
//! # The grammar, in this crate's words
//!
//! The notation is the WD's own (its 3.4.2): a letter per digit, `±` a sign, `T`, `W` and `Z`
//! the designators (3.4.3), `-` and `:` the separators (3.4.4). A form is admitted when the WD's
//! *body* defines it; a form it mentions only in a NOTE is not, because its notes are
//! informative. Forms the body permits by mutual agreement of the interchange partners are
//! admitted too: a validator is not a party to any agreement and cannot know one is absent, and
//! refusing them would be the false failure — the wrong direction for this crate — that
//! `doc/questions/Q53` chose to avoid.
//!
//! ## Dates (4.1)
//!
//! | form | basic | extended | WD clause |
//! |---|---|---|---|
//! | calendar date | `YYYYMMDD` | `YYYY-MM-DD` | 4.1.2.2 |
//! | a month | `YYYY-MM` | — | 4.1.2.3 a) |
//! | a year | `YYYY` | — | 4.1.2.3 b) |
//! | a century | `YY` | — | 4.1.2.3 c) |
//! | ordinal date | `YYYYDDD` | `YYYY-DDD` | 4.1.3.2 |
//! | week date | `YYYYWwwD` | `YYYY-Www-D` | 4.1.4.2 |
//! | a week | `YYYYWww` | `YYYY-Www` | 4.1.4.3 |
//!
//! Each may be **expanded** (4.1.2.4, 4.1.3.3, 4.1.4.4, resting on 3.5): a sign, then a year of
//! four digits and however many more the partners agreed, in place of `YYYY`; an expanded
//! century is a sign and two digits or more. The sign is required, the plus included (3.4.2),
//! and the WD's own minus sign is admitted beside the hyphen-minus it maps onto (3.4.1). In basic
//! format the agreed number of extra digits is written nowhere a validator can read, so a signed
//! run of digits is admitted as whichever expanded basic form its length allows —
//! [`Date::Agreed`].
//!
//! The elements' values, from 4.1.2.1, 4.1.3.1 and 4.1.4.1 over the calendars of 3.2: month
//! `01`–`12`; day of the month `01` to the month's length in the Gregorian calendar, applied
//! proleptically, with the leap rule 3.2.1 states; day of the year `001`–`365`, or `366` in a
//! leap year (Table 1); week `01` to `52` or `53`, by 2.2.10's definition of the week number — the
//! first week of a year is the one holding its first Thursday — which 3.2.2's reference point
//! makes computable for any year; day of the week `1`–`7` (Table 2). Every element of defined
//! length keeps its leading zeros (3.6).
//!
//! ## A time of day after a date (4.2, 4.3)
//!
//! A date and a time of day are one expression in the order date, `T`, time, zone (4.3.2). The
//! time is one of 4.2.2.2's and 4.2.2.3's — `hhmmss`, `hhmm` or `hh` in basic format, `hh:mm:ss`,
//! `hh:mm` or `hh` in extended — and its lowest element may carry a decimal fraction of at least
//! one digit after a comma or a full stop (4.2.2.4). The zone is empty for local time (4.2.2),
//! `Z` for UTC (4.2.4), or the difference from UTC appended without a space (4.2.5.2), which is
//! `±hhmm` or `±hh` in basic format and `±hh:mm` or `±hh` in extended (4.2.5.1). Hour `00`–`23`,
//! minute `00`–`59`, second `00`–`60` (4.2.1); the hour `24` that clause reserves for the end of
//! a day within a time interval, and a date is a time point.
//!
//! Two rules of 4.3.3 bind the combination: the date in front of a time is never one of reduced
//! accuracy (c), and the whole expression is in one format (d) — a `YYYY-MM-DD` date takes an
//! `hh:mm` time and a `±hh:mm` difference, never `hhmm` or `±hhmm`. An element with no separator
//! of its own, `hh` or `±hh`, is of either format.
//!
//! ## What is refused, and named
//!
//! A time of day on its own (4.2 without 4.1: the type is a date), a time interval (4.4, the
//! solidus) or a recurring one (4.5, `R`), a duration (4.4.3, `P`), a space anywhere (3.4.1), the
//! `T` omitted (4.3.2 permits that by agreement in a NOTE, and a note defines nothing), and any
//! character where the grammar expects another. [`Refusal`] says which, and the finding prints it,
//! so that a reader of a verdict sees the form the value missed rather than the name of a
//! standard.

use std::fmt::{self, Write as _};

/// The two formats of 4.1.2.2 and 4.2.2.2: without separators, and with them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// No separators: `YYYYMMDD`, `hhmmss`, `±hhmm`.
    Basic,
    /// Hyphens between date elements and colons between time elements.
    Extended,
}

/// The date half of a form, one of the WD's clause 4.1 representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Date {
    /// `YY` (4.1.2.3 c), or `±YY…` expanded (4.1.2.4 d).
    Century {
        /// Signed, with the digits the partners agreed (4.1.2.4).
        expanded: bool,
    },
    /// `YYYY` (4.1.2.3 b), or `±YYYY…` expanded (4.1.2.4 c).
    Year {
        /// Signed, with the digits the partners agreed (4.1.2.4).
        expanded: bool,
    },
    /// `YYYY-MM` (4.1.2.3 a), or `±YYYY…-MM` expanded (4.1.2.4 b).
    Month {
        /// Signed, with the digits the partners agreed (4.1.2.4).
        expanded: bool,
    },
    /// `YYYYMMDD` or `YYYY-MM-DD` (4.1.2.2), and the expanded extended `±YYYY…-MM-DD` (4.1.2.4 a).
    Calendar {
        /// Signed, with the digits the partners agreed (4.1.2.4).
        expanded: bool,
        /// Which of the two formats.
        format: Format,
    },
    /// `YYYYDDD` or `YYYY-DDD` (4.1.3.2), and the expanded extended `±YYYY…-DDD` (4.1.3.3).
    Ordinal {
        /// Signed, with the digits the partners agreed (4.1.3.3).
        expanded: bool,
        /// Which of the two formats.
        format: Format,
    },
    /// `YYYYWwwD`, `YYYY-Www-D` (4.1.4.2) and the week alone, `YYYYWww` or `YYYY-Www` (4.1.4.3),
    /// each with its expanded form (4.1.4.4).
    Week {
        /// Signed, with the digits the partners agreed (4.1.4.4).
        expanded: bool,
        /// Which of the two formats.
        format: Format,
        /// Whether the day of the week is stated.
        day: bool,
    },
    /// A sign and a run of digits: an expanded date in basic format, whose reading — a calendar
    /// date (4.1.2.4 a), an ordinal date (4.1.3.3), a year (4.1.2.4 c) or a century (4.1.2.4 d)
    /// — depends on how many digits the interchange partners agreed to add to the year, which
    /// no validator holds. Admitted as whichever of those its length allows.
    Agreed {
        /// How many digits follow the sign.
        digits: usize,
    },
}

impl Date {
    /// Whether this is one of the complete representations 4.3.3 c) lets a time of day follow.
    ///
    /// A signed run of digits is complete when some complete reading of it exists: an ordinal
    /// date needs a four-digit year and three digits of day.
    const fn complete(self) -> bool {
        match self {
            Self::Calendar { .. } | Self::Ordinal { .. } | Self::Week { day: true, .. } => true,
            Self::Agreed { digits } => digits >= 7,
            Self::Century { .. }
            | Self::Year { .. }
            | Self::Month { .. }
            | Self::Week { day: false, .. } => false,
        }
    }

    /// The format the date commits the whole expression to, where it has separators to commit
    /// with.
    ///
    /// A month or a week alone is written with a hyphen and is nonetheless 4.1.2.3's *basic*
    /// format; that does not matter here, because neither may precede a time (4.3.3 c) and the
    /// question only arises when one does.
    const fn format(self) -> Option<Format> {
        match self {
            Self::Calendar { format, .. }
            | Self::Ordinal { format, .. }
            | Self::Week { format, .. } => Some(format),
            Self::Agreed { .. } => Some(Format::Basic),
            Self::Century { .. } | Self::Year { .. } | Self::Month { .. } => None,
        }
    }
}

/// The lowest time element a time of day states (4.2.2.2, 4.2.2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    /// `hh`.
    Hour,
    /// `hhmm` or `hh:mm`.
    Minute,
    /// `hhmmss` or `hh:mm:ss`.
    Second,
}

/// The decimal sign of 4.2.2.4, either of the two ISO 31-0 allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decimal {
    /// `,`, the WD's preferred sign.
    Comma,
    /// `.`.
    FullStop,
}

/// The zone designator of 4.3.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// Nothing: local time (4.2.2).
    Local,
    /// `Z` (4.2.4).
    Utc,
    /// The difference from UTC (4.2.5.1, 4.2.5.2).
    Difference {
        /// `±hhmm` or `±hh:mm` rather than `±hh`.
        minutes: bool,
        /// Which format the difference is written in, where its spelling says.
        format: Option<Format>,
    },
}

/// The time-of-day half of a form (4.2, in the combination 4.3 defines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    /// The lowest element stated.
    pub precision: Precision,
    /// Which format the time is written in, where its spelling says; `hh` alone says nothing.
    pub format: Option<Format>,
    /// A decimal fraction of the lowest element: its sign and how many digits it has.
    pub fraction: Option<(Decimal, usize)>,
    /// The zone designator.
    pub zone: Zone,
}

/// One admitted representation, described as the form it is rather than the value it states.
///
/// [`fmt::Display`] prints it in the WD's format-representation notation — `YYYY-MM-DDThh:mm:ssZ`
/// — which is what `examples/dates.rs` counts by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form {
    /// The date.
    pub date: Date,
    /// The time of day, where one follows.
    pub time: Option<Time>,
}

impl fmt::Display for Form {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let year = |expanded: bool| if expanded { "±YYYYY" } else { "YYYY" };
        match self.date {
            Date::Century { expanded } => f.write_str(if expanded { "±YYY" } else { "YY" })?,
            Date::Year { expanded } => f.write_str(year(expanded))?,
            Date::Month { expanded } => write!(f, "{}-MM", year(expanded))?,
            Date::Calendar { expanded, format } => match format {
                Format::Basic => write!(f, "{}MMDD", year(expanded))?,
                Format::Extended => write!(f, "{}-MM-DD", year(expanded))?,
            },
            Date::Ordinal { expanded, format } => match format {
                Format::Basic => write!(f, "{}DDD", year(expanded))?,
                Format::Extended => write!(f, "{}-DDD", year(expanded))?,
            },
            Date::Week {
                expanded,
                format,
                day,
            } => {
                let (week, weekday) = match format {
                    Format::Basic => ("Www", "D"),
                    Format::Extended => ("-Www", "-D"),
                };
                write!(f, "{}{week}", year(expanded))?;
                if day {
                    f.write_str(weekday)?;
                }
            }
            Date::Agreed { digits } => write!(f, "±Y×{digits}")?,
        }
        let Some(time) = self.time else {
            return Ok(());
        };
        let colon = time.format == Some(Format::Extended);
        f.write_str("Thh")?;
        let lowest = match time.precision {
            Precision::Hour => 'h',
            Precision::Minute => {
                f.write_str(if colon { ":mm" } else { "mm" })?;
                'm'
            }
            Precision::Second => {
                f.write_str(if colon { ":mm:ss" } else { "mmss" })?;
                's'
            }
        };
        if let Some((sign, digits)) = time.fraction {
            f.write_char(match sign {
                Decimal::Comma => ',',
                Decimal::FullStop => '.',
            })?;
            for _ in 0..digits {
                f.write_char(lowest)?;
            }
        }
        match time.zone {
            Zone::Local => Ok(()),
            Zone::Utc => f.write_str("Z"),
            Zone::Difference { minutes: false, .. } => f.write_str("±hh"),
            Zone::Difference {
                minutes: true,
                format,
            } => f.write_str(if format == Some(Format::Extended) {
                "±hh:mm"
            } else {
                "±hhmm"
            }),
        }
    }
}

/// Why a value is none of the forms the WD defines for a date, in a phrase a finding prints
/// after the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// No characters at all.
    Nothing,
    /// A space or other white space, which 3.4.1 keeps out of every representation.
    Space,
    /// A solidus, which 4.4.2 makes the separator of a time interval.
    Interval,
    /// A leading `R`, the recurring-interval designator (3.4.3, 4.5).
    Recurring,
    /// A leading `P`, the duration designator (3.4.3, 4.4.3).
    Duration,
    /// A leading `T`: a time of day with no date in front of it (4.2.2.5).
    TimeOnly,
    /// An unsigned run of digits of a length no date form has (4.1).
    Digits(usize),
    /// A sign and fewer digits than the shortest expanded form, the century's two (4.1.2.4 d).
    Expanded(usize),
    /// A run of digits after the hyphen of a length no extended form has: a month has two, an
    /// ordinal day three (4.1.2.2, 4.1.3.2).
    AfterHyphen(usize),
    /// A month outside `01`–`12` (3.2.1's Table 1).
    Month(u32),
    /// A day of the month past the month's length (3.2.1's Table 1, with its leap rule).
    Day {
        /// The day stated.
        day: u32,
        /// The month stated.
        month: u32,
        /// How many days that month has in that year.
        length: u32,
    },
    /// A day of the year past the year's length (3.2.1's Table 1).
    Ordinal {
        /// The day stated.
        day: u32,
        /// How many days that year has.
        length: u32,
    },
    /// A week outside `01` to the year's last (2.2.10, 3.2.2).
    Week {
        /// The week stated.
        week: u32,
        /// How many weeks that year has.
        weeks: u32,
    },
    /// A day of the week outside `1`–`7` (3.2.2's Table 2).
    Weekday(u32),
    /// An hour of `24`, which 4.2.1 admits only for the end of a day within a time interval.
    EndOfDay,
    /// An hour past `24` (4.2.1).
    Hour(u32),
    /// A minute past `59` (4.2.1).
    Minute(u32),
    /// A second past `60` (4.2.1).
    Second(u32),
    /// A decimal sign with no digit after it (4.2.2.4).
    Fraction,
    /// A date of reduced accuracy in front of a time of day (4.3.3 c).
    Reduced,
    /// Basic and extended format in one expression (4.3.3 d).
    Mixed,
    /// The text ends where an element was expected.
    Missing(&'static str),
    /// A character where the grammar has no place for it.
    Unexpected {
        /// The character.
        found: char,
        /// What had been read when it was met.
        after: &'static str,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Nothing => f.write_str("which states nothing"),
            Self::Space => f.write_str("with a space, which no ISO 8601 representation contains"),
            Self::Interval => {
                f.write_str("with a solidus, which makes it a time interval rather than a date")
            }
            Self::Recurring => {
                f.write_str("which begins with R, the recurring-interval designator")
            }
            Self::Duration => f.write_str("which begins with P, the duration designator"),
            Self::TimeOnly => f.write_str("which is a time of day with no date in front of it"),
            Self::Digits(count) => write!(
                f,
                "a run of {count} digits, where a century has two, a year four, an ordinal date \
                 seven and a calendar date eight"
            ),
            Self::Expanded(count) => write!(
                f,
                "a sign and {count} digit(s), where the shortest expanded form has two"
            ),
            Self::AfterHyphen(count) => write!(
                f,
                "a run of {count} digit(s) after the year's hyphen, where a month has two and an \
                 ordinal day three"
            ),
            Self::Month(month) => write!(f, "a month of {month:02}, where the calendar has twelve"),
            Self::Day { day, month, length } => write!(
                f,
                "a day of {day:02} in month {month:02}, which has {length} days that year"
            ),
            Self::Ordinal { day, length } => {
                write!(
                    f,
                    "a day of the year of {day:03}, in a year of {length} days"
                )
            }
            Self::Week { week, weeks } => {
                write!(f, "a week of {week:02}, in a year of {weeks} weeks")
            }
            Self::Weekday(day) => {
                write!(
                    f,
                    "a day of the week of {day}, where Monday is 1 and Sunday 7"
                )
            }
            Self::EndOfDay => f.write_str(
                "an hour of 24, which ISO 8601 admits only for the end of a day within a time \
                 interval",
            ),
            Self::Hour(hour) => write!(f, "an hour of {hour:02}, past the day's 24"),
            Self::Minute(minute) => write!(f, "a minute of {minute:02}, past 59"),
            Self::Second(second) => {
                write!(f, "a second of {second:02}, past the leap second's 60")
            }
            Self::Fraction => f.write_str("a decimal sign followed by no digit"),
            Self::Reduced => f.write_str(
                "a date of reduced accuracy in front of a time of day, where the date has to be \
                 complete",
            ),
            Self::Mixed => f.write_str("basic and extended format mixed in one expression"),
            Self::Missing(wanted) => write!(f, "which ends where {wanted} was expected"),
            Self::Unexpected { found, after } => {
                write!(
                    f,
                    "with {found:?} after {after}, where the form has no place for it"
                )
            }
        }
    }
}

/// Whether a value is one of the forms the WD defines for a date, or a date and time of day,
/// and which.
///
/// The value arrives as the packet spelled it; a caller that wants surrounding white space
/// ignored trims first, because a space *inside* the value is a refusal (3.4.1).
///
/// # Errors
///
/// A [`Refusal`] naming the first thing the value does that no form does.
pub fn read(text: &str) -> Result<Form, Refusal> {
    if text.is_empty() {
        return Err(Refusal::Nothing);
    }
    if text.chars().any(char::is_whitespace) {
        return Err(Refusal::Space);
    }
    if text.contains('/') {
        return Err(Refusal::Interval);
    }
    let mut cursor = Cursor {
        chars: text.chars().collect(),
        at: 0,
        after: "the start",
    };
    match cursor.peek() {
        Some('R') => return Err(Refusal::Recurring),
        Some('P') => return Err(Refusal::Duration),
        Some('T') => return Err(Refusal::TimeOnly),
        _ => {}
    }
    let date = read_date(&mut cursor)?;
    let time = if cursor.take('T') {
        if !date.complete() {
            return Err(Refusal::Reduced);
        }
        cursor.after = "the time designator";
        Some(read_time(&mut cursor)?)
    } else {
        None
    };
    if let Some(found) = cursor.peek() {
        return Err(Refusal::Unexpected {
            found,
            after: cursor.after,
        });
    }
    // 4.3.3 d): one format for the whole expression.
    if let Some(time) = time {
        let zone = match time.zone {
            Zone::Difference { format, .. } => format,
            Zone::Local | Zone::Utc => None,
        };
        let mut formats = [date.format(), time.format, zone].into_iter().flatten();
        if let Some(first) = formats.next()
            && formats.any(|format| format != first)
        {
            return Err(Refusal::Mixed);
        }
    }
    Ok(Form { date, time })
}

/// A cursor over the value's characters, remembering what it last read for a refusal's phrase.
struct Cursor {
    chars: Vec<char>,
    at: usize,
    after: &'static str,
}

impl Cursor {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn digit_next(&self) -> bool {
        self.peek().is_some_and(|c| c.is_ascii_digit())
    }

    fn take(&mut self, wanted: char) -> bool {
        if self.peek() == Some(wanted) {
            self.at = self.at.saturating_add(1);
            true
        } else {
            false
        }
    }

    /// A sign: the plus, or the minus in either spelling 3.4.1 maps onto the hyphen-minus.
    fn sign(&mut self) -> bool {
        if matches!(self.peek(), Some('+' | '-' | '\u{2212}')) {
            self.at = self.at.saturating_add(1);
            true
        } else {
            false
        }
    }

    /// Every ASCII digit from here, as a string; empty where there is none.
    fn digits(&mut self) -> String {
        let start = self.at;
        while self.digit_next() {
            self.at = self.at.saturating_add(1);
        }
        self.chars
            .get(start..self.at)
            .unwrap_or_default()
            .iter()
            .collect()
    }

    /// Exactly `count` digits, as a number, or nothing without moving.
    fn fixed(&mut self, count: usize) -> Option<u32> {
        let end = self.at.checked_add(count)?;
        let slice = self.chars.get(self.at..end)?;
        if !slice.iter().all(char::is_ascii_digit) {
            return None;
        }
        self.at = end;
        Some(number(&slice.iter().collect::<String>()))
    }
}

/// A run of ASCII digits as a number, saturating — every caller has bounded the run's length,
/// and a saturated value fails the range check it is about to meet.
fn number(digits: &str) -> u32 {
    digits.bytes().fold(0u32, |acc, b| {
        acc.saturating_mul(10)
            .saturating_add(u32::from(b.saturating_sub(b'0')))
    })
}

/// A year as the calendar rules need it: its residue modulo 400, which is all the leap rule and
/// the week rule depend on, because the Gregorian calendar repeats every 400 years — 146 097
/// days, exactly 20 871 weeks.
///
/// Folding the digits keeps a year of any agreed length (3.5) from overflowing anything, and the
/// sign is applied to the residue: the year before `0000` is `-0001`, whose residue is 399.
#[derive(Debug, Clone, Copy)]
struct Year(u32);

impl Year {
    fn of(digits: &str, negative: bool) -> Self {
        // `acc` stays below 400, so the saturating forms never saturate; they satisfy the
        // arithmetic lint without a proof in a comment somewhere else.
        let residue = digits.bytes().fold(0u32, |acc, b| {
            acc.saturating_mul(10)
                .saturating_add(u32::from(b.saturating_sub(b'0')))
                % 400
        });
        Self(if negative {
            400u32.saturating_sub(residue) % 400
        } else {
            residue
        })
    }

    /// 3.2.1's rule: divisible by four, except a centennial year not divisible by four hundred.
    const fn leap(self) -> bool {
        self.0.is_multiple_of(4) && (!self.0.is_multiple_of(100) || self.0 == 0)
    }

    const fn days(self) -> u32 {
        if self.leap() { 366 } else { 365 }
    }

    /// Table 1's lengths.
    const fn month_length(self, month: u32) -> u32 {
        match month {
            2 => {
                if self.leap() {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }

    /// How many calendar weeks the year has, by 2.2.10: the first week is the one holding the
    /// year's first Thursday and the last precedes the next year's first, so a year has as many
    /// weeks as it has Thursdays — 53 when 1 January is a Thursday, or a Wednesday in a leap
    /// year, and 52 otherwise.
    ///
    /// 1 January's weekday comes from 3.2.2's reference point, Saturday for 1 January 2000, and
    /// from counting the leap days 3.2.1's rule puts between year 0 and this one; 2000 is a
    /// multiple of 400, so year 0's 1 January is a Saturday as well and the count runs from 0.
    fn weeks(self) -> u32 {
        let year = self.0;
        let leap_days_before = year
            .saturating_add(3)
            .saturating_div(4)
            .saturating_sub(year.saturating_add(99).saturating_div(100))
            .saturating_add(year.saturating_add(399).saturating_div(400));
        // Monday is 0 here, Table 2's numbering less one; Saturday is 5.
        let january_first = 5u32
            .saturating_add(year.saturating_mul(365))
            .saturating_add(leap_days_before)
            % 7;
        let (wednesday, thursday) = (2, 3);
        if january_first == thursday || (self.leap() && january_first == wednesday) {
            53
        } else {
            52
        }
    }
}

/// Clause 4.1: the date.
fn read_date(cursor: &mut Cursor) -> Result<Date, Refusal> {
    let negative = matches!(cursor.peek(), Some('-' | '\u{2212}'));
    let expanded = cursor.sign();
    let digits = cursor.digits();
    let count = digits.len();
    cursor.after = "the year";
    let year_ok = if expanded { count >= 4 } else { count == 4 };
    match cursor.peek() {
        None | Some('T') if expanded => {
            if count < 2 {
                return Err(Refusal::Expanded(count));
            }
            Ok(Date::Agreed { digits: count })
        }
        None | Some('T') => read_run(cursor, &digits),
        Some('W') => {
            if !year_ok {
                return Err(bad_year(expanded, count));
            }
            let year = Year::of(&digits, negative);
            cursor.take('W');
            let day = read_week(cursor, year)?;
            Ok(Date::Week {
                expanded,
                format: Format::Basic,
                day,
            })
        }
        Some('-') => {
            if !year_ok {
                return Err(bad_year(expanded, count));
            }
            let year = Year::of(&digits, negative);
            cursor.take('-');
            if cursor.take('W') {
                let day = read_week(cursor, year)?;
                return Ok(Date::Week {
                    expanded,
                    format: Format::Extended,
                    day,
                });
            }
            let run = cursor.digits();
            match run.len() {
                2 => {
                    let month = number(&run);
                    cursor.after = "the month";
                    if cursor.take('-') {
                        let Some(day) = cursor.fixed(2) else {
                            return Err(Refusal::Missing("a day of two digits"));
                        };
                        calendar(year, month, day)?;
                        cursor.after = "the day";
                        Ok(Date::Calendar {
                            expanded,
                            format: Format::Extended,
                        })
                    } else {
                        if month == 0 || month > 12 {
                            return Err(Refusal::Month(month));
                        }
                        Ok(Date::Month { expanded })
                    }
                }
                3 => {
                    ordinal(year, number(&run))?;
                    cursor.after = "the day of the year";
                    Ok(Date::Ordinal {
                        expanded,
                        format: Format::Extended,
                    })
                }
                other => Err(Refusal::AfterHyphen(other)),
            }
        }
        Some(found) => Err(Refusal::Unexpected {
            found,
            after: cursor.after,
        }),
    }
}

/// An unsigned run of digits and nothing after it but the end or a `T`: a century, a year, or
/// a basic-format ordinal or calendar date, told apart by length (4.1.2.2, 4.1.2.3, 4.1.3.2).
fn read_run(cursor: &mut Cursor, digits: &str) -> Result<Date, Refusal> {
    let count = digits.len();
    let year = Year::of(digits.get(..4).unwrap_or_default(), false);
    match count {
        2 => Ok(Date::Century { expanded: false }),
        4 => Ok(Date::Year { expanded: false }),
        7 => {
            ordinal(year, number(digits.get(4..).unwrap_or_default()))?;
            cursor.after = "the day of the year";
            Ok(Date::Ordinal {
                expanded: false,
                format: Format::Basic,
            })
        }
        8 => {
            calendar(
                year,
                number(digits.get(4..6).unwrap_or_default()),
                number(digits.get(6..).unwrap_or_default()),
            )?;
            cursor.after = "the day";
            Ok(Date::Calendar {
                expanded: false,
                format: Format::Basic,
            })
        }
        _ => Err(Refusal::Digits(count)),
    }
}

/// A year of the wrong length, said in the terms of the form being read.
const fn bad_year(expanded: bool, count: usize) -> Refusal {
    if expanded {
        Refusal::Expanded(count)
    } else {
        Refusal::Digits(count)
    }
}

/// After the `W`: the week, and whether a day of the week follows (4.1.4.2, 4.1.4.3).
fn read_week(cursor: &mut Cursor, year: Year) -> Result<bool, Refusal> {
    let Some(week) = cursor.fixed(2) else {
        return Err(Refusal::Missing("a week of two digits after W"));
    };
    let weeks = year.weeks();
    if week == 0 || week > weeks {
        return Err(Refusal::Week { week, weeks });
    }
    cursor.after = "the week";
    let separated = cursor.take('-');
    let Some(day) = cursor.fixed(1) else {
        if separated {
            return Err(Refusal::Missing("a day of the week after the hyphen"));
        }
        return Ok(false);
    };
    if day == 0 || day > 7 {
        return Err(Refusal::Weekday(day));
    }
    cursor.after = "the day of the week";
    Ok(true)
}

/// 4.1.2.1 over Table 1: a month the calendar has, and a day that month has.
fn calendar(year: Year, month: u32, day: u32) -> Result<(), Refusal> {
    if month == 0 || month > 12 {
        return Err(Refusal::Month(month));
    }
    let length = year.month_length(month);
    if day == 0 || day > length {
        return Err(Refusal::Day { day, month, length });
    }
    Ok(())
}

/// 4.1.3.1 over Table 1: a day of the year the year has.
fn ordinal(year: Year, day: u32) -> Result<(), Refusal> {
    let length = year.days();
    if day == 0 || day > length {
        return Err(Refusal::Ordinal { day, length });
    }
    Ok(())
}

/// Clause 4.2 after the `T`: the time of day and its zone designator.
fn read_time(cursor: &mut Cursor) -> Result<Time, Refusal> {
    let Some(hour) = cursor.fixed(2) else {
        return Err(Refusal::Missing("an hour of two digits after T"));
    };
    match hour {
        24 => return Err(Refusal::EndOfDay),
        25.. => return Err(Refusal::Hour(hour)),
        _ => {}
    }
    cursor.after = "the hour";
    let mut precision = Precision::Hour;
    let mut format = None;
    let separated = cursor.take(':');
    if separated || cursor.digit_next() {
        format = Some(if separated {
            Format::Extended
        } else {
            Format::Basic
        });
        let Some(minute) = cursor.fixed(2) else {
            return Err(Refusal::Missing("a minute of two digits"));
        };
        if minute > 59 {
            return Err(Refusal::Minute(minute));
        }
        precision = Precision::Minute;
        cursor.after = "the minute";
        // A colon after basic minutes, or digits straight after extended ones, is the other
        // format's spelling of the seconds (4.3.3 d).
        let again = cursor.take(':');
        if again != separated && (again || cursor.digit_next()) {
            return Err(Refusal::Mixed);
        }
        if again || cursor.digit_next() {
            let Some(second) = cursor.fixed(2) else {
                return Err(Refusal::Missing("a second of two digits"));
            };
            if second > 60 {
                return Err(Refusal::Second(second));
            }
            precision = Precision::Second;
            cursor.after = "the second";
        }
    }
    let fraction = match cursor.peek() {
        Some(sign @ (',' | '.')) => {
            cursor.take(sign);
            let digits = cursor.digits().len();
            if digits == 0 {
                return Err(Refusal::Fraction);
            }
            cursor.after = "the decimal fraction";
            Some((
                if sign == ',' {
                    Decimal::Comma
                } else {
                    Decimal::FullStop
                },
                digits,
            ))
        }
        _ => None,
    };
    let zone = if cursor.take('Z') {
        cursor.after = "the UTC designator";
        Zone::Utc
    } else if cursor.sign() {
        if cursor.fixed(2).is_none() {
            return Err(Refusal::Missing(
                "an hour of two digits in the difference from UTC",
            ));
        }
        cursor.after = "the difference from UTC";
        let separated = cursor.take(':');
        if separated || cursor.digit_next() {
            if cursor.fixed(2).is_none() {
                return Err(Refusal::Missing(
                    "a minute of two digits in the difference from UTC",
                ));
            }
            Zone::Difference {
                minutes: true,
                format: Some(if separated {
                    Format::Extended
                } else {
                    Format::Basic
                }),
            }
        } else {
            Zone::Difference {
                minutes: false,
                format: None,
            }
        }
    } else {
        Zone::Local
    };
    Ok(Time {
        precision,
        format,
        fraction,
        zone,
    })
}

#[cfg(test)]
mod tests {
    use super::{Refusal, Year, read};

    /// The form a value has, printed the WD's way, or the refusal's phrase.
    fn form(text: &str) -> String {
        match read(text) {
            Ok(form) => form.to_string(),
            Err(refusal) => format!("refused: {refusal}"),
        }
    }

    /// The six date profiles the XMP Specification lists — what a packet is written in — are
    /// each one of the WD's forms, so nothing the check used to admit is refused.
    #[test]
    fn the_xmp_profiles_are_all_forms_the_working_draft_defines() {
        for (value, pattern) in [
            ("2016", "YYYY"),
            ("2016-02", "YYYY-MM"),
            ("2016-02-01", "YYYY-MM-DD"),
            ("2016-02-01T13:19Z", "YYYY-MM-DDThh:mmZ"),
            ("2016-02-01T13:19:21+01:00", "YYYY-MM-DDThh:mm:ss±hh:mm"),
            ("2016-02-01T13:19:21.5-06:00", "YYYY-MM-DDThh:mm:ss.s±hh:mm"),
        ] {
            assert_eq!(form(value), pattern, "{value}");
        }
    }

    /// Clause 4.1's date forms: complete, reduced and expanded, in both formats.
    #[test]
    fn every_date_form_of_clause_four_point_one_is_admitted() {
        for (value, pattern) in [
            ("19850412", "YYYYMMDD"),
            ("1985-04-12", "YYYY-MM-DD"),
            ("1985-04", "YYYY-MM"),
            ("1985", "YYYY"),
            ("19", "YY"),
            ("1985102", "YYYYDDD"),
            ("1985-102", "YYYY-DDD"),
            ("1985W155", "YYYYWwwD"),
            ("1985-W15-5", "YYYY-Www-D"),
            ("1985W15", "YYYYWww"),
            ("1985-W15", "YYYY-Www"),
            ("+001985-04-12", "±YYYYY-MM-DD"),
            ("+001985-04", "±YYYYY-MM"),
            ("+001985-102", "±YYYYY-DDD"),
            ("+001985-W15-5", "±YYYYY-Www-D"),
            ("+001985W15", "±YYYYYWww"),
            ("-0002-04-12", "±YYYYY-MM-DD"),
            ("\u{2212}0002-04-12", "±YYYYY-MM-DD"),
            ("+0019850412", "±Y×10"),
            ("+001985", "±Y×6"),
            ("+0019", "±Y×4"),
        ] {
            assert_eq!(form(value), pattern, "{value}");
        }
    }

    /// Clause 4.2's times of day after a complete date, with every zone designator of 4.3.2.
    #[test]
    fn every_time_form_after_a_complete_date_is_admitted() {
        for (value, pattern) in [
            ("19850412T101530", "YYYYMMDDThhmmss"),
            ("19850412T101530Z", "YYYYMMDDThhmmssZ"),
            ("19850412T101530+0400", "YYYYMMDDThhmmss±hhmm"),
            ("19850412T101530+04", "YYYYMMDDThhmmss±hh"),
            ("1985-04-12T10:15:30", "YYYY-MM-DDThh:mm:ss"),
            ("1985-04-12T10:15:30+04", "YYYY-MM-DDThh:mm:ss±hh"),
            ("19850412T1015", "YYYYMMDDThhmm"),
            ("1985-04-12T10:15", "YYYY-MM-DDThh:mm"),
            ("1985102T1015Z", "YYYYDDDThhmmZ"),
            ("1985-102T10:15Z", "YYYY-DDDThh:mmZ"),
            ("1985W155T1015+0400", "YYYYWwwDThhmm±hhmm"),
            ("1985-W15-5T10:15+04", "YYYY-Www-DThh:mm±hh"),
            ("1985-04-12T10", "YYYY-MM-DDThh"),
            ("1985-04-12T10Z", "YYYY-MM-DDThhZ"),
            ("1985-04-12T10:15:30,5", "YYYY-MM-DDThh:mm:ss,s"),
            ("1985-04-12T10:15,8Z", "YYYY-MM-DDThh:mm,mZ"),
            ("1985-04-12T23,3+01:00", "YYYY-MM-DDThh,h±hh:mm"),
            ("19850412T232050,5", "YYYYMMDDThhmmss,s"),
            (
                "1985-04-12T10:15:30.123456789+01:00",
                "YYYY-MM-DDThh:mm:ss.sssssssss±hh:mm",
            ),
            ("1985-04-12T23:59:60Z", "YYYY-MM-DDThh:mm:ssZ"),
            ("1985-04-12T00:00:00", "YYYY-MM-DDThh:mm:ss"),
            ("+0019850412T101530", "±Y×10Thhmmss"),
        ] {
            assert_eq!(form(value), pattern, "{value}");
        }
    }

    /// What 4.3.3 forbids in a combination, and what 4.2.1 keeps out of a time point, each
    /// refused with its own phrase.
    #[test]
    fn the_combination_rules_are_held() {
        for (value, refusal) in [
            ("1985-04T10:15", Refusal::Reduced),
            ("1985T10", Refusal::Reduced),
            ("1985-W15T10", Refusal::Reduced),
            ("19T10", Refusal::Reduced),
            ("+001985T10", Refusal::Reduced),
            ("1985-04-12T101530", Refusal::Mixed),
            ("19850412T10:15:30", Refusal::Mixed),
            ("1985-04-12T10:15:30+0100", Refusal::Mixed),
            ("19850412T101530+01:00", Refusal::Mixed),
            ("1985-04-12T10:1530", Refusal::Mixed),
            ("19850412T1015:30", Refusal::Mixed),
            ("1985-04-12T24:00:00", Refusal::EndOfDay),
            ("1985-04-12T25:00:00", Refusal::Hour(25)),
            ("1985-04-12T10:60:00", Refusal::Minute(60)),
            ("1985-04-12T10:00:61", Refusal::Second(61)),
            ("1985-04-12T10:00:00.", Refusal::Fraction),
        ] {
            assert_eq!(read(value), Err(refusal), "{value}");
        }
    }

    /// The calendars' own limits, from 3.2's tables and rules.
    #[test]
    fn the_calendars_are_applied() {
        assert_eq!(
            read("2026-13-01"),
            Err(Refusal::Month(13)),
            "a month the calendar does not have"
        );
        assert_eq!(read("2026-00-01"), Err(Refusal::Month(0)));
        assert_eq!(
            read("2023-02-29"),
            Err(Refusal::Day {
                day: 29,
                month: 2,
                length: 28
            }),
            "a common year"
        );
        assert!(read("2024-02-29").is_ok(), "a leap year");
        assert!(
            read("2000-02-29").is_ok(),
            "a centennial year divisible by four hundred"
        );
        assert_eq!(
            read("1900-02-29"),
            Err(Refusal::Day {
                day: 29,
                month: 2,
                length: 28
            }),
            "a centennial year not divisible by four hundred"
        );
        assert!(
            read("0000-02-29").is_ok(),
            "year 0 is a leap year by 3.2.1's rule"
        );
        assert_eq!(
            read("2023-04-31"),
            Err(Refusal::Day {
                day: 31,
                month: 4,
                length: 30
            })
        );
        assert_eq!(
            read("20230431"),
            Err(Refusal::Day {
                day: 31,
                month: 4,
                length: 30
            }),
            "basic format, the same calendar"
        );
        assert_eq!(
            read("2023-366"),
            Err(Refusal::Ordinal {
                day: 366,
                length: 365
            })
        );
        assert!(read("2024-366").is_ok());
        assert_eq!(
            read("2023-000"),
            Err(Refusal::Ordinal {
                day: 0,
                length: 365
            })
        );
        // Years of 53 weeks by 2.2.10: 2015 begins on a Thursday, 2020 is a leap year that
        // begins on a Wednesday; 2023 begins on a Sunday and has 52.
        assert!(read("2015-W53-1").is_ok());
        assert!(read("2020-W53").is_ok());
        assert_eq!(
            read("2023-W53"),
            Err(Refusal::Week {
                week: 53,
                weeks: 52
            })
        );
        assert_eq!(
            read("2023-W00-1"),
            Err(Refusal::Week { week: 0, weeks: 52 })
        );
        assert_eq!(read("2023-W01-8"), Err(Refusal::Weekday(8)));
        assert_eq!(read("2023-W01-0"), Err(Refusal::Weekday(0)));
        // The residue: 1600, 2000 and -0400 are one year to the calendar.
        assert!(read("-0400-02-29").is_ok());
        assert!(read("+0000001600-02-29").is_ok());
        assert_eq!(
            read("-0001-02-29"),
            Err(Refusal::Day {
                day: 29,
                month: 2,
                length: 28
            }),
            "the year before 0000 is not a leap year"
        );
    }

    /// 3.2.2's reference point and the week rule, checked on years whose 1 January follows
    /// from the reference point alone.
    #[test]
    fn the_week_rule_rests_on_the_reference_point() {
        // 1 January 2000 is a Saturday (3.2.2); a year that begins on a Saturday has 52 weeks.
        assert_eq!(Year::of("2000", false).weeks(), 52);
        // 2004 is a leap year beginning on a Thursday: 53.
        assert_eq!(Year::of("2004", false).weeks(), 53);
        // 2009 begins on a Thursday: 53. 2010 on a Friday: 52.
        assert_eq!(Year::of("2009", false).weeks(), 53);
        assert_eq!(Year::of("2010", false).weeks(), 52);
        // Year 0 and year 2000 are the same year to the calendar.
        assert_eq!(Year::of("0000", false).0, Year::of("2000", false).0);
        assert_eq!(Year::of("0001", true).0, 399);
    }

    /// Values that are no form of any clause, each refused with the thing it does named.
    #[test]
    fn what_is_neither_a_date_nor_a_date_and_time_is_refused_by_name() {
        for (value, refusal) in [
            ("", Refusal::Nothing),
            ("2016-02-01 13:19:21", Refusal::Space),
            ("2016-02-01T13:19:21/2016-02-02T13:19:21", Refusal::Interval),
            ("P1Y2M", Refusal::Duration),
            ("R2/P1Y6M", Refusal::Interval),
            ("R2", Refusal::Recurring),
            ("T13:19:21", Refusal::TimeOnly),
            (
                "13:19:21",
                Refusal::Unexpected {
                    found: ':',
                    after: "the year",
                },
            ),
            ("2016-2-1", Refusal::AfterHyphen(1)),
            ("20160", Refusal::Digits(5)),
            ("+1", Refusal::Expanded(1)),
            ("16-02-01", Refusal::Digits(2)),
            (
                "D:20221116191452+00'00",
                Refusal::Unexpected {
                    found: 'D',
                    after: "the year",
                },
            ),
            (
                "2016-02-01T13:19:21Z+01:00",
                Refusal::Unexpected {
                    found: '+',
                    after: "the UTC designator",
                },
            ),
            (
                "2016-02-01TIME13:19:21+01:00",
                Refusal::Missing("an hour of two digits after T"),
            ),
            (
                "2017-07-26T14:02:0300:00",
                Refusal::Unexpected {
                    found: '0',
                    after: "the second",
                },
            ),
            ("2016-02-01T13:19:21.Z", Refusal::Fraction),
            (
                "1985-04-12T10:15:30+01:",
                Refusal::Missing("a minute of two digits in the difference from UTC"),
            ),
            (
                "1985-04-12t10:15:30",
                Refusal::Unexpected {
                    found: 't',
                    after: "the day",
                },
            ),
            (
                "1985-04-12T10:15:30z",
                Refusal::Unexpected {
                    found: 'z',
                    after: "the second",
                },
            ),
            ("1985-04-12T10:15:30 ", Refusal::Space),
        ] {
            assert_eq!(read(value), Err(refusal), "{value}");
        }
    }

    /// Every refusal prints as a phrase that continues "…and the packet states ‹value›, …".
    #[test]
    fn every_refusal_reads_as_a_continuation() {
        for refusal in [
            Refusal::Nothing,
            Refusal::Space,
            Refusal::Interval,
            Refusal::Recurring,
            Refusal::Duration,
            Refusal::TimeOnly,
            Refusal::Digits(5),
            Refusal::Expanded(1),
            Refusal::AfterHyphen(4),
            Refusal::Month(13),
            Refusal::Day {
                day: 30,
                month: 2,
                length: 29,
            },
            Refusal::Ordinal {
                day: 366,
                length: 365,
            },
            Refusal::Week {
                week: 53,
                weeks: 52,
            },
            Refusal::Weekday(8),
            Refusal::EndOfDay,
            Refusal::Hour(25),
            Refusal::Minute(60),
            Refusal::Second(61),
            Refusal::Fraction,
            Refusal::Reduced,
            Refusal::Mixed,
            Refusal::Missing("a minute of two digits"),
            Refusal::Unexpected {
                found: 'x',
                after: "the day",
            },
        ] {
            let phrase = refusal.to_string();
            assert!(
                phrase.chars().next().is_some_and(char::is_lowercase),
                "{phrase:?} does not continue a sentence"
            );
            assert!(
                !phrase.ends_with('.'),
                "{phrase:?} closes the sentence itself"
            );
        }
    }
}
