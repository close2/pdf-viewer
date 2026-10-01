//! A baseline `DCTDecode` codestream decoded in horizontal bands beside each other, where its
//! restart intervals make each band a codestream of its own (ADR 1433).
//!
//! # Why a restart interval is where a codestream can be cut
//!
//! ISO/IEC 10918-1 section E.2.4 (paraphrased, since only ISO 32000-2 is quoted in this tree)
//! resets the decoder at every restart marker, and what it carries from one MCU to the next is
//! gone: the DC predictions return to zero (section F.2.1.3.1), and the bit reader starts on
//! the byte after the marker, since section F.1.2.3 pads the segment before it to a whole byte.
//! So the entropy-coded data after a restart marker decodes with nothing from before it, which
//! is the whole purpose the standard gives the marker. `zune-jpeg` decodes a scan on
//! one thread, and its Huffman pass is most of what a page of one photograph costs to interpret
//! (ADR 1271 section 5) — but a codestream whose restart intervals begin at the start of an MCU
//! row can be cut into several codestreams, each one the frame's own header segments with a
//! smaller number of lines, the scan header, and the intervals that cover its rows.
//!
//! # Why each band decodes one MCU row either side of what it keeps
//!
//! The decoder's chroma upsampling is not a function of one MCU row: a vertically subsampled
//! component's first and last lines in a row are interpolated against the neighbouring rows,
//! and at a frame's top and bottom edge the decoder replicates instead. A band cut exactly at
//! its rows would put an edge where the whole frame has none, and every line next to a cut
//! would move. So a band starts decoding one interval-aligned stretch of MCU rows above the
//! first row it keeps and one MCU row below the last, and keeps only its own lines: each kept
//! line then has the same neighbours it has in the whole frame, and what the decoder computes
//! for it is the same arithmetic on the same coefficients. The first band starts at the frame's
//! top and the last ends at its bottom, where the whole frame's edges are.
//!
//! # What is admitted, and what is declined
//!
//! Admitted: a sequential Huffman frame of eight-bit precision (`SOF0`, `SOF1`), one scan
//! holding every component, a `DRI` stating a nonzero interval, sampling factors of the shapes
//! whose upsamplers the argument above was read against (a first component of 1×1, 2×1, 1×2
//! or 2×2 and the others 1×1, or one component of 1×1), and restart markers found where the
//! interval count puts them, numbered in order, with the scan ended by `EOI` or by the end of the
//! data. Anything else —
//! a progressive or lossless frame, a second scan, a marker out of place, a count that does not
//! add up — is declined, and so is any band the decoder refuses; the caller then decodes the
//! frame whole exactly as before, so a damaged codestream is read by the one decoder it always
//! was.
//!
//! **What the bands are is a function of the codestream alone**, never of how many threads
//! there are: [`BAND_LINES`] sizes them, and the pool only decides who decodes which.

use std::ops::Range;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// The fewest samples a frame must have before it is cut.
///
/// Below it the frame is decoded whole: each band re-reads the header segments and decodes the
/// rows it overlaps, and a frame that decodes in a few milliseconds has nothing to divide.
const BANDED_FLOOR: u64 = 1 << 20;

/// The fewest lines a band keeps, rounded up to whole MCU rows and whole restart periods.
///
/// Each band decodes one MCU row above and one below what it keeps, so the band's height is
/// what the overlap is paid against. On a 5 280 × 3 792 photograph, unpinned, 256 lines
/// interpreted the page in 25.7–27.8 ms against 26.8–36.6 at 512 and 31.6–42.0 at 1 024; pinned
/// to the faster cores 512 was a millisecond ahead. ADR 1433 has the table.
const BAND_LINES: u32 = 256;

/// The one scan of a baseline frame, as the parts a band is built from.
#[derive(Debug)]
struct Layout {
    /// The bytes before the scan header: `SOI`, the tables, the frame header, the `DRI`.
    header: Range<usize>,
    /// Where the frame header's number of lines `Y` sits, inside [`Self::header`].
    lines_at: usize,
    /// The scan header segment, marker included.
    scan: Range<usize>,
    /// Each restart interval's entropy-coded bytes, markers excluded, in order.
    intervals: Vec<Range<usize>>,
    /// The frame's number of lines.
    lines: u32,
    /// Lines per MCU row.
    mcu_lines: u32,
    /// MCUs in one row.
    mcus_per_row: u32,
    /// MCU rows in the frame.
    mcu_rows: u32,
    /// MCUs per restart interval.
    restart: u32,
}

/// Where one band decodes, and what of that it keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Band {
    /// The MCU rows decoded: interval-aligned at the top, one row past what is kept at the
    /// bottom unless the frame ends first.
    decoded: (u32, u32),
    /// The MCU rows kept.
    kept: (u32, u32),
}

/// Decodes `data` in bands on rayon's pool with the calling thread taking bands too, or `None`
/// where the codestream is not one the module comment admits or any band is refused.
///
/// `options` is what every band's decoder is built with; the caller's `RGBA` output is what
/// this was measured and argued for. The result is the frame's whole raster, `width × lines ×
/// channels` bytes, top row first.
/// `scan` is the caller's walk of `data`'s first scan, which this reads instead of walking it
/// again.
pub(super) fn decode(
    data: &[u8],
    scan: &super::FirstScan,
    options: zune_jpeg::zune_core::options::DecoderOptions,
    channels: usize,
) -> Option<Vec<u8>> {
    let layout = layout(data, scan)?;
    let width = frame_width(data, &layout)?;
    let samples = u64::from(width).saturating_mul(u64::from(layout.lines));
    if samples < BANDED_FLOOR {
        return None;
    }
    let bands = plan(&layout, BAND_LINES)?;
    decode_bands(data, &layout, &bands, width, options, channels)
}

/// The frame's number of samples per line, from the header [`layout`] already walked.
fn frame_width(data: &[u8], layout: &Layout) -> Option<u32> {
    let at = layout.lines_at.checked_add(2)?;
    Some(u32::from(u16::from_be_bytes([
        *data.get(at)?,
        *data.get(at.checked_add(1)?)?,
    ])))
}

/// Reads the marker segments up to the scan and the scan's restart structure, or `None` for any
/// codestream the module comment declines.
fn layout(data: &[u8], scan: &super::FirstScan) -> Option<Layout> {
    if data.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let field = |at: usize| -> Option<u16> {
        Some(u16::from_be_bytes([
            *data.get(at)?,
            *data.get(at.checked_add(1)?)?,
        ]))
    };
    let mut frame: Option<(usize, u32, u32, u8, u8, u8)> = None;
    let mut restart = 0u32;
    let mut at = 2usize;
    loop {
        let opens = at;
        if data.get(at) != Some(&0xFF) {
            return None;
        }
        while data.get(at) == Some(&0xFF) {
            at = at.checked_add(1)?;
        }
        let code = *data.get(at)?;
        at = at.checked_add(1)?;
        let length = usize::from(field(at)?);
        if length < 2 {
            return None;
        }
        let ends = at.checked_add(length)?;
        match code {
            // Baseline and extended sequential, Huffman-coded.
            0xC0 | 0xC1 => {
                if frame.is_some() {
                    return None;
                }
                frame = Some(frame_header(data, at)?);
            }
            // `DRI`: the interval, which a later `DRI` before the scan may restate.
            0xDD => restart = u32::from(field(at.checked_add(2)?)?),
            // Every other frame — progressive, lossless, hierarchical, arithmetic-coded — and
            // `DNL`, `DAC`, and the markers that have no business before the first scan.
            0xC2
            | 0xC3
            | 0xC5..=0xC7
            | 0xC9..=0xCB
            | 0xCD..=0xCF
            | 0xDC
            | 0xCC
            | 0xD0..=0xD9
            | 0x01 => return None,
            0xDA => {
                let (lines_at, lines, width, count, first, rest) = frame?;
                if *data.get(at.checked_add(2)?)? != count || restart == 0 || lines == 0 {
                    return None;
                }
                let (h, v) = (u32::from(first >> 4), u32::from(first & 0x0F));
                let shaped = rest == 0x11
                    && if count == 1 {
                        first == 0x11
                    } else {
                        matches!((h, v), (1 | 2, 1 | 2))
                    };
                if !shaped {
                    return None;
                }
                let (mcu_width, mcu_lines) = (h.checked_mul(8)?, v.checked_mul(8)?);
                let mcus_per_row = width.div_ceil(mcu_width);
                let mcu_rows = lines.div_ceil(mcu_lines);
                if scan.data_starts != ends {
                    return None;
                }
                let intervals = restart_intervals(data, scan)?;
                let total = u64::from(mcus_per_row).checked_mul(u64::from(mcu_rows))?;
                if u64::try_from(intervals.len()).ok()? != total.div_ceil(u64::from(restart)) {
                    return None;
                }
                return Some(Layout {
                    header: 0..opens,
                    lines_at,
                    scan: opens..ends,
                    intervals,
                    lines,
                    mcu_lines,
                    mcus_per_row,
                    mcu_rows,
                    restart,
                });
            }
            _ => {}
        }
        at = ends;
    }
}

/// A sequential frame header whose length field is at `at`: where its `Y` sits, `Y`, `X`, the
/// component count, the first component's sampling byte and the others' — or `None` for a
/// precision other than eight bits, a count other than one or three, or two chroma components
/// sampled differently.
fn frame_header(data: &[u8], at: usize) -> Option<(usize, u32, u32, u8, u8, u8)> {
    let field = |at: usize| -> Option<u16> {
        Some(u16::from_be_bytes([
            *data.get(at)?,
            *data.get(at.checked_add(1)?)?,
        ]))
    };
    if *data.get(at.checked_add(2)?)? != 8 {
        return None;
    }
    let lines = u32::from(field(at.checked_add(3)?)?);
    let width = u32::from(field(at.checked_add(5)?)?);
    let count = *data.get(at.checked_add(7)?)?;
    let sampling = |component: usize| -> Option<u8> {
        data.get(at.checked_add(9)?.checked_add(component.checked_mul(3)?)?)
            .copied()
    };
    let (first, rest) = match count {
        1 => (sampling(0)?, 0x11),
        3 => {
            let (second, third) = (sampling(1)?, sampling(2)?);
            if second != third {
                return None;
            }
            (sampling(0)?, second)
        }
        _ => return None,
    };
    Some((at.checked_add(3)?, lines, width, count, first, rest))
}

/// The entropy-coded data of the walked `scan`, split at its restart markers — or `None` where
/// a marker is out of sequence or the scan is ended by anything but `EOI` or the end of the
/// data.
///
/// Inside entropy-coded data an `FF` is followed by a stuffed `00`, by fill `FF`s, or by a
/// marker; the restart markers cycle through `RST0` to `RST7` in order, so a marker out of that
/// order is a codestream whose intervals cannot be trusted to be where the count puts them.
fn restart_intervals(data: &[u8], scan: &super::FirstScan) -> Option<Vec<Range<usize>>> {
    let mut intervals = Vec::new();
    let mut starts = scan.data_starts;
    for marker in &scan.inside {
        if let 0xD0..=0xD7 = marker.code {
            let expected = u8::try_from(intervals.len() % 8).ok()?;
            if marker.code != 0xD0_u8.checked_add(expected)? {
                return None;
            }
            intervals.push(starts..marker.opens);
            starts = marker.resumes;
        }
    }
    match scan.ends {
        // A scan the data ends without `EOI` — which the decoder reads to the end as it is, and
        // which the last band is therefore handed exactly as it stands.
        None => intervals.push(starts..data.len()),
        Some(marker) if marker.code == 0xD9 => intervals.push(starts..marker.opens),
        Some(_) => return None,
    }
    Some(intervals)
}

/// The bands a frame is cut into, each [`Band::kept`] at least `lines` tall where the frame
/// allows, or `None` where fewer than two would result.
///
/// A band may begin decoding only at an MCU row where a restart interval begins, which is every
/// `period`-th row: the smallest `p` with `p × mcus_per_row` a multiple of the interval. So a
/// band's height is a multiple of `period`, and the stretch it decodes above what it keeps is
/// one `period` — held to at most a quarter of the band, so that an interval longer than a row
/// cannot make the overlap the larger part of the work.
fn plan(layout: &Layout, lines: u32) -> Option<Vec<Band>> {
    let period = layout
        .restart
        .checked_div(gcd(layout.restart, layout.mcus_per_row))?;
    let wanted = lines.div_ceil(layout.mcu_lines.max(1)).max(1);
    let rows = wanted
        .div_ceil(period)
        .checked_mul(period)?
        .max(period.checked_mul(4)?);
    if rows >= layout.mcu_rows {
        return None;
    }
    let mut bands = Vec::new();
    let mut first = 0u32;
    while first < layout.mcu_rows {
        let last = first.saturating_add(rows).min(layout.mcu_rows);
        bands.push(Band {
            decoded: (
                first.saturating_sub(period),
                last.saturating_add(1).min(layout.mcu_rows),
            ),
            kept: (first, last),
        });
        first = last;
    }
    Some(bands)
}

/// Euclid's greatest common divisor, for the restart period.
fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a.checked_rem(b).unwrap_or(0));
    }
    a.max(1)
}

/// One band as a codestream of its own: the frame's header segments with `Y` set to the band's
/// lines, the scan header, the intervals covering its decoded rows, and `EOI`.
///
/// **The last band is handed the scan's tail exactly as the data carries it**, whatever ends it,
/// because that is what the whole frame's decoder reads there: the decoder pads a scan that
/// runs out of data differently from one ended by a marker, and the bottom rows of a truncated
/// frame are decoded the way the one decoder always decoded them. Every other band ends where a
/// restart marker stood, on a whole interval, and `EOI` is the marker in its place.
fn band_codestream(data: &[u8], layout: &Layout, band: Band) -> Option<Vec<u8>> {
    let (first, last) = band.decoded;
    let per_row = u64::from(layout.mcus_per_row);
    let restart = u64::from(layout.restart);
    let opening = usize::try_from(
        u64::from(first)
            .checked_mul(per_row)?
            .checked_div(restart)?,
    )
    .ok()?;
    let closing = usize::try_from(u64::from(last).checked_mul(per_row)?.div_ceil(restart)).ok()?;
    let finishes = last == layout.mcu_rows;
    let begins = layout.intervals.get(opening)?.start;
    let entropy = if finishes {
        begins..data.len()
    } else {
        begins..layout.intervals.get(closing.checked_sub(1)?)?.end
    };
    let lines = if finishes {
        layout
            .lines
            .checked_sub(first.checked_mul(layout.mcu_lines)?)?
    } else {
        last.checked_sub(first)?.checked_mul(layout.mcu_lines)?
    };
    let lines = u16::try_from(lines).ok()?;

    let header = data.get(layout.header.clone())?;
    let scan = data.get(layout.scan.clone())?;
    let coded = data.get(entropy)?;
    let mut codestream = Vec::with_capacity(
        header
            .len()
            .checked_add(scan.len())?
            .checked_add(coded.len())?
            .checked_add(2)?,
    );
    codestream.extend_from_slice(header);
    codestream
        .get_mut(layout.lines_at..layout.lines_at.checked_add(2)?)?
        .copy_from_slice(&lines.to_be_bytes());
    codestream.extend_from_slice(scan);
    codestream.extend_from_slice(coded);
    if !finishes {
        codestream.extend_from_slice(&[0xFF, 0xD9]);
    }
    Some(codestream)
}

/// Decodes every band into its own lines of one raster, on rayon's pool and the calling thread
/// together, or `None` if any band is refused or delivers other than the lines it owes.
///
/// **The calling thread takes bands too**, rather than handing them all to the pool and
/// waiting: `rayon::in_place_scope` runs its body here, so this thread — which on a page turn is
/// the interpreter's, usually on the faster of this machine's two core classes
/// (`doc/habits/measuring.md` 48) — draws from the same counter the pool's tasks do.
fn decode_bands(
    data: &[u8],
    layout: &Layout,
    bands: &[Band],
    width: u32,
    options: zune_jpeg::zune_core::options::DecoderOptions,
    channels: usize,
) -> Option<Vec<u8>> {
    let line = usize::try_from(width).ok()?.checked_mul(channels)?;
    let mcu_lines = usize::try_from(layout.mcu_lines).ok()?;
    let lines = usize::try_from(layout.lines).ok()?;
    let mut raster = vec![0u8; line.checked_mul(lines)?];
    // Each band's kept lines, as a slice of the raster it alone writes.
    let mut slots: Vec<Mutex<&mut [u8]>> = Vec::with_capacity(bands.len());
    let mut rest: &mut [u8] = &mut raster;
    for band in bands {
        let kept_lines = usize::try_from(band.kept.1)
            .ok()?
            .checked_mul(mcu_lines)?
            .min(lines)
            .checked_sub(usize::try_from(band.kept.0).ok()?.checked_mul(mcu_lines)?)?;
        let (own, after) =
            std::mem::take(&mut rest).split_at_mut_checked(kept_lines.checked_mul(line)?)?;
        slots.push(Mutex::new(own));
        rest = after;
    }

    let next = AtomicUsize::new(0);
    let refused = AtomicBool::new(false);
    let work = || {
        while !refused.load(Ordering::Relaxed) {
            let index = next.fetch_add(1, Ordering::Relaxed);
            let (Some(band), Some(slot)) = (bands.get(index), slots.get(index)) else {
                return;
            };
            let delivered = decode_band(data, layout, *band, line, mcu_lines, options, slot);
            if delivered.is_none() {
                refused.store(true, Ordering::Relaxed);
            }
        }
    };
    rayon::in_place_scope(|scope| {
        for _ in 1..rayon::current_num_threads().min(bands.len()) {
            scope.spawn(|_| work());
        }
        work();
    });
    drop(slots);
    (!refused.into_inner()).then_some(raster)
}

/// Decodes one band and copies the lines it keeps into its slot.
fn decode_band(
    data: &[u8],
    layout: &Layout,
    band: Band,
    line: usize,
    mcu_lines: usize,
    options: zune_jpeg::zune_core::options::DecoderOptions,
    slot: &Mutex<&mut [u8]>,
) -> Option<()> {
    let codestream = band_codestream(data, layout, band)?;
    let pixels = zune_jpeg::JpegDecoder::new_with_options(
        zune_jpeg::zune_core::bytestream::ZCursor::new(codestream.as_slice()),
        options,
    )
    .decode()
    .ok()?;
    let skipped = usize::try_from(band.kept.0.checked_sub(band.decoded.0)?)
        .ok()?
        .checked_mul(mcu_lines)?
        .checked_mul(line)?;
    let mut own = slot.lock().ok()?;
    let kept = pixels.get(skipped..skipped.checked_add(own.len())?)?;
    own.copy_from_slice(kept);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::{Band, Layout, decode_bands, layout as read_layout, plan};
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    use zune_jpeg::zune_core::options::DecoderOptions;

    /// The codestreams the byte-for-byte test decodes, each a 48 × 200 synthetic image encoded
    /// by `libjpeg-turbo`'s `cjpeg` at quality 90 with `-restart 1` (one MCU row an interval)
    /// under every sampling shape the module admits, one with `-restart 6B` (an interval of two
    /// rows, so a band may begin only on every second row), and the first of them with its
    /// closing `EOI` removed. Generated rather than crawled, so each exercises one thing.
    const FIXTURES: [(&str, &[u8]); 7] = [
        ("hv_row", include_bytes!("../../tests/restart/hv_row.jpg")),
        ("h_row", include_bytes!("../../tests/restart/h_row.jpg")),
        ("v_row", include_bytes!("../../tests/restart/v_row.jpg")),
        (
            "none_row",
            include_bytes!("../../tests/restart/none_row.jpg"),
        ),
        (
            "grey_row",
            include_bytes!("../../tests/restart/grey_row.jpg"),
        ),
        (
            "hv_across",
            include_bytes!("../../tests/restart/hv_across.jpg"),
        ),
        (
            "hv_truncated",
            include_bytes!("../../tests/restart/hv_truncated.jpg"),
        ),
    ];

    fn rgba() -> DecoderOptions {
        DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA)
    }

    /// Every fixture, cut into bands of several heights, decodes to the bytes the whole frame's
    /// decoder writes — the module comment's claim, held on each sampling shape it admits, on an
    /// interval longer than a row, and on a scan the data ends without `EOI`.
    #[test]
    fn bands_decode_to_the_whole_frames_bytes() {
        for (name, data) in FIXTURES {
            let whole = zune_jpeg::JpegDecoder::new_with_options(
                zune_jpeg::zune_core::bytestream::ZCursor::new(data),
                rgba(),
            )
            .decode()
            .expect("the whole frame decodes");
            let scan = super::super::first_scan(data).expect("a scan");
            let layout = read_layout(data, &scan).unwrap_or_else(|| panic!("{name} is admitted"));
            let width = super::frame_width(data, &layout).expect("a width");
            let mut cut = 0;
            for lines in [8, 16, 24, 40, 64] {
                let Some(bands) = plan(&layout, lines) else {
                    continue;
                };
                cut += 1;
                let banded = decode_bands(data, &layout, &bands, width, rgba(), 4)
                    .unwrap_or_else(|| panic!("{name} decodes in bands of {lines} lines"));
                assert!(
                    banded == whole,
                    "{name} in bands of {lines} lines moved a byte"
                );
            }
            assert!(cut >= 2, "{name} was cut at two band heights or more");
        }
    }

    fn layout(mcus_per_row: u32, mcu_rows: u32, restart: u32) -> Layout {
        Layout {
            header: 0..0,
            lines_at: 0,
            scan: 0..0,
            intervals: Vec::new(),
            lines: mcu_rows.saturating_mul(16),
            mcu_lines: 16,
            mcus_per_row,
            mcu_rows,
            restart,
        }
    }

    /// An interval of one MCU row: every row may start a band, so each band decodes exactly one
    /// row above and one below what it keeps, and the first and last stop at the frame's edges.
    #[test]
    fn an_interval_of_one_row_overlaps_by_one_row() {
        let bands = plan(&layout(330, 40, 330), 256).expect("two bands or more");
        assert_eq!(
            bands,
            vec![
                Band {
                    decoded: (0, 17),
                    kept: (0, 16)
                },
                Band {
                    decoded: (15, 33),
                    kept: (16, 32)
                },
                Band {
                    decoded: (31, 40),
                    kept: (32, 40)
                },
            ]
        );
    }

    /// An interval that is not a whole number of rows can be cut only where the count of MCUs
    /// before a row is a multiple of it — every third row for 330 MCUs a row and an interval of
    /// 990 — and the overlap above is that period.
    #[test]
    fn an_interval_across_rows_cuts_only_where_one_begins() {
        let bands = plan(&layout(330, 60, 990), 256).expect("two bands or more");
        assert!(bands.iter().all(|band| band.kept.0 % 3 == 0));
        assert!(
            bands
                .iter()
                .skip(1)
                .all(|band| band.kept.0 - band.decoded.0 == 3)
        );
    }

    /// A frame too short for two bands is left whole.
    #[test]
    fn a_frame_of_one_band_is_not_cut() {
        assert_eq!(plan(&layout(330, 16, 330), 256), None);
    }
}
