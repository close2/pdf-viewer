//! A sampled (type 1) shading is resolved at the device's grid, not at the display list's.
//!
//! ISO 32000-2 §8.7.4.5.2 defines the colour of a function-based shading "at every point in
//! the domain", so the only grid a raster can honestly stand on is the device's own — one
//! cell per device pixel of the drawn domain. The display list therefore carries a producer
//! (`pdf_render::DeferredColours`) rather than samples, and the backend resolves it per draw.
//!
//! # What the expected values come from
//!
//! [`test_scenes::sampled_colour_at`] is the closed form the scene's producer evaluates, so
//! every expected pixel below is that arithmetic at the pixel's own centre, mapped back
//! through the domain placement — never a value read off any backend. The scene's twelve
//! waves are sized so that a grid fixed at 128 cells when the list was built (the old
//! construction) interpolates up to eleven levels away from the closed form, which is what
//! lets these assertions fail when the resolution moves anywhere but the device.
//!
//! # Why the cases run at two scales
//!
//! Trap 2's rule, one axis over: a fixed grid can be made to pass at the one scale it was
//! chosen for. Rendering the *same display list* at 1× and 4× also asserts the shading
//! analogue of `zooming_rasterises_again_without_interpreting_again` — zooming re-resolves
//! the function instead of magnifying a raster the list froze.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic with a message is the intended failure mode, and the \
              arithmetic is on literal page coordinates that cannot overflow"
)]
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "every cast is a clamped colour channel or a device pixel index of a small \
              test raster"
)]

use std::sync::{Arc, Mutex};

use pdf_render::{
    BlendMode, Color, ColourGrid, ColoursAtDeviceScale, Command, DeferredColours, DisplayList,
    FillRule, Paint, Patch, Path, PathCommand, Point, Raster, Rasterizer, Shading, ShadingKind,
    Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;
use test_scenes::{SAMPLED_PAGE, sampled_colour_at, sampled_shading};

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// How far a channel may sit from the closed form.
///
/// At one cell per device pixel the producer's cell centres coincide with the pixel
/// centres, so the pattern's bilinear filter reads each texel exactly and only eight-bit
/// quantisation remains. The old fixed 128-cell grid is eleven levels away at the waves'
/// inflections, so the two regimes cannot be confused.
const TOLERANCE: i32 = 3;

/// The channel values the closed form gives at the centre of device pixel `(x, y)`.
fn expected_at(x: u32, y: u32, scale: f32) -> (u8, u8, u8) {
    // The raster's row zero is the page's top edge; the domain covers page (10, 10)-(410, 410).
    let page_x = (x as f32 + 0.5) / scale;
    let page_y = SAMPLED_PAGE - (y as f32 + 0.5) / scale;
    let colour = sampled_colour_at((page_x - 10.0) / 400.0, (page_y - 10.0) / 400.0);
    let level = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
    (level(colour.r), level(colour.g), level(colour.b))
}

/// Reads the pixel at device `(x, y)`.
fn at(raster: &Raster, x: u32, y: u32) -> (u8, u8, u8) {
    assert!(x < raster.width && y < raster.height, "inside the raster");
    let index = ((y as usize) * (raster.width as usize) + (x as usize)) * 4;
    let p = &raster.data[index..index + 3];
    (p[0], p[1], p[2])
}

/// §8.7.4.5.2's "the colour at every point in the domain", checked against the arithmetic.
///
/// The sample points walk across the waves, so among them are points near the inflections
/// where linear interpolation between the old grid's cells is furthest from the function.
#[test]
fn a_sampled_shading_carries_the_functions_value_at_each_device_pixel() {
    let list = sampled_shading();
    for scale in [1.0f32, 4.0] {
        let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("valid target");
        let raster = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("a sampled shading is supported");

        // Interior device pixels spread across the drawn domain, clear of the shape's own
        // antialiased edges.
        for step in 0..20u32 {
            let x = ((30.0 + f32::from(u16::try_from(step).expect("small")) * 17.0) * scale) as u32;
            let y = ((40.0 + f32::from(u16::try_from(step).expect("small")) * 11.0) * scale) as u32;
            let wanted = expected_at(x, y, scale);
            let got = at(&raster, x, y);
            let far = |a: u8, b: u8| (i32::from(a) - i32::from(b)).abs() > TOLERANCE;
            assert!(
                !(far(got.0, wanted.0) || far(got.1, wanted.1) || far(got.2, wanted.2)),
                "scale {scale}, device ({x}, {y}): got {got:?}, expected {wanted:?} \
                 within {TOLERANCE}"
            );
        }
    }
}

/// A producer that answers a flat colour and records every patch it was asked for.
#[derive(Debug)]
struct RecordingSource(Mutex<Vec<Patch>>);

impl ColoursAtDeviceScale for RecordingSource {
    fn colours(&self, patch: Patch) -> ColourGrid {
        self.0
            .lock()
            .expect("no poisoned lock in a test")
            .push(patch);
        let grid = patch.grid;
        ColourGrid {
            width: grid.width,
            height: grid.height,
            pixels: vec![Color::rgb(0.2, 0.4, 0.6); (grid.width as usize) * (grid.height as usize)]
                .into(),
            covers: [0.0, 1.0, 0.0, 1.0],
        }
    }

    fn is_opaque(&self) -> bool {
        true
    }
}

/// The shading equivalent of `zooming_rasterises_again_without_interpreting_again`.
///
/// One display list, rasterised at 1× and then at 4×: the producer must be asked again at
/// the finer grid, because the grid is the device's and the device changed. A construction
/// that froze the resolution into the list would answer both frames from one grid, which is
/// exactly what `FUNCTION_GRID` used to do.
#[test]
fn zooming_resolves_the_shading_again_without_rebuilding_the_list() {
    let source = Arc::new(RecordingSource(Mutex::new(Vec::new())));

    let mut list = DisplayList::new(Size::new(100.0, 100.0));
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 10.0)));
    path.push(PathCommand::LineTo(Point::new(90.0, 10.0)));
    path.push(PathCommand::LineTo(Point::new(90.0, 90.0)));
    path.push(PathCommand::LineTo(Point::new(10.0, 90.0)));
    path.push(PathCommand::Close);
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Shading(Arc::new(Shading {
            kind: Arc::new(ShadingKind::Sampled {
                domain: [0.0, 1.0, 0.0, 1.0],
                source: DeferredColours::new(Arc::clone(&source) as Arc<dyn ColoursAtDeviceScale>),
                // The oracle draws the producer's grid whatever else a shading carries, and
                // this file is about the grid it asks for.
                program: None,
            }),
            // The unit domain across 80 page units.
            transform: Transform::new(80.0, 0.0, 0.0, 80.0, 10.0, 10.0),
        })),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    let grids_at = |scale: f32| {
        source.0.lock().expect("no poisoned lock in a test").clear();
        let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("valid target");
        CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("a sampled shading is supported");
        let asked = source.0.lock().expect("no poisoned lock in a test").clone();
        assert!(!asked.is_empty(), "the producer was asked at scale {scale}");
        asked
    };

    let coarse = grids_at(1.0);
    let fine = grids_at(4.0);
    // The domain covers 80 device pixels at 1x, so every request at 4x is exactly four
    // times as fine — the grid follows the device, not the list.
    assert!(
        coarse
            .iter()
            .all(|patch| patch.grid.width == 80 && patch.grid.height == 80),
        "at 1x the domain covers 80 pixels; the producer was asked for {coarse:?}"
    );
    assert!(
        fine.iter()
            .all(|patch| patch.grid.width == 320 && patch.grid.height == 320),
        "at 4x the same list must ask four times as fine; the producer was asked for {fine:?}"
    );
    // The target is the whole page at both scales, so the domain is entirely on it and the
    // block is the whole lattice: a clip that narrowed a grid nothing is hiding would be a
    // clip that had lost a pixel.
    #[expect(
        clippy::float_cmp,
        reason = "the whole domain is `Patch::whole`'s own literal, set rather than computed, \
                  so equality with it is an identity check and not a tolerance"
    )]
    let unclipped = |patch: &Patch| patch.within == [0.0, 1.0, 0.0, 1.0];
    assert!(
        coarse.iter().chain(fine.iter()).all(unclipped),
        "a target containing the whole domain clips nothing away"
    );
}
