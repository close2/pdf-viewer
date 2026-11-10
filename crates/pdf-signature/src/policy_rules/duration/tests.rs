//! XML Schema Part 2 section 3.2.6.1's lexical form and Appendix E's sum, each expected value read
//! off that text: its own worked examples where it prints one, and the rule written out where it
//! does not.

use super::{Duration, utc};
use crate::x509::{self, Instant};

/// The instant at `year-month-day hour:minute:second` UTC, read as a `GeneralizedTime` is.
fn at(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Instant {
    let spelled = format!("{year:04}{month:02}{day:02}{hour:02}{minute:02}{second:02}Z");
    x509::read_time_digits(spelled.as_bytes(), true).expect("a time")
}

/// Appendix E.1's first example, in whole seconds: 2000-01-12T12:13:14Z plus `P1Y3M5DT7H10M3S`
/// is 2001-04-17T19:23:17Z (the printed example adds 3.3 s and ends at 17.3 s).
#[test]
fn the_appendixs_own_example_is_reached() {
    let duration = Duration::parse("P1Y3M5DT7H10M3S");
    let sum = duration.and_then(|d| d.added_to(at(2000, 1, 12, 12, 13, 14)));
    assert_eq!(sum, Some(at(2001, 4, 17, 19, 23, 17)));
    assert_eq!(sum.map(utc).as_deref(), Some("2001-04-17T19:23:17Z"));
}

/// Appendix E.1: the year and month are added first and the day is pinned into the month that
/// results, "April 31 turns into April 30"; E.2's example adds a day after a month and lands on
/// 2000-05-01. A month is therefore not a fixed number of seconds, which the plant of reading
/// `P1M` as thirty days would fail on March 31.
#[test]
fn a_month_pins_the_day_into_the_month_that_results() {
    let month = Duration::parse("P1M");
    let day = Duration::parse("P1D");
    assert_eq!(
        month.and_then(|m| m.added_to(at(2000, 3, 31, 0, 0, 0))),
        Some(at(2000, 4, 30, 0, 0, 0))
    );
    assert_eq!(
        month
            .and_then(|m| m.added_to(at(2000, 3, 30, 0, 0, 0)))
            .and_then(|e| day.and_then(|d| d.added_to(e))),
        Some(at(2000, 5, 1, 0, 0, 0))
    );
    // Subtracting is Appendix E's sum with the components negated: March 31 minus a month pins
    // into February, which in 2000 has 29 days (`maximumDayInMonthFor`'s 400-year rule), and
    // January minus a month carries into the year before.
    assert_eq!(
        month.and_then(|m| m.subtracted_from(at(2000, 3, 31, 6, 0, 0))),
        Some(at(2000, 2, 29, 6, 0, 0))
    );
    assert_eq!(
        Duration::parse("P1Y")
            .and_then(|y| y.subtracted_from(at(2001, 3, 31, 0, 0, 0)))
            .map(utc)
            .as_deref(),
        Some("2000-03-31T00:00:00Z")
    );
    assert_eq!(
        month.and_then(|m| m.subtracted_from(at(2027, 1, 15, 0, 0, 0))),
        Some(at(2026, 12, 15, 0, 0, 0))
    );
}

/// Section 3.2.6.1's form: `P` first, at least one component, a `T` only before a time
/// component, the components in their order, a digit after any decimal point, and an optional
/// leading minus sign.
#[test]
fn only_the_lexical_form_is_a_duration() {
    for refused in [
        "", "P", "PT", "P1DT", "1D", "P1S", "PT1D", "P1D1Y", "PT1.S", "P-1D", "P1.5D", "PT1H1H",
        "P1Y ", "+P1D",
    ] {
        assert_eq!(Duration::parse(refused), None, "{refused:?}");
    }
    assert_eq!(
        Duration::parse("PT36H"),
        Some(Duration::of_seconds(36 * 3_600))
    );
    assert_eq!(Duration::parse("P0D"), Some(Duration::of_seconds(0)));
    assert_eq!(Duration::parse("-PT60S"), Some(Duration::of_seconds(-60)));
    assert_eq!(Duration::parse("PT1M"), Duration::parse("PT60S"));
}

/// The module comment's rounding: a fractional second rounds the signed seconds up, so a
/// whole-second issuance compares against the rounded bound exactly as against the stated one.
/// `PT0.5S` after 12:00:00 admits 12:00:01 and not 12:00:00, which is what *at or after
/// 12:00:00.5* says of whole seconds; `-PT0.5S` keeps 12:00:00 as the bound, which is what *at or
/// after 11:59:59.5* says.
#[test]
fn a_fractional_second_rounds_up() {
    let noon = at(2026, 11, 5, 12, 0, 0);
    assert_eq!(
        Duration::parse("PT0.5S").and_then(|d| d.added_to(noon)),
        Some(at(2026, 11, 5, 12, 0, 1))
    );
    assert_eq!(
        Duration::parse("PT0.000S").and_then(|d| d.added_to(noon)),
        Some(noon)
    );
    assert_eq!(
        Duration::parse("-PT0.5S").and_then(|d| d.added_to(noon)),
        Some(noon)
    );
}
