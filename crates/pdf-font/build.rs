//! Packs `data/cmaps/` into one compressed blob and an index into it.
//!
//! ISO 32000-2 §9.7.5.2's predefined `CMap`s are 12 MB of PostScript, and `CLAUDE.md`'s startup
//! rule forbids parsing — or decompressing — any of it at launch. So each file is deflated on
//! its own here, concatenated, and named by an index of `(name, offset, length)`: opening a
//! document that wants `90ms-RKSJ-H` inflates 24 KB and touches nothing else, and a document
//! that wants none of them pays for a `static` array of 239 tuples.
//!
//! Deflate rather than a denser coder because `flate2` is already in this tree for §7.4.4 and a
//! second compressor would be a dependency bought for build-time data. The blob is 1.5 MB.
//!
//! This follows `pdf-spec`'s precedent — generated data written to `OUT_DIR` by a checked-in
//! script — with one difference: the input is committed here rather than read from a submodule,
//! for the reason `data/standard-fonts/PROVENANCE.md` gives about optional submodules.

// A build script's job is to abort the build when its input is malformed, and the panic
// message is the diagnostic a developer reads — the same argument `pdf-spec/build.rs` makes.
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "aborting the build is the intended and only useful failure mode here"
)]

use std::fmt::Write as _;
use std::io::Write as _;

/// A number written with `_` every three digits, because the generated file is linted like
/// every other and `clippy::unreadable_literal` is part of `pedantic`.
fn grouped(value: usize) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && digits.len().saturating_sub(at).is_multiple_of(3) {
            out.push('_');
        }
        out.push(digit);
    }
    out
}

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cmaps");
    println!("cargo::rerun-if-changed={}", root.display());

    let mut names: Vec<String> = std::fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("data/cmaps is readable: {error}"))
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        // The licence and the digests sit beside the data and are not data.
        // What sits beside the data and is not data.
        .filter(|name| !matches!(name.as_str(), "LICENSE_ADOBE" | "SHA256SUMS" | "PROVENANCE.md"))
        .collect();
    names.sort();
    assert!(
        !names.is_empty(),
        "data/cmaps holds no CMap, so no predefined CMap would resolve and nothing would say so"
    );

    let mut blob: Vec<u8> = Vec::new();
    let mut index = String::new();
    for name in &names {
        let bytes = std::fs::read(root.join(name))
            .unwrap_or_else(|error| panic!("{name} is readable: {error}"));
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
        encoder
            .write_all(&bytes)
            .unwrap_or_else(|error| panic!("{name} deflates: {error}"));
        let packed = encoder
            .finish()
            .unwrap_or_else(|error| panic!("{name} deflates: {error}"));
        let _ = writeln!(
            index,
            "    ({:?}, {}, {}, {}),",
            name,
            grouped(blob.len()),
            grouped(packed.len()),
            grouped(bytes.len())
        );
        blob.extend_from_slice(&packed);
    }

    let out = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR for a build script"),
    );
    std::fs::write(out.join("cmaps.bin"), &blob).expect("the blob is writable");
    std::fs::write(
        out.join("cmaps.rs"),
        format!(
            "/// Every predefined `CMap` this binary carries: name, offset, packed length,\n\
             /// and the length it inflates to.\n\
             pub(crate) static PREDEFINED: [(&str, usize, usize, usize); {}] = [\n{index}];\n",
            names.len()
        ),
    )
    .expect("the index is writable");
}
