//! RFC 0002 section 9's fourth layer: what a **foreign** reader makes of what this suite writes.
//!
//! Every other walk in this crate checks the suite's output with this tree's own parser and this
//! tree's own rasteriser. That answers "did we write what we meant to" and cannot answer "did we
//! write what the format says", because a misreading on the way out and the matching misreading
//! on the way back in agree with each other. Four writers exist — `attachments --attach`,
//! `split`, `merge` and `pages` — and until this walk none of their output had been shown to
//! anybody else.
//!
//! # The comparison is foreign-to-foreign, and that is the whole design
//!
//! poppler draws the source page, poppler draws the derived page, and the two are compared. If
//! they differ, the difference is **ours**: one reader, one page, two files that this suite says
//! carry the same content stream, the same resources and the same boxes. The oracle's question —
//! does our rasteriser agree with poppler's — is a different question with a different
//! instrument, and mixing the two would make a disagreement unattributable (trap 3, trap 9).
//! Nothing here compares a foreign reader against this tree's renderer.
//!
//! # What each reader is asked
//!
//! - **`qpdf --check`** — structural soundness. Its verdict on the derived file is compared with
//!   its verdict on the source: a file it accepted becoming one it reports errors in is ours,
//!   and a source it already refused says nothing about the writer.
//! - **`pdftoppm`** (poppler) and **`mutool draw`** (mupdf) — the page. Bit-identical is the
//!   assertion, for the reason `split_corpus.rs` gives: the same marks in the same space shall
//!   make the same pixels, and the comparison is one renderer against itself.
//! - **`mutool show`** — §14.7's carried structure tree, which is the part of a derived document
//!   **nothing rendered can see**. An assistive processor is its only reader, so a tree that
//!   only this program can make sense of is exactly what a raster gate is least placed to
//!   notice (ADR 0835 section 5). mupdf resolves the parent-tree entry for the page's own
//!   §14.7.5.4 key in both files, and the two shapes must agree: as many members, and an
//!   indirect reference in each position the source had one. That comparison found ADR 0838's
//!   defect on its first run.
//!
//! `pdfinfo`'s `Tagged:` line is poppler reading §14.7.1's `/MarkInfo /Marked` and is asked for
//! too, because it is the one thing a *second* foreign reader will say about tagging at all.
//!
//! # What is a failure and what is held
//!
//! **A refusal is not a failure** (trap 11): a document the suite declines by name, or one no
//! foreign reader would read in the first place, is the document's and is counted by reason. A
//! page that draws differently and nobody has read fails the run; a diagnosed one goes in
//! [`HELD`] with its reading, in the oracle's style.
//!
//! **And a timeout is not a failure either**, which this walk learned on its second run:
//! `issue19517.pdf` costs poppler 24 s and mupdf 17.6 s on the *source* and on the derived file
//! alike, so a budget of 20 s decides that document by which side of the bound the machine
//! landed on. A reader that outruns [`BUDGET`] has said nothing about the file, so the document
//! leaves that reader's comparison and is counted — the same treatment the *source* render's
//! timeout already got. `doc/todo/02` §2's rule about a gate that spawns another program is the
//! same rule.
//!
//! # The population
//!
//! Every corpus document that states §14.7.2's `/StructTreeRoot`, because the structure lane
//! needs a tagged population and there are few; plus every [`STRIDE`]th document, because the
//! rendering lane spawns fourteen processes per document and the whole corpus would be a walk
//! nobody runs. The stride is over a sorted list, so the sample is the same on every machine.
//!
//! # Running it
//!
//! ```text
//! tools/bounded.sh --data 8 --tree 12 -- cargo test --profile gates -p pdf-transform --test foreign_corpus -- --ignored --nocapture
//! ```

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    reason = "test code: an explanatory panic is the intended failure, and the census output \
              is the point of the run"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use pdf_render::Raster;
use pdf_syntax::{Document, Limits, SyntaxError};
use pdf_transform::attachments::{Action, AttachmentsPlan, Payload};
use pdf_transform::merge::{Input, MergePlan};
use pdf_transform::pages::{Edit, PagesPlan};
use pdf_transform::range::Selection;
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Refusal, Source, apply};
use pdfref::Reference;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

mod support;

/// The dots per inch both foreign rasters are drawn at.
///
/// Low for `split_corpus.rs`'s reason — the question is whether two rasters are the same rather
/// than how they look — and lower still matters here, because every page is drawn four times by
/// two other programs.
const DPI: u32 = 48;

/// One in this many corpus documents joins the rendering lane.
///
/// A bound on the wall clock rather than a statement about the corpus: a document costs up to
/// fourteen foreign process invocations, and the walk is meant to be run on every round that
/// touches a writer.
const STRIDE: usize = 8;

/// How long any one foreign program is given before it is killed.
///
/// It is reading corpus documents, some of which are fuzzed, so a reader that never returns is
/// an expected outcome and not a reason to hang the gate.
const BUDGET: Duration = Duration::from_secs(20);

/// The bytes `attachments --attach` files, as `writer_corpus.rs` does.
const PAYLOAD: &[u8] = b"pdf-transform foreign readback witness 898\n";

/// The name they are filed under.
const NAME: &str = "pdf-transform-witness-898.txt";

/// The verbs, in the order the census prints them.
const VERBS: [&str; 4] = ["attach", "split", "merge", "pages"];

/// The foreign renderers, and what each is called in the census.
const READERS: [Reference; 2] = [Reference::Poppler, Reference::MuPdf];

/// Documents whose derived page a foreign reader draws differently from the source page, each
/// with the reading of why the difference is not a defect — the document's own, or a decision
/// this suite has already recorded.
///
/// An undiagnosed difference fails the run. All three entries are one reading: ADR 0821 section 2 drops
/// §12.8.1's `/V` from a signature field that crosses into a merged document, because a
/// signature is over bytes the new file does not have, and mupdf draws an *unsigned* signature
/// widget as a placeholder rather than from its `/AP /N`. poppler draws all three identically,
/// which is what says the difference is in one reader's treatment of an unsigned field and not
/// in the marks the page carries.
const HELD: &[(&str, &str)] = &[
    (
        "bug854315.pdf",
        "§12.8.1: /Sig field Signature1 crosses without its /V (ADR 0821 section 2) and mupdf draws the \
         unsigned widget as a grey placeholder over its /Rect; poppler draws it identically",
    ),
    (
        "issue16553.pdf",
        "§12.8.1: the same — /SigFlags 3, one /FT /Sig field, unsigned after the merge",
    ),
    (
        "issue17069.pdf",
        "§12.8.1: the same — two /FT /Sig fields, unsigned after the merge",
    ),
];

/// Documents whose §14.7 parent-tree shape a foreign reader resolves differently, each with the
/// reading of why the difference is the document's.
///
/// Both are the same non-conformance and it is the source's: §14.7.2 makes the structure
/// hierarchy what `/StructTreeRoot`'s `/K` reaches, and Table 355 makes `/P` "( Required; shall
/// be an indirect reference )". An element that the source's *parent tree* names while its own
/// hierarchy reaches nothing of it is in the file and not in the tree, so this suite — which
/// carries the hierarchy — writes §7.3.9's null in that array position and says so out loud
/// (ADR 0839). Carrying it instead would mean writing an element with no `/P`, or inventing one.
const STRUCTURE_HELD: &[(&str, &str)] = &[
    (
        "bug1365930.pdf",
        "§14.7.2: /StructTreeRoot /K names one /Document element with no /K of its own, and the \
         whole /Article → /Story → paragraph subtree the parent tree names hangs off object 20, \
         which states no /P at all",
    ),
    (
        "paragraph_and_link.pdf",
        "§14.7.2: the array's index 4 names object 22, an /Artifact element with no /P that no \
         element's /K names either; the hierarchy under /StructTreeRoot /K does not reach it",
    ),
];

/// What one verb's output survived, per foreign reader.
#[derive(Default)]
struct Lane {
    /// Derived files this suite wrote.
    written: usize,
    /// Documents the suite declined by name, by reason.
    refused: Vec<(String, String)>,
    /// `qpdf --check` gave the derived file no worse a verdict than the source.
    qpdf_held: usize,
    /// It accepted the source and reported errors in what we wrote.
    qpdf_lost: Vec<(String, String)>,
    /// Per reader: the derived page and the source page drew bit-identically.
    identical: [usize; READERS.len()],
    /// A reader drew both and they differ.
    differ: Vec<(String, String)>,
    /// A reader drew the source page and could not draw ours.
    unreadable: Vec<(String, String)>,
    /// A reader outran [`BUDGET`] on ours, so the document leaves that reader's comparison.
    timed_out: Vec<(String, String)>,
    /// mupdf resolved the page's parent-tree entry in both files to the same shape.
    structure_agreed: usize,
    /// It did not, or poppler stopped calling the file tagged.
    structure_faults: Vec<(String, String)>,
}

/// What the walk found.
#[derive(Default)]
struct Tally {
    /// Documents in the sample.
    considered: usize,
    /// Documents that state a structure tree, so the structure lane has something to ask.
    tagged: usize,
    /// Documents this tree could not open, by reason.
    refused_open: Vec<(String, String)>,
    /// Documents no foreign reader would draw, so nothing they say about ours is ours.
    foreign_refused_source: Vec<(String, String)>,
    /// One entry per verb.
    lanes: BTreeMap<&'static str, Lane>,
    /// A document whose examination panicked, which principle 1 forbids.
    panicked: Vec<(String, String)>,
}

/// Adds to the shared tally, ignoring a poisoned lock.
fn record(tally: &Mutex<Tally>, update: impl FnOnce(&mut Tally)) {
    if let Ok(mut tally) = tally.lock() {
        update(&mut tally);
    }
}

/// Adds to one verb's lane.
fn lane(tally: &Mutex<Tally>, verb: &'static str, update: impl FnOnce(&mut Lane)) {
    record(tally, |t| update(t.lanes.entry(verb).or_default()));
}

/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// Both sides of every comparison here are drawn by *another* program, so the worker does not
/// decide what poppler and mupdf see. It decides what this tree can open at all — a document
/// whose first page is a `JBIG2Decode` image is one the suite refuses to derive from without it,
/// and the sample would quietly shrink. See `split_corpus.rs` for the same guard's other reason.
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so every document whose first page \
             carries one of the three sandboxed codecs would leave the sample: {error}"
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

/// The budget every derived file is written under.
fn budget() -> Budget {
    Budget {
        limits: Limits::DEFAULT,
        max_pixels: 1 << 24,
    }
}

/// The fixed second document `merge` puts beside each source: page 1 of a committed one.
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

/// The first output of a plan, or a refusal.
fn first_output(plan: &Plan, sources: &[Source], sinks: MemorySinks) -> Result<Vec<u8>, Refusal> {
    apply(plan, sources, &sinks, &Policy::default(), &budget())?;
    let mut outputs = sinks.into_outputs();
    if outputs.is_empty() {
        return Err(Refusal::Assembly("no file was written".to_owned()));
    }
    Ok(outputs.remove(0).1)
}

/// The four derived files, each of which states the source's page 1 as **its own** page 1.
///
/// That is what makes one comparison serve all four: `attach` writes §7.5.6's update over the
/// whole document, `split` takes page 1 out, `merge` puts page 1 first, and `pages` takes the
/// *last* page out. A one-page document has no page for `pages` to remove and leaves that lane.
fn derive(
    bytes: &[u8],
    second: &[u8],
    pages: usize,
) -> Vec<(&'static str, Result<Vec<u8>, Refusal>)> {
    let attach = first_output(
        &Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: Action::Attach {
                payload: Payload::new(PAYLOAD.to_vec()),
                name: NAME.to_owned(),
                description: None,
                date: None,
                names: "out.pdf".parse().expect("a pattern"),
                on_page: None,
            },
        }),
        &[Source::new(bytes.to_vec())],
        MemorySinks::new(),
    );
    let split = first_output(
        &Plan::Split(SplitPlan {
            source: 0,
            pages: "1".parse::<Selection>().expect("a selection"),
            pieces: Pieces::EachPage,
            names: "piece-%d.pdf".parse().expect("a pattern"),
        }),
        &[Source::new(bytes.to_vec())],
        MemorySinks::new(),
    );
    let merged = first_output(
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
        &[Source::new(bytes.to_vec()), Source::new(second.to_vec())],
        MemorySinks::new(),
    );
    let mut out = vec![("attach", attach), ("split", split), ("merge", merged)];
    if pages >= 2 {
        out.push((
            "pages",
            first_output(
                &Plan::Pages(PagesPlan {
                    source: 0,
                    edits: vec![Edit::Delete(
                        "r1".parse::<Selection>().expect("a selection"),
                    )],
                    names: "edited.pdf".parse().expect("a pattern"),
                }),
                &[Source::new(bytes.to_vec())],
                MemorySinks::new(),
            ),
        ));
    }
    out
}

/// Runs `command`, giving it at most [`BUDGET`], and answers its exit status.
///
/// Polled rather than waited on, because the standard library has no wait-with-deadline, and
/// `pdfref` enforces its renderers' budget the same way for the same reason. Neither stream goes
/// to a pipe: a pipe nobody drains while polling deadlocks a chatty program against its own
/// buffer, so a caller that wants the output redirects it to a file itself.
fn run_within(command: &mut Command) -> Option<ExitStatus> {
    /// How often the child is checked.
    const POLL: Duration = Duration::from_millis(20);

    let mut child = command.spawn().ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {}
            Err(_) => return None,
        }
        if started.elapsed() > BUDGET {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(POLL);
    }
}

/// `qpdf --check`'s exit code for this file, or `None` where qpdf could not be run.
///
/// qpdf answers 0 for a sound file, 3 where it has warnings, and 2 where it found errors. The
/// walk compares the code on the derived file with the code on the source rather than demanding
/// any particular one, because a corpus document qpdf already complains about says nothing about
/// what this suite wrote.
fn qpdf_code(path: &Path) -> Option<i32> {
    run_within(
        Command::new("qpdf")
            .arg("--check")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
    )?
    .code()
}

/// `pdfinfo`'s `Tagged:` line for this file: `Some(true)` where poppler calls it tagged.
fn poppler_says_tagged(path: &Path, work: &Path) -> Option<bool> {
    let out = work.join("pdfinfo.txt");
    let file = std::fs::File::create(&out).ok()?;
    let status = run_within(
        Command::new("pdfinfo")
            .arg(path)
            .stdout(Stdio::from(file))
            .stderr(Stdio::null()),
    )?;
    if !status.success() {
        return None;
    }
    let text = std::fs::read_to_string(&out).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("Tagged:"))
        .map(|value| value.trim() == "yes")
}

/// `mutool show <file> <path>`, answering what mupdf printed.
fn show(path: &Path, object_path: &str, work: &Path) -> Option<String> {
    let out = work.join("show.txt");
    let status = run_within(
        Command::new("mutool")
            .arg("show")
            .arg("-o")
            .arg(&out)
            .arg(path)
            .arg(object_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
    )?;
    if !status.success() {
        return None;
    }
    std::fs::read_to_string(&out).ok()
}

/// One top-level member of an array `mutool show` printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    /// An integer, which is what §14.7.5.4's number-tree keys are.
    Integer(i64),
    /// `N G R`.
    Reference,
    /// §7.3.9's null, which is what a producer writes for a marked-content identifier no
    /// structure element claims.
    Null,
    /// A nested array, a dictionary, or anything else.
    Other,
}

/// The top-level members of the outermost array in what `mutool show` printed, or `None` where
/// there is no such array.
///
/// mupdf prints an indirect object as `N G obj` … `endobj` around its value and separates every
/// token with whitespace, which is what makes this scan possible at all. It bails on a string:
/// a parent tree holds references, arrays and nulls, and a `(` there would mean the text is not
/// what this function is about.
fn top_level_items(text: &str) -> Option<Vec<Item>> {
    let body = text
        .trim()
        .strip_suffix("endobj")
        .map_or(text.trim(), str::trim);
    let body = match body.find("obj") {
        // `N G obj` only when the three tokens before `obj` are the label mupdf prints.
        Some(at)
            if body[..at]
                .split_whitespace()
                .all(|token| token.parse::<u32>().is_ok())
                && body[..at].split_whitespace().count() == 2 =>
        {
            body[at.saturating_add(3)..].trim()
        }
        _ => body,
    };
    let inner = body.strip_prefix('[')?.strip_suffix(']')?;
    if inner.contains('(') {
        return None;
    }
    let tokens: Vec<&str> = inner.split_whitespace().collect();
    let mut items = Vec::new();
    let mut at = 0;
    while let Some(token) = tokens.get(at) {
        match *token {
            "[" | "<<" => {
                let (open, close) = if *token == "[" {
                    ("[", "]")
                } else {
                    ("<<", ">>")
                };
                let mut depth = 1_usize;
                at = at.saturating_add(1);
                while depth > 0 {
                    match tokens.get(at) {
                        Some(next) if *next == open => depth = depth.saturating_add(1),
                        Some(next) if *next == close => depth = depth.saturating_sub(1),
                        Some(_) => {}
                        None => return None,
                    }
                    at = at.saturating_add(1);
                }
                items.push(Item::Other);
            }
            "null" => {
                items.push(Item::Null);
                at = at.saturating_add(1);
            }
            other => {
                if let Ok(value) = other.parse::<i64>() {
                    let is_reference = tokens
                        .get(at.saturating_add(1))
                        .is_some_and(|next| next.parse::<u32>().is_ok())
                        && tokens.get(at.saturating_add(2)) == Some(&"R");
                    if is_reference {
                        items.push(Item::Reference);
                        at = at.saturating_add(3);
                    } else {
                        items.push(Item::Integer(value));
                        at = at.saturating_add(1);
                    }
                } else {
                    items.push(Item::Other);
                    at = at.saturating_add(1);
                }
            }
        }
    }
    Some(items)
}

/// The `mutool show` array index — **one-based**, which is mupdf's — of the value stored under
/// `key` in a `/Nums` array, whose members alternate key and value (§7.9.7).
fn nums_value_index(text: &str, key: i64) -> Option<usize> {
    let items = top_level_items(text)?;
    items
        .chunks_exact(2)
        .position(|pair| pair.first() == Some(&Item::Integer(key)))
        .map(|pair| pair.saturating_mul(2).saturating_add(2))
}

/// A parent-tree value's shape, one character per member: `r` for an indirect reference, `-` for
/// null, `i` for an integer, `?` for anything else.
///
/// The shape rather than the members, because the members are different objects in two different
/// files by construction. What §14.7.5.4 makes comparable is the *index*: an `/MCID` is "a
/// zero-based index into the array", the content stream crosses byte for byte, so position `n`
/// names an element on both sides or on neither.
fn array_shape(text: &str) -> Option<String> {
    Some(
        top_level_items(text)?
            .into_iter()
            .map(|item| match item {
                Item::Reference => 'r',
                Item::Null => '-',
                Item::Integer(_) => 'i',
                Item::Other => '?',
            })
            .collect(),
    )
}

/// What mupdf makes of the parent-tree entry for the page whose §14.7.5.4 key is `key`.
///
/// `None` where mupdf will not answer — no `/Nums` at the root of the number tree (a large tree
/// states `/Kids` instead), no entry for the key, or a value that is not an array.
fn parent_tree_shape(path: &Path, key: i64, work: &Path) -> Option<String> {
    let nums = show(path, "trailer/Root/StructTreeRoot/ParentTree/Nums", work)?;
    let index = nums_value_index(&nums, key)?;
    let value = show(
        path,
        &format!("trailer/Root/StructTreeRoot/ParentTree/Nums/{index}"),
        work,
    )?;
    array_shape(&value)
}

/// Page 1's §14.7.5.4 key, where it states one.
fn first_page_key(document: &Document) -> Option<i64> {
    support::page_dictionaries(document)
        .first()
        .and_then(|page| document.get_key(page, "StructParents").as_integer())
}

/// One document through the four writers and the three foreign readers.
#[expect(
    clippy::too_many_lines,
    reason = "one document's examination, in the order the readers are asked; splitting it would \
              scatter the tally writes that make the census legible"
)]
fn examine(path: &Path, second: &[u8], base: &Path, tally: &Mutex<Tally>) {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let document = match Document::open_with_limits(bytes.clone(), Limits::DEFAULT) {
        Ok(document) => document,
        // A document needing a password is left out rather than counted as a refusal: the
        // foreign readers would have to be given it too, and this walk is not about encryption.
        Err(SyntaxError::PasswordRequired) => return,
        Err(error) => {
            record(tally, |t| t.refused_open.push((name, error.to_string())));
            return;
        }
    };
    let count = pdf_model::Pages::new(&document).len();
    let source_key = first_page_key(&document);
    let tagged = document.catalog().is_ok_and(|catalog| {
        document
            .get_key(&catalog, "StructTreeRoot")
            .as_dict()
            .is_some()
    });
    record(tally, |t| {
        t.considered = t.considered.saturating_add(1);
        if tagged {
            t.tagged = t.tagged.saturating_add(1);
        }
    });

    let work = base.join(name.replace(['/', '.'], "_"));
    if std::fs::create_dir_all(&work).is_err() {
        return;
    }
    let source_path = work.join("source.pdf");
    if std::fs::write(&source_path, &bytes).is_err() {
        let _ = std::fs::remove_dir_all(&work);
        return;
    }

    // What each reader makes of the *source*. A reader that will not draw this page cannot say
    // anything about ours, and a source qpdf already complains about is not evidence either.
    let source_rasters: Vec<Option<Raster>> = READERS
        .iter()
        .map(|reference| {
            reference
                .render_within(&source_path, 1, DPI, &work, BUDGET)
                .ok()
        })
        .collect();
    let source_qpdf = qpdf_code(&source_path);
    let source_tagged = poppler_says_tagged(&source_path, &work);
    let source_shape = source_key.and_then(|key| parent_tree_shape(&source_path, key, &work));
    if source_rasters.iter().all(Option::is_none) {
        record(tally, |t| {
            t.foreign_refused_source.push((
                name.clone(),
                "neither poppler nor mupdf drew page 1".to_owned(),
            ));
        });
    }

    for (verb, derived) in derive(&bytes, second, count) {
        let derived = match derived {
            Ok(derived) => derived,
            Err(refusal) => {
                lane(tally, verb, |l| {
                    l.refused.push((name.clone(), refusal.to_string()));
                });
                continue;
            }
        };
        let derived_path = work.join(format!("{verb}.pdf"));
        if std::fs::write(&derived_path, &derived).is_err() {
            continue;
        }
        lane(tally, verb, |l| l.written = l.written.saturating_add(1));

        // Structural soundness, as a change of verdict rather than as a verdict.
        if let (Some(source_code), Some(code)) = (source_qpdf, qpdf_code(&derived_path)) {
            if code == 2 && source_code != 2 {
                lane(tally, verb, |l| {
                    l.qpdf_lost.push((
                        name.clone(),
                        format!(
                            "qpdf --check answered {source_code} for the source and {code} here"
                        ),
                    ));
                });
            } else {
                lane(tally, verb, |l| l.qpdf_held = l.qpdf_held.saturating_add(1));
            }
        }

        // The page, one reader at a time, against that same reader's source page.
        for (index, reference) in READERS.iter().enumerate() {
            let Some(before) = source_rasters.get(index).and_then(Option::as_ref) else {
                continue;
            };
            let started = Instant::now();
            match reference.render_within(&derived_path, 1, DPI, &work, BUDGET) {
                Ok(after) if after == *before => {
                    lane(tally, verb, |l| {
                        if let Some(slot) = l.identical.get_mut(index) {
                            *slot = slot.saturating_add(1);
                        }
                    });
                }
                Ok(after) => {
                    let detail = match raster_compare::compare(before, &after) {
                        Ok(comparison) => format!(
                            "{} drew it differently: mean {:.3}, worst tile {:.3} of 255",
                            reference.name(),
                            comparison.mean_error,
                            comparison.worst_tile_error
                        ),
                        Err(error) => format!(
                            "{} drew {}x{} where the source page is {}x{} ({error})",
                            reference.name(),
                            after.width,
                            after.height,
                            before.width,
                            before.height
                        ),
                    };
                    lane(tally, verb, |l| l.differ.push((name.clone(), detail)));
                }
                // A reader that outran the budget has said nothing about the file. The budget is
                // a bound on this walk's wall clock, not a claim about a document — and
                // `issue19517.pdf` costs poppler 24 s and mupdf 17.6 s on the *source* and on
                // ours alike, so asserting on it would be asserting on which side of 20 s the
                // machine happened to land (`doc/todo/02` §2's rule about a gate that spawns
                // another program). Counted and printed; the document leaves this reader's
                // comparison, exactly as it does when the *source* render times out.
                Err(error) if started.elapsed() >= BUDGET => {
                    let detail = format!(
                        "{} outran the {}s budget on ours: {error}",
                        reference.name(),
                        BUDGET.as_secs()
                    );
                    lane(tally, verb, |l| l.timed_out.push((name.clone(), detail)));
                }
                Err(error) => {
                    let detail =
                        format!("{} drew the source and not ours: {error}", reference.name());
                    lane(tally, verb, |l| l.unreadable.push((name.clone(), detail)));
                }
            }
        }

        // §14.7, which nothing above can see.
        if source_tagged == Some(true) && poppler_says_tagged(&derived_path, &work) != Some(true) {
            lane(tally, verb, |l| {
                l.structure_faults.push((
                    name.clone(),
                    "pdfinfo calls the source tagged and not this file (§14.7.1's /MarkInfo)"
                        .to_owned(),
                ));
            });
        }
        if let Some(before) = source_shape.as_ref() {
            let read = Document::open_with_limits(derived.clone(), Limits::DEFAULT).ok();
            let key = read.as_ref().and_then(first_page_key);
            match key.and_then(|key| parent_tree_shape(&derived_path, key, &work)) {
                Some(after) if after == *before => {
                    lane(tally, verb, |l| {
                        l.structure_agreed = l.structure_agreed.saturating_add(1);
                    });
                }
                Some(after) => {
                    lane(tally, verb, |l| {
                        l.structure_faults.push((
                            name.clone(),
                            format!(
                                "§14.7.5.4: mupdf resolves the source page's parent-tree entry to \
                                 \"{before}\" and ours to \"{after}\""
                            ),
                        ));
                    });
                }
                None => {
                    lane(tally, verb, |l| {
                        l.structure_faults.push((
                            name.clone(),
                            format!(
                                "§14.7.5.4: mupdf resolves the source page's parent-tree entry to \
                                 \"{before}\" and finds none for ours"
                            ),
                        ));
                    });
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&work);
}

/// Prints one census list, capped, with its length.
fn print_list(what: &str, entries: &[(String, String)]) {
    println!("transform-foreign:   {what}: {}", entries.len());
    for (name, why) in entries.iter().take(40) {
        println!("    {name}: {why}");
    }
    if entries.len() > 40 {
        println!("    … and {} more", entries.len().saturating_sub(40));
    }
}

/// The readers this walk needs, and which of them are missing.
fn missing_readers() -> Vec<&'static str> {
    let mut missing = Vec::new();
    for reference in READERS {
        if !reference.is_available() {
            missing.push(reference.program());
        }
    }
    for program in ["qpdf", "pdfinfo"] {
        if Command::new(program)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_err()
        {
            missing.push(program);
        }
    }
    missing
}

/// Everything the run found, in the order it was asked.
fn census(tally: &Tally, documents: usize, elapsed: Duration) {
    println!(
        "transform-foreign: {} of {documents} documents in {:.1}s, {} threads, {DPI} dpi, stride \
         {STRIDE}",
        tally.considered,
        elapsed.as_secs_f64(),
        rayon::current_num_threads()
    );
    println!(
        "transform-foreign:   stating a structure tree: {}",
        tally.tagged
    );
    print_list("this tree could not open", &tally.refused_open);
    print_list(
        "no foreign reader drew the source",
        &tally.foreign_refused_source,
    );
    for verb in VERBS {
        let Some(lane) = tally.lanes.get(verb) else {
            continue;
        };
        println!(
            "transform-foreign:   {verb}: {} written, qpdf held {}, poppler identical {}, mupdf \
             identical {}, §14.7 shapes agreed {}",
            lane.written,
            lane.qpdf_held,
            lane.identical.first().copied().unwrap_or_default(),
            lane.identical.get(1).copied().unwrap_or_default(),
            lane.structure_agreed
        );
        print_list(&format!("{verb}: refused by name"), &lane.refused);
        print_list(&format!("{verb}: qpdf lost a sound file"), &lane.qpdf_lost);
        print_list(
            &format!("{verb}: a reader could not draw ours"),
            &lane.unreadable,
        );
        print_list(
            &format!("{verb}: a reader outran the budget on ours"),
            &lane.timed_out,
        );
        print_list(&format!("{verb}: drew differently"), &lane.differ);
        print_list(&format!("{verb}: §14.7 faults"), &lane.structure_faults);
    }
    print_list("panicked", &tally.panicked);
}

/// The documents the walk examines: every tagged one, plus every [`STRIDE`]th.
///
/// Taggedness is asked of the file here rather than inside `examine`, because a document that
/// costs fourteen foreign invocations is worth choosing before it is opened four times.
fn sample(files: &[PathBuf]) -> Vec<PathBuf> {
    files
        .iter()
        .enumerate()
        .filter(|(index, path)| {
            index.checked_rem(STRIDE) == Some(0) || {
                std::fs::read(path).ok().is_some_and(|bytes| {
                    Document::open_with_limits(bytes, Limits::DEFAULT).is_ok_and(|document| {
                        document.catalog().is_ok_and(|catalog| {
                            document
                                .get_key(&catalog, "StructTreeRoot")
                                .as_dict()
                                .is_some()
                        })
                    })
                })
            }
        })
        .map(|(_, path)| path.clone())
        .collect()
}

/// What fails the run, after the census has printed everything that did not.
fn verdict(tally: &Tally) {
    assert!(
        tally.panicked.is_empty(),
        "principle 1: no panic on any input"
    );
    for verb in VERBS {
        let Some(lane) = tally.lanes.get(verb) else {
            continue;
        };
        assert!(
            lane.qpdf_lost.is_empty(),
            "{verb}: qpdf --check accepted the source and reports errors in what we wrote"
        );
        assert!(
            lane.unreadable.is_empty(),
            "{verb}: a foreign reader drew the source page and could not draw ours"
        );
        for (name, why) in &lane.structure_faults {
            assert!(
                STRUCTURE_HELD.iter().any(|(held, _)| held == name),
                "{verb}: §14.7's carried tree is not what a foreign reader makes of the \
                 source's, and nobody has read it: {name}: {why}"
            );
        }
        for (name, why) in &lane.differ {
            assert!(
                HELD.iter().any(|(held, _)| held == name),
                "{verb}: a derived page a foreign reader draws differently and nobody has read: \
                 {name}: {why}"
            );
        }
    }
    let identical: usize = tally
        .lanes
        .values()
        .map(|lane| lane.identical.iter().sum::<usize>())
        .sum();
    assert!(
        identical > 0,
        "a corpus no foreign reader draws twice is not this corpus"
    );
    let agreed: usize = tally.lanes.values().map(|lane| lane.structure_agreed).sum();
    assert!(
        agreed > 0,
        "a sample with no carried structure tree cannot answer the question this walk exists for"
    );
}

/// The walk.
#[test]
#[ignore = "corpus-scale: every sampled document written four ways and read by poppler, mupdf and qpdf; run explicitly under the gates profile"]
fn what_this_suite_writes_is_read_the_same_way_by_poppler_and_mupdf() {
    require_the_sandbox();
    let Some(files) = corpus() else {
        // Prefixed, unlike this crate's other walks, so that `tools/state.sh`'s filter matches a
        // skipped run instead of reporting the section as having said nothing.
        println!("transform-foreign: skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let missing = missing_readers();
    if !missing.is_empty() {
        // A skip rather than a failure, and the one gate in this crate where that is right: with
        // no foreign reader there is no foreign readback, and every other property of these four
        // writers is already asserted by the four walks beside this one.
        println!(
            "transform-foreign: skipped: this walk's readers are not installed: {}",
            missing.join(", ")
        );
        return;
    }
    for reference in READERS {
        println!(
            "transform-foreign: {}: {}",
            reference.name(),
            reference.version().unwrap_or_default()
        );
    }

    let second = fixed_second();
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("foreign-readback");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).expect("a work directory");

    let sample = sample(&files);

    let tally = Mutex::new(Tally::default());
    let started = Instant::now();
    sample.par_iter().for_each(|path| {
        let name = path.display().to_string();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            examine(path, &second, &base, &tally);
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
    let _ = std::fs::remove_dir_all(&base);
    let tally = tally.into_inner().expect("reported through the tally");

    census(&tally, files.len(), elapsed);

    verdict(&tally);
}

/// The scan of `mutool show`'s output, over the two shapes a parent-tree value takes.
///
/// A test of the *instrument* rather than of the tree: everything this walk asserts about §14.7
/// is read out of mupdf's printed text, so a scan that misread it would make the lane say
/// whatever it liked (trap 27).
#[test]
fn the_scan_of_mupdfs_output_reads_both_forms_of_a_parent_tree_value() {
    // The number tree, direct in one file and naming an object in the other.
    assert_eq!(nums_value_index("[ 0 13 0 R ]", 0), Some(2));
    assert_eq!(nums_value_index("[ 0 [ 5 0 R 6 0 R ] ]", 0), Some(2));
    assert_eq!(nums_value_index("[ 0 13 0 R 1 14 0 R ]", 1), Some(4));
    assert_eq!(nums_value_index("[ 0 13 0 R ]", 7), None);
    // A nested array is one member, so it does not shift the key positions after it.
    assert_eq!(nums_value_index("[ 0 [ 5 0 R ] 3 9 0 R ]", 3), Some(4));
    // A dictionary is one member too, which is the form an object key's value takes when mupdf
    // resolves it.
    assert_eq!(
        nums_value_index("[ 0 << /S /P /P 6 0 R >> 1 9 0 R ]", 1),
        Some(4)
    );

    // The value, as mupdf prints it in each of its two forms.
    assert_eq!(array_shape("[ 5 0 R 6 0 R ]").as_deref(), Some("rr"));
    assert_eq!(
        array_shape("13 0 obj\n[ 12 0 R 15 0 R ]\nendobj\n").as_deref(),
        Some("rr")
    );
    // §7.3.9's null in a position no element claims, which a producer does write.
    assert_eq!(
        array_shape("[ null null 32 0 R null ]").as_deref(),
        Some("--r-")
    );
    // The defect this lane found: an array of one null where the source had two references.
    assert_ne!(array_shape("[ null ]").as_deref(), Some("rr"));
    assert_eq!(array_shape("null").as_deref(), None);
    assert_eq!(array_shape("<< /Nums [ 0 1 ] >>").as_deref(), None);
}
