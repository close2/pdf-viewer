//! An ASF file's timeline — the container Windows Media Video (`.wmv`) and Audio (`.wma`) files
//! are: Microsoft's *Advanced Systems Format (ASF) Specification*, revision 01.20.03 of December
//! 2004, sections 2 and 3.
//!
//! An ASF file is a sequence of objects, each a 16-byte GUID, a 64-bit size that counts the 24
//! bytes of this header, and the object's data, all stored little-endian (section 2.1). The Header Object (section 3.1) comes first
//! and holds the others, among them:
//!
//! - the File Properties Object (section 3.2), whose Play Duration is the time the file takes to
//!   play in 100-nanosecond units, less the Preroll in milliseconds that a player subtracts from
//!   it. The value includes an estimate of the last media object's duration where an exact one is
//!   unavailable, so the length this reads is the producer's statement of it; a file whose
//!   Broadcast Flag is set is still being written and the section calls its values invalid, so it
//!   states none;
//! - the Marker Object (section 3.7), the file's named jump points, each with a presentation time
//!   in 100-nanosecond units and a description.
//!
//! The specification gives the GUIDs as the text `75B22630-668E-11CF-A6D9-00AA0062CE6C` and says
//! all of the format is little-endian, without drawing the sixteen bytes. They are read here as the
//! four fields of the Windows `GUID` the text spells — a 32-bit and two 16-bit little-endian
//! numbers, then eight bytes in the order written — and the Header Object, which every file must
//! open with, is the check: a file whose first sixteen bytes are not that reading of
//! `ASF_Header_Object` is refused as not the container its type names (ADR 2488).
//!
//! The specification does not say whether a Marker's presentation time carries the Preroll its
//! payload presentation times carry, so a file with a Preroll that names markers states none that
//! can be placed, and is refused rather than guessed.

use super::{Budget, Bytes, MAX_ITEMS, MAX_MARKERS, Marker, Timeline, too_many, truncated};

/// `ASF_Header_Object`, section 10.1, as the sixteen bytes of the Windows `GUID` the text spells.
const HEADER_OBJECT: [u8; 16] = guid(
    0x75B2_2630,
    0x668E,
    0x11CF,
    [0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE, 0x6C],
);
/// `ASF_File_Properties_Object`, section 10.2.
const FILE_PROPERTIES_OBJECT: [u8; 16] = guid(
    0x8CAB_DCA1,
    0xA947,
    0x11CF,
    [0x8E, 0xE4, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65],
);
/// `ASF_Marker_Object`, section 10.2.
const MARKER_OBJECT: [u8; 16] = guid(
    0xF487_CD01,
    0xA951,
    0x11CF,
    [0x8E, 0xE6, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65],
);

/// The bytes of a GUID written `data1-data2-data3-data4`.
const fn guid(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> [u8; 16] {
    let one = data1.to_le_bytes();
    let two = data2.to_le_bytes();
    let three = data3.to_le_bytes();
    [
        one[0], one[1], one[2], one[3], two[0], two[1], three[0], three[1], data4[0], data4[1],
        data4[2], data4[3], data4[4], data4[5], data4[6], data4[7],
    ]
}

/// Bytes of an object's GUID and size, which its stated size counts.
const OBJECT_HEADER: u64 = 24;

/// Units of a presentation time and a play duration a second holds (100 ns each).
const UNITS_PER_SECOND: f64 = 10_000_000.0;

/// Reads the timeline of the ASF file `bytes`.
pub(super) fn timeline(bytes: &[u8]) -> Result<Timeline, String> {
    let mut file = Bytes { rest: bytes };
    let (Some(id), Some(size), Some(count)) = (file.array::<16>(), file.u64_le(), file.u32_le())
    else {
        return Err(truncated("ASF Header Object"));
    };
    if id != HEADER_OBJECT {
        return Err("its bytes are not the ASF file its type names".to_owned());
    }
    // Two reserved bytes close the Header Object's own fields, 30 bytes in all.
    let _reserved = file.take(2).ok_or_else(|| truncated("ASF Header Object"))?;
    let inside = size
        .checked_sub(OBJECT_HEADER + 6)
        .and_then(|inside| usize::try_from(inside).ok())
        .ok_or_else(|| truncated("ASF Header Object"))?;
    let mut objects = Bytes {
        rest: file
            .take(inside)
            .ok_or_else(|| truncated("ASF Header Object"))?,
    };
    let mut budget = Budget { visited: 0 };
    let mut properties = None;
    let mut marked = Vec::new();
    for _ in 0..count {
        budget.visit()?;
        let (Some(id), Some(size)) = (objects.array::<16>(), objects.u64_le()) else {
            return Err(truncated("ASF header object"));
        };
        let data = size
            .checked_sub(OBJECT_HEADER)
            .and_then(|data| usize::try_from(data).ok())
            .and_then(|data| objects.take(data))
            .ok_or_else(|| truncated("ASF header object"))?;
        if id == FILE_PROPERTIES_OBJECT {
            properties = Some(file_properties(data)?);
        } else if id == MARKER_OBJECT {
            markers(data, &mut marked)?;
        }
    }
    let Some((duration, preroll)) = properties else {
        return Err("its ASF Header Object holds no File Properties Object".to_owned());
    };
    if preroll != 0 && !marked.is_empty() {
        return Err(
            "its ASF file names markers and states a preroll, and the specification does not \
             say whether a marker's presentation time carries it"
                .to_owned(),
        );
    }
    Ok(Timeline {
        seconds: duration,
        frames: None,
        markers: marked,
    })
}

/// The File Properties Object's play length in seconds, where it states one, and its preroll in
/// milliseconds.
fn file_properties(data: &[u8]) -> Result<(Option<f64>, u64), String> {
    // File ID (16), File Size (8), Creation Date (8), Data Packets Count (8), then the fields read.
    let mut fields = Bytes { rest: data };
    fields
        .take(40)
        .ok_or_else(|| truncated("ASF File Properties Object"))?;
    let (Some(play), Some(_send), Some(preroll), Some(flags)) = (
        fields.u64_le(),
        fields.u64_le(),
        fields.u64_le(),
        fields.u32_le(),
    ) else {
        return Err(truncated("ASF File Properties Object"));
    };
    // Flags bit 0, the least significant, is the Broadcast Flag.
    if flags & 1 != 0 {
        return Ok((None, preroll));
    }
    // The preroll is in milliseconds and the play duration in 100 ns units: 10 000 of them a
    // millisecond. A preroll longer than the play duration leaves no length to state.
    let played = preroll
        .checked_mul(10_000)
        .and_then(|preroll| play.checked_sub(preroll));
    Ok((played.map(units_to_seconds), preroll))
}

/// Seconds that `units` of 100 ns make.
#[expect(
    clippy::cast_precision_loss,
    reason = "a duration is compared with a PDF number, which is a real (§7.3.3); a count past 2^53 \
              units loses only units no offset can state"
)]
fn units_to_seconds(units: u64) -> f64 {
    units as f64 / UNITS_PER_SECOND
}

/// The Marker Object's markers, named by their descriptions.
fn markers(data: &[u8], out: &mut Vec<Marker>) -> Result<(), String> {
    let mut fields = Bytes { rest: data };
    // Reserved GUID (16), Markers Count (4), Reserved (2), Name Length (2) and the Name.
    let _reserved = fields
        .take(16)
        .ok_or_else(|| truncated("ASF Marker Object"))?;
    let (Some(count), Some(_), Some(name_length)) =
        (fields.u32_le(), fields.u16_le(), fields.u16_le())
    else {
        return Err(truncated("ASF Marker Object"));
    };
    fields
        .take(usize::from(name_length))
        .ok_or_else(|| truncated("ASF Marker Object"))?;
    if usize::try_from(count).map_or(true, |count| count > MAX_MARKERS || count > MAX_ITEMS) {
        return Err(too_many());
    }
    for _ in 0..count {
        // Offset (8), Presentation Time (8), Entry Length (2), Send Time (4), Flags (4), Marker
        // Description Length (4, in WCHARs) and the Description.
        let (Some(_offset), Some(time), Some(entry), Some(_send), Some(_flags), Some(length)) = (
            fields.u64_le(),
            fields.u64_le(),
            fields.u16_le(),
            fields.u32_le(),
            fields.u32_le(),
            fields.u32_le(),
        ) else {
            return Err(truncated("ASF Marker Object"));
        };
        let wide = usize::try_from(length)
            .ok()
            .and_then(|length| length.checked_mul(2))
            .ok_or_else(|| truncated("ASF Marker Object"))?;
        // Section 3.7 makes the entry length the size of the Send Time, Flags and Marker
        // Description Length fields plus the bytes of the description.
        if usize::from(entry) != wide.saturating_add(12) {
            return Err("its ASF marker states an entry length that is not its own".to_owned());
        }
        let description = fields
            .take(wide)
            .ok_or_else(|| truncated("ASF Marker Object"))?;
        out.push(Marker {
            name: description_text(description),
            seconds: units_to_seconds(time),
        });
        if out.len() > MAX_MARKERS {
            return Err(too_many());
        }
    }
    Ok(())
}

/// A description: UTF-16 little-endian without a byte-order mark, ended by its NUL where it has
/// one (section 3.7 allows the terminator to be absent).
fn description_text(bytes: &[u8]) -> String {
    let units = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|unit| *unit != 0);
    char::decode_utf16(units)
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(id: [u8; 16], data: &[u8]) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend_from_slice(
            &u64::try_from(data.len().saturating_add(24))
                .expect("small")
                .to_le_bytes(),
        );
        out.extend_from_slice(data);
        out
    }

    /// A File Properties Object stating `play` (100 ns units), `preroll` (ms) and `flags`.
    fn properties(play: u64, preroll: u64, flags: u32) -> Vec<u8> {
        let mut data = vec![0; 40];
        data.extend_from_slice(&play.to_le_bytes());
        data.extend_from_slice(&play.to_le_bytes());
        data.extend_from_slice(&preroll.to_le_bytes());
        data.extend_from_slice(&flags.to_le_bytes());
        data.extend_from_slice(&[0; 12]);
        object(FILE_PROPERTIES_OBJECT, &data)
    }

    fn marker_object(entries: &[(u64, &str)]) -> Vec<u8> {
        let mut data = vec![0; 16];
        data.extend_from_slice(&u32::try_from(entries.len()).expect("small").to_le_bytes());
        data.extend_from_slice(&[0, 0, 0, 0]);
        for (time, text) in entries {
            let wide: Vec<u8> = text
                .encode_utf16()
                .chain([0])
                .flat_map(u16::to_le_bytes)
                .collect();
            data.extend_from_slice(&[0; 8]);
            data.extend_from_slice(&time.to_le_bytes());
            data.extend_from_slice(
                &u16::try_from(wide.len().saturating_add(12))
                    .expect("small")
                    .to_le_bytes(),
            );
            data.extend_from_slice(&[0; 8]);
            data.extend_from_slice(&u32::try_from(wide.len() / 2).expect("small").to_le_bytes());
            data.extend_from_slice(&wide);
        }
        object(MARKER_OBJECT, &data)
    }

    /// A file whose Header Object holds `objects`.
    fn file(objects: &[Vec<u8>]) -> Vec<u8> {
        let body: Vec<u8> = objects.concat();
        let mut out = HEADER_OBJECT.to_vec();
        out.extend_from_slice(
            &u64::try_from(body.len().saturating_add(30))
                .expect("small")
                .to_le_bytes(),
        );
        out.extend_from_slice(&u32::try_from(objects.len()).expect("small").to_le_bytes());
        out.extend_from_slice(&[1, 2]);
        out.extend(body);
        out
    }

    #[test]
    fn the_header_object_guid_is_the_windows_guid_the_text_spells() {
        // 75B22630-668E-11CF-A6D9-00AA0062CE6C
        assert_eq!(
            HEADER_OBJECT,
            [
                0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62,
                0xCE, 0x6C
            ]
        );
    }

    #[test]
    fn the_play_duration_less_the_preroll_is_the_length() {
        // 30 s of play in 100 ns units, 3 000 ms of preroll.
        let read = timeline(&file(&[properties(300_000_000, 3000, 2)])).expect("an ASF file");
        assert_eq!(read.seconds, Some(27.0));
        assert_eq!(read.frames, None);
        assert!(read.markers.is_empty());
        let read = timeline(&file(&[properties(300_000_000, 0, 2)])).expect("an ASF file");
        assert_eq!(read.seconds, Some(30.0));
    }

    #[test]
    fn a_broadcast_file_and_a_preroll_past_the_play_duration_state_no_length() {
        let live = timeline(&file(&[properties(300_000_000, 0, 1)])).expect("an ASF file");
        assert_eq!(live.seconds, None);
        let odd = timeline(&file(&[properties(10_000, 3000, 2)])).expect("an ASF file");
        assert_eq!(odd.seconds, None);
    }

    #[test]
    fn a_marker_is_named_by_its_description_and_placed_by_its_presentation_time() {
        let objects = [
            properties(300_000_000, 0, 2),
            marker_object(&[(0, "Start"), (150_000_000, "Middle")]),
        ];
        let read = timeline(&file(&objects)).expect("an ASF file");
        let names: Vec<_> = read
            .markers
            .iter()
            .map(|marker| (marker.name.as_str(), marker.seconds))
            .collect();
        assert_eq!(names, [("Start", 0.0), ("Middle", 15.0)]);
    }

    #[test]
    fn markers_beside_a_preroll_are_refused_rather_than_placed_by_a_guess() {
        let objects = [
            properties(300_000_000, 3000, 2),
            marker_object(&[(0, "Start")]),
        ];
        assert!(timeline(&file(&objects)).is_err());
    }

    #[test]
    fn a_foreign_file_a_file_without_properties_and_a_cut_one_are_refused() {
        assert!(timeline(b"RIFF\0\0\0\0AVI ").is_err());
        assert!(timeline(&file(&[marker_object(&[])])).is_err());
        let whole = file(&[properties(300_000_000, 0, 2), marker_object(&[(0, "A")])]);
        for cut in 0..whole.len() {
            assert!(
                timeline(&whole[..cut]).is_err(),
                "a file cut at {cut} was read"
            );
        }
    }

    #[test]
    fn a_marker_whose_entry_length_is_not_its_own_is_refused() {
        let mut marker = marker_object(&[(0, "A")]);
        // Offset (8) and presentation time (8) after the object's 24 + 16 + 4 + 2 + 2 bytes.
        let at = 24 + 16 + 4 + 2 + 2 + 16;
        marker[at] = marker[at].wrapping_add(2);
        assert!(timeline(&file(&[properties(1, 0, 2), marker])).is_err());
    }
}
