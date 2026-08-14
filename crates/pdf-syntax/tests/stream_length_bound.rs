//! Two things a filter can fail to give back, and why they are not the same thing.
//!
//! A stream that is *damaged* has given everything it had, and keeping the prefix is right:
//! "a partially-inflated content stream still renders most of a page" is the reason `flate` and
//! `lzw` salvage. A stream stopped by [`pdf_syntax::Limits::max_stream_len`] has a great deal
//! more to give and this reader declined to take it, which is a refusal and has to say so.
//!
//! **One code path served both until the four-hundred-and-seventy-first session**, and the
//! mechanism is worth stating because nothing about it looks like a bug: `io::Take` yields
//! end-of-file at its limit, and `read_to_end` reports end-of-file as `Ok`. So a decompression
//! bomb came back as a complete decode of its own first two gibibytes, with nothing reported —
//! `doc/HANDOVER.md` trap 5's rule broken by the one guard in `filter.rs` that had a plausible
//! excuse. `ASCII85Decode` and `RunLengthDecode` already refused properly, which is what made
//! the inconsistency findable at all. ADR 0306.
//!
//! Every fixture is generated and none is larger than a few hundred bytes: the bound under test
//! is a parameter, so a test may set it to eight rather than build two gibibytes to reach it.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that does not decode should fail loudly, and every fixture \
              here is a few hundred bytes"
)]

use pdf_syntax::{Damage, FilterRefusal, Limits, filter};

/// The default bounds with one of them moved.
fn bounded(max_stream_len: usize) -> Limits {
    Limits {
        max_stream_len,
        ..Limits::DEFAULT
    }
}

/// Deflates with the same library the decoder uses, which is what a producer would have done.
fn deflate(data: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(data).expect("in-memory write");
    encoder.finish().expect("finish")
}

/// Packs LZW codes high-order bit first, which is how §7.4.4.2 says they are written.
fn pack_codes(codes: &[u16], width: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut held: u32 = 0;
    let mut bits: u32 = 0;
    for code in codes {
        held = (held << width) | (u32::from(*code) & ((1u32 << width) - 1));
        bits += width;
        while bits >= 8 {
            out.push(((held >> (bits - 8)) & 0xFF) as u8);
            bits -= 8;
        }
    }
    if bits > 0 {
        out.push(((held << (8 - bits)) & 0xFF) as u8);
    }
    out
}

/// A deflate stream past the bound is refused by name, and hands back nothing.
///
/// The regression test for the defect itself. Before it, this call returned the first `limit`
/// bytes and `Ok`.
#[test]
fn flate_past_the_bound_is_refused_rather_than_clamped() {
    let compressed = deflate(&[b'a'; 4096]);
    let refusal = filter::decode_reported(b"FlateDecode", &compressed, None, bounded(64))
        .expect_err("4096 bytes past a bound of 64 must be refused");
    assert_eq!(refusal, FilterRefusal::TooLarge { limit: 64 });
    assert!(
        filter::decode(b"FlateDecode", &compressed, None, bounded(64)).is_none(),
        "and the byte-returning entry point must not hand back the clamped prefix"
    );
}

/// A deflate stream exactly at the bound is a complete decode, not a refusal.
///
/// The control, and it is the edge the ceiling is placed at: the decoder is given one byte more
/// than the bound so that reaching the bound and passing it are different events.
#[test]
fn flate_exactly_at_the_bound_still_decodes() {
    let compressed = deflate(&[b'a'; 64]);
    let decoded = filter::decode_reported(b"FlateDecode", &compressed, None, bounded(64))
        .expect("64 bytes under a bound of 64 decode");
    assert_eq!(decoded.data.len(), 64);
    assert_eq!(
        decoded.damage, None,
        "a whole stream that lands exactly on the bound is not damaged by landing there"
    );
}

/// A *truncated* deflate stream still hands back what it inflated, **and says that it did**.
///
/// The behaviour that must survive the fix, and the reason the two cases needed separating
/// rather than one of them being deleted. The `damage` half is ADR 0343's: keeping the prefix
/// was right and keeping it in silence was not, because `read_to_end` answers `Ok` both when
/// RFC 1951's final block was read and when the input merely ran out.
#[test]
fn flate_truncated_keeps_what_it_inflated_and_says_so() {
    let compressed = deflate(&[b'a'; 4096]);
    let cut = compressed
        .get(..compressed.len() - 4)
        .expect("the fixture is longer than four bytes");
    let decoded = filter::decode_reported(b"FlateDecode", cut, None, Limits::DEFAULT)
        .expect("a truncated stream keeps its partial output");
    assert!(
        !decoded.data.is_empty() && decoded.data.iter().all(|&byte| byte == b'a'),
        "the salvaged prefix is the bytes the encoder did emit"
    );
    assert_eq!(
        decoded.damage,
        Some(Damage::Truncated),
        "and a prefix handed over as though it were the whole stream is trap 5's fallback"
    );
}

/// The same pair for `LZWDecode`, whose guard had the same shape.
#[test]
fn lzw_past_the_bound_is_refused_rather_than_clamped() {
    // Ten literal codes, each one byte of output, against a bound of four.
    let packed = pack_codes(&[65, 66, 67, 68, 69, 70, 71, 72, 73, 74], 9);
    let refusal = filter::decode_reported(b"LZWDecode", &packed, None, bounded(4))
        .expect_err("ten bytes past a bound of four must be refused");
    assert_eq!(refusal, FilterRefusal::TooLarge { limit: 4 });
    assert!(
        filter::decode(b"LZWDecode", &packed, None, bounded(4)).is_none(),
        "and no clamped prefix comes back"
    );
}

/// A corrupt LZW stream keeps what it decoded, which is §7.4.4.2's other case.
#[test]
fn lzw_corrupt_keeps_what_it_decoded() {
    // Two literals, then a code past the end of the table.
    let packed = pack_codes(&[65, 66, 400], 9);
    let decoded = filter::decode_reported(b"LZWDecode", &packed, None, Limits::DEFAULT)
        .expect("partial output");
    assert_eq!(&*decoded.data, b"AB");
    assert_eq!(
        decoded.damage,
        Some(Damage::Corrupt),
        "a code past the end of the table is damage in the file, not the end of the data"
    );
}

/// The two filters that always refused properly answer with the same variant.
///
/// They are the evidence that made the other two findable: nothing had to be invented for
/// `flate` and `lzw`, only made to agree with the two beside them.
#[test]
fn ascii85_and_run_length_name_the_same_bound() {
    // `z` stands for four zero bytes, so eight of them are thirty-two.
    let refusal = filter::decode_reported(b"ASCII85Decode", b"zzzzzzzz~>", None, bounded(8))
        .expect_err("thirty-two bytes past a bound of eight");
    assert_eq!(refusal, FilterRefusal::TooLarge { limit: 8 });

    // 254 repeats the next byte three times; ten of those runs are thirty bytes.
    let mut data = Vec::new();
    for _ in 0..10 {
        data.extend_from_slice(&[254, b'x']);
    }
    data.push(128);
    let refusal = filter::decode_reported(b"RunLengthDecode", &data, None, bounded(8))
        .expect_err("thirty bytes past a bound of eight");
    assert_eq!(refusal, FilterRefusal::TooLarge { limit: 8 });
}

/// An unsupported filter and a bound are different answers, and a caller can tell.
///
/// The whole point of [`FilterRefusal`] having three variants rather than being a `bool`: an
/// image codec is something this module does not do, and a bound is something it declined to
/// do, and reporting the second as the first would say the file used a filter we lack.
#[test]
fn an_image_codec_is_not_a_bound() {
    let refusal = filter::decode_reported(b"JPXDecode", b"whatever", None, Limits::DEFAULT)
        .expect_err("an image codec belongs to the image pipeline");
    assert_eq!(refusal, FilterRefusal::Unsupported);
}
