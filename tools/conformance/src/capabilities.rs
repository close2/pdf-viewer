//! The third sweep: a reason that names a capability the program lacks, checked against the
//! tree's own vocabulary.
//!
//! # The shape it exists for
//!
//! A ledger note that explains a debt by what the program *is* rather than by what the standard
//! leaves open — "this crate has no events", "a window with scrolling and zoom, which this
//! program does not have" — decays at the pace of the whole program, and nothing fires when the
//! capability arrives. §12.6.3's row kept its sentence for forty-one sessions after
//! `Command::Pointer` landed; §12.3.2.1's for sixty-nine after scrolling and zoom entered the
//! vocabulary (ADRs 0122, 0162). The sweep has run as a grep since the hundred-and-twenty-second
//! session; this is the same instrument as a program, per `doc/todo/01`'s binding rule that a
//! sweep round commits one before running any (ADR 0319 records what a description costs).
//!
//! # What a program can settle that the grep could not
//!
//! The judgement each run redid by hand is one `grep` of this tree: take the thing the sentence
//! says is absent and ask whether any source file names it — "a capability recorded as absent is
//! worth one grep of your own tree before it is believed" (`doc/habits.md`, on §7.6.4.3.2's
//! Annex D tables). A hit whose lacking noun the tree names carries the [witness](Hit::witness),
//! path and line, and is printed first. The second judgement is the recorded tell that separates
//! the decaying population from the kept one: a claim about *the program* is the kind that
//! expires when a capability arrives anywhere, while a claim about *a crate* is usually a
//! boundary that crate deliberately keeps — no clock, no filesystem, no toolkit — and stays true
//! however much the program grows. [`Hit::subject`] says which kind a sentence is.
//!
//! # The three noise shapes, printed rather than filtered
//!
//! All are `doc/todo/01`'s findings about this sweep. **A true statement about a boundary a
//! crate keeps** is the dominant population — every source hit of most runs — and a witness does
//! not disprove one: "this crate has no clock" is true of `pdf-render` while `viewer-core` names
//! the clock it deliberately holds instead. **A correction quoting the wording it retired**
//! reproduces the claim inside a history sentence and is marked [`Hit::history`]. And **a
//! witness can be the one-short-key shape wearing an identifier**: `form::Control` witnesses
//! "no such control", which is Table 220's widget and not the user control the note means. A hit
//! is a reading list, not a verdict.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument, the same as every sweep here: the noise population is legitimate
//! prose — mostly boundary statements this project wants kept — and does not shrink under a
//! tighter predicate. It runs in seconds and its output is read.

use std::fmt;
use std::path::PathBuf;

use crate::ledger::Ledger;

/// The self-referential subjects a capability reason names.
///
/// Lower-cased substrings, matched against one sentence at a time. Split into the two
/// populations the runs have learnt to read differently: a claim about the whole program decays
/// when a capability arrives anywhere in it, and a claim about one crate is usually a boundary
/// that crate keeps on purpose.
pub const PROGRAM_SUBJECTS: [&str; 6] = [
    "this program",
    "this viewer",
    "this tree",
    "this one",
    "this project",
    "this processor",
];

/// The subjects that name one crate or host rather than the program.
pub const CRATE_SUBJECTS: [&str; 2] = ["this crate", "this host"];

/// The phrases that make a subject sentence a claim of absence.
///
/// Matched on a word boundary — "has no" may not continue into "has not" or "has none" —
/// and the longer spellings are listed so that each is judged as itself.
pub const LACKS: [&str; 8] = [
    "has none",
    "have none",
    "has nothing",
    "has no",
    "have no",
    "does not have",
    "do not have",
    "nothing in this",
];

/// The one claim that carries its subject inside itself.
pub const STANDALONE: [&str; 1] = ["which this is not"];

/// The phrases that make a claim sentence read like history rather than like a reason —
/// the correction-quoting-its-retired-wording shape, marked rather than filtered.
pub const HISTORY: [&str; 6] = [
    "used to",
    "said",
    "was corrected",
    "retired",
    "stood",
    "this row",
];

/// Words too common to search the tree for: the claim's own grammar and its subjects, not
/// its capability.
const STOP_WORDS: [&str; 33] = [
    "such",
    "this",
    "that",
    "with",
    "have",
    "does",
    "then",
    "them",
    "than",
    "what",
    "which",
    "into",
    "from",
    "kind",
    "sort",
    "there",
    "here",
    "when",
    "where",
    "whose",
    "these",
    "those",
    "tree",
    "crate",
    "program",
    "viewer",
    "processor",
    "project",
    "host",
    "nothing",
    "would",
    "still",
    "about",
];

/// Whose absence the sentence asserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    /// The program, the viewer, the tree: the population that decays, because a capability
    /// arriving anywhere in the program expires it.
    Program,
    /// One crate or host: usually a boundary it deliberately keeps, which stays true however
    /// much the program grows.
    Crate,
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Program => "the program",
            Self::Crate => "a crate",
        })
    }
}

/// A source file naming the capability a sentence says is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Witness {
    /// The file, relative to the workspace root.
    pub path: String,
    /// The 1-based line of the first occurrence.
    pub line: usize,
    /// The word that matched.
    pub term: String,
}

/// One capability-reason sentence, wherever it was found.
#[derive(Debug, Clone)]
pub struct Hit {
    /// Where it is: `doc/conformance/ledger.toml:123 (§12.6.3, partial)` or
    /// `crates/pdf-model/src/lib.rs:88`.
    pub location: String,
    /// The claiming sentence, whole.
    pub sentence: String,
    /// Whether the claim is about the program or about one crate.
    pub subject: Subject,
    /// The capability the sentence says is absent, as extracted.
    pub lacking: String,
    /// The first source file that names it, if any does.
    pub witness: Option<Witness>,
    /// Whether the sentence also reads like a correction's history.
    pub history: bool,
}

/// What one run of the sweep read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Hits in the ledger's notes.
    pub ledger: Vec<Hit>,
    /// Hits in comments under [`crate::SOURCE_ROOTS`].
    pub source: Vec<Hit>,
}

impl Report {
    /// How many hits in one population carry a witness.
    #[must_use]
    pub fn witnessed(hits: &[Hit]) -> usize {
        hits.iter().filter(|hit| hit.witness.is_some()).count()
    }

    /// How many hits in one population are about the given subject.
    #[must_use]
    pub fn about(hits: &[Hit], subject: Subject) -> usize {
        hits.iter().filter(|hit| hit.subject == subject).count()
    }
}

/// Runs the sweep over the ledger's notes and the tree's comments.
///
/// The claim scan skips [`crate::NOT_SCANNED`] — this module quotes the claim phrases as
/// examples, and a sweep that reports itself is the fastest way to have one switched off — and
/// the witness search skips it too, for the same sentence one step later: its documentation
/// names the capabilities it explains.
#[must_use]
pub fn sweep(ledger: &Ledger, sources: &[(PathBuf, String)]) -> Report {
    let lowered: Vec<(String, String)> = sources
        .iter()
        .map(|(path, text)| {
            (
                path.to_string_lossy().replace('\\', "/"),
                text.to_ascii_lowercase(),
            )
        })
        .collect();
    let mut report = Report::default();
    for row in &ledger.rows {
        let Some(note) = row.note.as_deref() else {
            continue;
        };
        for sentence in crate::unread::sentences(note) {
            if let Some(hit) = judge(sentence, None, &lowered) {
                report.ledger.push(Hit {
                    location: format!(
                        "{}:{} (§{}, {})",
                        crate::LEDGER,
                        row.line,
                        row.clause,
                        row.status.as_str()
                    ),
                    ..hit
                });
            }
        }
    }
    for (path, text) in sources {
        let shown = path.to_string_lossy().replace('\\', "/");
        if shown.starts_with(crate::NOT_SCANNED) {
            continue;
        }
        for (line, block) in crate::blockers::comment_blocks(text) {
            for sentence in crate::unread::sentences(&block) {
                if let Some(hit) = judge(sentence, Some(&shown), &lowered) {
                    report.source.push(Hit {
                        location: format!("{shown}:{line}"),
                        ..hit
                    });
                }
            }
        }
    }
    report
}

/// Judges one sentence: a [`Hit`] with an empty location if it claims an absent capability,
/// else `None`. `claiming` is the file the sentence lives in, which may not witness itself —
/// a comment naming the capability it denies would otherwise always confirm it.
fn judge(sentence: &str, claiming: Option<&str>, sources: &[(String, String)]) -> Option<Hit> {
    let lowered = sentence.to_ascii_lowercase();
    let program = PROGRAM_SUBJECTS
        .iter()
        .any(|phrase| lowered.contains(phrase));
    let crate_level = CRATE_SUBJECTS.iter().any(|phrase| lowered.contains(phrase));
    let lack = LACKS
        .iter()
        .find(|phrase| find_phrase(&lowered, phrase).is_some())
        .copied();
    let standalone = STANDALONE
        .iter()
        .find(|phrase| lowered.contains(*phrase))
        .copied();
    let claims = ((program || crate_level) && lack.is_some()) || standalone.is_some();
    if !claims {
        return None;
    }
    let subject = if program || standalone.is_some() {
        Subject::Program
    } else {
        Subject::Crate
    };
    let lacking = extract(&lowered, lack, standalone);
    let witness = terms(&lacking).iter().find_map(|term| {
        sources.iter().find_map(|(path, text)| {
            if path.starts_with(crate::NOT_SCANNED) || Some(path.as_str()) == claiming {
                return None;
            }
            names(text, term).map(|line| Witness {
                path: path.clone(),
                line,
                term: term.clone(),
            })
        })
    });
    Some(Hit {
        location: String::new(),
        sentence: sentence.to_owned(),
        subject,
        lacking,
        witness,
        history: HISTORY.iter().any(|phrase| lowered.contains(phrase)),
    })
}

/// The byte index of `phrase` in `lowered` where the next character does not continue a
/// word — so "has no" may not match inside "has not" or "has none".
fn find_phrase(lowered: &str, phrase: &str) -> Option<usize> {
    let mut from = 0usize;
    while let Some(found) = lowered[from..].find(phrase) {
        let at = from.saturating_add(found);
        let end = at.saturating_add(phrase.len());
        if lowered[end..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_ascii_alphabetic())
        {
            return Some(at);
        }
        from = end;
    }
    None
}

/// The capability phrase a claim sentence asserts is absent.
///
/// After "has no" and its kin the capability follows the phrase; before "which this is not"
/// and "has none" it precedes; "does not have" reads forward where its subject stands right
/// in front of it ("this tree does not have that document") and back where the subject is a
/// relative clause's ("a window with zoom, which this program does not have"). Either way it
/// is cut at the nearest punctuation and bounded at six words: extraction feeds a word
/// search, so erring towards too few words costs a witness, which the reading covers, rather
/// than inventing one.
fn extract(lowered: &str, lack: Option<&str>, standalone: Option<&str>) -> String {
    if let Some(phrase) = lack
        && let Some(index) = find_phrase(lowered, phrase)
    {
        let follows = match phrase {
            "has no" | "have no" | "has nothing" | "nothing in this" => true,
            "does not have" | "do not have" => {
                // The subject stands right in front of the verb either way; what tells a
                // relative clause ("a window, which this program does not have") from a plain
                // one ("this tree does not have that document") is the "which" in front of
                // the subject.
                let mut before = lowered[..index].trim_end();
                for subject in PROGRAM_SUBJECTS.iter().chain(CRATE_SUBJECTS.iter()) {
                    before = before.strip_suffix(subject).unwrap_or(before).trim_end();
                }
                !(before.ends_with("which") || before.ends_with(','))
            }
            _ => false,
        };
        if follows {
            let after = &lowered[index.saturating_add(phrase.len())..];
            let cut = after
                .find(['.', ',', ';', ':', '—', '(', '"', '`'])
                .unwrap_or(after.len());
            return after[..cut]
                .split_whitespace()
                .take(6)
                .collect::<Vec<_>>()
                .join(" ");
        }
        return preceding(lowered, index);
    }
    standalone
        .and_then(|phrase| lowered.find(phrase))
        .map(|index| preceding(lowered, index))
        .unwrap_or_default()
}

/// Up to six words in front of `index`, cut at the nearest punctuation, with the claim's own
/// connectives — a trailing subject, "which", a comma — peeled off first.
fn preceding(lowered: &str, index: usize) -> String {
    let mut before = lowered[..index].trim_end();
    loop {
        let length = before.len();
        before = before.trim_end_matches(',').trim_end();
        for subject in PROGRAM_SUBJECTS.iter().chain(CRATE_SUBJECTS.iter()) {
            before = before.strip_suffix(subject).unwrap_or(before).trim_end();
        }
        before = before.strip_suffix("which").unwrap_or(before).trim_end();
        if before.len() == length {
            break;
        }
    }
    let cut = before
        .rfind(['.', ',', ';', ':', '—', '(', '"', '`'])
        .map_or(0, |at| {
            // The punctuation may be multi-byte (the em dash), so step over the whole
            // character rather than one byte of it.
            at.saturating_add(before[at..].chars().next().map_or(1, char::len_utf8))
        });
    let words: Vec<&str> = before[cut..].split_whitespace().collect();
    let skip = words.len().saturating_sub(6);
    words[skip..].join(" ")
}

/// The words of a lacking phrase worth searching the tree for: four letters or more, not the
/// claim's own grammar, the longest first — a long word is rarer, so it is likelier to be the
/// capability itself rather than the prose around it.
fn terms(lacking: &str) -> Vec<String> {
    let mut terms: Vec<String> = lacking
        .split(|character: char| !character.is_ascii_alphabetic())
        .filter(|word| word.len() >= 4 && !STOP_WORDS.contains(word))
        .map(str::to_owned)
        .collect();
    terms.sort_by_key(|term| std::cmp::Reverse(term.len()));
    terms
}

/// The 1-based line of the first whole-word occurrence of `term` in `text`, singular or
/// plural. Both sides are already lower case.
fn names(text: &str, term: &str) -> Option<usize> {
    let singular = term.strip_suffix('s').unwrap_or(term);
    for candidate in [singular.to_owned(), format!("{singular}s")] {
        let mut from = 0usize;
        while let Some(found) = text[from..].find(&candidate) {
            let at = from.saturating_add(found);
            let end = at.saturating_add(candidate.len());
            let before = text[..at].chars().next_back();
            let after = text[end..].chars().next();
            let bounded = before.is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_')
                && after.is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
            if bounded {
                return Some(text[..at].matches('\n').count().saturating_add(1));
            }
            from = end;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{Row, Status};

    fn row(clause: &str, note: &str) -> Row {
        Row {
            clause: clause.parse().expect("a clause number"),
            title: String::new(),
            status: Status::Partial,
            code: Vec::new(),
            test: Vec::new(),
            exclusion: None,
            note: Some(note.to_owned()),
            line: 1,
        }
    }

    fn source(path: &str, text: &str) -> (PathBuf, String) {
        (PathBuf::from(path), text.to_owned())
    }

    /// Every spelling this ledger has used for "the program lacks X" is a claim, and a
    /// sentence about what a *clause* lacks is not one.
    #[test]
    fn every_spelling_of_the_claim_is_one() {
        let judge = |sentence: &str| judge(sentence, None, &[]);
        assert!(
            judge("Entering and pressing are a window's business and this crate has no events.")
                .is_some()
        );
        assert!(judge("A label for a panel this program has no printer's-mark list in.").is_some());
        assert!(
            judge("Properties of a window with zoom, which this program does not have.").is_some()
        );
        assert!(judge("A press renders separations, which this is not.").is_some());
        assert!(judge("Nothing in this program yet hands a structure tree to anybody.").is_some());
        assert!(judge("The clause has no default value for the entry.").is_none());
        assert!(judge("This program reads the entry since ADR 0123.").is_none());
        assert!(judge("Tk has not been absent since this tree drew it.").is_none());
    }

    /// A lacking noun the tree names is witnessed, with the path and line — the §12.3.2.1
    /// shape, where `Command::Zoom` existed under a row denying zoom.
    #[test]
    fn a_lacking_noun_the_tree_names_is_witnessed() {
        let ledger = Ledger {
            rows: vec![row(
                "12.6.3",
                "Entering and pressing are a window's business and this crate has no events.",
            )],
        };
        let sources = vec![source(
            "crates/viewer-core/src/lib.rs",
            "fn a() {}\npub enum Event { Pointer }\n",
        )];
        let report = sweep(&ledger, &sources);
        assert_eq!(report.ledger.len(), 1);
        let hit = report.ledger.first().expect("one hit");
        assert_eq!(hit.subject, Subject::Crate);
        let witness = hit.witness.as_ref().expect("a witness");
        assert_eq!(witness.path, "crates/viewer-core/src/lib.rs");
        assert_eq!(witness.line, 2);
        assert_eq!(witness.term, "events");
    }

    /// The capability in front of a relative clause is extracted from before the subject, so
    /// "zoom, which this program does not have" finds the tree's own `Zoom`.
    #[test]
    fn the_capability_before_a_relative_clause_is_extracted() {
        let ledger = Ledger {
            rows: vec![row(
                "12.3.2.1",
                "Properties of a window with scrolling and zoom, which this program does not \
                 have.",
            )],
        };
        let sources = vec![source(
            "crates/viewer-core/src/lib.rs",
            "pub enum Command { Zoom, Scroll }\n",
        )];
        let report = sweep(&ledger, &sources);
        let hit = report.ledger.first().expect("one hit");
        assert_eq!(hit.subject, Subject::Program);
        assert!(hit.lacking.contains("zoom"), "extracted: {}", hit.lacking);
        assert!(hit.witness.is_some());
    }

    /// A boundary nothing in the tree names stays unwitnessed, which is the claim holding on
    /// the sweep's own evidence.
    #[test]
    fn a_boundary_nobody_names_is_unwitnessed() {
        let ledger = Ledger {
            rows: vec![row("12.9.2", "This viewer has no measuring tools.")],
        };
        let sources = vec![source("crates/pdf-model/src/lib.rs", "fn draw() {}\n")];
        let report = sweep(&ledger, &sources);
        assert!(report.ledger.first().expect("one hit").witness.is_none());
    }

    /// A correction quoting the wording it retired is the known false-positive shape, marked
    /// rather than filtered.
    #[test]
    fn a_correction_quoting_its_retired_wording_is_marked_as_history() {
        let ledger = Ledger {
            rows: vec![row(
                "12.6.3",
                "This row said \"this crate has no events\" until the hundred-and-seventy-fourth.",
            )],
        };
        let report = sweep(&ledger, &[]);
        assert!(report.ledger.first().expect("one hit").history);
    }

    /// A comment is not witnessed by its own file: a module explaining the capability it
    /// denies would otherwise always confirm it.
    #[test]
    fn a_claim_is_not_witnessed_by_its_own_file() {
        let ledger = Ledger { rows: Vec::new() };
        let sources = vec![source(
            "crates/pdf-render/src/clock.rs",
            "// This crate has no clock to run one with.\nfn tick() { let clock = 0; }\n",
        )];
        let report = sweep(&ledger, &sources);
        let hit = report.source.first().expect("one hit");
        assert!(hit.witness.is_none());
    }

    /// The checker's own directory neither claims nor witnesses — its documentation quotes
    /// the phrases and names the capabilities it explains.
    #[test]
    fn the_checkers_own_documentation_is_neither_claim_nor_witness() {
        let ledger = Ledger {
            rows: vec![row("12.9.2", "This viewer has no measuring tools.")],
        };
        let sources = vec![source(
            "tools/conformance/src/capabilities.rs",
            "// A sentence like \"this crate has no measuring tools\" is the shape.\n",
        )];
        let report = sweep(&ledger, &sources);
        assert!(report.source.is_empty());
        assert!(report.ledger.first().expect("one hit").witness.is_none());
    }

    /// A singular in the tree witnesses a plural in the claim, and the match is a whole word:
    /// `panels` matches `panel`, and `panelled` matches neither.
    #[test]
    fn a_witness_is_a_whole_word_either_number() {
        assert_eq!(names("a panel here", "panels"), Some(1));
        assert_eq!(names("two\npanels here", "panel"), Some(2));
        assert_eq!(names("panelled walls", "panel"), None);
    }
}
