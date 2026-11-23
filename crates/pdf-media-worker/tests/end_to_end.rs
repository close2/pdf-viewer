//! Sounds decoded end to end through the confined worker: started at the first sound and not
//! before, cut into requests on frame boundaries, and answered with the samples §13.3 and G.711
//! define (ADR 3297).
//!
//! Every fixture is built here from Table 305's entries; no document's sound crosses into the tree.

use pdf_media_worker::sound::{self, Encoding, SoundFormat};
use pdf_media_worker::wire::MAX_RUN_BYTES;
use pdf_media_worker::{MediaError, MediaWorker};

/// The worker program Cargo built beside this test.
const WORKER: &str = env!("CARGO_BIN_EXE_pdf-media-worker");

/// A handle on the worker Cargo built.
fn worker() -> MediaWorker {
    MediaWorker::with_program(WORKER.into())
}

/// §13.3's µ-law format: "If the encoding is muLaw, R shall be 8000 samples per channel, C shall be
/// 1 channel, and B shall be 8 bits per channel."
const MU_LAW: SoundFormat = SoundFormat {
    rate: 8000,
    channels: 1,
    bits: 8,
    encoding: Encoding::MuLaw,
};

/// The worker starts at the first sound and not when the handle is made, confines itself, and
/// answers G.711 Table 2a's values; a second sound is answered by the same worker.
#[test]
fn a_mu_law_sound_is_decoded_by_a_worker_started_at_the_first_sound() {
    let mut media = worker();
    assert_eq!(media.starts(), 0, "a handle made is no worker started");
    // G.711 Table 2a: `10000000` is 8031 and `11111111` is 0; section 3.2: `00000010` is -7519.
    let sound = media
        .decode(&MU_LAW, &[0b1000_0000, 0b1111_1111, 0b0000_0010], 16)
        .expect("decoded");
    assert_eq!(sound.samples, vec![8031 << 18, 0, -7519 << 18]);
    assert_eq!((sound.rate, sound.channels), (8000, 1));
    if pdf_sandbox::lockdown::ENFORCED_BY_THIS_BUILD {
        assert!(
            sound.notes.is_empty(),
            "the worker said it ran short of its confinement: {:?}",
            sound.notes
        );
    }
    assert_eq!(media.starts(), 1);
    media
        .decode(&MU_LAW, &[0b1000_0000], 16)
        .expect("decoded again");
    assert_eq!(
        media.starts(),
        1,
        "the running worker answers the next sound"
    );
}

/// A sound longer than one request is cut on frame boundaries, and what comes back is the sound
/// decoded whole: three channels of twelve-bit samples make a frame that ends on a byte only every
/// nine bytes, so a cut anywhere else would split a sample.
#[test]
fn a_long_sound_is_cut_on_frames_and_decoded_whole() {
    let format = SoundFormat {
        rate: 11_025,
        channels: 3,
        bits: 12,
        encoding: Encoding::Signed,
    };
    let data: Vec<u8> = (0..MAX_RUN_BYTES * 2 + 1000)
        .map(|index| u8::try_from(index % 251).expect("under 251"))
        .collect();
    let mut media = worker();
    let sound = media.decode(&format, &data, usize::MAX).expect("decoded");
    let whole = sound::decode(&format, &data, usize::MAX).expect("decoded in place");
    assert_eq!(sound.samples.len(), whole.samples.len());
    assert!(sound.samples == whole.samples, "the cut changed a sample");
    assert_eq!(media.starts(), 1);
}

/// Table 305's `Raw` sixteen-bit stereo across requests: "unsigned values in the range 0 to 2B - 1",
/// so `0x8000` is silence, and the channels stay interleaved left first.
#[test]
fn raw_stereo_crosses_requests_interleaved() {
    let format = SoundFormat {
        rate: 22_050,
        channels: 2,
        bits: 16,
        encoding: Encoding::Raw,
    };
    let frames = MAX_RUN_BYTES / 4 + 3;
    let data: Vec<u8> = std::iter::repeat_n([0x80, 0x00, 0xFF, 0xFF], frames)
        .flatten()
        .collect();
    let sound = worker()
        .decode(&format, &data, usize::MAX)
        .expect("decoded");
    assert_eq!(sound.samples.len(), frames * 2);
    for pair in sound.samples.chunks(2) {
        assert_eq!(pair, [0, 0x7FFF << 16]);
    }
}

/// Refusals that need no worker start none.
#[test]
fn a_refused_sound_starts_no_worker() {
    let mut media = worker();
    let sixteen_bit_mu_law = SoundFormat { bits: 16, ..MU_LAW };
    assert!(matches!(
        media.decode(&sixteen_bit_mu_law, &[0; 4], 16),
        Err(MediaError::Format(_))
    ));
    assert!(matches!(
        media.decode(&MU_LAW, &[0; 17], 16),
        Err(MediaError::TooLarge {
            samples: 17,
            limit: 16
        })
    ));
    assert_eq!(media.starts(), 0);
}

/// A program that is not the media worker is refused at its greeting rather than believed.
#[test]
fn a_program_that_is_not_the_worker_is_refused() {
    let mut media = MediaWorker::with_program("/bin/true".into());
    match media.decode(&MU_LAW, &[0], 16) {
        Err(MediaError::Transport(_)) => {}
        other => panic!("expected a refused greeting, got {other:?}"),
    }
}

/// A RIFF WAVE file of sixteen-bit stereo PCM, with a `LIST` chunk of odd size before its `fmt `
/// chunk: the layout of Microsoft's and IBM's *Multimedia Programming Interface and Data
/// Specifications 1.0* (1991), built here.
fn wave_file(samples: &[i16]) -> Vec<u8> {
    let chunk = |out: &mut Vec<u8>, id: &[u8; 4], bytes: &[u8]| {
        out.extend_from_slice(id);
        out.extend_from_slice(&u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_le_bytes());
        out.extend_from_slice(bytes);
        if bytes.len() % 2 == 1 {
            out.push(0);
        }
    };
    let mut body = b"WAVE".to_vec();
    chunk(&mut body, b"LIST", b"INFO1");
    let mut fields = Vec::new();
    fields.extend_from_slice(&1u16.to_le_bytes()); // WAVE_FORMAT_PCM
    fields.extend_from_slice(&2u16.to_le_bytes()); // two channels
    fields.extend_from_slice(&22_050u32.to_le_bytes());
    fields.extend_from_slice(&(22_050u32 * 4).to_le_bytes());
    fields.extend_from_slice(&4u16.to_le_bytes()); // a block of two two-byte samples
    fields.extend_from_slice(&16u16.to_le_bytes());
    chunk(&mut body, b"fmt ", &fields);
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    chunk(&mut body, b"data", &data);
    let mut file = b"RIFF".to_vec();
    file.extend_from_slice(&u32::try_from(body.len()).unwrap_or(u32::MAX).to_le_bytes());
    file.extend_from_slice(&body);
    file
}

/// A WAVE file's header is read in the confined worker and its samples come back as a sound
/// object's would: sixteen-bit little-endian stereo, set in the top of each word, at the rate and
/// channels the `fmt ` chunk states (ADR 3349).
#[test]
fn a_wave_file_is_read_and_decoded_by_the_worker() {
    let mut media = worker();
    let file = wave_file(&[1, -1, i16::MAX, i16::MIN]);
    let sound = media.decode_wave(&file, 16).expect("decoded");
    assert_eq!((sound.rate, sound.channels), (22_050, 2));
    assert_eq!(
        sound.samples,
        vec![1 << 16, -1 << 16, i32::from(i16::MAX) << 16, i32::MIN]
    );
    assert_eq!(
        media.starts(),
        1,
        "one worker read the header and the samples"
    );
}

/// A file that is not RIFF WAVE, and one whose format this player does not read, are refused by
/// the worker with its sentence, and the worker keeps answering.
#[test]
fn a_wave_file_the_worker_does_not_read_is_refused_with_its_reason() {
    let mut media = worker();
    let refused = media
        .decode_wave(b"OggS and not a RIFF file", 16)
        .expect_err("not WAVE");
    assert!(
        matches!(&refused, MediaError::Refused(why) if why.contains("RIFF WAVE")),
        "{refused}"
    );
    let mut float = wave_file(&[0, 0]);
    // The format tag sits after "RIFF", the size, "WAVE", the LIST chunk (8 + 6) and the fmt
    // chunk's header: 2 is Microsoft ADPCM, which this player does not decode.
    float[12 + 14 + 8] = 2;
    let refused = media.decode_wave(&float, 16).expect_err("ADPCM");
    assert!(format!("{refused}").contains("format tag"), "{refused}");
    assert!(media.decode_wave(&wave_file(&[5, 6]), 16).is_ok());
    assert_eq!(media.starts(), 1, "a refusal is an answer, not a death");
}

/// A `WAVE_FORMAT_EXTENSIBLE` file of 32-bit IEEE floats whose `fmt ` and `data` chunks lie past
/// the first window, behind cover art larger than one: the worker walks it a window at a time, the
/// host checks each step, the floats come back at full scale, and a channel mask that is not the
/// first speakers crosses for the host to mix by (ADRs 3397, 3405).
#[test]
fn an_extensible_float_file_past_the_first_window_is_walked_and_decoded() {
    let chunk = |out: &mut Vec<u8>, id: &[u8; 4], bytes: &[u8]| {
        out.extend_from_slice(id);
        out.extend_from_slice(&u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_le_bytes());
        out.extend_from_slice(bytes);
        if bytes.len() % 2 == 1 {
            out.push(0);
        }
    };
    let mut body = b"WAVE".to_vec();
    chunk(&mut body, b"LIST", &vec![0x55; 300 << 10]);
    let mut fields = Vec::new();
    fields.extend_from_slice(&0xFFFEu16.to_le_bytes()); // WAVE_FORMAT_EXTENSIBLE
    fields.extend_from_slice(&2u16.to_le_bytes());
    fields.extend_from_slice(&48_000u32.to_le_bytes());
    fields.extend_from_slice(&(48_000u32 * 8).to_le_bytes());
    fields.extend_from_slice(&8u16.to_le_bytes());
    fields.extend_from_slice(&32u16.to_le_bytes());
    fields.extend_from_slice(&22u16.to_le_bytes()); // cbSize
    fields.extend_from_slice(&32u16.to_le_bytes()); // wValidBitsPerSample
    fields.extend_from_slice(&0x0000_0030u32.to_le_bytes()); // back left and back right
    fields.extend_from_slice(&3u32.to_le_bytes()); // KSDATAFORMAT_SUBTYPE_IEEE_FLOAT
    fields.extend_from_slice(&[
        0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
    ]);
    chunk(&mut body, b"fmt ", &fields);
    let data: Vec<u8> = [0.5f32, -0.25, 1.0, -1.0]
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    chunk(&mut body, b"data", &data);
    let mut file = b"RIFF".to_vec();
    file.extend_from_slice(&u32::try_from(body.len()).unwrap_or(u32::MAX).to_le_bytes());
    file.extend_from_slice(&body);
    let mut media = worker();
    let sound = media.decode_wave(&file, 16).expect("decoded");
    assert_eq!((sound.rate, sound.channels), (48_000, 2));
    assert_eq!(sound.samples, vec![1 << 30, -(1 << 29), i32::MAX, i32::MIN]);
    assert_eq!(sound.speakers, 0x30, "the mask crosses for the host's mix");
}
