//! The probe against the oracle: `render-cpu` drawing exactly the marks the probe carried.
//!
//! A difference here is the lane's coverage against §8.5.3.3's exact area — the bar RFC 0013
//! §4 reads — and nothing else, because the oracle is handed the same commands, each without
//! its clip, as the probe draws them.

use pdf_render::{DisplayList, Raster, RasterFormat, Rasterizer, TargetSpec};

use crate::mesh::Mesh;

/// The carried marks as a display list of their own.
pub(crate) fn list(page: &DisplayList, mesh: &Mesh) -> DisplayList {
    let mut list = DisplayList::new(page.page_size);
    for command in &mesh.carried {
        list.push(command.clone());
    }
    list
}

pub(crate) fn draw(list: &DisplayList, target: TargetSpec) -> Raster {
    render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the oracle draws every carried mark")
}

/// `tests/corpus.rs`'s three bounds — mean error, worst 32-pixel tile, structural similarity
/// — under which the corpus gate counts a page as agreeing with the oracle.
pub(crate) fn agreement(reference: &Raster, drawn: &[u8]) -> String {
    let ours = Raster {
        width: reference.width,
        height: reference.height,
        format: RasterFormat::Rgba8,
        data: drawn.to_vec(),
    };
    match raster_compare::compare(reference, &ours) {
        Ok(c) => format!(
            "mean {:.3}, worst tile {:.2}, similarity {:.4}: {}",
            c.mean_error,
            c.worst_tile_error,
            c.structural_similarity,
            if c.mean_error < 1.5 && c.worst_tile_error < 7.0 && c.structural_similarity > 0.99 {
                "inside the corpus gate's bounds"
            } else {
                "outside the corpus gate's bounds"
            }
        ),
        Err(error) => format!("not comparable: {error}"),
    }
}

/// FNV-1a over a frame's bytes, the digest `examples/zoom_frame.rs` prints.
pub(crate) fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// How the probe's pixels stand against the oracle's: how many differ, by how much, and how
/// many pixels the oracle inks that the probe leaves white — §10.7.4's disappearance — or the
/// other way round.
pub(crate) fn differ(reference: &[u8], drawn: &[u8]) -> String {
    let (mut differing, mut past_16, mut worst, mut vanished, mut extra, mut inked) =
        (0_u64, 0_u64, 0_u8, 0_u64, 0_u64, 0_u64);
    for (r, d) in reference.chunks_exact(4).zip(drawn.chunks_exact(4)) {
        let largest = r
            .iter()
            .zip(d)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0);
        let white = |p: &[u8]| p[..3].iter().all(|&c| c == 255);
        inked += u64::from(!white(r));
        if largest > 0 {
            differing += 1;
            past_16 += u64::from(largest > 16);
            worst = worst.max(largest);
            vanished += u64::from(!white(r) && white(d));
            extra += u64::from(white(r) && !white(d));
        }
    }
    format!(
        "{differing} pixel(s) differ, {past_16} by more than 16 levels, worst {worst}; \
         {vanished} of {inked} inked pixel(s) left white, {extra} white pixel(s) inked"
    )
}

pub(crate) fn png(path: &std::path::Path, (width, height): (u32, u32), rgba: &[u8]) {
    let file = std::fs::File::create(path).expect("the dump directory is writable");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(rgba))
        .expect("the PNG is written");
}
