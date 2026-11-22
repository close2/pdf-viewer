//! `pdf_media_worker::mp4` over files built here from ISO/IEC 14496-12's box layout (4.2, 8.2–8.7)
//! and ISO/IEC 14496-1's descriptors (7.2.6), each test stating the one fact it varies (ADR 3369).
//!
//! Neither standard is held in `doc/`; the layouts below are the module comment's paraphrase, and
//! the expected values follow from it by arithmetic. The last test reads the held list of
//! ISO/IEC 14496-26's conformance streams, which are fetched and never committed.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture built here fails loudly, and its offsets are a few hundred bytes"
)]

use std::fmt::Write as _;

use pdf_media_worker::mp4::{
    AccessUnit, MAX_ACCESS_UNITS, MAX_BOXES, OBJECT_TYPE_MPEG4_AUDIO, SoundTrack, sound_track,
};

/// A box: a 32-bit size, the type, the payload (4.2).
fn bx(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let size = u32::try_from(payload.len() + 8).expect("a test box is small");
    let mut out = size.to_be_bytes().to_vec();
    out.extend_from_slice(&kind);
    out.extend_from_slice(payload);
    out
}

/// A full box: version, flags, then the fields (4.2).
fn full(kind: [u8; 4], version: u8, flags: u32, fields: &[u8]) -> Vec<u8> {
    let mut payload = vec![version];
    payload.extend_from_slice(&flags.to_be_bytes()[1..]);
    payload.extend_from_slice(fields);
    bx(kind, &payload)
}

fn words(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

/// A descriptor: tag, a one-byte length, the payload (14496-1, 8.3.3).
fn descriptor(tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![
        tag,
        u8::try_from(payload.len()).expect("a short descriptor"),
    ];
    out.extend_from_slice(payload);
    out
}

/// AAC-LC at 48 kHz, two channels: object type 2, frequency index 3, channel configuration 2.
const CONFIG: [u8; 2] = [0x11, 0x90];

/// An `esds` box carrying `config` as MPEG-4 audio (14496-14, 5.6).
fn esds(es_flags: u8, es_extra: &[u8], stream_type_byte: u8, config: Option<&[u8]>) -> Vec<u8> {
    let mut decoder = vec![OBJECT_TYPE_MPEG4_AUDIO, stream_type_byte];
    decoder.extend_from_slice(&[0; 11]);
    if let Some(config) = config {
        decoder.extend(descriptor(5, config));
    }
    let mut stream = vec![0, 1, es_flags];
    stream.extend_from_slice(es_extra);
    stream.extend(descriptor(4, &decoder));
    full(*b"esds", 0, 0, &descriptor(3, &stream))
}

/// An audio sample entry of type `kind` and `version` with `children` after its fields (8.5.2).
fn entry(kind: [u8; 4], version: u16, data_reference: u16, children: &[u8]) -> Vec<u8> {
    let mut payload = vec![0; 6];
    payload.extend_from_slice(&data_reference.to_be_bytes());
    payload.extend_from_slice(&version.to_be_bytes());
    payload.extend_from_slice(&[0; 6]);
    payload.extend_from_slice(&2u16.to_be_bytes());
    payload.extend_from_slice(&16u16.to_be_bytes());
    payload.extend_from_slice(&[0; 4]);
    payload.extend_from_slice(&(48_000u32 << 16).to_be_bytes());
    payload.extend_from_slice(children);
    bx(kind, &payload)
}

/// What a test varies; every field's default is the ordinary AAC file.
#[derive(Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is one fact a test varies, and the tests name them by field"
)]
struct Spec {
    enabled: bool,
    handler: [u8; 4],
    entry: Vec<u8>,
    self_contained: bool,
    stts: Vec<u8>,
    stsc: Vec<u8>,
    sizes: Vec<u8>,
    /// Chunk offsets relative to the `mdat` payload; `co64` when `wide`.
    chunks: Vec<u64>,
    wide: bool,
    extra_movie: Vec<u8>,
    /// Puts `mdat` before `moov`.
    data_first: bool,
}

impl Default for Spec {
    fn default() -> Self {
        Spec {
            enabled: true,
            handler: *b"soun",
            entry: entry(*b"mp4a", 0, 1, &esds(0, &[], 0x15, Some(&CONFIG))),
            self_contained: true,
            stts: full(*b"stts", 0, 0, &words(&[1, 3, 1024])),
            // Chunk 1 holds two samples, chunk 2 one.
            stsc: full(*b"stsc", 0, 0, &words(&[2, 1, 2, 1, 2, 1, 1])),
            sizes: full(*b"stsz", 0, 0, &words(&[0, 3, 10, 20, 30])),
            chunks: vec![0, 40],
            wide: false,
            extra_movie: Vec::new(),
            data_first: false,
        }
    }
}

/// The `mdat` payload's bytes: 100 of them, enough for every table here.
const DATA: usize = 100;

fn track(spec: &Spec, data_at: u64) -> Vec<u8> {
    let tkhd = full(*b"tkhd", 0, u32::from(spec.enabled), &[0; 80]);
    let mdhd = full(*b"mdhd", 0, 0, &words(&[0, 0, 48_000, 3072, 0]));
    let mut handler = words(&[0]);
    handler.extend_from_slice(&spec.handler);
    handler.extend_from_slice(&[0; 13]);
    let hdlr = full(*b"hdlr", 0, 0, &handler);
    let url = full(*b"url ", 0, u32::from(spec.self_contained), &[]);
    let mut references = words(&[1]);
    references.extend(url);
    let dinf = bx(*b"dinf", &full(*b"dref", 0, 0, &references));
    let mut descriptions = words(&[1]);
    descriptions.extend_from_slice(&spec.entry);
    let stsd = full(*b"stsd", 0, 0, &descriptions);
    let offsets = if spec.wide {
        let mut table = words(&[u32::try_from(spec.chunks.len()).expect("few chunks")]);
        for chunk in &spec.chunks {
            table.extend_from_slice(&(data_at + chunk).to_be_bytes());
        }
        full(*b"co64", 0, 0, &table)
    } else {
        let mut table = vec![u32::try_from(spec.chunks.len()).expect("few chunks")];
        table.extend(
            spec.chunks
                .iter()
                .map(|chunk| u32::try_from(data_at + chunk).expect("a small file")),
        );
        full(*b"stco", 0, 0, &words(&table))
    };
    let stbl = bx(
        *b"stbl",
        &[
            stsd,
            spec.stts.clone(),
            spec.stsc.clone(),
            spec.sizes.clone(),
            offsets,
        ]
        .concat(),
    );
    let minf = bx(*b"minf", &[dinf, stbl].concat());
    let mdia = bx(*b"mdia", &[mdhd, hdlr, minf].concat());
    bx(*b"trak", &[tkhd, mdia].concat())
}

fn file_with_tracks(spec: &Spec, others: &[Vec<u8>]) -> Vec<u8> {
    let ftyp = bx(*b"ftyp", b"M4A \0\0\0\0M4A isom");
    let mdat = bx(*b"mdat", &[0xA5; DATA]);
    let movie_at = |data_at: u64| {
        let mut payload: Vec<u8> = others.concat();
        payload.extend(track(spec, data_at));
        payload.extend_from_slice(&spec.extra_movie);
        bx(*b"moov", &payload)
    };
    if spec.data_first {
        let data_at = (ftyp.len() + 8) as u64;
        [ftyp, mdat, movie_at(data_at)].concat()
    } else {
        // The movie's length does not depend on the offsets' values, so measure it once.
        let length = movie_at(0).len();
        let data_at = (ftyp.len() + length + 8) as u64;
        [ftyp, movie_at(data_at), mdat].concat()
    }
}

fn file(spec: &Spec) -> Vec<u8> {
    file_with_tracks(spec, &[])
}

fn read(spec: &Spec) -> Result<SoundTrack, String> {
    sound_track(&file(spec))
}

/// Where `mdat`'s payload begins in `file`.
fn data_at(file: &[u8]) -> u64 {
    let at = file
        .windows(4)
        .rposition(|window| window == b"mdat")
        .expect("the file has an mdat");
    (at + 4) as u64
}

fn units(at: u64, table: &[(u64, u32)]) -> Vec<AccessUnit> {
    table
        .iter()
        .map(|&(offset, size)| AccessUnit {
            offset: at + offset,
            size,
            duration: 1024,
        })
        .collect()
}

#[test]
fn an_aac_track_is_its_configuration_and_its_access_units_by_chunk() {
    // 8.7.4: chunk 1 holds two samples, chunk 2 the third; 8.7.3 gives each its size, and a
    // chunk's samples follow one another from the chunk's offset.
    let bytes = file(&Spec::default());
    let track = sound_track(&bytes).expect("the ordinary file reads");
    assert_eq!(track.object_type, OBJECT_TYPE_MPEG4_AUDIO);
    assert_eq!(track.decoder_config, CONFIG);
    assert_eq!(
        (track.channels, track.sample_rate, track.time_scale),
        (2, 48_000, 48_000)
    );
    let at = data_at(&bytes);
    assert_eq!(
        track.access_units,
        units(at, &[(0, 10), (10, 20), (40, 30)])
    );
}

#[test]
fn a_movie_after_its_media_data_reads_the_same() {
    let spec = Spec {
        data_first: true,
        ..Spec::default()
    };
    let bytes = file(&spec);
    let track = sound_track(&bytes).expect("a movie after mdat reads");
    let at = data_at(&bytes);
    assert_eq!(
        track.access_units,
        units(at, &[(0, 10), (10, 20), (40, 30)])
    );
}

#[test]
fn sixty_four_bit_offsets_and_compact_sizes_give_the_same_table() {
    // 8.7.5's co64 and 8.7.3.3's stz2 with eight-bit and four-bit fields.
    for (bits, packed) in [(8u8, vec![10u8, 20, 30]), (4, vec![0x9e, 0xf0])] {
        let mut fields = vec![0, 0, 0, bits];
        fields.extend(words(&[3]));
        fields.extend(packed);
        let spec = Spec {
            wide: true,
            sizes: full(*b"stz2", 0, 0, &fields),
            ..Spec::default()
        };
        let bytes = file(&spec);
        let track = sound_track(&bytes).expect("co64 and stz2 read");
        let at = data_at(&bytes);
        let expected = if bits == 8 {
            units(at, &[(0, 10), (10, 20), (40, 30)])
        } else {
            units(at, &[(0, 9), (9, 14), (40, 15)])
        };
        assert_eq!(track.access_units, expected, "{bits}-bit fields");
    }
}

#[test]
fn one_size_for_every_sample_is_read() {
    let spec = Spec {
        sizes: full(*b"stsz", 0, 0, &words(&[12, 3])),
        ..Spec::default()
    };
    let bytes = file(&spec);
    let at = data_at(&bytes);
    let track = sound_track(&bytes).expect("a constant size reads");
    assert_eq!(
        track.access_units,
        units(at, &[(0, 12), (12, 12), (40, 12)])
    );
}

#[test]
fn durations_follow_the_runs_of_stts() {
    let spec = Spec {
        stts: full(*b"stts", 0, 0, &words(&[3, 1, 1024, 0, 7, 2, 960])),
        ..Spec::default()
    };
    let track = read(&spec).expect("three runs read, the empty one skipped");
    let durations: Vec<u32> = track
        .access_units
        .iter()
        .map(|unit| unit.duration)
        .collect();
    assert_eq!(durations, [1024, 960, 960]);
}

#[test]
fn a_descriptor_s_optional_fields_are_stepped_over() {
    // 7.2.6.5: stream dependence (two bytes), a URL (a length and its bytes), an OCR stream (two).
    let spec = Spec {
        entry: entry(
            *b"mp4a",
            0,
            1,
            &esds(
                0xE0,
                &[0, 9, 3, b'a', b'b', b'c', 0, 4],
                0x15,
                Some(&CONFIG),
            ),
        ),
        ..Spec::default()
    };
    assert_eq!(
        read(&spec)
            .expect("the flags' fields are skipped")
            .decoder_config,
        CONFIG
    );
}

#[test]
fn the_first_enabled_sound_track_is_the_one_read() {
    // 8.3.2: a disabled track is treated as though it were not present; a picture track is not
    // sound.
    let disabled = track(
        &Spec {
            enabled: false,
            stts: full(*b"stts", 0, 0, &words(&[1, 3, 1])),
            ..Spec::default()
        },
        0,
    );
    let picture = track(
        &Spec {
            handler: *b"vide",
            ..Spec::default()
        },
        0,
    );
    let bytes = file_with_tracks(&Spec::default(), &[disabled, picture]);
    let track = sound_track(&bytes).expect("the third track is read");
    assert!(track.access_units.iter().all(|unit| unit.duration == 1024));
}

#[test]
fn a_large_size_and_a_size_of_zero_are_box_lengths() {
    // 4.2: size 1 takes a 64-bit size after the type; size 0 runs to the end of the file.
    let ordinary = file(&Spec::default());
    let ftyp = bx(*b"ftyp", b"M4A \0\0\0\0M4A isom");
    let mut free = 1u32.to_be_bytes().to_vec();
    free.extend_from_slice(b"free");
    free.extend_from_slice(&20u64.to_be_bytes());
    free.extend_from_slice(&[0; 4]);
    let mut bytes = ftyp.clone();
    bytes.extend_from_slice(&free);
    bytes.extend_from_slice(&ordinary[ftyp.len()..]);
    // The offsets now point twenty bytes early; the table's sizes are unchanged.
    let at = data_at(&bytes) - 20;
    let mdat = bytes.len() - DATA - 8;
    bytes[mdat..mdat + 4].copy_from_slice(&[0; 4]);
    let track = sound_track(&bytes).expect("a large size and a zero size read");
    assert_eq!(
        track.access_units,
        units(at, &[(0, 10), (10, 20), (40, 30)])
    );
}

fn declined(spec: &Spec, words_in_sentence: &str) {
    let sentence = read(spec).expect_err("the file is declined");
    assert!(
        sentence.contains(words_in_sentence),
        "{sentence:?} does not say {words_in_sentence:?}"
    );
}

#[test]
fn a_file_without_one_movie_is_declined() {
    assert!(
        sound_track(&bx(*b"mdat", &[0; 8]))
            .unwrap_err()
            .contains("no movie box")
    );
    let once = file(&Spec::default());
    let movie = bx(*b"moov", &[]);
    assert!(
        sound_track(&[once, movie].concat())
            .unwrap_err()
            .contains("two movie boxes")
    );
}

#[test]
fn a_fragmented_file_is_declined() {
    let spec = Spec {
        extra_movie: bx(*b"mvex", &[]),
        ..Spec::default()
    };
    declined(&spec, "fragmented");
}

#[test]
fn a_file_with_no_enabled_sound_track_is_declined() {
    declined(
        &Spec {
            enabled: false,
            ..Spec::default()
        },
        "no enabled sound track",
    );
    declined(
        &Spec {
            handler: *b"vide",
            ..Spec::default()
        },
        "no enabled sound track",
    );
}

#[test]
fn a_sample_entry_this_reader_does_not_read_is_named() {
    let descriptor = esds(0, &[], 0x15, Some(&CONFIG));
    let encrypted = Spec {
        entry: entry(*b"enca", 0, 1, &descriptor),
        ..Spec::default()
    };
    declined(&encrypted, "coded as enca");
    let version_one = Spec {
        entry: entry(*b"mp4a", 1, 1, &descriptor),
        ..Spec::default()
    };
    declined(&version_one, "version 1");
    let not_audio = Spec {
        entry: entry(*b"mp4a", 0, 1, &esds(0, &[], 0x11, Some(&CONFIG))),
        ..Spec::default()
    };
    declined(&not_audio, "not audio");
}

#[test]
fn a_descriptor_with_no_specific_info_carries_none() {
    let spec = Spec {
        entry: entry(*b"mp4a", 0, 1, &esds(0, &[], 0x15, None)),
        ..Spec::default()
    };
    let track = read(&spec).expect("no DecoderSpecificInfo reads");
    assert_eq!(track.decoder_config, Vec::<u8>::new());
}

#[test]
fn media_data_in_another_file_is_declined() {
    declined(
        &Spec {
            self_contained: false,
            ..Spec::default()
        },
        "in another file",
    );
    declined(
        &Spec {
            entry: entry(*b"mp4a", 0, 2, &esds(0, &[], 0x15, Some(&CONFIG))),
            ..Spec::default()
        },
        "data reference index of 2",
    );
}

#[test]
fn a_sample_table_that_disagrees_with_itself_is_declined() {
    let cases = [
        (
            Spec {
                stsc: full(*b"stsc", 0, 0, &words(&[1, 2, 2, 1])),
                ..Spec::default()
            },
            "does not begin at chunk 1",
        ),
        (
            Spec {
                stsc: full(*b"stsc", 0, 0, &words(&[2, 1, 2, 1, 1, 1, 1])),
                ..Spec::default()
            },
            "does not begin at chunk 1 and increase",
        ),
        (
            Spec {
                stsc: full(*b"stsc", 0, 0, &words(&[1, 1, 2, 2])),
                ..Spec::default()
            },
            "description 2",
        ),
        (
            Spec {
                stts: full(*b"stts", 0, 0, &words(&[1, 2, 1024])),
                ..Spec::default()
            },
            "fewer samples",
        ),
        (
            Spec {
                stts: full(*b"stts", 0, 0, &words(&[1, 4, 1024])),
                ..Spec::default()
            },
            "more samples",
        ),
        (
            Spec {
                stsc: full(*b"stsc", 0, 0, &words(&[1, 1, 2, 1])),
                ..Spec::default()
            },
            "more samples than the 3",
        ),
        (
            Spec {
                stsc: full(*b"stsc", 0, 0, &words(&[1, 1, 1, 1])),
                ..Spec::default()
            },
            "hold 2 samples where the sizes count 3",
        ),
        (
            Spec {
                stsc: full(*b"stsc", 0, 0, &words(&[2, 1, 1, 1, 3, 1, 1])),
                ..Spec::default()
            },
            "past the 2 chunks",
        ),
    ];
    for (spec, words_in_sentence) in cases {
        declined(&spec, words_in_sentence);
    }
}

#[test]
fn an_access_unit_past_the_file_is_declined() {
    let spec = Spec {
        chunks: vec![0, 90],
        ..Spec::default()
    };
    declined(&spec, "past the file's");
}

#[test]
fn a_count_the_box_cannot_hold_is_declined_before_anything_is_allocated() {
    // Each count is the document's claim; the bytes after it are the fact.
    let cases = [
        (
            Spec {
                sizes: full(*b"stsz", 0, 0, &words(&[0, 0x007F_FFFF, 10])),
                ..Spec::default()
            },
            "states 8388607 sizes",
        ),
        (
            Spec {
                sizes: full(*b"stsz", 0, 0, &words(&[0, u32::MAX])),
                ..Spec::default()
            },
            "a table of 4294967295 entries",
        ),
        (
            Spec {
                sizes: full(*b"stsz", 0, 0, &words(&[1, 0x0010_0000])),
                ..Spec::default()
            },
            "past the file's",
        ),
        (
            Spec {
                stts: full(*b"stts", 0, 0, &words(&[0x0010_0000, 3, 1024])),
                ..Spec::default()
            },
            "stts box that states 1048576 entries",
        ),
        (
            Spec {
                sizes: full(*b"stz2", 0, 0, &[&[0, 0, 0, 12][..], &words(&[3])].concat()),
                ..Spec::default()
            },
            "fields of 12 bits",
        ),
    ];
    for (spec, words_in_sentence) in cases {
        declined(&spec, words_in_sentence);
    }
}

#[test]
fn a_box_larger_than_what_holds_it_is_declined() {
    let mut bytes = file(&Spec::default());
    let movie = bytes
        .windows(4)
        .position(|window| window == b"moov")
        .expect("moov")
        - 4;
    bytes[movie..movie + 4].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(sound_track(&bytes).unwrap_err().contains("moov box of"));
}

#[test]
fn a_walk_past_its_box_budget_is_declined() {
    let free = bx(*b"free", &[]);
    let bytes: Vec<u8> = std::iter::repeat_n(free, MAX_BOXES + 1).flatten().collect();
    assert!(
        sound_track(&bytes)
            .unwrap_err()
            .contains("boxes this reader walks")
    );
}

#[test]
fn the_access_unit_bound_is_about_six_hours_of_aac() {
    // 1 024 samples a unit at 48 kHz: the bound is past five hours and under seven.
    let seconds = MAX_ACCESS_UNITS * 1024 / 48_000;
    assert!((5 * 3600..7 * 3600).contains(&seconds), "{seconds} s");
}

/// Reads every `.mp4` under `PDF_MEDIA_MP4_STREAMS` — the held list of ISO/IEC 14496-26's
/// `compressedMp4` streams (ADR 3369 names them and where they are fetched from) — and prints
/// each one's table summary for comparison. Each is MPEG-4 audio whose access units lie in the
/// file, in order and without overlap.
// not a gate: the held streams are fetched from ISO's bitstream directory and never committed, so no
// gate line can find them; step 9's AAC conformance gate over the same list is where they become one
// (ADR 3369).
#[test]
#[ignore = "reads the held conformance streams named by PDF_MEDIA_MP4_STREAMS, which are not committed"]
fn the_held_conformance_streams_read() {
    let directory = std::env::var_os("PDF_MEDIA_MP4_STREAMS")
        .expect("PDF_MEDIA_MP4_STREAMS names the held streams' directory");
    let mut names: Vec<_> = std::fs::read_dir(&directory)
        .expect("the directory reads")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "mp4"))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no stream in {directory:?}");
    for path in names {
        let bytes = std::fs::read(&path).expect("the stream reads");
        let track =
            sound_track(&bytes).unwrap_or_else(|sentence| panic!("{}: {sentence}", path.display()));
        assert_eq!(
            track.object_type,
            OBJECT_TYPE_MPEG4_AUDIO,
            "{}",
            path.display()
        );
        assert!(!track.decoder_config.is_empty(), "{}", path.display());
        let ordered = track
            .access_units
            .windows(2)
            .all(|pair| pair[0].offset + u64::from(pair[0].size) <= pair[1].offset);
        assert!(ordered, "{}: access units overlap", path.display());
        let bytes_total: u64 = track
            .access_units
            .iter()
            .map(|unit| u64::from(unit.size))
            .sum();
        let duration: u64 = track
            .access_units
            .iter()
            .map(|unit| u64::from(unit.duration))
            .sum();
        let mut config = String::new();
        for byte in &track.decoder_config {
            write!(config, "{byte:02x}").expect("a String takes every write");
        }
        println!(
            "{} units={} bytes={bytes_total} first={} duration={duration} scale={} channels={} rate={} config={config}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("?"),
            track.access_units.len(),
            track.access_units.first().map_or(0, |unit| unit.offset),
            track.time_scale,
            track.channels,
            track.sample_rate,
        );
    }
}
