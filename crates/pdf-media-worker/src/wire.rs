//! What crosses to a media worker and back, as bytes (ADR 3297).
//!
//! `confined-transport` carries a kind byte and a length and knows nothing of either; this module is
//! what the kind means and what the payload holds. RFC 0015 section 4.3's shape for audio: **one
//! request per run of whole frames, one reply of samples per request** — a sound's PCM is under half
//! a megabyte a second, so the ordinary wire carries it and no shared memory is needed.
//!
//! Every integer is little-endian and every payload begins with a version byte, so that a host and
//! a worker built from different trees refuse each other rather than misread each other. **The
//! host reads a reply as a compromised worker's**: the number of samples a request's bytes hold is
//! the host's own arithmetic, so a reply that disagrees with it is refused whatever it says, and
//! what a worker can still lie about is the sample values — which are sound, and bounded by their
//! width.

use crate::sound::{Decoded, Encoding, SoundFormat, sample_counts};

/// The eight bytes a media worker's greeting begins with.
///
/// Different from every other worker's, so that a host started on the wrong program refuses it at
/// its first nine bytes.
pub const MAGIC: &[u8; 8] = b"PDFMDW01";

/// The kind of a request: host to worker, a format and a run of whole frames.
pub const FRAME_DECODE: u8 = 1;
/// The kind of a reply: worker to host, the run's samples.
pub const FRAME_SAMPLES: u8 = 2;
/// The kind of a refusal: worker to host, one sentence of UTF-8 as the payload.
pub const FRAME_REFUSED: u8 = 3;

/// The first byte of every payload.
pub const VERSION: u8 = 1;

/// Longest run of a sound's bytes one request carries.
///
/// A quarter of a mebibyte is sixteen seconds of §13.3's widest recommended format (22,050 samples
/// a second, two channels of sixteen bits), so a sound reaches the sink in a handful of requests,
/// and its decoded copy at the worst case — one-bit samples, thirty-two bytes out for each byte in
/// — is 8 MiB, which `pdf_sandbox::lockdown::Profile::Media`'s ceiling is sized against.
pub const MAX_RUN_BYTES: usize = 256 << 10;

/// A request's bytes before its run: the version, `/R` as a `u32`, `/C` as a `u16`, `/B` and the
/// encoding's byte.
pub const REQUEST_HEADER_BYTES: usize = 1 + 4 + 2 + 1 + 1;

/// Largest request a worker reads.
pub const MAX_REQUEST_BYTES: usize = REQUEST_HEADER_BYTES + MAX_RUN_BYTES;

/// Most samples one request decodes to: a run of one-bit samples, eight to a byte.
pub const MAX_SAMPLES_PER_REQUEST: usize = MAX_RUN_BYTES * 8;

/// Longest refusal sentence either side keeps, in bytes.
pub const MAX_SENTENCE_BYTES: usize = 4096;

/// A request's bytes: `format`, then `run`.
#[must_use]
pub fn encode_request(format: &SoundFormat, run: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(REQUEST_HEADER_BYTES.saturating_add(run.len()));
    bytes.push(VERSION);
    bytes.extend_from_slice(&format.rate.to_le_bytes());
    bytes.extend_from_slice(&format.channels.to_le_bytes());
    bytes.push(format.bits);
    bytes.push(format.encoding.code());
    bytes.extend_from_slice(run);
    bytes
}

/// A request read back: its format and its run.
///
/// # Errors
///
/// The sentence a worker refuses with: another version, a header cut short, an encoding byte no
/// encoding has, or a run past [`MAX_RUN_BYTES`].
pub fn decode_request(payload: &[u8]) -> Result<(SoundFormat, &[u8]), String> {
    let Some((header, run)) = payload.split_at_checked(REQUEST_HEADER_BYTES) else {
        return Err(format!(
            "a request of {} bytes is shorter than its {REQUEST_HEADER_BYTES}-byte header",
            payload.len()
        ));
    };
    let [version, r0, r1, r2, r3, c0, c1, bits, encoding] = *header else {
        return Err("a request header of the wrong length".to_owned());
    };
    if version != VERSION {
        return Err(format!(
            "a request of version {version}, and this worker reads version {VERSION}"
        ));
    }
    let Some(encoding) = Encoding::from_code(encoding) else {
        return Err(format!(
            "an encoding byte {encoding} that names no encoding"
        ));
    };
    if run.len() > MAX_RUN_BYTES {
        return Err(format!(
            "a run of {} bytes is past the {MAX_RUN_BYTES} one request carries",
            run.len()
        ));
    }
    let format = SoundFormat {
        rate: u32::from_le_bytes([r0, r1, r2, r3]),
        channels: u16::from_le_bytes([c0, c1]),
        bits,
        encoding,
    };
    Ok((format, run))
}

/// A reply's bytes: the version, the count of samples after the last whole frame as a `u64`, then
/// every sample as four bytes.
#[must_use]
pub fn encode_samples(decoded: &Decoded) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(decoded.samples.len().saturating_mul(4).saturating_add(9));
    bytes.push(VERSION);
    bytes.extend_from_slice(&decoded.unframed.to_le_bytes());
    for sample in &decoded.samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

/// A reply read back, checked against the request it answers: `format` and the `run_len` bytes the
/// host sent.
///
/// # Errors
///
/// The sentence the host refuses the reply with: another version, or a count of samples other than
/// the one the request's bytes hold.
pub fn decode_samples(
    payload: &[u8],
    format: &SoundFormat,
    run_len: usize,
) -> Result<Decoded, String> {
    let (expected, expected_unframed) = sample_counts(format, run_len);
    let Some((&version, rest)) = payload.split_first() else {
        return Err("an empty reply".to_owned());
    };
    if version != VERSION {
        return Err(format!(
            "a reply of version {version}, and this host reads version {VERSION}"
        ));
    }
    let Some((unframed, samples)) = rest.split_first_chunk::<8>() else {
        return Err("a reply cut short before its count".to_owned());
    };
    let unframed = u64::from_le_bytes(*unframed);
    let (words, remainder) = samples.as_chunks::<4>();
    if !remainder.is_empty() || words.len() as u64 != expected || unframed != expected_unframed {
        return Err(format!(
            "a reply of {} sample bytes and {unframed} unframed, where the request holds {expected} \
             samples and {expected_unframed} unframed",
            samples.len()
        ));
    }
    Ok(Decoded {
        samples: words.iter().map(|word| i32::from_le_bytes(*word)).collect(),
        unframed,
    })
}

/// A refusal's sentence, cut at [`MAX_SENTENCE_BYTES`] on a character boundary.
#[must_use]
pub fn sentence(payload: &[u8]) -> String {
    let text = String::from_utf8_lossy(payload);
    let mut end = text.len().min(MAX_SENTENCE_BYTES);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    text.get(..end).unwrap_or_default().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sixteen-bit stereo format.
    const STEREO: SoundFormat = SoundFormat {
        rate: 22_050,
        channels: 2,
        bits: 16,
        encoding: Encoding::Signed,
    };

    #[test]
    fn a_request_crosses_whole() {
        let bytes = encode_request(&STEREO, &[1, 2, 3, 4]);
        assert_eq!(decode_request(&bytes), Ok((STEREO, &[1u8, 2, 3, 4][..])));
    }

    #[test]
    fn a_request_of_another_version_or_encoding_is_refused() {
        let mut bytes = encode_request(&STEREO, &[]);
        bytes[0] = 2;
        assert!(decode_request(&bytes).is_err());
        let mut bytes = encode_request(&STEREO, &[]);
        bytes[8] = 9;
        assert!(decode_request(&bytes).is_err());
        assert!(decode_request(&[VERSION, 0, 0]).is_err());
    }

    #[test]
    fn a_reply_that_disagrees_with_its_request_is_refused() {
        let decoded = Decoded {
            samples: vec![1, 2],
            unframed: 0,
        };
        let bytes = encode_samples(&decoded);
        // Four bytes of sixteen-bit stereo are one frame: two samples.
        assert_eq!(decode_samples(&bytes, &STEREO, 4), Ok(decoded));
        // Eight bytes would be four, so a worker answering two is not believed.
        assert!(decode_samples(&bytes, &STEREO, 8).is_err());
        assert!(decode_samples(&bytes[..bytes.len() - 1], &STEREO, 4).is_err());
    }
}
