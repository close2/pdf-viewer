//! What a page's sub-pixel strokes are, and what their caps are worth in ink.
//!
//! `render-quorra/examples/sub_pixel_marks` measures what each backend does with a mark thinner
//! than a device pixel, on synthetic pages where every dimension is chosen. This one asks the
//! other half of the question — **what a real page actually states** — because ISO 32000-2
//! §10.7.4's substitutions are conditioned on a width, and a page's own widths decide whether a
//! rule matters there at all.
//!
//! It prints, for one page at scale 1:
//!
//! - how many `Command::Stroke`s are under the device's quantum, and the distribution of their
//!   device widths;
//! - how many of them state a cap [`pdf_render::sub_pixel_caps`] draws, and how dark they are,
//!   because ink is a colour times an area and a census of areas alone is not comparable with a
//!   raster's;
//! - **what their caps' own area comes to**, in device pixels and in levels of 255 over the whole
//!   raster, which is the number to compare a before-and-after ink measurement with;
//! - and **§8.5.3.2's own marks** — a degenerate subpath's dot and a zero-length dash's — with the
//!   device widths they are stated at, because those are the two marks §10.7.4's substitution
//!   reaches through `pdf_render::point_mark` and a population is what says whether a document
//!   ever asks for one. The line is printed even where the count is zero, so that a sweep over a
//!   corpus can say what it looked at rather than only what it found.
//!
//! ```sh
//! cargo run --release -p pdf-model --example sub_pixel_width_census -- <file.pdf> [page]
//! ```
//!
//! On `issue12295.pdf` page 1 — the corpus's extreme, and ADR 0268's and ADR 0290's witness — it
//! says that all 65 859 sub-pixel strokes are 0.1366 of a device pixel wide, near-black, and that
//! their caps are 2170.93 device pixels of geometry, or 1.14 levels of 255 over the page. The page
//! took **0.133** of a level when those caps started being drawn, which is what says the residual
//! there is the eight-bit raster rather than the construction: each cap's ink lands as about half
//! a level on each of a few pixels, and half a level is what an eight-bit raster rounds away.
#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    reason = "a measurement example: its output is the point, a missing file or page should stop \
              it loudly rather than be reported as a page with no strokes on it, and one walk of \
              one page's commands answering every question this prints is clearer than four"
)]

use pdf_render::{Command, LineCap, Paint, PathCommand};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("a pdf");
    let index: usize = args.next().map_or(1, |n| n.parse().expect("a page number"));
    let bytes = std::fs::read(&path).expect("the document is readable");
    let document = pdf_syntax::Document::open(bytes).expect("a PDF");
    let page = pdf_model::Pages::new(&document)
        .get(index.saturating_sub(1))
        .expect("that page");
    let list = pdf_model::interpret(&document, &page).display_list;
    let target = pdf_render::TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a valid target");

    let mut widths: Vec<f32> = Vec::new();
    let mut capped = 0_usize;
    let mut cap_area = 0.0_f64;
    let mut solid = 0_usize;
    let mut darkness = 0.0_f64;
    // §8.5.3.2's two marks, each device width recorded once per mark.
    let mut dots: Vec<f32> = Vec::new();
    let mut dashes: Vec<f32> = Vec::new();
    for command in list.commands() {
        let Command::Stroke {
            transform,
            stroke,
            path,
            paint,
            ..
        } = command
        else {
            continue;
        };
        let at = transform.then(target.transform);
        let one_pixel = pdf_render::thinnest_line(at).unwrap_or(0.0);
        let width = stroke.device_width(at);
        if one_pixel > 0.0 {
            // The clause's own shapes, asked for with no transform so that the answer is the
            // document's marks rather than the substitution's.
            let stated = pdf_render::split_degenerate(path, stroke.cap, width, None);
            let made = stated.map_or(0, |split| {
                split
                    .dots
                    .commands()
                    .iter()
                    .filter(|c| matches!(c, PathCommand::MoveTo(_)))
                    .count()
            });
            dots.extend(std::iter::repeat_n(width / one_pixel, made));
            // A zero-length dash is dispensed by a backend's own dasher, so what a display list
            // can be asked is whether the pattern states one whose cap is drawn.
            if pdf_render::dashes_showing_direction(&stroke.dash_array, stroke.cap).is_some() {
                dashes.push(width / one_pixel);
            }
        }
        if !(one_pixel > 0.0 && width < one_pixel) {
            continue;
        }
        let device = width / one_pixel;
        widths.push(device);
        if let Paint::Solid(colour) = paint {
            solid = solid.saturating_add(1);
            darkness += f64::from(1.0 - (colour.r + colour.g + colour.b) / 3.0);
        }
        let Some(mark) = pdf_render::enlarged_mark(width, at) else {
            continue;
        };
        if pdf_render::sub_pixel_caps(path, stroke.cap, mark).is_none() {
            continue;
        }
        capped = capped.saturating_add(1);
        // Table 53's own areas, in device pixels: a subpath's two round caps are `pi w² / 4`
        // together and its two projecting square caps are `w²`. Every subpath is counted as open,
        // which `sub_pixel_caps` has just been asked about and which is why it is asked first.
        let ends = path
            .commands()
            .iter()
            .filter(|c| matches!(c, PathCommand::MoveTo(_)))
            .count() as f64;
        let w = f64::from(device);
        cap_area += ends
            * w
            * w
            * match stroke.cap {
                LineCap::Round => core::f64::consts::PI / 4.0,
                LineCap::Square => 1.0,
                LineCap::Butt => 0.0,
            };
    }

    report("§8.5.3.2 dots", &mut dots);
    report("zero-length dash patterns", &mut dashes);
    widths.sort_by(f32::total_cmp);
    let n = widths.len();
    println!("{path} page {index}: {n} strokes under the device's quantum");
    if n == 0 {
        return;
    }
    println!(
        "  device width: min {:.4} p25 {:.4} median {:.4} p75 {:.4} max {:.4}",
        widths[0],
        widths[n / 4],
        widths[n / 2],
        widths[3 * n / 4],
        widths[n.saturating_sub(1)]
    );
    println!(
        "  {capped} state a cap that is drawn; {solid} are a solid colour of mean darkness {:.3}",
        darkness / solid.max(1) as f64
    );
    let pixels = f64::from(target.width) * f64::from(target.height);
    println!(
        "  those caps' own area: {cap_area:.2} device pixels over a raster of {pixels:.0}, which \
         is {:.4} levels of 255 if every one of them were black",
        cap_area / pixels * 255.0
    );
}

/// One population of §8.5.3.2 marks, by the device width each is stated at.
///
/// The two thresholds are the ones `pdf_render::point_mark` turns on and are printed rather than
/// left to a reader's arithmetic: **1** device pixel, below which the mark is widened at all, and
/// **0.212**, below which even the widened mark cannot put one level of 255 into any pixel it
/// covers and is stated as the device pixel its own centre lies in. The second is
/// `sqrt(9 / (255 · π/4))` and the ninth is [`pdf_render::point_mark`]'s own bound on how many
/// pixels such a mark can be divided between.
fn report(what: &str, widths: &mut [f32]) {
    widths.sort_by(f32::total_cmp);
    let n = widths.len();
    let under_quantum = widths.iter().filter(|w| **w < 1.0).count();
    let concentrated = widths.iter().filter(|w| **w < 0.212).count();
    print!("  {what}: {n}, {under_quantum} under a device pixel, {concentrated} under 0.212");
    if n == 0 {
        println!();
        return;
    }
    println!(
        " — device width min {:.4} median {:.4} max {:.4}",
        widths[0],
        widths[n / 2],
        widths[n.saturating_sub(1)]
    );
}
