//! The twenty-fourth sweep: a `partial` row whose debt the standard states as a permission.
//!
//! # The shape it exists for
//!
//! `ledger.toml`'s own header defines `partial` as "some [normative requirements] are
//! [executed]; the note says which are not". A clause entry the standard offers with *may* or
//! *can* is not a normative requirement on a reader, so a row `partial` for one says a
//! requirement is unexecuted where the standard states none. The nine-hundred-and-twenty-eighth
//! session found four instances in one band of five rows — §12.11.5's `/RH`, §14.9.2's and
//! §14.9.2.2's Table 122 `/Lang`, §14.7.4.2's `/Schema` — and ADR 0896 records both the shape
//! and the reason no sweep in `doc/todo/01` can print it: **every one of them reads a row that
//! owes something and asks whether the owed thing exists in the tree.** This one is a row that
//! owes nothing and says so, and the discriminator is on neither side of that comparison. It is
//! the *clause's* modal verb.
//!
//! `CLAUDE.md` decides which way a permission goes, in the sentence it uses about flatness: "a
//! clause that permits is a clause that has been read, and it is a stronger answer than one that
//! does not apply".
//!
//! # The discriminator, and why the right-hand side is the standard
//!
//! ADR 0896 offers a grep over the note — a debt sentence containing *may*, *can* or *is
//! permitted*. That reads the row's own prose, and `doc/ledger-and-claims.md`'s standing rule is
//! that **a row's note is not evidence**. So this sweep reads the note only for its *quotations*,
//! locates each one in `doc/md/`, and takes the modal verb from **the standard's own sentence**
//! holding it ([`prose::Conversion::sentence_holding`]). What a row is ranked by is the strongest
//! modal any of its quotations turns out to sit under:
//!
//! | the strongest verb over the row's quotations | what it says about the status |
//! |---|---|
//! | `shall`, `shall not`, `required` | the row quotes a requirement; nothing to read here |
//! | `should` | a recommendation, which ISO's directives do not make a requirement |
//! | `may`, `can`, `need not`, `is permitted`, `optional` | ADR 0896's shape exactly |
//! | none of them | the row quotes a statement of fact and calls it a debt |
//!
//! A row quoting nothing the conversion holds is counted and listed last: there is no sentence to
//! read a verb off, and the note argues in prose alone.
//!
//! # The half that reads a table entry, and why the sentence scan alone is not enough
//!
//! **Calibrated against session 928's own four rows, the sentence scan finds one of them** — and
//! that is a fact about the shape rather than about the scan. Three of the four notes quote the
//! standard for the half of the clause they *implement*: §14.9.2.2 quotes the `shall` about a
//! language identifier's grammar, which it executes, while its debt is Table 122's `/Lang`, which
//! it never quotes at all. A row's debt is very often an **entry**, named as a key and attributed
//! to a table, with no sentence quoted anywhere near it.
//!
//! So the sweep asks the same question of the standard's tables. Every `(table, key)` a note
//! attributes — [`crate::tables::attributions_in`]'s rule, the ninth sweep's, so that a key
//! merely sharing a sentence with a table number is not a claim about it — is looked up in the
//! table's own row ([`crate::entries::descriptions_in`]), and two things are read off it: the
//! `( Optional )` or `( Required )` the standard opens the description with, and the verb
//! governing the rest of it. A row **every one of whose named entries the standard states as
//! optional** is ADR 0896's shape read off the standard's own tables, and it is
//! [`Rank::Optional`], the closest rank there is. Only the first of the two decides the rank —
//! [`Entry::is_optional`] carries why, and it is the one thing here the calibration settled
//! rather than the argument.
//!
//! The two halves catch different rows and neither subsumes the other, which is why both are
//! here: the entry half finds §14.9.2 and §14.9.2.2, and the sentence half finds §12.11.5.
//!
//! # The second column, and it is ADR 0897's instrument
//!
//! A flagged row is a **reading list entry and never a verdict**, and the nine-hundred-and-
//! twenty-eighth session is the proof: §14.7.4.2 was `partial` for a permission and stayed
//! `partial`, because the clause's real `shall` sat in the prose *after* Table 356 and the row
//! had never named it. A modal scan over that row alone gets the answer backwards.
//!
//! So every finding carries [`Finding::shalls`]: how many sentences of the clause's own span in
//! `doc/md/` carry a `shall` **outside its tables and its NOTEs**. That is ADR 0897's suggested
//! instrument — "for each `partial` row, the clause's `shall` sentences that are not inside a
//! table" — and it tells the two shapes apart before the reading starts. A flagged row over a
//! clause with no such sentence is §14.9.2.2's shape, where the status is the only claim; a
//! flagged row over a clause with several is §14.7.4.2's, where the row has read the wrong half.
//!
//! # The noise, printed rather than filtered
//!
//! - **A note that quotes the standard for its *implemented* half.** A row's quotations are not
//!   all about its debt, so a row whose only `shall` is one it executes reads as silent here.
//!   That direction hides a finding rather than inventing one, which is the direction to be loose
//!   in.
//! - **A quotation of something other than ISO 32000-2.** The conversion is fourteen documents,
//!   and a note quoting WTPDF or an application note is located in whichever of them holds it.
//! - **A sentence that carries two verbs.** "X shall be Y, and may be Z" ranks as a requirement,
//!   which is right for the question being asked: the row does quote one.
//! - **A NOTE's verb.** ISO's directives make a NOTE informative, and a `can` inside one is not a
//!   permission the standard grants — it is prose about one. The sentence is printed under every
//!   hit so that a reader sees the `NOTE` in front of it; deciding it here would need the
//!   conversion's line structure, which [`quote::normalise`] has already collapsed.
//! - **An aggregate row whose debt is a child's.** §8.11.1 names Table 98's `/Configs`, a
//!   permission, *and* §8.11.4.5's "shall be reapplied" — three words, under [`quote::MIN_WORDS`],
//!   so the sweep sees only the first. A family's status follows its children and no verb here
//!   says so.
//! - **An optional entry whose description puts a `shall` on a *processor*.** The rank asks the
//!   table's own word, so such a row is flagged and is a real `partial`. §12.5.6.19's `/MK` —
//!   "(Optional) An appearance characteristics dictionary … that shall be used in constructing a
//!   dynamic appearance stream" — is the standing example, and the verb printed beside the entry
//!   is what tells a reader before the clause is opened.

use std::collections::BTreeMap;

use crate::clause::{ClauseIndex, ClauseNumber};
use crate::ledger::{Ledger, Status};
use crate::prose::Conversion;
use crate::{quote, tables, unread};

/// How many of a row's quotations one block prints.
pub const SHOWN: usize = 3;

/// The verb governing the standard's sentence, in the order ISO's directives rank them.
///
/// Derived from the sentence rather than from the note, and ordered so that the strongest verb a
/// row quotes is a `max` over its quotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verb {
    /// None of the three: a statement of fact, or a NOTE's prose.
    Bare,
    /// *may*, *can*, *need not*, *is permitted*, *optional* — a permission.
    Permission,
    /// *should* — a recommendation, which is not a requirement.
    Recommendation,
    /// *shall*, *shall not*, *required* — a requirement.
    Requirement,
}

impl Verb {
    /// The verb governing one sentence of the standard.
    ///
    /// Word-bounded rather than by substring: *cannot* contains *can*, *shallow* contains
    /// *shall*, and trap 27 is the standing warning about an assertion on a substring. A
    /// *cannot* is deliberately **not** a permission — it is a statement about what the format
    /// can express, which is why §8.6.5.7's "cannot be specified in PDF" must not read as one.
    #[must_use]
    pub fn of(sentence: &str) -> Self {
        let words: Vec<String> = sentence
            .split(|character: char| !character.is_ascii_alphabetic())
            .filter(|word| !word.is_empty())
            .map(str::to_ascii_lowercase)
            .collect();
        let says = |wanted: &str| words.iter().any(|word| word == wanted);
        if says("shall") || says("required") {
            return Self::Requirement;
        }
        if says("should") {
            return Self::Recommendation;
        }
        if says("may") || says("can") || says("permitted") || says("optional") {
            return Self::Permission;
        }
        Self::Bare
    }

    /// How the report writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requirement => "shall",
            Self::Recommendation => "should",
            Self::Permission => "may",
            Self::Bare => "no modal verb",
        }
    }
}

/// One quotation of a row's note, located in the standard.
#[derive(Debug, Clone)]
pub struct Located {
    /// The quotation as the note writes it.
    pub quotation: String,
    /// The standard's sentence holding it.
    pub sentence: String,
    /// The verb that sentence carries.
    pub verb: Verb,
}

/// One table entry a note names, with what the standard's own table row says about it.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The table the note attributes it to.
    pub table: u16,
    /// The key.
    pub key: String,
    /// Whether the standard opens its description with `Optional`; `None` where it opens with
    /// neither word, which is what a table whose first column is not an entry list looks like.
    pub optional: Option<bool>,
    /// The verb governing the description.
    pub verb: Verb,
    /// The description itself, as the table states it.
    pub description: String,
}

impl Entry {
    /// Whether the standard states the entry as optional.
    ///
    /// **The description's own `shall` does not disqualify it**, and that is the one design
    /// decision here that was made by calibration rather than by argument (trap 13). The rule
    /// began as *optional and described without a `shall`*, and under it Table 122's `/Lang` —
    /// three of the four rows ADR 0896 records — is not a hit, because its description reads
    /// "( Optional; PDF 1.5 ) A name specifying the language of the font, which may be used …
    /// The value shall be a Language-Tag as defined in BCP 47." That `shall` constrains the
    /// **value**, and a reader that declines an optional entry never reaches it; §14.9.2.2's row
    /// implements the grammar it names and was `partial` for the entry all the same.
    ///
    /// So the question the rank asks is the table's own word and nothing else, and the verb
    /// governing the description is *printed* beside it rather than gating it — because an
    /// optional entry whose description does put a `shall` on a **processor** is a real debt, and
    /// that is a distinction a reader makes and a program does not.
    #[must_use]
    pub fn is_optional(&self) -> bool {
        self.optional == Some(true)
    }
}

/// Which reading list a row is on, closest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    /// Every table entry the note names is one the standard states as optional.
    Optional,
    /// The row's strongest quoted verb is a permission — ADR 0896's shape.
    Permission,
    /// The row quotes the standard and no quotation carries a modal verb at all.
    Bare,
    /// The row's strongest quoted verb is a recommendation.
    Recommendation,
    /// The conversion holds none of the row's quotations, so there is no sentence to read.
    Unquoted,
}

impl Rank {
    /// How the report writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Optional => "every table entry it names is one the standard states as optional",
            Self::Permission => "the strongest verb over its quotations is a permission",
            Self::Bare => "it quotes the standard and no quotation carries a modal verb",
            Self::Recommendation => "the strongest verb over its quotations is a recommendation",
            Self::Unquoted => "the conversion holds none of its quotations",
        }
    }
}

/// One `partial` row that quotes no requirement.
#[derive(Debug, Clone)]
pub struct Finding {
    /// The clause whose row it is.
    pub clause: ClauseNumber,
    /// The 1-based line of the ledger it starts on.
    pub line: usize,
    /// The standard's title for the clause.
    pub title: String,
    /// Which reading list it is on.
    pub rank: Rank,
    /// Every quotation of the note the conversion holds, in the order the note writes them.
    pub located: Vec<Located>,
    /// How many quotations the note makes that the conversion does not hold.
    pub unlocated: usize,
    /// Every table entry the note attributes to a numbered table, in the order it names them.
    pub entries: Vec<Entry>,
    /// ADR 0897's column: `shall` sentences in the clause's own prose, outside tables and NOTEs.
    pub shalls: usize,
}

/// What one run read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Run {
    /// How many `partial` rows were read.
    pub rows: usize,
    /// How many of them quote a requirement, and are therefore not findings.
    pub quoting_a_requirement: usize,
    /// The findings, closest rank first and then by clause number.
    pub findings: Vec<Finding>,
}

impl Run {
    /// How many findings sit on one rank.
    #[must_use]
    pub fn on(&self, rank: Rank) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.rank == rank)
            .count()
    }
}

/// Reads every `partial` row of `ledger` against the standard's own modal verbs.
///
/// `described` is [`crate::entries::descriptions_in`] over the standard's conversion.
#[must_use]
pub fn sweep(
    ledger: &Ledger,
    conversion: &Conversion,
    index: &ClauseIndex,
    described: &BTreeMap<(u16, String), String>,
) -> Run {
    let mut run = Run::default();
    for row in &ledger.rows {
        if row.status != Status::Partial {
            continue;
        }
        run.rows = run.rows.saturating_add(1);
        let note = row.note.as_deref().unwrap_or_default();

        let mut located = Vec::new();
        let mut unlocated = 0usize;
        for (_, quotation) in quote::quoted_spans(note) {
            match conversion.sentence_holding(&quotation) {
                Some(sentence) => {
                    let verb = Verb::of(&sentence);
                    located.push(Located {
                        quotation,
                        sentence,
                        verb,
                    });
                }
                None => unlocated = unlocated.saturating_add(1),
            }
        }

        let entries = entries_named(note, described);
        let optional = !entries.is_empty() && entries.iter().all(Entry::is_optional);

        let strongest = located.iter().map(|found| found.verb).max();
        let rank = match (optional, strongest) {
            (true, _) => Rank::Optional,
            (false, Some(Verb::Requirement)) => {
                run.quoting_a_requirement = run.quoting_a_requirement.saturating_add(1);
                continue;
            }
            (false, Some(Verb::Recommendation)) => Rank::Recommendation,
            (false, Some(Verb::Permission)) => Rank::Permission,
            (false, Some(Verb::Bare)) => Rank::Bare,
            (false, None) => Rank::Unquoted,
        };

        run.findings.push(Finding {
            clause: row.clause.clone(),
            line: row.line,
            title: row.title.clone(),
            rank,
            located,
            unlocated,
            entries,
            shalls: shall_sentences(index, &row.clause),
        });
    }
    // Within a rank, ADR 0897's column is the tie-break, fewest first: a flagged row over a
    // clause whose own prose states no `shall` at all is a status with nothing under it, and a
    // flagged row over a clause stating a hundred of them is far likelier to have read one of
    // them and quoted it beside a debt that is genuinely a permission.
    run.findings.sort_by(|left, right| {
        (left.rank, left.shalls, &left.clause).cmp(&(right.rank, right.shalls, &right.clause))
    });
    run
}

/// Every table entry a note attributes to a numbered table, with the standard's own answer.
///
/// The attribution rule is [`tables::attributions_in`]'s and not "a key in the same sentence as a
/// table number", for the ninth sweep's reason: a note listing four keys after `Table 356` is
/// claiming all four are that table's, and a note that merely mentions a number beside a key is
/// claiming nothing. A **denied** attribution — "Table 119 gives a Type 0 dictionary no
/// `/FontDescriptor`" — is dropped rather than read, because the note is saying the entry is not
/// there and there is no description to take a verb from.
#[must_use]
pub fn entries_named(note: &str, described: &BTreeMap<(u16, String), String>) -> Vec<Entry> {
    let mut found: Vec<Entry> = Vec::new();
    for sentence in unread::sentences(note) {
        if !sentence.contains("Table ") {
            continue;
        }
        for claim in tables::attributions_in(sentence) {
            if claim.denied
                || found
                    .iter()
                    .any(|seen| seen.table == claim.table && seen.key == claim.key)
            {
                continue;
            }
            let Some(description) = described.get(&(claim.table, claim.key.clone())) else {
                continue;
            };
            found.push(Entry {
                table: claim.table,
                key: claim.key,
                optional: requirement_of(description),
                verb: Verb::of(description),
                description: description.clone(),
            });
        }
    }
    found
}

/// Whether a table entry's description opens by calling the entry optional.
///
/// The standard opens every entry description with a parenthesis — `( Optional; PDF 2.0 )`,
/// `(Required)`, `( Required if the document is encrypted; otherwise optional )`. **Required is
/// looked for first** because that last shape names both words and is a requirement under its own
/// condition; an entry the standard makes conditionally required owes something to a reader that
/// meets the condition.
///
/// `None` where the description opens with neither word, which is what a table that is not an
/// entry list looks like and is not evidence in either direction.
#[must_use]
pub fn requirement_of(description: &str) -> Option<bool> {
    let opening = description
        .split_once('(')?
        .1
        .split_once(')')
        .map_or(description, |(inside, _)| inside)
        .to_ascii_lowercase();
    if opening.contains("required") {
        return Some(false);
    }
    opening.contains("optional").then_some(true)
}

/// How many sentences of a clause's own span carry a `shall` outside its tables and its NOTEs.
///
/// ADR 0897's instrument. Three kinds of line are not the clause's normative prose and are
/// dropped before the sentences are counted:
///
/// - a Markdown **table row**, which is how the conversion writes every one of the standard's
///   tables — and a table's own `shall`s are what [`crate::entries`] already reads;
/// - a **NOTE** or an **EXAMPLE**, which ISO's directives make informative, and which the
///   conversion promotes to its own line;
/// - the **heading** itself.
///
/// Counted over the clause's whole span, subclauses included, because that is what a row covers.
#[must_use]
pub fn shall_sentences(index: &ClauseIndex, number: &ClauseNumber) -> usize {
    let Some(heading) = index
        .headings()
        .iter()
        .rfind(|heading| &heading.number == number)
    else {
        return 0;
    };
    let mut count = 0usize;
    for line in index.text_in(heading.span.clone()).lines() {
        let line = line.trim();
        if line.starts_with('|')
            || line.starts_with('#')
            || line.starts_with("NOTE")
            || line.starts_with("EXAMPLE")
        {
            continue;
        }
        for sentence in line.split(". ") {
            if Verb::of(sentence) == Verb::Requirement {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::{Rank, Verb};

    /// The four verbs, and the two words that contain another verb inside them.
    ///
    /// Trap 27's shape: *cannot* holds *can* and *shallow* holds *shall*, and a substring test
    /// reads the first as a permission and the second as a requirement. §8.6.5.7's note quotes
    /// "cannot be specified in PDF", which is a statement about the format rather than a licence.
    #[test]
    fn a_verb_is_read_by_word_and_not_by_substring() {
        assert_eq!(Verb::of("the value shall be a name"), Verb::Requirement);
        assert_eq!(
            Verb::of("a processor should check the flag"),
            Verb::Recommendation
        );
        assert_eq!(Verb::of("the entry may be provided"), Verb::Permission);
        assert_eq!(Verb::of("this cannot be specified in PDF"), Verb::Bare);
        assert_eq!(Verb::of("the region is shallow"), Verb::Bare);
    }

    /// The strongest verb wins, which is what makes a row quoting one requirement silent.
    #[test]
    fn a_sentence_carrying_two_verbs_ranks_as_the_stronger() {
        assert_eq!(
            Verb::of("the array shall be present and may hold two entries"),
            Verb::Requirement
        );
        assert!(Verb::Requirement > Verb::Permission);
        assert!(Verb::Permission > Verb::Bare);
    }

    /// The standard's three shapes of opening parenthesis, and the one that names both words.
    ///
    /// "Required if the document is encrypted; otherwise optional" is a requirement under its own
    /// condition, so a reader that meets the condition owes it — which is why the two words are
    /// not read in the order they appear.
    #[test]
    fn an_entry_required_under_a_condition_is_not_optional() {
        use super::requirement_of;
        assert_eq!(
            requirement_of("( Optional; PDF 2.0 ) A file specification"),
            Some(true)
        );
        assert_eq!(
            requirement_of("(Required) The type of PDF object"),
            Some(false)
        );
        assert_eq!(
            requirement_of("( Required if the document is encrypted; otherwise optional )"),
            Some(false)
        );
        assert_eq!(requirement_of("A name specifying the language"), None);
    }

    /// The reading order, closest first.
    #[test]
    fn a_permission_outranks_every_other_finding() {
        let mut ranks = [
            Rank::Unquoted,
            Rank::Recommendation,
            Rank::Bare,
            Rank::Permission,
            Rank::Optional,
        ];
        ranks.sort_unstable();
        assert_eq!(ranks[0], Rank::Optional);
        assert_eq!(ranks[4], Rank::Unquoted);
    }
}
