//! `util.printf`: a format string's conversions written with the library's own number writers.
//!
//! Adobe's *JavaScript for Acrobat API Reference*, "util methods", describes each conversion as
//! `%[,nDecSep][cFlags][nWidth][.nPrecision]cConvChar` — a separator style from the same table
//! `AFNumber_Format`'s `sepStyle` reads ([`Separators`]), the flags `+`, space, `0` and `#`, a
//! minimum width padded on the left, a number of decimal places, and one of `d`, `f`, `s` and `x`
//! — and gives one example, which `tests` holds. The digits are [`digits_with`]'s, so a script's
//! `util.printf("%,0.2f", n)` and a one-call `AFNumber_Format(2, 0, …)` write the same characters.
//! What the page leaves unsaid is a documented choice each (ADR 1626):
//!
//! - **`%f` without a precision writes six places**, C's default: the page names C's function of
//!   the same name as this one's model.
//! - **`%s` of a number writes at most fifteen significant digits**: the page's own example writes
//!   one hundred π as `314.159265358979`, which is ECMAScript's string of the number rounded to
//!   fifteen, and a string argument is written as it is.
//! - **`%x` of a negative number writes its 32-bit two's complement**, C's `unsigned int`, since
//!   the page calls the conversion unsigned and says nothing of the sign; the digits are upper
//!   case, as the example's `13A` is.
//! - **`%%` writes one `%`**, C's spelling, which the page does not mention.
//! - **A conversion with no argument left, or an argument with no conversion, is refused**: the
//!   page requires the two counts to be equal.

use super::Refusal;
use super::number::{MAX_PLACES, Separators, digits_with, number_text};

/// One argument after the format string, as a script passed it.
#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    /// A number.
    Number(f64),
    /// Anything else, as the script's own conversion to a string wrote it.
    Text(String),
}

/// The widest field a conversion may ask for, in characters.
///
/// The width is the document's number, and every character of it is allocated; a field a
/// thousand characters wide is wider than any field a form has, and the bound refuses rather than
/// truncates so that the document is told.
pub const MAX_WIDTH: usize = 1024;

/// The significant digits `%s` writes a number with: the page's example's fifteen.
const STRING_DIGITS: usize = 15;

/// `util.printf(cFormat, …)`: the format string with each conversion replaced by its argument.
///
/// # Errors
///
/// [`Refusal`] for a conversion this grammar does not have, a separator style outside the page's
/// four, a width past [`MAX_WIDTH`] or a precision past the library's own bound, and a count of
/// arguments different from the count of conversions.
pub fn printf(format: &str, arguments: &[Argument]) -> Result<String, Refusal> {
    let mut out = String::new();
    let mut remaining = arguments.iter();
    let mut characters = format.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '%' {
            out.push(character);
            continue;
        }
        if characters.peek() == Some(&'%') {
            characters.next();
            out.push('%');
            continue;
        }
        let conversion = Conversion::read(&mut characters)?;
        let Some(argument) = remaining.next() else {
            return Err(Refusal::new(format!(
                "its format asks for more conversions than the {} argument(s) it was given, and \
                 the two counts are to be equal",
                arguments.len()
            )));
        };
        out.push_str(&conversion.write(argument));
    }
    if remaining.next().is_some() {
        return Err(Refusal::new(format!(
            "it was given {} argument(s), more than its format has conversions, and the two \
             counts are to be equal",
            arguments.len()
        )));
    }
    Ok(out)
}

/// One `%` conversion, read.
#[derive(Debug, Clone, Copy)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the page's four flags are four independent switches, and a field each reads as the \
              page lists them"
)]
struct Conversion {
    /// `nDecSep`, where one was written; a number without one is written with no separator and a
    /// period, style 1.
    separators: Option<Separators>,
    /// `+`: a sign always.
    plus: bool,
    /// Space: a space where there is no sign.
    space: bool,
    /// `0`: padded with zeros rather than spaces.
    zeros: bool,
    /// `#`: `%f` always writes its decimal point.
    alternate: bool,
    /// `nWidth`.
    width: usize,
    /// `nPrecision`.
    precision: Option<u32>,
    /// `cConvChar`.
    kind: Kind,
}

/// The four conversion characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `d`: an integer, truncated.
    Integer,
    /// `f`: a number with decimal places.
    Float,
    /// `s`: a string.
    String,
    /// `x`: an integer, truncated, in unsigned hexadecimal.
    Hex,
}

impl Conversion {
    /// Reads the conversion after a `%`.
    fn read(characters: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<Self, Refusal> {
        let mut conversion = Self {
            separators: None,
            plus: false,
            space: false,
            zeros: false,
            alternate: false,
            width: 0,
            precision: None,
            kind: Kind::String,
        };
        if characters.peek() == Some(&',') {
            characters.next();
            let style = characters
                .next()
                .and_then(|digit| digit.to_digit(10))
                .filter(|style| *style <= 3)
                .ok_or_else(|| {
                    Refusal::new(
                        "a comma in its format is not followed by one of the separator styles \
                         0 to 3",
                    )
                })?;
            conversion.separators = Separators::of(f64::from(style));
        }
        while let Some(flag) = characters.peek().copied() {
            match flag {
                '+' => conversion.plus = true,
                ' ' => conversion.space = true,
                '0' => conversion.zeros = true,
                '#' => conversion.alternate = true,
                _ => break,
            }
            characters.next();
        }
        conversion.width = number(characters, MAX_WIDTH, "width")?;
        if characters.peek() == Some(&'.') {
            characters.next();
            let places = number(
                characters,
                usize::try_from(MAX_PLACES).unwrap_or(usize::MAX),
                "precision",
            )?;
            conversion.precision = Some(u32::try_from(places).unwrap_or(MAX_PLACES));
        }
        conversion.kind = match characters.next() {
            Some('d') => Kind::Integer,
            Some('f') => Kind::Float,
            Some('s') => Kind::String,
            Some('x') => Kind::Hex,
            Some(other) => {
                return Err(Refusal::new(format!(
                    "its format has the conversion %{other}, and the conversions are d, f, s and x"
                )));
            }
            None => return Err(Refusal::new("its format ends inside a conversion")),
        };
        Ok(conversion)
    }

    /// The argument written through this conversion.
    fn write(&self, argument: &Argument) -> String {
        let number = || match argument {
            Argument::Number(value) => *value,
            Argument::Text(text) => super::make_number(text).unwrap_or(f64::NAN),
        };
        let style = self.separators.unwrap_or(Separators {
            group: None,
            point: '.',
        });
        let (sign, body) = match self.kind {
            Kind::String => {
                let text = match argument {
                    Argument::Number(value) => string_of(*value),
                    Argument::Text(text) => text.clone(),
                };
                return pad(&text, self.width, ' ');
            }
            Kind::Integer | Kind::Float | Kind::Hex if !number().is_finite() => {
                return pad(&number_text(number()), self.width, ' ');
            }
            Kind::Integer => {
                let (negative, digits) = digits_with(number().trunc(), 0, style);
                (negative, digits)
            }
            Kind::Float => {
                let places = self.precision.unwrap_or(6);
                let (negative, mut digits) = digits_with(number(), places, style);
                if places == 0 && self.alternate {
                    digits.push(style.point);
                }
                (negative, digits)
            }
            Kind::Hex => (false, hex(number())),
        };
        let sign = if sign {
            "-"
        } else if self.plus && self.kind != Kind::Hex {
            "+"
        } else if self.space && self.kind != Kind::Hex {
            " "
        } else {
            ""
        };
        if self.zeros {
            let wanted = self.width.saturating_sub(sign.chars().count());
            format!("{sign}{}", pad(&body, wanted, '0'))
        } else {
            pad(&format!("{sign}{body}"), self.width, ' ')
        }
    }
}

/// A run of decimal digits, `0` where there is none, refused past `ceiling`.
fn number(
    characters: &mut std::iter::Peekable<std::str::Chars<'_>>,
    ceiling: usize,
    what: &str,
) -> Result<usize, Refusal> {
    let mut value = 0_usize;
    while let Some(digit) = characters.peek().and_then(|digit| digit.to_digit(10)) {
        characters.next();
        value = value
            .saturating_mul(10)
            .saturating_add(usize::try_from(digit).unwrap_or(0));
        if value > ceiling {
            return Err(Refusal::new(format!(
                "its format asks for a {what} past {ceiling}"
            )));
        }
    }
    Ok(value)
}

/// `text` padded on the left with `fill` to `width` characters.
fn pad(text: &str, width: usize, fill: char) -> String {
    let short = width.saturating_sub(text.chars().count());
    let mut out: String = std::iter::repeat_n(fill, short).collect();
    out.push_str(text);
    out
}

/// `%s` of a number: ECMAScript's string of it rounded to [`STRING_DIGITS`] significant digits.
fn string_of(value: f64) -> String {
    if !value.is_finite() || value == 0.0 {
        return number_text(value);
    }
    let rounded = format!("{value:.*e}", STRING_DIGITS.saturating_sub(1))
        .parse::<f64>()
        .unwrap_or(value);
    number_text(rounded)
}

/// `%x` of a number: its integer part as C's `unsigned int`, in upper-case hexadecimal.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the wrap to 32 bits is the conversion: C's unsigned int, a documented choice"
)]
fn hex(value: f64) -> String {
    let whole = value.trunc().clamp(-2_147_483_648.0, 4_294_967_295.0);
    let bits = if whole < 0.0 {
        (whole as i64) as u32
    } else {
        whole as u32
    };
    format!("{bits:X}")
}

#[cfg(test)]
mod tests {
    use super::{Argument, printf};

    /// The "util methods" page's example, one hundred π through each of the four conversions.
    #[test]
    fn the_reference_s_example_writes_what_it_shows() {
        let n = std::f64::consts::PI * 100.0;
        let one = |format: &str| printf(format, &[Argument::Number(n)]).expect("formats");
        assert_eq!(one("Decimal format: %d"), "Decimal format: 314");
        assert_eq!(one("Hex format: %x"), "Hex format: 13A");
        assert_eq!(one("Float format: %.2f"), "Float format: 314.16");
        assert_eq!(one("String format: %s"), "String format: 314.159265358979");
    }

    #[test]
    fn separators_flags_and_width_are_the_page_s() {
        let one =
            |format: &str, value: f64| printf(format, &[Argument::Number(value)]).expect("formats");
        assert_eq!(one("%,0.2f", 1_234_567.891), "1,234,567.89");
        assert_eq!(one("%,2.2f", 1_234_567.891), "1.234.567,89");
        assert_eq!(one("%,3d", 1234.0), "1234");
        assert_eq!(one("%+d", 5.0), "+5");
        assert_eq!(one("% d", 5.0), " 5");
        assert_eq!(one("%05d", -42.0), "-0042");
        assert_eq!(one("%6.1f", 2.25), "   2.3");
        assert_eq!(one("%#.0f", 7.0), "7.");
        assert_eq!(one("%02d", 7.0), "07");
        assert_eq!(one("%4.4f", 0.5), "0.5000");
        assert_eq!(one("%f", 1.5), "1.500000");
        assert_eq!(one("%x", -1.0), "FFFFFFFF");
    }

    #[test]
    fn a_string_is_written_as_it_is_and_percent_doubles() {
        assert_eq!(
            printf(
                "%s%% of %s",
                &[
                    Argument::Number(12.0),
                    Argument::Text("the total".to_owned())
                ]
            )
            .expect("formats"),
            "12% of the total"
        );
    }

    #[test]
    fn a_count_that_differs_and_a_conversion_it_lacks_are_refused() {
        assert!(printf("%d %d", &[Argument::Number(1.0)]).is_err());
        assert!(printf("none", &[Argument::Number(1.0)]).is_err());
        assert!(printf("%q", &[Argument::Number(1.0)]).is_err());
        assert!(printf("%,9d", &[Argument::Number(1.0)]).is_err());
        assert!(printf("%99999d", &[Argument::Number(1.0)]).is_err());
    }
}
