//! How many horizontal strips of the target each command can mark, counted before anyone
//! parallelises the rasteriser.
//!
//! `render-cpu` draws single-threaded. The obvious way to change that is to cut the target into
//! horizontal strips and replay the display list into each on its own thread — the geometry
//! `Band` already has (ADR 0010), for a different reason, and rayon is already a dependency.
//!
//! **The number that decides whether that is worth building is not the thread count.** Session
//! 154 measured 19% of page 101's rasterisation to be per-command work that does not shrink with
//! the band — building a `tiny_skia::Path` from ours, taking a path's bounds, compiling a raster
//! pipeline — and a strip replay repeats all of it *once per strip a command touches*. So the
//! cost of the naive form is
//!
//! ```text
//! T(S)  ≈  fixed × (touches / commands) × S⁻¹ … per strip, plus the pixels that strip covers
//! ```
//!
//! and it turns on `touches / commands`: one command touching one strip is free to replay, one
//! command touching every strip is paid for *S* times. A page of small glyphs and a page of
//! full-width fills are opposite answers, so this prints the ratio rather than assuming either.
//!
//! It also prints how the covered area falls across the strips, because a perfectly cheap replay
//! is still bounded by its slowest strip: equal *rows* are not equal *work* when a page's ink is
//! in its middle third.
//!
//! ```sh
//! cargo run --release -p pdf-model --example strip_spans -- [file.pdf] [page] [scale]
//! ```
//!
//! With no arguments it reads page 101 of ISO 32000-2, which is the page `callgrind_rasterise`
//! measures and the page the 19% was measured on.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::print_stdout,
    reason = "a measurement tool: it should stop loudly if its input is missing, its \
              counters are bounded by one page's commands, and printing is the whole point"
)]
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "every cast here turns a device coordinate clamped to the target's own rows into a \
              row index, or a count of one page's commands into a ratio to print; both fit"
)]

use pdf_render::{ClipId, Command, DisplayList, PathCommand, Point, Transform};

/// Strip counts reported, which bracket the thread counts a laptop and a workstation offer.
const STRIPS: [u32; 4] = [2, 4, 8, 16];

/// A command's device-space extent, in target pixels: left, top, right, bottom.
///
/// Held as an option because a command whose clip admits nothing marks nothing, and that is a
/// different answer from "marks one strip".
type Extent = Option<[f32; 4]>;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().map_or_else(
        || {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../doc/ISO_32000-2_sponsored_EC3.pdf")
        },
        std::path::PathBuf::from,
    );
    let index: usize = args
        .next()
        .map_or(101, |n| n.parse().expect("a page number"));
    let scale: f32 = args.next().map_or(1.0, |n| n.parse().expect("a scale"));

    let bytes = std::fs::read(&path).expect("readable");
    let document = pdf_syntax::Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(index - 1)
        .expect("page exists");
    let list = pdf_model::interpret(&document, &page).display_list;
    let target = pdf_render::TargetSpec::for_page(&list, scale, 1 << 30).expect("a target");

    // One entry per command a backend would draw, group elements included: a group is composited
    // as one object, but its elements are what the rasteriser walks.
    let mut extents = Vec::new();
    let mut unknown = 0_usize;
    collect(
        &list,
        list.commands(),
        target.transform,
        None,
        &mut extents,
        &mut unknown,
    );

    println!(
        "{} page {index} at {scale}x: target {}x{}, {} drawable commands",
        path.display(),
        target.width,
        target.height,
        extents.len()
    );

    // The cost of one target row: every command that can mark it, times how wide it can mark.
    // Rows rather than pixels because a strip is a run of rows, so a row is the unit a split
    // can actually choose between.
    let rows = target.height as usize;
    let mut per_row = vec![0.0_f64; rows];
    for extent in extents.iter().flatten() {
        let span = f64::from(extent[2] - extent[0]).min(f64::from(target.width));
        let from = (extent[1].max(0.0) as usize).min(rows);
        let to = ((extent[3].max(0.0).ceil() as usize).max(from + 1)).min(rows);
        for cost in per_row.get_mut(from..to).into_iter().flatten() {
            *cost += span;
        }
    }

    // Every distinct clip chain, and the extent it covers. A strip replay cannot share a mask
    // cache across threads, so a chain is built once per strip it spans — and on the project's
    // worst page `MaskCache::get` is a quarter of the whole render (ADR 0103), which is why this
    // is counted beside the commands rather than assumed small.
    let chains: Vec<Extent> = clips(&list)
        .into_iter()
        .map(|id| chain(&list, id, target.transform))
        .collect();

    let drawn = extents.iter().flatten().count().max(1) as f64;
    let chained = chains.iter().flatten().count().max(1) as f64;
    println!("           ── equal rows ──      ── equal cost ──");
    println!("  strips   touches  slowest      touches  slowest   masks    (even split)");
    for strips in STRIPS {
        let even = equal_rows(rows, strips as usize);
        let balanced = equal_cost(&per_row, strips as usize);
        let (even_touches, even_slowest) = judge(&extents, &per_row, &even);
        let (balanced_touches, balanced_slowest) = judge(&extents, &per_row, &balanced);
        let (mask_touches, _) = judge(&chains, &per_row, &balanced);
        println!(
            "  {strips:>6}   {:>7.2}  {:>6.1}%      {:>7.2}  {:>6.1}%  {:>6.2}    {:>5.1}%",
            even_touches as f64 / drawn,
            100.0 * even_slowest,
            balanced_touches as f64 / drawn,
            100.0 * balanced_slowest,
            mask_touches as f64 / chained,
            100.0 / f64::from(strips),
        );
    }

    let marked = extents.iter().flatten().count();
    println!(
        "  {marked} commands mark something; {} are clipped away entirely, \
         {unknown} are of a kind this counter does not know",
        extents.len() - marked
    );
}

/// Every distinct clip chain the list refers to, leaves included and parents not.
///
/// A parent chain is built as part of its child's, so counting leaves is counting masks.
fn clips(list: &DisplayList) -> Vec<ClipId> {
    fn gather(commands: &[Command], into: &mut std::collections::BTreeSet<ClipId>) {
        for command in commands {
            if let Some(id) = command.clip() {
                let _ = into.insert(id);
            }
            if let Command::Group { commands, .. } = command {
                gather(commands, into);
            }
        }
    }
    let mut into = std::collections::BTreeSet::new();
    gather(list.commands(), &mut into);
    into.into_iter().collect()
}

/// Strip boundaries at equal heights: the obvious split, and the one a thread pool suggests.
fn equal_rows(rows: usize, strips: usize) -> Vec<usize> {
    (0..=strips).map(|i| rows * i / strips).collect()
}

/// Strip boundaries at equal estimated cost.
///
/// A page's ink is not spread evenly down it — `bug1721218_reduced.pdf` is one wide gradient over
/// a third of its height — so equal heights hand one thread most of the work. Cutting where the
/// *cost* is even is one prefix sum, and it is the difference between a decomposition worth
/// building and one that is not.
fn equal_cost(per_row: &[f64], strips: usize) -> Vec<usize> {
    let total: f64 = per_row.iter().sum();
    let mut boundaries = vec![0_usize];
    let mut running = 0.0;
    for row in 0..per_row.len() {
        running += per_row.get(row).copied().unwrap_or(0.0);
        let wanted = boundaries.len() as f64 * total / strips as f64;
        if running >= wanted && boundaries.len() < strips {
            boundaries.push(row + 1);
        }
    }
    while boundaries.len() <= strips {
        boundaries.push(per_row.len());
    }
    boundaries
}

/// What a split costs: how many (command, strip) pairs it replays, and the slowest strip's
/// share of the total estimated cost.
///
/// The slowest strip is the number that matters. Threads finish when the last one does, so a
/// split whose worst strip holds 70% of the work is a 1.4× speedup however many threads it has.
fn judge(extents: &[Extent], per_row: &[f64], boundaries: &[usize]) -> (u64, f64) {
    let strips = boundaries.len().saturating_sub(1);
    let mut touches = 0_u64;
    for extent in extents.iter().flatten() {
        let first = extent[1].max(0.0) as usize;
        let last = (extent[3].max(0.0).ceil() as usize).max(first + 1);
        touches += (0..strips)
            .filter(|strip| {
                let (top, bottom) = (boundaries[*strip], boundaries[strip + 1]);
                first < bottom && last > top
            })
            .count() as u64;
    }
    let cost = |strip: usize| -> f64 {
        per_row
            .get(boundaries[strip]..boundaries[strip + 1])
            .map_or(0.0, |rows| rows.iter().sum())
    };
    let total: f64 = (0..strips).map(cost).sum();
    let slowest = (0..strips).map(cost).fold(0.0_f64, f64::max);
    (touches, if total > 0.0 { slowest / total } else { 0.0 })
}

/// Every command a backend draws, with the device extent it can mark.
fn collect(
    list: &DisplayList,
    commands: &[Command],
    to_device: Transform,
    inherited: Extent,
    into: &mut Vec<Extent>,
    unknown: &mut usize,
) {
    for command in commands {
        let clip = command
            .clip()
            .map_or(inherited, |id| meet(inherited, chain(list, id, to_device)));
        let own = match command {
            Command::Fill {
                path, transform, ..
            } => outline(path, transform.then(to_device), 0.0),
            Command::Stroke {
                path,
                transform,
                stroke,
                ..
            } => {
                let placed = transform.then(to_device);
                // Half the stroke's device width lies outside the path on each side, and a
                // mitre can reach further; half is the honest bound for a *proxy* and the
                // error is a fraction of a pixel on a hairline.
                outline(path, placed, stroke.device_width(placed) / 2.0)
            }
            Command::Image { transform, .. } => {
                // §8.9.5.2's unit square is the image's geometry; the transform carries the rest.
                let unit = [
                    Point { x: 0.0, y: 0.0 },
                    Point { x: 1.0, y: 0.0 },
                    Point { x: 1.0, y: 1.0 },
                    Point { x: 0.0, y: 1.0 },
                ];
                corners(unit.iter().copied(), transform.then(to_device), 0.0)
            }
            Command::Group { commands, .. } => {
                // A group's elements are what the rasteriser walks, so they are counted rather
                // than their union — and they are counted under the group's clip.
                collect(list, commands, to_device, clip, into, unknown);
                continue;
            }
            // `Command` is `#[non_exhaustive]`, so a kind added later lands here. Counting it as
            // unbounded overstates the duplication a strip replay would pay, which is the
            // direction that cannot make parallelism look better than it is — and the count is
            // printed, because a silent catch-all is where a new command goes to be ignored.
            _ => {
                *unknown += 1;
                None
            }
        };
        into.push(meet(own, clip));
    }
}

/// A clip chain's device extent: every clip in it, met.
fn chain(list: &DisplayList, leaf: ClipId, to_device: Transform) -> Extent {
    let mut extent: Extent = None;
    let mut current = Some(leaf);
    let mut depth = 0;
    let mut first = true;
    while let Some(id) = current {
        depth += 1;
        if depth > 64 {
            break;
        }
        let Some(clip) = list.clip(id) else { break };
        let one = outline(&clip.path, clip.transform.then(to_device), 0.0);
        extent = if first { one } else { meet(extent, one) };
        first = false;
        current = clip.parent;
    }
    extent
}

/// A path's device extent, grown by `outset` on every side.
fn outline(path: &pdf_render::Path, transform: Transform, outset: f32) -> Extent {
    corners(path.commands().iter().flat_map(points), transform, outset)
}

/// The extent of a set of points under `transform`, grown by `outset`.
///
/// Control points rather than the curve: a Bézier lies inside its control polygon's convex hull,
/// so this is a bound and never an underestimate — which is the direction a proxy for "can this
/// command mark this strip" has to err in.
fn corners(points: impl Iterator<Item = Point>, transform: Transform, outset: f32) -> Extent {
    let mut extent = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    let mut any = false;
    for point in points {
        any = true;
        let x = transform.a * point.x + transform.c * point.y + transform.e;
        let y = transform.b * point.x + transform.d * point.y + transform.f;
        extent = [
            extent[0].min(x),
            extent[1].min(y),
            extent[2].max(x),
            extent[3].max(y),
        ];
    }
    any.then(|| {
        [
            extent[0] - outset,
            extent[1] - outset,
            extent[2] + outset,
            extent[3] + outset,
        ]
    })
}

/// The intersection of two extents, where `None` on either side means "unbounded".
///
/// `None` is what an unclipped command carries, and an unclipped command is bounded by the
/// target rather than by nothing — but the target's own bound is applied by the strip
/// arithmetic, so treating it as unbounded here loses no row.
fn meet(left: Extent, right: Extent) -> Extent {
    match (left, right) {
        (Some(left), Some(right)) => {
            let met = [
                left[0].max(right[0]),
                left[1].max(right[1]),
                left[2].min(right[2]),
                left[3].min(right[3]),
            ];
            (met[0] < met[2] && met[1] < met[3]).then_some(met)
        }
        (Some(one), None) | (None, Some(one)) => Some(one),
        (None, None) => None,
    }
}

/// The points one path command names.
fn points(command: &PathCommand) -> Vec<Point> {
    match *command {
        PathCommand::MoveTo(p) | PathCommand::LineTo(p) => vec![p],
        PathCommand::CurveTo(a, b, c) => vec![a, b, c],
        PathCommand::Close => Vec::new(),
    }
}
