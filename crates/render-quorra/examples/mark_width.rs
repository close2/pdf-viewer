//! The instrument behind `doc/QUORRA_HAIRLINE_MARKS.md`: how many pixel rows a
//! §10.7.4 mark occupies, and how much ink it carries, per backend and per scale.
//!
//! Renders `issue4260_reduced.pdf` — a grid ruled with zero-height rectangles —
//! through `render-cpu` and quorra at 1x, 2x and 4x, then walks a column that
//! crosses only the horizontal rules and prints each line's starting row, the
//! rows it touches, and its total ink. One device pixel of ink split across two
//! rows is the fuzzy hairline the document describes; one row at full ink is
//! what the clause's letter prescribes.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    missing_docs
)]

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;

fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/issue4260_reduced.pdf");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let document = Document::open(bytes).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("exists");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    for scale in [1.0_f32, 2.0, 4.0] {
        let target = TargetSpec::for_page(&list, scale, 1 << 30).unwrap();
        let rasters = [
            (
                "cpu",
                render_cpu::CpuRasterizer::new()
                    .rasterize(&list, target)
                    .unwrap(),
            ),
            (
                "quorra",
                render_quorra::QuorraRasterizer::new_headless()
                    .unwrap()
                    .rasterize(&list, target)
                    .unwrap(),
            ),
        ];
        for (name, raster) in rasters {
            let (w, h) = (raster.width as usize, raster.height as usize);
            let ink = |x: usize, y: usize| {
                let at = (y * w + x) * 4;
                255 - raster.data[at]
                    .min(raster.data[at + 1])
                    .min(raster.data[at + 2])
            };
            // A column that crosses the horizontal rules but no vertical line:
            // probe a few and keep the first whose ink runs are all short.
            let mut runs = Vec::new();
            for candidate in [w / 7, w / 5, w / 3, w * 2 / 5, w * 3 / 5] {
                runs.clear();
                let mut y = h / 4;
                while y < h && runs.len() < 4 {
                    if ink(candidate, y) > 8 {
                        let start = y;
                        let mut total = 0.0;
                        while y < h && ink(candidate, y) > 8 {
                            total += f32::from(ink(candidate, y)) / 255.0;
                            y += 1;
                        }
                        runs.push((start, y - start, (total * 100.0).round() / 100.0));
                    } else {
                        y += 1;
                    }
                }
                if !runs.is_empty() && runs.iter().all(|(_, rows, _)| *rows <= 6) {
                    break;
                }
            }
            println!("scale {scale}: {name:>6} {w}x{h}: (row, rows-touched, total-ink) {runs:?}");
        }
    }
}
