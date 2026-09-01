//! Reference `XObject`s: ISO 32000-2 §8.10.4, and the permission this reader exercises.
//!
//! # Why this file exists
//!
//! §8.10.4.1 gives a reader two ways to be right about a form `XObject` carrying `/Ref`, and
//! this one takes the first:
//!
//! > PDF processors that do not recognise the Ref entry shall simply display or print the
//! > proxy as an ordinary form XObject.
//!
//! Nothing in `crates/`, `tools/` or `fuzz/` names `Ref` or a reference `XObject`, so "do not
//! recognise" is literally true of this tree rather than a decision taken at the entry — and
//! *that is the claim these tests hold*. It is a claim about an absence, which is the shape
//! that rots quietly: a later round adding a report, a refusal, or half an import would break
//! §8.10.4.1's sentence while every corpus gate stayed green, because **no document of the
//! curated 1251 nor of the 65 944-document `CC-MAIN-2021-31` crawl states a reference
//! `XObject`** — 67 195 of the 67 460 on this disk — (§8.10.4's row has the census, and *Why the fixtures are hand-built* below has
//! how it was calibrated).
//!
//! The three rows this file is evidence for named `tests/corpus.rs` — a *gate*'s file, which
//! passes for every document in the corpus and asserts nothing about `/Ref` at all. A file
//! passes whatever it contains; a test fails when the thing it is about stops being true.
//!
//! # Why the fixtures are hand-built
//!
//! `doc/traps/parsers-and-streams.md`'s trap 4 prefers real documents, and there are none.
//! `examples/absence_audit` asks §8.10.4.1's own condition — a form `XObject` whose form
//! dictionary holds a `/Ref` dictionary, the subtype included, which is what tells Table 93's
//! entry from Table 355's array on a `/TOCI` structure element — and finds **no witness in the
//! curated 1251 nor in the `SafeDocs` `CC-MAIN-2021-31` crawl's 65 944**: 67 195 of the 67 460
//! PDFs on this disk, the remainder being `corpus-cache/openpreserve`'s 267, which that example
//! has no scope for. The corpora are named because a claim of absence is refuted by one witness
//! and a widening is where witnesses arrive; the block was calibrated by pointing it at
//! `/Group`, which names 75 of `doc/pdf.js`'s documents, so the zero is a measurement rather
//! than a blind spot.
//!
//! The same reasoning §8.10.3's row records for `/Group << /S /Softness >>` applies — a
//! requirement no file exercises is held by a fixture or by nothing.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and this page is 100 units \
              square where no arithmetic can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

/// A two-page fixture: a containing page drawing a proxy, and a target page it names.
///
/// `reference` is the proxy form's `/Ref` entry, written whole so a test can leave it out —
/// which is what makes the pair of interpretations below differ in that entry and nothing
/// else. The proxy fills a 100-unit square while its `/BBox` is half that, so a reader that
/// stopped clipping it draws a different page; the target page fills itself blue and carries a
/// `Square` annotation whose appearance is green, which are §8.10.4.3's two subjects made
/// visible.
fn fixture(reference: &str) -> Vec<u8> {
    let proxy = "1 0 0 rg 0 0 100 100 re f";
    let target = "0 0 1 rg 0 0 100 100 re f";
    let appearance = "0 1 0 rg 0 0 30 30 re f";
    let page = "/Fm Do";

    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 50 50] {reference} \
         /Length {} >>\nstream\n{proxy}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 7 0 R /Annots [8 0 R] >>\nendobj\n\
         7 0 obj\n<< /Length {} >>\nstream\n{target}\nendstream\nendobj\n\
         8 0 obj\n<< /Type /Annot /Subtype /Square /Rect [60 60 90 90] /F 4 \
         /AP << /N 9 0 R >> >>\nendobj\n\
         9 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 30 30] /Length {} >>\n\
         stream\n{appearance}\nendstream\nendobj\n",
        page.len() + 1,
        proxy.len() + 1,
        target.len() + 1,
        appearance.len() + 1
    );

    assemble(&body)
}

/// Table 95's reference dictionary, filled in: the target document's file, and its page.
///
/// `/F` and `/Page` are the two the table makes *required*; `/ID` is optional and is here
/// because a reader that started following `/Ref` would use it, so a test of not following it
/// should state everything a follower needs.
const REF: &str = "/Ref << /F << /Type /Filespec /F (target.pdf) /UF (target.pdf) >> \
                   /Page 1 /ID [<0102> <0304>] >>";

/// Wraps a body of numbered objects in §7.5's header, cross-reference table and trailer.
fn assemble(body: &str) -> Vec<u8> {
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

/// Interprets the fixture's page at `index`.
fn interpret(bytes: &[u8], index: usize) -> pdf_model::Interpretation {
    let document = Document::open(bytes.to_vec()).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(index)
        .expect("the fixture's page");
    pdf_model::interpret(&document, &page)
}

/// The fixture rendered at one pixel per unit, as RGBA rows.
fn raster(interpretation: &pdf_model::Interpretation) -> Vec<[u8; 4]> {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 100x100 target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the fixture rasterises");
    raster
        .data
        .chunks_exact(4)
        .map(|bytes| [bytes[0], bytes[1], bytes[2], bytes[3]])
        .collect()
}

/// The RGBA pixel at device `(x, y)` of a 100-pixel-square raster.
fn pixel(interpretation: &pdf_model::Interpretation, x: usize, y: usize) -> [u8; 4] {
    raster(interpretation)[y * 100 + x]
}

/// §8.10.4.1's provision for a reader like this one, exercised and pinned.
///
/// > PDF processors that do not recognise the Ref entry shall simply display or print the
/// > proxy as an ordinary form XObject.
///
/// *Simply* and *ordinary* are the load-bearing words, and the strongest reading of them is
/// that the entry changes nothing: the same proxy with and without `/Ref` produces the same
/// display list, command for command. The pixels are asserted as well as the list because
/// "ordinary form `XObject`" carries §8.10.1's step c) with it — a proxy is clipped by its own
/// `/BBox` like any other form, and a reader that exempted one would draw past it.
///
/// Nothing is reported, and that is the clause's doing rather than an oversight
/// (`doc/traps/instruments-and-reports.md`'s trap 11): §8.10.4.1 states the alternative this
/// reader takes, so there is no gap to name. A report here would take every page holding a
/// proxy out of the oracle's comparison for a requirement the standard says is met.
#[test]
fn a_proxy_carrying_ref_is_drawn_as_an_ordinary_form_xobject() {
    let with = interpret(&fixture(REF), 0);
    let without = interpret(&fixture(""), 0);

    assert!(with.is_complete(), "{:?}", with.unsupported);
    assert!(without.is_complete(), "{:?}", without.unsupported);
    assert_eq!(
        with.display_list.commands(),
        without.display_list.commands(),
        "/Ref changed what the proxy draws"
    );

    // Device (10, 90) is page (10, 10), inside the `/BBox`; (60, 40) is page (60, 60), outside
    // it and inside the rectangle the proxy fills.
    assert_eq!(pixel(&with, 10, 90), [255, 0, 0, 255], "the proxy paints");
    assert_eq!(
        pixel(&with, 60, 40),
        [255, 255, 255, 255],
        "and is clipped by its own /BBox, as any form is"
    );
}

/// §8.10.4.3's two considerations have no imported page to be about.
///
/// Both are conditional on importing the target page — its annotations "shall be included in
/// the rendering of the imported page", and its logical structure "may be ignored" — and
/// neither arises while §8.10.4.1's proxy is what is drawn. That is an argument about an
/// absence, so what holds it is a fixture whose target page is *reachable*: page 1 of this
/// very file, filled blue, carrying a `Square` annotation whose appearance is green. Table
/// 95's `/F` and `/Page` both name it.
///
/// Neither colour may appear anywhere on the containing page. The test fails on the day a
/// round imports the target — which is the point: the row it is evidence for says the two
/// considerations do not arise, and an import that leaves this test standing has either
/// carried the annotation appearances (§8.10.4.3's first consideration) or drawn nothing.
#[test]
fn no_content_of_the_target_page_reaches_the_containing_page() {
    let containing = raster(&interpret(&fixture(REF), 0));

    // The target's own colours, as it and its annotation state them. Read off the fixture
    // rather than off a render: a match at *any* pixel is the failure, so an exact equality is
    // what discriminates.
    let filled = [0, 0, 255, 255];
    let annotated = [0, 255, 0, 255];
    assert!(
        !containing.contains(&filled),
        "the target page's own fill reached the containing page"
    );
    assert!(
        !containing.contains(&annotated),
        "the target page's annotation appearance reached the containing page"
    );

    // And the fixture is not vacuous: the target page draws both when it is the page being
    // interpreted, so the absences above are this reader declining to import rather than a
    // document that states nothing.
    let target = raster(&interpret(&fixture(REF), 1));
    assert!(target.contains(&filled), "the target page states a fill");
    assert!(
        target.contains(&annotated),
        "the target page states an annotation appearance"
    );
}
