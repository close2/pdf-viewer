//! Every corpus document the suite can open, split at its first page and drawn twice.
//!
//! RFC 0002 section 9 gives a transform four layers of correctness, strongest first, and this walk is
//! the second and the third of them over the corpus population:
//!
//! 1. **Self read-back.** The piece is opened by this tree's own reader — the one 974 corpus
//!    documents go through — and its declared structure is checked: one page, the catalog
//!    reachable, the page's `/Contents` byte-identical to the source page's where the source
//!    states one, because RFC 0002 section 11.1's redrawn exclusion is that "every content stream in
//!    their output is a producer's, carried byte for byte".
//! 2. **The raster oracle, and it is the load-bearing one.** Page 1 of the piece and page 1 of
//!    the source are drawn by the same backend at the same scale, and the two rasters must be
//!    **bit-identical**. RFC 0002 section 9: "[t]his turns 'appearance-preserving' from a claim into a
//!    gate", and it is derivable from the specification rather than from any other tool — the
//!    same content stream, the same resources and the same boxes shall mark the same pixels.
//!
//! Determinism is asserted beside them: the same document split twice writes the same bytes,
//! which is RFC 0002 section 9's first layer and what makes the other two tests rather than demos.
//!
//! # What is a failure and what is held
//!
//! The assertions bind from the first run. **A refusal is not a failure**: a document the
//! serializer declines by name, or one with no page to split, is *the document's*, counted by
//! reason and printed (trap 11). What the walk cannot explain goes in [`HELD`] with a
//! diagnosis, in the oracle's style, and an undiagnosed difference fails the run.
//!
//! Everything is in memory: the corpus is never written to. The reader and the rasteriser that
//! judge the writer are this tree's own, which is trap 8 and is stated as such;
//! `tests/split.rs` holds `qpdf --check` over committed fixtures as the foreign evidence.
//!
//! # Running it
//!
//! ```text
//! tools/bounded.sh --data 4 --tree 8 -- cargo test --profile gates -p pdf-transform --test split_corpus -- --ignored --nocapture
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

use pdf_syntax::{Document, Limits, Object, SyntaxError};
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Refusal, Secret, Source, apply};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

mod support;

use support::check_structure;

/// The dots per inch both rasters are drawn at.
///
/// Low, because the question is whether two rasters are the same rather than how they look, and
/// a corpus-wide pass at 150 dpi would spend its wall clock on pixels nobody compares. The
/// scale still exercises every stage: the same interpreter, the same rasteriser, the same
/// fonts.
const DPI: f32 = 48.0;

/// The corpus documents that refuse §7.6.4.1's default user password, with the password each
/// one's own pdf.js issue records — `writer_corpus.rs`'s list, so that the population is every
/// document the suite can open rather than every document that opens for free.
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

/// Documents whose piece does not draw as its source page does, each with its diagnosis.
///
/// Empty is the state to keep. An entry here is a *reading* of why the difference is the
/// document's rather than the suite's, and the walk fails on any difference it does not name.
const HELD: &[(&str, &str)] = &[];

/// What the walk found.
#[derive(Default)]
struct Tally {
    /// Documents the suite could not open, by reason.
    refused_open: Vec<(String, String)>,
    /// Documents `split` declined by name, by reason.
    split_refused: Vec<(String, String)>,
    /// Documents with no page to split.
    pageless: Vec<(String, String)>,
    /// Documents split into a piece that this tree read back as one page.
    split: usize,
    /// The piece did not open, or did not hold one page.
    reread_failed: Vec<(String, String)>,
    /// The piece's content stream is not the source page's, byte for byte.
    contents_differ: Vec<(String, String)>,
    /// Pieces whose page 1 drew bit-identically to the source's page 1.
    identical: usize,
    /// Pieces whose page 1 drew differently.
    differ: Vec<(String, String)>,
    /// Documents whose page neither side would draw, so nothing was compared.
    undrawn: Vec<(String, String)>,
    /// The same document split twice did not write the same bytes.
    nondeterministic: Vec<(String, String)>,
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

/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a separate program, and Cargo
/// does not build another package's binaries when it tests this one (trap 10). Both sides of
/// this walk's comparison are drawn by the same build, so a missing worker would not make them
/// *disagree* — it would make them agree on pages with the images missing, which is a weaker
/// gate wearing the same number. The walk refuses to be that.
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
        // A corpus-wide pass holds several rasters at once across rayon, so the per-page
        // ceiling is well under the seam's default; a page past it is refused on both sides
        // and lands in `undrawn` rather than in a difference.
        max_pixels: 1 << 24,
    }
}

/// Splits page 1 out of these bytes, answering the piece.
fn split_first_page(name: &str, bytes: &[u8]) -> Result<Vec<u8>, Refusal> {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Split(SplitPlan {
            source: 0,
            pages: "1".parse::<Selection>().expect("a selection"),
            pieces: Pieces::EachPage,
            names: "piece.pdf".parse().expect("a pattern"),
        }),
        &[source(name, bytes)],
        &sinks,
        &Policy::default(),
        &budget(),
    )?;
    let mut outputs = sinks.into_outputs();
    if outputs.is_empty() {
        let why = report
            .refused
            .first()
            .map_or_else(|| "no piece was written".to_owned(), |d| d.detail.clone());
        return Err(Refusal::Pattern(why));
    }
    Ok(outputs.remove(0).1)
}

/// Draws page 1 of these bytes as a PPM, or `None` where nothing was drawn.
fn draw(name: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Render(RenderPlan {
            source: 0,
            pages: "1".parse::<Selection>().expect("a selection"),
            size: Sizing::Dpi(DPI),
            format: ImageFormat::Ppm,
            page_box: None,
            annotations: true,
            names: "page.ppm".parse().expect("a pattern"),
            strips: None,
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

/// Page 1's `/Contents` as the file holds it: the encoded bytes of every stream in it, joined.
///
/// Read off the document rather than through `Page::content`, because the claim being tested is
/// that the *encoded* bytes crossed untouched — a comparison of decoded content would pass on a
/// piece whose streams had been re-encoded, which RFC 0002 section 11.1 does not permit.
fn first_page_contents(document: &Document) -> Option<Vec<u8>> {
    let pages = pdf_model::Pages::new(document);
    let page = pages.get(0)?;
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

/// One document through the walk.
fn examine(path: &Path, tally: &Mutex<Tally>) {
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
    if pdf_model::Pages::new(&document).is_empty() {
        record(tally, |t| {
            t.pageless
                .push((name, "the document has no page".to_owned()));
        });
        return;
    }

    let piece = match split_first_page(&name, &bytes) {
        Ok(piece) => piece,
        Err(Refusal::Selection { error, .. }) => {
            record(tally, |t| t.pageless.push((name, error.to_string())));
            return;
        }
        Err(other) => {
            record(tally, |t| t.split_refused.push((name, other.to_string())));
            return;
        }
    };

    // RFC 0002 section 9's first layer, asserted where it is cheapest to assert: the same plan over
    // the same source twice.
    match split_first_page(&name, &bytes) {
        Ok(again) if again == piece => {}
        Ok(_) => {
            record(tally, |t| {
                t.nondeterministic
                    .push((name.clone(), "two splits, two files".to_owned()));
            });
        }
        Err(error) => {
            record(tally, |t| {
                t.nondeterministic
                    .push((name.clone(), format!("the second split refused: {error}")));
            });
        }
    }

    reread_and_draw(&name, &document, &bytes, &piece, tally);
}

/// RFC 0002 section 9's layers 2 and 3 over one piece.
fn reread_and_draw(
    name: &str,
    document: &Document,
    bytes: &[u8],
    piece: &[u8],
    tally: &Mutex<Tally>,
) {
    let name = name.to_owned();
    // Layer 2: the piece opens, holds exactly the one page, and carries the producer's stream.
    let read = match Document::open_with_limits(piece.to_vec(), Limits::DEFAULT) {
        Ok(read) => read,
        Err(error) => {
            record(tally, |t| {
                t.reread_failed
                    .push((name, format!("does not open: {error}")));
            });
            return;
        }
    };
    let pages = pdf_model::Pages::new(&read);
    if pages.len() != 1 {
        record(tally, |t| {
            t.reread_failed
                .push((name, format!("{} pages, not 1", pages.len())));
        });
        return;
    }
    match (first_page_contents(document), first_page_contents(&read)) {
        (Some(before), Some(after)) if before == after => {}
        (None, None) => {}
        (before, after) => {
            record(tally, |t| {
                t.contents_differ.push((
                    name.clone(),
                    format!(
                        "{} encoded bytes became {}",
                        before.map_or_else(|| "no".to_owned(), |b| b.len().to_string()),
                        after.map_or_else(|| "no".to_owned(), |b| b.len().to_string())
                    ),
                ));
            });
        }
    }
    record(tally, |t| t.split = t.split.saturating_add(1));

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
            t.structure_faults.push((name.clone(), fault.clone()));
        }
    });

    // Layer 3, the load-bearing one.
    match (draw(&name, bytes), draw(&name, piece)) {
        (Some(before), Some(after)) if before == after => {
            record(tally, |t| t.identical = t.identical.saturating_add(1));
        }
        (Some(before), Some(after)) => {
            record(tally, |t| {
                t.differ.push((
                    name,
                    format!(
                        "{} bytes of raster became {}{}",
                        before.len(),
                        after.len(),
                        if before.len() == after.len() {
                            ", same size"
                        } else {
                            ""
                        }
                    ),
                ));
            });
        }
        (before, after) => {
            record(tally, |t| {
                t.undrawn.push((
                    name,
                    format!(
                        "source drew {}, piece drew {}",
                        if before.is_some() { "yes" } else { "no" },
                        if after.is_some() { "yes" } else { "no" }
                    ),
                ));
            });
        }
    }
}

/// Prints one census list, capped, with its length.
fn print_list(what: &str, entries: &[(String, String)]) {
    println!("transform-split:   {what}: {}", entries.len());
    for (name, why) in entries.iter().take(40) {
        println!("    {name}: {why}");
    }
    if entries.len() > 40 {
        println!("    … and {} more", entries.len().saturating_sub(40));
    }
}

/// The walk.
#[test]
#[ignore = "corpus-scale: every document's first page split, re-read and drawn twice; run explicitly under the gates profile"]
fn every_corpus_documents_first_page_survives_being_split_out() {
    require_the_sandbox();
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
        "transform-split: {} documents in {:.1}s, {} threads, {DPI} dpi",
        files.len(),
        elapsed.as_secs_f64(),
        rayon::current_num_threads()
    );
    print_list("refused open", &tally.refused_open);
    print_list("no page to split", &tally.pageless);
    print_list("split refused by name", &tally.split_refused);
    println!(
        "transform-split:   split, re-read as one page: {}",
        tally.split
    );
    println!(
        "transform-split:   drawn bit-identically to the source page: {}",
        tally.identical
    );
    print_list("nothing drawn on one side or both", &tally.undrawn);
    print_list(
        "the content stream did not cross byte for byte",
        &tally.contents_differ,
    );
    print_list("drew differently", &tally.differ);
    println!(
        "transform-split:   §14.7 structure trees carried: {}, elements: {}, parent-tree keys resolving: {}",
        tally.structure_carried, tally.structure_elements, tally.structure_resolved
    );
    print_list("§14.7 structure faults", &tally.structure_faults);
    print_list("two splits, two files", &tally.nondeterministic);
    print_list("panicked", &tally.panicked);

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
        "this tree must read back the one page it wrote"
    );
    assert!(
        tally.contents_differ.is_empty(),
        "§11.1: every content stream in the output is a producer's, carried byte for byte"
    );
    assert!(
        tally.structure_faults.is_empty(),
        "§14.7: a carried structure tree states what its clauses require, or none at all"
    );

    for (name, why) in &tally.differ {
        assert!(
            HELD.iter().any(|(held, _)| held == name),
            "a piece that draws differently and nobody has read: {name}: {why}"
        );
    }
    assert!(
        tally.identical > 0,
        "a corpus with no page to split is not this corpus"
    );
}
