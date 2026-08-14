//! The fourth sweep: a claim one round retired, hunted wherever else this project writes it.
//!
//! # The shape it exists for
//!
//! Two clauses describing one mechanism is the commonest shape in `doc/todo/01`, and correcting
//! one leaves the other lying. §8.9.6.1 still said a graphics-state soft mask was "reported
//! rather than applied on 28 corpus documents" fourteen sessions after §11.6.4.3 retired that
//! exact sentence; §12.5.6.22's `/FixedPrint` was still explained by the refusal ADR 0168
//! dismantled; four rows and four source comments went on saying this program reads no `XMP`
//! after the round that gave it one, three of the six written *by* that round. The sweep has run
//! as a grep since the two-hundred-and-sixteenth session and `doc/adr/` joined its targets in the
//! four-hundred-and-twenty-ninth (ADR 0265). This is the same instrument as a program, per
//! `doc/todo/01`'s binding rule that a sweep round commits one before running any.
//!
//! # What it reads
//!
//! The ledger's notes, every `//` comment under [`crate::SOURCE_ROOTS`], and every Markdown
//! document under `doc/` that this project wrote except [`NOT_SWEPT`]. The last is wider than
//! the by-hand runs' `doc/adr/` on the evidence of the five-hundred-and-first, whose second
//! finding was `doc/todo/README.md`'s index line for an item closed one wave earlier: **an index
//! row decays at its item's pace, not its own**, and nothing was looking at it.
//!
//! # Why it takes its nouns from the caller
//!
//! Every other sweep here derives its own population: a blocker phrase, a capability phrase, a
//! `/Key`, a table number. This one cannot. **What was retired is what the last rounds decided**,
//! and a program has no way to know that a `RetainedScene` replaced a frame or that `hollow` is
//! now what a glyph's bowl is called. So the nouns are arguments — one run per sweep round, over
//! the nouns those rounds gave the tree — which is also what the by-hand runs did, from a list
//! written into the round's own record. `doc/todo/01`'s refinement of the sweep applies
//! unchanged: **run it over the mechanism rather than over the exact string** — `NoZoom`,
//! `uncoloured`, `/AIS` — because the sentence that stayed is rarely worded like the sentence
//! that went.
//!
//! # What a program settles that the grep could not
//!
//! A grep prints matching lines and leaves the whole judgement to the reader. The judgement has
//! a shape, and it is the sweep's own subject: a round retires a claim by *recording* the
//! retirement where it made it — "this row said X until the four-hundredth", "since ADR 0234" —
//! and leaves the plain claim standing somewhere else. So each mention is classified
//! [`Kind::Correction`] or [`Kind::Standing`], and **a noun carrying both is the defect's exact
//! signature**: somebody wrote the correction here and not there. Those nouns print first, and a
//! noun mentioned only in corrections is a round that swept its own work properly.
//!
//! # The noise, printed rather than filtered
//!
//! The classification is a reading order, not a verdict, and both directions are known to be
//! wrong sometimes. A correction sentence *quotes* the wording it retired, which is this sweep's
//! oldest false positive and is why the quotation is not filtered out — a live claim has hidden
//! inside one (§12.5.6.4's popup, ADR 0294). A standing mention is usually an ordinary true
//! sentence about the noun rather than the retired claim, and one short noun lands in three
//! clauses that mean three different things (`/BG` in Table 57 and Table 232). Read the sentence
//! before believing a hit.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument, and one more that is this sweep's own: its input is a judgement
//! about what the last rounds retired, so a build failure would depend on what somebody typed on
//! the command line. It runs in seconds over the ledger, the tree's comments and every ADR.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::ledger::Ledger;

/// The phrases that make a sentence a record of a correction rather than a plain claim.
///
/// Lower-cased substrings, matched against one sentence at a time. These are the ways this
/// ledger, this tree's comments and these ADRs write "what was here before was wrong": a
/// retirement narrated in place, which is exactly what the *other* place is missing.
pub const CORRECTIONS: [&str; 12] = [
    "used to",
    "this row",
    "the row said",
    "said \"",
    "was corrected",
    "retired",
    "no longer",
    "until the",
    "since adr",
    "which was false",
    "stopped being true",
    "is false",
];

/// Whether a mention narrates a retirement or simply makes the claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The sentence records that something was retired, corrected or superseded — the place a
    /// round *did* its sweeping.
    Correction,
    /// The sentence simply uses the noun. The population a retired claim survives in.
    Standing,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Correction => "correction",
            Self::Standing => "standing",
        })
    }
}

/// One sentence naming one of the nouns.
#[derive(Debug, Clone)]
pub struct Mention {
    /// Where it is: `doc/conformance/ledger.toml:123 (§8.9.6.1, partial)`,
    /// `crates/pdf-model/src/lib.rs:88` or `doc/adr/0234-….md:41`.
    pub location: String,
    /// The sentence, whole.
    pub sentence: String,
    /// Whether it narrates a retirement or makes a claim.
    pub kind: Kind,
}

/// Every mention of one noun, across the three populations.
#[derive(Debug, Clone)]
pub struct Found {
    /// The noun as the caller wrote it.
    pub noun: String,
    /// The mentions, corrections first.
    pub mentions: Vec<Mention>,
}

impl Found {
    /// How many mentions narrate a retirement.
    #[must_use]
    pub fn corrections(&self) -> usize {
        self.mentions
            .iter()
            .filter(|mention| mention.kind == Kind::Correction)
            .count()
    }

    /// How many mentions simply make a claim.
    #[must_use]
    pub fn standing(&self) -> usize {
        self.mentions
            .iter()
            .filter(|mention| mention.kind == Kind::Standing)
            .count()
    }

    /// Whether a correction and a standing claim share this noun — the defect's signature, and
    /// the reason this sweep reads its output in an order rather than as a list.
    #[must_use]
    pub fn both_shapes(&self) -> bool {
        self.corrections() > 0 && self.standing() > 0
    }
}

/// The one directory under `doc/` whose prose this sweep does not read.
///
/// A history file is one round's own record and **no round edits another's**
/// (`doc/todo/02` §6), so a retired claim standing in one is not a defect to correct — it is
/// what that round wrote on the day it wrote it. Reading them would put every round's account
/// of every noun in front of the reader with nothing to do about any of it. Everything else
/// under `doc/` that this project wrote is in: the ADRs, because a claim a later round
/// disproves is amended in the ADR that made it (ADR 0265), and `doc/todo/` and the standing
/// documents, because an index row decays at its item's pace rather than its own — which is
/// where the five-hundred-and-first run's second finding was.
pub const NOT_SWEPT: &str = "doc/history";

/// Runs the sweep over the ledger's notes, the tree's comments and this project's prose.
///
/// `sources` are the Rust files under [`crate::SOURCE_ROOTS`] and `documents` the Markdown
/// under `doc/`, each with its text. Two directories are skipped: [`NOT_SWEPT`], and the
/// checker's own, as everywhere here — this module quotes the sweep's findings as examples, and
/// a sweep that reports itself is the fastest way to have one switched off.
#[must_use]
pub fn sweep(
    nouns: &[String],
    ledger: &Ledger,
    sources: &[(PathBuf, String)],
    documents: &[(PathBuf, String)],
) -> Vec<Found> {
    let mut places: Vec<(String, String)> = Vec::new();
    for row in &ledger.rows {
        if let Some(note) = row.note.as_deref() {
            places.push((
                format!(
                    "{}:{} (§{}, {})",
                    crate::LEDGER,
                    row.line,
                    row.clause,
                    row.status.as_str()
                ),
                note.to_owned(),
            ));
        }
    }
    for (path, text) in sources {
        let shown = shown(path);
        if shown.starts_with(crate::NOT_SCANNED) {
            continue;
        }
        for (line, block) in crate::blockers::comment_blocks(text) {
            places.push((format!("{shown}:{line}"), block));
        }
    }
    for (path, text) in documents {
        let shown = shown(path);
        if shown.starts_with(NOT_SWEPT) {
            continue;
        }
        for (line, block) in paragraphs(text) {
            places.push((format!("{shown}:{line}"), block));
        }
    }

    nouns
        .iter()
        .map(|noun| {
            let wanted = noun.to_ascii_lowercase();
            let mut mentions: Vec<Mention> = Vec::new();
            for (location, block) in &places {
                for sentence in crate::unread::sentences(block) {
                    if names(sentence, &wanted) {
                        mentions.push(Mention {
                            location: location.clone(),
                            sentence: sentence.to_owned(),
                            kind: kind_of(sentence),
                        });
                    }
                }
            }
            mentions.sort_by_key(|mention| mention.kind == Kind::Standing);
            Found {
                noun: noun.clone(),
                mentions,
            }
        })
        .collect()
}

/// A path as it is printed and compared: relative, with forward slashes.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Whether one sentence names the noun.
///
/// Case-insensitive, and bounded on the left only: a noun is looked for as a *stem*, so
/// `Corrupt` finds `Corrupted`, `prefix` finds `prefixes` and `joining` finds `joining_type`,
/// while `hollow` does not match `unhollow`. That asymmetry is deliberate — this sweep hunts a
/// mechanism rather than a token, and the cost of the loose right-hand side is a hit to read
/// rather than a claim missed. An underscore counts as a boundary, so `render_retained` is found
/// under `retained` and `damaged_prefix` under `prefix`.
fn names(sentence: &str, wanted: &str) -> bool {
    let lowered = sentence.to_ascii_lowercase();
    let mut from = 0usize;
    while let Some(found) = lowered.get(from..).and_then(|rest| rest.find(wanted)) {
        let at = from.saturating_add(found);
        let bounded = lowered[..at]
            .chars()
            .next_back()
            .is_none_or(|character| !character.is_ascii_alphanumeric());
        if bounded {
            return true;
        }
        from = at.saturating_add(1);
    }
    false
}

/// Markdown's strikethrough, which is this project's own notation for a claim that was retired.
///
/// `doc/todo/30`'s closed items and `doc/todo/01`'s "what is still owed" list both strike the
/// sentence and write the correction after it, so the retired wording ends its own sentence and
/// the correction begins the next one — and a sweep that reads one sentence at a time would file
/// the struck half as a standing claim, which is the opposite of what the marks say.
pub const STRUCK: &str = "~~";

/// Whether a sentence narrates a retirement.
fn kind_of(sentence: &str) -> Kind {
    if sentence.contains(STRUCK) {
        return Kind::Correction;
    }
    let lowered = sentence.to_ascii_lowercase();
    if CORRECTIONS.iter().any(|phrase| lowered.contains(phrase)) {
        Kind::Correction
    } else {
        Kind::Standing
    }
}

/// A Markdown document's paragraphs, each with the 1-based line it starts on.
///
/// A paragraph is a run of non-blank lines, joined with spaces, and a fenced code block is not
/// prose: an ADR quoting a shell command or a struct would otherwise contribute the tree's own
/// identifiers as claims about them. Headings are kept, because an ADR's title is where its
/// claim is often stated.
fn paragraphs(text: &str) -> Vec<(usize, String)> {
    let mut blocks = Vec::new();
    let mut current: Option<(usize, String)> = None;
    let mut fenced = false;
    for (index, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            continue;
        }
        if fenced {
            continue;
        }
        if line.trim().is_empty() {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
        } else if let Some((_, block)) = current.as_mut() {
            block.push(' ');
            block.push_str(line.trim());
        } else {
            current = Some((index.saturating_add(1), line.trim().to_owned()));
        }
    }
    if let Some(block) = current.take() {
        blocks.push(block);
    }
    blocks
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

    fn file(path: &str, text: &str) -> (PathBuf, String) {
        (PathBuf::from(path), text.to_owned())
    }

    fn ledger(rows: Vec<Row>) -> Ledger {
        Ledger { rows }
    }

    /// The sweep's own subject: one row records the retirement and another still makes the
    /// claim, so the noun carries both shapes and is the one to read first.
    #[test]
    fn a_noun_carrying_both_shapes_is_the_defects_signature() {
        let ledger = ledger(vec![
            row(
                "11.6.4.3",
                "This row said the soft mask was reported until the two-hundred-and-first.",
            ),
            row(
                "8.9.6.1",
                "A soft mask is reported rather than applied on 28 corpus documents.",
            ),
        ]);
        let nouns = vec!["soft mask".to_owned()];
        let found = sweep(&nouns, &ledger, &[], &[]);
        let first = found.first().expect("one noun");
        assert_eq!(first.corrections(), 1);
        assert_eq!(first.standing(), 1);
        assert!(first.both_shapes());
    }

    /// A round that swept its own work leaves only the correction, which is the clean answer
    /// this sweep can give.
    #[test]
    fn a_noun_only_ever_corrected_carries_one_shape() {
        let ledger = ledger(vec![row(
            "9.4.4",
            "The row used to say writing mode 1 was refused, which was false from the \
             thirty-sixth.",
        )]);
        let nouns = vec!["writing mode".to_owned()];
        let found = sweep(&nouns, &ledger, &[], &[]);
        let first = found.first().expect("one noun");
        assert_eq!(first.standing(), 0);
        assert!(!first.both_shapes());
    }

    /// A comment and an ADR paragraph are searched as well as the ledger — `doc/adr/` joined
    /// this sweep's targets in the four-hundred-and-twenty-ninth (ADR 0265).
    #[test]
    fn comments_and_adrs_are_swept_beside_the_ledger() {
        let sources = vec![file(
            "crates/pdf-model/src/content.rs",
            "// The transfer function describes a marking device.\nfn draw() {}\n",
        )];
        let documents = vec![file(
            "doc/adr/0204-a-transfer-function-is-not-a-marking-devices.md",
            "# 0204\n\nThe phrase marking device is not in ISO 32000-2 at all.\n",
        )];
        let nouns = vec!["marking device".to_owned()];
        let found = sweep(&nouns, &ledger(Vec::new()), &sources, &documents);
        let first = found.first().expect("one noun");
        assert_eq!(first.mentions.len(), 2);
        assert!(
            first
                .mentions
                .iter()
                .any(|mention| mention.location.starts_with("doc/adr/"))
        );
    }

    /// A history file is one round's record and no round edits another's, so it is read by
    /// nothing here however many of the nouns it names.
    #[test]
    fn a_rounds_own_record_is_not_swept() {
        let documents = vec![file(
            "doc/history/216-a-retired-claim-is-a-string-and-strings-are-greppable.md",
            "The soft mask was reported rather than applied on 28 corpus documents.\n",
        )];
        let nouns = vec!["soft mask".to_owned()];
        let found = sweep(&nouns, &ledger(Vec::new()), &[], &documents);
        assert!(found.first().expect("one noun").mentions.is_empty());
    }

    /// The checker's own directory neither claims nor is swept: it quotes this sweep's
    /// findings as examples.
    #[test]
    fn the_checkers_own_documentation_is_not_swept() {
        let sources = vec![file(
            "tools/conformance/src/retired.rs",
            "// A sentence saying the soft mask is reported is the shape.\n",
        )];
        let nouns = vec!["soft mask".to_owned()];
        let found = sweep(&nouns, &ledger(Vec::new()), &sources, &[]);
        assert!(found.first().expect("one noun").mentions.is_empty());
    }

    /// A noun is a stem with a bounded left edge: `Corrupt` finds `Corrupted` and `prefix`
    /// finds `damaged_prefix`, and neither finds a word that merely ends in it.
    #[test]
    fn a_noun_is_a_stem_bounded_on_the_left() {
        assert!(names("The stream is Corrupted here.", "corrupt"));
        assert!(names("The damaged_prefix case.", "prefix"));
        assert!(!names("An incorrupt file.", "corrupt"));
        assert!(names("A prefix.", "prefix"));
    }

    /// A fenced code block is not prose: an ADR showing a struct would otherwise contribute
    /// the tree's identifiers as claims about them.
    #[test]
    fn a_fenced_block_is_not_a_paragraph() {
        let text = "One prose line.\n\n```rust\nstruct RetainedScene;\n```\n\nAnother line.\n";
        let blocks = paragraphs(text);
        assert_eq!(blocks.len(), 2);
        assert!(!blocks.iter().any(|(_, block)| block.contains("Retained")));
        assert_eq!(blocks.first().expect("a block").0, 1);
    }

    /// Every phrase this project writes a retirement with is one, and an ordinary sentence
    /// about the same noun is not.
    #[test]
    fn every_spelling_of_a_retirement_is_one() {
        assert_eq!(
            kind_of("The row used to say the mask was reported."),
            Kind::Correction
        );
        assert_eq!(
            kind_of("Retired by ADR 0234 in the three-hundred-and-ninety-seventh."),
            Kind::Correction
        );
        assert_eq!(
            kind_of("The mask is built since ADR 0027."),
            Kind::Correction
        );
        assert_eq!(
            kind_of("The soft mask is applied to the group's result."),
            Kind::Standing
        );
    }

    /// A struck sentence is a retirement whatever words are inside it, because the marks are
    /// this project's own notation for one — and the correction after it is a *different*
    /// sentence, which is exactly where a sentence-scoped sweep would go wrong.
    #[test]
    fn a_struck_sentence_is_a_retirement() {
        assert_eq!(
            kind_of("~~The mask is reported rather than applied.~~"),
            Kind::Correction
        );
    }
}
