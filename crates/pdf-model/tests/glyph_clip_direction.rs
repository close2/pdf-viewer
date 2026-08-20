//! Two substituted faces in one clipping path, ISO 32000-2 §9.3.6.
//!
//! # What this file is about
//!
//! §9.3.6 combines the glyph outlines a text object accumulated in a clipping render mode:
//!
//! > At the end of the text object identified by the ET operator the accumulated glyph
//! > outlines, if any, shall be combined into a single path, treating the individual outlines
//! > as subpaths of that path and applying the non-zero winding number rule
//!
//! and its NOTE 2 states the consequence:
//!
//! > Due to the use of non-zero winding number rule, the direction of the paths comprising each
//! > glyph can cause different output for overlapping glyphs.
//!
//! So two glyphs drawn in opposite directions **cancel** where they overlap instead of uniting.
//! Neither direction is the standard's — it states none — but §9.6.2.2 names its fourteen faces
//! as one set of Type 1 fonts, and a document may draw two of them into one path. A processor
//! whose stand-ins for two of the fourteen disagree about direction therefore manufactures a
//! hole no set of fourteen Type 1 programs would produce. `OverlappingGlyphClipping.pdf` in
//! `doc/corpora/pdf-differences` is the page that showed it (session 558); ADR 0396 is the fix.
//!
//! # Why it is asserted by construction rather than on that page
//!
//! The corpus page is one arrangement of two faces, and what went wrong was a property of the
//! *set*. So each pair is built here and asked the one question that needs no reference and no
//! knowledge of the glyphs' shapes: **a union contains each of its parts**. Under cancellation
//! the overlap is taken out of both, so the combined clip passes less ink than the larger glyph
//! alone — which is exactly what this tree drew for a sans-plus-serif pair and not for either
//! pair from one family.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and these counts are pixel \
              tallies of a 200-unit page that cannot overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 200-unit page below.
const GENEROUS: u64 = 1 << 30;

/// A one-page fixture naming two of §9.6.2.2's fourteen, neither embedded.
///
/// `/FSans` and `/FSerif` are answered by two different compiled-in programs — Liberation Sans
/// is an `sfnt` and Foxit's serif a bare CFF — which is the pair whose conventions disagreed.
fn fixture(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
         /Resources << /Font << /FSans 5 0 R /FSerif 6 0 R /FSansBold 7 0 R \
         /FSerifItalic 8 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Times-Bold >>\nendobj\n\
         7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\nendobj\n\
         8 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Times-BoldItalic >>\nendobj\n",
        content.len() + 1,
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Which pixels a page marks, which is what a clip decides here.
///
/// The whole page is filled black through whatever clip the text object left, so the mask is
/// the clip's own area in device pixels and nothing else.
fn marked(content: &str) -> Vec<bool> {
    let document = Document::open(fixture(content)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported");
    raster
        .data
        .chunks_exact(4)
        .map(|pixel| pixel[3] > 128)
        .collect()
}

/// How many pixels a mask marks.
fn count(mask: &[bool]) -> usize {
    mask.iter().filter(|marked| **marked).count()
}

/// The two masks combined pixel by pixel, which is what the two glyphs' *shapes* say the
/// combined clip should be — a union under the non-zero rule, whatever the two directions.
fn combine(one: &[bool], other: &[bool], join: fn(bool, bool) -> bool) -> Vec<bool> {
    one.iter()
        .zip(other)
        .map(|(&one, &other)| join(one, other))
        .collect()
}

/// Where the first and the second glyph of a pair sit.
///
/// Far enough apart that neither capital contains the other and close enough that they share a
/// substantial area — the arrangement where a union and a cancellation are furthest apart, and
/// the test asserts both halves of that rather than trusting the numbers.
const PLACES: [u32; 2] = [20, 38];

/// One `B` at 150 pt from `font`, added to the clipping path and then filled through.
fn one(font: &str, at: u32) -> String {
    format!("BT /{font} 150 Tf 7 Tr 1 0 0 1 {at} 30 Tm (B) Tj ET\n0 0 200 200 re f")
}

/// Two `B`s at 150 pt from two fonts, one text object, both added to the same clipping path.
fn both(first: &str, second: &str) -> String {
    let [near, far] = PLACES;
    format!(
        "BT 7 Tr /{first} 150 Tf 1 0 0 1 {near} 30 Tm (B) Tj \
         /{second} 150 Tf 1 0 0 1 {far} 30 Tm (B) Tj ET\n0 0 200 200 re f"
    )
}

/// Two glyphs in one clipping path unite, whichever two faces they come from.
///
/// Each glyph's own clip is rasterised alone, so the union and the overlap are **measured**
/// rather than assumed, and the combined clip is then held against the union pixel by pixel.
/// That is what makes this tight: a cancellation differs from the union by exactly the overlap,
/// which is thousands of pixels here, while antialiasing along an edge differs by tens.
///
/// The two same-family pairs pass on either side of ADR 0396 and are here for that reason — a
/// test whose three cases all failed would not have said that the *set* was inconsistent while
/// each family was fine.
#[test]
fn two_substituted_glyphs_in_one_clip_unite_rather_than_cancel() {
    for (first, second) in [
        // Both from the compiled-in sans, which is Liberation Sans in two weights.
        ("FSans", "FSansBold"),
        // Both from the compiled-in serif, which is two of Foxit's bare CFF programs.
        ("FSerif", "FSerifItalic"),
        // One of each, which is what `OverlappingGlyphClipping.pdf` does.
        ("FSans", "FSerif"),
    ] {
        let (alone, other) = (
            marked(&one(first, PLACES[0])),
            marked(&one(second, PLACES[1])),
        );
        let together = marked(&both(first, second));
        let union = combine(&alone, &other, |a, b| a || b);
        let overlap = count(&combine(&alone, &other, |a, b| a && b));
        let missing = count(&combine(&union, &together, |union, together| {
            union && !together
        }));

        assert!(
            count(&alone) > 0 && count(&other) > 0,
            "{first} and {second} must both draw, or this asserts nothing"
        );
        assert!(
            overlap > 1000,
            "{first} and {second} must overlap substantially, or a cancellation would be \
             invisible here — they share {overlap} pixels"
        );
        assert!(
            missing * 20 < overlap,
            "§9.3.6 combines the outlines under the non-zero winding number rule, so the \
             clip is the union of the two glyphs: {first} + {second} loses {missing} of the \
             union's {} pixels, where cancelling the {overlap}-pixel overlap would lose it all",
            count(&union),
        );
    }
}

/// The same glyph twice in one place is itself, not nothing.
///
/// The sharpest form of the rule and the one with no arithmetic in it: two identical outlines
/// wound the same way have every winding number doubled, so the non-zero rule paints the glyph;
/// wound opposite ways they cancel to nothing at all. A face that disagreed *with itself* would
/// show here, and this is also what a document gets when it accumulates one word twice.
#[test]
fn a_glyph_accumulated_twice_is_still_itself() {
    for font in ["FSans", "FSerif"] {
        let once = count(&marked(&one(font, PLACES[0])));
        let at = PLACES[0];
        let twice = count(&marked(&format!(
            "BT 7 Tr /{font} 150 Tf 1 0 0 1 {at} 30 Tm (B) Tj \
             1 0 0 1 {at} 30 Tm (B) Tj ET\n0 0 200 200 re f"
        )));

        assert!(once > 0, "{font} must draw");
        assert_eq!(twice, once, "{font} drawn twice in one clip");
    }
}
