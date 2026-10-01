//! A residue clip meeting a mark as a set, measured against the closed form: the polygon
//! arithmetic that states a set's area in a pixel, the 64-gon clip, and the device arms the
//! meet must agree on — shared by the path lane's fixture and the image lane's (ADR 1467,
//! ADR 1480).

use raster_gpu::{Coverage, Device, Options, Target, Viewport};
use raster_scene::{Affine, ClipId, FillRule, Point, Scene, SceneBuilder, Segment};

use super::probe::alpha;

/// A closed polygon as segments.
pub fn polygon_path(points: &[(f64, f64)]) -> Vec<Segment> {
    let mut path = Vec::with_capacity(points.len() + 1);
    for (i, &(x, y)) in points.iter().enumerate() {
        let p = Point::new(x as f32, y as f32);
        path.push(if i == 0 {
            Segment::MoveTo(p)
        } else {
            Segment::LineTo(p)
        });
    }
    path.push(Segment::Close);
    path
}

pub fn residue_clip(device: &mut Device, builder: &mut SceneBuilder, path: &[Segment]) -> ClipId {
    let outline = device.upload_outline(path).unwrap();
    builder
        .clip(outline, Affine::IDENTITY, FillRule::NonZero, None)
        .unwrap()
}

pub fn rendered(device: &mut Device, scene: &Scene, width: u32, height: u32) -> Vec<u8> {
    device
        .render(
            scene,
            &Viewport::full(width, height, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("renders")
        .into_raster()
        .unwrap()
        .into_pixels()
}

/// The clip: a regular 64-gon of circumradius 6.1 about `(8.3, 8.6)`, wound
/// counter-clockwise in the `y`-down device space the areas below are taken in.
pub fn clip_polygon() -> Vec<(f64, f64)> {
    (0..64)
        .map(|i| {
            let angle = std::f64::consts::TAU * f64::from(i) / 64.0;
            (8.3 + 6.1 * angle.cos(), 8.6 + 6.1 * angle.sin())
        })
        .collect()
}

/// A convex `subject` less everything outside the convex, counter-clockwise `window`
/// (Sutherland–Hodgman), in `f64`.
pub fn clip_convex(subject: &[(f64, f64)], window: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = subject.to_vec();
    for k in 0..window.len() {
        let (from, to) = (window[k], window[(k + 1) % window.len()]);
        let side = |point: (f64, f64)| {
            (to.0 - from.0) * (point.1 - from.1) - (to.1 - from.1) * (point.0 - from.0)
        };
        let input = std::mem::take(&mut out);
        for m in 0..input.len() {
            let (here, next) = (input[m], input[(m + 1) % input.len()]);
            let (sh, sn) = (side(here), side(next));
            if sh >= 0.0 {
                out.push(here);
            }
            if (sh >= 0.0) != (sn >= 0.0) {
                let t = sh / (sh - sn);
                out.push((
                    here.0 + t * (next.0 - here.0),
                    here.1 + t * (next.1 - here.1),
                ));
            }
        }
        if out.is_empty() {
            break;
        }
    }
    out
}

/// A polygon's area, the shoelace sum halved.
pub fn area(points: &[(f64, f64)]) -> f64 {
    (0..points.len())
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

/// The area of a convex polygon inside pixel `(x, y)`, §10.7.4's `[x, x+1) × [y, y+1)`.
pub fn in_pixel(polygon: &[(f64, f64)], x: u32, y: u32) -> f64 {
    let (x, y) = (f64::from(x), f64::from(y));
    area(&clip_convex(
        polygon,
        &[(x, y), (x + 1.0, y), (x + 1.0, y + 1.0), (x, y + 1.0)],
    ))
}

/// The device a fixture is drawn on: the walk's own lane (`Coverage::Gpu`, which keeps
/// every residue-clipped mark on the walk's thread) or the fan-out's (`Coverage::Cpu`), at a
/// thread count.
pub fn device_with(coverage: Coverage, threads: usize) -> Device {
    Device::headless(&Options {
        adapter: Some("llvmpipe".into()),
        coverage,
        encode_threads: threads,
        ..Options::default()
    })
    .expect("llvmpipe is present wherever this suite runs")
}

/// Every arm the exact meet must agree on: the fan-out's commit at one and four threads,
/// and the walk's own tile.
pub const ARMS: [(Coverage, usize); 3] =
    [(Coverage::Cpu, 1), (Coverage::Cpu, 4), (Coverage::Gpu, 1)];

/// Every pixel of `pixels` against the closed form: the intersection's area `met(x, y)` to
/// within the half level one rounding leaves, and the counts of pixels both sets cut where a product and `min` of
/// the two closed-form coverages miss it by more than a level.
pub fn held_to_the_intersection(
    pixels: &[u8],
    (width, height): (u32, u32),
    mark: impl Fn(u32, u32) -> f64,
    clip: impl Fn(u32, u32) -> f64,
    met: impl Fn(u32, u32) -> f64,
) -> (usize, usize, usize, usize) {
    let (mut both, mut product_below, mut product_above, mut min_above) = (0, 0, 0, 0);
    for y in 0..height {
        for x in 0..width {
            let (s, c, i) = (mark(x, y), clip(x, y), met(x, y));
            if s > 0.0 && s < 1.0 && c > 0.0 && c < 1.0 {
                both += 1;
                product_below += usize::from(255.0 * s * c < 255.0 * i - 1.0);
                product_above += usize::from(255.0 * s * c > 255.0 * i + 1.0);
                min_above += usize::from(255.0 * s.min(c) > 255.0 * i + 1.0);
            }
            let drawn = f64::from(alpha(pixels, width, x, y));
            assert!(
                (drawn - 255.0 * i).abs() <= 0.5 + 1e-6,
                "pixel ({x}, {y}): {drawn} against the intersection's {:.2} (s {s:.4}, c {c:.4})",
                255.0 * i
            );
        }
    }
    (both, product_below, product_above, min_above)
}
