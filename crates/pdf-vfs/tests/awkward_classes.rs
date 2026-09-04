//! Every layout entry, against a document of each awkward class, through the confined transport.
//!
//! # The question this asks, and why it is not the read walk's
//!
//! `tests/read_corpus.rs` (session 914) asks whether the two transports *agree*: every file the
//! layout offers, over the 974 pdf.js documents, held byte for byte against the generator
//! `crate::layout` names. This asks a narrower question over a wider population: **does the
//! confined worker survive at all**, for a document of each class that is awkward in a way the
//! pdf.js corpus under-populates, and drawn from every corpus on the disk rather than from that
//! one.
//!
//! The reason for a second instrument is three findings of one shape. A confined worker was
//! killed by `SIGSYS` three times — session 902's `available_parallelism` reading
//! `/proc/self/cgroup`, session 911's allocator sizing an arena from `/sys` on a pool thread,
//! session 914's `pdf_font::substitute` walking `/usr/share/fonts` — and each was found by
//! somebody taking a path no test had taken. Session 916 reported a fourth, "a page of an
//! *encrypted* document kills the confined worker", and session 917 measured it: it is session
//! 914's, with an encrypted document as the accident that carried it. Encryption was never the
//! variable. **A class of document nothing has put through the confinement is a kill nobody has
//! found yet**, so the classes are enumerated and swept rather than waited for.
//!
//! # What is swept
//!
//! - **The population is derived, never named** (trap 25). Every corpus root on this disk is
//!   sampled at a fixed stride, each sampled document is *classified* by opening it — encrypted,
//!   locked, an encryption this reader does not implement, pageless, damaged, huge, and by a walk
//!   of its objects for a `/JBIG2Decode` or `/JPXDecode` image — and each class takes the first
//!   [`PER_CLASS`] documents that fall into it. A class no corpus fills is printed as empty
//!   rather than passing silently.
//! - **The tree is walked from `/` rather than from a list**, so a row added to
//!   [`pdf_vfs::layout::LAYOUT`] is swept without this file being edited: every directory is
//!   listed, the first [`WIDTH`] of its entries are `stat`ed and read, and a directory recurses.
//!   [`WIDTH`] is what keeps this cheap on the class where the read walk is expensive — a `stat`
//!   generates (RFC 0003 section 5.5), so `pages/` of a thousand-page document is a thousand page
//!   extractions and session 911 measured that at 2 min 45 s.
//! - **The transport is the confined one**, over a [`FileBacking`], which is the posture a face
//!   has: the document crosses as a descriptor the worker could not have opened (ADR 0812).
//!
//! # What fails it
//!
//! A refusal is not a failure (trap 11): a locked document, a page past the budget, a codec this
//! tree does not have, a document `pdf_transform` declines by name — each is counted and printed.
//! **What fails is a death**: a worker killed by a signal, which is a `WorkerError::Transport`
//! whose sentence names one. The run also checks that the mount *recovers* from any error it
//! meets — session 902 built that, and a recovery nothing exercises is a claim — by asking the
//! tree its page count afterwards.
//!
//! # Running it
//!
//! ```text
//! cargo build --profile gates -p pdf-vfs -p pdf-sandbox --bins
//! tools/bounded.sh --data 12 --tree 12 -- \
//!     cargo test --profile gates -p pdf-vfs --test awkward_classes -- --ignored --nocapture
//! ```

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    reason = "test code: an explanatory panic is the intended failure, and the matrix is the \
              point of the run"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use pdf_syntax::{Document, FileBytes, Limits, Object, SyntaxError};
use pdf_transform::Budget;
use pdf_vfs::layout::Kind;
use pdf_vfs::worker::WorkerError;
use pdf_vfs::{Config, ConfinedWorkers, FileBacking, Vfs, VfsError};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// How many documents are classified from each corpus root.
///
/// A stride over the sorted names rather than the first N, because a corpus directory's first
/// hundred names are one contributor's and one generator's. Classification opens the document and
/// walks its objects, which is the cost this bounds.
const SAMPLE_PER_ROOT: usize = 1200;

/// How many documents of each class are swept, at most, from each root.
const PER_CLASS: usize = 8;

/// How many distinct refusal reasons the report prints.
const REASONS_PRINTED: usize = 20;

/// How many entries of each directory are `stat`ed and read.
///
/// A `stat` generates, so this is the whole cost of the walk on a long document. Two rather than
/// one because the first entry of a directory is the one every other test reads.
const WIDTH: usize = 2;

/// The pixel ceiling both a mount and its worker are held to, `tests/read_corpus.rs`'s.
///
/// 2²⁴ is four times a 300 dpi A4 page and a sixteenth of `Budget::default`'s, which matters here:
/// the *huge* class is in the population on purpose and a gibibyte of RGBA a page across rayon is
/// how the machine went into swap (ADR 0798).
fn budget() -> Budget {
    Budget {
        limits: Limits::DEFAULT,
        max_pixels: 1 << 24,
    }
}

/// A class of document that is awkward in a way worth sweeping on its own.
///
/// The vocabulary is `safedocs::survey::Outcome`'s, which already names five of these for the
/// corpus survey, plus the three this round's question adds: a document too large to be walked
/// cheaply, and the two image codecs that are a separate program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Class {
    /// §7.6 encryption that opens under §7.6.4.1's default user password.
    Encrypted,
    /// §7.6 encryption that refuses it: a person would be asked for a password.
    Locked,
    /// §7.6 encryption this reader does not implement.
    EncryptionUnread,
    /// Opens, and reaches no page.
    Pageless,
    /// Opens only because the cross-reference table was rebuilt by scanning.
    Damaged,
    /// Does not open at all.
    Unopenable,
    /// A hundred pages or more, or eight mebibytes or more of file.
    Huge,
    /// States a `/JBIG2Decode` image, which is decoded by another program still.
    Jbig2,
    /// States a `/JPXDecode` image, likewise.
    Jpeg2000,
    /// None of the above: the control, because a sweep that only meets awkward documents cannot
    /// say whether what it found is the class or the walk.
    Plain,
}

impl Class {
    /// Every one, in the order the report prints them.
    const ALL: &'static [Self] = &[
        Self::Encrypted,
        Self::Locked,
        Self::EncryptionUnread,
        Self::Pageless,
        Self::Damaged,
        Self::Unopenable,
        Self::Huge,
        Self::Jbig2,
        Self::Jpeg2000,
        Self::Plain,
    ];

    /// What it is called in the report.
    fn name(self) -> &'static str {
        match self {
            Self::Encrypted => "encrypted",
            Self::Locked => "locked",
            Self::EncryptionUnread => "encryption unimplemented",
            Self::Pageless => "pageless",
            Self::Damaged => "damaged",
            Self::Unopenable => "unopenable",
            Self::Huge => "huge",
            Self::Jbig2 => "jbig2",
            Self::Jpeg2000 => "jpeg 2000",
            Self::Plain => "plain (control)",
        }
    }
}

/// What one question of the tree answered.
#[derive(Debug, Clone)]
enum Outcome {
    /// It answered, with this many bytes where it was a file.
    Answered(u64),
    /// It refused, by name.
    Refused(String),
    /// The worker died: the failure this sweep exists to find.
    Killed(String),
}

/// One document's whole walk.
#[derive(Debug)]
struct Walked {
    /// Which document.
    name: String,
    /// Which class it was swept as.
    class: Class,
    /// Every path visited and what it answered.
    cells: Vec<(String, Outcome)>,
    /// Whether the mount was still usable after the walk, which is session 902's recovery.
    recovered: bool,
}

/// Whether a refusal is a worker that died rather than one that declined.
///
/// `WorkerError::Transport` is every way the wire can fail, and `confined_transport`'s supervision
/// words a signal death as `killed by signal N` — so this asks for the sentence rather than for
/// the variant. A broker that could not *start* a worker is the same variant and is not a death.
fn killed(error: &VfsError) -> Option<String> {
    match error {
        VfsError::Worker(WorkerError::Transport(detail))
            if detail.to_string().contains("killed by signal") =>
        {
            Some(detail.to_string())
        }
        _ => None,
    }
}

/// The corpus roots on this disk, each with the name it is reported under.
fn roots() -> Vec<(String, PathBuf)> {
    let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    let mut consider = |name: &str, path: PathBuf| {
        if path.is_dir() {
            found.push((name.to_owned(), path));
        }
    };
    consider("pdf.js", tree.join("doc/pdf.js/test/pdfs"));
    for corpora in [tree.join("doc/corpora"), tree.join("corpus-cache")] {
        let Ok(entries) = std::fs::read_dir(&corpora) else {
            continue;
        };
        let mut names: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        names.sort();
        for path in names {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let name = name.to_owned();
                consider(&name, path);
            }
        }
    }
    found
}

/// Every `.pdf` under a root, sorted, however deep.
fn documents_under(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// A fixed-stride sample of a population, so that a root's first names are not the whole answer.
fn sampled(mut documents: Vec<PathBuf>, most: usize) -> Vec<PathBuf> {
    if documents.len() <= most {
        return documents;
    }
    let stride = documents.len().checked_div(most).unwrap_or(1).max(1);
    documents = documents.into_iter().step_by(stride).take(most).collect();
    documents
}

/// Which classes this document falls into, by opening it and looking.
///
/// A document is in as many as it satisfies: an encrypted, damaged, thousand-page scan states
/// three things about itself and each is a reason to sweep it.
fn classify(path: &Path) -> Vec<Class> {
    let mut classes = Vec::new();
    let Ok(metadata) = std::fs::metadata(path) else {
        return classes;
    };
    let Ok(bytes) = FileBytes::on_disk(path) else {
        return classes;
    };
    let document = match Document::open_with_limits(bytes, Limits::DEFAULT) {
        Ok(document) => document,
        Err(SyntaxError::PasswordRequired) => {
            classes.push(Class::Locked);
            return classes;
        }
        Err(error) => {
            classes.push(if error.to_string().contains("encrypt") {
                Class::EncryptionUnread
            } else {
                Class::Unopenable
            });
            return classes;
        }
    };

    if document.is_encrypted() {
        classes.push(Class::Encrypted);
    }
    if document.was_recovered() {
        classes.push(Class::Damaged);
    }
    let pages = pdf_model::Pages::new(&document).len();
    if pages == 0 {
        classes.push(Class::Pageless);
    }
    if pages >= 100 || metadata.len() >= 8 << 20 {
        classes.push(Class::Huge);
    }
    for (filter, class) in [
        ("JBIG2Decode", Class::Jbig2),
        ("JPXDecode", Class::Jpeg2000),
    ] {
        if states_filter(&document, filter) {
            classes.push(class);
        }
    }
    if classes.is_empty() {
        classes.push(Class::Plain);
    }
    classes
}

/// Whether any object of this document is a stream filtered by this name.
///
/// The objects are walked rather than the bytes scanned, because a compressed object stream hides
/// every name a `grep` would look for, and those are exactly the documents a modern producer
/// writes.
fn states_filter(document: &Document, filter: &str) -> bool {
    document.xref().object_numbers().any(|number| {
        let object = document.get(pdf_syntax::ObjectId::new(number, 0));
        let Object::Stream(stream) = &object else {
            return false;
        };
        let filters = document.get_key(&stream.dict, "Filter");
        match &filters {
            Object::Name(name) => name.as_str() == Some(filter),
            Object::Array(names) => names.iter().any(|entry| {
                matches!(document.resolve(entry), Object::Name(name) if name.as_str() == Some(filter))
            }),
            _ => false,
        }
    })
}

/// A tree over this document, on the confined transport, with no password.
///
/// No password on purpose: the *locked* class is in the sweep to see that a mount refuses it by
/// name, and one supplied here would take that row out of the matrix.
fn mounted(path: &Path) -> Vfs {
    Vfs::new(
        Box::new(FileBacking::new(path)),
        Box::new(ConfinedWorkers::default()),
        Config {
            budget: budget(),
            ..Config::default()
        },
    )
}

/// Walks the whole tree of one mount, bounded by [`WIDTH`] at each directory.
fn walk(vfs: &Vfs, path: &str, cells: &mut Vec<(String, Outcome)>) {
    let entries = match vfs.list(path) {
        Ok(entries) => {
            cells.push((path.to_owned(), Outcome::Answered(0)));
            entries
        }
        Err(error) => {
            cells.push((path.to_owned(), outcome_of(&error)));
            return;
        }
    };
    for entry in entries.iter().take(WIDTH) {
        let child = if path == "/" {
            format!("/{}", entry.name)
        } else {
            format!("{path}/{}", entry.name)
        };
        match entry.kind {
            Kind::Directory => walk(vfs, &child, cells),
            Kind::File => cells.push((child.clone(), read_one(vfs, &child))),
        }
    }
}

/// A `stat` and a whole read of one file, which is what `cp` out of a mount does.
fn read_one(vfs: &Vfs, path: &str) -> Outcome {
    match vfs.stat(path) {
        Err(error) => outcome_of(&error),
        Ok(_) => match vfs.open(path) {
            Err(error) => outcome_of(&error),
            Ok(handle) => Outcome::Answered(handle.len()),
        },
    }
}

/// One error, read as a death or as a refusal.
fn outcome_of(error: &VfsError) -> Outcome {
    killed(error).map_or_else(|| Outcome::Refused(error.to_string()), Outcome::Killed)
}

/// Phase one: every root sampled, classified, and the first [`PER_CLASS`] of each class kept.
///
/// Per root rather than over the whole population, so that one collection of 65 944 documents
/// cannot fill every bucket by itself. The count of documents classified comes back beside the
/// choice, because it is the denominator the report's numbers mean anything against.
fn choose(roots: &[(String, PathBuf)]) -> (BTreeMap<Class, Vec<(String, PathBuf)>>, usize) {
    let mut chosen: BTreeMap<Class, Vec<(String, PathBuf)>> = BTreeMap::new();
    let mut classified = 0usize;
    for (root_name, root) in roots {
        let sample = sampled(documents_under(root), SAMPLE_PER_ROOT);
        classified = classified.saturating_add(sample.len());
        let verdicts = Mutex::new(Vec::new());
        sample.par_iter().for_each(|path| {
            let classes = classify(path);
            verdicts
                .lock()
                .expect("the verdicts")
                .push((path.clone(), classes));
        });
        let mut verdicts = verdicts.into_inner().expect("the verdicts");
        verdicts.sort_by(|left, right| left.0.cmp(&right.0));
        let mut taken: BTreeMap<Class, usize> = BTreeMap::new();
        for (path, classes) in verdicts {
            for class in classes {
                let count = taken.entry(class).or_default();
                if *count >= PER_CLASS {
                    continue;
                }
                *count = count.saturating_add(1);
                let name = format!(
                    "{root_name}/{}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
                chosen.entry(class).or_default().push((name, path.clone()));
            }
        }
    }
    (chosen, classified)
}

/// Phase two: one mount per chosen document, its whole tree walked through the confinement.
fn sweep(chosen: &BTreeMap<Class, Vec<(String, PathBuf)>>) -> Vec<Walked> {
    let flat: Vec<(Class, String, PathBuf)> = chosen
        .iter()
        .flat_map(|(class, documents)| {
            documents
                .iter()
                .map(move |(name, path)| (*class, name.clone(), path.clone()))
        })
        .collect();
    let walked = Mutex::new(Vec::new());
    flat.par_iter().for_each(|(class, name, path)| {
        let vfs = mounted(path);
        let mut cells = Vec::new();
        walk(&vfs, "/", &mut cells);
        // Session 902's recovery: a worker that died is thrown away and the *next* operation
        // starts a fresh one. Asked after the walk, so that a death anywhere in it is followed
        // by a question the mount still has to answer.
        //
        // **What is asked is that the answer is not a corpse** rather than that it is a page
        // count, and the first form of this check got that wrong: a locked document, an
        // unopenable one and an encryption this reader does not implement all answer `Err` here
        // for ever and are *right* to, so requiring `Ok` reported eleven recovery failures that
        // were eleven refusals by name (trap 11).
        let recovered = vfs.pages().err().as_ref().and_then(killed).is_none();
        walked.lock().expect("the walks").push(Walked {
            name: name.clone(),
            class: *class,
            cells,
            recovered,
        });
    });
    let mut walked = walked.into_inner().expect("the walks");
    walked.sort_by(|left, right| (left.class, &left.name).cmp(&(right.class, &right.name)));
    walked
}

/// How many cells of these walks are of one kind.
fn count(walks: &[&Walked], of: fn(&Outcome) -> bool) -> usize {
    walks
        .iter()
        .map(|walk| walk.cells.iter().filter(|(_, out)| of(out)).count())
        .sum()
}

/// The class-by-class report, and the two lists that decide whether the run passes.
fn report(walked: &[Walked]) -> (Vec<String>, Vec<String>) {
    let mut kills: Vec<String> = Vec::new();
    let mut unrecovered: Vec<String> = Vec::new();
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    for class in Class::ALL {
        let mine: Vec<&Walked> = walked.iter().filter(|walk| walk.class == *class).collect();
        if mine.is_empty() {
            println!(
                "vfs-awkward:   {:<24} no document on this disk",
                class.name()
            );
            continue;
        }
        let answered = count(&mine, |out| matches!(out, Outcome::Answered(_)));
        let refused = count(&mine, |out| matches!(out, Outcome::Refused(_)));
        let died = count(&mine, |out| matches!(out, Outcome::Killed(_)));
        let bytes: u64 = mine
            .iter()
            .flat_map(|walk| walk.cells.iter())
            .map(|(_, outcome)| match outcome {
                Outcome::Answered(bytes) => *bytes,
                Outcome::Refused(_) | Outcome::Killed(_) => 0,
            })
            .sum();
        println!(
            "vfs-awkward:   {:<24} {} document(s), {answered} answered ({:.1} MiB), \
             {refused} refused, {died} killed",
            class.name(),
            mine.len(),
            bytes as f64 / (1024.0 * 1024.0)
        );
        for entry in mine {
            if !entry.recovered {
                unrecovered.push(entry.name.clone());
            }
            for (path, outcome) in &entry.cells {
                match outcome {
                    Outcome::Answered(_) => {}
                    Outcome::Refused(sentence) => {
                        let seen = refusals.entry(first_sentence(sentence)).or_default();
                        *seen = seen.saturating_add(1);
                    }
                    Outcome::Killed(detail) => {
                        kills.push(format!("{} {path}: {detail}", entry.name));
                    }
                }
            }
        }
    }

    println!("vfs-awkward: refused, by reason:");
    let mut by_count: Vec<(&String, &usize)> = refusals.iter().collect();
    by_count.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
    for (reason, seen) in by_count.iter().take(REASONS_PRINTED) {
        println!("vfs-awkward:   {seen:>4}  {reason}");
    }
    (kills, unrecovered)
}

/// Fails the run if this build cannot reach the sandboxed image decoder.
///
/// Trap 10: `JBIG2Decode` and `JPXDecode` are decoded by a separate program Cargo does not build
/// for a test of another package, and two of this sweep's ten classes are named after them. A run
/// that could not reach it would walk those documents with their images missing and would survive
/// for that reason.
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so the JBIG2 and JPEG 2000 classes \
             would be swept with their images missing and would pass for that reason: {error}"
        );
    }
}

/// The whole matrix.
#[test]
#[ignore = "walks every corpus on the disk: run it as doc/todo/02 section 2 says"]
fn every_layout_entry_against_every_awkward_class() {
    require_the_sandbox();

    let started = Instant::now();
    let roots = roots();
    assert!(!roots.is_empty(), "no corpus root on this disk");

    let (chosen, classified) = choose(&roots);
    println!(
        "vfs-awkward: {} root(s), {classified} document(s) classified, {} chosen",
        roots.len(),
        chosen.values().map(Vec::len).sum::<usize>()
    );

    let walked = sweep(&chosen);
    let (kills, unrecovered) = report(&walked);
    println!(
        "vfs-awkward: killed: {}, did not recover: {}, in {:.1}s",
        kills.len(),
        unrecovered.len(),
        started.elapsed().as_secs_f64()
    );

    assert!(
        kills.is_empty(),
        "the confined worker was killed by a signal on {} question(s):\n{}",
        kills.len(),
        kills.join("\n")
    );
    assert!(
        unrecovered.is_empty(),
        "a mount did not answer after its walk, so a dead worker was not replaced: {unrecovered:?}"
    );
}

/// A refusal's first line with the path taken off it, so that the report groups by *reason*
/// rather than by document.
fn first_sentence(sentence: &str) -> String {
    let head = sentence.lines().next().unwrap_or(sentence);
    match head.split_once(": ") {
        Some((_, reason)) if !reason.is_empty() => reason.to_owned(),
        _ => head.to_owned(),
    }
}
