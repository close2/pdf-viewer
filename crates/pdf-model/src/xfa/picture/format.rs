//! XFA 3.3's picture clauses, written for output formatting: a canonical value written out by a
//! picture in the prevailing locale, the inverse of [`super`]'s input parsing.
//!
//! Chapter 26 (pages 1150–1186) defines each symbol for both directions at once, and this module
//! writes what each symbol's output half says: a literal verbatim and `?`, `*` and `+` as a space
//! (page 1152); the date symbols of page 1167, the time symbols of page 1173, the numeric symbols
//! of pages 1177–1180 with the `%` of page 1181, the text symbols of page 1183, and the null and
//! zero categories of pages 1184–1185. Chapter 25's `Format` (page 1125) says what each category
//! formats: a date clause a date or the date of a date-time, a time clause a time or the time of
//! a date-time, a numeric clause a number, a text clause text. XFA 3.3 is cited by chapter and
//! printed page and paraphrased (RFC 0011's preamble).
//!
//! **A value an alternative cannot write is `None`, and the next alternative is tried** (page
//! 1159, EXAMPLE 26.12). Chapter 26 states no rounding and no truncation for a value too large
//! for its picture, so where writing it would drop a digit, a sign or a part the value states,
//! or would have to invent a part it does not state, the alternative does not write it — the
//! choice ADR 2434 records, because a format test passed that way would pass a value its picture
//! cannot show, and a `bind` picture's data read back by the same picture would be another value.
//! A time's fields below the picture's precision are dropped, as EXAMPLE 26.22 drops `h:MM A`'s
//! seconds; a time with a zone is not written, since no zone symbol is read here (page 1176) and
//! the clock without its zone would be another instant.

use std::fmt::Write as _;

use super::{Category, Field, Piece, Token};
use crate::xfa::canonical::{self, Date, Decimal, Time};

/// The output of an alternative's `pieces` for the canonical `value`, `None` where one of its
/// clauses cannot write it.
pub(super) fn pieces(pieces: &[Piece], value: &str) -> Option<String> {
    let mut out = String::new();
    for piece in pieces {
        match piece {
            Piece::Literal(text) => out.push_str(text),
            Piece::Clause(category, tokens) => out.push_str(&clause(*category, tokens, value)?),
        }
    }
    Some(out)
}

/// One clause's output for `value`.
fn clause(category: Category, tokens: &[Token], value: &str) -> Option<String> {
    // Page 1125: a date or a time clause formats the date or the time of a date-time as well.
    let (date_part, time_part) = match value.split_once('T') {
        Some((date, time)) if canonical::is_date_time(value) => (date, time),
        _ => (value, value),
    };
    match category {
        Category::Date => date(tokens, canonical::date(date_part)?),
        Category::Time => match canonical::time(time_part)? {
            (clock, None) => time(tokens, clock),
            (_, Some(_)) => None,
        },
        Category::Num => number(tokens, &canonical::decimal(value)?),
        Category::Text => text(tokens, value),
        // Page 1184: a null clause writes a null value, which this program's data holds as the
        // empty string; page 1185 a zero clause a zero. Both may use the numeric symbols.
        Category::Null if value.is_empty() => number(
            tokens,
            &Decimal {
                negative: false,
                whole: String::new(),
                fraction: None,
            },
        ),
        Category::Zero => {
            let zero = canonical::decimal(value)?;
            (zero.whole.is_empty()
                && zero
                    .fraction
                    .as_deref()
                    .is_none_or(|f| f.bytes().all(|b| b == b'0')))
            .then(|| number(tokens, &zero))
            .flatten()
        }
        Category::Null => None,
    }
}

/// Writes a token every category writes alike, and says whether it was one.
fn common(token: &Token, out: &mut String) -> bool {
    match token {
        Token::Literal(text) => out.push_str(text),
        Token::Any | Token::Spaces(_) => out.push(' '),
        _ => return false,
    }
    true
}

/// A date or time number: a symbol of fixed width zero-padded to it, one of variable width
/// written in as few digits as the value needs (page 1167's `D` against `DD`).
fn digits(out: &mut String, value: u16, min: usize, max: usize) {
    if min == max {
        let _ = write!(out, "{value:0max$}");
    } else {
        out.push_str(&value.to_string());
    }
}

/// A name from the locale's list, or `None` where the list does not hold the value's.
fn name(out: &mut String, names: &[String], value: u16, base: u16) -> Option<()> {
    let at = usize::from(value.checked_sub(base)?);
    out.push_str(names.get(at)?);
    Some(())
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "a year is at most 9999, a day of the year at most 366, and a weekday at most 7"
)]
fn date_field(field: Field, date: Date) -> Option<u16> {
    let day = || Some((date.month?, date.day?));
    Some(match field {
        Field::Day => u16::from(date.day?),
        Field::Month => u16::from(date.month?),
        Field::DayOfYear => {
            let (month, day) = day()?;
            (1..month)
                .map(|m| u16::from(canonical::days_in_month(date.year, m)))
                .sum::<u16>()
                + u16::from(day)
        }
        Field::Weekday => {
            let (month, day) = day()?;
            u16::from(canonical::weekday(date.year, month, day))
        }
        // `e` counts from Monday (page 1168).
        Field::IsoWeekday => {
            let (month, day) = day()?;
            u16::from((canonical::weekday(date.year, month, day) + 5) % 7 + 1)
        }
        Field::Year2 => date.year % 100,
        Field::Year4 => date.year,
        // Chapter 24 writes no year before the common era, which is the locale's second era
        // name (the parser's `Era` 1).
        Field::Era => 1,
        _ => return None,
    })
}

fn date(tokens: &[Token], value: Date) -> Option<String> {
    let mut out = String::new();
    for token in tokens {
        if common(token, &mut out) {
            continue;
        }
        match token {
            Token::Digits(field, min, max, _) => {
                digits(&mut out, date_field(*field, value)?, *min, *max);
            }
            Token::Name(field, names, base) => {
                name(&mut out, names, date_field(*field, value)?, *base)?;
            }
            _ => return None,
        }
    }
    Some(out)
}

fn time_field(field: Field, value: Time) -> Option<u16> {
    let hour = u16::from(value.hour);
    Some(match field {
        Field::Hour12 if hour % 12 == 0 => 12,
        Field::Hour12 | Field::Hour11 => hour % 12,
        Field::Hour24 if hour == 0 => 24,
        Field::Hour23 | Field::Hour24 => hour,
        Field::Minute => u16::from(value.minute?),
        Field::Second => u16::from(value.second?),
        Field::Millisecond => value.millisecond?,
        Field::Meridiem => u16::from(hour >= 12),
        _ => return None,
    })
}

fn time(tokens: &[Token], value: Time) -> Option<String> {
    let mut out = String::new();
    for token in tokens {
        if common(token, &mut out) {
            continue;
        }
        match token {
            Token::Digits(field, min, max, _) => {
                digits(&mut out, time_field(*field, value)?, *min, *max);
            }
            Token::Name(field, names, base) => {
                name(&mut out, names, time_field(*field, value)?, *base)?;
            }
            _ => return None,
        }
    }
    Some(out)
}

fn text(tokens: &[Token], value: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = value.chars();
    for token in tokens {
        if common(token, &mut out) {
            continue;
        }
        match token {
            // Page 1183: each symbol writes one character of the value, which it must accept.
            Token::Character(accepts) => out.push(chars.next().filter(|&c| accepts(c))?),
            _ => return None,
        }
    }
    chars.next().is_none().then_some(out)
}

/// A number multiplied by a hundred, page 1181's output for a `%` picture: the radix moved two
/// digits right.
fn hundredfold(value: &Decimal) -> Decimal {
    let fraction = value.fraction.as_deref().unwrap_or("");
    let moved: String = fraction.chars().chain("00".chars()).take(2).collect();
    let rest = fraction.get(2..).filter(|rest| !rest.is_empty());
    Decimal {
        negative: value.negative,
        whole: format!("{}{moved}", value.whole)
            .trim_start_matches('0')
            .to_owned(),
        fraction: rest.map(str::to_owned),
    }
}

/// A numeric clause's output for `value` (pages 1177–1181).
///
/// The integer digits are set right-aligned into the integer symbols; a position the value
/// leaves empty is a leading zero, which `9` writes as `0`, `z` as nothing and `Z` as a space
/// (page 1179), and a grouping separator before the first digit written follows the padding
/// beside it. The radix is written where a `9` or `Z` follows it, where an `8` follows it and the
/// value has fraction digits (page 1180), and where only `z` follows it and the value has a
/// radix (page 1179); `v` never writes it (page 1178). A fraction digit is written where the
/// value has one; past them `9` writes `0`, and `z` and `Z` write nothing (EXAMPLE 26.25); an `8`
/// writes every fraction digit left (page 1180).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "digit positions are counts of the picture's symbols, bounded by its MAX_PICTURE bytes"
)]
fn number(tokens: &[Token], value: &Decimal) -> Option<String> {
    let percent = tokens.iter().any(|t| matches!(t, Token::Symbol(_, true)));
    let value = if percent {
        hundredfold(value)
    } else {
        value.clone()
    };
    let radix_at = tokens
        .iter()
        .position(|t| matches!(t, Token::Radix(..)))
        .unwrap_or(tokens.len());
    let is_digit = |t: &&Token| matches!(t, Token::NumDigit { .. });
    let integer_places = tokens[..radix_at].iter().filter(is_digit).count();
    let fraction_tokens = || tokens[radix_at..].iter();
    let expands = fraction_tokens().any(|t| matches!(t, Token::NumDigit { expands: true, .. }));
    let fraction_places = fraction_tokens().filter(is_digit).count();
    let whole: Vec<char> = value.whole.chars().collect();
    let fraction: Vec<char> = value.fraction.as_deref().unwrap_or("").chars().collect();
    if whole.len() > integer_places || (!expands && fraction.len() > fraction_places) {
        return None;
    }
    let signed = tokens
        .iter()
        .any(|t| matches!(t, Token::Sign(..) | Token::Negative(..) | Token::Bracket(_)));
    if value.negative && !signed {
        return None;
    }
    let padding = integer_places - whole.len();
    let fixed_after = fraction_tokens().any(|t| {
        matches!(
            t,
            Token::NumDigit { required: true, .. } | Token::NumDigit { space: true, .. }
        )
    });
    let mut out = String::new();
    let (mut place, mut started, mut padded_with_space) = (0, false, false);
    let mut fraction_at = 0;
    for (at, token) in tokens.iter().enumerate() {
        if common(token, &mut out) || marks(token, value.negative, &mut out) {
            continue;
        }
        match token {
            // An `8` never stands here: `super::tokens` refuses one left of the radix for
            // formatting, page 1181 leaving its meaning undefined.
            Token::NumDigit {
                required, space, ..
            } if at < radix_at => {
                if place < padding {
                    match (required, space) {
                        (true, _) => {
                            out.push('0');
                            started = true;
                        }
                        (false, true) => {
                            out.push(' ');
                            padded_with_space = true;
                        }
                        (false, false) => {}
                    }
                } else {
                    out.push(*whole.get(place - padding)?);
                    started = true;
                }
                place += 1;
            }
            Token::NumDigit {
                required, expands, ..
            } => {
                if *expands {
                    out.extend(fraction.get(fraction_at..).unwrap_or(&[]));
                    fraction_at = fraction.len();
                } else if let Some(&digit) = fraction.get(fraction_at) {
                    out.push(digit);
                    fraction_at += 1;
                } else if *required {
                    out.push('0');
                }
            }
            Token::Grouping(symbol) => {
                if started {
                    out.push_str(symbol);
                } else if padded_with_space {
                    out.push(' ');
                }
            }
            Token::Radix(symbol, _, implied) => {
                let written = if fixed_after {
                    true
                } else if expands {
                    !fraction.is_empty()
                } else {
                    value.fraction.is_some()
                };
                if written && !*implied {
                    out.push_str(symbol);
                }
            }
            _ => return None,
        }
    }
    Some(out)
}

/// Writes a numeric clause's sign, negative mark, bracket or locale symbol (page 1178), and says
/// whether `token` was one.
fn marks(token: &Token, negative: bool, out: &mut String) -> bool {
    match token {
        Token::Sign(minus, space) => {
            if negative {
                out.push_str(minus);
            } else if *space {
                out.push(' ');
            }
        }
        Token::Negative(text, spaces) => {
            if negative {
                out.push_str(text);
            } else if *spaces {
                out.push_str("  ");
            }
        }
        Token::Bracket(open) => out.push(match (negative, open) {
            (true, true) => '(',
            (true, false) => ')',
            (false, _) => ' ',
        }),
        Token::Symbol(symbol, _) => out.push_str(symbol),
        _ => return false,
    }
    true
}
