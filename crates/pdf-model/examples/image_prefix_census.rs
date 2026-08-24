//! What `Document::image_stream` decodes, and how often one page asks it for the same bytes.
//!
//! ISO 32000-2 §7.4 lets an image's `/Filter` be a chain whose last entry is a codec —
//! `[/ASCIIHexDecode /JBIG2Decode]` is §7.4.7's own worked example — and
//! [`pdf_syntax::Document::image_stream`] runs everything *before* the codec, handing the codec's
//! own bytes back. Where there is no codec at all, "everything before it" is the whole chain, so
//! that call inflates the samples themselves.
//!
//! ```sh
//! cargo run --release -p pdf-model --example image_prefix_census -- doc/pdf.js/test/pdfs
//! ```
//!
//! # What it counts, and the factor it deliberately does not
//!
//! **The population** is every image `XObject` in the file whose prefix chain runs a filter — the
//! only ones for which that call costs more than an `Arc::clone` — with what the prefix produces,
//! grouped by the chain as `/Filter` writes it. That is one pass over every image in the file, and
//! it is what a memo would hold.
//!
//! **The multiplier is not here, and that is deliberate**: how many times one interpretation asks
//! for the same image is a property of *this tree* rather than of the file, so it belongs to a
//! measurement of the tree. ADR 0585 took it with `callgrind_interpret` and
//! `viewer-core/examples/find_cost`, which is the instrument that can see it. Multiplying a static
//! count by a number read out of the source is how a price comes to describe code that has moved.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus's images; a measurement rather than a shipped path"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Object, ObjectId};

/// Every `.pdf` under the directories named on the command line.
fn corpus(args: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for arg in args {
        let root = Path::new(arg);
        if root.is_file() {
            files.push(root.to_path_buf());
            continue;
        }
        let Ok(entries) = std::fs::read_dir(root) else {
            eprintln!("cannot read {}", root.display());
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "pdf") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Table 5's `/Filter`, which is a name or an array of them; `Document::filter_chain` is the
/// shipped reading and is private, so this is the census's own and deliberately the simple case.
fn filter_chain(document: &Document, stream: &pdf_syntax::Stream) -> Vec<Vec<u8>> {
    match document.get_key(&stream.dict, "Filter") {
        Object::Name(name) => vec![name.as_bytes().to_vec()],
        Object::Array(items) => items
            .iter()
            .filter_map(|item| match document.resolve(item) {
                Object::Name(name) => Some(name.as_bytes().to_vec()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args = if args.is_empty() {
        vec![
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../doc/pdf.js/test/pdfs")
                .to_string_lossy()
                .into_owned(),
        ]
    } else {
        args
    };

    // chain (as written) -> (images, encoded bytes, bytes the prefix produces)
    let mut chains: BTreeMap<String, (usize, u64, u64)> = BTreeMap::new();
    let mut documents_with_work = 0usize;
    let mut documents = 0usize;
    let mut worked = 0usize;
    let mut images = 0usize;
    let mut prefix_bytes = 0u64;
    // The largest single prefix decode, and where it is.
    let mut largest = (0u64, String::new());
    // Per document, so that a witness page can be chosen rather than guessed at.
    let mut per_document: Vec<(u64, String, usize)> = Vec::new();

    for path in corpus(&args) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents += 1;
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let mut here = 0usize;
        let mut here_bytes = 0u64;
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            let Some(stream) = object.as_stream() else {
                continue;
            };
            if !matches!(
                document.get_key(&stream.dict, "Subtype"),
                Object::Name(ref n) if n.as_bytes() == b"Image"
            ) {
                continue;
            }
            images += 1;
            let filters = filter_chain(&document, stream);
            let codec_at = filters
                .len()
                .checked_sub(1)
                .filter(|last| pdf_syntax::filter::is_image_codec(&filters[*last]));
            let prefix = codec_at.unwrap_or(filters.len());
            if prefix == 0 {
                continue;
            }
            let Some(source) = document.image_stream(stream) else {
                continue;
            };
            worked += 1;
            here += 1;
            let produced = source.data.len() as u64;
            prefix_bytes += produced;
            here_bytes += produced;
            if produced > largest.0 {
                largest = (produced, format!("{name} object {number}"));
            }
            let written: Vec<String> = filters
                .iter()
                .map(|f| String::from_utf8_lossy(f.as_slice()).into_owned())
                .collect();
            let entry = chains
                .entry(format!("[{}]", written.join(" ")))
                .or_default();
            entry.0 += 1;
            entry.1 += stream.data.len() as u64;
            entry.2 += produced;
        }
        if here > 0 {
            documents_with_work += 1;
            per_document.push((here_bytes, name, here));
        }
    }
    per_document.sort_by_key(|entry| std::cmp::Reverse(entry.0));

    println!("{documents} documents, {images} image XObjects");
    println!(
        "{worked} of them run a filter before the codec, in {documents_with_work} documents, \
         producing {prefix_bytes} B"
    );
    println!(
        "largest single prefix decode: {} B, {}",
        largest.0, largest.1
    );
    println!();
    for (chain, (count, encoded, produced)) in &chains {
        println!("{chain}: {count} images, {encoded} B encoded -> {produced} B");
    }
    println!();
    println!("the heaviest documents, which is where a witness page comes from:");
    for (bytes, name, count) in per_document.iter().take(15) {
        println!("  {name}: {count} images, {bytes} B");
    }
}
