//! One page through both of quorra's coverage lanes, beside the CPU oracle.
//!
//! `tests/corpus.rs` runs one lane per invocation and writes an artefact only where that lane
//! differs, so a page that differs on **one** lane leaves no picture of the other — which is
//! exactly the population ADR 0413 is about: the two lanes' differing sets have the
//! same size and not the same members. This renders each named document's first page three
//! times — the oracle, `Coverage::Cpu`, `Coverage::Gpu` — prints all three pairwise
//! comparisons, and writes the three rasters so the side-by-side can be looked at.
//!
//! ```sh
//! cargo run --release -p render-quorra --example lane_diff -- bug1743245 issue16500
//! ```
//!
//! With no arguments it takes the four documents that round diagnosed.
#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a measurement example: the numbers on stdout are the point, and a missing corpus \
              document must stop the run rather than be reported as a lane's answer"
)]

use std::path::{Path, PathBuf};

use pdf_render::{Raster, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use render_quorra::QuorraRasterizer;

/// The pages ADR 0413 named: two each lane differs on alone.
const DEFAULT: [&str; 4] = ["bug1743245", "issue21068", "bug1863910", "issue16500"];

fn main() {
    let wanted: Vec<String> = std::env::args().skip(1).collect();
    let stems: Vec<String> = if wanted.is_empty() {
        DEFAULT.iter().map(|s| (*s).to_owned()).collect()
    } else {
        wanted
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    for stem in &stems {
        let path = root.join(format!("{stem}.pdf"));
        let bytes = std::fs::read(&path).expect("the corpus document");
        let document = Document::open(bytes).expect("opens");
        let pages = pdf_model::Pages::new(&document);
        let page = pages.get(0).expect("has a first page");
        let list = pdf_model::content::interpret(&document, &page).display_list;
        let target = TargetSpec::for_page(&list, 1.0, 64 << 20).unwrap();
        let cpu = CpuRasterizer::new().rasterize(&list, target).unwrap();

        println!("{stem}: {}x{}", cpu.width, cpu.height);
        let mut lanes: Vec<(&str, Raster)> = Vec::new();
        for (name, coverage) in [
            ("cpu-lane", quorra_gpu::Coverage::Cpu),
            ("gpu-lane", quorra_gpu::Coverage::Gpu),
        ] {
            // The gate's own options, quantum off, so these numbers are that gate's numbers.
            let mut backend = QuorraRasterizer::with_options(&quorra_gpu::Options {
                glyph_quantum: None,
                ..render_quorra::options()
            })
            .expect("adapter");
            backend.set_coverage(coverage);
            let ours = backend.rasterize(&list, target).unwrap();
            let c = raster_compare::compare(&cpu, &ours).unwrap();
            println!(
                "  oracle vs {name}: mean {:.4} worst tile {:.2} at {:?} differing {:.4} \
                 ssim {:.5}",
                c.mean_error,
                c.worst_tile_error,
                c.worst_tile_at,
                c.differing_fraction,
                c.structural_similarity
            );
            lanes.push((name, ours));
        }
        let c = raster_compare::compare(&lanes[0].1, &lanes[1].1).unwrap();
        println!(
            "  cpu-lane vs gpu-lane: mean {:.4} worst tile {:.2} at {:?} differing {:.4} \
             ssim {:.5}",
            c.mean_error,
            c.worst_tile_error,
            c.worst_tile_at,
            c.differing_fraction,
            c.structural_similarity
        );

        let out: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/tmp/quorra-lanes")
            .join(stem);
        std::fs::create_dir_all(&out).unwrap();
        write(&out.join("oracle.png"), &cpu);
        for (name, raster) in &lanes {
            write(&out.join(format!("{name}.png")), raster);
        }
    }
}

fn write(at: &Path, raster: &Raster) {
    let file = std::fs::File::create(at).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), raster.width, raster.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&raster.data)
        .unwrap();
}
