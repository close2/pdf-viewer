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

use std::fmt::Write as _;

use pdf_syntax::{Damage, Document, FilterRefusal, Limits, ObjectId, filter};

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

/// §7.3.8.2's `/Length` when it is an indirect reference, which a parser cannot follow.
///
/// > Every stream dictionary shall have a Length entry that indicates how many bytes of the PDF
/// > file are used for the stream's data.
///
/// Table 5 makes it "(Required; shall be an indirect reference)" for a producer that does not
/// know the length until the data is written, so this is a route the standard *requires* rather
/// than an oddity — and `Parser` cannot take it, because resolving a reference needs the document
/// that parsing builds. Its fallback is §7.3.8.1's delimiter: search for `endstream` and drop one
/// preceding end-of-line, which "there **should** be". Where a producer wrote none, the byte
/// dropped is the data's own.
///
/// **The pair differs only in whether `/Length` is written directly**, and both files' data is
/// the same six bytes ending in a newline with `endstream` hard against them. Two of the 65 944
/// crawled documents are this shape, and what made them findable is that the lost byte was the
/// last of a `FlateDecode` stream's final block: the stream then reads as *damaged* while being
/// whole, which ADR 0366's object-stream rule turned into a refusal before this was found.
#[test]
fn an_indirect_length_is_the_files_own_statement_of_where_a_stream_ends() {
    let build = |indirect: bool| {
        let length = if indirect { "5 0 R" } else { "6" };
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for body in [
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>".to_owned(),
            // No end-of-line between the data and `endstream`: §7.3.8.1 asks for one with a
            // *should*, and a file that omits it is the case under test.
            format!("<< /Length {length} >>\nstream\nhello\nendstream"),
            "6".to_owned(),
        ] {
            offsets.push(out.len());
            let number = offsets.len();
            let _ = write!(out, "{number} 0 obj\n{body}\nendobj\n");
        }
        let table_at = out.len();
        out.push_str("xref\n0 6\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{table_at}\n%%EOF\n"
        );
        let document = Document::open(out.into_bytes()).expect("the fixture's objects are intact");
        let object = document.get(ObjectId {
            number: 4,
            generation: 0,
        });
        object
            .as_stream()
            .expect("object 4 is the stream")
            .data
            .to_vec()
    };

    assert_eq!(
        build(false),
        b"hello\n".to_vec(),
        "a direct length is the parser's own answer and states six bytes"
    );
    assert_eq!(
        build(true),
        b"hello\n".to_vec(),
        "and an indirect one states the same six: the reference is resolved rather than \
         guessed at, so the data's own last byte is not read as the delimiter's"
    );
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
