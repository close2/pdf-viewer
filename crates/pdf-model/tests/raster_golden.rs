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
//! 0945 records a page drawn in the wrong place at the right weight for hundreds of sessions
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
//! cargo build --profile gates -p pdf-sandbox --bins     # trap 10: the worker decodes three filters
//! cargo test  --profile gates -p pdf-model --test raster_golden -- --ignored --nocapture
//! ```
//!
//! **To regenerate, after a change that moves pixels on purpose:**
//!
//! ```text
//! PDFVIEWER_RASTER_GOLDEN=update cargo test --profile gates -p pdf-model --test raster_golden -- --ignored --nocapture
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
//! parallel but each on one thread. Two consecutive `update` runs produce byte-identical files,
//! which is how the round that wrote this proved it (ADR 1016). What *is* a source of difference
//! is the sandbox worker — `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a
//! separate program, and a page whose image it could not decode holds no command for it — so this
//! gate refuses to measure without it, like every other corpus gate.
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

/// Pixels one page may cost, so that a malformed extent cannot exhaust this process. The same
/// figure `tests/corpus.rs` and `examples/raster_digest` use.
const PIXEL_BUDGET: u64 = 64 << 20;

/// The scale every corpus gate rasterises at: 72 dpi, one pixel per default user-space unit.
const SCALE: f32 = 1.0;

/// The environment variable that turns the check into a regeneration.
const UPDATE_VARIABLE: &str = "PDFVIEWER_RASTER_GOLDEN";

/// The corpus documents that refuse §7.6.4.1's default user password, with the password each
/// one's own pdf.js issue records — the same eight `pdf-syntax`'s `encryption.rs` verifies and
/// the accessibility census opens. A locked page has no raster to hold; with the password it has.
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

/// The password on record for one file, or the empty default §7.6.4.1 starts with.
fn password_for(path: &Path) -> &'static str {
    let name = path.file_name().map(|name| name.to_string_lossy());
    KNOWN_PASSWORDS
        .iter()
        .find(|(known, _)| name.as_deref() == Some(*known))
        .map_or("", |(_, password)| *password)
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
    let Ok(bytes) = std::fs::read(path) else {
        return refused(Outcome::Unreadable);
    };
    let document = match Document::open(bytes.clone()) {
        Ok(document) => document,
        Err(SyntaxError::PasswordRequired) => {
            match Document::open_with_password(bytes, Limits::default(), password_for(path)) {
                Ok(document) => document,
                Err(SyntaxError::PasswordRequired) => return refused(Outcome::Locked),
                Err(_) => return refused(Outcome::Unopened),
            }
        }
        Err(_) => return refused(Outcome::Unopened),
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
    match CpuRasterizer::new().rasterize(&interpretation.display_list, target) {
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
