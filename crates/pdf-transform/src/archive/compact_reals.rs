//! An embedded CFF program's DICT reals that state no digit, restated as the zero this reader drew
//! with, in the same bytes (ADR 3426).
//!
//! ISO 32000-2 §9.9's Table 124 holds a `/FontFile3` of `/Subtype` `Type1C` or `CIDFontType0C` to
//! Adobe Technical Note #5176, and ISO 19005-2 section 6.2.11.2 and ISO 19005-4 section 6.2.10.2
//! hold the file to it (`pdf_archive`'s `fonts/compact-font-reals-state-a-number`). The note encodes
//! a real operand as a decimal number's nibbles; one with no digit before its exponent or its end
//! states no number, and this reader, `read-fonts` after `FreeType`, reads it as zero.
//!
//! **The value written is that zero, and the bytes it is written in are the operand's own**
//! (`pdf_font::cff::with_digitless_reals_restated`): zero digits ended by the 0xf nibble, over the
//! width this reader tokenised the operand at. Nothing after the operand moves, so every offset the
//! program states stays true and every other byte of it is unchanged — the restatement says what
//! the program was read as, and decides nothing a page shows. That is checked here rather than
//! assumed: every glyph of the restated program draws the outline the original drew, segment for
//! segment, or the program is refused by name and the finding stands.
//!
//! What stays refused, by name: a program written inside another object, a stream this reader
//! does not decode, and a program `/OpenType` wraps, which the row does not read.

use std::collections::BTreeMap;

use pdf_syntax::{Document, Object, ObjectId};
use skrifa::outline::OutlinePen;

use super::decision::Because;

/// The requirement this module answers, by the identifier the validator reports it under.
pub(super) const ROW: &str = "fonts/compact-font-reals-state-a-number";

/// Why a program's reals are not restated.
const NOT_RESTATED: &str = "an embedded CFF program states a DICT real with no digit, which \
     Adobe Technical Note #5176 does not admit, and this conversion restates such a real as the \
     zero this reader read only in a bare CFF program that is an object of its own, decodes, and \
     draws every glyph as it did; this program is not that";

/// The restated programs, each keyed by the object it goes into, in the source's numbering.
#[derive(Debug, Default)]
pub(super) struct Restated {
    /// Each program stream, whole: new bytes, and the `/Filter` they are written under.
    pub(super) programs: BTreeMap<ObjectId, Object>,
    /// How many operands each program had restated.
    pub(super) operands: BTreeMap<ObjectId, usize>,
}

/// Every program [`ROW`] failed at, restated, or why one cannot be.
///
/// # Errors
///
/// [`NOT_RESTATED`] where any failing program is outside what the module comment lists; a
/// requirement half answered is a requirement refused.
pub(super) fn restated(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<Restated, Because> {
    let refused = Because::NotBuiltYet(NOT_RESTATED);
    let (named, elsewhere) = super::sites::objects_named(input, &[ROW])?;
    if elsewhere {
        return Err(refused);
    }
    let mut restated = Restated::default();
    for id in named {
        let Object::Stream(stream) = document.get(id) else {
            return Err(refused);
        };
        let original = document.decoded_stream_data(&stream).ok_or(refused)?;
        let operands = pdf_font::cff::digitless_reals(&original)
            .map_err(|_| refused)?
            .len();
        let program = pdf_font::cff::with_digitless_reals_restated(&original)
            .ok()
            .flatten()
            .ok_or(refused)?;
        if !draws_as_it_did(&original, &program) {
            return Err(refused);
        }
        let written = super::remedies::restated_stream(&stream.dict, &program).ok_or(refused)?;
        restated.programs.insert(id, written);
        restated.operands.insert(id, operands);
    }
    Ok(restated)
}

/// Whether every glyph of `restated` draws the outline the same glyph of `original` drew.
///
/// A glyph neither program draws counts as drawn alike; one only one of them draws does not.
fn draws_as_it_did(original: &[u8], restated: &[u8]) -> bool {
    let Ok(glyphs) = pdf_font::cff::glyph_count(original) else {
        return false;
    };
    if pdf_font::cff::glyph_count(restated).ok() != Some(glyphs) {
        return false;
    }
    (0..glyphs).all(|glyph| {
        let Ok(glyph) = u16::try_from(glyph) else {
            return false;
        };
        let mut before = Segments::default();
        let mut after = Segments::default();
        let drew_before = pdf_font::cff::draw(original, glyph, &mut before).is_ok();
        let drew_after = pdf_font::cff::draw(restated, glyph, &mut after).is_ok();
        drew_before == drew_after && before.0 == after.0
    })
}

/// An outline as the segments a pen was given, each point's coordinates by their bits.
#[derive(Default)]
struct Segments(Vec<(u8, [u32; 6])>);

impl OutlinePen for Segments {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push((0, [x.to_bits(), y.to_bits(), 0, 0, 0, 0]));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push((1, [x.to_bits(), y.to_bits(), 0, 0, 0, 0]));
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.push((
            2,
            [cx0.to_bits(), cy0.to_bits(), x.to_bits(), y.to_bits(), 0, 0],
        ));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.push((
            3,
            [
                cx0.to_bits(),
                cy0.to_bits(),
                cx1.to_bits(),
                cy1.to_bits(),
                x.to_bits(),
                y.to_bits(),
            ],
        ));
    }
    fn close(&mut self) {
        self.0.push((4, [0; 6]));
    }
}
