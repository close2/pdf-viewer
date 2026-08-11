//! A path whose device coordinates leave the scan converter's arithmetic (ISO 32000-2 §10.7).
//!
//! §10.7 leaves scan conversion to the device and bounds no coordinate, and §7.3.3 hands the
//! range of a number to the implementation — Annex C, which it points at for the figures, is
//! informative and states none for a coordinate. So a conforming file may state a path of any
//! size and a damaged one certainly does. Two of the 65 944 crawled documents the
//! four-hundred-and-thirty-third session surveyed state fills reaching 10²⁵ device units on a
//! 368 × 542 page, and both **aborted the process**: `tiny-skia`'s anti-aliased blitter works
//! in supersampled 16.16 fixed point, walks its run buffer past the end when a path leaves
//! that range, and unwraps a `None`. Under `[profile.release]`'s `panic = "abort"` that is the
//! whole program rather than one page.
//!
//! The witnesses are named rather than committed — corpus `cc-main-2021-31`, archive `0300`,
//! `0300856.pdf`, SHA-256 `09bafa06b40c6cfab5bfca03ac5e819efab3adec6d8ebdc52408b9cca23a7b1b`;
//! archive `4605`, `4605705.pdf`, SHA-256
//! `94a3b4969b1f16611801bc27c03ceb3ace5b73bfa1a18f9e0f061a4e5681f5c0` — and the fixture below
//! is *generated*, so this costs `doc/todo/03`'s
//! promotion budget nothing. `CLAUDE.md` requires a crasher to become a permanent regression
//! test and this is it. ADR 0269, `crates/render-cpu/src/scan.rs`.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// A one-page PDF whose only content is `operators`.
fn page_drawing(operators: &str) -> Document {
    let length = operators.len();
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 368 542] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {length} >>\nstream\n{operators}\nendstream\nendobj\n"
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// Interprets page one and rasterises it, returning how many bytes of the raster are not zero.
fn drawn(document: &Document) -> usize {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .expect("the fixture has a page");
    let interpretation = pdf_model::interpret(document, &page);
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 64 << 20)
        .expect("a 368x542 page is inside the pixel budget");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("the page rasterises");
    raster.data.iter().filter(|&&byte| byte != 0).count()
}

/// The shape both witnesses state: a triangle with one corner on the page and one at 10⁷.
///
/// Before the four-hundred-and-thirty-third session this aborted the process. It is asserted
/// to *draw* rather than merely to return, because a guard that dropped the command would
/// pass a test that only asked for survival — and `CLAUDE.md`'s trap 5 is that unsupported
/// input must stay loud, not that it may quietly vanish.
#[test]
fn a_fill_reaching_ten_million_units_is_drawn_rather_than_aborting() {
    let document = page_drawing("0 0 0 rg\n10 10 m 10000000 2000000000 l 30 10 l f\n");
    assert!(
        drawn(&document) > 0,
        "the part of the spike that crosses the page is marked"
    );
}

/// The same shape stroked, which reaches the scan converter through a different call.
#[test]
fn a_stroke_reaching_ten_million_units_is_drawn_rather_than_aborting() {
    let document = page_drawing("0 0 0 RG\n4 w\n10 10 m 10000000 2000000000 l 30 10 l S\n");
    assert!(
        drawn(&document) > 0,
        "the part of the stroked spike that crosses the page is marked"
    );
}

/// The same shape as a clip, which reaches it through `Mask::fill_path` instead.
///
/// The rectangle drawn afterwards is what proves the clip was built at all: a mask that came
/// back empty would leave the page blank and the assertion would say so.
#[test]
fn a_clip_reaching_ten_million_units_is_built_rather_than_aborting() {
    let document = page_drawing(
        "q\n10 10 m 10000000 2000000000 l 30 10 l W n\n0 0 0 rg\n0 0 368 542 re f\nQ\n",
    );
    assert!(
        drawn(&document) > 0,
        "the part of the page the clip admits is marked"
    );
}

/// A page whose geometry stays small keeps drawing what it always did.
///
/// The control: without it, a guard that turned every page blank would pass the three above.
#[test]
fn an_ordinary_fill_is_unaffected() {
    let document = page_drawing("0 0 0 rg\n10 10 100 100 re f\n");
    assert!(
        drawn(&document) > 10_000,
        "a 100x100 black square marks at least ten thousand bytes"
    );
}
