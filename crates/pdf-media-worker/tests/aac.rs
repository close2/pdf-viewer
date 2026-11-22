//! An MP4 clip's AAC decoded end to end through the confined worker (ADRs 3381, 3382, 3383): the
//! file walked by its top-level boxes, its movie sent whole, its access units in runs to the decoder
//! the worker holds, and the result presented as the track's edit list says.
//!
//! Every fixture is built here: a tone encoded by `rusty_aac`'s own AAC-LC encoder and laid out in
//! ISO/IEC 14496-12's boxes. The expected samples are the tone that went in — what the edit list
//! promises a player presents once the encoder's priming is skipped — so the check is the edit list's
//! arithmetic and the decoder's round trip, not a copy of any decoder's output.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::trivially_copy_pass_by_ref,
    reason = "test code: a fixture built here fails loudly, its offsets are a few kilobytes, and a box type reads as a literal"
)]

use pdf_media_worker::mp4::{self, Edit, SoundTrack, presentation};
use pdf_media_worker::{MediaError, MediaWorker};

/// The worker program Cargo built beside this test.
const WORKER: &str = env!("CARGO_BIN_EXE_pdf-media-worker");

const RATE: u32 = 48_000;

/// A box: its size, its type, its payload (ISO/IEC 14496-12, 4.2).
fn bx(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = u32::try_from(payload.len() + 8)
        .expect("a small box")
        .to_be_bytes()
        .to_vec();
    bytes.extend_from_slice(kind);
    bytes.extend_from_slice(payload);
    bytes
}

/// A full box: version, flags, fields.
fn full(kind: &[u8; 4], version: u8, flags: u32, fields: &[u8]) -> Vec<u8> {
    let mut payload = flags.to_be_bytes().to_vec();
    payload[0] = version;
    payload.extend_from_slice(fields);
    bx(kind, &payload)
}

fn words(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

/// An ISO/IEC 14496-1 descriptor with a one-byte length.
fn descriptor(tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![
        tag,
        u8::try_from(payload.len()).expect("a short descriptor"),
    ];
    bytes.extend_from_slice(payload);
    bytes
}

/// A 440 Hz tone at half scale, `frames` long, the same in each of `channels`.
fn tone(frames: usize, channels: u16) -> Vec<f32> {
    (0..frames)
        .flat_map(|n| {
            #[expect(clippy::cast_precision_loss, reason = "a test tone's index")]
            let value = (n as f32 * 440.0 * std::f32::consts::TAU / RATE as f32).sin() * 0.5;
            std::iter::repeat_n(value, usize::from(channels))
        })
        .collect()
}

/// The tone encoded: its `AudioSpecificConfig`, its access units, and the encoder's priming.
fn encoded(input: &[f32], channels: u16) -> (Vec<u8>, Vec<Vec<u8>>, u32) {
    let mut encoder = rusty_aac::AacEncoder::new(rusty_aac::AacEncoderConfig::default());
    encoder.push_pcm(input, channels, RATE).expect("encoded");
    encoder.finish();
    let mut units = Vec::new();
    while let Ok(packet) = encoder.next_packet() {
        units.push(packet.data);
    }
    let config = rusty_aac::audio_specific_config_bytes(RATE, channels);
    (config, units, encoder.priming_samples())
}

/// An M4A file holding `units` under `config`, its movie after its media data, with `edits` as
/// `(segment duration, media time, rate)` entries and the movie's time scale `movie_scale`.
fn m4a(config: &[u8], units: &[Vec<u8>], channels: u16, edits: &[(u32, i32, u32)]) -> Vec<u8> {
    let ftyp = bx(b"ftyp", b"M4A \0\0\0\0M4A isom");
    let free = bx(b"free", &[0; 12]);
    let data: Vec<u8> = units.concat();
    let data_at = ftyp.len() + free.len() + 8;
    let mdat = bx(b"mdat", &data);

    let mut decoder = vec![0x40, 0x15, 0, 0, 0];
    decoder.extend(words(&[128_000, 128_000]));
    decoder.extend(descriptor(5, config));
    let mut stream = vec![0, 1, 0];
    stream.extend(descriptor(4, &decoder));
    stream.extend(descriptor(6, &[2]));
    let esds = full(b"esds", 0, 0, &descriptor(3, &stream));
    let mut entry = vec![0; 6];
    entry.extend_from_slice(&1u16.to_be_bytes());
    entry.extend_from_slice(&[0; 8]);
    entry.extend_from_slice(&channels.to_be_bytes());
    entry.extend_from_slice(&16u16.to_be_bytes());
    entry.extend_from_slice(&[0; 4]);
    entry.extend(words(&[RATE << 16]));
    entry.extend(esds);
    let mut stsd = words(&[1]);
    stsd.extend(bx(b"mp4a", &entry));
    let count = u32::try_from(units.len()).expect("few units");
    let mut sizes = words(&[0, count]);
    for unit in units {
        sizes.extend(words(&[u32::try_from(unit.len()).expect("a small unit")]));
    }
    let stbl = bx(
        b"stbl",
        &[
            full(b"stsd", 0, 0, &stsd),
            full(b"stts", 0, 0, &words(&[1, count, 1024])),
            full(b"stsc", 0, 0, &words(&[1, 1, count, 1])),
            full(b"stsz", 0, 0, &sizes),
            full(
                b"stco",
                0,
                0,
                &words(&[1, u32::try_from(data_at).expect("a small file")]),
            ),
        ]
        .concat(),
    );
    let mut references = words(&[1]);
    references.extend(full(b"url ", 0, 1, &[]));
    let minf = bx(
        b"minf",
        &[bx(b"dinf", &full(b"dref", 0, 0, &references)), stbl].concat(),
    );
    let mut handler = words(&[0]);
    handler.extend_from_slice(b"soun");
    handler.extend_from_slice(&[0; 13]);
    let mdia = bx(
        b"mdia",
        &[
            full(b"mdhd", 0, 0, &words(&[0, 0, RATE, count * 1024, 0])),
            full(b"hdlr", 0, 0, &handler),
            minf,
        ]
        .concat(),
    );
    let mut track = full(b"tkhd", 0, 1, &[0; 80]);
    if !edits.is_empty() {
        let mut list = words(&[u32::try_from(edits.len()).expect("few edits")]);
        for &(duration, time, rate) in edits {
            list.extend(words(&[duration, time.cast_unsigned(), rate]));
        }
        track.extend(bx(b"edts", &full(b"elst", 0, 0, &list)));
    }
    track.extend(mdia);
    let mvhd = full(b"mvhd", 0, 0, &words(&[0, 0, RATE, 0, 0x0001_0000]));
    let moov = bx(b"moov", &[mvhd, bx(b"trak", &track)].concat());
    [ftyp, free, mdat, moov].concat()
}

/// The root of the mean square of `a − b`, over their common length.
fn rms(a: &[f64], b: &[f64]) -> f64 {
    let sum: f64 = a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum();
    #[expect(clippy::cast_precision_loss, reason = "a test's sample count")]
    let count = a.len().min(b.len()).max(1) as f64;
    (sum / count).sqrt()
}

fn fractions(samples: &[i32]) -> Vec<f64> {
    samples
        .iter()
        .map(|&sample| f64::from(sample) / 2_147_483_648.0)
        .collect()
}

const ONE: u32 = 0x0001_0000;

/// The edit list's one entry skips the encoder's priming and keeps the input's length, so what the
/// worker answers is the tone that went in, sample for sample in time.
#[test]
fn an_m4a_tone_is_the_tone_once_its_priming_is_skipped() {
    let frames = 9_000;
    let input = tone(frames, 2);
    let (config, units, priming) = encoded(&input, 2);
    let edit = (
        u32::try_from(frames).expect("short"),
        i32::try_from(priming).expect("short"),
        ONE,
    );
    let file = m4a(&config, &units, 2, &[edit]);
    let mut media = MediaWorker::with_program(WORKER.into());
    let sound = media.decode_mp4(&file, 1 << 20).expect("decoded");
    assert_eq!((sound.rate, sound.channels), (RATE, 2));
    assert_eq!(
        sound.samples.len(),
        frames * 2,
        "the kept length, and no padding"
    );
    let input: Vec<f64> = input.iter().map(|&x| f64::from(x)).collect();
    let output = fractions(&sound.samples);
    // A lossy coder at 128 kbit/s keeps a tone within a few hundredths of full scale; the same tone
    // a priming frame late would be off by the tone's own size.
    let aligned = rms(&output, &input);
    assert!(
        aligned < 0.02,
        "the decoded tone is {aligned} RMS from the input"
    );
    let late = rms(&output[2 * 1024..], &input);
    assert!(late > 0.1, "a tone 1 024 samples late is {late} RMS away");
    if pdf_sandbox::lockdown::ENFORCED_BY_THIS_BUILD {
        assert!(sound.notes.is_empty(), "{:?}", sound.notes);
    }
}

/// With no edit list the media plays from its first decoded sample to its last: the priming and the
/// padding included, a whole number of frames.
#[test]
fn without_an_edit_list_every_decoded_frame_plays() {
    let (config, units, _) = encoded(&tone(3_000, 1), 1);
    let file = m4a(&config, &units, 1, &[]);
    let sound = MediaWorker::with_program(WORKER.into())
        .decode_mp4(&file, 1 << 20)
        .expect("decoded");
    assert_eq!(sound.samples.len(), units.len() * 1024);
}

/// A leading empty edit is silence before the media (14496-12, 8.6.6: a media time of −1).
#[test]
fn a_leading_empty_edit_is_silence_first() {
    let (config, units, priming) = encoded(&tone(4_000, 1), 1);
    let edits = [
        (480, -1, ONE),
        (4_000, i32::try_from(priming).expect("short"), ONE),
    ];
    let file = m4a(&config, &units, 1, &edits);
    let sound = MediaWorker::with_program(WORKER.into())
        .decode_mp4(&file, 1 << 20)
        .expect("decoded");
    assert_eq!(sound.samples.len(), 480 + 4_000);
    assert!(sound.samples[..480].iter().all(|&sample| sample == 0));
}

/// A list this player does not apply plays the whole media and says so.
#[test]
fn an_edit_list_of_two_stretches_is_said_and_the_media_plays_whole() {
    let (config, units, _) = encoded(&tone(3_000, 1), 1);
    let edits = [(1_000, 0, ONE), (1_000, 2_000, ONE)];
    let file = m4a(&config, &units, 1, &edits);
    let sound = MediaWorker::with_program(WORKER.into())
        .decode_mp4(&file, 1 << 20)
        .expect("decoded");
    assert_eq!(sound.samples.len(), units.len() * 1024);
    assert!(
        sound.notes.iter().any(|note| note.contains("not applied")),
        "{:?}",
        sound.notes
    );
}

/// A clip past the caller's bound is refused by its length, never cut silently — with no edit list,
/// and with one that keeps more than the bound.
#[test]
fn a_clip_past_the_bound_is_refused() {
    let (config, units, priming) = encoded(&tone(6_000, 1), 1);
    let kept = (6_000, i32::try_from(priming).expect("short"), ONE);
    for edits in [&[][..], &[kept][..]] {
        let file = m4a(&config, &units, 1, edits);
        let refused = MediaWorker::with_program(WORKER.into()).decode_mp4(&file, 2_048);
        assert!(
            matches!(refused, Err(MediaError::TooLarge { limit: 2_048, .. })),
            "{refused:?}"
        );
    }
}

/// A file with no movie, or whose boxes do not fit it, is refused with the worker's sentence.
#[test]
fn a_file_without_a_movie_or_with_a_box_past_its_end_is_refused() {
    let mut media = MediaWorker::with_program(WORKER.into());
    let no_movie = [bx(b"ftyp", b"M4A \0\0\0\0"), bx(b"mdat", &[1; 40])].concat();
    let refused = media.decode_mp4(&no_movie, 1 << 20);
    assert!(
        matches!(&refused, Err(MediaError::Refused(why)) if why.contains("no movie box")),
        "{refused:?}"
    );
    let mut cut = bx(b"moov", &[0; 40]);
    cut.truncate(20);
    let refused = media.decode_mp4(&cut, 1 << 20);
    assert!(
        matches!(refused, Err(MediaError::Refused(_))),
        "{refused:?}"
    );
}

fn track(edits: Vec<Edit>, movie_time_scale: Option<u32>) -> SoundTrack {
    SoundTrack {
        object_type: mp4::OBJECT_TYPE_MPEG4_AUDIO,
        decoder_config: Vec::new(),
        channels: 2,
        sample_rate: 24_000,
        time_scale: 24_000,
        access_units: Vec::new(),
        movie_time_scale,
        edits,
    }
}

/// The edit list's figures move from the movie's clock to the media's, and a zero duration keeps
/// the rest of the media (the documented choice).
#[test]
fn an_edit_list_s_figures_are_moved_to_the_media_s_clock() {
    let edit = |segment_duration, media_time| Edit {
        segment_duration,
        media_time,
        media_rate: 0x0001_0000,
    };
    // A movie clock of 1 000 a second: 50 ms of delay and 2 s kept, at 24 kHz.
    let shown = presentation(&track(vec![edit(50, -1), edit(2_000, 2_112)], Some(1_000)));
    assert_eq!(
        (shown.delay, shown.skip, shown.keep, shown.note),
        (1_200, 2_112, Some(48_000), None)
    );
    let rest = presentation(&track(vec![edit(0, 1_024)], None));
    assert_eq!((rest.skip, rest.keep), (1_024, None));
    let no_clock = presentation(&track(vec![edit(10, 1_024)], None));
    assert!(no_clock.note.is_some());
    let dwell = presentation(&track(
        vec![Edit {
            media_rate: 0,
            ..edit(10, 0)
        }],
        Some(1_000),
    ));
    assert!(dwell.note.is_some());
    assert_eq!(
        presentation(&track(Vec::new(), None)),
        mp4::Presentation::default()
    );
}

/// A box at the top is placed by its header: a size of 0 runs to the file's end, a size of 1 is
/// followed by a 64-bit size, `uuid` by sixteen more bytes; a box past the file is declined.
#[test]
fn a_top_level_box_is_placed_by_its_header() {
    let placed =
        mp4::top_level_box(&[0, 0, 0, 0, b'm', b'd', b'a', b't'], 100, 1_000).expect("placed");
    assert_eq!((placed.payload, placed.end), (108, 1_000));
    let mut large = vec![0, 0, 0, 1];
    large.extend_from_slice(b"mdat");
    large.extend_from_slice(&40u64.to_be_bytes());
    let placed = mp4::top_level_box(&large, 0, 1_000).expect("placed");
    assert_eq!((placed.payload, placed.end), (16, 40));
    let mut uuid = words(&[40]);
    uuid.extend_from_slice(b"uuid");
    uuid.extend_from_slice(&[7; 16]);
    let placed = mp4::top_level_box(&uuid, 0, 1_000).expect("placed");
    assert_eq!(placed.payload, 24);
    assert!(mp4::top_level_box(&words(&[2_000, 0]), 0, 1_000).is_err());
    assert!(mp4::top_level_box(&words(&[4, 0]), 0, 1_000).is_err());
    assert!(mp4::top_level_box(&[0, 0], 0, 1_000).is_err());
}
