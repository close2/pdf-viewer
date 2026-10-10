//! What a page's strokes cost the strip planner, and whether the cuts they now grant are exact.
//!
//! `pdf_render::unsplittable_rows` names the rows a strip may not begin at, and it counts every
//! stroke's whole device extent, so a page frame drawn as `re S` forbids every row and the page is
//! drawn on one thread. `render-cpu` asks `pdf_render::unsplittable_rows_where` instead and forbids
//! no row for a stroke its exact-area converter draws a row at a time (ADR 1819). This prints the
//! page's strokes tallied by the class that decides that, the legal rows and the cuts under both
//! answers — the backend's taken from `CpuRasterizer::strip_plan`, so it is the plan the draw uses —
//! and, with `--draw`, the page drawn whole and in four and sixteen strips, compared byte for byte,
//! with the clock of each.
//!
//! ```sh
//! cargo run --release -p pdf-model --example stroke_cut_census -- <file.pdf> [page] [scale] [--draw] [--dump <dir>]
//! ```
//!
//! The default scale is 150 dpi, which is `quorra-transform render`'s.
#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a measurement tool: it stops loudly on a missing input, its counters are bounded by \
              one page's commands and rows, a segment's being horizontal or vertical is an exact \
              property of the coordinates the file stated, and printing is the whole point"
)]

use std::collections::BTreeMap;
use std::time::Instant;

use pdf_render::{
    Command, DisplayList, LineCap, LineJoin, PathCommand, Rasterizer as _, Transform,
};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let draw = args.iter().any(|arg| arg == "--draw");
    args.retain(|arg| arg != "--draw");
    // `--dump <dir>` writes each raster the comparison drew as raw RGBA, `<strips>-<w>x<h>.rgba`,
    // for the moved pixels to be painted and looked at (trap 1).
    let dump = args.iter().position(|arg| arg == "--dump").map(|at| {
        args.drain(at..=at.saturating_add(1))
            .nth(1)
            .expect("a directory")
    });
    // `--at-row <r>` lists the commands whose extent crosses device row `r`, for reading what a
    // moved row holds.
    let at_row: Option<f32> = args.iter().position(|arg| arg == "--at-row").map(|at| {
        args.drain(at..=at.saturating_add(1))
            .nth(1)
            .and_then(|row| row.parse().ok())
            .expect("a row")
    });
    let mut args = args.into_iter();
    let path = args.next().expect("a path to a PDF");
    let index: usize = args.next().map_or(1, |n| n.parse().expect("a page number"));
    let scale: f32 = args
        .next()
        .map_or(150.0 / 72.0, |n| n.parse().expect("a scale"));

    let started = Instant::now();
    let bytes = std::fs::read(&path).expect("readable");
    let document = pdf_syntax::Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(index - 1)
        .expect("page exists");
    let list = pdf_model::interpret(&document, &page).display_list;
    let target = pdf_render::TargetSpec::for_page(&list, scale, 1 << 34).expect("a target");
    println!(
        "{path} page {index} at {scale}: {}x{}, interpreted in {:.2} s",
        target.width,
        target.height,
        started.elapsed().as_secs_f64()
    );

    if let Some(row) = at_row {
        crossing(&list, target, row);
    }
    let mut classes: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut extents = vec![false; target.height as usize];
    tally(
        list.commands(),
        target.transform,
        &mut classes,
        &mut extents,
    );
    println!("  strokes by class (commands, device rows summed over their extents):");
    for (class, (count, rows)) in &classes {
        println!("    {count:>9}  {rows:>12}  {class}");
    }

    // The rows the backend's own question leaves forbidden, recovered from the plan rather than
    // asked again: a cut it makes is a row it permits.
    let planned = Instant::now();
    let plan = render_cpu::CpuRasterizer::new()
        .with_strips(16)
        .strip_plan(&list, target);
    let planning = planned.elapsed();
    let now = pdf_render::unsplittable_rows(&list, target);
    let before: Vec<bool> = now.iter().zip(&extents).map(|(a, b)| *a || *b).collect();
    debug_assert_eq!(before, now, "the default rule is every stroke's extent");
    let legal = |rows: &[bool]| rows.iter().filter(|row| !**row).count();
    let height = target.height as usize;
    println!(
        "  legal cut rows with every stroke's extent forbidden: {} of {height}; with none: {}",
        legal(&before),
        legal(&pdf_render::unsplittable_rows_where(&list, target, &|_| {
            true
        })),
    );
    let costs = pdf_render::row_costs(&pdf_render::command_extents(&list, target), target);
    let cuts = pdf_render::strip_boundaries_avoiding(&costs, &before, 16, 64);
    println!(
        "  before: sixteen asked, {} granted, cuts {cuts:?}",
        cuts.len() - 1
    );
    println!(
        "     now: sixteen asked, {} granted ({:.1} ms to plan), cuts {plan:?}",
        plan.len() - 1,
        planning.as_secs_f64() * 1000.0
    );

    if draw {
        divisions(&list, target, dump.as_deref());
    }
}

/// The page drawn whole and in two, four and sixteen strips, compared byte for byte, with the clock
/// of each, how many of the moved pixels lie beside a cut, and — with `dump` — the rasters.
fn divisions(list: &DisplayList, target: pdf_render::TargetSpec, dump: Option<&str>) {
    let whole = drawn(list, target, 1);
    let write = |strips: u32, data: &[u8]| {
        if let Some(dir) = &dump {
            let name = format!("{dir}/{strips}-{}x{}.rgba", target.width, target.height);
            std::fs::write(name, data).expect("a writable directory");
        }
    };
    write(1, &whole.0);
    for strips in [2, 4, 16] {
        let split = drawn(list, target, strips);
        write(strips, &split.0);
        let (moved, worst, first) = compare(&whole.0, &split.0, target.width);
        // A cut re-stating geometry moves pixels on the rows beside it; a strip's shifted
        // origin (ADR 0219) moves them anywhere below its first row. Counting the moved
        // pixels within two rows of a cut tells the two apart.
        let cuts = render_cpu::CpuRasterizer::new()
            .with_strips(strips)
            .strip_plan(list, target);
        let near = whole
            .0
            .chunks_exact(4)
            .zip(split.0.chunks_exact(4))
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .filter(|(at, _)| {
                let row = (at / target.width as usize) as u32;
                cuts.iter()
                    .skip(1)
                    .take(cuts.len().saturating_sub(2))
                    .any(|cut| row.abs_diff(*cut) <= 2)
            })
            .count();
        println!(
            "  {strips:>2} strips: {near} of the moved pixels lie within two rows of a cut of {cuts:?}"
        );
        println!(
            "  {strips:>2} strips: {:.2} s against {:.2} s whole; {moved} pixels differ, worst {worst}, first {first:?}",
            split.1, whole.1
        );
    }
}

/// Every top-level command whose extent crosses device row `row`, printed whole.
fn crossing(list: &DisplayList, target: pdf_render::TargetSpec, row: f32) {
    for command in list.commands() {
        if let Some(extent) = command.device_bounds(target.transform)
            && extent.min.y < row + 1.0
            && extent.max.y > row
        {
            println!("  at row {row}: {extent:?} {command:?}");
        }
    }
}

/// One rasterisation at a stated strip count, and its wall clock in seconds.
fn drawn(list: &DisplayList, target: pdf_render::TargetSpec, strips: u32) -> (Vec<u8>, f64) {
    let at = Instant::now();
    let raster = render_cpu::CpuRasterizer::new()
        .with_strips(strips)
        .rasterize(list, target)
        .expect("a supported page");
    (raster.data, at.elapsed().as_secs_f64())
}

/// Pixels that differ, the worst byte, and the first differing pixel.
fn compare(one: &[u8], other: &[u8], width: u32) -> (usize, u8, Option<(usize, usize)>) {
    let mut moved = 0;
    let mut worst = 0;
    let mut first = None;
    for (index, (a, b)) in one.chunks_exact(4).zip(other.chunks_exact(4)).enumerate() {
        let most = a
            .iter()
            .zip(b)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        if most > 0 {
            moved += 1;
            worst = worst.max(most);
            first.get_or_insert((index % width as usize, index / width as usize));
        }
    }
    (moved, worst, first)
}

/// Tallies every stroke by class and marks the rows its whole extent covers.
fn tally(
    commands: &[Command],
    to_device: Transform,
    classes: &mut BTreeMap<String, (u64, u64)>,
    extents: &mut [bool],
) {
    for command in commands {
        match command {
            Command::Group { commands, .. } => tally(commands, to_device, classes, extents),
            Command::Shaped { object, shape } => {
                tally(std::slice::from_ref(object), to_device, classes, extents);
                tally(std::slice::from_ref(shape), to_device, classes, extents);
            }
            Command::Stroke {
                path,
                transform,
                stroke,
                ..
            } => {
                let at = transform.then(to_device);
                let (mut curved, mut oblique) = (false, false);
                let mut previous = None;
                let mut start = None;
                for segment in path.commands() {
                    match *segment {
                        PathCommand::MoveTo(p) => {
                            previous = Some(p);
                            start = Some(p);
                        }
                        PathCommand::LineTo(p) => {
                            if let Some(from) = previous {
                                oblique |= from.x != p.x && from.y != p.y;
                            }
                            previous = Some(p);
                        }
                        PathCommand::CurveTo(_, _, p) => {
                            curved = true;
                            previous = Some(p);
                        }
                        PathCommand::Close => {
                            if let (Some(from), Some(to)) = (previous, start) {
                                oblique |= from.x != to.x && from.y != to.y;
                            }
                            previous = start;
                        }
                    }
                }
                let geometry = match (curved, oblique) {
                    (true, _) => "curved",
                    (false, true) => "oblique",
                    (false, false) => "axis-aligned",
                };
                let cap = match stroke.cap {
                    LineCap::Butt => "butt",
                    LineCap::Round => "round",
                    LineCap::Square => "square",
                };
                let join = match stroke.join {
                    LineJoin::Miter => "mitre",
                    LineJoin::Round => "round",
                    LineJoin::Bevel => "bevel",
                };
                let width = stroke.device_width(at) * at.min_stretch();
                let thickness = if width <= 1.0 { "<=1px" } else { ">1px" };
                let class = format!(
                    "{geometry} cap={cap} join={join} limit={} {thickness}{}{}",
                    stroke.miter_limit,
                    if stroke.dash_array.is_empty() {
                        ""
                    } else {
                        " dashed"
                    },
                    if at.preserves_axes() { "" } else { " rotated" },
                );
                let mut rows = 0;
                if let Some(extent) = command.device_bounds(to_device) {
                    // `strips::mark`'s own range: a cut along `y = r` chops the extent when `top < r < bottom`.
                    let from = (extent.min.y.floor() + 1.0).max(0.0) as usize;
                    let to = (extent.max.y.ceil().max(0.0) as usize).min(extents.len());
                    for row in extents.get_mut(from..to).into_iter().flatten() {
                        *row = true;
                    }
                    rows = to.saturating_sub(from) as u64;
                }
                let entry = classes.entry(class).or_default();
                entry.0 += 1;
                entry.1 += rows;
            }
            _ => {}
        }
    }
}
