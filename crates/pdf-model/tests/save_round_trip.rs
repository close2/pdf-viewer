//! Every corpus document, edited, saved, and read back by three readers (ADR 0334).
//!
//! ADR 0323's second instrument: the corpus and the oracle judge a raster, and §7.5.6's
//! incremental update — the one form of writing `CLAUDE.md` permits — had never been judged at
//! corpus scale by anybody but its own author. One synthetic [`pdf_model::view::ViewState`] edit
//! per document (a free text annotation always, a field value wherever a text field exists),
//! saved, and then three **exact** assertions with no tolerance anywhere:
//!
//! 1. **The prefix property.** §7.5.6 appends — "leaving its original contents intact" — so the
//!    producer's bytes are byte-for-byte a prefix of the saved file.
//! 2. **Our own readback.** This tree re-opens its save and reads the edit back. Necessary, and
//!    insufficient alone: a measurement taken with the instrument under test (trap 8).
//! 3. **The references see it too.** poppler and mupdf open the saved file and report the edit —
//!    the annotation *where it was put*, the field holding its value — which is the half that
//!    catches a malformed update this tree happens to be able to re-read. Table 231 bit 14's
//!    password rule is the precedent for what this exists to catch: the save path wrote a
//!    person's typed password into the file for as long as nobody independent read the file
//!    back (ADR 0247).
//!
//! # What each reference is asked (trap 3)
//!
//! *"Open this file and say what is in it"* — never to render, and never a coordinate in a frame
//! this instrument has not measured. mupdf answers from its raw object layer, so its coordinates
//! are the file's own `/Rect` and no page-space convention is in the answer at all. poppler has
//! no CLI that prints an annotation or a field value, so the question goes through poppler-glib
//! (`tests/save_round_trip/poppler_witness.py`), whose annotation area was **measured** before it
//! was compared: the `/Rect` translated by the crop box's own origin, y still upward, with the
//! page's `/Rotate` then applied — each case pinned against a hand-built fixture in session 499,
//! and [`poppler_area`] is that measurement as arithmetic.
//!
//! # The refusal census, and the two `Restrict` levels
//!
//! The denominator is every corpus document; each one that is not judged says why, because a
//! refusal shrinks the judged set and that arithmetic must stay visible (trap 11). Two kinds are
//! *populations of interest* rather than gaps:
//!
//! - **Refused by policy.** Under the default `Restrict(On)` a document whose Table 22 flags or
//!   §12.8.2.2 certification withhold the operation is refused — the same
//!   [`pdf_model::restriction::asserted`] answer `viewer-core`'s `refusal` acts on — and that is
//!   **the policy working**, counted by clause, never folded into a failure count. The same
//!   documents are then run again as `Restrict(Off)`, which `CLAUDE.md` says shall always be
//!   possible, so the three assertions still bind them and the count that differs between the two
//!   levels is exactly the policy's own population.
//! - **Refused by construction.** [`pdf_syntax::write::UpdateError`] names every file §7.5.6
//!   cannot honestly be appended to (a table rebuilt by scanning has no offset to chain to, an
//!   attachment-only-encrypted file's key cannot encrypt what is written); each is counted under
//!   its own variant.
//!
//! Encrypted documents are **in** the population: §7.6.2's ciphers run on the way out, and
//! §7.6.3.2's fresh initialisation vectors only make their saves non-deterministic between runs
//! (ADR 0129), which costs nothing here because nothing here is cached.
//!
//! # Why nothing is cached, and what bounds the runtime instead
//!
//! `pdfref`'s cache keys on the invocation, the renderer's version and the *document's* SHA-256
//! (trap 10a) — and a freshly-written file's hash changes whenever the writer changes, which is
//! most rounds that would run this, and on **every** run for an encrypted document. A cache
//! keyed on bytes that are new each time is a cache that never hits, so this instrument does not
//! use one and says so. What bounds the runtime instead is that the questions are object reads
//! rather than renders — milliseconds against `pdftoppm`'s seconds — measured at well under the
//! oracle's cost for the whole population, so no sampling is needed and the denominator is every
//! document that saves. (The oracle-over-saved-files half of ADR 0323 stays unbuilt; ADR 0334
//! prices it.)
//!
//! # Where this lives, and why it is its own binary
//!
//! `doc/todo/02` §2's gate lines each hand `-- --ignored` to a whole test binary, so an ignored
//! test placed in `corpus.rs`'s or `text_extraction.rs`'s binary joins that gate's run and its
//! output *immediately* — and ADR 0323's own rule is that an instrument's numbers enter §2 only
//! after they have held across rounds. A binary of its own keeps the instrument invokable by
//! name today and keeps the eventual gate line running exactly this and nothing else:
//!
//! ```text
//! cargo test --profile gates -p pdf-model --test save_round_trip -- --ignored --nocapture
//! ```

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure, and the census output \
              is the point of the run"
)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use pdf_model::Pages;
use pdf_model::restriction::{self, Operation, Restriction};
use pdf_model::view::{Entered, ViewState};
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Limits, Object, SyntaxError};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// The free text the edit types, and a string no corpus document contains.
///
/// A sentinel rather than prose for ADR 0129's reason: the discriminating assertion on an
/// encrypted document is that this string is *absent* from the appended bytes (it was
/// encrypted), and present in every reader's answer — and only a string no PDF contains can
/// make both statements.
const FREE_TEXT_WITNESS: &str = "pdf-viewer save round-trip free text witness 499";

/// The value the field edit types, distinct from the annotation's so that a reference
/// answering one question cannot pass as answering the other.
const FIELD_WITNESS: &str = "pdf-viewer field witness 499";

/// The corpus documents that refuse §7.6.4.1's default user password, with the password each
/// one's own pdf.js issue records.
///
/// The same eight `pdf-syntax`'s `encryption.rs` verifies; the instrument's rule is ADR 0323's
/// "without a password or with the corpus's known ones". `print_protection.pdf`'s `1234` is its
/// **owner** password, which §7.6.4.1 gives full access — so Table 22 withholds nothing from it
/// and the policy census must not count it.
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

/// How long one reference invocation may run before it is killed — `pdfref`'s own bound, kept
/// here because there is deliberately no unbounded way to wait on a reference.
const REFERENCE_BUDGET: Duration = Duration::from_secs(30);

/// Passwords the *references* are handed where the one this reader authenticated with does
/// not serve them — each a measured gap in the reference, not a defect in the file.
///
/// - `pr6531_2.pdf`: the empty string is this file's **owner** password (§7.6.4.4.11's
///   Algorithm 12 — the corpus's one file exercising that branch), which this reader
///   authenticates and mupdf 1.28 refuses. The references get the user password pdf.js pull
///   request 6531 records, which both accept.
/// - `saslprep-r6.pdf`: the password is one §7.6.4.3.3's `SASLprep` *changes* (U+00AA
///   normalises to `a`, U+00AD maps to nothing), and neither reference implements the
///   preprocessing — both refuse the stated password and both accept its normalised form,
///   measured in session 499. The normalised form is the same key, so handing it over asks
///   the reference the same question in the spelling it understands.
const REFERENCE_PASSWORDS: &[(&str, &str)] = &[
    ("pr6531_2.pdf", "asdfasdf"),
    ("saslprep-r6.pdf", "SaSLprep"),
];

/// Documents the references are expected to disagree about, each with the diagnosis.
///
/// Named rather than counted, the `KNOWN_SLOW` discipline: a new disagreement fails the run
/// even though a total has not moved, and resolving one deletes an entry. Every entry must be
/// diagnosed as our defect (then it is work, not an entry), the reference's gap, or a question
/// §7.5.6 answers — a name with no diagnosis is not allowed in.
const KNOWN_DISAGREEMENTS: &[(&str, &str)] = &[];

/// One reference's answer about a saved file, parsed from the witness scripts' line protocol.
struct Answer {
    /// Every free text annotation of page one: contents and four rectangle numbers.
    free_texts: Vec<(String, [f64; 4])>,
    /// Every text field the script reported: qualified name and value.
    fields: Vec<(String, String)>,
    /// The reference opened the file and could not reach a first page.
    page_one_missing: bool,
    /// The reference could not authenticate and read on regardless (mupdf's own behaviour).
    unauthenticated: bool,
}

/// What one level's sweep across the corpus produced.
#[derive(Default)]
struct Level {
    /// Documents saved and judged.
    saved: usize,
    /// Free text and field edits that were checked against all three readers.
    free_texts_checked: usize,
    fields_checked: usize,
    /// Documents where no edit applied, so there was nothing to save: name and why.
    nothing_to_save: Vec<(String, String)>,
    /// Saves the writer refused, by `UpdateError` — §7.5.6 cannot honestly be appended.
    save_refused: Vec<(String, String)>,
    /// Assertion 1 failures: the producer's bytes were not a prefix of the save.
    prefix_failed: Vec<String>,
    /// Assertion 2 failures: this tree could not read its own save back.
    readback_failed: Vec<(String, String)>,
    /// A reference would not answer at all: refused the file, crashed, or timed out.
    reference_failed: Vec<(String, String)>,
    /// Assertion 3 failures: a reference answered and did not see the edit.
    disagreed: Vec<(String, String)>,
    /// A reference that cannot read the **original** cannot witness a save of it, so it is
    /// excluded from assertion 3's denominator for that document — printed by reason, because
    /// trap 11's arithmetic binds an exclusion exactly as it binds a report.
    reference_excluded: Vec<(String, String)>,
    /// A document whose examination panicked, which principle 1 forbids.
    panicked: Vec<(String, String)>,
}

/// The whole census.
#[derive(Default)]
struct Tally {
    /// Documents that could not be opened, by reason.
    refused_open: Vec<(String, String)>,
    /// Documents with no page an annotation can be attached to, by reason: no reachable page
    /// one at all, or a page one that is not an indirect object — `issue9105_other.pdf`'s
    /// `/Kids` holds an *inline* page dictionary, and §7.5.6's update replaces objects, so a
    /// page that is not one cannot be given an `/Annots`, nor be named by Table 166's `/P`,
    /// "[a]n indirect reference to the page object with which this annotation is associated".
    pageless: Vec<(String, String)>,
    /// Documents carrying a fillable text field (the `Edit::SetField` population).
    with_text_field: usize,
    /// The policy census: what `Restrict(On)` refused, by document and clause.
    policy_refused: Vec<(String, String)>,
    /// The sweep under the default `Restrict(On)`.
    on: Level,
    /// The sweep under `Restrict(Off)`, over only the documents `On` refused something of —
    /// the population that differs between the levels is exactly the policy's own count.
    off: Level,
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

/// Where the saved files are written for the references to read.
///
/// A file that passes is deleted; one that fails any assertion is left here and named, the
/// oracle-artefact habit — what is on disk afterwards is exactly the set worth looking at.
fn artefacts() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("save-round-trip");
    std::fs::create_dir_all(&dir).expect("the target tmpdir is writable");
    dir
}

/// The witness rectangle, anchored to the page's own crop box.
///
/// ADR 0323 says "a fixed rectangle", and fixed *in the page's own frame* is the version that
/// tests something everywhere: corpus pages exist whose boxes exclude the origin entirely
/// (`160F-2019.pdf`'s crop box starts at x = −11.96), and an annotation placed off the visible
/// page would leave "where it was put" unwitnessed. Sixteen points in from the crop box's
/// lower-left corner, 180 × 60 — the same rectangle for every document, relative to the one
/// corner every page has.
fn witness_rect(crop: [f32; 4]) -> [f32; 4] {
    [
        crop[0] + 16.0,
        crop[1] + 16.0,
        crop[0] + 196.0,
        crop[1] + 76.0,
    ]
}

/// A restriction, as one census line names it.
fn reason(restriction: Restriction) -> String {
    match restriction {
        Restriction::Certified { level } => {
            format!("certified by its author (§12.8.2.2, {level:?})")
        }
        Restriction::AccessDenied { bit } => {
            format!(
                "encryption withholds it (§7.6.4.2 Table 22, bit {})",
                bit.position()
            )
        }
        Restriction::FieldLocked => "a signature locks the field (§12.7.5.5)".to_owned(),
        Restriction::FieldCovered => {
            "a signature's FieldMDP transform covers the field (§12.8.2.4)".to_owned()
        }
        Restriction::AnnotationLocked => "LockedContents (Table 167 bit 10)".to_owned(),
    }
}

/// The first fillable text field of the document, with the page index its widget is on.
///
/// "Wherever a text field exists" (ADR 0323): the field must be one `Edit::SetField` can
/// honestly fill, so Table 227 bit 1's read-only fields and Table 231 bit 14's password fields
/// are passed over — the second because [`ViewState::save`] rightly withholds their values
/// (ADR 0247), which would leave assertion 3 asking the references to see a value the file
/// must not contain.
fn fillable_text_field(document: &Document, pages: &Pages) -> Option<(String, usize)> {
    document.catalog().ok()?.get("AcroForm")?;
    let view = ViewState::of(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        for field in pdf_model::form::fields(document, &page, &view) {
            if field.read_only {
                continue;
            }
            if let pdf_model::form::Control::Text(control) = field.control
                && !control.password
            {
                return Some((field.name.qualified, index));
            }
        }
    }
    None
}

/// Runs one witness script with [`REFERENCE_BUDGET`] and hands back its stdout.
///
/// Polls and kills rather than waiting, `pdfref::Reference::render_within`'s rule: a reference
/// that never returns must cost this gate thirty seconds, not forever.
fn ask(mut command: std::process::Command) -> Result<String, String> {
    use std::io::Read as _;

    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().map_err(|error| format!("spawn: {error}"))?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() > REFERENCE_BUDGET => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("timed out after {REFERENCE_BUDGET:?}"));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(error) => return Err(format!("wait: {error}")),
        }
    };
    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_string(&mut stdout);
    }
    if status.success() {
        Ok(stdout)
    } else {
        let mut stderr = String::new();
        if let Some(mut pipe) = child.stderr.take() {
            let _ = pipe.read_to_string(&mut stderr);
        }
        Err(format!(
            "exited {status}: {}",
            stderr.lines().next().unwrap_or("")
        ))
    }
}

/// Parses the witness scripts' shared line protocol.
fn parse(output: &str) -> Answer {
    let mut answer = Answer {
        free_texts: Vec::new(),
        fields: Vec::new(),
        page_one_missing: false,
        unauthenticated: false,
    };
    for line in output.lines() {
        let cells: Vec<&str> = line.split('\t').collect();
        match cells.as_slice() {
            ["freetext", contents, x0, y0, x1, y1] => {
                let numbers: Vec<f64> = [x0, y0, x1, y1]
                    .iter()
                    .filter_map(|cell| cell.parse().ok())
                    .collect();
                if let Ok(rect) = <[f64; 4]>::try_from(numbers) {
                    answer.free_texts.push(((*contents).to_owned(), rect));
                }
            }
            ["field", name, value] => {
                answer
                    .fields
                    .push(((*name).to_owned(), (*value).to_owned()));
            }
            ["pageone", "missing"] => answer.page_one_missing = true,
            ["auth", "failed"] => answer.unauthenticated = true,
            _ => {}
        }
    }
    answer
}

/// Where poppler reports an annotation, measured rather than assumed (trap 3).
///
/// Session 499 pinned poppler-glib's annotation-mapping area against hand-built fixtures whose
/// crop box differs from their media box, at every `/Rotate`: the area is the `/Rect`
/// translated by the crop box's origin, y still upward, with the page's rotation then applied
/// as a rotation of the *page*, not of the rectangle's corners. This function is that
/// measurement; a poppler answer is compared in poppler's own frame, never poppler's answer
/// re-projected into ours.
fn poppler_area(rect: [f32; 4], crop: [f32; 4], rotate: u16) -> [f64; 4] {
    let (tx0, ty0) = (f64::from(rect[0] - crop[0]), f64::from(rect[1] - crop[1]));
    let (tx1, ty1) = (f64::from(rect[2] - crop[0]), f64::from(rect[3] - crop[1]));
    let (width, height) = (f64::from(crop[2] - crop[0]), f64::from(crop[3] - crop[1]));
    match rotate {
        90 => [ty0, width - tx1, ty1, width - tx0],
        180 => [width - tx1, height - ty1, width - tx0, height - ty0],
        270 => [height - ty1, tx0, height - ty0, tx1],
        _ => [tx0, ty0, tx1, ty1],
    }
}

/// Whether two rectangles agree within `tolerance` points on every edge.
///
/// The tolerances are representational, not judgemental: mupdf parses reals into 32-bit
/// floats, poppler's script prints four decimals, and this crate wrote the numbers from `f32`
/// — so 0.05 pt admits every representation of one rectangle and no different rectangle.
fn rects_agree(a: [f64; 4], b: [f64; 4], tolerance: f64) -> bool {
    a.iter()
        .zip(b.iter())
        .all(|(a, b)| (a - b).abs() <= tolerance)
}

/// What one pass wrote and expects the readers to see.
struct Performed {
    /// The annotation added, its object number and its rectangle.
    free_text: Option<(ObjectId, [f32; 4])>,
    /// The field filled: qualified name, widget's page index, and the value the document now
    /// holds — read back through [`ViewState::field_value`] *before* saving, because
    /// §12.7.5.3's truncation means the stored value is the document's, not the keystroke's.
    field: Option<(String, usize, String)>,
    /// The saved file.
    written: Vec<u8>,
}

/// Applies the permitted edits and saves, or says why not.
fn perform(
    document: &Document,
    page_id: ObjectId,
    rect: [f32; 4],
    add_free_text: bool,
    field: Option<&(String, usize)>,
) -> Result<Performed, (String, bool)> {
    let mut view = ViewState::of(document);
    let free_text = if add_free_text {
        match view.add_free_text(document, page_id, rect, FREE_TEXT_WITNESS, [0.7, 0.1, 0.1]) {
            Some(id) => Some((id, rect)),
            None => {
                return Err((
                    "the witness rectangle was refused (a non-finite crop box)".to_owned(),
                    false,
                ));
            }
        }
    } else {
        None
    };
    let field = field.and_then(|(name, page_index)| {
        if view.set_field(document, name, &Entered::Text(FIELD_WITNESS.to_owned())) == 0 {
            return None;
        }
        let held = view.field_value(document, name).map(|shown| shown.text)?;
        (!held.is_empty()).then_some((name.clone(), *page_index, held))
    });
    if free_text.is_none() && field.is_none() {
        return Err(("no edit applied".to_owned(), false));
    }
    match view.save(document) {
        Ok(written) => Ok(Performed {
            free_text,
            field,
            written: written.bytes,
        }),
        Err(error) => Err((error.to_string(), true)),
    }
}

/// The object `key` names in the dictionary object `id`, resolved.
fn entry(document: &Document, id: ObjectId, key: &str) -> Object {
    document
        .get(id)
        .as_dict()
        .map_or(Object::Null, |dict| document.get_key(dict, key))
}

/// Assertion 2: this tree re-opens its save and reads the edit back.
fn read_back(performed: &Performed, password: &str, page_id: ObjectId) -> Result<(), String> {
    let saved = Document::open_with_password(performed.written.clone(), Limits::DEFAULT, password)
        .map_err(|error| format!("the saved file does not re-open: {error}"))?;
    if let Some((id, rect)) = performed.free_text {
        if entry(&saved, id, "Subtype")
            .as_name()
            .is_none_or(|name| name.as_bytes() != b"FreeText")
        {
            return Err(format!("object {id:?} is not a free text annotation"));
        }
        let contents = entry(&saved, id, "Contents");
        let contents = contents
            .as_string()
            .map(pdf_syntax::text_string::text_string);
        if contents.as_deref() != Some(FREE_TEXT_WITNESS) {
            return Err(format!(
                "the annotation's contents read back as {contents:?}"
            ));
        }
        let stored: Vec<f64> = entry(&saved, id, "Rect")
            .as_array()
            .map(|edges| edges.iter().filter_map(Object::as_number).collect())
            .unwrap_or_default();
        let unchanged = <[f64; 4]>::try_from(stored)
            .is_ok_and(|stored| rects_agree(stored, rect.map(f64::from), 1e-6));
        if !unchanged {
            return Err("the annotation's /Rect read back changed".to_owned());
        }
        let attached = entry(&saved, page_id, "Annots")
            .as_array()
            .is_some_and(|entries| entries.iter().any(|entry| entry.as_reference() == Some(id)));
        if !attached {
            return Err("the page's /Annots does not name the annotation".to_owned());
        }
    }
    if let Some((name, _, held)) = &performed.field {
        let read = ViewState::of(&saved)
            .field_value(&saved, name)
            .map(|shown| shown.text);
        if read.as_deref() != Some(held.as_str()) {
            return Err(format!(
                "field {name:?} read back as {read:?}, not {held:?}"
            ));
        }
    }
    Ok(())
}

/// Assertion 3, one reference: the answer contains the edit.
fn check_answer(
    who: &str,
    answer: &Answer,
    performed: &Performed,
    expected_rect: Option<[f64; 4]>,
) -> Result<(), String> {
    if let Some(expected) = expected_rect {
        let Some((_, reported)) = answer
            .free_texts
            .iter()
            .find(|(contents, _)| contents == FREE_TEXT_WITNESS)
        else {
            return Err(format!(
                "{who} reports no free text with the witness contents"
            ));
        };
        if !rects_agree(*reported, expected, 0.05) {
            return Err(format!(
                "{who} places the annotation at {reported:?}, not {expected:?}"
            ));
        }
    }
    if let Some((name, _, held)) = &performed.field {
        let Some((_, value)) = answer.fields.iter().find(|(reported, _)| reported == name) else {
            return Err(format!("{who} reports no field named {name:?}"));
        };
        if value != held {
            return Err(format!("{who} reads {name:?} as {value:?}, not {held:?}"));
        }
    }
    Ok(())
}

/// One witness-script invocation, ready to point at either the saved file or the original.
fn witness_command(
    who: &str,
    file: &Path,
    password: &str,
    field_page: i64,
) -> std::process::Command {
    let scripts = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/save_round_trip");
    if who == "poppler" {
        let mut command = std::process::Command::new("python3");
        command
            .arg(scripts.join("poppler_witness.py"))
            .arg(file)
            .arg(password)
            .arg(field_page.to_string());
        command
    } else {
        let mut command = std::process::Command::new("mutool");
        command
            .arg("run")
            .arg(scripts.join("mupdf_witness.js"))
            .arg(file)
            .arg(password);
        command
    }
}

/// Why a reference that did not see the edit cannot be *asked*, where that is so.
///
/// The classification the exclusion category rests on: the same script pointed at the
/// **original** document. A reference that refuses the original, cannot reach its first page,
/// or cannot authenticate it is not a witness to what a save appended — mupdf and poppler do
/// not perform this reader's §7.7.3.3 page-tree recovery, and a fuzzed file several of them
/// refuse outright is still one this reader opens. `None` means the reference reads the
/// original fine, so its silence about the save is a real disagreement.
fn cannot_witness(who: &str, original: &Path, password: &str, field_page: i64) -> Option<String> {
    match ask(witness_command(who, original, password, field_page)) {
        Err(error) => Some(format!("{who} cannot read the original: {error}")),
        Ok(output) => {
            let answer = parse(&output);
            if answer.page_one_missing {
                Some(format!("{who} cannot reach the original's first page"))
            } else if answer.unauthenticated {
                Some(format!("{who} cannot authenticate the original"))
            } else {
                None
            }
        }
    }
}

/// Assertion 3, both references, over a saved file written to disk for them.
#[expect(
    clippy::too_many_arguments,
    reason = "test plumbing: the alternative is a one-use struct restating each name"
)]
fn references_see(
    name: &str,
    stem: &str,
    original: &Path,
    performed: &Performed,
    password: &str,
    crop: [f32; 4],
    rotate: u16,
    level: &mut Level,
) {
    let path = artefacts().join(format!("{stem}-{name}"));
    std::fs::write(&path, &performed.written).expect("the artefact directory is writable");
    let field_page = performed
        .field
        .as_ref()
        .and_then(|(_, page, _)| i64::try_from(*page).ok())
        .unwrap_or(-1);

    let raw_rect = performed.free_text.map(|(_, rect)| rect.map(f64::from));
    let expected_area = performed
        .free_text
        .map(|(_, rect)| poppler_area(rect, crop, rotate));

    let mut healthy = true;
    for (who, expected) in [("poppler", expected_area), ("mupdf", raw_rect)] {
        let (was_error, difference) = match ask(witness_command(who, &path, password, field_page)) {
            Ok(output) => match check_answer(who, &parse(&output), performed, expected) {
                Ok(()) => continue,
                Err(difference) => (false, difference),
            },
            Err(error) => (true, format!("{who}: {error}")),
        };
        // The reference did not see the edit. Whether that says anything about the save
        // depends on whether it can read the file the save was appended to.
        if let Some(why) = cannot_witness(who, original, password, field_page) {
            level.reference_excluded.push((name.to_owned(), why));
            continue;
        }
        healthy = false;
        if was_error {
            level.reference_failed.push((name.to_owned(), difference));
        } else {
            level.disagreed.push((name.to_owned(), difference));
        }
    }
    if healthy {
        let _ = std::fs::remove_file(&path);
    } else {
        println!("  kept for inspection: {}", path.display());
    }
}

/// One document through one level's whole pipeline: edit, save, three assertions.
#[expect(
    clippy::too_many_arguments,
    reason = "test plumbing: the alternative is a one-use struct restating each name"
)]
fn sweep(
    name: &str,
    stem: &str,
    original: &Path,
    bytes: &[u8],
    document: &Document,
    password: &str,
    reference_password: &str,
    page_id: ObjectId,
    crop: [f32; 4],
    rotate: u16,
    add_free_text: bool,
    field: Option<&(String, usize)>,
    level: &mut Level,
) {
    if !add_free_text && field.is_none() {
        // Not a gap: the operations were withheld under `Restrict(On)`, and the policy census
        // above this level's numbers names each document and clause.
        level.nothing_to_save.push((
            name.to_owned(),
            "every edit is withheld by the document — the policy census names why".to_owned(),
        ));
        return;
    }
    let performed = match perform(document, page_id, witness_rect(crop), add_free_text, field) {
        Ok(performed) => performed,
        Err((why, was_save)) => {
            if was_save {
                level.save_refused.push((name.to_owned(), why));
            } else {
                level.nothing_to_save.push((name.to_owned(), why));
            }
            return;
        }
    };
    level.saved = level.saved.saturating_add(1);
    if performed.free_text.is_some() {
        level.free_texts_checked = level.free_texts_checked.saturating_add(1);
    }
    if performed.field.is_some() {
        level.fields_checked = level.fields_checked.saturating_add(1);
    }

    // Assertion 1, the prefix property: §7.5.6 "leaving its original contents intact".
    if !performed.written.starts_with(bytes) {
        level.prefix_failed.push(name.to_owned());
        return;
    }
    // Assertion 2, our own readback.
    if let Err(why) = read_back(&performed, password, page_id) {
        level.readback_failed.push((name.to_owned(), why));
        return;
    }
    // Assertion 3, the independent readers.
    references_see(
        name,
        stem,
        original,
        &performed,
        reference_password,
        crop,
        rotate,
        level,
    );
}

/// Opens one corpus document under ADR 0323's rule — the default user password first, then the
/// corpus's known ones — recording a refusal by reason where neither serves.
fn open_corpus_document(
    name: &str,
    bytes: Vec<u8>,
    password: &str,
    tally: &Mutex<Tally>,
) -> Option<Document> {
    let refused = match Document::open_with_password(bytes, Limits::DEFAULT, password) {
        Ok(document) => return Some(document),
        Err(SyntaxError::PasswordRequired) => "needs a password nobody has recorded".to_owned(),
        Err(SyntaxError::UnsupportedEncryption { .. }) => {
            "encryption this reader does not implement".to_owned()
        }
        Err(error) => error.to_string(),
    };
    record(tally, |t| t.refused_open.push((name.to_owned(), refused)));
    None
}

/// Opens, edits, saves and verifies one corpus document, under both `Restrict` levels.
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
        .map_or("", |(_, password)| *password);
    let Some(document) = open_corpus_document(&name, bytes.clone(), password, tally) else {
        return;
    };
    let pages = Pages::new(&document);
    let Some(page) = pages.get(0) else {
        record(tally, |t| {
            t.pageless.push((name, "no reachable page one".to_owned()));
        });
        return;
    };
    let Some(page_id) = page.id else {
        record(tally, |t| {
            t.pageless.push((
                name,
                "page one is not an indirect object, so an update cannot address it".to_owned(),
            ));
        });
        return;
    };
    let (crop, rotate) = (page.crop_box, page.rotate);

    let field = fillable_text_field(&document, &pages);
    if field.is_some() {
        record(tally, |t| {
            t.with_text_field = t.with_text_field.saturating_add(1);
        });
    }

    // The policy, asked exactly where `viewer-core` asks it: `restriction::asserted`, once per
    // operation, with `Restrict(On)` refusing whatever the document asserts.
    let against_annotating = restriction::asserted(&document, Operation::Annotate, None, None);
    let against_filling = field.as_ref().map_or_else(Vec::new, |(name, _)| {
        restriction::asserted(&document, Operation::FillInForm, Some(name), None)
    });
    for (verb, against) in [
        ("adding an annotation", &against_annotating),
        ("filling in a form field", &against_filling),
    ] {
        for restriction in against {
            let line = format!("{verb}: {}", reason(*restriction));
            record(tally, |t| t.policy_refused.push((name.clone(), line)));
        }
    }

    let reference_password = REFERENCE_PASSWORDS
        .iter()
        .find(|(known, _)| *known == name)
        .map_or(password, |(_, password)| *password);
    let mut on = Level::default();
    let mut off = Level::default();
    let annotate_on = against_annotating.is_empty();
    let fill_on = against_filling.is_empty();
    sweep(
        &name,
        "on",
        path,
        &bytes,
        &document,
        password,
        reference_password,
        page_id,
        crop,
        rotate,
        annotate_on,
        field.as_ref().filter(|_| fill_on),
        &mut on,
    );
    // The differing population only: where `On` refused nothing, `Off` would repeat the same
    // save byte for byte (and not even that for an encrypted document, ADR 0129) and judge
    // nothing new.
    if !annotate_on || (field.is_some() && !fill_on) {
        sweep(
            &name,
            "off",
            path,
            &bytes,
            &document,
            password,
            reference_password,
            page_id,
            crop,
            rotate,
            true,
            field.as_ref(),
            &mut off,
        );
    }
    if std::env::var_os("PDFVIEWER_CORPUS_TRACE").is_some() {
        eprintln!("done  {name}");
    }
    record(tally, move |t| {
        merge(&mut t.on, on);
        merge(&mut t.off, off);
    });
}

/// Folds one document's level results into the corpus-wide ones.
fn merge(into: &mut Level, from: Level) {
    into.saved = into.saved.saturating_add(from.saved);
    into.free_texts_checked = into
        .free_texts_checked
        .saturating_add(from.free_texts_checked);
    into.fields_checked = into.fields_checked.saturating_add(from.fields_checked);
    into.nothing_to_save.extend(from.nothing_to_save);
    into.save_refused.extend(from.save_refused);
    into.prefix_failed.extend(from.prefix_failed);
    into.readback_failed.extend(from.readback_failed);
    into.reference_failed.extend(from.reference_failed);
    into.disagreed.extend(from.disagreed);
    into.reference_excluded.extend(from.reference_excluded);
    into.panicked.extend(from.panicked);
}

/// Whether the machine can run both witness scripts, with the reason where it cannot.
///
/// The `viewer-ffi` C-compiler precedent: the smoke test skips loudly on a machine without the
/// references, because failing there would make it a coin toss — but the corpus instrument
/// *panics* instead, because a census silently missing its third assertion is worse than none.
fn references_available() -> Result<(), String> {
    let poppler = std::process::Command::new("python3")
        .args(["-c", "import gi; gi.require_version('Poppler', '0.18')"])
        .output();
    match poppler {
        Ok(output) if output.status.success() => {}
        Ok(_) => return Err("python3 has no poppler-glib (gi Poppler 0.18)".to_owned()),
        Err(error) => return Err(format!("python3 is not runnable: {error}")),
    }
    match std::process::Command::new("mutool").arg("-v").output() {
        Ok(_) => Ok(()),
        Err(error) => Err(format!("mutool is not runnable: {error}")),
    }
}

/// Prints one census list, capped, with its length.
fn print_list(what: &str, entries: &[(String, String)]) {
    println!("  {what}: {}", entries.len());
    for (name, why) in entries.iter().take(30) {
        println!("    {name}: {why}");
    }
    if entries.len() > 30 {
        println!("    … and {} more", entries.len().saturating_sub(30));
    }
}

/// Prints one level's numbers and failures.
fn print_level(label: &str, level: &Level) {
    println!(
        "{label}: {} saved ({} free texts, {} field values checked)",
        level.saved, level.free_texts_checked, level.fields_checked
    );
    print_list("nothing to save", &level.nothing_to_save);
    print_list(
        "save refused (§7.5.6 cannot be appended)",
        &level.save_refused,
    );
    println!("  prefix failed: {:?}", level.prefix_failed);
    print_list("readback failed", &level.readback_failed);
    print_list(
        "reference cannot witness (it cannot read the original; excluded)",
        &level.reference_excluded,
    );
    print_list("reference would not answer", &level.reference_failed);
    print_list("reference disagreed", &level.disagreed);
    print_list("panicked", &level.panicked);
}

/// The instrument. Ignored: minutes over 974 documents and two subprocesses per save; run it
/// deliberately, in release or under the gates profile.
///
/// # What is asserted now, and what is only printed
///
/// The three exact assertions bind from the first run: no panic, every save a prefix, every
/// save read back, and every reference answer either agreeing or in [`KNOWN_DISAGREEMENTS`]
/// with a diagnosis. The census *counts* — saves, policy refusals, construction refusals,
/// exclusions — are printed by reason and deliberately not ratcheted yet: ADR 0323's rule is
/// that an instrument's numbers enter `doc/todo/02` §2 only after they have held across
/// rounds, and a ratchet invented on a first run is a number nobody has watched hold.
#[test]
#[ignore = "corpus-scale; spawns poppler and mupdf per document — run explicitly, in release"]
fn every_corpus_document_saves_and_three_readers_see_the_edit() {
    if let Err(missing) = references_available() {
        panic!("the references are not runnable, so assertion 3 cannot be made: {missing}");
    }
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
            record(&tally, |t| t.on.panicked.push((name, what)));
        }
    });
    let elapsed = started.elapsed();
    let tally = tally.into_inner().expect("reported through the tally");

    println!(
        "{} documents in {:.1}s under Restrict(On), the default",
        files.len(),
        elapsed.as_secs_f64()
    );
    print_list("refused open", &tally.refused_open);
    print_list("no page to annotate", &tally.pageless);
    println!(
        "  documents with a fillable text field: {}",
        tally.with_text_field
    );
    print_list(
        "refused by policy under Restrict(On) — the policy working",
        &tally.policy_refused,
    );
    print_level("Restrict(On)", &tally.on);
    print_level(
        "Restrict(Off), over the policy-refused population only",
        &tally.off,
    );

    // The three exact assertions. 1 and 2 admit no known-failure list: a save whose prefix or
    // readback fails is wrong, full stop.
    assert!(
        tally.on.panicked.is_empty() && tally.off.panicked.is_empty(),
        "principle 1: no panic on any input"
    );
    assert!(
        tally.on.prefix_failed.is_empty() && tally.off.prefix_failed.is_empty(),
        "§7.5.6: the producer's bytes must be a prefix of every save"
    );
    assert!(
        tally.on.readback_failed.is_empty() && tally.off.readback_failed.is_empty(),
        "this tree must read back what it wrote"
    );
    // Assertion 3 admits named, diagnosed exceptions and nothing else.
    for level in [&tally.on, &tally.off] {
        for (name, difference) in level.disagreed.iter().chain(&level.reference_failed) {
            assert!(
                KNOWN_DISAGREEMENTS.iter().any(|(known, _)| known == name),
                "an undiagnosed disagreement: {name}: {difference}"
            );
        }
    }
}

/// One document through the whole instrument, un-ignored, so the witness scripts cannot rot
/// unnoticed between explicit runs.
///
/// Skips — printing why — on a machine without the corpus or the references, the same decision
/// `viewer-ffi`'s C-compiler gate made: a test that fails wherever poppler-glib is not
/// installed would be a statement about the machine, not the code.
#[test]
fn the_witness_readers_see_one_documents_saved_edit() {
    if let Err(missing) = references_available() {
        println!("skipped: {missing}");
        return;
    }
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs/160F-2019.pdf");
    if !path.exists() {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    }
    let tally = Mutex::new(Tally::default());
    examine(&path, &tally);
    let tally = tally.into_inner().expect("nothing panicked");
    assert_eq!(tally.on.saved, 1, "the fixture saves");
    assert_eq!(tally.on.fields_checked, 1, "160F-2019.pdf has a text field");
    assert!(tally.on.prefix_failed.is_empty());
    assert!(
        tally.on.readback_failed.is_empty(),
        "{:?}",
        tally.on.readback_failed
    );
    assert!(
        tally.on.reference_failed.is_empty() && tally.on.disagreed.is_empty(),
        "poppler and mupdf see the edit: {:?} {:?}",
        tally.on.reference_failed,
        tally.on.disagreed
    );
}
