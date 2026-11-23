//! ISO/IEC 14496-26's AAC conformance streams decoded through the confined worker and compared with
//! the suite's reference waveforms (ADR 3382).
//!
//! The streams and the references are ISO's, fetched from its Publicly Available Standards and never
//! committed: `PDF_MEDIA_MP4_STREAMS` names the directory of `compressedMp4/` files and
//! `PDF_MEDIA_AAC_REFERENCES` the directory of `referencesWav/` files. The references are 24-bit
//! PCM; a stream with several is referenced one output channel a file (`_f00`, `_b00`, `_s00`,
//! `_l00`), and each such file is compared with the decoded channel nearest it, because the suite's
//! names label a speaker and not a decoder's output order (al07 decodes as L, R, C, LFE, Ls, Rs,
//! and its references are named f01, f02, f00, l00, b00, b01).
//!
//! **The tolerance is not the standard's.** The suite's readme files state no criterion for
//! comparing a decoder's output with a reference, and ISO/IEC 14496-4 and -26, which would, are not
//! held (Q988's kind). The verdict printed is RFC 0012 section 3.3's bar — the largest difference at
//! most one least significant bit of sixteen — named as that RFC's choice, and every figure is
//! printed so that the standard's criterion can be applied once its text is held. Both sides are
//! aligned by the lag in ±4 096 samples that minimises their difference over 16 384 frames from
//! where the reference first carries signal, and the lag is printed: the references may or may not carry the encoder's priming, and
//! the walk does not assume either.
// not a gate: it reads streams the tree does not hold, prints a verdict a person reads, and
// asserts only that every stream decodes; the standard's criterion is not held to assert against.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a walk over held files a person runs: a missing file fails loudly, and its sample counts and lags are a few million at most"
)]

use std::path::{Path, PathBuf};

use pdf_media_worker::MediaWorker;

const WORKER: &str = env!("CARGO_BIN_EXE_pdf-media-worker");

/// One 16-bit least significant bit, on the worker's top-aligned scale.
const LSB16: f64 = 65_536.0;

/// The reference files for `stream`, by the suite's naming.
fn references(directory: &Path, stream: &str) -> Vec<PathBuf> {
    // The two HE-AAC streams have no reference under their own names; the suite's `add-opt/`
    // holds these candidates, each compared and printed.
    let stems: Vec<String> = match stream {
        "al_sbr_cm_48_2" => vec!["al_sbr_hq_cm_48_2".into(), "al_sbr_lp_cm_48_2".into()],
        "al_sbr_ps_01_new" => vec!["al_sbr_ps_01_bl".into(), "al_sbr_ps_01_ur".into()],
        _ => vec![stream.to_owned()],
    };
    let mut found = Vec::new();
    for stem in stems {
        let whole = directory.join(format!("{stem}.wav"));
        if whole.exists() {
            found.push(whole);
            continue;
        }
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        let mut channels: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .and_then(|name| name.strip_prefix(&format!("{stem}_")))
                    .is_some_and(|suffix| {
                        suffix.len() == 3
                            && matches!(suffix.as_bytes()[0], b'f' | b'b' | b's' | b'l')
                            && suffix[1..].bytes().all(|byte| byte.is_ascii_digit())
                    })
            })
            .collect();
        channels.sort();
        found.extend(channels);
    }
    found
}

/// A PCM WAVE file's channels and its samples, top-aligned in `i32`s: the `fmt ` chunk's channel
/// count and width, and the `data` chunk.
fn wave(path: &Path) -> (usize, Vec<i32>) {
    let bytes = std::fs::read(path).expect("a reference");
    let mut at = 12;
    let (mut channels, mut bits) = (0usize, 0usize);
    while at + 8 <= bytes.len() {
        let kind = &bytes[at..at + 4];
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().expect("four")) as usize;
        let body = &bytes[at + 8..(at + 8 + size).min(bytes.len())];
        if kind == b"fmt " {
            channels = usize::from(u16::from_le_bytes([body[2], body[3]]));
            bits = usize::from(u16::from_le_bytes([body[14], body[15]]));
        } else if kind == b"data" {
            let width = bits / 8;
            let samples = body
                .chunks_exact(width)
                .map(|sample| {
                    let mut word = [0u8; 4];
                    word[4 - width..].copy_from_slice(sample);
                    i32::from_le_bytes(word)
                })
                .collect();
            return (channels, samples);
        }
        at += 8 + size + (size & 1);
    }
    panic!("{} holds no data chunk", path.display());
}

/// Channel `channel` of interleaved `samples` in `channels`.
fn channel(samples: &[i32], channels: usize, channel: usize) -> Vec<f64> {
    samples
        .iter()
        .skip(channel)
        .step_by(channels)
        .map(|&sample| f64::from(sample))
        .collect()
}

/// The largest difference and the RMS difference, in 16-bit LSB, of `a` against `b` delayed by
/// `lag`, over their overlap, and the overlap's length.
fn compared(a: &[f64], b: &[f64], lag: i64, limit: usize) -> (f64, f64, usize) {
    let (a, b) = if lag >= 0 {
        (a.get(lag as usize..).unwrap_or_default(), b)
    } else {
        (a, b.get(lag.unsigned_abs() as usize..).unwrap_or_default())
    };
    let length = a.len().min(b.len()).min(limit);
    let mut largest = 0f64;
    let mut sum = 0f64;
    for (x, y) in a.iter().zip(b).take(length) {
        let difference = (x - y).abs() / LSB16;
        largest = largest.max(difference);
        sum += difference * difference;
    }
    (largest, (sum / length.max(1) as f64).sqrt(), length)
}

/// The RMS difference of `decoded` against `reference` delayed by `lag`, over the 16 384
/// reference frames from `from`.
fn windowed(decoded: &[f64], reference: &[f64], lag: i64, from: usize) -> f64 {
    let mut sum = 0f64;
    let mut count = 0usize;
    for index in from..from + 16_384 {
        let (Some(y), Some(x)) = (
            reference.get(index),
            index
                .checked_add_signed(lag as isize)
                .and_then(|at| decoded.get(at)),
        ) else {
            continue;
        };
        let difference = (x - y) / LSB16;
        sum += difference * difference;
        count += 1;
    }
    if count == 0 {
        f64::INFINITY
    } else {
        (sum / count as f64).sqrt()
    }
}

/// Where the reference first carries signal — a sample past sixteen 16-bit LSB — so that a lag
/// and a pairing are chosen where a wrong one shows: over digital silence every lag and every
/// channel scores nothing, and the first of them wins (al07's four quiet-opening channels were
/// paired and aligned that way, ADR 3400).
fn signal_start(reference: &[f64]) -> usize {
    reference
        .iter()
        .position(|sample| sample.abs() > 16.0 * LSB16)
        .unwrap_or(0)
}

/// The lag in ±4 096 minimising the RMS difference over 16 384 frames from where the reference
/// first carries signal.
fn best_lag(decoded: &[f64], reference: &[f64]) -> i64 {
    let from = signal_start(reference);
    (-4096..=4096)
        .min_by(|&left, &right| {
            let l = windowed(decoded, reference, left, from);
            let r = windowed(decoded, reference, right, from);
            l.total_cmp(&r)
        })
        .unwrap_or(0)
}

#[test]
#[ignore = "reads ISO/IEC 14496-26's streams and references from PDF_MEDIA_MP4_STREAMS and PDF_MEDIA_AAC_REFERENCES"]
fn the_held_conformance_streams_decode_against_their_references() {
    let streams = PathBuf::from(std::env::var("PDF_MEDIA_MP4_STREAMS").expect("the streams"));
    let referenced =
        PathBuf::from(std::env::var("PDF_MEDIA_AAC_REFERENCES").expect("the references"));
    let mut names: Vec<String> = std::fs::read_dir(&streams)
        .expect("a directory")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            name.strip_suffix(".mp4").map(str::to_owned)
        })
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no stream in {}", streams.display());
    let mut media = MediaWorker::with_program(WORKER.into());
    let mut failed = Vec::new();
    for name in &names {
        let file = std::fs::read(streams.join(format!("{name}.mp4"))).expect("a stream");
        let sound = match media.decode_mp4(&file, 1 << 26) {
            Ok(sound) => sound,
            Err(error) => {
                println!("{name}: NOT DECODED: {error}");
                failed.push(name.clone());
                continue;
            }
        };
        let decoded_channels = usize::from(sound.channels);
        let frames = sound.samples.len() / decoded_channels.max(1);
        println!(
            "{name}: decoded {} Hz, {} channel(s), {frames} frames; notes {:?}",
            sound.rate, sound.channels, sound.notes
        );
        let files = references(&referenced, name);
        if files.is_empty() {
            println!("  no reference");
        }
        for path in files {
            let (ref_channels, samples) = wave(&path);
            let label = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("?");
            for reference_channel in 0..ref_channels {
                let reference = channel(&samples, ref_channels, reference_channel);
                // A whole-program reference pairs channel for channel; a one-channel file is
                // paired with the decoded channel nearest it.
                let candidates: Vec<usize> = if ref_channels == decoded_channels {
                    vec![reference_channel]
                } else {
                    (0..decoded_channels).collect()
                };
                // The nearest channel is chosen at whole-frame lags where the reference first
                // carries signal, and the lag searched sample by sample for it alone.
                let from = signal_start(&reference);
                let nearest = candidates.into_iter().min_by(|&left, &right| {
                    let near = |decoded_channel| {
                        let decoded = channel(&sound.samples, decoded_channels, decoded_channel);
                        [-2048, -1024, 0, 1024, 2048]
                            .into_iter()
                            .map(|lag| windowed(&decoded, &reference, lag, from))
                            .fold(f64::INFINITY, f64::min)
                    };
                    near(left).total_cmp(&near(right))
                });
                let verdict = nearest.map(|decoded_channel| {
                    let decoded = channel(&sound.samples, decoded_channels, decoded_channel);
                    let lag = best_lag(&decoded, &reference);
                    let (largest, rms, length) = compared(&decoded, &reference, lag, usize::MAX);
                    (largest, rms, length, lag, decoded_channel)
                });
                if let Some((largest, rms, length, lag, decoded_channel)) = verdict {
                    let bar = if largest <= 1.0 { "WITHIN1" } else { "OFF" };
                    println!(
                        "  {label} ch {reference_channel} ({} frames) vs decoded ch \
                         {decoded_channel}: lag {lag}, max {largest:.3} LSB16, RMS {rms:.4} \
                         LSB16 over {length}: {bar}",
                        reference.len()
                    );
                }
            }
        }
    }
    assert!(failed.is_empty(), "streams not decoded: {failed:?}");
}
