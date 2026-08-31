//! How many documents ask a *substituted* face for a vertical form, and how many forms this
//! machine's faces cannot supply.
//!
//! The instrument behind ADR 0764, and it exists because the shortfall it measures had no number
//! at all. ISO 32000-2 §9.7.5.1's NOTE makes a vertical `CMap`'s CID a choice of shape —
//!
//! > Writing mode is specified as part of the CMap because, in some cases, different shapes are
//! > used when writing horizontally and vertically. In such cases, the horizontal and vertical
//! > variants of a CMap specify different CIDs for a given character code.
//!
//! — and §9.7.4.2 leaves a substitute reachable only by character, so the choice survives only
//! where the chosen face states an OpenType `vert` or `vrt2` form (`pdf_font::vertical`). Where
//! it does not, the page draws the producer's character standing up. ADR 0763 decided that is
//! counted rather than reported, on ADR 0152's arithmetic, and left it counted by nothing.
//!
//! # Two populations, and they are different questions
//!
//! **The clause's**, which is every number but the last and is a property of the *files*: a
//! `Type0` font whose encoding states writing mode 1, whose descendant embeds no program, and
//! whose character collection Table 116 publishes a vertical `CMap` for. That is where the
//! question can arise at all, and it is the same on every machine.
//!
//! **The program's**, which is the last number: how many codes those documents draw whose form
//! this machine's chosen faces do not state. §9.5's NOTE 5 puts the choice of face outside the
//! standard, so this half is a fact about this machine's font catalogue and says so.
//!
//! Trap 13's second shape is the reason both are printed: a census derived from the clause is not
//! a census of the defect, and one derived from the defect cannot say whether a zero means the
//! files are absent or the faces are good.
//!
//! ```sh
//! cargo run --release -p pdf-model --example vertical_form_census              # curated
//! cargo run --release -p pdf-model --example vertical_form_census -- --pdfjs
//! cargo run --release -p pdf-model --example vertical_form_census -- --crawl   # CC-MAIN-2021-31
//! ```
//!
//! **Only a document the clause's population contains is interpreted**, and then every page of
//! it: the dictionary scan is what says whether there is anything to draw, and a page-one-only
//! reading would have missed a book whose vertical setting starts on page two. Everything else
//! is opened, scanned and dropped.
//!
//! `PDFVIEWER_TRACE_VERTICAL_FORM=1` names each code on stderr with its font and character.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus's fonts and pages, each bounded by a document this loop \
              has already read into memory; a measurement rather than a shipped path"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Object, ObjectId};
use rayon::prelude::*;

/// How many witnessing document names are printed per finding before the list is truncated.
const MAX_NAMED: usize = 12;

/// Which population a run is over.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// The pdf.js corpus alone — the population the corpus gate's own silence lines are over.
    PdfJs,
    /// That, the `doc/corpora/` submodules and this project's own fixtures, which is where the
    /// PDF Association's `VerticalText.pdf` witness lives.
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

/// What one document's `Type0` dictionaries said, and what its pages then drew.
#[derive(Default)]
struct Answer {
    /// `Type0` font dictionaries seen at all.
    fonts: usize,
    /// Those whose encoding states writing mode 1 (§9.7.5.1).
    vertical: usize,
    /// Those, whose descendant embeds no program, so §9.7.4.2's substitution applies.
    substituted: usize,
    /// Those, whose collection Table 116 publishes a vertical `CMap` for — the clause's whole
    /// population, since a collection with no pair has no CID a table could rank.
    rankable: usize,
    /// Pages interpreted, which is every page of a document with a rankable font and none of any
    /// other.
    pages: usize,
    /// Codes drawn upright where the collection named a vertical form: this machine's half.
    lost: usize,
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
            Some((name, measure(&document)))
        })
        .collect();

    let mut fonts = 0_usize;
    let mut vertical = 0_usize;
    let mut substituted = 0_usize;
    let mut rankable = 0_usize;
    let mut pages = 0_usize;
    let mut at_risk: Vec<&str> = Vec::new();
    let mut losing: Vec<(&str, usize)> = Vec::new();
    for (name, answer) in &measured {
        fonts += answer.fonts;
        vertical += answer.vertical;
        substituted += answer.substituted;
        rankable += answer.rankable;
        pages += answer.pages;
        if answer.rankable > 0 {
            at_risk.push(name.as_str());
        }
        if answer.lost > 0 {
            losing.push((name.as_str(), answer.lost));
        }
    }
    let lost: usize = losing.iter().map(|(_, count)| *count).sum();

    println!(
        "{} document(s) opened, {fonts} Type0 font dictionaries",
        measured.len()
    );
    println!("  {vertical:6} state writing mode 1 (§9.7.5.1)");
    println!("  {substituted:6} of those embed no program, so a face stands in (§9.7.4.2)");
    println!("  {rankable:6} of those name a collection Table 116 publishes a vertical CMap for");
    println!(
        "\n**the clause's population: {} document(s)**, {pages} page(s) interpreted: {}",
        at_risk.len(),
        truncated(&at_risk)
    );
    println!(
        "\n**this machine's faces: {lost} code(s) drawn upright over {} document(s)** \
         (§9.5 NOTE 5 — which face stands in is this catalogue's)",
        losing.len()
    );
    losing.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    for (name, count) in losing.iter().take(MAX_NAMED) {
        println!("  {count:6} {name}");
    }
}

/// One flat list of names, truncated the way the other censuses print them.
fn truncated(names: &[&str]) -> String {
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

/// Every `Type0` this document contains, read for the four facts above — and then, only where
/// one of them can lose a form, every page.
///
/// **Every object the table names *and every dictionary nested inside one*.** A font dictionary
/// need not be an indirect object at all: `issue11555.pdf` writes its whole `Type0` inline in the
/// page's `/Resources`, and a walk of `object_numbers()` alone finds not one font in it — which
/// is how the first version of this census reported that the pdf.js corpus states no vertical
/// substituted font while the corpus's own `90ms-RKSJ-V` document sat in it (trap 25: a
/// population that misses what arrived reads exactly like a clean tree). The recursion is finite
/// and needs no cycle guard: a *direct* object is a tree, and the only references followed are
/// the ones this function resolves by name.
fn measure(document: &Document) -> Answer {
    let mut answer = Answer::default();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let mut fonts = Vec::new();
        collect_type0(document, &object, &mut fonts);
        for dict in fonts {
            measure_font(document, &dict, &mut answer);
        }
    }
    if answer.rankable == 0 {
        return answer;
    }
    let pages = pdf_model::Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        answer.pages += 1;
        answer.lost += pdf_model::interpret(document, &page).codes_without_a_vertical_form;
    }
    answer
}

/// Every `Type0` dictionary inside one object, itself included.
fn collect_type0(document: &Document, object: &Object, into: &mut Vec<pdf_syntax::Dictionary>) {
    match object {
        Object::Dictionary(dict) => {
            if document
                .get_key(dict, "Subtype")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Type0")
            {
                into.push(dict.clone());
            }
            for (_, value) in dict.iter() {
                collect_type0(document, value, into);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                collect_type0(document, value, into);
            }
        }
        Object::Array(items) => {
            for item in items {
                collect_type0(document, item, into);
            }
        }
        _ => {}
    }
}

/// One `Type0` dictionary against the four facts, counting each it satisfies.
fn measure_font(document: &Document, dict: &pdf_syntax::Dictionary, answer: &mut Answer) {
    answer.fonts += 1;
    if !states_writing_mode_1(document, dict) {
        return;
    }
    answer.vertical += 1;
    let descendants = document.get_key(dict, "DescendantFonts");
    let Some(descendant) = descendants
        .as_array()
        .and_then(|array| array.first())
        .map(|first| document.resolve(first))
    else {
        return;
    };
    let Some(descendant) = descendant.as_dict() else {
        return;
    };
    if embeds_a_program(document, descendant) {
        return;
    }
    answer.substituted += 1;
    let info = document.get_key(descendant, "CIDSystemInfo");
    let Some(info) = info.as_dict() else {
        return;
    };
    let registry = string(document, info, "Registry");
    let ordering = string(document, info, "Ordering");
    if pdf_font::predefined::has_vertical_forms(&registry, &ordering) {
        answer.rankable += 1;
    }
}

/// Whether the file itself says this font is set downwards.
///
/// Table 116 states the naming rule for the predefined `CMap`s — of each collection's pair,
/// "those ending in V specify vertical writing mode" — and §9.7.5.3 gives an embedded one a
/// `/WMode` entry saying the same thing. Both are the *file's* statement, which is what a census
/// should read; what the loaded `CMap` resolved to is the reader's, and asking it here would
/// measure this program rather than the corpus.
fn states_writing_mode_1(document: &Document, font: &pdf_syntax::Dictionary) -> bool {
    let encoding = document.get_key(font, "Encoding");
    if let Some(name) = encoding.as_name() {
        return name.as_bytes().ends_with(b"-V") || name.as_bytes() == b"V";
    }
    encoding.as_stream().is_some_and(|stream| {
        // The entry is an integer by Table 120; taken as one rather than compared as a float,
        // since a `/WMode 1.0` is not what any file writes and rounding one would be inventing
        // a statement.
        document
            .get_key(&stream.dict, "WMode")
            .as_integer()
            .is_some_and(|mode| mode == 1)
    })
}

/// Whether the descendant's descriptor carries a program, in any of Table 122's three entries.
fn embeds_a_program(document: &Document, descendant: &pdf_syntax::Dictionary) -> bool {
    let descriptor = document.get_key(descendant, "FontDescriptor");
    let Some(descriptor) = descriptor.as_dict() else {
        return false;
    };
    ["FontFile", "FontFile2", "FontFile3"]
        .iter()
        .any(|key| document.get_key(descriptor, key).as_stream().is_some())
}

/// One string entry of a dictionary, as the text §9.7.3 states it in.
fn string(document: &Document, dict: &pdf_syntax::Dictionary, key: &str) -> String {
    match document.get_key(dict, key) {
        Object::String(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        _ => String::new(),
    }
}
