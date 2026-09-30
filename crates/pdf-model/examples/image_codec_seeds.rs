//! Writes every `JBIG2Decode` and `JPXDecode` codestream a set of documents holds as seeds for the
//! `jbig2` and `jpx` fuzz targets, in the framing each target reads.
//!
//! A codec fuzzed from nothing never forms a segment header or a box, so each of those targets
//! needs real codestreams to mutate, and the documents on this disk are where they are. The
//! framing is the targets' own: `jbig2` reads a two-byte big-endian length, the `/JBIG2Globals`
//! stream (Table 12) and then the page's segments; `jpx` reads one request byte and then the file.
//! Seeds are named by content so a re-run adds only what is new, and a codestream larger than
//! `--max` bytes (256 KiB by default, `page`'s ceiling for the same reason) is skipped.
//!
//! ```sh
//! cargo run --release -p pdf-model --example image_codec_seeds -- \
//!     <jbig2 corpus dir> <jpx corpus dir> [--max <bytes>] <pdf>...
//! ```
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(
    clippy::arithmetic_side_effects,
    reason = "counters over one directory's codestreams and an index into the argument list; a \
              seeding tool rather than a shipped path"
)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use pdf_syntax::Document;

/// Writes `bytes` under `dir`, named by a hash of them; whether it was new.
fn seed(dir: &Path, bytes: &[u8]) -> bool {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let path = dir.join(format!("{:016x}", hasher.finish()));
    if path.exists() {
        return false;
    }
    std::fs::write(&path, bytes).expect("the corpus directory is writable");
    true
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut max = 256 * 1024;
    if let Some(at) = args.iter().position(|arg| arg == "--max") {
        max = args
            .get(at + 1)
            .and_then(|value| value.parse().ok())
            .expect("--max takes a byte count");
        args.drain(at..at + 2);
    }
    let mut args = args.into_iter();
    let jbig2_dir = PathBuf::from(args.next().expect("the jbig2 corpus directory"));
    let jpx_dir = PathBuf::from(args.next().expect("the jpx corpus directory"));
    std::fs::create_dir_all(&jbig2_dir).expect("the jbig2 directory is creatable");
    std::fs::create_dir_all(&jpx_dir).expect("the jpx directory is creatable");

    let (mut jbig2, mut jpx, mut skipped) = (0usize, 0usize, 0usize);
    for path in args {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        for number in document.xref().object_numbers() {
            let object = document.get(pdf_syntax::ObjectId::new(number, 0));
            let Some(stream) = object.as_stream() else {
                continue;
            };
            let Some(image) = document.image_stream(stream) else {
                continue;
            };
            let Some(codec) = image.codec.as_deref() else {
                continue;
            };
            if image.data.len() > max {
                skipped += 1;
                continue;
            }
            match codec {
                b"JBIG2Decode" => {
                    let globals = image
                        .parms
                        .as_ref()
                        .map(|parms| document.get_key(parms, "JBIG2Globals"))
                        .and_then(|globals| {
                            globals
                                .as_stream()
                                .and_then(|stream| document.decoded_stream_data(stream))
                        })
                        .map(|data| data.to_vec())
                        .unwrap_or_default();
                    let Ok(length) = u16::try_from(globals.len()) else {
                        skipped += 1;
                        continue;
                    };
                    let mut framed = length.to_be_bytes().to_vec();
                    framed.extend_from_slice(&globals);
                    framed.extend_from_slice(&image.data);
                    jbig2 += usize::from(seed(&jbig2_dir, &framed));
                }
                b"JPXDecode" => {
                    let mut framed = vec![0u8];
                    framed.extend_from_slice(&image.data);
                    jpx += usize::from(seed(&jpx_dir, &framed));
                }
                _ => {}
            }
        }
    }
    println!(
        "image_codec_seeds: {jbig2} new jbig2 seeds, {jpx} new jpx seeds, {skipped} over --max or a globals stream past 64 KiB"
    );
}
