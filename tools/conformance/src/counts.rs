//! The tenth sweep: does the family a sentence counts hold that many rows?
//!
//! # The shape it exists for
//!
//! A parent row that says "three of the twenty" is making a **checkable claim about the rows below
//! it**, and nothing in this project checked one. [`crate::ledger`]'s gate reads a row's own fields;
//! the sixth sweep compares a parent's *status* with its children's; and a count in prose is
//! neither. Its findings are `doc/todo/01`'s fifth failure shape at family scale — a parent row is
//! not maintained by the sessions that implement its members — and they have been large: §12.7.6's
//! "the other two are refused by name" stood for **280 sessions** with three other rows holding the
//! right answer, and §11.7 stated its own family's count twice, four sentences apart, disagreeing.
//!
//! # Why the hand-runs' number said nothing
//!
//! Ten runs printed 16, 185, 124, 10, 160, 17, 70, 25, 41 and 4 counted claims over a ledger whose
//! families barely moved, because each session wrote the pattern that morning: a number word beside
//! a verb of implementation, or every digit in a note, or only the phrase "aggregate of the N
//! below". That is sweep 14's own lesson before ADR 0397 fixed it — **the obvious discriminator
//! here is a vocabulary of counting, and a vocabulary written from memory measures the session**.
//!
//! # The discriminator, and it is two measurements this project already has
//!
//! Neither is a vocabulary of counting.
//!
//! - **Attribution, which is the ninth sweep's** ([`crate::tables`]). There a key is a claim about a
//!   table only where the sentence attributes it; here a cardinal is a claim about a family only
//!   where it **governs one of the ledger's own words for its rows** ([`NOUNS`]) within [`REACH`]
//!   words. "Three of the clause's properties", "two entries that would add to a document" and
//!   "[f]our of them are stream filters" are the noise every hand-run printed, and all three are
//!   counts of something that is not a family. And the container is the clause the sentence
//!   **names**, exactly as a table is: that is how §12.6.3's count of §12.6.4's family — invisible
//!   to every hand-run, found by the blame band in the five-hundred-and-twenty-fifth — becomes a
//!   claim this sweep can judge at all.
//! - **The family's arithmetic, which is the sixth sweep's with the sign reversed.** The sixth reads
//!   the statuses of the rows below a clause to judge the parent's **status**; this reads the same
//!   numbers to judge the parent's **prose**. So the answer side is derived from the file rather
//!   than decided by a person: a [`Family`] publishes every cardinality its own rows can produce —
//!   the sizes for a denominator, the per-status counts for a part — and a number none of them
//!   produces is the reading list.
//!
//! **Both together make the level a property of the ledger**, which is what ADRs 0360, 0388 and
//! 0397 each asked of the sweep they took over: two runs of this program over the same tree print
//! the same numbers, and a number that moves is the ledger or the tree moving.
//!
//! # The two rungs that are counted rather than printed
//!
//! - **Agreement.** Some cardinality of the family is the number, and which one is printed with the
//!   hit so that the convention this ledger keeps is legible rather than re-derived: a count that
//!   excludes the family's own `General` row is right, and five parent rows read correctly only that
//!   way (§14.10.3, §14.10.4, §14.8.4.7, §7.6.4.4, §14.8.2).
//! - **Childless.** The clause the sentence attributes the count to has no rows below it, so the
//!   sentence is counting something the ledger does not hold. A sweep that read those as defects
//!   would print the same dozen every round.
//!
//! # The contradiction, which needs no family at all
//!
//! Two claims in one place, about one family, with the same noun and different numbers, are wrong
//! whatever the ledger holds — and this is the only check here whose evidence is entirely inside the
//! prose. §11.7 said "[t]wo of its five subclauses are satisfied" and, four sentences later, "four
//! of its five subclauses are satisfied", for **410 sessions**; §14.11 counted its seven subclauses
//! twice. Both are the sixth failure shape — a document corrected by appending, read from the
//! correction backwards — and a reader who starts at the correction never sees the first half.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument, and three noise shapes survive every rule above:
//!
//! - **A count of the standard's rows rather than the ledger's.** "Two subclauses of the standard
//!   disagreeing" governs a structural noun and counts an argument.
//! - **A count qualified by a status.** "The two rows below it that are `writer-side`" is a part with
//!   its denominator elided, and this sweep judges a number with no `of` in front of it as a size.
//!   Admitting a status count there would make the accept-set of a bare number almost everything a
//!   small family can produce, which costs the denominators every defect so far has been in.
//! - **A count of the rows *beside* the clause.** §8.11.1's three subclauses and §11.7.4.1's four
//!   are the family the row sits *in* rather than the family below it, which is this ledger's
//!   commonest idiom for a `General` row.
//! - **A round's own record.** This project quotes every count it retires in the sentence that
//!   retires it, so [`retired::kind_of`] marks a correction rather than dropping it.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::clause::ClauseNumber;
use crate::ledger::{Ledger, Status};
use crate::retired::{self, Kind};

/// The ledger's own words for the rows below a clause.
///
/// Deliberately short, and every one of them names a *row* rather than a thing a clause states. A
/// noun this list does not hold — `entries`, `properties`, `filters`, `styles` — is a sentence
/// counting the standard's furniture rather than the ledger's, which is the noise the hand-runs
/// printed most. `below` is here because "the twelve below" is this ledger's own elliptical form for
/// the same population.
pub const NOUNS: [&str; 7] = [
    "row",
    "rows",
    "subclause",
    "subclauses",
    "child",
    "children",
    "below",
];

/// How many words may stand between a cardinal and the noun it governs.
///
/// Wide enough for the qualifications this project writes — "twelve `partial` rows", "seven of its
/// own direct children" — and no wider. A cardinal further than this from a structural noun is in a
/// different noun phrase.
pub const REACH: usize = 3;

/// The one noun that has to follow the cardinal immediately.
///
/// `below` is an ellipsis rather than a noun — "the twelve below" — so a word between it and the
/// number is the real noun, and "its own table twelve lines below" counts lines. Measured on the
/// first run, where that sentence was the sweep's sharpest-looking false positive.
pub const ADJACENT: &str = "below";

/// The punctuation that ends a cardinal's reach.
///
/// A semicolon or a colon joins two independent clauses and an em dash starts an aside, so a noun
/// after one belongs to a different sentence in all but name: "named three of the ten; §12.6.4's
/// row carries the count" counts action types and then mentions a row.
pub const BREAKS: [char; 3] = [';', ':', '—'];

/// The statuses that owe nothing further.
pub const SETTLED: [Status; 4] = [
    Status::Implemented,
    Status::Inapplicable,
    Status::OutOfScope,
    Status::WriterSide,
];

/// What a cardinal in a sentence is counting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// The population itself — "the twelve rows below", or the `M` of "N of the M subclauses".
    Size,
    /// A part of it — the `N` of "N of the M subclauses", which is a count of rows in some state.
    Part,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Size => "the family's size",
            Self::Part => "a part of the family",
        })
    }
}

/// What the rows below one clause can be counted as.
///
/// **The sixth sweep's arithmetic, published rather than compared.** Every number here is derived
/// from the ledger, so a claim is judged against the file instead of against a reader's memory of
/// the convention.
#[derive(Debug, Clone, Default)]
pub struct Family {
    /// How many rows are direct children.
    pub direct: usize,
    /// How many direct children are not the family's own `General` row.
    ///
    /// The convention five parent rows count by, and the reason it is a cardinality rather than a
    /// correction: §14.8.4.7's "the three below" is right about four rows, one of which is `General`.
    ///
    /// **A second convention is derived rather than listed here**, because it is arithmetic on these
    /// numbers: this project counts "§11.7 — fourteen rows" and "§7.6, a clause family of 34
    /// subclauses" with the clause's *own* row in the family, so `descendants + 1` and `direct + 1`
    /// are cardinalities too. Measured on the first run, where that idiom was most of the noise.
    pub direct_named: usize,
    /// How many rows are descendants at any depth.
    pub descendants: usize,
    /// How many descendants are not a `General` row.
    pub descendants_named: usize,
    /// How many direct children carry each status.
    pub by_status: BTreeMap<Status, usize>,
    /// How many descendants carry each status.
    pub deep_by_status: BTreeMap<Status, usize>,
}

impl Family {
    /// Whether the ledger holds any row below the clause at all.
    #[must_use]
    pub fn exists(&self) -> bool {
        self.descendants > 0
    }

    /// Every cardinality a claim in one role may legitimately be, with the name of each.
    #[must_use]
    pub fn cardinalities(&self, role: Role) -> Vec<(usize, String)> {
        let mut found = vec![
            (self.direct, "its direct children".to_owned()),
            (
                self.direct_named,
                "its direct children bar the General row".to_owned(),
            ),
            (self.descendants, "its descendants".to_owned()),
            (
                self.descendants_named,
                "its descendants bar the General rows".to_owned(),
            ),
            (
                self.descendants.saturating_add(1),
                "its rows, the clause's own included".to_owned(),
            ),
            (
                self.direct.saturating_add(1),
                "its children, the clause's own row included".to_owned(),
            ),
        ];
        if role == Role::Part {
            for (status, count) in &self.by_status {
                found.push((*count, format!("its `{status}` children")));
            }
            for (status, count) in &self.deep_by_status {
                found.push((*count, format!("its `{status}` descendants")));
            }
            found.push((self.settled(false), "its settled children".to_owned()));
            found.push((self.settled(true), "its settled descendants".to_owned()));
            found.push((
                self.direct.saturating_sub(self.settled(false)),
                "its owing children".to_owned(),
            ));
            found.push((
                self.descendants.saturating_sub(self.settled(true)),
                "its owing descendants".to_owned(),
            ));
        }
        found.retain(|(count, _)| *count > 0);
        found
    }

    /// How many rows owe nothing further, over the descendants or over the direct children.
    #[must_use]
    pub fn settled(&self, deep: bool) -> usize {
        let counted = if deep {
            &self.deep_by_status
        } else {
            &self.by_status
        };
        SETTLED
            .iter()
            .map(|status| counted.get(status).copied().unwrap_or_default())
            .fold(0usize, usize::saturating_add)
    }

    /// What the family says about one number in one role.
    #[must_use]
    pub fn judge(&self, role: Role, count: usize) -> Verdict {
        if !self.exists() {
            return Verdict::Childless;
        }
        match self
            .cardinalities(role)
            .into_iter()
            .find(|(cardinality, _)| *cardinality == count)
        {
            Some((_, named)) => Verdict::Agrees(named),
            None => Verdict::Absent,
        }
    }

    /// The cardinality nearest one number, for the order the suspects are read in.
    #[must_use]
    pub fn nearest(&self, role: Role, count: usize) -> Option<usize> {
        self.cardinalities(role)
            .into_iter()
            .map(|(cardinality, _)| cardinality.abs_diff(count))
            .min()
    }
}

/// Every clause's family, built once from the ledger.
#[derive(Debug, Clone, Default)]
pub struct Families {
    /// One entry per row, whether or not anything sits below it.
    families: BTreeMap<ClauseNumber, Family>,
}

impl Families {
    /// Builds every family the ledger's rows form.
    #[must_use]
    pub fn of(ledger: &Ledger) -> Self {
        let mut families: BTreeMap<ClauseNumber, Family> = ledger
            .rows
            .iter()
            .map(|row| (row.clause.clone(), Family::default()))
            .collect();
        for row in &ledger.rows {
            let general = row.title.eq_ignore_ascii_case("general");
            for (clause, family) in &mut families {
                if !clause.is_ancestor_of(&row.clause) {
                    continue;
                }
                family.descendants = family.descendants.saturating_add(1);
                let deep = family.deep_by_status.entry(row.status).or_default();
                *deep = deep.saturating_add(1);
                if !general {
                    family.descendants_named = family.descendants_named.saturating_add(1);
                }
                if row.clause.depth() != clause.depth().saturating_add(1) {
                    continue;
                }
                family.direct = family.direct.saturating_add(1);
                let counted = family.by_status.entry(row.status).or_default();
                *counted = counted.saturating_add(1);
                if !general {
                    family.direct_named = family.direct_named.saturating_add(1);
                }
            }
        }
        Self { families }
    }

    /// One clause's family, and an empty one for a clause the ledger has no row for.
    #[must_use]
    pub fn at(&self, clause: &ClauseNumber) -> Family {
        self.families.get(clause).cloned().unwrap_or_default()
    }

    /// How many clauses have a row below them.
    #[must_use]
    pub fn parents(&self) -> usize {
        self.families
            .values()
            .filter(|family| family.exists())
            .count()
    }
}

/// What a family says about a number a sentence attributes to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Some cardinality of the family is the number, and this is its name.
    Agrees(String),
    /// No cardinality of the family is the number — the reading list.
    Absent,
    /// The clause the count is attributed to has no rows below it, so the sentence is counting
    /// something this ledger does not hold.
    Childless,
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Agrees(named) => write!(f, "agrees with {named}"),
            Self::Absent => f.write_str("no cardinality of the family"),
            Self::Childless => f.write_str("the clause has no rows below it"),
        }
    }
}

/// One number one sentence attributes to one family, before the ledger is asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// The number itself.
    pub count: usize,
    /// What it counts.
    pub role: Role,
    /// The noun that made it a claim about rows.
    pub noun: String,
    /// The clause the sentence names before the number, where it names one.
    ///
    /// `None` is the elliptical form — "its five subclauses", "the twelve rows below" — which in a
    /// ledger note means the row's own clause and in a document means nothing checkable.
    pub named: Option<ClauseNumber>,
}

/// One claim, judged, with everything a reader needs to decide it.
#[derive(Debug, Clone)]
pub struct Counted {
    /// The number and what it counts.
    pub claim: Claim,
    /// The family it was attributed to.
    pub clause: ClauseNumber,
    /// Whether the sentence named that clause, or it is the row the sentence is in.
    pub from_sentence: bool,
    /// What the family says.
    pub verdict: Verdict,
    /// What the family's own rows can be counted as, for the correction a defect needs.
    pub cardinalities: Vec<(usize, String)>,
    /// How far the number is from the nearest of them.
    pub distance: usize,
    /// Where the sentence is.
    pub location: String,
    /// The sentence, whole, because a suspect is a reading list and not a finding.
    pub sentence: String,
    /// Whether the sentence narrates a correction — the shape that quotes the count it retired.
    pub kind: Kind,
}

/// Two claims in one place about one family, disagreeing.
#[derive(Debug, Clone)]
pub struct Contradiction {
    /// The family both count.
    pub clause: ClauseNumber,
    /// The noun both govern.
    pub noun: String,
    /// What they count.
    pub role: Role,
    /// Where the place is.
    pub location: String,
    /// The two numbers and the sentences that state them, in the order they are written.
    pub stated: Vec<(usize, String)>,
}

/// What one run read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Every claim the run judged, in the order it read them.
    pub counted: Vec<Counted>,
    /// How many sentences govern a structural noun at all, attributing a number or not.
    pub sentences: usize,
    /// How many clauses the ledger gives a row below them.
    pub parents: usize,
    /// The places stating two different numbers for one family and noun.
    pub contradictions: Vec<Contradiction>,
}

impl Report {
    /// The claims the family agrees with.
    #[must_use]
    pub fn agreeing(&self) -> usize {
        self.counted
            .iter()
            .filter(|counted| matches!(counted.verdict, Verdict::Agrees(_)))
            .count()
    }

    /// The claims attributed to a clause with no rows below it.
    #[must_use]
    pub fn childless(&self) -> usize {
        self.counted
            .iter()
            .filter(|counted| counted.verdict == Verdict::Childless)
            .count()
    }

    /// The suspects, sharpest first.
    ///
    /// A standing claim above a correction, for the ninth sweep's reason, and then **nearest
    /// first**: a number one out from a cardinality the family produces is the shape of every
    /// defect this sweep has found — §14.8.2's twelve over thirteen, §12.6.3's eleven over ten — and
    /// a number nothing near is usually prose about the standard.
    #[must_use]
    pub fn suspects(&self) -> Vec<&Counted> {
        let mut suspects: Vec<&Counted> = self
            .counted
            .iter()
            .filter(|counted| counted.verdict == Verdict::Absent)
            .collect();
        suspects.sort_by(|left, right| {
            (left.kind == Kind::Correction, left.distance, &left.location).cmp(&(
                right.kind == Kind::Correction,
                right.distance,
                &right.location,
            ))
        });
        suspects
    }
}

/// What one place counts: a family, a word for its rows, and which of the two roles.
///
/// The key a contradiction is found under, because all three have to be the same before two numbers
/// disagree about anything.
type Counting = (ClauseNumber, String, Role);

/// Runs the sweep over the ledger's notes, the tree's comments and this project's prose.
///
/// `sources` are the Rust files under [`crate::SOURCE_ROOTS`] and `documents` the Markdown under
/// `doc/`. Two directories are read by nothing, for the reasons [`retired::NOT_SWEPT`] and
/// [`crate::NOT_SCANNED`] give: a round's own record is not another round's to correct, and this
/// checker's own prose states the wrong counts as examples.
#[must_use]
pub fn sweep(
    ledger: &Ledger,
    sources: &[(PathBuf, String)],
    documents: &[(PathBuf, String)],
) -> Report {
    let families = Families::of(ledger);
    let mut places: Vec<(String, Option<ClauseNumber>, String)> = Vec::new();
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
                Some(row.clause.clone()),
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
            places.push((format!("{shown}:{line}"), None, block));
        }
    }
    for (path, text) in documents {
        let shown = shown(path);
        if shown.starts_with(retired::NOT_SWEPT) {
            continue;
        }
        for (line, block) in retired::paragraphs(text) {
            places.push((format!("{shown}:{line}"), None, block));
        }
    }

    let mut report = Report {
        parents: families.parents(),
        ..Report::default()
    };
    for (location, own, block) in &places {
        let mut stated: BTreeMap<Counting, Vec<(usize, usize, String)>> = BTreeMap::new();
        for (which, sentence) in crate::unread::sentences(block).into_iter().enumerate() {
            if !governs_a_noun(sentence) {
                continue;
            }
            report.sentences = report.sentences.saturating_add(1);
            let kind = retired::kind_of(sentence);
            for claim in claims_in(sentence) {
                let Some(clause) = claim.named.clone().or_else(|| own.clone()) else {
                    continue;
                };
                let family = families.at(&clause);
                let verdict = family.judge(claim.role, claim.count);
                stated
                    .entry((clause.clone(), claim.noun.clone(), claim.role))
                    .or_default()
                    .push((which, claim.count, sentence.to_owned()));
                report.counted.push(Counted {
                    clause,
                    from_sentence: claim.named.is_some(),
                    verdict,
                    cardinalities: family.cardinalities(claim.role),
                    distance: family
                        .nearest(claim.role, claim.count)
                        .unwrap_or(usize::MAX),
                    location: location.clone(),
                    sentence: sentence.to_owned(),
                    kind,
                    claim,
                });
            }
        }
        for ((clause, noun, role), numbers) in stated {
            // Two numbers in **different sentences**. One sentence naming two is a phrase with two
            // roles in it — "one of the fourteen rows", "19 unreviewed rows out of 20" — and the
            // shape this looks for is the sixth failure shape: a count appended to a paragraph
            // whose first half nobody re-read, "four sentences later" in §11.7's own words.
            let disagreeing = numbers.iter().any(|(which, count, _)| {
                numbers
                    .iter()
                    .any(|(other, seen, _)| other != which && seen != count)
            });
            if disagreeing {
                report.contradictions.push(Contradiction {
                    clause,
                    noun,
                    role,
                    location: location.clone(),
                    stated: numbers
                        .into_iter()
                        .map(|(_, count, sentence)| (count, sentence))
                        .collect(),
                });
            }
        }
    }
    report
}

/// A path as this project writes one.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Whether a sentence governs one of [`NOUNS`] at all, cardinal or not.
///
/// The population's denominator, counted so that a clean run says what it was clean over.
#[must_use]
pub fn governs_a_noun(sentence: &str) -> bool {
    words_of(sentence)
        .iter()
        .any(|word| NOUNS.contains(&word.text.as_str()))
}

/// Every number one sentence attributes to a family.
///
/// The attribution is what makes a number a claim about the rows below a clause rather than a
/// number that happens to share a sentence with one; the module documentation says which forms
/// count and why the noun list is short.
#[must_use]
pub fn claims_in(sentence: &str) -> Vec<Claim> {
    let words = words_of(sentence);
    let mut found = Vec::new();
    for (position, word) in words.iter().enumerate() {
        let Some(count) = word.cardinal else {
            continue;
        };
        let Some((at, noun)) = noun_after(&words, position) else {
            continue;
        };
        let named = clause_before(sentence, at);
        // "N of the M subclauses" states the population at M and a count of it at N, so the
        // number before the `of` is a part of the family and the number governing the noun is
        // its size. Nothing else in this ledger's prose puts two cardinals in one noun phrase.
        if let Some(part) = part_before(&words, position) {
            found.push(Claim {
                count: part,
                role: Role::Part,
                noun: noun.clone(),
                named: named.clone(),
            });
        }
        // A bare `one` before a noun is English's determiner rather than a quantity — "its one
        // unsettled child", "one row per page" — and every hit of it on the first run was that. A
        // part of one keeps its claim, because a denominator is what makes it arithmetic.
        if count > 1 {
            found.push(Claim {
                count,
                role: Role::Size,
                noun,
                named,
            });
        }
    }
    found
}

/// One word of a sentence, as the extraction needs it.
struct Word {
    /// The byte offset it begins at.
    offset: usize,
    /// The word with this project's markup and punctuation removed, lower-cased.
    text: String,
    /// The quantity it states, read off the **raw** word by [`cardinal_of`].
    cardinal: Option<usize>,
    /// Whether the punctuation after it ends a cardinal's reach ([`BREAKS`]).
    breaks: bool,
}

/// The words of a sentence, each with the byte offset it begins at.
fn words_of(sentence: &str) -> Vec<Word> {
    let mut words = Vec::new();
    let mut start: Option<usize> = None;
    for (offset, character) in sentence.char_indices() {
        match (character.is_whitespace(), start) {
            (false, None) => start = Some(offset),
            (true, Some(at)) => {
                words.push(word_at(sentence, at, offset));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(at) = start {
        words.push(word_at(sentence, at, sentence.len()));
    }
    words
}

/// One word of a sentence, read out of the span it occupies.
fn word_at(sentence: &str, at: usize, end: usize) -> Word {
    let raw = sentence.get(at..end).unwrap_or("");
    Word {
        offset: at,
        text: raw
            .chars()
            .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
            .flat_map(char::to_lowercase)
            .collect(),
        cardinal: cardinal_of(raw),
        breaks: raw.chars().any(|character| BREAKS.contains(&character)),
    }
}

/// The structural noun a cardinal governs, with the offset it stands at.
///
/// Within [`REACH`] words, not across a [`BREAKS`] mark, and immediately for [`ADJACENT`]. An `of`
/// between the number and the noun means this number is a **part** of the population rather than the
/// population — "one of four rows" — and the size claim is the cardinal after that `of`, so nothing
/// is raised here; otherwise one phrase would count the family twice and contradict itself.
fn noun_after(words: &[Word], position: usize) -> Option<(usize, String)> {
    if words.get(position)?.breaks {
        return None;
    }
    for step in 1..=REACH {
        let word = words.get(position.saturating_add(step))?;
        if NOUNS.contains(&word.text.as_str()) && (step == 1 || word.text != ADJACENT) {
            return Some((word.offset, word.text.clone()));
        }
        if word.breaks || word.text == "of" {
            return None;
        }
    }
    None
}

/// The cardinal before an `of` immediately preceding this one, which is a part of the population.
fn part_before(words: &[Word], position: usize) -> Option<usize> {
    let mut seen_of = false;
    for step in 1..=REACH {
        let word = words.get(position.checked_sub(step)?)?;
        if word.breaks {
            return None;
        }
        if word.text == "of" {
            seen_of = true;
            continue;
        }
        if seen_of {
            return word.cardinal;
        }
    }
    None
}

/// The last clause number the sentence names before one byte offset.
///
/// The container, exactly as the ninth sweep takes a table from the sentence that names it, and read
/// up to the **noun** rather than to the number so that "the eleven §12.6.4 subclauses" is a claim
/// about the family it names. [`crate::blockers::clauses_in`] keeps first occurrences, so a clause
/// repeated later in the prefix is attributed from where it was first written — which changes
/// nothing about *which* clause is named and is worth knowing before reading an attribution.
fn clause_before(sentence: &str, offset: usize) -> Option<ClauseNumber> {
    crate::blockers::clauses_in(sentence.get(..offset)?)
        .into_iter()
        .next_back()
}

/// The tens this project writes as words.
const TENS: [(&str, usize); 8] = [
    ("twenty", 20),
    ("thirty", 30),
    ("forty", 40),
    ("fifty", 50),
    ("sixty", 60),
    ("seventy", 70),
    ("eighty", 80),
    ("ninety", 90),
];

/// The units and teens this project writes as words.
const UNITS: [(&str, usize); 19] = [
    ("one", 1),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
    ("ten", 10),
    ("eleven", 11),
    ("twelve", 12),
    ("thirteen", 13),
    ("fourteen", 14),
    ("fifteen", 15),
    ("sixteen", 16),
    ("seventeen", 17),
    ("eighteen", 18),
    ("nineteen", 19),
];

/// One word as a cardinal, in digits or in the words this project writes.
///
/// The word is taken **raw**, with only the markup this project wraps a word in trimmed off either
/// end (`**14**`, `` `14` ``, `12,`), because what is inside a word decides whether it is a quantity
/// at all. Three shapes are refused and each was a false cardinal on the first run:
///
/// - **A number with a full stop in it.** `§9.6` reduced to its digits is `96`, and `1.7` is a
///   version. Neither is a count of anything.
/// - **A leading zero.** `ADR 0027` is a name.
/// - **Three digits or more.** A number of rows is a small number, and admitting `1024` or a year
///   would make every measurement in a comment a count of somebody's family.
#[must_use]
pub fn cardinal_of(word: &str) -> Option<usize> {
    let word: &str = word.trim_matches(|character: char| !character.is_ascii_alphanumeric());
    if word.is_empty()
        || !word
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return None;
    }
    if word.chars().all(|character| character.is_ascii_digit()) {
        if word.len() > 1 && word.starts_with('0') {
            return None;
        }
        return word.parse().ok().filter(|count| *count < 100);
    }
    let word = word.to_ascii_lowercase();
    if let Some((tens, units)) = word.split_once('-') {
        let tens = TENS.iter().find(|(name, _)| *name == tens)?.1;
        let units = UNITS
            .iter()
            .find(|(name, _)| *name == units)
            .map(|(_, value)| *value)
            .filter(|value| *value < 10)?;
        return Some(tens.saturating_add(units));
    }
    UNITS
        .iter()
        .chain(TENS.iter())
        .find(|(name, _)| *name == word)
        .map(|(_, value)| *value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Row;

    fn row(clause: &str, status: Status, title: &str, note: &str) -> Row {
        Row {
            clause: clause.parse().expect("a clause number"),
            title: title.to_owned(),
            status,
            code: Vec::new(),
            test: Vec::new(),
            exclusion: None,
            note: Some(note.to_owned()),
            line: 1,
        }
    }

    fn clause(number: &str) -> ClauseNumber {
        number.parse().expect("a clause number")
    }

    /// A family of five with one `General` row can be counted four ways, and the ledger's own
    /// convention — the count that leaves `General` out — is one of them.
    #[test]
    fn a_familys_cardinalities_come_off_the_rows() {
        let ledger = Ledger {
            rows: vec![
                row("11.7", Status::Partial, "Colour", "the parent."),
                row("11.7.1", Status::Inapplicable, "General", "one sentence."),
                row("11.7.2", Status::Partial, "Groups", "owed."),
                row("11.7.3", Status::Implemented, "Spot", "done."),
                row("11.7.4", Status::Partial, "Overprint", "owed."),
                row("11.7.4.1", Status::Inapplicable, "General", "one sentence."),
            ],
        };
        let family = Families::of(&ledger).at(&clause("11.7"));
        assert_eq!(family.direct, 4);
        assert_eq!(family.direct_named, 3, "§11.7.1 is the General row");
        assert_eq!(family.descendants, 5);
        assert_eq!(
            family.settled(false),
            2,
            "one inapplicable, one implemented"
        );
        assert_eq!(
            family.judge(Role::Size, 4),
            Verdict::Agrees("its direct children".to_owned())
        );
        assert_eq!(
            family.judge(Role::Size, 6),
            Verdict::Agrees("its rows, the clause's own included".to_owned()),
            "this project counts a family with the clause's own row in it"
        );
        assert_eq!(family.judge(Role::Size, 9), Verdict::Absent);
    }

    /// A cardinal is a claim only where it governs one of the ledger's own words for a row.
    #[test]
    fn a_count_of_something_else_is_not_a_claim() {
        assert!(claims_in("Four of them are stream filters.").is_empty());
        assert!(claims_in("Three of the clause's properties are read.").is_empty());
        assert_eq!(
            claims_in("Of the twelve rows below, two are owed.").len(),
            1
        );
    }

    /// "N of the M subclauses" is two claims: M is the population and N is a part of it.
    #[test]
    fn a_denominator_and_a_part_are_two_claims() {
        let claims = claims_in("Two of its five subclauses are satisfied.");
        assert_eq!(claims.len(), 2);
        assert_eq!(claims[0].count, 2);
        assert_eq!(claims[0].role, Role::Part);
        assert_eq!(claims[1].count, 5);
        assert_eq!(claims[1].role, Role::Size);
        assert_eq!(claims[1].noun, "subclauses");
    }

    /// The container is the clause the sentence names, which is how a count of another family's
    /// rows becomes checkable at all.
    #[test]
    fn a_named_clause_is_the_family_the_count_is_about() {
        let claims = claims_in("Every action is one of the eleven §12.6.4 subclauses performed.");
        assert_eq!(claims.len(), 2, "one of the eleven is a part and a size");
        assert_eq!(
            claims[1].named.as_ref(),
            Some(&clause("12.6.4")),
            "a clause between the number and the noun is the family the count is about"
        );
        let claims = claims_in("§12.6.4's eighteen children are what a trigger reaches.");
        assert_eq!(claims[0].named.as_ref(), Some(&clause("12.6.4")));
    }

    /// A number the family cannot produce is the reading list, and a number it can is counted.
    #[test]
    fn the_number_a_family_cannot_produce_is_the_suspect() {
        let ledger = Ledger {
            rows: vec![
                row(
                    "14.8.2",
                    Status::Partial,
                    "Structure",
                    "Of the twelve rows below, two of the three subclauses are owed.",
                ),
                row("14.8.2.1", Status::Implemented, "General", "done."),
                row("14.8.2.2", Status::Implemented, "Order", "done."),
                row("14.8.2.3", Status::Partial, "Text", "owed."),
            ],
        };
        let report = sweep(&ledger, &[], &[]);
        let suspects = report.suspects();
        assert_eq!(suspects.len(), 1, "twelve is no cardinality of three rows");
        assert_eq!(suspects[0].claim.count, 12);
        assert_eq!(suspects[0].clause, clause("14.8.2"));
        assert!(!suspects[0].from_sentence, "the row's own family");
        assert_eq!(
            report.agreeing(),
            2,
            "the three subclauses are the direct children and two of them are `implemented`"
        );
        assert_eq!(report.parents, 1);
    }

    /// A clause with no rows below it is counting something the ledger does not hold, which is a
    /// rung rather than a finding.
    #[test]
    fn a_count_under_a_childless_clause_is_counted_not_printed() {
        let ledger = Ledger {
            rows: vec![row(
                "8.5.3.3.1",
                Status::Partial,
                "General",
                "Two subclauses of the standard disagree.",
            )],
        };
        let report = sweep(&ledger, &[], &[]);
        assert!(report.suspects().is_empty());
        assert_eq!(report.childless(), 1);
    }

    /// Two numbers for one family in one note are wrong whatever the ledger holds, and this is the
    /// only check here whose evidence is entirely inside the prose.
    #[test]
    fn one_place_counting_a_family_twice_contradicts_itself() {
        let ledger = Ledger {
            rows: vec![
                row(
                    "11.7",
                    Status::Partial,
                    "Colour",
                    "Two of its five subclauses are satisfied. Four of its five subclauses are \
                     satisfied.",
                ),
                row("11.7.1", Status::Inapplicable, "General", "one sentence."),
                row("11.7.2", Status::Partial, "Groups", "owed."),
                row("11.7.3", Status::Implemented, "Spot", "done."),
                row("11.7.4", Status::Partial, "Overprint", "owed."),
                row("11.7.5", Status::Partial, "Marks", "owed."),
            ],
        };
        let report = sweep(&ledger, &[], &[]);
        assert_eq!(report.contradictions.len(), 1);
        let contradiction = &report.contradictions[0];
        assert_eq!(contradiction.role, Role::Part);
        assert_eq!(
            contradiction
                .stated
                .iter()
                .map(|(count, _)| *count)
                .collect::<Vec<usize>>(),
            vec![2, 4]
        );
    }

    /// A cardinal is a small number written as digits or as this project's words, and a hyphenated
    /// ten is one number.
    #[test]
    fn a_cardinal_is_read_in_both_forms() {
        assert_eq!(cardinal_of("twelve"), Some(12));
        assert_eq!(cardinal_of("twenty-five"), Some(25));
        assert_eq!(cardinal_of("18"), Some(18));
        assert_eq!(cardinal_of("1024"), None, "not a count of rows");
        assert_eq!(cardinal_of("0027"), None, "an ADR's name, not a quantity");
        assert_eq!(cardinal_of("twenty-twenty"), None);
        assert_eq!(cardinal_of("several"), None);
    }

    /// The three shapes the first run's noise had, each removed by a rule about the sentence
    /// rather than about the subject.
    #[test]
    fn the_reach_stops_at_a_real_noun_and_at_a_break() {
        assert!(
            claims_in("Its own table twelve lines below says the opposite.").is_empty(),
            "`below` is an ellipsis and `lines` is the noun"
        );
        assert_eq!(
            claims_in("Of the twelve below, four are owed.").len(),
            1,
            "the ellipsis counts where the number governs it directly"
        );
        assert!(
            claims_in("It named three of the ten; §12.6.4's row carries the count.").is_empty(),
            "a semicolon ends the reach"
        );
    }

    /// The checker's own directory states the wrong counts as examples, so it witnesses nothing.
    #[test]
    fn the_checkers_own_documentation_is_not_a_place() {
        let ledger = Ledger {
            rows: vec![row("11.7", Status::Partial, "Colour", "the parent.")],
        };
        let sources = vec![(
            PathBuf::from("tools/conformance/src/counts.rs"),
            "//! Two of its five subclauses are satisfied.\n".to_owned(),
        )];
        assert_eq!(sweep(&ledger, &sources, &[]).sentences, 0);
    }
}
