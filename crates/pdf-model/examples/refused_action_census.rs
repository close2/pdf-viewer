//! Table 201's twenty action types, counted over every document this tree can reach, and what
//! this reader answers for each.
//!
//! Five ledger rows in this family are `reported` — §12.6.4.3's remote go-to, §12.6.4.6's launch,
//! §12.6.4.9's sound, §12.6.4.10's movie and §12.7.6.2's submit-form — and that status is a claim
//! with two halves: the action is not performed, *and* a person is told which one declined and
//! why. The second half is what a corpus can rank. A refusal no document ever reaches is a
//! sentence nobody will read; a refusal several documents reach is one a user meets.
//!
//! **This example exists because those counts were being written without a command.** The
//! six-hundred-and-twenty-sixth session pinned the click path for three of the five and recorded
//! the population in a comment — "of the 974 corpus documents exactly one states a `/S /Launch`
//! action" — with nothing in the tree that could produce the number again. `CLAUDE.md`'s rule is
//! that a fact which can be counted is not written down; what is written down is the command, and
//! for this population there was none. `doc/todo/01` states the same rule for a ledger note.
//!
//! What is counted is a dictionary the *standard* would call an action, and what is reported
//! beside it is what `pdf_model::action::read` made of it — so the two sides of every row come
//! from different places. Table 201's twenty names are this file's, because the table is what the
//! census is about; the verdict is the reader's own, because a list of refusals copied out of
//! `action.rs` would measure the copy.
//!
//! Three verdicts, and they are the three a row can rest on:
//!
//! - **performed** — `read` returned an action that is not [`Action::Refused`].
//! - **refused** — it returned [`Action::Refused`], carrying the sentence a host prints.
//! - **not an action** — it returned nothing, because §12.6.2 makes an entry the type requires
//!   absent. `GoToDp` without `/Dp` and `Trans` without `/Trans` are the two that reach this
//!   deliberately, and a count here is a document to look at rather than a defect by itself.
//!
//! `Thread` is the one name that appears under two verdicts, and that is the clause rather than
//! the census: §12.6.4.7's action is performed for a thread in this file and refused for one in
//! another, so a per-name total would hide the distinction the code draws.
//!
//! **Every object the cross-reference table lists is walked, and so is every dictionary inside
//! one.** An action hangs off a link annotation, an outline item, a catalog `/OpenAction`, a
//! widget's `/AA` and another action's `/Next`, so a census that visited one of those would
//! measure the walk rather than the corpus — which is `structure_destination_census`'s reason for
//! walking the objects instead. That walk alone is not enough here, and the difference is a
//! finding rather than a detail: **an action dictionary written *directly* inside its annotation
//! or outline item has no object number**, and both of the names these rows are about are written
//! that way and only that way. Bounded to numbered objects, this census reported zero `/S /GoToR`
//! and zero `/S /SubmitForm` over a corpus that states one of each.
//!
//! References are never followed while walking an object's body, because what a reference names
//! is another numbered object with its own turn — following one would count a shared action once
//! per outline item pointing at it.
//!
//! ```sh
//! cargo run --release -p pdf-model --example refused_action_census -- \
//!     doc/pdf.js/test/pdfs/*.pdf doc/corpora/*/**/*.pdf
//! ```
//!
//! An argument beginning with `@` names a file of paths, one to a line, for a population larger
//! than a command line holds.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::action::{self, Action};
use pdf_model::navigation::{Style, transition};
use pdf_syntax::{Document, Object, ObjectId};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// Table 201's action types, in the order the table lists them.
///
/// The standard's list rather than this tree's: the census asks which of the *standard's* types
/// the world writes, so a name this reader has never heard of has to be able to come back zero
/// rather than to be invisible.
const TABLE_201: [&str; 20] = [
    "GoTo",
    "GoToR",
    "GoToE",
    "GoToDp",
    "Launch",
    "Thread",
    "URI",
    "Sound",
    "Movie",
    "Hide",
    "Named",
    "SubmitForm",
    "ResetForm",
    "ImportData",
    "SetOCGState",
    "Rendition",
    "Trans",
    "GoTo3DView",
    "JavaScript",
    "RichMediaExecute",
];

/// How many nodes of one indirect object's body are walked before the census gives up on it.
///
/// A census reads whatever the corpus holds, including a file built to make a reader loop, and
/// `pdf-syntax` bounds parse depth rather than the *breadth* an object's body can have. Reaching
/// this is a document to look at rather than a number to trust; no corpus document does.
const MAX_NODES: usize = 100_000;

/// What this reader made of one action dictionary.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Verdict {
    /// An action this program carries out.
    Performed,
    /// [`Action::Refused`]: named at runtime, and not carried out.
    Refused,
    /// Nothing at all — an entry the type requires is absent.
    NotAnAction,
}

impl Verdict {
    /// The word this verdict is printed as.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Performed => "performed",
            Self::Refused => "refused",
            Self::NotAnAction => "not an action",
        }
    }
}

/// What one document contributes.
#[derive(Default)]
struct Counts {
    /// Documents that opened.
    opened: usize,
    /// Documents that did not.
    unopenable: usize,
    /// Action dictionaries, by Table 201 name and by what the reader answered.
    dictionaries: BTreeMap<(&'static str, Verdict), usize>,
    /// Documents stating at least one dictionary of each name, by name and verdict.
    documents: BTreeMap<(&'static str, Verdict), usize>,
    /// The file names stating each refused name, so that a test can name a witness.
    witnesses: BTreeMap<&'static str, BTreeSet<String>>,
    /// The two places ISO 19444-1's XFDF reaches this program from a document — Table 240 bit 6
    /// on a submit-form action and an import-data action naming an `.xfdf` file — by the file
    /// names that state each.
    ///
    /// Counted beside the verdicts rather than in a census of its own because it is the same
    /// walk: both are action dictionaries, and both are written inline often enough that a
    /// second walk bounded to numbered objects would report zero for the same reason this one
    /// was rewritten to.
    xfdf: BTreeMap<&'static str, BTreeSet<String>>,
    /// §12.6.4.4 Table 204's `/F`, by the form §7.11 gives the specification, with the documents
    /// that state one.
    ///
    /// The entry is optional and its absence is the table's own default — "the source and target
    /// share the same root document" — so what is counted here is the half that names a *file on
    /// a disk*, which is the half only a host can reach.
    roots: BTreeMap<&'static str, BTreeSet<String>>,
    /// §12.6.4.8 Table 211's `/Base`, and the documents whose `/URI` action states a reference
    /// that needs one.
    ///
    /// A document-level fact beside an action-level one, which is the pair the clause states: a
    /// partial reference is resolved against `/Base` where the catalog has one and against the
    /// location of the document itself where it does not.
    uris: BTreeMap<&'static str, BTreeSet<String>>,
    /// §12.6.4.15 Table 219's `/Trans` style, by Table 164's `/S` name, with the documents that
    /// state a transition *action* asking for it.
    ///
    /// A transition action's `/Trans` is the same Table 164 dictionary a page's own `/Trans`
    /// holds, so the verdict above (`performed`) is one fact and *which effect* the action names
    /// is another — and the latter is what §12.6.4.15's row rests a witness on. `presentation_census`
    /// asks the same of a *page's* `/Trans`; this asks it of the action, so a name Table 164 does
    /// not define — the crawl's `/Blend` — is counted where a producer wrote one.
    styles: BTreeMap<String, BTreeSet<String>>,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.opened = self.opened.saturating_add(other.opened);
        self.unopenable = self.unopenable.saturating_add(other.unopenable);
        for (key, count) in &other.dictionaries {
            bump(&mut self.dictionaries, *key, *count);
        }
        for (key, count) in &other.documents {
            bump(&mut self.documents, *key, *count);
        }
        for (name, files) in &other.witnesses {
            self.witnesses
                .entry(name)
                .or_default()
                .extend(files.clone());
        }
        for (name, files) in &other.xfdf {
            self.xfdf.entry(name).or_default().extend(files.clone());
        }
        for (name, files) in &other.roots {
            self.roots.entry(name).or_default().extend(files.clone());
        }
        for (name, files) in &other.uris {
            self.uris.entry(name).or_default().extend(files.clone());
        }
        for (name, files) in &other.styles {
            self.styles
                .entry(name.clone())
                .or_default()
                .extend(files.clone());
        }
    }
}

/// Adds `by` to one tally, saturating.
///
/// A free function rather than `*entry.or_default() += by` because that is the arithmetic
/// `clippy::arithmetic_side_effects` objects to, and a census that wrapped silently would be
/// worse than one that stopped counting.
fn bump(
    counts: &mut BTreeMap<(&'static str, Verdict), usize>,
    key: (&'static str, Verdict),
    by: usize,
) {
    let entry = counts.entry(key).or_default();
    *entry = entry.saturating_add(by);
}

fn main() {
    let paths = paths();
    let total = paths.par_iter().map(|path| document_counts(path)).reduce(
        Counts::default,
        |mut left, right| {
            left.absorb(&right);
            left
        },
    );

    println!(
        "{} document(s) opened, {} did not",
        total.opened, total.unopenable
    );
    println!("\nTable 201, by name and by what this reader answered:");
    for name in TABLE_201 {
        let mut said = false;
        for verdict in [Verdict::Performed, Verdict::Refused, Verdict::NotAnAction] {
            let dictionaries = total.dictionaries.get(&(name, verdict)).copied();
            let Some(dictionaries) = dictionaries else {
                continue;
            };
            let documents = total.documents.get(&(name, verdict)).copied().unwrap_or(0);
            println!(
                "  /S /{name:<17} {dictionaries:>5} dictionar(ies) in {documents:>4} \
                 document(s) — {}",
                verdict.as_str()
            );
            said = true;
        }
        if !said {
            println!("  /S /{name:<17}     0");
        }
    }

    println!("\nISO 19444-1's XFDF, where a document asks for it:");
    for (what, files) in &total.xfdf {
        let mut names: Vec<&str> = files.iter().map(String::as_str).collect();
        names.sort_unstable();
        println!(
            "  {what}: {} document(s) — {}",
            names.len(),
            names.join(", ")
        );
    }
    if total.xfdf.is_empty() {
        println!("  none");
    }

    println!("\n§12.6.4.4 Table 204's /F, the root document of the target:");
    say(&total.roots);

    println!("\n§12.6.4.8 Table 211's /Base, and the references that need one:");
    say(&total.uris);

    println!("\n§12.6.4.15 Table 164's /S, where a /S /Trans action names a transition:");
    if total.styles.is_empty() {
        println!("  none");
    } else {
        for (style, files) in &total.styles {
            let mut names: Vec<&str> = files.iter().map(String::as_str).collect();
            names.sort_unstable();
            let shown = names.len().min(12);
            println!(
                "  /S {style}: {} document(s) — {}{}",
                names.len(),
                names[..shown].join(", "),
                if names.len() > shown { ", …" } else { "" }
            );
        }
    }

    println!("\nWhich documents state a refused action:");
    for (name, files) in &total.witnesses {
        let mut names: Vec<&str> = files.iter().map(String::as_str).collect();
        names.sort_unstable();
        println!("  /S /{name}: {}", names.join(", "));
    }
}

/// Prints one tally of documents per key, naming a few of them.
///
/// The names are what makes a count checkable — `CLAUDE.md`'s rule is that a finding is not a
/// finding until its hits have been read against the standard — and they are capped because a
/// population of ninety thousand can put a thousand file names on one line.
fn say(tally: &BTreeMap<&'static str, BTreeSet<String>>) {
    if tally.is_empty() {
        println!("  none");
        return;
    }
    for (what, files) in tally {
        let mut names: Vec<&str> = files.iter().map(String::as_str).collect();
        names.sort_unstable();
        let shown = names.len().min(12);
        println!(
            "  {what}: {} document(s) — {}{}",
            names.len(),
            names[..shown].join(", "),
            if names.len() > shown { ", …" } else { "" }
        );
    }
}

/// The population, with `@file` expanded to the paths it lists.
fn paths() -> Vec<String> {
    let mut paths = Vec::new();
    for argument in std::env::args().skip(1) {
        if let Some(list) = argument.strip_prefix('@') {
            match std::fs::read_to_string(list) {
                Ok(text) => paths.extend(
                    text.lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .map(ToOwned::to_owned),
                ),
                Err(error) => eprintln!("{list}: {error}"),
            }
        } else {
            paths.push(argument);
        }
    }
    paths
}

/// Walks every object the cross-reference table lists in one document, and inside each of them.
fn document_counts(path: &str) -> Counts {
    let mut counts = Counts::default();
    let Ok(bytes) = std::fs::read(path) else {
        counts.unopenable = 1;
        return counts;
    };
    let Ok(document) = Document::open(bytes) else {
        counts.unopenable = 1;
        return counts;
    };
    counts.opened = 1;
    let name = path.rsplit('/').next().unwrap_or(path).to_owned();

    // Table 211's `/Base` is the catalog's and not an action's, so it is read once per document.
    let base = base_uri(&document);
    if base.is_some() {
        counts
            .uris
            .entry("a catalog stating Table 211's /Base")
            .or_default()
            .insert(name.clone());
    }

    let mut seen: BTreeSet<(&'static str, Verdict)> = BTreeSet::new();
    let numbers: Vec<u32> = document.xref().object_numbers().collect();
    for number in numbers {
        let object = document.get(ObjectId::new(number, 0));
        // Only the object's own body, never through a `Reference`: what a reference names is
        // another numbered object with its own turn in this loop, so following one here would
        // count a shared action once per outline item that points at it.
        let mut pending = vec![object];
        let mut depth = 0_usize;
        while let Some(object) = pending.pop() {
            depth = depth.saturating_add(1);
            if depth > MAX_NODES {
                break;
            }
            match object {
                Object::Array(items) => pending.extend(items),
                Object::Dictionary(dict) => {
                    if let Some(what) = xfdf_in(&document, &dict) {
                        counts.xfdf.entry(what).or_default().insert(name.clone());
                    }
                    if let Some(what) = root_in(&document, &dict) {
                        counts.roots.entry(what).or_default().insert(name.clone());
                    }
                    if let Some(what) = partial_uri_in(&document, &dict, base.is_some()) {
                        counts.uris.entry(what).or_default().insert(name.clone());
                    }
                    if let Some(style) = trans_style_in(&document, &dict) {
                        counts.styles.entry(style).or_default().insert(name.clone());
                    }
                    if let Some(verdict) = action_in(&document, &dict) {
                        bump(&mut counts.dictionaries, verdict, 1);
                        if seen.insert(verdict) {
                            bump(&mut counts.documents, verdict, 1);
                        }
                        if verdict.1 == Verdict::Refused {
                            counts
                                .witnesses
                                .entry(verdict.0)
                                .or_default()
                                .insert(name.clone());
                        }
                    }
                    pending.extend(dict.iter().map(|(_, value)| value.clone()));
                }
                Object::Stream(stream) => {
                    pending.extend(stream.dict.iter().map(|(_, value)| value.clone()));
                }
                _ => {}
            }
        }
    }
    counts
}

/// Table 201's name and this reader's verdict, for a dictionary that states an action.
///
/// **A direct dictionary is read as itself rather than through a reference**, which is what makes
/// the walk above find the two names the corpus only ever writes inline: `outlines_for_editor.pdf`
/// states `/A << /Type /Action /S /GoToR … >>` in an outline item and `webCapture.pdf` states a
/// `/S /SubmitForm` inside a widget, and neither has an object number of its own. A census bounded
/// to numbered objects reported zero for both, which is a bound reported as a finding.
fn action_in(
    document: &Document,
    dict: &pdf_syntax::Dictionary,
) -> Option<(&'static str, Verdict)> {
    // Table 201's `/Type` is optional with one permitted value, so a dictionary that states a
    // different one has not stated an action however its `/S` reads. Without this a structure
    // element's `/S` and a transparency group's would both be counted.
    if let Some(kind) = document.get_key(dict, "Type").as_name()
        && kind.as_bytes() != b"Action"
    {
        return None;
    }
    let stated = document.get_key(dict, "S");
    let stated = stated.as_name()?;
    let name = TABLE_201
        .iter()
        .find(|entry| entry.as_bytes() == stated.as_bytes())?;

    // The reader's own answer, through the reader's own entry point. `read` follows `/Next`, and
    // every action in that chain is a dictionary this walk reaches on its own, so only the first
    // answer belongs to this one.
    let verdict = match action::read(document, &Object::Dictionary(dict.clone())).first() {
        Some(Action::Refused(_)) => Verdict::Refused,
        Some(_) => Verdict::Performed,
        None => Verdict::NotAnAction,
    };
    Some((name, verdict))
}

/// Table 164's `/S`, where this dictionary is a `/S /Trans` action that names a `/Trans`.
///
/// The action's `/Trans` is Table 164's dictionary — the same one a page's own `/Trans` holds —
/// so `navigation::transition` reads it here exactly as it reads a page's, and an absent `/S`
/// reads as the table's default `R`. A name Table 164 does not define comes back under its own
/// spelling, which is how the crawl's `/Blend` earns a line rather than hiding inside `performed`.
fn trans_style_in(document: &Document, dict: &pdf_syntax::Dictionary) -> Option<String> {
    if let Some(kind) = document.get_key(dict, "Type").as_name()
        && kind.as_bytes() != b"Action"
    {
        return None;
    }
    if document.get_key(dict, "S").as_name()?.as_bytes() != b"Trans" {
        return None;
    }
    Some(style_name(&transition(document, dict)?.style))
}

/// Table 164's own spelling of a style, and the file's spelling of a name the table does not hold.
fn style_name(style: &Style) -> String {
    match style {
        Style::Split => "Split".to_owned(),
        Style::Blinds => "Blinds".to_owned(),
        Style::Box => "Box".to_owned(),
        Style::Wipe => "Wipe".to_owned(),
        Style::Dissolve => "Dissolve".to_owned(),
        Style::Glitter => "Glitter".to_owned(),
        Style::Replace => "R".to_owned(),
        Style::Fly => "Fly".to_owned(),
        Style::Push => "Push".to_owned(),
        Style::Cover => "Cover".to_owned(),
        Style::Uncover => "Uncover".to_owned(),
        Style::Fade => "Fade".to_owned(),
        Style::Unrecognised(name) if name.as_bytes().is_empty() => "(empty /S)".to_owned(),
        Style::Unrecognised(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
    }
}

/// Table 211's `/Base`, from the catalog's `/URI` dictionary — §12.6.4.8's own sentence.
fn base_uri(document: &Document) -> Option<String> {
    let catalog = document.catalog().ok()?;
    let uri = document.get_key(&catalog, "URI");
    let dict = uri.as_dict()?;
    match document.get_key(dict, "Base") {
        Object::String(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
        _ => None,
    }
}

/// Which form §7.11 gives Table 204's `/F`, where this dictionary is a `/GoToE` that states one.
fn root_in(document: &Document, dict: &pdf_syntax::Dictionary) -> Option<&'static str> {
    if let Some(kind) = document.get_key(dict, "Type").as_name()
        && kind.as_bytes() != b"Action"
    {
        return None;
    }
    let stated = document.get_key(dict, "S");
    if stated.as_name()?.as_bytes() != b"GoToE" {
        return None;
    }
    let spec = pdf_model::file_spec::FileSpec::parse(document, &document.get_key(dict, "F"))?;
    Some(match spec.url() {
        Some(url) if pdf_model::uri::is_absolute(&url) => {
            "/S /GoToE with /F, §7.11.5's URL form, absolute"
        }
        Some(_) => "/S /GoToE with /F, §7.11.5's URL form, relative",
        None => "/S /GoToE with /F, §7.11.2's path form",
    })
}

/// Whether this dictionary is a `/URI` action whose reference is partial, and what answered it.
fn partial_uri_in(
    document: &Document,
    dict: &pdf_syntax::Dictionary,
    based: bool,
) -> Option<&'static str> {
    if let Some(kind) = document.get_key(dict, "Type").as_name()
        && kind.as_bytes() != b"Action"
    {
        return None;
    }
    let stated = document.get_key(dict, "S");
    if stated.as_name()?.as_bytes() != b"URI" {
        return None;
    }
    let Object::String(bytes) = document.get_key(dict, "URI") else {
        return None;
    };
    if pdf_model::uri::is_absolute(&String::from_utf8_lossy(&bytes)) {
        return None;
    }
    Some(if based {
        "a partial /URI in a document with a /Base"
    } else {
        "a partial /URI in a document with no /Base"
    })
}

/// Whether this action dictionary asks for ISO 19444-1's XFDF, and which way.
///
/// Two clauses and two spellings. §12.7.6.2's Table 240 bit 6 — set, "field names and values shall
/// be submitted as XFDF" — is read off `/Flags` here rather than through `action::read`, because
/// the census asks what
/// *documents* state and a reader's verdict is the other column. §12.7.6.4's Table 243 names "the
/// FDF, XFDF or any other data format file", and the only thing that says which is the file name's
/// extension, which `action::data_format` reads for both clauses.
fn xfdf_in(document: &Document, dict: &pdf_syntax::Dictionary) -> Option<&'static str> {
    if let Some(kind) = document.get_key(dict, "Type").as_name()
        && kind.as_bytes() != b"Action"
    {
        return None;
    }
    let stated = document.get_key(dict, "S");
    let stated = stated.as_name()?;
    match stated.as_bytes() {
        b"SubmitForm" => {
            let flags = document.get_key(dict, "Flags").as_integer().unwrap_or(0);
            u32::try_from(flags)
                .ok()
                .map(action::SubmitFlags)
                .filter(|flags| flags.xfdf())
                .map(|_| "/S /SubmitForm with Table 240 bit 6 (XFDF)")
        }
        b"ImportData" => match action::read(document, &Object::Dictionary(dict.clone())).first() {
            Some(Action::ImportData(import)) if import.format == action::DataFormat::Xfdf => {
                Some("/S /ImportData naming an .xfdf file")
            }
            _ => None,
        },
        _ => None,
    }
}
