//! What a toolkit window's zoomed page costs drawn whole and drawn as its window, and how the
//! window's pixels compare with the same part of the page drawn whole (ADR 1930).
//!
//! ```text
//! cargo run --release -p viewer-host --example window_against_whole -- document.pdf width height rounds zoom...
//! ```
//!
//! The page one request a window of `width` × `height` device pixels makes, its target magnified by
//! each `zoom` over the fit as notches would, the page centred in the window as a notch about the
//! window's middle leaves it. Whether [`viewer_host::oversize::oversized`] draws it as its window,
//! and, where it does, two arms through [`viewer_host::Drawing`] polled every
//! [`viewer_host::drawing::POLL`], alternated `rounds` times so a neighbour's load lands on both:
//! **whole**, `Drawing::ask`, as a page under the window's extent is drawn; **window**,
//! `Drawing::ask_window` with the window [`viewer_host::oversize::Oversized::due`] asks for.
//! Printed per run: milliseconds from the ask to the landing of each, the bytes each copies into
//! the toolkit at an update, how many pixels of the window differ from the whole page's same
//! rectangle, and by how much, and a digest of each drawing's pixels — which is what says that two
//! builds drew the same window and the same page (ADR 1941); and the differing pixels by their
//! distance to the window's edge and by the class of the smallest mark holding each — its kind,
//! whether it is rectangles, whether it reaches past `tiny-skia`'s 8191 (ADR 2058). Where
//! `WINDOW_AGAINST_WHOLE_ISOLATE` is set, the most different pixel's marks are narrowed to the
//! fewest that, drawn apart from the rest, still differ there, and printed. Then, after the
//! drawings so that none of them is handed the bounds it would otherwise measure, how many of the
//! list's commands the page's own target and the window keep, and the commands the page's target
//! leaves out, by kind and by the side of the page they lie off, and the share the window keeps as
//! [`pdf_render::DisplayList::share_reaching`] estimates it for [`oversized`], with how long the
//! estimate took (ADR 1994). `rounds` of 0 prints that alone. Where `WINDOW_AGAINST_WHOLE_COPY` is
//! set, each round first copies the window's commands into a list of their own and lets it go, as
//! every window drawn before ADR 1980 did, so that what that copy cost the page drawn after it is
//! measured in one build (ADR 1994).

use std::time::{Duration, Instant};

use pdf_render::{Raster, TargetSpec, Transform};
use viewer_core::{Command, DocumentId, Event, PageGeometry, RenderRequest, Rendered, Viewer};
use viewer_host::drawing::{Drawing, POLL};
use viewer_host::oversize::{Oversized, Window, largest_centred, oversized};

#[expect(
    clippy::expect_used,
    reason = "a measurement tool should stop loudly where its document is missing"
)]
fn first_request(path: &str, width: u32, height: u32) -> RenderRequest {
    let bytes = pdf_syntax::FileBytes::on_disk(std::path::Path::new(path)).expect("readable");
    let mut viewer = Viewer::new(0, 0, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(0),
            bytes,
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.anticipate().for_each(drop);
    viewer
        .handle(Command::Resize {
            width,
            height,
            scale: 1.0,
        })
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .expect("a viewport asks for page one")
}

/// One draw through the drawing thread, of the whole target or of `window`: when it landed, and
/// its pixels.
#[expect(
    clippy::panic,
    reason = "a measurement tool should stop loudly where its frame did not land"
)]
fn drawn(request: &RenderRequest, window: Option<Window>) -> (Duration, Raster) {
    let mut drawing: Drawing = Drawing::new();
    let began = Instant::now();
    match window {
        Some(window) => drawing.ask_window(request.clone(), window),
        None => drawing.ask(request.clone()),
    }
    loop {
        let mut landed: Vec<_> = drawing.collect();
        landed.extend(drawing.windows().into_iter().map(|(finished, _)| finished));
        if let Some(done) = landed.into_iter().next() {
            let framed = began.elapsed();
            let Some(Rendered::Raster(raster)) = done.outcome else {
                panic!("the page did not land as a raster");
            };
            return (framed, raster);
        }
        std::thread::sleep(POLL);
    }
}

/// One pixel of the window that differs from the page's: its column and row in the window, and
/// the difference of the channel that differs most, the window's level less the page's.
type Difference = (usize, usize, i16);

/// How many pixels of `window`'s raster differ from the same rectangle of `whole`, the largest
/// difference of one channel, the rectangle of the window the differing pixels lie in, and each
/// of them.
fn compared(
    whole: &Raster,
    window: Window,
    drawn: &Raster,
) -> (usize, u8, Option<[usize; 4]>, Vec<Difference>) {
    let (mut differing, mut worst) = (0_usize, 0_u8);
    let mut extent: Option<[usize; 4]> = None;
    let mut differences = Vec::new();
    let stride = usize::try_from(whole.width)
        .unwrap_or(usize::MAX)
        .saturating_mul(4);
    let row_bytes = usize::try_from(drawn.width)
        .unwrap_or(usize::MAX)
        .saturating_mul(4);
    for (row, drawn_row) in drawn.data.chunks_exact(row_bytes.max(1)).enumerate() {
        let from = usize::try_from(window.y)
            .unwrap_or(usize::MAX)
            .saturating_add(row)
            .saturating_mul(stride)
            .saturating_add(
                usize::try_from(window.x)
                    .unwrap_or(usize::MAX)
                    .saturating_mul(4),
            );
        let Some(whole_row) = whole.data.get(from..from.saturating_add(row_bytes)) else {
            continue;
        };
        for (column, (a, b)) in whole_row
            .chunks_exact(4)
            .zip(drawn_row.chunks_exact(4))
            .enumerate()
        {
            if a != b {
                differing = differing.saturating_add(1);
                let [x0, y0, x1, y1] = extent.unwrap_or([column, row, column, row]);
                extent = Some([x0.min(column), y0.min(row), x1.max(column), y1.max(row)]);
                worst = a
                    .iter()
                    .zip(b)
                    .map(|(a, b)| a.abs_diff(*b))
                    .fold(worst, u8::max);
                let signed = a
                    .iter()
                    .zip(b)
                    .map(|(a, b)| i16::from(*b).saturating_sub(i16::from(*a)))
                    .max_by_key(|difference| difference.abs())
                    .unwrap_or(0);
                differences.push((column, row, signed));
            }
        }
    }
    (differing, worst, extent, differences)
}

/// `render-cpu`'s `scan::SUPERSAMPLED_LIMIT`, which that crate keeps to itself: the largest device
/// coordinate `tiny-skia`'s anti-aliased converter can express.
const SUPERSAMPLED_LIMIT: f32 = 8191.0;

/// The side of the cells [`Under`] files the window's commands into, in device pixels.
const CELL: f32 = 64.0;

/// A command's kind, as the census and the classes name it.
fn kind(command: &pdf_render::Command) -> &'static str {
    match command {
        pdf_render::Command::Fill { .. } => "fill",
        pdf_render::Command::Stroke { .. } => "stroke",
        pdf_render::Command::Image { .. } => "image",
        pdf_render::Command::Shaped { .. } => "shaped",
        pdf_render::Command::Group { .. } => "group",
        _ => "another kind",
    }
}

/// The window's top-level commands filed by the cells of [`CELL`] pixels their extents on the
/// page's own grid meet, so that the smallest command holding a pixel is asked of a cell's few
/// rather than of the page's millions.
struct Under {
    /// The window's corner on the page.
    corner: (f32, f32),
    /// Cells across.
    columns: usize,
    /// Each cell's commands: index and extent.
    cells: Vec<Vec<(usize, pdf_render::Rect)>>,
}

impl Under {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a window of a few thousand pixels, in cells of 64"
    )]
    fn new(request: &RenderRequest, window: Window) -> Self {
        let target = request.target;
        let bounds = request.list.mark_bounds(target.transform);
        let corner = (window.x as f32, window.y as f32);
        let columns = (window.width as f32 / CELL).ceil() as usize;
        let rows = (window.height as f32 / CELL).ceil() as usize;
        let mut cells = vec![Vec::new(); columns.saturating_mul(rows)];
        let cell_of = |value: f32, origin: f32, limit: usize| {
            (((value - origin) / CELL).floor().max(0.0) as usize).min(limit.saturating_sub(1))
        };
        for index in 0..request.list.commands().len() {
            let Some(reach) = bounds.reach(index, target.transform) else {
                continue;
            };
            let (right, bottom) = (
                corner.0 + window.width as f32,
                corner.1 + window.height as f32,
            );
            if reach.max.x < corner.0
                || reach.max.y < corner.1
                || reach.min.x > right
                || reach.min.y > bottom
            {
                continue;
            }
            for row in cell_of(reach.min.y, corner.1, rows)..=cell_of(reach.max.y, corner.1, rows) {
                for column in cell_of(reach.min.x, corner.0, columns)
                    ..=cell_of(reach.max.x, corner.0, columns)
                {
                    if let Some(cell) =
                        cells.get_mut(row.saturating_mul(columns).saturating_add(column))
                    {
                        cell.push((index, reach));
                    }
                }
            }
        }
        Self {
            corner,
            columns,
            cells,
        }
    }

    /// The smallest command whose extent holds the window's pixel (`column`, `row`), and its
    /// extent.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a pixel of a window of a few thousand pixels"
    )]
    fn smallest(&self, column: usize, row: usize) -> Option<(usize, pdf_render::Rect)> {
        let (x, y) = (
            self.corner.0 + column as f32 + 0.5,
            self.corner.1 + row as f32 + 0.5,
        );
        let cell = ((row as f32 / CELL) as usize)
            .saturating_mul(self.columns)
            .saturating_add((column as f32 / CELL) as usize);
        let area =
            |reach: &pdf_render::Rect| (reach.max.x - reach.min.x) * (reach.max.y - reach.min.y);
        self.cells
            .get(cell)?
            .iter()
            .filter(|(_, reach)| {
                reach.min.x <= x && x <= reach.max.x && reach.min.y <= y && y <= reach.max.y
            })
            .min_by(|(_, a), (_, b)| area(a).total_cmp(&area(b)))
            .copied()
    }
}

/// What the smallest command holding a pixel is, as a class: its kind, whether a fill is
/// axis-aligned rectangles on the device's grid (`pdf_render::device_rectangles`, which
/// `render-cpu` draws by a closed form of its own, ADR 0476), and whether its extent passes
/// [`SUPERSAMPLED_LIMIT`] on the page's grid.
fn class_of(request: &RenderRequest, under: Option<(usize, pdf_render::Rect)>) -> String {
    let Some((index, reach)) = under else {
        return "under no command".to_owned();
    };
    let Some(command) = request.list.commands().get(index) else {
        return "under no command".to_owned();
    };
    let rectangles = match command {
        pdf_render::Command::Fill {
            path, transform, ..
        } => {
            pdf_render::device_rectangles(path, transform.then(request.target.transform)).is_some()
        }
        _ => false,
    };
    let past = reach.min.x < -SUPERSAMPLED_LIMIT
        || reach.min.y < -SUPERSAMPLED_LIMIT
        || reach.max.x > SUPERSAMPLED_LIMIT
        || reach.max.y > SUPERSAMPLED_LIMIT;
    format!(
        "{}{}{}",
        kind(command),
        if rectangles { " of rectangles" } else { "" },
        if past { ", past the range" } else { "" }
    )
}

/// The differing pixels of `window` sorted by where a cause would act (ADR 2058): by their
/// distance to the window's nearest edge, where a seam of the window's own origin would show, and
/// by the class of the smallest mark holding each ([`class_of`]). Each class is split at a
/// difference of 2 and of 8 levels, and counted lighter and darker in the window.
fn classified(request: &RenderRequest, window: Window, differences: &[Difference]) {
    // [count, over 2, over 8, lighter, darker] per class.
    let tally = |row: &mut [usize; 5], difference: i16| {
        let counted = [
            true,
            difference.abs() > 2,
            difference.abs() > 8,
            difference > 0,
            difference < 0,
        ];
        for (count, counts) in row.iter_mut().zip(counted) {
            *count = count.saturating_add(usize::from(counts));
        }
    };
    let (width, height) = (
        usize::try_from(window.width).unwrap_or(usize::MAX),
        usize::try_from(window.height).unwrap_or(usize::MAX),
    );
    let under = Under::new(request, window);
    let mut by_edge = [[0_usize; 5]; 5];
    let mut by_mark: std::collections::BTreeMap<String, [usize; 5]> =
        std::collections::BTreeMap::new();
    for &(column, row, difference) in differences {
        let edge = column
            .min(row)
            .min(width.saturating_sub(column.saturating_add(1)))
            .min(height.saturating_sub(row.saturating_add(1)));
        let class = match edge {
            0 => 0,
            1..=2 => 1,
            3..=8 => 2,
            9..=32 => 3,
            _ => 4,
        };
        tally(&mut by_edge[class], difference);
        tally(
            by_mark
                .entry(class_of(request, under.smallest(column, row)))
                .or_default(),
            difference,
        );
    }
    let said = |[count, two, eight, lighter, darker]: [usize; 5]| {
        format!(
            "{count} ({two} by more than 2, {eight} by more than 8; {lighter} lighter, \
             {darker} darker)"
        )
    };
    for (class, name) in ["0", "1-2", "3-8", "9-32", "33 and more"]
        .iter()
        .enumerate()
    {
        println!(
            "  differing {name} pixels from the window's edge: {}",
            said(by_edge[class])
        );
    }
    for (class, counts) in by_mark {
        println!("  differing under a {class}: {}", said(counts));
    }
}

/// The difference at the page's pixel (`x`, `y`) between `commands` of `request`'s list drawn
/// apart from the rest, in their order, under the page's own target and under the window's, one
/// strip each: the window's level less the page's, of the channel that differs most.
#[expect(
    clippy::expect_used,
    reason = "a measurement tool should stop loudly where a few marks cannot be drawn"
)]
fn drawn_apart(
    request: &RenderRequest,
    window: Window,
    commands: &[usize],
    (column, row): (u32, u32),
) -> i16 {
    use pdf_render::Rasterizer as _;
    let target = request.target;
    // The list itself with every command but these taken off, so that the clips and soft masks
    // they name are still the list's.
    let mut apart = pdf_render::DisplayList::clone(&request.list);
    let taken = apart.split_off_commands(0);
    for &index in commands {
        if let Some(command) = taken.get(index) {
            apart.push(command.clone());
        }
    }
    drop(taken);
    let mut rasteriser = render_cpu::CpuRasterizer::new().with_strips(1);
    let on_page = rasteriser
        .rasterize(&apart, target)
        .expect("the marks draw on the page");
    let in_window = rasteriser
        .rasterize_window(&apart, target, window.shown())
        .expect("the marks draw in the window");
    let level = |raster: &Raster, (column, row): (u32, u32)| -> [u8; 4] {
        let at = u64::from(row)
            .saturating_mul(u64::from(raster.width))
            .saturating_add(u64::from(column))
            .saturating_mul(4);
        usize::try_from(at)
            .ok()
            .and_then(|at| raster.data.get(at..at.saturating_add(4)))
            .and_then(|pixel| pixel.try_into().ok())
            .unwrap_or([0; 4])
    };
    let a = level(&on_page, (column, row));
    let b = level(
        &in_window,
        (
            column.saturating_sub(window.x),
            row.saturating_sub(window.y),
        ),
    );
    a.iter()
        .zip(b)
        .map(|(a, b)| i16::from(b).saturating_sub(i16::from(*a)))
        .max_by_key(|difference| difference.abs())
        .unwrap_or(0)
}

/// Where `WINDOW_AGAINST_WHOLE_ISOLATE` is set: the window's most different pixel, and the fewest
/// of the top-level commands holding it that, drawn apart from the rest on the page's whole target
/// and on the window's, still differ there — each taken out in turn and left out where the
/// difference stays, so that the cause is narrowed to the marks it needs and printed for a person
/// to read (ADR 2058).
fn isolated(request: &RenderRequest, window: Window, differences: &[Difference]) {
    if std::env::var_os("WINDOW_AGAINST_WHOLE_ISOLATE").is_none() {
        return;
    }
    let Some(&(column, row, difference)) = differences
        .iter()
        .max_by_key(|(_, _, difference)| difference.abs())
    else {
        return;
    };
    let pixel = (
        window
            .x
            .saturating_add(u32::try_from(column).unwrap_or(u32::MAX)),
        window
            .y
            .saturating_add(u32::try_from(row).unwrap_or(u32::MAX)),
    );
    #[expect(
        clippy::cast_precision_loss,
        reason = "a pixel of a target below MAX_EXTENT = 2^24, which f32 holds exactly"
    )]
    let (x, y) = (pixel.0 as f32 + 0.5, pixel.1 as f32 + 0.5);
    let target = request.target;
    let bounds = request.list.mark_bounds(target.transform);
    let mut kept: Vec<usize> = (0..request.list.commands().len())
        .filter(|&index| {
            bounds.reach(index, target.transform).is_some_and(|reach| {
                reach.min.x <= x && x <= reach.max.x && reach.min.y <= y && y <= reach.max.y
            })
        })
        .collect();
    println!(
        "  the most different pixel, by {difference}, is the page's {pixel:?}; the {} commands \
         holding it, drawn apart, differ there by {}",
        kept.len(),
        drawn_apart(request, window, &kept, pixel)
    );
    let mut at = 0;
    while at < kept.len() {
        let mut fewer = kept.clone();
        fewer.remove(at);
        if drawn_apart(request, window, &fewer, pixel) == 0 {
            at = at.saturating_add(1);
        } else {
            kept = fewer;
        }
    }
    for index in kept {
        let described: String = request
            .list
            .commands()
            .get(index)
            .map(|command| format!("{command:?}").chars().take(900).collect())
            .unwrap_or_default();
        println!("    still differing with command {index}: {described}");
    }
}

/// `raster`'s rectangle `[x0, y0, x1, y1]`, inclusive, as a binary PPM at `path` — for looking at
/// where two drawings differ (trap 1).
fn dump(raster: &Raster, [x0, y0, x1, y1]: [usize; 4], path: &std::path::Path) {
    let width = usize::try_from(raster.width).unwrap_or(usize::MAX);
    let mut bytes = format!(
        "P6\n{} {}\n255\n",
        x1.saturating_sub(x0).saturating_add(1),
        y1.saturating_sub(y0).saturating_add(1)
    )
    .into_bytes();
    for row in y0..=y1 {
        for column in x0..=x1 {
            let at = row
                .saturating_mul(width)
                .saturating_add(column)
                .saturating_mul(4);
            bytes.extend(
                raster
                    .data
                    .get(at..at.saturating_add(3))
                    .unwrap_or(&[0, 0, 0]),
            );
        }
    }
    if let Err(problem) = std::fs::write(path, bytes) {
        eprintln!("{}: {problem}", path.display());
    }
}

/// A digest of a drawing's pixels, to compare two builds' drawings by.
fn digest(raster: &Raster) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    raster.data.hash(&mut hasher);
    hasher.finish()
}

/// Where the two drawings differ, both of them of that rectangle, for a person to look at — where
/// `WINDOW_AGAINST_WHOLE_DUMP` names a directory to write them into.
fn dump_both(whole: &Raster, window: Window, drawn: &Raster, extent: [usize; 4], zoom: f32) {
    let Some(directory) = std::env::var_os("WINDOW_AGAINST_WHOLE_DUMP") else {
        return;
    };
    let [x0, y0, x1, y1] = extent;
    let x = usize::try_from(window.x).unwrap_or(usize::MAX);
    let y = usize::try_from(window.y).unwrap_or(usize::MAX);
    let directory = std::path::Path::new(&directory);
    dump(
        whole,
        [
            x0.saturating_add(x),
            y0.saturating_add(y),
            x1.saturating_add(x),
            y1.saturating_add(y),
        ],
        &directory.join(format!("{zoom}-whole.ppm")),
    );
    dump(drawn, extent, &directory.join(format!("{zoom}-window.ppm")));
    println!("  differing within the window's {extent:?}");
}

/// How many of `request`'s commands the page's whole target and `window` keep, and what the whole
/// target leaves out: each top-level command whose bound misses the target, by its kind and by
/// the side of the target it lies off (ADR 1994).
fn census(request: &RenderRequest, window: Window) {
    let list = &request.list;
    let target = request.target;
    let all = list.commands().len();
    let kept = |spec: TargetSpec| list.reaching(spec).map_or(all, |kept| kept.len());
    let began = Instant::now();
    let estimated = list.share_reaching(window.target_of(target));
    let asked = began.elapsed();
    let (whole, windowed) = (kept(target), kept(window.target_of(target)));
    #[expect(
        clippy::cast_precision_loss,
        reason = "a share of a few million commands, printed to a tenth of a per cent"
    )]
    let share = |part: usize| part as f64 * 100.0 / all.max(1) as f64;
    println!(
        "  commands: {all} in the list; the page's target keeps {whole} ({:.1} %); the window keeps \
         {windowed} ({:.1} %), estimated {:.1} % in {} µs",
        share(whole),
        share(windowed),
        estimated * 100.0,
        asked.as_micros()
    );
    let bounds = list.mark_bounds(target.transform);
    #[expect(
        clippy::cast_precision_loss,
        reason = "a target's extent is below MAX_EXTENT = 2^24, which f32 holds exactly"
    )]
    let (width, height) = (target.width as f32, target.height as f32);
    let mut tally: std::collections::BTreeMap<(&str, &str), usize> =
        std::collections::BTreeMap::new();
    for (index, command) in list.commands().iter().enumerate() {
        if !bounds.misses_target(index, target) {
            continue;
        }
        let kind = kind(command);
        let side = bounds
            .reach(index, target.transform)
            .map_or("unbounded", |reach| {
                if reach.max.y < 0.0 {
                    "above"
                } else if reach.min.y > height {
                    "below"
                } else if reach.max.x < 0.0 {
                    "left"
                } else if reach.min.x > width {
                    "right"
                } else {
                    "inside"
                }
            });
        let count = tally.entry((kind, side)).or_default();
        *count = count.saturating_add(1);
    }
    for ((kind, side), count) in tally {
        println!("  left out by the page's target: {count} {kind} {side}");
    }
}

fn millis(duration: Duration) -> String {
    format!("{:.0}", duration.as_secs_f64() * 1000.0)
}

#[expect(
    clippy::expect_used,
    reason = "a measurement tool should stop loudly where its arguments are wrong"
)]
fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [path, width, height, rounds, zooms @ ..] = arguments.as_slice() else {
        eprintln!("usage: window_against_whole document.pdf width height rounds zoom...");
        std::process::exit(2);
    };
    let (width, height): (u32, u32) = (
        width.parse().expect("a width"),
        height.parse().expect("a height"),
    );
    let rounds: usize = rounds.parse().expect("a count of rounds");
    let fit = first_request(path, width, height);
    println!(
        "RAYON_NUM_THREADS {:?}; fit {}x{}; window {width}x{height}",
        std::env::var("RAYON_NUM_THREADS").ok(),
        fit.target.width,
        fit.target.height
    );
    for zoom in zooms {
        let zoom: f32 = zoom.parse().expect("a magnification");
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss,
            reason = "a page of a few thousand pixels, magnified a few times"
        )]
        let target = TargetSpec {
            width: (fit.target.width as f32 * zoom).ceil() as u32,
            height: (fit.target.height as f32 * zoom).ceil() as u32,
            transform: fit.target.transform.then(Transform::scale(zoom, zoom)),
        };
        let request = RenderRequest {
            target,
            ..fit.clone()
        };
        // The page centred in the window along both sides, as a notch at the window's centre
        // leaves a page that started centred.
        #[expect(
            clippy::cast_precision_loss,
            reason = "a page of a few thousand pixels"
        )]
        let geometry = PageGeometry {
            page: pdf_render::Size::new(target.width as f32, target.height as f32),
            scale: zoom,
            width: target.width,
            height: target.height,
            origin: (
                (width as f32 - target.width as f32) / 2.0,
                (height as f32 - target.height as f32) / 2.0,
            ),
        };
        let mut held = Oversized::default();
        let windowed = held.asked(&request, (width, height));
        let due = held.due(request.document, |_| Some(geometry), (width, height));
        // Where the page is not drawn as its window, the window it would be drawn as were the
        // bound lower, so that the price of a window that is most of its page is measured too
        // (ADR 1941).
        let window = due.first().map_or_else(
            || largest_centred(target, (width, height)),
            |(_, window)| *window,
        );
        let share = (f64::from(window.width) * f64::from(window.height))
            / (f64::from(target.width) * f64::from(target.height));
        println!(
            "zoom {zoom}: {}x{} under {:?}, oversized {}, window {window:?}, {:.0} % of the page",
            target.width,
            target.height,
            target.transform,
            oversized(&request, (width, height)) && windowed,
            share * 100.0
        );
        let copying = std::env::var_os("WINDOW_AGAINST_WHOLE_COPY").is_some();
        for round in 0..rounds {
            if copying && let Some(kept) = request.list.reaching(window.target_of(target)) {
                drop(kept.copied());
            }
            let (whole, whole_page) = drawn(&request, None);
            let (windowed, window_page) = drawn(&request, Some(window));
            let (differing, worst, extent, differences) =
                compared(&whole_page, window, &window_page);
            if let Some(extent) = extent {
                dump_both(&whole_page, window, &window_page, extent, zoom);
            }
            println!(
                "  round {round}: whole landed {} ms, {} bytes; window landed {} ms, {} bytes; \
                 {differing} of the window's pixels differ from the page's, by at most {worst}; \
                 digests {:016x} {:016x}",
                millis(whole),
                whole_page.data.len(),
                millis(windowed),
                window_page.data.len(),
                digest(&whole_page),
                digest(&window_page)
            );
            classified(&request, window, &differences);
            isolated(&request, window, &differences);
        }
        census(&request, window);
    }
}
