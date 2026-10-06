//! A colour ramp, sampled: the sweep between two stops, evaluated once on the CPU.
//!
//! ISO 32000-2 §8.7.4.5's axial and radial shadings are a colour function over a
//! parameter, and this device draws them by reading a table rather than by evaluating
//! one per pixel. The table's arithmetic is deliberately ours rather than the driver's
//! (ADR 0011): the shader reads it with `textureLoad` at a rounded index, so every
//! adapter gets the same texel, where filtering between two texels would not promise
//! that.
//!
//! **A table may not move a hard step** (ADR 1389). §8.7.4.5.3 makes a point's colour the
//! function's at that point's own `t`, so where the function jumps — a §7.10.4 stitching
//! bound, which arrives here as two stops at one offset — the pixel whose centre lies
//! below the bound takes the lower piece whatever grid the table is on. So the table is
//! cut at every such bound into **segments**, each sampled on its own grid, and the bounds
//! travel beside it as exact `f32`s: the shader finds the segment by comparing `t` with
//! the bounds, and rounds only within it. A smooth ramp is one segment, and its texels
//! and its lookup are what they were before the cut, to the bit.
//!
//! Nothing here touches the GPU, which is why it is not in the file that owns one. The
//! texture these bytes become is [`super::textures`]'s, and which ramps a frame needs
//! at all is [`super::resident`]'s.

use raster_scene::{Color, Stop};

/// Texels per row of a sampled ramp, and the most texels the colour row spends on one
/// ramp. At 4096 a smooth ramp's nearest-texel rounding moves `t` by at most half of
/// `1/4095`, a fraction of one 8-bit level between any two stops; a hard step is not
/// rounded at all (ADR 1389), so this number no longer bounds where a step lands.
pub(super) const RAMP_RESOLUTION: u32 = 4096;

/// Rows of a sampled ramp's texture: the colours, the segment bounds, the segment
/// layout — the three the shader's `ramp_texel` reads, in that order.
pub(super) const RAMP_ROWS: u32 = 3;

/// The most segments a ramp is cut into. Row 1 holds the first offset and then one bound
/// per segment, so it holds at most `RAMP_RESOLUTION − 1` segments, and each needs at least
/// one colour texel; past this a ramp is sampled as one segment, which rounds its steps to
/// the nearest texel (half of `1/4095` of the parameter either way), the answer this table
/// gave every ramp before ADR 1389. No stitching the corpus holds comes near it.
const MAX_SEGMENTS: usize = 1024;

/// One run of stops with no hard step inside it, and where its texels sit in row 0.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Segment {
    /// The stops `first..=last` of the ramp.
    first: usize,
    last: usize,
    /// Its first colour texel, and how many it has.
    base: u32,
    texels: u32,
}

/// The ramp's segments: cut after every stop whose successor shares its offset.
///
/// A run of three or more stops at one offset leaves zero-width segments between them;
/// one in the middle of the ramp can hold no `t` — the interval that starts at the
/// offset is the last stop's (§7.10.4, closed on the left) — so it is dropped. A
/// zero-width segment at either end is kept: §7.10.4's degenerate first interval is
/// closed on both sides, and the last interval is closed on the right, so each end
/// offset is a `t` with a colour of its own.
fn segments(stops: &[Stop]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut start = 0;
    for (i, pair) in stops.windows(2).enumerate() {
        #[expect(clippy::float_cmp)] // exact: a coincident pair is the step itself
        if pair[0].offset == pair[1].offset {
            runs.push((start, i));
            start = i.saturating_add(1);
        }
    }
    if !stops.is_empty() {
        runs.push((start, stops.len().saturating_sub(1)));
    }
    let count = runs.len();
    runs.into_iter()
        .enumerate()
        .filter(|&(index, (first, last))| {
            #[expect(clippy::float_cmp)] // exact, as above
            let zero_width = stops[first].offset == stops[last].offset;
            !zero_width || index == 0 || index.saturating_add(1) == count
        })
        .map(|(_, run)| run)
        .collect()
}

/// Share row 0's texels between the segments in proportion to their width, at least
/// one each: `1 + ⌊(w / W) · (N − K)⌋` for a segment of width `w` of `W`, `K` segments
/// and `N` texels, which sums to at most `N`. One segment spanning the ramp gets all `N`.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // floor of 0..=N
#[expect(clippy::cast_precision_loss)] // counts below 4096
fn lay_out(stops: &[Stop], runs: &[(usize, usize)]) -> Vec<Segment> {
    let (Some(first), Some(last)) = (stops.first(), stops.last()) else {
        return Vec::new();
    };
    let whole = last.offset - first.offset;
    let free = RAMP_RESOLUTION.saturating_sub(runs.len() as u32) as f32;
    let mut base = 0_u32;
    runs.iter()
        .map(|&(from, to)| {
            let width = stops[to].offset - stops[from].offset;
            let share = if whole > 0.0 {
                (width / whole * free).floor() as u32
            } else {
                0
            };
            let texels = share.saturating_add(1);
            let segment = Segment {
                first: from,
                last: to,
                base,
                texels,
            };
            base = base.saturating_add(texels);
            segment
        })
        .collect()
}

/// Sample a validated ramp to [`RAMP_RESOLUTION`] × [`RAMP_ROWS`] RGBA8 texels, on the
/// CPU (ADR 0011, ADR 1389).
///
/// - **Row 0**, the colours, straight RGBA: segment `k`'s texel `j` of `n` is the ramp at
///   `lo + (hi − lo) · j / (n − 1)`, evaluated over that segment's stops alone, so its last
///   texel is the colour *below* the step that ends it.
/// - **Row 1**, the bounds, each an `f32`'s little-endian bytes: texel 0 is the ramp's
///   first offset, texel `k + 1` segment `k`'s upper offset — so texel `k` is segment `k`'s
///   lower offset for every `k`.
/// - **Row 2**, the layout: texel 0 the segment count as a little-endian `u32`, texel
///   `k + 1` segment `k`'s first texel and texel count as two little-endian `u16`s.
///
/// The shader's `ramp_texel` is the reader and [`texel_for`] its statement in Rust.
// Every offset below is a texel slot inside one of the three rows, whose length is fixed.
#[expect(clippy::arithmetic_side_effects)]
pub(super) fn sample_ramp(stops: &[Stop]) -> Vec<u8> {
    let row = RAMP_RESOLUTION as usize * 4;
    let mut out = vec![0_u8; row.saturating_mul(RAMP_ROWS as usize)];
    let Some(first) = stops.first() else {
        return out;
    };
    let mut runs = segments(stops);
    if runs.len() > MAX_SEGMENTS {
        runs = vec![(0, stops.len().saturating_sub(1))];
    }
    let layout = lay_out(stops, &runs);
    let (colours, rest) = out.split_at_mut(row);
    let (bounds, table) = rest.split_at_mut(row);
    bounds[0..4].copy_from_slice(&first.offset.to_le_bytes());
    let count = u32::try_from(layout.len()).unwrap_or(u32::MAX);
    table[0..4].copy_from_slice(&count.to_le_bytes());
    for (k, segment) in layout.iter().enumerate() {
        let own = &stops[segment.first..=segment.last];
        let (lo, hi) = (stops[segment.first].offset, stops[segment.last].offset);
        let slot = k.saturating_add(1).saturating_mul(4);
        bounds[slot..slot + 4].copy_from_slice(&hi.to_le_bytes());
        let (base, texels) = (
            u16::try_from(segment.base).unwrap_or(u16::MAX),
            u16::try_from(segment.texels).unwrap_or(u16::MAX),
        );
        table[slot..slot + 2].copy_from_slice(&base.to_le_bytes());
        table[slot + 2..slot + 4].copy_from_slice(&texels.to_le_bytes());
        let last = f32::from(texels.saturating_sub(1).max(1));
        let start = segment.base as usize * 4;
        let run = &mut colours[start..start + segment.texels as usize * 4];
        sample_segment(own, (lo, hi), last, run);
    }
    out
}

/// One segment's colour texels: texel `j` is [`ramp_color_at`] of `own` at
/// `lo + (hi − lo) · j / last`, each component scaled to 255 and rounded half away from zero.
///
/// **The same bytes as asking [`ramp_color_at`] at every texel, in about a fifth less time**
/// (ADR 1567): `t` never falls as `j` rises — the stops were validated ascending at upload, and
/// a product by a non-negative constant and a sum with a constant are monotone in IEEE
/// arithmetic — so the stop above `t` is found by a cursor that only moves forward rather than
/// by a scan from the first stop each texel, and the interval arithmetic is
/// [`ramp_color_at`]'s statement for statement. The rounding is [`texel_byte`]. `tests.rs`
/// holds the table to the per-texel statement over every ramp shape it generates.
// Every slot is inside `run`, which is four bytes a texel; `j` is below 4096.
#[expect(clippy::arithmetic_side_effects, clippy::cast_precision_loss)]
fn sample_segment(own: &[Stop], (lo, hi): (f32, f32), last: f32, run: &mut [u8]) {
    let Some(first) = own.first() else {
        return;
    };
    let mut above = 1_usize;
    for (j, texel) in run.chunks_exact_mut(4).enumerate() {
        let t = lo + (hi - lo) * (j as f32 / last);
        let color = if t <= first.offset {
            first.color
        } else {
            while own.get(above).is_some_and(|stop| t >= stop.offset) {
                above += 1;
            }
            match (own.get(above.saturating_sub(1)), own.get(above)) {
                (Some(previous), Some(stop)) => {
                    let span = stop.offset - previous.offset;
                    let u = (t - previous.offset) / span;
                    let mix = |a: f32, b: f32| a + (b - a) * u;
                    Color::new(
                        mix(previous.color.r, stop.color.r),
                        mix(previous.color.g, stop.color.g),
                        mix(previous.color.b, stop.color.b),
                        mix(previous.color.a, stop.color.a),
                    )
                }
                (Some(previous), None) => previous.color,
                (None, _) => first.color,
            }
        };
        for (byte, component) in texel.iter_mut().zip([color.r, color.g, color.b, color.a]) {
            *byte = texel_byte(component);
        }
    }
}

/// A colour component in `0..=1` as a texel byte: `(component · 255).round()`, saturated into
/// a byte, computed without a call.
///
/// `f32::round` is a library call on the baseline x86-64 this crate is built for, which has no
/// rounding instruction, and a table is four of them per texel: on `bug1721218_reduced.pdf`'s
/// 132 ramps a frame that was 49 M instructions of its 150 M (ADR 1567). For `x` in
/// `0 ≤ x < 2²³`, truncation is exact and so is `x − ⌊x⌋` (Sterbenz), so rounding half away from
/// zero is the whole part plus one where that fraction is at least a half — the same integer
/// `round` gives. A component below zero, above one or not a number becomes what the saturating
/// cast of `round`'s answer gives it: 0, 255 and 0.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn texel_byte(component: f32) -> u8 {
    let scaled = component * 255.0;
    let whole = scaled as u32;
    let up = u32::from(scaled - whole as f32 >= 0.5);
    u8::try_from(whole.saturating_add(up)).unwrap_or(u8::MAX)
}

/// The fewest ramps a frame realises at once before their tables are made on threads beside
/// each other rather than on the frame's own: a table is about 30 µs of one thread, a thread's
/// start tens of microseconds.
const PARALLEL_FLOOR: usize = 8;

/// [`sample_ramp`] of each of `ramps`, in order, made on up to `threads` threads where there are
/// at least [`PARALLEL_FLOOR`] of them (ADR 1567).
///
/// A table is a pure function of its own stops, so which thread made it changes no byte. The
/// threads are a scope inside the frame that asked, as the encode's fan-out is (ADR 0023's "take
/// one rather than make one"): nothing outlives the call, and a host that allowed one thread
/// starts none. A first sight of `bug1721218_reduced.pdf`'s group realises 132 ramps — 67 for its
/// chromatic frame and 65 for its black one — and making them was 6 ms of the frame's own thread.
pub(super) fn sample_ramps(ramps: &[&[Stop]], threads: usize) -> Vec<Vec<u8>> {
    if threads <= 1 || ramps.len() < PARALLEL_FLOOR {
        return ramps.iter().map(|stops| sample_ramp(stops)).collect();
    }
    let share = ramps.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let made: Vec<_> = ramps
            .chunks(share)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|stops| sample_ramp(stops))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        made.into_iter()
            .zip(ramps.chunks(share))
            .flat_map(|(thread, chunk)| {
                // A sampling thread runs arithmetic over a slice it was handed and cannot
                // fail; were it ever to panic, its tables are made again here, so the frame
                // still has every one in order.
                thread
                    .join()
                    .unwrap_or_else(|_| chunk.iter().map(|stops| sample_ramp(stops)).collect())
            })
            .collect()
    })
}

/// The colour texel the shader's `ramp_texel` reads for `t`, stated in Rust over the bytes
/// [`sample_ramp`] writes — the lookup the tests hold to the clauses. The WGSL function
/// is this function's copy, statement for statement.
///
/// The segment is the first whose upper offset is **above** `t` (§7.10.4's intervals are
/// closed on the left), or the last; a `t` at or below the first offset is the first
/// segment's (its degenerate interval is closed on both sides). Within the segment the
/// index rounds to the nearest of its own texels.
#[cfg(test)]
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[expect(clippy::cast_precision_loss)]
#[expect(clippy::arithmetic_side_effects)] // slots inside the table
pub(super) fn texel_for(table: &[u8], t: f32) -> usize {
    let row = RAMP_RESOLUTION as usize * 4;
    let word = |r: usize, x: usize| {
        let at = r * row + x * 4;
        u32::from_le_bytes([table[at], table[at + 1], table[at + 2], table[at + 3]])
    };
    let bound = |x: usize| f32::from_bits(word(1, x));
    let count = word(2, 0) as usize;
    let mut k = 0;
    if t > bound(0) {
        let (mut lo, mut hi) = (0, count - 1);
        while lo < hi {
            let mid = usize::midpoint(lo, hi);
            if t < bound(mid + 1) {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        k = lo;
    }
    let (lower, upper) = (bound(k), bound(k + 1));
    let info = word(2, k + 1);
    let (base, texels) = ((info & 0xffff) as usize, (info >> 16) as usize);
    if texels <= 1 || upper <= lower {
        return base;
    }
    let u = ((t - lower) / (upper - lower)).clamp(0.0, 1.0);
    base + (u * (texels - 1) as f32).round() as usize
}

/// The ramp's colour at `t`: constant before the first and after the last stop, and
/// linearly interpolated between neighbours.
///
/// A ramp is a shading's colour function already sampled onto stops — ISO 32000-2
/// §8.7.4.5.3 for the axial case and §8.7.4.5.4 for the radial both say of `Domain`
/// that "[t]he variable t becomes the input argument to the colour function(s)" — and
/// the caller places *two stops at one offset* wherever that function jumps, which is a
/// §7.10.4 stitching bound. So this is the reconstruction of a stitching function, and
/// which side of a bound owns the bound is §7.10.4's to answer rather than ours.
///
/// Three clauses decide the three cases. Outside the stops, §7.10.1's Table 38 entry
/// for `Domain`:
///
/// > Input values outside the declared domain shall be clipped to the nearest boundary
/// > value.
///
/// Between two stops, §7.10.3's type 2 with the exponent of 1 that a colour ramp is:
///
/// > Each input value x shall return n values, given by yj = C0j + x N × (C1j − C0j),
/// > for 0 ≤ j < n.
///
/// On a bound, §7.10.4 (subscripts are set below the line in the original and written
/// inline here):
///
/// > The Bounds array shall describe a series of k half-open intervals, closed on the
/// > left and open on the right with the following exceptions:
/// >
/// > - the last interval, shall always be closed on the right,
/// > - if Domain0 = Bounds0 then the first interval shall be closed on both the left
/// >   and right and the second (next) interval shall be open on the left.
///
/// *Closed on the left* is why the loop below compares with `<` rather than `<=`: a `t`
/// exactly on a stop's offset belongs to the interval that **starts** there, so where
/// two stops share an offset the later one's colour applies at exactly that point. The
/// two exceptions are the two ends, and they do not point the same way:
///
/// - at the ramp's **first** offset the first interval is degenerate and closed on both
///   sides, so a coincident pair there takes the **earlier** stop's colour — which is
///   what the `t <= first.offset` below is, and it is the clause and not a convenience;
/// - at the **last** offset the last interval is closed on the right and the one before
///   it is open there, so a coincident pair takes the **later** stop's colour — which is
///   what falling out of the loop gives.
///
/// That these degenerate ends are defined at all, rather than merely not forbidden, is
/// §7.10.4 saying it a second time about the encoding: "If the last bound, Boundsk-2,
/// is equal to Domain1, then x ′ shall be defined to be Encode2(k-1). For the degenerate
/// case, if the first bound, Bounds0, is equal to Domain0 then x′ shall be defined to be
/// Encode0." — `Encode0` being the *first* subfunction's pair and `Encode2(k-1)` the
/// last's. ADR 0055 has the reasoning and the corpus round behind the comparison.
///
/// **The table asks this at no texel**: [`sample_segment`] walks a segment's texels with a
/// cursor over the same intervals, and `tests.rs` holds the table to this function asked at
/// every texel (ADR 1567), as [`texel_for`] states the shader's lookup.
#[cfg(test)]
fn ramp_color_at(stops: &[Stop], t: f32) -> Color {
    // Upload refused empty ramps; transparent black would still be an honest
    // answer for one, not an approximation of anything.
    let Some(first) = stops.first() else {
        return Color::new(0.0, 0.0, 0.0, 0.0);
    };
    // §7.10.1's clipping below the first offset, and §7.10.4's second exception on it.
    if t <= first.offset {
        return first.color;
    }
    let mut previous = *first;
    for stop in stops.iter().skip(1) {
        if t < stop.offset {
            // `previous.offset <= t < stop.offset` here: the loop returns at the first
            // stop *above* `t`, so `previous` is never above it either. The span is
            // therefore strictly positive without asking the upload's validation for it,
            // and `u` stays in `0..1` — which is the half-open interval, in arithmetic.
            let span = stop.offset - previous.offset;
            let u = (t - previous.offset) / span;
            let mix = |a: f32, b: f32| a + (b - a) * u;
            return Color::new(
                mix(previous.color.r, stop.color.r),
                mix(previous.color.g, stop.color.g),
                mix(previous.color.b, stop.color.b),
                mix(previous.color.a, stop.color.a),
            );
        }
        previous = *stop;
    }
    // Above the last offset, §7.10.1 clips; on it, §7.10.4's last interval is closed on
    // the right, so a coincident pair at the end takes this later colour.
    previous.color
}

#[cfg(test)]
mod tests;
