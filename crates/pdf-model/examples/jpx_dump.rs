//! Writes one document's `/JPXDecode` codestreams out as files, for work outside this tree.
//!
//! `tests/jpeg2000.rs` compares them against ISO/IEC 15444-5's reference software and holds
//! thirteen that differ. Diagnosing one of those means decoding it repeatedly, at different
//! resolution levels, against `opj_decompress` — which is easier with the codestream on disk
//! than inside a PDF.
//!
//! ```sh
//! cargo run --release -p pdf-model --example jpx_dump -- doc/pdf.js/test/pdfs/issue5475.pdf /tmp/jpx
//! ```
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout)]

use std::path::PathBuf;

use pdf_syntax::Document;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().expect("a PDF path"));
    let out = PathBuf::from(args.next().unwrap_or_else(|| "/tmp/jpx".into()));
    std::fs::create_dir_all(&out).expect("the output directory is writable");

    let stem = path.file_stem().expect("a file name").to_string_lossy();
    let bytes = std::fs::read(&path).expect("the document is readable");
    let document = Document::open(bytes).expect("it opens");

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
        if image
            .codec
            .as_ref()
            .is_none_or(|name| name.as_slice() != b"JPXDecode")
        {
            continue;
        }
        let target = out.join(format!("{stem}-{number}.j2k"));
        std::fs::write(&target, &image.data).expect("the codestream is writable");
        println!("{} ({} bytes)", target.display(), image.data.len());
    }
}
