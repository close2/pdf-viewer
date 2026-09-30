//! What a layout needs to know about an `sfnt` face it will set shaped text in.
//!
//! [`crate::substitute::installed_covering`] answers with a face's bytes; a layout that writes a
//! composite font around those bytes (ISO 32000-2 §9.7.4, a `CIDFontType2` whose CIDs are the
//! face's glyph indices) has to state each glyph's width and the face's vertical extent in the
//! dictionaries it writes, because §9.7.4.3 makes `/W` — and not the program — what the
//! interpreter advances by. Every number here is read from the face through `skrifa`, the one
//! `sfnt` reader in this tree.

use skrifa::MetadataProvider as _;
use skrifa::instance::{LocationRef, Size};

/// One face's answers, in the units §9.2.4 puts glyph space in: thousandths of an em.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceMetrics {
    /// The face's ascent above the baseline, for Table 120's `/Ascent`.
    pub ascent: f32,
    /// The face's descent below it, negative, for Table 120's `/Descent`.
    pub descent: f32,
    /// The union of every glyph's extent, for Table 120's `/FontBBox`.
    pub bounding_box: [f32; 4],
}

/// The glyph `character` maps to through the face's own `cmap`, and its advance in thousandths
/// of an em.
///
/// `None` where the face maps it to nothing, or states no advance for the glyph it maps to.
#[must_use]
pub fn glyph(program: &[u8], character: char) -> Option<(u16, f32)> {
    let font = skrifa::FontRef::new(program).ok()?;
    let glyph = font.charmap().map(character)?;
    let index = u16::try_from(glyph.to_u32()).ok()?;
    let units = f32::from(
        font.metrics(Size::unscaled(), LocationRef::default())
            .units_per_em,
    );
    if units <= 0.0 {
        return None;
    }
    let advance = font
        .glyph_metrics(Size::unscaled(), LocationRef::default())
        .advance_width(glyph)?;
    Some((index, advance * 1000.0 / units))
}

/// The face's vertical extent and bounding box, `None` where it states no units per em.
#[must_use]
pub fn metrics(program: &[u8]) -> Option<FaceMetrics> {
    let font = skrifa::FontRef::new(program).ok()?;
    let stated = font.metrics(Size::unscaled(), LocationRef::default());
    let units = f32::from(stated.units_per_em);
    if units <= 0.0 {
        return None;
    }
    let scale = 1000.0 / units;
    let bounding_box = stated
        .bounds
        .map_or([0.0, stated.descent, units, stated.ascent], |b| {
            [b.x_min, b.y_min, b.x_max, b.y_max]
        });
    Some(FaceMetrics {
        ascent: stated.ascent * scale,
        descent: stated.descent * scale,
        bounding_box: bounding_box.map(|value| value * scale),
    })
}
