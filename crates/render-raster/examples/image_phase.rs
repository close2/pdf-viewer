//! Whether each backend filters an image drawn at **exactly one device pixel per sample** — and
//! §10.7.4 says which answer is right: the centre of each device pixel is mapped back into source
//! space to decide its colour, and there is no averaging over the pixel area. At a native placement
//! every device pixel maps to one sample, so the clause's answer is that sample, unfiltered;
//! `pdf_render::Image::is_smoothed` answers `false` there before it reads `/Interpolate` (ADR 1107
//! section 3), and ADR 0025's departure stays scoped to the *reduced* case, where several samples
//! share a pixel. Round 1097 first read this the other way, as a defect of the oracle's; the clause
//! decided it, and this example is the measurement that separates the two answers.
//!
//! # The placement that separates them
//!
//! An image drawn at its own sample resolution composes with `render_cpu`'s pattern transform to a
//! **pure translation**, and `tiny-skia`'s `Pattern::push_stages` point-samples every such
//! placement (`tiny-skia-0.12.0/src/shaders/pattern.rs`: `if ts.is_identity() || ts.is_translate()
//! { quality = Nearest }`, with its whole-pixel refinement unreachable underneath). Now that the
//! request is `Nearest` at a native placement too, request and delivery agree and the oracle draws
//! what §10.7.4 states. `render_raster::scene` hands quorra the samples with a **mirrored copy** of
//! the old rule deciding (ADR 0702), and quorra filters — the departure `doc/QUORRA_FEEDBACK.md`
//! section 47 asks about, with this example as its measurement.
//!
//! What this prints is the two answers as numbers. The image is eight rows of alternating black
//! and white drawn onto eight device rows at ten sub-pixel offsets, and **the ink column is the
//! tell**: a filter's answer moves with the phase, so its ink walks a straight line, while the
//! clause's point sample changes only where the nearest sample flips, so its ink is a staircase.
//! At a half-sample phase the filter puts one uniform grey down — the `levels` column reads 1
//! there and 2 for the point sample, which is still drawing the stripes the document states.
//!
//! The control is the same image and the same offsets under a scale of 1.5, which is not a
//! native placement, so both backends filter (ADR 0025) and agree.
//!
//! ```sh
//! cargo run --release -p render-raster --example image_phase
//! ```

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "an example: its whole output is stdout, an absent device must fail loudly rather \
              than be reported as a backend's answer, every index and sum is inside a raster \
              this file sized itself, and the casts are between that raster's sides and the page \
              coordinates this file states — a two-figure constant either way, positive by \
              construction and exact in both representations"
)]

use std::collections::BTreeSet;

use pdf_render::{
    BlendMode, Command, DisplayList, Image, Raster, Rasterizer, SampleAlpha, Size, TargetSpec,
    Transform,
};

/// The image's side in samples, and the device side it covers at a scale of one.
const SIDE: u32 = 8;
/// The page, big enough to hold the placement at either scale with room around it.
const PAGE: f32 = 32.0;
/// Where the placement starts before the sub-pixel offset is added.
const ORIGIN: f32 = 8.0;

/// Eight rows alternating between black and white, opaque throughout.
fn stripes() -> Image {
    let mut data = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for row in 0..SIDE {
        let level = if row % 2 == 0 { 0 } else { 255 };
        for _ in 0..SIDE {
            data.extend_from_slice(&[level, level, level, 255]);
        }
    }
    Image {
        width: SIDE,
        height: SIDE,
        data: data.into(),
        // ISO 32000-2 §8.9.5.3's entry. It is "only a hint" there, and `smoothed` takes it as one
        // that turns filtering on; stating it here keeps both cases on the same side of that rule
        // so that the *scale* is the only thing varying.
        interpolate: true,
        // §11.6.4.2: an image with no mask has shape 1.0 inside its rectangle.
        sample_alpha: SampleAlpha::Shape,
    }
}

/// One page holding the striped image at `scale` device pixels per sample, offset by `offset`.
fn page(scale: f32, offset: f32) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let side = SIDE as f32 * scale;
    list.push(Command::Image {
        image: stripes().into(),
        transform: Transform::scale(side, side)
            .then(Transform::translate(ORIGIN + offset, ORIGIN + offset)),
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The distinct grey levels the placement's interior holds, and the ink the backend put there.
///
/// The window is the device rectangle the placement covers, less its boundary pixels: a pixel the
/// image only partly covers is paper as well as image, and what is being measured is the filter
/// rather than the edge. `TargetSpec::for_page` flips about the page's height (trap 12a), so the
/// device rows run from the other end of the page.
fn levels(raster: &Raster, scale: f32, offset: f32) -> (usize, f64) {
    let side = SIDE as f32 * scale;
    let left = (ORIGIN + offset).ceil() as u32;
    let right = (ORIGIN + offset + side).floor() as u32;
    let bottom = (PAGE - (ORIGIN + offset)).floor() as u32;
    let top = (PAGE - (ORIGIN + offset + side)).ceil() as u32;
    let mut seen = BTreeSet::new();
    let mut ink = 0.0_f64;
    for y in top..bottom {
        for x in left..right {
            let at = ((y * raster.width + x) * 4) as usize;
            seen.insert(raster.data[at]);
            ink += f64::from(255 - raster.data[at]);
        }
    }
    (seen.len(), ink / 255.0)
}

fn main() {
    let mut cpu = render_cpu::CpuRasterizer::new();
    let mut quorra =
        render_raster::QuorraRasterizer::new_headless().expect("a headless raster device");

    for (what, scale) in [("1:1", 1.0_f32), ("control, 1.5:1", 1.5_f32)] {
        println!(
            "{what} — eight alternating rows on {} device rows, both backends asked to filter",
            (SIDE as f32 * scale).ceil()
        );
        println!("  offset   cpu levels  cpu ink     raster levels  raster ink");
        for step in 0..10_i16 {
            let offset = f32::from(step) / 10.0;
            let list = page(scale, offset);
            let target = TargetSpec::for_page(&list, 1.0, 1 << 24).expect("a 32x32 target");
            let ours = cpu.rasterize(&list, target).expect("the oracle draws it");
            let theirs = quorra.rasterize(&list, target).expect("raster draws it");
            let (cl, ci) = levels(&ours, scale, offset);
            let (rl, ri) = levels(&theirs, scale, offset);
            println!("   {offset:>4.2}   {cl:>10}  {ci:>9.3}   {rl:>13}  {ri:>10.3}");
        }
    }
}
