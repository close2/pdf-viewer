//! §14.11.2.2's guidelines: the page boundaries drawn on the screen, in the style the page asks.
//!
//! # The clause is a permission, and this is the offer
//!
//! ISO 32000-2 §14.11.2.2:
//!
//! > Interactive PDF processors may offer the ability to display guidelines on the screen for the
//! > various page boundaries.
//!
//! So nothing here runs unless a person has asked for it
//! ([`crate::view::ViewState::set_guidelines`]), and nothing a document says turns it on: the
//! page's `/BoxColorInfo` decides how a guideline looks, never whether there is one.
//!
//! # What is read, and from where
//!
//! Table 396 has an entry for each boundary "other than the media box", each a Table 397 box
//! style dictionary, and conditions every one of them on the page itself: the `/TrimBox` style
//! "shall be ignored if no trim box is defined in the page object", and the same sentence for the
//! other three. So a guideline is drawn for each of the crop, bleed, trim and art boxes the page
//! defines ([`Page::defines`]), and for none it takes by default — a box the page does not define
//! *is* the crop box (§14.11.2.1), and a second rectangle on top of the first would say that the
//! page states a boundary it does not.
//!
//! The rectangle is the one §14.11.2.1 makes of the entry: "\[i\]f the bounds of the crop, trim,
//! bleed or art box extends outside of the bounds of the media box, a processor shall treat the
//! box as its intersection with the media box", which [`Page::boundary`] has already applied.
//!
//! # Two kinds of default, and only one of them is the standard's
//!
//! Inside a box style dictionary an absent key has Table 397's default — `[0.0 0.0 0.0]`, `1`,
//! `S`, `[3]`. A box style dictionary that is absent altogether is the other case, and the
//! clause hands it to this program: "\[i\]f a given entry is absent, the interactive PDF processor
//! shall use its own current default settings instead." [`own_default`] is those settings, and
//! it is **a documented choice**: Table 397's own defaults for the width and the style, with a
//! colour per boundary so that four rectangles drawn together can be told apart. No clause states
//! those colours; they are this program's, and nothing else.
//!
//! A value that is present and is not what Table 397 says it is — a colour that is not three
//! numbers in the range 0.0 to 1.0, a negative width, a style this program does not know, a dash
//! array §8.4.3.6 would not accept — is drawn with that key's Table 397 default and **said**, in
//! [`Guidelines::malformed`], because a silent substitution would show a reader a style the page
//! never asked for as though it had (`doc/traps/parsers-and-streams.md` trap 5). ADR 2440.

use pdf_syntax::{Dictionary, Document, Object};

use crate::page::{Boundary, Page};

/// The four boundaries Table 396 has an entry for, in the order a guideline for each is drawn.
///
/// The table's own order, which is also inside-out for a page laid out as §14.11.2.1's Figure 116
/// draws it — so where two guidelines coincide, the one drawn last is the art box's.
pub const STYLED: [Boundary; 4] = [
    Boundary::Crop,
    Boundary::Bleed,
    Boundary::Trim,
    Boundary::Art,
];

/// Table 397: how one boundary's guideline looks.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxStyle {
    /// `/C`: three components in the `DeviceRGB` colour space, "of the colour that shall be used
    /// for displaying the guidelines".
    pub colour: [f32; 3],
    /// `/W`: "\[t\]he guideline width in default user space units".
    pub width: f32,
    /// `/S` and `/D` together: empty for the solid rectangle `S` names, and for `D` the dash
    /// array "in default user space units, in the same format as in the line dash pattern
    /// parameter of the graphics state", whose phase "shall not be specified and shall be
    /// assumed to be 0".
    pub dash: Vec<f32>,
}

impl BoxStyle {
    /// Table 397's defaults, every key absent: black, one unit wide, solid.
    pub const TABLE_397: Self = Self {
        colour: [0.0, 0.0, 0.0],
        width: 1.0,
        dash: Vec::new(),
    };
}

/// Table 397's `/D` default, `[3]`: three units on and three off.
const DEFAULT_DASH: [f32; 1] = [3.0];

/// One guideline: a boundary the page defines, where it lies and how it looks.
#[derive(Debug, Clone, PartialEq)]
pub struct Guideline {
    /// Which of Table 396's four boundaries.
    pub boundary: Boundary,
    /// The rectangle in default user space, intersected with the media box (§14.11.2.1).
    pub rect: [f32; 4],
    /// How it is drawn.
    pub style: BoxStyle,
    /// Whether the page's `/BoxColorInfo` styled it, rather than [`own_default`].
    pub stated: bool,
}

/// What a page's boundaries look like as guidelines, and what in its `/BoxColorInfo` could not be
/// read as Table 397 states it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Guidelines {
    /// One per boundary the page defines, in [`STYLED`]'s order.
    pub lines: Vec<Guideline>,
    /// One sentence per value drawn with its Table 397 default instead of as written.
    pub malformed: Vec<String>,
}

/// This program's own default settings for a boundary whose style the page does not state —
/// §14.11.2.2's "its own current default settings".
///
/// **A choice, and the clause's to give**: Table 397's width and style, and one colour per
/// boundary so that four rectangles over one page can be told apart — the crop box in Table
/// 397's own black, the bleed box blue, the trim box green and the art box red. ADR 2440.
#[must_use]
pub fn own_default(boundary: Boundary) -> BoxStyle {
    let colour = match boundary {
        Boundary::Media | Boundary::Crop => [0.0, 0.0, 0.0],
        Boundary::Bleed => [0.0, 0.0, 1.0],
        Boundary::Trim => [0.0, 0.6, 0.0],
        Boundary::Art => [1.0, 0.0, 0.0],
    };
    BoxStyle {
        colour,
        ..BoxStyle::TABLE_397
    }
}

/// The guidelines for one page: one for each of its defined boundaries Table 396 names.
#[must_use]
pub fn of(document: &Document, page: &Page) -> Guidelines {
    let info = document.get_key(&page.dict, "BoxColorInfo");
    let mut malformed = Vec::new();
    let info = match &info {
        Object::Null => None,
        stated => {
            let dictionary = stated.as_dict();
            if dictionary.is_none() {
                malformed.push(
                    "/BoxColorInfo is not a box colour information dictionary (Table 396), so \
                     every guideline is drawn in this program's own default settings"
                        .to_owned(),
                );
            }
            dictionary
        }
    };
    let mut lines = Vec::new();
    for boundary in STYLED {
        if !page.defines(document, boundary) {
            continue;
        }
        let key = key_of(boundary);
        let (style, stated) = match info.map(|info| document.get_key(info, key)) {
            None | Some(Object::Null) => (own_default(boundary), false),
            Some(entry) => {
                if let Some(dictionary) = entry.as_dict() {
                    (box_style(document, dictionary, key, &mut malformed), true)
                } else {
                    malformed.push(format!(
                        "/BoxColorInfo /{key} is not a box style dictionary (Table 397), so its \
                         guideline is drawn in this program's own default settings"
                    ));
                    (own_default(boundary), false)
                }
            }
        };
        lines.push(Guideline {
            boundary,
            rect: page.boundary(boundary),
            style,
            stated,
        });
    }
    Guidelines { lines, malformed }
}

/// Table 396's key for a boundary.
fn key_of(boundary: Boundary) -> &'static str {
    match boundary {
        Boundary::Media => "MediaBox",
        Boundary::Crop => "CropBox",
        Boundary::Bleed => "BleedBox",
        Boundary::Trim => "TrimBox",
        Boundary::Art => "ArtBox",
    }
}

/// Reads one Table 397 box style dictionary, each absent or unreadable key at its default.
fn box_style(
    document: &Document,
    style: &Dictionary,
    key: &str,
    malformed: &mut Vec<String>,
) -> BoxStyle {
    let mut said = |entry: &str, what: &str| {
        malformed.push(format!(
            "/BoxColorInfo /{key} /{entry} is not {what} (Table 397), so its default is drawn"
        ));
    };
    let colour = match document.get_key(style, "C") {
        Object::Null => BoxStyle::TABLE_397.colour,
        stated => colour(document, &stated).unwrap_or_else(|| {
            said("C", "three numbers in the range 0.0 to 1.0");
            BoxStyle::TABLE_397.colour
        }),
    };
    let width = match document.get_key(style, "W") {
        Object::Null => BoxStyle::TABLE_397.width,
        stated => stated
            .as_number()
            .map(narrow)
            .filter(|width| width.is_finite() && *width >= 0.0)
            .unwrap_or_else(|| {
                said("W", "a width in default user space units");
                BoxStyle::TABLE_397.width
            }),
    };
    let dashed = match document.get_key(style, "S") {
        Object::Null => false,
        stated => match stated.as_name().map(pdf_syntax::Name::as_bytes) {
            Some(b"S") => false,
            Some(b"D") => true,
            // "Other guideline styles may be defined in the future": one this program does not
            // know is not one it can draw, so the default stands and the name is said.
            _ => {
                said("S", "one of the guideline styles S and D");
                false
            }
        },
    };
    let dash = if dashed {
        match document.get_key(style, "D") {
            Object::Null => DEFAULT_DASH.to_vec(),
            stated => dash(document, &stated).unwrap_or_else(|| {
                said(
                    "D",
                    "a dash array of nonnegative numbers, not all zero (§8.4.3.6)",
                );
                DEFAULT_DASH.to_vec()
            }),
        }
    } else {
        Vec::new()
    };
    BoxStyle {
        colour,
        width,
        dash,
    }
}

/// `/C`: three numbers, each in Table 397's range 0.0 to 1.0, or `None`.
fn colour(document: &Document, stated: &Object) -> Option<[f32; 3]> {
    let items = stated.as_array()?;
    let [r, g, b] = items else {
        return None;
    };
    let component = |item: &Object| {
        document
            .resolve(item)
            .as_number()
            .map(narrow)
            .filter(|value| (0.0..=1.0).contains(value))
    };
    Some([component(r)?, component(g)?, component(b)?])
}

/// `/D`: §8.4.3.6's dash array, whose "numbers shall be nonnegative and not all zero", or
/// `None`. An empty array is §8.4.3.6's solid line, which `S` already names, so it
/// is not a pattern a dashed guideline can be drawn with either.
fn dash(document: &Document, stated: &Object) -> Option<Vec<f32>> {
    let items = stated.as_array()?;
    let lengths = items
        .iter()
        .map(|item| document.resolve(item).as_number().map(narrow))
        .collect::<Option<Vec<f32>>>()?;
    let usable = lengths
        .iter()
        .all(|length| length.is_finite() && *length >= 0.0)
        && lengths.iter().any(|length| *length > 0.0);
    usable.then_some(lengths)
}

/// Narrows a PDF number to the precision the display list carries.
fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a guideline's colour, width and dash are drawn in the display list's f32, and \
                  a value past f32's range is no guideline anybody could see"
    )]
    {
        value as f32
    }
}
