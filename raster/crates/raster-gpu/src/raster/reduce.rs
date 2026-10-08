//! The caller's image-filtering decisions, resolved here per placement (ADR 0089).
//!
//! ISO 32000-2 §8.9.5.3's `/Interpolate` rule and the caller's documented
//! area-averaging departure from §10.7.4 used to be settled upstream and baked into
//! the scene — which was correct at exactly one viewport, and cost the caller's
//! page-space scenes their survival on any page with a picture on it (their ADR
//! 0702's ledger). Under [`ImageFilter::Auto`](raster_scene::ImageFilter) the flag
//! crosses the boundary instead, and this module answers the two questions where the
//! placement is known — the same amendment pattern as the stroke width (ADR 0085) and
//! the collapsed fill (ADR 0086), with the same containment: **the functions here mirror
//! the caller's `pdf_render` statement for statement** (`smoothed`, `factor`, `Reduction`,
//! `Bands`, `Reciprocals`, `average_block`, `round_div` — their `paint.rs`), the two below
//! that do not are held to those by tests, their CPU oracle keeps the originals, and the
//! cross-backend gates compare the two continuously. The reduced samples are byte-identical by construction:
//! the arithmetic is integer sums, divisions and exact reciprocal multiplications with no
//! float in the data path.
//!
//! Their row split is mirrored too, on the threads the host permits
//! ([`Options::encode_threads`](crate::Options::encode_threads)) rather than on a pool
//! of this library's: each cell of a reduction is made once per `(image, factors)` for the
//! device's life ([`crate::device`]'s cache), but that once is the first frame that
//! minifies the image past a new integer factor, which is a page turn onto the
//! photograph — a 5280×3792 one paid 53 ms of the turn's 55 ms of transfer on one
//! thread. Every output row is a function of its own band of source rows, so dividing
//! the rows moves no byte ([`PARALLEL_FLOOR`] and the caller's ADR 1433 have the
//! numbers).
//!
//! **Two constructions of the caller's bytes that are not their statements** (ADR 1493).
//! The device fills a reduced texture in the regions a frame samples rather than whole, so
//! [`area_averaged_cells`] reduces any rectangle of the grid, and [`reduce_row`] works on the
//! rectangle's source columns. Every cell is a function of its own block alone —
//! [`average_block`]'s, or the column sums' that the caller's proof makes the same byte for an
//! opaque block — so a window is the whole grid's cells, cropped. And a band that is not
//! opaque is read off premultiplied column sums ([`translucent_row`]) rather than block by
//! block: the same three sums and the same alpha sum, added in another order, which integer
//! addition does not see.

use std::ops::Range;

use raster_scene::ImageSpec;

/// Whether to filter between the samples of a `width` × `height` grid drawn under
/// `placement` — the caller's `smoothed`, statement for statement.
///
/// `placement` maps the unit square onto the device (§8.9.5.1), so the length of its
/// two columns is how many device pixels the image covers.
///
/// **One device pixel per sample is point-sampled before anything else is asked**, because
/// there ISO 32000-2 §10.7.4 states the colour outright:
///
/// > The position of the centre of such a pixel -in other words, the point whose coordinate
/// > values have fractional parts of one-half -shall be mapped back into source space to
/// > determine how to colour the pixel. There shall not be averaging over the pixel area.
///
/// At that placement every pixel centre maps back inside exactly one sample whatever the
/// sub-pixel offset, so the caller's reduction departure has no subject and `/Interpolate`
/// has nothing to smooth. A filter there lays the mean of four samples on every pixel of an
/// image whose origin sits on a half pixel — a whole image shifted by half a sample — which
/// is what the caller's `render-raster` sweep `tests/masked_image_edge.rs` measured before this
/// branch existed (the caller's ADR 1302, their feedback sections 47 and 48). The caller's
/// [`Reduction`] asks the same question of the reduced grid, so an exact integer reduction
/// lands here too.
///
/// Otherwise `/Interpolate` true filters; a *magnified* image — a sample covering more than
/// one device pixel, the case §8.9.5.3 is about — is drawn as flat rectangles, and a reduced
/// one keeps the filter on (their ADR 0025 carries the §10.7.4 argument).
pub(crate) fn smoothed(width: u32, height: u32, interpolate: bool, placement: &[f32; 6]) -> bool {
    let across = length(placement[0], placement[1]);
    let down = length(placement[2], placement[3]);
    // The equality is the condition rather than an approximation of it, exactly as the
    // caller's is: a placement a hair either side of native is magnified or reduced and is
    // classified below.
    #[expect(clippy::float_cmp, clippy::cast_precision_loss)]
    let native = across == width as f32 && down == height as f32;
    if native {
        return false;
    }
    if interpolate {
        return true;
    }
    #[expect(clippy::cast_precision_loss)] // dimensions are far below f32's exact range
    let magnified = across > width as f32 || down > height as f32;
    !magnified
}

/// The reduced grid a placement asks for, or `None` where no device pixel gathers two
/// samples — the caller's `Image::reduction`, statement for statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Reduction {
    /// Whole samples averaged per output cell, per axis.
    pub factors: (u32, u32),
    /// The reduced grid.
    pub width: u32,
    pub height: u32,
    /// The reduced grid's own answer to the filter question, by the same rule.
    pub smoothed: bool,
}

/// See [`Reduction`]. `None` for an inconsistent spec exactly as theirs refuses one,
/// and the consistency question is asked **before** the factors for their stated
/// reason: `factor` clamps into `1.0 ..= width`, which a zero width makes panic.
pub(crate) fn reduction(
    spec: &ImageSpec,
    interpolate: bool,
    placement: &[f32; 6],
) -> Option<Reduction> {
    if !spec.is_consistent() {
        return None;
    }
    let factors = (
        factor(spec.width, length(placement[0], placement[1])),
        factor(spec.height, length(placement[2], placement[3])),
    );
    if factors.0 <= 1 && factors.1 <= 1 {
        return None;
    }
    let width = spec.width.div_ceil(factors.0);
    let height = spec.height.div_ceil(factors.1);
    Some(Reduction {
        factors,
        width,
        height,
        smoothed: smoothed(width, height, interpolate, placement),
    })
}

/// Source samples below which a reduction runs on the calling thread alone — the
/// caller's `PARALLEL_FLOOR`, the same number for their reason: under it, starting
/// threads costs more than the rows they would share.
const PARALLEL_FLOOR: u64 = 65_536;

/// Output rows one thread takes at a time from the shared counter.
///
/// Small against a photograph's thousands of rows, so that a thread on this machine's
/// slower core class takes fewer bands rather than an equal share it finishes last.
const ROWS_PER_TAKE: usize = 16;

/// Averages each block of samples that would share one device pixel — the caller's
/// `Image::area_averaged`, statement for statement (premultiplied sums, proportional
/// band boundaries, round-to-nearest), with its rows divided among up to `threads`
/// threads as the module comment says. The bytes are theirs to the last one, at any
/// value of `threads`. The device asks [`area_averaged_cells`] for windows of it; the whole
/// grid is what the tests hold every window to.
#[cfg(test)]
pub(crate) fn area_averaged(spec: &ImageSpec, reduced: Reduction, threads: usize) -> ImageSpec {
    let Reduction { width, height, .. } = reduced;
    ImageSpec {
        width,
        height,
        data: area_averaged_cells(spec, reduced, 0..width, 0..height, threads, |_| ()).into(),
    }
}

/// The cells `columns` × `rows` of [`area_averaged`]'s grid, row-major, four straight-alpha
/// bytes each: the same bytes as the whole grid's cells at those places, because each cell
/// reads only its own block (the module comment). A range past the grid is cut at its edge.
///
/// `finish` is handed each row as it is made, on the thread that made it — where the
/// device premultiplies a texture's row, so that step is divided among the threads with the
/// reduction instead of running on one thread after it (ADR 1493).
pub(crate) fn area_averaged_cells(
    spec: &ImageSpec,
    reduced: Reduction,
    columns: Range<u32>,
    rows: Range<u32>,
    threads: usize,
    finish: impl Fn(&mut [u8]) + Sync,
) -> Vec<u8> {
    let Reduction { width, height, .. } = reduced;
    let columns = columns.start.min(width)..columns.end.min(width);
    let rows = rows.start.min(height)..rows.end.min(height);
    let row_bands = Bands::new(spec.height, height);
    let column_bands = Bands::new(spec.width, width);
    let spans: Vec<(u32, u32)> = columns
        .clone()
        .map(|out_x| column_bands.at(out_x))
        .collect();
    let reciprocals = Reciprocals::new(
        u64::from(spec.width.div_ceil(width.max(1)))
            .saturating_mul(u64::from(spec.height.div_ceil(height.max(1)))),
    );
    let row_bytes = (columns.len()).saturating_mul(4);
    let mut data: Vec<u8> = vec![0; row_bytes.saturating_mul(rows.len())];
    let first_row = rows.start as usize;
    let fill = |out_y: usize, row: &mut [u8], sums: &mut Vec<u32>| {
        let at = u32::try_from(first_row.saturating_add(out_y)).unwrap_or(u32::MAX);
        reduce_row(spec, row_bands.at(at), &spans, &reciprocals, sums, row);
        finish(row);
    };
    let take = row_bytes.saturating_mul(ROWS_PER_TAKE);
    // The window's own source samples decide whether threads pay, not the image's.
    let source_columns = spans
        .first()
        .zip(spans.last())
        .map_or(0, |(first, last)| last.1.saturating_sub(first.0));
    let source_rows = row_bands
        .at(rows.end)
        .0
        .saturating_sub(row_bands.at(rows.start).0);
    let samples = u64::from(source_columns).saturating_mul(u64::from(source_rows));
    if threads > 1 && samples >= PARALLEL_FLOOR && take > 0 && data.len() > take {
        // Each thread takes the next band of rows from one counter; a band is written by
        // exactly the thread that took it, through the lock that hands it over.
        let bands: Vec<std::sync::Mutex<&mut [u8]>> =
            data.chunks_mut(take).map(std::sync::Mutex::new).collect();
        let next = std::sync::atomic::AtomicUsize::new(0);
        let work = || {
            let mut sums = Vec::new();
            loop {
                let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(band) = bands.get(index) else {
                    return;
                };
                let Ok(mut band) = band.lock() else {
                    return;
                };
                let first = index.saturating_mul(ROWS_PER_TAKE);
                for (offset, row) in band.chunks_exact_mut(row_bytes).enumerate() {
                    fill(first.saturating_add(offset), row, &mut sums);
                }
            }
        };
        let helpers = threads.min(bands.len()).saturating_sub(1);
        crate::threads::count(helpers);
        std::thread::scope(|scope| {
            for _ in 0..helpers {
                scope.spawn(work);
            }
            work();
        });
    } else {
        let mut sums = Vec::new();
        for (out_y, row) in data.chunks_exact_mut(row_bytes).enumerate() {
            fill(out_y, row, &mut sums);
        }
    }
    data
}

/// The caller's `factor`: how many source samples share a device pixel along one axis,
/// floored, clamped into the image.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // their casts, mirrored: dimensions are far below f32's exact range and the clamp
// bounds the result
fn factor(samples: u32, device: f32) -> u32 {
    if !device.is_finite() || device <= 0.0 {
        return 1;
    }
    ((samples as f32) / device)
        .floor()
        .clamp(1.0, samples as f32) as u32
}

/// The caller's `geom::length`.
fn length(dx: f32, dy: f32) -> f32 {
    (dx * dx + dy * dy).sqrt()
}

/// Proportional band boundaries — the caller's `Bands`, with their reason: fixed
/// multiples of the factor leave a short block at the edge occupying a whole output
/// cell, which squeezes the image into less than the unit square.
#[derive(Clone, Copy)]
struct Bands {
    samples: u64,
    cells: u64,
}

impl Bands {
    fn new(samples: u32, cells: u32) -> Self {
        Self {
            samples: u64::from(samples),
            cells: u64::from(cells.max(1)),
        }
    }

    fn at(self, index: u32) -> (u32, u32) {
        let edge = |i: u64| {
            let scaled = i.saturating_mul(self.samples).checked_div(self.cells);
            u32::try_from(scaled.unwrap_or(0).min(self.samples)).unwrap_or(u32::MAX)
        };
        let start = edge(u64::from(index));
        (start, edge(u64::from(index).saturating_add(1)).max(start))
    }
}

/// The largest block, in samples, whose opaque mean is read off column sums — the caller's
/// `SMALL_BLOCK`, for their reason: the sums stay far inside `u32` and [`Reciprocals`] is
/// exact up to it.
const SMALL_BLOCK: u64 = 4_095;

/// `⌈2³² ÷ c⌉` for every sample count `c` up to a reduction's largest block — the caller's
/// `Reciprocals`, whose comment proves the product the quotient for a numerator under
/// `256·c` and `c` up to [`SMALL_BLOCK`]. Empty past that, and every cell then takes
/// [`average_block`].
struct Reciprocals(Vec<u64>);

impl Reciprocals {
    fn new(largest: u64) -> Self {
        if largest > SMALL_BLOCK {
            return Self(Vec::new());
        }
        Self(
            (0..=largest)
                .map(|count| (1u64 << 32).div_ceil(count.max(1)))
                .collect(),
        )
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn get(&self, count: u32) -> Option<u64> {
        self.0.get(usize::try_from(count).ok()?).copied()
    }
}

/// The caller's `reduce_row`, over the source columns `spans` reaches rather than the whole
/// width: a band of source rows `y0..y1` that is opaque throughout those columns summed down
/// each of them once and every cell read off those sums by [`Reciprocals`], and any other
/// band's cells [`average_block`]'s. Their comment has the proof that an opaque block's plain
/// mean, rounded as `⌊(s + ⌊c/2⌋) ÷ c⌋`, is the premultiplied arithmetic's byte — which is
/// also why the window may take the sums where the whole width would not: either path gives
/// an opaque block that byte.
#[expect(clippy::arithmetic_side_effects)] // bounded by `SMALL_BLOCK` as the caller's is
// (every sum under 2^20, every product under 2^52), and mirrored
fn reduce_row(
    spec: &ImageSpec,
    (y0, y1): (u32, u32),
    spans: &[(u32, u32)],
    reciprocals: &Reciprocals,
    sums: &mut Vec<u32>,
    row: &mut [u8],
) {
    let line = (spec.width as usize) * 4;
    let (left, right) = spans
        .first()
        .zip(spans.last())
        .map_or((0, 0), |(first, last)| (first.0 as usize, last.1 as usize));
    // Each source row's part under the window, in order; `None` where the data is short.
    let window: Option<Vec<&[u8]>> = (y0..y1)
        .map(|y| {
            let start = (y as usize) * line;
            spec.data.get(start + left * 4..start + right * 4)
        })
        .collect();
    let band = if reciprocals.is_empty() || line == 0 || right <= left {
        None
    } else {
        window
    };
    let Some(band) = band else {
        for (cell, &(x0, x1)) in row.chunks_exact_mut(4).zip(spans) {
            cell.copy_from_slice(&average_block(spec, x0, y0, x1, y1));
        }
        return;
    };
    if !band.iter().all(|row| opaque_band(row)) {
        translucent_row(&band, left, spans, y1 - y0, sums, row);
        return;
    }
    sums.clear();
    sums.resize((right - left) * 4, 0);
    for source in band {
        for (sum, &byte) in sums.iter_mut().zip(source) {
            *sum += u32::from(byte);
        }
    }
    let lines = y1 - y0;
    for (cell, &(x0, x1)) in row.chunks_exact_mut(4).zip(spans) {
        let count = (x1 - x0) * lines;
        let mut block = [0u32; 4];
        let columns = sums
            .get((x0 as usize - left) * 4..(x1 as usize - left) * 4)
            .unwrap_or_default();
        for column in columns.chunks_exact(4) {
            block[0] += column[0];
            block[1] += column[1];
            block[2] += column[2];
            block[3] += column[3];
        }
        match reciprocals.get(count) {
            Some(reciprocal) if count > 0 && block[3] == 255 * count => {
                let mean = |sum: u32| {
                    let rounded = (u64::from(sum + count / 2) * reciprocal) >> 32;
                    u8::try_from(rounded).unwrap_or(u8::MAX)
                };
                cell.copy_from_slice(&[mean(block[0]), mean(block[1]), mean(block[2]), 255]);
            }
            _ => cell.copy_from_slice(&average_block(spec, x0, y0, x1, y1)),
        }
    }
}
/// A band that is not opaque throughout, its cells read off premultiplied column sums: each
/// column's `Σ c·a` for the three components and `Σ a` taken once down the band, each cell's
/// sums added across its columns, and the cell finished by [`average_block`]'s own last
/// lines. Integer addition does not care about order, so every cell divides the same three
/// sums by the same alpha sum as [`average_block`] would, and is its byte (ADR 1493); what
/// changes is that a sample is read once per band rather than through a per-cell loop.
///
/// Bounded as the opaque path is: a block holds at most [`SMALL_BLOCK`] samples, so a column
/// of one holds at most that many products of at most 255 · 255, under 2^28, and so does a
/// cell.
#[expect(clippy::arithmetic_side_effects)] // bounded as the line above states
fn translucent_row(
    band: &[&[u8]],
    left: usize,
    spans: &[(u32, u32)],
    lines: u32,
    sums: &mut Vec<u32>,
    row: &mut [u8],
) {
    // The first row is written over the sums rather than added to zeros: clearing a row of
    // sums per output row was a fifth of this function's instructions on a threefold
    // reduction, whose band is three rows.
    sums.resize(band.first().map_or(0, |source| source.len()), 0);
    let mut sources = band.iter();
    if let Some(first) = sources.next() {
        for (sum, sample) in sums.chunks_exact_mut(4).zip(first.chunks_exact(4)) {
            let alpha = u32::from(sample[3]);
            sum[0] = u32::from(sample[0]) * alpha;
            sum[1] = u32::from(sample[1]) * alpha;
            sum[2] = u32::from(sample[2]) * alpha;
            sum[3] = alpha;
        }
    }
    for source in sources {
        for (sum, sample) in sums.chunks_exact_mut(4).zip(source.chunks_exact(4)) {
            let alpha = u32::from(sample[3]);
            sum[0] += u32::from(sample[0]) * alpha;
            sum[1] += u32::from(sample[1]) * alpha;
            sum[2] += u32::from(sample[2]) * alpha;
            sum[3] += alpha;
        }
    }
    for (cell, &(x0, x1)) in row.chunks_exact_mut(4).zip(spans) {
        let count = u64::from((x1 - x0) * lines);
        let mut block = [0u32; 4];
        let columns = sums
            .get((x0 as usize - left) * 4..(x1 as usize - left) * 4)
            .unwrap_or_default();
        for column in columns.chunks_exact(4) {
            block[0] += column[0];
            block[1] += column[1];
            block[2] += column[2];
            block[3] += column[3];
        }
        let alpha_sum = u64::from(block[3]);
        if count == 0 || alpha_sum == 0 {
            cell.copy_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        cell.copy_from_slice(&[
            round_div(u64::from(block[0]), alpha_sum),
            round_div(u64::from(block[1]), alpha_sum),
            round_div(u64::from(block[2]), alpha_sum),
            round_div(alpha_sum, count),
        ]);
    }
}

/// Whether every sample of a band of RGBA rows is opaque — the caller's `opaque_band`: eight
/// bytes folded at a time, the two alpha lanes tested once.
fn opaque_band(band: &[u8]) -> bool {
    const ALPHAS: u64 = u64::from_le_bytes([0, 0, 0, 0xFF, 0, 0, 0, 0xFF]);
    let mut words = band.chunks_exact(8);
    let folded = words.by_ref().fold(u64::MAX, |all, word| {
        let mut eight = [0u8; 8];
        eight.copy_from_slice(word);
        all & u64::from_le_bytes(eight)
    });
    folded & ALPHAS == ALPHAS
        && words
            .remainder()
            .get(3)
            .is_none_or(|&alpha| alpha == u8::MAX)
}

/// The caller's `average_block`: the mean of one block as straight-alpha RGBA8,
/// averaged premultiplied and divided back out, with their overflow argument (a block
/// holds at most `u32::MAX` samples of at most `255 × 255` each — under 2⁴⁸ in a u64).
#[expect(clippy::arithmetic_side_effects)] // bounded as the line above states, and
// mirrored: a saturating version here would be a different arithmetic than the oracle's
fn average_block(spec: &ImageSpec, x0: u32, y0: u32, x1: u32, y1: u32) -> [u8; 4] {
    let mut colour = [0u64; 3];
    let mut alpha_sum = 0u64;
    let mut count = 0u64;
    for y in y0..y1 {
        let row = (y as usize) * (spec.width as usize);
        let from = (row + x0 as usize) * 4;
        let to = (row + x1 as usize) * 4;
        let Some(span) = spec.data.get(from..to) else {
            continue;
        };
        for sample in span.chunks_exact(4) {
            let alpha = u64::from(sample[3]);
            for (sum, component) in colour.iter_mut().zip(sample) {
                *sum += u64::from(*component) * alpha;
            }
            alpha_sum += alpha;
            count += 1;
        }
    }
    if count == 0 || alpha_sum == 0 {
        return [0, 0, 0, 0];
    }
    let mut out = [0u8; 4];
    for (channel, sum) in out.iter_mut().zip(colour) {
        *channel = round_div(sum, alpha_sum);
    }
    out[3] = round_div(alpha_sum, count);
    out
}

/// The caller's `round_div`: round to nearest, clamp into a byte — truncation would
/// darken every reduced image by up to one level per component.
fn round_div(numerator: u64, denominator: u64) -> u8 {
    if denominator == 0 {
        return 0;
    }
    let rounded = numerator
        .saturating_add(denominator / 2)
        .checked_div(denominator)
        .unwrap_or(0);
    u8::try_from(rounded.min(u64::from(u8::MAX))).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn spec(width: u32, height: u32, data: Vec<u8>) -> ImageSpec {
        ImageSpec {
            width,
            height,
            data: Arc::from(data),
        }
    }

    /// A 4×4 checkerboard reduced twofold: every block holds two black and two white
    /// opaque samples, and their premultiplied mean is exactly 128 (255·2/4 → 127.5,
    /// rounded up) — the arithmetic asserted at the byte, which is the claim "mirrored
    /// statement for statement" has to cash.
    #[test]
    fn a_checkerboard_reduces_to_its_mean() {
        let mut data = Vec::new();
        for y in 0..4u32 {
            for x in 0..4u32 {
                let on = (x + y) % 2 == 0;
                let v = if on { 255 } else { 0 };
                data.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let source = spec(4, 4, data);
        let placement = [2.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        let reduced = reduction(&source, false, &placement).expect("factor 2 both ways");
        assert_eq!(reduced.factors, (2, 2));
        assert_eq!((reduced.width, reduced.height), (2, 2));
        let out = area_averaged(&source, reduced, 1);
        for cell in out.data.chunks_exact(4) {
            assert_eq!(cell, [128, 128, 128, 255]);
        }
    }

    /// The filter rule at every rung: magnified without `/Interpolate` is flat
    /// rectangles; below the image's own size the filter stays on; the flag filters
    /// everywhere but at one device pixel per sample, where §10.7.4's point sample is
    /// the answer whatever the flag says.
    #[test]
    fn the_filter_follows_the_clause() {
        assert!(!smoothed(8, 8, false, &[16.0, 0.0, 0.0, 16.0, 0.0, 0.0]));
        assert!(smoothed(8, 8, false, &[4.0, 0.0, 0.0, 4.0, 0.0, 0.0]));
        assert!(smoothed(8, 8, true, &[16.0, 0.0, 0.0, 16.0, 0.0, 0.0]));
        assert!(!smoothed(8, 8, false, &[8.0, 0.0, 0.0, 8.0, 0.5, 0.5]));
        assert!(!smoothed(8, 8, true, &[8.0, 0.0, 0.0, -8.0, 0.5, 8.5]));
    }

    /// A transparent sample pulls nothing: the mean is premultiplied, so a block of
    /// one opaque red and one transparent green is red at half alpha, not brown.
    #[test]
    fn transparency_carries_no_colour() {
        let data = vec![255, 0, 0, 255, 0, 255, 0, 0];
        let source = spec(2, 1, data);
        let out = area_averaged(
            &source,
            Reduction {
                factors: (2, 1),
                width: 1,
                height: 1,
                smoothed: true,
            },
            1,
        );
        assert_eq!(&out.data[..], &[255, 0, 0, 128]);
    }

    /// A row read off column sums is the per-block arithmetic's, cell for cell: an opaque
    /// grid and one whose every seventh alpha is not 255, each reduced at factors whose blocks
    /// are uneven, against [`average_block`] asked of each cell's block.
    #[test]
    fn a_row_of_column_sums_is_the_per_block_arithmetic() {
        let (width, height) = (301_u32, 257_u32);
        let mut state = 0x9E37_79B9_u32;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_le_bytes()
        };
        for opaque in [true, false] {
            let data: Vec<u8> = (0..width * height)
                .flat_map(|index| {
                    let v = next();
                    let alpha = if opaque || index % 7 != 0 { 255 } else { v[3] };
                    [v[0], v[1], v[2], alpha]
                })
                .collect();
            let source = spec(width, height, data);
            for placement in [
                [150.0, 0.0, 0.0, 128.0, 0.0, 0.0],
                [100.0, 0.0, 0.0, 70.0, 0.0, 0.0],
                [7.0, 0.0, 0.0, 5.0, 0.0, 0.0],
            ] {
                let reduced = reduction(&source, false, &placement).expect("a reduction");
                let out = area_averaged(&source, reduced, 1);
                let rows = Bands::new(height, reduced.height);
                let columns = Bands::new(width, reduced.width);
                for (index, cell) in out.data.chunks_exact(4).enumerate() {
                    let index = u32::try_from(index).expect("a small grid");
                    let (x, y) = (index % reduced.width, index / reduced.width);
                    let ((x0, x1), (y0, y1)) = (columns.at(x), rows.at(y));
                    assert_eq!(
                        cell,
                        average_block(&source, x0, y0, x1, y1),
                        "cell ({x}, {y})"
                    );
                }
            }
        }
    }

    /// A translucent band's cells read off premultiplied column sums are the per-block
    /// arithmetic's (ADR 1493): every alpha drawn at random, a quarter of the samples
    /// transparent and whole transparent rows, so that a cell with no alpha at all is met, at
    /// uneven factors.
    #[test]
    fn a_translucent_row_is_the_per_block_arithmetic() {
        let (width, height) = (211_u32, 173_u32);
        let mut state = 0x6C07_8965_u32;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_le_bytes()
        };
        let data: Vec<u8> = (0..width * height)
            .flat_map(|index| {
                let v = next();
                let alpha = if (index / width) % 9 == 4 || v[0] < 64 {
                    0
                } else {
                    v[3]
                };
                [v[0], v[1], v[2], alpha]
            })
            .collect();
        let source = spec(width, height, data);
        for placement in [
            [100.0, 0.0, 0.0, 70.0, 0.0, 0.0],
            [30.0, 0.0, 0.0, 17.0, 0.0, 0.0],
            [5.0, 0.0, 0.0, 3.0, 0.0, 0.0],
        ] {
            let reduced = reduction(&source, false, &placement).expect("a reduction");
            let out = area_averaged(&source, reduced, 1);
            let rows = Bands::new(height, reduced.height);
            let columns = Bands::new(width, reduced.width);
            for (index, cell) in out.data.chunks_exact(4).enumerate() {
                let index = u32::try_from(index).expect("a small grid");
                let (x, y) = (index % reduced.width, index / reduced.width);
                let ((x0, x1), (y0, y1)) = (columns.at(x), rows.at(y));
                assert_eq!(
                    cell,
                    average_block(&source, x0, y0, x1, y1),
                    "cell ({x}, {y})"
                );
            }
        }
    }

    /// [`Reciprocals`]' product is the quotient at every count it holds, checked at each
    /// numerator where a floor changes — one below and at every multiple of the count — up to
    /// past the largest an opaque block's rounded sum can be.
    #[test]
    fn a_reciprocal_is_the_quotient() {
        let reciprocals = Reciprocals::new(SMALL_BLOCK);
        for count in 1..=u32::try_from(SMALL_BLOCK).unwrap_or(0) {
            let reciprocal = reciprocals.get(count).expect("a reciprocal");
            for multiple in 0..=256u64 {
                for numerator in [
                    (multiple * u64::from(count)).saturating_sub(1),
                    multiple * u64::from(count),
                ] {
                    assert_eq!(
                        (numerator * reciprocal) >> 32,
                        numerator / u64::from(count),
                        "{numerator} / {count}"
                    );
                }
            }
        }
    }

    /// A window of cells is the whole grid's cells at those places (ADR 1493): an opaque grid
    /// and one whose every seventh alpha is not 255, at uneven factors, every window that
    /// starts and ends on or beside an edge and some inside, on one thread and on four —
    /// including windows whose source columns are opaque where the whole band is not, which
    /// take the column sums where the whole grid takes the per-block arithmetic.
    #[test]
    fn a_window_of_cells_is_the_whole_grid_cropped() {
        let (width, height) = (301_u32, 257_u32);
        let mut state = 0x2545_F491_u32;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_le_bytes()
        };
        for opaque in [true, false] {
            let data: Vec<u8> = (0..width * height)
                .flat_map(|index| {
                    let v = next();
                    // Transparency only in the right third, so a window to its left has
                    // opaque bands the whole grid does not.
                    let alpha = if opaque || index % width < 200 || index % 7 != 0 {
                        255
                    } else {
                        v[3]
                    };
                    [v[0], v[1], v[2], alpha]
                })
                .collect();
            let source = spec(width, height, data);
            for placement in [
                [100.0, 0.0, 0.0, 70.0, 0.0, 0.0],
                [7.0, 0.0, 0.0, 5.0, 0.0, 0.0],
            ] {
                let reduced = reduction(&source, false, &placement).expect("a reduction");
                let whole = area_averaged(&source, reduced, 1);
                let (w, h) = (reduced.width, reduced.height);
                for (columns, rows) in [
                    (0..w, 0..h),
                    (0..1, 0..1),
                    (w - 1..w, h - 1..h),
                    (1..w / 2, 2..h - 1),
                    (w / 3..w, 0..h / 2),
                    (0..w / 2, h / 3..h + 5),
                ] {
                    for threads in [1, 4] {
                        let cells = area_averaged_cells(
                            &source,
                            reduced,
                            columns.clone(),
                            rows.clone(),
                            threads,
                            |_| (),
                        );
                        let mut cropped = Vec::new();
                        for y in rows.start..rows.end.min(h) {
                            let start = ((y * w + columns.start) * 4) as usize;
                            let end = ((y * w + columns.end.min(w)) * 4) as usize;
                            cropped.extend_from_slice(&whole.data[start..end]);
                        }
                        assert_eq!(
                            cells, cropped,
                            "{columns:?} × {rows:?}, {threads} thread(s)"
                        );
                    }
                }
            }
        }
    }

    /// Dividing the rows among threads moves no byte: a grid above [`PARALLEL_FLOOR`]
    /// whose bands do not divide its rows evenly, with alpha varying so that the
    /// premultiplied sums are exercised, reduced on one thread and on five.
    #[test]
    fn a_thread_count_changes_no_byte() {
        let (width, height) = (301_u32, 257_u32);
        let data: Vec<u8> = (0..width * height * 4)
            .map(|index| u8::try_from((index.wrapping_mul(2_654_435_761)) >> 24).unwrap_or(0))
            .collect();
        let source = spec(width, height, data);
        let placement = [100.0, 0.0, 0.0, 70.0, 0.0, 0.0];
        let reduced = reduction(&source, false, &placement).expect("a reduction");
        let one = area_averaged(&source, reduced, 1);
        let five = area_averaged(&source, reduced, 5);
        assert!(u64::from(width) * u64::from(height) >= PARALLEL_FLOOR);
        assert_eq!(one.data, five.data);
    }
}
