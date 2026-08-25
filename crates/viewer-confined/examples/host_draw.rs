//! What a page's marks cost the **host** that draws them, and whether anything predicts it.
//!
//! ```sh
//! cargo run --release -p viewer-confined --example host_draw -- [--scale N] [--levels K] <file.pdf>…
//! ```
//!
//! `examples/list_against_raster` prices a display list against its raster in *bytes*, which is
//! what decided ADR 0607's payload choice and is the right measure for a **pipe**. This asks the
//! other question, which that decision left open: since ADR 0633 the host draws the marks, so
//! what does a host know about what that will cost it? A display list of 990 kB is ten thousand
//! page-covering fills — **size predicts transport, not work**.
//!
//! # The columns
//!
//! - **arm**, **bytes** — which payload the page crosses as, from `wire::crossing`, and what it
//!   comes to.
//! - **pixels** — the target's own pixel count.
//! - **work** — the summed device-pixel span of every command that can mark a row: the total of
//!   [`pdf_render::row_costs`] over [`pdf_render::command_extents`]. This is the only pre-draw
//!   cost estimate the tree contains, and it is not here because it is good — it is here because
//!   it is what a host would reach for. It is already computed on **every** CPU draw, to choose
//!   where the strips are cut, and thrown away afterwards.
//! - **cover** — `work ÷ pixels`: how many times over the page's own area is painted.
//! - **plan** — what computing `work` and the unsplittable rows costs, which is the part of a
//!   draw that runs *before* the first interrupt check.
//! - **ms** — the measured rasterisation.
//!
//! # What it found, which is the reason to keep it (ADR 0650)
//!
//! **`work` does not predict `ms`.** Over `doc/pdf.js`'s first pages the two correlate at 0.115
//! by Pearson and 0.649 by Spearman — 0.161 and 0.650 over the 952 of them whose target is under
//! 3 Mpixels, so a giant `/MediaBox` is not the confound — and only 8 of the 40 slowest pages are
//! among the 40 with the most work.
//!
//! One pair says it better than the coefficients. `personwithdog.pdf` is 484 704 pixels at a
//! `cover` of **0.2** and draws for **162.4 ms**; `pattern_text_embedded_font.pdf` is 501 832
//! pixels at a `cover` of **593.5** and draws in **15.9 ms**. Same size of page, the estimate
//! three thousand times apart, the clock ten times apart the other way round.
//!
//! That is not a defect in the estimate — [`pdf_render::strips`] says in its own words that it
//! "ignores edge building, which is proportional to a path's complexity rather than to its
//! bounding box", and it is written to choose *where* to cut rather than to predict a time. It is
//! a finding about the host: **there is nothing in this tree a host can ask how long a display
//! list will take**, which is why ADR 0650's answer is an interrupt rather than a budget.
#![expect(
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "an example whose entire output is a measurement; its counters are bounded by one \
              page's commands, and the ratios it prints are printed to one decimal"
)]

use std::time::Instant;

use pdf_model::interpret;
use pdf_model::page::Pages;
use pdf_render::{DisplayList, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// The hostile document, shared with `tests/confined.rs`.
#[path = "../tests/support/amplification.rs"]
mod amplification;

/// Levels of the amplification fixture to price when no file is named.
const FIXTURE_LEVELS: usize = 4;

fn main() {
    let mut scale = 1.0_f32;
    let mut levels = FIXTURE_LEVELS;
    let mut files: Vec<String> = Vec::new();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--scale" => scale = arguments.next().and_then(|v| v.parse().ok()).unwrap_or(1.0),
            "--levels" => levels = arguments.next().and_then(|v| v.parse().ok()).unwrap_or(4),
            _ => files.push(argument),
        }
    }

    println!("name\tarm\tbytes\tpixels\twork\tcover\tplan ms\tms");

    if files.is_empty() {
        for level in 1..=levels {
            let bytes = amplification::document(level, amplification::BRANCH);
            let name = format!("amplification-{level}({} B)", bytes.len());
            report(&name, &bytes, scale);
        }
        return;
    }

    for path in files {
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |file| file.to_string_lossy().into_owned());
        let Ok(bytes) = std::fs::read(&path) else {
            println!("{name}\tunreadable");
            continue;
        };
        report(&name, &bytes, scale);
    }
}

/// Prices one document's first page.
fn report(name: &str, bytes: &[u8], scale: f32) {
    let Ok(document) = Document::open(bytes.to_vec()) else {
        println!("{name}\tunopened");
        return;
    };
    let Some(page) = Pages::new(&document).get(0) else {
        println!("{name}\tno page");
        return;
    };
    let list = interpret(&document, &page).display_list;
    let target = match TargetSpec::for_page(&list, scale, viewer_core::MAX_PIXELS) {
        Ok(target) => target,
        Err(problem) => {
            println!("{name}\tno target: {problem}");
            return;
        }
    };
    let pixels = u64::from(target.width) * u64::from(target.height);
    let arm = match viewer_confined::wire::crossing(&list, pixels * 4) {
        viewer_confined::Crossing::List(encoded) => format!("list\t{}", encoded.len()),
        viewer_confined::Crossing::Raster(_) => format!("raster\t{}", pixels * 4),
    };

    let planning = Instant::now();
    let work = work(&list, target);
    let planned = planning.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    let drawn = CpuRasterizer::new().rasterize(&list, target);
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    let verdict = match drawn {
        Ok(_) => format!("{elapsed:.1}"),
        Err(problem) => format!("refused ({problem})"),
    };
    println!(
        "{name}\t{arm}\t{pixels}\t{work:.0}\t{:.1}\t{planned:.1}\t{verdict}",
        work / pixels as f64,
    );
}

/// The pre-draw pass: the cost estimate and the rows a strip may not begin at.
///
/// Both halves, because both run before a draw's first interrupt check and neither can be
/// interrupted. Returns the estimate; the rows are computed for the timing and dropped.
fn work(list: &DisplayList, target: TargetSpec) -> f64 {
    let extents = pdf_render::command_extents(list, target);
    let total = pdf_render::row_costs(&extents, target).iter().sum();
    let _unsplittable = pdf_render::unsplittable_rows(list, target);
    total
}
