//! Code 39, XFA 3.3's `code3Of9` and `logmars`, from MIL-STD-1189A.
//!
//! # The source
//!
//! The template reference (XFA 3.3 page 617) defines `code3Of9` by ANSI/AIM BC1-1995, and the
//! bibliography (page 1565) defines `logmars` by MIL-STD-1189B, the Department of Defense's
//! standard for the same symbology. Neither is held. What is held is **MIL-STD-1189A**, *Standard
//! Department of Defense Bar Code Symbology*, 4 September 1984 — a work of the United States
//! government, reproduced whole as Appendix E of a Naval Postgraduate School thesis that the
//! Defense Technical Information Center published as ADA180561 (1987) and the Internet Archive
//! hosts (`doc/third-party-data.md` has the address and the file's hash). Its paragraphs are cited
//! below as paragraph N, and its tables by letter and number.
//!
//! # What the source fixes
//!
//! - **The character set.** MIL-STD-1189A paragraph 4.1: 43 characters — the digits, the capital letters,
//!   `-`, `.`, `$`, `/`, `+`, `%` and space — and the asterisk as the start and stop delimiter,
//!   which table E.1's note says is the first and last character of every message.
//! - **Each character's elements.** Paragraph 4.1 again: nine elements, five bars and four spaces, three of
//!   them wide and six narrow. The standard's table E.1, *Code Configuration*, prints each character's five bars
//!   and four spaces as binary digits, 1 for wide, read left to right; its drawn pattern
//!   alternates bar and space starting and ending with a bar, and [`TABLE`] is table E.1 in that
//!   order.
//! - **The gap between characters.** Paragraph 5.2.2: at least one narrow element and at most three. This
//!   crate writes the minimum, a narrow space.
//! - **The widths.** Paragraph 5.2: a narrow element of 0.190 mm to 1.016 mm, and a wide-to-narrow ratio of
//!   2.2 to 3.0 below 0.381 mm and 2.0 to 3.0 from there up. XFA 3.3 page 440 states its own limits
//!   for `code3of9` (0.191 mm to 1.25 mm, ratio 2.2 to 3.0) and for `logmars` (0.125 mm to
//!   1.25 mm, ratio 2.0 to 3.0); [`Geometry::check`] holds a template to those, which is what the
//!   template's grammar admits.
//!
//! # The check character and the extended set, from open source under A857
//!
//! MIL-STD-1189A states neither, and the texts that do (ISO/IEC 16388, MIL-STD-1189B) are not
//! held. The owner's answer A857 lifts principle 5 for this symbology alone: its tables may be
//! derived from permissively licensed implementations. Both below are derived from `rxing` 0.9.4
//! (Apache-2.0, a port of `ZXing`), `src/oned/oned_constants.rs` and `src/oned/code_39_writer.rs`,
//! and `doc/third-party-data.md` records the copy.
//!
//! - **The check character.** Each of the 43 data characters has a value, its position in
//!   [`VALUES`]; the check character is the one whose value is the sum of the message's values
//!   modulo 43, placed after the last data character and before the stop delimiter. XFA 3.3 page
//!   440 ticks `auto` and `none` for `code3of9` and `code3of9extended`, and page 613 defines `auto`
//!   as the format's default checksum, so `auto` writes it and `none` does not; the other keywords
//!   are not ticked and are [`Refusal::Checksum`].
//! - **The extended set.** `code3Of9extended` has "no official standard" (page 617); the
//!   convention every implementation shares writes each of the 128 ASCII characters as one or two
//!   Code 39 characters, a shift (`$`, `%`, `/`, `+`) before a letter for those outside the base
//!   set, and [`EXTENDED`] is that table. The check character of an extended symbol is computed
//!   over the characters written, shifts included.
//! - **`logmars`.** Page 440 ticks no checksum column for it, and page 613 says the attribute is
//!   ignored for some types: a `logmars` symbol carries no check character whatever `checksum`
//!   says.
//!
//! # What is still open
//!
//! - **The upper bound on module width.** MIL-STD-1189A stops at 1.016 mm where XFA 3.3 page 440
//!   allows 1.25 mm; the template's limit is the one held, and a symbol between the two is wider
//!   than the 1984 standard describes.

use crate::{Checksum, Element, Linear, Refusal, Symbology};

/// MIL-STD-1189A table E.1, *Code Configuration*: each character, its five bars and its four
/// spaces, 1 for a wide element, left to right.
pub const TABLE: [(char, &str, &str); 44] = [
    ('1', "10001", "0100"),
    ('2', "01001", "0100"),
    ('3', "11000", "0100"),
    ('4', "00101", "0100"),
    ('5', "10100", "0100"),
    ('6', "01100", "0100"),
    ('7', "00011", "0100"),
    ('8', "10010", "0100"),
    ('9', "01010", "0100"),
    ('0', "00110", "0100"),
    ('A', "10001", "0010"),
    ('B', "01001", "0010"),
    ('C', "11000", "0010"),
    ('D', "00101", "0010"),
    ('E', "10100", "0010"),
    ('F', "01100", "0010"),
    ('G', "00011", "0010"),
    ('H', "10010", "0010"),
    ('I', "01010", "0010"),
    ('J', "00110", "0010"),
    ('K', "10001", "0001"),
    ('L', "01001", "0001"),
    ('M', "11000", "0001"),
    ('N', "00101", "0001"),
    ('O', "10100", "0001"),
    ('P', "01100", "0001"),
    ('Q', "00011", "0001"),
    ('R', "10010", "0001"),
    ('S', "01010", "0001"),
    ('T', "00110", "0001"),
    ('U', "10001", "1000"),
    ('V', "01001", "1000"),
    ('W', "11000", "1000"),
    ('X', "00101", "1000"),
    ('Y', "10100", "1000"),
    ('Z', "01100", "1000"),
    ('-', "00011", "1000"),
    ('.', "10010", "1000"),
    (' ', "01010", "1000"),
    ('*', "00110", "1000"),
    ('$', "00000", "1110"),
    ('/', "00000", "1101"),
    ('+', "00000", "1011"),
    ('%', "00000", "0111"),
];

/// The start and stop delimiter, MIL-STD-1189A paragraph 3.22 and table E.1's note.
pub const DELIMITER: char = '*';

/// The 43 data characters in the order of their check values, 0 to 42: `rxing` 0.9.4's
/// `ALPHABET_STRING` (`src/oned/oned_constants.rs`, Apache-2.0), under A857.
pub const VALUES: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";

/// The full-ASCII set `code3Of9extended` writes: entry *n* is ASCII character *n* as one or two
/// base characters. Derived from `rxing` 0.9.4's `tryToConvertToExtendedMode`
/// (`src/oned/code_39_writer.rs`, Apache-2.0), under A857, and transcribed row by row.
pub const EXTENDED: [&str; 128] = [
    "%U", "$A", "$B", "$C", "$D", "$E", "$F", "$G", // 0-7
    "$H", "$I", "$J", "$K", "$L", "$M", "$N", "$O", // 8-15
    "$P", "$Q", "$R", "$S", "$T", "$U", "$V", "$W", // 16-23
    "$X", "$Y", "$Z", "%A", "%B", "%C", "%D", "%E", // 24-31
    " ", "/A", "/B", "/C", "/D", "/E", "/F", "/G", // 32-39
    "/H", "/I", "/J", "/K", "/L", "-", ".", "/O", // 40-47
    "0", "1", "2", "3", "4", "5", "6", "7", // 48-55
    "8", "9", "/Z", "%F", "%G", "%H", "%I", "%J", // 56-63
    "%V", "A", "B", "C", "D", "E", "F", "G", // 64-71
    "H", "I", "J", "K", "L", "M", "N", "O", // 72-79
    "P", "Q", "R", "S", "T", "U", "V", "W", // 80-87
    "X", "Y", "Z", "%K", "%L", "%M", "%N", "%O", // 88-95
    "%W", "+A", "+B", "+C", "+D", "+E", "+F", "+G", // 96-103
    "+H", "+I", "+J", "+K", "+L", "+M", "+N", "+O", // 104-111
    "+P", "+Q", "+R", "+S", "+T", "+U", "+V", "+W", // 112-119
    "+X", "+Y", "+Z", "%P", "%Q", "%R", "%S", "%T", // 120-127
];

/// Whether this module encodes the symbology: `code3Of9`, `code3Of9extended`, and `logmars`,
/// which names Code 39 by the standard that defines it (XFA 3.3 page 1565).
#[must_use]
pub fn serves(symbology: Symbology) -> bool {
    matches!(
        symbology.name(),
        "code3Of9" | "code3Of9extended" | "logmars"
    )
}

/// The modulo-43 check character of a message of base characters, or `None` if one of them is
/// not among the 43 data characters ([`VALUES`]).
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the sum is reduced modulo 43 at every step, so it never passes 84"
)]
pub fn check_character(message: &str) -> Option<char> {
    let mut sum = 0_usize;
    for held in message.chars() {
        sum = (sum + VALUES.find(held)?) % 43;
    }
    VALUES.chars().nth(sum)
}

/// One character's nine elements, from [`TABLE`], or `None` for a character it does not hold.
#[must_use]
pub fn character(character: char) -> Option<[Element; 9]> {
    let (_, bars, spaces) = TABLE.iter().find(|(held, _, _)| *held == character)?;
    let bars = bars.as_bytes();
    let spaces = spaces.as_bytes();
    let mut elements = [Element {
        bar: true,
        wide: false,
    }; 9];
    for (index, element) in elements.iter_mut().enumerate() {
        let (from, at) = if index % 2 == 0 {
            (bars, index / 2)
        } else {
            (spaces, index / 2)
        };
        *element = Element {
            bar: index % 2 == 0,
            wide: from.get(at) == Some(&b'1'),
        };
    }
    Some(elements)
}

/// The base characters a symbology writes for `data`: the data itself for `code3Of9` and
/// `logmars`, each character's [`EXTENDED`] entry for `code3Of9extended`.
fn base_characters(symbology: Symbology, data: &str) -> Result<String, Refusal> {
    let extended = symbology.name() == "code3Of9extended";
    let unencodable = |character| Refusal::Unencodable {
        symbology,
        character,
    };
    let mut message = String::with_capacity(data.len());
    for held in data.chars() {
        if extended {
            let written = u8::try_from(held)
                .ok()
                .and_then(|ascii| EXTENDED.get(usize::from(ascii)))
                .ok_or_else(|| unencodable(held))?;
            message.push_str(written);
        } else if held != DELIMITER && character(held).is_some() {
            message.push(held);
        } else {
            return Err(unencodable(held));
        }
    }
    Ok(message)
}

/// Encode `data` between start and stop delimiters, a narrow space between characters.
///
/// # Errors
///
/// [`Refusal::Unencodable`] for a character outside MIL-STD-1189A paragraph 4.1's set (the delimiter
/// included, which may not appear in the message), or for `code3Of9extended` outside ASCII;
/// [`Refusal::Checksum`] for a `checksum` page 440 does not tick for `code3Of9` or
/// `code3Of9extended`; and [`Refusal::NotBuilt`] for a symbology this module does not serve.
pub fn encode(symbology: Symbology, checksum: Checksum, data: &str) -> Result<Linear, Refusal> {
    if !serves(symbology) {
        return Err(Refusal::NotBuilt(symbology));
    }
    let with_check = match (symbology.name(), checksum) {
        // Page 440 ticks no checksum column for logmars: the attribute is ignored (page 613).
        ("logmars", _) | (_, Checksum::None) => false,
        (_, Checksum::Auto) => true,
        _ => {
            return Err(Refusal::Checksum {
                symbology,
                checksum,
            });
        }
    };
    let mut message = base_characters(symbology, data)?;
    if with_check {
        // Every base character was checked against the table, whose 43 data characters are VALUES.
        message.extend(check_character(&message));
    }
    let framed = std::iter::once(DELIMITER)
        .chain(message.chars())
        .chain(std::iter::once(DELIMITER));
    let mut elements = Vec::new();
    for (position, held) in framed.enumerate() {
        if position > 0 {
            elements.push(Element {
                bar: false,
                wide: false,
            });
        }
        // Every character was found in the table above, and the delimiter is in it.
        elements.extend(character(held).into_iter().flatten());
    }
    Ok(Linear { elements })
}

/// A template's widths for one Code 39 symbol: the narrow element's width in millimetres and the
/// wide-to-narrow ratio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    /// The narrow element's width, millimetres (the template's `moduleWidth`).
    pub narrow_mm: f64,
    /// The wide-to-narrow ratio (the template's `wideNarrowRatio`).
    pub ratio: f64,
}

impl Geometry {
    /// Hold the widths to XFA 3.3 page 440's limits for the symbology: `code3of9` and
    /// `code3of9extended` 0.191 mm to 1.25 mm and a ratio of 2.2 to 3.0, `logmars` 0.125 mm to 1.25 mm and 2.0 to 3.0.
    ///
    /// # Errors
    ///
    /// [`Refusal::Geometry`] naming the limit crossed, and [`Refusal::NotBuilt`] for a symbology
    /// this module does not serve.
    pub fn check(self, symbology: Symbology) -> Result<Self, Refusal> {
        let (least_width, least_ratio) = match symbology.name() {
            "code3Of9" | "code3Of9extended" => (0.191, 2.2),
            "logmars" => (0.125, 2.0),
            _ => return Err(Refusal::NotBuilt(symbology)),
        };
        if !(least_width..=1.25).contains(&self.narrow_mm) {
            return Err(Refusal::Geometry {
                symbology,
                limit: "module width",
            });
        }
        if !(least_ratio..=3.0).contains(&self.ratio) {
            return Err(Refusal::Geometry {
                symbology,
                limit: "wide-to-narrow ratio",
            });
        }
        Ok(self)
    }
}
