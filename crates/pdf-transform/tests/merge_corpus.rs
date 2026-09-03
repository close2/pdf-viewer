//! Every corpus document the suite can open, merged with a fixed second document and drawn.
//!
//! RFC 0002 section 9's layers 2 and 3 over the corpus population, in `split_corpus.rs`'s shape
//! and for its reasons:
//!
//! 1. **Self read-back.** The merged file opens through this tree's own reader, holds exactly
//!    the pages it was given, and carries each page's `/Contents` byte for byte — RFC 0002
//!    section 11.1's "every content stream in their output is a producer's, carried byte for
//!    byte".
//! 2. **The raster oracle, and it is the load-bearing one.** The corpus document's page 1 and
//!    the merged document's page 1 are drawn by the same backend at the same scale and must be
//!    **bit-identical**, and the fixed document's page draws as its own page 1 did in the second
//!    slot. That turns "the reconciliations did not change what a page marks" from a claim into
//!    a gate.
//!
//! Determinism is asserted beside them, and so is one thing `split_corpus` has no equivalent of:
//! **the reconciliations are checked against what the sources stated** — a document with an
//! outline contributes its top-level items to the merged chain, one with `/PageLabels` keeps its
//! first page's label, one with a catalog `/OutputIntents` has it on its own pages rather than on
//! the merged catalog (§14.11.5), and a `/Names` `/Dests` key that collides is renamed (§7.9.6).
//! A merge that quietly dropped one of those would draw identically and pass every other line.
//!
//! # The fixed second document
//!
//! `PDF20_AN001-BPC.pdf`'s **first page**, taken out by `split` so that the second input is one
//! page rather than five: five pages beside every corpus document would make the walk mostly a
//! measurement of the same five pages. It states an outline, `/PageLabels` and an `/AcroForm`,
//! so every corpus document that states one of those meets a collision rather than an empty
//! slot.
//!
//! # What is a failure and what is held
//!
//! A refusal is not a failure: a document `merge` declines by name — §12.7.4.2's field
//! collision above all — is *the document's*, counted by reason and printed (trap 11). What the
//! walk cannot explain goes in [`HELD`] with a diagnosis, and an undiagnosed difference fails
//! the run.
//!
//! Everything is in memory: the corpus is never written to. The reader and the rasteriser that
//! judge the writer are this tree's own, which is trap 8; `tests/merge.rs` holds `qpdf --check`
//! over the committed fixtures as the foreign evidence, and a corpus-wide *foreign* readback is
//! owed for the same reason `doc/todo/57` §5 records it for the other two writers.
//!
//! # Running it
//!
//! ```text
//! tools/bounded.sh --data 4 --tree 8 -- cargo test --profile gates -p pdf-transform --test merge_corpus -- --ignored --nocapture
//! ```

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    reason = "test code: an explanatory panic is the intended failure, and the census output \
              is the point of the run"
)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use pdf_model::Pages;
use pdf_model::page_label::PageLabels;
use pdf_syntax::{Document, Limits, Object, SyntaxError};
use pdf_transform::merge::{Input, MergePlan};
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Refusal, Secret, Source, apply};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

mod support;

use support::check_structure;

/// The dots per inch both rasters are drawn at — `split_corpus`'s, and for its reason: the
/// question is whether two rasters are the same rather than how they look.
const DPI: f32 = 48.0;

/// The corpus documents that refuse §7.6.4.1's default user password, with the password each
/// one's own pdf.js issue records.
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

/// Documents whose merged page does not draw as its source page does, each with its diagnosis.
///
/// Empty is the state to keep. An entry here is a *reading* of why the difference is the
/// document's rather than the suite's, and the walk fails on any difference it does not name.
const HELD: &[(&str, &str)] = &[];

/// What the walk found.
#[derive(Default)]
struct Tally {
    /// Documents the suite could not open, by reason.
    refused_open: Vec<(String, String)>,
    /// Documents with no page to merge.
    pageless: Vec<(String, String)>,
    /// Documents `merge` declined by name, by reason.
    merge_refused: Vec<(String, String)>,
    /// Documents merged into a file this tree read back as two pages.
    merged: usize,
    /// The merged file did not open, or did not hold two pages.
    reread_failed: Vec<(String, String)>,
    /// A carried page's content stream is not its source page's, byte for byte.
    contents_differ: Vec<(String, String)>,
    /// Merged documents whose two pages drew bit-identically to the pages they came from.
    identical: usize,
    /// Merged documents whose pages drew differently.
    differ: Vec<(String, String)>,
    /// Documents whose page neither side would draw, so nothing was compared.
    undrawn: Vec<(String, String)>,
    /// The same merge twice did not write the same bytes.
    nondeterministic: Vec<(String, String)>,
    /// A reconciliation did not do what the source's own construct says it should have.
    unreconciled: Vec<(String, String)>,
    /// Documents that contributed an outline, page labels, an output intent, a form or a
    /// destination-name collision — the population each reconciliation was actually exercised on.
    exercised: [usize; 5],
    /// Outputs that state §14.7.2's `/StructTreeRoot`, so a source's tagging survived.
    structure_carried: usize,
    /// Structure elements the outputs hold in total.
    structure_elements: usize,
    /// Parent-tree keys that resolved: a page's to an array, an object's to a reference.
    structure_resolved: usize,
    /// Outputs whose §14.7 structure states something a clause forbids.
    structure_faults: Vec<(String, String)>,
    /// A document whose examination panicked, which principle 1 forbids.
    panicked: Vec<(String, String)>,
}

/// Adds to the shared tally, ignoring a poisoned lock.
fn record(tally: &Mutex<Tally>, update: impl FnOnce(&mut Tally)) {
    if let Ok(mut tally) = tally.lock() {
        update(&mut tally);
    }
}

/// Fails the gate if this build cannot reach the sandboxed image decoder (trap 10).
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so both sides of every comparison \
             would be drawn without CCITT, JBIG2 or JPEG 2000 images: {error}"
        );
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

/// The password the corpus records for this document, or the empty one.
fn password_for(name: &str) -> &'static str {
    KNOWN_PASSWORDS
        .iter()
        .find(|(known, _)| *known == name)
        .map_or("", |(_, password)| password)
}

/// A source over these bytes, with the corpus's known password where the file has one.
fn source(name: &str, bytes: &[u8]) -> Source {
    let password = password_for(name);
    if password.is_empty() {
        Source::new(bytes.to_vec())
    } else {
        Source::with_password(bytes.to_vec(), Secret::from(password.to_owned()))
    }
}

/// The budget both sides are drawn under, so that a page refused for size is refused twice.
fn budget() -> Budget {
    Budget {
        limits: Limits::DEFAULT,
        max_pixels: 1 << 24,
    }
}

/// The fixed second document: page 1 of a committed one, taken out by `split`.
fn fixed_second() -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
    let bytes = std::fs::read(path).expect("a committed document");
    let sinks = MemorySinks::new();
    apply(
        &Plan::Split(SplitPlan {
            source: 0,
            pages: "1".parse::<Selection>().expect("a selection"),
            pieces: Pieces::EachPage,
            names: "second.pdf".parse().expect("a pattern"),
        }),
        &[Source::new(bytes)],
        &sinks,
        &Policy::default(),
        &budget(),
    )
    .expect("the split applies");
    sinks
        .into_outputs()
        .into_iter()
        .next()
        .expect("one piece")
        .1
}

/// Merges page 1 of `bytes` with page 1 of `second`, answering the merged file.
fn merge_first_pages(name: &str, bytes: &[u8], second: &[u8]) -> Result<Vec<u8>, Refusal> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Merge(MergePlan {
            inputs: vec![
                Input {
                    source: 0,
                    pages: "1".parse::<Selection>().expect("a selection"),
                },
                Input {
                    source: 1,
                    pages: "1".parse::<Selection>().expect("a selection"),
                },
            ],
            collate: false,
            names: "merged.pdf".parse().expect("a pattern"),
        }),
        &[source(name, bytes), Source::new(second.to_vec())],
        &sinks,
        &Policy::default(),
        &budget(),
    )?;
    let mut outputs = sinks.into_outputs();
    if outputs.is_empty() {
        return Err(Refusal::Assembly("no file was written".to_owned()));
    }
    Ok(outputs.remove(0).1)
}

/// Draws one page of these bytes as a PPM, or `None` where nothing was drawn.
fn draw(name: &str, bytes: &[u8], page: usize) -> Option<Vec<u8>> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Render(RenderPlan {
            source: 0,
            pages: page.to_string().parse::<Selection>().expect("a selection"),
            size: Sizing::Dpi(DPI),
            format: ImageFormat::Ppm,
            page_box: None,
            annotations: true,
            names: "page.ppm".parse().expect("a pattern"),
        }),
        &[source(name, bytes)],
        &sinks,
        &Policy::default(),
        &budget(),
    )
    .ok()?;
    let mut outputs = sinks.into_outputs();
    (!outputs.is_empty()).then(|| outputs.remove(0).1)
}

/// One page's `/Contents` as the file holds it: the *encoded* bytes, joined.
fn encoded_contents(document: &Document, index: usize) -> Option<Vec<u8>> {
    let page = Pages::new(document).get(index)?;
    let mut out = Vec::new();
    match document.get_key(&page.dict, "Contents") {
        Object::Stream(stream) => out.extend_from_slice(&stream.data),
        Object::Array(items) => {
            for item in &items {
                if let Some(stream) = document.resolve(item).as_stream() {
                    out.extend_from_slice(&stream.data);
                }
            }
        }
        _ => return None,
    }
    Some(out)
}

/// Whether a catalog entry is stated.
fn states(document: &Document, key: &str) -> bool {
    document
        .catalog()
        .ok()
        .is_some_and(|catalog| catalog.get(key).is_some())
}

/// Whether a document's outline holds at least one top-level item.
fn has_outline_items(document: &Document) -> bool {
    document.catalog().ok().is_some_and(|catalog| {
        document
            .get_key(&catalog, "Outlines")
            .as_dict()
            .and_then(|outlines| outlines.get("First"))
            .and_then(Object::as_reference)
            .is_some_and(|id| document.get(id).as_dict().is_some())
    })
}

/// Whether a document's interactive form states at least one root field.
fn has_root_fields(document: &Document) -> bool {
    document.catalog().ok().is_some_and(|catalog| {
        let form = document.get_key(&catalog, "AcroForm");
        form.as_dict().is_some_and(|form| {
            document
                .get_key(form, "Fields")
                .as_array()
                .is_some_and(|fields| !fields.is_empty())
        })
    })
}

/// One document through the walk.
fn examine(path: &Path, second: &[u8], tally: &Mutex<Tally>) {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let document =
        match Document::open_with_password(bytes.clone(), Limits::DEFAULT, password_for(&name)) {
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
    if Pages::new(&document).is_empty() {
        record(tally, |t| {
            t.pageless
                .push((name, "the document has no page".to_owned()));
        });
        return;
    }

    let merged = match merge_first_pages(&name, &bytes, second) {
        Ok(merged) => merged,
        Err(Refusal::Selection { error, .. }) => {
            record(tally, |t| t.pageless.push((name, error.to_string())));
            return;
        }
        Err(other) => {
            record(tally, |t| t.merge_refused.push((name, other.to_string())));
            return;
        }
    };

    // RFC 0002 section 9's first layer, asserted where it is cheapest to assert.
    match merge_first_pages(&name, &bytes, second) {
        Ok(again) if again == merged => {}
        Ok(_) => record(tally, |t| {
            t.nondeterministic
                .push((name.clone(), "two merges, two files".to_owned()));
        }),
        Err(error) => record(tally, |t| {
            t.nondeterministic
                .push((name.clone(), format!("the second merge refused: {error}")));
        }),
    }

    reread(&name, &document, &merged, tally);
    check_reconciliations(&name, &document, &merged, tally);
    redraw(&name, &bytes, second, &merged, tally);
}

/// RFC 0002 section 9's layer 2 over one merged file.
fn reread(name: &str, document: &Document, merged: &[u8], tally: &Mutex<Tally>) {
    let read = match Document::open_with_limits(merged.to_vec(), Limits::DEFAULT) {
        Ok(read) => read,
        Err(error) => {
            record(tally, |t| {
                t.reread_failed
                    .push((name.to_owned(), format!("does not open: {error}")));
            });
            return;
        }
    };
    let pages = Pages::new(&read);
    if pages.len() != 2 {
        record(tally, |t| {
            t.reread_failed
                .push((name.to_owned(), format!("{} pages, not 2", pages.len())));
        });
        return;
    }
    match (encoded_contents(document, 0), encoded_contents(&read, 0)) {
        (Some(before), Some(after)) if before == after => {}
        (None, None) => {}
        (before, after) => record(tally, |t| {
            t.contents_differ.push((
                name.to_owned(),
                format!(
                    "{} encoded bytes became {}",
                    before.map_or_else(|| "no".to_owned(), |b| b.len().to_string()),
                    after.map_or_else(|| "no".to_owned(), |b| b.len().to_string())
                ),
            ));
        }),
    }
    record(tally, |t| t.merged = t.merged.saturating_add(1));

    // §14.7: the carry, judged against the clauses rather than against what the writer meant —
    // every page's key resolves in the output's own parent tree, every element's `/Pg` names a
    // page this document holds, and `/ParentTreeNextKey` is greater than every key in use.
    let structure = check_structure(&read);
    record(tally, |t| {
        if structure.carried {
            t.structure_carried = t.structure_carried.saturating_add(1);
        }
        t.structure_elements = t.structure_elements.saturating_add(structure.elements);
        t.structure_resolved = t.structure_resolved.saturating_add(
            structure
                .resolved_pages
                .saturating_add(structure.resolved_objects),
        );
        for fault in &structure.faults {
            t.structure_faults.push((name.to_owned(), fault.clone()));
        }
    });
}

/// Each reconciliation checked against what the source stated, and counted so that the run says
/// how large each population actually was (trap 25: a check nothing exercises reads as a pass).
fn check_reconciliations(name: &str, document: &Document, merged: &[u8], tally: &Mutex<Tally>) {
    let Ok(read) = Document::open_with_limits(merged.to_vec(), Limits::DEFAULT) else {
        return;
    };
    let wrong = |detail: String| {
        record(tally, |t| t.unreconciled.push((name.to_owned(), detail)));
    };

    // §12.3.3: the source's outline contributes to the merged chain. Table 150 makes `/First`
    // "( Required if there are any open or closed outline entries )", so an `/Outlines` without
    // one states no items and there is nothing for the merge to carry.
    if has_outline_items(document) {
        record(tally, |t| t.exercised[0] = t.exercised[0].saturating_add(1));
        if !states(&read, "Outlines") {
            wrong(
                "§12.3.3: the source's outline has items and the merge has no /Outlines".to_owned(),
            );
        }
    }
    // §12.4.2: page 1 keeps the label it had.
    let labels = PageLabels::read(document);
    if !labels.is_empty() {
        record(tally, |t| t.exercised[1] = t.exercised[1].saturating_add(1));
        let after = PageLabels::read(&read);
        if after.label(0) != labels.label(0) {
            wrong(format!(
                "§12.4.2: page 1's label was {:?} and is now {:?}",
                labels.label(0),
                after.label(0)
            ));
        }
    }
    // §14.11.5: the array is on the source's own page, not on the merged catalog.
    if states(document, "OutputIntents") {
        record(tally, |t| t.exercised[2] = t.exercised[2].saturating_add(1));
        if states(&read, "OutputIntents") {
            wrong(
                "§14.11.5: the merged catalog states an array, which would claim the other \
                 source's pages too"
                    .to_owned(),
            );
        }
        let on_page = Pages::new(&read)
            .get(0)
            .is_some_and(|page| page.dict.get("OutputIntents").is_some());
        if !on_page {
            wrong("§14.11.5: the carried page states no page-level array".to_owned());
        }
    }
    // §12.7.3: a source with a form contributes it. Table 224's `/Fields` is "( Required ) An
    // array of references to the document's root fields", so an `/AcroForm` with none states no
    // interactive form and the merge states none either.
    if has_root_fields(document) {
        record(tally, |t| t.exercised[3] = t.exercised[3].saturating_add(1));
        if !states(&read, "AcroForm") {
            wrong(
                "§12.7.3: the source states root fields and the merge has no /AcroForm".to_owned(),
            );
        }
    }
    // §7.9.6: a source with named destinations contributes them, under keys that do not overlap.
    if states(document, "Names") || states(document, "Dests") {
        record(tally, |t| t.exercised[4] = t.exercised[4].saturating_add(1));
        let Ok(catalog) = read.catalog() else {
            return;
        };
        let names = read.get_key(&catalog, "Names");
        if let Some(names) = names.as_dict() {
            let root = read.get_key(names, "Dests");
            if let Some(root) = root.as_dict() {
                let keys: Vec<Vec<u8>> =
                    pdf_syntax::tree::name_entries(root, &|object| read.resolve(object))
                        .into_iter()
                        .map(|(key, _)| key)
                        .collect();
                let mut sorted = keys.clone();
                sorted.sort();
                sorted.dedup();
                if sorted.len() != keys.len() {
                    wrong("§7.9.6: the merged /Dests tree holds a key twice".to_owned());
                } else if sorted != keys {
                    wrong("§7.9.6: the merged /Dests tree is not in lexical order".to_owned());
                }
            }
        }
    }
}

/// RFC 0002 section 9's layer 3 over both carried pages.
fn redraw(name: &str, bytes: &[u8], second: &[u8], merged: &[u8], tally: &Mutex<Tally>) {
    let before = draw(name, bytes, 1);
    let fixed = draw("second.pdf", second, 1);
    let first_after = draw(name, merged, 1);
    let second_after = draw(name, merged, 2);
    match (before, first_after) {
        (Some(before), Some(after)) if before == after => {
            record(tally, |t| t.identical = t.identical.saturating_add(1));
        }
        (Some(before), Some(after)) => record(tally, |t| {
            t.differ.push((
                name.to_owned(),
                format!(
                    "page 1: {} bytes of raster became {}{}",
                    before.len(),
                    after.len(),
                    if before.len() == after.len() {
                        ", same size"
                    } else {
                        ""
                    }
                ),
            ));
        }),
        (before, after) => record(tally, |t| {
            t.undrawn.push((
                name.to_owned(),
                format!(
                    "source drew {}, merge drew {}",
                    if before.is_some() { "yes" } else { "no" },
                    if after.is_some() { "yes" } else { "no" }
                ),
            ));
        }),
    }
    // The fixed document's page is the same page every time, so a difference here is the
    // *merge's* — a reconciliation that reached across from the corpus document and changed
    // what the other source's page marks.
    match (fixed, second_after) {
        (Some(before), Some(after)) if before == after => {}
        (Some(_), Some(_)) => record(tally, |t| {
            t.differ.push((
                name.to_owned(),
                "page 2: the fixed second document's page drew differently beside this one"
                    .to_owned(),
            ));
        }),
        _ => {}
    }
}

/// Prints one census list, capped, with its length.
fn print_list(what: &str, entries: &[(String, String)]) {
    println!("transform-merge:   {what}: {}", entries.len());
    for (name, why) in entries.iter().take(40) {
        println!("    {name}: {why}");
    }
    if entries.len() > 40 {
        println!("    … and {} more", entries.len().saturating_sub(40));
    }
}

/// The walk.
#[test]
#[ignore = "corpus-scale: every document's first page merged with a fixed second, re-read and drawn twice; run explicitly under the gates profile"]
fn every_corpus_documents_first_page_survives_being_merged() {
    require_the_sandbox();
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let second = fixed_second();
    let tally = Mutex::new(Tally::default());
    let started = Instant::now();
    files.par_iter().for_each(|path| {
        let name = path.display().to_string();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            examine(path, &second, &tally);
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
        "transform-merge: {} documents in {:.1}s, {} threads, {DPI} dpi",
        files.len(),
        elapsed.as_secs_f64(),
        rayon::current_num_threads()
    );
    print_census(&tally);

    assert!(
        tally.panicked.is_empty(),
        "principle 1: no panic on any input"
    );
    assert!(
        tally.nondeterministic.is_empty(),
        "RFC 0002 section 9: same sources, same plan, same bytes"
    );
    assert!(
        tally.reread_failed.is_empty(),
        "this tree must read back the two pages it wrote"
    );
    assert!(
        tally.structure_faults.is_empty(),
        "§14.7: a carried structure tree states what its clauses require, or none at all"
    );
    assert!(
        tally.contents_differ.is_empty(),
        "§11.1: every content stream in the output is a producer's, carried byte for byte"
    );
    assert!(
        tally.unreconciled.is_empty(),
        "a reconciliation that loses what its source stated is the silent coexistence this verb \
         exists to avoid"
    );
    for (name, why) in &tally.differ {
        assert!(
            HELD.iter().any(|(held, _)| held == name),
            "a merged page that draws differently and nobody has read: {name}: {why}"
        );
    }
    assert!(
        tally.identical > 0,
        "a corpus with no page to merge is not this corpus"
    );
    // trap 25: a check whose population is empty reads as a pass.
    assert!(
        tally.exercised.iter().all(|count| *count > 0),
        "every reconciliation must have been exercised by some document: {:?}",
        tally.exercised
    );
}

/// What §14.7's carry wrote across the walk, and every fault a clause names.
fn print_structure(tally: &Tally) {
    println!(
        "transform-merge:   §14.7 structure trees carried: {}, elements: {}, parent-tree keys \
         resolving: {}",
        tally.structure_carried, tally.structure_elements, tally.structure_resolved
    );
    print_list("§14.7 structure faults", &tally.structure_faults);
}

/// Every list and count the walk gathered, printed in the order a person reads them.
fn print_census(tally: &Tally) {
    print_list("refused open", &tally.refused_open);
    print_list("no page to merge", &tally.pageless);
    print_list("merge refused by name", &tally.merge_refused);
    println!(
        "transform-merge:   merged, re-read as two pages: {}",
        tally.merged
    );
    println!(
        "transform-merge:   page 1 drawn bit-identically to the source page: {}",
        tally.identical
    );
    println!(
        "transform-merge:   reconciliations exercised — outline {}, page labels {}, output \
         intent {}, form {}, named destinations {}",
        tally.exercised[0],
        tally.exercised[1],
        tally.exercised[2],
        tally.exercised[3],
        tally.exercised[4]
    );
    print_list("nothing drawn on one side or both", &tally.undrawn);
    print_list(
        "the content stream did not cross byte for byte",
        &tally.contents_differ,
    );
    print_list(
        "a reconciliation lost what the source stated",
        &tally.unreconciled,
    );
    print_list("drew differently", &tally.differ);
    print_structure(tally);
    print_list("two merges, two files", &tally.nondeterministic);
    print_list("panicked", &tally.panicked);
}
