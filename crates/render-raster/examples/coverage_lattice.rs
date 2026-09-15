//! Where a backend's partial coverages *land* — the instrument that says whether a rasteriser
//! states an edge on a lattice or anywhere the geometry puts it.
//!
//! # What it is for
//!
//! `ink_ladder` (ADR 1064's record) separates "the same ink in different places" from "different
//! amounts of ink". Where it answers *the same ink*, the remaining question is which backend's
//! placement is the coarser, and that is what this asks. ISO 32000-2 §10.7.4:
//!
//! > Its coordinates are mapped into device space but not rounded to device pixel boundaries.
//!
//! `tiny-skia`'s anti-aliased path scan converter supersamples four times per pixel row and
//! quantises a run along `x` to quarter-pixel steps, so the coverage it can state for a pixel a
//! single edge crosses is a multiple of a sixteenth. This counts how many of a page's *partial*
//! pixels sit on that lattice.
//!
//! # The control, which is what makes the count a measurement (trap 13)
//!
//! A count with no control is a sentence about the instrument. The control here is the second
//! backend on the same page: `render-raster` resolves a path analytically and has no such
//! quantum, so its share must come out near what chance gives — the lattice's tolerance band as
//! a fraction of the 0–255 range. A run where both backends read high has measured the page's
//! colours rather than either converter.
//!
//! # Why only two-colour pages count
//!
//! A partial coverage is only readable as a coverage where the page states one ink over one
//! backdrop: anywhere else the level is a blend of two colours and says nothing about the scan
//! converter. Pages whose non-white, non-black pixels are more than `MIXED_SHARE` of the inked
//! ones are reported as `mixed` and counted for neither backend.
//!
//! # Running it
//!
//! ```sh
//! cargo run --release -p render-raster --example coverage_lattice -- endchar issue15150
//! ```

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    reason = "a measurement example: the table on stdout is the point, a missing corpus document \
              must stop the run rather than be reported as a backend's answer, and the sums are \
              over rasters whose pixel budget is bounded below f64's exact integer range"
)]

use std::path::Path;

use pdf_render::{Raster, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use render_raster::QuorraRasterizer;

/// Pixel budget per page.
const PIXEL_BUDGET: u64 = 1 << 28;

/// How far from a lattice point a level may sit and still be counted on it, in levels of 255.
///
/// One and a half levels, because the two backends round `255 * area` differently and
/// `tiny-skia` carries its own 8.8 fixed point on top of that; a band narrower than the
/// disagreement between two correct roundings would count a lattice edge as off it. The band is
/// what the control's expected share is computed from, below.
const TOLERANCE: f32 = 1.5;

/// The lattice: sixteen steps of `255 / 16` across the range.
const STEPS: u32 = 16;

/// The share of inked pixels that may be neither the page's ink nor its backdrop before the
/// page's levels stop being readable as coverages at all.
const MIXED_SHARE: f64 = 0.35;

fn main() {
    let stems: Vec<String> = std::env::args().skip(1).collect();
    assert!(
        !stems.is_empty(),
        "name at least one document stem under doc/pdf.js/test/pdfs"
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut backend = QuorraRasterizer::with_options(&render_raster::options()).expect("adapter");
    backend.set_coverage(raster_gpu::Coverage::Cpu);
    let chance = f64::from(TOLERANCE * 2.0 + 1.0) * f64::from(STEPS) / 256.0;
    println!("partial coverages on the 1/{STEPS} lattice, page one, at 1x");
    println!(
        "chance, for a band of +/-{TOLERANCE} levels: {:.1}%",
        chance * 100.0
    );
    println!(
        "{:26} {:>10} {:>12} {:>10} {:>12}",
        "document", "cpu n", "cpu on", "raster n", "raster on"
    );
    for stem in &stems {
        let bytes = std::fs::read(root.join(format!("{stem}.pdf"))).expect("the corpus document");
        let document = Document::open(bytes).expect("opens");
        let pages = pdf_model::Pages::new(&document);
        let page = pages.get(0).expect("has a first page");
        let list = pdf_model::content::interpret(&document, &page).display_list;
        print!("{stem:26}");
        let Ok(target) = TargetSpec::for_page(&list, 1.0, PIXEL_BUDGET) else {
            println!("  no target");
            continue;
        };
        let oracle = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("draws");
        report(&oracle);
        match backend.rasterize(&list, target) {
            Ok(ours) => report(&ours),
            Err(why) => print!(" raster refused: {why}"),
        }
        println!();
    }
}

/// Prints one backend's partial-pixel count and the share of it on the lattice.
///
/// A level is read as a coverage by projecting the pixel onto the page's own ink: with `t` the
/// pixel's distance from white per channel and `K` the darkest such vector on the page, a pixel
/// covered by a fraction `c` of that one ink over white has `t = c * K`. So a pixel whose `t` is
/// parallel to `K` states `|t| / |K|`, and one that is not is a second ink or a blend of two and
/// is counted as `mixed` instead.
fn report(raster: &Raster) {
    let ink = darkest(raster);
    let norm = ink.0 * ink.0 + ink.1 * ink.1 + ink.2 * ink.2;
    if norm <= 0.0 {
        print!(" {:>10} {:>12}", 0, "no ink");
        return;
    }
    let (mut partial, mut on, mut mixed, mut inked) = (0_u64, 0_u64, 0_u64, 0_u64);
    for pixel in raster.data.chunks_exact(4) {
        let t = (
            255.0 - f32::from(pixel[0]),
            255.0 - f32::from(pixel[1]),
            255.0 - f32::from(pixel[2]),
        );
        let length = t.0 * t.0 + t.1 * t.1 + t.2 * t.2;
        if length <= 0.0 {
            continue;
        }
        inked += 1;
        // The projection of `t` on `K`, and the residue perpendicular to it. A pixel more than a
        // level and a half off the ray is not this page's one ink at any coverage.
        let along = (t.0 * ink.0 + t.1 * ink.1 + t.2 * ink.2) / norm;
        let residue = (t.0 - along * ink.0)
            .hypot(t.1 - along * ink.1)
            .hypot(t.2 - along * ink.2);
        if residue > TOLERANCE || along <= 0.0 {
            mixed += 1;
            continue;
        }
        if along >= 1.0 - TOLERANCE / 255.0 {
            continue;
        }
        partial += 1;
        if on_lattice(along * 255.0) {
            on += 1;
        }
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "counts over a raster bounded by PIXEL_BUDGET = 2^28, inside f64's exact range"
    )]
    let (mixed_share, share) = (
        mixed as f64 / (inked.max(1)) as f64,
        on as f64 / (partial.max(1)) as f64,
    );
    if mixed_share > MIXED_SHARE {
        print!(" {partial:>10} {:>12}", "mixed");
        return;
    }
    print!(" {partial:>10} {:>11.1}%", share * 100.0);
}

/// The page's own ink: the largest per-channel distance from white any pixel states.
fn darkest(raster: &Raster) -> (f32, f32, f32) {
    let mut best = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
    for pixel in raster.data.chunks_exact(4) {
        let t = (
            255.0 - f32::from(pixel[0]),
            255.0 - f32::from(pixel[1]),
            255.0 - f32::from(pixel[2]),
        );
        let length = t.0 * t.0 + t.1 * t.1 + t.2 * t.2;
        if length > best.3 {
            best = (t.0, t.1, t.2, length);
        }
    }
    (best.0, best.1, best.2)
}

/// Whether `level` lies within [`TOLERANCE`] of a multiple of `255 / STEPS`.
fn on_lattice(level: f32) -> bool {
    #[expect(clippy::cast_precision_loss, reason = "STEPS is 16")]
    let step = 255.0 / STEPS as f32;
    let index = (level / step).round();
    (level - index * step).abs() <= TOLERANCE
}
