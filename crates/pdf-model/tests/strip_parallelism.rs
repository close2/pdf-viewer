//! What a *real page* drawn in strips owes the page drawn in one piece.
//!
//! `render-cpu` has this property as a test already — `render-cpu/tests/strip_parallelism.rs`,
//! ADR 0139 — and that one draws the six `test-scenes` fixtures at every division and demands
//! the bytes be equal. It passed for two hundred and twenty-six sessions while the property was
//! false, because a fixture is a dozen shapes at round coordinates and the departure needs a
//! mark whose device position lands within an `ulp` of a supersample row. Trap 12b: a suite of
//! small scenes tests small scenes.
//!
//! So this is the same question asked of the thing the property is claimed *for* — a page
//! interpreted from a document in this repository, at the width a window would fit it to. It
//! lives here rather than beside its sibling because building one costs a parser, an
//! interpreter and a font stack, which is this crate.
//!
//! # What it asserts, and why it is three things rather than one
//!
//! ADR 0219 established that byte-for-byte equality is **not achievable** and said why: this
//! backend now hands a strip exactly the matrix it hands the whole page with a whole number of
//! rows subtracted, and `tiny-skia` still maps a point as `y·sy + ty`, where subtracting an
//! integer from `ty` changes the magnitude the sum rounds at. So what is left to assert is what
//! that residual is allowed to look like:
//!
//! 1. **The page the defect was found on is exact.** `PDF20_AN001-BPC.pdf` page 1 at 500 pixels
//!    wide differed at (411, 659) — 95 whole against 79 split — and does not any more.
//! 2. **No pixel may move by more than one supersample.** `tiny-skia` averages sixteen samples,
//!    so an edge that lands on the other side of one sample row is worth 16 of 255. A *chopped*
//!    path — ADR 0138's defect, and the one `unsplittable_rows` exists to prevent — reached 32,
//!    48 and 64, and a mark drawn in the wrong place is worth everything. This is the assertion
//!    that guards the geometry rule, and it fails on the code ADR 0219 replaced.
//! 3. **Barely any pixel may move at all.** One in ten thousand, against a measured worst of
//!    one in eighty thousand: enough headroom for a font or an antialiasing change to move the
//!    handful of edges that sit on a sample row, and nowhere near enough for anything
//!    systematic. This is the half of the guard that catches a *small* chop, which assertion 2
//!    cannot: ADR 0138's cheapest scene moved its worst byte by only 16 — and it moved 247
//!    pixels doing it, because re-parameterising an edge moves the whole edge.

#![expect(
    clippy::expect_used,
    reason = "test code: a document or a page this repository commits must be readable, and a \
              rasteriser that refuses one of its pages should fail loudly"
)]

use pdf_render::{DisplayList, Rasterizer as _, TargetSpec};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything these pages request.
const GENEROUS: u64 = 1 << 30;

/// Divisions asked for, from two to more than any machine offers.
///
/// The planner grants fewer where the page's curves forbid the cuts, and fewer again where a
/// strip would fall below its minimum height — which is the point: whatever it grants, the page
/// is the same page.
const DIVISIONS: [u32; 9] = [2, 3, 4, 5, 8, 12, 16, 24, 32];

/// Most a pixel may move: one of `tiny-skia`'s sixteen supersamples, 16 of 255.
const SUPERSAMPLE: u8 = 16;

/// Most pixels that may move at all, as one in this many.
const RARE: usize = 10_000;

/// Pages, each with the width in pixels a window would fit it to, and whether it must be exact.
///
/// The first is the counter-example ADR 0219 was written for, and it is exact. The other two are
/// the pages ADR 0139 measured its split on: one page-wide clip that forbids nearly every cut,
/// and a dense text page that grants nearly half its rows — the two ends of what the planner
/// does, and both of them hold a few edges that sit on a sample row.
const PAGES: [(&str, usize, u32, bool); 3] = [
    ("PDF20_AN001-BPC.pdf", 0, 500, true),
    ("ISO_32000-2_sponsored_EC3.pdf", 5, 1192, false),
    ("ISO_32000-2_sponsored_EC3.pdf", 100, 800, false),
];

/// The display list and target for one page of one committed document.
fn page(file: &str, index: usize, width: u32) -> (DisplayList, TargetSpec) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(file);
    let bytes = std::fs::read(&path).expect("a document this repository commits");
    let document = pdf_syntax::Document::open(bytes).expect("a valid document");
    let page = pdf_model::Pages::new(&document)
        .get(index)
        .expect("the page exists");
    let list = pdf_model::interpret(&document, &page).display_list;
    // The scale a window `width` pixels wide fits the page at, taken from the page's own target
    // rather than from its media box so that a rotated or cropped page fits too.
    let unscaled = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("a valid target");
    #[expect(
        clippy::cast_precision_loss,
        reason = "two target widths under 2^24, where every integer is exact in f32"
    )]
    let scale = width as f32 / unscaled.width as f32;
    let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("a valid target");
    (list, target)
}

/// One page's pixels, drawn in the number of strips asked for.
fn drawn(list: &DisplayList, target: TargetSpec, strips: u32) -> Vec<u8> {
    CpuRasterizer::new()
        .with_strips(strips)
        .rasterize(list, target)
        .expect("the CPU backend draws this page")
        .data
}

/// Every page, at every division, within what ADR 0219 leaves of the property.
#[test]
fn a_real_page_drawn_in_strips_is_the_page_drawn_whole() {
    for (file, index, width, exact) in PAGES {
        let (list, target) = page(file, index, width);
        let whole = drawn(&list, target, 1);
        let pixels = (target.width as usize).saturating_mul(target.height as usize);

        for strips in DIVISIONS {
            let split = drawn(&list, target, strips);
            let moved: Vec<usize> = whole
                .chunks_exact(4)
                .zip(split.chunks_exact(4))
                .enumerate()
                .filter_map(|(at, (ours, theirs))| (ours != theirs).then_some(at))
                .collect();
            let worst = whole
                .iter()
                .zip(&split)
                .map(|(ours, theirs)| ours.abs_diff(*theirs))
                .max()
                .unwrap_or(0);
            // Where the first one is, not the count alone: a failure here is a handful of
            // pixels in eight megabytes, and where they are is the whole of the diagnosis.
            let first = moved.first().copied().unwrap_or(0);
            let where_and_what = format!(
                "{file} page {} at {}x{} in {strips} strips: {} pixels moved, worst {worst}, \
                 first at ({}, {}) — {:?} whole against {:?} split",
                index.saturating_add(1),
                target.width,
                target.height,
                moved.len(),
                first % target.width as usize,
                first / target.width as usize,
                &whole[first.saturating_mul(4)..first.saturating_mul(4).saturating_add(4)],
                &split[first.saturating_mul(4)..first.saturating_mul(4).saturating_add(4)],
            );

            if exact {
                assert!(moved.is_empty(), "{where_and_what}");
            }
            assert!(
                worst <= SUPERSAMPLE,
                "{where_and_what}\na pixel moved by more than one supersample, which is a \
                 chopped path rather than a rounded one — see ADR 0138 and \
                 `pdf_render::unsplittable_rows`",
            );
            assert!(
                moved.len().saturating_mul(RARE) <= pixels,
                "{where_and_what}\nmore than one pixel in {RARE} moved, which is more than \
                 `tiny-skia`'s arithmetic at a shifted origin can account for — see ADR 0219",
            );
        }
    }
}
