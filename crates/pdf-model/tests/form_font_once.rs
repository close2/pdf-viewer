//! A form's `/DA` font is loaded once for §12.7.4.3's layout and the `Tf` of the appearance the
//! layout writes, out of the interpreter's own font cache (ADR 1974).
//!
//! The fixture is one page with one text field whose value has no appearance stream, so the
//! interpreter lays it out under the form's `/DA` and draws the stream it wrote; `/DR` names the
//! font by reference, which is the object the stream's own `/Resources` names it by. What the
//! cache says it was asked, and answered, is the measurement.

#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;

use pdf_model::content::{FontCache, FontCacheReport, interpret_with_fonts};
use pdf_model::view::ViewState;
use pdf_syntax::Document;

/// A document of `objects`, numbered from 1, the first the catalog.
fn assembled(objects: &[&str]) -> Document {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// One page, one text field with a value and no `/AP`, and Courier as object 5 under `/DR`.
fn form() -> Document {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /DA (/Cour 0 Tf 0 g) \
         /DR << /Font << /Cour 5 0 R >> >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Field) /V (Set in Courier) /F 4 \
         /Rect [10 100 210 130] /P 3 0 R >>",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>",
    ])
}

/// What `fonts` says after page one of `document` is interpreted once through it.
fn interpreted(document: &Document, fonts: &FontCache) -> FontCacheReport {
    let pages = pdf_model::Pages::new(document);
    let page = pages.get(0).expect("the fixture has one page");
    let drawn = interpret_with_fonts(document, &page, &ViewState::of(document), fonts);
    assert!(
        !drawn.display_list.commands().is_empty(),
        "the field's value is drawn: {:?}",
        drawn.unsupported
    );
    fonts.report()
}

/// The layout loads Courier and keeps it; the appearance's `Tf` is answered from the cache.
///
/// Without the layout asking the cache, the layout's load is its own and the `Tf` misses and
/// loads Courier a second time: `hits` is 0.
#[test]
fn the_layout_and_the_appearance_s_tf_load_the_font_once() {
    let document = form();
    let fonts = FontCache::new();
    let report = interpreted(&document, &fonts);
    assert_eq!(
        (report.fonts, report.misses, report.hits),
        (1, 1, 1),
        "{report:?}"
    );
}

/// A cache that outlives the interpretation, as an open document's does, answers the next
/// interpretation's layout and `Tf` both, and nothing is loaded again.
#[test]
fn a_second_interpretation_loads_no_font() {
    let document = form();
    let fonts = FontCache::new();
    let first = interpreted(&document, &fonts);
    let second = interpreted(&document, &fonts);
    assert_eq!(second.misses, first.misses, "{first:?} then {second:?}");
    assert_eq!(
        second.hits,
        first.hits.saturating_add(2),
        "{first:?} then {second:?}"
    );
}
