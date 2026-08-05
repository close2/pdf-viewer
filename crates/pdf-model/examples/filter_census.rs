//! How many corpus documents carry each sandboxed image codec, and how large each codestream is.
//!
//! The instrument behind `doc/todo/_image-codecs-and-the-sandbox.md`. Three filters reach
//! `pdf-sandbox` — §7.4.6's `CCITTFaxDecode`, §7.4.7's `JBIG2Decode`, §7.4.9's `JPXDecode` — and
//! any argument about whether they are worth a confined process starts with how much of the
//! corpus asks for them. Counted per document *and* per image, because the two disagree by a
//! factor of seventeen on one of the three.
//!
//! ```sh
//! cargo run --release -p pdf-model --example filter_census
//! ```
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(
    clippy::arithmetic_side_effects,
    reason = "counters over 974 documents' images; the corpus is four orders of magnitude \
              below what a usize counts, and this is a measurement rather than a shipped path"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_syntax::Document;

fn corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("the submodule is checked out")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|e| e == "pdf"))
        .collect();
    files.sort();
    files
}

fn main() {
    let codecs = [
        &b"JBIG2Decode"[..],
        &b"JPXDecode"[..],
        &b"CCITTFaxDecode"[..],
    ];
    // codec -> (documents, images, largest pixel count, total pixels)
    let mut tally: BTreeMap<&[u8], (usize, usize, u64, u64)> = BTreeMap::new();
    let mut documents: BTreeMap<&[u8], Vec<String>> = BTreeMap::new();

    for path in corpus() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let mut seen: BTreeMap<&[u8], usize> = BTreeMap::new();
        for number in document.xref().object_numbers() {
            let object = document.get(pdf_syntax::ObjectId {
                number,
                generation: 0,
            });
            let Some(stream) = object.as_stream() else {
                continue;
            };
            let Some(image) = document.image_stream(stream) else {
                continue;
            };
            let Some(codec) = image.codec.as_ref() else {
                continue;
            };
            let Some(which) = codecs.iter().find(|c| **c == codec.as_slice()) else {
                continue;
            };
            let pixels = image.data.len() as u64;
            let entry = tally.entry(which).or_insert((0, 0, 0, 0));
            entry.1 += 1;
            entry.2 = entry.2.max(pixels);
            entry.3 += pixels;
            *seen.entry(which).or_insert(0) += 1;
        }
        for (which, count) in seen {
            tally.entry(which).or_insert((0, 0, 0, 0)).0 += 1;
            documents
                .entry(which)
                .or_default()
                .push(format!("{name} ({count})"));
        }
    }

    for (codec, (docs, images, largest, total)) in &tally {
        println!(
            "{}: {docs} documents, {images} images, largest codestream {largest} B, {total} B total",
            String::from_utf8_lossy(codec)
        );
    }
    println!();
    for (codec, names) in &documents {
        println!("{}: {}", String::from_utf8_lossy(codec), names.join(", "));
    }
}
