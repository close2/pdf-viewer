//! How many of the type 1 shadings that exist a device will evaluate, and why not the rest.
//!
//! The population behind ADR 0376. ISO 32000-2 §8.7.4.5.2's type 1 shading is the one paint
//! whose colour is a program, and quorra's `Paint::Function` evaluates that program in a
//! generated fragment shader instead of on the processor — four to five orders of magnitude,
//! on their measurement. What no clause can say is *how many documents it reaches*, because
//! that is decided twice: once by this tree, which builds a `ShadingProgram` only where the
//! device's own colour components are the function's outputs, and once by the device, which
//! refuses a program whose disagreement with an independent evaluation it cannot bound
//! (quorra's ADR 0053, `Agreement::Unbounded`).
//!
//! This counts both refusals, apart, over whatever files it is given. It drives the shipping
//! path rather than reproducing it — the same `QuorraRasterizer`, the same
//! `Encoder::function_paint` — so what it reports is what a page would do, not what a second
//! implementation of the conditions thinks it would.
//!
//! **The rasterisation is deliberately small.** A refused shading falls back to the grid, and
//! the grid is one function evaluation per device pixel; at a hundred pixels a side the
//! fallback costs a millisecond instead of a second, and neither answer depends on the size.
//!
//! **The pre-filter is sound rather than a shortcut**, and it is the argument
//! `pdf-model`'s `type4_type_census` states: §7.5.7 heads its exclusion list "The following
//! objects shall not be stored in an object stream:" with "Stream objects", and a type 4
//! function is a stream — so a document whose raw bytes never say `/FunctionType` holds no
//! type 4 function anywhere, and a type 1 shading without one is not a candidate.
//!
//! ```sh
//! cargo run --release -p render-quorra --example function_paint_census -- \
//!     doc/pdf.js/test/pdfs/*.pdf doc/corpora/*/**/*.pdf doc/corpora-own/*.pdf
//! ```
//!
//! An argument beginning with `@` names a file of paths, one to a line.
#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a measurement whose output is its purpose: it stops loudly where no adapter can \
              be had, and its counters over a corpus's shadings are far below what a usize \
              counts"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_quorra::QuorraRasterizer;

/// The page is drawn this many pixels on its longer side. See the module note.
const PROBE: f32 = 100.0;

fn paths() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for argument in std::env::args().skip(1) {
        if let Some(list) = argument.strip_prefix('@') {
            let text = std::fs::read_to_string(list).expect("the list of paths is readable");
            files.extend(
                text.lines()
                    .filter(|line| !line.is_empty())
                    .map(PathBuf::from),
            );
        } else {
            files.push(PathBuf::from(argument));
        }
    }
    files
}

/// Whether this document can hold a type 4 function at all — see the module note.
fn may_hold_a_program(bytes: &[u8]) -> bool {
    bytes
        .windows(b"/FunctionType".len())
        .any(|window| window == b"/FunctionType")
}

fn main() {
    let mut quorra = QuorraRasterizer::new_headless().expect("an adapter");
    println!("adapter: {}", quorra.adapter_description());

    let mut scanned = 0usize;
    let mut candidates = 0usize;
    let mut evaluated = 0usize;
    let mut refused = 0usize;
    // The device's ground, verbatim, with the documents that reached it.
    let mut grounds: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut took_the_device: Vec<String> = Vec::new();

    for path in paths() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        scanned += 1;
        if !may_hold_a_program(&bytes) {
            continue;
        }
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let pages = pdf_model::Pages::new(&document);
        for index in 0..pages.len() {
            let Some(page) = pages.get(index) else {
                continue;
            };
            let list = pdf_model::content::interpret(&document, &page).display_list;
            let size = list.page_size;
            let longest = size.width.max(size.height).max(1.0);
            let scale = PROBE / longest;
            // `for_page` does the rounding and the flip, so this example states a scale and
            // nothing else — the same target a gate would build.
            let Ok(target) = TargetSpec::for_page(&list, scale, 1 << 30) else {
                continue;
            };
            if quorra.rasterize(&list, target).is_err() {
                continue;
            }
            let paints = quorra.last_function_paints();
            if paints.evaluated() == 0 && paints.refusals().is_empty() {
                continue;
            }
            candidates += 1;
            evaluated += paints.evaluated() as usize;
            refused += paints.refusals().len();
            let where_ = format!("{name} p{}", index + 1);
            if paints.evaluated() > 0 {
                took_the_device.push(where_.clone());
            }
            for ground in paints.refusals() {
                grounds
                    .entry(ground.clone())
                    .or_default()
                    .push(where_.clone());
            }
        }
    }

    println!("\n{scanned} file(s) read, {candidates} page(s) carrying a §8.7.4.5.2 program");
    println!("  device-evaluated shadings: {evaluated}");
    println!("  refused, drawn from the grid: {refused}");
    if !took_the_device.is_empty() {
        println!("\npages the device evaluated:");
        for page in &took_the_device {
            println!("  {page}");
        }
    }
    if !grounds.is_empty() {
        println!("\ngrounds, most common first:");
        let mut rows: Vec<(&String, &Vec<String>)> = grounds.iter().collect();
        rows.sort_by_key(|(_, pages)| std::cmp::Reverse(pages.len()));
        for (ground, pages) in rows {
            println!("  {:>4}  {ground}", pages.len());
            for page in pages.iter().take(4) {
                println!("          {page}");
            }
        }
    }
}
