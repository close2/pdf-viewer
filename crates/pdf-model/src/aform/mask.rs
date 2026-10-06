//! `AFSpecial`'s four masks and the arbitrary mask: `util.printx` written, and its mask read.
//!
//! Adobe's *JavaScript for Acrobat API Reference* documents `printx` on its "util methods" page:
//! a table of masking characters — `?` copies the next character, `X` the next alphanumeric one,
//! `A` the next letter, `9` the next digit, each "skipping any others", `*` the rest, `\` escapes,
//! and `>`, `<` and `=` switch case translation — and one example, a telephone number pulled out
//! of letters on either side, which is a fixture. The Interapplication reference's
//! `SetJavaScriptAction` names the four special formats (`0` zip code, `1` zip + 4, `2` phone,
//! `3` SSN) and nothing about their masks; the masks below are the ones those four names denote in
//! the United States conventions they come from, and the arbitrary mask's letters are a convention
//! documented nowhere (ADR 1578 section 7).

/// The case translation `printx`'s `>`, `<` and `=` select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Case {
    /// `=`, the default: as the source has it.
    Preserve,
    /// `>`: upper case.
    Upper,
    /// `<`: lower case.
    Lower,
}

/// `util.printx`: a source string written through a mask.
///
/// **The choice where the table is silent** (ADR 1578 section 7): when the source runs out, the
/// mask stops at the first place-holder it cannot fill, and literal characters written since the
/// last place-holder that *was* filled are dropped with it — so `99999-9999` over `12345` writes
/// `12345`, not `12345-`.
#[must_use]
pub fn print_mask(mask: &str, source: &str) -> String {
    let source: Vec<char> = source.chars().collect();
    let mut next = 0_usize;
    let mut out = String::new();
    let mut pending = String::new();
    let mut case = Case::Preserve;
    let mut chars = mask.chars();
    while let Some(m) = chars.next() {
        let wanted: Option<fn(char) -> bool> = match m {
            '?' => Some(|_| true),
            'X' => Some(char::is_alphanumeric),
            'A' => Some(char::is_alphabetic),
            '9' => Some(|c: char| c.is_ascii_digit()),
            _ => None,
        };
        if let Some(wanted) = wanted {
            while source.get(next).is_some_and(|c| !wanted(*c)) {
                next = next.saturating_add(1);
            }
            let Some(&c) = source.get(next) else {
                return out;
            };
            next = next.saturating_add(1);
            out.push_str(&pending);
            pending.clear();
            push_cased(&mut out, c, case);
            continue;
        }
        match m {
            '*' => {
                out.push_str(&pending);
                pending.clear();
                for c in source.get(next..).unwrap_or_default() {
                    push_cased(&mut out, *c, case);
                }
                next = source.len();
            }
            '\\' => {
                if let Some(escaped) = chars.next() {
                    pending.push(escaped);
                }
            }
            '>' => case = Case::Upper,
            '<' => case = Case::Lower,
            '=' => case = Case::Preserve,
            literal => pending.push(literal),
        }
    }
    out.push_str(&pending);
    out
}

/// Appends a character under a case translation.
fn push_cased(out: &mut String, c: char, case: Case) {
    match case {
        Case::Preserve => out.push(c),
        Case::Upper => out.extend(c.to_uppercase()),
        Case::Lower => out.extend(c.to_lowercase()),
    }
}

/// One of the four masks the Interapplication reference's `psf` menu names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Special {
    /// 0: a five-digit zip code.
    Zip,
    /// 1: zip + 4, nine digits.
    ZipPlusFour,
    /// 2: a telephone number, seven digits or ten with an area code.
    Phone,
    /// 3: a social security number, nine digits.
    SocialSecurity,
}

impl Special {
    /// The format a `psf` argument names, or `None` outside 0–3.
    pub(crate) fn of(psf: f64) -> Option<Self> {
        Some(match super::number::whole(psf)? {
            0 => Self::Zip,
            1 => Self::ZipPlusFour,
            2 => Self::Phone,
            3 => Self::SocialSecurity,
            _ => return None,
        })
    }

    /// The `printx` mask this format writes a value through.
    ///
    /// A phone number takes the area-code mask once ten digits are present and the local one
    /// before, because a seven-digit number is a whole local number and three more digits are
    /// what an area code is.
    pub(crate) fn mask(self, value: &str) -> &'static str {
        match self {
            Self::Zip => "99999",
            Self::ZipPlusFour => "99999-9999",
            Self::Phone if digit_count(value) >= 10 => "(999) 999-9999",
            Self::Phone => "999-9999",
            Self::SocialSecurity => "999-99-9999",
        }
    }

    /// How many digits a complete value of this format has: one count, or two for a phone.
    fn complete(self) -> &'static [usize] {
        match self {
            Self::Zip => &[5],
            Self::ZipPlusFour | Self::SocialSecurity => &[9],
            Self::Phone => &[7, 10],
        }
    }

    /// The digits of one complete value of this format, as a person would type them.
    pub(crate) fn example(self) -> &'static str {
        match self {
            Self::Zip => "12345",
            Self::ZipPlusFour | Self::SocialSecurity => "123456789",
            Self::Phone => "4155551234",
        }
    }

    /// The characters a person may type besides digits: the masks' own punctuation.
    fn punctuation(self) -> &'static [char] {
        match self {
            Self::Zip => &[],
            Self::ZipPlusFour | Self::SocialSecurity => &['-', ' '],
            Self::Phone => &['(', ')', '-', ' ', '.'],
        }
    }

    /// `AFSpecial_Keystroke`'s judgement of a field's text.
    ///
    /// **The choice** (ADR 1578 section 7): the text may hold digits and the format's own
    /// punctuation and nothing else; while typing it may hold up to the format's largest digit
    /// count, and at commit exactly one of its complete counts, or be empty. What a person typed is
    /// kept as typed — the format script is what shows the mask.
    pub(crate) fn accepts(self, text: &str, will_commit: bool) -> bool {
        if text.is_empty() {
            return true;
        }
        if !text
            .chars()
            .all(|c| c.is_ascii_digit() || self.punctuation().contains(&c))
        {
            return false;
        }
        let digits = digit_count(text);
        let most = self.complete().iter().copied().max().unwrap_or(0);
        if will_commit {
            self.complete().contains(&digits)
        } else {
            digits <= most
        }
    }
}

/// How many ASCII digits a text holds.
fn digit_count(text: &str) -> usize {
    text.chars().filter(char::is_ascii_digit).count()
}

/// Whether a character fills one position of `AFSpecial_KeystrokeEx`'s arbitrary mask.
///
/// **The convention** (ADR 1578 section 7), documented nowhere by Adobe: `9` a digit, `A` a
/// letter, `O` a letter or digit, `X` any character, and every other character of the mask itself.
fn fills(mask: char, c: char) -> bool {
    match mask {
        '9' => c.is_ascii_digit(),
        'A' => c.is_alphabetic(),
        'O' => c.is_alphanumeric(),
        'X' => true,
        literal => literal == c,
    }
}

/// Whether a mask character is a place-holder rather than a literal.
fn is_placeholder(mask: char) -> bool {
    matches!(mask, '9' | 'A' | 'O' | 'X')
}

/// `AFSpecial_KeystrokeEx`'s judgement of a field's text, and the text it commits.
///
/// While typing, the text must be a prefix of the mask, or a prefix of the mask with its literals
/// taken out — a person typing `5551234` into `999-9999` has typed a telephone number. At commit
/// it must fill the mask whole, either way; where it filled the mask without its literals, the
/// committed text is the mask's literals put back, which is the only place this format can show
/// them, since the arbitrary mask has no format script. `None` refuses.
pub(crate) fn arbitrary(mask: &str, text: &str, will_commit: bool) -> Option<String> {
    if text.is_empty() {
        return Some(String::new());
    }
    let mask: Vec<char> = mask.chars().collect();
    let typed: Vec<char> = text.chars().collect();
    let whole = matches_prefix(&mask, &typed);
    let placeholders: Vec<char> = mask
        .iter()
        .copied()
        .filter(|m| is_placeholder(*m))
        .collect();
    let bare = matches_prefix(&placeholders, &typed);
    if !will_commit {
        return (whole || bare).then(|| text.to_owned());
    }
    if whole && typed.len() == mask.len() {
        return Some(text.to_owned());
    }
    if bare && typed.len() == placeholders.len() {
        let mut out = String::new();
        let mut from = typed.iter();
        for m in &mask {
            if is_placeholder(*m) {
                out.push(*from.next()?);
            } else {
                out.push(*m);
            }
        }
        return Some(out);
    }
    None
}

/// An arbitrary mask with each place-holder filled: `1` for a digit, `A` for a letter or either,
/// `X` for any character, and every literal as the mask states it.
pub(crate) fn filled(mask: &str) -> String {
    mask.chars()
        .map(|m| match m {
            '9' => '1',
            'A' | 'O' => 'A',
            'X' => 'X',
            literal => literal,
        })
        .collect()
}

/// Whether `typed` fills the first positions of `mask`.
fn matches_prefix(mask: &[char], typed: &[char]) -> bool {
    typed.len() <= mask.len() && typed.iter().zip(mask).all(|(c, m)| fills(*m, *c))
}

#[cfg(test)]
mod tests {
    use super::{Special, arbitrary, print_mask};

    #[test]
    fn a_short_source_stops_at_the_first_unfilled_place_holder() {
        assert_eq!(print_mask("99999-9999", "12345"), "12345");
        assert_eq!(print_mask(">AAA", "abc"), "ABC");
        assert_eq!(print_mask("\\99", "7"), "97");
    }

    #[test]
    fn a_phone_number_takes_the_area_code_mask_at_ten_digits() {
        assert_eq!(Special::Phone.mask("5551234"), "999-9999");
        assert_eq!(Special::Phone.mask("4155551234"), "(999) 999-9999");
        assert!(Special::Phone.accepts("(415) 555-1234", true));
        assert!(!Special::Phone.accepts("41555", true));
        assert!(Special::Phone.accepts("41555", false));
    }

    #[test]
    fn an_arbitrary_mask_puts_its_literals_back_at_commit() {
        assert_eq!(
            arbitrary("999-9999", "5551234", true).as_deref(),
            Some("555-1234")
        );
        assert_eq!(
            arbitrary("999-9999", "555-1234", true).as_deref(),
            Some("555-1234")
        );
        assert_eq!(
            arbitrary("999-9999", "555-12", false).as_deref(),
            Some("555-12")
        );
        assert_eq!(arbitrary("999-9999", "55a", false), None);
    }
}
