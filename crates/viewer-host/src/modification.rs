//! The clock `viewer_core` does not have, for ISO 32000-2 Table 166's `/M`.
//!
//! Table 166's entry is "[t]he date and time when the annotation was most recently modified", and
//! `CLAUDE.md`'s rule 3 gives the renderer no clock: a date invented there would be a claim about
//! a machine nothing in that crate can see. This module is the party with the clock, exactly as
//! [`crate::policy`] is the party with the filesystem, and [`now`] is what a host hands to
//! `viewer_core::Command::Clock`.
//!
//! **The zone is UT and that is a choice, recorded as one.** §7.9.4's grammar offers a local time
//! with an offset, and the standard library offers no local offset at all — so a host here can
//! either state UT, which it knows, or guess an offset, which would be a wrong claim in a file
//! rather than a missing one. `Z00'00'` is what §7.9.4's own grammar writes for UT, and a reader
//! comparing two dates gets the right answer from it (`pdf_syntax::Date::instant`). ADR 1160.

use std::time::{SystemTime, UNIX_EPOCH};

use viewer_core::Command;

/// The command a host runs *before* one a person gave, where Table 166's `/M` depends on it.
///
/// One statement for all three windows, which is why it is here rather than in each of their
/// pump loops: a host that reads its clock somewhere else reads it at a different moment, and a
/// window whose saves carry no `/M` at all looks exactly like one whose clock could not be read.
///
/// `Some` for `viewer_core::Command::Save` and nothing else. §7.5.6's update is the only thing
/// that writes the entry, and [`now`] is read immediately before it because that is the instant
/// the entry is about — a clock kept from launch would state when the window opened. ADR 1160.
#[must_use]
pub fn before(command: &Command) -> Option<Command> {
    matches!(command, Command::Save).then(|| Command::Clock(now()))
}

/// What this machine's clock says, as ISO 32000-2 §7.9.4's date in UT.
///
/// `None` where the clock reads before 1970 or so far after it that the arithmetic below would
/// not hold — a host with no usable clock is a host that says nothing, which is the answer
/// `viewer_core` already has a meaning for.
#[must_use]
pub fn now() -> Option<pdf_syntax::Date> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    at_unix_seconds(i64::try_from(seconds.as_secs()).ok()?)
}

/// The same date from a stated instant, which is what makes [`now`] testable.
///
/// Seconds since 1970-01-01T00:00:00 UT, the unit `SystemTime` counts in.
#[must_use]
pub fn at_unix_seconds(seconds: i64) -> Option<pdf_syntax::Date> {
    let days = seconds.div_euclid(86_400);
    let within = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days)?;
    Some(pdf_syntax::Date {
        year,
        month,
        day,
        hour: u8::try_from(within.div_euclid(3600)).ok()?,
        minute: u8::try_from(within.div_euclid(60).rem_euclid(60)).ok()?,
        second: u8::try_from(within.rem_euclid(60)).ok()?,
        // `Some(0)` rather than `None`: §7.9.4 makes an absent zone mean GMT for comparison, and
        // `pdf_syntax::Date` keeps the two apart because one is a claim and the other is silence.
        // This is a claim — the clock was read in UT.
        offset: Some(0),
    })
}

/// The year, month and day `days` after 1970-01-01, proleptic Gregorian.
///
/// Howard Hinnant's `civil_from_days`, the inverse of the `days_from_civil` `pdf_syntax::Date`
/// already uses to order two dates. March is treated as the first month of the year so that the
/// leap day falls at the *end*, which is what removes every special case from the arithmetic.
///
/// `None` for a day whose year is not one §7.9.4's four-digit grammar can spell, which no clock
/// on this machine reaches.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the formula is stated as one expression on purpose; `days` is a whole number of \
              days derived from a `u64` of seconds, so the largest intermediate — era * 146_097 \
              — is under 2^53 and no operation can overflow i64"
)]
fn civil_from_days(days: i64) -> Option<(i32, u8, u8)> {
    let shifted = days + 719_468;
    // The era is a 400-year cycle, which is the period of the Gregorian leap rule.
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_of_year = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_of_year + 2) / 5 + 1;
    let month = month_of_year + if month_of_year < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    Some((
        i32::try_from(year).ok().filter(|year| *year <= 9999)?,
        u8::try_from(month).ok()?,
        u8::try_from(day).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{at_unix_seconds, before, now};
    use viewer_core::Command;

    /// A save is the one command this host reads its clock for, and it reads it beforehand.
    ///
    /// The claim is both halves: Table 166's `/M` is written by §7.5.6's update and by nothing
    /// else, so a clock read before any other command would be a host stating a time no entry is
    /// about. Exhaustive it cannot be — `Command` is a large enumeration — so what is asserted is
    /// the one that answers and a sample of the ones that must not, including the clock command
    /// itself, whose answering here would be a loop (ADR 1160).
    #[test]
    fn a_save_is_preceded_by_this_machines_clock_and_nothing_else_is() {
        let Some(Command::Clock(Some(stated))) = before(&Command::Save) else {
            panic!("a save states the time it happened");
        };
        let read_again = now().expect("this machine's clock reads as a date");
        assert!(
            read_again.instant().saturating_sub(stated.instant()) <= 1,
            "the clock is read at the save rather than kept from somewhere earlier: \
             {stated:?} against {read_again:?}"
        );
        assert_eq!(
            stated.offset,
            Some(0),
            "§7.9.4's UT, which this host states"
        );
        for other in [
            Command::Undo,
            Command::Redo,
            Command::Copy,
            Command::Clock(None),
            Command::Close(viewer_core::DocumentId(0)),
        ] {
            assert!(before(&other).is_none(), "{other:?} writes no /M");
        }
    }

    /// The inverse is the function `pdf_syntax` already has, so the pair is what is asserted.
    ///
    /// Not a table of dates somebody typed: `pdf_syntax::Date::instant` computes minutes from a
    /// date by Hinnant's forward algorithm, so a date this module builds from an instant and
    /// hands back must name that same instant. Four instants across two eras, a leap day and the
    /// epoch itself.
    #[test]
    fn a_date_built_from_an_instant_names_that_instant_again() {
        for seconds in [0_i64, 951_827_696, 1_760_000_000, 4_102_444_800] {
            let date = at_unix_seconds(seconds).expect("a clock reading inside the grammar");
            assert_eq!(
                date.instant(),
                seconds.div_euclid(60),
                "{seconds} came back as {date:?}"
            );
            assert_eq!(
                i64::from(date.second),
                seconds.rem_euclid(60),
                "{seconds} came back as {date:?}"
            );
        }
    }

    /// §7.9.4's own worked example, read the other way round.
    ///
    /// The clause spells "December 23, 1998, at 7:52 PM, U.S. Pacific Standard Time" as
    /// `D:199812231952-08'00`, which is 1998-12-24T03:52 UT — so the instant that date names,
    /// handed to this module, is that day in UT rather than the producer's local one.
    #[test]
    fn the_clauses_own_example_comes_back_in_ut() {
        let stated = pdf_syntax::Date::parse("D:199812231952-08'00").expect("§7.9.4's example");
        let here = at_unix_seconds(stated.instant() * 60).expect("a date inside the grammar");
        assert_eq!(
            (here.year, here.month, here.day, here.hour, here.minute),
            (1998, 12, 24, 3, 52),
            "{here:?}"
        );
        assert_eq!(here.offset, Some(0), "the clock was read in UT");
    }

    /// The clock on this machine is a date this grammar can spell.
    #[test]
    fn this_machines_clock_reads_as_a_date() {
        let read = now().expect("a clock after 1970 and before the year 10000");
        assert!(read.year >= 2024 && read.year <= 9999, "{read:?}");
        assert!((1..=12).contains(&read.month), "{read:?}");
        assert!((1..=31).contains(&read.day), "{read:?}");
    }
}
