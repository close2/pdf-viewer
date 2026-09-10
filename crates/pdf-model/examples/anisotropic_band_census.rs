//! How wide the sub-pixel substitution's band actually becomes, when the placement is anisotropic.
//!
//! ISO 32000-2 §10.7.4's substitution for a stroke too thin for the raster to measure states the
//! rule at one device pixel and carries the width it gave up in the paint's alpha
//! (`pdf_render::substitute_width`, ADR 0268). That width is `1 / min_stretch`, which is one
//! device pixel across **whichever way the mark runs** — the right answer for §8.5.3.2's dot and
//! Table 53's cap, which are shapes of the line's width in every direction, and the wrong one for
//! the swept **band** of a stroke, which is thin along one direction only.
//!
//! A band swept along `u` under `T` has device thickness `w · |u| · |det T| / |T u|`, so stating
//! it at `1 / min_stretch` gives it a device thickness of `t / (w · min_stretch)` — one device
//! pixel where `u` runs along the transform's *longer* axis, and up to `max_stretch /
//! min_stretch` pixels where it runs along the shorter one. This census prints that factor, which
//! is what says whether the substitution paints pixels §10.7.4's own sentence does not reach:
//!
//! > A shape shall be scan-converted by painting any pixel whose half-open square region
//! > intersects the shape, no matter how small the intersection is.
//!
//! **And the second figure is the residual the fix leaves**, so that this instrument prices the
//! tree it is run against rather than the one it was written for. `band_substitute_width` keeps
//! the narrowest of a path's own directions, so the direction that wanted the most gets exactly a
//! device pixel and the one that wanted the least gets `min / max` of one. That ratio is 1 for a
//! path running in one direction — which is what a turned rule is — and above 1 only where a path
//! states several directions an anisotropic placement disagrees about. ADR 0945.
//!
//! ```sh
//! cargo run --release -p pdf-model --example anisotropic_band_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! One page per document — the page the oracle judges — at scale 1, which is the scale every
//! verdict in this tree is taken at.
#![expect(
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    reason = "a measurement example: its output is the point, the counters are bounded by one \
              page's commands, the bucket is a positive width rounded to a whole device pixel, \
              and one walk of one page answering every question it prints is clearer than four"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use pdf_render::{Command, Path, PathCommand, Point, Transform};

/// What one stroke's geometry is made of, which decides what a per-direction width could be
/// stated from.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Shape {
    /// One `m` and one `l`: a single straight segment, whose direction is the whole path's.
    OneSegment,
    /// Straight segments only, so every direction is a segment's.
    Straight,
    /// At least one curve, whose direction varies along it.
    Curved,
}

/// The device thickness of a band of path-width `w` swept along `u` under `to_device`.
fn band_thickness(w: f32, u: (f32, f32), to_device: Transform) -> f32 {
    let length = u.0.hypot(u.1);
    let mapped =
        (to_device.a * u.0 + to_device.c * u.1).hypot(to_device.b * u.0 + to_device.d * u.1);
    if mapped <= 0.0 {
        return 0.0;
    }
    w * length * to_device.determinant().abs() / mapped
}

/// Every straight segment direction a path states, and what the path is made of.
fn directions(path: &Path) -> (Vec<(f32, f32)>, Shape) {
    let mut out = Vec::new();
    let mut shape = Shape::OneSegment;
    let mut current = Point::new(0.0, 0.0);
    let mut start = Point::new(0.0, 0.0);
    let mut segments = 0_usize;
    for command in path.commands() {
        match *command {
            PathCommand::MoveTo(p) => {
                current = p;
                start = p;
            }
            PathCommand::LineTo(p) => {
                out.push((p.x - current.x, p.y - current.y));
                current = p;
                segments += 1;
            }
            PathCommand::CurveTo(_, _, p) => {
                current = p;
                segments += 1;
                shape = Shape::Curved;
            }
            PathCommand::Close => {
                out.push((start.x - current.x, start.y - current.y));
                current = start;
                segments += 1;
            }
        }
    }
    if shape != Shape::Curved && segments > 1 {
        shape = Shape::Straight;
    }
    (out, shape)
}

fn main() {
    let files: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let mut reached = 0_usize;
    let mut by_shape: BTreeMap<u8, usize> = BTreeMap::new();
    // The distribution of the widest substitute band a stroke produces, in whole device pixels.
    let mut buckets: BTreeMap<u32, usize> = BTreeMap::new();
    // And of how many times thinner than a device pixel the *thinnest* direction is left, at the
    // width the tree now chooses: the residual, which is 1 wherever a path runs one way.
    let mut residuals: BTreeMap<u32, usize> = BTreeMap::new();
    let mut worst: Vec<(f32, String, usize, Shape)> = Vec::new();
    let mut worst_residual: Vec<(f32, String)> = Vec::new();

    for file in &files {
        let Ok(bytes) = std::fs::read(file) else {
            continue;
        };
        let Ok(document) = pdf_syntax::Document::open(bytes) else {
            continue;
        };
        let Some(page) = pdf_model::Pages::new(&document).get(0) else {
            continue;
        };
        let list = pdf_model::interpret(&document, &page).display_list;
        let Ok(target) = pdf_render::TargetSpec::for_page(&list, 1.0, 1 << 30) else {
            continue;
        };
        let mut page_worst = 0.0_f32;
        let mut page_residual = 0.0_f32;
        let mut page_count = 0_usize;
        let mut page_shape = Shape::OneSegment;
        for command in list.commands() {
            let Command::Stroke {
                transform,
                stroke,
                path,
                ..
            } = command
            else {
                continue;
            };
            let at = transform.then(target.transform);
            let (Some(one_pixel), Some(substitute)) = (
                pdf_render::thinnest_line(at),
                pdf_render::substitute_width(at),
            ) else {
                continue;
            };
            let width = stroke.device_width(at);
            // `draw_sub_pixel_rule`'s own trigger, and then the exact route's condition: a stroke
            // the bands construction takes never reaches the widening this census is about.
            if width > one_pixel {
                continue;
            }
            if width < one_pixel && at.preserves_axes() && pdf_render::only_flat_subpaths(path) {
                continue;
            }
            let (dirs, shape) = directions(path);
            let widest = dirs
                .iter()
                .map(|&u| band_thickness(substitute, u, at))
                .fold(0.0_f32, f32::max);
            if widest <= 0.0 {
                continue;
            }
            // What the tree chooses now, and the thinnest band it leaves.
            let residual = pdf_render::band_substitute_width(path, at).map_or(1.0, |chosen| {
                let thinnest = dirs
                    .iter()
                    .map(|&u| band_thickness(chosen, u, at))
                    .fold(f32::INFINITY, f32::min);
                if thinnest > 0.0 { 1.0 / thinnest } else { 1.0 }
            });
            reached += 1;
            page_count += 1;
            *by_shape.entry(shape as u8).or_default() += 1;
            *buckets.entry(widest.round() as u32).or_default() += 1;
            *residuals.entry(residual.round() as u32).or_default() += 1;
            if widest > page_worst {
                page_worst = widest;
                page_shape = shape;
            }
            page_residual = page_residual.max(residual);
        }
        if page_worst > 1.5 {
            worst.push((
                page_worst,
                file.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                page_count,
                page_shape,
            ));
        }
        if page_residual > 1.5 {
            worst_residual.push((
                page_residual,
                file.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            ));
        }
    }

    println!("documents {}", files.len());
    println!("strokes reaching the widening {reached}");
    for (bucket, count) in &buckets {
        println!("  substitute band {bucket:>4} device pixels wide: {count}");
    }
    for (bucket, count) in &residuals {
        println!("  thinnest direction left 1/{bucket} of a device pixel: {count}");
    }
    for (shape, count) in &by_shape {
        let name = match shape {
            0 => "one segment",
            1 => "straight, several segments",
            _ => "curved",
        };
        println!("  {name}: {count}");
    }
    worst.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!(
        "pages whose widest substitute band exceeds 1.5 device pixels: {}",
        worst.len()
    );
    worst_residual.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!(
        "pages the narrowest width still leaves a direction under 2/3 of a pixel on: {}",
        worst_residual.len()
    );
    for (residual, name) in worst_residual.iter().take(30) {
        println!("  1/{residual:.3} of a pixel  {name}");
    }
    for (widest, name, count, shape) in worst.iter().take(30) {
        let made_of = match shape {
            Shape::OneSegment => "one segment",
            Shape::Straight => "straight",
            Shape::Curved => "curved",
        };
        println!("  {widest:9.3} px  {name}  ({count} strokes, widest is {made_of})");
    }
}
