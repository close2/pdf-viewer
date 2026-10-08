//! What a *real page* drawn in strips owes the page drawn in one piece.
//!
//! `render-cpu` has this property as a test already — `render-cpu/tests/strip_parallelism.rs`,
//! ADR 0139 — and that one draws the six `test-scenes` fixtures at every division and demands
//! the bytes be equal. It passes while the property is false, because a fixture is a dozen
//! shapes at round coordinates and the departure needs a
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
//! 1. **No page is held to byte equality, and the reason is the paragraph above.** This item read
//!    "the page the defect was found on is exact" and named `PDF20_AN001-BPC.pdf` page 1 at 500
//!    pixels wide, which differed at (411, 659) — 95 whole against 79 split — before ADR 0219.
//!    That page was exact for as long as the oracle measured a coverage to one of `tiny-skia`'s
//!    sixteen supersamples, which is a quantum wide enough to swallow an `ulp` in `ty`. It stopped
//!    being exact when the oracle stopped having one (ADR 1082): `render_cpu::area` computes the
//!    area a shape covers, so an `ulp` of difference in where the shape *is* now shows as a level
//!    of difference in what the pixel gets, and the page moves **3 pixels of 353 500 by one level**
//!    — one in 117 833, against assertion 3's one in ten thousand. A converter that is byte-exact
//!    under a shifted origin is one with somewhere to hide the shift, which is what the module
//!    comment above says is not achievable. Assertions 2 and 3 are what guard ADR 0138's defect,
//!    and both fail on the code ADR 0219 replaced: its residual was 16 levels and 247 pixels.
//! 2. **No pixel may move by more than one level of 255**, and that bound is derived rather than
//!    borrowed from a converter. An `ulp` of `ty` at these magnitudes is about `6 × 10⁻⁵` of a
//!    device pixel; an exact converter turns a position into an area, so it can move a boundary
//!    pixel's coverage by that much and no more — which crosses a rounding step only where the
//!    value already sat within `6 × 10⁻⁵` of one, and can never cross two. Measured over all 27
//!    cases below, the worst is **1**. That is a bound on **one coverage**, and a pixel is not
//!    always one: where it takes the rounded results of two marks' edges, or blends a source
//!    through a function steeper than one, the corpus's first pages move two levels and three, so
//!    `raster_golden.rs` holds all 967 of them to this file's two bounds or to a ceiling it names
//!    page by page (ADR 1758). Where the pixel is a blend, the bound still holds of each sample it
//!    blends, and `a_blended_photograph_moves_only_where_one_of_its_samples_moved` holds it there
//!    (ADR 1768). A *chopped* path — ADR 0138's defect, and the one
//!    `unsplittable_rows` exists to prevent — reached 16, 32, 48 and 64, and a mark drawn in the
//!    wrong place is worth everything, so this bound is sixteen times tighter than the one it
//!    replaces and catches every scene that defect produced.
//! 3. **Barely any pixel may move at all.** One in a thousand, against a measured worst of one in
//!    3 986. The measured figure rose as assertion 2's bound fell, and for the same reason: a
//!    converter with a quantum has somewhere to hide an `ulp` and one that computes an area does
//!    not, so more boundary pixels show the shift and each shows less of it. This is no longer
//!    what catches a chop — assertion 2 does that alone now — and what it guards is anything
//!    *systematic*, which a shifted origin is not.

#![expect(
    clippy::expect_used,
    reason = "test code: a document or a page this repository commits must be readable, and a \
              rasteriser that refuses one of its pages, or a photograph that moves past its \
              bound, should fail loudly"
)]

use pdf_render::{BlendMode, Command, DisplayList, Rasterizer as _, TargetSpec};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything these pages request.
const GENEROUS: u64 = 1 << 30;

/// Divisions asked for, from two to more than any machine offers.
///
/// The planner grants fewer where the page's curves forbid the cuts, and fewer again where a
/// strip would fall below its minimum height — which is the point: whatever it grants, the page
/// is the same page.
const DIVISIONS: [u32; 9] = [2, 3, 4, 5, 8, 12, 16, 24, 32];

/// Most a pixel may move: one level of 255, which is what an `ulp` of `ty` can buy an exact
/// converter — see the module comment's second item.
const ONE_LEVEL: u8 = 1;

/// Most pixels that may move at all, as one in this many.
const RARE: usize = 1_000;

/// Pages, each with the width in pixels a window would fit it to, and whether it must be exact.
///
/// The first is the counter-example ADR 0219 was written for. The other two are the pages ADR 0139
/// measured its split on: one page-wide clip that forbids nearly every cut, and a dense text page
/// that grants nearly half its rows — the two ends of what the planner does, and both of them hold
/// a few edges that sit on a sample row. **None of the three is exact**, and the module comment's
/// first item is why the first one stopped being.
const PAGES: [(&str, usize, u32, bool); 3] = [
    ("PDF20_AN001-BPC.pdf", 0, 500, false),
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
                worst <= ONE_LEVEL,
                "{where_and_what}\na pixel moved by more than one level, which is more than an \
                 `ulp` of `ty` can buy an exact converter — a chopped path rather than a rounded \
                 one, see ADR 0138 and `pdf_render::unsplittable_rows`",
            );
            assert!(
                moved.len().saturating_mul(RARE) <= pixels,
                "{where_and_what}\nmore than one pixel in {RARE} moved, which is more than this \
                 backend's arithmetic at a shifted origin can account for — see ADR 0219",
            );
        }
    }
}

/// The first page `raster_golden.rs` names past the division bound: sixteen cells, each one
/// photograph drawn over another under one of ISO 32000-2 §11.3.5's sixteen blend modes.
const BLENDED: &str = "pdf.js/test/pdfs/blendmode.pdf";

/// The divisions `raster_golden.rs` measures every first page at.
const GOLDEN_DIVISIONS: [u32; 4] = [2, 4, 8, 16];

/// Each photograph of a cell drawn alone moves by at most one level when the page is divided, and
/// in the four cells this backend composites itself no pixel moves unless one of its two samples
/// moved — so what `raster_golden.rs` holds of this page as a ceiling is the blend of a one-level
/// sample, and nothing else (ADR 1768).
///
/// # The two halves, and why each is the one it is
///
/// - **The inputs are held to this file's derived bound.** Each image is drawn with the other image
///   of its cell made transparent — its alpha set to zero, so that the list keeps every command and
///   the strip planner cuts the page where it cuts the page itself. A strip hands `tiny-skia` the
///   page's matrix with whole rows subtracted (ADR 0219), so a sample's position moves by less than
///   an `ulp` and its filtered value by less than a level: one rounding step at most, the bound the
///   module comment derives for one coverage. The source is drawn under Normal, over the page's
///   white, so what is held is the sample and not a blend of it.
/// - **The output is held to its inputs where both reach the blend as eight-bit rasters.** Table
///   135's four modes are composited by `render_cpu`'s own `blend::composite` from a layer the
///   source was drawn into and the destination, both rounded to eight bits first, so a composed
///   pixel that moved where neither input moved would be a second mechanism. The other twelve are
///   `tiny-skia`'s, which blends the source before it is rounded, so a move of less than a level in
///   the source reaches the output without showing in the source drawn alone; for those the first
///   half is the whole of what is checkable.
///
/// What the blend makes of a one-level move is the blend function's own slope at that pixel, and
/// that has no bound to state: Table 134's `ColorDodge` divides the backdrop by `1 − Cs`, a slope
/// of three at a source of 177, and Table 135's functions are discontinuous where a colour is
/// neutral. ADR 1768 has the ladder of offsets that measures it; this test holds the half of the
/// argument that is derived.
#[test]
fn a_blended_photograph_moves_only_where_one_of_its_samples_moved() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(BLENDED);
    let bytes = std::fs::read(&path).expect("the pdf.js corpus is checked out");
    let document = pdf_syntax::Document::open(bytes).expect("a valid document");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the page exists");
    let list = pdf_model::interpret(&document, &page).display_list;
    // One pixel per unit, the scale `raster_golden.rs` measured the page at.
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("a valid target");

    let cells = cells_of(&list, target);
    assert_eq!(
        cells.len(),
        16,
        "blendmode.pdf draws one cell per blend mode, each a backdrop and a source"
    );
    let backdrops = with_images(&list, &|index, image| {
        if index % 2 == 1 {
            hide(image);
        }
    });
    let sources = with_images(&list, &|index, image| {
        if index % 2 == 0 {
            hide(image);
        } else {
            normal(image);
        }
    });

    let whole = [&list, &backdrops, &sources].map(|list| drawn(list, target, 1));
    for strips in GOLDEN_DIVISIONS {
        let split = [&list, &backdrops, &sources].map(|list| drawn(list, target, strips));
        for cell in &cells {
            let within = |which: usize| -> Vec<(u32, u32, u8)> {
                moved_within(&whole[which], &split[which], target.width, cell)
            };
            let (composed, backdrop, source) = (within(0), within(1), within(2));
            for (input, name) in [(&backdrop, "backdrop"), (&source, "source")] {
                if let Some((x, y, levels)) =
                    input.iter().copied().find(|moved| moved.2 > ONE_LEVEL)
                {
                    panic!(
                        "the {name} of the {:?} cell, drawn alone in {strips} strips, moved by \
                         {levels} levels at ({x}, {y}): more than one rounding step of one sample, \
                         which is more than an `ulp` of `ty` can buy (ADR 0219)",
                        cell.blend
                    );
                }
            }
            if composited_here(cell.blend) {
                let unexplained = composed.iter().find(|(x, y, _)| {
                    !backdrop
                        .iter()
                        .chain(&source)
                        .any(|(bx, by, _)| (bx, by) == (x, y))
                });
                assert!(
                    unexplained.is_none(),
                    "the {:?} cell in {strips} strips moved at {unexplained:?} where neither of its \
                     two photographs did: a mechanism other than a shifted sample (ADR 1768)",
                    cell.blend
                );
            }
        }
    }
}

/// One cell of `blendmode.pdf`: the blend its source is drawn under, and the device pixels both
/// of its images cover whole, as `[x0, x1) × [y0, y1)`.
#[derive(Debug, Clone, Copy)]
struct Cell {
    blend: BlendMode,
    x: (u32, u32),
    y: (u32, u32),
}

/// The cells, read off the list's image commands in painting order: a backdrop under Normal, then
/// the source over it under the cell's mode, at the same placement.
fn cells_of(list: &DisplayList, target: TargetSpec) -> Vec<Cell> {
    let mut images = Vec::new();
    collect_images(list.commands(), &mut images);
    images
        .chunks_exact(2)
        .map(|pair| {
            let (backdrop, source) = (pair[0], pair[1]);
            assert_eq!(
                backdrop.blend(),
                BlendMode::Normal,
                "a cell's first image is its backdrop, drawn under Normal"
            );
            let bounds = source
                .device_bounds(target.transform)
                .expect("an image states its extent");
            assert_eq!(
                backdrop.device_bounds(target.transform),
                Some(bounds),
                "a cell's two images share one placement"
            );
            Cell {
                blend: source.blend(),
                x: (
                    whole_pixel_after(bounds.min.x),
                    whole_pixel_before(bounds.max.x),
                ),
                y: (
                    whole_pixel_after(bounds.min.y),
                    whole_pixel_before(bounds.max.y),
                ),
            }
        })
        .collect()
}

/// The first pixel index at or after a device coordinate on the page.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a coordinate inside a 596 × 842 target, rounded first, so the cast is exact"
)]
fn whole_pixel_after(coordinate: f32) -> u32 {
    coordinate.ceil().max(0.0) as u32
}

/// The pixel index that ends at or before a device coordinate on the page, exclusive.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a coordinate inside a 596 × 842 target, rounded first, so the cast is exact"
)]
fn whole_pixel_before(coordinate: f32) -> u32 {
    coordinate.floor().max(0.0) as u32
}

/// Every image command, depth first, in painting order.
fn collect_images<'a>(commands: &'a [Command], into: &mut Vec<&'a Command>) {
    for command in commands {
        match command {
            Command::Group { commands, .. } => collect_images(commands, into),
            Command::Image { .. } => into.push(command),
            _ => {}
        }
    }
}

/// The list with every image command passed through `edit`, numbered depth first; nothing is
/// removed, so the strip planner sees the page it sees in the list itself.
fn with_images(list: &DisplayList, edit: &dyn Fn(usize, &mut Command)) -> DisplayList {
    fn walk(
        commands: &[Command],
        edit: &dyn Fn(usize, &mut Command),
        next: &mut usize,
    ) -> Vec<Command> {
        commands
            .iter()
            .map(|command| {
                let mut command = command.clone();
                match &mut command {
                    Command::Group { commands, .. } => *commands = walk(commands, edit, next),
                    Command::Image { .. } => {
                        edit(*next, &mut command);
                        *next = next.saturating_add(1);
                    }
                    _ => {}
                }
                command
            })
            .collect()
    }
    let mut edited = list.clone();
    let _ = edited.split_off_commands(0);
    for command in walk(list.commands(), edit, &mut 0) {
        edited.push(command);
    }
    edited
}

/// Makes an image transparent and leaves its extent, so that it marks nothing and the planner
/// still counts it.
fn hide(image: &mut Command) {
    if let Command::Image { alpha, .. } = image {
        *alpha = 0.0;
    }
}

/// Draws an image under Normal, so that the page shows its samples rather than a blend of them.
fn normal(image: &mut Command) {
    if let Command::Image { blend, .. } = image {
        *blend = BlendMode::Normal;
    }
}

/// Whether `render_cpu` composites this mode itself from two eight-bit rasters: Table 135's four
/// non-separable modes, which its `blend` module takes back from `tiny-skia` (ADR 0047).
fn composited_here(blend: BlendMode) -> bool {
    matches!(
        blend,
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity
    )
}

/// The pixels of `cell` that differ between two rasters, each with the most any of its colour
/// channels moved.
fn moved_within(whole: &[u8], split: &[u8], width: u32, cell: &Cell) -> Vec<(u32, u32, u8)> {
    let mut moved = Vec::new();
    for y in cell.y.0..cell.y.1 {
        for x in cell.x.0..cell.x.1 {
            let at = usize::try_from(y)
                .ok()
                .zip(usize::try_from(x).ok())
                .zip(usize::try_from(width).ok())
                .map(|((y, x), width)| y.saturating_mul(width).saturating_add(x).saturating_mul(4))
                .expect("a pixel index on a page this size");
            let levels = whole[at..at.saturating_add(3)]
                .iter()
                .zip(&split[at..at.saturating_add(3)])
                .map(|(ours, theirs)| ours.abs_diff(*theirs))
                .max()
                .unwrap_or(0);
            if levels > 0 {
                moved.push((x, y, levels));
            }
        }
    }
    moved
}
