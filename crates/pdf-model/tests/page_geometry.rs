//! What a page *is*, before anything is drawn on it: ISO 32000-2 §7.7.3.
//!
//! Four numbers and two entries decide the size, orientation and origin of every page, and
//! getting any of them wrong moves the whole picture rather than one object on it. The
//! oracle catches that as a `GEOMETRY` verdict — "the comparison cannot even proceed" — which
//! is a blunt instrument: it says the page is the wrong size and nothing about which entry
//! made it so. This file states each rule on its own.
//!
//! **`/UserUnit` is the one that had never been read**, and the twenty-ninth session found
//! that all three of the oracle's geometry disagreements were it. Two documents carried
//! `/UserUnit 3` and came out a third of the size two references produced; the third,
//! recorded in `oracle.rs` as "the reverse case … has not been looked into", writes
//! `/MediaBox [0 0 8.5 11]` with `/UserUnit 72` — a page stated in **inches**.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::float_cmp,
    reason = "test code: a malformed fixture should fail loudly, the fixtures are small pages \
              where no index can overflow, and a page boundary is read from the file's own \
              decimal literals and arrives exactly"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the small pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// A one-page PDF with the given extra entries on the page object, drawing one black square
/// in the bottom-left corner of user space.
fn page_with(entries: &str) -> Vec<u8> {
    let content = "0 g 0 0 10 10 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] {entries} \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len().saturating_add(1)
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
    out.into_bytes()
}

/// Renders page one at one device pixel per seventy-second of an inch.
fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS)
        .expect("a small page is a valid target");
    CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("the display list holds nothing the CPU backend refuses")
}

/// Whether the pixel at `(x, y)` carries the fixture's black square.
///
/// Darkness rather than alpha: §11.4.7 makes the page an isolated group imposed on the
/// medium, so every pixel of a finished render is opaque and an alpha test would say the
/// whole page is marked.
fn marked(raster: &pdf_render::Raster, x: u32, y: u32) -> bool {
    let at = ((y * raster.width + x) * 4) as usize;
    raster.data[at] < 128
}

/// §7.7.3.3 Table 31's `/UserUnit` scales the page and everything on it.
///
/// > A positive number that shall give the size of default user space units, in multiples of
/// > 1 ⁄ 72 inch. The range of supported values shall be implementation-dependent. Default
/// > value: 1.0 (user space unit is 1 ⁄ 72 inch).
///
/// A device asked to draw at a resolution is asked in inches, so a unit that is three
/// seventy-seconds of an inch makes the page three times as large in device pixels and
/// everything on it with it. That is what `mutool` and `ghostscript` do; `poppler` does not,
/// and the clause is what settles it rather than the vote.
#[test]
fn a_user_unit_scales_the_page_and_its_contents() {
    let plain = render(page_with(""));
    assert_eq!((plain.width, plain.height), (100, 50));
    assert!(marked(&plain, 5, 45), "the square is at the bottom left");
    assert!(!marked(&plain, 25, 45), "and is ten units across");

    let scaled = render(page_with("/UserUnit 3"));
    assert_eq!(
        (scaled.width, scaled.height),
        (300, 150),
        "three seventy-seconds of an inch per unit is three times the page"
    );
    assert!(
        marked(&scaled, 25, 145),
        "the square scales with the page — 10 units is now 30 pixels"
    );
    assert!(!marked(&scaled, 35, 145), "and stops where 30 pixels stop");
}

/// A page whose `/MediaBox` is stated in inches, which is what `/UserUnit 72` means.
///
/// `issue19176.pdf` is this fixture: `[0 0 8.5 11]` with `/UserUnit 72` is US Letter. It sat
/// on the oracle's geometry list under a comment calling it "the reverse case" of the two
/// documents that scale *up*, and it is the same entry read the same way.
#[test]
fn a_page_stated_in_inches_is_the_size_those_inches_are() {
    let document = Document::open(page_with_media_box("[0 0 8.5 11]", "/UserUnit 72"))
        .expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let list = pdf_model::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new().rasterize(&list, target).expect("ok");
    assert_eq!((raster.width, raster.height), (612, 792), "US Letter");
}

/// The entry must be positive, and a value that is not is the default rather than a refusal.
///
/// Table 31 says "a positive number". Zero would collapse the page to nothing and a negative
/// one would turn it inside out; both are malformed rather than meaningful, and §7.7.3.3
/// states no recovery, so the default of 1.0 is this reader's documented choice.
#[test]
fn a_user_unit_that_is_not_positive_is_the_default() {
    for stated in ["/UserUnit 0", "/UserUnit -3", "/UserUnit (three)"] {
        let raster = render(page_with(stated));
        assert_eq!(
            (raster.width, raster.height),
            (100, 50),
            "{stated} must leave the page its stated size"
        );
    }
}

/// `/UserUnit` is **not** inheritable, and §7.7.3.3 is what says so.
///
/// > Attributes that are not explicitly identified in the table as inheritable shall not be
/// > inherited.
///
/// §7.7.3.4 describes how the inheritable ones are found; that sentence, which closes the
/// question for every entry the table does not mark, is one clause earlier. Four entries in
/// Table 31 are marked inheritable — `/Resources`, `/MediaBox`, `/CropBox`
/// and `/Rotate` — and `/UserUnit` sits among a dozen that are not. Reading it through the
/// same inheritance the media box uses is the obvious mistake and would scale pages that
/// state nothing.
#[test]
fn a_user_unit_on_an_ancestor_is_not_inherited() {
    let content = "0 g 0 0 10 10 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /UserUnit 3 /MediaBox [0 0 100 50] \
         /Rotate 90 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len().saturating_add(1)
    );
    let raster = render(assemble(&body));
    // `/MediaBox` and `/Rotate` are inheritable, so the page is 100x50 turned on its side.
    assert_eq!(
        (raster.width, raster.height),
        (50, 100),
        "the inheritable entries apply and /UserUnit does not"
    );
}

/// The same builder, with the media box spelled out.
fn page_with_media_box(media_box: &str, entries: &str) -> Vec<u8> {
    let content = "0 g 0 0 10 10 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox {media_box} {entries} \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len().saturating_add(1)
    );
    assemble(&body)
}

/// Wraps numbered objects in a header, a cross-reference table and a trailer.
fn assemble(body: &str) -> Vec<u8> {
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
    out.into_bytes()
}

/// A one-page PDF with a catalog entry as well as page entries, filling the whole medium.
///
/// The content is a 100×50 black rectangle covering the media box, so what any test of a
/// boundary asks is *where the ink stops* — which is the only observable difference between
/// displaying one boundary and clipping to another.
fn page_and_catalog(catalog: &str, entries: &str) -> Vec<u8> {
    let content = "0 g 0 0 100 50 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {catalog} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] {entries} \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len().saturating_add(1)
    );
    assemble(&body)
}

/// §14.11.2.1's five boundaries, with the clause's defaults and its intersection rule.
///
/// The three production boxes default to the crop box, are read from the page object alone
/// (§7.7.3.4 makes only four entries inheritable, and none of these is one), and are clipped
/// to the medium: "[i]f the bounds of the crop, trim, bleed or art box extends outside of the
/// bounds of the media box, a processor shall treat the box as its intersection with the
/// media box."
#[test]
fn the_five_page_boundaries_default_and_are_clipped_to_the_medium() {
    use pdf_model::page::Boundary;

    let bytes = page_and_catalog(
        "",
        "/CropBox [10 5 90 45] /BleedBox [0 0 200 200] /TrimBox [20 10 80 40]",
    );
    let document = Document::open(bytes).expect("a valid fixture");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");

    assert_eq!(page.boundary(Boundary::Media), [0.0, 0.0, 100.0, 50.0]);
    assert_eq!(page.boundary(Boundary::Crop), [10.0, 5.0, 90.0, 45.0]);
    assert_eq!(
        page.boundary(Boundary::Bleed),
        [0.0, 0.0, 100.0, 50.0],
        "a bleed box past the medium is its intersection with it"
    );
    assert_eq!(page.boundary(Boundary::Trim), [20.0, 10.0, 80.0, 40.0]);
    assert_eq!(
        page.boundary(Boundary::Art),
        page.boundary(Boundary::Crop),
        "an absent art box is the crop box"
    );
    assert_eq!(
        page.display_box, page.crop_box,
        "with no /ViewArea, the displayed region is the crop box"
    );
}

/// §12.2's `/ViewArea` decides which boundary is displayed, and `/ViewClip` what is clipped.
///
/// Table 147 states them as two questions: `/ViewArea` is "the name of the page boundary
/// representing the area of a page that shall be displayed when viewing the document on the
/// screen", `/ViewClip` "the name of the page boundary to which the contents of a page shall
/// be clipped". So a document may show the medium and still clip the ink to its trim box, and
/// the margin between the two is blank rather than absent — which is what this measures: the
/// raster is the media box's 100×50, and the ink stops at the trim box.
///
/// Both entries are deprecated in PDF 2.0 and both are read, for the reason §8.6.5.1's
/// withdrawn `CalCMYK` is: deprecation tells a *writer* what to stop doing.
#[test]
fn view_area_and_view_clip_choose_what_is_displayed_and_what_is_clipped() {
    let bytes = page_and_catalog(
        "/ViewerPreferences << /ViewArea /MediaBox /ViewClip /TrimBox >>",
        "/CropBox [10 5 90 45] /TrimBox [20 10 80 40]",
    );
    let raster = render(bytes);
    assert_eq!(
        (raster.width, raster.height),
        (100, 50),
        "the media box is displayed, not the crop box"
    );
    assert!(
        marked(&raster, 50, 25),
        "the trim box's interior is painted"
    );
    assert!(
        !marked(&raster, 5, 25),
        "and a point inside the medium but outside the trim box is not"
    );
    assert!(
        !marked(&raster, 50, 45),
        "which holds on the other axis too"
    );
}

/// With no preference, the crop box is both, which is Table 147's own default.
///
/// The point of the assertion is that the feature costs nothing when it is not asked for:
/// every corpus document takes this path, and a display list with a clip in it would be a
/// different display list.
#[test]
fn no_view_preference_displays_the_crop_box_and_clips_to_nothing_else() {
    let bytes = page_and_catalog("", "/CropBox [10 5 90 45] /TrimBox [20 10 80 40]");
    let raster = render(bytes);
    assert_eq!((raster.width, raster.height), (80, 40));
    assert!(marked(&raster, 5, 20), "the crop box's interior is painted");
    assert!(marked(&raster, 75, 20), "all of it, trim box or no");
}

/// ISO 32000-2 §7.7.3.1: the tree may be any shape, and the walk must not assume one.
///
/// > Compliant PDF processors shall be prepared to handle any form of tree structure built of
/// > such nodes.
///
/// The tree below is deliberately none of the shapes the clause's NOTE describes: it is
/// unbalanced, its depths differ between branches, one intermediate node omits `/Type`
/// (§7.7.3.2's Table 30 requires it and real files leave it out, so the walk decides a leaf by
/// the absence of `/Kids`), and one node states a `/Count` that is wrong. What the pages'
/// *order* has to be is the order the `/Kids` arrays give, depth first, which is the only thing
/// that makes a page number mean anything — so each leaf carries a media box of its own width
/// and the assertion reads them back as a sequence.
#[test]
fn a_page_tree_of_any_shape_yields_its_pages_in_the_order_its_kids_arrays_give() {
    // 2 ─┬─ 3 (a page, width 10)
    //    ├─ 4 ─┬─ 5 (a page, width 20)
    //    │     └─ 6 ─── 7 (a page, width 30)   ← a node with no /Type
    //    └─ 8 (a page, width 40)
    let page = |number: u32, parent: u32, width: u32| {
        format!(
            "{number} 0 obj\n<< /Type /Page /Parent {parent} 0 R \
             /MediaBox [0 0 {width} 10] /Resources << >> >>\nendobj\n"
        )
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R 8 0 R] /Count 4 >>\nendobj\n\
         {}\
         4 0 obj\n<< /Type /Pages /Parent 2 0 R /Kids [5 0 R 6 0 R] /Count 99 >>\nendobj\n\
         {}\
         6 0 obj\n<< /Parent 4 0 R /Kids [7 0 R] >>\nendobj\n\
         {}{}",
        page(3, 2, 10),
        page(5, 4, 20),
        page(7, 6, 30),
        page(8, 2, 40)
    );

    let document = Document::open(assemble(&body)).expect("a valid PDF");
    let pages = pdf_model::Pages::new(&document);
    assert_eq!(
        pages.len(),
        4,
        "four leaves, whatever the /Count entries say"
    );

    let widths: Vec<u32> = (0..4)
        .map(|index| {
            let page = pages.get(index).expect("each of the four leaves");
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "test code: every media box here is a small literal"
            )]
            let width = page.media_box[2] as u32;
            width
        })
        .collect();
    assert_eq!(
        widths,
        vec![10, 20, 30, 40],
        "depth first, in each node's own /Kids order"
    );
}
