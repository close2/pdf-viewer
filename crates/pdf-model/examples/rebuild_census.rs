//! What a rebuilt cross-reference table loses to ISO 32000-2 §7.5.7's object streams.
//!
//! `xref::rebuild` reconstructs a table by scanning for `N G obj` headers, which §C.4 licenses:
//!
//! > When a PDF processor reads a PDF file with a damaged or missing cross-reference table, it
//! > may attempt to rebuild the table by scanning all the objects in the file.
//!
//! An object inside an object stream has no such header — §7.5.7 stores it "as an alternative to
//! [its] being stored at the outermost PDF file level" — so a scan that stops at that level has
//! found some of the objects in the file rather than all of them. This counts the difference, on
//! the file's own statements rather than on this reader's:
//!
//! > N pairs of integers separated by white-space, where the first integer in each pair shall
//! > represent the object number of a compressed object and the second integer shall represent the
//! > byte offset in the decoded stream of that object
//!
//! Every question below is asked of the *documents*, not of the recovery: the header pairs are
//! read here rather than through [`pdf_syntax::Document`]'s expansion, so the count does not
//! depend on the code it is measuring (`doc/HANDOVER.md` trap 8). What does come from the reader
//! is where its table puts each named number — at a byte offset, inside a stream, or nowhere —
//! which is what makes one instrument print both arms of a before-and-after.
//!
//! ```sh
//! cargo run --profile gates -p pdf-model --example rebuild_census -- <file-or-directory>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]
#![expect(
    clippy::struct_excessive_bools,
    reason = "one document's answers to independent yes-or-no questions, which is what a census \
              row is; folding them into an enum would state a relationship the questions do not \
              have"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rayon::prelude::{IntoParallelRefIterator as _, ParallelIterator as _};

use pdf_syntax::{Document, Lexer, Limits, Location, Object, ObjectId, Parser, Token};

/// What one document says about the question.
#[derive(Default)]
struct Finding {
    /// The document opened at all.
    opened: bool,
    /// Its cross-reference table was rebuilt by scanning.
    recovered: bool,
    /// Object streams the scan found at the outermost level.
    streams: usize,
    /// Those whose data no filter chain here could decode, so their contents are unnamed.
    undecodable: usize,
    /// Those whose decode stopped short of the stream the file says it is (ADR 0343).
    damaged: usize,
    /// What Table 16's `/N` says those streams hold, decodable or not.
    counted: usize,
    /// Object numbers the readable headers name.
    named: usize,
    /// Named numbers the table locates at a byte offset — the top-level scan found them too.
    at_top_level: usize,
    /// Named numbers whose stream sits *later* in the file than that top-level object.
    at_top_level_but_older: usize,
    /// Named numbers that collide with a top-level object out of a *damaged* stream's header.
    at_top_level_from_damage: usize,
    /// Named numbers the table locates inside an object stream. Zero before the recovery lands.
    in_stream: usize,
    /// Named numbers the table locates nowhere at all — lost.
    unreachable: usize,
    /// Named numbers that resolve to something other than null through the reader.
    resolvable: usize,
    /// What those object streams decode to, in bytes — the work a recovery would spend.
    decoded_bytes: u64,
    /// The trailer states no `/Root` that resolves to a dictionary.
    rootless: bool,
    /// A compressed object states `/Type /Catalog`, whoever else does.
    catalog_compressed: bool,
}

impl Finding {
    /// Whether this document loses an object to the gap.
    const fn loses(&self) -> bool {
        self.unreachable > 0 || self.undecodable > 0
    }
}

/// Every finding added up, which is what the run prints.
#[derive(Default)]
struct Totals {
    /// Files read off the disk, opened, and rebuilt by scanning.
    read: usize,
    /// As above.
    opened: usize,
    /// As above.
    recovered: usize,
    /// Rebuilt documents carrying at least one object stream, and how many streams in all.
    with_streams: usize,
    /// As above.
    streams: usize,
    /// Documents with a stream no filter chain here decodes, and streams whose decode stops short.
    undecodable_documents: usize,
    /// As above.
    damaged_streams: usize,
    /// What `/N` counts, and what the headers name.
    counted: usize,
    /// As above.
    named: usize,
    /// Where the table puts each named number: an offset, a stream, or nowhere.
    at_top_level: usize,
    /// As above.
    in_stream: usize,
    /// As above.
    unreachable: usize,
    /// Of the collisions, those whose stream is later in the file and those out of damage.
    older: usize,
    /// As above.
    collisions_from_damage: usize,
    /// Named numbers that resolve to an object, and documents losing at least one.
    resolvable: usize,
    /// As above.
    losing: usize,
    /// Rebuilt documents with no resolvable `/Root` and a compressed catalogue.
    rootless_with_compressed_catalog: usize,
    /// The widest object-stream expansion and the most streams one document carries.
    widest_decode: u64,
    /// As above.
    most_streams: usize,
    /// How many rebuilt documents carry how many object streams.
    per_document: BTreeMap<usize, usize>,
}

impl Totals {
    /// Adds one document's finding.
    fn add(&mut self, finding: &Finding) {
        self.read = self.read.saturating_add(1);
        if !finding.opened {
            return;
        }
        self.opened = self.opened.saturating_add(1);
        if !finding.recovered {
            return;
        }
        self.recovered = self.recovered.saturating_add(1);
        let documents = self.per_document.entry(finding.streams).or_default();
        *documents = documents.saturating_add(1);
        if finding.streams == 0 {
            return;
        }
        self.with_streams = self.with_streams.saturating_add(1);
        self.streams = self.streams.saturating_add(finding.streams);
        self.counted = self.counted.saturating_add(finding.counted);
        self.named = self.named.saturating_add(finding.named);
        self.at_top_level = self.at_top_level.saturating_add(finding.at_top_level);
        self.older = self.older.saturating_add(finding.at_top_level_but_older);
        self.damaged_streams = self.damaged_streams.saturating_add(finding.damaged);
        self.collisions_from_damage = self
            .collisions_from_damage
            .saturating_add(finding.at_top_level_from_damage);
        self.in_stream = self.in_stream.saturating_add(finding.in_stream);
        self.unreachable = self.unreachable.saturating_add(finding.unreachable);
        self.resolvable = self.resolvable.saturating_add(finding.resolvable);
        self.widest_decode = self.widest_decode.max(finding.decoded_bytes);
        self.most_streams = self.most_streams.max(finding.streams);
        if finding.undecodable > 0 {
            self.undecodable_documents = self.undecodable_documents.saturating_add(1);
        }
        if finding.rootless && finding.catalog_compressed {
            self.rootless_with_compressed_catalog =
                self.rootless_with_compressed_catalog.saturating_add(1);
        }
        if finding.loses() {
            self.losing = self.losing.saturating_add(1);
        }
    }

    /// Prints the summary, one claim per line.
    fn print(&self) {
        println!(
            "rebuild census: {} document(s) read, {} opened, {} rebuilt by scanning",
            self.read, self.opened, self.recovered
        );
        println!(
            "  of the rebuilt: {} carry object streams the scan found, {} stream(s) in all, {} \
             document(s) with a stream no filter chain here decodes, {} stream(s) whose decode \
             stops short",
            self.with_streams, self.streams, self.undecodable_documents, self.damaged_streams
        );
        println!(
            "  those streams hold {} object(s) by their own /N, of which {} are named by a header \
             this reads",
            self.counted, self.named
        );
        println!(
            "  the table puts {} of the named numbers at a byte offset ({} with the stream later \
             in the file than that object, {} named by a stream whose decode stops short), {} \
             inside an object stream, and {} nowhere",
            self.at_top_level,
            self.older,
            self.collisions_from_damage,
            self.in_stream,
            self.unreachable
        );
        println!(
            "  {} of the named numbers resolve to an object through the reader",
            self.resolvable
        );
        println!(
            "  {} document(s) lose at least one object to the gap",
            self.losing
        );
        println!(
            "  {} of them state no resolvable /Root while a compressed object is a /Type /Catalog",
            self.rootless_with_compressed_catalog
        );
        println!(
            "  the widest of them expands {} byte(s) of object stream, and the most object streams \
             one document carries is {}",
            self.widest_decode, self.most_streams
        );
        print!("  object streams per rebuilt document:");
        for (count, documents) in &self.per_document {
            print!(" {count}×{documents}");
        }
        println!();
    }
}

fn main() {
    let files = collect(std::env::args().skip(1).map(PathBuf::from));
    let findings: Vec<(PathBuf, Finding)> = files
        .par_iter()
        .map(|path| (path.clone(), examine(path)))
        .collect();

    let mut totals = Totals::default();
    let mut lines: Vec<String> = Vec::new();
    for (path, finding) in &findings {
        totals.add(finding);
        if !finding.recovered || finding.streams == 0 {
            continue;
        }
        // A line for every document the recovery reaches, losing or not: the population is what a
        // before-and-after has to survey, and a document that loses nothing on one arm is exactly
        // the one a later round would otherwise leave out of the other.
        lines.push(format!(
            "  {}: {} object stream(s), {} named, {} lost, {} undecodable and {} damaged \
             stream(s), {} colliding with a top-level object{}",
            path.display(),
            finding.streams,
            finding.named,
            finding.unreachable,
            finding.undecodable,
            finding.damaged,
            finding.at_top_level,
            if finding.rootless && finding.catalog_compressed {
                ", and its /Root is compressed"
            } else {
                ""
            }
        ));
    }

    lines.sort();
    for line in &lines {
        println!("{line}");
    }
    totals.print();
}

/// Every file named, and every `.pdf` under every directory named.
fn collect(paths: impl Iterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for path in paths {
        if path.is_dir() {
            walk(&path, &mut found);
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Adds every `.pdf` under `directory`, depth first.
fn walk(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, found);
        } else if path.extension().is_some_and(|kind| kind == "pdf") {
            found.push(path);
        }
    }
}

/// Asks one document every question above.
fn examine(path: &Path) -> Finding {
    let mut finding = Finding::default();
    let Ok(bytes) = std::fs::read(path) else {
        return finding;
    };
    let Ok(document) = Document::open(bytes) else {
        return finding;
    };
    finding.opened = true;
    if !document.was_recovered() {
        return finding;
    }
    finding.recovered = true;
    finding.rootless = document.catalog().is_err();

    // The object streams the scan found: an object stream is itself a stream, which §7.5.7
    // forbids being stored in one, so every one of them has an `N G obj` header the scan reaches.
    let table = document.xref();
    let streams: Vec<(u32, usize)> = table
        .object_numbers()
        .filter_map(|number| match table.location(number) {
            Some(Location::Offset(offset)) => Some((number, offset)),
            _ => None,
        })
        .filter(|&(number, _)| is_object_stream(&document, number))
        .collect();
    finding.streams = streams.len();

    for (number, offset) in streams {
        let object = document.get(ObjectId::new(number, 0));
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let count = usize::try_from(
            document
                .get_key(&stream.dict, "N")
                .as_integer()
                .unwrap_or(0),
        )
        .unwrap_or(0);
        finding.counted = finding.counted.saturating_add(count);
        let Ok(decoded) = document.decoded_stream_data_reported(stream) else {
            finding.undecodable = finding.undecodable.saturating_add(1);
            continue;
        };
        let damaged = decoded.damage.is_some();
        if damaged {
            finding.damaged = finding.damaged.saturating_add(1);
        }
        let data = decoded.data;
        finding.decoded_bytes = finding
            .decoded_bytes
            .saturating_add(u64::try_from(data.len()).unwrap_or(u64::MAX));
        let first = document
            .get_key(&stream.dict, "First")
            .as_integer()
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        for (member, at) in header_pairs(&data, first, count) {
            finding.named = finding.named.saturating_add(1);
            match document.xref().location(member) {
                Some(Location::Offset(elsewhere)) => {
                    finding.at_top_level = finding.at_top_level.saturating_add(1);
                    if elsewhere < offset {
                        finding.at_top_level_but_older =
                            finding.at_top_level_but_older.saturating_add(1);
                    }
                    if damaged {
                        finding.at_top_level_from_damage =
                            finding.at_top_level_from_damage.saturating_add(1);
                    }
                }
                Some(Location::InStream { .. }) => {
                    finding.in_stream = finding.in_stream.saturating_add(1);
                }
                None => finding.unreachable = finding.unreachable.saturating_add(1),
            }
            if !matches!(document.get(ObjectId::new(member, 0)), Object::Null) {
                finding.resolvable = finding.resolvable.saturating_add(1);
            }
            if !finding.catalog_compressed && is_catalog(&data, first.saturating_add(at)) {
                finding.catalog_compressed = true;
            }
        }
    }
    finding
}

/// Whether the object at `number` states Table 16's `/Type /ObjStm`.
fn is_object_stream(document: &Document, number: u32) -> bool {
    document
        .get_key_of(ObjectId::new(number, 0), "Type")
        .and_then(|kind| kind.as_name().map(|name| name == &"ObjStm"))
        .unwrap_or(false)
}

/// §7.5.7's `N` pairs, read straight out of the decoded prefix.
///
/// Bounded by `count`, which is Table 16's `/N`, because `/First` is "[t]he byte offset in the
/// decoded stream of the first compressed object" and not the end of the pairs: a producer that
/// leaves white-space between the last pair and the first object gives a prefix whose tail is the
/// object's own bytes, and reading past `/N` takes integers out of it. Stops early at the first
/// pair the data does not carry, which is what a truncated header is.
fn header_pairs(data: &[u8], first: usize, count: usize) -> Vec<(u32, usize)> {
    let mut lexer = Lexer::new(data.get(..first).unwrap_or_default());
    let mut pairs = Vec::new();
    for _ in 0..count {
        let (Some(Token::Integer(number)), Some(Token::Integer(at))) =
            (lexer.next_token(), lexer.next_token())
        else {
            return pairs;
        };
        let (Ok(number), Ok(at)) = (u32::try_from(number), usize::try_from(at)) else {
            return pairs;
        };
        pairs.push((number, at));
    }
    pairs
}

/// Whether the object at `start` in a decoded object stream is a document catalogue.
fn is_catalog(data: &[u8], start: usize) -> bool {
    let mut parser = Parser::at(data, start, Limits::DEFAULT);
    parser.parse_object().is_ok_and(|object| {
        object
            .as_dict()
            .and_then(|dict| dict.get("Type"))
            .and_then(Object::as_name)
            .is_some_and(|name| name == &"Catalog")
    })
}
