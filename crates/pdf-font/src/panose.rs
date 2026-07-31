//! ISO 32000-2 §9.8.3.2's `/Style` dictionary: a `CIDFont`'s classification, for substituting it.
//!
//! Table 122 gives a `CIDFont`'s descriptor a `/Style`, and §9.8.3.2 says the whole of what is in
//! it: "[o]nly the Panose entry is defined", whose value "shall be a 12-byte string" of two
//! parts — the font family class and subclass bytes "given in the `FamilyClass` field of the
//! \"OS/2\" table in a TrueType font", then "[t]en bytes for the PANOSE classification number
//! for the font".
//!
//! # Where the meaning of those bytes comes from, and why that matters here
//!
//! ISO 32000-2 states the **layout** and hands the **meaning** to two documents outside itself:
//! Microsoft's *TrueType 1.0 Font Files Technical Specification* for the family class, and
//! Hewlett-Packard's *PANOSE Classification Metrics Guide* for the ten digits. Both are in
//! clause 2's normative references, so reading them is reading this standard — but it means the
//! digit values below are cited to PANOSE and not to a clause, and a reader that treated them as
//! ISO's would be unable to say where they came from.
//!
//! Only the digits this program can act on are interpreted: whether the face has serifs, whether
//! it is monospaced, whether it is bold, and whether it is a symbol font. The other six — stroke
//! variation, arm style, letterform, midline, x-height, contrast — describe shapes a substitute
//! either has or has not, and choosing between installed families on them would be guessing with
//! more decimal places.
//!
//! # What this is for
//!
//! Substitution, and nothing else. A font with a `/FontFile` is drawn from its own program and
//! never asks. 20 of the 974 corpus documents state a `/Style`, on 23 descriptors between them.

/// A `/Style` dictionary's `/Panose` value, as far as this program reads it.
///
/// Constructed only from a well-formed twelve-byte string: the clause states that length, and a
/// string of any other length has not stated a classification. Nothing here pads or truncates,
/// because a PANOSE digit read at the wrong offset is a confident wrong answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panose {
    /// The ten PANOSE digits, without the two family-class bytes that precede them.
    digits: [u8; 10],
    /// The OS/2 `sFamilyClass` class, the high byte of the first two.
    class: u8,
}

impl Panose {
    /// Reads §9.8.3.2's twelve bytes.
    ///
    /// `None` for anything that is not exactly twelve bytes long.
    #[must_use]
    pub fn read(bytes: &[u8]) -> Option<Self> {
        let bytes: &[u8; 12] = bytes.try_into().ok()?;
        let mut digits = [0u8; 10];
        digits.copy_from_slice(bytes.get(2..12)?);
        Some(Self {
            digits,
            class: *bytes.first()?,
        })
    }

    /// PANOSE digit 1, `bFamilyType`: which of PANOSE's five classifications applies.
    ///
    /// This one decides whether the other nine mean anything: the digits of a Latin Text face
    /// and those of a Latin Symbol face are different scales that happen to share positions.
    #[must_use]
    pub fn family_type(self) -> FamilyType {
        match self.digits.first() {
            Some(2) => FamilyType::LatinText,
            Some(3) => FamilyType::LatinHandWritten,
            Some(4) => FamilyType::LatinDecorative,
            Some(5) => FamilyType::LatinSymbol,
            _ => FamilyType::Unstated,
        }
    }

    /// Whether the face has serifs, where PANOSE says.
    ///
    /// Digit 2 is `bSerifStyle`, whose values run through the serifed shapes — cove, square,
    /// thin, oval, triangle — and then the sans ones, so the answer is which side of that
    /// boundary the value falls on. `None` where the digit is 0 (`Any`) or 1 (`No Fit`), which
    /// PANOSE defines as *no classification* rather than as a value, and where the family type
    /// makes the scale inapplicable.
    #[must_use]
    pub fn is_serif(self) -> Option<bool> {
        if self.family_type() != FamilyType::LatinText {
            return None;
        }
        match self.digits.get(1)? {
            0 | 1 => None,
            2..=10 => Some(true),
            _ => Some(false),
        }
    }

    /// Whether the face is monospaced, where PANOSE says.
    ///
    /// Digit 4 is `bProportion`, whose last value is `Monospaced`. The rest — old style, modern,
    /// even width, extended, condensed — are proportions this program cannot act on, since it
    /// chooses among installed families rather than stretching one.
    #[must_use]
    pub fn is_monospaced(self) -> Option<bool> {
        if self.family_type() != FamilyType::LatinText {
            return None;
        }
        match self.digits.get(3)? {
            0 | 1 => None,
            9 => Some(true),
            _ => Some(false),
        }
    }

    /// Whether the face is bold, where PANOSE says.
    ///
    /// Digit 3 is `bWeight`, running from very light to extra black. The threshold is `Demi`,
    /// which is where this crate already draws the line for `/FontWeight`: Table 120's numeric
    /// scale calls it 600, and a face marked `Demi` and one marked `600` are the same face
    /// described twice.
    #[must_use]
    pub fn is_bold(self) -> Option<bool> {
        match self.digits.get(2)? {
            0 | 1 => None,
            weight => Some(*weight >= 7),
        }
    }

    /// The OS/2 family class, the first of §9.8.3.2's twelve bytes.
    ///
    /// Read and not acted on. The IBM family classes overlap PANOSE's digits on everything this
    /// program decides — class 8 is sans serif, classes 1 to 7 are serif families — and where
    /// the two disagree there is no rule saying which wins. Carried so that a caller can see it.
    #[must_use]
    pub fn family_class(self) -> u8 {
        self.class
    }
}

/// PANOSE digit 1's five classifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyType {
    /// Latin text — the only one whose remaining digits this program reads.
    LatinText,
    /// Latin hand written.
    LatinHandWritten,
    /// Latin decorative.
    LatinDecorative,
    /// Latin symbol: a face whose glyphs are not letters.
    LatinSymbol,
    /// `Any` or `No Fit`, or a value PANOSE does not define.
    Unstated,
}

#[cfg(test)]
mod tests {
    use super::{FamilyType, Panose};

    /// §9.8.3.2's own EXAMPLE, read digit by digit.
    ///
    /// The clause writes it as `/Style <</Panose <01 05 02 02 03 00 00 00 00 00 00 00>>>`.
    ///
    /// Family class 1 with subclass 5, then the ten digits: family type 2 (Latin Text), serif
    /// style 2 (Cove — a serifed face), weight 3 (Light), proportion 0 (Any). So the clause's
    /// own example is a light serif face of unstated proportion, and every one of those four
    /// answers is a different digit read at a different offset.
    #[test]
    fn the_clauses_own_example_classifies_a_light_serif_face() {
        let panose = Panose::read(&[0x01, 0x05, 0x02, 0x02, 0x03, 0, 0, 0, 0, 0, 0, 0])
            .expect("twelve bytes");
        assert_eq!(panose.family_type(), FamilyType::LatinText);
        assert_eq!(panose.family_class(), 1);
        assert_eq!(panose.is_serif(), Some(true), "serif style 2 is Cove");
        assert_eq!(panose.is_bold(), Some(false), "weight 3 is Light");
        assert_eq!(
            panose.is_monospaced(),
            None,
            "proportion 0 is Any, which PANOSE defines as no classification at all"
        );
    }

    /// The boundaries: where serifs stop, where bold starts, and what monospaced is.
    #[test]
    fn the_digits_boundaries_are_panoses_own() {
        let with = |serif: u8, weight: u8, proportion: u8| {
            Panose::read(&[0, 0, 2, serif, weight, proportion, 0, 0, 0, 0, 0, 0])
                .expect("twelve bytes")
        };
        assert_eq!(with(10, 5, 3).is_serif(), Some(true), "10 is Triangle");
        assert_eq!(with(11, 5, 3).is_serif(), Some(false), "11 is Normal Sans");
        assert_eq!(with(0, 5, 3).is_serif(), None, "0 is Any");
        assert_eq!(with(1, 5, 3).is_serif(), None, "1 is No Fit");

        assert_eq!(with(3, 6, 3).is_bold(), Some(false), "6 is Medium");
        assert_eq!(
            with(3, 7, 3).is_bold(),
            Some(true),
            "7 is Demi, which is 600"
        );
        assert_eq!(with(3, 11, 3).is_bold(), Some(true), "11 is Extra Black");

        assert_eq!(with(3, 5, 9).is_monospaced(), Some(true));
        assert_eq!(with(3, 5, 2).is_monospaced(), Some(false), "2 is Old Style");
    }

    /// A family type other than Latin Text makes the Latin digits inapplicable.
    ///
    /// PANOSE's five classifications are five *scales*: digit 2 of a symbol face is a kind of
    /// symbol rather than a kind of serif, so answering "is it serifed" from it would be reading
    /// one table with another's key.
    #[test]
    fn the_latin_text_digits_do_not_apply_to_another_family_type() {
        let symbol = Panose::read(&[0, 0, 5, 3, 8, 9, 0, 0, 0, 0, 0, 0]).expect("twelve bytes");
        assert_eq!(symbol.family_type(), FamilyType::LatinSymbol);
        assert_eq!(symbol.is_serif(), None);
        assert_eq!(symbol.is_monospaced(), None);
        assert_eq!(
            symbol.is_bold(),
            Some(true),
            "weight is the one digit PANOSE keeps across its classifications"
        );
    }

    /// A string that is not twelve bytes has not stated a classification.
    #[test]
    fn only_twelve_bytes_are_a_panose_number() {
        assert!(Panose::read(&[]).is_none());
        assert!(Panose::read(&[0; 11]).is_none());
        assert!(Panose::read(&[0; 13]).is_none());
        assert!(Panose::read(&[0; 12]).is_some());
    }
}
