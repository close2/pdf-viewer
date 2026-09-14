//! Two state machines held to each other: the survey's resource selections against the
//! interpreter's, over every document the validator's corpus holds.
//!
//! # What this gate is for
//!
//! `crate::survey` re-derives from content-stream tokens what `pdf_model`'s interpreter also
//! derives — the resource dictionary in force, `q`/`Q`, which nested streams a page runs — and
//! `doc/questions/A61` decided that both machines stay, **held to each other by this test**, with
//! an observer on the interpreter revisited only if this gate fires systemically. The question
//! both machines answer at every operator is ISO 32000-2 §7.8.3's: which entry of which
//! subdictionary of the resource dictionary in force does this name select. This gate walks each
//! page with both, through `pdf_model::content::ledger::Ledger`, and asserts that within every
//! content stream both machines ran they made the same selections — every one the survey judged
//! is one the interpreter made, and every one the interpreter made is one the survey judged.
//!
//! # The scope, which is A61's own condition
//!
//! The comparison is confined to **the constructs the survey walks**: the page's content, form
//! `XObject`s reached through `Do`, tiling patterns, Type 3 glyph descriptions and annotation
//! appearance streams. A soft mask's transparency group is one the survey deliberately does not
//! walk, so a selection the interpreter made under one is not something the survey failed to
//! judge; an unscoped comparison would turn every such selection into a false failure, which is
//! the one direction `survey.rs`'s own header says the crate may not err in. Symmetrically, a
//! stream only one machine ran — an appearance state the annotation is not showing, a glyph no
//! text-showing operator reached, a form under optional content the default configuration hides
//! — has no second reading to disagree with, so it is counted and printed by route rather than
//! judged. A selection defect in the *invoking* stream is always caught there, in a place both
//! machines ran. ADR 1055 states the rules and the two calibrations that were run against them.
//!
//! Two rules narrow the selections compared, and each is a clause rather than a convenience:
//!
//! - §8.6.8 Table 74 lets `cs` name a colour space family directly, so a family name is not a
//!   resource name and a lookup of one that finds nothing is not a selection. The interpreter
//!   probes its memo with one; the survey never looks. A lookup that *finds* something is kept,
//!   because then a machine has selected it.
//! - §14.6.1's `DP` marks a point: nothing is drawn under it and nothing encloses it, so the
//!   interpreter has no reason to read its property list and does not. The survey reads it for
//!   ISO 19005-2 section 6.7.4's `/Lang`. That is one machine asking a question the other has no
//!   use for, not two answers to one question.
//!
//! # Running it
//!
//! Ignored by default, because it needs `doc/veraPDF-corpus`, which is 239 MB and not part of a
//! checkout. Further roots may be named in `PDFVIEWER_CROSS_CHECK_ROOTS`, colon-separated.
//!
//! ```sh
//! tools/bounded.sh -- cargo test --profile gates -p pdf-archive --test cross_check -- --ignored --nocapture
//! ```

// no sandbox worker: a `Do` selects its XObject before the image is decoded, and neither machine's selection moves with whether `CCITTFaxDecode`, `JBIG2Decode` or `JPXDecode` can be run; what the worker changes is what the image draws, which is not what this gate reads (`tools/conformance/tests/sandbox_gates.rs`).

#![expect(
    clippy::print_stdout,
    reason = "test code whose product is a report a person reads, and whose failure names every \
              disagreement before it fails"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use pdf_archive::survey::Survey;
use pdf_model::Pages;
use pdf_model::content::ledger::{Frame, Ledger, Occurrence, Outcome, Route, Selection};
use pdf_syntax::{Document, FileBytes, ObjectId};

/// Where the corpus is, when somebody has fetched it.
fn corpus() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/veraPDF-corpus");
    root.is_dir().then_some(root)
}

/// Every `.pdf` under a root, in a stable order.
fn documents(root: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            documents(&path, into);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        {
            into.push(path);
        }
    }
}

/// How many frames deep the survey follows a chain of nested streams: its page and the eight
/// `crate::survey::MAX_FORM_DEPTH` allows beneath it. A place longer than this is beyond the
/// survey's bound, which is a bound and not a reading.
const SURVEY_DEPTH: usize = 9;

/// One content stream a machine ran, named by the frames that reached it.
///
/// The outermost frame is the page's content, keyed by the page object, or an annotation's
/// appearance, keyed by the stream; so a place needs no page number of its own to be the same
/// place on both sides.
type Place = Vec<Frame>;

/// One selection's identity: what was asked, and what was found.
type Key = (&'static str, Vec<u8>, Outcome);

/// What one machine selected in one place, with where it first did so.
type Selected = BTreeMap<Key, Occurrence>;

/// Which of the two machines a selection belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Machine {
    Survey,
    Interpreter,
}

/// §8.6.8 Table 74's family names, which `cs` may state directly and which name no resource.
fn names_a_family(name: &[u8]) -> bool {
    matches!(
        name,
        b"DeviceGray" | b"DeviceRGB" | b"DeviceCMYK" | b"Pattern" | b"G" | b"RGB" | b"CMYK"
    )
}

/// Whether a selection is inside the comparison's scope — see the module comment.
fn in_scope(machine: Machine, selection: &Selection, at: &Occurrence) -> bool {
    if selection
        .frames
        .iter()
        .any(|frame| frame.route == Route::SoftMask)
    {
        return false;
    }
    if selection.category == "ColorSpace"
        && names_a_family(&selection.name)
        && selection.outcome == Outcome::Missing
    {
        return false;
    }
    if machine == Machine::Survey && selection.category == "Properties" && at.operator == b"DP" {
        return false;
    }
    true
}

/// Sorts one ledger's selections by the place they were made, keeping what is in scope.
fn by_place(machine: Machine, ledger: &Ledger, into: &mut BTreeMap<Place, Selected>) {
    for (selection, at) in ledger.selections() {
        if !in_scope(machine, selection, at) {
            continue;
        }
        into.entry(selection.frames.clone())
            .or_default()
            .entry((
                selection.category,
                selection.name.clone(),
                selection.outcome.clone(),
            ))
            .or_insert_with(|| at.clone());
    }
}

/// The frames of a place, for a sentence.
fn describe(frames: &[Frame]) -> String {
    frames
        .iter()
        .map(|frame| {
            let route = match frame.route {
                Route::Page => "page content",
                Route::Form => "form",
                Route::TilingPattern => "tiling pattern",
                Route::Type3Glyph => "Type 3 glyph",
                Route::Appearance => "appearance",
                Route::SoftMask => "soft mask",
            };
            match frame.stream {
                Some(id) => format!("{route} {id}"),
                None => route.to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join(" > ")
}

/// An outcome, for a sentence, cut short where a direct object is long.
fn describe_outcome(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Reference(id) => id.to_string(),
        Outcome::Missing => "nothing (the subdictionary does not define it)".to_owned(),
        Outcome::Direct(text) => {
            let cut: String = text.chars().take(80).collect();
            if cut.len() < text.len() {
                format!("{cut}…")
            } else {
                cut
            }
        }
    }
}

/// The innermost route of a place, for the census.
fn innermost(frames: &[Frame]) -> &'static str {
    match frames.last().map(|frame| frame.route) {
        Some(Route::Page) | None => "page content",
        Some(Route::Form) => "form XObject",
        Some(Route::TilingPattern) => "tiling pattern",
        Some(Route::Type3Glyph) => "Type 3 glyph",
        Some(Route::Appearance) => "annotation appearance",
        Some(Route::SoftMask) => "soft mask group",
    }
}

/// The tallies one run accumulates.
#[derive(Debug, Default)]
struct Tally {
    documents: usize,
    unopenable: usize,
    pages: usize,
    paired_places: usize,
    selections_compared: usize,
    survey_only_places: BTreeMap<&'static str, usize>,
    interpreter_only_places: BTreeMap<&'static str, usize>,
    /// Places under a soft mask's group, and the selections made there, which the scope keeps
    /// out of the comparison. Counted so that the scoping is visible as a number rather than as
    /// silence: the calibration ADR 1055 records plants a defect there and reads this line.
    soft_mask_places: usize,
    soft_mask_selections: usize,
    truncated_ledgers: usize,
    disagreements: Vec<String>,
    /// Every place one machine alone ran, named, where `PDFVIEWER_CROSS_CHECK_VERBOSE` is set.
    unpaired: Vec<String>,
}

/// Which page a place is on, for a sentence: the page object its outermost frame names, or the
/// page whose annotation pass ran the appearance it starts with.
fn page_of(
    place: &Place,
    pages: &Pages<'_>,
    appearance_pages: &BTreeMap<ObjectId, usize>,
) -> String {
    let Some(top) = place.first() else {
        return "no page".to_owned();
    };
    let index = match (top.route, top.stream) {
        (Route::Page, Some(id)) => pages.index_of(id),
        (Route::Appearance, Some(id)) => appearance_pages.get(&id).copied(),
        _ => None,
    };
    index.map_or_else(
        || "a page this gate could not number".to_owned(),
        |index| format!("page {}", index.saturating_add(1)),
    )
}

/// One disagreement, as a sentence a person can act on.
fn disagreement(
    document: &str,
    page: &str,
    place: &Place,
    only: Machine,
    key: &Key,
    at: &Occurrence,
) -> String {
    let (category, name, outcome) = key;
    let machine = match only {
        Machine::Survey => "the survey judged and the interpreter never selected",
        Machine::Interpreter => "the interpreter selected and the survey never judged",
    };
    format!(
        "{document} {page}, {}: {machine} /{category} /{} -> {} (operator {} of that stream, {})",
        describe(place),
        String::from_utf8_lossy(name),
        describe_outcome(outcome),
        at.ordinal,
        String::from_utf8_lossy(&at.operator),
    )
}

/// Compares the two machines over one document.
fn cross_check(path: &Path, tally: &mut Tally) {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let Ok(bytes) = FileBytes::on_disk(path) else {
        tally.unopenable = tally.unopenable.saturating_add(1);
        return;
    };
    let Ok(document) = Document::open(bytes) else {
        tally.unopenable = tally.unopenable.saturating_add(1);
        return;
    };
    tally.documents = tally.documents.saturating_add(1);

    let mut survey_places: BTreeMap<Place, Selected> = BTreeMap::new();
    let (_, survey_ledger) = Survey::ledgered(&document);
    if survey_ledger.truncated() {
        tally.truncated_ledgers = tally.truncated_ledgers.saturating_add(1);
    }
    by_place(Machine::Survey, &survey_ledger, &mut survey_places);

    let verbose = std::env::var_os("PDFVIEWER_CROSS_CHECK_VERBOSE").is_some();
    let pages = Pages::new(&document);
    let mut interpreter_places: BTreeMap<Place, Selected> = BTreeMap::new();
    let mut appearance_pages: BTreeMap<ObjectId, usize> = BTreeMap::new();
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        tally.pages = tally.pages.saturating_add(1);
        let (_, ledger) = pdf_model::content::interpret_ledgered(&document, &page);
        if ledger.truncated() {
            tally.truncated_ledgers = tally.truncated_ledgers.saturating_add(1);
        }
        let mut masked: BTreeSet<&Place> = BTreeSet::new();
        for (selection, _) in ledger.selections() {
            if let Some(top) = selection.frames.first()
                && top.route == Route::Appearance
                && let Some(id) = top.stream
            {
                appearance_pages.entry(id).or_insert(index);
            }
            if selection
                .frames
                .iter()
                .any(|frame| frame.route == Route::SoftMask)
            {
                masked.insert(&selection.frames);
                tally.soft_mask_selections = tally.soft_mask_selections.saturating_add(1);
                if verbose {
                    tally.unpaired.push(format!(
                        "{name} page {}: under a soft mask, not judged: {} /{} /{} -> {}",
                        index.saturating_add(1),
                        describe(&selection.frames),
                        selection.category,
                        String::from_utf8_lossy(&selection.name),
                        describe_outcome(&selection.outcome),
                    ));
                }
            }
        }
        tally.soft_mask_places = tally.soft_mask_places.saturating_add(masked.len());
        by_place(Machine::Interpreter, &ledger, &mut interpreter_places);
    }

    compare(
        &name,
        &pages,
        &appearance_pages,
        &survey_places,
        &interpreter_places,
        tally,
    );
}

/// Holds the two machines' places to each other: paired ones selection by selection, and the
/// rest counted by route.
fn compare(
    name: &str,
    pages: &Pages<'_>,
    appearance_pages: &BTreeMap<ObjectId, usize>,
    survey_places: &BTreeMap<Place, Selected>,
    interpreter_places: &BTreeMap<Place, Selected>,
    tally: &mut Tally,
) {
    let verbose = std::env::var_os("PDFVIEWER_CROSS_CHECK_VERBOSE").is_some();
    let survey_keys: BTreeSet<&Place> = survey_places.keys().collect();
    let interpreter_keys: BTreeSet<&Place> = interpreter_places.keys().collect();
    for place in survey_keys.difference(&interpreter_keys) {
        let count = tally
            .survey_only_places
            .entry(innermost(place))
            .or_default();
        *count = count.saturating_add(1);
        if verbose {
            let page = page_of(place, pages, appearance_pages);
            tally.unpaired.push(format!(
                "{name} {page}: only the survey walked {}",
                describe(place)
            ));
        }
    }
    // Every stream the survey walked, by the object it is, under each top-level place: a form
    // the survey visited once on a page (its `visited` set) is walked on that page, whatever
    // route the interpreter took to it the second time.
    let survey_walked: BTreeSet<(&Frame, ObjectId)> = survey_keys
        .iter()
        .flat_map(|place| {
            let top = &place[0];
            place
                .iter()
                .filter_map(move |frame| frame.stream.map(|id| (top, id)))
        })
        .collect();
    for place in interpreter_keys.difference(&survey_keys) {
        let count = tally
            .interpreter_only_places
            .entry(innermost(place))
            .or_default();
        *count = count.saturating_add(1);
        if verbose {
            let page = page_of(place, pages, appearance_pages);
            tally.unpaired.push(format!(
                "{name} {page}: only the interpreter ran {}",
                describe(place)
            ));
        }
        // The other half of "the constructs the survey walks": a stream the interpreter ran
        // through a route the survey does walk — `Do`, a pattern, a glyph, an appearance — and
        // within the survey's own nesting bound, which the survey nevertheless reached nowhere
        // on this page, is a stream the survey's walk lost. A stream this program composed has
        // no object and the survey has nothing to walk; a chain deeper than the survey follows
        // is its bound and not its reading.
        let Some((top, last)) = place.first().zip(place.last()) else {
            continue;
        };
        if top.stream.is_none() || place.len() > SURVEY_DEPTH {
            continue;
        }
        if let Some(id) = last.stream
            && !survey_walked.contains(&(top, id))
        {
            let page = page_of(place, pages, appearance_pages);
            tally.disagreements.push(format!(
                "{name} {page}, {}: the interpreter ran this stream and the survey never walked \
                 it on this page",
                describe(place)
            ));
        }
    }
    for place in survey_keys.intersection(&interpreter_keys) {
        tally.paired_places = tally.paired_places.saturating_add(1);
        let survey = &survey_places[*place];
        let interpreter = &interpreter_places[*place];
        tally.selections_compared = tally
            .selections_compared
            .saturating_add(survey.len().max(interpreter.len()));
        let page = page_of(place, pages, appearance_pages);
        for (key, at) in survey {
            if !interpreter.contains_key(key) {
                tally.disagreements.push(disagreement(
                    name,
                    &page,
                    place,
                    Machine::Survey,
                    key,
                    at,
                ));
            }
        }
        for (key, at) in interpreter {
            if !survey.contains_key(key) {
                tally.disagreements.push(disagreement(
                    name,
                    &page,
                    place,
                    Machine::Interpreter,
                    key,
                    at,
                ));
            }
        }
    }
}

fn report(tally: &Tally, elapsed: std::time::Duration) {
    println!(
        "cross-check: {} documents opened, {} not opened, {} pages",
        tally.documents, tally.unopenable, tally.pages
    );
    println!(
        "cross-check: {} places both machines ran, {} selections compared, {} disagreements",
        tally.paired_places,
        tally.selections_compared,
        tally.disagreements.len()
    );
    for (route, count) in &tally.survey_only_places {
        println!("cross-check: {count} places only the survey walked, innermost a {route}");
    }
    for (route, count) in &tally.interpreter_only_places {
        println!("cross-check: {count} places only the interpreter ran, innermost a {route}");
    }
    println!(
        "cross-check: {} places under a soft mask's group with {} selections, not judged (A61's scope)",
        tally.soft_mask_places, tally.soft_mask_selections
    );
    for line in &tally.unpaired {
        println!("  {line}");
    }
    if tally.truncated_ledgers > 0 {
        println!(
            "cross-check: {} ledgers hit MAX_SELECTIONS",
            tally.truncated_ledgers
        );
    }
    for line in &tally.disagreements {
        println!("  {line}");
    }
    println!("cross-check: {elapsed:.1?}");
}

#[test]
#[ignore = "needs doc/veraPDF-corpus, which is 239 MB and not part of a checkout"]
fn the_survey_and_the_interpreter_select_the_same_resources() {
    let mut roots: Vec<PathBuf> = corpus().into_iter().collect();
    if let Ok(more) = std::env::var("PDFVIEWER_CROSS_CHECK_ROOTS") {
        roots.extend(
            more.split(':')
                .filter(|root| !root.is_empty())
                .map(PathBuf::from),
        );
    }
    if roots.is_empty() {
        println!("doc/veraPDF-corpus is not here; nothing to compare");
        return;
    }
    let started = Instant::now();
    let mut paths = Vec::new();
    for root in &roots {
        documents(root, &mut paths);
    }
    let mut tally = Tally::default();
    for path in &paths {
        cross_check(path, &mut tally);
    }
    report(&tally, started.elapsed());
    assert!(
        tally.documents > 0,
        "no document opened under {roots:?}, so nothing was compared"
    );
    assert!(
        tally.paired_places > 0,
        "no content stream was run by both machines, so nothing was compared"
    );
    assert!(
        tally.disagreements.is_empty(),
        "{} resource selections on which the survey and the interpreter disagree, listed above",
        tally.disagreements.len()
    );
}
