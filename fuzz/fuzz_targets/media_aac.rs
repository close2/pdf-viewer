//! Fuzzes the media worker's MP4 clip path end to end in one process (ADRs 3381, 3382): the file's
//! top-level boxes placed one request at a time, its movie read, and its AAC access units decoded by
//! `rusty_aac`'s scalar build in runs — each request answered by `pdf_media_worker::Worker` exactly as
//! the confined worker answers it, and each reply read by the host's own checks.
//!
//! The input is an MP4 file an untrusted document embedded, and the access units are its AAC stream:
//! the bytes the decoder reads are the fuzzer's. Beyond never panicking — overflow checks stay on in
//! this profile — three properties:
//!
//! - **The host accepts every honest answer.** Each reply the worker gives is read back by the
//!   host's `decode_boxed`, `decode_track` and `decode_pcm` against the request it answers, so a
//!   worker whose honest answer the host refused would be caught here rather than as a sound that
//!   never plays.
//! - **A run decodes to at most `MAX_SAMPLES_PER_REQUEST` samples**, the bound the worker's address
//!   space is sized against.
//! - **The walk moves on**: every box ends after it begins, so the host's walk ends.
//!
//! Seeded by `fuzz/seed_media_aac.py` from ISO/IEC 14496-26's streams where they are held, which
//! are never committed.

#![no_main]
#![expect(
    clippy::expect_used,
    reason = "a fuzz target states its properties by failing: `expect` is how a violated one reaches libFuzzer, and each message names the property"
)]

use libfuzzer_sys::fuzz_target;
use pdf_media_worker::Worker;
use pdf_media_worker::mp4::TOP_HEADER_BYTES;
use pdf_media_worker::wire::{
    FRAME_BOX, FRAME_BOXED, FRAME_MOVIE, FRAME_PCM, FRAME_TRACK, FRAME_UNITS, MAX_MOVIE_BYTES,
    MAX_SAMPLES_PER_REQUEST, MAX_UNITS_PER_RUN, decode_boxed, decode_pcm, decode_track,
    encode_box_request, encode_movie_request, encode_units,
};

/// The most runs one input decodes: enough to carry a stream's state across several requests,
/// few enough that an input is a fraction of a second.
const MAX_RUNS: usize = 8;

fuzz_target!(|file: &[u8]| {
    let mut worker = Worker::default();
    let total = u64::try_from(file.len()).unwrap_or(u64::MAX);
    let mut at = 0u64;
    let mut movie = None;
    for _ in 0..64 {
        if at >= total {
            break;
        }
        let start = usize::try_from(at).expect("inside the file");
        let header = &file[start..start.saturating_add(TOP_HEADER_BYTES).min(file.len())];
        let Ok((kind, reply)) =
            worker.answer(FRAME_BOX, Some(encode_box_request(total, at, header)))
        else {
            return;
        };
        assert_eq!(kind, FRAME_BOXED, "a box request is answered with a box");
        let found = decode_boxed(&reply, at, total).expect("the host accepts an honest box");
        assert!(found.end > at, "the walk moves on");
        if &found.kind == b"moov" {
            movie = Some((found.payload, found.end));
        }
        at = found.end;
    }
    let Some((start, end)) = movie else {
        return;
    };
    let movie =
        &file[usize::try_from(start).expect("inside")..usize::try_from(end).expect("inside")];
    if movie.len() > MAX_MOVIE_BYTES {
        return;
    }
    let Ok((kind, reply)) = worker.answer(FRAME_MOVIE, Some(encode_movie_request(total, movie)))
    else {
        return;
    };
    assert_eq!(
        kind, FRAME_TRACK,
        "a movie request is answered with a track"
    );
    let track = decode_track(&reply, total).expect("the host accepts an honest track");
    for run in track.units.chunks(MAX_UNITS_PER_RUN).take(MAX_RUNS) {
        let units: Vec<&[u8]> = run
            .iter()
            .map(|&(offset, size)| {
                let offset = usize::try_from(offset).expect("inside");
                &file[offset..offset.saturating_add(usize::try_from(size).expect("small"))]
            })
            .collect();
        let Ok((kind, reply)) = worker.answer(FRAME_UNITS, Some(encode_units(&units))) else {
            return;
        };
        assert_eq!(kind, FRAME_PCM, "a run is answered with samples");
        let pcm = decode_pcm(&reply, units.len()).expect("the host accepts an honest run");
        assert!(
            pcm.samples.len() <= MAX_SAMPLES_PER_REQUEST,
            "a run within the request's bound"
        );
    }
});
