//! A RIFF WAVE file's header read in the worker: where its samples are and how to decode them
//! (ADR 3349).
//!
//! A rendition's media clip carries a *file* — Table 285's `/D` is "\[a] full file specification or
//! form `XObject`" — and §13.2.4.2 lets its `/CT` name the type. A RIFF WAVE file is the type this
//! program plays itself, and its header is the document's bytes like every other byte of it, so it
//! is read here, behind `Profile::Media`, and never in the host: the host learns where the samples
//! are and what format they are in, checks both against the file it holds, and sends the samples
//! back as ordinary runs ([`crate::wire`]).
//!
//! **The layout is the RIFF form's**, from Microsoft's and IBM's *Multimedia Programming Interface
//! and Data Specifications 1.0* (1991), with RFC 2361's registry for the format tags; neither is
//! held in `doc/`, so what follows paraphrases the facts this reader depends on and cites them:
//!
//! - a file is a `RIFF` chunk whose form type is `WAVE`; a chunk is a four-character identifier, a
//!   32-bit little-endian size and that many bytes, followed by one pad byte where the size is odd;
//! - the `fmt ` chunk comes before the `data` chunk, and begins with `wFormatTag`, `nChannels`,
//!   `nSamplesPerSec`, `nAvgBytesPerSec`, `nBlockAlign` and `wBitsPerSample`, each little-endian;
//! - a PCM sample (`WAVE_FORMAT_PCM`, 1) sits in the smallest whole number of bytes that holds it,
//!   least significant byte first, its amplitude in the most significant bits; one of one to eight
//!   bits is unsigned and one of nine or more is two's complement;
//! - RFC 2361's `WAVE_FORMAT_ALAW` (6) and `WAVE_FORMAT_MULAW` (7) are eight-bit G.711 codes, one
//!   byte each.
//!
//! **A sample is decoded as its container.** A twelve-bit sample in two bytes is read as sixteen
//! bits whose low four are zero, which is the same number set in the top of an `i32`, so the reply
//! names the container's width and [`crate::sound::Encoding::Little`].
//!
//! **What this reader declines, each with a sentence**: `WAVE_FORMAT_EXTENSIBLE` and every other
//! tag (the 1991 form names PCM alone, RFC 2361 lists the rest, and none of them is a format this
//! program decodes), a block alignment that disagrees with the channels and the width, a `data`
//! chunk before the `fmt ` chunk, and a header whose `fmt ` chunk or `data` chunk header lies past
//! the [`crate::wire::MAX_RUN_BYTES`] the host shows the worker — one request's bound, so that the
//! header is read in one exchange (ADR 3349 prices it).

use crate::sound::{Encoding, SoundFormat};

/// Where a WAVE file's samples are, and how to read them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// The samples' format, in the decoder's own terms.
    pub format: SoundFormat,
    /// Where the `data` chunk's bytes begin, from the start of the file.
    pub offset: u64,
    /// How many of them there are: the chunk's stated size, cut to the file's end.
    pub length: u64,
}

/// `WAVE_FORMAT_PCM`.
const PCM: u16 = 1;
/// RFC 2361's `WAVE_FORMAT_ALAW`.
const ALAW: u16 = 6;
/// RFC 2361's `WAVE_FORMAT_MULAW`.
const MULAW: u16 = 7;
/// The bytes of a chunk's identifier and size.
const CHUNK_HEADER: usize = 8;
/// The bytes of the `fmt ` chunk's common fields, which every format tag begins with.
const FORMAT_BYTES: usize = 16;

/// Reads the header of a WAVE file of `total` bytes, of which `head` is the beginning.
///
/// # Errors
///
/// The sentence the worker refuses with: a file that is not RIFF WAVE, a format this program does
/// not decode, a header that disagrees with itself, or one that lies past `head`.
pub fn layout(head: &[u8], total: u64) -> Result<Layout, String> {
    let (Some(b"RIFF"), Some(b"WAVE")) = (head.get(0..4), head.get(8..12)) else {
        return Err(
            "the file does not begin as a RIFF WAVE file does (\"RIFF\", a size, \"WAVE\")"
                .to_owned(),
        );
    };
    let mut at = 12usize;
    let mut format: Option<SoundFormat> = None;
    loop {
        let Some(header) = head.get(at..at.saturating_add(CHUNK_HEADER)) else {
            return Err(if u64::try_from(at).is_ok_and(|at| at >= total) {
                "the file ends before its data chunk".to_owned()
            } else {
                format!(
                    "the file's format and data chunks do not begin within the {} bytes this \
                     player reads a header from",
                    head.len()
                )
            });
        };
        let [i0, i1, i2, i3, s0, s1, s2, s3] = *header else {
            return Err("a chunk header of the wrong length".to_owned());
        };
        let id = [i0, i1, i2, i3];
        let size = u32::from_le_bytes([s0, s1, s2, s3]);
        let body = at.saturating_add(CHUNK_HEADER);
        match &id {
            b"fmt " => {
                let Some(fields) = usize::try_from(size)
                    .ok()
                    .and_then(|size| head.get(body..body.saturating_add(size)))
                else {
                    return Err(format!(
                        "the file's fmt chunk of {size} bytes runs past the {} bytes this player \
                         reads a header from",
                        head.len()
                    ));
                };
                format = Some(read_format(fields)?);
            }
            b"data" => {
                let Some(format) = format else {
                    return Err(
                        "the file's data chunk comes before its fmt chunk, and the RIFF WAVE form \
                         places the format first"
                            .to_owned(),
                    );
                };
                let offset = u64::try_from(body).unwrap_or(u64::MAX);
                let length = u64::from(size).min(total.saturating_sub(offset));
                return Ok(Layout {
                    format,
                    offset,
                    length,
                });
            }
            _ => {}
        }
        // A chunk's body is followed by a pad byte where its size is odd.
        let padded = u64::from(size).saturating_add(u64::from(size & 1));
        at = usize::try_from(padded)
            .ok()
            .and_then(|padded| body.checked_add(padded))
            .ok_or_else(|| "a chunk's size runs past any file this player reads".to_owned())?;
    }
}

/// The `fmt ` chunk's common fields, read and checked.
fn read_format(fields: &[u8]) -> Result<SoundFormat, String> {
    let Some((common, _)) = fields.split_first_chunk::<FORMAT_BYTES>() else {
        return Err(format!(
            "the file's fmt chunk holds {} bytes, fewer than the {FORMAT_BYTES} every format \
             begins with",
            fields.len()
        ));
    };
    let [t0, t1, c0, c1, r0, r1, r2, r3, _, _, _, _, a0, a1, b0, b1] = *common;
    let tag = u16::from_le_bytes([t0, t1]);
    let channels = u16::from_le_bytes([c0, c1]);
    let rate = u32::from_le_bytes([r0, r1, r2, r3]);
    let block_align = u16::from_le_bytes([a0, a1]);
    let bits = u16::from_le_bytes([b0, b1]);
    let (encoding, container_bits) = match tag {
        PCM => {
            if !(1..=32).contains(&bits) {
                return Err(format!(
                    "the file states {bits}-bit PCM samples, outside the 1 to 32 bits this player \
                     reads"
                ));
            }
            let width = bits.div_ceil(8).saturating_mul(8);
            let encoding = if width == 8 {
                Encoding::Raw
            } else {
                Encoding::Little
            };
            (encoding, width)
        }
        ALAW | MULAW => {
            if bits != 8 {
                return Err(format!(
                    "the file states {bits}-bit G.711 codes, and a G.711 code is eight bits"
                ));
            }
            let encoding = if tag == ALAW {
                Encoding::ALaw
            } else {
                Encoding::MuLaw
            };
            (encoding, 8)
        }
        other => {
            return Err(format!(
                "the file's format tag {other:#06x} is not PCM (1), A-law (6) or µ-law (7), the \
                 formats this player decodes (RFC 2361)"
            ));
        }
    };
    let expected = u32::from(channels).saturating_mul(u32::from(container_bits / 8));
    if u32::from(block_align) != expected {
        return Err(format!(
            "the file's block alignment of {block_align} bytes is not its {channels} channel(s) \
             of {container_bits}-bit samples"
        ));
    }
    let format = SoundFormat {
        rate,
        channels,
        bits: u8::try_from(container_bits).unwrap_or(u8::MAX),
        encoding,
    };
    match format.refusal() {
        Some(refusal) => Err(refusal),
        None => Ok(format),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A WAVE file of `tag`, `channels`, `rate`, `bits` and `block_align`, with `before` chunks
    /// ahead of the format and `data` as its samples.
    fn file(
        tag: u16,
        channels: u16,
        rate: u32,
        bits: u16,
        block_align: u16,
        before: &[(&[u8; 4], &[u8])],
        data: &[u8],
    ) -> Vec<u8> {
        let mut body = b"WAVE".to_vec();
        for (id, bytes) in before {
            chunk(&mut body, **id, bytes);
        }
        let mut fields = Vec::new();
        fields.extend_from_slice(&tag.to_le_bytes());
        fields.extend_from_slice(&channels.to_le_bytes());
        fields.extend_from_slice(&rate.to_le_bytes());
        let average = rate.saturating_mul(u32::from(block_align));
        fields.extend_from_slice(&average.to_le_bytes());
        fields.extend_from_slice(&block_align.to_le_bytes());
        fields.extend_from_slice(&bits.to_le_bytes());
        chunk(&mut body, *b"fmt ", &fields);
        chunk(&mut body, *b"data", data);
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(&u32::try_from(body.len()).unwrap_or(u32::MAX).to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    fn chunk(out: &mut Vec<u8>, id: [u8; 4], bytes: &[u8]) {
        out.extend_from_slice(&id);
        out.extend_from_slice(&u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_le_bytes());
        out.extend_from_slice(bytes);
        if bytes.len() % 2 == 1 {
            out.push(0);
        }
    }

    fn read(file: &[u8]) -> Result<Layout, String> {
        layout(file, file.len() as u64)
    }

    /// Sixteen-bit stereo PCM: two channels of two-byte containers, read as little-endian, the
    /// `data` chunk's bytes located after the twelve-byte RIFF header and two chunk headers.
    #[test]
    fn sixteen_bit_stereo_pcm_is_located_and_read_little_endian() {
        let wave = file(PCM, 2, 44_100, 16, 4, &[], &[1, 2, 3, 4]);
        let found = read(&wave).expect("a PCM file");
        assert_eq!(found.format.encoding, Encoding::Little);
        assert_eq!((found.format.bits, found.format.channels), (16, 2));
        assert_eq!(found.format.rate, 44_100);
        assert_eq!((found.offset, found.length), (12 + 8 + 16 + 8, 4));
    }

    /// Eight-bit PCM is unsigned, which is §13.3's `Raw` read the same way; a twelve-bit sample is
    /// read as its sixteen-bit container.
    #[test]
    fn eight_bit_pcm_is_unsigned_and_twelve_bits_are_their_container() {
        let found = read(&file(PCM, 1, 8000, 8, 1, &[], &[128])).expect("eight bits");
        assert_eq!(
            (found.format.encoding, found.format.bits),
            (Encoding::Raw, 8)
        );
        let found = read(&file(PCM, 1, 8000, 12, 2, &[], &[0, 16])).expect("twelve bits");
        assert_eq!(
            (found.format.encoding, found.format.bits),
            (Encoding::Little, 16)
        );
    }

    /// RFC 2361's tags 6 and 7 are G.711's A-law and µ-law, eight bits a code.
    #[test]
    fn a_law_and_mu_law_tags_are_g711_codes() {
        let found = read(&file(ALAW, 1, 8000, 8, 1, &[], &[0xD5])).expect("A-law");
        assert_eq!(found.format.encoding, Encoding::ALaw);
        let found = read(&file(MULAW, 1, 8000, 8, 1, &[], &[0xFF])).expect("µ-law");
        assert_eq!(found.format.encoding, Encoding::MuLaw);
        assert!(read(&file(MULAW, 1, 8000, 16, 2, &[], &[])).is_err());
    }

    /// A chunk before the format is stepped over with its pad byte where its size is odd.
    #[test]
    fn an_odd_chunk_before_the_format_is_stepped_over_with_its_pad() {
        let wave = file(PCM, 1, 8000, 16, 2, &[(b"LIST", b"abc")], &[0, 1]);
        let found = read(&wave).expect("a file with a LIST chunk");
        assert_eq!(found.offset, 12 + 8 + 4 + 8 + 16 + 8);
    }

    /// A `data` chunk that claims more than the file holds is cut to the file's end.
    #[test]
    fn a_data_chunk_past_the_file_is_cut_to_its_end() {
        let mut wave = file(PCM, 1, 8000, 16, 2, &[], &[0, 1, 2, 3]);
        let at = wave.len().saturating_sub(8);
        wave.splice(at..at.saturating_add(4), u32::MAX.to_le_bytes());
        assert_eq!(read(&wave).expect("a long data chunk").length, 4);
    }

    /// Every refusal says why: not RIFF WAVE, another tag, a disagreeing block alignment, data
    /// before the format, and a header past what the worker was shown.
    #[test]
    fn headers_this_player_does_not_read_are_refused() {
        assert!(read(b"RIFX\0\0\0\0WAVE").is_err());
        assert!(read(&file(0xFFFE, 2, 48_000, 16, 4, &[], &[])).is_err());
        assert!(read(&file(3, 1, 48_000, 32, 4, &[], &[])).is_err());
        assert!(read(&file(PCM, 2, 8000, 16, 2, &[], &[])).is_err());
        let mut body = b"RIFF\0\0\0\0WAVE".to_vec();
        chunk(&mut body, *b"data", &[0, 0]);
        assert!(read(&body).is_err());
        let wave = file(PCM, 1, 8000, 16, 2, &[(b"junk", &[0; 64])], &[0, 1]);
        let shown = layout(&wave[..40], wave.len() as u64).expect_err("past the head");
        assert!(shown.contains("within the 40 bytes"), "{shown}");
        assert!(read(&file(PCM, 0, 8000, 16, 0, &[], &[])).is_err());
    }
}
