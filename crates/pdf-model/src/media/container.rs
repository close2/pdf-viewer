//! §13.2.4.3's next-level clip as its own container states it: how long it runs, how many pictures
//! it holds, and which named offsets it carries — read from the container's headers, never from
//! its samples.
//!
//! §13.2.4.3 decides two of a media clip section's offset rules by facts only the media data
//! holds:
//!
//! > However, if E specifies an offset beyond the end of the next-level media clip, the end value
//! > shall be used instead, and there is no effect on viability.
//!
//! and, of an offset by marker, "if the marker is not present in the underlying media clip, the
//! existence of the entry shall be ignored, and there is no effect on viability". So the judgement
//! needs the clip's length, its frame count and its marker table. Each is a header field or a list
//! of names, and reading one decodes no sound and no picture, which is what `CLAUDE.md`'s clause 13
//! entry excludes: the entry keeps "§13.2.2's viability computed and reported in the clause's own
//! terms" in scope, and these are the clause's own terms (ADR 2452).
//!
//! # The containers read
//!
//! **RIFF WAVE**, Microsoft's and IBM's *Multimedia Programming Interface and Data Specifications
//! 1.0* (1991), the form [`super::wave()`] writes. Its `fmt ` chunk states the sampling rate and the
//! bytes of one block, its `data` chunk the samples' bytes, and a compressed format's `fact` chunk
//! its length in samples; its `cue ` chunk lists cue points by identifier with each one's sample
//! position in play order, and a `LIST` chunk of form `adtl` names them in `labl` chunks. A cue
//! point no label names has no name a Table 299 `/M` could match.
//!
//! **AVI**, the RIFF form of type `AVI ` the same document defines: its `hdrl` list's `avih`
//! main header states the microseconds of one frame and the frames in the file, and an `OpenDML`
//! file, which continues past its first RIFF form, states the whole file's frames in its `odml`
//! list's `dmlh` header. The form names no offsets.
//!
//! **AIFF and AIFF-C**, Apple's *Audio Interchange File Format* 1.3 (1989) and its compressed
//! extension: an IFF `FORM` of type `AIFF` or `AIFC` whose `COMM` chunk states the sample frames
//! and their rate — an 80-bit IEEE 754 extended number — and whose `MARK` chunk lists markers, each
//! an identifier, a position in sample frames and a name.
//!
//! **The ISO base media file format**, ISO/IEC 14496-12, which MP4 and 3GPP files are, together
//! with the `QuickTime` file format it was taken from. A file is a sequence of boxes, each a 32-bit
//! size and a four-character type, a size of 1 meaning a 64-bit size follows and 0 meaning the box
//! runs to the end of what holds it. The `moov` box's `mvhd` states the presentation's time scale
//! and duration — a duration of all ones is one the file does not know — and each `trak` holds a
//! `tkhd` naming its track, an `hdlr` saying what the track carries (`vide` for pictures), and a
//! sample table whose `stsz` or `stz2` counts its samples, each of a picture track's samples one
//! frame. A fragmented file (`mvex` in `moov`) carries samples outside `moov`, so its frame count is
//! not read and its length is `mehd`'s, where it states one.
//!
//! **Named offsets in that format are `QuickTime`'s chapter lists**: a track that a `tref`'s `chap`
//! box names is a text track, and each of its samples is a 16-bit big-endian length followed by
//! that chapter's title, starting at the sample's time — the sum of the `stts` deltas before it, in
//! the track's `mdhd` time scale. The titles are read from the file at the offsets the sample table
//! gives (`stsc`, `stco` or `co64`, `stsz`); a title is text, and reading it plays nothing. A file
//! that lists no chapter track names no offset, and a marker over it is absent.
//!
//! **SWF**, Adobe's *SWF File Format Specification* version 19 (`swf`): the header's frame rate
//! and frame count give the length and the picture count, and the frame labels of the main
//! timeline are its named offsets.
//!
//! **ASF**, Microsoft's *Advanced Systems Format (ASF) Specification* revision 01.20.03, which
//! Windows Media files are (`asf`): the File Properties Object states the play duration and the
//! Marker Object names jump points.
//!
//! **An MPEG-2 program stream**, ITU-T H.222.0 | ISO/IEC 13818-1 (1995) clause 2.5.3
//! (`program_stream`): its first pack header identifies it, and the clause states neither a
//! length nor a named offset, so a marker over one is absent.
//!
//! What is **not** read, with the reason: MPEG audio (MP3) frames are defined by ISO/IEC 11172-3
//! and 13818-3, and an MPEG-1 system stream by ISO/IEC 11172-1, none of which this tree holds, and
//! a container is never read from a secondary description of it as though it were the
//! specification (`CLAUDE.md` principle 5; ADRs 2488 and 2489).
//!
//! # Bounds
//!
//! The input is a file an untrusted document embedded. Every walk is bounded by [`MAX_ITEMS`]
//! boxes or chunks, nesting is followed only down the fixed path each format gives the field it
//! wants, a marker table is bounded by [`MAX_MARKERS`] and refused past it, and every
//! size is checked against the bytes that hold it before it is used, so no field the file states
//! can make the reader allocate or loop past what the file itself is.

/// Most boxes or chunks one read visits, the nested ones included.
///
/// A container holds a few dozen header boxes; a file that states more is built to make a reader
/// work, and the clip's offsets are then not judged.
pub const MAX_ITEMS: usize = 4096;

/// Most named offsets read from one clip.
pub const MAX_MARKERS: usize = 4096;

mod asf;
mod program_stream;
mod swf;

/// What a clip's container states about its timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    /// How many seconds the clip runs, where its container states it.
    pub seconds: Option<f64>,
    /// How many frames its pictures are, where it holds a picture track whose samples are counted
    /// in its headers; `None` for a medium without pictures, which has no frame an offset names.
    pub frames: Option<u64>,
    /// The offsets it names, each with where it falls.
    pub markers: Vec<Marker>,
}

/// One named offset a container states: a WAVE cue point's label, a `QuickTime` chapter's title, an
/// ASF marker's description, or an SWF frame label.
#[derive(Debug, Clone, PartialEq)]
pub struct Marker {
    /// The name, which a Table 299 `/M` is matched against.
    pub name: String,
    /// Where it falls, in seconds from the clip's start.
    pub seconds: f64,
}

/// The container families this reader knows, by Table 285's `/CT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Wave,
    Avi,
    Aiff,
    BaseMedia,
    Swf,
    Asf,
    ProgramStream,
}

/// The family a content type names, matched as RFC 2045 section 5.1 matches a type: without case,
/// and without the parameters after `;`.
fn family(content_type: &str) -> Option<Family> {
    // RFC 2361 registers `audio/vnd.wave`; the other three are the names files carry.
    const WAVE: [&str; 4] = ["audio/vnd.wave", "audio/wav", "audio/wave", "audio/x-wav"];
    // RFC 4337 registers the three `mp4` types and RFC 3839 and RFC 4393 the 3GPP ones;
    // `QuickTime`'s is Apple's, and the two `x-m4` names are the iTunes extensions files carry.
    const BASE_MEDIA: [&str; 10] = [
        "video/mp4",
        "audio/mp4",
        "application/mp4",
        "video/quicktime",
        "video/3gpp",
        "audio/3gpp",
        "video/3gpp2",
        "audio/3gpp2",
        "video/x-m4v",
        "audio/x-m4a",
    ];
    // `video/vnd.avi` is the registered name and the other three the names files carry.
    const AVI: [&str; 4] = [
        "video/vnd.avi",
        "video/avi",
        "video/msvideo",
        "video/x-msvideo",
    ];
    const AIFF: [&str; 3] = ["audio/aiff", "audio/x-aiff", "audio/x-aifc"];
    // The flash movie type is registered with the IANA as `application/vnd.adobe.flash.movie`; the
    // other name is the one the files carry.
    const SWF: [&str; 2] = [
        "application/x-shockwave-flash",
        "application/vnd.adobe.flash.movie",
    ];
    // The IANA registers `application/vnd.ms-asf`; the three `x-ms` names are Windows Media's.
    const ASF: [&str; 4] = [
        "application/vnd.ms-asf",
        "video/x-ms-asf",
        "video/x-ms-wmv",
        "audio/x-ms-wma",
    ];
    // RFC 2046 registers `video/mpeg`; the other two are the names files carry.
    const PROGRAM_STREAM: [&str; 3] = ["video/mpeg", "video/mpg", "video/x-mpeg"];
    let essence = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim();
    let named = |list: &[&str]| list.iter().any(|name| essence.eq_ignore_ascii_case(name));
    if named(&WAVE) {
        Some(Family::Wave)
    } else if named(&AVI) {
        Some(Family::Avi)
    } else if named(&AIFF) {
        Some(Family::Aiff)
    } else if named(&BASE_MEDIA) {
        Some(Family::BaseMedia)
    } else if named(&SWF) {
        Some(Family::Swf)
    } else if named(&ASF) {
        Some(Family::Asf)
    } else if named(&PROGRAM_STREAM) {
        Some(Family::ProgramStream)
    } else {
        None
    }
}

/// Whether this reader reads the container `content_type` names.
#[must_use]
pub fn is_read(content_type: &str) -> bool {
    family(content_type).is_some()
}

/// Reads the timeline of `bytes`, a clip whose Table 285 `/CT` is `content_type`.
///
/// # Errors
///
/// The sentence a person is told: a type whose container this reader does not read, bytes that
/// are not the container the type names, a header that is cut short or states a size its file
/// does not hold, and a file past one of the bounds.
pub fn timeline(content_type: &str, bytes: &[u8]) -> Result<Timeline, String> {
    match family(content_type) {
        Some(Family::Wave) => wave(bytes),
        Some(Family::Avi) => avi(bytes),
        Some(Family::Aiff) => aiff(bytes),
        Some(Family::BaseMedia) => base_media(bytes),
        Some(Family::Swf) => swf::timeline(bytes),
        Some(Family::Asf) => asf::timeline(bytes),
        Some(Family::ProgramStream) => program_stream::timeline(bytes),
        None => Err(unread(content_type)),
    }
}

/// The sentence for a type whose container this reader has no arm for.
///
/// MPEG audio is named with the reason it stays unread, because it is the type the corpus holds
/// most of: its frame headers are ISO/IEC 11172-3's and ISO/IEC 13818-3's, and this tree holds
/// neither text.
#[must_use]
pub fn unread(content_type: &str) -> String {
    let essence = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim();
    let because = if ["audio/mpeg", "audio/mp3", "audio/x-mpeg", "audio/mpeg3"]
        .iter()
        .any(|name| essence.eq_ignore_ascii_case(name))
    {
        "; MPEG audio frames are defined by ISO/IEC 11172-3 and 13818-3, which this program does \
         not hold"
    } else {
        ""
    };
    format!(
        "this program reads the timeline of no {content_type:?} file, so it cannot say how long \
         the clip runs or which markers it names{because}"
    )
}

/// A run of bytes read from the front, every read checked against what is left.
struct Bytes<'a> {
    rest: &'a [u8],
}

impl<'a> Bytes<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        if count > self.rest.len() {
            return None;
        }
        let (head, tail) = self.rest.split_at(count);
        self.rest = tail;
        Some(head)
    }

    fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.take(N).and_then(|bytes| bytes.try_into().ok())
    }

    fn u8(&mut self) -> Option<u8> {
        self.array::<1>().map(|[byte]| byte)
    }

    fn u16_be(&mut self) -> Option<u16> {
        self.array().map(u16::from_be_bytes)
    }

    fn u32_be(&mut self) -> Option<u32> {
        self.array().map(u32::from_be_bytes)
    }

    fn u64_be(&mut self) -> Option<u64> {
        self.array().map(u64::from_be_bytes)
    }

    fn u16_le(&mut self) -> Option<u16> {
        self.array().map(u16::from_le_bytes)
    }

    fn u32_le(&mut self) -> Option<u32> {
        self.array().map(u32::from_le_bytes)
    }

    fn u64_le(&mut self) -> Option<u64> {
        self.array().map(u64::from_le_bytes)
    }
}

/// The count of items visited, against [`MAX_ITEMS`].
struct Budget {
    visited: usize,
}

impl Budget {
    fn visit(&mut self) -> Result<(), String> {
        self.visited = self.visited.saturating_add(1);
        if self.visited > MAX_ITEMS {
            return Err(format!(
                "its container holds more than {MAX_ITEMS} boxes or chunks, past what this reader \
                 walks"
            ));
        }
        Ok(())
    }
}

/// Seconds `count` units of `per_second` make, where the rate is not zero.
#[expect(
    clippy::cast_precision_loss,
    reason = "a length in seconds is compared with a PDF number, which is a real (§7.3.3); a \
              count past 2^53 units loses only units no offset can state"
)]
fn seconds(count: u64, per_second: u64) -> Option<f64> {
    (per_second != 0).then(|| count as f64 / per_second as f64)
}

/// The sentence for a marker table past [`MAX_MARKERS`], which is refused rather than cut: a name
/// left unread would be judged absent.
fn too_many() -> String {
    format!("its container names more than {MAX_MARKERS} offsets, past what this reader reads")
}

fn truncated(what: &str) -> String {
    format!("its {what} is cut short or states more bytes than the file holds")
}

/// A RIFF WAVE file's timeline.
fn wave(bytes: &[u8]) -> Result<Timeline, String> {
    let body = form(bytes, *b"RIFF", &[*b"WAVE"], Bytes::u32_le)?;
    let mut budget = Budget { visited: 0 };
    let mut format = None;
    let mut data_bytes = None;
    let mut fact = None;
    let mut cues = Vec::new();
    let mut labels = Vec::new();
    for (id, chunk) in riff_chunks(body, &mut budget)? {
        match &id {
            b"fmt " => {
                let mut fields = Bytes { rest: chunk };
                let tag = fields.u16_le();
                let _channels = fields.u16_le();
                let rate = fields.u32_le();
                let _average = fields.u32_le();
                let block = fields.u16_le();
                match (tag, rate, block) {
                    (Some(tag), Some(rate), Some(block)) => format = Some((tag, rate, block)),
                    _ => return Err(truncated("WAVE fmt chunk")),
                }
            }
            b"data" => data_bytes = Some(chunk.len()),
            b"fact" => {
                fact = Some(
                    Bytes { rest: chunk }
                        .u32_le()
                        .ok_or_else(|| truncated("WAVE fact chunk"))?,
                );
            }
            b"cue " => cues = wave_cues(chunk)?,
            b"LIST" if chunk.starts_with(b"adtl") => {
                for (id, sub) in riff_chunks(&chunk[4..], &mut budget)? {
                    if &id == b"labl" {
                        let mut fields = Bytes { rest: sub };
                        let name = fields
                            .u32_le()
                            .ok_or_else(|| truncated("WAVE labl chunk"))?;
                        // The label is a null-terminated string.
                        let text = fields.rest.split(|byte| *byte == 0).next().unwrap_or(&[]);
                        labels.push((name, String::from_utf8_lossy(text).into_owned()));
                    }
                }
            }
            _ => {}
        }
    }
    let Some((tag, rate, block)) = format else {
        return Err("its WAVE file states no fmt chunk, so it has no sampling rate".into());
    };
    // A PCM file's length is its data's blocks; any other format states its length in samples in
    // a `fact` chunk, because its blocks need not each hold one sample of every channel.
    let samples = if tag == 1 {
        data_bytes
            .zip((block != 0).then_some(block))
            .and_then(|(bytes, block)| u64::try_from(bytes.checked_div(usize::from(block))?).ok())
    } else {
        fact.map(u64::from)
    };
    let rate = u64::from(rate);
    let markers = cues
        .iter()
        .filter_map(|(name, position)| {
            let label = labels.iter().find(|(id, _)| id == name)?;
            Some(Marker {
                name: label.1.clone(),
                seconds: seconds(u64::from(*position), rate)?,
            })
        })
        .collect();
    Ok(Timeline {
        seconds: samples.and_then(|samples| seconds(samples, rate)),
        frames: None,
        markers,
    })
}

/// The body of a RIFF or IFF form: the chunks after its form type, which must be one of `kinds`.
///
/// The form's size counts the form type and its chunks; a file longer than that carries bytes
/// that are not the form's (an `OpenDML` file's later forms among them), and a shorter one is read
/// as far as it goes.
fn form<'a>(
    bytes: &'a [u8],
    magic: [u8; 4],
    kinds: &[[u8; 4]],
    size: fn(&mut Bytes<'a>) -> Option<u32>,
) -> Result<&'a [u8], String> {
    let mut file = Bytes { rest: bytes };
    let label = String::from_utf8_lossy(&magic).into_owned();
    if file.array::<4>() != Some(magic) {
        return Err(format!(
            "its bytes are not a {label} file, which its content type says they are"
        ));
    }
    let stated = size(&mut file).ok_or_else(|| truncated(&format!("{label} header")))?;
    match file.array::<4>() {
        Some(kind) if kinds.contains(&kind) => {}
        _ => {
            return Err(format!(
                "its {label} file is not of the form its content type names"
            ));
        }
    }
    let length = usize::try_from(stated)
        .unwrap_or(usize::MAX)
        .saturating_sub(4);
    Ok(&file.rest[..length.min(file.rest.len())])
}

/// An AVI file's timeline: its frames, and their duration.
fn avi(bytes: &[u8]) -> Result<Timeline, String> {
    let body = form(bytes, *b"RIFF", &[*b"AVI "], Bytes::u32_le)?;
    let mut budget = Budget { visited: 0 };
    let mut main = None;
    let mut grand = None;
    for (id, chunk) in riff_chunks(body, &mut budget)? {
        if &id != b"LIST" {
            continue;
        }
        let (list, inside) = chunk.split_at(chunk.len().min(4));
        if list == b"hdrl" {
            for (id, header) in riff_chunks(inside, &mut budget)? {
                if &id == b"avih" {
                    // The main header's first field is the microseconds of one frame, and its
                    // fifth the frames in the file.
                    let mut fields = Bytes { rest: header };
                    let period = fields.u32_le();
                    fields.take(12);
                    main = period.zip(fields.u32_le());
                }
            }
        } else if list == b"odml" {
            for (id, header) in riff_chunks(inside, &mut budget)? {
                if &id == b"dmlh" {
                    grand = Bytes { rest: header }.u32_le();
                }
            }
        }
    }
    let Some((period, frames)) = main else {
        return Err("its AVI file states no avih main header".into());
    };
    let frames = u64::from(grand.unwrap_or(frames));
    Ok(Timeline {
        seconds: seconds(frames.saturating_mul(u64::from(period)), 1_000_000),
        frames: Some(frames),
        markers: Vec::new(),
    })
}

/// An AIFF or AIFF-C file's timeline: its sample frames over their rate, and its markers.
fn aiff(bytes: &[u8]) -> Result<Timeline, String> {
    let body = form(bytes, *b"FORM", &[*b"AIFF", *b"AIFC"], Bytes::u32_be)?;
    let mut budget = Budget { visited: 0 };
    let mut common = None;
    let mut marks = Vec::new();
    for (id, chunk) in iff_chunks(body, &mut budget)? {
        match &id {
            b"COMM" => {
                // Channels (16 bits), sample frames (32), sample size (16), then the rate.
                let mut fields = Bytes { rest: chunk };
                fields.take(2);
                let frames = fields.u32_be();
                fields.take(2);
                let rate = fields.array::<10>().and_then(extended);
                common = Some(
                    frames
                        .zip(rate)
                        .ok_or_else(|| truncated("AIFF COMM chunk"))?,
                );
            }
            b"MARK" => {
                let mut fields = Bytes { rest: chunk };
                let count = fields
                    .u16_be()
                    .ok_or_else(|| truncated("AIFF MARK chunk"))?;
                if usize::from(count) > MAX_MARKERS {
                    return Err(too_many());
                }
                for _ in 0..count {
                    // An identifier (16 bits), a position in sample frames (32), and a Pascal
                    // string padded to an even length with its count byte.
                    let (Some(_id), Some(position), Some(length)) =
                        (fields.u16_be(), fields.u32_be(), fields.u8())
                    else {
                        return Err(truncated("AIFF MARK chunk"));
                    };
                    let name = fields
                        .take(usize::from(length))
                        .ok_or_else(|| truncated("AIFF MARK chunk"))?;
                    if length % 2 == 0 {
                        fields.take(1);
                    }
                    marks.push((String::from_utf8_lossy(name).into_owned(), position));
                }
            }
            _ => {}
        }
    }
    let Some((frames, rate)) = common else {
        return Err("its AIFF file states no COMM chunk, so it has no sampling rate".into());
    };
    let at = |position: u32| (rate > 0.0).then(|| f64::from(position) / rate);
    let markers = marks
        .into_iter()
        .filter_map(|(name, position)| {
            Some(Marker {
                name,
                seconds: at(position)?,
            })
        })
        .collect();
    Ok(Timeline {
        seconds: at(frames),
        frames: None,
        markers,
    })
}

/// An 80-bit IEEE 754 extended number, big-endian — a sign bit, a 15-bit exponent biased by
/// 16383, and a 64-bit significand whose leading bit is explicit — as the nearest `f64`; `None`
/// for a value that is not a finite, non-negative rate.
#[expect(
    clippy::cast_precision_loss,
    reason = "a sampling rate is a whole number far below 2^53; the significand's low bits are \
              rounded as any extended-to-double conversion rounds them"
)]
fn extended(bytes: [u8; 10]) -> Option<f64> {
    let [sign_exponent_high, exponent_low, significand @ ..] = bytes;
    let head = u16::from_be_bytes([sign_exponent_high, exponent_low]);
    let significand = u64::from_be_bytes(significand);
    if head & 0x8000 != 0 || head & 0x7FFF == 0x7FFF {
        return None;
    }
    // The bias and the significand's 63 fraction bits, from an exponent field of at most 0x7FFE.
    let exponent = i32::from(head & 0x7FFF).saturating_sub(16383 + 63);
    let value = significand as f64 * 2f64.powi(exponent);
    value.is_finite().then_some(value)
}

/// A form's chunks, each its identifier and its data.
type Chunks<'a> = Vec<([u8; 4], &'a [u8])>;

/// The chunks of an IFF form's body, big-endian sizes, each padded to an even length.
fn iff_chunks<'a>(body: &'a [u8], budget: &mut Budget) -> Result<Chunks<'a>, String> {
    chunks(body, budget, Bytes::u32_be)
}

/// The chunks of a RIFF list's body, each with its identifier; a chunk's data is padded to an even
/// length.
fn riff_chunks<'a>(body: &'a [u8], budget: &mut Budget) -> Result<Chunks<'a>, String> {
    chunks(body, budget, Bytes::u32_le)
}

/// The chunks of a form's body, sizes read by `size`, each padded to an even length.
fn chunks<'a>(
    body: &'a [u8],
    budget: &mut Budget,
    size: fn(&mut Bytes<'a>) -> Option<u32>,
) -> Result<Chunks<'a>, String> {
    let mut chunks = Vec::new();
    let mut rest = Bytes { rest: body };
    while !rest.rest.is_empty() {
        budget.visit()?;
        let (Some(id), Some(size)) = (rest.array::<4>(), size(&mut rest)) else {
            return Err(truncated("chunk header"));
        };
        let size = usize::try_from(size).map_err(|_| truncated("chunk"))?;
        let data = rest.take(size).ok_or_else(|| truncated("chunk"))?;
        chunks.push((id, data));
        if size % 2 == 1 {
            // The pad byte; a file that ends without it has lost nothing the chunk held.
            let _ = rest.take(1);
        }
    }
    Ok(chunks)
}

/// A `cue ` chunk's points: each one's identifier and its sample position in play order.
fn wave_cues(chunk: &[u8]) -> Result<Vec<(u32, u32)>, String> {
    let mut fields = Bytes { rest: chunk };
    let count = fields.u32_le().ok_or_else(|| truncated("WAVE cue chunk"))?;
    if usize::try_from(count).map_or(true, |count| count > MAX_MARKERS) {
        return Err(too_many());
    }
    let mut cues = Vec::new();
    for _ in 0..count {
        // Each point is six 32-bit fields: its identifier, its position, the chunk it is in, that
        // chunk's start, the block's start and the sample's offset in the block.
        let point = fields.take(24).ok_or_else(|| truncated("WAVE cue chunk"))?;
        let mut point = Bytes { rest: point };
        if let (Some(name), Some(position)) = (point.u32_le(), point.u32_le()) {
            cues.push((name, position));
        }
    }
    Ok(cues)
}

/// One box of the base media file format — `QuickTime`'s atom — by its type and its payload.
#[derive(Clone, Copy)]
struct Atom<'a> {
    kind: [u8; 4],
    payload: &'a [u8],
    /// Where the payload starts in the whole file, for a sample table's absolute offsets.
    at: usize,
}

/// The boxes in `bytes`, which start at `at` in the whole file.
fn boxes<'a>(bytes: &'a [u8], at: usize, budget: &mut Budget) -> Result<Vec<Atom<'a>>, String> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        budget.visit()?;
        let mut header = Bytes {
            rest: &bytes[offset..],
        };
        let (Some(size), Some(kind)) = (header.u32_be(), header.array::<4>()) else {
            return Err(truncated("box header"));
        };
        let (size, header_length): (usize, usize) = match size {
            0 => (bytes.len().saturating_sub(offset), 8),
            1 => {
                let large = header.u64_be().ok_or_else(|| truncated("box header"))?;
                (usize::try_from(large).unwrap_or(usize::MAX), 16)
            }
            size => (usize::try_from(size).unwrap_or(usize::MAX), 8),
        };
        let header_length = if &kind == b"uuid" {
            header_length.saturating_add(16)
        } else {
            header_length
        };
        let end = offset
            .checked_add(size)
            .filter(|end| *end <= bytes.len() && size >= header_length)
            .ok_or_else(|| truncated(&format!("{} box", String::from_utf8_lossy(&kind))))?;
        let start = offset.saturating_add(header_length);
        found.push(Atom {
            kind,
            payload: &bytes[start..end],
            at: at.saturating_add(start),
        });
        offset = end;
    }
    Ok(found)
}

/// The first box of `kind` in `list`.
fn first<'a, 'b>(list: &'b [Atom<'a>], kind: [u8; 4]) -> Option<&'b Atom<'a>> {
    list.iter().find(|item| item.kind == kind)
}

/// The children of a box whose payload is boxes.
fn children<'a>(parent: &Atom<'a>, budget: &mut Budget) -> Result<Vec<Atom<'a>>, String> {
    boxes(parent.payload, parent.at, budget)
}

/// A full box's version, its flags skipped, and the fields after them.
fn full(payload: &[u8]) -> Option<(u8, Bytes<'_>)> {
    let mut fields = Bytes { rest: payload };
    let version = fields.u8()?;
    fields.take(3)?;
    Some((version, fields))
}

/// A `mvhd` or `mdhd` box's time scale and duration, the duration `None` where all its bits are set,
/// the file's way of saying it does not know.
fn scale_and_duration(payload: &[u8]) -> Option<(u32, Option<u64>)> {
    let (version, mut fields) = full(payload)?;
    let (scale, duration) = if version == 1 {
        fields.take(16)?;
        let scale = fields.u32_be()?;
        let duration = fields.u64_be()?;
        (scale, (duration != u64::MAX).then_some(duration))
    } else {
        fields.take(8)?;
        let scale = fields.u32_be()?;
        let duration = fields.u32_be()?;
        (scale, (duration != u32::MAX).then_some(u64::from(duration)))
    };
    Some((scale, duration))
}

/// One `trak` box, read for what the timeline needs.
struct Track<'a> {
    id: Option<u32>,
    handler: Option<[u8; 4]>,
    scale: Option<u32>,
    /// The track identifiers its `tref`'s `chap` box names.
    chapters: Vec<u32>,
    table: Option<Atom<'a>>,
}

fn track<'a>(trak: &Atom<'a>, budget: &mut Budget) -> Result<Track<'a>, String> {
    let inside = children(trak, budget)?;
    let id = first(&inside, *b"tkhd").and_then(|tkhd| {
        let (version, mut fields) = full(tkhd.payload)?;
        // The creation and modification times come first, 64 bits each in version 1.
        fields.take(if version == 1 { 16 } else { 8 })?;
        fields.u32_be()
    });
    let mut chapters = Vec::new();
    if let Some(tref) = first(&inside, *b"tref") {
        for reference in children(tref, budget)? {
            if &reference.kind == b"chap" {
                let mut ids = Bytes {
                    rest: reference.payload,
                };
                while let Some(id) = ids.u32_be() {
                    chapters.push(id);
                }
            }
        }
    }
    let mut handler = None;
    let mut scale = None;
    let mut table = None;
    if let Some(mdia) = first(&inside, *b"mdia") {
        let media = children(mdia, budget)?;
        scale = first(&media, *b"mdhd")
            .and_then(|mdhd| scale_and_duration(mdhd.payload))
            .map(|(scale, _)| scale);
        handler = first(&media, *b"hdlr").and_then(|hdlr| {
            let (_, mut fields) = full(hdlr.payload)?;
            fields.take(4)?;
            fields.array::<4>()
        });
        if let Some(minf) = first(&media, *b"minf") {
            let information = children(minf, budget)?;
            table = first(&information, *b"stbl").copied();
        }
    }
    Ok(Track {
        id,
        handler,
        scale,
        chapters,
        table,
    })
}

/// The sample count a sample table's `stsz` or `stz2` states.
fn sample_count(table: &[Atom<'_>]) -> Option<u32> {
    if let Some(stsz) = first(table, *b"stsz") {
        let (_, mut fields) = full(stsz.payload)?;
        fields.take(4)?;
        return fields.u32_be();
    }
    let stz2 = first(table, *b"stz2")?;
    let (_, mut fields) = full(stz2.payload)?;
    fields.take(4)?;
    fields.u32_be()
}

/// An ISO base media file's timeline.
fn base_media(bytes: &[u8]) -> Result<Timeline, String> {
    let mut budget = Budget { visited: 0 };
    let top = boxes(bytes, 0, &mut budget)?;
    let Some(moov) = first(&top, *b"moov") else {
        return Err("its file holds no moov box, so it states no presentation".into());
    };
    let movie = children(moov, &mut budget)?;
    let header = first(&movie, *b"mvhd")
        .and_then(|mvhd| scale_and_duration(mvhd.payload))
        .ok_or_else(|| truncated("mvhd box"))?;
    let fragmented = first(&movie, *b"mvex");
    let duration = match fragmented {
        // A fragmented file's movie header covers the samples in `moov` only; `mehd` states the
        // whole, in the movie's time scale.
        Some(mvex) => first(&children(mvex, &mut budget)?, *b"mehd").and_then(|mehd| {
            let (version, mut fields) = full(mehd.payload)?;
            if version == 1 {
                fields.u64_be()
            } else {
                fields.u32_be().map(u64::from)
            }
        }),
        None => header.1,
    };
    let mut tracks = Vec::new();
    for trak in movie.iter().filter(|item| &item.kind == b"trak") {
        tracks.push(track(trak, &mut budget)?);
    }
    let frames = match fragmented {
        Some(_) => None,
        None => tracks
            .iter()
            .filter(|track| track.handler == Some(*b"vide"))
            .find_map(|track| {
                let table = track.table.as_ref()?;
                let table = children(table, &mut budget).ok()?;
                sample_count(&table).map(u64::from)
            }),
    };
    let mut markers = Vec::new();
    let named: Vec<u32> = tracks
        .iter()
        .flat_map(|track| track.chapters.clone())
        .collect();
    for id in named {
        let Some(chapter) = tracks.iter().find(|track| track.id == Some(id)) else {
            return Err(format!(
                "its chapter list names track {id}, which the file does not hold"
            ));
        };
        chapters(bytes, chapter, &mut budget, &mut markers)?;
    }
    Ok(Timeline {
        seconds: duration.and_then(|duration| seconds(duration, u64::from(header.0))),
        frames,
        markers,
    })
}

/// A `QuickTime` chapter track's titles, each at its sample's time.
///
/// Each chunk holds the samples its `stsc` run states, one after another from its offset; a table
/// that places fewer samples than it counts leaves titles unread, and is refused rather than read
/// in part, because a title not read would be judged absent.
fn chapters(
    file: &[u8],
    chapter: &Track<'_>,
    budget: &mut Budget,
    markers: &mut Vec<Marker>,
) -> Result<(), String> {
    let table = chapter.table.as_ref().ok_or_else(unread_table)?;
    let table = children(table, budget)?;
    let scale = u64::from(chapter.scale.ok_or_else(unread_table)?);
    let sizes = sample_sizes(&table)?;
    let starts = sample_starts(&table, sizes.len())?;
    let offsets = chunk_offsets(&table)?;
    let runs = chunk_runs(&table)?;

    let count = sizes.len();
    let mut sample = 0usize;
    for (chunk_index, chunk_offset) in offsets.iter().enumerate() {
        let number = u32::try_from(chunk_index.saturating_add(1)).unwrap_or(u32::MAX);
        let per_chunk = runs
            .iter()
            .rev()
            .find(|(first_chunk, _)| *first_chunk <= number)
            .map_or(0, |(_, per_chunk)| *per_chunk);
        let mut at = usize::try_from(*chunk_offset).map_err(|_| unread_table())?;
        for _ in 0..per_chunk {
            if sample == count {
                return Ok(());
            }
            let (Some(size), Some(start)) = (sizes.get(sample), starts.get(sample)) else {
                return Err(unread_table());
            };
            let size = usize::try_from(*size).map_err(|_| unread_table())?;
            let end = at
                .checked_add(size)
                .filter(|end| *end <= file.len())
                .ok_or_else(unread_table)?;
            let name = title(&file[at..end]).ok_or_else(|| truncated("chapter title"))?;
            markers.push(Marker {
                name,
                seconds: seconds(*start, scale).ok_or_else(unread_table)?,
            });
            if markers.len() > MAX_MARKERS {
                return Err(too_many());
            }
            at = end;
            sample = sample.saturating_add(1);
        }
    }
    if sample < count {
        return Err(unread_table());
    }
    Ok(())
}

fn unread_table() -> String {
    truncated("chapter track's sample table")
}

/// A full box's fields after its version and flags, where `table` holds a box of `kind`.
fn table_box<'a>(table: &[Atom<'a>], kind: [u8; 4]) -> Result<Bytes<'a>, String> {
    let found = first(table, kind).ok_or_else(unread_table)?;
    full(found.payload)
        .map(|(_, fields)| fields)
        .ok_or_else(unread_table)
}

/// `stsz`: one size for every sample, or a size each, for at most [`MAX_MARKERS`] samples.
fn sample_sizes(table: &[Atom<'_>]) -> Result<Vec<u32>, String> {
    let mut fields = table_box(table, *b"stsz")?;
    let common = fields.u32_be().ok_or_else(unread_table)?;
    let count =
        usize::try_from(fields.u32_be().ok_or_else(unread_table)?).map_err(|_| too_many())?;
    if count > MAX_MARKERS {
        return Err(too_many());
    }
    let mut sizes = Vec::with_capacity(count);
    for _ in 0..count {
        sizes.push(if common == 0 {
            fields.u32_be().ok_or_else(unread_table)?
        } else {
            common
        });
    }
    Ok(sizes)
}

/// `stts`: runs of samples sharing one duration, each sample starting where the last ended; the
/// starts of the first `count`.
fn sample_starts(table: &[Atom<'_>], count: usize) -> Result<Vec<u64>, String> {
    let mut fields = table_box(table, *b"stts")?;
    let runs = fields.u32_be().ok_or_else(unread_table)?;
    let mut starts = Vec::with_capacity(count);
    let mut time = 0u64;
    'runs: for _ in 0..runs {
        let (Some(run), Some(delta)) = (fields.u32_be(), fields.u32_be()) else {
            return Err(unread_table());
        };
        for _ in 0..run {
            if starts.len() == count {
                break 'runs;
            }
            starts.push(time);
            time = time.saturating_add(u64::from(delta));
        }
    }
    if starts.len() < count {
        return Err(unread_table());
    }
    Ok(starts)
}

/// `stco` or `co64`: where each chunk starts in the file.
fn chunk_offsets(table: &[Atom<'_>]) -> Result<Vec<u64>, String> {
    let wide = first(table, *b"stco").is_none();
    let mut fields = table_box(table, if wide { *b"co64" } else { *b"stco" })?;
    let entries = fields.u32_be().ok_or_else(unread_table)?;
    let mut offsets = Vec::new();
    for _ in 0..entries {
        let offset = if wide {
            fields.u64_be()
        } else {
            fields.u32_be().map(u64::from)
        };
        offsets.push(offset.ok_or_else(unread_table)?);
    }
    Ok(offsets)
}

/// `stsc`: runs of chunks, each from its first chunk (numbered from 1) holding a number of samples.
fn chunk_runs(table: &[Atom<'_>]) -> Result<Vec<(u32, u32)>, String> {
    let mut fields = table_box(table, *b"stsc")?;
    let entries = fields.u32_be().ok_or_else(unread_table)?;
    let mut runs = Vec::new();
    for _ in 0..entries {
        let (Some(first_chunk), Some(per_chunk), Some(_description)) =
            (fields.u32_be(), fields.u32_be(), fields.u32_be())
        else {
            return Err(unread_table());
        };
        runs.push((first_chunk, per_chunk));
    }
    Ok(runs)
}

/// A text sample's title: a 16-bit length, then the text, in UTF-16 where it starts with a byte
/// order mark and in UTF-8 otherwise.
fn title(sample: &[u8]) -> Option<String> {
    let mut fields = Bytes { rest: sample };
    let length = usize::from(fields.u16_be()?);
    let text = fields.take(length)?;
    if let Some(units) = text.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = units
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();
        return Some(String::from_utf16_lossy(&units));
    }
    if let Some(units) = text.strip_prefix(&[0xFF, 0xFE]) {
        let units: Vec<u16> = units
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        return Some(String::from_utf16_lossy(&units));
    }
    Some(String::from_utf8_lossy(text).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn riff_chunk(id: [u8; 4], data: &[u8]) -> Vec<u8> {
        let mut chunk = id.to_vec();
        chunk.extend_from_slice(&u32::try_from(data.len()).expect("small").to_le_bytes());
        chunk.extend_from_slice(data);
        if data.len() % 2 == 1 {
            chunk.push(0);
        }
        chunk
    }

    /// A PCM WAVE file of `frames` 16-bit mono samples at `rate`, with `extra` chunks after them.
    fn wave_file(rate: u32, frames: u32, extra: &[Vec<u8>]) -> Vec<u8> {
        let mut format = Vec::new();
        format.extend_from_slice(&1u16.to_le_bytes());
        format.extend_from_slice(&1u16.to_le_bytes());
        format.extend_from_slice(&rate.to_le_bytes());
        format.extend_from_slice(&rate.saturating_mul(2).to_le_bytes());
        format.extend_from_slice(&2u16.to_le_bytes());
        format.extend_from_slice(&16u16.to_le_bytes());
        let mut body = b"WAVE".to_vec();
        body.extend(riff_chunk(*b"fmt ", &format));
        body.extend(riff_chunk(
            *b"data",
            &vec![0; usize::try_from(frames).expect("small").saturating_mul(2)],
        ));
        for chunk in extra {
            body.extend_from_slice(chunk);
        }
        let mut file = b"RIFF".to_vec();
        file.extend_from_slice(&u32::try_from(body.len()).expect("small").to_le_bytes());
        file.extend(body);
        file
    }

    #[test]
    fn a_pcm_wave_file_runs_for_its_blocks_over_its_rate() {
        let file = wave_file(8000, 16000, &[]);
        let read = timeline("audio/wav", &file).expect("a WAVE file");
        assert_eq!(read.seconds, Some(2.0));
        assert_eq!(read.frames, None, "a sound has no frame an offset names");
        assert!(read.markers.is_empty());
    }

    #[test]
    fn a_labelled_cue_point_is_a_marker_and_an_unlabelled_one_is_not() {
        let mut cue = 2u32.to_le_bytes().to_vec();
        for (name, position) in [(1u32, 4000u32), (2, 8000)] {
            cue.extend_from_slice(&name.to_le_bytes());
            cue.extend_from_slice(&position.to_le_bytes());
            cue.extend_from_slice(b"data");
            cue.extend_from_slice(&[0; 8]);
            cue.extend_from_slice(&position.to_le_bytes());
        }
        let mut label = 1u32.to_le_bytes().to_vec();
        label.extend_from_slice(b"Chapter One\0");
        let mut list = b"adtl".to_vec();
        list.extend(riff_chunk(*b"labl", &label));
        let file = wave_file(
            8000,
            16000,
            &[riff_chunk(*b"cue ", &cue), riff_chunk(*b"LIST", &list)],
        );
        let read = timeline("audio/x-wav; rate=8000", &file).expect("a WAVE file");
        assert_eq!(
            read.markers,
            vec![Marker {
                name: "Chapter One".into(),
                seconds: 0.5
            }]
        );
    }

    #[test]
    fn a_wave_type_over_other_bytes_and_an_unread_type_are_said() {
        assert!(timeline("audio/wav", b"OggS").is_err());
        assert!(timeline("video/x-msvideo", b"RIFF").is_err());
        assert!(!is_read("audio/mpeg"));
        let said = timeline("audio/mpeg", b"ID3").expect_err("MPEG audio is unread");
        assert!(said.contains("11172-3"), "{said}");
        assert!(unread("video/x-flv").contains("video/x-flv"));
        assert!(!unread("video/x-flv").contains("11172"));
    }

    #[test]
    fn the_flash_windows_media_and_program_stream_types_are_read_by_their_own_names() {
        for name in [
            "application/x-shockwave-flash",
            "Application/Vnd.Adobe.Flash.Movie; version=9",
            "video/x-ms-wmv",
            "audio/x-ms-wma",
            "application/vnd.ms-asf",
            "video/x-ms-asf",
            "video/mpeg",
            "video/x-mpeg",
        ] {
            assert!(is_read(name), "{name}");
        }
        // Each type reads its own container and no other: a SWF file under an ASF type is not one.
        assert!(timeline("video/x-ms-wmv", b"FWS\x09").is_err());
    }

    #[test]
    fn a_chunk_stating_more_than_its_file_is_refused() {
        let mut file = wave_file(8000, 8, &[]);
        let at = file.len() - 16 - 4;
        file[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(timeline("audio/wav", &file).is_err());
    }

    #[test]
    fn an_avi_file_runs_for_its_frames_and_an_opendml_header_states_the_whole_file() {
        let mut avih = Vec::new();
        for field in [40_000u32, 0, 0, 0, 250, 0, 1, 0, 320, 240] {
            avih.extend_from_slice(&field.to_le_bytes());
        }
        let mut hdrl = b"hdrl".to_vec();
        hdrl.extend(riff_chunk(*b"avih", &avih));
        let file = |extra: &[u8]| {
            let mut body = b"AVI ".to_vec();
            body.extend(riff_chunk(*b"LIST", &hdrl));
            body.extend_from_slice(extra);
            let mut file = b"RIFF".to_vec();
            file.extend_from_slice(&u32::try_from(body.len()).expect("small").to_le_bytes());
            file.extend(body);
            file
        };
        let read = timeline("video/x-msvideo", &file(&[])).expect("an AVI file");
        assert_eq!((read.seconds, read.frames), (Some(10.0), Some(250)));
        let mut odml = b"odml".to_vec();
        odml.extend(riff_chunk(*b"dmlh", &500u32.to_le_bytes()));
        let read = timeline("video/avi", &file(&riff_chunk(*b"LIST", &odml))).expect("OpenDML");
        assert_eq!((read.seconds, read.frames), (Some(20.0), Some(500)));
    }

    #[test]
    fn an_aiff_file_runs_for_its_sample_frames_and_names_its_markers() {
        // 44 100 as an 80-bit extended number: 2^15 times a significand of 44 100 / 2^15.
        let mut comm = 1u16.to_be_bytes().to_vec();
        comm.extend_from_slice(&88_200u32.to_be_bytes());
        comm.extend_from_slice(&16u16.to_be_bytes());
        comm.extend_from_slice(&(16383u16 + 15).to_be_bytes());
        comm.extend_from_slice(&(44_100u64 << 48).to_be_bytes());
        let mut marks = 2u16.to_be_bytes().to_vec();
        for (id, position, name) in [(1u16, 0u32, "Start"), (2, 44_100, "Chorus")] {
            marks.extend_from_slice(&id.to_be_bytes());
            marks.extend_from_slice(&position.to_be_bytes());
            marks.push(u8::try_from(name.len()).expect("short"));
            marks.extend_from_slice(name.as_bytes());
            if name.len() % 2 == 0 {
                marks.push(0);
            }
        }
        let chunk = |id: &[u8; 4], data: &[u8]| {
            let mut chunk = id.to_vec();
            chunk.extend_from_slice(&u32::try_from(data.len()).expect("small").to_be_bytes());
            chunk.extend_from_slice(data);
            if data.len() % 2 == 1 {
                chunk.push(0);
            }
            chunk
        };
        let mut body = b"AIFF".to_vec();
        body.extend(chunk(b"COMM", &comm));
        body.extend(chunk(b"MARK", &marks));
        let mut file = b"FORM".to_vec();
        file.extend_from_slice(&u32::try_from(body.len()).expect("small").to_be_bytes());
        file.extend(body);
        let read = timeline("audio/aiff", &file).expect("an AIFF file");
        assert_eq!(read.seconds, Some(2.0));
        let names: Vec<(&str, f64)> = read
            .markers
            .iter()
            .map(|marker| (marker.name.as_str(), marker.seconds))
            .collect();
        assert_eq!(names, vec![("Start", 0.0), ("Chorus", 1.0)]);
    }

    fn mp4_box(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut item = u32::try_from(payload.len().saturating_add(8))
            .expect("small")
            .to_be_bytes()
            .to_vec();
        item.extend_from_slice(&kind);
        item.extend_from_slice(payload);
        item
    }

    fn full_box(kind: [u8; 4], fields: &[u8]) -> Vec<u8> {
        let mut payload = vec![0; 4];
        payload.extend_from_slice(fields);
        mp4_box(kind, &payload)
    }

    fn header(scale: u32, duration: u32) -> Vec<u8> {
        let mut fields = vec![0; 8];
        fields.extend_from_slice(&scale.to_be_bytes());
        fields.extend_from_slice(&duration.to_be_bytes());
        fields
    }

    fn table(boxes: &[Vec<u8>]) -> Vec<u8> {
        mp4_box(*b"stbl", &boxes.concat())
    }

    fn trak(id: u32, handler: [u8; 4], scale: u32, extra: &[Vec<u8>], stbl: &[u8]) -> Vec<u8> {
        let mut tkhd = vec![0; 8];
        tkhd.extend_from_slice(&id.to_be_bytes());
        let mut hdlr = vec![0; 4];
        hdlr.extend_from_slice(&handler);
        let mdia = mp4_box(
            *b"mdia",
            &[
                full_box(*b"mdhd", &header(scale, 0)),
                full_box(*b"hdlr", &hdlr),
                mp4_box(*b"minf", stbl),
            ]
            .concat(),
        );
        let mut inside = full_box(*b"tkhd", &tkhd);
        for item in extra {
            inside.extend_from_slice(item);
        }
        inside.extend(mdia);
        mp4_box(*b"trak", &inside)
    }

    fn count(kind: [u8; 4], samples: u32) -> Vec<u8> {
        let mut fields = 0u32.to_be_bytes().to_vec();
        fields.extend_from_slice(&samples.to_be_bytes());
        full_box(kind, &fields)
    }

    #[test]
    fn an_mp4_runs_for_its_movie_header_and_counts_its_picture_samples() {
        let video = trak(1, *b"vide", 600, &[], &table(&[count(*b"stsz", 250)]));
        let moov = mp4_box(
            *b"moov",
            &[full_box(*b"mvhd", &header(600, 6000)), video].concat(),
        );
        let file = [mp4_box(*b"ftyp", b"isom\0\0\0\0"), moov].concat();
        let read = timeline("video/mp4", &file).expect("an MP4 file");
        assert_eq!(read.seconds, Some(10.0));
        assert_eq!(read.frames, Some(250));
        assert!(read.markers.is_empty());
    }

    #[test]
    fn an_unknown_duration_and_a_fragmented_file_state_no_length_or_frames() {
        let video = trak(1, *b"vide", 600, &[], &table(&[count(*b"stsz", 250)]));
        let moov = mp4_box(
            *b"moov",
            &[full_box(*b"mvhd", &header(600, u32::MAX)), video.clone()].concat(),
        );
        let read = timeline("video/mp4", &moov).expect("an MP4 file");
        assert_eq!((read.seconds, read.frames), (None, Some(250)));
        let fragmented = mp4_box(
            *b"moov",
            &[
                full_box(*b"mvhd", &header(600, 0)),
                mp4_box(*b"mvex", &full_box(*b"mehd", &1200u32.to_be_bytes())),
                video,
            ]
            .concat(),
        );
        let read = timeline("video/mp4", &fragmented).expect("an MP4 file");
        assert_eq!((read.seconds, read.frames), (Some(2.0), None));
    }

    #[test]
    fn a_quicktime_chapter_track_names_its_titles_at_their_times() {
        // Two chapter samples, "Intro" at 0 s and "Chapter One" at 3 s, in one chunk at the file's
        // front after the `ftyp` box.
        let ftyp = mp4_box(*b"ftyp", b"qt  \0\0\0\0");
        let mut samples = Vec::new();
        for text in ["Intro", "Chapter One"] {
            samples.extend_from_slice(&u16::try_from(text.len()).expect("short").to_be_bytes());
            samples.extend_from_slice(text.as_bytes());
        }
        let mdat = mp4_box(*b"mdat", &samples);
        let chunk_at = u32::try_from(ftyp.len() + 8).expect("small");
        let mut deltas = 1u32.to_be_bytes().to_vec();
        deltas.extend_from_slice(&2u32.to_be_bytes());
        deltas.extend_from_slice(&3000u32.to_be_bytes());
        let mut sizes = 0u32.to_be_bytes().to_vec();
        sizes.extend_from_slice(&2u32.to_be_bytes());
        sizes.extend_from_slice(&7u32.to_be_bytes());
        sizes.extend_from_slice(&13u32.to_be_bytes());
        let mut runs = 1u32.to_be_bytes().to_vec();
        for field in [1u32, 2, 1] {
            runs.extend_from_slice(&field.to_be_bytes());
        }
        let mut offsets = 1u32.to_be_bytes().to_vec();
        offsets.extend_from_slice(&chunk_at.to_be_bytes());
        let text = trak(
            2,
            *b"text",
            1000,
            &[],
            &table(&[
                full_box(*b"stts", &deltas),
                full_box(*b"stsz", &sizes),
                full_box(*b"stsc", &runs),
                full_box(*b"stco", &offsets),
            ]),
        );
        let chap = mp4_box(*b"chap", &2u32.to_be_bytes());
        let video = trak(
            1,
            *b"vide",
            600,
            &[mp4_box(*b"tref", &chap)],
            &table(&[count(*b"stsz", 10)]),
        );
        let moov = mp4_box(
            *b"moov",
            &[full_box(*b"mvhd", &header(600, 6000)), video, text].concat(),
        );
        let file = [ftyp, mdat, moov].concat();
        let read = timeline("video/quicktime", &file).expect("a QuickTime file");
        assert_eq!(
            read.markers,
            vec![
                Marker {
                    name: "Intro".into(),
                    seconds: 0.0
                },
                Marker {
                    name: "Chapter One".into(),
                    seconds: 3.0
                },
            ]
        );
    }

    #[test]
    fn a_box_past_its_file_and_a_file_of_too_many_boxes_are_refused() {
        let mut moov = mp4_box(*b"moov", &[0; 8]);
        moov[3] = 200;
        assert!(timeline("video/mp4", &moov).is_err());
        let many: Vec<u8> = (0..=MAX_ITEMS)
            .flat_map(|_| mp4_box(*b"free", &[]))
            .collect();
        let refused = timeline("video/mp4", &many).expect_err("past the bound");
        assert!(refused.contains("4096"), "{refused}");
    }
}
