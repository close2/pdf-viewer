//! The one encoder: samples to ISO 32000-2 §7.4.8's `DCTDecode` data, for a caller that asked.
//!
//! **Nothing a document says reaches this.** The transform suite's two explicit requests —
//! `optimize --images` and `render --format jpeg` — are its only callers, because the owner's
//! condition on the encoder is that nothing is re-encoded unless a format is explicitly requested
//! (`doc/questions/A01`). It lives in this crate rather than beside its callers for the reason the
//! decoders do: the samples it is handed were decoded from an untrusted file, and the encoder's
//! SIMD dependencies carry `unsafe`, so the worker's confinement and budgets are where those
//! samples meet that code (ADR 2138).
//!
//! # What the clause fixes, and what it leaves to the encoder
//!
//! §7.4.8 fixes the format and leaves the rest to the encoder:
//!
//! > The DCTDecode filter decodes grayscale or colour image data that has been encoded in the
//! > JPEG baseline format in accordance with ISO/IEC 10918 (all parts).
//!
//! > The values of these parameters, which include the dimensions of the image and the number of
//! > components per sample, are entirely under the control of the encoder and shall be stored in
//! > the encoded data.
//!
//! So the output is sequential baseline, and the two settings that decide it are stated here
//! rather than inherited from a preset: progression is off — "[t]here is no benefit to using
//! progressive JPEG for stream data that is embedded in a PDF file" (§7.4.8 NOTE 4) — and the
//! quantisation tables are held to eight bits, because ISO/IEC 10918-1's baseline process admits
//! no sixteen-bit table and the encoder would otherwise write one at a low quality. What remains
//! is the encoder's own and the clause's permission: trellis quantisation and optimised Huffman
//! tables, both of which a baseline decoder reads unchanged.
//!
//! **Three components are written as `YCbCr` with no Adobe marker**, and Table 13 is why that is
//! the right pairing: where the APP14 marker is absent and `/DecodeParms` states no
//! `/ColorTransform`, the table makes the default `ColorTransform` 1 for an image of three
//! components, which is the transform from `YCbCr` back to RGB that this encoder's forward
//! conversion inverts. One component is never transformed. Four components are not encoded at
//! all: the encoder offers no CMYK path, and a caller with a `DeviceCMYK` image keeps its own bytes.

use crate::protocol::{DctComponents, DctParameters};

/// The most samples — pixels times components — one encode is given.
///
/// The decoders' own bound ([`crate::decode::MAX_SAMPLES`]), and measured against the same
/// gigabyte: the encoder's working set at 4:2:0 with trellis quantisation is its colour planes,
/// one 128-byte coefficient block and one 256-byte raw block per eight-by-eight luma block and per
/// pair of chroma blocks, about five bytes a sample beside the request's own copy of the samples
/// and the answer — 2^26 samples is under half the ceiling. An image past it is refused before an
/// allocation, and its caller keeps what the producer wrote.
pub(crate) const MAX_SAMPLES: u64 = crate::decode::MAX_SAMPLES;

/// The encoder's own allocation ceiling, in bytes, which it checks against its estimate before it
/// allocates: half of the worker's gigabyte, so that the estimate being wrong by the factor ADR
/// 0233 once found in a decoder's still leaves the process inside its limit.
const MAX_ALLOCATION: usize = 1 << 29;

/// Encodes `samples` as baseline JPEG, or says why not.
///
/// # Errors
///
/// A sentence naming what was refused: a quality outside 1 to 100, an empty grid, samples that
/// disagree with the grid they claim, a grid past [`MAX_SAMPLES`], or the encoder's own refusal.
pub(crate) fn dct(samples: &[u8], parameters: DctParameters) -> Result<Vec<u8>, String> {
    let DctParameters {
        width,
        height,
        components,
        quality,
    } = parameters;
    if !(1..=100).contains(&quality) {
        return Err(format!("a JPEG quality is 1 to 100, not {quality}"));
    }
    if width == 0 || height == 0 {
        return Err(format!(
            "an empty {width}x{height} image has nothing to encode"
        ));
    }
    let channels: u64 = match components {
        DctComponents::Gray => 1,
        DctComponents::Rgb => 3,
    };
    let expected = u64::from(width)
        .saturating_mul(u64::from(height))
        .saturating_mul(channels);
    if expected > MAX_SAMPLES {
        return Err(format!(
            "a {width}x{height} image of {channels} components is {expected} samples, past the \
             {MAX_SAMPLES} one encode is given"
        ));
    }
    if u64::try_from(samples.len()).unwrap_or(u64::MAX) != expected {
        return Err(format!(
            "a {width}x{height} image of {channels} components is {expected} samples, and {} \
             arrived",
            samples.len()
        ));
    }

    let encoder = mozjpeg_rs::Encoder::new(mozjpeg_rs::Preset::BaselineBalanced)
        .quality(quality)
        .progressive(false)
        .force_baseline(true)
        .limits(mozjpeg_rs::Limits {
            max_alloc_bytes: MAX_ALLOCATION,
            ..mozjpeg_rs::Limits::default()
        });
    let jpeg = match components {
        DctComponents::Gray => encoder.encode_gray(samples, width, height),
        DctComponents::Rgb => encoder.encode_rgb(samples, width, height),
    };
    jpeg.map_err(|error| format!("the DCT encoder refused: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameters(width: u32, height: u32, components: DctComponents) -> DctParameters {
        DctParameters {
            width,
            height,
            components,
            quality: 80,
        }
    }

    /// A gradient, so that the quantisation has something to do.
    fn ramp(width: u32, height: u32, channels: u32) -> Vec<u8> {
        (0..width.saturating_mul(height).saturating_mul(channels))
            .map(|index| u8::try_from(index % 251).unwrap_or(0))
            .collect()
    }

    /// ISO/IEC 10918-1 Table B.1: the markers that open the file and name its process. SOF0 is
    /// "[b]aseline DCT", the process §7.4.8 names; SOF2 would be progressive.
    fn frame_marker(jpeg: &[u8]) -> Option<u8> {
        jpeg.windows(2)
            .position(|pair| pair[0] == 0xFF && (0xC0..=0xC2).contains(&pair[1]))
            .and_then(|at| jpeg.get(at.saturating_add(1)).copied())
    }

    #[test]
    fn three_components_are_written_as_baseline() {
        let jpeg = dct(&ramp(64, 48, 3), parameters(64, 48, DctComponents::Rgb))
            .expect("a well-formed grid encodes");
        assert_eq!(jpeg.get(..2), Some(&[0xFF, 0xD8][..]), "SOI");
        assert_eq!(jpeg.get(jpeg.len() - 2..), Some(&[0xFF, 0xD9][..]), "EOI");
        assert_eq!(frame_marker(&jpeg), Some(0xC0), "SOF0, baseline");
    }

    /// Table 13's default `ColorTransform` is 1 only where no APP14 marker says otherwise, so an
    /// Adobe marker in the output would decide the transform instead of the default this encoder
    /// is paired with.
    #[test]
    fn no_adobe_marker_overrides_table_13s_default() {
        let jpeg = dct(&ramp(32, 32, 3), parameters(32, 32, DctComponents::Rgb))
            .expect("a well-formed grid encodes");
        assert!(!jpeg.windows(2).any(|pair| pair == [0xFF, 0xEE]));
    }

    /// At the lowest quality the encoder would write a sixteen-bit table unless held to baseline;
    /// ISO/IEC 10918-1 B.2.4.1 makes a table's precision `Pq` 0, eight bits, for the baseline
    /// process.
    #[test]
    fn the_lowest_quality_keeps_eight_bit_tables() {
        let jpeg = dct(
            &ramp(16, 16, 1),
            DctParameters {
                quality: 1,
                ..parameters(16, 16, DctComponents::Gray)
            },
        )
        .expect("a well-formed grid encodes");
        let at = jpeg
            .windows(2)
            .position(|pair| pair == [0xFF, 0xDB])
            .expect("a DQT segment");
        let pq_tq = jpeg.get(at + 4).copied().expect("the table's first byte");
        assert_eq!(pq_tq >> 4, 0, "Pq 0: eight-bit precision");
        assert_eq!(frame_marker(&jpeg), Some(0xC0));
    }

    #[test]
    fn samples_that_disagree_with_the_grid_are_refused() {
        let refused = dct(&[0; 10], parameters(4, 4, DctComponents::Gray))
            .expect_err("ten samples are not a 4x4 grid");
        assert!(refused.contains("16 samples"), "{refused}");
    }

    #[test]
    fn a_grid_past_the_budget_is_refused_before_anything_is_allocated() {
        let refused = dct(&[], parameters(1 << 14, 1 << 13, DctComponents::Rgb))
            .expect_err("2^29 samples is past the budget");
        assert!(refused.contains("past the"), "{refused}");
    }

    #[test]
    fn a_quality_outside_one_to_a_hundred_is_refused() {
        for quality in [0, 101] {
            assert!(
                dct(
                    &ramp(8, 8, 1),
                    DctParameters {
                        quality,
                        ..parameters(8, 8, DctComponents::Gray)
                    }
                )
                .is_err()
            );
        }
    }

    /// RFC 0002 section 9 makes a written file a function of the plan, so the encoder's answer may
    /// not follow the processor it runs on: the CPU's own SIMD dispatch and the encoder's scalar
    /// path write the same bytes.
    #[test]
    fn the_bytes_do_not_follow_the_processors_simd() {
        let samples = ramp(96, 80, 3);
        let detected = dct(&samples, parameters(96, 80, DctComponents::Rgb)).expect("encodes");
        let scalar = mozjpeg_rs::Encoder::new(mozjpeg_rs::Preset::BaselineBalanced)
            .quality(80)
            .progressive(false)
            .force_baseline(true)
            .simd_ops(mozjpeg_rs::simd::SimdOps::scalar())
            .encode_rgb(&samples, 96, 80)
            .expect("encodes");
        assert_eq!(detected, scalar);
    }
}
