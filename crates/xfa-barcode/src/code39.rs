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
//! # What it leaves open, written down under principle 5
//!
//! - **The check character.** MIL-STD-1189A states no check character. ISO/IEC 16388 and
//!   MIL-STD-1189B are said to define an optional modulo-43 one, and XFA 3.3 page 440 allows
//!   `checksum` `auto` for `code3of9`; neither text is held, so `auto` is
//!   [`Refusal::Checksum`] rather than a rule reconstructed from memory or another program.
//!   `none`, page 613's default, is built.
//! - **The extended set.** `code3Of9extended` has "no official standard" (page 617) and
//!   MIL-STD-1189A prints no full-ASCII table, so it is not built.
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

/// Whether this module encodes the symbology: `code3Of9`, and `logmars`, which names the same
/// symbology by the standard that defines it (XFA 3.3 page 1565).
#[must_use]
pub fn serves(symbology: Symbology) -> bool {
    matches!(symbology.name(), "code3Of9" | "logmars")
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

/// Encode `data` between start and stop delimiters, a narrow space between characters.
///
/// # Errors
///
/// [`Refusal::Unencodable`] for a character outside MIL-STD-1189A paragraph 4.1's set (the delimiter
/// included, which may not appear in the message), [`Refusal::Checksum`] for any `checksum` but
/// `none` (the module comment says why `auto` waits), and [`Refusal::NotBuilt`] for a symbology
/// this module does not serve.
pub fn encode(symbology: Symbology, checksum: Checksum, data: &str) -> Result<Linear, Refusal> {
    if !serves(symbology) {
        return Err(Refusal::NotBuilt(symbology));
    }
    if checksum != Checksum::None {
        return Err(Refusal::Checksum {
            symbology,
            checksum,
        });
    }
    if let Some(character) = data
        .chars()
        .find(|held| *held == DELIMITER || character(*held).is_none())
    {
        return Err(Refusal::Unencodable {
            symbology,
            character,
        });
    }
    let message = std::iter::once(DELIMITER)
        .chain(data.chars())
        .chain(std::iter::once(DELIMITER));
    let mut elements = Vec::new();
    for (position, held) in message.enumerate() {
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
    /// Hold the widths to XFA 3.3 page 440's limits for the symbology: `code3of9` 0.191 mm to
    /// 1.25 mm and a ratio of 2.2 to 3.0, `logmars` 0.125 mm to 1.25 mm and 2.0 to 3.0.
    ///
    /// # Errors
    ///
    /// [`Refusal::Geometry`] naming the limit crossed, and [`Refusal::NotBuilt`] for a symbology
    /// this module does not serve.
    pub fn check(self, symbology: Symbology) -> Result<Self, Refusal> {
        let (least_width, least_ratio) = match symbology.name() {
            "code3Of9" => (0.191, 2.2),
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
