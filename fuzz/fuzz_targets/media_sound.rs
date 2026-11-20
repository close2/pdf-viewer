//! Fuzzes the media worker's entry point and its wire from both sides (ADR 3297): a request as the
//! confined worker reads and decodes it, and a reply as the host that is not confined reads it.
//!
//! **Both ends are a parser over the other's bytes.** `pdf-media-worker` decodes ISO 32000-2
//! §13.3's samples behind `pdf_sandbox::lockdown::Profile::Media`, and what reaches its standard
//! input is whatever reached it; what it writes back is read by a host that holds no confinement,
//! so the host's reader is a parser over bytes a subverted worker chose (RFC 0015 section 4.7).
//!
//! The input is one message, decoded as a request and then decoded by `sound::decode` exactly as
//! the worker's `answer` does, and decoded again as a reply checked against the request it would
//! answer. Beyond never panicking — overflow checks stay on in this profile — three properties:
//!
//! - **The decoder answers exactly the samples the request's bytes hold.** `sample_counts` is the
//!   host's own arithmetic, and the reply check refuses any other count, so a decoder that wrote
//!   one sample more or fewer would be refused by the host it serves.
//! - **What the worker encodes the host reads back to the same.**
//! - **What decodes as a request re-encodes to the same bytes.**
//!
//! The same input is then a RIFF WAVE layout request (ADR 3349): its file's header read by
//! `wave::layout` as the worker's `answer` reads it, with one more property — **the samples the
//! worker places lie inside the file the request describes**, so the host's own check on the reply
//! (`decode_placed`) accepts every honest answer.

#![no_main]
#![expect(
    clippy::expect_used,
    reason = "a fuzz target states its properties by failing: `expect` is how a violated one reaches libFuzzer, and each message names the property"
)]

use libfuzzer_sys::fuzz_target;
use pdf_media_worker::sound::{decode, sample_counts};
use pdf_media_worker::wire::{
    FRAME_LAYOUT, FRAME_PLACED, MAX_SAMPLES_PER_REQUEST, decode_layout_request, decode_placed,
    decode_request, decode_samples, encode_request, encode_samples,
};

fuzz_target!(|data: &[u8]| {
    if let Ok((format, run)) = decode_request(data) {
        assert_eq!(
            encode_request(&format, run),
            data,
            "a request decoded re-encodes to its own bytes"
        );
        if let Ok(decoded) = decode(&format, run, MAX_SAMPLES_PER_REQUEST) {
            let (samples, unframed) = sample_counts(&format, run.len());
            assert_eq!(
                (decoded.samples.len() as u64, decoded.unframed),
                (samples, unframed),
                "the decoder answers the samples the request holds"
            );
            let reply = encode_samples(&decoded);
            let read = decode_samples(&reply, &format, run.len())
                .expect("the host reads back what the worker encoded");
            assert_eq!(read, decoded, "the reply crosses unchanged");
        }
        // The same bytes as a reply a subverted worker sent: refused or read, never a panic.
        let _ = decode_samples(run, &format, run.len());
    }
    if let Ok((total, head)) = decode_layout_request(data) {
        if let Ok((kind, reply)) = pdf_media_worker::answer(FRAME_LAYOUT, Some(data.to_vec())) {
            assert_eq!(kind, FRAME_PLACED, "a layout request is answered with a layout");
            let placed = decode_placed(&reply, total)
                .expect("the host accepts the samples the worker placed inside the file");
            assert_eq!(
                Ok(placed),
                pdf_media_worker::wave::layout(head, total),
                "the reply is the layout the reader found"
            );
        }
        // The same bytes as a layout reply a subverted worker sent: refused or read, never a panic.
        let _ = decode_placed(head, total);
    }
});
