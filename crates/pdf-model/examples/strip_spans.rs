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
//!
//! **It was built, refused, and built again**, and this example now prints all three stages:
//! ADR 0137's touch ratio, ADR 0138's refusal, and ADR 0139's constrained split with the wall
//! clock beside it. The last two columns are the ones a session should read — how many strips
//! the page's own geometry grants, and what the render then takes.
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

use pdf_render::{ClipId, Command, DisplayList, Point, Rect, Transform};

/// Strip counts reported, which bracket the thread counts a laptop and a workstation offer.
const STRIPS: [u32; 4] = [2, 4, 8, 16];

/// A command's device-space extent, in target pixels: left, top, right, bottom.
///
/// Held as an option because a command whose clip admits nothing marks nothing, and that is a
/// different answer from "marks one strip".
type Extent = Option<Rect>;

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

    // `pdf_render::strips` is what a backend would use, so this measures the shipped plan
    // rather than a second copy of it that could drift from it.
    let extents = pdf_render::command_extents(&list, target);
    let per_row = pdf_render::row_costs(&extents, target);

    println!(
        "{} page {index} at {scale}x: target {}x{}, {} drawable commands",
        path.display(),
        target.width,
        target.height,
        extents.len()
    );

    let rows = target.height as usize;
    let drawn = extents.iter().flatten().count().max(1) as f64;
    let chains: Vec<Option<Rect>> = clips(&list)
        .into_iter()
        .map(|id| chain_extent(&list, id, target.transform))
        .collect();
    let chained = chains.iter().flatten().count().max(1) as f64;

    println!(
        "           \u{2500}\u{2500} equal rows \u{2500}\u{2500}      \u{2500}\u{2500} equal cost \u{2500}\u{2500}"
    );
    println!("  strips   touches  slowest      touches  slowest   masks    (even split)");
    for strips in STRIPS {
        let even = equal_rows(rows, strips as usize);
        let balanced = pdf_render::strip_boundaries(&per_row, strips);
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
        "  {marked} commands mark something; {} are clipped away entirely",
        extents.len() - marked
    );

    // ADR 0139: a cut may only fall on a row no re-stated segment crosses — a curve, an
    // oblique edge, anything a stroker touches — because chopping one at a strip's edge
    // changes its coverage. This is what that constraint costs on a real page, and it is the
    // number that decided whether to build it. **`replay` is the other half of the
    // decision**: the shipped planner refuses a division that replays more than a quarter more
    // work than the list itself, so a row above 1.25 is a row it would not take however good
    // its `slowest` looks. The last column is what the whole thing buys.
    let curved = pdf_render::unsplittable_rows(&list, target);
    let legal = curved.iter().filter(|curved| !**curved).count();
    println!(
        "  {legal} of {rows} rows ({:.1}%) are legal cut rows",
        100.0 * legal as f64 / rows.max(1) as f64
    );
    println!("  strips   granted  slowest      ideal   replay      drawn in");
    for strips in STRIPS {
        let legal = pdf_render::strip_boundaries_avoiding(&per_row, &curved, strips, 32);
        let (_, slowest) = judge(&extents, &per_row, &legal);
        println!(
            "  {strips:>6}   {:>7}  {:>6.1}%     {:>5.1}%   {:>6.2}      {:>8.1} ms",
            legal.len().saturating_sub(1),
            100.0 * slowest,
            100.0 / f64::from(strips),
            pdf_render::replay_ratio(&extents, &legal),
            best_of(&list, target, strips).as_secs_f64() * 1000.0,
        );
    }
    println!(
        "  {:>6}   {:>7}  {:>6.1}%     {:>5.1}%   {:>6.2}      {:>8.1} ms",
        1,
        1,
        100.0,
        100.0,
        1.0,
        best_of(&list, target, 1).as_secs_f64() * 1000.0,
    );
}

/// The fastest of five renders at a given strip count.
///
/// Wall clock, which this file's own header says is unusable — for *instruction* counting,
/// which is what a serial change is measured by. A parallel one cannot be: callgrind counts
/// instructions across every thread, so a perfect eight-way split measures as no change at
/// all. The fastest of five is the least contended run rather than the mean, which is the
/// figure least polluted by whatever else the machine is doing.
fn best_of(list: &DisplayList, target: pdf_render::TargetSpec, strips: u32) -> std::time::Duration {
    use pdf_render::Rasterizer as _;
    let mut best = std::time::Duration::MAX;
    for _ in 0..5 {
        let mut backend = render_cpu::CpuRasterizer::new().with_strips(strips);
        let at = std::time::Instant::now();
        let raster = backend.rasterize(list, target).expect("a supported page");
        best = best.min(at.elapsed());
        drop(raster);
    }
    best
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

/// A clip chain's device extent, met down the chain.
fn chain_extent(list: &DisplayList, leaf: ClipId, to_device: Transform) -> Option<Rect> {
    let mut extent: Option<Rect> = None;
    let mut current = Some(leaf);
    for _ in 0..64 {
        let Some(id) = current else { break };
        let Some(clip) = list.clip(id) else { break };
        let one = clip.path.bounds(clip.transform.then(to_device))?;
        extent = Some(extent.map_or(one, |held| Rect {
            min: Point::new(held.min.x.max(one.min.x), held.min.y.max(one.min.y)),
            max: Point::new(held.max.x.min(one.max.x), held.max.y.min(one.max.y)),
        }));
        current = clip.parent;
    }
    extent
}

/// Strip boundaries at equal heights: the obvious split, and the one a thread pool suggests.
fn equal_rows(rows: usize, strips: usize) -> Vec<u32> {
    (0..=strips)
        .map(|i| u32::try_from(rows * i / strips).unwrap_or(u32::MAX))
        .collect()
}

/// What a split costs: how many (command, strip) pairs it replays, and the slowest strip's
/// share of the total estimated cost.
///
/// The slowest strip is the number that matters. Threads finish when the last one does, so a
/// split whose worst strip holds 70% of the work is a 1.4× speedup however many threads it has.
fn judge(extents: &[Extent], per_row: &[f64], boundaries: &[u32]) -> (u64, f64) {
    let strips = boundaries.len().saturating_sub(1);
    let mut touches = 0_u64;
    for extent in extents.iter().flatten() {
        let first = extent.min.y.max(0.0) as usize;
        let last = (extent.max.y.max(0.0).ceil() as usize).max(first + 1);
        touches += (0..strips)
            .filter(|strip| {
                let (top, bottom) = (boundaries[*strip] as usize, boundaries[strip + 1] as usize);
                first < bottom && last > top
            })
            .count() as u64;
    }
    let cost = |strip: usize| -> f64 {
        per_row
            .get(boundaries[strip] as usize..boundaries[strip + 1] as usize)
            .map_or(0.0, |rows| rows.iter().sum())
    };
    let total: f64 = (0..strips).map(cost).sum();
    let slowest = (0..strips).map(cost).fold(0.0_f64, f64::max);
    (touches, if total > 0.0 { slowest / total } else { 0.0 })
}
