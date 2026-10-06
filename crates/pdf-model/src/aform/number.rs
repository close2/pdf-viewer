//! The numeric half of the library: reading a number out of a field, writing one into it.
//!
//! Every rule here is a documented choice (ADR 1578): Adobe's *JavaScript for Acrobat API
//! Reference* does not describe these functions at all, and its *Interapplication Communication
//! API Reference* gives, under the Acrobat Forms plug-in's `SetJavaScriptAction`, an argument menu
//! and no algorithm. Where the menu is silent the function's name, `util.printf`'s documented
//! separator table (the same reference's "util methods" page, `nDecSep`), and the arithmetic a
//! person entering decimal digits means decide; where none of those does, the ADR names the
//! choice.

/// The separator and decimal-point styles, `util.printf`'s `nDecSep` table plus the one Acrobat's
/// own number dialog offers beyond it.
///
/// Adobe's "util methods" page lists four: comma-separated with a period decimal point, no
/// separator with a period, period-separated with a comma, no separator with a comma. The
/// Interapplication reference documents only the first two for `AFNumber_Format`'s `sepStyle`, and
/// a fifth — apostrophe-separated with a period, `1'234.56` — is what Acrobat's dialog writes as
/// style 4 and is documented nowhere; it is taken as the de-facto convention it is (ADR 1578
/// section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Separators {
    /// The thousands separator, or `None`.
    pub(crate) group: Option<char>,
    /// The decimal point.
    pub(crate) point: char,
}

impl Separators {
    /// The style a `sepStyle` argument names, or `None` for a number outside 0–4.
    pub(crate) fn of(style: f64) -> Option<Self> {
        let style = whole(style)?;
        Some(match style {
            0 => Self {
                group: Some(','),
                point: '.',
            },
            1 => Self {
                group: None,
                point: '.',
            },
            2 => Self {
                group: Some('.'),
                point: ',',
            },
            3 => Self {
                group: None,
                point: ',',
            },
            4 => Self {
                group: Some('\''),
                point: '.',
            },
            _ => return None,
        })
    }
}

/// A non-negative whole number argument, or `None` for a fraction, a negative, or a huge one.
pub(crate) fn whole(value: f64) -> Option<u32> {
    if !(0.0..=4_294_967_295.0).contains(&value) || value.fract() != 0.0 {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "checked just above: a whole number in u32's range"
    )]
    Some(value as u32)
}

/// `AFMakeNumber`: the number a field's text states, or `None` where it states none.
///
/// **The choice** (ADR 1578 section 1): the text, with surrounding white space removed, must be
/// *wholly* a decimal number — an optional sign, digits with at most one decimal point, an optional
/// exponent — and the decimal point may be written `.` or `,`, because the comma styles store what
/// a person typed in them. Nothing else is read: not a currency symbol, not a thousands separator,
/// not a trailing word. A field holding `$1,234.50` therefore states no number, which is the
/// honest answer — reading one out of it would mean guessing which of its two punctuation marks is
/// the decimal point.
#[must_use]
pub fn make_number(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let mut normalised = String::with_capacity(text.len());
    let mut points = 0_usize;
    let mut digits = 0_usize;
    let mut chars = text.chars().peekable();
    if let Some(&sign) = chars.peek()
        && (sign == '+' || sign == '-')
    {
        normalised.push(sign);
        chars.next();
    }
    let mut exponent = false;
    while let Some(c) = chars.next() {
        match c {
            '0'..='9' => {
                digits = digits.saturating_add(1);
                normalised.push(c);
            }
            '.' | ',' if !exponent => {
                points = points.saturating_add(1);
                normalised.push('.');
            }
            'e' | 'E' if digits > 0 && !exponent => {
                exponent = true;
                normalised.push('e');
                if let Some(&sign) = chars.peek()
                    && (sign == '+' || sign == '-')
                {
                    normalised.push(sign);
                    chars.next();
                }
                if !chars.peek().is_some_and(char::is_ascii_digit) {
                    return None;
                }
            }
            _ => return None,
        }
    }
    if digits == 0 || points > 1 {
        return None;
    }
    let value: f64 = normalised.parse().ok()?;
    value.is_finite().then_some(value)
}

/// `AFExtractNums`: every run of decimal digits in the text, in order, or `None` where there is
/// none.
///
/// **The choice** (ADR 1578 section 1): a run is maximal digits; anything else separates runs. A
/// text that *begins* with `.` or `,` has a zero in front of its first run, so `.5` is the two
/// runs `0` and `5` — the reading of a leading decimal point as a fraction of nothing.
#[must_use]
pub fn extract_nums(text: &str) -> Option<Vec<String>> {
    let mut runs = Vec::new();
    if text.starts_with(['.', ',']) {
        runs.push("0".to_owned());
    }
    let mut current = String::new();
    for c in text.chars() {
        if c.is_ascii_digit() {
            current.push(c);
        } else if !current.is_empty() {
            runs.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        runs.push(current);
    }
    (!runs.is_empty()).then_some(runs)
}

/// A number written the way ECMAScript's `Number.prototype.toString` writes it (ECMA-262 section
/// 6.1.6.1.20, the abstract operation for a Number's string), which is how a calculated value
/// becomes a field's text.
///
/// The shortest digits that read back as the same double, then the standard's placement rule:
/// plain notation for exponents from −6 to 20, exponent notation outside them.
#[must_use]
pub fn number_text(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let (negative, digits, point) = decimal(value);
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    let k = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    let n = i64::from(point);
    if k <= n && n <= 21 {
        out.push_str(&digits);
        for _ in k..n {
            out.push('0');
        }
    } else if 0 < n && n <= 21 {
        let split = usize::try_from(n).unwrap_or(0);
        out.push_str(digits.get(..split).unwrap_or_default());
        out.push('.');
        out.push_str(digits.get(split..).unwrap_or_default());
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        for _ in n..0 {
            out.push('0');
        }
        out.push_str(&digits);
    } else {
        let exponent = n.saturating_sub(1);
        out.push_str(digits.get(..1).unwrap_or_default());
        if digits.len() > 1 {
            out.push('.');
            out.push_str(digits.get(1..).unwrap_or_default());
        }
        out.push('e');
        out.push(if exponent < 0 { '-' } else { '+' });
        out.push_str(&exponent.unsigned_abs().to_string());
    }
    out
}

/// The shortest decimal digits of a finite non-zero double, and where the decimal point sits.
///
/// `(negative, digits, point)` with `digits` free of leading and trailing zeros and the value equal
/// to `0.digits × 10^point`. Read off the standard library's shortest round-trip exponent
/// notation, so that no digit is invented here.
fn decimal(value: f64) -> (bool, String, i32) {
    let text = format!("{:e}", value.abs());
    let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0').to_owned();
    let digits = if digits.is_empty() {
        "0".to_owned()
    } else {
        digits
    };
    (value < 0.0, digits, exponent.saturating_add(1))
}

/// A number rounded to `places` decimal places **in decimal**, as `(negative, integer, fraction)`.
///
/// **The choice** (ADR 1578 section 1): rounding is half away from zero on the number's shortest
/// decimal digits, never on its binary value. A person who typed `1.005` into a two-place field
/// typed a number ending in five, and rounding the double nearest to it would show `1.00` for a
/// reason nobody can see. The sign is decided after rounding, so a value that rounds to zero is
/// written without one.
fn rounded(value: f64, places: u32) -> (bool, String, String) {
    if value == 0.0 || !value.is_finite() {
        return (false, "0".to_owned(), "0".repeat(places_len(places)));
    }
    let (negative, digits, point) = decimal(value);
    // The digit string padded so that it covers the integer part and `places` decimals plus one.
    let places_i = i64::from(places);
    let point_i = i64::from(point);
    let mut all: Vec<u8> = digits.bytes().map(|b| b.saturating_sub(b'0')).collect();
    let mut integer_len = point_i;
    if point_i <= 0 {
        // Leading zeros between the point and the first significant digit.
        let pad = usize::try_from(point_i.unsigned_abs()).unwrap_or(0);
        let mut padded = vec![0_u8; pad.saturating_add(1)];
        padded.append(&mut all);
        all = padded;
        integer_len = 1;
    }
    let wanted = usize::try_from(integer_len.saturating_add(places_i)).unwrap_or(0);
    while all.len() <= wanted {
        all.push(0);
    }
    let round_up = all.get(wanted).is_some_and(|d| *d >= 5);
    all.truncate(wanted);
    if round_up {
        let mut at = all.len();
        let mut carry = true;
        while carry && at > 0 {
            at = at.saturating_sub(1);
            if let Some(digit) = all.get_mut(at) {
                if *digit == 9 {
                    *digit = 0;
                } else {
                    *digit = digit.saturating_add(1);
                    carry = false;
                }
            }
        }
        if carry {
            all.insert(0, 1);
            integer_len = integer_len.saturating_add(1);
        }
    }
    let split = usize::try_from(integer_len).unwrap_or(0).min(all.len());
    let to_text = |part: &[u8]| -> String {
        part.iter()
            .map(|d| char::from(b'0'.saturating_add(*d)))
            .collect()
    };
    let integer = to_text(all.get(..split).unwrap_or_default());
    let integer = integer.trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer }.to_owned();
    let fraction = to_text(all.get(split..).unwrap_or_default());
    let zero = integer == "0" && fraction.bytes().all(|b| b == b'0');
    (negative && !zero, integer, fraction)
}

/// `places` as a length, bounded by what [`MAX_PLACES`] allows a caller to ask for.
fn places_len(places: u32) -> usize {
    usize::try_from(places.min(MAX_PLACES)).unwrap_or(0)
}

/// Most decimal places a format may ask for.
///
/// A double carries seventeen significant digits; past a few dozen places every further digit is a
/// zero this program would be allocating for a hostile argument. The bound refuses rather than
/// truncates, so a document asking for more is told so (ADR 1578 section 2).
pub(crate) const MAX_PLACES: u32 = 64;

/// A number written with a separator style and a number of decimal places, without its sign.
///
/// `util.printf`'s `%,Nd.Pf` conversion as the "util methods" page describes it — the digits of the
/// integer part grouped in threes from the decimal point, the fraction padded or rounded to the
/// places asked for — with [`rounded`]'s decimal rounding.
pub(crate) fn digits_with(value: f64, places: u32, style: Separators) -> (bool, String) {
    let (negative, integer, fraction) = rounded(value, places);
    let mut out = String::new();
    let count = integer.chars().count();
    for (index, c) in integer.chars().enumerate() {
        if index > 0
            && let Some(group) = style.group
            && count.saturating_sub(index) % 3 == 0
        {
            out.push(group);
        }
        out.push(c);
    }
    if places > 0 {
        out.push(style.point);
        out.push_str(&fraction);
    }
    (negative, out)
}

/// How `AFNumber_Format` shows a negative number: the Interapplication reference's `negStyle`
/// menu, `MinusBlack`, `Red`, `ParensBlack`, `ParensRed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Negative {
    /// 0: a minus sign, in the field's own colour.
    MinusBlack,
    /// 1: in red, and with no sign — the menu names a colour and nothing else, so the colour is
    /// the whole of what marks the number negative (ADR 1578 section 2).
    Red,
    /// 2: in parentheses, in the field's own colour.
    ParensBlack,
    /// 3: in parentheses and red.
    ParensRed,
}

impl Negative {
    /// The style a `negStyle` argument names, or `None` outside 0–3.
    pub(crate) fn of(style: f64) -> Option<Self> {
        Some(match whole(style)? {
            0 => Self::MinusBlack,
            1 => Self::Red,
            2 => Self::ParensBlack,
            3 => Self::ParensRed,
            _ => return None,
        })
    }
}

/// `AFNumber_Format`'s text for one number, and whether it is to be drawn red.
pub(crate) fn number_format(
    value: f64,
    places: u32,
    style: Separators,
    negative: Negative,
    currency: &str,
    prepend: bool,
) -> (String, bool) {
    let (is_negative, digits) = digits_with(value, places, style);
    let mut body = String::new();
    if prepend {
        body.push_str(currency);
    }
    body.push_str(&digits);
    if !prepend {
        body.push_str(currency);
    }
    if !is_negative {
        return (body, false);
    }
    match negative {
        Negative::MinusBlack => (format!("-{body}"), false),
        Negative::Red => (body, true),
        Negative::ParensBlack => (format!("({body})"), false),
        Negative::ParensRed => (format!("({body})"), true),
    }
}

/// `AFNumber_Keystroke`'s judgement of a field's text, for one decimal point.
///
/// **The choice** (ADR 1578 section 3): while a person is typing (`will_commit` false) the text
/// must be a *prefix* of a number — an optional sign, digits, at most one decimal point, which may
/// be the last thing typed — so that every number can be typed one character at a time and nothing
/// that is not a number can. At commit it must be a whole number of that shape, or empty. The
/// decimal point is the style's own — a period style takes `.` and never `,`, its grouping mark —
/// and a comma style takes `,` **or** `.`, one of them once: its committed value is stored with a
/// period (ADR 1578 section 3), so a field read back for editing holds one, and refusing it would
/// refuse every keystroke into a value the field itself wrote.
pub(crate) fn number_keystroke(text: &str, point: char, will_commit: bool) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return true;
    }
    let body = text.strip_prefix(['+', '-']).unwrap_or(text);
    let mut digits = 0_usize;
    let mut points = 0_usize;
    for c in body.chars() {
        if c.is_ascii_digit() {
            digits = digits.saturating_add(1);
        } else if c == point || (point == ',' && c == '.') {
            points = points.saturating_add(1);
        } else {
            return false;
        }
    }
    if points > 1 {
        return false;
    }
    !will_commit || digits > 0
}

/// `AFRange_Validate`'s answer: `None` where the value is inside its bounds, or the sentence for
/// the bound it is outside.
///
/// Both bounds are inclusive — the argument names say *greater than* and *less than* and the
/// Interapplication reference gives this function no description at all, so the reading that never
/// refuses the bound itself is the one taken; a form asking for `0` to `100` means a person may
/// enter `100` (ADR 1578 section 4).
pub(crate) fn range_refusal(value: f64, low: Option<f64>, high: Option<f64>) -> Option<String> {
    let under = low.is_some_and(|low| value < low);
    let over = high.is_some_and(|high| value > high);
    if !under && !over {
        return None;
    }
    Some(match (low, high) {
        (Some(low), Some(high)) => format!(
            "the value must be greater than or equal to {} and less than or equal to {}",
            number_text(low),
            number_text(high)
        ),
        (Some(low), None) => format!(
            "the value must be greater than or equal to {}",
            number_text(low)
        ),
        (None, Some(high)) => format!(
            "the value must be less than or equal to {}",
            number_text(high)
        ),
        (None, None) => return None,
    })
}

/// The arithmetic `AFSimple_Calculate` names: the Interapplication reference's `AVG`, `SUM`, `PRD`,
/// `MIN` and `MAX`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arithmetic {
    /// The mean of every value.
    Average,
    /// Their sum.
    Sum,
    /// Their product.
    Product,
    /// The least.
    Minimum,
    /// The greatest.
    Maximum,
}

impl Arithmetic {
    /// The function a `cFunction` argument names, exactly as the menu spells it.
    pub(crate) fn of(name: &str) -> Option<Self> {
        Some(match name {
            "AVG" => Self::Average,
            "SUM" => Self::Sum,
            "PRD" => Self::Product,
            "MIN" => Self::Minimum,
            "MAX" => Self::Maximum,
            _ => return None,
        })
    }

    /// The function over the values, as the text the calculated field takes.
    ///
    /// **The choices** (ADR 1578 section 5): a value that states no number counts as zero, which
    /// is what a blank line in a column of figures is to the person adding it up; no values at all
    /// is zero; the mean divides by how many values were counted, blanks included. The result is
    /// rounded to fifteen significant digits before it is written, because a double carries fifteen
    /// faithfully and the sixteenth is the binary representation showing through — `0.1 + 0.2` is a
    /// sum of two decimals a person typed, and the field should say `0.3`.
    pub(crate) fn over(self, values: &[f64]) -> String {
        if values.is_empty() {
            return "0".to_owned();
        }
        let result = match self {
            Self::Sum => values.iter().sum(),
            Self::Average => {
                let sum: f64 = values.iter().sum();
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a count of form fields, far below 2^52"
                )]
                let count = values.len() as f64;
                sum / count
            }
            Self::Product => values.iter().product(),
            Self::Minimum => values.iter().copied().fold(f64::INFINITY, f64::min),
            Self::Maximum => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        };
        number_text(significant(result))
    }
}

/// A double rounded to fifteen significant decimal digits.
fn significant(value: f64) -> f64 {
    if !value.is_finite() || value == 0.0 {
        return value;
    }
    format!("{value:.14e}").parse().unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::{Separators, digits_with, make_number, number_text, rounded};

    #[test]
    fn number_text_follows_the_placement_rule() {
        assert_eq!(number_text(3.0), "3");
        assert_eq!(number_text(-2.5), "-2.5");
        assert_eq!(number_text(0.000_001), "0.000001");
        assert_eq!(number_text(0.000_000_1), "1e-7");
        assert_eq!(number_text(1e21), "1e+21");
        assert_eq!(
            number_text(123_456_789_012_345_680_000.0),
            "123456789012345680000"
        );
    }

    #[test]
    fn rounding_is_decimal_and_half_away_from_zero() {
        assert_eq!(rounded(1.005, 2), (false, "1".to_owned(), "01".to_owned()));
        assert_eq!(rounded(-0.004, 2), (false, "0".to_owned(), "00".to_owned()));
        assert_eq!(rounded(9.995, 2), (false, "10".to_owned(), "00".to_owned()));
        assert_eq!(rounded(0.05, 1), (false, "0".to_owned(), "1".to_owned()));
    }

    #[test]
    fn grouping_counts_from_the_decimal_point() {
        let comma = Separators::of(0.0).expect("style 0");
        assert_eq!(digits_with(1_234_567.891, 2, comma).1, "1,234,567.89");
        assert_eq!(digits_with(123.0, 0, comma).1, "123");
    }

    #[test]
    fn a_number_is_the_whole_text() {
        assert_eq!(make_number(" 12.5 "), Some(12.5));
        assert_eq!(make_number("12,5"), Some(12.5));
        assert_eq!(make_number("$12"), None);
        assert_eq!(make_number("1,234.5"), None);
        assert_eq!(make_number(""), None);
    }
}
