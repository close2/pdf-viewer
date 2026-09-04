//! Every corpus on this disk, read through the whole of RFC 0003 section 4's layout.
//!
//! `doc/todo/58` §5 has owed this one since the core landed, and the nine-hundred-and-eleventh
//! session is the argument for it: that round mounted the face by hand over four committed
//! documents and found ten defects, of which **the three deepest were reads** — a page with two
//! images that killed the confined worker, a listing that produced 1674 `EIO`s over an ordering
//! defect in §14.7's structure carry, and a second listing that cost more than the first. Not one
//! of the three could be seen by `tests/a_face.rs`, because not one of them is in the four
//! documents it carries. The write side has had a 974-document walk since session 909
//! (`tests/write_corpus.rs`); this is its counterpart, and it is deliberately the *same* shape.
//!
//! For every document of the population below, over one mount:
//!
//! 1. **the whole layout is listed** — the root, `pages/`, `renders/` and each resolution under
//!    it, `images/` and each page's directory under it, `text/`, `attachments/`, `meta/`;
//! 2. **every entry is `stat`ed**, which by RFC 0003 section 5.5's rule *generates*;
//! 3. **every file is read**, and held against the reader the layout says it delegates to;
//! 4. **the listings are read a second time**, and what that costs is counted rather than timed.
//!
//! # The population is every corpus on the disk, and it is classified
//!
//! Until the nine-hundred-and-nineteenth session this walk was `doc/pdf.js`'s alone, and a second
//! instrument — `tests/awkward_classes.rs`, session 917 — asked a *narrower* question over a
//! *wider* population: does the confined worker survive at all, for a document of each class the
//! pdf.js corpus under-populates. Two instruments each asking half a question are two things to
//! keep in agreement, and `doc/todo/58` §4 said which way they merge — widen this one, and its
//! byte comparison covers those classes too. So, since ADR 0878:
//!
//! - **every `doc/pdf.js` document is walked**, as before, which is what keeps the figures this
//!   walk has printed since session 914 comparable with the sessions that wrote them down;
//! - **every other corpus root on this disk** — the `doc/corpora` submodules and the
//!   `corpus-cache` collections, whichever of them this machine has — is sampled at a fixed
//!   stride, classified, and the first [`PER_CLASS`] documents of each class are walked beside
//!   them. That is where damaged, huge, JBIG2 and JPEG 2000 documents actually live, and a
//!   machine with none of those roots checked out walks exactly what it walked before;
//! - **every document of the population is classified**, pdf.js's included, so the matrix the run
//!   prints is over the whole walk rather than over its widening;
//! - **and a widened document is read to a smaller depth**, which is [`Bounds`] and is the one
//!   place the two halves of the population differ. Listings are whole for both — every name of
//!   every directory against the layout's own — and what is bounded is the *reads*: a widened
//!   document's first [`PAGES_SAMPLED`] pages. A second bound stood beside it for one round, on
//!   the entries of a directory, and [`Bounds`] says why it is gone.
//!
//! A class is not a diagnosis: a document is in as many of them as it satisfies, and *plain* is
//! in the list because a sweep that meets only awkward documents cannot say whether what it found
//! is the class or the walk. Session 917 is why the matrix is printed rather than the total: with
//! `no_machine_fonts()` taken out of the worker, the control class died more often than the
//! encrypted one, which is ADR 0876's misattribution reproduced at corpus scale.
//!
//! # A death is not a refusal
//!
//! The other half of what came over from that instrument. A refusal is a sentence a face can show
//! and is counted here by reason; a **death** is a worker killed by a signal — what a face's user
//! sees as a folder that stops answering — and `confined-transport`'s supervision words one as
//! `killed by signal N`. Any sentence naming one fails this run wherever it appears, in an open, a
//! listing, a read or a comparison; and each mount is asked one more question after its walk, so
//! that session 902's recovery of a dead worker is measured rather than claimed.
//!
//! # What each file is held to, and why that is not a second implementation
//!
//! RFC 0003 section 7 forbids this crate a second implementation of anything, and
//! `crate::layout::Generator` names the delegate of every row. So the expectation for each file
//! is *computed here by that delegate* rather than described:
//!
//! - `pages/NNNN.pdf` is `pdf_transform::apply`'s own piece for that page, byte for byte
//!   (`Plan::Split`, `Pieces::EachPage`), which is what `cp` out of a mount **is**.
//! - `renders/DPIdpi/NNNN.png` is `Plan::Render` at that resolution, byte for byte.
//! - `images/NNNN/NAME` is one output of `Plan::Images` for that page, under the name that run's
//!   own sink was opened with — and the *listing* is that run's whole set of names, which is the
//!   property `crate::layout`'s departure from RFC section 4 was taken to obtain.
//! - `text/NNNN.txt` is `pdf_model::interpret`'s readback, byte for byte, and `text/document.txt`
//!   is those joined by a form feed.
//! - `attachments/NAME` is what `Plan::Attachments`'s `Save` writes for the name the document
//!   files it under, and the listing is that inventory with §7.11.4's names made safe.
//! - `meta/info.json` states exactly what `pdf_model::metadata::Information` answers, entry by
//!   entry; `meta/xmp.xml` is the catalog's `/Metadata` stream decoded, byte for byte; and
//!   `meta/outline.json` is checked against `pdf_model::outline::Outline`'s own item count.
//!
//! # The transport is the confined one
//!
//! `doc/todo/58` §4: "[n]o face ships before it exists", because a mount is entered by anything
//! that touches a folder. So the tree under test here is mounted on
//! [`pdf_vfs::ConfinedWorkers`] over a [`pdf_vfs::FileBacking`] — the posture a face has, with
//! the document crossing as a descriptor (ADR 0812) — while every expectation above is computed
//! **in this process** by the same `pdf_transform` plan [`pdf_vfs::worker::InProcess`] runs. Each
//! comparison is therefore two things at once: the delegation check RFC section 7 asks for, and
//! the two-transport comparison `tests/confined.rs` makes on four documents, here made over the
//! corpus. Round 911's worker kill was invisible to every in-process test and would fail this
//! walk on the document that carries it.
//!
//! **This process puts itself in the worker's font posture**, and that is not a convenience: a
//! confined worker cannot read `/usr/share/fonts` and says so before it is confined (ADR 0870), so
//! a document naming an uninstalled face is drawn from the compiled-in faces there and from the
//! machine's here. Comparing the two would be comparing two machines. `no_machine_fonts` is
//! therefore stated in this binary as well, which is what makes every byte comparison below a
//! statement about the *transport*. The fidelity that costs is `doc/todo/58`'s to close, and the
//! walk is the instrument that will say when it has been.
//!
//! The three `meta/` files have no plan to compare against — their JSON is composed inside
//! `crate::worker` — so those are compared against a second tree over the same file mounted
//! **in process**, which is cheap (three small files) and is the only place the two transports
//! are asked the same question directly.
//!
//! # What is a failure and what is held
//!
//! A refusal is not a failure (trap 11): a document `pdf_transform` declines by name, a page the
//! rasteriser will not draw under [`budget`], a codec nothing here has — each is counted by
//! reason and printed. What fails the run is a *disagreement*: a file whose bytes are not the
//! delegate's, a listing whose shape is not the layout's, a `stat` whose size is not the bytes',
//! a second reading that differs from the first, or a panic. [`HELD`] is where a difference goes
//! that has been read and diagnosed as the document's; empty is the state to keep.
//!
//! # Running it
//!
//! ```text
//! cargo build --profile gates -p pdf-vfs --bins
//! tools/bounded.sh --data 12 --tree 12 -- cargo test --profile gates -p pdf-vfs --test read_corpus -- --ignored --nocapture
//! ```

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    reason = "test code: an explanatory panic is the intended failure, and the census output \
              is the point of the run"
)]

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use corpus_classes::{Choice, Chosen, Class, Contribution, PDFJS, is_a_death};
use pdf_syntax::{Document, FileBytes, Limits};
use pdf_transform::attachments::{Action, AttachmentsPlan};
use pdf_transform::images::ImagesPlan;
use pdf_transform::json::Value;
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Secret, Source, apply};
use pdf_vfs::layout::Kind;
use pdf_vfs::worker::{InProcess, Worker, WorkerError, Workers};
use pdf_vfs::{Config, ConfinedWorkers, DirEntry, FileBacking, Vfs};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// How many of a document's pages are read, `stat`ed and compared.
///
/// The listings are always whole — every name of every directory is checked against the layout,
/// whatever the page count — and this bounds only the *reads*, which are where the wall clock is:
/// one page of this walk is a page extraction, two renders (150 and 300 dpi), one image
/// extraction and one text readback, and the second pass repeats them.
///
/// Sixteen, and the figure is `doc/pdf.js`'s rather than a guess: 1747 pages over 973 documents,
/// of which one document holds 352 and two more hold 55 and 23. A per-document ceiling costs
/// this walk 4 % of its pages and takes the longest document's serial run — one rayon task —
/// from 352 pages to 16. Every page of 966 of the 973 documents is still read.
const PAGES_READ: usize = 16;

/// How many pages of a *widened* document are read, `stat`ed and compared.
///
/// **Two depths, one instrument**, and the second one is the cost session 917 gave as its reason
/// for keeping two (ADR 0878). At sixteen pages the widened population is not a slower walk but a
/// different one: `tika-issue-tracker/batch1/PDFBOX/PDFBOX-186-0.pdf` was still generating after
/// **25 minutes** on one document, in the worker and in this process alike, and the run's peak was
/// 9.03 GiB of the 12 the bound allows. Neither side of that comparison can be interrupted —
/// `Vfs` reaches no `Canceller` (`doc/todo/58` §4) and a thread computing an expectation cannot be
/// stopped at all — so the bound has to be on what is *asked for*.
///
/// Two, which is the width session 917's sweep used over the same roots for the same reason: what
/// the widening is for is a class of document reaching the generators at all, and the second page
/// is there because the first page of a document is the one every other test reads.
const PAGES_SAMPLED: usize = 2;

/// What a document of this root has read of it.
///
/// **Two depths, one instrument.** `doc/pdf.js` is walked exactly as it was before the widening,
/// which is what keeps the figures printed since session 914 comparable; every other root is
/// sampled, because the population that carries the awkward classes carries the pathological
/// documents too, and a gate one document can hold for half an hour is not a gate (ADR 0878).
///
/// **A second bound stood here for one round and has come off, which is the point of it.** Session
/// 919 found `tika-issue-tracker/batch1/PDFBOX/PDFBOX-186-0.pdf` — 10 084 images on one page, so
/// `/images/0001/` is a directory of ten thousand files — holding this walk for twenty-five
/// minutes, and bounded the reads to four entries a directory to get past it. That was a bound on
/// the *instrument*: a walk that skips the pathological case cannot see the next one. Round 923
/// measured what was actually costing the time (a name validated by re-running the extraction that
/// named it, ADR 0886) and fixed it in the core, and that document's whole ten-thousand-entry
/// directory is now listed, `stat`ed and read in seconds. So the entries are whole again, and only
/// the pages are bounded.
#[derive(Debug, Clone, Copy)]
struct Bounds {
    /// How many of the document's pages are read.
    pages: usize,
}

/// What this document is walked to, by the root it came from.
fn bounds(chosen: &Chosen) -> Bounds {
    if chosen.root == PDFJS {
        Bounds { pages: PAGES_READ }
    } else {
        Bounds {
            pages: PAGES_SAMPLED,
        }
    }
}

/// The corpus documents that refuse §7.6.4.1's default user password, with the password each
/// one's own pdf.js issue records.
///
/// `tests/write_corpus.rs`'s list, and the same reason: the population is every document the
/// suite can open rather than every document that opens for free. `Vfs` itself has no way to be
/// given one (`doc/todo/58` §5 records the `SecretSource` shortfall), so the walk supplies it at
/// [`KeyedWorkers`], which is the seam such a source would use.
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

/// Documents whose reading the walk cannot explain, each with its diagnosis.
///
/// Empty is the state to keep. An entry here is a *reading* of why the difference is the
/// document's rather than the core's, and the walk fails on any difference it does not name.
const HELD: &[(&str, &str)] = &[];

/// The §14.3.3 entries `meta/info.json` states, in the order `crate::worker` writes them.
const INFORMATION_KEYS: &[&str] = &[
    "title", "author", "subject", "keywords", "creator", "producer", "created", "modified",
    "trapped",
];

/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// Trap 10: `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a separate program
/// Cargo does not build for a test of another package. Here it would not make the two sides
/// *disagree* — it would make them agree on pages whose images are missing, which is a weaker
/// gate wearing the same number.
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so both sides of every comparison \
             would be drawn without CCITT, JBIG2 or JPEG 2000 images: {error}"
        );
    }
}

/// How many documents are classified from each corpus root other than `doc/pdf.js`.
///
/// Classification opens the document and walks its objects, which is the cost this bounds; the
/// stride is [`corpus_classes::sampled`]'s.
const SAMPLE_PER_ROOT: usize = 1200;

/// How many documents of each class are walked from each root other than `doc/pdf.js`.
///
/// The whole cost of the widening, and it is chosen against this walk's own wall clock rather
/// than for coverage: a widened document is walked exactly as a pdf.js one is — up to
/// [`PAGES_READ`] pages, each extracted, drawn twice, read for its images and for its text, on
/// both sides of the comparison — and the *huge* class is in the population on purpose.
const PER_CLASS: usize = 6;

/// The population: every `doc/pdf.js` document, and a class-balanced sample of every other root.
///
/// `doc/pdf.js` whole, because the figures this walk has printed since session 914 are over it and
/// a widening that moved them would make them incomparable; every other root sampled, because
/// that is where damaged, huge, JBIG2 and JPEG 2000 documents live and walking all of them is a
/// day rather than a gate. A machine with no other corpus checked out walks exactly what it
/// walked before, and the report says which roots it found.
fn chosen() -> (Vec<Chosen>, Vec<Contribution>) {
    let choice = Choice {
        whole: vec![PDFJS.to_owned()],
        sample_per_root: SAMPLE_PER_ROOT,
        per_class: PER_CLASS,
    };
    let (chosen, contributions) =
        corpus_classes::population(&corpus_classes::roots(), &choice, &|name| {
            password_for(name).to_owned()
        });
    // A walk over a thousand documents that has to be run for *one* of them is how a slow
    // document is diagnosed, and there is no other way in: the population is derived, so it
    // cannot be narrowed by editing a list. The filter is named in the run's output so that a
    // figure taken under it can never be read as the whole walk's.
    match std::env::var("PDFVFS_READ_ONLY") {
        Ok(only) if !only.is_empty() => {
            println!("vfs-read: PDFVFS_READ_ONLY={only} — this is not the whole walk");
            let kept = chosen
                .into_iter()
                .filter(|one| one.display.contains(&only))
                .collect();
            (kept, contributions)
        }
        _ => (chosen, contributions),
    }
}

/// The password the corpus records for this document, or the empty one.
fn password_for(name: &str) -> &'static str {
    KNOWN_PASSWORDS
        .iter()
        .find(|(known, _)| *known == name)
        .map_or("", |(_, password)| password)
}

/// The budget both sides are drawn under, so that a page refused for size is refused twice.
///
/// `Budget::default()`'s 2²⁸ pixels is a gibibyte of RGBA a page, and this walk holds several
/// rasters at once across rayon; 2²⁴ is `write_corpus.rs`'s ceiling and still four times a 300
/// dpi A4 page. It is the mount's own `Config::budget` as well as this process's expectation, so
/// the two sides refuse the same pages.
fn budget() -> Budget {
    Budget {
        limits: Limits::DEFAULT,
        max_pixels: 1 << 24,
    }
}

/// The configuration the walk mounts with: the layout's own resolutions, this walk's budget.
fn config() -> Config {
    Config {
        budget: budget(),
        ..Config::default()
    }
}

/// Which transport a tree is mounted on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Transport {
    /// Another process, confined: what a face has.
    Confined,
    /// This process: what the `meta/` comparison needs a second answer from.
    Here,
}

/// Workers that know §7.6.4.1's password for this document, on the transport named.
#[derive(Debug)]
struct KeyedWorkers(&'static str, Transport);

impl Workers for KeyedWorkers {
    fn spawn(
        &self,
        bytes: FileBytes,
        password: Option<Secret>,
        policy: Policy,
        budget: Budget,
    ) -> Result<Box<dyn Worker>, WorkerError> {
        let secret =
            password.or_else(|| (!self.0.is_empty()).then(|| Secret::from(self.0.to_owned())));
        match self.1 {
            Transport::Confined => ConfinedWorkers::start(
                &bytes,
                secret.as_ref(),
                policy,
                budget,
                pdf_vfs::MachineFaces::Withheld,
            )
            .map(|worker| Box::new(worker) as Box<dyn Worker>),
            Transport::Here => {
                let source = match secret {
                    Some(secret) => Source::with_password(bytes, secret),
                    None => Source::new(bytes),
                };
                // One strip: a rayon task per document already, and a worker that split a render
                // across the pool inside one of them would be measuring the scheduler.
                Ok(Box::new(InProcess::new(source, policy, budget, Some(1))))
            }
        }
    }
}

/// A tree over the corpus file itself, on one transport.
///
/// [`FileBacking`] rather than the memory one `write_corpus.rs` uses, and the difference is the
/// point: nothing here writes, and a document on disk crosses the confinement as a descriptor
/// (ADR 0812) instead of as a frame, which is what a mount does and what the message budget
/// would otherwise bound.
fn mounted(path: &Path, name: &str, transport: Transport) -> Vfs {
    Vfs::new(
        Box::new(FileBacking::new(path)),
        Box::new(KeyedWorkers(password_for(name), transport)),
        config(),
    )
}

/// A transform source over this document, with the corpus's known password where it has one.
fn source(name: &str, path: &Path) -> Result<Source, String> {
    let bytes = FileBytes::on_disk(path).map_err(|why| format!("{}: {why}", path.display()))?;
    let password = password_for(name);
    Ok(if password.is_empty() {
        Source::new(bytes)
    } else {
        Source::with_password(bytes, Secret::from(password.to_owned()))
    })
}

/// One plan's outputs, in the order the sinks were opened, or the refusal as a sentence.
fn produce(name: &str, path: &Path, plan: &Plan) -> Result<Vec<(String, Vec<u8>)>, String> {
    let sinks = MemorySinks::new();
    apply(
        plan,
        &[source(name, path)?],
        &sinks,
        &Policy::default(),
        &budget(),
    )
    .map_err(|why| why.to_string())?;
    Ok(sinks.into_outputs())
}

/// A one-page selection, which every generator of this layout takes.
fn one(page: usize) -> Selection {
    page.to_string().parse::<Selection>().expect("a selection")
}

/// The pattern `crate::worker` opens a one-output run's sink with.
fn page_pattern() -> pdf_transform::pattern::Pattern {
    "%d".parse().expect("a pattern")
}

/// A 64-bit digest of some bytes, so that the second pass can compare without holding the corpus.
fn digest(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// How a listing spells one ordinal in a document of this many pages.
///
/// RFC 0003 section 4: "zero-padded ordinal; width from page count", with four the floor.
fn stem(pages: usize, ordinal: usize) -> String {
    let width = pages.to_string().len().max(4);
    format!("{ordinal:0width$}")
}

/// What the walk found, one field per claim.
#[derive(Default)]
struct Tally {
    /// Documents the core could not open, by reason.
    refused_open: Vec<(String, String)>,
    /// Documents the whole layout was walked over.
    walked: usize,
    /// Documents with no page at all.
    pageless: usize,
    /// Directories listed, and entries they named.
    directories: usize,
    /// Entries `stat`ed.
    statted: usize,
    /// Files read.
    read: usize,
    /// Bytes read out of the trees.
    bytes: u64,
    /// Files whose bytes are the delegate's, per layout row.
    matched: BTreeMap<&'static str, usize>,
    /// Files the tree refused by name, per layout row, with the reason.
    refused: Vec<(String, String)>,
    /// Files whose bytes are **not** the delegate's.
    differ: Vec<(String, String)>,
    /// A listing whose names, count or order are not the layout's.
    listing_failed: Vec<(String, String)>,
    /// A `stat` whose size is not the size of the bytes that came out.
    size_failed: Vec<(String, String)>,
    /// A second reading of a file that is not the first reading.
    unstable: Vec<(String, String)>,
    /// A second `stat` pass that generated a file the first pass had already produced.
    regenerated: Vec<(String, String)>,
    /// The two transports gave different bytes for one of `meta/`'s three files.
    transport_differ: Vec<(String, String)>,
    /// Files the walk produced no expectation for, because the delegate refused them too.
    both_refused: usize,
    /// Pages past [`PAGES_READ`], listed and not read.
    pages_not_read: usize,
    /// A document whose examination panicked, which principle 1 forbids.
    panicked: Vec<(String, String)>,
    /// A question whose sentence names a signal: the confined worker died.
    killed: Vec<(String, String)>,
    /// A mount that did not answer after its walk, so a dead worker was not replaced.
    unrecovered: Vec<String>,
    /// What each class of document cost and answered, over the whole population.
    classes: BTreeMap<Class, ClassTally>,
    /// Every document's wall clock, so that the slowest few can be printed.
    took: Vec<(String, f64)>,
}

/// One row of the matrix the run prints.
///
/// A document counts in every class it falls into, so these columns sum to more than the
/// population: a class is a property rather than a slot.
#[derive(Default, Clone, Copy)]
struct ClassTally {
    /// Documents of this class walked.
    documents: usize,
    /// Files read out of their trees.
    read: usize,
    /// Bytes those files held.
    bytes: u64,
    /// Questions the tree refused by name, the open included.
    refused: usize,
    /// Questions whose sentence names a signal.
    killed: usize,
}

/// Adds to the shared tally, ignoring a poisoned lock (another document's panic is already being
/// reported; losing one entry to it changes nothing).
fn record(tally: &Mutex<Tally>, update: impl FnOnce(&mut Tally)) {
    if let Ok(mut tally) = tally.lock() {
        update(&mut tally);
    }
}

/// One document's own accumulator, merged into the shared tally at the end.
///
/// A per-document tally rather than a lock per file: this walk touches tens of files a document
/// and a mutex on each would be measuring the lock.
#[derive(Default)]
struct Local {
    directories: usize,
    statted: usize,
    read: usize,
    bytes: u64,
    matched: BTreeMap<&'static str, usize>,
    refused: Vec<String>,
    differ: Vec<String>,
    listing_failed: Vec<String>,
    size_failed: Vec<String>,
    unstable: Vec<String>,
    regenerated: Vec<String>,
    transport_differ: Vec<String>,
    both_refused: usize,
    pages_not_read: usize,
    /// Every file read in the first pass, by path, as (size, digest).
    seen: BTreeMap<String, (u64, u64)>,
    /// Whether the whole layout was walked, which a document the core cannot open is not.
    walked: bool,
    /// Whether it opened and reached no page.
    pageless: bool,
    /// Why the core would not open it at all.
    refused_open: Option<String>,
    /// Whether the mount answered one more question after the walk (session 902's recovery).
    recovered: bool,
    /// What this document cost, wall clock, on one rayon task.
    ///
    /// Printed for the slowest few rather than asserted on: a document that costs minutes is a
    /// fact about a *face* — a person's file manager waiting on one `stat` — and this walk is
    /// where it shows up first. What bounds the run is [`PAGES_SAMPLED`], not a threshold here.
    took: std::time::Duration,
}

impl Local {
    /// Counts one file whose bytes were the delegate's, under its layout row.
    fn agreed(&mut self, row: &'static str) {
        let count = self.matched.entry(row).or_default();
        *count = count.saturating_add(1);
    }

    /// Every sentence this document's walk produced, whichever column it fell into.
    ///
    /// One iterator rather than a check at each site, so that [`is_a_death`] is asked of all of
    /// them: a worker that dies does so under whichever question was being asked, and the column
    /// that catches the answer is not the one that decides whether it was a death.
    fn sentences(&self) -> impl Iterator<Item = &String> {
        self.refused_open
            .iter()
            .chain(&self.refused)
            .chain(&self.differ)
            .chain(&self.listing_failed)
            .chain(&self.size_failed)
            .chain(&self.unstable)
            .chain(&self.regenerated)
            .chain(&self.transport_differ)
    }
}

/// Reads one file of the tree and holds it against the bytes its delegate produced.
///
/// `expected` is `Ok` with the delegate's bytes, or `Err` with the sentence the delegate refused
/// by — in which case the tree is *required* to refuse it too, which is the half of trap 5 a
/// walk can check: a file that comes out where the delegate produced nothing is bytes this crate
/// invented.
fn one_file(
    vfs: &Vfs,
    local: &mut Local,
    row: &'static str,
    path: &str,
    expected: &Result<Vec<u8>, String>,
) {
    let stat = vfs.stat(path);
    let opened = vfs.open(path);
    match (&stat, &opened) {
        (Ok(attributes), Ok(handle)) => {
            local.statted = local.statted.saturating_add(1);
            local.read = local.read.saturating_add(1);
            local.bytes = local.bytes.saturating_add(handle.len());
            if attributes.kind != Kind::File {
                local
                    .listing_failed
                    .push(format!("{path}: stat says it is a directory"));
            }
            // RFC 0003 section 5.5: a `stat` may not estimate, because "an under-estimate
            // silently truncates a page".
            if attributes.size != Some(handle.len()) {
                local.size_failed.push(format!(
                    "{path}: stat said {:?} and {} bytes came out",
                    attributes.size,
                    handle.len()
                ));
            }
            local
                .seen
                .insert(path.to_owned(), (handle.len(), digest(handle.bytes())));
            match expected {
                Ok(wanted) if wanted == handle.bytes() => local.agreed(row),
                Ok(wanted) => local.differ.push(format!(
                    "{path}: {} bytes out of the tree, {} from {row}'s own generator",
                    handle.len(),
                    wanted.len()
                )),
                Err(why) => local.differ.push(format!(
                    "{path}: {} bytes out of the tree where {row}'s generator refused: {why}",
                    handle.len()
                )),
            }
        }
        (_, Err(error)) => {
            match expected {
                Ok(wanted) => local.differ.push(format!(
                    "{path}: the tree refused it ({error}) where {row}'s generator wrote {} bytes",
                    wanted.len()
                )),
                Err(_) => local.both_refused = local.both_refused.saturating_add(1),
            }
            local.refused.push(format!("{path}: {error}"));
        }
        (Err(error), Ok(_)) => {
            local
                .size_failed
                .push(format!("{path}: opens and does not stat: {error}"));
        }
    }
}

/// Every name a directory lists, in the order it lists them, or the error as a sentence.
fn listing(vfs: &Vfs, path: &str) -> Result<Vec<DirEntry>, String> {
    vfs.list(path).map_err(|error| format!("{path}: {error}"))
}

/// Holds one listing to the names the layout states, in order.
fn holds_names(local: &mut Local, path: &str, listed: &[DirEntry], wanted: &[String]) {
    let got: Vec<&str> = listed.iter().map(|entry| entry.name.as_str()).collect();
    if got.len() != wanted.len() {
        local.listing_failed.push(format!(
            "{path}: lists {} names and the document has {}",
            got.len(),
            wanted.len()
        ));
        return;
    }
    for (at, (had, want)) in got.iter().zip(wanted.iter()).enumerate() {
        if had != want {
            local.listing_failed.push(format!(
                "{path}: entry {at} is {had:?} and the layout spells it {want:?}"
            ));
            return;
        }
    }
}

/// The root: six directories, each of them one.
fn root(vfs: &Vfs, local: &mut Local) {
    match listing(vfs, "/") {
        Ok(listed) => {
            local.directories = local.directories.saturating_add(1);
            let mut names: Vec<String> = listed.iter().map(|entry| entry.name.clone()).collect();
            names.sort();
            if names != ["attachments", "images", "meta", "pages", "renders", "text"] {
                local
                    .listing_failed
                    .push(format!("/: the root lists {names:?}"));
            }
            for entry in &listed {
                if entry.kind != Kind::Directory {
                    local
                        .listing_failed
                        .push(format!("/{}: the root named a file", entry.name));
                }
                let path = format!("/{}", entry.name);
                match vfs.stat(&path) {
                    Ok(attributes) => {
                        local.statted = local.statted.saturating_add(1);
                        if attributes.kind != Kind::Directory || attributes.size.is_some() {
                            local
                                .listing_failed
                                .push(format!("{path}: stats as {attributes:?}"));
                        }
                    }
                    Err(error) => local.listing_failed.push(format!(
                        "{path}: the root named it and it does not stat: {error}"
                    )),
                }
            }
        }
        Err(why) => local.listing_failed.push(why),
    }
}

/// `pages/`: one name per page, and each of them `pdf_transform`'s own piece.
fn pages(vfs: &Vfs, local: &mut Local, name: &str, path: &Path, count: usize, bounds: Bounds) {
    let Ok(listed) = listing(vfs, "/pages").inspect_err(|why| local.refused.push(why.clone()))
    else {
        return;
    };
    local.directories = local.directories.saturating_add(1);
    let wanted: Vec<String> = (1..=count)
        .map(|page| format!("{}.pdf", stem(count, page)))
        .collect();
    holds_names(local, "/pages", &listed, &wanted);
    for (at, entry) in listed.iter().enumerate() {
        let page = at.saturating_add(1);
        if page > bounds.pages {
            local.pages_not_read = local.pages_not_read.saturating_add(1);
            continue;
        }
        let expected = produce(
            name,
            path,
            &Plan::Split(SplitPlan {
                source: 0,
                pages: one(page),
                pieces: Pieces::EachPage,
                names: page_pattern(),
            }),
        )
        .and_then(|outputs| {
            outputs
                .into_iter()
                .next()
                .map(|(_, bytes)| bytes)
                .ok_or_else(|| String::from("split wrote no piece"))
        });
        one_file(
            vfs,
            local,
            "/pages/NNNN.pdf",
            &format!("/pages/{}", entry.name),
            &expected,
        );
    }
}

/// `renders/`: the resolutions the core offers, and each page drawn at each of them.
fn renders(vfs: &Vfs, local: &mut Local, name: &str, path: &Path, count: usize, bounds: Bounds) {
    let Ok(listed) = listing(vfs, "/renders").inspect_err(|why| local.refused.push(why.clone()))
    else {
        return;
    };
    local.directories = local.directories.saturating_add(1);
    let wanted: Vec<String> = config()
        .resolutions
        .iter()
        .map(|dpi| format!("{dpi}dpi"))
        .collect();
    holds_names(local, "/renders", &listed, &wanted);
    for (dpi, directory) in config().resolutions.iter().zip(listed.iter()) {
        let at = format!("/renders/{}", directory.name);
        let Ok(pages) = listing(vfs, &at).inspect_err(|why| local.refused.push(why.clone())) else {
            continue;
        };
        local.directories = local.directories.saturating_add(1);
        let wanted: Vec<String> = (1..=count)
            .map(|page| format!("{}.png", stem(count, page)))
            .collect();
        holds_names(local, &at, &pages, &wanted);
        for (index, entry) in pages.iter().enumerate() {
            let page = index.saturating_add(1);
            if page > bounds.pages {
                local.pages_not_read = local.pages_not_read.saturating_add(1);
                continue;
            }
            // `Sizing::Dpi` is ISO 32000-2 §8.3.2.3's 72 units to the inch, which is the
            // conversion `crate::worker::dpi_as_scale` hands it and nothing computes here.
            let expected = produce(
                name,
                path,
                &Plan::Render(RenderPlan {
                    source: 0,
                    pages: one(page),
                    size: Sizing::Dpi(f32::from(u16::try_from(*dpi).unwrap_or(u16::MAX))),
                    format: ImageFormat::Png,
                    page_box: None,
                    annotations: true,
                    names: page_pattern(),
                    strips: Some(1),
                }),
            )
            .and_then(|outputs| {
                outputs
                    .into_iter()
                    .next()
                    .map(|(_, bytes)| bytes)
                    .ok_or_else(|| String::from("render wrote no page"))
            });
            one_file(
                vfs,
                local,
                "/renders/DPI/NNNN.png",
                &format!("{at}/{}", entry.name),
                &expected,
            );
        }
    }
}

/// `images/`: a directory per page, whose listing **is** the extraction's own output names.
fn images(vfs: &Vfs, local: &mut Local, name: &str, path: &Path, count: usize, bounds: Bounds) {
    let Ok(listed) = listing(vfs, "/images").inspect_err(|why| local.refused.push(why.clone()))
    else {
        return;
    };
    local.directories = local.directories.saturating_add(1);
    let wanted: Vec<String> = (1..=count).map(|page| stem(count, page)).collect();
    holds_names(local, "/images", &listed, &wanted);
    for (index, entry) in listed.iter().enumerate() {
        let page = index.saturating_add(1);
        if page > bounds.pages {
            continue;
        }
        let at = format!("/images/{}", entry.name);
        let produced = produce(
            name,
            path,
            &Plan::Images(ImagesPlan {
                source: 0,
                pages: one(page),
                min_pixels: 0,
                list_only: false,
                native: true,
                no_mask: false,
                format: ImageFormat::Png,
                names: "%02d".parse().expect("a pattern"),
            }),
        );
        let inventory = match listing(vfs, &at) {
            Ok(inventory) => {
                local.directories = local.directories.saturating_add(1);
                inventory
            }
            Err(why) => {
                // A page whose extraction the tree declines is a refusal by name; the delegate
                // has to decline it too, or the listing is hiding an answer.
                if produced.as_ref().is_ok_and(|outputs| !outputs.is_empty()) {
                    local.listing_failed.push(format!(
                        "{at}: the tree will not list it where the extraction produced files: {why}"
                    ));
                } else {
                    local.refused.push(why);
                }
                continue;
            }
        };
        let Ok(outputs) = produced else {
            if !inventory.is_empty() {
                local.listing_failed.push(format!(
                    "{at}: lists {} names where the extraction refused",
                    inventory.len()
                ));
            }
            continue;
        };
        // The whole argument for `crate::layout`'s departure from RFC 0003 section 4: a listing
        // and a read are one call, so the names cannot disagree.
        let mut wanted: Vec<String> = outputs.iter().map(|(name, _)| name.clone()).collect();
        wanted.sort();
        // The listing is whole — every name of it against the extraction's own — and only the
        // *reads* are bounded, which is the split `PAGES_READ` already makes for pages.
        holds_names(local, &at, &inventory, &wanted);
        for entry in &inventory {
            let expected = outputs
                .iter()
                .find(|(name, _)| *name == entry.name)
                .map(|(_, bytes)| bytes.clone())
                .ok_or_else(|| format!("{:?} is not one of the extraction's outputs", entry.name));
            one_file(
                vfs,
                local,
                "/images/NNNN/NAME",
                &format!("{at}/{}", entry.name),
                &expected,
            );
        }
    }
}

/// `text/`: one file per page and the joined document, each `interpret`'s own readback.
fn text(vfs: &Vfs, local: &mut Local, path: &Path, name: &str, count: usize, bounds: Bounds) {
    let Ok(listed) = listing(vfs, "/text").inspect_err(|why| local.refused.push(why.clone()))
    else {
        return;
    };
    local.directories = local.directories.saturating_add(1);
    let mut wanted: Vec<String> = (1..=count)
        .map(|page| format!("{}.txt", stem(count, page)))
        .collect();
    wanted.push(String::from("document.txt"));
    holds_names(local, "/text", &listed, &wanted);

    // One open of the document for every page's readback, which is what `interpret` needs and
    // what the confined worker does on its own side.
    let read = Document::open_with_password(
        std::fs::read(path).unwrap_or_default(),
        Limits::DEFAULT,
        password_for(name),
    );
    let Ok(document) = read else {
        local
            .refused
            .push(String::from("/text: the document does not open here"));
        return;
    };
    let model = pdf_model::Pages::new(&document);
    let mut joined: Vec<u8> = Vec::new();
    let mut whole_is_known = count <= bounds.pages;
    // `document.txt` is every page joined, so a document longer than the walk's depth has no
    // expectation for it — and until session 919 this loop *still* interpreted every page of one,
    // to throw the text away. Reading past the depth when nothing will be compared is what made a
    // long document cost what a long document costs (ADR 0878).
    let read_to = if whole_is_known {
        count
    } else {
        count.min(bounds.pages)
    };
    for page in 1..=read_to {
        let expected = model.get(page.saturating_sub(1)).map_or_else(
            || Err(format!("page {page} could not be read")),
            |found| Ok(pdf_model::interpret(&document, &found).text.into_bytes()),
        );
        if page <= bounds.pages {
            one_file(
                vfs,
                local,
                "/text/NNNN.txt",
                &format!("/text/{}.txt", stem(count, page)),
                &expected,
            );
        }
        match expected {
            Ok(bytes) if whole_is_known => {
                if page > 1 {
                    joined.push(0x0c);
                }
                joined.extend_from_slice(&bytes);
            }
            Ok(_) => {}
            Err(_) => whole_is_known = false,
        }
    }
    // RFC 0003 section 4: "every page's readback in page order, separated by a form feed".
    let expected = if whole_is_known {
        Ok(joined)
    } else {
        Err(String::from(
            "a page past this walk's ceiling, or one that could not be read",
        ))
    };
    if whole_is_known {
        one_file(
            vfs,
            local,
            "/text/document.txt",
            "/text/document.txt",
            &expected,
        );
    }
}

/// `attachments/`: §7.11.4's embedded files, under the names the document files them by.
fn attachments(vfs: &Vfs, local: &mut Local, name: &str, path: &Path) {
    let Ok(listed) =
        listing(vfs, "/attachments").inspect_err(|why| local.refused.push(why.clone()))
    else {
        return;
    };
    local.directories = local.directories.saturating_add(1);
    // §7.11.4's inventory as the verb itself reports it. `Action::List` writes no output — what
    // it produces is a *report* — so the names come out of that rather than out of a sink, and
    // they are the document's own spelling; the tree's listing is those made safe.
    let inventory = attachment_names(name, path);
    if inventory.is_err() && !listed.is_empty() {
        local.listing_failed.push(format!(
            "/attachments: lists {} names where the attachments verb refused the document",
            listed.len()
        ));
        return;
    }
    let names = inventory.unwrap_or_default();
    for entry in &listed {
        if entry.kind != Kind::File {
            local.listing_failed.push(format!(
                "/attachments/{}: listed as a directory",
                entry.name
            ));
        }
        // A directory entry is not a path, and §7.11.4's names are the document's to choose.
        if entry.name.contains('/') || entry.name == "." || entry.name == ".." {
            local.listing_failed.push(format!(
                "/attachments/{}: not a name a directory can hold",
                entry.name
            ));
        }
    }
    // The bytes: the tree's file against what the verb saves for the name the *document* uses.
    // The tree's own listing is the sanitised name, so the pairing is by position — which is a
    // check of the mapping as well as of the bytes, since `crate::attachments` builds both from
    // one inventory in one order.
    if names.len() != listed.len() {
        local.listing_failed.push(format!(
            "/attachments: lists {} names and §7.11.4's inventory has {}",
            listed.len(),
            names.len()
        ));
        return;
    }
    for (entry, document_name) in listed.iter().zip(names.iter()) {
        let expected = produce(
            name,
            path,
            &Plan::Attachments(AttachmentsPlan {
                source: 0,
                action: Action::Save {
                    name: document_name.clone(),
                    names: page_pattern(),
                },
            }),
        )
        .and_then(|outputs| {
            outputs
                .into_iter()
                .next()
                .map(|(_, bytes)| bytes)
                .ok_or_else(|| format!("the verb saved nothing for {document_name:?}"))
        });
        one_file(
            vfs,
            local,
            "/attachments/NAME",
            &format!("/attachments/{}", entry.name),
            &expected,
        );
    }
}

/// §7.11.4's names as the document files them, in the order the verb reports them.
///
/// The one place this file reads a `Report` rather than a sink: `Action::List` writes no output,
/// so the inventory *is* the report, and it is the same call `crate::worker` makes for
/// `Query::AttachmentInventory`.
fn attachment_names(name: &str, path: &Path) -> Result<Vec<String>, String> {
    let sinks = MemorySinks::new();
    let plan = Plan::Attachments(AttachmentsPlan {
        source: 0,
        action: Action::List,
    });
    let report = apply(
        &plan,
        &[source(name, path)?],
        &sinks,
        &Policy::default(),
        &budget(),
    )
    .map_err(|why| why.to_string())?;
    Ok(report
        .listed
        .into_iter()
        .filter_map(|listed| match listed {
            pdf_transform::Listed::Attachment(entry) => Some(entry.name),
            pdf_transform::Listed::Image(_) => None,
        })
        .collect())
}

/// `meta/`: §14.3.3's entries, §14.3.2's packet and §12.3.3's outline.
///
/// The three files whose composition lives in `crate::worker` rather than in a plan, so the
/// second tree — the same document, the *other* transport — is what each is compared against,
/// and the model's own readers are what the values are then held to.
fn meta(vfs: &Vfs, here: &Vfs, local: &mut Local, path: &Path, name: &str) {
    let Ok(listed) = listing(vfs, "/meta").inspect_err(|why| local.refused.push(why.clone()))
    else {
        return;
    };
    local.directories = local.directories.saturating_add(1);
    for wanted in ["info.json", "outline.json"] {
        if !listed.iter().any(|entry| entry.name == wanted) {
            local
                .listing_failed
                .push(format!("/meta: does not list {wanted}"));
        }
    }
    let states_xmp = listed.iter().any(|entry| entry.name == "xmp.xml");

    for entry in &listed {
        let at = format!("/meta/{}", entry.name);
        // The other transport's answer is the expectation, which makes this the one place the
        // two are asked the same question directly.
        let expected = here
            .open(&at)
            .map(|handle| handle.bytes().to_vec())
            .map_err(|error| error.to_string());
        if let (Ok(mine), Err(why)) = (vfs.open(&at), &expected) {
            local.transport_differ.push(format!(
                "{at}: the confined worker answered {} bytes and this process refused: {why}",
                mine.len()
            ));
        }
        one_file(vfs, local, "/meta/NAME", &at, &expected);
    }

    let Ok(document) = Document::open_with_password(
        std::fs::read(path).unwrap_or_default(),
        Limits::DEFAULT,
        password_for(name),
    ) else {
        return;
    };
    // §14.3.3, entry by entry: what the file states is what the model's reader answers.
    if let Ok(handle) = vfs.open("/meta/info.json") {
        let stated = String::from_utf8_lossy(handle.bytes()).into_owned();
        let information = pdf_model::metadata::Information::read(&document);
        let values = [
            Value::optional(information.title.clone()),
            Value::optional(information.author.clone()),
            Value::optional(information.subject.clone()),
            Value::optional(information.keywords.clone()),
            Value::optional(information.creator.clone()),
            Value::optional(information.producer.clone()),
            Value::optional(information.created.clone()),
            Value::optional(information.modified.clone()),
            Value::text(match information.trapped {
                pdf_model::metadata::Trapped::Fully => "True",
                pdf_model::metadata::Trapped::NotYet => "False",
                pdf_model::metadata::Trapped::Unknown => "Unknown",
            }),
        ];
        for (key, value) in INFORMATION_KEYS.iter().zip(values.iter()) {
            let line = format!("\"{key}\": {}", value.render().trim_end());
            if !stated.contains(&line) {
                local.differ.push(format!(
                    "/meta/info.json: does not state {line} — §14.3.3's entry as \
                     `pdf_model::metadata` reads it"
                ));
            }
        }
    }
    // §14.3.2's stream is the one file whose existence is the document's to state.
    let packet = document
        .catalog()
        .ok()
        .map(|catalog| document.get_key(&catalog, "Metadata"))
        .and_then(|object| object.as_stream().cloned())
        .and_then(|stream| {
            document
                .decoded_stream_data(&stream)
                .map(|bytes| bytes.to_vec())
        });
    match (states_xmp, packet) {
        (true, Some(expected)) => {
            if vfs
                .open("/meta/xmp.xml")
                .is_ok_and(|handle| handle.bytes() != expected.as_slice())
            {
                local.differ.push(String::from(
                    "/meta/xmp.xml: not the catalog's own /Metadata stream, decoded",
                ));
            }
        }
        (true, None) => local.listing_failed.push(String::from(
            "/meta: lists xmp.xml where the catalog states no /Metadata stream",
        )),
        (false, Some(_)) => local.listing_failed.push(String::from(
            "/meta: does not list xmp.xml where the catalog states a /Metadata stream",
        )),
        (false, None) => {}
    }
}

/// The second pass: the listings again, then every file again.
///
/// Two claims, and neither of them is a clock (round 911 measured the regression it is about
/// with one, and `Vfs::generated` is what actually discriminates):
///
/// - **A second `stat` of a file this generation has already produced generates nothing.** ADR
///   0865 section 3 put the sizes in the cache past eviction for exactly this, and a corpus is
///   where a document too large for the cache's budget is found.
/// - **A second reading is the first reading.** RFC 0003 section 5.4's generation key is asked
///   before every answer, so an unchanged file answers the same bytes.
fn again(vfs: &Vfs, local: &mut Local) {
    let before = vfs.generated();
    let paths: Vec<String> = local.seen.keys().cloned().collect();
    for path in &paths {
        match vfs.stat(path) {
            Ok(attributes) => {
                if attributes.size != local.seen.get(path).map(|(size, _)| *size) {
                    local.size_failed.push(format!(
                        "{path}: the second stat says {:?} and the first said {:?}",
                        attributes.size,
                        local.seen.get(path).map(|(size, _)| *size)
                    ));
                }
            }
            Err(error) => local.size_failed.push(format!(
                "{path}: read once and does not stat again: {error}"
            )),
        }
    }
    let after = vfs.generated();
    if after > before {
        local.regenerated.push(format!(
            "a second stat of {} files this generation had already produced generated {} of them \
             again",
            paths.len(),
            after.saturating_sub(before)
        ));
    }
    for path in &paths {
        match vfs.open(path) {
            Ok(handle) => {
                let now = (handle.len(), digest(handle.bytes()));
                if local.seen.get(path) != Some(&now) {
                    local
                        .unstable
                        .push(format!("{path}: read twice, two answers"));
                }
            }
            Err(error) => local.unstable.push(format!(
                "{path}: read once and refused the second time: {error}"
            )),
        }
    }
}

/// One document of the population through the walk.
fn examine(chosen: &Chosen, tally: &Mutex<Tally>) {
    let began = Instant::now();
    let (name, path) = (chosen.name.as_str(), chosen.path.as_path());
    let vfs = mounted(path, name, Transport::Confined);
    let mut local = Local::default();
    match vfs.pages() {
        Ok(count) => {
            local.walked = true;
            local.pageless = count == 0;
            let here = mounted(path, name, Transport::Here);
            let bounds = bounds(chosen);
            root(&vfs, &mut local);
            pages(&vfs, &mut local, name, path, count, bounds);
            renders(&vfs, &mut local, name, path, count, bounds);
            images(&vfs, &mut local, name, path, count, bounds);
            text(&vfs, &mut local, path, name, count, bounds);
            attachments(&vfs, &mut local, name, path);
            meta(&vfs, &here, &mut local, path, name);
            again(&vfs, &mut local);
        }
        Err(error) => local.refused_open = Some(error.to_string()),
    }
    // Session 902's recovery, asked *after* the walk so that a death anywhere in it is followed
    // by a question the mount still has to answer. What is asked is that the answer is not a
    // corpse rather than that it is a page count: a locked document, an unopenable one and an
    // encryption this reader does not implement each answer `Err` here for ever and are right to
    // (trap 11, and session 917 got this wrong the first time).
    local.recovered = vfs
        .pages()
        .err()
        .is_none_or(|error| !is_a_death(&error.to_string()));
    local.took = began.elapsed();
    merge(chosen, local, tally);
}

/// One document's own tally folded into the run's, under its classes and with its deaths.
fn merge(chosen: &Chosen, local: Local, tally: &Mutex<Tally>) {
    let deaths: Vec<String> = local
        .sentences()
        .filter(|sentence| is_a_death(sentence))
        .cloned()
        .collect();
    let refusals = local
        .refused
        .len()
        .saturating_add(usize::from(local.refused_open.is_some()));
    let name = chosen.display.clone();
    record(tally, |t| {
        for class in &chosen.classes {
            let row = t.classes.entry(*class).or_default();
            row.documents = row.documents.saturating_add(1);
            row.read = row.read.saturating_add(local.read);
            row.bytes = row.bytes.saturating_add(local.bytes);
            row.refused = row.refused.saturating_add(refusals);
            row.killed = row.killed.saturating_add(deaths.len());
        }
        for sentence in deaths {
            t.killed.push((name.clone(), sentence));
        }
        if !local.recovered {
            t.unrecovered.push(name.clone());
        }
        if let Some(why) = local.refused_open {
            t.refused_open.push((name.clone(), why));
        }
        t.took.push((name.clone(), local.took.as_secs_f64()));
        t.walked = t.walked.saturating_add(usize::from(local.walked));
        t.pageless = t.pageless.saturating_add(usize::from(local.pageless));
        t.directories = t.directories.saturating_add(local.directories);
        t.statted = t.statted.saturating_add(local.statted);
        t.read = t.read.saturating_add(local.read);
        t.bytes = t.bytes.saturating_add(local.bytes);
        t.both_refused = t.both_refused.saturating_add(local.both_refused);
        t.pages_not_read = t.pages_not_read.saturating_add(local.pages_not_read);
        for (row, count) in local.matched {
            let held = t.matched.entry(row).or_default();
            *held = held.saturating_add(count);
        }
        for (column, entries) in [
            (&mut t.refused, local.refused),
            (&mut t.differ, local.differ),
            (&mut t.listing_failed, local.listing_failed),
            (&mut t.size_failed, local.size_failed),
            (&mut t.unstable, local.unstable),
            (&mut t.regenerated, local.regenerated),
            (&mut t.transport_differ, local.transport_differ),
        ] {
            for why in entries {
                column.push((name.clone(), why));
            }
        }
    });
}

/// Mebibytes, without a cast the workspace's lints would have to be argued with.
fn mib(bytes: u64) -> f64 {
    f64::from(u32::try_from(bytes >> 10).unwrap_or(u32::MAX)) / 1024.0
}

/// Prints one census list, capped, with its length.
fn print_list(what: &str, entries: &[(String, String)]) {
    println!("vfs-read:   {what}: {}", entries.len());
    for (name, why) in entries.iter().take(40) {
        println!("    {name}: {why}");
    }
    if entries.len() > 40 {
        println!("    … and {} more", entries.len().saturating_sub(40));
    }
}

/// The walk.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the census: every column of the tally printed and then asserted on, in one place, \
              so that what the run says and what fails it cannot drift apart"
)]
#[ignore = "corpus-scale: the whole layout listed, stat'd and read for every document, through the confined worker; run explicitly under the gates profile"]
fn every_corpus_document_reads_as_the_layouts_own_generators_do() {
    require_the_sandbox();
    // Before anything reads a document, and for the life of the process: see the module comment.
    // A confined worker has no filesystem, so its substitutions come from the compiled-in faces,
    // and an expectation computed here with the machine's fonts would differ from the tree's
    // answer for a reason that is not the tree's.
    pdf_font::substitute::no_machine_fonts();
    let started = Instant::now();
    let (documents, contributions) = chosen();
    if documents.is_empty() {
        println!("skipped: no corpus root on this disk holds a document");
        return;
    }

    let tally = Mutex::new(Tally::default());
    documents.par_iter().for_each(|document| {
        let name = document.display.clone();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            examine(document, &tally);
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
        "vfs-read: {} documents in {:.1}s, {} threads, confined transport, {PAGES_READ} pages a \
         doc/pdf.js document and {PAGES_SAMPLED} of every other root's",
        documents.len(),
        elapsed.as_secs_f64(),
        rayon::current_num_threads()
    );
    for contribution in &contributions {
        println!(
            "vfs-read:   {}: {} classified, {} walked",
            contribution.root, contribution.classified, contribution.chosen
        );
    }
    print_list("refused open", &tally.refused_open);
    println!(
        "vfs-read:   documents walked: {}, with no page: {}",
        tally.walked, tally.pageless
    );
    println!(
        "vfs-read:   directories listed: {}, entries stat'd: {}, files read: {} ({:.1} MiB)",
        tally.directories,
        tally.statted,
        tally.read,
        mib(tally.bytes)
    );
    for (row, count) in &tally.matched {
        println!("vfs-read:   {row}: {count} files are their own generator's bytes");
    }
    println!(
        "vfs-read:   the tree and the generator both refused: {}, pages past the ceiling: {}",
        tally.both_refused, tally.pages_not_read
    );
    print_list("refused by name", &tally.refused);
    print_list("not the generator's bytes", &tally.differ);
    print_list("the listing is not the layout's", &tally.listing_failed);
    print_list("a stat that is not the bytes", &tally.size_failed);
    print_list("read twice, two answers", &tally.unstable);
    print_list("a second stat generated again", &tally.regenerated);
    print_list("the two transports disagree", &tally.transport_differ);
    print_list("panicked", &tally.panicked);
    print_list("killed", &tally.killed);
    println!("vfs-read:   did not recover: {}", tally.unrecovered.len());

    let mut slowest = tally.took.clone();
    slowest.sort_by(|left, right| right.1.total_cmp(&left.1));
    println!("vfs-read: the five slowest documents, in seconds:");
    for (name, seconds) in slowest.iter().take(5) {
        println!("vfs-read:   {seconds:8.1}  {name}");
    }

    println!("vfs-read: by class, over the whole population:");
    for class in Class::ALL {
        match tally.classes.get(class) {
            None => println!("vfs-read:   {:<24} no document on this disk", class.name()),
            Some(row) => println!(
                "vfs-read:   {:<24} {} document(s), {} files read ({:.1} MiB), {} refused, \
                 {} killed",
                class.name(),
                row.documents,
                row.read,
                mib(row.bytes),
                row.refused,
                row.killed
            ),
        }
    }

    assert!(
        tally.panicked.is_empty(),
        "principle 1: no panic on any input"
    );
    assert!(
        tally.killed.is_empty(),
        "RFC 0003 section 6: the confined worker was killed by a signal on {} question(s), which \
         is what a face's user sees as a folder that stops answering:\n{}",
        tally.killed.len(),
        tally
            .killed
            .iter()
            .take(40)
            .map(|(name, why)| format!("    {name}: {why}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        tally.unrecovered.is_empty(),
        "ADR 0847: a mount did not answer after its walk, so a dead worker was not replaced: {:?}",
        tally.unrecovered
    );
    assert!(
        tally.listing_failed.is_empty(),
        "RFC 0003 section 4: a listing names what the document has, at the width the layout states"
    );
    assert!(
        tally.size_failed.is_empty(),
        "RFC 0003 section 5.5: a stat states the true size, because it generated the bytes"
    );
    assert!(
        tally.unstable.is_empty(),
        "RFC 0003 section 5.4: the generation key is asked before every answer, so an unchanged \
         document answers the same bytes"
    );
    assert!(
        tally.regenerated.is_empty(),
        "ADR 0865 section 3: a size outlives the bytes it was measured from, so a second listing \
         costs no generation"
    );
    assert!(
        tally.transport_differ.is_empty(),
        "ADR 0841 section 2: the confinement is a transport change and nothing else"
    );

    for (name, why) in &tally.differ {
        assert!(
            HELD.iter().any(|(held, _)| held == name),
            "a file that is not what its own generator produced, and nobody has read it: \
             {name}: {why}"
        );
    }
    for row in [
        "/pages/NNNN.pdf",
        "/renders/DPI/NNNN.png",
        "/images/NNNN/NAME",
        "/text/NNNN.txt",
        "/text/document.txt",
        "/attachments/NAME",
        "/meta/NAME",
    ] {
        assert!(
            tally.matched.get(row).is_some_and(|count| *count > 0),
            "no corpus document produced a file at {row}, so this walk measured nothing of it"
        );
    }
}
