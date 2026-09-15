//! This backend's **own** ink on every tracked corpus first page, one line per document.
//!
//! # What it is for
//!
//! `tests/raster_golden.rs` says *which* pages a drawing change moved; it is a digest, so it
//! cannot say by how much or in which direction. `doc/todo/02` §7 asks a round that changes
//! drawing for the size and the sign of what it moved, and `doc/todo/00`'s step 7 answers that
//! against the *lightest reference* on the ambiguous bucket — a different population and a
//! different question. This is the plainest form of the measurement: run it before a change and
//! after, and the difference of the two files is the mover list with a number beside every name.
//!
//! **There is no reference in this instrument and no verdict in it.** It measures one program
//! twice, which is what `CLAUDE.md` principle 5 permits a change detector to do and all this is.
//! Whether a page that moved moved the right way is `tests/oracle.rs`' question, and a look at the
//! page is trap 1's.
//!
//! # The number
//!
//! The sum over the raster of `765 - r - g - b`, divided by the page's pixel count and by 765, so
//! it is the mean coverage of one ink over the sheet in `0.0..=1.0`. Under §11.3.6's Normal blend
//! over an opaque backdrop a composited channel is linear in the source's coverage, so the sum is
//! linear in the ink the page received whatever colour it is in; dividing by the pixel count makes
//! two pages of different sizes comparable rather than ranking them by area.
//!
//! # Running it
//!
//! ```sh
//! cargo run --release -p pdf-model --example own_ink > before.tsv
//! # make the change
//! cargo run --release -p pdf-model --example own_ink > after.tsv
//! diff before.tsv after.tsv
//! ```
//!
//! The population is the tracked corpus — `doc/pdf.js/test/pdfs`, a submodule pinned by commit —
//! derived from the directory rather than listed (trap 25), and the count is printed beside the
//! rows. A document that will not open, states no first page or refuses the rasteriser prints its
//! own word in place of a number, so a page that stops drawing is a changed line rather than a
//! missing one.

#![expect(
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    reason = "a measurement example: the table on stdout is the point, and the sums are over \
              rasters whose pixel budget is bounded below f64's exact integer range"
)]

use std::path::{Path, PathBuf};

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// The scale and budget every corpus gate rasterises at — `tests/raster_golden.rs`' own, so that
/// the two instruments are looking at one picture.
const SCALE: f32 = 1.0;

/// Pixels per page, as that gate bounds them.
const PIXEL_BUDGET: u64 = 64 << 20;

fn main() {
    let Some(corpus) = corpus() else {
        println!("# no tracked corpus at doc/pdf.js/test/pdfs");
        return;
    };
    println!("# mean coverage of the sheet, cpu backend, first page, scale {SCALE}");
    println!("# {} tracked documents", corpus.len());
    for path in &corpus {
        let name = path
            .file_name()
            .map_or_else(|| String::from("?"), |name| name.to_string_lossy().into());
        println!("{name}\t{}", ink(path));
    }
}

/// One document's first page as a mean coverage, or the word for why there is no number.
fn ink(path: &Path) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        return String::from("unreadable");
    };
    let Ok(document) = Document::open(bytes) else {
        return String::from("unopened");
    };
    let Some(page) = pdf_model::Pages::new(&document).get(0) else {
        return String::from("no-page");
    };
    let list = pdf_model::content::interpret(&document, &page).display_list;
    let Ok(target) = TargetSpec::for_page(&list, SCALE, PIXEL_BUDGET) else {
        return String::from("no-target");
    };
    let Ok(raster) = CpuRasterizer::new().rasterize(&list, target) else {
        return String::from("refused");
    };
    let sum: u64 = raster
        .data
        .chunks_exact(4)
        .map(|pixel| 765 - u64::from(pixel[0]) - u64::from(pixel[1]) - u64::from(pixel[2]))
        .sum();
    let pixels = u64::from(raster.width) * u64::from(raster.height);
    if pixels == 0 {
        return String::from("no-pixels");
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "765 times a raster under the pixel budget is inside f64's exact integer range"
    )]
    let mean = sum as f64 / (pixels as f64 * 765.0);
    format!("{mean:.9}")
}

/// Every tracked corpus document, sorted; `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    if files.is_empty() {
        return None;
    }
    files.sort();
    Some(files)
}
