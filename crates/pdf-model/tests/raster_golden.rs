//! A golden of this program's **own** output: the first page of every tracked corpus document,
//! rasterised by the CPU backend, digested, and held **by name** in `tests/raster_golden.tsv`.
//!
//! # What this is, and what it is not
//!
//! Every other corpus gate in this tree compares us with somebody else — poppler, mupdf,
//! ghostscript, pdf.js, hayro — and reaches a verdict with a tolerance in it. On the 43% of the
//! oracle's population it calls `ambiguous`, no reference agrees with any other closely enough for
//! anybody to be called wrong, so *nothing holds our pixels there*: an interpreter regression on
//! such a page is invisible unless it moves enough ink for the sweep's one-level alarm, and ADR
//! 0945 records a page drawn in the wrong place at the right weight and unnoticed
//! (`doc/reviews/984-direction-and-boundaries.md` Finding 4, ADR 1005 §4).
//!
//! This gate compares this program with **itself**. That is a change detector and not a
//! correctness claim, and `CLAUDE.md` principle 5 does not govern it: a digest that moves says
//! *these pages draw differently from the commit that last held them*, and nothing about which of
//! the two was right. The round that moved them says why, in the same commit, and the diff of the
//! file is the review — which is what `doc/traps/pixels-and-rasterisers.md` trap 1 already asks
//! every pixel round to do by eye.
//!
//! # Held by name, in both directions
//!
//! The file is ADR 0970's construction and not a count, for ADR 0970's reason: a count over a
//! population cannot tell a page that left from a page that was never there. So:
//!
//! - a page on disk whose digest **differs** from its entry **fails**, naming the page and which
//!   layer moved (below);
//! - a page on disk with **no entry** is **reported and does not fail** — a name that is absent
//!   proves nothing — so a corpus that grew, or a machine whose submodule is at a newer commit,
//!   passes and says what it could not hold;
//! - an entry whose page is **not on disk** is **reported and does not fail** — a name that leaves
//!   is examined rather than deleted quietly (ADR 0282's rule), and `update` removes it in the open.
//!
//! The population is the **tracked** corpus alone — `doc/pdf.js/test/pdfs`, a submodule pinned by
//! commit — and never the gitignored specifications in `doc/`, which come and go by machine and
//! are exactly the population ADR 0962 and ADR 0970 found moving a ratchet under it. The
//! denominator is printed (trap 25).
//!
//! # Three digests per page, so the diff says which layer moved
//!
//! Beside the raster, the display list's `Debug` rendering and the interpretation's reports are
//! digested too — the two things `examples/display_list_digest` hashes, for trap 37's reason: an
//! interpretation is what the program drew *and* what it said, and a change can move only the
//! sentence. Holding all three in one file costs a string per page and buys the diagnosis a
//! moved page needs first: a raster that moved under an unchanged list is a rasteriser change; a
//! list that moved is an interpreter change, whether or not a pixel followed; a report that moved
//! alone is a change in what this reader accuses a file of.
//!
//! # The digest is SHA-256, cut to sixteen hex digits, and the two examples' hash would not do
//!
//! `examples/raster_digest` and `examples/display_list_digest` hash with
//! `std::collections::hash_map::DefaultHasher`, which the standard library documents as
//! unspecified across releases — good enough for two arms run in one sitting, and disqualifying
//! for a file that is committed and read by a later toolchain. SHA-256 is already in this crate's
//! graph for §12.8.3. Sixteen hex digits are sixty-four bits, which is ample for *detecting a
//! difference* (the only question asked here), and the extent column beside them makes a lone
//! collision not a false pass.
//!
//! # Running it
//!
//! ```text
//! ulimit -u 8192; tools/bounded.sh --lock --round <session> --tree 12 \
//!     --build '--profile gates -p pdf-sandbox --bins' -- \
//!     cargo test --profile gates -p pdf-model --test raster_golden -- --ignored --nocapture
//! ```
//!
//! which is `tools/state.sh --round <session> golden`. The worker decodes three filters and Cargo
//! does not build another package's binary for a test (trap 10), so the walk's own `--build` makes
//! it inside the hold, as old as the walk rather than as the moment it stopped queueing (trap 109,
//! ADR 1710).
//!
//! **To regenerate, after a change that moves pixels on purpose:**
//!
//! ```text
//! ulimit -u 8192; PDFVIEWER_RASTER_GOLDEN=update tools/bounded.sh --lock \
//!     --round <session> --tree 12 --build '--profile gates -p pdf-sandbox --bins' -- \
//!     cargo test --profile gates -p pdf-model --test raster_golden -- --ignored --nocapture
//! ```
//!
//! which rewrites `tests/raster_golden.tsv` whole, sorted, and prints exactly which entries
//! changed and how — the same classification the check prints — so the round can name them in
//! its history file and commit the file with the change. An environment variable rather than a
//! flag because everything after `--` belongs to the test harness, which refuses a flag it does
//! not know.
//!
//! # Determinism
//!
//! The CPU rasteriser is deterministic for a given display list, and the interpreter is a pure
//! function of the document (`CLAUDE.md`'s immutable `Document`); the pages are rasterised in
//! parallel and each page whole — [`STRIPS`] is a count this file states rather than one the
//! machine supplies, because a page's pixels depend on its division (ADR 0219) and a count
//! asked of `available_parallelism` made the digests a fact about the cores the process was given
//! (ADR 1734, ADR 1742). Two consecutive `update` runs produce byte-identical files,
//! which is how the round that wrote this proved it (ADR 1016). What *is* a source of difference
//! is the sandbox worker — `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a
//! separate program, and a page whose image it could not decode holds no command for it — so this
//! gate refuses to measure without it, like every other corpus gate.
//!
//! # Beside it, how far a division moves the same pages
//!
//! How much a page depends on its division is a property of this backend claimed for every page,
//! and `strip_parallelism.rs` asserts it over three. So the second ignored test here draws each
//! first page again at [`DIVISIONS`] and holds it to that file's two bounds, or to a ceiling of its
//! own where [`PAST_THE_BOUND`] names the page and says what moves on it — trap 66's construction:
//! a claim about every page is held over the pages there are (ADR 1758). It rides the same walk;
//! with ADR 0219's defect or ADR 0138's put back it fails, naming 7 and 14 pages.
//!
//! # Calibrated both ways (trap 13)
//!
//! With one pixel of every drawn page inverted in a scratch build, every drawn page is named and
//! nothing else is; with `Medium::PAGE_ONLY`'s surround moved off white, exactly the pages whose
//! extent is not a whole number of pixels are named — the population `examples/raster_digest`
//! recorded — and none of the others. With one entry deleted from the file the run passes and
//! reports the page as unheld. ADR 1016 has the runs.

#![expect(
    clippy::panic,
    clippy::print_stdout,
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure, and the report is the \
              point of the run"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use pdf_model::{Pages, interpret};
use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::{Document, Limits, SyntaxError};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use render_cpu::CpuRasterizer;
use sha2::{Digest as _, Sha256};

#[path = "support/corpus_passwords.rs"]
#[expect(
    dead_code,
    reason = "the references' spelling of a password is the oracle's; this gate opens the \
              document itself and has no reference to hand one to"
)]
mod corpus_passwords;

/// Pixels one page may cost, so that a malformed extent cannot exhaust this process. The same
/// figure `tests/corpus.rs` and `examples/raster_digest` use.
const PIXEL_BUDGET: u64 = 64 << 20;

/// The scale every corpus gate rasterises at: 72 dpi, one pixel per default user-space unit.
const SCALE: f32 = 1.0;

/// How many strips each page is drawn in: one, the page undivided.
///
/// ADR 0219 decides which division a page's pixels are: the page is what is drawn and the division
/// is an implementation detail, so the answer is the one that exists when there is no division —
/// strips reproduce it up to `tiny-skia`'s arithmetic at a shifted origin, one supersample on an
/// edge that lands on a sample row. Where the caller states nothing,
/// `render_cpu::plan_strips` takes the count from `available_parallelism`, which is the CPUs a
/// process may use and not a property of the page (ADR 1742). The pages are already drawn in
/// parallel across the corpus, so what a page's own strips would add is little.
const STRIPS: u32 = 1;

/// The environment variable that turns the check into a regeneration.
const UPDATE_VARIABLE: &str = "PDFVIEWER_RASTER_GOLDEN";

/// What became of one page, in one word the file can hold.
///
/// A change of outcome is a change — a page that drew and now is refused, or the reverse — so
/// the word is held beside the digests rather than dropped as "not a raster".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// Opened, interpreted and rasterised.
    Drawn,
    /// The file could not be read from disk.
    Unreadable,
    /// `Document::open` refused it.
    Unopened,
    /// §7.6.4.1's default user password and the one on record both refused.
    Locked,
    /// The page tree has no first page.
    NoPage,
    /// `TargetSpec::for_page` refused the extent.
    NoTarget,
    /// The rasteriser refused the display list.
    Refused,
}

impl Outcome {
    const fn word(self) -> &'static str {
        match self {
            Self::Drawn => "drawn",
            Self::Unreadable => "unreadable",
            Self::Unopened => "unopened",
            Self::Locked => "locked",
            Self::NoPage => "no-page",
            Self::NoTarget => "no-target",
            Self::Refused => "refused",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        [
            Self::Drawn,
            Self::Unreadable,
            Self::Unopened,
            Self::Locked,
            Self::NoPage,
            Self::NoTarget,
            Self::Refused,
        ]
        .into_iter()
        .find(|outcome| outcome.word() == word)
    }
}

/// One line of the file: everything held about one page.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    outcome: Outcome,
    /// `width x height` of the raster, when there was one.
    extent: Option<(u32, u32)>,
    /// Digest of the raster's bytes, when there was one.
    raster: Option<String>,
    /// Digest of the display list's `Debug` rendering, when the page was interpreted.
    list: Option<String>,
    /// Digest of the interpretation's reports, when the page was interpreted.
    reports: Option<String>,
}

impl Entry {
    /// The line as the file holds it, without its key.
    fn to_line(&self) -> String {
        let extent = self
            .extent
            .map_or_else(|| "-".to_owned(), |(w, h)| format!("{w}x{h}"));
        let column = |digest: &Option<String>| digest.clone().unwrap_or_else(|| "-".to_owned());
        format!(
            "{}\t{extent}\t{}\t{}\t{}",
            self.outcome.word(),
            column(&self.raster),
            column(&self.list),
            column(&self.reports)
        )
    }

    /// The inverse of [`Self::to_line`]; `None` for a line the file's format does not admit.
    fn parse(line: &str) -> Option<Self> {
        let mut columns = line.split('\t');
        let outcome = Outcome::parse(columns.next()?)?;
        let extent = match columns.next()? {
            "-" => None,
            extent => {
                let (w, h) = extent.split_once('x')?;
                Some((w.parse().ok()?, h.parse().ok()?))
            }
        };
        let digest = |column: &str| (column != "-").then(|| column.to_owned());
        let raster = digest(columns.next()?);
        let list = digest(columns.next()?);
        let reports = digest(columns.next()?);
        if columns.next().is_some() {
            return None;
        }
        Some(Self {
            outcome,
            extent,
            raster,
            list,
            reports,
        })
    }
}

/// Which layer moved between two entries for one page — the sentence a round reads first.
///
/// The order matters: an outcome change subsumes the digests, a list change explains a raster
/// change, and a raster change under an unchanged list is the rasteriser's alone.
fn what_moved(was: &Entry, is: &Entry) -> Option<String> {
    if was == is {
        return None;
    }
    if was.outcome != is.outcome {
        return Some(format!(
            "outcome: was {}, is {}",
            was.outcome.word(),
            is.outcome.word()
        ));
    }
    let raster = was.raster != is.raster || was.extent != is.extent;
    let list = was.list != is.list;
    let reports = was.reports != is.reports;
    let mut sentence = match (raster, list) {
        (true, true) => "raster and list (an interpreter change)".to_owned(),
        (true, false) => "raster only (a rasteriser change under an unchanged list)".to_owned(),
        (false, true) => "list only (an interpreter change no pixel shows)".to_owned(),
        (false, false) => "reports only (a change in the diagnosis, trap 37)".to_owned(),
    };
    if reports && (raster || list) {
        sentence.push_str(", and the reports");
    }
    if was.extent != is.extent {
        let show = |extent: Option<(u32, u32)>| {
            extent.map_or_else(|| "-".to_owned(), |(w, h)| format!("{w}x{h}"))
        };
        let _ = write!(
            sentence,
            "; extent {} → {}",
            show(was.extent),
            show(is.extent)
        );
    }
    Some(sentence)
}

/// The sixteen hex digits this file holds of a SHA-256.
fn digest(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    let mut hex = String::with_capacity(16);
    for byte in &hash[..8] {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// The key one page is held under: `<file name> p1`, ADR 0970's shape.
fn key_for(path: &Path) -> String {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    format!("{name} p1")
}

/// The password `corpus_passwords` publishes for one file, or the empty default §7.6.4.1 starts
/// with. A locked page has no raster to hold; with the password it has.
fn password_for(path: &Path) -> &'static str {
    path.file_name()
        .and_then(|name| corpus_passwords::corpus_password(&name.to_string_lossy()))
        .map_or("", |known| known.password)
}

/// Opens one document with the empty default password, then with the one on record; the outcome
/// that refused it otherwise.
fn open(path: &Path) -> Result<Document, Outcome> {
    let bytes = std::fs::read(path).map_err(|_| Outcome::Unreadable)?;
    match Document::open(bytes.clone()) {
        Ok(document) => Ok(document),
        Err(SyntaxError::PasswordRequired) => {
            Document::open_with_password(bytes, Limits::default(), password_for(path)).map_err(
                |error| match error {
                    SyntaxError::PasswordRequired => Outcome::Locked,
                    _ => Outcome::Unopened,
                },
            )
        }
        Err(_) => Err(Outcome::Unopened),
    }
}

/// Opens, interprets and rasterises one document's first page, and digests all three.
fn examine(path: &Path) -> Entry {
    let refused = |outcome: Outcome| Entry {
        outcome,
        extent: None,
        raster: None,
        list: None,
        reports: None,
    };
    let document = match open(path) {
        Ok(document) => document,
        Err(outcome) => return refused(outcome),
    };
    let Some(page) = Pages::new(&document).get(0) else {
        return refused(Outcome::NoPage);
    };
    let interpretation = interpret(&document, &page);
    let list = Some(digest(
        format!("{:?}", interpretation.display_list).as_bytes(),
    ));
    let reports = Some(digest(
        format!("{:?}", interpretation.unsupported).as_bytes(),
    ));
    let interpreted = |outcome: Outcome| Entry {
        outcome,
        extent: None,
        raster: None,
        list: list.clone(),
        reports: reports.clone(),
    };
    let Ok(target) = TargetSpec::for_page(&interpretation.display_list, SCALE, PIXEL_BUDGET) else {
        return interpreted(Outcome::NoTarget);
    };
    match CpuRasterizer::new()
        .with_strips(STRIPS)
        .rasterize(&interpretation.display_list, target)
    {
        Ok(raster) => Entry {
            outcome: Outcome::Drawn,
            extent: Some((raster.width, raster.height)),
            raster: Some(digest(&raster.data)),
            list,
            reports,
        },
        Err(_) => interpreted(Outcome::Refused),
    }
}

/// Every tracked corpus document, sorted; `None` when the submodule is not checked out.
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

/// Where the golden lives: beside this file, in the crate's `tests/`, like
/// `ambiguous_undiagnosed.txt`.
fn golden_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/raster_golden.tsv")
}

/// The file's entries by key. `None` when there is no file; a line the format does not admit is a
/// failure, because a golden that cannot be read is a golden that holds nothing.
fn read_golden(path: &Path) -> Option<BTreeMap<String, Entry>> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut entries = BTreeMap::new();
    for (number, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, rest)) = line.split_once('\t') else {
            panic!(
                "{}:{}: no key column",
                path.display(),
                number.saturating_add(1)
            );
        };
        let Some(entry) = Entry::parse(rest) else {
            panic!(
                "{}:{}: not a line this gate wrote: {line:?}",
                path.display(),
                number.saturating_add(1)
            );
        };
        assert!(
            entries.insert(key.to_owned(), entry).is_none(),
            "{}:{}: {key:?} is held twice",
            path.display(),
            number.saturating_add(1)
        );
    }
    Some(entries)
}

/// Writes the file whole: a header saying what it is, then one sorted line per page.
fn write_golden(path: &Path, entries: &BTreeMap<String, Entry>) {
    let mut text = String::from(
        "# The first page of every tracked corpus document, as this program draws it — held by \
         name.\n\
         #\n\
         # Written by `tests/raster_golden.rs`, and read by it: a page whose line differs fails \
         the gate\n\
         # naming the page and which layer moved; a page absent from here is reported and passes; \
         a line\n\
         # whose page is not on disk is reported and passes. This compares the program with \
         *itself* —\n\
         # a change detector, not a correctness claim — so a round that changes pixels on purpose\n\
         # regenerates it and commits it with the change:\n\
         #\n\
         #     PDFVIEWER_RASTER_GOLDEN=update cargo test --profile gates -p pdf-model --test \
         raster_golden -- --ignored --nocapture\n\
         #\n\
         # Columns: key, outcome, raster extent, then sixteen hex digits of a SHA-256 over the \
         raster's\n\
         # bytes, over the display list's Debug rendering, and over the interpretation's reports; \
         `-`\n\
         # where the page produced none. ADR 1016 has the argument.\n\
         \n",
    );
    for (key, entry) in entries {
        let _ = writeln!(text, "{key}\t{}", entry.to_line());
    }
    std::fs::write(path, text).expect("the golden file can be written beside its test");
}

/// Refuses to measure without the sandboxed decoder — trap 10, ADR 0557 — for the reason every
/// corpus gate does: a page whose JBIG2, JPX or CCITT image the worker did not decode holds no
/// command for it, so a digest taken without the worker is a digest of the build and not of the
/// tree, and it would fail every such page against a file written with it.
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so the digests below would be of the \
             build rather than of the tree: {error}"
        );
    }
}

/// The gate.
#[test]
#[ignore = "the whole tracked corpus rasterised; run explicitly, under the gates profile"]
fn the_first_page_of_every_tracked_document_draws_what_it_drew() {
    require_the_sandbox();
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let updating = std::env::var_os(UPDATE_VARIABLE).is_some_and(|value| value == "update");
    let path = golden_path();
    let held = read_golden(&path);
    assert!(
        held.is_some() || updating,
        "{} is not there, so nothing is held: write it with {UPDATE_VARIABLE}=update",
        path.display()
    );
    let held = held.unwrap_or_default();

    let started = Instant::now();
    let measured: BTreeMap<String, Entry> = files
        .par_iter()
        .map(|path| (key_for(path), examine(path)))
        .collect();
    let seconds = started.elapsed().as_secs_f64();

    // The three classes, each by name. A page that is on disk and in the file with the same line
    // is *held* and needs no sentence.
    let mut moved: Vec<(&str, String)> = Vec::new();
    let mut unheld: Vec<&str> = Vec::new();
    let mut held_count = 0_usize;
    for (key, is) in &measured {
        match held.get(key) {
            None => unheld.push(key),
            Some(was) => match what_moved(was, is) {
                Some(sentence) => moved.push((key, sentence)),
                None => held_count = held_count.saturating_add(1),
            },
        }
    }
    let left: Vec<&str> = held
        .keys()
        .filter(|key| !measured.contains_key(*key))
        .map(String::as_str)
        .collect();
    let drawn = measured
        .values()
        .filter(|entry| entry.outcome == Outcome::Drawn)
        .count();

    println!(
        "{} tracked documents on disk, {drawn} first pages drawn, {} entries in {} — {seconds:.1} s",
        files.len(),
        held.len(),
        path.display()
    );
    println!(
        "held {held_count}, moved {}, unheld {}, left {}",
        moved.len(),
        unheld.len(),
        left.len()
    );
    for (key, sentence) in &moved {
        println!("  moved: {key}: {sentence}");
    }
    for key in &unheld {
        println!("  unheld: {key} (on disk, not in the file — proves nothing)");
    }
    for key in &left {
        println!("  left: {key} (in the file, not on disk — examine it before it goes)");
    }

    if updating {
        write_golden(&path, &measured);
        println!(
            "{} rewritten: {} entries; the {} moved, {} joined and {} removed above are the diff \
             to review and commit with the change that made it",
            path.display(),
            measured.len(),
            moved.len(),
            unheld.len(),
            left.len()
        );
        return;
    }
    assert!(
        moved.is_empty(),
        "{} first pages draw differently from the commit that last held them (named above). If \
         that is the change this round intends, regenerate with {UPDATE_VARIABLE}=update, read the \
         list it prints, and commit the file with the change",
        moved.len()
    );
}

/// The divisions every first page is drawn in beside the undivided one: from the fewest a machine
/// can cut a page into to [`render_cpu::MAX_STRIPS`], which is what [`CpuRasterizer::new`] asks for
/// on any machine of at least sixteen CPUs and what the six arms' oracle states (ADR 1742).
///
/// Four rather than [`render_cpu::MAX_STRIPS`] alone because the population's worst is not at
/// sixteen: `blendmode.pdf` moves three levels at two, four and eight strips and two at sixteen
/// (ADR 1758).
const DIVISIONS: [u32; 4] = [2, 4, 8, render_cpu::MAX_STRIPS];

/// Most one pixel may move between the page drawn whole and the page drawn in strips, on a page
/// not named in [`PAST_THE_BOUND`]: one level of 255, `strip_parallelism.rs`'s bound.
///
/// It is derived for **one** coverage: an `ulp` of `ty` moves an exact converter's area by far less
/// than a level, so it can cross one rounding step and never two. A pixel that composites the
/// rounded results of several marks, or blends one through a function steeper than one, is not
/// one coverage, and ADR 1758 measures what that costs over the corpus.
const ONE_LEVEL: u8 = 1;

/// Most pixels that may move at all on a page not named in [`PAST_THE_BOUND`], as one in this
/// many: `strip_parallelism.rs`'s bound.
const RARE: usize = 1_000;

/// The first pages that pass one of the two bounds above, each read with its moved pixels painted
/// on the page: the key, the most pixels it moves at any of [`DIVISIONS`], the most levels any of
/// them moves, and what the moved pixels are.
///
/// Held as ceilings rather than as digests: a page here that moves more fails, and a page that
/// falls back inside both bounds is reported as able to leave. ADR 1758 has the measurement.
const PAST_THE_BOUND: [(&str, usize, u8, &str); 8] = [
    (
        "blendmode.pdf p1",
        83,
        3,
        "single pixels inside the photographs blended under eleven of the sixteen modes: one \
         photograph's sample moved one level by the shifted origin, through the blend function's \
         slope at that pixel, which reaches three under ColorDodge and Hue — \
         `strip_parallelism.rs` holds the sample to its one level (ADR 1768)",
    ),
    (
        "comments.pdf p1",
        113,
        2,
        "single glyph pixels under the highlight annotations' Multiply composite",
    ),
    (
        "highlights.pdf p1",
        112,
        2,
        "single glyph pixels under the highlight annotations' Multiply composite",
    ),
    (
        "issue12810.pdf p1",
        249,
        2,
        "the two levels where a diagonal stroke meets a vertical one: two marks' edges in a pixel",
    ),
    (
        "issue1350.pdf p1",
        1078,
        1,
        "five rows of the form's three boxes, each horizontal edge on a sample row",
    ),
    (
        "issue7014.pdf p1",
        327,
        2,
        "row 369, where the underline's bar ends and the highlight below it is clipped: two marks' \
         edges in one pixel row",
    ),
    (
        "issue7020.pdf p1",
        102,
        1,
        "the horizontal tops of the Kannada glyphs, on sample rows",
    ),
    (
        "pdfjs_wikipedia.pdf p1",
        713,
        1,
        "the heading rules and glyph tops, on sample rows",
    ),
];

/// How far one first page moves when it is divided: the most pixels and the most levels over
/// [`DIVISIONS`], and the division and first pixel of the worst, for the failure's sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Division {
    /// Pixels on the page.
    pixels: usize,
    /// Most pixels that differ from the undivided page at any one division.
    moved: usize,
    /// Most levels any byte moves at any division.
    worst: u8,
    /// The division that moved most pixels, and the first of them as `(x, y)`.
    first: Option<(u32, u32, u32)>,
    /// A division the rasteriser refused although it drew the page whole.
    refused: Option<u32>,
}

/// One first page drawn whole and at every division, compared byte for byte; `None` when the
/// document gives the rasteriser no page to draw, which [`examine`] names.
fn divided(path: &Path) -> Option<Division> {
    let document = open(path).ok()?;
    let page = Pages::new(&document).get(0)?;
    let list = interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, SCALE, PIXEL_BUDGET).ok()?;
    let drawn = |strips: u32| {
        CpuRasterizer::new()
            .with_strips(strips)
            .rasterize(&list, target)
            .ok()
    };
    let whole = drawn(STRIPS)?;
    let mut division = Division {
        pixels: whole.data.len() / 4,
        moved: 0,
        worst: 0,
        first: None,
        refused: None,
    };
    for strips in DIVISIONS {
        let Some(split) = drawn(strips) else {
            division.refused = division.refused.or(Some(strips));
            continue;
        };
        let mut moved = 0_usize;
        let mut first = None;
        for (at, (ours, theirs)) in whole
            .data
            .chunks_exact(4)
            .zip(split.data.chunks_exact(4))
            .enumerate()
        {
            if ours == theirs {
                continue;
            }
            moved = moved.saturating_add(1);
            first = first.or(Some(at));
            for (one, other) in ours.iter().zip(theirs) {
                division.worst = division.worst.max(one.abs_diff(*other));
            }
        }
        if moved > division.moved {
            division.moved = moved;
            division.first = first.and_then(|at| {
                let width = usize::try_from(whole.width).ok()?;
                let x = u32::try_from(at.checked_rem(width)?).ok()?;
                let y = u32::try_from(at.checked_div(width)?).ok()?;
                Some((strips, x, y))
            });
        }
    }
    Some(division)
}

/// Every first page of the tracked corpus drawn in strips is the page drawn whole within
/// `strip_parallelism.rs`'s two bounds, or is a page [`PAST_THE_BOUND`] names and within its own
/// ceilings — trap 66's construction for strips: a property claimed of every page is held over
/// the pages there are, not over three fixtures (ADR 1758).
#[test]
#[ignore = "the whole tracked corpus rasterised five times; run explicitly, under the gates profile"]
fn every_first_page_drawn_in_strips_is_the_page_drawn_whole_within_the_bound() {
    require_the_sandbox();
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let started = Instant::now();
    let measured: BTreeMap<String, Option<Division>> = files
        .par_iter()
        .map(|path| (key_for(path), divided(path)))
        .collect();
    let seconds = started.elapsed().as_secs_f64();
    let past: BTreeMap<&str, (usize, u8)> = PAST_THE_BOUND
        .iter()
        .map(|(key, moved, worst, _)| (*key, (*moved, *worst)))
        .collect();

    let drawn: Vec<(&str, Division)> = measured
        .iter()
        .filter_map(|(key, division)| division.map(|division| (key.as_str(), division)))
        .collect();
    let mut failures: Vec<String> = Vec::new();
    for (key, division) in &drawn {
        let sentence = || {
            let (strips, x, y) = division.first.unwrap_or_default();
            format!(
                "{key}: {} of {} pixels moved, worst {} levels; most at {strips} strips, the first \
                 at ({x}, {y})",
                division.moved, division.pixels, division.worst
            )
        };
        if let Some(strips) = division.refused {
            failures.push(format!(
                "{key}: drawn whole and refused at {strips} strips — a division may not change \
                 whether a page is drawn"
            ));
            continue;
        }
        let within_the_bound =
            division.worst <= ONE_LEVEL && division.moved.saturating_mul(RARE) <= division.pixels;
        match past.get(key) {
            None if !within_the_bound => failures.push(format!(
                "{} — past one level or one pixel in {RARE}, and not a page PAST_THE_BOUND names",
                sentence()
            )),
            Some(&(moved, worst)) if division.moved > moved || division.worst > worst => failures
                .push(format!(
                    "{} — past its own ceiling of {moved} pixels and {worst} levels",
                    sentence()
                )),
            Some(_) if within_the_bound => println!(
                "  can leave PAST_THE_BOUND: {} — inside both bounds now",
                sentence()
            ),
            _ => {}
        }
    }
    for key in past.keys() {
        if !measured.get(*key).is_some_and(Option::is_some) {
            println!("  left: {key} is named in PAST_THE_BOUND and was not drawn");
        }
    }

    let moved_pages = drawn.iter().filter(|(_, d)| d.moved > 0).count();
    let worst = drawn.iter().map(|(_, d)| d.worst).max().unwrap_or(0);
    let most = drawn.iter().map(|(_, d)| d.moved).max().unwrap_or(0);
    println!(
        "{} first pages drawn whole and at {DIVISIONS:?} strips — {seconds:.1} s: {moved_pages} \
         move at some division, at most {most} pixels and {worst} levels; {} named past the bound",
        drawn.len(),
        past.len()
    );
    for failure in &failures {
        println!("  past: {failure}");
    }
    assert!(
        failures.is_empty(),
        "{} first pages move further between one strip and many than the bound admits (named \
         above): a chopped path or a misplaced strip moves sixteen levels and more (ADR 0138), and \
         what a shifted origin costs is ADR 0219's and ADR 1758's",
        failures.len()
    );
}

/// A line survives the round trip through the file's format, for every outcome.
#[test]
fn a_line_round_trips_through_the_file_format() {
    let drawn = Entry {
        outcome: Outcome::Drawn,
        extent: Some((612, 792)),
        raster: Some("0123456789abcdef".to_owned()),
        list: Some("fedcba9876543210".to_owned()),
        reports: Some("00000000ffffffff".to_owned()),
    };
    let refused = Entry {
        outcome: Outcome::Refused,
        extent: None,
        raster: None,
        list: Some("fedcba9876543210".to_owned()),
        reports: Some("00000000ffffffff".to_owned()),
    };
    let unopened = Entry {
        outcome: Outcome::Unopened,
        extent: None,
        raster: None,
        list: None,
        reports: None,
    };
    for entry in [drawn, refused, unopened] {
        assert_eq!(Entry::parse(&entry.to_line()).as_ref(), Some(&entry));
    }
    assert_eq!(Entry::parse("drawn\t1x1\tabc"), None, "too few columns");
    assert_eq!(Entry::parse("painted\t-\t-\t-\t-"), None, "no such outcome");
}

/// The classification names the layer that moved, and the order of precedence holds.
#[test]
fn the_classification_names_the_layer_that_moved() {
    let base = Entry {
        outcome: Outcome::Drawn,
        extent: Some((612, 792)),
        raster: Some("aaaaaaaaaaaaaaaa".to_owned()),
        list: Some("bbbbbbbbbbbbbbbb".to_owned()),
        reports: Some("cccccccccccccccc".to_owned()),
    };
    assert_eq!(what_moved(&base, &base), None);

    let raster_only = Entry {
        raster: Some("dddddddddddddddd".to_owned()),
        ..base.clone()
    };
    assert!(
        what_moved(&base, &raster_only).is_some_and(|sentence| sentence.starts_with("raster only")),
        "a raster change under an unchanged list is the rasteriser's"
    );

    let both = Entry {
        list: Some("eeeeeeeeeeeeeeee".to_owned()),
        ..raster_only.clone()
    };
    assert!(
        what_moved(&base, &both).is_some_and(|sentence| sentence.starts_with("raster and list")),
        "a raster change under a moved list is the interpreter's"
    );

    let reports_only = Entry {
        reports: Some("ffffffffffffffff".to_owned()),
        ..base.clone()
    };
    assert!(
        what_moved(&base, &reports_only)
            .is_some_and(|sentence| sentence.starts_with("reports only")),
        "a change in the diagnosis alone is trap 37's, and it is seen"
    );

    let refused = Entry {
        outcome: Outcome::Refused,
        extent: None,
        raster: None,
        ..base.clone()
    };
    assert_eq!(
        what_moved(&base, &refused).as_deref(),
        Some("outcome: was drawn, is refused")
    );
}
