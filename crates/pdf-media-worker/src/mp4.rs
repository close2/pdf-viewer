//! An MP4 file's sound track read in the worker: where each of its access units is, how long each
//! lasts, and the configuration its decoder starts from (ADR 3369).
//!
//! A rendition's media clip carries a file (Table 285's `/D`), and compressed sound in one is
//! overwhelmingly AAC in an MP4 file (RFC 0015 section 5). The file is the document's bytes like
//! every other byte of it, so it is read here, behind `Profile::Media`, and the host learns only
//! the table this module returns, which it checks against the file it holds. This module decodes
//! nothing: it finds the compressed access units, the decoder's configuration and what the track's
//! edit list presents, and [`crate::aac`] decodes (ADRs 3381–3383). A file too long for one request
//! is walked by [`top_level_box`] one box at a time and its movie read by [`movie_sound_track`].
//!
//! # The layout read
//!
//! **ISO/IEC 14496-12, the ISO base media file format**, which MP4 (ISO/IEC 14496-14), M4A and
//! 3GPP files are. Its fourth edition, ISO/IEC 14496-12:2012, is held at
//! `doc/ISO-IEC-14496-12-2012.pdf` (A988), and every fact below was read against it and is cited
//! by its clause there (ADR 3460); what follows restates those facts rather than quoting them.
//! ISO/IEC 14496-14 is not held:
//!
//! - **4.2, the box**: a 32-bit big-endian size, then a four-character type; a size of 1 means a
//!   64-bit size follows the type, a size of 0 means the box is the last in the file and runs to
//!   its end — inside another box, a choice this reader makes, to the end of what holds it — and
//!   a type of `uuid` is followed by a sixteen-byte extended type. A *full box* begins its payload
//!   with an eight-bit version and 24 bits of flags.
//! - **8.2.1, `moov`**, the movie box, holds the metadata; there is one. **8.8.1, `mvex`** in it says
//!   the file is fragmented, and a fragmented file's samples are described outside `moov`.
//! - **8.2.2, `mvhd`**, the movie header: its time scale, units per second of the movie's own
//!   clock, after 32-bit creation and modification times in version 0 and 64-bit in version 1.
//! - **8.6.5, `edts`**, and **8.6.6, `elst`** in it, a track's edit list: entries of a segment
//!   duration in the movie's time scale, a media time in the media's (−1 for an empty edit) — 32
//!   bits each in version 0, 64 in version 1 — and a 16.16 rate; [`presentation`] says how they are
//!   read.
//! - **8.3.1, `trak`**, one per track, holds **8.3.2, `tkhd`**, whose flag `0x000001` says the track
//!   is enabled (a disabled track is treated as though it were not present), and **8.4.1, `mdia`**,
//!   which holds **8.4.2, `mdhd`** — the media's time scale and duration, 32-bit in version 0 and
//!   64-bit in version 1 — **8.4.3, `hdlr`**, whose handler type `soun` names a sound track, and
//!   **8.4.4, `minf`**.
//! - **8.7.1, `dinf`**, and **8.7.2, `dref`** in it: a sample entry's data reference index names
//!   an entry there, and an entry whose flag `0x000001` is set says the media data is in this file.
//! - **8.5.1, `stbl`**, the sample table: **8.5.2, `stsd`**, the sample descriptions, an audio
//!   one an `AudioSampleEntry` of six reserved bytes, a 16-bit data reference index, eight reserved
//!   bytes, a 16-bit channel count, a 16-bit sample size, four reserved bytes and a 32-bit 16.16
//!   sampling rate, followed by boxes; **8.6.1.2, `stts`**, runs of (sample count, sample delta)
//!   in the media's time scale; **8.7.4, `stsc`**, runs of (first chunk, samples per chunk, sample
//!   description index), each run lasting until the next run's first chunk and the last to the
//!   last chunk; **8.7.3, `stsz`**, one size for every sample or a 32-bit size each, and **`stz2`**,
//!   sizes packed in fields of 4, 8 or 16 bits; **8.7.5, `stco`** and **`co64`**, each chunk's
//!   offset from the start of the file, 32 or 64 bits. A chunk's samples are contiguous and in
//!   order, so a sample's offset is its chunk's offset plus the sizes of the samples before it in
//!   that chunk.
//!
//! **ISO/IEC 14496-14, 5.6, `esds`**: an MPEG-4 audio sample entry (`mp4a`) carries a full box
//! holding one `ES_Descriptor` of **ISO/IEC 14496-1, 7.2.6.5**: tag 3, an expandable length (up to
//! four bytes, seven bits each, the high bit saying another follows), a 16-bit ES identifier, a
//! byte of flags — stream dependence, URL and OCR stream, whose fields follow when set — and then a
//! `DecoderConfigDescriptor` (7.2.6.6, tag 4): an object type indication byte, a byte whose upper
//! six bits are the stream type (5, audio), 24 bits of buffer size and two 32-bit bit rates, and
//! then a `DecoderSpecificInfo` (7.2.6.7, tag 5), whose bytes for an object type of `0x40` are
//! ISO/IEC 14496-3's `AudioSpecificConfig`. 14496-1 and 14496-3 are not held either.
//!
//! # What this reader declines, each with a sentence
//!
//! A file with no `moov`, or with two; a fragmented file; a file with no enabled sound track; a
//! sound track whose first sample entry is not `mp4a` (`enca`, an encrypted one, among them); an
//! `AudioSampleEntry` of a version other than 0 — `QuickTime`'s versions 1 and 2, which lengthen the
//! entry, and the later editions' `AudioSampleEntryV1`, where the fourth edition holds those bytes
//! reserved as zero (8.5.2.2); samples that name a description other than the
//! first; media data in another file; a sample table that disagrees with itself (a chunk run that
//! does not begin at chunk 1 or does not increase, durations for more or fewer samples than the
//! sizes count, chunks that hold more or fewer samples than there are); an access unit that lies
//! past the file's end; and more than [`MAX_ACCESS_UNITS`] access units.
//!
//! A video track ([`movie_video_track`]) is read the same way under the `vide` handler, and declines
//! a first sample entry that is not `avc1` or `avc3` and an `avcC` record that is not version 1,
//! states NAL lengths of three bytes, or holds less than it counts.
//!
//! # The bounds
//!
//! The reader walks a slice, so a box is never larger than what holds it, and every table's entry
//! count is checked against the bytes its box holds before it is used — a count is the document's
//! claim and the slice is the fact. The only allocation that grows with the file is the access-unit
//! table, which is [`MAX_ACCESS_UNITS`] entries at most and only ever as many as `stsz` has room to
//! state (or, for one size for every sample, as many as the file has room to hold). The boxes
//! walked are counted against [`MAX_BOXES`].

/// The most access units one track may hold: past it a file is declined.
///
/// An AAC access unit is 1 024 samples (960 in the short-frame form), so this is about six hours
/// of 48 kHz sound and sixteen bytes an entry — 16 MiB at most for the table.
pub const MAX_ACCESS_UNITS: usize = 1 << 20;

/// The most boxes one file's walk visits: past it the file is declined.
pub const MAX_BOXES: usize = 1 << 16;

/// The most bytes of decoder configuration carried back.
///
/// ISO/IEC 14496-3's `AudioSpecificConfig` is a few bytes, and a program configuration element in
/// it a few hundred at most; a descriptor longer than this is declined rather than copied.
pub const MAX_DECODER_CONFIG_BYTES: usize = 4096;

/// ISO/IEC 14496-1's object type indication for ISO/IEC 14496-3 audio, the one AAC is coded as in
/// an MP4 file.
pub const OBJECT_TYPE_MPEG4_AUDIO: u8 = 0x40;

/// One access unit: a sample, in 14496-12's word, which for AAC is one raw data block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessUnit {
    /// Where its bytes begin, from the start of the file.
    pub offset: u64,
    /// How many bytes it is.
    pub size: u32,
    /// How long it lasts, in [`SoundTrack::time_scale`] units.
    pub duration: u32,
}

/// A file's sound track: what its decoder starts from, and where every access unit is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundTrack {
    /// The `DecoderConfigDescriptor`'s object type indication ([`OBJECT_TYPE_MPEG4_AUDIO`] for AAC).
    pub object_type: u8,
    /// The `DecoderSpecificInfo`'s bytes — for AAC, the `AudioSpecificConfig` — or none where the
    /// descriptor carries none, which the decoder judges.
    pub decoder_config: Vec<u8>,
    /// The sample entry's channel count.
    pub channels: u16,
    /// The sample entry's sampling rate, its whole part (the field is 16.16 fixed point).
    pub sample_rate: u32,
    /// The `mdhd` time scale: units per second of every [`AccessUnit::duration`].
    pub time_scale: u32,
    /// The access units, in decoding order.
    pub access_units: Vec<AccessUnit>,
    /// The `mvhd` time scale, the clock of every [`Edit::segment_duration`], where the movie has
    /// a header.
    pub movie_time_scale: Option<u32>,
    /// The track's edit list (`edts`, `elst`), empty where it has none; [`presentation`] reads it.
    pub edits: Vec<Edit>,
}

/// Reads the first enabled sound track of the MP4 file `file`.
///
/// # Errors
///
/// The sentence the worker refuses with: a file that is not one this reader reads, a track it
/// declines, or a table that disagrees with itself or with the file (the module's list).
pub fn sound_track(file: &[u8]) -> Result<SoundTrack, String> {
    let mut budget = Budget::default();
    let mut movie = None;
    for found in Boxes::new(file, &mut budget) {
        let found = found?;
        if &found.kind == b"moov" {
            if movie.is_some() {
                return Err("the file holds two movie boxes (moov), and has one".to_owned());
            }
            movie = Some(found.payload);
        }
    }
    let movie = movie.ok_or_else(|| "the file holds no movie box (moov)".to_owned())?;
    let file_len = u64::try_from(file.len()).unwrap_or(u64::MAX);
    read_movie(movie, file_len, &mut budget)
}

/// Reads the first enabled sound track of the movie box whose payload is `movie`, in a file of
/// `file_len` bytes — the half of [`sound_track`] the worker runs when the host has sent it only the
/// movie (ADR 3381), every access unit checked against the file's length.
///
/// # Errors
///
/// As [`sound_track`].
pub fn movie_sound_track(movie: &[u8], file_len: u64) -> Result<SoundTrack, String> {
    read_movie(movie, file_len, &mut Budget::default())
}

/// [`movie_sound_track`] under a walk's budget already begun.
fn read_movie(movie: &[u8], file_len: u64, budget: &mut Budget) -> Result<SoundTrack, String> {
    let mut tracks = Vec::new();
    let mut movie_time_scale = None;
    for found in Boxes::new(movie, budget) {
        let found = found?;
        match &found.kind {
            b"mvex" => {
                return Err(
                    "the file is fragmented (mvex), and this reader reads only a movie's own \
                     sample tables"
                        .to_owned(),
                );
            }
            b"mvhd" => movie_time_scale = Some(movie_header_time_scale(found.payload)?),
            b"trak" => tracks.push(found.payload),
            _ => {}
        }
    }
    for track in tracks {
        if let Some(media) = enabled_sound_media(track, budget)? {
            let mut read = read_track(media, file_len, budget)?;
            read.movie_time_scale = movie_time_scale;
            if let Some(edits) = child(track, *b"edts", budget)?
                && let Some(list) = child(edits, *b"elst", budget)?
            {
                read.edits = edit_list(list)?;
            }
            return Ok(read);
        }
    }
    Err("the file holds no enabled sound track".to_owned())
}

/// A file's video track: the H.264 parameter sets its decoder starts from, how its samples frame
/// their NAL units, and where every access unit is (ISO/IEC 14496-15, the AVC file format).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoTrack {
    /// The sample entry's width in pixels: the largest picture the track declares.
    pub width: u16,
    /// The sample entry's height in pixels.
    pub height: u16,
    /// The bytes of the length before each NAL unit in a sample: 1, 2 or 4.
    pub length_size: u8,
    /// The `avcC` record's sequence and picture parameter sets, each a NAL unit without its length.
    pub parameter_sets: Vec<Vec<u8>>,
    /// The `mdhd` time scale: units per second of every [`AccessUnit::duration`].
    pub time_scale: u32,
    /// The access units, in decoding order.
    pub access_units: Vec<AccessUnit>,
}

/// The most bytes of parameter sets one `avcC` record may carry back.
///
/// A sequence parameter set with every scaling list and a VUI is a few hundred bytes; 31 of them and
/// 255 picture parameter sets, each as long, would still be under this, and a record past it is
/// declined rather than copied.
pub const MAX_PARAMETER_SET_BYTES: usize = 64 << 10;

/// Reads the first enabled video track of the movie box whose payload is `movie`, in a file of
/// `file_len` bytes, which must be H.264 in an `avc1` or `avc3` sample entry (ISO/IEC 14496-15).
///
/// # Errors
///
/// The sentence the worker refuses with: no enabled video track, a sample entry of another coding,
/// an `avcC` record that is not version 1 or does not hold what it counts, or a sample table that
/// disagrees with itself or with the file (the module's list).
pub fn movie_video_track(movie: &[u8], file_len: u64) -> Result<VideoTrack, String> {
    let budget = &mut Budget::default();
    let mut tracks = Vec::new();
    for found in Boxes::new(movie, budget) {
        let found = found?;
        match &found.kind {
            b"mvex" => {
                return Err(
                    "the file is fragmented (mvex), and this reader reads only a movie's own \
                     sample tables"
                        .to_owned(),
                );
            }
            b"trak" => tracks.push(found.payload),
            _ => {}
        }
    }
    for track in tracks {
        if let Some(media) = enabled_media(track, *b"vide", budget)? {
            let tables = Tables::read(media, budget)?;
            let entry = video_description(tables.descriptions, budget)?;
            tables.data_is_here(entry.data_reference, budget)?;
            let access_units = tables.access_units(file_len, budget)?;
            return Ok(VideoTrack {
                width: entry.width,
                height: entry.height,
                length_size: entry.length_size,
                parameter_sets: entry.parameter_sets,
                time_scale: tables.time_scale,
                access_units,
            });
        }
    }
    Err("the file holds no enabled video track".to_owned())
}

/// What a video track's first sample entry says.
struct VideoDescription {
    data_reference: u16,
    width: u16,
    height: u16,
    length_size: u8,
    parameter_sets: Vec<Vec<u8>>,
}

/// The bytes of `SampleEntry`'s and `VisualSampleEntry`'s fields before its boxes (14496-12,
/// 8.5.2.2): six reserved and the data reference index, sixteen of pre-defined and reserved, the
/// width and height, two resolutions, a reserved word, the frame count, a 32-byte compressor name,
/// the depth and a pre-defined word.
const VISUAL_SAMPLE_ENTRY_FIELDS: usize = 78;

/// Reads `stsd`'s first entry, which must be an `avc1` or `avc3` visual sample entry holding an
/// `avcC` record.
fn video_description(descriptions: &[u8], budget: &mut Budget) -> Result<VideoDescription, String> {
    let (_, _, mut fields) = full(descriptions).ok_or_else(|| truncated("stsd"))?;
    let count = fields.u32().ok_or_else(|| truncated("stsd"))?;
    if count == 0 {
        return Err("a sample description box (stsd) with no entry".to_owned());
    }
    let entry = Boxes::new(fields.rest, budget)
        .next()
        .ok_or_else(|| truncated("stsd"))??;
    if &entry.kind != b"avc1" && &entry.kind != b"avc3" {
        return Err(format!(
            "a video track coded as {}, where this reader reads H.264 (avc1, avc3)",
            four(entry.kind)
        ));
    }
    let mut fields = Fields {
        rest: entry.payload,
    };
    fields.skip(6).ok_or_else(|| truncated("avc1"))?;
    let data_reference = fields.u16().ok_or_else(|| truncated("avc1"))?;
    fields.skip(16).ok_or_else(|| truncated("avc1"))?;
    let width = fields.u16().ok_or_else(|| truncated("avc1"))?;
    let height = fields.u16().ok_or_else(|| truncated("avc1"))?;
    let boxes = entry
        .payload
        .get(VISUAL_SAMPLE_ENTRY_FIELDS..)
        .ok_or_else(|| truncated("avc1"))?;
    let record = child(boxes, *b"avcC", budget)?.ok_or_else(|| missing("avcC", "avc1"))?;
    let (length_size, parameter_sets) = avc_configuration(record)?;
    Ok(VideoDescription {
        data_reference,
        width,
        height,
        length_size,
        parameter_sets,
    })
}

/// Reads `avcC`'s `AVCDecoderConfigurationRecord` (ISO/IEC 14496-15, 5.3.3.1): a version byte of 1,
/// three bytes of profile, compatibility and level, six reserved bits and the NAL length's size
/// less one, then three reserved bits and a five-bit count of sequence parameter sets, each a
/// 16-bit length and its NAL unit, then a byte's count of picture parameter sets, each as long. The
/// high profiles' fields that may follow are not needed: the decoder reads chroma format and bit
/// depth from the sequence parameter set itself. ISO/IEC 14496-15 is not held; the layout is cited.
fn avc_configuration(record: &[u8]) -> Result<(u8, Vec<Vec<u8>>), String> {
    let mut fields = Fields { rest: record };
    let version = fields.u8().ok_or_else(|| truncated("avcC"))?;
    if version != 1 {
        return Err(format!(
            "an AVC configuration record (avcC) of version {version}, where this reader reads 1"
        ));
    }
    fields.skip(3).ok_or_else(|| truncated("avcC"))?;
    let length_size = (fields.u8().ok_or_else(|| truncated("avcC"))? & 0x03).saturating_add(1);
    if length_size == 3 {
        return Err(
            "an AVC configuration record (avcC) whose NAL lengths are 3 bytes, which \
                    ISO/IEC 14496-15 does not allow"
                .to_owned(),
        );
    }
    let mut parameter_sets = Vec::new();
    let mut held = 0usize;
    let sequence_sets = fields.u8().ok_or_else(|| truncated("avcC"))? & 0x1F;
    for index in 0..2 {
        let count = if index == 0 {
            sequence_sets
        } else {
            fields.u8().ok_or_else(|| truncated("avcC"))?
        };
        for _ in 0..count {
            let length = usize::from(fields.u16().ok_or_else(|| truncated("avcC"))?);
            let unit = fields.take(length).ok_or_else(|| truncated("avcC"))?;
            held = held.saturating_add(length);
            if held > MAX_PARAMETER_SET_BYTES {
                return Err(format!(
                    "an AVC configuration record (avcC) of more than {MAX_PARAMETER_SET_BYTES} \
                     bytes of parameter sets"
                ));
            }
            parameter_sets.push(unit.to_vec());
        }
    }
    Ok((length_size, parameter_sets))
}

/// `mvhd`'s time scale (14496-12, 8.2.2): 32-bit creation and modification times in version 0 and
/// 64-bit in version 1 come before it.
fn movie_header_time_scale(header: &[u8]) -> Result<u32, String> {
    let (version, _, mut fields) = full(header).ok_or_else(|| truncated("mvhd"))?;
    let times = match version {
        0 => 8,
        1 => 16,
        _ => return Err(format!("a movie header (mvhd) of version {version}")),
    };
    fields.skip(times).ok_or_else(|| truncated("mvhd"))?;
    fields.u32().ok_or_else(|| truncated("mvhd"))
}

/// The most entries one track's edit list may hold: past it the file is declined.
///
/// A sound track's list is one entry, or an empty edit and one; a thousand is far past any list a
/// player could honour and bounds the copy.
pub const MAX_EDITS: usize = 1024;

/// One entry of a track's edit list (14496-12, 8.6.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edit {
    /// How long the edit lasts, in the movie's time scale (`mvhd`'s).
    pub segment_duration: u64,
    /// Where in the media the edit begins, in the media's time scale; `-1` for an empty edit.
    pub media_time: i64,
    /// The rate the media plays at, 16.16 fixed point (`media_rate_integer` and
    /// `media_rate_fraction`); `0x0001_0000` for the ordinary rate and 0 for a dwell.
    pub media_rate: i32,
}

/// Reads `elst` (14496-12, 8.6.6): a 32-bit entry count, then each entry's segment duration and
/// media time — 32 bits in version 0, 64 in version 1 — and its 16.16 rate.
fn edit_list(list: &[u8]) -> Result<Vec<Edit>, String> {
    let (version, _, fields) = full(list).ok_or_else(|| truncated("elst"))?;
    let width = match version {
        0 => 12,
        1 => 20,
        _ => return Err(format!("an edit list (elst) of version {version}")),
    };
    let table = Entries::new(list, width, "elst")?;
    if table.count > MAX_EDITS {
        return Err(format!(
            "an edit list (elst) of {} entries, past the {MAX_EDITS} this reader reads",
            table.count
        ));
    }
    let mut fields = Fields {
        rest: fields.rest.get(4..).ok_or_else(|| truncated("elst"))?,
    };
    let mut edits = Vec::with_capacity(table.count);
    for _ in 0..table.count {
        let (segment_duration, media_time) = if version == 1 {
            let duration = fields.u64().ok_or_else(|| truncated("elst"))?;
            let time = fields.u64().ok_or_else(|| truncated("elst"))?;
            (duration, i64::from_be_bytes(time.to_be_bytes()))
        } else {
            let duration = fields.u32().ok_or_else(|| truncated("elst"))?;
            let time = fields.u32().ok_or_else(|| truncated("elst"))?;
            (
                u64::from(duration),
                i64::from(i32::from_be_bytes(time.to_be_bytes())),
            )
        };
        let rate = fields.u32().ok_or_else(|| truncated("elst"))?;
        edits.push(Edit {
            segment_duration,
            media_time,
            media_rate: i32::from_be_bytes(rate.to_be_bytes()),
        });
    }
    Ok(edits)
}

/// What a track's edit list makes of its media, in the media's time scale.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Presentation {
    /// Silence before the media begins: the empty edits that lead the list.
    pub delay: u64,
    /// The media's first units, not presented — an AAC encoder's priming samples.
    pub skip: u64,
    /// How much of the media after [`Self::skip`] is presented, or `None` for all of it.
    pub keep: Option<u64>,
    /// The sentence for an edit list this reader does not apply.
    pub note: Option<String>,
}

/// The ordinary rate of an edit, 1 in 16.16 fixed point.
const RATE_ONE: i32 = 0x0001_0000;

/// What `track`'s edit list presents (ISO/IEC 14496-12:2012, 8.6.5 and 8.6.6; the segment
/// duration is in the movie header's time scale and the media time in the media's, 8.6.6.3).
///
/// An edit list maps the movie's timeline onto the media's: each entry presents, for its segment
/// duration in the movie's time scale, the media from its media time on, and an entry whose media
/// time is −1 is an empty edit, presenting nothing for its duration. A track with no list presents
/// its media from its start. This is how an AAC file states its encoder's priming samples — the
/// decoder's first output before the first real sample — and the padding after the last: one entry
/// whose media time is the priming and whose duration is the sound's own.
///
/// This player applies the shape a sound track has: empty edits first, which become silence, then
/// one edit at the ordinary rate. A segment duration of 0 is read as "the rest of the media" — a
/// choice: 8.6.6.1 gives a zero segment duration a meaning only in the empty initial movie of a
/// fragmented file, and says nothing of one in an unfragmented file, which is all this reader
/// takes. Any
/// other list is not applied and the [`Presentation::note`] says so: the media plays whole.
#[must_use]
pub fn presentation(track: &SoundTrack) -> Presentation {
    let edits = &track.edits;
    if edits.is_empty() {
        return Presentation::default();
    }
    let leading = edits
        .iter()
        .take_while(|edit| edit.media_time == -1)
        .count();
    let not_applied = |why: &str| Presentation {
        note: Some(format!(
            "the sound track's edit list (ISO/IEC 14496-12's elst) of {} entries {why}, so it was \
             not applied and the whole of the media plays",
            edits.len()
        )),
        ..Presentation::default()
    };
    let [edit] = edits.get(leading..).unwrap_or_default() else {
        return not_applied("presents other than one stretch of the media");
    };
    if edit.media_rate != RATE_ONE || edit.media_time < 0 {
        return not_applied("plays the media at a rate other than 1");
    }
    let delay_movie: u64 = edits
        .iter()
        .take(leading)
        .fold(0, |sum, edit| sum.saturating_add(edit.segment_duration));
    let needs_movie_scale = delay_movie != 0 || edit.segment_duration != 0;
    let movie_scale = track.movie_time_scale.filter(|scale| *scale != 0);
    let movie_scale = match movie_scale {
        Some(scale) => scale,
        None if needs_movie_scale => {
            return not_applied("needs the movie header's (mvhd) time scale, which is absent or 0");
        }
        None => 1,
    };
    Presentation {
        delay: rescaled(delay_movie, movie_scale, track.time_scale),
        skip: edit.media_time.unsigned_abs(),
        keep: (edit.segment_duration != 0)
            .then(|| rescaled(edit.segment_duration, movie_scale, track.time_scale)),
        note: None,
    }
}

/// `units` of a clock of `from` per second as the nearest whole number of a clock of `to`.
#[must_use]
pub fn rescaled(units: u64, from: u32, to: u32) -> u64 {
    // A 64-bit count times a 32-bit rate, and half a 32-bit divisor, stay far inside 128 bits; a
    // clock of 0 units a second moves nothing.
    let scaled = u128::from(units)
        .saturating_mul(u128::from(to))
        .saturating_add(u128::from(from / 2))
        .checked_div(u128::from(from))
        .unwrap_or(0);
    u64::try_from(scaled).unwrap_or(u64::MAX)
}

/// The most bytes of a top-level box's header [`top_level_box`] reads: a 64-bit size and a `uuid`
/// type's extended type.
pub const TOP_HEADER_BYTES: usize = 32;

/// One box at the top of a file, placed by its header (14496-12, 4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopBox {
    /// Its four-character type.
    pub kind: [u8; 4],
    /// Where its payload begins, from the start of the file.
    pub payload: u64,
    /// Where it ends, and the next box begins.
    pub end: u64,
}

/// The box whose header is `header` — at most [`TOP_HEADER_BYTES`] of the file from `at` — in a file
/// of `total` bytes.
///
/// The step the worker takes for a host walking a file too long to cross whole (ADR 3381): the host
/// holds the file and sends each box's first bytes, and learns where the box ends.
///
/// # Errors
///
/// The sentence for a header cut short, or a box that ends before its header or past the file.
pub fn top_level_box(header: &[u8], at: u64, total: u64) -> Result<TopBox, String> {
    let mut fields = Fields { rest: header };
    let (Some(size), Some(kind)) = (fields.u32(), fields.array::<4>()) else {
        return Err(truncated("a box header"));
    };
    let (size, mut header_len) = match size {
        0 => (total.saturating_sub(at), 8u64),
        1 => (fields.u64().ok_or_else(|| truncated("a box header"))?, 16),
        size => (u64::from(size), 8),
    };
    if &kind == b"uuid" {
        fields.skip(16).ok_or_else(|| truncated("a box header"))?;
        header_len = header_len.saturating_add(16);
    }
    let end = at.checked_add(size).filter(|end| *end <= total);
    let Some(end) = end.filter(|_| size >= header_len) else {
        return Err(format!(
            "a {} box of {size} bytes at {at}, where {} remain",
            four(kind),
            total.saturating_sub(at)
        ));
    };
    Ok(TopBox {
        kind,
        payload: at.saturating_add(header_len),
        end,
    })
}

/// A track's `mdia` payload where the track is enabled and its handler is `soun`.
fn enabled_sound_media<'a>(
    track: &'a [u8],
    budget: &mut Budget,
) -> Result<Option<&'a [u8]>, String> {
    enabled_media(track, *b"soun", budget)
}

/// A track's `mdia` payload where the track is enabled and its handler is `handler` (14496-12,
/// 8.3.2 and 8.4.3).
fn enabled_media<'a>(
    track: &'a [u8],
    handler: [u8; 4],
    budget: &mut Budget,
) -> Result<Option<&'a [u8]>, String> {
    let header = child(track, *b"tkhd", budget)?.ok_or_else(|| missing("tkhd", "trak"))?;
    let (_, flags, _) = full(header).ok_or_else(|| truncated("tkhd"))?;
    if flags & TRACK_ENABLED == 0 {
        return Ok(None);
    }
    let media = child(track, *b"mdia", budget)?.ok_or_else(|| missing("mdia", "trak"))?;
    let reference = child(media, *b"hdlr", budget)?.ok_or_else(|| missing("hdlr", "mdia"))?;
    let (_, _, mut fields) = full(reference).ok_or_else(|| truncated("hdlr"))?;
    fields.skip(4).ok_or_else(|| truncated("hdlr"))?;
    let kind = fields.array::<4>().ok_or_else(|| truncated("hdlr"))?;
    Ok((kind == handler).then_some(media))
}

/// `tkhd`'s flag for an enabled track (14496-12, 8.3.2).
const TRACK_ENABLED: u32 = 0x00_0001;
/// A data reference entry's flag for media data in this file (14496-12, 8.7.2).
const SELF_CONTAINED: u32 = 0x00_0001;

/// Reads a sound track's `mdia` payload into its table.
fn read_track(media: &[u8], file_len: u64, budget: &mut Budget) -> Result<SoundTrack, String> {
    let tables = Tables::read(media, budget)?;
    let description = sample_description(tables.descriptions, budget)?;
    tables.data_is_here(description.data_reference, budget)?;
    let access_units = tables.access_units(file_len, budget)?;
    Ok(SoundTrack {
        object_type: description.object_type,
        decoder_config: description.decoder_config,
        channels: description.channels,
        sample_rate: description.sample_rate,
        time_scale: tables.time_scale,
        access_units,
        movie_time_scale: None,
        edits: Vec::new(),
    })
}

/// What every track's `mdia` holds whatever its handler: the media's clock, its sample
/// descriptions, and the sample table the access units are laid out from (14496-12, 8.4 to 8.7).
struct Tables<'a> {
    time_scale: u32,
    descriptions: &'a [u8],
    info: &'a [u8],
    table: &'a [u8],
}

impl<'a> Tables<'a> {
    fn read(media: &'a [u8], budget: &mut Budget) -> Result<Self, String> {
        let time_scale = media_time_scale(
            child(media, *b"mdhd", budget)?.ok_or_else(|| missing("mdhd", "mdia"))?,
        )?;
        let info = child(media, *b"minf", budget)?.ok_or_else(|| missing("minf", "mdia"))?;
        let table = child(info, *b"stbl", budget)?.ok_or_else(|| missing("stbl", "minf"))?;
        let descriptions =
            child(table, *b"stsd", budget)?.ok_or_else(|| missing("stsd", "stbl"))?;
        Ok(Self {
            time_scale,
            descriptions,
            info,
            table,
        })
    }

    /// Refuses a description whose data reference names media data outside this file.
    fn data_is_here(&self, data_reference: u16, budget: &mut Budget) -> Result<(), String> {
        let data = child(self.info, *b"dinf", budget)?.ok_or_else(|| missing("dinf", "minf"))?;
        let references = child(data, *b"dref", budget)?.ok_or_else(|| missing("dref", "dinf"))?;
        data_is_here(references, data_reference, budget)
    }

    fn access_units(&self, file_len: u64, budget: &mut Budget) -> Result<Vec<AccessUnit>, String> {
        let table = self.table;
        let sizes = match child(table, *b"stsz", budget)? {
            Some(sizes) => Sizes::from_stsz(sizes, file_len)?,
            None => Sizes::from_stz2(
                child(table, *b"stz2", budget)?.ok_or_else(|| missing("stsz or stz2", "stbl"))?,
            )?,
        };
        let times = Entries::new(
            child(table, *b"stts", budget)?.ok_or_else(|| missing("stts", "stbl"))?,
            8,
            "stts",
        )?;
        let runs = Entries::new(
            child(table, *b"stsc", budget)?.ok_or_else(|| missing("stsc", "stbl"))?,
            12,
            "stsc",
        )?;
        let offsets = match child(table, *b"stco", budget)? {
            Some(offsets) => Entries::new(offsets, 4, "stco")?,
            None => Entries::new(
                child(table, *b"co64", budget)?.ok_or_else(|| missing("stco or co64", "stbl"))?,
                8,
                "co64",
            )?,
        };
        access_units(&sizes, &times, &runs, &offsets, file_len)
    }
}

/// `mdhd`'s time scale (14496-12, 8.4.2), which must be a number of units per second.
fn media_time_scale(header: &[u8]) -> Result<u32, String> {
    let (version, _, mut fields) = full(header).ok_or_else(|| truncated("mdhd"))?;
    let times = match version {
        0 => 8,
        1 => 16,
        _ => return Err(format!("a media header (mdhd) of version {version}")),
    };
    fields.skip(times).ok_or_else(|| truncated("mdhd"))?;
    match fields.u32() {
        Some(0) => Err("a media header (mdhd) whose time scale is 0".to_owned()),
        Some(scale) => Ok(scale),
        None => Err(truncated("mdhd")),
    }
}

/// What a sound track's first sample entry says.
struct Description {
    data_reference: u16,
    channels: u16,
    sample_rate: u32,
    object_type: u8,
    decoder_config: Vec<u8>,
}

/// The bytes of `SampleEntry`'s and `AudioSampleEntry`'s fields before its boxes.
const AUDIO_SAMPLE_ENTRY_FIELDS: usize = 28;

/// Reads `stsd`'s first entry, which must be an `mp4a` audio sample entry of version 0.
fn sample_description(descriptions: &[u8], budget: &mut Budget) -> Result<Description, String> {
    let (_, _, mut fields) = full(descriptions).ok_or_else(|| truncated("stsd"))?;
    let count = fields.u32().ok_or_else(|| truncated("stsd"))?;
    if count == 0 {
        return Err("a sample description box (stsd) with no entry".to_owned());
    }
    let entry = Boxes::new(fields.rest, budget)
        .next()
        .ok_or_else(|| truncated("stsd"))??;
    if &entry.kind != b"mp4a" {
        return Err(format!(
            "a sound track coded as {}, where this reader reads MPEG-4 audio (mp4a)",
            four(entry.kind)
        ));
    }
    let mut fields = Fields {
        rest: entry.payload,
    };
    fields.skip(6).ok_or_else(|| truncated("mp4a"))?;
    let data_reference = fields.u16().ok_or_else(|| truncated("mp4a"))?;
    let version = fields.u16().ok_or_else(|| truncated("mp4a"))?;
    if version != 0 {
        return Err(format!(
            "an audio sample entry of version {version}, where this reader reads version 0"
        ));
    }
    fields.skip(6).ok_or_else(|| truncated("mp4a"))?;
    let channels = fields.u16().ok_or_else(|| truncated("mp4a"))?;
    fields.skip(6).ok_or_else(|| truncated("mp4a"))?;
    let sample_rate = fields.u32().ok_or_else(|| truncated("mp4a"))? >> 16;
    let boxes = entry
        .payload
        .get(AUDIO_SAMPLE_ENTRY_FIELDS..)
        .ok_or_else(|| truncated("mp4a"))?;
    let descriptor = child(boxes, *b"esds", budget)?.ok_or_else(|| missing("esds", "mp4a"))?;
    let (object_type, decoder_config) = elementary_stream(descriptor)?;
    Ok(Description {
        data_reference,
        channels,
        sample_rate,
        object_type,
        decoder_config,
    })
}

/// The `ES_Descriptor`'s tag (14496-1, 7.2.2.1).
const ES_DESCRIPTOR: u8 = 3;
/// The `DecoderConfigDescriptor`'s tag.
const DECODER_CONFIG_DESCRIPTOR: u8 = 4;
/// The `DecoderSpecificInfo`'s tag.
const DECODER_SPECIFIC_INFO: u8 = 5;
/// The audio stream type (14496-1, 7.2.6.6.2).
const AUDIO_STREAM: u8 = 5;

/// Reads `esds`: the object type and the decoder-specific bytes (14496-14, 5.6; 14496-1, 7.2.6).
fn elementary_stream(descriptor: &[u8]) -> Result<(u8, Vec<u8>), String> {
    let (_, _, fields) = full(descriptor).ok_or_else(|| truncated("esds"))?;
    let mut stream = tagged(fields, ES_DESCRIPTOR)?;
    stream.skip(2).ok_or_else(|| truncated("ES_Descriptor"))?;
    let flags = stream.u8().ok_or_else(|| truncated("ES_Descriptor"))?;
    if flags & 0x80 != 0 {
        stream.skip(2).ok_or_else(|| truncated("ES_Descriptor"))?;
    }
    if flags & 0x40 != 0 {
        let length = stream.u8().ok_or_else(|| truncated("ES_Descriptor"))?;
        stream
            .skip(usize::from(length))
            .ok_or_else(|| truncated("ES_Descriptor"))?;
    }
    if flags & 0x20 != 0 {
        stream.skip(2).ok_or_else(|| truncated("ES_Descriptor"))?;
    }
    let mut config = tagged(stream, DECODER_CONFIG_DESCRIPTOR)?;
    let object_type = config
        .u8()
        .ok_or_else(|| truncated("DecoderConfigDescriptor"))?;
    let stream_type = config
        .u8()
        .ok_or_else(|| truncated("DecoderConfigDescriptor"))?
        >> 2;
    if stream_type != AUDIO_STREAM {
        return Err(format!(
            "a sound track whose elementary stream is of type {stream_type}, not audio"
        ));
    }
    config
        .skip(11)
        .ok_or_else(|| truncated("DecoderConfigDescriptor"))?;
    if config.rest.is_empty() {
        return Ok((object_type, Vec::new()));
    }
    let specific = tagged(config, DECODER_SPECIFIC_INFO)?;
    if specific.rest.len() > MAX_DECODER_CONFIG_BYTES {
        return Err(format!(
            "a decoder configuration of {} bytes, past the {MAX_DECODER_CONFIG_BYTES} this reader \
             carries",
            specific.rest.len()
        ));
    }
    Ok((object_type, specific.rest.to_vec()))
}

/// The payload of the descriptor of tag `tag` at the start of `fields`.
///
/// A descriptor's length is an expandable size (14496-1, 8.3.3): up to four bytes of seven bits,
/// each but the last with its high bit set. A length past what holds the descriptor is declined.
fn tagged(mut fields: Fields<'_>, tag: u8) -> Result<Fields<'_>, String> {
    let name = match tag {
        ES_DESCRIPTOR => "ES_Descriptor",
        DECODER_CONFIG_DESCRIPTOR => "DecoderConfigDescriptor",
        _ => "DecoderSpecificInfo",
    };
    let found = fields.u8().ok_or_else(|| truncated(name))?;
    if found != tag {
        return Err(format!(
            "a descriptor of tag {found} where the {name} (tag {tag}) belongs"
        ));
    }
    let mut length = 0usize;
    for _ in 0..4 {
        let byte = fields.u8().ok_or_else(|| truncated(name))?;
        length = (length << 7) | usize::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            let rest = fields.rest.get(..length).ok_or_else(|| truncated(name))?;
            return Ok(Fields { rest });
        }
    }
    Err(format!("a {name} whose length runs past four bytes"))
}

/// Checks that `dref`'s entry `index` (from 1) says the media data is in this file.
fn data_is_here(references: &[u8], index: u16, budget: &mut Budget) -> Result<(), String> {
    let (_, _, mut fields) = full(references).ok_or_else(|| truncated("dref"))?;
    fields.skip(4).ok_or_else(|| truncated("dref"))?;
    let wanted = usize::from(index)
        .checked_sub(1)
        .ok_or_else(|| "a sample entry whose data reference index is 0".to_owned())?;
    let entry = Boxes::new(fields.rest, budget)
        .nth(wanted)
        .ok_or_else(|| format!("a data reference index of {index} that dref does not hold"))??;
    let (_, flags, _) = full(entry.payload).ok_or_else(|| truncated("dref"))?;
    if flags & SELF_CONTAINED == 0 {
        return Err("a sound track whose media data is in another file".to_owned());
    }
    Ok(())
}

/// A sample table's sizes: one for every sample, or one each.
enum Sizes<'a> {
    Constant {
        size: u32,
        count: usize,
    },
    Each {
        table: &'a [u8],
        field_bits: u8,
        count: usize,
    },
}

impl<'a> Sizes<'a> {
    /// Reads `stsz` (14496-12, 8.7.3.2).
    fn from_stsz(payload: &'a [u8], file_len: u64) -> Result<Self, String> {
        let (_, _, mut fields) = full(payload).ok_or_else(|| truncated("stsz"))?;
        let size = fields.u32().ok_or_else(|| truncated("stsz"))?;
        let count = to_count(fields.u32().ok_or_else(|| truncated("stsz"))?)?;
        if size == 0 {
            return Self::each(fields.rest, 32, count, "stsz");
        }
        // One size for every sample states no table, so the count is bounded by the bytes those
        // samples would take rather than by the box.
        let room = file_len.checked_div(u64::from(size)).unwrap_or(0);
        if u64::try_from(count).unwrap_or(u64::MAX) > room {
            return Err(format!(
                "{count} samples of {size} bytes each, past the file's {file_len} bytes"
            ));
        }
        Ok(Sizes::Constant { size, count })
    }

    /// Reads `stz2` (14496-12, 8.7.3.3).
    fn from_stz2(payload: &'a [u8]) -> Result<Self, String> {
        let (_, _, mut fields) = full(payload).ok_or_else(|| truncated("stz2"))?;
        fields.skip(3).ok_or_else(|| truncated("stz2"))?;
        let field_bits = fields.u8().ok_or_else(|| truncated("stz2"))?;
        if !matches!(field_bits, 4 | 8 | 16) {
            return Err(format!(
                "a compact sample size box (stz2) with fields of {field_bits} bits, where 4, 8 \
                 and 16 are defined"
            ));
        }
        let count = to_count(fields.u32().ok_or_else(|| truncated("stz2"))?)?;
        Self::each(fields.rest, field_bits, count, "stz2")
    }

    /// A table of `count` sizes of `field_bits` each, checked against the bytes that hold it.
    fn each(table: &'a [u8], field_bits: u8, count: usize, name: &str) -> Result<Self, String> {
        let bits = count.checked_mul(usize::from(field_bits));
        if bits.is_none_or(|bits| bits.div_ceil(8) > table.len()) {
            return Err(format!(
                "a {name} box that states {count} sizes and holds {} bytes",
                table.len()
            ));
        }
        Ok(Sizes::Each {
            table,
            field_bits,
            count,
        })
    }

    fn count(&self) -> usize {
        match self {
            Sizes::Constant { count, .. } | Sizes::Each { count, .. } => *count,
        }
    }

    /// The size of sample `index` (from 0), which [`Self::each`] has checked the table holds.
    fn get(&self, index: usize) -> Option<u32> {
        match *self {
            Sizes::Constant { size, .. } => Some(size),
            Sizes::Each {
                table, field_bits, ..
            } => match field_bits {
                4 => {
                    let byte = *table.get(index / 2)?;
                    // The first of each pair is the byte's upper half.
                    Some(u32::from(if index.is_multiple_of(2) {
                        byte >> 4
                    } else {
                        byte & 0x0f
                    }))
                }
                8 => table.get(index).copied().map(u32::from),
                16 => Some(u32::from(u16::from_be_bytes(be(
                    table,
                    index.checked_mul(2)?,
                )?))),
                _ => Some(u32::from_be_bytes(be(table, index.checked_mul(4)?)?)),
            },
        }
    }
}

/// A full box's table of fixed-width entries after its 32-bit entry count.
struct Entries<'a> {
    table: &'a [u8],
    width: usize,
    count: usize,
}

impl<'a> Entries<'a> {
    fn new(payload: &'a [u8], width: usize, name: &str) -> Result<Self, String> {
        let (_, _, mut fields) = full(payload).ok_or_else(|| truncated(name))?;
        let count = to_count(fields.u32().ok_or_else(|| truncated(name))?)?;
        if count
            .checked_mul(width)
            .is_none_or(|bytes| bytes > fields.rest.len())
        {
            return Err(format!(
                "a {name} box that states {count} entries and holds {} bytes",
                fields.rest.len()
            ));
        }
        Ok(Entries {
            table: fields.rest,
            width,
            count,
        })
    }

    /// Field `field` (a 32-bit one, from 0) of entry `index`.
    fn u32(&self, index: usize, field: usize) -> Option<u32> {
        let at = index
            .checked_mul(self.width)?
            .checked_add(field.checked_mul(4)?)?;
        be(self.table, at).map(u32::from_be_bytes)
    }

    /// Entry `index` as a chunk offset, 32 or 64 bits by the table's width.
    fn offset(&self, index: usize) -> Option<u64> {
        if self.width == 8 {
            be(self.table, index.checked_mul(8)?).map(u64::from_be_bytes)
        } else {
            self.u32(index, 0).map(u64::from)
        }
    }
}

/// Lays the samples into their chunks and gives each its duration (14496-12, 8.7.4 and 8.6.1.2).
fn access_units(
    sizes: &Sizes<'_>,
    times: &Entries<'_>,
    runs: &Entries<'_>,
    offsets: &Entries<'_>,
    file_len: u64,
) -> Result<Vec<AccessUnit>, String> {
    let count = sizes.count();
    if count > MAX_ACCESS_UNITS {
        return Err(format!(
            "{count} access units, past the {MAX_ACCESS_UNITS} this reader reads"
        ));
    }
    let mut durations = Durations::new(times);
    let mut units = Vec::with_capacity(count);
    let mut previous = 0usize;
    for run in 0..runs.count {
        let (Some(first), Some(per_chunk), Some(description)) =
            (runs.u32(run, 0), runs.u32(run, 1), runs.u32(run, 2))
        else {
            return Err(truncated("stsc"));
        };
        if description != 1 {
            return Err(format!(
                "samples that name sample description {description}, where this reader reads \
                 the first"
            ));
        }
        // Chunks are numbered from 1, and each run lasts until the next run's first chunk, so the
        // runs must begin at 1 and increase; the last lasts to the last chunk.
        let first = usize::try_from(first).unwrap_or(usize::MAX);
        let increases = if run == 0 {
            first == 1
        } else {
            first > previous
        };
        if !increases {
            return Err(
                "a sample-to-chunk run (stsc) that does not begin at chunk 1 and increase"
                    .to_owned(),
            );
        }
        previous = first;
        let following = run.saturating_add(1);
        let end = if following < runs.count {
            let next = runs.u32(following, 0).ok_or_else(|| truncated("stsc"))?;
            usize::try_from(next)
                .unwrap_or(usize::MAX)
                .saturating_sub(1)
        } else {
            offsets.count
        };
        if first > offsets.count || end > offsets.count {
            return Err(format!(
                "a sample-to-chunk run (stsc) past the {} chunks the offsets name",
                offsets.count
            ));
        }
        // `first` is at least 1: the first run's is 1 and the rest increase.
        for index in first.saturating_sub(1)..end {
            let mut at = offsets
                .offset(index)
                .ok_or_else(|| truncated("chunk offsets"))?;
            for _ in 0..per_chunk {
                if units.len() == count {
                    return Err(format!(
                        "chunks that hold more samples than the {count} the sizes count"
                    ));
                }
                let size = sizes.get(units.len()).ok_or_else(|| truncated("sizes"))?;
                let duration = durations.next().ok_or_else(|| {
                    format!("durations (stts) for fewer samples than the {count} the sizes count")
                })?;
                let unit_end = at
                    .checked_add(u64::from(size))
                    .filter(|end| *end <= file_len);
                let Some(unit_end) = unit_end else {
                    return Err(format!(
                        "an access unit at {at} of {size} bytes, past the file's {file_len}"
                    ));
                };
                units.push(AccessUnit {
                    offset: at,
                    size,
                    duration,
                });
                at = unit_end;
            }
        }
    }
    if units.len() != count {
        return Err(format!(
            "chunks that hold {} samples where the sizes count {count}",
            units.len()
        ));
    }
    if durations.next().is_some() {
        return Err(format!(
            "durations (stts) for more samples than the {count} the sizes count"
        ));
    }
    Ok(units)
}

/// `stts`'s runs, one duration per sample.
struct Durations<'a, 'b> {
    times: &'b Entries<'a>,
    run: usize,
    left: u32,
    delta: u32,
}

impl<'a, 'b> Durations<'a, 'b> {
    fn new(times: &'b Entries<'a>) -> Self {
        Durations {
            times,
            run: 0,
            left: 0,
            delta: 0,
        }
    }
}

impl Iterator for Durations<'_, '_> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        // A run of zero samples is skipped; the walk is bounded by the table's entries.
        while self.left == 0 {
            if self.run >= self.times.count {
                return None;
            }
            self.left = self.times.u32(self.run, 0)?;
            self.delta = self.times.u32(self.run, 1)?;
            self.run = self.run.saturating_add(1);
        }
        self.left = self.left.saturating_sub(1);
        Some(self.delta)
    }
}

/// A table's 32-bit entry count as a `usize`, declined past [`MAX_ACCESS_UNITS`] times eight —
/// beyond any table a track this reader reads can need, and a bound on every product taken of it.
fn to_count(count: u32) -> Result<usize, String> {
    usize::try_from(count)
        .ok()
        .filter(|count| *count <= MAX_ACCESS_UNITS * 8)
        .ok_or_else(|| format!("a table of {count} entries"))
}

/// One box: its type and its payload.
struct Found<'a> {
    kind: [u8; 4],
    payload: &'a [u8],
}

/// The boxes of a payload, in order, each counted against the walk's [`Budget`].
struct Boxes<'a, 'b> {
    rest: &'a [u8],
    budget: &'b mut Budget,
    failed: bool,
}

impl<'a, 'b> Boxes<'a, 'b> {
    fn new(payload: &'a [u8], budget: &'b mut Budget) -> Self {
        Boxes {
            rest: payload,
            budget,
            failed: false,
        }
    }
}

impl<'a> Iterator for Boxes<'a, '_> {
    type Item = Result<Found<'a>, String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() || self.failed {
            return None;
        }
        let found = self.budget.visit().and_then(|()| split_box(self.rest));
        match found {
            Ok((found, rest)) => {
                self.rest = rest;
                Some(Ok(found))
            }
            Err(sentence) => {
                self.failed = true;
                Some(Err(sentence))
            }
        }
    }
}

/// The box at the start of `bytes`, and what follows it (14496-12, 4.2).
fn split_box(bytes: &[u8]) -> Result<(Found<'_>, &[u8]), String> {
    let mut header = Fields { rest: bytes };
    let (Some(size), Some(kind)) = (header.u32(), header.array::<4>()) else {
        return Err(truncated("a box header"));
    };
    let (size, mut header_len): (usize, usize) = match size {
        0 => (bytes.len(), 8),
        1 => {
            let large = header.u64().ok_or_else(|| truncated("a box header"))?;
            (usize::try_from(large).unwrap_or(usize::MAX), 16)
        }
        size => (usize::try_from(size).unwrap_or(usize::MAX), 8),
    };
    if &kind == b"uuid" {
        header_len = header_len.saturating_add(16);
    }
    if size < header_len || size > bytes.len() {
        return Err(format!(
            "a {} box of {size} bytes where {} remain",
            four(kind),
            bytes.len()
        ));
    }
    let payload = &bytes[header_len..size];
    Ok((Found { kind, payload }, &bytes[size..]))
}

/// The payload of the first child of `kind` in `parent`'s payload.
fn child<'a>(
    parent: &'a [u8],
    kind: [u8; 4],
    budget: &mut Budget,
) -> Result<Option<&'a [u8]>, String> {
    for found in Boxes::new(parent, budget) {
        let found = found?;
        if found.kind == kind {
            return Ok(Some(found.payload));
        }
    }
    Ok(None)
}

/// A full box's version, flags, and the fields after them.
fn full(payload: &[u8]) -> Option<(u8, u32, Fields<'_>)> {
    let mut fields = Fields { rest: payload };
    let [version, flags @ ..] = fields.array::<4>()?;
    let [high, middle, low] = flags;
    Some((version, u32::from_be_bytes([0, high, middle, low]), fields))
}

/// `N` big-endian bytes at `at` of `table`.
fn be<const N: usize>(table: &[u8], at: usize) -> Option<[u8; N]> {
    table.get(at..at.checked_add(N)?)?.try_into().ok()
}

/// A box type for a sentence.
fn four(kind: [u8; 4]) -> String {
    kind.iter()
        .map(|&byte| {
            if byte.is_ascii_graphic() || byte == b' ' {
                char::from(byte)
            } else {
                '?'
            }
        })
        .collect()
}

fn truncated(what: &str) -> String {
    format!("{what} that ends before its fields do")
}

fn missing(what: &str, parent: &str) -> String {
    format!("a {parent} box with no {what} box")
}

/// The walk's count of boxes visited.
#[derive(Default)]
struct Budget {
    visited: usize,
}

impl Budget {
    fn visit(&mut self) -> Result<(), String> {
        self.visited = self.visited.saturating_add(1);
        if self.visited > MAX_BOXES {
            return Err(format!("more than the {MAX_BOXES} boxes this reader walks"));
        }
        Ok(())
    }
}

/// A cursor over big-endian fields.
struct Fields<'a> {
    rest: &'a [u8],
}

impl<'a> Fields<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let (taken, rest) = self.rest.split_at_checked(count)?;
        self.rest = rest;
        Some(taken)
    }

    fn skip(&mut self, count: usize) -> Option<()> {
        self.take(count).map(|_| ())
    }

    fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.take(N)?.try_into().ok()
    }

    fn u8(&mut self) -> Option<u8> {
        self.array::<1>().map(|[byte]| byte)
    }

    fn u16(&mut self) -> Option<u16> {
        self.array().map(u16::from_be_bytes)
    }

    fn u32(&mut self) -> Option<u32> {
        self.array().map(u32::from_be_bytes)
    }

    fn u64(&mut self) -> Option<u64> {
        self.array().map(u64::from_be_bytes)
    }
}
