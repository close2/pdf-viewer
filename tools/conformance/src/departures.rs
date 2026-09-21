//! The reading list a `departed` row's argument needs, and the check that the argument exists.
//!
//! # What this is for
//!
//! `departed` says a clause is executed except for one sentence, "decided against with its cost
//! recorded", and [`crate::ledger::check`] asks the row to name the ADR that recorded it. That
//! check is deliberately the shallowest one that can be wrong in only one direction: it refuses a
//! row naming no argument and **cannot judge whether the argument still fits** (ADR 1119). Nothing
//! else re-read one, so a `departed` row joined the settled statuses and its argument became a
//! dated document — the shape `doc/habits/the-ledger-and-claims-about-this-tree.md` names, where a
//! capability recorded as blocked on a decision outlives the decision and nothing fires when the
//! stated blocker expires.
//!
//! This module does the mechanical half of the re-reading and leaves the judgement where it
//! belongs. For every `departed` row it prints the ADR its note's first sentence names — the
//! deciding argument — and every **later** ADR that cites either that ADR or the row's own clause
//! number. That is the set of documents a person has to read to find out whether the premise is
//! still true; deciding whether it is remains a person's, because no program can read a premise.
//!
//! # What it reads, and what it refuses to read
//!
//! ADR numbers and clause numbers, and nothing else. It does not read a note's prose, score it, or
//! infer a category from it: a rule that asked whether an argument *fits* would be a judgement
//! wearing a program, which is the reason [`crate::ledger::Ledger::is_aggregate`] reads no prose
//! either (ADR 1035 section 4). The one place it does look at a sentence boundary is to tell the
//! *deciding* ADR from the dozens a long note accumulates, and that rule is stated below with what
//! it costs.
//!
//! ADR 1166.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::ledger::{Ledger, Status};

/// Where the ADRs are, relative to the workspace root.
pub const ADR_DIRECTORY: &str = "doc/adr";

/// Why a later ADR is on a row's reading list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Because {
    /// It cites the ADR the row's first sentence names.
    CitesTheArgument(u16),
    /// It cites the row's clause number.
    CitesTheClause,
}

/// One ADR on a reading list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Later {
    /// Its number.
    pub number: u16,
    /// Its path, relative to the workspace root — what a reader opens.
    pub path: PathBuf,
    /// Why it is listed, in ascending order and without repeats.
    pub because: Vec<Because>,
}

/// One `departed` row, with the documents a re-reader needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Departure {
    /// The clause, as the row writes it.
    pub clause: String,
    /// The ledger line the row starts on.
    pub line: usize,
    /// The ADRs the note's first sentence names — the deciding argument. Empty is a finding.
    pub deciding: Vec<u16>,
    /// Every ADR the note names anywhere, deciding ones included.
    pub named: Vec<u16>,
    /// Every ADR later than the deciding one that cites it or the clause.
    pub later: Vec<Later>,
}

/// Why the reading list could not be built.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The ADRs could not be listed or read.
    #[error("cannot read {path}: {source}")]
    Unreadable {
        /// The path that was tried.
        path: String,
        /// The underlying failure.
        source: std::io::Error,
    },
}

/// Every ADR on disk, by number, as (path relative to `root`, text).
///
/// A number with two files is a mistake somebody would want to hear about, and the later path
/// wins here rather than being reported: this module's subject is the reading list, and
/// `records.rs` is what watches `doc/adr/` for its own shape.
///
/// # Errors
///
/// If [`ADR_DIRECTORY`] cannot be listed, or one of its files cannot be read. A reading list
/// built from a directory that would not open is a clean answer about the instrument rather
/// than about the tree (trap 13).
pub fn adrs(root: &Path) -> Result<BTreeMap<u16, (PathBuf, String)>, Error> {
    let directory = root.join(ADR_DIRECTORY);
    let unreadable = |path: &Path, source: std::io::Error| Error::Unreadable {
        path: path.display().to_string(),
        source,
    };
    let entries = std::fs::read_dir(&directory).map_err(|why| unreadable(&directory, why))?;
    let mut found = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(|why| unreadable(&directory, why))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(number) = leading_number(name) else {
            continue;
        };
        if !is_markdown(name) {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|why| unreadable(&path, why))?;
        let relative = Path::new(ADR_DIRECTORY).join(name);
        found.insert(number, (relative, text));
    }
    Ok(found)
}

/// Every ADR number with a file in [`ADR_DIRECTORY`].
///
/// The listing alone, without the texts — what [`crate::ledger::check`] needs to say whether a
/// `departed` row's argument is a document somebody can open.
///
/// # Errors
///
/// If the directory cannot be listed.
pub fn numbers_on_disk(root: &Path) -> Result<BTreeSet<u16>, Error> {
    let directory = root.join(ADR_DIRECTORY);
    let entries = std::fs::read_dir(&directory).map_err(|source| Error::Unreadable {
        path: directory.display().to_string(),
        source,
    })?;
    let mut found = BTreeSet::new();
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if let Some(number) = leading_number(name)
            && is_markdown(name)
        {
            found.insert(number);
        }
    }
    Ok(found)
}

/// Whether the name is a Markdown file's.
fn is_markdown(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

/// The four-digit number an ADR's file name opens with, if it does.
fn leading_number(name: &str) -> Option<u16> {
    let digits: String = name.chars().take(4).collect();
    if digits.len() == 4 && digits.chars().all(|character| character.is_ascii_digit()) {
        digits.parse().ok()
    } else {
        None
    }
}

/// The note's first sentence: everything up to the first full stop followed by white space.
///
/// **A `departed` note's first sentence is where the departure and its ADR are**, by the
/// convention the status was created with — the rest of a note is the clause's other
/// requirements, and a long one accumulates dozens of ADR numbers that have nothing to do with
/// the decision. Taking them all would make the reading list unreadable, which is the way an
/// instrument stops being run.
///
/// The cost of the rule is that a clause number's own full stops must not end a sentence, and
/// they do not: a full stop inside `12.11.3` is followed by a digit rather than by white space.
/// A note whose first sentence ends at the very end of the note, with nothing after it, yields
/// the whole note, which is the same answer.
#[must_use]
pub fn first_sentence(note: &str) -> &str {
    let mut characters = note.char_indices().peekable();
    while let Some((at, character)) = characters.next() {
        if character == '.'
            && characters
                .peek()
                .is_some_and(|(_, next)| next.is_whitespace())
        {
            return note.get(..=at).unwrap_or(note);
        }
    }
    note
}

/// Every ADR number cited in `text`.
///
/// A citation is the word `ADR` or `ADRs` followed by a run of four-digit numbers separated by
/// white space, commas, solidi or the word `and` — which is how this tree writes one, in all
/// three forms: `ADR 0036`, `ADRs 0803, 0814, 1144, 1145` and `ADRs 0718/0725/0737`. The run
/// stops at the first thing that is neither a separator nor a four-digit number, so
/// `ADR 1035 section 3` names one ADR and `ADR 0036, 974 documents` also names one.
#[must_use]
pub fn citations(text: &str) -> BTreeSet<u16> {
    let bytes = text.as_bytes();
    let mut found = BTreeSet::new();
    for (at, _) in text.match_indices("ADR") {
        let mut cursor = at.saturating_add(3);
        if bytes.get(cursor) == Some(&b's') {
            cursor = cursor.saturating_add(1);
        }
        // A word beginning with `ADR` that is not the word — nothing in this tree writes one,
        // and refusing it costs nothing and keeps the matcher honest about what it matched.
        if bytes.get(cursor).is_some_and(u8::is_ascii_alphanumeric) {
            continue;
        }
        loop {
            let after = skip_separators(bytes, cursor);
            let Some(number) = four_digits(bytes, after) else {
                break;
            };
            found.insert(number);
            cursor = after.saturating_add(4);
        }
    }
    found
}

/// Past white space, commas, solidi and the word `and`.
fn skip_separators(bytes: &[u8], mut at: usize) -> usize {
    loop {
        match bytes.get(at) {
            Some(b' ' | b',' | b'/' | b'\n' | b'\t') => at = at.saturating_add(1),
            Some(b'a') if bytes.get(at.saturating_add(1)..at.saturating_add(3)) == Some(b"nd") => {
                at = at.saturating_add(3);
            }
            _ => return at,
        }
    }
}

/// Exactly four digits at `at`, not followed by a fifth.
fn four_digits(bytes: &[u8], at: usize) -> Option<u16> {
    let digits = bytes.get(at..at.saturating_add(4))?;
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    if bytes
        .get(at.saturating_add(4))
        .is_some_and(u8::is_ascii_digit)
    {
        return None;
    }
    std::str::from_utf8(digits).ok()?.parse().ok()
}

/// Whether `text` cites `clause` — the clause itself and not a subclause of it.
///
/// `§12.5.5` in a document about `§12.5.5.1` is a different subject, so the match ends where the
/// number does: the character after it may not be a digit, nor a full stop before one.
#[must_use]
pub fn cites_clause(text: &str, clause: &str) -> bool {
    let marker = format!("\u{a7}{clause}");
    text.match_indices(&marker).any(|(at, found)| {
        let rest = &text[at.saturating_add(found.len())..];
        let mut characters = rest.chars();
        match characters.next() {
            Some(character) if character.is_ascii_digit() => false,
            Some('.') => !characters.next().is_some_and(|next| next.is_ascii_digit()),
            _ => true,
        }
    })
}

/// The reading list for every `departed` row in the ledger.
///
/// # Errors
///
/// If [`ADR_DIRECTORY`] cannot be read.
pub fn read(ledger: &Ledger, root: &Path) -> Result<Vec<Departure>, Error> {
    let adrs = adrs(root)?;
    Ok(departures(ledger, &adrs))
}

/// The reading list, from an already-read set of ADRs.
#[must_use]
pub fn departures(ledger: &Ledger, adrs: &BTreeMap<u16, (PathBuf, String)>) -> Vec<Departure> {
    let mut found = Vec::new();
    for row in &ledger.rows {
        if row.status != Status::Departed {
            continue;
        }
        let note = row.note.as_deref().unwrap_or_default();
        let deciding: Vec<u16> = citations(first_sentence(note)).into_iter().collect();
        let named: Vec<u16> = citations(note).into_iter().collect();
        let clause = row.clause.to_string();
        // Later than the *last* of the deciding ADRs: a re-reader wants what came after the
        // argument was complete. With none named there is no floor and every ADR citing the
        // clause is later than nothing, which is the right answer for a row whose argument
        // cannot be found.
        let floor = deciding.iter().copied().max().unwrap_or(0);
        let mut later = Vec::new();
        for (&number, (path, text)) in adrs {
            if number <= floor {
                continue;
            }
            let cited = citations(text);
            let mut because: Vec<Because> = deciding
                .iter()
                .filter(|argument| cited.contains(argument))
                .map(|&argument| Because::CitesTheArgument(argument))
                .collect();
            if cites_clause(text, &clause) {
                because.push(Because::CitesTheClause);
            }
            if !because.is_empty() {
                later.push(Later {
                    number,
                    path: path.clone(),
                    because,
                });
            }
        }
        found.push(Departure {
            clause,
            line: row.line,
            deciding,
            named,
            later,
        });
    }
    found
}

/// The reading list as a person reads it.
#[must_use]
pub fn report(departures: &[Departure]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{} departed row(s); for each, the ADR its first sentence names and every later ADR \
         citing that argument or the clause",
        departures.len()
    );
    for departure in departures {
        let _ = writeln!(out);
        let deciding = if departure.deciding.is_empty() {
            "no ADR in its first sentence".to_owned()
        } else {
            departure
                .deciding
                .iter()
                .map(|number| format!("ADR {number:04}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let _ = writeln!(
            out,
            "\u{a7}{} (line {}) — decided by {}",
            departure.clause, departure.line, deciding
        );
        let elsewhere: Vec<String> = departure
            .named
            .iter()
            .filter(|number| !departure.deciding.contains(number))
            .map(|number| format!("{number:04}"))
            .collect();
        if !elsewhere.is_empty() {
            let _ = writeln!(out, "  the note names elsewhere: {}", elsewhere.join(", "));
        }
        if departure.later.is_empty() {
            let _ = writeln!(out, "  nothing later cites the argument or the clause");
            continue;
        }
        let _ = writeln!(out, "  since, to re-read ({}):", departure.later.len());
        for later in &departure.later {
            let mut why: Vec<String> = Vec::new();
            for because in &later.because {
                why.push(match *because {
                    Because::CitesTheArgument(argument) => format!("cites ADR {argument:04}"),
                    Because::CitesTheClause => format!("cites \u{a7}{}", departure.clause),
                });
            }
            let _ = writeln!(out, "    {} — {}", later.path.display(), why.join(", "));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_citation_is_read_in_all_three_forms_this_tree_writes() {
        assert_eq!(citations("decided in ADR 0036."), BTreeSet::from([36]));
        assert_eq!(
            citations("(ADRs 0803, 0814, 1144, 1145)"),
            BTreeSet::from([803, 814, 1144, 1145])
        );
        assert_eq!(
            citations("contradicted by ADRs 0718/0725/0737 within two rounds"),
            BTreeSet::from([718, 725, 737])
        );
        assert_eq!(citations("ADRs 0187 and 1085"), BTreeSet::from([187, 1085]));
    }

    /// The run has to stop, or a number that is not an ADR joins the reading list. Both of these
    /// appear in the ledger as written.
    #[test]
    fn a_citation_run_stops_at_the_first_thing_that_is_not_a_number() {
        assert_eq!(
            citations("recorded as a departure in ADR 1035 section 3"),
            BTreeSet::from([1035])
        );
        assert_eq!(
            citations("ADR 0036 decided and priced, over 974 documents"),
            BTreeSet::from([36])
        );
        assert!(citations("ADDRESS 0036 is not a citation").is_empty());
    }

    #[test]
    fn the_first_sentence_ends_at_a_full_stop_and_not_inside_a_clause_number() {
        assert_eq!(
            first_sentence("Not obeyed, as \u{a7}12.11.3 states. The rest follows."),
            "Not obeyed, as \u{a7}12.11.3 states."
        );
        assert_eq!(first_sentence("One sentence only."), "One sentence only.");
    }

    /// A reading list that answered a subclause's citations for its parent would hand a re-reader
    /// documents about a different requirement, which is how a long list stops being read.
    #[test]
    fn a_clause_citation_does_not_match_a_subclause_of_it() {
        assert!(cites_clause("read against \u{a7}12.5.5 here", "12.5.5"));
        assert!(cites_clause("\u{a7}12.5.5, and the rest", "12.5.5"));
        assert!(cites_clause("\u{a7}12.5.5. The next sentence", "12.5.5"));
        assert!(!cites_clause("\u{a7}12.5.5.1 is another row", "12.5.5"));
        assert!(!cites_clause("\u{a7}12.5.51 is not this", "12.5.5"));
    }

    /// Trap 13: the instrument is run against the thing it looks for. A row decided by an ADR
    /// nothing has cited since gets an empty list, and one whose argument a later ADR cites gets
    /// that ADR — otherwise an empty list would mean nothing either way.
    #[test]
    fn a_later_adr_citing_the_argument_is_on_the_list_and_an_earlier_one_is_not() {
        let ledger = Ledger::parse(
            "[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"departed\"\n\
             code = [\"a.rs\"]\ntest = [\"t.rs\"]\n\
             note = \"One sentence declined, decided in ADR 0036. And more prose, ADR 0587.\"\n",
        )
        .expect("the fixture is in the subset");
        let adrs = BTreeMap::from([
            (
                20_u16,
                (
                    PathBuf::from("doc/adr/0020-before.md"),
                    "ADR 0036 cannot be cited before it exists, but the text can say it".to_owned(),
                ),
            ),
            (
                464,
                (
                    PathBuf::from("doc/adr/0464-after.md"),
                    "revisits ADR 0036 on its own terms".to_owned(),
                ),
            ),
            (
                900,
                (
                    PathBuf::from("doc/adr/0900-elsewhere.md"),
                    "about \u{a7}8.1 and nothing else".to_owned(),
                ),
            ),
            (
                901,
                (
                    PathBuf::from("doc/adr/0901-unrelated.md"),
                    "about \u{a7}8.1.1 and ADR 0999".to_owned(),
                ),
            ),
        ]);
        let found = departures(&ledger, &adrs);
        let departure = found.first().expect("one departed row");
        assert_eq!(departure.deciding, vec![36]);
        assert_eq!(departure.named, vec![36, 587]);
        let listed: Vec<u16> = departure.later.iter().map(|later| later.number).collect();
        assert_eq!(listed, vec![464, 900]);
        assert_eq!(
            departure.later.first().map(|later| later.because.clone()),
            Some(vec![Because::CitesTheArgument(36)])
        );
    }
}
