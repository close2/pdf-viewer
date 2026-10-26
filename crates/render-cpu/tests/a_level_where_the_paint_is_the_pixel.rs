//! Where an opaque paint covers a pixel wholly, the oracle writes the level nearest each
//! component's exact value — ADR 2311's rule, which the device writes (ADRs 2324, 2325) — whether
//! the paint is a solid colour or an axial or radial ramp, and whatever shape carries it.
//!
//! ISO 32000-2 §10.7 leaves the conversion of a colour to the device's bits to the device, so the
//! rule is this tree's documented choice and the expected level is computed here from it: twice the
//! level as the floor of `510 c` in `f64`, where a 24-bit significand times 510 is exact, and halved
//! rounding up. The greys are the ones whose `f32` product with 255 lands on a half: the library's
//! `memset` of a run of whole pixels breaks every such half upwards and its high-precision store
//! breaks half of them to the even level, so each case below is a level the oracle missed before
//! ADRs 2356 and 2357.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic with a message is the intended failure mode, and the \
              arithmetic is on literal page coordinates and levels that cannot overflow"
)]
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "every cast is a clamped colour level or a device pixel index of a small test raster"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Ramp,
    Raster, Rasterizer, Shading, ShadingKind, Size, Stop, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// The level nearest `component`'s exact value, half away from zero.
fn level(component: f32) -> u8 {
    let doubled = (f64::from(component) * 510.0).floor() as u32;
    doubled.div_ceil(2) as u8
}

/// The greys within four ulps of each half level `(k + ½) / 255` that the library wrote off their
/// level: its `memset`'s `(c · 255 + ½)` in `f32` truncated, or its store's nearest-even.
fn greys_the_library_misses() -> Vec<f32> {
    let mut greys = Vec::new();
    for k in 0..255_u16 {
        let half = ((f32::from(k) + 0.5) / 255.0).to_bits();
        for bits in half - 4..=half + 4 {
            let grey = f32::from_bits(bits);
            let product = grey * 255.0;
            let truncated = (product + 0.5) as u8;
            let stored = product.round_ties_even() as u8;
            if truncated != level(grey) || stored != level(grey) {
                greys.push(grey);
            }
        }
    }
    greys
}

/// A closed path through `points`.
fn polygon(points: &[(f32, f32)]) -> Arc<Path> {
    let mut path = Path::new();
    for (index, &(x, y)) in points.iter().enumerate() {
        let point = Point::new(x, y);
        path.push(if index == 0 {
            PathCommand::MoveTo(point)
        } else {
            PathCommand::LineTo(point)
        });
    }
    path.push(PathCommand::Close);
    Arc::new(path)
}

/// One fill of `path` with `paint` on a 40-unit page, drawn at 1x.
fn draw(path: Arc<Path>, paint: Paint) -> Raster {
    let mut list = DisplayList::new(Size::new(40.0, 40.0));
    list.push(Command::Fill {
        path,
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("a fill is supported")
}

/// The red, green and blue of device pixel `(x, y)`.
fn at(raster: &Raster, x: u32, y: u32) -> (u8, u8, u8) {
    let index = ((y as usize) * (raster.width as usize) + (x as usize)) * 4;
    let pixel = &raster.data[index..index + 3];
    (pixel[0], pixel[1], pixel[2])
}

/// A ramp of one grey from end to end.
fn flat_ramp(grey: f32) -> Ramp {
    let colour = Color::rgb(grey, grey, grey);
    Ramp {
        stops: Arc::from([Stop { at: 0.0, colour }, Stop { at: 1.0, colour }]),
    }
}

/// ADR 2357: an opaque solid paint's interior is the level, as a rectangle — the library's `memset`
/// — and as a triangle, whose interior the high-precision store writes.
#[test]
fn an_opaque_fill_is_the_level_nearest_its_colour_whatever_its_shape() {
    let greys = greys_the_library_misses();
    assert!(
        greys.len() > 100,
        "{} greys the library misses",
        greys.len()
    );
    let rectangle = polygon(&[(5.0, 5.0), (35.0, 5.0), (35.0, 35.0), (5.0, 35.0)]);
    let triangle = polygon(&[(2.0, 2.0), (38.0, 2.0), (2.0, 38.0)]);
    for grey in greys {
        let want = level(grey);
        let paint = Paint::Solid(Color::rgb(grey, grey, grey));
        let square = draw(Arc::clone(&rectangle), paint.clone());
        let corner = draw(Arc::clone(&triangle), paint);
        for (raster, shape) in [(&square, "rectangle"), (&corner, "triangle")] {
            assert_eq!(
                at(raster, 10, 30),
                (want, want, want),
                "a {shape} of {grey}, {} levels",
                f64::from(grey) * 255.0
            );
        }
    }
}

/// ADR 2356: an opaque axial and an opaque radial ramp are each the level at a pixel they cover
/// wholly, as the device's shading lane writes them (ADR 2310).
#[test]
fn an_opaque_ramp_is_the_level_nearest_its_colour() {
    let square = polygon(&[(5.0, 5.0), (35.0, 5.0), (35.0, 35.0), (5.0, 35.0)]);
    for grey in greys_the_library_misses() {
        let want = level(grey);
        for kind in [
            ShadingKind::Axial {
                start: Point::new(0.0, 0.0),
                end: Point::new(40.0, 13.0),
                ramp: flat_ramp(grey),
                extend: (true, true),
            },
            ShadingKind::Radial {
                start: Point::new(20.0, 20.0),
                start_radius: 0.0,
                end: Point::new(21.0, 20.0),
                end_radius: 60.0,
                ramp: flat_ramp(grey),
                extend: (true, true),
            },
        ] {
            let paint = Paint::Shading(Arc::new(Shading {
                background: None,
                kind: Arc::new(kind),
                transform: Transform::IDENTITY,
            }));
            let raster = draw(Arc::clone(&square), paint);
            for (x, y) in [(10, 10), (20, 20), (30, 12)] {
                assert_eq!(
                    at(&raster, x, y),
                    (want, want, want),
                    "a ramp of {grey}, {} levels, at ({x}, {y})",
                    f64::from(grey) * 255.0
                );
            }
        }
    }
}

/// ADR 2356: a ramp's cell is its colour at the `t` of the pixel's centre — a two-stop grey ramp
/// along x, read at three pixels against the clause's interpolation evaluated here.
#[test]
fn a_ramps_cell_is_its_colour_at_the_pixels_own_t() {
    let square = polygon(&[(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)]);
    let (from, to) = (Color::rgb(0.1, 0.2, 0.3), Color::rgb(0.9, 0.6, 0.7));
    let paint = Paint::Shading(Arc::new(Shading {
        background: None,
        kind: Arc::new(ShadingKind::Axial {
            start: Point::new(0.0, 0.0),
            end: Point::new(40.0, 0.0),
            ramp: Ramp {
                stops: Arc::from([
                    Stop {
                        at: 0.0,
                        colour: from,
                    },
                    Stop {
                        at: 1.0,
                        colour: to,
                    },
                ]),
            },
            extend: (false, false),
        }),
        transform: Transform::IDENTITY,
    }));
    let raster = draw(square, paint);
    for x in [0_u16, 13, 39] {
        let t = (f32::from(x) + 0.5) / 40.0;
        let channel = |a: f32, b: f32| level(a + (b - a) * t);
        assert_eq!(
            at(&raster, u32::from(x), 20),
            (
                channel(from.r, to.r),
                channel(from.g, to.g),
                channel(from.b, to.b)
            ),
            "column {x}, t = {t}"
        );
    }
}
