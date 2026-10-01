//! Fuzzes the `JPXDecode` filter, ISO 32000-2 §7.4.9: this tree's header reader and the confined
//! worker's decode of the same bytes.
//!
//! Two readers of one untrusted input. [`pdf_model::jpeg2000::Headers::parse`] reads ISO/IEC
//! 15444-1 Annex I's boxes and the `SIZ` marker segment, and T.801's baseline boxes, without
//! decoding a sample. `pdf_sandbox`'s filter hands the codestream to `hayro-jpeg2000` inside the
//! sandboxed process (principle 3) and keeps the sample budget — a step down the resolution
//! progression for a viewer, §7.4.9 NOTE 3, or a refusal for a writer that asked for the whole
//! grid (ADR 1333). [`pdf_sandbox::decode_here`] runs the very functions the worker calls, on this
//! thread, where libFuzzer's own `-timeout` bounds it (ADR 1447).
//!
//! The first byte chooses the request — the stepped-down decode or the whole one, colours or
//! palette indices — and the rest is the file. Beyond never panicking, a raster is checked against
//! [`pdf_sandbox::Raster`]'s documented shape: its samples fill the grid it states at the
//! precision it states, a reduced grid is no larger than the stated one, and a whole decode stays
//! inside the samples it was allowed.

#![no_main]
#![expect(
    clippy::panic,
    reason = "a fuzz target states its properties by failing: `panic!` is how a violated one reaches libFuzzer, and each message here names the property rather than the call"
)]

use libfuzzer_sys::fuzz_target;
use pdf_sandbox::{Decoded, Request};

/// What a whole decode is allowed here: a sixteenth of the ordinary worker's budget, so that one
/// input costs milliseconds while the refusal past it is still reached.
const WHOLE_SAMPLES: u64 = 1 << 22;

fuzz_target!(|data: &[u8]| {
    let Some((&head, file)) = data.split_first() else {
        return;
    };
    let _ = pdf_model::jpeg2000::Headers::parse(file);

    let indices = head & 1 != 0;
    let whole = head & 2 != 0;
    let request = if whole {
        Request::JpxWhole {
            data: file,
            indices,
            samples: WHOLE_SAMPLES,
        }
    } else {
        Request::Jpx {
            data: file,
            indices,
        }
    };
    let Ok(decoded) = pdf_sandbox::decode_here(&request) else {
        return;
    };
    let Decoded::Raster(raster) = decoded else {
        panic!("JPXDecode answers a continuous-tone raster");
    };
    let samples = u64::from(raster.width)
        .checked_mul(u64::from(raster.height))
        .and_then(|pixels| pixels.checked_mul(u64::try_from(raster.channels()).ok()?));
    let bytes = samples
        .and_then(|samples| samples.checked_mul(u64::try_from(raster.bytes_per_sample()).ok()?));
    assert_eq!(
        u64::try_from(raster.data.len()).ok(),
        bytes,
        "the samples fill the grid at the stated precision"
    );
    assert!(
        (1..=16).contains(&raster.precision),
        "a sample is 1 to 16 bits"
    );
    assert!(
        raster.width <= raster.stated_width && raster.height <= raster.stated_height,
        "a reduced grid is no larger than the codestream's own"
    );
    if whole {
        assert_eq!(
            (raster.width, raster.height),
            (raster.stated_width, raster.stated_height),
            "a whole decode is at the codestream's own grid or refused"
        );
        assert!(
            samples.is_some_and(|samples| samples <= WHOLE_SAMPLES),
            "a whole decode stays inside the samples it was allowed"
        );
    }
});
