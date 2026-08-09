//! ISO 32000-2 §7.9.4's date string type.
//!
//! A date is not a PDF object type of its own — §7.9.4's NOTE 1 says so: "[a] date string can be
//! any valid PDF string object … The description above relates to the text string value after
//! appropriate processing." So a date is a *text string* (§7.9.2.2, [`crate::text_string`]) whose
//! characters happen to spell one, which is why this module sits beside that one rather than
//! anywhere near a clock.
//!
//! # The grammar, which is entirely made of defaults
//!
//! §7.9.4:
//!
//! > The prefix ' D: ' shall be present, the year field (YYYY) shall be present and all other
//! > fields may be present but only if all of their preceding fields are also present. The
//! > APOSTROPHE following the hour offset field (HH) shall only be present if the HH field is
//! > present. The minute offset field (mm) shall only be present if the APOSTROPHE following the
//! > hour offset field (HH) is present. The default values for MM and DD shall be both 01; all
//! > other numerical fields shall default to zero values.
//!
//! Two things fall out of that and both are load-bearing. **A date may be four characters of
//! payload** — `D:1998` is a conforming date meaning 1998-01-01T00:00:00 — so a parser that
//! demands fourteen digits rejects valid files. And **the fields are a prefix**: there is no
//! shape in which a later field appears without an earlier one, so the parse is a walk that
//! stops when the string does.
//!
//! # The offset, and the one case the clause decides rather than leaves open
//!
//! §7.9.4 again:
//!
//! > A PLUS SIGN as the value of the O field signifies that local time is now and later than UT,
//! > a HYPHEN-MINUS signifies that local time is earlier than UT, and the LATIN CAPITAL LETTER Z
//! > signifies that local time is equal to UT. If no UT information is specified, the
//! > relationship of the specified time to UT shall be considered to be GMT. Regardless of
//! > whether the time zone is specified, the rest of the date shall be specified in local time.
//!
//! An absent offset is therefore **not** unknown: the clause says to treat it as GMT. [`Self::
//! offset`] still records that the file said nothing, because a viewer showing a date should be
//! able to say "as written" rather than inventing a zone the producer did not claim — but
//! [`Date::instant`], which is what an ordering is built on, takes the clause's answer.
//!
//! # Errata Collection 3 has struck out the last two sentences of that quotation
//!
//! **And nothing in this tree could see it until the four-hundred-and-sixteenth session**, which
//! is the point of the finding rather than a detail of it. The sponsored copy records EC3 as
//! *annotations* — a `StrikeOut` over the retired words, a `Caret` carrying the replacement — and
//! the Markdown conversion the conformance gate checks quotations against dropped every
//! annotation in all fourteen documents. So the body text above is the unamended 2020 text, the
//! gate verifies it happily, and the standard says something else. `tools/spec-errata` is what
//! reads the annotations back; ADR 0252 is the argument, and `doc/todo/48` carries the other 150
//! passages it found — 37 when this line was written, because the checker's comparison could not
//! see a passage the two extractions space differently (ADR 0253).
//!
//! Issue #251, on page 133, `/State` `Completed` — Table 174's "[t]he change has been completed":
//!
//! - "If no UT information is specified, the relationship of the specified time to UT shall be
//!   considered to be GMT" is struck, and the caret beside it reads "the missing timezone offset
//!   shall be assumed to be the same as Greenwich Mean Time's timezone offset (+0'00)". **Nothing
//!   here changes**: both sentences make an absent offset zero, which is what [`Date::instant`]
//!   does, and the replacement says it in the units [`Self::offset`] is stored in.
//! - "Regardless of whether the time zone is specified, the rest of the date shall be specified in
//!   local time." is struck **with no replacement**. That sentence is quoted below on [`Date`]
//!   itself, so what it was carrying has to be re-derived rather than assumed: the grammar does
//!   it. `O HH'mm` is an offset *from* the other fields, and an offset from a value already in UT
//!   would be an offset from nothing — so the fields are local time because the field that
//!   converts them exists, not because a sentence said so. The parse is unchanged either way,
//!   since it stores what the file wrote.
//!
//! Both quotations above are the annotations' own text, read out of the PDF rather than out of
//! `doc/md/`, which is why neither is a rustdoc blockquote: the gate could not check one.
//!
//! # Why an ordering exists at all
//!
//! §12.3.5.1's Table 156 collection sort may name a field whose `/Subtype` is `D`, and a portable
//! collection's whole purpose is to be shown as a sortable list ([`crate::Document`]'s callers
//! in `pdf-model`). Sorting those as *strings* is wrong in exactly the case the offset exists
//! for: `D:20240101120000+05'00` is earlier than `D:20240101090000-05'00` and sorts after it.
//! So [`Date`] orders by the instant it names, which is the one arithmetic in this module.
//!
//! # NOTE 2's older convention is accepted
//!
//! §7.9.4's NOTE 2:
//!
//! > PDF versions up to and including 1.7 defined a date string to include a terminating
//! > apostrophe. PDF processors are recommended to accept date strings that still follow that
//! > convention.
//!
//! A recommendation this reader takes, and the corpus says why: it is the commonest spelling in
//! the 974 documents. NOTE 3's `Z` followed by offsets is accepted for the same reason it is
//! written down — "the letter Z can optionally be followed by hour and minute offsets, which are
//! zero in this case" — and a `Z` followed by *non*-zero offsets is a file contradicting itself,
//! which is refused rather than guessed at.

/// ISO 32000-2 §7.9.4's date, parsed.
///
/// Every field is the file's own, in local time — which the 2020 text said outright and Errata
/// Collection 3 struck out (Issue #251; see the module documentation). What carries it now is the
/// grammar rather than a sentence: `O HH'mm` offsets the other fields, so those fields are the
/// local ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Date {
    /// `YYYY`, the one field the clause makes required.
    ///
    /// Signed because [`Self::instant`]'s arithmetic is, not because a PDF may state a negative
    /// year: the grammar is four digits.
    pub year: i32,
    /// `MM`, 1 to 12. **Default 1**, which the clause states rather than leaving to a reader.
    pub month: u8,
    /// `DD`, 1 to 31. Default 1.
    pub day: u8,
    /// `HH`, 0 to 23. Default 0.
    pub hour: u8,
    /// `mm`, 0 to 59. Default 0.
    pub minute: u8,
    /// `SS`, 0 to 59. Default 0.
    pub second: u8,
    /// The `O`, `HH` and `mm` offset fields, in minutes east of UT.
    ///
    /// `None` is *the file stated no zone*, which the clause makes mean GMT for the purpose of
    /// comparing two dates and which [`Self::instant`] therefore reads as zero. Kept apart from
    /// `Some(0)` because `Z` is a producer saying "UT" and an absent field is a producer saying
    /// nothing, and only one of those is a claim.
    pub offset: Option<i16>,
}

/// Days in each month of a non-leap year, indexed from January.
const MONTH_LENGTHS: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

impl Date {
    /// Parses §7.9.4's date string, or answers `None` for a string that is not one.
    ///
    /// Strict about the clause and tolerant of NOTE 2, which is not a contradiction: the note is
    /// part of the clause and recommends accepting the trailing apostrophe PDF 1.7 required.
    /// Everything else the clause states as a range — a month of 13, an hour of 24, an offset
    /// minute of 60 — makes the string not a date, and the caller keeps the bytes. Nothing here
    /// clamps: a clamp would turn a producer's mistake into a plausible wrong answer, which is
    /// what this project's own habits call the worse failure.
    ///
    /// Whether 1998-02-30 is a date is decided here too, and it is not: the clause's `DD (01-31)`
    /// is a range on the *field*, and a reader that accepted every value in it would order
    /// nonexistent days between real ones.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        // "The prefix 'D:' shall be present". Some producers omit it, and this does not accept
        // those: the clause makes it the one thing that distinguishes a date from any other
        // string, and a `/Title` of `20240101` is not a date.
        let rest = text.strip_prefix("D:")?;
        let bytes = rest.as_bytes();

        // The six numeric fields are one run of digits, and "all other fields may be present but
        // only if all of their preceding fields are also present" makes its length decisive: 4,
        // 6, 8, 10, 12 or 14 and nothing between. Splitting on the run rather than indexing at
        // fixed offsets is what lets the zone begin wherever the digits stop — the seconds field
        // of `D:199812231952-08'00` is absent, and reading two bytes at offset 12 would read the
        // sign.
        let numeric = bytes
            .iter()
            .position(|byte| !byte.is_ascii_digit())
            .unwrap_or(bytes.len());
        if !matches!(numeric, 4 | 6 | 8 | 10 | 12 | 14) {
            return None;
        }
        let field = |at: usize, width: usize| -> Option<u32> {
            let end = at.checked_add(width)?;
            let slice = bytes.get(at..end).filter(|_| end <= numeric)?;
            std::str::from_utf8(slice).ok()?.parse().ok()
        };

        let year = field(0, 4)?;
        let month = field(4, 2).unwrap_or(1);
        let day = field(6, 2).unwrap_or(1);
        let hour = field(8, 2).unwrap_or(0);
        let minute = field(10, 2).unwrap_or(0);
        let second = field(12, 2).unwrap_or(0);

        let month = u8::try_from(month)
            .ok()
            .filter(|value| (1..=12).contains(value))?;
        let day = u8::try_from(day).ok().filter(|value| *value >= 1)?;
        if day > days_in_month(i32::try_from(year).ok()?, month) {
            return None;
        }
        let hour = u8::try_from(hour).ok().filter(|value| *value <= 23)?;
        let minute = u8::try_from(minute).ok().filter(|value| *value <= 59)?;
        let second = u8::try_from(second).ok().filter(|value| *value <= 59)?;

        let offset = match zone(bytes.get(numeric..).unwrap_or_default())? {
            Zone::Absent => None,
            Zone::At(minutes) => Some(minutes),
        };
        Some(Self {
            year: i32::try_from(year).ok()?,
            month,
            day,
            hour,
            minute,
            second,
            offset,
        })
    }

    /// Minutes since 1970-01-01T00:00 UT, which is what two dates are compared by.
    ///
    /// The clause's own answer for a date that states no zone: "[i]f no UT information is
    /// specified, the relationship of the specified time to UT shall be considered to be GMT."
    #[must_use]
    pub fn instant(&self) -> i64 {
        let days = days_from_civil(self.year, self.month, self.day);
        let minutes = days
            .saturating_mul(24 * 60)
            .saturating_add(i64::from(self.hour).saturating_mul(60))
            .saturating_add(i64::from(self.minute));
        minutes.saturating_sub(i64::from(self.offset.unwrap_or(0)))
    }
}

impl PartialOrd for Date {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Date {
    /// By the instant each names, seconds included.
    ///
    /// The seconds are compared separately rather than folded into [`Date::instant`] so that the
    /// instant stays a whole number of minutes, which is the unit an offset is stated in.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.instant()
            .cmp(&other.instant())
            .then(self.second.cmp(&other.second))
    }
}

impl std::fmt::Display for Date {
    /// The date as the clause spells it, with every field written out.
    ///
    /// Always the full form, even where the file stated a prefix: a viewer showing `D:1998` as
    /// `1998-01-01T00:00:00` is showing the clause's own defaults, which are what the date
    /// *means*. The zone is written only where the file claimed one, for [`Date::offset`]'s
    /// reason.
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )?;
        match self.offset {
            None => Ok(()),
            Some(0) => out.write_str("Z"),
            Some(minutes) => {
                let sign = if minutes < 0 { '-' } else { '+' };
                let absolute = minutes.unsigned_abs();
                write!(out, "{sign}{:02}:{:02}", absolute / 60, absolute % 60)
            }
        }
    }
}

/// What the `O`, `HH`, `'` and `mm` fields said, which is three answers rather than two.
///
/// A stated zone and an absent one differ in what the *producer claimed* — see [`Date::offset`] —
/// and both differ from a string that is not an offset at all, which is this function's `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Zone {
    /// The string ended before the `O` field. The clause makes this GMT.
    Absent,
    /// A stated offset, in minutes east of UT.
    At(i16),
}

/// The `O`, `HH`, `'` and `mm` fields, which are one unit because each requires the last.
///
/// Every arithmetic below is on values the parse has already bounded — a sign in −1..=1, hours in
/// 0..=23, minutes in 0..=59 — so the products cannot leave `i16`.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "sign is -1..=1, hours 0..=23 and minutes 0..=59, all checked above, so \
              sign * (hours * 60 + minutes) is within -1439..=1439"
)]
fn zone(bytes: &[u8]) -> Option<Zone> {
    let Some((sign, rest)) = bytes.split_first() else {
        return Some(Zone::Absent);
    };
    let sign = match sign {
        b'+' => 1,
        b'-' => -1,
        // NOTE 3: "[t]he letter Z can optionally be followed by hour and minute offsets, which
        // are zero in this case." So a `Z` is parsed like any other sign and then checked to be
        // zero, rather than short-circuited — a `Z05'00` is a file contradicting itself and is
        // refused, where a `Z00'00` is the note's own spelling.
        b'Z' => 0,
        _ => return None,
    };

    // Two digits at `at`, or `Ok(None)` where the string ends first and `Err` where what is
    // there is not two digits — three outcomes, because a field that is absent is permitted and
    // a field that is malformed is not.
    let two = |at: usize| -> Result<Option<i16>, ()> {
        let end = at.checked_add(2).ok_or(())?;
        match rest.get(at..end) {
            None => Ok(None),
            Some(slice) => std::str::from_utf8(slice)
                .ok()
                .filter(|text| text.bytes().all(|byte| byte.is_ascii_digit()))
                .and_then(|text| text.parse::<i16>().ok())
                .map(Some)
                .ok_or(()),
        }
    };

    let Some(hours) = two(0).ok()? else {
        // NOTE 2's older convention, and the bare `Z`: nothing after the sign at all.
        return (rest.is_empty() || rest == b"'").then_some(Zone::At(0));
    };
    if hours > 23 {
        return None;
    }
    // "The APOSTROPHE following the hour offset field (HH) shall only be present if the HH field
    // is present. The minute offset field (mm) shall only be present if the APOSTROPHE … is
    // present."
    let minutes = match rest.get(2) {
        None => 0,
        Some(b'\'') => {
            let Some(minutes) = two(3).ok()? else {
                // NOTE 2 again: PDF 1.7's terminating apostrophe, with no minutes after it.
                return rest.get(3).is_none().then_some(Zone::At(sign * hours * 60));
            };
            if minutes > 59 {
                return None;
            }
            // A trailing apostrophe after the minutes is the same older convention.
            match rest.get(5) {
                None => {}
                Some(b'\'') if rest.get(6).is_none() => {}
                Some(_) => return None,
            }
            minutes
        }
        Some(_) => return None,
    };
    let magnitude = hours * 60 + minutes;
    // NOTE 3's condition, checked rather than assumed: a `Z` says local time "is equal to UT",
    // so a `Z` followed by anything but zeros is a file saying two different things.
    if sign == 0 && magnitude != 0 {
        return None;
    }
    Some(Zone::At(sign * magnitude))
}

/// How many days month `month` of year `year` has, leap years included.
fn days_in_month(year: i32, month: u8) -> u8 {
    if month == 2 && is_leap(year) {
        return 29;
    }
    MONTH_LENGTHS
        .get(usize::from(month).saturating_sub(1))
        .copied()
        .unwrap_or(0)
}

/// The proleptic Gregorian leap rule, which is what §7.9.4's `DD (01-31)` is checked against.
fn is_leap(year: i32) -> bool {
    year.rem_euclid(4) == 0 && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0)
}

/// Days from 1970-01-01 to `year-month-day`, proleptic Gregorian.
///
/// Howard Hinnant's `days_from_civil`, which is exact for every year an `i32` holds and needs no
/// table and no loop. March is treated as the first month of the year so that the leap day falls
/// at the *end*, which is what removes every special case from the arithmetic.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the formula is stated as one expression on purpose; every operand is derived from \
              an i32 year widened to i64 and a validated month and day, and the largest \
              intermediate — era * 146_097 — is under 2^53 for every four-digit year a PDF can \
              state, so no operation can overflow i64"
)]
fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let year = i64::from(year);
    let month = i64::from(month);
    let day = i64::from(day);
    let year = if month <= 2 { year - 1 } else { year };
    // The era is a 400-year cycle, which is the period of the Gregorian leap rule.
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::Date;

    /// §7.9.4's own worked example: "December 23, 1998, at 7:52 PM, U.S. Pacific Standard Time,
    /// is represented by the string: D:199812231952-08'00".
    #[test]
    fn the_clauses_own_example_parses_to_what_it_says_it_means() {
        let date = Date::parse("D:199812231952-08'00").expect("the clause's example");
        assert_eq!(
            (date.year, date.month, date.day),
            (1998, 12, 23),
            "December 23, 1998"
        );
        assert_eq!(
            (date.hour, date.minute, date.second),
            (19, 52, 0),
            "7:52 PM"
        );
        assert_eq!(date.offset, Some(-8 * 60), "U.S. Pacific Standard Time");
        assert_eq!(date.to_string(), "1998-12-23T19:52:00-08:00");
    }

    /// "[T]he year field (YYYY) shall be present and all other fields may be present but only if
    /// all of their preceding fields are also present. The default values for MM and DD shall be
    /// both 01; all other numerical fields shall default to zero values."
    #[test]
    fn every_field_but_the_year_may_be_absent_and_the_clause_states_its_default() {
        let date = Date::parse("D:1998").expect("a year alone is a date");
        assert_eq!(
            (date.year, date.month, date.day),
            (1998, 1, 1),
            "MM and DD default to 01"
        );
        assert_eq!((date.hour, date.minute, date.second), (0, 0, 0));
        assert_eq!(date.offset, None, "no zone stated is not a zone of zero");

        for (text, expected) in [
            ("D:199812", "1998-12-01T00:00:00"),
            ("D:19981223", "1998-12-23T00:00:00"),
            ("D:1998122319", "1998-12-23T19:00:00"),
            ("D:199812231952", "1998-12-23T19:52:00"),
            ("D:19981223195231", "1998-12-23T19:52:31"),
        ] {
            assert_eq!(
                Date::parse(text).map(|date| date.to_string()).as_deref(),
                Some(expected),
                "{text}"
            );
        }
    }

    /// NOTE 2's older convention — "PDF versions up to and including 1.7 defined a date string to
    /// include a terminating apostrophe" — and NOTE 3's `Z` with zero offsets after it.
    #[test]
    fn the_notes_two_older_spellings_are_accepted() {
        for text in [
            "D:19981223195200-08'00'",
            "D:19981223195200-08'",
            "D:19981223195200Z",
            "D:19981223195200Z00'00",
            "D:19981223195200Z'",
        ] {
            assert!(Date::parse(text).is_some(), "{text}");
        }
        assert_eq!(
            Date::parse("D:19981223195200Z00'00").and_then(|date| date.offset),
            Some(0)
        );
        assert_eq!(
            Date::parse("D:19981223195200Z05'00"),
            None,
            "NOTE 3 makes a Z's offsets zero; anything else is a file contradicting itself"
        );
    }

    /// Every range the clause states, and the one it does not state and a calendar does.
    #[test]
    fn a_field_outside_the_clauses_range_is_not_a_date() {
        for text in [
            "19981223195200",         // no `D:` prefix
            "D:199a",                 // not digits
            "D:19981323",             // MM (01-12)
            "D:19981200",             // DD (01-31), and 00 is outside it
            "D:19981223245200",       // HH (00-23)
            "D:19981223196000",       // mm (00-59)
            "D:19981223195260",       // SS (00-59)
            "D:19981223195200-24'00", // offset HH (00-23)
            "D:19981223195200-08'60", // offset mm (00-59)
            "D:19981223195200X08'00", // O is one of +, - or Z
            "D:19980230",             // February has no thirtieth
        ] {
            assert_eq!(Date::parse(text), None, "{text}");
        }
        assert!(
            Date::parse("D:20000229").is_some(),
            "2000 is a leap year by the four-hundred rule"
        );
        assert_eq!(
            Date::parse("D:19000229"),
            None,
            "1900 is not, by the hundred rule"
        );
    }

    /// The reason an ordering exists: two dates whose *strings* sort one way and whose instants
    /// sort the other, which is exactly what §12.3.5.1's Table 156 collection sort would get wrong.
    #[test]
    fn dates_order_by_the_instant_rather_than_by_the_string() {
        let east = Date::parse("D:20240101120000+05'00").expect("a date");
        let west = Date::parse("D:20240101090000-05'00").expect("a date");
        assert!(
            "D:20240101090000-05'00" < "D:20240101120000+05'00",
            "as strings"
        );
        assert!(east < west, "07:00 UT is before 14:00 UT");

        // The clause's own answer for an absent zone, which is GMT and not "unknown".
        let bare = Date::parse("D:20240101070000").expect("a date");
        assert_eq!(bare.instant(), east.instant());
    }

    /// The epoch itself, and a day either side of it, since every comparison rests on this.
    #[test]
    fn the_instant_is_minutes_from_the_epoch() {
        assert_eq!(Date::parse("D:19700101").expect("a date").instant(), 0);
        assert_eq!(
            Date::parse("D:19700102").expect("a date").instant(),
            24 * 60
        );
        assert_eq!(
            Date::parse("D:19691231").expect("a date").instant(),
            -24 * 60
        );
        assert_eq!(
            Date::parse("D:20000301").expect("a date").instant(),
            11017 * 24 * 60,
            "the day after a leap day in a four-hundred-rule leap year"
        );
    }
}
