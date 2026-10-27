//! Fuzzes `pdf_model::media::container`, the reader of a media clip's container headers that
//! ISO 32000-2 §13.2.4.3's offset rules are judged by (ADR 2452).
//!
//! The input is a file an untrusted document embedded, and the reader walks it by the sizes the
//! file states. The first byte chooses the content type — one of each container family the reader
//! knows, and one it does not — and the rest is the file. Beyond never panicking, what it returns
//! is checked against what its bounds promise: no more named offsets than
//! [`pdf_model::media::container::MAX_MARKERS`], and every length and position a finite,
//! non-negative number of seconds.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_model::media::container::{MAX_MARKERS, timeline};

/// One type of each family the reader knows, and one it refuses before reading a byte.
const TYPES: [&str; 5] = [
    "audio/wav",
    "video/avi",
    "audio/aiff",
    "video/mp4",
    "video/mpeg",
];

fuzz_target!(|data: &[u8]| {
    let Some((&head, file)) = data.split_first() else {
        return;
    };
    let Some(&content_type) = usize::from(head)
        .checked_rem(TYPES.len())
        .and_then(|index| TYPES.get(index))
    else {
        return;
    };
    let Ok(read) = timeline(content_type, file) else {
        return;
    };
    assert!(
        read.markers.len() <= MAX_MARKERS,
        "{} markers, past the bound",
        read.markers.len()
    );
    let sound = |seconds: f64| seconds.is_finite() && seconds >= 0.0;
    assert!(
        read.seconds.is_none_or(sound),
        "a length that is not a finite, non-negative number of seconds"
    );
    assert!(
        read.markers.iter().all(|marker| sound(marker.seconds)),
        "a marker at a position that is not a finite, non-negative number of seconds"
    );
});
