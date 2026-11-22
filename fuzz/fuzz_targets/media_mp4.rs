//! Fuzzes `pdf_media_worker::mp4::sound_track`, the confined media worker's reader of an MP4
//! file's sound track (ADR 3369).
//!
//! The input is a file an untrusted document embedded, and the reader walks it by the sizes and
//! counts the file states. Beyond never panicking — overflow checks stay on in this profile — what
//! it returns is checked against what its bounds promise, which is what the host will check the
//! table against when it crosses the wire:
//!
//! - **no more access units than [`MAX_ACCESS_UNITS`]**;
//! - **every access unit lies inside the file** it was read from;
//! - **no more decoder configuration than [`MAX_DECODER_CONFIG_BYTES`]**.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_media_worker::mp4::{MAX_ACCESS_UNITS, MAX_DECODER_CONFIG_BYTES, sound_track};

fuzz_target!(|file: &[u8]| {
    let Ok(track) = sound_track(file) else {
        return;
    };
    assert!(
        track.access_units.len() <= MAX_ACCESS_UNITS,
        "{} access units, past the bound",
        track.access_units.len()
    );
    let length = u64::try_from(file.len()).unwrap_or(u64::MAX);
    assert!(
        track.access_units.iter().all(|unit| unit
            .offset
            .checked_add(u64::from(unit.size))
            .is_some_and(|end| end <= length)),
        "an access unit past the file's end"
    );
    assert!(
        track.decoder_config.len() <= MAX_DECODER_CONFIG_BYTES,
        "a decoder configuration past the bound"
    );
});
