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
//!   byte each;
//! - `WAVE_FORMAT_EXTENSIBLE` (`0xFFFE`, from Microsoft's *Multiple Channel Audio Data and WAVE
//!   Files*, likewise not held) extends the `fmt ` chunk past its common fields with `cbSize`,
//!   `wValidBitsPerSample`, `dwChannelMask` and a sixteen-byte `SubFormat` GUID; a GUID ending in
//!   the base `-0000-0010-8000-00AA00389B71` names, in its first four bytes, the format tag the
//!   short form would have stated, and RFC 2361's `WAVE_FORMAT_IEEE_FLOAT` (3) is a 32-bit IEEE 754
//!   number in each sample's container, full scale at ±1.
//!
//! **A sample is decoded as its container.** A twelve-bit sample in two bytes is read as sixteen
//! bits whose low four are zero, which is the same number set in the top of an `i32`, so the reply
//! names the container's width and [`crate::sound::Encoding::Little`]. An extensible file's
//! `wValidBitsPerSample` is therefore read as a claim about the low bits, not about where the
//! samples are.
//!
//! **The chunks are walked one window at a time** (ADR 3397). The host shows the worker at most
//! [`crate::wire::MAX_RUN_BYTES`] of the file from an offset, and the worker answers either where the
//! samples are or the offset of the next chunk it has not read, with the format it has read so far;
//! the host checks that the offset moves forward inside the file and shows the next window. A file
//! whose header fits the first window — every file this player has met — is read in one exchange,
//! and one whose `LIST`, `bext` or cover art pushes `data` further is read in one more per chunk
//! that ends past a window, never by the host.
//!
//! **What this reader declines, each with a sentence**: every tag but PCM, IEEE float, A-law and
//! µ-law (none other is a format this program decodes), 64-bit floats, an extensible `SubFormat`
//! outside the base GUID, a block alignment that disagrees with the channels and the width, a `data`
//! chunk before the `fmt ` chunk, and a `fmt ` chunk longer than one window.

use crate::sound::{Encoding, SoundFormat};

/// What a `fmt ` chunk states: the samples' format in the decoder's own terms, and an extensible
/// file's `dwChannelMask`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Described {
    /// The samples' format.
    pub format: SoundFormat,
    /// The speaker positions the channels are assigned to, one bit each in the extensible form's
    /// order, or zero where the file states none — the short form never does.
    pub channel_mask: u32,
}

/// Where a WAVE file's samples are, and how to read them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// The samples' format, and the channels' speakers.
    pub described: Described,
    /// Where the `data` chunk's bytes begin, from the start of the file.
    pub offset: u64,
    /// How many of them there are: the chunk's stated size, cut to the file's end.
    pub length: u64,
}

/// One window's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Found {
    /// The samples, found.
    Placed(Layout),
    /// The walk goes on at `at`, the first chunk header this window did not hold whole, carrying
    /// the format read so far.
    Onward {
        /// Where the next window begins, always past the window's own beginning.
        at: u64,
        /// The format, once a `fmt ` chunk has been read.
        described: Option<Described>,
    },
}

/// `WAVE_FORMAT_PCM`.
const PCM: u16 = 1;
/// RFC 2361's `WAVE_FORMAT_IEEE_FLOAT`.
const IEEE_FLOAT: u16 = 3;
/// RFC 2361's `WAVE_FORMAT_ALAW`.
const ALAW: u16 = 6;
/// RFC 2361's `WAVE_FORMAT_MULAW`.
const MULAW: u16 = 7;
/// `WAVE_FORMAT_EXTENSIBLE`.
const EXTENSIBLE: u16 = 0xFFFE;
/// The last twelve bytes of every `SubFormat` GUID that names a short-form tag in its first four:
/// `-0000-0010-8000-00AA00389B71`, as the GUID's fields are laid out in the file.
const BASE_GUID_TAIL: [u8; 12] = [
    0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];
/// The bytes of a chunk's identifier and size.
const CHUNK_HEADER: usize = 8;
/// The bytes of the `fmt ` chunk's common fields, which every format tag begins with.
const FORMAT_BYTES: usize = 16;
/// The bytes of an extensible `fmt ` chunk: the common fields, `cbSize`, `wValidBitsPerSample`,
/// `dwChannelMask` and `SubFormat`.
const EXTENSIBLE_BYTES: usize = FORMAT_BYTES + 2 + 2 + 4 + 16;
/// The bytes of the RIFF header: `RIFF`, the size and `WAVE`.
const RIFF_HEADER: u64 = 12;

/// Reads one window of a WAVE file of `total` bytes: `window` is the file from `at`, and
/// `described` the format an earlier window read.
///
/// # Errors
///
/// The sentence the worker refuses with: a file that is not RIFF WAVE, a format this program does
/// not decode, a header that disagrees with itself, a file that ends before its `data` chunk, or a
/// window that does not hold the one chunk header it begins with.
pub fn layout(
    window: &[u8],
    at: u64,
    total: u64,
    mut described: Option<Described>,
) -> Result<Found, String> {
    let mut chunk = at;
    if at == 0 {
        let (Some(b"RIFF"), Some(b"WAVE")) = (window.get(0..4), window.get(8..12)) else {
            return Err(
                "the file does not begin as a RIFF WAVE file does (\"RIFF\", a size, \"WAVE\")"
                    .to_owned(),
            );
        };
        chunk = RIFF_HEADER;
    }
    loop {
        if chunk >= total {
            return Err("the file ends before its data chunk".to_owned());
        }
        // Where `chunk` falls in the window; past it, the next window begins there.
        let within = usize::try_from(chunk.saturating_sub(at)).unwrap_or(usize::MAX);
        let onward = Found::Onward {
            at: chunk,
            described,
        };
        let Some(header) = window.get(within..within.saturating_add(CHUNK_HEADER)) else {
            return if chunk > at {
                Ok(onward)
            } else {
                Err(format!(
                    "a window of {} bytes at {at} does not hold the chunk header it begins with",
                    window.len()
                ))
            };
        };
        let [i0, i1, i2, i3, s0, s1, s2, s3] = *header else {
            return Err("a chunk header of the wrong length".to_owned());
        };
        let id = [i0, i1, i2, i3];
        let size = u32::from_le_bytes([s0, s1, s2, s3]);
        let body = within.saturating_add(CHUNK_HEADER);
        match &id {
            b"fmt " => {
                let Some(fields) = usize::try_from(size)
                    .ok()
                    .and_then(|size| window.get(body..body.saturating_add(size)))
                else {
                    return if chunk > at {
                        Ok(onward)
                    } else {
                        Err(format!(
                            "the file's fmt chunk of {size} bytes is longer than the {} bytes \
                             this player reads a chunk from",
                            window.len()
                        ))
                    };
                };
                described = Some(read_format(fields)?);
            }
            b"data" => {
                let Some(described) = described else {
                    return Err(
                        "the file's data chunk comes before its fmt chunk, and the RIFF WAVE form \
                         places the format first"
                            .to_owned(),
                    );
                };
                let offset = chunk.saturating_add(8);
                let length = u64::from(size).min(total.saturating_sub(offset));
                return Ok(Found::Placed(Layout {
                    described,
                    offset,
                    length,
                }));
            }
            _ => {}
        }
        // A chunk's body is followed by a pad byte where its size is odd.
        let padded = u64::from(size).saturating_add(u64::from(size & 1));
        chunk = chunk
            .checked_add(8)
            .and_then(|body| body.checked_add(padded))
            .ok_or_else(|| "a chunk's size runs past any file this player reads".to_owned())?;
    }
}

/// The `fmt ` chunk's fields, read and checked.
fn read_format(fields: &[u8]) -> Result<Described, String> {
    let Some((common, extension)) = fields.split_first_chunk::<FORMAT_BYTES>() else {
        return Err(format!(
            "the file's fmt chunk holds {} bytes, fewer than the {FORMAT_BYTES} every format \
             begins with",
            fields.len()
        ));
    };
    let [t0, t1, c0, c1, r0, r1, r2, r3, _, _, _, _, a0, a1, b0, b1] = *common;
    let mut tag = u16::from_le_bytes([t0, t1]);
    let channels = u16::from_le_bytes([c0, c1]);
    let rate = u32::from_le_bytes([r0, r1, r2, r3]);
    let block_align = u16::from_le_bytes([a0, a1]);
    let bits = u16::from_le_bytes([b0, b1]);
    let mut channel_mask = 0;
    if tag == EXTENSIBLE {
        (tag, channel_mask) = extensible(extension, bits, fields.len())?;
    }
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
        IEEE_FLOAT => {
            if bits != 32 {
                return Err(format!(
                    "the file states {bits}-bit IEEE float samples, and this player reads the \
                     32-bit kind"
                ));
            }
            (Encoding::Float, 32)
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
                "the file's format tag {other:#06x} is not PCM (1), IEEE float (3), A-law (6) or \
                 µ-law (7), the formats this player decodes (RFC 2361)"
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
        None => Ok(Described {
            format,
            channel_mask,
        }),
    }
}

/// An extensible `fmt ` chunk's tail after the common fields: the short-form tag its `SubFormat`
/// names, and its channel mask. `bits` is the container's width and `length` the chunk's.
fn extensible(extension: &[u8], bits: u16, length: usize) -> Result<(u16, u32), String> {
    let Some(&[_, _, v0, v1, m0, m1, m2, m3, g0, g1, g2, g3, ref tail @ ..]) =
        extension.first_chunk::<{ EXTENSIBLE_BYTES - FORMAT_BYTES }>()
    else {
        return Err(format!(
            "the file's extensible fmt chunk holds {length} bytes, fewer than the \
             {EXTENSIBLE_BYTES} its SubFormat ends at"
        ));
    };
    let valid = u16::from_le_bytes([v0, v1]);
    if valid > bits {
        return Err(format!(
            "the file states {valid} valid bits in a {bits}-bit container"
        ));
    }
    if *tail != BASE_GUID_TAIL || [g2, g3] != [0, 0] {
        return Err(
            "the file's extensible SubFormat is not a GUID that names a format tag, so it \
             names no format this player decodes"
                .to_owned(),
        );
    }
    Ok((
        u16::from_le_bytes([g0, g1]),
        u32::from_le_bytes([m0, m1, m2, m3]),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A WAVE file of `tag`, `channels`, `rate`, `bits` and `block_align`, with `before` chunks
    /// ahead of the format, `extension` after the format's common fields, and `data` as its
    /// samples.
    #[expect(
        clippy::too_many_arguments,
        reason = "one argument a field of the chunk the tests vary"
    )]
    fn file_with(
        tag: u16,
        channels: u16,
        rate: u32,
        bits: u16,
        block_align: u16,
        before: &[(&[u8; 4], &[u8])],
        extension: &[u8],
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
        fields.extend_from_slice(extension);
        chunk(&mut body, *b"fmt ", &fields);
        chunk(&mut body, *b"data", data);
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(&u32::try_from(body.len()).unwrap_or(u32::MAX).to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    fn file(
        tag: u16,
        channels: u16,
        rate: u32,
        bits: u16,
        block_align: u16,
        before: &[(&[u8; 4], &[u8])],
        data: &[u8],
    ) -> Vec<u8> {
        file_with(tag, channels, rate, bits, block_align, before, &[], data)
    }

    /// The extensible form's tail: `cbSize` 22, `valid` bits, `mask`, and a `SubFormat` GUID
    /// naming `subtag` over `tail`.
    fn extensible(valid: u16, mask: u32, subtag: u16, tail: [u8; 12]) -> Vec<u8> {
        let mut out = 22u16.to_le_bytes().to_vec();
        out.extend_from_slice(&valid.to_le_bytes());
        out.extend_from_slice(&mask.to_le_bytes());
        out.extend_from_slice(&u32::from(subtag).to_le_bytes());
        out.extend_from_slice(&tail);
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

    /// The whole file in one window.
    fn read(file: &[u8]) -> Result<Layout, String> {
        match layout(file, 0, file.len() as u64, None)? {
            Found::Placed(found) => Ok(found),
            Found::Onward { at, .. } => Err(format!("onward at {at}")),
        }
    }

    /// The file walked as the host walks it, `size` bytes a window.
    fn walk(file: &[u8], size: usize) -> Result<(Layout, usize), String> {
        let total = file.len() as u64;
        let (mut at, mut described) = (0u64, None);
        for windows in 1..64 {
            let window = &file[usize::try_from(at).expect("a test file")..];
            let window = window.get(..size).unwrap_or(window);
            match layout(window, at, total, described)? {
                Found::Placed(found) => return Ok((found, windows)),
                Found::Onward {
                    at: next,
                    described: read,
                } => {
                    assert!(next > at, "a walk moves forward");
                    (at, described) = (next, read);
                }
            }
        }
        Err("no end".to_owned())
    }

    /// Sixteen-bit stereo PCM: two channels of two-byte containers, read as little-endian, the
    /// `data` chunk's bytes located after the twelve-byte RIFF header and two chunk headers.
    #[test]
    fn sixteen_bit_stereo_pcm_is_located_and_read_little_endian() {
        let wave = file(PCM, 2, 44_100, 16, 4, &[], &[1, 2, 3, 4]);
        let found = read(&wave).expect("a PCM file");
        let format = found.described.format;
        assert_eq!(format.encoding, Encoding::Little);
        assert_eq!((format.bits, format.channels), (16, 2));
        assert_eq!(format.rate, 44_100);
        assert_eq!((found.offset, found.length), (12 + 8 + 16 + 8, 4));
        assert_eq!(found.described.channel_mask, 0);
    }

    /// Eight-bit PCM is unsigned, which is §13.3's `Raw` read the same way; a twelve-bit sample is
    /// read as its sixteen-bit container.
    #[test]
    fn eight_bit_pcm_is_unsigned_and_twelve_bits_are_their_container() {
        let found = read(&file(PCM, 1, 8000, 8, 1, &[], &[128])).expect("eight bits");
        let format = found.described.format;
        assert_eq!((format.encoding, format.bits), (Encoding::Raw, 8));
        let found = read(&file(PCM, 1, 8000, 12, 2, &[], &[0, 16])).expect("twelve bits");
        let format = found.described.format;
        assert_eq!((format.encoding, format.bits), (Encoding::Little, 16));
    }

    /// RFC 2361's tags 6 and 7 are G.711's A-law and µ-law, eight bits a code.
    #[test]
    fn a_law_and_mu_law_tags_are_g711_codes() {
        let found = read(&file(ALAW, 1, 8000, 8, 1, &[], &[0xD5])).expect("A-law");
        assert_eq!(found.described.format.encoding, Encoding::ALaw);
        let found = read(&file(MULAW, 1, 8000, 8, 1, &[], &[0xFF])).expect("µ-law");
        assert_eq!(found.described.format.encoding, Encoding::MuLaw);
        assert!(read(&file(MULAW, 1, 8000, 16, 2, &[], &[])).is_err());
    }

    /// RFC 2361's tag 3 is a 32-bit float; a 64-bit one is declined.
    #[test]
    fn ieee_float_is_read_at_thirty_two_bits_and_declined_at_sixty_four() {
        let found = read(&file(IEEE_FLOAT, 2, 48_000, 32, 8, &[], &[0; 8])).expect("float");
        let format = found.described.format;
        assert_eq!((format.encoding, format.bits), (Encoding::Float, 32));
        let refused = read(&file(IEEE_FLOAT, 1, 48_000, 64, 8, &[], &[])).expect_err("double");
        assert!(refused.contains("64-bit IEEE float"), "{refused}");
    }

    /// `WAVE_FORMAT_EXTENSIBLE`: the `SubFormat` GUID names PCM or IEEE float by the short form's
    /// tag, the channel mask is carried, and valid bits fewer than the container's are the
    /// container's samples.
    #[test]
    fn an_extensible_format_is_read_by_its_subformat_and_keeps_its_mask() {
        let pcm = extensible(20, 0x3F, PCM, BASE_GUID_TAIL);
        let wave = file_with(EXTENSIBLE, 6, 48_000, 24, 18, &[], &pcm, &[0; 18]);
        let found = read(&wave).expect("extensible PCM");
        let format = found.described.format;
        assert_eq!((format.encoding, format.bits), (Encoding::Little, 24));
        assert_eq!((format.channels, found.described.channel_mask), (6, 0x3F));
        assert_eq!(found.offset, 12 + 8 + 40 + 8);
        let float = extensible(32, 0x3, IEEE_FLOAT, BASE_GUID_TAIL);
        let wave = file_with(EXTENSIBLE, 2, 48_000, 32, 8, &[], &float, &[0; 8]);
        let found = read(&wave).expect("extensible float");
        assert_eq!(found.described.format.encoding, Encoding::Float);
    }

    /// An extensible chunk is refused for a `SubFormat` outside the base GUID, a tag no decoder
    /// reads, more valid bits than the container holds, or fewer than its forty bytes.
    #[test]
    fn extensible_formats_this_player_does_not_read_are_refused() {
        let mut foreign = BASE_GUID_TAIL;
        foreign[11] = 0;
        let cases = [
            extensible(16, 0x3, PCM, foreign),
            extensible(16, 0x3, 0x55, BASE_GUID_TAIL),
            extensible(17, 0x3, PCM, BASE_GUID_TAIL),
        ];
        for extension in cases {
            let wave = file_with(EXTENSIBLE, 2, 48_000, 16, 4, &[], &extension, &[]);
            assert!(read(&wave).is_err());
        }
        let short = file_with(EXTENSIBLE, 2, 48_000, 16, 4, &[], &[22, 0, 16, 0], &[]);
        let refused = read(&short).expect_err("short");
        assert!(refused.contains("fewer than the 40"), "{refused}");
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

    /// Chunks that push the format and the samples past a window are walked a window at a time:
    /// the walk goes on at the first header a window does not hold, carries the format it read,
    /// and finds the same samples as one window over the whole file.
    #[test]
    fn a_header_past_one_window_is_walked_window_by_window() {
        let art = vec![7u8; 300];
        let wave = file(
            PCM,
            2,
            8000,
            16,
            4,
            &[(b"LIST", &art), (b"bext", &art)],
            &[1; 8],
        );
        let (whole, windows) = walk(&wave, wave.len()).expect("one window");
        assert_eq!(windows, 1);
        let (walked, windows) = walk(&wave, 64).expect("small windows");
        assert_eq!(walked, whole);
        assert_eq!(windows, 3, "RIFF and LIST, then bext, then fmt and data");
        assert_eq!(
            layout(&wave[..64], 0, wave.len() as u64, None),
            Ok(Found::Onward {
                at: 12 + 8 + 300,
                described: None
            })
        );
        // The format read in one window rides into the next.
        let first_data = wave.len() - 16;
        let format_window = &wave[..first_data];
        let Ok(Found::Onward { at, described }) = layout(format_window, 0, wave.len() as u64, None)
        else {
            panic!("onward");
        };
        assert_eq!(at, first_data as u64);
        assert_eq!(described, Some(whole.described));
    }

    /// Every refusal says why: not RIFF WAVE, another tag, a disagreeing block alignment, data
    /// before the format, a window that does not hold its first header, a format longer than a
    /// window, and a file that ends before its data.
    #[test]
    fn headers_this_player_does_not_read_are_refused() {
        assert!(read(b"RIFX\0\0\0\0WAVE").is_err());
        assert!(read(&file(0x0002, 2, 48_000, 16, 4, &[], &[])).is_err());
        assert!(read(&file(PCM, 2, 8000, 16, 2, &[], &[])).is_err());
        let mut body = b"RIFF\0\0\0\0WAVE".to_vec();
        chunk(&mut body, *b"data", &[0, 0]);
        assert!(read(&body).is_err());
        let wave = file(PCM, 1, 8000, 16, 2, &[(b"junk", &[0; 64])], &[0, 1]);
        let short = layout(&wave[84..86], 84, wave.len() as u64, None).expect_err("short");
        assert!(short.contains("does not hold the chunk header"), "{short}");
        let long = file_with(PCM, 1, 8000, 16, 2, &[], &[0; 64], &[0, 1]);
        let refused = layout(&long[12..40], 12, long.len() as u64, None).expect_err("long fmt");
        assert!(refused.contains("longer than the 28 bytes"), "{refused}");
        let cut = &wave[..84];
        assert_eq!(
            layout(cut, 0, cut.len() as u64, None),
            Err("the file ends before its data chunk".to_owned())
        );
        assert!(read(&file(PCM, 0, 8000, 16, 0, &[], &[])).is_err());
    }
}
