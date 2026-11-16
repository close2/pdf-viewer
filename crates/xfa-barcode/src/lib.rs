//! The barcode symbologies an XFA 3.3 template's `barcode` element names, encoded into bars and
//! spaces from public sources.
//!
//! # Why this crate exists, and where its symbols are drawn
//!
//! ISO 32000-2 §K.2 names XFA 3.3 as the format of an interactive form's XFA resource, and XFA
//! 3.3 chapter 12, *Using Barcodes*, pages 418–444, makes a barcode a field like any other "except
//! that its data is presented as an appropriately formed set of bars" (paraphrased). The
//! symbologies themselves are not XFA's: the `type` property's list in the template reference,
//! pages 616–620, names each one and the document that defines it. So this crate does one thing —
//! turns a field's data into the elements of the symbol the template typed — and decides nothing
//! about where the symbol goes or how large it is drawn.
//!
//! **What draws its output is the dynamic half's drawing piece** (`doc/todo/78` section 4), under
//! the authoring line's construction *a page laid out by a producer's own XFA template* (ADR
//! 2876): the type and the data are the template's, the position XFA 3.3's. It is not drawn into
//! a static form's widget: XFA's static half keeps every mark the `AcroForm` widget's
//! (`doc/scope-ratifications.md`), and a widget without an appearance is drawn by ISO 32000-2
//! Paragraph 12.7.4.3's variable text, which knows no barcode. ADR 2877 has the argument.
//!
//! # What is built, and what is refused by name
//!
//! A symbology is built only where a public source defines it; every other one the template
//! reference names is [`Refusal::NotBuilt`], said by its XFA identifier, and the window's drawing
//! piece says so — XFA 3.3 page 419 leaves an unsupported type's result to the application. What
//! each module rests on is in its own comment, and what its source leaves open is written there
//! too, under `CLAUDE.md` principle 5.
//!
//! - [`code39`]: Code 39 (`code3Of9`, and `logmars`, the Department of Defense's name for it) from MIL-STD-1189A.

#![forbid(unsafe_code)]

pub mod code39;

use std::fmt;

/// A barcode type XFA 3.3's template reference defines, by its identifier (pages 616–620).
///
/// The identifiers are matched without regard to case: the `type` property's description on page
/// 616, and chapter 12's *Barcode type* on page 421, both say its value is case-insensitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbology(&'static str);

/// Every identifier the `type` property's list defines, as the template reference spells them
/// (pages 616–620). `rfid` is among them and is not a visual barcode at all.
const DEFINED: [&str; 62] = [
    "aztec",
    "codabar",
    "code2Of5Industrial",
    "code2Of5Interleaved",
    "code2Of5Matrix",
    "code2Of5Standard",
    "code3Of9",
    "code3Of9extended",
    "code11",
    "code49",
    "code93",
    "code128",
    "code128A",
    "code128B",
    "code128C",
    "code128SSCC",
    "dataMatrix",
    "ean8",
    "ean8add2",
    "ean8add5",
    "ean13",
    "ean13add2",
    "ean13add5",
    "ean13pwcd",
    "fim",
    "logmars",
    "maxicode",
    "msi",
    "pdf417",
    "pdf417macro",
    "plessey",
    "postAUSCust2",
    "postAUSCust3",
    "postAUSReplyPaid",
    "postAUSStandard",
    "postUKRM4SCC",
    "postUSDPBC",
    "postUSIMB",
    "postUSStandard",
    "postUS5Zip",
    "QRCode",
    "rfid",
    "rss14",
    "rss14Expanded",
    "rss14Limited",
    "rss14Stacked",
    "rss14StackedOmni",
    "rss14Truncated",
    "telepen",
    "ucc128",
    "ucc128random",
    "ucc128sscc",
    "upcA",
    "upcAadd2",
    "upcAadd5",
    "upcApwcd",
    "upcE",
    "upcEadd2",
    "upcEadd5",
    "upcean2",
    "upcean5",
    "upsMaxicode",
];

impl Symbology {
    /// The defined symbology a template's `type` value names, or `None` for a value the template
    /// reference does not define (a custom type, page 419, which this program implements none of).
    #[must_use]
    pub fn from_type(value: &str) -> Option<Self> {
        DEFINED
            .iter()
            .find(|defined| defined.eq_ignore_ascii_case(value.trim()))
            .map(|defined| Self(defined))
    }

    /// The identifier as the template reference spells it.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.0
    }

    /// Whether this crate encodes the symbology.
    #[must_use]
    pub fn is_built(self) -> bool {
        code39::serves(self)
    }
}

impl fmt::Display for Symbology {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// The checksum a template asks for: the `checksum` property, XFA 3.3 template reference page 613.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checksum {
    /// `none`: insert no checksum — the default, always allowed though it may be ignored.
    None,
    /// `auto`: insert the default checksum of the barcode format.
    Auto,
    /// `1mod10`.
    OneModTen,
    /// `2mod10`.
    TwoModTen,
    /// `1mod10_1mod11`: a "1 modulo 10" checksum followed by a "1 modulo 11" one.
    OneModTenOneModEleven,
}

impl Checksum {
    /// The keyword a template writes, or `None` for one page 613 does not list.
    #[must_use]
    pub fn from_keyword(value: &str) -> Option<Self> {
        Some(match value {
            "none" => Self::None,
            "auto" => Self::Auto,
            "1mod10" => Self::OneModTen,
            "2mod10" => Self::TwoModTen,
            "1mod10_1mod11" => Self::OneModTenOneModEleven,
            _ => return None,
        })
    }

    /// The keyword as page 613 spells it.
    #[must_use]
    pub fn keyword(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Auto => "auto",
            Self::OneModTen => "1mod10",
            Self::TwoModTen => "2mod10",
            Self::OneModTenOneModEleven => "1mod10_1mod11",
        }
    }
}

/// Why a symbol was not encoded, each said in terms a window can repeat to the reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The symbology is one XFA 3.3 defines and this crate does not build; ADR 2877 says why each
    /// waits.
    NotBuilt(Symbology),
    /// A character the symbology's character set does not hold.
    Unencodable {
        /// The symbology.
        symbology: Symbology,
        /// The character.
        character: char,
    },
    /// A checksum the symbology's source does not define, or one this crate cannot compute from
    /// the source it holds.
    Checksum {
        /// The symbology.
        symbology: Symbology,
        /// The checksum asked for.
        checksum: Checksum,
    },
    /// A module width or a wide-to-narrow ratio outside the limits XFA 3.3 page 440 gives the
    /// symbology.
    Geometry {
        /// The symbology.
        symbology: Symbology,
        /// Which limit, in words.
        limit: &'static str,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotBuilt(symbology) => {
                write!(
                    formatter,
                    "the barcode type {symbology} is not drawn by this program"
                )
            }
            Self::Unencodable {
                symbology,
                character,
            } => write!(
                formatter,
                "{symbology} cannot encode the character {character:?}"
            ),
            Self::Checksum {
                symbology,
                checksum,
            } => write!(
                formatter,
                "{symbology} is not drawn with the checksum {}",
                checksum.keyword()
            ),
            Self::Geometry { symbology, limit } => {
                write!(
                    formatter,
                    "{symbology}'s {limit} is outside XFA 3.3's limits"
                )
            }
        }
    }
}

impl std::error::Error for Refusal {}

/// One element of a linear symbol: a bar or a space, narrow or wide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element {
    /// A bar (printed) rather than a space.
    pub bar: bool,
    /// Wide rather than narrow.
    pub wide: bool,
}

/// A linear symbol: its elements left to right, from the first bar of the start character to the
/// last bar of the stop character. The quiet zone is not in it — XFA 3.3 page 418 leaves the
/// blank space around a barcode to the form's author.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Linear {
    /// The elements, alternating bar and space.
    pub elements: Vec<Element>,
}

/// A printed bar of a linear symbol, placed along the symbol in the units it was laid out in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    /// The distance from the symbol's left edge to the bar's.
    pub x: f64,
    /// The bar's width.
    pub width: f64,
}

impl Linear {
    /// The symbol's bars, with a narrow element `narrow` wide and a wide one `narrow × ratio`.
    #[must_use]
    pub fn bars(&self, narrow: f64, ratio: f64) -> Vec<Bar> {
        let wide = narrow * ratio;
        let mut x = 0.0;
        let mut bars = Vec::new();
        for element in &self.elements {
            let width = if element.wide { wide } else { narrow };
            if element.bar {
                bars.push(Bar { x, width });
            }
            x += width;
        }
        bars
    }

    /// The symbol's whole width at the same two widths.
    #[must_use]
    pub fn width(&self, narrow: f64, ratio: f64) -> f64 {
        self.elements
            .iter()
            .map(|element| if element.wide { narrow * ratio } else { narrow })
            .sum()
    }
}

/// Encode a field's data as the symbol its template's `barcode` element types.
///
/// # Errors
///
/// [`Refusal::NotBuilt`] for a symbology this crate does not build, and the symbology's own
/// refusals for data or a checksum it cannot carry.
pub fn encode(symbology: Symbology, checksum: Checksum, data: &str) -> Result<Linear, Refusal> {
    if code39::serves(symbology) {
        return code39::encode(symbology, checksum, data);
    }
    Err(Refusal::NotBuilt(symbology))
}
