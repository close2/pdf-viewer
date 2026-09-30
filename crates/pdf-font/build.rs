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
    unicode_tables();
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

/// Reads one Unicode Character Database file from `data/unicode/`, its comments and blank lines
/// dropped and each remaining line split at its semicolons, trimmed.
fn ucd(name: &str) -> Vec<Vec<String>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/unicode")
        .join(name);
    println!("cargo::rerun-if-changed={}", path.display());
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("data/unicode/{name} is readable: {error}"));
    text.lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.split(';')
                .map(|field| field.trim().to_owned())
                .collect()
        })
        .collect()
}

/// A character as a Rust literal that is its escape, so the generated file is ASCII and no
/// combining mark or compatibility character sits in it unescaped.
fn literal(character: char) -> String {
    format!("'\\u{{{:x}}}'", u32::from(character))
}

/// A code point field, `0644` or `0600..0605`, as its first and last scalar values.
fn points(field: &str) -> (char, char) {
    let scalar = |hex: &str| {
        u32::from_str_radix(hex, 16)
            .ok()
            .and_then(char::from_u32)
            .unwrap_or_else(|| panic!("{hex} is not a scalar value"))
    };
    match field.split_once("..") {
        Some((first, last)) => (scalar(first), scalar(last)),
        None => (scalar(field), scalar(field)),
    }
}

/// Writes `unicode.rs` into `OUT_DIR`: the four tables `shaping` reads, from the UCD's own files.
///
/// Compiled in rather than parsed at launch, which is `CLAUDE.md`'s rule for data (ADR 1086's
/// precedent for `CaseFolding.txt`); ADR 1414 says which fields of which file each table is.
fn unicode_tables() {
    let mut out = String::new();

    // DerivedJoiningType.txt: every code point whose `Joining_Type` is not U, as ranges.
    let mut joining: Vec<(char, char, char)> = ucd("DerivedJoiningType.txt")
        .iter()
        .map(|fields| {
            let (first, last) = points(&fields[0]);
            let kind = fields[1]
                .chars()
                .next()
                .filter(|kind| matches!(kind, 'R' | 'L' | 'D' | 'C' | 'T'))
                .unwrap_or_else(|| {
                    panic!("DerivedJoiningType.txt: {} is no joining type", fields[1])
                });
            (first, last, kind)
        })
        .collect();
    joining.sort_unstable();
    let _ = writeln!(
        out,
        "/// `DerivedJoiningType.txt`: first, last, and the `Joining_Type` letter, sorted.\n\
         pub(crate) static JOINING_TYPES: [(char, char, u8); {}] = [",
        joining.len()
    );
    for (first, last, kind) in &joining {
        let _ = writeln!(
            out,
            "    ({}, {}, b'{kind}'),",
            literal(*first),
            literal(*last)
        );
    }
    out.push_str("];\n");

    // ArabicShaping.txt's fourth field, for the two joining groups the obligatory ligature names.
    let shaping = ucd("ArabicShaping.txt");
    for group in ["LAM", "ALEF"] {
        let mut members: Vec<char> = shaping
            .iter()
            .filter(|fields| fields.get(3).is_some_and(|field| field == group))
            .map(|fields| points(&fields[0]).0)
            .collect();
        members.sort_unstable();
        let _ = writeln!(
            out,
            "/// `ArabicShaping.txt`: the characters of `Joining_Group` {group}, sorted.\n\
             pub(crate) static {group}_GROUP: [char; {}] = [{}];",
            members.len(),
            members
                .iter()
                .map(|member| literal(*member))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    presentation_tables(&mut out);
    mirroring_table(&mut out);

    let target = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR for a build script"),
    );
    std::fs::write(target.join("unicode.rs"), out).expect("the tables are writable");
}

/// `UnicodeData.txt`'s presentation forms, as the two tables `shaping::joining` reads.
fn presentation_tables(out: &mut String) {
    // UnicodeData.txt's decomposition field over the two presentation-form blocks: a single
    // character under one of the four positional tags is that character's form, and two
    // characters under one are a ligature of them.
    let mut forms: Vec<(char, u8, char)> = Vec::new();
    let mut ligatures: Vec<(char, char, u8, char)> = Vec::new();
    for fields in ucd("UnicodeData.txt") {
        // Filtered on the number before it is made a `char`: the file lists the surrogate
        // ranges too, which are code points and not scalar values.
        let number = u32::from_str_radix(&fields[0], 16)
            .unwrap_or_else(|error| panic!("UnicodeData.txt: {}: {error}", fields[0]));
        if !matches!(number, 0xfb50..=0xfdff | 0xfe70..=0xfeff) {
            continue;
        }
        let (point, _) = points(&fields[0]);
        let Some((tag, rest)) = fields[5]
            .strip_prefix('<')
            .and_then(|field| field.split_once('>'))
        else {
            continue;
        };
        let form = match tag {
            "isolated" => 0,
            "final" => 1,
            "initial" => 2,
            "medial" => 3,
            _ => continue,
        };
        let parts: Vec<char> = rest.split_whitespace().map(|part| points(part).0).collect();
        match parts.as_slice() {
            [base] => forms.push((*base, form, point)),
            [first, second] => ligatures.push((*first, *second, form, point)),
            _ => {}
        }
    }
    forms.sort_unstable();
    for pair in forms.windows(2) {
        assert!(
            (pair[0].0, pair[0].1) != (pair[1].0, pair[1].1),
            "UnicodeData.txt states two presentation forms for {:?} in one position",
            pair[0].0
        );
    }
    let _ = writeln!(
        out,
        "/// `UnicodeData.txt`: base character, position (0 isolated, 1 final, 2 initial,\n\
         /// 3 medial) and the presentation form decomposing to it under that tag, sorted.\n\
         pub(crate) static PRESENTATION_FORMS: [(char, u8, char); {}] = [",
        forms.len()
    );
    for (base, form, point) in &forms {
        let _ = writeln!(
            out,
            "    ({}, {form}, {}),",
            literal(*base),
            literal(*point)
        );
    }
    out.push_str("];\n");
    ligatures.sort_unstable();
    let _ = writeln!(
        out,
        "/// `UnicodeData.txt`: the two-character presentation forms, as first, second, position\n\
         /// and the form, sorted.\n\
         pub(crate) static PRESENTATION_LIGATURES: [(char, char, u8, char); {}] = [",
        ligatures.len()
    );
    for (first, second, form, point) in &ligatures {
        let _ = writeln!(
            out,
            "    ({}, {}, {form}, {}),",
            literal(*first),
            literal(*second),
            literal(*point)
        );
    }
    out.push_str("];\n");
}

/// `BidiMirroring.txt`, as the table `shaping::bidi` reads for rule L4.
fn mirroring_table(out: &mut String) {
    // BidiMirroring.txt: `Bidi_Mirroring_Glyph`, the character whose glyph mirrors another's.
    let mut mirrors: Vec<(char, char)> = ucd("BidiMirroring.txt")
        .iter()
        .map(|fields| (points(&fields[0]).0, points(&fields[1]).0))
        .collect();
    mirrors.sort_unstable();
    let _ = writeln!(
        out,
        "/// `BidiMirroring.txt`: a character and the one whose glyph is its mirror image, sorted.\n\
         pub(crate) static MIRRORING: [(char, char); {}] = [{}];",
        mirrors.len(),
        mirrors
            .iter()
            .map(|(from, to)| format!("({}, {})", literal(*from), literal(*to)))
            .collect::<Vec<_>>()
            .join(", ")
    );
}
