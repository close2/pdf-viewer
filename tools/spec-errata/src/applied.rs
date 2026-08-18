//! The seventeenth sweep: does a place that **records** an erratum quote the text it removed?
//!
//! # The shape it exists for
//!
//! The five-hundred-and-ninetieth session found the §14.8.4.7.2 ledger row naming Errata
//! Collection 3's Issue #437 — recorded there since the four-hundred-and-eighteenth — and then
//! quoting the sentence that erratum struck out, two sentences later, while four places in
//! `crates/` quoted the same struck sentence as current text. Its lesson is the whole of this
//! module: **a row that records an erratum is not a row that has applied it.**
//!
//! Nothing in this tree looked for that. [`crate::landings`] — `check` — asks whether a
//! quotation lands on struck text and knows nothing about whether the writer had read the
//! erratum, so a place that names the erratum and then quotes the old words reads exactly like a
//! place that never heard of it, and it sorts into whichever bucket [`crate::Landing::in_clause`]
//! puts it in. A row that names the erratum looks maximally diligent, which is precisely why
//! nobody re-reads it.
//!
//! # The discriminator
//!
//! An erratum in these files is two marks over one passage — a `StrikeOut` over the words it
//! removes and a `Caret` whose `/Contents` is what replaces them, joined by Table 172's `/IRT`
//! (see [`crate::Note::change`]) — so a place naming `Issue #NNN` is standing on one side of that
//! change or on the other, and **the erratum supplies both sides of the comparison**.
//!
//! That is what makes this sweep sharper than every other one over quotations, and the
//! difference is worth stating: `check` has to *infer* which clause a quotation belongs to from
//! the nearest citation above it, and says so by calling its own buckets a sort order rather than
//! a verdict. Here the erratum is named as **data**, by the writer, inside the place itself.
//! Nothing is guessed, and a hit means one thing: **a quotation, in a place that names Issue
//! #NNN, matching what #NNN struck out and not matching what #NNN put there instead.**
//!
//! # The noise, printed rather than filtered
//!
//! - **A correction quoting the wording it retired.** The oldest false positive in this family
//!   — `doc/todo/01`'s fourth sweep, and the shape ADRs 0336, 0345 and 0372 each record for
//!   their own — and here it is the *commonest* hit by construction, because the honest way to
//!   record an erratum is to say what the sentence used to be. §9.10.3's row does exactly that,
//!   in as many words. It is marked [`Hit::history`] from the prose either side of the quotation
//!   ([`HISTORY`], [`HISTORY_WINDOW`]) and still printed: a note that *only* looks like
//!   history is where a live defect has hidden before, and the §14.8.4.7.2 row this sweep was
//!   built for reads like a correction three sentences above the stale quotation.
//! - **`doc/errata-read.md` is that shape from end to end**, being the reading itself rather than
//!   a claim about the code, so its hits are counted apart ([`Report::reading`]) instead of being
//!   read as a population of defects.
//! - **A `#NNN` that names no erratum.** This project writes `#` before a number for other
//!   things — another project's issue, a footnote, a count. A number no erratum carries is
//!   dropped and counted ([`Report::unknown`]), so that a clean run says how much of the
//!   population it was able to judge rather than only that it found nothing.
//! - **An erratum that deletes without replacing** has no replacement side at all, so every
//!   quotation of its struck text is a hit. That is right rather than a defect in the instrument
//!   — there is nothing for the writer to have moved *to* — and it is worth knowing when reading
//!   one.
//!
//! # Why it is not a gate
//!
//! [`crate::landings`]' reason, unchanged: this parses fourteen PDFs, and `doc/todo/48` is
//! explicit that a gate must not. It runs in seconds and its output is read.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::{Error, Note, Quoted, Role};

/// How many characters either side of a quotation are read for the marks of a correction.
///
/// **Either side, and that is not symmetry for its own sake.** This project writes a correction
/// both ways round: `standard.rs` says "[t]he clause used to say so in a `shall` and no longer
/// does" and then quotes the retired sentence, and `appearance.rs` sets the retired blockquote
/// first and explains underneath that "the blockquote above is the old one". A window that
/// looked only backwards would mark the first and miss the second, and the second is the shape a
/// module doc takes.
///
/// The size is a window rather than the whole place, and the reason is what it costs at each
/// end: a ledger note runs to two thousand characters and holds a dozen corrections, so marking
/// the *note* would mark every quotation in the ledger, while a sentence is too little for a
/// correction this project writes as a paragraph. Four hundred is measured rather than chosen —
/// it is the largest window under which the §14.8.4.7.2 row this sweep was built for still reads
/// as unmarked, and the smallest under which `standard.rs`'s and `appearance.rs`'s corrections
/// both read as marked.
pub const HISTORY_WINDOW: usize = 400;

/// The phrases that make the prose around a quotation read like a correction.
///
/// Deliberately different from [`conformance::blockers::HISTORY`], which this could otherwise
/// have borrowed. That list carries `said` and `this row`, which are the ordinary connective
/// tissue of a ledger note — the §14.8.4.7.2 row's stale quotation was introduced by "that is
/// this row's one reader-side requirement" — so borrowing it would have marked the one defect
/// this sweep exists for as noise. What is here is a phrase that retires *the quoted words*.
///
/// `struck` and `strikes` are bare rather than `struck out`, because this tree writes the verb
/// with every object a strikeout can take — "strikes the whole sentence", "struck outright",
/// "strikes §9.6.2.2's". They are the commonest mark by a distance, which is what one would
/// expect: a writer recording an erratum says what it removed.
pub const HISTORY: [&str; 11] = [
    "used to",
    "was corrected",
    "retired",
    "no longer",
    "struck",
    "strikes",
    "amended",
    "stays",
    "the old one",
    "stood",
    "before the erratum",
];

/// The one document whose every quotation of retired text is correct writing.
///
/// `doc/errata-read.md` *is* the reading — one row per struck passage, with the struck words
/// beside the verdict — so a sweep that read it as a population of defects would report the
/// instrument's own record back at it. Counted rather than dropped, because a count is what says
/// the population was looked at.
pub const THE_READING: &str = "errata-read.md";

/// One change Errata Collection 3 states: the words it removes and the words it puts there.
#[derive(Debug, Clone)]
pub struct Erratum {
    /// The `Issue #NNN` numbers its `/Subj` names, without the mark.
    ///
    /// A list because this collection states one change under two issue numbers where two
    /// reports were resolved together — `Issue #47 and #48` over §9.6.2.1, `Issue #72 and #719`
    /// over §14.8.6.3 — and a place citing either of them is citing this change.
    pub issues: Vec<String>,
    /// The document it is in, without its directory.
    pub document: String,
    /// The page it is on, counting from one.
    pub page: usize,
    /// The section §12.3.3's outline puts that page in.
    pub section: Option<String>,
    /// What it strikes out, as the strikeout's own quadrilaterals cover it.
    pub struck: Vec<String>,
    /// What the group's carets put in its place, in the order the file states them.
    pub replacement: Vec<String>,
}

impl Erratum {
    /// Whether it strikes out enough words for a quotation to be compared against.
    ///
    /// [`crate::MIN_WORDS`], the same threshold `check` uses and for the same reason: below it a
    /// passage is a phrase two different sentences share rather than a sentence.
    #[must_use]
    pub fn comparable(&self) -> bool {
        self.struck.iter().any(|passage| long_enough(passage))
    }
}

/// One quotation inside a place, with where it sits and which population it belongs to.
#[derive(Debug, Clone)]
pub struct Span {
    /// Its byte offset in the place's own text, which is what the history window is measured
    /// back from.
    pub at: usize,
    /// Which of this project's populations of quotation it is in.
    pub kind: Quoted,
    /// The quoted words.
    pub text: String,
}

/// One piece of this project's prose that may name an erratum, with the quotations inside it.
///
/// The unit matters as much as the match, and it is the smallest thing that can hold both halves
/// of the claim: a ledger row's note, a run of comment lines, a Markdown paragraph or table row.
/// Anything wider — a whole file — would put every issue number beside every quotation and
/// manufacture hits; anything narrower would separate the `Issue #NNN` from the words it governs,
/// which is exactly the pairing this sweep is about.
#[derive(Debug, Clone)]
pub struct Place {
    /// Where it is, as a reader would go to it.
    pub location: String,
    /// Its whole text, markers stripped and lines joined.
    pub text: String,
    /// The quotations inside it.
    pub quotations: Vec<Span>,
}

/// One place that names an erratum and quotes the words that erratum removed.
#[derive(Debug, Clone)]
pub struct Hit {
    /// Where the place is.
    pub location: String,
    /// Which population the quotation is in.
    pub kind: Quoted,
    /// The issue number the place names, as it names it.
    pub issue: String,
    /// The quotation, as this tree wrote it.
    pub quotation: String,
    /// The passage the erratum struck out.
    pub struck: String,
    /// What the erratum puts in its place, empty where it only deletes.
    pub replacement: Vec<String>,
    /// Where the erratum is: document, page and section.
    pub erratum: String,
    /// Whether the prose in front of the quotation reads like a correction — the known
    /// false-positive shape, marked rather than dropped.
    pub history: bool,
    /// Whether the place is in [`THE_READING`], whose every quotation of retired text is right.
    pub reading: bool,
}

/// What one run of the sweep read, and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// How many changes the errata state, over all the documents read.
    pub errata: usize,
    /// How many of those strike out enough words to compare a quotation against.
    pub comparable: usize,
    /// How many distinct issue numbers those carry.
    pub issues: usize,
    /// How many places were scanned.
    pub places: usize,
    /// How many of them name at least one erratum.
    pub citing: usize,
    /// How many `#NNN` tokens named no erratum in these documents.
    pub unknown: usize,
    /// How many quotation-against-erratum comparisons were made.
    pub compared: usize,
    /// Quotations matching the erratum's own replacement: the erratum is applied here.
    pub applied: usize,
    /// Quotations matching both sides, which the change did not alter enough to tell apart.
    pub both: usize,
    /// The hits, worst first — a live quotation of retired text before a correction quoting it.
    pub hits: Vec<Hit>,
}

impl Report {
    /// How many hits are in [`THE_READING`] rather than in a claim about this tree.
    #[must_use]
    pub fn reading(&self) -> usize {
        self.hits.iter().filter(|hit| hit.reading).count()
    }

    /// How many hits carry no mark of a correction — the list to read first.
    #[must_use]
    pub fn unmarked(&self) -> usize {
        self.hits
            .iter()
            .filter(|hit| !hit.history && !hit.reading)
            .count()
    }
}

/// The changes the errata state, one per §12.5.6.2 group.
///
/// A strikeout and the caret that replaces it are one change stated in two marks, and
/// [`Note::change`] is what joins them. Replies are left out: §12.5.6.4's state annotations say
/// how far a change got and state no text.
///
/// Only a note whose `/Subj` names an issue number is collected, because an annotation nothing
/// can be cited by is an annotation this sweep cannot be asked about.
#[must_use]
pub fn errata(notes: &[Note]) -> Vec<Erratum> {
    let mut grouped: BTreeMap<(String, pdf_syntax::ObjectId), Vec<&Note>> = BTreeMap::new();
    for note in notes {
        if note.role == Role::Reply {
            continue;
        }
        let (Some(change), Some(subject)) = (note.change, note.subject.as_deref()) else {
            continue;
        };
        if issues_named(subject).is_empty() {
            continue;
        }
        grouped
            .entry((note.document.clone(), change))
            .or_default()
            .push(note);
    }
    grouped
        .into_values()
        .filter_map(|members| {
            let first = members.first()?;
            let mut issues: Vec<String> = Vec::new();
            for member in &members {
                for issue in member
                    .subject
                    .as_deref()
                    .map(issues_named)
                    .unwrap_or_default()
                {
                    if !issues.contains(&issue) {
                        issues.push(issue);
                    }
                }
            }
            Some(Erratum {
                issues,
                document: first.document.clone(),
                page: first.page,
                section: first.section.clone(),
                struck: members
                    .iter()
                    .filter(|member| member.retires_text())
                    .filter_map(|member| member.covered.clone())
                    .collect(),
                // A caret is what these files put a replacement in. A `Text` annotation in the
                // same group is the editor's instruction — "Insert a new table (Table 125a)
                // after the bulleted list as follows" — which is prose about the change rather
                // than the standard's own new words, and reading it as a replacement would let
                // an instruction acquit a stale quotation.
                replacement: members
                    .iter()
                    .filter(|member| member.subtype == "Caret")
                    .filter_map(|member| member.contents.clone())
                    .collect(),
            })
        })
        .collect()
}

/// Every issue number a piece of text names, in the order it names them.
///
/// A `#` followed by digits, where nothing or a non-alphanumeric character precedes the mark, so
/// that a Rust attribute (`#[expect`), a Markdown heading (`# `) and a colour (`#00ff00`, whose
/// first two characters are digits and whose third is not) all fail it for reasons that are about
/// the token rather than about a list of exceptions. Four digits at most: this collection numbers
/// its issues in the hundreds, and a longer run is a quantity.
///
/// **What makes the token a citation is not this function**, which only finds the shape: it is
/// that some erratum carries the number, which [`sweep`] asks and counts the misses of.
#[must_use]
pub fn issues_named(text: &str) -> Vec<String> {
    /// The longest run of digits that is still an issue number rather than a quantity.
    const MOST_DIGITS: usize = 4;

    let mut found: Vec<String> = Vec::new();
    let characters: Vec<char> = text.chars().collect();
    for (index, character) in characters.iter().enumerate() {
        if *character != '#' {
            continue;
        }
        if index
            .checked_sub(1)
            .and_then(|back| characters.get(back))
            .is_some_and(|before| before.is_alphanumeric())
        {
            continue;
        }
        let digits: String = characters
            .iter()
            .skip(index.saturating_add(1))
            .take(MOST_DIGITS)
            .take_while(|candidate| candidate.is_ascii_digit())
            .collect();
        if digits.is_empty() {
            continue;
        }
        // A trailing letter makes the run part of a longer word — `#2a` is a label rather than
        // an issue — and a fifth digit makes it a quantity.
        let after = characters.get(index.saturating_add(1).saturating_add(digits.len()));
        if after.is_some_and(|candidate| candidate.is_alphanumeric()) {
            continue;
        }
        if !found.contains(&digits) {
            found.push(digits);
        }
    }
    found
}

/// Runs the sweep: every place, against every erratum it names.
#[must_use]
pub fn sweep(errata: &[Erratum], places: &[Place]) -> Report {
    let comparable: Vec<&Erratum> = errata
        .iter()
        .filter(|erratum| erratum.comparable())
        .collect();
    let issues: BTreeSet<&str> = errata
        .iter()
        .flat_map(|erratum| erratum.issues.iter().map(String::as_str))
        .collect();
    let known: BTreeSet<&str> = issues.clone();
    let mut report = Report {
        errata: errata.len(),
        comparable: comparable.len(),
        issues: issues.len(),
        places: places.len(),
        ..Report::default()
    };

    for place in places {
        let named = issues_named(&place.text);
        if named.is_empty() {
            continue;
        }
        let mut cited = false;
        for issue in &named {
            if !known.contains(issue.as_str()) {
                report.unknown = report.unknown.saturating_add(1);
                continue;
            }
            cited = true;
            for erratum in comparable
                .iter()
                .filter(|erratum| erratum.issues.contains(issue))
            {
                judge(&mut report, place, issue, erratum);
            }
        }
        if cited {
            report.citing = report.citing.saturating_add(1);
        }
    }
    // A live quotation of retired text before a correction quoting one, and this project's own
    // record of the reading last: the order is the reading list.
    report.hits.sort_by_key(|hit| {
        (
            hit.reading,
            hit.history,
            hit.location.clone(),
            hit.issue.clone(),
        )
    });
    report
}

/// Judges every quotation of one place against one erratum it names.
fn judge(report: &mut Report, place: &Place, issue: &str, erratum: &Erratum) {
    for span in &place.quotations {
        let struck = erratum
            .struck
            .iter()
            .filter(|passage| long_enough(passage))
            .find(|passage| about(&span.text, passage));
        let replaced = erratum
            .replacement
            .iter()
            .filter(|passage| long_enough(passage))
            .any(|passage| about(&span.text, passage));
        report.compared = report.compared.saturating_add(1);
        match (struck, replaced) {
            (Some(_), true) => report.both = report.both.saturating_add(1),
            (None, true) => report.applied = report.applied.saturating_add(1),
            (None, false) => {}
            (Some(passage), false) => report.hits.push(Hit {
                location: place.location.clone(),
                kind: span.kind,
                issue: issue.to_owned(),
                quotation: span.text.clone(),
                struck: passage.clone(),
                replacement: erratum.replacement.clone(),
                erratum: format!(
                    "{} p.{} {}",
                    erratum.document,
                    erratum.page,
                    erratum.section.as_deref().unwrap_or("(no section)")
                ),
                history: reads_like_history(&place.text, span.at, span.text.len()),
                reading: place.location.contains(THE_READING),
            }),
        }
    }
}

/// Whether a quotation is about a passage, by the comparison two extractions of one sentence can
/// share.
///
/// [`crate::overlaps`], which is `check`'s: both sides are extractions of the same glyphs by
/// different programs, and the foldings that survive are the ones ADRs 0253, 0375 and this
/// round's own have each paid for. Asking the question with a second comparison would make this
/// sweep's *level* incomparable with `check`'s, which is ADR 0360's argument about the fifth
/// sweep one population over.
fn about(quotation: &str, passage: &str) -> bool {
    crate::overlaps(quotation, &crate::squeezed(passage))
}

/// Whether a passage carries enough words to be compared at all.
fn long_enough(passage: &str) -> bool {
    passage.split_whitespace().count() >= crate::MIN_WORDS
}

/// Whether the prose around a quotation retires the words it quotes.
///
/// [`HISTORY_WINDOW`] characters either side, lower-cased, against [`HISTORY`]. The window is
/// taken on character boundaries so that a place written in this project's own typography — em
/// dashes, section signs, curly marks — cannot slice one in half.
fn reads_like_history(text: &str, at: usize, length: usize) -> bool {
    let start = boundary_before(text, at, HISTORY_WINDOW);
    let end = boundary_after(text, at.saturating_add(length), HISTORY_WINDOW);
    let window = text.get(start..end).unwrap_or(text).to_lowercase();
    HISTORY.iter().any(|phrase| window.contains(phrase))
}

/// The character boundary at most `back` characters before `at`.
fn boundary_before(text: &str, at: usize, back: usize) -> usize {
    text.get(..at).map_or(0, |before| {
        before
            .char_indices()
            .rev()
            .take(back)
            .last()
            .map_or(at, |(start, _)| start)
    })
}

/// The character boundary at most `forward` characters after `at`.
fn boundary_after(text: &str, at: usize, forward: usize) -> usize {
    text.get(at..).map_or(text.len(), |after| {
        after
            .char_indices()
            .take(forward.saturating_add(1))
            .last()
            .map_or(text.len(), |(end, _)| at.saturating_add(end))
    })
}

/// Every ledger row's note, as a place.
///
/// # Errors
///
/// [`Error::Ledger`] where the ledger cannot be read or parsed.
pub fn ledger_places(ledger: &Path) -> Result<Vec<Place>, Error> {
    let rows = conformance::ledger::Ledger::read(ledger)?;
    Ok(rows
        .rows
        .iter()
        .filter_map(|row| {
            let note = row.note.as_deref()?;
            Some(Place {
                location: format!(
                    "{}:{} (§{}, {})",
                    ledger.display(),
                    row.line,
                    row.clause,
                    row.status.as_str()
                ),
                quotations: marked(note, Quoted::LedgerNote),
                text: note.to_owned(),
            })
        })
        .collect())
}

/// Every run of comment lines under `roots`, as a place.
///
/// # Errors
///
/// [`Error::Sources`] where a root cannot be walked, [`Error::Unreadable`] where a file under
/// one cannot be read.
pub fn source_places(roots: &[PathBuf]) -> Result<Vec<Place>, Error> {
    let files = conformance::citation::rust_sources(roots)
        .map_err(|error| Error::Sources(roots.first().cloned().unwrap_or_default(), error))?;
    let mut places = Vec::new();
    for file in files {
        let shown = file.to_string_lossy().replace('\\', "/");
        let source = std::fs::read_to_string(&file)
            .map_err(|error| Error::Unreadable(file.clone(), error))?;
        for (line, place) in comment_runs(&source) {
            places.push(Place {
                location: format!("{shown}:{line}"),
                ..place
            });
        }
    }
    Ok(places)
}

/// Every block of every Markdown document under `directory` that this project wrote, as a place.
///
/// [`conformance::prose::blocks`] decides what a block is, and its rule is the one this sweep
/// needs without amendment: a table row is its own block. `doc/errata-read.md` is a table of a
/// hundred and twenty errata, one per row, and reading the whole table as one place would put
/// every issue number beside every struck passage in it.
///
/// # Errors
///
/// [`Error::Documents`] where `directory` cannot be walked, [`Error::Unreadable`] where a
/// document under it cannot be read.
pub fn document_places(directory: &Path) -> Result<Vec<Place>, Error> {
    let mut places = Vec::new();
    for file in conformance::prose::documents(directory)? {
        let shown = file.to_string_lossy().replace('\\', "/");
        if shown.starts_with(conformance::retired::NOT_SWEPT) {
            continue;
        }
        let text = std::fs::read_to_string(&file)
            .map_err(|error| Error::Unreadable(file.clone(), error))?;
        for block in conformance::prose::blocks(&text) {
            let quotations = match block.shape {
                conformance::prose::Shape::Blockquote => {
                    if long_enough(&block.text) {
                        vec![Span {
                            at: 0,
                            kind: Quoted::Document,
                            text: block.text.clone(),
                        }]
                    } else {
                        Vec::new()
                    }
                }
                _ => marked(&block.text, Quoted::Document),
            };
            places.push(Place {
                location: format!("{shown}:{}", block.line),
                text: block.text,
                quotations,
            });
        }
    }
    Ok(places)
}

/// The quoted spans of a piece of prose, with where each one sits in it.
///
/// [`conformance::quote::quoted_spans`] is the rule, shared with every other sweep over this
/// project's prose; what is added here is the offset, which the history window is measured back
/// from. The spans come out in the order they were written, so one forward scan finds them all,
/// and a span the scan cannot find again — which nothing has produced — is placed at the start
/// rather than dropped.
fn marked(text: &str, kind: Quoted) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut from = 0usize;
    for span in crate::quoted_spans(text) {
        let at = text.get(from..).and_then(|rest| rest.find(span.as_str()));
        let at = at.map_or(from, |offset| from.saturating_add(offset));
        from = at.saturating_add(span.len());
        spans.push(Span {
            at,
            kind,
            text: span,
        });
    }
    spans
}

/// Every run of consecutive comment lines in a Rust source, as a place.
///
/// **The run is the whole comment, blockquotes included**, which is where this differs from
/// [`crate::landings`]' scanner and why it needs one of its own. That one ends a block at a
/// blockquote line, because two quotations either side of one are two quotations; here the
/// question is what the *comment* claims, and a doc comment that names an erratum in its prose
/// and quotes the standard in a blockquote underneath is one claim made in two shapes. Ending
/// the run at the blockquote would separate the erratum from the words it governs, which is the
/// pairing this sweep exists to check.
///
/// A fenced example inside a doc comment is skipped, for the reason the citation scanner skips
/// one: a string literal in a sample is data rather than a claim.
fn comment_runs(source: &str) -> Vec<(usize, Place)> {
    let mut runs: Vec<(usize, Place)> = Vec::new();
    let mut run = Run::default();
    for (index, line) in source.lines().enumerate() {
        match comment_body(line) {
            Some((is_doc, body)) => run.take(index.saturating_add(1), is_doc, body),
            None => run.close(&mut runs),
        }
    }
    run.close(&mut runs);
    runs
}

/// The text of one comment line, and whether it is a doc comment.
///
/// `///` and `//!` are tried before `//`, since both start with it.
fn comment_body(line: &str) -> Option<(bool, &str)> {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix("///")
        .or_else(|| trimmed.strip_prefix("//!"))
        .map(|body| (true, body.trim()))
        .or_else(|| trimmed.strip_prefix("//").map(|body| (false, body.trim())))
}

/// One run of comment lines under construction, and the stretch of like-shaped lines inside it.
///
/// A *stretch* is what decides how a quotation is delimited — a blockquote is one quotation
/// whole, and prose is whatever pairs of marks it holds — while the *run* is what decides which
/// erratum a quotation is beside. Keeping both is why this is a small state machine rather than
/// two passes: the run's text is what [`issues_named`] reads and what the history window is
/// measured back through, and the stretch's offset into it is what makes the window findable.
#[derive(Debug, Default)]
struct Run {
    /// The 1-based line the run starts on.
    start: usize,
    /// Everything the run says, markers stripped and joined with one space.
    text: String,
    /// What it quotes, with each span's offset into [`Self::text`].
    quotations: Vec<Span>,
    /// The lines of the stretch not yet closed.
    stretch: Vec<String>,
    /// Where that stretch begins in [`Self::text`].
    at: usize,
    /// Whether it is a blockquote.
    quoted: bool,
    /// Whether it is a doc comment rather than an ordinary one.
    doc: bool,
    /// Whether the lines are inside a fenced example, which is data rather than a claim.
    fenced: bool,
}

impl Run {
    /// Adds one comment line.
    fn take(&mut self, line: usize, is_doc: bool, body: &str) {
        if self.text.is_empty() && self.stretch.is_empty() {
            self.start = line;
            self.doc = is_doc;
        }
        if body.starts_with("```") {
            self.flush();
            self.fenced = !self.fenced;
            return;
        }
        if self.fenced {
            return;
        }
        let is_quote = body.starts_with('>');
        let body = body.trim_start_matches('>').trim();
        if body.is_empty() {
            return;
        }
        if self.stretch.is_empty() || is_quote != self.quoted || is_doc != self.doc {
            self.flush();
            self.quoted = is_quote;
            self.doc = is_doc;
            self.at = self.text.len();
        }
        if !self.text.is_empty() {
            self.text.push(' ');
        }
        self.text.push_str(body);
        self.stretch.push(body.to_owned());
    }

    /// Closes the current stretch, collecting what it quotes.
    fn flush(&mut self) {
        if self.stretch.is_empty() {
            return;
        }
        let joined = std::mem::take(&mut self.stretch).join(" ");
        if self.quoted {
            if long_enough(&joined) {
                self.quotations.push(Span {
                    at: self.at,
                    kind: Quoted::Blockquote,
                    text: joined,
                });
            }
            return;
        }
        let kind = if self.doc {
            Quoted::Prose
        } else {
            Quoted::Comment
        };
        for span in marked(&joined, kind) {
            self.quotations.push(Span {
                at: self.at.saturating_add(span.at),
                ..span
            });
        }
    }

    /// Ends the run at the first line that is not a comment, and starts a fresh one.
    fn close(&mut self, runs: &mut Vec<(usize, Place)>) {
        self.flush();
        if self.text.is_empty() {
            *self = Self::default();
            return;
        }
        runs.push((
            self.start,
            Place {
                location: String::new(),
                text: std::mem::take(&mut self.text),
                quotations: std::mem::take(&mut self.quotations),
            },
        ));
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One erratum, as the two marks these files state it in.
    fn change(issue: &str, struck: &str, replacement: &str) -> Erratum {
        Erratum {
            issues: vec![issue.to_owned()],
            document: "ISO_32000-2_sponsored_EC3.pdf".to_owned(),
            page: 779,
            section: Some("14.8.4.7.2 Annot and Form".to_owned()),
            struck: vec![struck.to_owned()],
            replacement: if replacement.is_empty() {
                Vec::new()
            } else {
                vec![replacement.to_owned()]
            },
        }
    }

    /// Table 368's `Form` description, as Errata Collection 3's Issue #437 changes it.
    fn form_description() -> Erratum {
        change(
            "437",
            "Either an association between content enclosed by the Form structure element and a \
             corresponding widget annotation or a mechanism to include a widget annotation in the \
             structure tree.",
            "Encloses a PDF widget annotation and associated content, if any.",
        )
    }

    fn place(text: &str) -> Place {
        Place {
            location: "doc/conformance/ledger.toml:1 (§14.8.4.7.2, partial)".to_owned(),
            quotations: marked(text, Quoted::LedgerNote),
            text: text.to_owned(),
        }
    }

    /// **The defect this sweep was built for**, planted: the §14.8.4.7.2 row as it stood before
    /// the five-hundred-and-ninetieth session — naming Issue #437 and then quoting the sentence
    /// #437 struck out, in `CLAUDE.md`'s own `[e]` spelling of an altered first letter.
    ///
    /// The row records the erratum three sentences above the stale quotation, which is what makes
    /// it invisible to a reader and to every other sweep: it looks maximally diligent.
    #[test]
    fn a_row_that_records_an_erratum_and_quotes_what_it_struck_is_named() {
        let note = "The word was *association* until the four-hundred-and-eighteenth session and \
                    Errata Collection 3 makes it enclosure (Issue #437). `Form` reaches a person \
                    as a control since the five-hundred-and-third session, and that is this row's \
                    one reader-side requirement: the type is \"[e]ither an association between \
                    content enclosed by the Form structure element and a corresponding widget \
                    annotation or a mechanism to include a widget annotation in the structure \
                    tree\", with one widget, so one control.";
        let report = sweep(&[form_description()], &[place(note)]);
        assert_eq!(report.citing, 1, "the note names #437");
        assert_eq!(report.hits.len(), 1, "and quotes what #437 struck out");
        let hit = report.hits.first().expect("one hit");
        assert_eq!(hit.issue, "437");
        assert!(
            !hit.history,
            "nothing in front of the quotation retires the words it quotes, which is the \
             property that makes this a defect rather than a correction"
        );
    }

    /// The same row after the correction: the quotation is the erratum's own replacement, so the
    /// erratum is applied here and there is nothing to report.
    #[test]
    fn a_row_quoting_the_replacement_has_applied_the_erratum() {
        let note = "Errata Collection 3's Issue #437 makes the type \"[e]ncloses a PDF widget \
                    annotation and associated content, if any\", which is one widget and so one \
                    control.";
        let report = sweep(&[form_description()], &[place(note)]);
        assert!(report.hits.is_empty());
        assert_eq!(report.applied, 1);
    }

    /// A correction quoting the wording it retired is the oldest false positive in this family,
    /// and it is marked rather than dropped.
    #[test]
    fn a_correction_quoting_the_wording_it_retired_is_marked() {
        let note = "Issue #437 strikes out the sentence this row used to quote — \"[e]ither an \
                    association between content enclosed by the Form structure element and a \
                    corresponding widget annotation or a mechanism to include a widget annotation \
                    in the structure tree\" — and puts an enclosure in its place.";
        let report = sweep(&[form_description()], &[place(note)]);
        assert_eq!(report.hits.len(), 1, "printed rather than filtered");
        assert!(report.hits.first().expect("one hit").history);
    }

    /// An erratum that only deletes has no replacement side, so a quotation of what it removed
    /// is a hit with nothing to have moved to.
    #[test]
    fn an_erratum_that_only_deletes_still_answers() {
        let erratum = change(
            "529",
            "the EmbeddedFiles name tree shall contain exactly one entry",
            "",
        );
        let note = "Issue #529 governs it: \"the EmbeddedFiles name tree shall contain exactly \
                    one entry\" is what a wrapper states.";
        let report = sweep(&[erratum], &[place(note)]);
        assert_eq!(report.hits.len(), 1);
        assert!(report.hits.first().expect("one hit").replacement.is_empty());
    }

    /// A number no erratum carries is not a citation, and the count of those is what lets a
    /// clean run say what it was clean over.
    #[test]
    fn a_number_no_erratum_carries_is_counted_rather_than_compared() {
        let note = "The experiment is another project's issue #1234, and Issue #437 is the \
                    clause.";
        let report = sweep(&[form_description()], &[place(note)]);
        assert_eq!(report.unknown, 1, "no erratum carries 1234");
        assert_eq!(report.citing, 1, "and #437 is this collection's");
        // And a number welded to a word is not a token at all: `mozilla/pdf.js#12725` is one
        // reference, which is how this project writes another repository's issue.
        assert!(issues_named("`mozilla/pdf.js#12725`").is_empty());
    }

    /// The token has to be an issue number rather than any `#`: an attribute, a heading, a
    /// colour and a label all fail for reasons about the token itself.
    #[test]
    fn only_an_issue_number_is_read_as_one() {
        assert_eq!(issues_named("Issue #437 and #133"), vec!["437", "133"]);
        assert!(issues_named("#[expect(clippy::pedantic)]").is_empty());
        assert!(issues_named("# A heading, and ## another").is_empty());
        assert!(issues_named("the colour #00ff00").is_empty());
        assert!(issues_named("label #2a").is_empty());
        assert!(issues_named("a run of #12345 digits").is_empty());
    }

    /// A doc comment that names the erratum in its prose and quotes the standard in a blockquote
    /// underneath is one claim in two shapes, so the run must not end at the blockquote.
    #[test]
    fn a_comment_run_keeps_its_blockquote_with_the_prose_that_names_the_erratum() {
        // Written as lines rather than as one literal so that no line of *this* file begins
        // with a doc-comment marker: the conformance gate would otherwise read the fixture's
        // blockquote as a quotation this crate attributes to a clause.
        let source = [
            "/// Errata Collection 3's Issue #437 rewrites this.",
            "///",
            "/// > Either an association between content enclosed by the Form structure element \
             and a corresponding widget annotation or a mechanism to include a widget annotation \
             in the structure tree.",
            "fn f() {}",
        ]
        .join("\n");
        let runs = comment_runs(&source);
        assert_eq!(runs.len(), 1, "one comment, one place");
        let (line, place) = runs.into_iter().next().expect("one run");
        assert_eq!(line, 1);
        let report = sweep(&[form_description()], &[place]);
        assert_eq!(report.hits.len(), 1);
        assert_eq!(
            report.hits.first().expect("one hit").kind,
            Quoted::Blockquote
        );
    }

    /// A fenced example inside a doc comment is data rather than a claim.
    #[test]
    fn a_fenced_example_is_not_a_quotation() {
        let source = [
            "/// Issue #437, demonstrated:",
            "/// ```text",
            "/// \"Either an association between content enclosed by the Form structure element \
             and a corresponding widget annotation or a mechanism to include a widget annotation \
             in the structure tree.\"",
            "/// ```",
            "fn f() {}",
        ]
        .join("\n");
        let runs = comment_runs(&source);
        let (_, place) = runs.into_iter().next().expect("one run");
        assert!(sweep(&[form_description()], &[place]).hits.is_empty());
    }
}
