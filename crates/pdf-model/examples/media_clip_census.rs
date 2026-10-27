//! Counts the media documents of a corpus by what each clip's own container lets §13.2.2's
//! viability and §13.2.4.3's offsets be judged from.
//!
//! §13.2.4.3 decides an `/E` "beyond the end of the next-level media clip" and a marker "not
//! present in the underlying media clip" by facts only the clip's media data holds, so the
//! judgement reads the container's header (ADR 2452). A clip whose container this program reads
//! has those facts; a clip of any other type is an "unread container", and the report says why.
//!
//! Every media clip data dictionary (Table 285, `/S /MCD`) of every document is looked at once,
//! whichever rendition, annotation or action reaches it, and sorted by its `/CT`:
//!
//! - **read**: the container's header gave a timeline ([`pdf_model::media::clip_timeline`], the
//!   code the judgement runs);
//! - **unread container**: the type names a container the reader has no arm for;
//! - **not judged**: the data is named by a URL or a file outside the document, is not a file
//!   stream, or the container is cut short, damaged or past a bound.
//!
//! The population is every PDF under `doc/pdf.js`, `doc/corpora`, `doc/corpora-own` and
//! `doc/veraPDF-corpus` and under the whole of `corpus-cache` (the `openpreserve`, `safedocs` and
//! `tika-issue-tracker` collections), none of it narrowed beforehand:
//!
//! ```sh
//! { find -L doc/pdf.js doc/corpora doc/corpora-own doc/veraPDF-corpus -iname '*.pdf'
//!   find -L "$PWD/corpus-cache" -iname '*.pdf'; } > list.txt
//! cargo run --release -p pdf-model --example media_clip_census -- @list.txt
//! ```
//!
//! An argument `@list` names a file of paths, one a line, because the corpora's population is
//! more than one command line holds; a list that cannot be read stops the census with an error.
//!
//! The census opens every object of a document, so a document whose clips sit in an object stream
//! is found. Every path in the list lands in exactly one of three counts: opened, skipped for size,
//! or unreadable (not read, or not a document `pdf_syntax` opens), so the denominator adds up.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example whose entire output is a measurement and its refusals"
)]

use std::collections::BTreeMap;

use pdf_model::media::clip_timeline;
use pdf_model::media::container::{Timeline, is_read};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// Largest document the census opens, in bytes.
const MAX_DOCUMENT: u64 = 48 * 1024 * 1024;

/// What one content type contributed.
#[derive(Default)]
struct Row {
    documents: usize,
    clips: usize,
    read: usize,
    unread: usize,
    not_judged: usize,
    /// Why clips of this type were not judged, as the report would say it, each with its count.
    reasons: BTreeMap<String, usize>,
    /// Of the clips read, how many state a length in seconds, a frame count, and a marker table.
    with_length: usize,
    with_frames: usize,
    with_markers: usize,
}

/// A content type without its parameters and case, as Table 285's `/CT` is matched.
fn essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

/// Whether `dict` is Table 285's media clip data.
fn is_clip_data(document: &Document, dict: &Dictionary) -> bool {
    document
        .get_key(dict, "S")
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"MCD")
}

/// The documents to open: each argument is a path, and `@list` names a file of paths, one a line.
fn paths() -> Vec<String> {
    let mut out = Vec::new();
    for argument in std::env::args().skip(1) {
        match argument.strip_prefix('@') {
            Some(list) => match std::fs::read_to_string(list) {
                Ok(text) => out.extend(text.lines().map(str::to_owned)),
                Err(error) => {
                    eprintln!("media_clip_census: cannot read the list {list}: {error}");
                    std::process::exit(2);
                }
            },
            None => out.push(argument),
        }
    }
    out
}

/// Every media clip data dictionary inside `object`, the direct ones nested in a rendition or a
/// section included: an indirect reference is not followed, because each indirect object is
/// visited on its own.
fn collect(document: &Document, object: &Object, out: &mut Vec<Dictionary>, depth: usize) {
    // Direct nesting past this depth is not a clip's: Table 284's chain is a few levels.
    if depth > 32 {
        return;
    }
    match object {
        Object::Dictionary(dict) => {
            if is_clip_data(document, dict) {
                out.push(dict.clone());
            }
            for (_, value) in dict.iter() {
                collect(document, value, out, depth.saturating_add(1));
            }
        }
        Object::Stream(stream) => {
            collect(
                document,
                &Object::Dictionary(stream.dict.clone()),
                out,
                depth.saturating_add(1),
            );
        }
        Object::Array(items) => {
            for item in items {
                collect(document, item, out, depth.saturating_add(1));
            }
        }
        _ => {}
    }
}

/// Counts one clip's verdict into its content type's row.
fn tally(row: &mut Row, content_type: &str, verdict: Result<Timeline, String>) {
    match verdict {
        Ok(timeline) => {
            row.read = row.read.saturating_add(1);
            row.with_length = row
                .with_length
                .saturating_add(usize::from(timeline.seconds.is_some()));
            row.with_frames = row
                .with_frames
                .saturating_add(usize::from(timeline.frames.is_some()));
            row.with_markers = row
                .with_markers
                .saturating_add(usize::from(!timeline.markers.is_empty()));
        }
        Err(reason) => {
            if is_read(content_type) {
                row.not_judged = row.not_judged.saturating_add(1);
            } else {
                row.unread = row.unread.saturating_add(1);
            }
            // A reason may name bytes of the file (a box type); only printable ASCII is kept.
            let printable: String = reason
                .chars()
                .map(|c| {
                    if c.is_ascii_graphic() || c == ' ' {
                        c
                    } else {
                        '?'
                    }
                })
                .collect();
            let count = row.reasons.entry(printable).or_insert(0);
            *count = count.saturating_add(1);
        }
    }
}

fn main() {
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    let mut opened = 0_usize;
    let mut skipped = 0_usize;
    let mut unreadable = 0_usize;
    let mut media_documents = 0_usize;
    let mut lines: Vec<String> = Vec::new();
    for path in paths() {
        // A document past this size is skipped: the census asks about clips, and a walk of every
        // object of a file this large is the memory the corpus walks' bound exists for.
        match std::fs::metadata(&path) {
            Ok(meta) if meta.len() > MAX_DOCUMENT => {
                skipped = skipped.saturating_add(1);
                continue;
            }
            Ok(_) => {}
            Err(_) => {
                unreadable = unreadable.saturating_add(1);
                continue;
            }
        }
        let Ok(bytes) = std::fs::read(&path) else {
            unreadable = unreadable.saturating_add(1);
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            unreadable = unreadable.saturating_add(1);
            continue;
        };
        opened = opened.saturating_add(1);
        let numbers: Vec<u32> = document.xref().object_numbers().collect();
        let mut seen_types: Vec<String> = Vec::new();
        for number in numbers {
            let id = ObjectId {
                number,
                generation: 0,
            };
            let mut found = Vec::new();
            collect(&document, &document.get(id), &mut found, 0);
            for dict in found {
                let content_type = match document.get_key(&dict, "CT") {
                    Object::String(bytes) => essence(&String::from_utf8_lossy(&bytes)),
                    _ => "(no /CT)".to_owned(),
                };
                let row = rows.entry(content_type.clone()).or_default();
                row.clips = row.clips.saturating_add(1);
                if !seen_types.contains(&content_type) {
                    seen_types.push(content_type.clone());
                    row.documents = row.documents.saturating_add(1);
                }
                tally(row, &content_type, clip_timeline(&document, &dict));
            }
        }
        if !seen_types.is_empty() {
            media_documents = media_documents.saturating_add(1);
            let name = path.rsplit('/').next().unwrap_or(&path);
            lines.push(format!("  {name}: {}", seen_types.join(", ")));
        }
    }
    println!(
        "{opened} document(s) opened, {skipped} skipped for size, {unreadable} unreadable, {media_documents} with media clip data"
    );
    println!(
        "{:<36} {:>5} {:>5} {:>5} {:>7} {:>10} {:>6} {:>6} {:>7}",
        "content type",
        "docs",
        "clips",
        "read",
        "unread",
        "not judged",
        "length",
        "frames",
        "markers"
    );
    let mut totals = Row::default();
    for (content_type, row) in &rows {
        println!(
            "{content_type:<36} {:>5} {:>5} {:>5} {:>7} {:>10} {:>6} {:>6} {:>7}",
            row.documents,
            row.clips,
            row.read,
            row.unread,
            row.not_judged,
            row.with_length,
            row.with_frames,
            row.with_markers
        );
        for (reason, count) in &row.reasons {
            println!("    {count} without a timeline: {reason}");
        }
        totals.clips = totals.clips.saturating_add(row.clips);
        totals.read = totals.read.saturating_add(row.read);
        totals.unread = totals.unread.saturating_add(row.unread);
        totals.not_judged = totals.not_judged.saturating_add(row.not_judged);
    }
    println!(
        "{:<36} {:>5} {:>5} {:>5} {:>7} {:>10}",
        "all", media_documents, totals.clips, totals.read, totals.unread, totals.not_judged
    );
    for line in lines {
        println!("{line}");
    }
}
