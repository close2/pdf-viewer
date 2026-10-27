//! A SWF file's timeline: Adobe's *SWF File Format Specification*, version 19, chapter 2 (the SWF
//! header and the SWF file structure) and the two tags that name a frame.
//!
//! The header states the playback rate (an 8.8 fixed-point number of frames per second) and the
//! total number of frames, so the file's nominal length is the one over the other. The rate is
//! the desired playback rate, which the specification does not guarantee, and a movie's own
//! actions can stop or jump its timeline; the container's own terms for the length are all this
//! reads (ADR 2488).
//!
//! The named offsets are the frame labels of the main timeline: the `FrameLabel` tag (type 43),
//! whose `NamedAnchor` form adds one byte after the name, and the `DefineSceneAndFrameLabelData`
//! tag (type 86), whose scenes and frame labels each carry a frame number. A label is placed at
//! the frame the `ShowFrame` tags (type 1) before it number. Tags inside a `DefineSprite` (type 39)
//! name the frames of the sprite's own timeline, which the main timeline's clip does not play, so
//! the walk steps over a sprite by the length its tag states.
//!
//! A compressed file (signature `CWS`) holds the rest of the file after the eight-byte start as one
//! zlib stream (RFC 1950); the stream is inflated under the length the header states, and a file
//! whose inflated bytes are not that length is refused, because a label read from a short file
//! would be judged absent. The LZMA form (`ZWS`) is a container this program has no decoder for.

use pdf_syntax::Limits;
use pdf_syntax::filter::{FilterRefusal, decode_reported};

use super::{Bytes, MAX_MARKERS, Marker, Timeline, too_many, truncated};

/// Most tags one walk visits.
///
/// A tag is stepped over by its stated length, so a walk costs one step a tag whatever the tag
/// holds. The figure is this reader's own work budget, not one the specification states: it is
/// far past the tags of any movie whose frame labels a viewer would honour, and a file with more
/// is refused by name (the error says the tags exceed what the reader walks) rather than read in
/// part, because a label left unread would be judged absent.
const MAX_TAGS: usize = 1 << 17;

/// The tag types this reads, from the specification's list of tag types.
const SHOW_FRAME: u16 = 1;
const END: u16 = 0;
const FRAME_LABEL: u16 = 43;
const DEFINE_SCENE_AND_FRAME_LABEL_DATA: u16 = 86;

/// The first SWF version that stores strings as UTF-8 (the chapter on data types) and may
/// compress the file.
const UTF8_FROM_VERSION: u8 = 6;

/// Reads the timeline of the SWF file `bytes`.
pub(super) fn timeline(bytes: &[u8]) -> Result<Timeline, String> {
    let mut start = Bytes { rest: bytes };
    let signature = start
        .array::<3>()
        .ok_or_else(|| truncated("SWF signature"))?;
    let (Some(version), Some(length)) = (start.u8(), start.u32_le()) else {
        return Err(truncated("SWF header"));
    };
    let inflated;
    let body: &[u8] = match &signature {
        b"FWS" => {
            if usize::try_from(length).map_or(true, |length| length > bytes.len()) {
                return Err(truncated("SWF file"));
            }
            start.rest
        }
        b"CWS" => {
            if version < UTF8_FROM_VERSION {
                return Err(
                    "its SWF file is compressed but states a version before 6, which the \
                     specification does not permit to be"
                        .to_owned(),
                );
            }
            inflated = inflate(start.rest, length)?;
            &inflated
        }
        b"ZWS" => {
            return Err(
                "its SWF file is LZMA compressed, which this program has no decoder for".to_owned(),
            );
        }
        _ => return Err("its bytes are not the SWF file its type names".to_owned()),
    };
    let mut fields = Bytes { rest: body };
    skip_rectangle(&mut fields)?;
    let (Some(rate), Some(count)) = (fields.u16_le(), fields.u16_le()) else {
        return Err(truncated("SWF header"));
    };
    // An 8.8 fixed-point number: the high byte is whole frames and the low byte is 256ths.
    let per_second = (rate != 0).then(|| f64::from(rate) / 256.0);
    let labels = labels(fields.rest, version)?;
    let mut markers = Vec::with_capacity(labels.len());
    for (name, frame) in labels {
        let Some(per_second) = per_second else {
            return Err(
                "its SWF header states a frame rate of zero, so its frame labels fall nowhere"
                    .to_owned(),
            );
        };
        markers.push(Marker {
            name,
            seconds: f64::from(frame) / per_second,
        });
    }
    Ok(Timeline {
        seconds: per_second.map(|per_second| f64::from(count) / per_second),
        frames: Some(u64::from(count)),
        markers,
    })
}

/// Inflates the zlib stream of a `CWS` file under the length its header states.
///
/// The header's length covers the eight bytes before the stream, so the stream inflates to that
/// many less, and the decode is limited to it: a stream that inflates to more is refused as the
/// decoder's own bound, one that inflates to less ends short and is refused here.
fn inflate(stream: &[u8], length: u32) -> Result<Vec<u8>, String> {
    let inflated = usize::try_from(length)
        .ok()
        .and_then(|length| length.checked_sub(8))
        .filter(|inflated| *inflated <= Limits::DEFAULT.max_stream_len)
        .ok_or_else(|| {
            "its SWF header states a length this program does not inflate to, or one shorter than \
             its own header"
                .to_owned()
        })?;
    let limits = Limits {
        max_stream_len: inflated,
        ..Limits::DEFAULT
    };
    let decoded =
        decode_reported(b"FlateDecode", stream, None, limits).map_err(|refusal| match refusal {
            FilterRefusal::TooLarge { .. } => {
                "its SWF file's zlib stream inflates past the length its header states".to_owned()
            }
            FilterRefusal::Corrupt | FilterRefusal::Unsupported => {
                "its SWF file's body is not a zlib stream".to_owned()
            }
        })?;
    if decoded.damage.is_some() || decoded.data.len() != inflated {
        return Err(truncated("SWF file"));
    }
    Ok(decoded.data.to_vec())
}

/// Steps over the header's RECT: five bits stating `Nbits`, then four signed fields of that many
/// bits, the whole padded to a whole byte (the chapter on bit values).
fn skip_rectangle(fields: &mut Bytes<'_>) -> Result<(), String> {
    let first = fields.rest.first().ok_or_else(|| truncated("SWF header"))?;
    let bits = usize::from(first >> 3);
    let total = 5usize.saturating_add(bits.saturating_mul(4));
    fields
        .take(total.div_ceil(8))
        .map(|_| ())
        .ok_or_else(|| truncated("SWF header"))
}

/// A null-terminated STRING at the front of `bytes`, and what follows it.
fn string(bytes: &[u8], version: u8) -> Result<(String, &[u8]), String> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| truncated("SWF string"))?;
    let (text, rest) = bytes.split_at(end);
    let name = if version >= UTF8_FROM_VERSION {
        std::str::from_utf8(text)
            .map_err(|_| "its SWF file names a frame in bytes that are not UTF-8".to_owned())?
            .to_owned()
    } else if text.is_ascii() {
        String::from_utf8_lossy(text).into_owned()
    } else {
        // Before SWF 6 a string is ANSI or shift-JIS and the file does not say which, so a name
        // past ASCII has no reading a marker could be matched against.
        return Err(
            "its SWF file names a frame in an encoding the specification leaves unstated"
                .to_owned(),
        );
    };
    Ok((name, rest.get(1..).unwrap_or_default()))
}

/// An `EncodedU32`, the variable-length 32-bit unsigned integer: seven bits a byte, low
/// bytes first, the high bit saying another byte follows, at most five.
fn encoded_u32(bytes: &mut Bytes<'_>) -> Option<u32> {
    let mut value = 0u64;
    for index in 0..5u32 {
        let byte = bytes.u8()?;
        value |= u64::from(byte & 0x7F) << index.saturating_mul(7);
        if byte & 0x80 == 0 {
            break;
        }
    }
    u32::try_from(value & u64::from(u32::MAX)).ok()
}

/// The main timeline's named frames, each with its zero-based frame number.
fn labels(tags: &[u8], version: u8) -> Result<Vec<(String, u32)>, String> {
    let mut rest = Bytes { rest: tags };
    let mut found = Vec::new();
    let mut frame = 0u32;
    for _ in 0..MAX_TAGS {
        let header = rest.u16_le().ok_or_else(|| truncated("SWF tag"))?;
        let (kind, short) = (header >> 6, usize::from(header & 0x3F));
        let length = if short == 0x3F {
            let long = rest.u32_le().ok_or_else(|| truncated("SWF tag"))?;
            usize::try_from(long).map_err(|_| truncated("SWF tag"))?
        } else {
            short
        };
        let body = rest.take(length).ok_or_else(|| truncated("SWF tag"))?;
        match kind {
            END => return Ok(found),
            SHOW_FRAME => frame = frame.saturating_add(1),
            FRAME_LABEL => {
                let (name, _anchor) = string(body, version)?;
                found.push((name, frame));
            }
            DEFINE_SCENE_AND_FRAME_LABEL_DATA => scenes_and_labels(body, version, &mut found)?,
            _ => {}
        }
        if found.len() > MAX_MARKERS {
            return Err(too_many());
        }
    }
    Err(
        "its SWF file holds more tags than this reader walks, so its frame labels are not read"
            .to_owned(),
    )
}

/// A `DefineSceneAndFrameLabelData` tag's scenes and frame labels: a count, then for each an
/// `EncodedU32` frame number or offset and a STRING.
fn scenes_and_labels(
    body: &[u8],
    version: u8,
    found: &mut Vec<(String, u32)>,
) -> Result<(), String> {
    let mut fields = Bytes { rest: body };
    for _ in 0..2 {
        let count = encoded_u32(&mut fields).ok_or_else(|| truncated("SWF scene tag"))?;
        if usize::try_from(count).map_or(true, |count| count > MAX_MARKERS) {
            return Err(too_many());
        }
        for _ in 0..count {
            let frame = encoded_u32(&mut fields).ok_or_else(|| truncated("SWF scene tag"))?;
            let (name, rest) = string(fields.rest, version)?;
            fields.rest = rest;
            found.push((name, frame));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    /// A tag with a short or long header as its body's length needs.
    pub(crate) fn tag(kind: u16, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        if body.len() < 0x3F {
            let length = u16::try_from(body.len()).expect("short");
            out.extend_from_slice(&((kind << 6) | length).to_le_bytes());
        } else {
            out.extend_from_slice(&((kind << 6) | 0x3F).to_le_bytes());
            out.extend_from_slice(&u32::try_from(body.len()).expect("small").to_le_bytes());
        }
        out.extend_from_slice(body);
        out
    }

    /// An uncompressed SWF file of `version` whose frames run at `rate` (8.8) for `count` frames,
    /// holding `tags` before its End tag.
    pub(crate) fn file(version: u8, rate: u16, count: u16, tags: &[Vec<u8>]) -> Vec<u8> {
        // A RECT of Nbits 15 with Xmin, Ymin 0 and Xmax, Ymax 4000: 5 + 60 bits, padded to 9 bytes.
        let mut bits = String::from("01111");
        for value in [0u32, 4000, 0, 4000] {
            let _ = write!(bits, "{value:015b}");
        }
        while bits.len() % 8 != 0 {
            bits.push('0');
        }
        let mut body: Vec<u8> = bits
            .as_bytes()
            .chunks(8)
            .map(|chunk| {
                u8::from_str_radix(std::str::from_utf8(chunk).expect("ascii"), 2).expect("bits")
            })
            .collect();
        body.extend_from_slice(&rate.to_le_bytes());
        body.extend_from_slice(&count.to_le_bytes());
        for one in tags {
            body.extend_from_slice(one);
        }
        body.extend(tag(END, &[]));
        let mut out = b"FWS".to_vec();
        out.push(version);
        out.extend_from_slice(
            &u32::try_from(body.len().saturating_add(8))
                .expect("small")
                .to_le_bytes(),
        );
        out.extend(body);
        out
    }

    #[test]
    fn a_header_states_the_length_as_its_frames_over_its_rate() {
        // 24 frames a second is 0x1800 in 8.8, so 72 frames run for three seconds.
        let read = timeline(&file(9, 0x1800, 72, &[])).expect("a SWF file");
        assert_eq!(read.seconds, Some(3.0));
        assert_eq!(read.frames, Some(72));
        assert!(read.markers.is_empty());
        // A rate with a fraction: 12.5 frames a second is 0x0C80.
        let read = timeline(&file(9, 0x0C80, 25, &[])).expect("a SWF file");
        assert_eq!(read.seconds, Some(2.0));
    }

    #[test]
    fn a_frame_label_falls_at_the_frame_the_show_frame_tags_before_it_number() {
        let tags = [
            tag(SHOW_FRAME, &[]),
            tag(SHOW_FRAME, &[]),
            tag(FRAME_LABEL, b"intro\0"),
            tag(SHOW_FRAME, &[]),
            // A named anchor adds a byte after the name.
            tag(FRAME_LABEL, b"anchor\0\x01"),
        ]
        .to_vec();
        let read = timeline(&file(8, 0x0A00, 10, &tags)).expect("a SWF file");
        let at = |name: &str| {
            read.markers
                .iter()
                .find(|marker| marker.name == name)
                .map(|marker| marker.seconds)
        };
        assert_eq!(at("intro"), Some(0.2));
        assert_eq!(at("anchor"), Some(0.3));
    }

    #[test]
    fn a_sprite_s_own_labels_are_not_the_main_timeline_s() {
        let sprite = tag(
            39,
            &[1, 0, 1, 0, 43 << 6 | 6, 0, b'i', b'n', b'n', b'e', b'r'],
        );
        let read = timeline(&file(8, 0x0A00, 10, &[sprite])).expect("a SWF file");
        assert!(read.markers.is_empty());
    }

    #[test]
    fn scenes_and_frame_labels_of_the_scene_tag_are_markers() {
        let mut body = vec![2, 0];
        body.extend_from_slice(b"Scene 1\0");
        body.extend_from_slice(&[40]);
        body.extend_from_slice(b"Scene 2\0");
        body.extend_from_slice(&[1, 5]);
        body.extend_from_slice(b"mid\0");
        let read = timeline(&file(9, 0x0A00, 100, &[tag(86, &body)])).expect("a SWF file");
        let names: Vec<_> = read
            .markers
            .iter()
            .map(|marker| (marker.name.as_str(), marker.seconds))
            .collect();
        assert_eq!(names, [("Scene 1", 0.0), ("Scene 2", 4.0), ("mid", 0.5)]);
    }

    #[test]
    fn a_compressed_file_is_inflated_under_its_stated_length() {
        let plain = file(9, 0x1800, 72, &[tag(FRAME_LABEL, b"go\0")]);
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, &plain[8..]).expect("write to memory");
        let stream = encoder.finish().expect("deflate");
        let mut packed = b"CWS".to_vec();
        packed.push(9);
        packed.extend_from_slice(&plain[4..8]);
        packed.extend(&stream);
        let read = timeline(&packed).expect("a compressed SWF file");
        assert_eq!(read.seconds, Some(3.0));
        assert_eq!(read.markers.len(), 1);
        // The same stream under a header stating a longer file is short of it.
        let mut longer = packed.clone();
        longer[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(timeline(&longer).is_err());
        let mut shorter = packed.clone();
        shorter[4..8].copy_from_slice(&20u32.to_le_bytes());
        assert!(
            timeline(&shorter)
                .expect_err("past the stated length")
                .contains("inflates past the length its header states")
        );
        let mut garbage = b"CWS".to_vec();
        garbage.push(9);
        garbage.extend_from_slice(&plain[4..8]);
        garbage.extend_from_slice(&[0xFF; 16]);
        assert!(
            timeline(&garbage)
                .expect_err("not zlib")
                .contains("not a zlib stream")
        );
    }

    #[test]
    fn an_lzma_file_a_foreign_file_and_a_cut_one_are_refused() {
        let mut lzma = file(13, 0x1800, 1, &[]);
        lzma[0] = b'Z';
        assert!(timeline(&lzma).expect_err("LZMA").contains("LZMA"));
        assert!(timeline(b"GIF89a").is_err());
        let whole = file(9, 0x1800, 1, &[tag(FRAME_LABEL, b"x\0")]);
        for cut in 0..whole.len().saturating_sub(1) {
            assert!(
                timeline(&whole[..cut]).is_err(),
                "a file cut at {cut} was read"
            );
        }
    }

    #[test]
    fn a_zero_rate_names_no_length_and_no_place_for_a_label() {
        let read = timeline(&file(9, 0, 5, &[])).expect("a SWF file");
        assert_eq!(read.seconds, None);
        assert!(timeline(&file(9, 0, 5, &[tag(FRAME_LABEL, b"x\0")])).is_err());
    }

    #[test]
    fn a_name_before_utf_8_is_matched_only_where_it_is_ascii() {
        assert!(timeline(&file(5, 0x1800, 1, &[tag(FRAME_LABEL, b"caf\xe9\0")])).is_err());
        assert!(timeline(&file(5, 0x1800, 1, &[tag(FRAME_LABEL, b"cafe\0")])).is_ok());
        let utf8 = tag(FRAME_LABEL, "caf\u{e9}\0".as_bytes());
        let read = timeline(&file(9, 0x1800, 1, &[utf8])).expect("a SWF file");
        assert_eq!(read.markers[0].name, "caf\u{e9}");
    }
}
