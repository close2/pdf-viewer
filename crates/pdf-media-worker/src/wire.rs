//! What crosses to a media worker and back, as bytes (ADR 3297).
//!
//! `confined-transport` carries a kind byte and a length and knows nothing of either; this module is
//! what the kind means and what the payload holds. RFC 0015 section 4.3's shape for audio: **one
//! request per run of whole frames, one reply of samples per request** — a sound's PCM is under half
//! a megabyte a second, so the ordinary wire carries it and no shared memory is needed.
//!
//! Every integer is little-endian and every payload begins with a version byte, so that a host and
//! a worker built from different trees refuse each other rather than misread each other. **The
//! host reads a reply as a compromised worker's**: the number of samples a request's bytes hold is
//! the host's own arithmetic, so a reply that disagrees with it is refused whatever it says, and
//! what a worker can still lie about is the sample values — which are sound, and bounded by their
//! width.

use crate::sound::{Decoded, Encoding, SoundFormat, sample_counts};
use crate::wave::{Described, Found, Layout};

/// The eight bytes a media worker's greeting begins with.
///
/// Different from every other worker's, so that a host started on the wrong program refuses it at
/// its first nine bytes.
pub const MAGIC: &[u8; 8] = b"PDFMDW01";

/// The kind of a request: host to worker, a format and a run of whole frames.
pub const FRAME_DECODE: u8 = 1;
/// The kind of a reply: worker to host, the run's samples.
pub const FRAME_SAMPLES: u8 = 2;
/// The kind of a refusal: worker to host, one sentence of UTF-8 as the payload.
pub const FRAME_REFUSED: u8 = 3;

/// The kind of a request: host to worker, one window of a RIFF WAVE file, where it begins, the
/// file's whole length and the format an earlier window read (ADRs 3349, 3397).
pub const FRAME_LAYOUT: u8 = 4;
/// The kind of a reply: worker to host, where the file's samples are and their format.
pub const FRAME_PLACED: u8 = 5;

/// The first byte of every payload.
pub const VERSION: u8 = 2;

/// Longest run of a sound's bytes one request carries.
///
/// A quarter of a mebibyte is sixteen seconds of §13.3's widest recommended format (22,050 samples
/// a second, two channels of sixteen bits), so a sound reaches the sink in a handful of requests,
/// and its decoded copy at the worst case — one-bit samples, thirty-two bytes out for each byte in
/// — is 8 MiB, which `pdf_sandbox::lockdown::Profile::Media`'s ceiling is sized against.
pub const MAX_RUN_BYTES: usize = 256 << 10;

/// A request's bytes before its run: the version, `/R` as a `u32`, `/C` as a `u16`, `/B` and the
/// encoding's byte.
pub const REQUEST_HEADER_BYTES: usize = 1 + 4 + 2 + 1 + 1;

/// Largest request a worker reads.
pub const MAX_REQUEST_BYTES: usize = REQUEST_HEADER_BYTES + MAX_RUN_BYTES;

/// Most samples one request decodes to: a run of one-bit samples, eight to a byte.
pub const MAX_SAMPLES_PER_REQUEST: usize = MAX_RUN_BYTES * 8;

/// A format carried across the wave walk: a byte saying whether there is one, then `/R` as a
/// `u32`, `/C` as a `u16`, `/B`, the encoding's byte and the channel mask as a `u32`.
pub const DESCRIBED_BYTES: usize = 1 + 4 + 2 + 1 + 1 + 4;

/// A layout request's bytes before its window: the version, the file's length and the window's
/// offset as `u64`s, and the format read so far.
pub const LAYOUT_HEADER_BYTES: usize = 1 + 8 + 8 + DESCRIBED_BYTES;

/// Largest layout request a worker reads: its header and one window of [`MAX_RUN_BYTES`].
pub const MAX_LAYOUT_REQUEST_BYTES: usize = LAYOUT_HEADER_BYTES + MAX_RUN_BYTES;

/// A layout reply's bytes: the version, the format as a decode request states it, and the samples'
/// offset and length as `u64`s, and the channel mask as a `u32`.
pub const PLACED_BYTES: usize = REQUEST_HEADER_BYTES + 8 + 8 + 4;

/// The kind of a reply: worker to host, where the wave walk's next window begins and the format
/// read so far (ADR 3397).
pub const FRAME_ONWARD: u8 = 12;

/// An onward reply's bytes: the version, the next window's offset as a `u64`, and the format read
/// so far.
pub const ONWARD_BYTES: usize = 1 + 8 + DESCRIBED_BYTES;

/// Longest refusal sentence either side keeps, in bytes.
pub const MAX_SENTENCE_BYTES: usize = 4096;

/// The kind of a request: host to worker, the first bytes of a box at the top of an MP4 file, where
/// it is, and the file's length (ADR 3381).
pub const FRAME_BOX: u8 = 6;
/// The kind of a reply: worker to host, the box's type, where its payload begins and where it ends.
pub const FRAME_BOXED: u8 = 7;
/// The kind of a request: host to worker, an MP4 file's movie box's payload and the file's length.
pub const FRAME_MOVIE: u8 = 8;
/// The kind of a reply: worker to host, the sound track's access units and what its edit list
/// presents; the worker's AAC decoder is ready for the first run.
pub const FRAME_TRACK: u8 = 9;
/// The kind of a request: host to worker, the next run of the clip's access units.
pub const FRAME_UNITS: u8 = 10;
/// The kind of a reply: worker to host, the run's samples, their rate and channels.
pub const FRAME_PCM: u8 = 11;

/// Longest movie box payload a [`FRAME_MOVIE`] request carries.
///
/// The one request past [`MAX_RUN_BYTES`], because the sample tables cannot be cut: a movie is a
/// few kilobytes a minute of sound, so eight mebibytes is hours of it with room for cover art. The
/// worker holds it once and its access-unit table — sixteen bytes a unit, at most
/// [`crate::mp4::MAX_ACCESS_UNITS`] — and writes the table back at twelve, which
/// `pdf_sandbox::lockdown::Profile::Media`'s ceiling holds (ADR 3381).
pub const MAX_MOVIE_BYTES: usize = 8 << 20;

/// A movie request's bytes before the movie: the version and the file's length as a `u64`.
pub const MOVIE_HEADER_BYTES: usize = 1 + 8;

/// Largest movie request a worker reads.
pub const MAX_MOVIE_REQUEST_BYTES: usize = MOVIE_HEADER_BYTES + MAX_MOVIE_BYTES;

/// A box request's bytes before the header: the version, the file's length and the box's place.
pub const BOX_REQUEST_HEADER_BYTES: usize = 1 + 8 + 8;

/// A box reply's bytes: the version, the type, and where the payload begins and the box ends.
pub const BOXED_BYTES: usize = 1 + 4 + 8 + 8;

/// Largest access unit a run carries: a run's bytes less its version and one unit's size.
pub const MAX_UNIT_BYTES: usize = MAX_RUN_BYTES - 1 - 4 - 4;

/// Most access units one run carries.
///
/// Thirty-two units are under a second and a half of 48 kHz sound, which the scalar decoder answers
/// well inside [`crate::PROCESSOR_BUDGET`] in a debug build (ADR 3381).
pub const MAX_UNITS_PER_RUN: usize = 32;

/// A box request's bytes: the version, `total`, `at`, then `header` — at most
/// [`crate::mp4::TOP_HEADER_BYTES`] of the file from `at`.
#[must_use]
pub fn encode_box_request(total: u64, at: u64, header: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(BOX_REQUEST_HEADER_BYTES.saturating_add(header.len()));
    bytes.push(VERSION);
    bytes.extend_from_slice(&total.to_le_bytes());
    bytes.extend_from_slice(&at.to_le_bytes());
    bytes.extend_from_slice(header);
    bytes
}

/// A box request read back: the file's length, the box's place and its first bytes.
///
/// # Errors
///
/// The sentence for another version, a request cut short, or a header past
/// [`crate::mp4::TOP_HEADER_BYTES`].
pub fn decode_box_request(payload: &[u8]) -> Result<(u64, u64, &[u8]), String> {
    let rest = versioned(payload, "box request")?;
    let Some((total, rest)) = rest.split_first_chunk::<8>() else {
        return Err("a box request cut short before its length".to_owned());
    };
    let Some((at, header)) = rest.split_first_chunk::<8>() else {
        return Err("a box request cut short before its place".to_owned());
    };
    if header.len() > crate::mp4::TOP_HEADER_BYTES {
        return Err(format!(
            "a box header of {} bytes, past the {} one is read from",
            header.len(),
            crate::mp4::TOP_HEADER_BYTES
        ));
    }
    Ok((u64::from_le_bytes(*total), u64::from_le_bytes(*at), header))
}

/// A box reply's bytes.
#[must_use]
pub fn encode_boxed(found: &crate::mp4::TopBox) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(BOXED_BYTES);
    bytes.push(VERSION);
    bytes.extend_from_slice(&found.kind);
    bytes.extend_from_slice(&found.payload.to_le_bytes());
    bytes.extend_from_slice(&found.end.to_le_bytes());
    bytes
}

/// A box reply read back, checked against the box at `at` in a file of `total` bytes the host
/// holds: the payload begins after a header of at least eight bytes, and the box ends inside the
/// file and no earlier than its payload begins — so that a walk by these answers always moves on.
///
/// # Errors
///
/// The sentence the host refuses the reply with.
pub fn decode_boxed(payload: &[u8], at: u64, total: u64) -> Result<crate::mp4::TopBox, String> {
    let rest = versioned(payload, "box reply")?;
    let (Some((kind, rest)), true) = (rest.split_first_chunk::<4>(), rest.len() == BOXED_BYTES - 1)
    else {
        return Err(format!(
            "a box reply of {} bytes, where one is {BOXED_BYTES}",
            payload.len()
        ));
    };
    let (start, end) = rest.split_at(8);
    let start = u64::from_le_bytes(start.try_into().unwrap_or_default());
    let end = u64::from_le_bytes(end.try_into().unwrap_or_default());
    let header_ends = at.checked_add(8);
    if header_ends.is_none_or(|least| start < least) || end < start || end > total {
        return Err(format!(
            "a box reply placing a box at {at} with its payload at {start} and its end at {end}, \
             in a file of {total}"
        ));
    }
    Ok(crate::mp4::TopBox {
        kind: *kind,
        payload: start,
        end,
    })
}

/// A movie request's bytes: the version, `total`, then the movie box's payload.
#[must_use]
pub fn encode_movie_request(total: u64, movie: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(MOVIE_HEADER_BYTES.saturating_add(movie.len()));
    bytes.push(VERSION);
    bytes.extend_from_slice(&total.to_le_bytes());
    bytes.extend_from_slice(movie);
    bytes
}

/// A movie request read back: the file's length and the movie.
///
/// # Errors
///
/// The sentence for another version, a request cut short, or a movie past [`MAX_MOVIE_BYTES`] or
/// longer than the file.
pub fn decode_movie_request(payload: &[u8]) -> Result<(u64, &[u8]), String> {
    let rest = versioned(payload, "movie request")?;
    let Some((total, movie)) = rest.split_first_chunk::<8>() else {
        return Err("a movie request cut short before its length".to_owned());
    };
    let total = u64::from_le_bytes(*total);
    if movie.len() > MAX_MOVIE_BYTES || u64::try_from(movie.len()).map_or(true, |len| len > total) {
        return Err(format!(
            "a movie of {} bytes, past the {MAX_MOVIE_BYTES} one request carries or the {total} \
             the file holds",
            movie.len()
        ));
    }
    Ok((total, movie))
}

/// A sound track as the host needs it: where each access unit is and what the edit list presents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// The media's time scale, the clock of [`Self::presentation`]'s figures.
    pub time_scale: u32,
    /// What the edit list presents, in the media's time scale.
    pub presentation: crate::mp4::Presentation,
    /// Each access unit's offset in the file and its size, in decoding order.
    pub units: Vec<(u64, u32)>,
}

/// A track reply's bytes before its note: the version, the time scale, the delay, the skip and the
/// kept length (all ones for "all of it"), and the note's length.
const TRACK_HEADER_BYTES: usize = 1 + 4 + 8 + 8 + 8 + 2;

/// A track reply's bytes.
#[must_use]
pub fn encode_track(track: &Track) -> Vec<u8> {
    let note = track.presentation.note.as_deref().unwrap_or_default();
    let note = note
        .get(..note.len().min(MAX_SENTENCE_BYTES))
        .unwrap_or_default();
    let mut bytes = Vec::with_capacity(
        TRACK_HEADER_BYTES
            .saturating_add(note.len())
            .saturating_add(4)
            .saturating_add(track.units.len().saturating_mul(12)),
    );
    bytes.push(VERSION);
    bytes.extend_from_slice(&track.time_scale.to_le_bytes());
    bytes.extend_from_slice(&track.presentation.delay.to_le_bytes());
    bytes.extend_from_slice(&track.presentation.skip.to_le_bytes());
    bytes.extend_from_slice(&track.presentation.keep.unwrap_or(u64::MAX).to_le_bytes());
    // `note` was cut to `MAX_SENTENCE_BYTES`, which a `u16` holds.
    bytes.extend_from_slice(&u16::try_from(note.len()).unwrap_or(0).to_le_bytes());
    bytes.extend_from_slice(note.as_bytes());
    bytes.extend_from_slice(
        &u32::try_from(track.units.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    for (offset, size) in &track.units {
        bytes.extend_from_slice(&offset.to_le_bytes());
        bytes.extend_from_slice(&size.to_le_bytes());
    }
    bytes
}

/// A track reply read back, checked against the file of `total` bytes the host holds: every
/// access unit lies inside it and fits a run, there are at most
/// [`crate::mp4::MAX_ACCESS_UNITS`] of them, and the time scale is a clock.
///
/// # Errors
///
/// The sentence the host refuses the reply with.
pub fn decode_track(payload: &[u8], total: u64) -> Result<Track, String> {
    let wrong = || {
        format!(
            "a track reply of {} bytes that its counts do not fit",
            payload.len()
        )
    };
    let mut fields = Little(versioned(payload, "track reply")?);
    let time_scale = fields.u32().ok_or_else(wrong)?;
    let delay = fields.u64().ok_or_else(wrong)?;
    let skip = fields.u64().ok_or_else(wrong)?;
    let keep = fields.u64().ok_or_else(wrong)?;
    let note_len = usize::from(fields.u16().ok_or_else(wrong)?);
    let note = fields.take(note_len).ok_or_else(wrong)?;
    let count = usize::try_from(fields.u32().ok_or_else(wrong)?).unwrap_or(usize::MAX);
    if time_scale == 0 || count > crate::mp4::MAX_ACCESS_UNITS || note_len > MAX_SENTENCE_BYTES {
        return Err(format!(
            "a track reply of time scale {time_scale}, {count} access units and a note of \
             {note_len} bytes"
        ));
    }
    let (entries, remainder) = fields.0.as_chunks::<12>();
    if !remainder.is_empty() || entries.len() != count {
        return Err(wrong());
    }
    let mut units = Vec::with_capacity(count);
    for entry in entries {
        let (offset, size) = entry.split_at(8);
        let offset = u64::from_le_bytes(offset.try_into().unwrap_or_default());
        let size = u32::from_le_bytes(size.try_into().unwrap_or_default());
        let inside = offset
            .checked_add(u64::from(size))
            .is_some_and(|end| end <= total);
        if !inside || usize::try_from(size).map_or(true, |size| size > MAX_UNIT_BYTES) {
            return Err(format!(
                "a track reply placing an access unit of {size} bytes at {offset}, in a file of \
                 {total} and runs of {MAX_UNIT_BYTES}"
            ));
        }
        units.push((offset, size));
    }
    Ok(Track {
        time_scale,
        presentation: crate::mp4::Presentation {
            delay,
            skip,
            keep: (keep != u64::MAX).then_some(keep),
            note: (!note.is_empty()).then(|| sentence(note)),
        },
        units,
    })
}

/// A run request's bytes: the version, the count of `units`, each one's size, then their bytes.
#[must_use]
pub fn encode_units(units: &[&[u8]]) -> Vec<u8> {
    let total: usize = units.iter().map(|unit| unit.len().saturating_add(4)).sum();
    let mut bytes = Vec::with_capacity(total.saturating_add(5));
    bytes.push(VERSION);
    bytes.extend_from_slice(&u32::try_from(units.len()).unwrap_or(u32::MAX).to_le_bytes());
    for unit in units {
        bytes.extend_from_slice(&u32::try_from(unit.len()).unwrap_or(u32::MAX).to_le_bytes());
    }
    for unit in units {
        bytes.extend_from_slice(unit);
    }
    bytes
}

/// A run request read back: its access units.
///
/// # Errors
///
/// The sentence for another version, more than [`MAX_UNITS_PER_RUN`] units, or sizes that do not
/// add up to the bytes that follow them.
pub fn decode_units(payload: &[u8]) -> Result<Vec<&[u8]>, String> {
    let mut fields = Little(versioned(payload, "run request")?);
    let count = fields
        .u32()
        .ok_or_else(|| "a run request cut short before its count".to_owned())?;
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    if count > MAX_UNITS_PER_RUN {
        return Err(format!(
            "a run of {count} access units, past the {MAX_UNITS_PER_RUN} one carries"
        ));
    }
    let mut sizes = Vec::with_capacity(count);
    for _ in 0..count {
        let size = fields
            .u32()
            .ok_or_else(|| "a run request cut short in its sizes".to_owned())?;
        sizes.push(usize::try_from(size).unwrap_or(usize::MAX));
    }
    let mut units = Vec::with_capacity(count);
    for size in sizes {
        units.push(
            fields
                .take(size)
                .ok_or_else(|| "a run whose sizes pass the bytes it carries".to_owned())?,
        );
    }
    if !fields.0.is_empty() {
        return Err("a run that carries bytes past its units' sizes".to_owned());
    }
    Ok(units)
}

/// A run reply's bytes: the version, the rate, the channels, the count of concealed units, the
/// speakers' channel mask, then every sample as four bytes.
#[must_use]
pub fn encode_pcm(pcm: &crate::aac::Pcm) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(pcm.samples.len().saturating_mul(4).saturating_add(11));
    bytes.push(VERSION);
    bytes.extend_from_slice(&pcm.rate.to_le_bytes());
    bytes.extend_from_slice(&pcm.channels.to_le_bytes());
    bytes.extend_from_slice(&pcm.concealed.to_le_bytes());
    bytes.extend_from_slice(&pcm.speakers.to_le_bytes());
    for sample in &pcm.samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

/// A run reply read back, checked against the run of `units` access units it answers: whole
/// frames of 1 to [`crate::aac::MAX_CHANNELS`] channels, at most
/// [`crate::aac::MAX_FRAMES_PER_UNIT`] a unit, no more units concealed than were sent, and a rate
/// wherever there are samples.
///
/// # Errors
///
/// The sentence the host refuses the reply with.
pub fn decode_pcm(payload: &[u8], units: usize) -> Result<crate::aac::Pcm, String> {
    let wrong = || format!("a run reply of {} bytes", payload.len());
    let mut fields = Little(versioned(payload, "run reply")?);
    let rate = fields.u32().ok_or_else(wrong)?;
    let channels = fields.u16().ok_or_else(wrong)?;
    let concealed = fields.u32().ok_or_else(wrong)?;
    let speakers = fields.u32().ok_or_else(wrong)?;
    let (words, remainder) = fields.0.as_chunks::<4>();
    let frames = words.len().checked_div(usize::from(channels));
    let most = units.saturating_mul(crate::aac::MAX_FRAMES_PER_UNIT);
    let whole = frames.is_some_and(|frames| {
        frames.saturating_mul(usize::from(channels)) == words.len() && frames <= most
    });
    let formatted =
        channels <= crate::aac::MAX_CHANNELS && (words.is_empty() || (channels != 0 && rate != 0));
    let counted = usize::try_from(concealed).is_ok_and(|concealed| concealed <= units);
    if !remainder.is_empty() || !(whole || words.is_empty()) || !formatted || !counted {
        return Err(format!(
            "a run reply of {} samples in {channels} channel(s) at {rate} Hz with {concealed} \
             concealed, for a run of {units} access units",
            words.len()
        ));
    }
    Ok(crate::aac::Pcm {
        rate,
        channels,
        concealed,
        speakers,
        samples: words.iter().map(|word| i32::from_le_bytes(*word)).collect(),
    })
}

/// `payload` after its version byte, which must be [`VERSION`].
fn versioned<'a>(payload: &'a [u8], what: &str) -> Result<&'a [u8], String> {
    match payload.split_first() {
        None => Err(format!("an empty {what}")),
        Some((&VERSION, rest)) => Ok(rest),
        Some((version, _)) => Err(format!(
            "a {what} of version {version}, and this side reads version {VERSION}"
        )),
    }
}

/// A cursor over little-endian fields.
struct Little<'a>(&'a [u8]);

impl<'a> Little<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let (taken, rest) = self.0.split_at_checked(count)?;
        self.0 = rest;
        Some(taken)
    }

    fn u16(&mut self) -> Option<u16> {
        self.take(2)?.try_into().ok().map(u16::from_le_bytes)
    }

    fn u32(&mut self) -> Option<u32> {
        self.take(4)?.try_into().ok().map(u32::from_le_bytes)
    }

    fn u64(&mut self) -> Option<u64> {
        self.take(8)?.try_into().ok().map(u64::from_le_bytes)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1)?.first().copied()
    }

    fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.take(N)?.try_into().ok()
    }
}

/// A request's bytes: `format`, then `run`.
#[must_use]
pub fn encode_request(format: &SoundFormat, run: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(REQUEST_HEADER_BYTES.saturating_add(run.len()));
    bytes.push(VERSION);
    bytes.extend_from_slice(&format.rate.to_le_bytes());
    bytes.extend_from_slice(&format.channels.to_le_bytes());
    bytes.push(format.bits);
    bytes.push(format.encoding.code());
    bytes.extend_from_slice(run);
    bytes
}

/// A request read back: its format and its run.
///
/// # Errors
///
/// The sentence a worker refuses with: another version, a header cut short, an encoding byte no
/// encoding has, or a run past [`MAX_RUN_BYTES`].
pub fn decode_request(payload: &[u8]) -> Result<(SoundFormat, &[u8]), String> {
    let Some((header, run)) = payload.split_at_checked(REQUEST_HEADER_BYTES) else {
        return Err(format!(
            "a request of {} bytes is shorter than its {REQUEST_HEADER_BYTES}-byte header",
            payload.len()
        ));
    };
    let [version, r0, r1, r2, r3, c0, c1, bits, encoding] = *header else {
        return Err("a request header of the wrong length".to_owned());
    };
    if version != VERSION {
        return Err(format!(
            "a request of version {version}, and this worker reads version {VERSION}"
        ));
    }
    let Some(encoding) = Encoding::from_code(encoding) else {
        return Err(format!(
            "an encoding byte {encoding} that names no encoding"
        ));
    };
    if run.len() > MAX_RUN_BYTES {
        return Err(format!(
            "a run of {} bytes is past the {MAX_RUN_BYTES} one request carries",
            run.len()
        ));
    }
    let format = SoundFormat {
        rate: u32::from_le_bytes([r0, r1, r2, r3]),
        channels: u16::from_le_bytes([c0, c1]),
        bits,
        encoding,
    };
    Ok((format, run))
}

/// A layout request's bytes: the version, `total`, `at`, the format read so far, then `window` —
/// at most [`MAX_RUN_BYTES`] of the file from `at`, which the caller cuts.
#[must_use]
pub fn encode_layout_request(
    total: u64,
    at: u64,
    described: Option<&Described>,
    window: &[u8],
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(LAYOUT_HEADER_BYTES.saturating_add(window.len()));
    bytes.push(VERSION);
    bytes.extend_from_slice(&total.to_le_bytes());
    bytes.extend_from_slice(&at.to_le_bytes());
    encode_described(&mut bytes, described);
    bytes.extend_from_slice(window);
    bytes
}

/// A layout request read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutRequest<'a> {
    /// The file's length.
    pub total: u64,
    /// Where the window begins in the file.
    pub at: u64,
    /// The format an earlier window read.
    pub described: Option<Described>,
    /// The file's bytes from `at`.
    pub window: &'a [u8],
}

/// A layout request read back.
///
/// # Errors
///
/// The sentence a worker refuses with: another version, a header cut short, a format no decoder
/// reads, a window past [`MAX_RUN_BYTES`] or past the length the file claims.
pub fn decode_layout_request(payload: &[u8]) -> Result<LayoutRequest<'_>, String> {
    let Some((&version, rest)) = payload.split_first() else {
        return Err("an empty layout request".to_owned());
    };
    if version != VERSION {
        return Err(format!(
            "a request of version {version}, and this worker reads version {VERSION}"
        ));
    }
    let Some((total, rest)) = rest.split_first_chunk::<8>() else {
        return Err("a layout request cut short before its length".to_owned());
    };
    let Some((at, rest)) = rest.split_first_chunk::<8>() else {
        return Err("a layout request cut short before its offset".to_owned());
    };
    let (total, at) = (u64::from_le_bytes(*total), u64::from_le_bytes(*at));
    let (described, window) = decode_described(rest)?;
    let end = u64::try_from(window.len())
        .ok()
        .and_then(|len| at.checked_add(len));
    if window.len() > MAX_RUN_BYTES || end.is_none_or(|end| end > total) {
        return Err(format!(
            "a window of {} bytes at {at} is past the {MAX_RUN_BYTES} one request carries or \
             the {total} the file holds",
            window.len()
        ));
    }
    Ok(LayoutRequest {
        total,
        at,
        described,
        window,
    })
}

/// `described`'s bytes, or the marker of none.
fn encode_described(bytes: &mut Vec<u8>, described: Option<&Described>) {
    let Some(described) = described else {
        bytes.extend_from_slice(&[0; DESCRIBED_BYTES]);
        return;
    };
    bytes.push(1);
    // The decode request's header less its version byte.
    bytes.extend_from_slice(
        encode_request(&described.format, &[])
            .get(1..)
            .unwrap_or_default(),
    );
    bytes.extend_from_slice(&described.channel_mask.to_le_bytes());
}

/// A carried format read back, and the bytes after it.
fn decode_described(payload: &[u8]) -> Result<(Option<Described>, &[u8]), String> {
    let Some((fields, rest)) = payload.split_first_chunk::<DESCRIBED_BYTES>() else {
        return Err("a carried format cut short".to_owned());
    };
    let [
        present,
        r0,
        r1,
        r2,
        r3,
        c0,
        c1,
        bits,
        encoding,
        m0,
        m1,
        m2,
        m3,
    ] = *fields;
    match present {
        0 => Ok((None, rest)),
        1 => {
            let header = [VERSION, r0, r1, r2, r3, c0, c1, bits, encoding];
            let (format, _) = decode_request(&header)?;
            if let Some(refusal) = format.refusal() {
                return Err(refusal);
            }
            let channel_mask = u32::from_le_bytes([m0, m1, m2, m3]);
            Ok((
                Some(Described {
                    format,
                    channel_mask,
                }),
                rest,
            ))
        }
        other => Err(format!(
            "a carried format marked {other}, where one is 0 or 1"
        )),
    }
}

/// A window's answer's kind and bytes: [`FRAME_PLACED`] or [`FRAME_ONWARD`].
#[must_use]
pub fn encode_found(found: &Found) -> (u8, Vec<u8>) {
    match found {
        Found::Placed(layout) => (FRAME_PLACED, encode_placed(layout)),
        Found::Onward { at, described } => {
            let mut bytes = Vec::with_capacity(ONWARD_BYTES);
            bytes.push(VERSION);
            bytes.extend_from_slice(&at.to_le_bytes());
            encode_described(&mut bytes, described.as_ref());
            (FRAME_ONWARD, bytes)
        }
    }
}

/// An onward reply read back, checked against the window it answers: the next window begins past
/// `at`, the window's own beginning, and inside the file of `total` bytes.
///
/// # Errors
///
/// The sentence the host refuses the reply with.
pub fn decode_onward(payload: &[u8], at: u64, total: u64) -> Result<Found, String> {
    let Some((&version, rest)) = payload.split_first() else {
        return Err("an empty onward reply".to_owned());
    };
    if version != VERSION || payload.len() != ONWARD_BYTES {
        return Err(format!(
            "an onward reply of {} bytes and version {version}, where one is {ONWARD_BYTES} \
             bytes of version {VERSION}",
            payload.len()
        ));
    }
    let Some((next, rest)) = rest.split_first_chunk::<8>() else {
        return Err("an onward reply cut short before its offset".to_owned());
    };
    let next = u64::from_le_bytes(*next);
    if next <= at || next >= total {
        return Err(format!(
            "an onward reply naming {next}, which is not past the window at {at} and inside the \
             {total} bytes the file holds"
        ));
    }
    let (described, _) = decode_described(rest)?;
    Ok(Found::Onward {
        at: next,
        described,
    })
}

/// A layout reply's bytes.
#[must_use]
pub fn encode_placed(layout: &Layout) -> Vec<u8> {
    let mut bytes = encode_request(&layout.described.format, &[]);
    bytes.extend_from_slice(&layout.offset.to_le_bytes());
    bytes.extend_from_slice(&layout.length.to_le_bytes());
    bytes.extend_from_slice(&layout.described.channel_mask.to_le_bytes());
    bytes
}

/// A layout reply read back, checked against the file of `total` bytes the host holds: the
/// samples lie inside it and their format is one the decoder reads.
///
/// # Errors
///
/// The sentence the host refuses the reply with.
pub fn decode_placed(payload: &[u8], total: u64) -> Result<Layout, String> {
    let Some((format, offsets)) = payload.split_at_checked(REQUEST_HEADER_BYTES) else {
        return Err("a layout reply cut short before its format".to_owned());
    };
    let (format, _) = decode_request(format)?;
    let Some((offset, rest)) = offsets.split_first_chunk::<8>() else {
        return Err("a layout reply cut short before its offset".to_owned());
    };
    let Some((length, mask)) = rest.split_first_chunk::<8>() else {
        return Err("a layout reply cut short before its length".to_owned());
    };
    let Ok(mask) = <[u8; 4]>::try_from(mask) else {
        return Err(format!(
            "a layout reply of {} bytes, where one is {PLACED_BYTES}",
            payload.len()
        ));
    };
    let offset = u64::from_le_bytes(*offset);
    let length = u64::from_le_bytes(*length);
    if offset.checked_add(length).is_none_or(|end| end > total) {
        return Err(format!(
            "a layout reply placing {length} bytes at {offset}, past the {total} the file holds"
        ));
    }
    if let Some(refusal) = format.refusal() {
        return Err(refusal);
    }
    Ok(Layout {
        described: Described {
            format,
            channel_mask: u32::from_le_bytes(mask),
        },
        offset,
        length,
    })
}

/// A reply's bytes: the version, the count of samples after the last whole frame as a `u64`, then
/// every sample as four bytes.
#[must_use]
pub fn encode_samples(decoded: &Decoded) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(decoded.samples.len().saturating_mul(4).saturating_add(9));
    bytes.push(VERSION);
    bytes.extend_from_slice(&decoded.unframed.to_le_bytes());
    for sample in &decoded.samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

/// A reply read back, checked against the request it answers: `format` and the `run_len` bytes the
/// host sent.
///
/// # Errors
///
/// The sentence the host refuses the reply with: another version, or a count of samples other than
/// the one the request's bytes hold.
pub fn decode_samples(
    payload: &[u8],
    format: &SoundFormat,
    run_len: usize,
) -> Result<Decoded, String> {
    let (expected, expected_unframed) = sample_counts(format, run_len);
    let Some((&version, rest)) = payload.split_first() else {
        return Err("an empty reply".to_owned());
    };
    if version != VERSION {
        return Err(format!(
            "a reply of version {version}, and this host reads version {VERSION}"
        ));
    }
    let Some((unframed, samples)) = rest.split_first_chunk::<8>() else {
        return Err("a reply cut short before its count".to_owned());
    };
    let unframed = u64::from_le_bytes(*unframed);
    let (words, remainder) = samples.as_chunks::<4>();
    if !remainder.is_empty() || words.len() as u64 != expected || unframed != expected_unframed {
        return Err(format!(
            "a reply of {} sample bytes and {unframed} unframed, where the request holds {expected} \
             samples and {expected_unframed} unframed",
            samples.len()
        ));
    }
    Ok(Decoded {
        samples: words.iter().map(|word| i32::from_le_bytes(*word)).collect(),
        unframed,
    })
}

/// A refusal's sentence, cut at [`MAX_SENTENCE_BYTES`] on a character boundary.
#[must_use]
pub fn sentence(payload: &[u8]) -> String {
    let text = String::from_utf8_lossy(payload);
    let mut end = text.len().min(MAX_SENTENCE_BYTES);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    text.get(..end).unwrap_or_default().to_owned()
}

/// The kind of a request: host to worker, an MP4 file's movie box's payload and the file's length,
/// for its video track (ADR 3527). The request's bytes are [`encode_movie_request`]'s.
pub const FRAME_VIDEO_MOVIE: u8 = 13;
/// The kind of a reply: worker to host, the video track's picture size, clock and access units;
/// the worker's H.264 session is ready for the first run.
pub const FRAME_VIDEO_TRACK: u8 = 14;
/// The kind of a stream (`confined_transport::stream`): host to worker, one run of the video
/// track's access units; worker to host, each picture the run lets out in a slot of the ring, and a
/// closing that says how the run ended.
pub const FRAME_PICTURES: u8 = 15;
/// The kind of a reply: worker to host, the sentence that the movie's video track is interlaced
/// H.264, which this worker hands to this machine's own player rather than decode.
pub const FRAME_INTERLACED: u8 = 16;

/// Largest access unit of a video run, in bytes.
///
/// An H.264 access unit is bounded by its level's coded picture buffer, which for the 1080-line
/// levels is tens of megabits; four mebibytes holds an intra picture of a 1080p stream at a quality
/// past what a document's clip carries, and a unit past it is refused by its size (ADR 3527).
pub const MAX_VIDEO_UNIT_BYTES: usize = 4 << 20;

/// Largest video run request a worker reads: its version, its last-run flag, a count, and units of
/// at most [`MAX_VIDEO_UNIT_BYTES`] together.
pub const MAX_VIDEO_RUN_BYTES: usize = 1 + 1 + 4 + 4 + MAX_VIDEO_UNIT_BYTES;

/// A video track as the host needs it: the picture size it declares, its clock, and where each
/// access unit is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoTrack {
    /// The sample entry's width.
    pub width: u16,
    /// The sample entry's height.
    pub height: u16,
    /// The media's time scale, the clock of every duration.
    pub time_scale: u32,
    /// Each access unit's offset in the file, size and duration, in decoding order.
    pub units: Vec<(u64, u32, u32)>,
    /// Each access unit's composition offset, in decoding order, or empty where the track states
    /// none (`crate::mp4::VideoTrack::composition_offsets`).
    pub composition_offsets: Vec<i64>,
    /// Whether each access unit is a sync sample, in decoding order
    /// (`crate::mp4::VideoTrack::sync_samples`, every one where the track has no `stss`).
    pub sync: Vec<bool>,
    /// The movie header's time scale, where there is one.
    pub movie_time_scale: Option<u32>,
    /// The track's edit list.
    pub edits: Vec<crate::mp4::Edit>,
}

/// A video track reply's bytes before its units: the version, the width, the height, the time scale
/// and the count.
const VIDEO_TRACK_HEADER_BYTES: usize = 1 + 2 + 2 + 4 + 4;

/// A video track reply's bytes for each access unit: its offset, size and duration, its composition
/// offset, and whether it is a sync sample.
const VIDEO_UNIT_BYTES: usize = 8 + 4 + 4 + 8 + 1;

/// A video track reply's bytes for each edit: its segment duration, media time and rate.
const EDIT_BYTES: usize = 8 + 8 + 4;

/// A video track reply's bytes.
#[must_use]
pub fn encode_video_track(track: &VideoTrack) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        VIDEO_TRACK_HEADER_BYTES
            .saturating_add(track.units.len().saturating_mul(VIDEO_UNIT_BYTES))
            .saturating_add(8)
            .saturating_add(track.edits.len().saturating_mul(EDIT_BYTES)),
    );
    bytes.push(VERSION);
    bytes.extend_from_slice(&track.width.to_le_bytes());
    bytes.extend_from_slice(&track.height.to_le_bytes());
    bytes.extend_from_slice(&track.time_scale.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(track.units.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    for (index, (offset, size, duration)) in track.units.iter().enumerate() {
        bytes.extend_from_slice(&offset.to_le_bytes());
        bytes.extend_from_slice(&size.to_le_bytes());
        bytes.extend_from_slice(&duration.to_le_bytes());
        let composition = track.composition_offsets.get(index).copied().unwrap_or(0);
        bytes.extend_from_slice(&composition.to_le_bytes());
        bytes.push(u8::from(track.sync.get(index).copied().unwrap_or(true)));
    }
    bytes.extend_from_slice(&track.movie_time_scale.unwrap_or(0).to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(track.edits.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    for edit in &track.edits {
        bytes.extend_from_slice(&edit.segment_duration.to_le_bytes());
        bytes.extend_from_slice(&edit.media_time.to_le_bytes());
        bytes.extend_from_slice(&edit.media_rate.to_le_bytes());
    }
    bytes
}

/// A video track reply read back, checked against the file of `total` bytes the host holds: the
/// picture is at most [`crate::video::MAX_PICTURE_PIXELS`], every access unit lies inside the file
/// and is at most [`MAX_VIDEO_UNIT_BYTES`], there are at most [`crate::mp4::MAX_ACCESS_UNITS`], and
/// the time scale is a clock.
///
/// # Errors
///
/// The sentence the host refuses the reply with.
pub fn decode_video_track(payload: &[u8], total: u64) -> Result<VideoTrack, String> {
    let mut fields = Little(versioned(payload, "video track reply")?);
    let cut = || "a video track reply cut short".to_owned();
    let width = fields.u16().ok_or_else(cut)?;
    let height = fields.u16().ok_or_else(cut)?;
    let time_scale = fields.u32().ok_or_else(cut)?;
    let count = usize::try_from(fields.u32().ok_or_else(cut)?).unwrap_or(usize::MAX);
    if width == 0 || height == 0 {
        return Err(format!("a video track of {width} by {height}"));
    }
    if usize::from(width).saturating_mul(usize::from(height)) > crate::video::MAX_PICTURE_PIXELS {
        return Err(format!(
            "a video track of {width} by {height}, past the {} samples a picture may hold",
            crate::video::MAX_PICTURE_PIXELS
        ));
    }
    if time_scale == 0 {
        return Err("a video track whose time scale is 0".to_owned());
    }
    if count > crate::mp4::MAX_ACCESS_UNITS
        || fields.0.len() < count.saturating_mul(VIDEO_UNIT_BYTES)
    {
        return Err(format!(
            "a video track reply of {} bytes that its count of {count} does not fit",
            payload.len()
        ));
    }
    let mut units = Vec::with_capacity(count);
    let mut composition_offsets = Vec::with_capacity(count);
    let mut sync = Vec::with_capacity(count);
    for _ in 0..count {
        let offset = fields.u64().ok_or_else(cut)?;
        let size = fields.u32().ok_or_else(cut)?;
        let duration = fields.u32().ok_or_else(cut)?;
        let fits = usize::try_from(size).is_ok_and(|size| size <= MAX_VIDEO_UNIT_BYTES);
        if !fits
            || offset
                .checked_add(u64::from(size))
                .is_none_or(|end| end > total)
        {
            return Err(format!(
                "an access unit of {size} bytes at {offset}, which a file of {total} bytes or a \
                 run does not hold"
            ));
        }
        units.push((offset, size, duration));
        composition_offsets.push(i64::from_le_bytes(fields.array().ok_or_else(cut)?));
        sync.push(match fields.u8().ok_or_else(cut)? {
            0 => false,
            1 => true,
            other => return Err(format!("a sync flag of {other}")),
        });
    }
    let movie_time_scale = Some(fields.u32().ok_or_else(cut)?).filter(|scale| *scale != 0);
    let edit_count = usize::try_from(fields.u32().ok_or_else(cut)?).unwrap_or(usize::MAX);
    if edit_count > crate::mp4::MAX_EDITS || fields.0.len() != edit_count.saturating_mul(EDIT_BYTES)
    {
        return Err(format!(
            "a video track reply whose {edit_count} edits its {} last bytes do not fit",
            fields.0.len()
        ));
    }
    let mut edits = Vec::with_capacity(edit_count);
    for _ in 0..edit_count {
        edits.push(crate::mp4::Edit {
            segment_duration: fields.u64().ok_or_else(cut)?,
            media_time: i64::from_le_bytes(fields.array().ok_or_else(cut)?),
            media_rate: i32::from_le_bytes(fields.array().ok_or_else(cut)?),
        });
    }
    Ok(VideoTrack {
        width,
        height,
        time_scale,
        units,
        composition_offsets,
        sync,
        movie_time_scale,
        edits,
    })
}

/// A video run request's bytes: the version, whether this is the track's last run — whose closing
/// follows every picture the session still holds — then [`encode_units`]'s count, sizes and units.
#[must_use]
pub fn encode_video_run(last: bool, units: &[&[u8]]) -> Vec<u8> {
    let mut bytes = vec![VERSION, u8::from(last)];
    bytes.extend_from_slice(&encode_units(units));
    bytes
}

/// A video run request read back: whether it is the last, and its access units.
///
/// # Errors
///
/// As [`decode_units`], and a flag other than 0 or 1.
pub fn decode_video_run(payload: &[u8]) -> Result<(bool, Vec<&[u8]>), String> {
    let rest = versioned(payload, "video run request")?;
    let (&flag, units) = rest
        .split_first()
        .ok_or_else(|| "a video run request cut short before its flag".to_owned())?;
    let last = match flag {
        0 => false,
        1 => true,
        other => {
            return Err(format!(
                "a video run request whose last-run flag is {other}"
            ));
        }
    };
    Ok((last, decode_units(units)?))
}

/// A picture item's bytes before its planes: the version, the width, the height, and its
/// [`crate::video::Signal`]'s five codes (`video_full_range_flag` as 0 or 1, then
/// `colour_primaries`, `transfer_characteristics`, `matrix_coefficients` and
/// `chroma_sample_loc_type`, a byte each).
pub const PICTURE_HEADER_BYTES: usize = 1 + 4 + 4 + 5;

/// The bytes a picture of `width` by `height` takes as an item: its header, the luma plane and two
/// chroma planes of half its width and height, rounded up.
#[must_use]
pub fn picture_bytes(width: usize, height: usize) -> usize {
    let chroma = width.div_ceil(2).saturating_mul(height.div_ceil(2));
    PICTURE_HEADER_BYTES
        .saturating_add(width.saturating_mul(height))
        .saturating_add(chroma.saturating_mul(2))
}

/// Writes `picture` as an item into `slot` and returns its length, or `None` where the slot is too
/// small or the planes are not the picture's size.
pub fn encode_picture_into(slot: &mut [u8], picture: &crate::video::Picture) -> Option<usize> {
    let length = picture_bytes(picture.width, picture.height);
    let chroma = picture
        .width
        .div_ceil(2)
        .saturating_mul(picture.height.div_ceil(2));
    if picture.y.len() != picture.width.saturating_mul(picture.height)
        || picture.u.len() != chroma
        || picture.v.len() != chroma
    {
        return None;
    }
    let mut cursor = slot.get_mut(..length)?;
    let width = u32::try_from(picture.width).ok()?.to_le_bytes();
    let height = u32::try_from(picture.height).ok()?.to_le_bytes();
    let signal = picture.signal;
    let codes = [
        u8::from(signal.full_range),
        signal.colour_primaries,
        signal.transfer_characteristics,
        signal.matrix_coefficients,
        signal.chroma_sample_loc_type,
    ];
    for bytes in [
        &[VERSION][..],
        &width,
        &height,
        &codes,
        &picture.y,
        &picture.u,
        &picture.v,
    ] {
        let (head, rest) = std::mem::take(&mut cursor).split_at_mut_checked(bytes.len())?;
        head.copy_from_slice(bytes);
        cursor = rest;
    }
    Some(length)
}

/// A picture item read back, checked: its size is at most `width` by `height` — the track's — and
/// its planes are exactly what its size says.
///
/// # Errors
///
/// The sentence the host refuses the item with.
pub fn decode_picture(
    item: &[u8],
    width: usize,
    height: usize,
) -> Result<crate::video::Picture, String> {
    let mut fields = Little(versioned(item, "picture")?);
    let cut = || "a picture cut short".to_owned();
    let pictured_width = usize::try_from(fields.u32().ok_or_else(cut)?).unwrap_or(usize::MAX);
    let pictured_height = usize::try_from(fields.u32().ok_or_else(cut)?).unwrap_or(usize::MAX);
    if pictured_width == 0
        || pictured_height == 0
        || pictured_width > width
        || pictured_height > height
    {
        return Err(format!(
            "a picture of {pictured_width} by {pictured_height} from a track of {width} by {height}"
        ));
    }
    let codes = fields.take(5).ok_or_else(cut)?;
    let &[
        full_range,
        colour_primaries,
        transfer_characteristics,
        matrix_coefficients,
        siting,
    ] = codes
    else {
        return Err(cut());
    };
    if full_range > 1 {
        return Err(format!("a picture whose full-range flag is {full_range}"));
    }
    let signal = crate::video::Signal {
        full_range: full_range == 1,
        colour_primaries,
        transfer_characteristics,
        matrix_coefficients,
        chroma_sample_loc_type: siting,
    };
    if item.len() != picture_bytes(pictured_width, pictured_height) {
        return Err(format!(
            "a picture of {pictured_width} by {pictured_height} in {} bytes",
            item.len()
        ));
    }
    let luma = pictured_width.saturating_mul(pictured_height);
    let chroma = pictured_width
        .div_ceil(2)
        .saturating_mul(pictured_height.div_ceil(2));
    let y = fields.take(luma).ok_or_else(cut)?.to_vec();
    let u = fields.take(chroma).ok_or_else(cut)?.to_vec();
    let v = fields.take(chroma).ok_or_else(cut)?.to_vec();
    Ok(crate::video::Picture {
        width: pictured_width,
        height: pictured_height,
        y,
        u,
        v,
        signal,
    })
}

/// How a video run ended, the closing of its stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEnd {
    /// Every unit decoded; the pictures it let out were the items.
    Decoded,
    /// The stream is interlaced, which is handed to this machine's own player; the sentence says so.
    Interlaced(String),
    /// The worker refused the run, in this sentence.
    Refused(String),
}

/// A video run's closing bytes: a status byte, then the sentence where there is one, cut to fit a
/// closing.
#[must_use]
pub fn encode_run_end(end: &RunEnd) -> Vec<u8> {
    let (status, text) = match end {
        RunEnd::Decoded => (0, ""),
        RunEnd::Interlaced(text) => (1, text.as_str()),
        RunEnd::Refused(text) => (2, text.as_str()),
    };
    let mut cut = text.len().min(MAX_RUN_END_SENTENCE_BYTES);
    while !text.is_char_boundary(cut) {
        cut = cut.saturating_sub(1);
    }
    let mut bytes = vec![status];
    bytes.extend_from_slice(text.get(..cut).unwrap_or_default().as_bytes());
    bytes
}

/// Longest sentence a run's closing carries: a closing is at most four kibibytes with its status.
pub const MAX_RUN_END_SENTENCE_BYTES: usize = 4000;

/// A video run's closing read back.
///
/// # Errors
///
/// The sentence for an empty closing or a status no build writes.
pub fn decode_run_end(closing: &[u8]) -> Result<RunEnd, String> {
    match closing.split_first() {
        Some((0, [])) => Ok(RunEnd::Decoded),
        Some((1, text)) => Ok(RunEnd::Interlaced(sentence(text))),
        Some((2, text)) => Ok(RunEnd::Refused(sentence(text))),
        _ => Err(format!(
            "a video run's closing of {} bytes that no build writes",
            closing.len()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sixteen-bit stereo format.
    const STEREO: SoundFormat = SoundFormat {
        rate: 22_050,
        channels: 2,
        bits: 16,
        encoding: Encoding::Signed,
    };

    #[test]
    fn a_request_crosses_whole() {
        let bytes = encode_request(&STEREO, &[1, 2, 3, 4]);
        assert_eq!(decode_request(&bytes), Ok((STEREO, &[1u8, 2, 3, 4][..])));
    }

    #[test]
    fn a_request_of_another_version_or_encoding_is_refused() {
        let mut bytes = encode_request(&STEREO, &[]);
        bytes[0] = VERSION + 1;
        assert!(decode_request(&bytes).is_err());
        let mut bytes = encode_request(&STEREO, &[]);
        bytes[8] = 9;
        assert!(decode_request(&bytes).is_err());
        assert!(decode_request(&[VERSION, 0, 0]).is_err());
    }

    #[test]
    fn a_reply_that_disagrees_with_its_request_is_refused() {
        let decoded = Decoded {
            samples: vec![1, 2],
            unframed: 0,
        };
        let bytes = encode_samples(&decoded);
        // Four bytes of sixteen-bit stereo are one frame: two samples.
        assert_eq!(decode_samples(&bytes, &STEREO, 4), Ok(decoded));
        // Eight bytes would be four, so a worker answering two is not believed.
        assert!(decode_samples(&bytes, &STEREO, 8).is_err());
        assert!(decode_samples(&bytes[..bytes.len() - 1], &STEREO, 4).is_err());
    }

    #[test]
    fn a_box_reply_that_does_not_move_the_walk_on_is_refused() {
        let placed = |payload, end| {
            encode_boxed(&crate::mp4::TopBox {
                kind: *b"free",
                payload,
                end,
            })
        };
        assert!(decode_boxed(&placed(108, 200), 100, 1_000).is_ok());
        // A payload inside the header, an end before the payload, and an end past the file.
        assert!(decode_boxed(&placed(104, 200), 100, 1_000).is_err());
        assert!(decode_boxed(&placed(108, 107), 100, 1_000).is_err());
        assert!(decode_boxed(&placed(108, 1_001), 100, 1_000).is_err());
    }

    #[test]
    fn a_track_reply_crosses_whole_and_one_past_the_file_is_refused() {
        let track = Track {
            time_scale: 48_000,
            presentation: crate::mp4::Presentation {
                delay: 0,
                skip: 1_024,
                keep: Some(9_000),
                note: Some("said".to_owned()),
            },
            units: vec![(40, 10), (50, 20)],
        };
        assert_eq!(decode_track(&encode_track(&track), 70), Ok(track.clone()));
        assert!(decode_track(&encode_track(&track), 69).is_err());
        let unbounded = Track {
            units: vec![(0, u32::try_from(MAX_UNIT_BYTES + 1).expect("small"))],
            ..track
        };
        assert!(decode_track(&encode_track(&unbounded), u64::MAX).is_err());
    }

    #[test]
    fn a_run_crosses_whole_and_a_reply_it_cannot_hold_is_refused() {
        let units: [&[u8]; 2] = [&[1, 2, 3], &[]];
        assert_eq!(decode_units(&encode_units(&units)), Ok(units.to_vec()));
        let mut long = encode_units(&units);
        long.push(9);
        assert!(decode_units(&long).is_err());
        let pcm = crate::aac::Pcm {
            rate: 48_000,
            channels: 2,
            concealed: 0,
            speakers: 0x3,
            samples: vec![0; 4_096],
        };
        assert_eq!(decode_pcm(&encode_pcm(&pcm), 1), Ok(pcm.clone()));
        // Two units' worth of frames for one unit, an odd sample in stereo, and more concealed
        // than sent.
        let doubled = crate::aac::Pcm {
            samples: vec![0; 8_192],
            ..pcm.clone()
        };
        assert!(decode_pcm(&encode_pcm(&doubled), 1).is_err());
        let odd = crate::aac::Pcm {
            samples: vec![0; 3],
            ..pcm.clone()
        };
        assert!(decode_pcm(&encode_pcm(&odd), 1).is_err());
        let concealed = crate::aac::Pcm {
            concealed: 2,
            ..pcm
        };
        assert!(decode_pcm(&encode_pcm(&concealed), 1).is_err());
    }
}
