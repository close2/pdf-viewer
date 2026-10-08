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
    script_table();
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
    presentation_decompositions(&mut out);
    canonical_decompositions(&mut out);
    combining_marks(&mut out);
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

/// `UnicodeData.txt`'s decompositions of the presentation-form blocks, as the table
/// `shaping::fold` reads.
///
/// Alphabetic Presentation Forms (U+FB00 to U+FB4F) and Arabic Presentation Forms-A and -B
/// (U+FB50 to U+FDFF, U+FE70 to U+FEFF): every character there with a decomposition, under any
/// tag or none, expanded through the table itself until no part of it is a presentation form. A
/// decomposition holding U+0020 is left out — those are the isolated forms of the vowel marks,
/// a space and a mark, and folding one would put a word break where the page shows none.
fn presentation_decompositions(out: &mut String) {
    let mut raw: std::collections::BTreeMap<char, Vec<char>> = std::collections::BTreeMap::new();
    for fields in ucd("UnicodeData.txt") {
        let number = u32::from_str_radix(&fields[0], 16)
            .unwrap_or_else(|error| panic!("UnicodeData.txt: {}: {error}", fields[0]));
        if !matches!(number, 0xfb00..=0xfdff | 0xfe70..=0xfeff) || fields[5].is_empty() {
            continue;
        }
        let (point, _) = points(&fields[0]);
        let parts = fields[5]
            .split_once('>')
            .map_or(fields[5].as_str(), |(_, rest)| rest);
        let parts: Vec<char> = parts
            .split_whitespace()
            .map(|part| points(part).0)
            .collect();
        if parts.is_empty() || parts.contains(&' ') {
            continue;
        }
        raw.insert(point, parts);
    }
    let expand = |parts: &[char]| {
        let mut expanded: Vec<char> = parts.to_vec();
        // The blocks' own canonical decompositions nest one deep (a Hebrew letter with two
        // marks decomposes to the form with one); four rounds is more than the data needs.
        for _ in 0..4 {
            expanded = expanded
                .iter()
                .flat_map(|part| raw.get(part).cloned().unwrap_or_else(|| vec![*part]))
                .collect();
        }
        expanded
    };
    // The letters in one pool and each form an offset and a length into it, rather than a
    // `&str` per form: every pointer in a `static` is a relocation the dynamic loader applies
    // before `main`, and nine hundred of them were measured on `launch_path`'s instruction count
    // of an open (ADR 1465).
    let mut letters: Vec<char> = Vec::new();
    let mut entries: Vec<(char, usize, usize)> = Vec::new();
    for (point, parts) in &raw {
        let expanded = expand(parts);
        entries.push((*point, letters.len(), expanded.len()));
        letters.extend(expanded);
    }
    let _ = writeln!(
        out,
        "/// `UnicodeData.txt`: the letters the presentation forms decompose to, end to end.\n\
         pub(crate) static PRESENTATION_LETTERS: [char; {}] = [{}];",
        letters.len(),
        letters
            .iter()
            .map(|letter| literal(*letter))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(
        out,
        "/// `UnicodeData.txt`: a presentation form, and where its decomposition starts in\n\
         /// `PRESENTATION_LETTERS` and how long it is, sorted.\n\
         pub(crate) static PRESENTATION_DECOMPOSITIONS: [(char, u16, u8); {}] = [",
        entries.len()
    );
    for (point, start, length) in &entries {
        let start = u16::try_from(*start).expect("the pool is a few thousand letters");
        let length = u8::try_from(*length).expect("a decomposition is at most eighteen letters");
        let _ = writeln!(out, "    ({}, {start}, {length}),", literal(*point));
    }
    out.push_str("];\n");
}

/// `UnicodeData.txt`'s canonical decompositions, as the table `shaping::fold` reads to compare
/// two spellings the Unicode Standard calls the same text.
///
/// Field 5 of every line whose decomposition carries no `<tag>` — a tagged one is a compatibility
/// decomposition, which changes what the text says — expanded through the field itself until no
/// part decomposes further: `U+1EA5` is `U+00E2 U+0301` in the file and `a U+0302 U+0301` here.
/// The Hangul syllables are not in the field; the Unicode Standard decomposes them by arithmetic,
/// and `shaping::fold` does too. Letters in one pool, as the presentation table keeps them, so
/// the tables hold no pointer for the loader to relocate before `main` (ADR 1465).
fn canonical_decompositions(out: &mut String) {
    let mut raw: std::collections::BTreeMap<char, Vec<char>> = std::collections::BTreeMap::new();
    for fields in ucd("UnicodeData.txt") {
        if fields[5].is_empty() || fields[5].starts_with('<') {
            continue;
        }
        let (point, _) = points(&fields[0]);
        let parts: Vec<char> = fields[5]
            .split_whitespace()
            .map(|part| points(part).0)
            .collect();
        raw.insert(point, parts);
    }
    let mut letters: Vec<char> = Vec::new();
    let mut entries: Vec<(char, usize, usize)> = Vec::new();
    for (point, parts) in &raw {
        let mut expanded: Vec<char> = parts.clone();
        // The field nests at most three deep (a Greek letter with breathing, accent and
        // iota subscript); a fixed point is reached well inside eight rounds.
        for _ in 0..8 {
            expanded = expanded
                .iter()
                .flat_map(|part| raw.get(part).cloned().unwrap_or_else(|| vec![*part]))
                .collect();
        }
        assert!(
            expanded.iter().all(|part| !raw.contains_key(part)),
            "UnicodeData.txt: the decomposition of {point:?} did not reach a fixed point"
        );
        entries.push((*point, letters.len(), expanded.len()));
        letters.extend(expanded);
    }
    let _ = writeln!(
        out,
        "/// `UnicodeData.txt`: the characters the canonical decompositions expand to, end to end.\n\
         pub(crate) static CANONICAL_LETTERS: [char; {}] = [{}];",
        letters.len(),
        letters
            .iter()
            .map(|letter| literal(*letter))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(
        out,
        "/// `UnicodeData.txt`: a character with a canonical decomposition, and where its full\n\
         /// expansion starts in `CANONICAL_LETTERS` and how long it is, sorted.\n\
         pub(crate) static CANONICAL_DECOMPOSITIONS: [(char, u16, u8); {}] = [",
        entries.len()
    );
    for (point, start, length) in &entries {
        let start = u16::try_from(*start).expect("the pool is a few thousand characters");
        let length = u8::try_from(*length).expect("a canonical decomposition is a few characters");
        let _ = writeln!(out, "    ({}, {start}, {length}),", literal(*point));
    }
    out.push_str("];\n");
}

/// `UnicodeData.txt`'s nonspacing marks that canonical ordering moves, as the table
/// `shaping::fold` reads to tell a mark from a letter.
///
/// General category `Mn` (field 2) with a canonical combining class (field 3) other than zero,
/// as ranges of consecutive code points sharing one class. ADR 1477 is why the class is asked as
/// well as the category: a nonspacing mark of class zero — a Devanagari or Thai vowel sign — is
/// part of how the syllable is spelled rather than something a writer may leave off.
fn combining_marks(out: &mut String) {
    let mut ranges: Vec<(char, char, u8)> = Vec::new();
    for fields in ucd("UnicodeData.txt") {
        if fields[2] != "Mn" {
            continue;
        }
        let class: u8 = fields[3]
            .parse()
            .unwrap_or_else(|error| panic!("UnicodeData.txt: {}: {error}", fields[0]));
        if class == 0 {
            continue;
        }
        let (point, _) = points(&fields[0]);
        match ranges.last_mut() {
            Some((_, last, same))
                if *same == class && u32::from(*last).checked_add(1) == Some(u32::from(point)) =>
            {
                *last = point;
            }
            _ => ranges.push((point, point, class)),
        }
    }
    let _ = writeln!(
        out,
        "/// `UnicodeData.txt`: first, last, and the canonical combining class of every run of\n\
         /// nonspacing marks whose class is not zero, sorted.\n\
         pub(crate) static COMBINING_MARKS: [(char, char, u8); {}] = [",
        ranges.len()
    );
    for (first, last, class) in &ranges {
        let _ = writeln!(
            out,
            "    ({}, {}, {class}),",
            literal(*first),
            literal(*last)
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

/// Writes `scripts.rs` into `OUT_DIR`: `Scripts.txt`'s ranges under each one's ISO 15924 code, the
/// table `pairs` resolves a character's script by (ADR 1696).
///
/// `PropertyValueAliases.txt`'s `sc` rows give each `Script` value's short alias, which is the
/// ISO 15924 code the OpenType script tags are registered beside. `Common` and `Inherited` are
/// left out — the reader resolves both from their neighbours — and adjacent ranges of one script
/// are merged, so what is compiled in is the fewest ranges that say the same thing.
fn script_table() {
    let mut codes = std::collections::BTreeMap::new();
    for fields in ucd("PropertyValueAliases.txt") {
        if fields.first().is_some_and(|property| property == "sc") {
            codes.insert(fields[2].clone(), fields[1].clone());
        }
    }
    let mut ranges: Vec<(char, char, String)> = ucd("Scripts.txt")
        .iter()
        .filter(|fields| !matches!(fields[1].as_str(), "Common" | "Inherited"))
        .map(|fields| {
            let (first, last) = points(&fields[0]);
            let code = codes
                .get(&fields[1])
                .unwrap_or_else(|| panic!("Scripts.txt: {} has no sc alias", fields[1]));
            assert!(
                code.len() == 4 && code.is_ascii(),
                "{code} is no ISO 15924 code"
            );
            (first, last, code.clone())
        })
        .collect();
    ranges.sort_unstable();
    let mut merged: Vec<(char, char, String)> = Vec::new();
    for (first, last, code) in ranges {
        if let Some(previous) = merged.last_mut()
            && previous.2 == code
            && u32::from(previous.1).checked_add(1) == Some(u32::from(first))
        {
            previous.1 = last;
            continue;
        }
        merged.push((first, last, code));
    }
    let mut out = String::new();
    let _ = writeln!(
        out,
        "/// `Scripts.txt`: first, last, and the ISO 15924 code of the range's `Script`, sorted; \
         `Common` and `Inherited` are not listed.\n\
         pub(crate) static SCRIPTS: [(char, char, [u8; 4]); {}] = [",
        merged.len()
    );
    for (first, last, code) in &merged {
        let _ = writeln!(
            out,
            "    ({}, {}, *b\"{code}\"),",
            literal(*first),
            literal(*last)
        );
    }
    out.push_str("];\n");
    let target = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR for a build script"),
    );
    std::fs::write(target.join("scripts.rs"), out).expect("the table is writable");
}
