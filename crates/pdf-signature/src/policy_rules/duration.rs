//! The durations a signature policy states, and the instants they move a time to.
//!
//! ETSI TS 119 172-2 clause 4.23.3 types its `Freshness` members `xs:duration`, and TS 119 172-3
//! clause 4.23.3.2 writes the same values as a `PrintableString` "expressing the duration as
//! defined for the duration type" of the XML Schema, so both syntaxes hand this module one lexical
//! form: XML Schema Part 2 section 3.2.6.1's `PnYnMnDTnHnMnS`, with an optional leading minus sign.
//! ETSI TR 102 272 clause 6.8's `DeltaTime` is four integers of seconds, minutes, hours and days,
//! which is a whole number of seconds and needs nothing here.
//!
//! # Adding a duration to an instant
//!
//! A duration's value is six-dimensional (section 3.2.6), so "one month" is not a number of
//! seconds, and section 3.2.6 sends every comparison to Appendix E, which defines `S + D` for a
//! date-time `S` and a duration `D`: the year and month are added first, the day is pinned into
//! the month that results (April 31 becomes April 30), and only then are the day, hour, minute and
//! second added, with carries into the month and year and sixty seconds to every minute. Since
//! every instant here is UTC, the second step is the same as adding that many seconds to the
//! instant, which is what [`Duration::added_to`] does after the first.
//!
//! # Fractional seconds
//!
//! Section 3.2.6.1 lets the seconds carry decimal digits to any precision. Every instant this
//! module meets is a whole second — RFC 5280 section 4.1.2.5.2 forbids a fractional second in a
//! certificate's or a list's `GeneralizedTime`, and [`Instant`] holds whole seconds — and both
//! comparisons a policy's duration enters are of a whole-second issuance against a bound, so the
//! signed seconds component is rounded up to the next whole second, which decides both exactly:
//! *issued after V − D* and *issued at or after B + D* hold for a whole-second issuance under the
//! rounded duration exactly where they hold under the stated one.

use crate::x509::{self, Instant};

/// XML Schema Part 2 section 3.2.6's duration, its seconds rounded up to a whole second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duration {
    /// The signed number of months, years counted as twelve.
    months: i64,
    /// The signed number of seconds in the day, hour, minute and second components.
    seconds: i64,
}

impl Duration {
    /// A duration of whole seconds, as TR 102 272 clause 6.8's `DeltaTime` states one.
    #[must_use]
    pub const fn of_seconds(seconds: i64) -> Self {
        Self { months: 0, seconds }
    }

    /// Section 3.2.6.1's lexical form, or `None` where `text` is not one.
    ///
    /// Every component is optional but at least one is present, a `T` is followed by at least one
    /// time component, the components come in their fixed order, and a decimal point in the
    /// seconds is followed by at least one digit; a value too large for this arithmetic is not a
    /// duration it can add.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (negative, rest) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let rest = rest.strip_prefix('P')?;
        let (date, time) = match rest.split_once('T') {
            Some((date, time)) => {
                if time.is_empty() {
                    return None;
                }
                (date, Some(time))
            }
            None => (rest, None),
        };
        let mut months: i64 = 0;
        let mut seconds: i64 = 0;
        let mut stated = false;
        let mut date = date;
        for (designator, unit_months) in [('Y', 12_i64), ('M', 1)] {
            if let Some((digits, after)) = component(date, designator) {
                months = months.checked_add(whole(digits)?.checked_mul(unit_months)?)?;
                date = after;
                stated = true;
            }
        }
        if let Some((digits, after)) = component(date, 'D') {
            seconds = whole(digits)?.checked_mul(86_400)?;
            date = after;
            stated = true;
        }
        if !date.is_empty() {
            return None;
        }
        if let Some(mut time) = time {
            let mut timed = false;
            for (designator, unit) in [('H', 3_600_i64), ('M', 60)] {
                if let Some((digits, after)) = component(time, designator) {
                    seconds = seconds.checked_add(whole(digits)?.checked_mul(unit)?)?;
                    time = after;
                    timed = true;
                }
            }
            if let Some((digits, after)) = component(time, 'S') {
                let (whole_part, fraction) = match digits.split_once('.') {
                    Some((whole_part, fraction)) => {
                        if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
                            return None;
                        }
                        (whole_part, fraction.bytes().any(|b| b != b'0'))
                    }
                    None => (digits, false),
                };
                let mut stated_seconds = whole(whole_part)?;
                // Rounded up as the signed value, which for a positive duration is away from
                // nought and for a negative one towards it (the module comment's argument).
                if fraction && !negative {
                    stated_seconds = stated_seconds.checked_add(1)?;
                }
                seconds = seconds.checked_add(stated_seconds)?;
                time = after;
                timed = true;
            }
            if !time.is_empty() || !timed {
                return None;
            }
            stated = true;
        }
        if !stated {
            return None;
        }
        if negative {
            months = months.checked_neg()?;
            seconds = seconds.checked_neg()?;
        }
        Some(Self { months, seconds })
    }

    /// `at + self` by Appendix E, or `None` past this arithmetic's range.
    #[must_use]
    pub fn added_to(self, at: Instant) -> Option<Instant> {
        shifted(at, self.months, self.seconds)
    }

    /// `at - self`: Appendix E's sum with every component of the duration negated.
    #[must_use]
    pub fn subtracted_from(self, at: Instant) -> Option<Instant> {
        shifted(at, self.months.checked_neg()?, self.seconds.checked_neg()?)
    }
}

/// The digits before `designator` at the front of `text`, and what follows it; `None` where the
/// front of `text` is not digits followed by that letter.
fn component(text: &str, designator: char) -> Option<(&str, &str)> {
    let end = text.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
    let (digits, rest) = text.split_at(end);
    let rest = rest.strip_prefix(designator)?;
    (!digits.is_empty()).then_some((digits, rest))
}

/// An unsigned integer of section 3.2.6.1's pattern `[0-9]+`.
fn whole(digits: &str) -> Option<i64> {
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Appendix E's sum: the months onto the civil date, the day pinned into the month that results,
/// then the seconds.
fn shifted(at: Instant, months: i64, seconds: i64) -> Option<Instant> {
    let (days, second_of_day) = (
        at.unix_seconds().div_euclid(86_400),
        at.unix_seconds().rem_euclid(86_400),
    );
    let (year, month, day) = civil_from_days(days)?;
    // Months are counted from nought so that the carry into the year is a Euclidean division,
    // which is Appendix E's `fQuotient(temp, 1, 13)` and `modulo(temp, 1, 13)`.
    let total = year
        .checked_mul(12)?
        .checked_add(month.checked_sub(1)?)?
        .checked_add(months)?;
    let (year, month) = (total.div_euclid(12), total.rem_euclid(12).checked_add(1)?);
    let day = day.min(days_in_month(year, month));
    let days = x509::days_from_civil(year, month, day)?;
    let start = days.checked_mul(86_400)?.checked_add(second_of_day)?;
    Some(Instant::from_unix_seconds(start.checked_add(seconds)?))
}

/// Appendix E's `maximumDayInMonthFor`.
const fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year.rem_euclid(400) == 0 || (year.rem_euclid(100) != 0 && year.rem_euclid(4) == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

/// The proleptic Gregorian date `days` after 1970-01-01: Howard Hinnant's `civil_from_days`, the
/// inverse of [`x509::days_from_civil`] and shifted to a March year for the same reason.
fn civil_from_days(days: i64) -> Option<(i64, i64, i64)> {
    let shifted = days.checked_add(719_468)?;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era = day_of_era
        .checked_sub(day_of_era.checked_div(1_460)?)?
        .checked_add(day_of_era.checked_div(36_524)?)?
        .checked_sub(day_of_era.checked_div(146_096)?)?
        .checked_div(365)?;
    let day_of_year = day_of_era.checked_sub(
        year_of_era
            .checked_mul(365)?
            .checked_add(year_of_era.checked_div(4)?)?
            .checked_sub(year_of_era.checked_div(100)?)?,
    )?;
    let month_shift = day_of_year
        .checked_mul(5)?
        .checked_add(2)?
        .checked_div(153)?;
    let day = day_of_year
        .checked_sub(
            month_shift
                .checked_mul(153)?
                .checked_add(2)?
                .checked_div(5)?,
        )?
        .checked_add(1)?;
    let month = if month_shift < 10 {
        month_shift.checked_add(3)?
    } else {
        month_shift.checked_sub(9)?
    };
    let year = year_of_era
        .checked_add(era.checked_mul(400)?)?
        .checked_add(i64::from(month <= 2))?;
    Some((year, month, day))
}

/// An instant as `YYYY-MM-DDTHH:MM:SSZ`, for the sentences that name a bound.
#[must_use]
pub fn utc(at: Instant) -> String {
    let seconds = at.unix_seconds();
    let second_of_day = seconds.rem_euclid(86_400);
    match civil_from_days(seconds.div_euclid(86_400)) {
        Some((year, month, day)) => format!(
            "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
            second_of_day / 3_600,
            second_of_day % 3_600 / 60,
            second_of_day % 60
        ),
        None => format!("{seconds} s after 1970-01-01T00:00:00Z"),
    }
}

#[cfg(test)]
mod tests;
