//! What §10.7.5's grid fit does to the pages that ask for it: hairlines made crisp, seams opened
//! and closed, and what the fit costs the CPU rasteriser — ADR 2530's instrument.
//!
//! ISO 32000-2 §10.7.5's first requirement, which `pdf_render::grid_fit` constructs:
//!
//! > When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
//! > automatically be adjusted as necessary to produce lines of uniform thickness.
//!
//! For each document's first page that paints at least one stroke under `/SA true`, the page is
//! drawn at 1× twice by `render-cpu`, with the fit on and off, and three things are counted:
//!
//! - **crisp** — of the axis segments the fit moved, at least three device pixels long and
//!   painted in a solid colour that differs from what lies three pixels beside them, how many
//!   have a pixel of the stroke's own colour (within three levels) across their midpoint in each
//!   arm. A one-pixel rule drawn across two rows has none; one drawn on one row has one.
//! - **shared** — of the fitted segments, how many lie along an edge of another mark at the same
//!   coordinates: an axis edge of a fill or of another stroke, fitted or not, within one and a
//!   half device pixels across and overlapping it for at least three along. That is where a seam
//!   can open.
//! - **seams** — along a shared stretch, the pixels in the two rows beside the fitted rule's row
//!   that are a *valley* in one arm and not in the other: lighter by at least a quarter of the
//!   range than both neighbours across the rule. A valley that appears is a gap opened between
//!   the rule and the mark beside it; one that disappears is the two run together. Both are
//!   counted, because the fit can do either.
//! - **time** — the draw of each arm, the least of `--runs` interleaved draws, in milliseconds.
//!
//! ```sh
//! cargo run --release -p pdf-model --example stroke_grid_fit_measure -- --pdfjs --crops <dir>
//! cargo run --release -p pdf-model --example stroke_grid_fit_measure -- <file.pdf>... --runs 3
//! ```
//!
//! `--crops <dir>` writes, for the three pages with the most seam pixels and the three with the
//! most segments made crisp, a 48 × 48 crop at the worst seam or the first crisp segment, the
//! unfitted arm left and the fitted arm right, magnified four times.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::expect_used,
    reason = "a measurement example: its output is the point, every count is bounded by one \
              page's pixels at 1x, and every cast is of a device coordinate inside that page"
)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use pdf_render::{Command, Paint, PathCommand, Raster, Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;

/// Levels a crisp pixel may differ from the stroke's colour by, per channel.
const CRISP: i32 = 3;
/// Luminance a valley pixel is lighter than both its neighbours by: a quarter of the range.
const VALLEY: i32 = 64;
/// How far across, in device pixels, another mark's edge may lie from a fitted rule's centre and
/// still be at its coordinates: the widest fitted rule's whole width, so two rules that touch count.
const BESIDE: f32 = pdf_render::FIT_BELOW;
/// Side of a crop, in device pixels.
const CROP: u32 = 48;
/// Magnification of a written crop.
const ZOOM: u32 = 4;

/// One page's counts, and where its crops would be cut.
#[derive(Default)]
struct Page {
    file: String,
    sa_strokes: usize,
    fitted_strokes: usize,
    sampled: usize,
    shared: usize,
    /// Fitted strokes by requested device width: under half a pixel (already one pixel wide by
    /// the clause's second requirement), half to one, and one to one and a half.
    widths: [usize; 3],
    crisp_off: usize,
    crisp_on: usize,
    opened: usize,
    closed: usize,
    changed: usize,
    ms_off: f64,
    ms_on: f64,
    worst_seam: Option<(u32, u32)>,
    first_crisp: Option<(u32, u32)>,
    rasters: Option<(Raster, Raster)>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut files = Vec::new();
    let mut crops: Option<PathBuf> = None;
    let mut runs = 1_usize;
    let mut words = args.iter();
    while let Some(word) = words.next() {
        match word.as_str() {
            "--pdfjs" => {
                let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
                let mut found: Vec<PathBuf> = std::fs::read_dir(&root)
                    .expect("the pdf.js corpus")
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("pdf")))
                    .collect();
                found.sort();
                files.extend(found);
            }
            "--crops" => crops = words.next().map(PathBuf::from),
            "--runs" => runs = words.next().and_then(|n| n.parse().ok()).unwrap_or(1),
            file => files.push(PathBuf::from(file)),
        }
    }
    let mut pages: Vec<Page> = files
        .iter()
        .filter_map(|file| measure(file, runs, crops.is_some()))
        .collect();
    report(&pages);
    if let Some(dir) = crops {
        write_crops(&mut pages, &dir);
    }
}

/// Interprets one document's first page and, where it strokes under `/SA true`, draws it twice.
fn measure(path: &Path, runs: usize, keep: bool) -> Option<Page> {
    let bytes = std::fs::read(path).ok()?;
    let document = pdf_syntax::Document::open(bytes).ok()?;
    let page = pdf_model::Pages::new(&document).get(0)?;
    let list = pdf_model::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, 1.0, 1 << 30).ok()?;
    let name = path.file_name()?.to_string_lossy().into_owned();
    let mut counts = Page {
        file: name,
        ..Page::default()
    };
    let mut segments = Vec::new();
    // The axis edges of every mark, each with the index of the command that made it, so that a
    // fitted rule is compared with every other fill and stroke and never with itself.
    let mut edges = Vec::new();
    for (index, command) in list.commands().iter().enumerate() {
        if let Command::Fill {
            path, transform, ..
        } = command
        {
            let at = transform.then(target.transform);
            edges.extend(axis_segments(path, at).into_iter().map(|e| (index, e)));
            continue;
        }
        let Command::Stroke {
            path,
            transform,
            stroke,
            paint,
            ..
        } = command
        else {
            continue;
        };
        let at = transform.then(target.transform);
        let own = axis_segments(path, at);
        edges.extend(own.iter().map(|&e| (index, e)));
        if !stroke.adjust {
            continue;
        }
        counts.sa_strokes += 1;
        if pdf_render::grid_fit(path, stroke, at).is_none() {
            continue;
        }
        counts.fitted_strokes += 1;
        let requested = pdf_render::thinnest_line(at).map_or(0.0, |one| stroke.width / one);
        counts.widths[usize::from(requested >= 0.5) + usize::from(requested >= 1.0)] += 1;
        let rgb = match paint {
            Paint::Solid(colour) if colour.a >= 1.0 => {
                Some([colour.r, colour.g, colour.b].map(|c| (c * 255.0).round() as i32))
            }
            _ => None,
        };
        segments.extend(own.into_iter().map(|s| (index, s, rgb)));
    }
    if counts.sa_strokes == 0 {
        return None;
    }
    let draw = |fitted: bool| {
        let start = Instant::now();
        let raster = CpuRasterizer::new()
            .with_stroke_adjustment(fitted)
            .rasterize(&list, target)
            .ok();
        (raster, start.elapsed().as_secs_f64() * 1000.0)
    };
    let (mut off, mut on) = (None, None);
    (counts.ms_off, counts.ms_on) = (f64::MAX, f64::MAX);
    for _ in 0..runs.max(1) {
        let (raster, ms) = draw(false);
        (off, counts.ms_off) = (raster, counts.ms_off.min(ms));
        let (raster, ms) = draw(true);
        (on, counts.ms_on) = (raster, counts.ms_on.min(ms));
    }
    let (off, on) = (off?, on?);
    tally_seams((&off, &on), &segments, &edges, &mut counts);
    counts.changed = changed(&off, &on);
    for (_, segment, rgb) in segments {
        let Some(rgb) = rgb else { continue };
        let Some((crisp_off, crisp_on)) = crisp(&off, &on, segment, rgb) else {
            continue;
        };
        counts.sampled += 1;
        counts.crisp_off += usize::from(crisp_off);
        counts.crisp_on += usize::from(crisp_on);
        if crisp_on && !crisp_off && counts.first_crisp.is_none() {
            counts.first_crisp = Some(segment.midpoint());
        }
    }
    if keep {
        counts.rasters = Some((off, on));
    }
    Some(counts)
}

/// An axis segment in device space: the fixed coordinate, and the run along the axis.
#[derive(Clone, Copy)]
struct Segment {
    horizontal: bool,
    across: f32,
    from: f32,
    to: f32,
}

impl Segment {
    fn midpoint(self) -> (u32, u32) {
        let along = f32::midpoint(self.from, self.to).max(0.0) as u32;
        let across = self.across.max(0.0) as u32;
        if self.horizontal {
            (along, across)
        } else {
            (across, along)
        }
    }
}

/// The straight segments of `path` along a device axis and at least three device pixels long.
fn axis_segments(path: &pdf_render::Path, at: pdf_render::Transform) -> Vec<Segment> {
    let mut out = Vec::new();
    let (mut from, mut start) = (None, None);
    for command in path.commands() {
        let (a, b) = match *command {
            PathCommand::MoveTo(p) => {
                from = Some(at.apply(p));
                start = from;
                continue;
            }
            PathCommand::LineTo(p) => {
                let b = at.apply(p);
                let Some(a) = from.replace(b) else { continue };
                (a, b)
            }
            PathCommand::Close => {
                let (Some(a), Some(b)) = (from, start) else {
                    continue;
                };
                from = Some(b);
                (a, b)
            }
            PathCommand::CurveTo(_, _, p) => {
                from = Some(at.apply(p));
                continue;
            }
        };
        let (dx, dy) = ((b.x - a.x).abs(), (b.y - a.y).abs());
        if dy < 0.02 && dx >= 3.0 {
            out.push(Segment {
                horizontal: true,
                across: a.y,
                from: a.x.min(b.x),
                to: a.x.max(b.x),
            });
        } else if dx < 0.02 && dy >= 3.0 {
            out.push(Segment {
                horizontal: false,
                across: a.x,
                from: a.y.min(b.y),
                to: a.y.max(b.y),
            });
        }
    }
    out
}

/// One pixel's red, green and blue, or `None` outside the raster.
fn pixel(raster: &Raster, x: i64, y: i64) -> Option<[i32; 3]> {
    if x < 0 || y < 0 || x >= i64::from(raster.width) || y >= i64::from(raster.height) {
        return None;
    }
    let at = ((y as usize) * raster.width as usize + x as usize) * 4;
    let p = raster.data.get(at..at + 3)?;
    Some([i32::from(p[0]), i32::from(p[1]), i32::from(p[2])])
}

/// Whether each arm has a pixel of the stroke's colour across the segment's midpoint, or `None`
/// where the stroke's colour is what lies beside the segment anyway.
fn crisp(off: &Raster, on: &Raster, segment: Segment, rgb: [i32; 3]) -> Option<(bool, bool)> {
    let along = f32::midpoint(segment.from, segment.to).floor() as i64;
    let centre = segment.across.floor() as i64;
    let at = |raster: &Raster, offset: i64| {
        if segment.horizontal {
            pixel(raster, along, centre + offset)
        } else {
            pixel(raster, centre + offset, along)
        }
    };
    let matches = |p: [i32; 3]| (0..3).all(|i| (p[i] - rgb[i]).abs() <= CRISP);
    let beside = [at(off, -3)?, at(off, 3)?];
    if beside.iter().any(|&p| matches(p)) {
        return None;
    }
    let has = |raster: &Raster| (-1..=1).any(|o| at(raster, o).is_some_and(matches));
    Some((has(off), has(on)))
}

/// Luminance, in the levels of one channel.
fn luminance(p: [i32; 3]) -> i32 {
    (p[0] * 299 + p[1] * 587 + p[2] * 114) / 1000
}

/// Whether a pixel is lighter by [`VALLEY`] than both its neighbours across a rule, the rows
/// above and below for a horizontal one and the columns either side for a vertical one.
fn valley(raster: &Raster, x: i64, y: i64, horizontal: bool) -> bool {
    let Some(here) = pixel(raster, x, y).map(luminance) else {
        return false;
    };
    let lighter = |a: Option<[i32; 3]>, b: Option<[i32; 3]>| match (a, b) {
        (Some(a), Some(b)) => here - luminance(a) >= VALLEY && here - luminance(b) >= VALLEY,
        _ => false,
    };
    if horizontal {
        lighter(pixel(raster, x, y - 1), pixel(raster, x, y + 1))
    } else {
        lighter(pixel(raster, x - 1, y), pixel(raster, x + 1, y))
    }
}

/// The seam pixels already counted, and how many fall in each crop-sized tile.
#[derive(Default)]
struct Tally {
    seen: std::collections::HashSet<(i64, i64)>,
    tiles: std::collections::HashMap<(u32, u32), usize>,
}

/// One fitted segment: the command that drew it, its device geometry, and its solid colour.
type Fitted = (usize, Segment, Option<[i32; 3]>);

/// Counts the seams along every stretch a fitted segment shares with another mark, and puts the
/// worst seam at the centre of the crop-sized tile that holds the most of them.
fn tally_seams(
    (off, on): (&Raster, &Raster),
    segments: &[Fitted],
    edges: &[(usize, Segment)],
    counts: &mut Page,
) {
    let mut tally = Tally::default();
    for &(index, segment, _) in segments {
        for (from, to) in shared_stretches((index, segment), edges) {
            counts.shared += 1;
            seams(off, on, (segment, from, to), counts, &mut tally);
        }
    }
    counts.worst_seam = tally
        .tiles
        .into_iter()
        .max_by_key(|&(tile, n)| (n, std::cmp::Reverse(tile)))
        .map(|((tx, ty), _)| (tx * CROP + CROP / 2, ty * CROP + CROP / 2));
}

/// How many pixels the two arms differ at.
fn changed(off: &Raster, on: &Raster) -> usize {
    off.data
        .chunks_exact(4)
        .zip(on.data.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count()
}

/// The stretches along `segment`, made by command `own`, where another command's axis edge runs
/// at the same coordinates: the same orientation, within [`BESIDE`] across, overlapping for three
/// pixels or more.
fn shared_stretches(
    (own, segment): (usize, Segment),
    edges: &[(usize, Segment)],
) -> Vec<(f32, f32)> {
    edges
        .iter()
        .filter(|&&(index, e)| {
            index != own
                && e.horizontal == segment.horizontal
                && (e.across - segment.across).abs() <= BESIDE
        })
        .filter_map(|(_, e)| {
            let (from, to) = (e.from.max(segment.from), e.to.min(segment.to));
            (to - from >= 3.0).then_some((from, to))
        })
        .collect()
}

/// Counts, along one shared stretch, the valleys beside the fitted rule's row that one arm has
/// and the other does not, and adds each to the crop-sized tile it falls in.
fn seams(
    off: &Raster,
    on: &Raster,
    (segment, from, to): (Segment, f32, f32),
    counts: &mut Page,
    tally: &mut Tally,
) {
    let row = segment.across.floor() as i64;
    for along in (from.ceil() as i64)..(to.floor() as i64) {
        for across in [row - 1, row + 1] {
            let (x, y) = if segment.horizontal {
                (along, across)
            } else {
                (across, along)
            };
            // Two rules either side of one row both see it; it is one seam pixel.
            if !tally.seen.insert((x, y)) {
                continue;
            }
            let (was, is) = (
                valley(off, x, y, segment.horizontal),
                valley(on, x, y, segment.horizontal),
            );
            if is && !was {
                counts.opened += 1;
            } else if was && !is {
                counts.closed += 1;
            } else {
                continue;
            }
            *tally
                .tiles
                .entry((x.max(0) as u32 / CROP, y.max(0) as u32 / CROP))
                .or_default() += 1;
        }
    }
}

fn report(pages: &[Page]) {
    let mut totals = Page::default();
    let mut moved_pages = 0_usize;
    for page in pages {
        println!(
            "{}: {} /SA strokes, {} fitted; {} segments sampled, crisp {} unfitted and {} fitted; \
             {} shared stretches; {} pixels changed, seams {} opened and {} closed; {:.2} ms \
             unfitted, {:.2} ms fitted",
            page.file,
            page.sa_strokes,
            page.fitted_strokes,
            page.sampled,
            page.crisp_off,
            page.crisp_on,
            page.shared,
            page.changed,
            page.opened,
            page.closed,
            page.ms_off,
            page.ms_on
        );
        moved_pages += usize::from(page.changed > 0);
        totals.sa_strokes += page.sa_strokes;
        totals.fitted_strokes += page.fitted_strokes;
        totals.sampled += page.sampled;
        totals.shared += page.shared;
        for (total, n) in totals.widths.iter_mut().zip(page.widths) {
            *total += n;
        }
        totals.crisp_off += page.crisp_off;
        totals.crisp_on += page.crisp_on;
        totals.changed += page.changed;
        totals.opened += page.opened;
        totals.closed += page.closed;
        totals.ms_off += page.ms_off;
        totals.ms_on += page.ms_on;
    }
    println!(
        "fitted strokes by requested device width: {} under 0.5 px, {} from 0.5 to 1 px, {} from \
         1 to 1.5 px",
        totals.widths[0], totals.widths[1], totals.widths[2]
    );
    println!(
        "summary: {} first pages stroke under /SA, {moved_pages} of them drawn differently; \
         {} /SA strokes, {} fitted; {} segments sampled, crisp {} unfitted and {} fitted; {} \
         shared stretches; {} pixels changed, seams {} opened and {} closed; {:.1} ms unfitted, \
         {:.1} ms fitted",
        pages.len(),
        totals.sa_strokes,
        totals.fitted_strokes,
        totals.sampled,
        totals.crisp_off,
        totals.crisp_on,
        totals.shared,
        totals.changed,
        totals.opened,
        totals.closed,
        totals.ms_off,
        totals.ms_on
    );
}

/// Writes the three worst pages' seam crops and the three best pages' crisp crops into `dir`.
fn write_crops(pages: &mut [Page], dir: &Path) {
    std::fs::create_dir_all(dir).expect("a crops directory");
    let mut by_seams: Vec<usize> = (0..pages.len())
        .filter(|&i| pages[i].worst_seam.is_some())
        .collect();
    by_seams.sort_by_key(|&i| std::cmp::Reverse(pages[i].opened + pages[i].closed));
    let mut by_crisp: Vec<usize> = (0..pages.len())
        .filter(|&i| pages[i].first_crisp.is_some())
        .collect();
    by_crisp
        .sort_by_key(|&i| std::cmp::Reverse(pages[i].crisp_on.saturating_sub(pages[i].crisp_off)));
    for (kind, chosen) in [("worst", &by_seams), ("best", &by_crisp)] {
        for (rank, &i) in chosen.iter().take(3).enumerate() {
            let page = &pages[i];
            let at = if kind == "worst" {
                page.worst_seam
            } else {
                page.first_crisp
            };
            let (Some((off, on)), Some(at)) = (&page.rasters, at) else {
                continue;
            };
            let out = dir.join(format!("{kind}-{}-{}.png", rank + 1, page.file));
            write_crop(off, on, at, &out);
            eprintln!("{kind} {}: {} at {at:?}", rank + 1, out.display());
        }
    }
}

/// One crop: the unfitted arm left, the fitted arm right, each magnified [`ZOOM`] times.
fn write_crop(off: &Raster, on: &Raster, (cx, cy): (u32, u32), out: &Path) {
    let side = CROP * ZOOM;
    let (width, height) = (side * 2 + ZOOM, side);
    let mut data = vec![255_u8; (width * height * 4) as usize];
    let x0 = i64::from(cx) - i64::from(CROP / 2);
    let y0 = i64::from(cy) - i64::from(CROP / 2);
    for (arm, raster) in [off, on].into_iter().enumerate() {
        for y in 0..height {
            for x in 0..side {
                let source = pixel(raster, x0 + i64::from(x / ZOOM), y0 + i64::from(y / ZOOM))
                    .unwrap_or([255, 0, 255]);
                let left = arm as u32 * (side + ZOOM) + x;
                let at = ((y * width + left) * 4) as usize;
                for (i, value) in source.iter().enumerate() {
                    data[at + i] = *value as u8;
                }
                data[at + 3] = 255;
            }
        }
    }
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("header");
        writer.write_image_data(&data).expect("data");
    }
    std::fs::write(out, png).expect("written");
}
