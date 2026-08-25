//! What each backend's image filter does to the *colour* of a partly transparent sample.
//!
//! ISO 32000-2 §8.9.6.2 states the rule this measures, and states it as a `shall`:
//!
//! > If image interpolation (see 8.9.5.3, "Image interpolation") is requested during stencil
//! > masking, the effect shall be to smooth the edges of the mask, not to interpolate the
//! > painted colour values.
//!
//! A stencil decodes to the fill colour where its bits mark and `[0, 0, 0, 0]` where they do
//! not, so a filter that interpolates the four RGBA components *as they are stored* mixes the
//! painted colour with the black those cleared samples carry — and every edge of the stencil
//! comes out half black. Filtering the same raster premultiplied gives the painted colour at
//! partial coverage exactly, which is the clause's own distinction between smoothing the mask
//! and interpolating the colour. `pdf_render::Image::average_block` states the same rule for
//! the reduction this crate does itself.
//!
//! Nothing else in this tree can see the difference, and the reason is worth knowing: the
//! image scenes in both cross-backend suites are **opaque**, and on an opaque raster straight
//! and premultiplied filtering are the same arithmetic. So this example draws the one scene
//! that separates them — an image whose samples alternate between an opaque colour and a
//! cleared one — and prints, for each backend, the colour it puts on the pixels that are
//! neither fully covered nor fully clear.
//!
//! ```sh
//! cargo run --release -p render-quorra --example filtered_edge_colour
//! ```
//!
//! What a correct backend prints is the fill colour at every alpha. A backend filtering
//! straight alpha prints roughly half of it, and the "worst gap" line is how far.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "an example: its whole output is stdout, an absent device should fail loudly, \
              every index is inside a 200x200 raster this file sized itself, and the same \
              bound is what keeps the pixel arithmetic below it far from any overflow"
)]

use pdf_render::{BlendMode, Command, DisplayList, Image, Rasterizer, Size, TargetSpec, Transform};

/// The opaque colour the fixture's marked samples carry, and the one every backend owes back.
const PAINTED: [u8; 4] = [255, 0, 0, 255];

fn main() {
    // Four by four, alternating between `PAINTED` and a cleared sample — which is exactly what
    // `pdf_model::image::decode` produces for a stencil whose current colour is red.
    let mut data = Vec::with_capacity(4 * 4 * 4);
    for row in 0..4u32 {
        for column in 0..4u32 {
            if (row + column) % 2 == 0 {
                data.extend_from_slice(&PAINTED);
            } else {
                data.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    let mut list = DisplayList::new(Size::new(200.0, 200.0));
    list.push(Command::Image {
        image: Image {
            width: 4,
            height: 4,
            data: data.into(),
            // §8.9.5.3's entry, which is the condition §8.9.6.2's sentence is stated under.
            interpolate: true,
        }
        .into(),
        transform: Transform::scale(160.0, 160.0).then(Transform::translate(20.0, 20.0)),
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a 200x200 target");
    // `Medium::NONE` throughout, so what is read back is the mark's own colour rather than the
    // mark composited over a background — which would hide the difference in the blend.
    let rasters: [(&str, pdf_render::Raster); 3] = [
        (
            "cpu",
            render_cpu::CpuRasterizer::new()
                .with_medium(pdf_render::Medium::NONE)
                .rasterize(&list, target)
                .expect("the CPU oracle draws every scene"),
        ),
        (
            "vello",
            render_gpu::GpuRasterizer::new_headless()
                .expect("a headless vello device")
                .with_medium(pdf_render::Medium::NONE)
                .rasterize(&list, target)
                .expect("vello draws the scene"),
        ),
        (
            "quorra",
            render_quorra::QuorraRasterizer::new_headless()
                .expect("a headless quorra device")
                .with_medium(pdf_render::Medium::NONE)
                .rasterize(&list, target)
                .expect("quorra draws the scene"),
        ),
    ];

    // One scanline through the middle of the image, where the filter ramps between cells.
    let row = 100u32;
    for (name, raster) in &rasters {
        let mut worst: (u32, u8, [u8; 4]) = (0, 0, [0; 4]);
        let mut partial = 0u32;
        for x in 20..180 {
            let at = ((row * raster.width + x) * 4) as usize;
            let pixel = [
                raster.data[at],
                raster.data[at + 1],
                raster.data[at + 2],
                raster.data[at + 3],
            ];
            // Fully covered and fully clear pixels say nothing about the filter.
            if pixel[3] <= 24 || pixel[3] >= 232 {
                continue;
            }
            partial = partial.saturating_add(1);
            let gap = PAINTED[0].saturating_sub(pixel[0]);
            if gap > worst.1 {
                worst = (x, gap, pixel);
            }
        }
        println!(
            "{name:>6}: {partial} partly covered pixels, worst departure from the painted \
             colour {} at x={} ({:?})",
            worst.1, worst.0, worst.2
        );
    }
}
