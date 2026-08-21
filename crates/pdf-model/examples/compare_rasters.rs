//! Prints the oracle's own four measurements between two PNGs on disk.
//!
//! `crates/pdf-model/tests/oracle.rs` computes these against the reference renderers and prints
//! them per page; this is the same arithmetic, `raster_compare::compare`, pointed at two files a
//! person names. It is here because diagnosing a contradicted page means asking the same question
//! of a raster the gate never made — a closed form written out pixel by pixel, a reference against
//! another reference, one scale against another — and doing that by eye or by a reimplementation
//! of the metric answers a different question than the gate did.
//!
//! `cargo run -p pdf-model --example compare_rasters -- <left.png> <right.png>`

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::panic,
    reason = "a diagnostic binary: its output is the point, and a missing argument, an unreadable \
              file or a colour type it does not convert should stop it loudly rather than be \
              handled"
)]
fn main() {
    let mut args = std::env::args().skip(1);
    let left = args.next().expect("a left png");
    let right = args.next().expect("a right png");
    let left = read(&left);
    let right = read(&right);
    let comparison = raster_compare::compare(&left, &right).expect("comparable rasters");
    println!(
        "mean {:.4}  worst tile {:.2} at {:?}  max {}  differing {:.4}%  ssim {:.5}  worst tile ssim {:.5} at {:?}",
        comparison.mean_error,
        comparison.worst_tile_error,
        comparison.worst_tile_at,
        comparison.max_error,
        comparison.differing_fraction * 100.0,
        comparison.structural_similarity,
        comparison.worst_tile_similarity,
        comparison.worst_tile_similarity_at,
    );
}

/// Reads a PNG into the eight-bit RGBA raster the comparison wants.
fn read(path: &str) -> pdf_render::Raster {
    let file = std::fs::File::open(path).expect("an openable png");
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().expect("a png header");
    let mut buffer = vec![0; reader.output_buffer_size().expect("a bounded png")];
    let info = reader.next_frame(&mut buffer).expect("png pixels");
    let data = match info.color_type {
        png::ColorType::Rgba => buffer[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => buffer[..info.buffer_size()]
            .chunks_exact(3)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        other => panic!("unsupported colour type {other:?}"),
    };
    pdf_render::Raster {
        width: info.width,
        height: info.height,
        format: pdf_render::RasterFormat::Rgba8,
        data,
    }
}
