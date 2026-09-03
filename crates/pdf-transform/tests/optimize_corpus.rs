//! Every corpus document the suite can open, rewritten smaller, re-read and drawn twice.
//!
//! RFC 0002 section 9 gives a transform four layers of correctness, and this walk is the second
//! and the third of them over the corpus population — plus the one property gate section 9 asks
//! of this verb alone.
//!
//! 1. **Self read-back.** The rewritten file is opened by this tree's own reader, and it must
//!    hold every page the source held. Its object graph is then judged against §7.5.5 and
//!    §7.5.7 by `support::check_optimized`, which asks the *output* whether it conforms rather
//!    than whether it matches what `optimize` intended (trap 8).
//! 2. **The raster oracle, and it is the load-bearing one.** Page 1 of the rewrite and page 1
//!    of the source are drawn by the same backend at the same scale, and the two rasters must
//!    be **bit-identical**: every pass here is lossless, so RFC 0002 section 9's tolerance for
//!    a lossy optimise never applies and there is nothing to tolerate.
//! 3. **`optimize` is idempotent** — "its own output, optimized again, is byte-identical". This
//!    is the property gate session 888 could not take because the verb did not exist, and it is
//!    the one instrument that can see a pass whose result depends on anything but its input.
//!
//! **The content comparison here is of *decoded* bytes, and that is what makes this walk's
//! question different from `split_corpus.rs`'s.** That walk compares `/Contents` encoded,
//! because `split` promises pass-through. This verb promises `CLAUDE.md`'s other arm — "carried
//! byte for byte **or recompressed without reinterpretation**" — so the encoded bytes are
//! *expected* to change and what must not change is what they decode to.
//!
//! And one thing no layer above can see: **whether pruning removed something that mattered**.
//! A rewrite that dropped half the document's objects would still draw page 1 correctly if none
//! of them was on page 1. So the walk also holds the output's own closure — after pruning,
//! nothing the file holds may be unreachable from §7.5.5's `/Root`, §7.5.7's carriers and
//! §7.5.8's cross-reference stream excepted by their own clauses — and the §14.7 structure
//! tree, which a raster gate is least placed to notice (ADR 0835).
//!
//! # What is a failure and what is held
//!
//! The assertions bind from the first run. **A refusal is not a failure**: a document this verb
//! declines by name is *the document's*, counted by reason and printed (trap 11). What the walk
//! cannot explain goes in [`HELD`] with a diagnosis, and an undiagnosed difference fails the run.
//!
//! Everything is in memory: the corpus is never written to. The reader and the rasteriser that
//! judge the writer are this tree's own, which is trap 8; `tests/optimize.rs` holds
//! `qpdf --check` over committed fixtures as the foreign evidence.
//!
//! # Running it
//!
//! ```text
//! tools/bounded.sh --data 12 --tree 12 -- cargo test --profile gates -p pdf-transform --test optimize_corpus -- --ignored --nocapture
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

use pdf_syntax::serialize::{ObjectStreams, Streams};
use pdf_syntax::{Document, Limits, Object, SyntaxError};
use pdf_transform::optimize::{OptimizePlan, Savings};
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::{Budget, MemorySinks, Origin, Plan, Policy, Refusal, Secret, Source, apply};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

mod support;

use support::{check_optimized, check_structure};

/// The dots per inch both rasters are drawn at, for `split_corpus.rs`'s reason.
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

/// Documents whose rewrite does not draw as the source does, each with its diagnosis.
///
/// Empty is the state to keep. An entry here is a *reading* of why the difference is the
/// document's rather than the verb's, and the walk fails on any difference it does not name.
const HELD: &[(&str, &str)] = &[];

/// What the walk found.
#[derive(Default)]
struct Tally {
    /// Documents the suite could not open, by reason.
    refused_open: Vec<(String, String)>,
    /// Documents `optimize` declined by name, by reason.
    refused: Vec<(String, String)>,
    /// Documents with no page.
    pageless: Vec<(String, String)>,
    /// Documents rewritten and read back with the pages they started with.
    rewritten: usize,
    /// The rewrite did not open, or lost a page.
    reread_failed: Vec<(String, String)>,
    /// The rewrite's decoded page content is not the source's.
    contents_differ: Vec<(String, String)>,
    /// Rewrites whose page 1 drew bit-identically to the source's.
    identical: usize,
    /// Rewrites whose page 1 drew differently.
    differ: Vec<(String, String)>,
    /// Documents whose page neither side would draw, so nothing was compared.
    undrawn: Vec<(String, String)>,
    /// The same document rewritten twice did not write the same bytes.
    nondeterministic: Vec<(String, String)>,
    /// A rewrite of a rewrite that was not the rewrite.
    not_idempotent: Vec<(String, String)>,
    /// Outputs stating something §7.5.5 or §7.5.7 forbids.
    faults: Vec<(String, String)>,
    /// Outputs holding an object no path from a trailer root reaches.
    unreachable: Vec<(String, String)>,
    /// Rewrites whose §14.7 structure states something a clause forbids that its source did
    /// not.
    structure_faults: Vec<(String, String)>,
    /// §14.7 faults the *sources* already state, which a rewrite carries because it copies.
    structure_faults_carried: usize,
    /// Sources that state a §14.7 structure tree.
    structure_before: usize,
    /// Outputs that still state one.
    structure_after: usize,
    /// Source bytes, over the documents that were rewritten.
    bytes_before: u64,
    /// Output bytes, over the same.
    bytes_after: u64,
    /// Bytes the same documents come to with §7.5.7's object streams switched off.
    bytes_without_object_streams: u64,
    /// And with recompression switched off as well.
    bytes_without_recompression: u64,
    /// And with pruning switched off too, which is the serializer copying what it was given.
    bytes_without_pruning: u64,
    /// Objects the sources declare, less §7.5.4's free head.
    objects_before: u64,
    /// Objects the outputs hold.
    objects_after: u64,
    /// Objects the outputs hold inside a §7.5.7 carrier.
    compressed: u64,
    /// Streams the outputs re-encoded because the result was smaller.
    recompressed: u64,
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
///
/// Both sides of this walk's comparison are drawn by the same build, so a missing worker would
/// not make them disagree — it would make them agree on pages with the images missing, which is
/// a weaker gate wearing the same number.
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

/// Rewrites these bytes under `plan`, answering the file and what the report said it saved.
fn rewrite_reported(
    name: &str,
    bytes: &[u8],
    plan: OptimizePlan,
) -> Result<(Vec<u8>, Option<Savings>), Refusal> {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Optimize(plan),
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
            .map_or_else(|| "nothing was written".to_owned(), |d| d.detail.clone());
        return Err(Refusal::Pattern(why));
    }
    let savings = report
        .outputs
        .first()
        .and_then(|output| match output.origin {
            Origin::Optimized { savings, .. } => Some(savings),
            _ => None,
        });
    Ok((outputs.remove(0).1, savings))
}

/// The same, where only the bytes are wanted.
fn rewrite(name: &str, bytes: &[u8], plan: OptimizePlan) -> Result<Vec<u8>, Refusal> {
    rewrite_reported(name, bytes, plan).map(|(bytes, _)| bytes)
}

/// The lossless default.
fn default_plan() -> OptimizePlan {
    OptimizePlan {
        source: 0,
        names: "out.pdf".parse().expect("a pattern"),
        prune: true,
        object_streams: ObjectStreams::DEFAULT,
        streams: Streams::DEFAULT,
    }
}

/// Draws page 1 of these bytes as a PPM, or `None` where nothing was drawn.
fn draw(name: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Render(RenderPlan {
            source: 0,
            pages: "1".parse().expect("a selection"),
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

/// Every page's `/Contents`, decoded and concatenated, in page order.
///
/// §7.8.2 makes an array of content streams one stream — "the division between streams may
/// occur only at the boundaries between lexical tokens" — so a comparison of the concatenation
/// is a comparison of the page's marks whatever shape the producer stored them in. It is the
/// *decoded* bytes because this verb is permitted to change the encoding and nothing else.
fn decoded_contents(document: &Document) -> Vec<Vec<u8>> {
    let pages = pdf_model::Pages::new(document);
    (0..pages.len())
        .map(|index| {
            let mut out = Vec::new();
            let Some(page) = pages.get(index) else {
                return out;
            };
            let parts = match document.get_key(&page.dict, "Contents") {
                Object::Array(items) => items,
                other => vec![other],
            };
            for part in &parts {
                if let Some(stream) = document.resolve(part).as_stream()
                    && let Some(data) = document.decoded_stream_data(stream)
                {
                    out.extend_from_slice(&data);
                }
            }
            out
        })
        .collect()
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
    let pages = pdf_model::Pages::new(&document).len();
    if pages == 0 {
        record(tally, |t| {
            t.pageless
                .push((name, "the document has no page".to_owned()));
        });
        return;
    }

    let once = match rewrite_reported(&name, &bytes, default_plan()) {
        Ok((out, savings)) => {
            if let Some(savings) = savings {
                record(tally, |t| {
                    t.recompressed = t.recompressed.saturating_add(savings.recompressed);
                });
            }
            out
        }
        Err(error) => {
            // The exit status goes in beside the message because RFC 0002 section 4.4 makes it
            // the part a caller acts on — 2 is "the *file* defeated us" and 4 is "*we*
            // declined" — and the census is the only place the corpus says which documents fall
            // on which side. ADR 0852: three of these were 2 while their own message said they
            // were a refusal by name.
            record(tally, |t| {
                t.refused
                    .push((name, format!("exit {}: {error}", error.exit().code())));
            });
            return;
        }
    };

    // RFC 0002 section 9's first layer, and then the property gate this verb owes: the same
    // plan over the same source twice, and then over the *output*, which must change nothing.
    match rewrite(&name, &bytes, default_plan()) {
        Ok(again) if again == once => {}
        Ok(_) => record(tally, |t| {
            t.nondeterministic
                .push((name.clone(), "two rewrites, two files".to_owned()));
        }),
        Err(error) => record(tally, |t| {
            t.nondeterministic
                .push((name.clone(), format!("the second rewrite refused: {error}")));
        }),
    }
    match rewrite(&name, &once, default_plan()) {
        Ok(twice) if twice == once => {}
        Ok(twice) => record(tally, |t| {
            t.not_idempotent.push((
                name.clone(),
                format!(
                    "{} bytes became {} on a second pass",
                    once.len(),
                    twice.len()
                ),
            ));
        }),
        Err(error) => record(tally, |t| {
            t.not_idempotent.push((
                name.clone(),
                format!("the output would not reopen: {error}"),
            ));
        }),
    }

    attribute(&name, &bytes, &once, tally);
    reread_and_draw(&name, &document, pages, &bytes, &once, tally);
}

/// The A/B that says what each pass is worth: the same document with each one switched off.
///
/// Principle 2's rule — an optimisation is justified by a benchmark — over the population the
/// verb is for, rather than over one file somebody chose.
fn attribute(name: &str, bytes: &[u8], once: &[u8], tally: &Mutex<Tally>) {
    let without_object_streams = rewrite(
        name,
        bytes,
        OptimizePlan {
            object_streams: ObjectStreams::Disable,
            ..default_plan()
        },
    );
    let without_recompression = rewrite(
        name,
        bytes,
        OptimizePlan {
            object_streams: ObjectStreams::Disable,
            streams: Streams::Carry,
            ..default_plan()
        },
    );
    let without_pruning = rewrite(
        name,
        bytes,
        OptimizePlan {
            prune: false,
            object_streams: ObjectStreams::Disable,
            streams: Streams::Carry,
            ..default_plan()
        },
    );
    let (Ok(a), Ok(b), Ok(c)) = (
        &without_object_streams,
        &without_recompression,
        &without_pruning,
    ) else {
        return;
    };
    let (a, b, c) = (a.len() as u64, b.len() as u64, c.len() as u64);
    let source = bytes.len() as u64;
    let after = once.len() as u64;
    record(tally, |t| {
        t.bytes_before = t.bytes_before.saturating_add(source);
        t.bytes_after = t.bytes_after.saturating_add(after);
        t.bytes_without_object_streams = t.bytes_without_object_streams.saturating_add(a);
        t.bytes_without_recompression = t.bytes_without_recompression.saturating_add(b);
        t.bytes_without_pruning = t.bytes_without_pruning.saturating_add(c);
    });
}

/// RFC 0002 section 9's layers 2 and 3, plus the two structural questions a raster cannot ask.
fn reread_and_draw(
    name: &str,
    document: &Document,
    pages: usize,
    bytes: &[u8],
    once: &[u8],
    tally: &Mutex<Tally>,
) {
    let name = name.to_owned();
    let read = match Document::open_with_limits(once.to_vec(), Limits::DEFAULT) {
        Ok(read) => read,
        Err(error) => {
            record(tally, |t| {
                t.reread_failed
                    .push((name, format!("does not open: {error}")));
            });
            return;
        }
    };
    let after = pdf_model::Pages::new(&read).len();
    if after != pages {
        record(tally, |t| {
            t.reread_failed
                .push((name, format!("{pages} pages became {after}")));
        });
        return;
    }
    record(tally, |t| t.rewritten = t.rewritten.saturating_add(1));

    if decoded_contents(&read) != decoded_contents(document) {
        record(tally, |t| {
            t.contents_differ.push((
                name.clone(),
                "a page's decoded content is not the producer's".to_owned(),
            ));
        });
    }

    check_structure_of(&name, document, &read, tally);

    match (draw(&name, bytes), draw(&name, once)) {
        (Some(before), Some(after)) if before == after => {
            record(tally, |t| t.identical = t.identical.saturating_add(1));
        }
        (Some(before), Some(after)) => {
            record(tally, |t| {
                t.differ.push((
                    name,
                    format!("{} bytes of raster became {}", before.len(), after.len()),
                ));
            });
        }
        (before, after) => {
            record(tally, |t| {
                t.undrawn.push((
                    name,
                    format!(
                        "source drew {}, rewrite drew {}",
                        if before.is_some() { "yes" } else { "no" },
                        if after.is_some() { "yes" } else { "no" }
                    ),
                ));
            });
        }
    }
}

/// The two structural questions no raster comparison can ask, over one rewrite.
fn check_structure_of(name: &str, document: &Document, read: &Document, tally: &Mutex<Tally>) {
    let name = name.to_owned();
    // §7.5.5 and §7.5.7, asked of the output: nothing unreachable survived pruning, and every
    // carrier states Table 16's entries with the offsets the clause orders.
    let check = check_optimized(read);
    record(tally, |t| {
        t.objects_before = t.objects_before.saturating_add(declared(document));
        t.objects_after = t
            .objects_after
            .saturating_add(u64::try_from(check.declared).unwrap_or(0));
        t.compressed = t
            .compressed
            .saturating_add(u64::try_from(check.compressed).unwrap_or(0));
        for fault in &check.faults {
            t.faults.push((name.clone(), fault.clone()));
        }
        if !check.unreachable.is_empty() {
            t.unreachable.push((
                name.clone(),
                format!(
                    "{} object(s) no path from /Root reaches: {:?}",
                    check.unreachable.len(),
                    check.unreachable.iter().take(8).collect::<Vec<_>>()
                ),
            ));
        }
    });

    // §14.7, which no raster comparison can see (ADR 0835). **The claim is comparative, and it
    // has to be**: a rewrite copies a tree rather than rebuilding one, so a source whose parent
    // tree already breaks §14.7.5.4 produces an output that breaks it in exactly the same way,
    // and that is the verb behaving correctly. What would be a defect is a fault the source did
    // not have. The comparison is of *counts* rather than of the sentences, because a fault
    // names the object it is about and a rewrite renumbers — comparing the strings would call
    // every carried fault a new one, which is what the first run of this walk did over 103 of
    // them.
    let before = check_structure(document);
    let after = check_structure(read);
    record(tally, |t| {
        if before.carried {
            t.structure_before = t.structure_before.saturating_add(1);
        }
        if after.carried {
            t.structure_after = t.structure_after.saturating_add(1);
        }
        t.structure_faults_carried = t
            .structure_faults_carried
            .saturating_add(before.faults.len());
        if before.carried && !after.carried {
            t.structure_faults.push((
                name.clone(),
                "§14.7: the source states a /StructTreeRoot and the rewrite does not".to_owned(),
            ));
        }
        if after.faults.len() > before.faults.len() {
            t.structure_faults.push((
                name.clone(),
                format!(
                    "§14.7: the source states {} fault(s) and the rewrite {}: {:?}",
                    before.faults.len(),
                    after.faults.len(),
                    after.faults.iter().take(3).collect::<Vec<_>>()
                ),
            ));
        }
        if after.elements != before.elements {
            t.structure_faults.push((
                name.clone(),
                format!(
                    "§14.7: {} structure elements became {}",
                    before.elements, after.elements
                ),
            ));
        }
    });
}

/// How many objects a document's trailer declares, less §7.5.4's free head.
fn declared(document: &Document) -> u64 {
    document
        .trailer()
        .get("Size")
        .and_then(Object::as_integer)
        .and_then(|size| u64::try_from(size).ok())
        .map_or(0, |size| size.saturating_sub(1))
}

/// Prints one census list, capped, with its length.
fn print_list(what: &str, entries: &[(String, String)]) {
    println!("transform-optimize:   {what}: {}", entries.len());
    for (name, why) in entries.iter().take(40) {
        println!("    {name}: {why}");
    }
    if entries.len() > 40 {
        println!("    … and {} more", entries.len().saturating_sub(40));
    }
}

/// A saving as a percentage of the source, or `0.0` where nothing was measured.
#[expect(
    clippy::cast_precision_loss,
    reason = "a percentage printed to two decimals over corpus-scale byte counts, where the \
              52-bit mantissa is exact for every total this walk can reach"
)]
fn percent(before: u64, after: u64) -> f64 {
    if before == 0 {
        return 0.0;
    }
    100.0 * (before as f64 - after as f64) / before as f64
}

/// Everything the walk counted, printed. The counts are a function of the corpus and of what
/// this walk asks, so they are printed and not ratcheted (`doc/todo/05`, ADR 0835 section 2).
fn census(tally: &Tally, files: usize, elapsed: std::time::Duration) {
    println!(
        "transform-optimize: {files} documents in {:.1}s, {} threads, {DPI} dpi",
        elapsed.as_secs_f64(),
        rayon::current_num_threads()
    );
    print_list("refused open", &tally.refused_open);
    print_list("no page", &tally.pageless);
    print_list("refused by name", &tally.refused);
    println!(
        "transform-optimize:   rewritten and read back with their pages: {}",
        tally.rewritten
    );
    println!(
        "transform-optimize:   drawn bit-identically to the source page: {}",
        tally.identical
    );
    print_list("did not read back with its pages", &tally.reread_failed);
    print_list("nothing drawn on one side or both", &tally.undrawn);
    println!(
        "transform-optimize:   bytes {} → {} ({:.2}% saved)",
        tally.bytes_before,
        tally.bytes_after,
        percent(tally.bytes_before, tally.bytes_after)
    );
    println!(
        "transform-optimize:     the serializer copying what it was given: {} ({:.2}%)",
        tally.bytes_without_pruning,
        percent(tally.bytes_before, tally.bytes_without_pruning)
    );
    println!(
        "transform-optimize:     + §7.5.5 reachability pruning: {} ({:.2}%)",
        tally.bytes_without_recompression,
        percent(tally.bytes_before, tally.bytes_without_recompression)
    );
    println!(
        "transform-optimize:     + §7.4 recompression: {} ({:.2}%)",
        tally.bytes_without_object_streams,
        percent(tally.bytes_before, tally.bytes_without_object_streams)
    );
    println!(
        "transform-optimize:     + §7.5.7 object streams: {} ({:.2}%)",
        tally.bytes_after,
        percent(tally.bytes_before, tally.bytes_after)
    );
    println!(
        "transform-optimize:   objects {} → {}, of which {} compressed; streams re-encoded: {}",
        tally.objects_before, tally.objects_after, tally.compressed, tally.recompressed
    );
    println!(
        "transform-optimize:   §14.7 structure trees: {} in the sources, {} in the rewrites; \
         {} fault(s) the sources already state and the rewrites carry",
        tally.structure_before, tally.structure_after, tally.structure_faults_carried
    );
    print_list("decoded content differs", &tally.contents_differ);
    print_list("§7.5.5 or §7.5.7 faults", &tally.faults);
    print_list("unreachable objects survived pruning", &tally.unreachable);
    print_list("§14.7 faults a rewrite added", &tally.structure_faults);
    print_list("drew differently", &tally.differ);
    print_list("two rewrites, two files", &tally.nondeterministic);
    print_list("not idempotent", &tally.not_idempotent);
    print_list("panicked", &tally.panicked);
}

/// The walk.
#[test]
#[ignore = "corpus-scale: every document rewritten four ways, re-read, drawn twice and rewritten again; run explicitly under the gates profile"]
fn every_corpus_document_is_rewritten_smaller_and_says_the_same_thing() {
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

    census(&tally, files.len(), elapsed);

    assert!(
        tally.panicked.is_empty(),
        "principle 1: no panic on any input"
    );
    assert!(
        tally.nondeterministic.is_empty(),
        "RFC 0002 section 9: same sources, same plan, same bytes"
    );
    assert!(
        tally.not_idempotent.is_empty(),
        "RFC 0002 section 9: optimize is idempotent — its own output, optimized again, is \
         byte-identical"
    );
    assert!(
        tally.reread_failed.is_empty(),
        "this tree must read back every page it wrote"
    );
    assert!(
        tally.contents_differ.is_empty(),
        "CLAUDE.md: recompressed without reinterpretation — the decoded bytes are the \
         producer's"
    );
    assert!(
        tally.faults.is_empty(),
        "§7.5.5 and §7.5.7: a rewritten file states what its clauses require"
    );
    assert!(
        tally.unreachable.is_empty(),
        "§7.5.5: after pruning, every object the file holds is one some path from /Root reaches"
    );
    assert!(
        tally.structure_faults.is_empty(),
        "§14.7: a tree the source stated is a tree the rewrite states, element for element, \
         with no fault the source did not have"
    );

    for (name, why) in &tally.differ {
        assert!(
            HELD.iter().any(|(held, _)| held == name),
            "a rewrite that draws differently and nobody has read: {name}: {why}"
        );
    }
    assert!(
        tally.bytes_after < tally.bytes_before,
        "a verb called optimize made the corpus larger"
    );
    assert!(
        tally.identical > 0,
        "a corpus with no page to draw is not this corpus"
    );
}
