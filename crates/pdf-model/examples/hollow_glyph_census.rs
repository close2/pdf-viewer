//! How many corpus fonts embed a `TrueType` program that describes no glyph at all, and how
//! many of those reach their glyphs through a `/CIDToGIDMap` stream.
//!
//! The instrument behind ADR 0350's trap-8 claim. `OmniPage` CSDK 22 writes an invisible OCR
//! layer through a **hollow** `CIDFontType2` — `head` and `hmtx` present, every `loca` entry
//! equal to the next, which is the table's own statement that the glyph is empty — with a
//! `/CIDToGIDMap` remapping stream, and a reader that derives a character's vertical extent
//! from the outlines paints no selection at all on such a file. That fixture is hand-built,
//! and "hand-built because the corpus has none" is a claim about a population rather than an
//! impression: this counts it, per font and per document, over whatever files it is given.
//!
//! What it separates, because the two are different findings: a program in which *every* glyph
//! is empty, and one in which some are (`issue14821.pdf` is the second kind, and §9.7.4.2's row
//! has carried it since the two-hundred-and-forty-eighth session). The intersection this exists
//! for is the first kind under a stream.
//!
//! The `loca` is read here by hand rather than through `skrifa`, for the same reason
//! `composite_fonts.rs` reads `hmtx` by hand: a census that asked the renderer's own library
//! what the renderer's own library thinks would measure the reader rather than the corpus.
//!
//! ```sh
//! cargo run --release -p pdf-model --example hollow_glyph_census              # curated
//! cargo run --release -p pdf-model --example hollow_glyph_census -- --pdfjs
//! cargo run --release -p pdf-model --example hollow_glyph_census -- --crawl   # CC-MAIN-2021-31
//! ```
//!
//! **The three scopes are the six-hundred-and-eighty-sixth session's.** ADR 0350's claim was
//! measured over `doc/pdf.js` alone, before `CC-MAIN-2021-31` was on this disk, and a negative
//! decays when the population grows (ADR 0490). The control run is stated beside the crawl run
//! rather than merged with it, because one number over both would hide which of the two moved.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(
    clippy::arithmetic_side_effects,
    reason = "counters and table offsets over a corpus's fonts; every quantity is bounded \
              by a program this loop has already read into memory, and this is a measurement \
              rather than a shipped path"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_syntax::{Document, ObjectId};
use rayon::prelude::*;

/// How many witnessing document names are printed per finding before the list is truncated.
const MAX_NAMED: usize = 12;

/// Which population a run is over.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// The pdf.js corpus alone — the population ADR 0350's claim was measured over.
    PdfJs,
    /// That, the four `doc/corpora/` submodules, and this project's own fixtures.
    Curated,
    /// The `SafeDocs` `CC-MAIN-2021-31` crawl under `corpus-cache/`, and nothing else.
    Crawl,
}

fn corpus(scope: Scope) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    let roots: &[&str] = match scope {
        Scope::PdfJs => &["doc/pdf.js/test/pdfs"],
        Scope::Curated => &["doc/pdf.js/test/pdfs", "doc/corpora", "doc/corpora-own"],
        Scope::Crawl => &["corpus-cache/safedocs/cc-main-2021-31"],
    };
    for relative in roots {
        collect(&root.join(relative), &mut files);
    }
    files.sort();
    files.dedup();
    files
}

/// Every `.pdf` under one directory, recursively.
fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        {
            into.push(path);
        }
    }
}

/// How a `CIDFontType2` reaches a glyph index, as its own dictionary states it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Route {
    /// A `/CIDToGIDMap` stream: Table 115's two big-endian bytes at 2c.
    Stream,
    /// `/Identity`, or the entry absent, which §9.7.4.2 makes the same thing.
    Identity,
}

/// What one embedded program says about its glyphs.
struct Program {
    glyphs: usize,
    empty: usize,
}

/// What one document's `CIDFontType2` dictionaries said.
#[derive(Default)]
struct Answer {
    /// `CIDFontType2` dictionaries seen at all.
    fonts: usize,
    /// Those with a `/FontFile2` this census could read.
    embedded: usize,
    /// Those reaching their glyphs through a `/CIDToGIDMap` stream.
    through_stream: usize,
    /// Those embedding a program whose every glyph is empty.
    wholly_hollow: usize,
    /// Those embedding a program some of whose glyphs are empty.
    partly_hollow: usize,
    /// The intersection this census exists for, named per font.
    hollow_under_stream: Vec<String>,
}

fn main() {
    let scope = if std::env::args().any(|a| a == "--crawl") {
        Scope::Crawl
    } else if std::env::args().any(|a| a == "--pdfjs") {
        Scope::PdfJs
    } else {
        Scope::Curated
    };
    let files = corpus(scope);
    eprintln!("{} PDF(s) in the population", files.len());

    let measured: Vec<(String, Answer)> = files
        .par_iter()
        .filter_map(|path| {
            let bytes = std::fs::read(path).ok()?;
            let document = Document::open(bytes).ok()?;
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let answer = measure(&document, &name);
            Some((name, answer))
        })
        .collect();

    let mut fonts = 0_usize;
    let mut embedded = 0_usize;
    let mut through_stream = 0_usize;
    let mut wholly_hollow = 0_usize;
    let mut partly_hollow = 0_usize;
    let mut hollow_under_stream: Vec<String> = Vec::new();
    let mut hollow_documents: BTreeMap<String, usize> = BTreeMap::new();
    let mut stream_documents: BTreeMap<String, usize> = BTreeMap::new();
    for (name, answer) in &measured {
        fonts += answer.fonts;
        embedded += answer.embedded;
        through_stream += answer.through_stream;
        wholly_hollow += answer.wholly_hollow;
        partly_hollow += answer.partly_hollow;
        hollow_under_stream.extend(answer.hollow_under_stream.iter().cloned());
        if answer.through_stream > 0 {
            *stream_documents.entry(name.clone()).or_default() += answer.through_stream;
        }
        if answer.wholly_hollow > 0 {
            *hollow_documents.entry(name.clone()).or_default() += answer.wholly_hollow;
        }
    }

    println!(
        "{} document(s) opened, {fonts} CIDFontType2 dictionaries, {embedded} of them \
         with an embedded /FontFile2 this census could read",
        measured.len()
    );
    println!("  {through_stream:6} reach their glyphs through a /CIDToGIDMap stream");
    println!("  {wholly_hollow:6} embed a program whose every glyph is empty");
    println!("  {partly_hollow:6} embed a program some of whose glyphs are empty");
    println!(
        "\n**{} font(s) combine both — a wholly hollow program under a remapping stream**: {}",
        hollow_under_stream.len(),
        truncated(&hollow_under_stream)
    );
    println!(
        "\n{} document(s) state a /CIDToGIDMap stream: {}",
        stream_documents.len(),
        listed(&stream_documents)
    );
    println!(
        "\n{} document(s) embed a wholly hollow program: {}",
        hollow_documents.len(),
        listed(&hollow_documents)
    );
}

/// Every `CIDFontType2` this document's cross-reference table names, read for its glyphs.
fn measure(document: &Document, name: &str) -> Answer {
    let mut answer = Answer::default();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let Some(dict) = object.as_dict() else {
            continue;
        };
        let subtype = document.get_key(dict, "Subtype");
        if subtype
            .as_name()
            .is_none_or(|n| n.as_bytes() != b"CIDFontType2")
        {
            continue;
        }
        answer.fonts += 1;

        let mapping = document.get_key(dict, "CIDToGIDMap");
        let route = if mapping.as_stream().is_some() {
            Route::Stream
        } else {
            Route::Identity
        };
        if route == Route::Stream {
            answer.through_stream += 1;
        }

        let Some(program) = embedded_program(document, dict) else {
            continue;
        };
        answer.embedded += 1;
        if program.empty == 0 {
            continue;
        }
        if program.empty == program.glyphs {
            answer.wholly_hollow += 1;
            if route == Route::Stream {
                answer
                    .hollow_under_stream
                    .push(format!("{name} ({number})"));
            }
        } else {
            answer.partly_hollow += 1;
        }
    }
    answer
}

/// Names with their font counts, as the other censuses print them.
fn listed(documents: &BTreeMap<String, usize>) -> String {
    let named: Vec<String> = documents
        .iter()
        .take(MAX_NAMED)
        .map(|(name, count)| format!("{name} ({count})"))
        .collect();
    if documents.len() > MAX_NAMED {
        format!(
            "{}, … and {} more",
            named.join(", "),
            documents.len() - MAX_NAMED
        )
    } else {
        named.join(", ")
    }
}

/// One flat list of names, truncated the same way.
fn truncated(names: &[String]) -> String {
    if names.len() > MAX_NAMED {
        format!(
            "{}, … and {} more",
            names[..MAX_NAMED].join(", "),
            names.len() - MAX_NAMED
        )
    } else {
        names.join(", ")
    }
}

/// The descendant's own `/FontFile2`, read for how many of its glyphs describe nothing.
///
/// `None` where there is no program, where it cannot be decoded, or where the three tables this
/// question needs are not all there — a program without `loca` or `maxp` states nothing about
/// its glyphs either way, and counting it as hollow would be inventing the finding.
fn embedded_program(document: &Document, descendant: &pdf_syntax::Dictionary) -> Option<Program> {
    let descriptor = document.get_key(descendant, "FontDescriptor");
    let descriptor = descriptor.as_dict()?;
    let file = document.get_key(descriptor, "FontFile2");
    let stream = file.as_stream()?;
    let bytes = document.decoded_stream_data(stream)?;

    let (head, _) = sfnt_table(&bytes, *b"head")?;
    let (maxp, _) = sfnt_table(&bytes, *b"maxp")?;
    let (loca, length) = sfnt_table(&bytes, *b"loca")?;
    let long = i16::from_be_bytes([*bytes.get(head + 50)?, *bytes.get(head + 51)?]) != 0;
    let glyphs = usize::from(u16::from_be_bytes([
        *bytes.get(maxp + 4)?,
        *bytes.get(maxp + 5)?,
    ]));

    let width = if long { 4 } else { 2 };
    // A `loca` needs one entry per glyph plus the end sentinel; a shorter one is truncated and
    // the glyphs past its end are unknown rather than empty.
    if length < (glyphs + 1) * width {
        return None;
    }
    let offset = |index: usize| -> Option<u32> {
        let at = loca + index * width;
        if long {
            Some(u32::from_be_bytes([
                *bytes.get(at)?,
                *bytes.get(at + 1)?,
                *bytes.get(at + 2)?,
                *bytes.get(at + 3)?,
            ]))
        } else {
            Some(u32::from(u16::from_be_bytes([
                *bytes.get(at)?,
                *bytes.get(at + 1)?,
            ])))
        }
    };

    let mut empty = 0_usize;
    for index in 0..glyphs {
        // The table's own statement that the glyph is empty: its data starts where it ends.
        if offset(index)? == offset(index + 1)? {
            empty += 1;
        }
    }
    Some(Program { glyphs, empty })
}

/// Finds one table in an sfnt's directory, returning its offset and length.
fn sfnt_table(program: &[u8], tag: [u8; 4]) -> Option<(usize, usize)> {
    let count = usize::from(u16::from_be_bytes([*program.get(4)?, *program.get(5)?]));
    for index in 0..count {
        let record = program.get(12 + index * 16..12 + index * 16 + 16)?;
        if record[..4] != tag {
            continue;
        }
        let offset = u32::from_be_bytes([record[8], record[9], record[10], record[11]]);
        let length = u32::from_be_bytes([record[12], record[13], record[14], record[15]]);
        return Some((usize::try_from(offset).ok()?, usize::try_from(length).ok()?));
    }
    None
}
