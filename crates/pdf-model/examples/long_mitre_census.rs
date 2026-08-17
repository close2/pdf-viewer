//! How often a document asks for a mitre a rasteriser's own join code will not draw.
//!
//! ISO 32000-2 §8.4.3.5 bounds the ratio of mitre length to line width by the file's own `M`, and
//! `doc/todo/11` §6 found `tiny-skia`'s stroker refusing every ratio above 90.51 — an angle test
//! applied *before* the limit is read — so a file stating a large limit gets a bevel where its own
//! arithmetic asks for a spike. That defect has one witness in `pdf-differences` and none anybody
//! had counted, and the count is what says whether the construction that fixes it is worth its
//! risk: it is the population on which `render-cpu` and the two graphics-device backends drew
//! different pictures.
//!
//! For each document's first page this prints, over `Command::Stroke`s alone:
//!
//! - how many state §8.4.3.4's mitre join with a limit at or above the ratio the stroker refuses,
//!   which is the cheap pre-filter the fix itself uses;
//! - how many actually **have** such a join — the ratio `1 / sin(φ/2)` at or above 90.51 while at
//!   or under the file's own limit — and the sharpest ratio and mitre length each page states;
//! - how many of those the fix declines and why: a dash pattern, whose cuts decide where a join
//!   still exists, and a stroke at or under one device pixel, where the library draws a hairline
//!   with no joins at all and §10.7.4's own substitutions own the geometry.
//!
//! ```sh
//! cargo run --release -p pdf-model --example long_mitre_census -- doc/pdf.js/test/pdfs/*.pdf
//! cargo run --release -p pdf-model --example long_mitre_census -- doc/corpora/pdf-differences/*/*.pdf
//! ```
#![expect(
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    reason = "a measurement example: its output is the point, and every count below is bounded \
              by the number of path commands on one page"
)]

use pdf_render::{Command, LineJoin};

/// The ratio `tiny-skia`'s stroker bevels above whatever the file's limit says — `render-cpu`'s
/// `BEVELLED_BY_THE_STROKER`, restated here because an example may not reach into a backend.
const REFUSED_ABOVE: f64 = 90.51;

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    if files.is_empty() {
        println!("usage: long_mitre_census <file.pdf>...");
        return;
    }

    let mut opened = 0_usize;
    let mut pages_with_a_large_limit = 0_usize;
    let mut pages_with_a_long_mitre = 0_usize;
    let mut strokes_with_a_large_limit = 0_usize;
    let mut strokes_with_a_long_mitre = 0_usize;
    let mut declined_dashed = 0_usize;
    let mut declined_thin = 0_usize;

    for file in &files {
        let Ok(bytes) = std::fs::read(file) else {
            continue;
        };
        let Ok(document) = pdf_syntax::Document::open(bytes) else {
            continue;
        };
        let Some(page) = pdf_model::Pages::new(&document).get(0) else {
            continue;
        };
        opened += 1;
        let list = pdf_model::interpret(&document, &page).display_list;
        let Ok(target) = pdf_render::TargetSpec::for_page(&list, 1.0, 1 << 30) else {
            continue;
        };

        let mut limits = 0_usize;
        let mut long = 0_usize;
        let mut dashed = 0_usize;
        let mut thin = 0_usize;
        let mut sharpest = 0.0_f64;
        let mut longest = 0.0_f64;
        for command in list.commands() {
            let Command::Stroke {
                path,
                transform,
                stroke,
                ..
            } = command
            else {
                continue;
            };
            if stroke.join != LineJoin::Miter || f64::from(stroke.miter_limit) < REFUSED_ABOVE {
                continue;
            }
            limits += 1;
            let at = transform.then(target.transform);
            let width = stroke.device_width(at);
            let ratio = pdf_render::sharpest_admitted_mitre(path, stroke).map_or(0.0, f64::from);
            if ratio < REFUSED_ABOVE {
                continue;
            }
            long += 1;
            sharpest = sharpest.max(ratio);
            longest = longest.max(ratio * f64::from(width));
            if !stroke.dash_array.is_empty() {
                dashed += 1;
            }
            if pdf_render::thinnest_line(at).is_some_and(|one_pixel| width <= one_pixel) {
                thin += 1;
            }
        }

        strokes_with_a_large_limit += limits;
        strokes_with_a_long_mitre += long;
        declined_dashed += dashed;
        declined_thin += thin;
        if limits > 0 {
            pages_with_a_large_limit += 1;
        }
        if long > 0 {
            pages_with_a_long_mitre += 1;
            println!(
                "{file}: {long} of {limits} strokes state a mitre over the ratio the stroker \
                 draws; sharpest {sharpest:.3}, longest mitre {longest:.1} device pixels\
                 {}{}",
                if dashed > 0 {
                    format!(", {dashed} dashed (declined)")
                } else {
                    String::new()
                },
                if thin > 0 {
                    format!(", {thin} at or under a device pixel (declined)")
                } else {
                    String::new()
                }
            );
        }
    }

    println!(
        "{opened} first pages of {} files: {pages_with_a_large_limit} state a mitre join whose \
         limit admits a ratio over {REFUSED_ABOVE}, {pages_with_a_long_mitre} actually have one",
        files.len()
    );
    println!(
        "  strokes: {strokes_with_a_large_limit} with such a limit, {strokes_with_a_long_mitre} \
         with such a join, of which {declined_dashed} dashed and {declined_thin} at or under a \
         device pixel are declined"
    );
}
