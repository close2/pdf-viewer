//! A pixel of a nearest-sampled image is coloured by the sample its centre maps back to on the
//! page, whichever strip draws it.
//!
//! # What the expected values come from
//!
//! ISO 32000-2 §10.7.4, of a sampled image:
//!
//! > The position of the centre of such a pixel -in other words, the point whose coordinate values
//! > have fractional parts of one-half -shall be mapped back into source space to determine how to
//! > colour the pixel.
//!
//! The pixel is the page's — a pixel boundary falls on an integer coordinate *in device space* —
//! and how a page is divided into strips is this program's choice, so the sample a centre maps to
//! is a function of the page's device coordinates and of nothing else. The witness is
//! `bug1799927.pdf` at four times: a line of 7 × 10 stencils the edge between whose sample rows 1
//! and 2 lies within a rounding of the centre of device row 2361, so a strip that mapped the centre
//! from its own first row took row 1 and drew the row white where the page takes row 2 and draws
//! it black (ADR 2100).

#![expect(
    clippy::arithmetic_side_effects,
    reason = "test code: indices into a raster of a few million pixels"
)]

use pdf_render::{
    BlendMode, Command, DisplayList, Image, Raster, Rasterizer as _, SampleAlpha, Size, TargetSpec,
};
use render_cpu::CpuRasterizer;

/// The witness's page, A4 in points.
const PAGE: Size = Size::new(595.0, 842.0);

/// The witness's magnification.
const SCALE: f32 = 4.0;

/// One of the witness's stencils: seven by ten samples, the third row's middle three opaque black
/// and the rest transparent, so a centre that maps to the row before draws nothing.
fn stencil() -> Image {
    let mut data = vec![0_u8; 7 * 10 * 4];
    for column in 2..5 {
        data[(2 * 7 + column) * 4 + 3] = 255;
    }
    Image {
        width: 7,
        height: 10,
        data: data.into(),
        interpolate: false,
        sample_alpha: SampleAlpha::Shape,
    }
}

/// The witness's page with one of its stencils where the file places it.
fn page() -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    list.push(Command::Image {
        image: stencil().into(),
        transform: pdf_render::Transform::new(-4.361, 0.0, 0.0, -6.23, 332.653, 256.609),
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// Every pixel at which two drawings differ, as its column and row.
fn differences(one: &Raster, other: &Raster) -> Vec<(usize, usize)> {
    let width = one.width as usize;
    one.data
        .chunks_exact(4)
        .zip(other.data.chunks_exact(4))
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(index, _)| (index % width, index / width))
        .collect()
}

/// The stencil drawn in one strip and with one cut at rows from the planner's floor down to the
/// stencil's own top: the bytes are the page's at each. On the tree before ADR 2100 the first cut,
/// at row 64, already moved eight pixels of row 2361 from black to white.
#[test]
fn a_cut_above_a_nearest_sampled_image_draws_the_page() {
    let list = page();
    let target = TargetSpec::for_page(&list, SCALE, 1 << 26).expect("a target");
    let whole = CpuRasterizer::new()
        .with_strips(1)
        .rasterize(&list, target)
        .expect("an image draws");
    let mut moved = Vec::new();
    for cut in (64..2341).step_by(37) {
        let strips = CpuRasterizer::new()
            .with_cuts(&[0, cut, target.height])
            .rasterize(&list, target)
            .expect("an image draws");
        let apart = differences(&whole, &strips);
        if !apart.is_empty() {
            moved.push((cut, apart.len(), apart[0]));
        }
    }
    assert!(
        moved.is_empty(),
        "cuts at which the stencil is not the page's (cut, pixels, first): {moved:?}"
    );
}

/// A ten-by-ten image of a hundred distinct samples, one device pixel a sample, placed at a
/// half-pixel origin, where every pixel centre lies exactly on the edge two samples share: each
/// interior pixel takes the sample whose upper-left corner its centre is (§8.9.4 gives a sample
/// one square unit from its own indices, and §10.7.4 identifies a pixel by its corner of minimum
/// coordinates). At every origin from half a pixel to two hundred and a half, and in one strip and
/// in four, so a centre computed by dividing first and multiplying back, which loses the exact
/// edge at some origins and not at others, is seen.
#[test]
fn a_centre_on_a_samples_edge_takes_the_sample_whose_corner_it_is() {
    let colour = |index: usize| -> [u8; 4] {
        let index = u8::try_from(index).expect("a hundred samples");
        [index * 2, 255 - index * 2, (index % 10) * 25, 255]
    };
    let data: Vec<u8> = (0..100).flat_map(colour).collect();
    let image = Image {
        width: 10,
        height: 10,
        data: data.into(),
        interpolate: false,
        sample_alpha: SampleAlpha::Shape,
    };
    let side = 260.0;
    for origin in 0..200_u16 {
        let at = f32::from(origin) + 0.5;
        let mut list = DisplayList::new(Size::new(side, side));
        // The unit square's top edge at device row `at`: the page's own transform puts user `y` at
        // device row `side − y`.
        list.push(Command::Image {
            image: image.clone().into(),
            transform: pdf_render::Transform::new(10.0, 0.0, 0.0, 10.0, at, side - at - 10.0),
            alpha: 1.0,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("a target");
        for strips in [1, 4] {
            let raster = CpuRasterizer::new()
                .with_strips(strips)
                .rasterize(&list, target)
                .expect("an image draws");
            // The pixels whose centres are inside the square and not on its edge: columns and rows
            // `origin + 1` to `origin + 9`, whose centres are sample edges 1 to 9.
            for row in 1..10_usize {
                for column in 1..10_usize {
                    let (x, y) = (usize::from(origin) + column, usize::from(origin) + row);
                    let at = (y * raster.width as usize + x) * 4;
                    assert_eq!(
                        raster.data[at..at + 4],
                        colour(row * 10 + column),
                        "origin {origin}.5, {strips} strips, pixel ({x}, {y})"
                    );
                }
            }
        }
    }
}
