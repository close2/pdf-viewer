//! `FormCalc`'s values and the promotions between them: XFA 3.3 chapter 25, *Simple Expressions*
//! (pages 1061–1062), cited and paraphrased.
//!
//! The language is typeless (page 1049): a value is a number, a string or null, and an operator
//! promotes its operands to what it needs.

/// One `FormCalc` value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// The null value of production 12, and a container that holds no value.
    Null,
    /// An IEEE 754 64-bit number (page 1056).
    Number(f64),
    /// A string.
    String(String),
}

impl Value {
    /// The value promoted to a number (page 1061): a number is itself, null is 0, and a string is
    /// the number it can be fully converted to, else 0.
    ///
    /// Full conversion is read as the whole string, white space of production 3 around it
    /// aside, spelling an optionally signed production 11 number literal other than `nan` and
    /// `inf` — the literal grammar is the only spelling of a number the chapter defines, and page
    /// 1057 keeps the two special values out of every result.
    #[must_use]
    pub fn number(&self) -> f64 {
        match self {
            Self::Null => 0.0,
            Self::Number(n) => *n,
            Self::String(text) => numeric(text).unwrap_or(0.0),
        }
    }

    /// The value promoted to a boolean (page 1061): true where it can be fully converted to a
    /// nonzero number, so null and a string that spells no number are false.
    #[must_use]
    pub fn boolean(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Number(n) => *n != 0.0,
            Self::String(text) => numeric(text).is_some_and(|n| n != 0.0),
        }
    }

    /// The value promoted to a string (page 1061): null is the empty string, a number is written
    /// with at most eleven fractional digits, the precision page 1062 displays a result at, and no
    /// trailing zeros.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Number(n) => format_number(*n),
            Self::String(text) => text.clone(),
        }
    }

    /// Whether the value is null.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

/// A number written as page 1062 displays a result.
#[must_use]
pub fn format_number(n: f64) -> String {
    let text = format!("{n:.11}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        &text
    };
    // `-0.00000000000001` rounds to a negative zero, which is no number a person writes.
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

/// The count of ASCII digits in `bytes` from `at` on.
fn digits_from(bytes: &[u8], at: usize) -> usize {
    bytes.get(at..).map_or(0, |rest| {
        rest.iter().take_while(|b| b.is_ascii_digit()).count()
    })
}

/// The number `text` fully converts to, if it does. The offsets are `saturating_add`s of counts
/// bounded by the slice's length, so the sums cannot wrap.
fn numeric(text: &str) -> Option<f64> {
    let text = text.trim_matches(|c| matches!(c, '\t' | '\u{b}' | '\u{c}' | ' ' | '\n' | '\r'));
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    let bytes = unsigned.as_bytes();
    let integral = digits_from(bytes, 0);
    let mut at = integral;
    let mut fraction = 0;
    if bytes.get(at) == Some(&b'.') {
        at = at.saturating_add(1);
        fraction = digits_from(bytes, at);
        at = at.saturating_add(fraction);
    }
    if integral.saturating_add(fraction) == 0 {
        return None;
    }
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        at = at.saturating_add(1);
        if matches!(bytes.get(at), Some(b'+' | b'-')) {
            at = at.saturating_add(1);
        }
        let exponent = digits_from(bytes, at);
        if exponent == 0 {
            return None;
        }
        at = at.saturating_add(exponent);
    }
    if at != bytes.len() {
        return None;
    }
    text.parse::<f64>().ok().filter(|n| n.is_finite())
}
