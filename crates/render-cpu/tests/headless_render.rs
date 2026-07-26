//! Spike A: prove that a page can be rendered headlessly and reproducibly.
//!
//! This is the foundation the comparison harness stands on. Three properties must
//! hold before any PDF-specific rendering work is worth starting:
//!
//! 1. A display list renders to pixels with no window, no display server and no GPU.
//! 2. The output is **byte-identical** across runs. Without that, golden-image tests
//!    and cross-backend diffing are both meaningless.
//! 3. The page-to-device transform places geometry where the document says, including
//!    the y-flip between PDF's bottom-left origin and a raster's top-left.
//!
//! A PNG is written to `target/spike/` for visual inspection. Note that the *tests*
//! assert on raw pixel bytes rather than on encoded PNG bytes: PNG encoding involves
//! compression settings that are not part of what we are trying to pin down, so the
//! image is a debugging artefact, not the thing under test.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic with a message is the intended failure mode here, and \
              the arithmetic is on literal page dimensions that cannot overflow"
)]

use pdf_render::{Raster, Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// Reads a pixel as `(r, g, b, a)`.
fn pixel(raster: &Raster, x: u32, y: u32) -> (u8, u8, u8, u8) {
    assert!(
        x < raster.width && y < raster.height,
        "({x},{y}) is outside the raster"
    );
    // Row-major, 4 bytes per pixel, no row padding — as documented on `Raster`.
    let index = ((y as usize) * (raster.width as usize) + (x as usize)) * 4;
    let p = &raster.data[index..index + 4];
    (p[0], p[1], p[2], p[3])
}

/// Renders the shared `basic` scene with antialiasing disabled, so that
/// axis-aligned fills cover whole pixels and assertions can be exact.
fn render_aliased() -> Raster {
    let list = test_scenes::basic();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_anti_alias(false)
        .rasterize(&list, target)
        .expect("basic scene is supported")
}

fn render_at(scale: f32) -> Raster {
    let list = test_scenes::basic();
    let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("A4 target is valid");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("scene uses only supported commands")
}

#[test]
fn renders_headlessly_without_a_display_or_gpu() {
    let raster = render_at(1.0);
    assert_eq!((raster.width, raster.height), (595, 842));
    assert_eq!(
        raster.data.len(),
        595 * 842 * 4,
        "four bytes per pixel, no row padding"
    );
}

/// The property the whole golden-image and cross-backend strategy depends on.
#[test]
fn output_is_byte_identical_across_runs() {
    let first = render_at(150.0 / 72.0);
    let second = render_at(150.0 / 72.0);
    assert_eq!(
        first.data, second.data,
        "CPU rasterisation must be reproducible"
    );
}

/// Antialiasing is off here so that a fill covers whole pixels exactly, which lets
/// these assertions be exact rather than approximate.
#[test]
fn fill_lands_where_the_page_coordinates_say() {
    let raster = render_aliased();

    // Page (200,200) is inside the red square. Device y is flipped: 842 - 200 = 642.
    assert_eq!(
        pixel(&raster, 200, 642),
        (255, 0, 0, 255),
        "centre of the red square"
    );

    // Page (200,400) is above the square, so it stays background white.
    assert_eq!(
        pixel(&raster, 200, 442),
        (255, 255, 255, 255),
        "above the red square"
    );
}

/// The y-flip is the single easiest thing to get wrong, and getting it wrong renders
/// a page that is vertically mirrored but otherwise plausible.
#[test]
fn the_page_is_not_vertically_mirrored() {
    let raster = render_aliased();

    // The red square sits low on the page (y 100..300 from the bottom), so it must
    // appear in the LOWER half of the raster. Under a mirrored transform the same
    // pixel would be white and its reflection red.
    assert_eq!(
        pixel(&raster, 200, 642),
        (255, 0, 0, 255),
        "red square in the lower half"
    );
    assert_eq!(
        pixel(&raster, 200, 200),
        (255, 255, 255, 255),
        "upper half is empty here"
    );
}

#[test]
fn clips_restrict_painting_to_their_interior() {
    let raster = render_aliased();

    // Page (425,425) is inside both the green square and its clip.
    assert_eq!(
        pixel(&raster, 425, 842 - 425),
        (0, 255, 0, 255),
        "inside the clip"
    );

    // Page (475,475) is inside the green square but OUTSIDE the clip, so the clip is
    // doing work. Without clipping this pixel would also be green.
    assert_eq!(
        pixel(&raster, 475, 842 - 475),
        (255, 255, 255, 255),
        "clipped away"
    );
}

#[test]
fn strokes_are_drawn_with_their_width() {
    let raster = render_aliased();

    // Centre of the 10-unit-wide line at page y=600.
    assert_eq!(
        pixel(&raster, 300, 842 - 600),
        (0, 0, 255, 255),
        "on the stroke"
    );
    // 20 units above it, clear of a 10-wide stroke centred on the path.
    assert_eq!(
        pixel(&raster, 300, 842 - 620),
        (255, 255, 255, 255),
        "clear of the stroke"
    );
}

/// Writes a PNG for visual inspection. Not an assertion about encoded bytes — see the
/// module comment — but a failure to encode at all is still worth catching.
#[test]
fn writes_an_inspectable_png() {
    let raster = render_at(150.0 / 72.0);

    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("spike");
    std::fs::create_dir_all(&dir).expect("target tmpdir is writable");
    let path = dir.join("headless_render.png");

    let file = std::fs::File::create(&path).expect("can create the artefact");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), raster.width, raster.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .expect("header is valid")
        .write_image_data(&raster.data)
        .expect("pixel data matches the declared dimensions");

    let written = std::fs::metadata(&path).expect("artefact exists").len();
    assert!(written > 0, "PNG at {} is empty", path.display());
    println!("wrote {} ({written} bytes)", path.display());
}
