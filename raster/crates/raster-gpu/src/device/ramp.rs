//! A colour ramp, as a table of its own stops: what the shader interpolates between.
//!
//! ISO 32000-2 §8.7.4.5's axial and radial shadings are a colour function over a parameter,
//! and §8.7.4.5.3 makes a point's colour that function's value at the point's own `t` (Table
//! 79's `Domain`: "\[t]he variable t becomes the input argument to the colour function(s)").
//! A ramp is that function already sampled onto stops, linear between neighbours (§7.10.3's
//! type 2 with `N` of 1) and stitched at coincident pairs (§7.10.4), so the colour at `t` is
//! the interpolation between the two stops either side of `t`, evaluated **at `t`**.
//!
//! **So the table holds the stops and not a grid of colours sampled from them** (ADR 2310).
//! A grid read at its nearest texel moves `t` by up to half a texel, and what that costs in
//! colour is the ramp's slope times half a texel: a fraction of a level for a ramp that crosses
//! the range once, and four tenths of one for `issue14165.pdf`'s, which crosses it eleven times
//! along a page-tall axis — 149 of the page's 842 rows a level off the clause's value. Reading
//! the stops instead puts every pixel at its own `t` on every ramp, hard steps included, and
//! the arithmetic stays ours rather than the driver's (ADR 0011): the shader reads each word
//! with `textureLoad`, no sampler filters a ramp, and the colour leaves the shader as the
//! level nearest it, so no adapter's conversion of a fragment's output picks the byte.
//!
//! Nothing here touches the GPU, which is why it is not in the file that owns one. The
//! texture these bytes become is [`super::textures`]'s, and which ramps a frame needs at all
//! is [`super::resident`]'s.

use raster_scene::{Color, Stop};

/// Stops per row of a ramp's table: the texture's width once a ramp has this many stops,
/// and the ramp's own stop count below it, so a two-stop ramp is a texture two texels wide.
pub(super) const RAMP_WIDTH: u32 = 4096;

/// Rows of one block of a ramp's table: the stops' offsets, then their red, green, blue and
/// alpha, each texel one `f32`'s little-endian bytes — the five words the shader's
/// `ramp_stop` reads for one stop.
pub(super) const RAMP_ROWS: u32 = 5;

/// The most stops one ramp's table holds: [`RAMP_WIDTH`] a block and as many blocks as stay
/// within [`RAMP_WIDTH`] rows, the side this device already asks of every adapter for a ramp.
///
/// It is a bound on a texture's side rather than on fidelity, and it is far from anything a
/// shading states: `pdf_render::Ramp` holds at most its 4096 samples of a function, or three
/// a stitching interval where a function has more intervals than a third of that, before its
/// collinear stops are dropped. The upload refuses a longer ramp by name
/// (`ResourceProblem::RampTooLong`) rather than drawing part of it.
pub(crate) const MAX_RAMP_STOPS: usize = (RAMP_WIDTH as usize) * (RAMP_WIDTH / RAMP_ROWS) as usize;

/// A ramp's table: the texture's two sides and its RGBA8 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RampTable {
    /// Texels per row.
    pub(super) width: u32,
    /// Rows, [`RAMP_ROWS`] a block.
    pub(super) height: u32,
    /// `width × height` texels of four bytes.
    pub(super) bytes: Vec<u8>,
}

/// A validated ramp as its table (ADR 2310).
///
/// Stop `i` sits in column `i mod W` of block `⌊i / W⌋`, `W` being the table's width; the
/// block's rows are the offset and the four components, each the `f32`'s little-endian bytes.
/// A last block the stops do not fill is filled with the last stop again: the shader takes the
/// stop count from the texture's size, and a stop repeated at the last offset changes no `t`'s
/// colour — every `t` below that offset finds its interval before reaching them, and every `t`
/// at or above it takes the last stop's colour either way, §7.10.1's clipping and §7.10.4's
/// last interval closed on the right.
///
/// The upload refused an empty ramp and one past [`MAX_RAMP_STOPS`]; asked anyway, an empty
/// ramp is one transparent stop, a colour and not a texture of no texels.
// Every slot below is inside `bytes`, whose length is the table's own texel count.
#[expect(clippy::arithmetic_side_effects)]
pub(super) fn ramp_table(stops: &[Stop]) -> RampTable {
    let transparent = [Stop {
        offset: 0.0,
        color: Color::new(0.0, 0.0, 0.0, 0.0),
    }];
    let stops = if stops.is_empty() {
        &transparent
    } else {
        stops
    };
    let stops = &stops[..stops.len().min(MAX_RAMP_STOPS)];
    let width = stops.len().min(RAMP_WIDTH as usize);
    let blocks = stops.len().div_ceil(width);
    let rows = RAMP_ROWS as usize;
    let mut bytes = vec![0_u8; width * rows * blocks * 4];
    let last = stops[stops.len() - 1];
    for slot in 0..width * blocks {
        let stop = stops.get(slot).copied().unwrap_or(last);
        let (block, column) = (slot / width, slot % width);
        let words = [
            stop.offset,
            stop.color.r,
            stop.color.g,
            stop.color.b,
            stop.color.a,
        ];
        for (row, word) in words.into_iter().enumerate() {
            let at = ((block * rows + row) * width + column) * 4;
            bytes[at..at + 4].copy_from_slice(&word.to_le_bytes());
        }
    }
    RampTable {
        width: u32::try_from(width).unwrap_or(RAMP_WIDTH),
        height: u32::try_from(rows * blocks).unwrap_or(u32::MAX),
        bytes,
    }
}

/// The colour the shader's `ramp_interpolated` gives `t`, stated in Rust over the bytes
/// [`ramp_table`] writes — the lookup the tests hold to [`ramp_color_at`] bit for bit. The
/// WGSL function is this function's copy, statement for statement, and the shader's `fs_main`
/// hands the target the level nearest an opaque answer, `⌊255 c + ½⌋ / 255`, so that no adapter's
/// conversion of a fragment's output chooses the byte instead (ADR 2310); an answer below full
/// alpha reaches the blend as it is (ADR 2350).
///
/// At or below the first offset the first stop's colour (§7.10.4's degenerate first interval,
/// closed on both sides); otherwise the first stop whose offset is **above** `t` is found by
/// halving (§7.10.4's intervals are closed on the left), and the colour is the interpolation
/// between it and the stop before it; with none above, the last stop's.
#[cfg(test)]
#[expect(clippy::arithmetic_side_effects)] // slots inside the table
pub(super) fn colour_for(table: &RampTable, t: f32) -> Color {
    let width = table.width as usize;
    let count = width * (table.height / RAMP_ROWS) as usize;
    let word = |index: usize, row: usize| {
        let at = (((index / width) * RAMP_ROWS as usize + row) * width + index % width) * 4;
        f32::from_le_bytes([
            table.bytes[at],
            table.bytes[at + 1],
            table.bytes[at + 2],
            table.bytes[at + 3],
        ])
    };
    let colour = |index: usize| {
        Color::new(
            word(index, 1),
            word(index, 2),
            word(index, 3),
            word(index, 4),
        )
    };
    if t <= word(0, 0) {
        return colour(0);
    }
    let (mut lo, mut hi) = (1, count);
    while lo < hi {
        let mid = usize::midpoint(lo, hi);
        if t < word(mid, 0) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    if lo == count {
        return colour(count - 1);
    }
    let (previous, next) = (colour(lo - 1), colour(lo));
    let low = word(lo - 1, 0);
    let u = (t - low) / (word(lo, 0) - low);
    let mix = |a: f32, b: f32| a + (b - a) * u;
    Color::new(
        mix(previous.r, next.r),
        mix(previous.g, next.g),
        mix(previous.b, next.b),
        mix(previous.a, next.a),
    )
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
/// **The device asks this nowhere**: the shader's `ramp_interpolated` is [`colour_for`]'s copy,
/// and `tests.rs` holds that statement to this one over every ramp shape it generates.
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
