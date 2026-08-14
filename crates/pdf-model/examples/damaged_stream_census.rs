//! How many documents hold a stream that decodes only as far as its damage, and where.
//!
//! The instrument behind ADR 0343. `doc/todo/03` §8 asked whether drawing the successfully
//! decoded prefix of a stream that then fails is a recovery a reader may perform, and named the
//! population to measure it over: the crawl's undecodable `/Contents` parts. The decision needs
//! two numbers that no gate prints, and this counts both.
//!
//! - **What the rule buys**, on page one's `/Contents`: how many documents keep a prefix, how
//!   many bytes, and whether interpreting that prefix produces any drawing command at all. A
//!   recovery that recovers no marks is still worth having as a *report*, and this is what says
//!   which of the two it is on any given file.
//! - **How wide the silence was**, over every stream object in the file: an [`ISO 32000-2
//!   §7.4`](super) filter that stopped short used to hand its prefix back indistinguishably from
//!   a whole decode, so a partial ICC profile, font program or image reached the code that reads
//!   it with nothing said. `/Contents` is the route this round made loud; the rest is the number
//!   that says what is still owed.
//!
//! ```sh
//! cargo run --release -p pdf-model --example damaged_stream_census -- <dir-or-file>...
//! ```
//!
//! One process per directory is the method the surveys use and the reason is the same:
//! `render-cpu` rasterises under `panic = "abort"`, so one document's abort would take every
//! other verdict with it. This example does not rasterise, but it does parse hostile input.
#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the measurement"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus four orders of magnitude below what a usize counts, and \
              this is a measurement rather than a shipped path"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::{Damage, Document, ObjectId};

use pdf_model::page::ContentIssue;

/// What one population came to.
#[derive(Default)]
struct Tally {
    /// Files looked at.
    files: usize,
    /// Files [`Document::open`] accepted.
    opened: usize,
    /// Documents whose page-one `/Contents` reported [`ContentIssue::Damaged`].
    contents_damaged: usize,
    /// Of those, the ones whose damage is [`Damage::Truncated`].
    contents_truncated: usize,
    /// Of those, the ones whose damage is [`Damage::Corrupt`].
    contents_corrupt: usize,
    /// Documents whose page-one `/Contents` reported [`ContentIssue::Undecodable`] — nothing
    /// survived at all, which is the case the prefix rule cannot help.
    contents_undecodable: usize,
    /// Bytes kept across every damaged `/Contents` part.
    contents_kept: usize,
    /// Damaged `/Contents` documents whose kept prefix yields at least one drawing command.
    contents_drew: usize,
    /// Stream objects examined anywhere in a document.
    streams: usize,
    /// Those that decoded only as far as their damage.
    streams_damaged: usize,
    /// Documents holding at least one such stream.
    documents_with_damage: usize,
    /// The documents worth naming, with what they said.
    witnesses: Vec<String>,
}

impl Tally {
    fn absorb(&mut self, other: Self) {
        self.files += other.files;
        self.opened += other.opened;
        self.contents_damaged += other.contents_damaged;
        self.contents_truncated += other.contents_truncated;
        self.contents_corrupt += other.contents_corrupt;
        self.contents_undecodable += other.contents_undecodable;
        self.contents_kept += other.contents_kept;
        self.contents_drew += other.contents_drew;
        self.streams += other.streams;
        self.streams_damaged += other.streams_damaged;
        self.documents_with_damage += other.documents_with_damage;
        self.witnesses.extend(other.witnesses);
    }
}

fn main() {
    let roots: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if roots.is_empty() {
        println!("usage: damaged_stream_census <dir-or-file>...");
        return;
    }

    let mut files = Vec::new();
    for root in &roots {
        collect(root, &mut files);
    }
    files.sort();

    let mut total = Tally::default();
    for path in &files {
        total.absorb(examine(path));
    }

    println!(
        "{} files, {} opened: {} page-one /Contents damaged ({} truncated, {} corrupt), \
         {} undecodable",
        total.files,
        total.opened,
        total.contents_damaged,
        total.contents_truncated,
        total.contents_corrupt,
        total.contents_undecodable,
    );
    println!(
        "  the prefix rule keeps {} bytes over those parts, and {} of {} damaged documents \
         draw at least one command from it",
        total.contents_kept, total.contents_drew, total.contents_damaged,
    );
    println!(
        "  over every stream object: {} of {} streams damaged, in {} documents",
        total.streams_damaged, total.streams, total.documents_with_damage,
    );
    for witness in &total.witnesses {
        println!("    {witness}");
    }
}

/// Every `.pdf` under `root`, or `root` itself where it is one.
fn collect(root: &Path, into: &mut Vec<PathBuf>) {
    if root.is_file() {
        into.push(root.to_path_buf());
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path.extension().is_some_and(|e| e == "pdf") {
            into.push(path);
        }
    }
}

fn examine(path: &Path) -> Tally {
    let mut tally = Tally {
        files: 1,
        ..Tally::default()
    };
    let Ok(bytes) = std::fs::read(path) else {
        return tally;
    };
    let Ok(document) = Document::open(bytes) else {
        return tally;
    };
    tally.opened = 1;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    // Every stream in the file, which is the width of the silence rather than the part of it
    // this round made loud.
    let mut damaged_here = Vec::new();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let Some(stream) = object.as_stream() else {
            continue;
        };
        tally.streams += 1;
        if let Ok(decoded) = document.decoded_stream_data_reported(stream)
            && let Some(damage) = decoded.damage
        {
            tally.streams_damaged += 1;
            damaged_here.push((number, damage, decoded.data.len()));
        }
    }
    if !damaged_here.is_empty() {
        tally.documents_with_damage = 1;
    }

    // Page one's `/Contents`, which is where `doc/todo/03` §8's question was asked.
    let pages = pdf_model::Pages::new(&document);
    let Some(page) = pages.get(0) else {
        return tally;
    };
    let (_content, issues) = page.content_with_report(&document);
    let mut damaged_contents = None;
    for issue in &issues {
        match issue {
            ContentIssue::Damaged { damage, kept, .. } => {
                tally.contents_damaged = 1;
                tally.contents_kept += kept;
                match damage {
                    Damage::Truncated => tally.contents_truncated = 1,
                    Damage::Corrupt => tally.contents_corrupt = 1,
                }
                damaged_contents = Some((*damage, *kept));
            }
            ContentIssue::Undecodable { .. } => tally.contents_undecodable = 1,
            _ => {}
        }
    }

    if let Some((damage, kept)) = damaged_contents {
        // Whether the prefix is worth drawing rather than merely worth reporting.
        let commands = pdf_model::interpret(&document, &page)
            .display_list
            .commands()
            .len();
        if commands > 0 {
            tally.contents_drew = 1;
        }
        tally.witnesses.push(format!(
            "{name}: /Contents {damage:?}, {kept} bytes kept, {commands} commands"
        ));
    } else if tally.contents_undecodable == 1 {
        tally
            .witnesses
            .push(format!("{name}: /Contents undecodable, nothing survived"));
    } else if !damaged_here.is_empty() {
        let (number, damage, kept) = damaged_here[0];
        tally.witnesses.push(format!(
            "{name}: object {number} {damage:?}, {kept} bytes kept, and nothing says so \
             ({} damaged streams)",
            damaged_here.len()
        ));
    }
    tally
}
