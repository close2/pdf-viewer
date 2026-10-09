//! RFC 0008 section 6.7's `script_corpus` gate in its Tier 0 form: every field script the census
//! population carries, committed once, and every field's displayed value held **by name**.
//!
//! # What this is
//!
//! Tier 0 runs the `AF*` library natively where a field's `/AA` script is one call with literal
//! arguments (`pdf_model::aform`, ADRs 1578 and 1579). Its output is a *value* — the text a field
//! displays — so this gate is a change detector over values, in `raster_golden`'s discipline: a
//! golden of this program's own output, `tests/script_corpus.tsv`, held by name in both
//! directions (ADR 0970). It is not a correctness claim — RFC 0008 section 8 item 5 says so: the
//! library is documented choice from end to end, and a value that changes is a diff to read.
//!
//! # The population, derived rather than written down
//!
//! RFC 0008 section 3's census walked every corpus `tools/state.sh` names plus the two crawls under
//! `corpus-cache/`, and that is this gate's population: `doc/pdf.js/test/pdfs` (one level),
//! `doc/corpora`, and `corpus-cache/`'s `openpreserve`, `tika-issue-tracker` and `safedocs`, each
//! walked for every `.pdf` on disk. A document is *in* the gate where one of its fields states a
//! Table 199 `/K`, `/F`, `/V` or `/C` ECMAScript action — the census's own four field sites — so
//! the gate's population is the census's field-script documents, found the way the census found
//! them, and printed (trap 25). A corpus that is not on this machine is said to be missing, not
//! counted as empty.
//!
//! # What one document's line holds
//!
//! For every field with a script at one of the four sites, a synthetic commit of the value it
//! already holds: [`ViewState::set_field`] with its own text (Table 199's `/K` in its typing form,
//! then Table 224's `/CO` walked), then [`ViewState::commit_field`] (`/K`'s commit form, then
//! `/V`, then `/CO` again). Then every field's [`ViewState::displayed_value`] — `/F` applied —
//! digested in name order. Beside the digest, five counts, each a column so that a change in one
//! is visible as itself: fields, sites Tier 0 runs, sites it reports as not run, fields whose own
//! value the commit did not take, and commits a script refused.
//!
//! # Running it
//!
//! ```text
//! tools/bounded.sh --lock --round <session> --data 8 --tree 12 -- \
//!     cargo test --profile gates -p pdf-model --test script_corpus -- --ignored --nocapture
//! PDFVIEWER_SCRIPT_CORPUS=update …   # rewrites tests/script_corpus.tsv and prints what moved
//! ```

// no sandbox worker: the walk reads field dictionaries, sets and commits field values and asks what each displays; no content stream is interpreted, so no image reaches `pdf-sandbox` (`tools/conformance/tests/sandbox_gates.rs`).

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

use pdf_model::aform::Trigger;
use pdf_model::aform::site::{self, Site};
use pdf_model::view::{Committed, Entered, ViewState, widgets_by_field_name};
use pdf_syntax::{Document, Limits};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use sha2::{Digest as _, Sha256};

#[path = "support/corpus_passwords.rs"]
#[expect(
    dead_code,
    reason = "the references' spelling of a password is the oracle's; this gate opens the \
              document itself and has no reference to hand one to"
)]
mod corpus_passwords;

#[path = "support/script_population.rs"]
mod script_population;

use script_population::{MAX_FILE_BYTES, password_for, population, repository};

/// The environment variable that turns the check into a regeneration.
const UPDATE_VARIABLE: &str = "PDFVIEWER_SCRIPT_CORPUS";

/// Table 199's four triggers, in the order a line counts them.
const TRIGGERS: [Trigger; 4] = [
    Trigger::Keystroke,
    Trigger::Format,
    Trigger::Validate,
    Trigger::Calculate,
];

/// One document's line.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Line {
    /// Fields the document has.
    fields: usize,
    /// Sites whose script is one call Tier 0 runs.
    run: usize,
    /// Sites whose script Tier 0 reports as not run.
    not_run: usize,
    /// Fields whose own value the synthetic commit did not take.
    untaken: usize,
    /// Commits a script refused.
    refused: usize,
    /// SHA-256 over every field's name and displayed value, cut to sixteen hex digits.
    digest: String,
}

impl Line {
    /// The tab-separated columns after the key.
    fn to_columns(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            self.fields, self.run, self.not_run, self.untaken, self.refused, self.digest
        )
    }

    /// Reads the columns back.
    fn parse(columns: &str) -> Option<Self> {
        let mut parts = columns.split('\t');
        let mut number = || parts.next()?.parse::<usize>().ok();
        let line = Self {
            fields: number()?,
            run: number()?,
            not_run: number()?,
            untaken: number()?,
            refused: number()?,
            digest: String::new(),
        };
        let digest = columns.rsplit('\t').next()?.to_owned();
        Some(Self { digest, ..line })
    }
}

/// What examining one document found.
enum Examined {
    /// No field states a script at a Table 199 site: outside the gate.
    Outside,
    /// The document could not be opened, or was over the size bound.
    Unopened,
    /// The document's line.
    Held(Line),
}

/// Opens one document and commits every scripted field once.
fn examine(path: &Path) -> Examined {
    if std::fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_FILE_BYTES) {
        return Examined::Unopened;
    }
    let Ok(bytes) = std::fs::read(path) else {
        return Examined::Unopened;
    };
    let Ok(document) = Document::open_with_password(bytes, Limits::DEFAULT, password_for(path))
    else {
        return Examined::Unopened;
    };
    let table = widgets_by_field_name(&document);
    let mut run = 0_usize;
    let mut not_run = 0_usize;
    let mut scripted: Vec<&String> = Vec::new();
    for (name, widgets) in &table {
        let Some(first) = widgets.first() else {
            continue;
        };
        let object = document.get(*first);
        let Some(widget) = object.as_dict() else {
            continue;
        };
        let mut any = false;
        for trigger in TRIGGERS {
            match site::of_widget(&document, widget, trigger) {
                Site::Absent => {}
                Site::Library(_) => {
                    run = run.saturating_add(1);
                    any = true;
                }
                Site::NotRun(_) => {
                    not_run = not_run.saturating_add(1);
                    any = true;
                }
            }
        }
        if any {
            scripted.push(name);
        }
    }
    if scripted.is_empty() {
        return Examined::Outside;
    }

    let mut view = ViewState::of(&document);
    let mut untaken = 0_usize;
    let mut refused = 0_usize;
    for name in scripted {
        let Some(shown) = view.field_value(&document, name) else {
            continue;
        };
        if shown.obscured {
            continue;
        }
        if view.set_field(&document, name, &Entered::Text(shown.text)) == 0 {
            untaken = untaken.saturating_add(1);
            continue;
        }
        if matches!(view.commit_field(&document, name), Committed::Refused(_)) {
            refused = refused.saturating_add(1);
        }
    }

    let mut hasher = Sha256::new();
    for name in table.keys() {
        let shown = view.displayed_value(&document, name).unwrap_or_default();
        hasher.update(name.as_bytes());
        hasher.update(b"\t");
        hasher.update(shown.as_bytes());
        hasher.update(b"\n");
    }
    let mut digest = String::new();
    for byte in hasher.finalize().iter().take(8) {
        let _ = write!(digest, "{byte:02x}");
    }
    Examined::Held(Line {
        fields: table.len(),
        run,
        not_run,
        untaken,
        refused,
        digest,
    })
}

/// Where the golden lives: beside this file.
fn golden_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/script_corpus.tsv")
}

/// Reads the golden, or `None` where there is none yet.
fn read_golden(path: &Path) -> Option<BTreeMap<String, Line>> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut held = BTreeMap::new();
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
        let Some(entry) = Line::parse(rest) else {
            panic!(
                "{}:{}: not a line this gate wrote: {line:?}",
                path.display(),
                number.saturating_add(1)
            );
        };
        assert!(
            held.insert(key.to_owned(), entry).is_none(),
            "{}:{}: {key:?} is held twice",
            path.display(),
            number.saturating_add(1)
        );
    }
    Some(held)
}

/// Writes the golden whole, sorted.
fn write_golden(path: &Path, lines: &BTreeMap<String, Line>) {
    let mut text = String::from(
        "# Written by `tests/script_corpus.rs`, and read by it: a document whose line differs \
         fails, naming it.\n\
         # Columns: document, fields, sites Tier 0 runs, sites reported as not run, fields whose \
         own value the commit did not take, commits refused, digest of every displayed value.\n\
         # Regenerate after a change that moves values on purpose:\n\
         #     PDFVIEWER_SCRIPT_CORPUS=update cargo test --profile gates -p pdf-model --test \
         script_corpus -- --ignored --nocapture\n",
    );
    for (key, line) in lines {
        let _ = writeln!(text, "{key}\t{}", line.to_columns());
    }
    std::fs::write(path, text).expect("the golden file can be written beside its test");
}

/// What a walk found: the lines, how many documents did not open, and which panicked.
struct Walked {
    /// Every document in the gate, by key.
    measured: BTreeMap<String, Line>,
    /// Documents that could not be opened or were over the size bound.
    unopened: usize,
    /// Documents whose examination panicked.
    panicked: Vec<String>,
}

/// Examines every file, in parallel.
fn walk(files: &[PathBuf], root: &Path) -> Walked {
    let results: Vec<(String, Result<Examined, ()>)> = files
        .par_iter()
        .map(|path| {
            let key = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .into_owned();
            let examined = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| examine(path)))
                .map_err(|_| ());
            (key, examined)
        })
        .collect();
    let mut walked = Walked {
        measured: BTreeMap::new(),
        unopened: 0,
        panicked: Vec::new(),
    };
    for (key, examined) in results {
        match examined {
            Ok(Examined::Held(line)) => {
                walked.measured.insert(key, line);
            }
            Ok(Examined::Unopened) => walked.unopened = walked.unopened.saturating_add(1),
            Ok(Examined::Outside) => {}
            Err(()) => walked.panicked.push(key),
        }
    }
    walked
}

/// Rewrites the golden from a walk, printing every line that moved, arrived or left.
fn regenerate(path: &Path, measured: &BTreeMap<String, Line>) {
    let held = read_golden(path).unwrap_or_default();
    for (key, line) in measured {
        match held.get(key) {
            Some(was) if was == line => {}
            Some(was) => println!(
                "moved: {key}: {} -> {}",
                was.to_columns(),
                line.to_columns()
            ),
            None => println!("added: {key}: {}", line.to_columns()),
        }
    }
    for key in held.keys().filter(|key| !measured.contains_key(*key)) {
        println!("removed: {key}");
    }
    write_golden(path, measured);
}

/// Every line on disk that differs from the one held, and how many are unheld and absent.
fn compare(
    held: &BTreeMap<String, Line>,
    measured: &BTreeMap<String, Line>,
) -> (Vec<String>, usize, usize) {
    let mut moved = Vec::new();
    let mut unheld = 0_usize;
    for (key, line) in measured {
        match held.get(key) {
            Some(was) if was == line => {}
            Some(was) => moved.push(format!(
                "{key}: {} -> {}",
                was.to_columns(),
                line.to_columns()
            )),
            None => unheld = unheld.saturating_add(1),
        }
    }
    let absent = held
        .keys()
        .filter(|key| !measured.contains_key(*key))
        .count();
    (moved, unheld, absent)
}

#[test]
#[ignore = "walks the census population; run behind the heavy-walk lock"]
fn every_scripted_field_displays_what_it_displayed_when_held() {
    let started = Instant::now();
    let root = repository();
    let files = population(&root);
    let walked = walk(&files, &root);
    let measured = &walked.measured;
    let sum = |column: fn(&Line) -> usize| -> usize { measured.values().map(column).sum() };
    println!(
        "{} PDF(s) walked, {} not opened; {} carry a field script at a Table 199 site: \
         {} sites run by Tier 0, {} reported as not run, {} values untaken, {} commits refused",
        files.len(),
        walked.unopened,
        measured.len(),
        sum(|line| line.run),
        sum(|line| line.not_run),
        sum(|line| line.untaken),
        sum(|line| line.refused),
    );
    assert!(
        walked.panicked.is_empty(),
        "the dispatch panicked on {} document(s): {:?}",
        walked.panicked.len(),
        walked.panicked
    );

    let path = golden_path();
    if std::env::var(UPDATE_VARIABLE).as_deref() == Ok("update") {
        regenerate(&path, measured);
        println!(
            "{} written, {} s",
            path.display(),
            started.elapsed().as_secs()
        );
        return;
    }
    let Some(held) = read_golden(&path) else {
        panic!(
            "{} does not exist; write it with {UPDATE_VARIABLE}=update",
            path.display()
        );
    };
    let (moved, unheld, absent) = compare(&held, measured);
    println!(
        "{} held, {unheld} on disk and unheld, {absent} held and not on disk, {} moved; {} s",
        held.len(),
        moved.len(),
        started.elapsed().as_secs()
    );
    assert!(
        moved.is_empty(),
        "{} document(s) display different values from the ones held:\n{}",
        moved.len(),
        moved.join("\n")
    );
}
