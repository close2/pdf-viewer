//! Every corpus document the suite can open, given an embedded file and asked for it back.
//!
//! RFC 0002 section 9 calls a corpus pass over every document "the suite's equivalent of the
//! render corpus gate", and `doc/todo/57` §5 owed this one before the serializer lands: the
//! writer's population had been the two committed documents and the corpus's `attachment.pdf`.
//! This walks `doc/pdf.js`'s corpus — the same population every other gate rasterises — and
//! for each document the suite opens:
//!
//! 1. **attaches** a small file into §7.7.4's `/EmbeddedFiles` tree by `attachments --attach`,
//!    and holds §7.5.6's prefix property — "leaving its original contents intact" — the
//!    producer's bytes byte for byte under the update;
//! 2. **reads it back** through `attachments --save` on the updated bytes, the payload equal,
//!    and through `--list`, the name present in the tree and every file the document already
//!    carried still listed beside it;
//! 3. **removes it** by `attachments --remove`, the prefix property holding once more, the
//!    entry gone from the listing and the document's own files still there;
//! 4. **attaches it to page 1** by §12.5.6.15's annotation, and reads it back from there.
//!
//! Everything is in memory: the corpus is never written to, and nothing spawns a process — the
//! reader that judges the writer is this tree's own, which is trap 8's "measured with the
//! instrument under test" and is stated as such. `tests/writer.rs` holds `qpdf --check` over
//! the committed fixtures as the foreign evidence; a corpus-wide foreign readback is the
//! oracle-shaped instrument ADR 0334 prices and this does not take.
//!
//! # What is asserted, and what is only printed
//!
//! The exact assertions bind from the first run and admit no known-failure list: no panic,
//! every update a prefix of its input, every payload read back equal, every removal leaving
//! the listing as it was. **A refusal is not a failure**: a document the writer declines by
//! `UpdateError` — a table rebuilt by scanning has no offset to chain to — or one with no page
//! to file on is *the document's*, counted by reason and printed, never folded into a failure
//! count (trap 11). What the walk cannot yet explain goes in [`HELD`] with a diagnosis, in the
//! oracle's style, and an undiagnosed refusal fails the run. The census counts are printed and
//! not ratcheted, on ADR 0323's rule that a number enters a ratchet after it has held.
//!
//! **The holder shapes are counted too**, because `doc/todo/57` §1 asked for a census of the
//! one `attach` rewrites without a corpus witness — the catalog's `/Names` indirect with the
//! tree direct inside it — and the walk is where such a count belongs.
//!
//! # Running it
//!
//! ```text
//! tools/bounded.sh --data 4 -- cargo test --profile gates -p pdf-transform --test writer_corpus -- --ignored --nocapture
//! ```
//!
//! Its own binary rather than a test in `gate.rs`, for the reason `save_round_trip.rs` gives:
//! `-- --ignored` runs every ignored test in a binary, and the transform gate's floor is a
//! wall-clock number this walk would sit inside.
//!
// no sandbox worker: nothing here decodes an image — the walk writes and reads embedded
// files, and §7.4.6, §7.4.7 and §7.4.9 are never on its path.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "test code: an explanatory panic is the intended failure, and the census output \
              is the point of the run"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use pdf_syntax::{Document, Limits, Object, SyntaxError};
use pdf_transform::attachments::{Action, AttachmentsPlan, OnPage, Payload};
use pdf_transform::{Budget, Listed, MemorySinks, Plan, Policy, Refusal, Secret, Source, apply};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// The bytes attached: a sentence no corpus document contains, so that reading them back is a
/// statement about the update and not about the file.
const PAYLOAD: &[u8] = b"pdf-transform writer corpus witness 875\n";

/// The name the file is filed under, and the same for the annotation's file specification.
const NAME: &str = "pdf-transform-witness-875.txt";

/// The corpus documents that refuse §7.6.4.1's default user password, with the password each
/// one's own pdf.js issue records — `save_round_trip.rs`'s list, so that the population is
/// every document the suite can open rather than every document that opens for free.
const KNOWN_PASSWORDS: &[(&str, &str)] = &[
    ("issue15893_reduced.pdf", "test"),
    ("issue3371.pdf", "ELXRTQWS"),
    ("bug1782186.pdf", "Hello"),
    ("issue6010_1.pdf", "abc"),
    ("issue6010_2.pdf", "\u{E6}\u{F8}\u{E5}"),
    ("saslprep-r6.pdf", "S\u{AA}SL\u{AD}prep"),
    ("pr6531_1.pdf", "asdfasdf"),
    ("print_protection.pdf", "1234"),
];

/// Documents the walk cannot explain yet, each with its diagnosis: a refusal that is neither
/// an `UpdateError` nor a page the document does not have. Empty is the state to keep.
const HELD: &[(&str, &str)] = &[];

/// What one document's examination produced.
#[derive(Default)]
struct Tally {
    /// Documents the suite could not open, by reason.
    refused_open: Vec<(String, String)>,
    /// The holder shape of every document that opened: how the catalog reaches the tree.
    holders: BTreeMap<&'static str, usize>,
    /// Documents the writer declined by name — §7.5.6 cannot honestly be appended — by
    /// `UpdateError`.
    update_refused: Vec<(String, String)>,
    /// A refusal that is neither of the two kinds this walk expects, which [`HELD`] must name.
    unexplained: Vec<(String, String)>,
    /// Documents attached into the tree, read back, and the file removed again.
    attached: usize,
    /// §7.5.6's prefix property failed, on the attach or on the remove.
    prefix_failed: Vec<(String, String)>,
    /// The payload did not come back equal, or the listing was not what it should be.
    readback_failed: Vec<(String, String)>,
    /// Documents with no page 1 to file an annotation on, by reason.
    pageless: Vec<(String, String)>,
    /// Documents attached to page 1 and read back from there.
    page_attached: usize,
    /// A document whose examination panicked, which principle 1 forbids.
    panicked: Vec<(String, String)>,
}

/// Adds to the shared tally, ignoring a poisoned lock (another document's panic is already
/// being reported; losing one entry to it changes nothing).
fn record(tally: &Mutex<Tally>, update: impl FnOnce(&mut Tally)) {
    if let Ok(mut tally) = tally.lock() {
        update(&mut tally);
    }
}

/// The corpus files, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    if files.is_empty() {
        return None;
    }
    files.sort();
    Some(files)
}

/// A source over these bytes, with the corpus's known password where the file has one.
fn source(name: &str, bytes: &[u8]) -> Source {
    match KNOWN_PASSWORDS.iter().find(|(known, _)| *known == name) {
        Some((_, password)) => {
            Source::with_password(bytes.to_vec(), Secret::from((*password).to_owned()))
        }
        None => Source::new(bytes.to_vec()),
    }
}

/// How the catalog reaches §7.7.4's `/EmbeddedFiles` tree: the shapes `attach`'s holder
/// rewrite distinguishes, read off the document independently of the crate.
fn holder_shape(document: &Document) -> &'static str {
    let Ok(catalog) = document.catalog() else {
        return "no catalog";
    };
    let names_entry = catalog.get("Names");
    let names = names_entry.map(|entry| document.resolve(entry));
    let tree_entry = names
        .as_ref()
        .and_then(Object::as_dict)
        .and_then(|names| names.get("EmbeddedFiles"));
    match (names_entry, tree_entry) {
        (None, _) => "no /Names",
        (Some(_), None) if names.as_ref().and_then(Object::as_dict).is_none() => {
            "/Names is not a dictionary"
        }
        (Some(Object::Reference(_)), None) => "/Names indirect, no tree",
        (Some(_), None) => "/Names direct, no tree",
        (Some(Object::Reference(_)), Some(Object::Reference(_))) => {
            "/Names indirect, tree indirect"
        }
        (Some(Object::Reference(_)), Some(_)) => "/Names indirect, tree direct",
        (Some(_), Some(Object::Reference(_))) => "/Names direct, tree indirect",
        (Some(_), Some(_)) => "/Names direct, tree direct",
    }
}

/// A report and the outputs it accounts for, by name.
type Applied = (pdf_transform::Report, Vec<(String, Vec<u8>)>);

/// One `attachments` action over these bytes, answering the report and the outputs.
fn attachments(name: &str, bytes: &[u8], action: Action) -> Result<Applied, Refusal> {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Attachments(AttachmentsPlan { source: 0, action }),
        &[source(name, bytes)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )?;
    Ok((report, sinks.into_outputs()))
}

/// The attach action into the tree, or onto page 1.
fn attach_action(on_page: bool) -> Action {
    Action::Attach {
        payload: Payload::new(PAYLOAD.to_vec()),
        name: NAME.to_owned(),
        description: None,
        date: None,
        names: "out.pdf".parse().expect("a pattern"),
        on_page: on_page.then_some(OnPage {
            page: 1,
            rect: None,
            icon: None,
        }),
    }
}

/// Every file the document lists, as (name, page).
fn listing(name: &str, bytes: &[u8]) -> Result<Vec<(String, Option<usize>)>, String> {
    let (report, _) = attachments(name, bytes, Action::List).map_err(|e| e.to_string())?;
    Ok(report
        .listed
        .iter()
        .map(|entry| match entry {
            Listed::Attachment(entry) => (entry.name.clone(), entry.page),
            Listed::Image(_) => panic!("an image in an attachment listing"),
        })
        .collect())
}

/// Reads the witness back by name and checks its bytes.
fn read_back(name: &str, bytes: &[u8]) -> Result<(), String> {
    let (_, outputs) = attachments(
        name,
        bytes,
        Action::Save {
            name: NAME.to_owned(),
            names: "%t".parse().expect("a pattern"),
        },
    )
    .map_err(|e| format!("--save refused: {e}"))?;
    match outputs.as_slice() {
        [(_, payload)] if payload == PAYLOAD => Ok(()),
        [(_, payload)] => Err(format!(
            "--save gave {} bytes, not the {} attached",
            payload.len(),
            PAYLOAD.len()
        )),
        other => Err(format!("--save wrote {} outputs", other.len())),
    }
}

/// One document through the four steps.
fn examine(path: &Path, tally: &Mutex<Tally>) {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    if std::env::var_os("PDFVIEWER_CORPUS_TRACE").is_some() {
        eprintln!("start {name}");
    }
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let password = KNOWN_PASSWORDS
        .iter()
        .find(|(known, _)| *known == name)
        .map_or("", |(_, password)| password);
    let document = match Document::open_with_password(bytes.clone(), Limits::DEFAULT, password) {
        Ok(document) => document,
        Err(SyntaxError::PasswordRequired) => {
            record(tally, |t| {
                t.refused_open
                    .push((name, "needs a password nobody has recorded".to_owned()));
            });
            return;
        }
        Err(error) => {
            record(tally, |t| t.refused_open.push((name, error.to_string())));
            return;
        }
    };
    let shape = holder_shape(&document);
    record(tally, |t| {
        let count = t.holders.entry(shape).or_default();
        *count = count.saturating_add(1);
    });
    if tree_round_trip(&name, &bytes, tally) {
        page_round_trip(&name, &bytes, tally);
    }
    if std::env::var_os("PDFVIEWER_CORPUS_TRACE").is_some() {
        eprintln!("done  {name}");
    }
}

/// The listing sorted, for comparing two of them regardless of the order the homes are
/// walked in.
fn sorted(mut listing: Vec<(String, Option<usize>)>) -> Vec<(String, Option<usize>)> {
    listing.sort();
    listing
}

/// Step 1: the update carrying the witness in the tree, or `None` with the refusal counted.
fn attach_into_tree(name: &str, bytes: &[u8], tally: &Mutex<Tally>) -> Option<Vec<u8>> {
    let name = name.to_owned();
    match attachments(&name, bytes, attach_action(false)) {
        Ok((_, mut outputs)) => Some(outputs.remove(0).1),
        Err(Refusal::Update { error, .. }) => {
            record(tally, |t| t.update_refused.push((name, error.to_string())));
            None
        }
        // The document opened already, so this is `attach` finding no catalog to hold the
        // tree — §7.5.5's `/Root` naming something that is not a dictionary — which is the
        // file's, in the same family as the writer's own `NoRoot`.
        Err(Refusal::Unopenable { error, .. }) => {
            record(tally, |t| {
                t.update_refused
                    .push((name, format!("no catalog to hold the tree ({error})")));
            });
            None
        }
        Err(other) => {
            record(tally, |t| t.unexplained.push((name, other.to_string())));
            None
        }
    }
}

/// Steps 1 to 3: into the tree, read back, and out again. Answers whether the document is one
/// the writer takes at all, so that the page route is tried only where it can be.
fn tree_round_trip(name: &str, bytes: &[u8], tally: &Mutex<Tally>) -> bool {
    let name = name.to_owned();
    let before = match listing(&name, bytes) {
        Ok(before) => sorted(before),
        Err(why) => {
            record(tally, |t| {
                t.readback_failed
                    .push((name, format!("listing the source: {why}")));
            });
            return false;
        }
    };
    // 1. Attach into the tree.
    let Some(updated) = attach_into_tree(&name, bytes, tally) else {
        return false;
    };
    if !updated.starts_with(bytes) {
        record(tally, |t| t.prefix_failed.push((name, "attach".to_owned())));
        return false;
    }
    // 2. Read it back, and everything the document had beside it.
    if let Err(why) = read_back(&name, &updated) {
        record(tally, |t| t.readback_failed.push((name, why)));
        return false;
    }
    let mut with_witness = before.clone();
    with_witness.push((NAME.to_owned(), None));
    match listing(&name, &updated).map(sorted) {
        Ok(after) if after == sorted(with_witness) => {}
        Ok(after) => {
            record(tally, |t| {
                t.readback_failed.push((
                    name,
                    format!(
                        "after attach the listing is {after:?}, not {before:?} plus the witness"
                    ),
                ));
            });
            return false;
        }
        Err(why) => {
            record(tally, |t| {
                t.readback_failed
                    .push((name, format!("listing after attach: {why}")));
            });
            return false;
        }
    }
    // 3. Remove it.
    let removed = match attachments(
        &name,
        &updated,
        Action::Remove {
            name: NAME.to_owned(),
            names: "out.pdf".parse().expect("a pattern"),
        },
    ) {
        Ok((_, mut outputs)) => outputs.remove(0).1,
        Err(error) => {
            record(tally, |t| {
                t.readback_failed
                    .push((name, format!("--remove refused: {error}")));
            });
            return false;
        }
    };
    if !removed.starts_with(&updated) {
        record(tally, |t| t.prefix_failed.push((name, "remove".to_owned())));
        return false;
    }
    match listing(&name, &removed).map(sorted) {
        Ok(after) if after == before => {}
        Ok(after) => {
            record(tally, |t| {
                t.readback_failed.push((
                    name,
                    format!("after remove the listing is {after:?}, not the source's {before:?}"),
                ));
            });
            return false;
        }
        Err(why) => {
            record(tally, |t| {
                t.readback_failed
                    .push((name, format!("listing after remove: {why}")));
            });
            return false;
        }
    }
    record(tally, |t| t.attached = t.attached.saturating_add(1));
    true
}

/// Step 4: on page 1, and read back from there.
fn page_round_trip(name: &str, bytes: &[u8], tally: &Mutex<Tally>) {
    let name = name.to_owned();
    let on_page = match attachments(&name, bytes, attach_action(true)) {
        Ok((_, mut outputs)) => outputs.remove(0).1,
        Err(Refusal::NoSuchPage { count, .. }) => {
            record(tally, |t| {
                t.pageless
                    .push((name, format!("the document has {count} pages")));
            });
            return;
        }
        Err(Refusal::Update { error, .. }) => {
            record(tally, |t| t.pageless.push((name, error.to_string())));
            return;
        }
        Err(other) => {
            record(tally, |t| {
                t.unexplained.push((name, format!("--to-page 1: {other}")));
            });
            return;
        }
    };
    if !on_page.starts_with(bytes) {
        record(tally, |t| {
            t.prefix_failed.push((name, "attach --to-page".to_owned()));
        });
        return;
    }
    if let Err(why) = read_back(&name, &on_page) {
        record(tally, |t| {
            t.readback_failed
                .push((name, format!("--to-page 1: {why}")));
        });
        return;
    }
    match listing(&name, &on_page) {
        Ok(after) if after.contains(&(NAME.to_owned(), Some(1))) => {}
        Ok(after) => {
            record(tally, |t| {
                t.readback_failed.push((
                    name,
                    format!(
                        "--to-page 1: the listing {after:?} does not file the witness on page 1"
                    ),
                ));
            });
            return;
        }
        Err(why) => {
            record(tally, |t| {
                t.readback_failed
                    .push((name, format!("listing after --to-page: {why}")));
            });
            return;
        }
    }
    record(tally, |t| {
        t.page_attached = t.page_attached.saturating_add(1);
    });
}

/// Prints one census list, capped, with its length.
fn print_list(what: &str, entries: &[(String, String)]) {
    println!("transform-writer:   {what}: {}", entries.len());
    for (name, why) in entries.iter().take(40) {
        println!("    {name}: {why}");
    }
    if entries.len() > 40 {
        println!("    … and {} more", entries.len().saturating_sub(40));
    }
}

/// The walk.
#[test]
#[ignore = "corpus-scale: every document attached into, read back and removed from; run explicitly under the gates profile"]
fn every_corpus_document_takes_an_attachment_and_gives_it_back() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let tally = Mutex::new(Tally::default());
    let started = Instant::now();
    files.par_iter().for_each(|path| {
        let name = path.display().to_string();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            examine(path, &tally);
        }));
        if let Err(payload) = outcome {
            let what = payload
                .downcast_ref::<&str>()
                .map(ToString::to_string)
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "non-string panic payload".to_owned());
            record(&tally, |t| t.panicked.push((name, what)));
        }
    });
    let elapsed = started.elapsed();
    let tally = tally.into_inner().expect("reported through the tally");

    println!(
        "transform-writer: {} documents in {:.1}s, {} threads",
        files.len(),
        elapsed.as_secs_f64(),
        rayon::current_num_threads()
    );
    print_list("refused open", &tally.refused_open);
    println!("transform-writer:   holder shapes (how the catalog reaches /EmbeddedFiles):");
    for (shape, count) in &tally.holders {
        println!("    {count:5}  {shape}");
    }
    print_list(
        "attach refused by the writer (§7.5.6 cannot be appended)",
        &tally.update_refused,
    );
    println!(
        "transform-writer:   attached into the tree, read back and removed: {}",
        tally.attached
    );
    print_list("no page to file on", &tally.pageless);
    println!(
        "transform-writer:   attached to page 1 and read back: {}",
        tally.page_attached
    );
    print_list("prefix failed", &tally.prefix_failed);
    print_list("readback failed", &tally.readback_failed);
    print_list("unexplained refusals", &tally.unexplained);
    print_list("panicked", &tally.panicked);

    assert!(
        tally.panicked.is_empty(),
        "principle 1: no panic on any input"
    );
    assert!(
        tally.prefix_failed.is_empty(),
        "§7.5.6: the input's bytes must be a prefix of every update"
    );
    assert!(
        tally.readback_failed.is_empty(),
        "this tree must read back what it wrote"
    );
    for (name, why) in &tally.unexplained {
        assert!(
            HELD.iter().any(|(held, _)| held == name),
            "an undiagnosed refusal: {name}: {why}"
        );
    }
    assert!(
        tally.attached > 0,
        "a corpus with no document to attach into is not this corpus"
    );
}
