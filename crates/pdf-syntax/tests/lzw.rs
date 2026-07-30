//! `LZWDecode` against real streams, checked without asking another decoder.
//!
//! ISO 32000-2 §7.4.4.2 states the whole algorithm and supplies a worked example, which is
//! what `filter.rs`'s unit tests decode. This file asks the other question: does it work on
//! data a real producer wrote, at a size where a subtle desynchronisation would show?
//!
//! **No reference decoder is involved, and none is needed**, because each of these streams
//! says how long it should be and what it should contain — in its *own document*, in
//! dictionaries that were written by the same producer and compressed separately:
//!
//! - An image's `/Width`, `/Height` and `/BitsPerComponent` fix the decoded length exactly.
//!   57 × 78 samples at eight bits is 4446 bytes and nothing else.
//! - An `[/Indexed /DeviceRGB 255 …]` lookup stream is 256 entries of three components, so
//!   768 bytes and nothing else.
//! - A `/ToUnicode` stream is a `CMap` program, and every `CMap` begins with the same
//!   PostScript preamble.
//!
//! An LZW decoder that gets the code width wrong produces plausible bytes from the point of
//! the error onward and a *different length*, because the codes after it name table entries
//! of other lengths. So a length that matches to the byte over four thousand of them is a
//! strong statement, and the three fixtures reach different parts of the algorithm: 4446
//! bytes from 469 is deep enough to grow the code width twice, 768 from 488 exercises long
//! runs, and 247 from 192 is a stream that barely compresses.
//!
//! This is the shape `doc/HANDOVER.md` calls *a corpus stating an invariant about itself*,
//! and it is worth more here than any reference: `LZWDecode` is not reached by any corpus
//! page one, so neither gate can see it at all.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code, and deliberately loud: a missing corpus is a skip, a corpus that \
              does not contain what the test names is a failure"
)]

use std::path::PathBuf;

use pdf_syntax::{Document, Object, ObjectId};

/// The corpus, or `None` where the submodule is not checked out.
fn corpus() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    root.is_dir().then_some(root)
}

/// Opens a named corpus document. Present-but-unreadable is a failure, not a skip.
fn open(name: &str) -> Option<Document> {
    let path = corpus()?.join(name);
    let Ok(bytes) = std::fs::read(&path) else {
        panic!("the corpus is present but {} is missing", path.display());
    };
    Some(Document::open(bytes).expect("the document opens"))
}

/// Every stream of a document whose filter chain names `LZWDecode`, decoded.
fn lzw_streams(document: &Document) -> Vec<(u32, Vec<u8>)> {
    let mut out = Vec::new();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let Object::Stream(stream) = &object else {
            continue;
        };
        let filter = document.get_key(&stream.dict, "Filter");
        let names: Vec<Vec<u8>> = match &filter {
            Object::Name(name) => vec![name.as_bytes().to_vec()],
            Object::Array(items) => items
                .iter()
                .filter_map(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
                .collect(),
            _ => Vec::new(),
        };
        if !names.iter().any(|name| name == b"LZWDecode") {
            continue;
        }
        let data = document
            .decoded_stream_data(stream)
            .unwrap_or_else(|| panic!("object {number} declares LZWDecode and did not decode"));
        out.push((number, data.to_vec()));
    }
    out
}

/// An LZW-compressed image decodes to exactly the number of samples its dictionary states.
///
/// `XiaoBiaoSong.pdf` writes `[/ASCII85Decode /LZWDecode]`, so this also checks that the
/// pipeline runs in the right order — §7.4.1's "cascaded to form a pipeline that passes the
/// stream through two or more decoding transformations in sequence".
#[test]
fn an_lzw_image_decodes_to_exactly_its_declared_sample_count() {
    let Some(document) = open("XiaoBiaoSong.pdf") else {
        println!("skipped: the pdf.js corpus submodule is not checked out");
        return;
    };
    let streams = lzw_streams(&document);
    assert!(
        !streams.is_empty(),
        "XiaoBiaoSong.pdf is named here because it carries LZW streams"
    );

    let mut checked = 0usize;
    for (number, data) in &streams {
        let object = document.get(ObjectId {
            number: *number,
            generation: 0,
        });
        let Object::Stream(stream) = &object else {
            continue;
        };
        let (Some(width), Some(height), Some(bits)) = (
            document.get_key(&stream.dict, "Width").as_integer(),
            document.get_key(&stream.dict, "Height").as_integer(),
            document
                .get_key(&stream.dict, "BitsPerComponent")
                .as_integer(),
        ) else {
            continue;
        };
        assert_eq!(bits, 8, "the fixture's image is eight bits per component");
        let expected = usize::try_from(width * height).expect("a small image");
        assert_eq!(
            data.len(),
            expected,
            "object {number}: a {width}x{height} eight-bit image is {expected} bytes"
        );
        checked += 1;
    }
    assert_eq!(checked, 1, "one image, and it was found");
}

/// An LZW-compressed colour table decodes to exactly `(hival + 1) x components` bytes.
#[test]
fn an_lzw_colour_table_decodes_to_exactly_its_declared_size() {
    let Some(document) = open("XiaoBiaoSong.pdf") else {
        println!("skipped: the pdf.js corpus submodule is not checked out");
        return;
    };
    // `[/Indexed /DeviceRGB 255 9 0 R]`: 256 entries of three components.
    let table = lzw_streams(&document)
        .into_iter()
        .find(|(_, data)| data.len() != 4446)
        .expect("the document's other LZW stream is its colour table");
    assert_eq!(
        table.1.len(),
        768,
        "an /Indexed /DeviceRGB space with hival 255 has 256 entries of three components"
    );
}

/// An LZW-compressed `/ToUnicode` stream decodes to a `CMap` program.
///
/// A `/ToUnicode` stream is a `CMap`, and every `CMap` begins with the same PostScript
/// preamble. Thirty-six bytes of exactly the right text cannot come out of a decoder that is
/// even one code out of step.
#[test]
fn an_lzw_tounicode_stream_decodes_to_a_cmap_program() {
    let Some(document) = open("bug864847.pdf") else {
        println!("skipped: the pdf.js corpus submodule is not checked out");
        return;
    };
    let streams = lzw_streams(&document);
    assert_eq!(
        streams.len(),
        1,
        "bug864847.pdf is named here because it carries exactly one LZW stream"
    );
    let (_, data) = streams.first().expect("checked above");
    let head = String::from_utf8_lossy(data.get(..36).unwrap_or_default()).into_owned();
    assert!(
        head.starts_with("/CIDInit /ProcSet findresource begin"),
        "a /ToUnicode stream is a CMap program, and this one starts {head:?}"
    );
    assert!(
        data.windows(9)
            .any(|window| window == b"endcmap\r\n" || window == b"endcmap\n\n")
            || data.ends_with(b"end\n")
            || String::from_utf8_lossy(data).contains("endcmap"),
        "and it ends like one"
    );
}
