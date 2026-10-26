//! Table 380's inline-level layout attributes, as one element states them.
//!
//! ISO 32000-2 §14.8.5.4.4 lists nine. Four of them describe something a sighted reader sees the
//! text *do* — a line under it, over it or through it, and a run raised or lowered off the line —
//! and a platform's text interface has a word for each, so they are read here for the
//! accessibility tree (ADR 2384). The other five are a layout engine's: `/LineHeight` and
//! `/GlyphOrientationVertical` parameterise how lines and glyphs are set, `/TextDecorationThickness`
//! sizes a rule, and `/RubyAlign` and `/RubyPosition` place a ruby annotation against its base;
//! this program draws the appearance those parameters already produced, and no platform text
//! interface carries any of them.
//!
//! **Everything here is what the element states of itself**, before any inheritance, because
//! Table 380 gives the four entries three different reaches and the two that depend on the
//! element's §14.8.4.1 category — which for some types depends on where the element sits — can only be resolved by a
//! caller walking the tree: `viewer_core`'s walk does it.

use pdf_syntax::{Dictionary, Document};

use super::{Tree, ranked};

/// Table 380's `/TextDecorationType`: the text decoration "to be applied to the element's text".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextDecoration {
    /// "No text decoration", the entry's default.
    #[default]
    None,
    /// "A line below the text".
    Underline,
    /// "A line above the text".
    Overline,
    /// "A line through the middle of the text".
    LineThrough,
}

impl TextDecoration {
    /// The entry's four names, or `None` for a name the table does not list.
    #[must_use]
    pub fn read(name: &[u8]) -> Option<Self> {
        match name {
            b"None" => Some(Self::None),
            b"Underline" => Some(Self::Underline),
            b"Overline" => Some(Self::Overline),
            b"LineThrough" => Some(Self::LineThrough),
            _ => None,
        }
    }
}

/// Table 380's `/TextPosition` (PDF 2.0): "\[t]he position of the element relative the immediately
/// surrounding content".
///
/// The cell is careful to claim a position and not a meaning — "`TextPosition` does not imply any
/// specific semantic" — and its NOTE 2 gives the reason: a raised run may be "a footnote number,
/// an exponent, an index or some other use of the text". A platform's sub- and superscript are the
/// same kind of statement, a position, which is why the two meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextPosition {
    /// "Position is elevated, like for superscript."
    Sup,
    /// "Position is lowered, like for subscript".
    Sub,
    /// "Position is neither elevated nor lowered", the entry's default.
    #[default]
    Normal,
}

impl TextPosition {
    /// The entry's three names, or `None` for a name the table does not list.
    #[must_use]
    pub fn read(name: &[u8]) -> Option<Self> {
        match name {
            b"Sup" => Some(Self::Sup),
            b"Sub" => Some(Self::Sub),
            b"Normal" => Some(Self::Normal),
            _ => None,
        }
    }
}

/// The four Table 380 entries one element states of itself, each `None` where it states nothing.
///
/// A name or a value the table does not define is read as the entry's **default** rather than as
/// silence, which is [`Tree::writing_mode`]'s reading of the same kind of cell: the value "shall"
/// be one the table lists, so an element stating another has not stated a decoration or a position,
/// and the default is what an element stating none of them means — and it stops an inherited value
/// at the element that wrote the defect rather than carrying an ancestor's past it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InlineStatements {
    /// `/TextDecorationType` — "(Optional; inheritable only for directly nested ILSEs)".
    pub decoration: Option<TextDecoration>,
    /// `/TextDecorationColor` — "(Optional; inheritable)": "an array of three numbers in the range
    /// 0.0 to 1.0, representing the red, green, and blue values, respectively, of an RGB colour
    /// space". An array of another shape, or a component outside the range, is not a colour and
    /// is read as unstated, because the entry's default is not a value this crate can name: it is
    /// "the current fill colour in effect at the start of its associated content".
    pub decoration_colour: Option<[f32; 3]>,
    /// `/BaselineShift` — "(Optional; not inheritable)", in default user space units, positive
    /// "toward the before edge". `None` where the element states none, which the cell's default
    /// of 0 answers; a value that is not a finite number is 0 as well.
    pub baseline_shift: Option<f32>,
    /// `/TextPosition` — "(Optional; inheritable; PDF 2.0)".
    pub position: Option<TextPosition>,
}

impl Tree {
    /// What this element states of Table 380's decoration, baseline shift and text position.
    ///
    /// The element's attribute objects are read once and ranked once per entry, by §14.8.5.3's
    /// priority as [`Self::attribute`] ranks them; the fourth priority, inheritance, is not
    /// applied here (see the module comment for why).
    #[must_use]
    pub fn inline_statements(&self, document: &Document, element: &Dictionary) -> InlineStatements {
        let attached = self.attributes(document, element);
        let stated = |name: &str| ranked(document, &attached, name, |_| true);
        let decoration = stated("TextDecorationType").map(|object| {
            object
                .as_name()
                .and_then(|name| TextDecoration::read(name.as_bytes()))
                .unwrap_or_default()
        });
        let position = stated("TextPosition").map(|object| {
            object
                .as_name()
                .and_then(|name| TextPosition::read(name.as_bytes()))
                .unwrap_or_default()
        });
        let baseline_shift = stated("BaselineShift").map(|object| {
            object
                .as_number()
                .filter(|number| number.is_finite())
                .map_or(0.0, narrowed)
        });
        let decoration_colour = stated("TextDecorationColor").and_then(|object| {
            let components = object.as_array()?;
            let [red, green, blue] = components else {
                return None;
            };
            let mut colour = [0.0; 3];
            for (into, component) in colour.iter_mut().zip([red, green, blue]) {
                let value = document.resolve(component).as_number()?;
                if !(0.0..=1.0).contains(&value) {
                    return None;
                }
                *into = narrowed(value);
            }
            Some(colour)
        });
        InlineStatements {
            decoration,
            decoration_colour,
            baseline_shift,
            position,
        }
    }
}

/// A PDF number as the `f32` the rest of the structure reading keeps.
fn narrowed(number: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a baseline shift is a page distance, bounded by §14.11.2's 14 400 units, and a \
                  colour component is in 0.0 to 1.0"
    )]
    {
        number as f32
    }
}
