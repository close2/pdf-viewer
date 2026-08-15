//! The ninth sweep: does the table a sentence cites state the key it attributes to it?
//!
//! # The shape it exists for
//!
//! `tools/conformance`'s gate checks that a cited table **exists** and prints its title — the
//! eighty-second session added that after finding three ISO 32000-1 numbers in the ledger — and a
//! number that exists and names the wrong table reads exactly like a right one. Every other sweep
//! in `doc/todo/01` reads what a row *claims*; this one reads a *number*, against the entries the
//! standard's own table states.
//!
//! Its findings arrive in **blocks**: a wrong table number does not come alone, it comes as a run
//! of consecutive rows or one `enum`'s doc comments, written in one sitting against the older
//! standard. The first run corrected eighteen citations across two blocks of §12.5.6 and §14.8.5;
//! the five-hundred-and-thirty-seventh's found seven, six of them in `pdf-model` and five of those
//! in one `enum`.
//!
//! # What is a claim about a table, and what is only a key in the same sentence
//!
//! Reading every key in every sentence that names a table was 545 hits to nothing in the
//! five-hundred-and-thirty-seventh, because a sentence naming a table usually goes on to name the
//! dictionary the table describes. So a key counts only where the sentence **attributes** it: the
//! possessive (`Table 191's /H`), one of [`ATTRIBUTIONS`]' verbs (`Table 124 defines /FontFile2`),
//! or an apposition (`Table 385, with a /Subtype`), and then only within [`WINDOW`] words and
//! before the sentence turns to another table.
//!
//! A sentence can also attribute a key in order to say the table does **not** state it — "Table
//! 119 gives a Type 0 dictionary no `/FontDescriptor`" — and that is this project reading a table
//! correctly. A denial is not dropped, it is *judged the other way round*: the standard agreeing
//! is nothing, and the standard contradicting it is the same defect from the far end.
//!
//! # The answers that are not suspects, and why each is printed as a count
//!
//! - **Agrees.** The table states the key and the sentence claims it, or the table does not and
//!   the sentence denies it. Nothing to read.
//! - **Keyless.** The table exists and states no entries at all: a *flags* table's first column is
//!   a bit position (Table 22), an abbreviation table's is a full name (Table 92), and the
//!   conversion displaces a column often enough (Table 200) that an absence here is a question for
//!   `doc/*.pdf` rather than a finding. A sweep that read these as defects would print the same
//!   dozen every round until somebody switched it off.
//! - **Unknown.** No table of that number is captioned anywhere in the conversion. That is the
//!   gate's own subject for a Rust citation, and this sweep is the only thing that asks it of a
//!   ledger note or a document.
//!
//! # What the program adds over the grep
//!
//! Two judgements every hand-run redid. **Which table does state the key** — the answer a person
//! had to look up for each suspect, and the sentence a correction is written from ("`/AP` is
//! Table 166's"); a suspect whose key exactly one other table states is the shape every defect
//! this sweep has found had. And **whether the sentence is a correction or a standing claim**
//! ([`retired::kind_of`]), because this project writes the retired number into the sentence that
//! retires it, so its own records read as hits for ever.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument. Three noise shapes survive every rule above, and all three are
//! correct English about the right numbers:
//!
//! - **A table's *value* named beside its entry.** "Table 169's cloudy `/BE`" — `/BE` is the
//!   annotation's entry and Table 169 is what its value is. The sweep prints which table states
//!   the key, which is what a reader needs to tell this from a defect in one line.
//! - **A rule a table states *about* a key it does not state.** "Table 177 makes the file's own
//!   `/AP` decisive over its `/DA`" is §12.5.6.6's own sentence, in the `/DA` row, about an entry
//!   Table 166 states — and the five-hundred-and-twenty-fifth session read one of these as a
//!   defect and corrected it *into* this form.
//! - **A round's own record.** `doc/todo/01` and the ADRs quote every number they retired.
//! - **A denial whose negation is about something else.** "Table 31 makes a page stating no
//!   `/Contents` an empty page" denies the *page* an entry rather than the table, and "Table 185
//!   states no such ordering for `/InkList`" denies an ordering. Which noun a negation attaches to
//!   is not a question this program can ask, and the three it produced on the first run are all of
//!   this shape.
//!
//! **It is a reading list**, and what decides a hit is a question no program can ask: whether the
//! sentence means *this table states this entry* or *this table is what this entry is about*.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::entries;
use crate::ledger::Ledger;
use crate::retired::{self, Kind};

/// The words that attribute the key after them to the table before them.
///
/// Lower-cased, matched whole against the word following `Table N`. The possessive is handled
/// separately because it is written against the number itself, and an apposition (`Table 385,
/// with a /Subtype`) is a comma. Deliberately short: a verb this list does not hold — `is`, in
/// "Table 227 is the flags inside `/Ff`" — is a sentence saying the key is somewhere *else*, and
/// admitting it would put the sweep's commonest false positive back in the reading list.
pub const ATTRIBUTIONS: [&str; 12] = [
    "defines",
    "states",
    "gives",
    "lists",
    "names",
    "makes",
    "puts",
    "requires",
    "carries",
    "sets",
    "specifies",
    "documents",
];

/// How many words may stand between a possessive and the key it attributes.
///
/// Wide enough for the adjectives this project writes — `Table 187's required /FS` — and no
/// wider, **measured rather than chosen**: at six, "Table 349, whole, from the trailer's `/Info`"
/// and "Table 164's transition dictionary and the page's `/Dur`" are attributions, and both name
/// a key belonging to the dictionary the sentence has moved on to. A possessive is a noun phrase
/// and its key comes almost immediately; the words *after* the first key are not counted here at
/// all, because a list of entries under one table is one attribution however long it runs.
pub const WINDOW: usize = 3;

/// How many words a verb reaches over, which is further.
///
/// `Table N's X` attributes X directly; `Table N gives … no /FontDescriptor` is a whole predicate,
/// and the qualification between the verb and the key is the sentence doing its job rather than
/// changing subject. Every denial this project writes has that shape.
pub const VERB_WINDOW: usize = 6;

/// The standard's numbered tables, reduced to what this sweep asks of them.
#[derive(Debug, Clone, Default)]
pub struct Tables {
    /// Every captioned table number, with its title.
    captions: BTreeMap<u16, String>,
    /// The keys of every table whose first column is `Key`.
    keys: BTreeMap<u16, Vec<String>>,
    /// Which tables state a key, for the answer a suspect needs.
    stating: BTreeMap<String, Vec<u16>>,
}

impl Tables {
    /// Reads every table out of the standard's conversion.
    ///
    /// The captions are taken here rather than from [`crate::clause::ClauseIndex::table_title`]
    /// because this sweep needs the numbers of the tables that state **no** keys as much as it
    /// needs the others: a keyless table is what tells a citation of Table 22 apart from a
    /// citation of a table the standard does not have.
    #[must_use]
    pub fn read(text: &str) -> Self {
        let mut tables = Self::default();
        for line in text.lines() {
            let bare = line.trim_start_matches('#').trim();
            let Some(caption) = bare.strip_prefix("Table ") else {
                continue;
            };
            let Some((number, title)) = caption.split_once('-') else {
                continue;
            };
            let (Ok(number), title) = (number.trim().parse::<u16>(), title.trim()) else {
                continue;
            };
            if !title.is_empty() {
                tables
                    .captions
                    .entry(number)
                    .or_insert_with(|| title.to_owned());
            }
        }
        for table in entries::tables_in(text) {
            for key in &table.keys {
                let stating = tables.stating.entry(key.clone()).or_default();
                if !stating.contains(&table.number) {
                    stating.push(table.number);
                }
            }
            tables.keys.entry(table.number).or_insert(table.keys);
        }
        tables
    }

    /// How many tables the conversion captions.
    #[must_use]
    pub fn captioned(&self) -> usize {
        self.captions.len()
    }

    /// How many of them state entries in a first column named `Key`.
    #[must_use]
    pub fn keyed(&self) -> usize {
        self.keys.len()
    }

    /// One table's title, or `None` where the conversion captions no such table.
    #[must_use]
    pub fn title(&self, number: u16) -> Option<&str> {
        self.captions.get(&number).map(String::as_str)
    }

    /// The tables that do state one key, in number order.
    #[must_use]
    pub fn stating(&self, key: &str) -> &[u16] {
        self.stating.get(key).map_or(&[], Vec::as_slice)
    }

    /// What the standard says about one claim, which is the claim's own direction and the table's
    /// answer together.
    #[must_use]
    pub fn judge(&self, claim: &Claim) -> Verdict {
        let stated = match self.keys.get(&claim.table) {
            Some(keys) => keys.iter().any(|stated| stated == &claim.key),
            None if self.captions.contains_key(&claim.table) => return Verdict::Keyless,
            None => return Verdict::Unknown,
        };
        match (stated, claim.denied) {
            (true, false) | (false, true) => Verdict::Agrees,
            (false, false) => Verdict::Absent,
            (true, true) => Verdict::Denied,
        }
    }
}

/// What the standard says about a key a sentence attributes to a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The sentence and the table say the same thing — the key is stated and the sentence claims
    /// it, or the key is absent and the sentence says so. Nothing is owed.
    Agrees,
    /// The sentence gives the table a key it does not state — the reading list.
    Absent,
    /// The sentence says the table states no such key and the table states it — the same defect
    /// from the other end, and the reason a denial is checked rather than dropped.
    Denied,
    /// The table exists and states no entries at all: a flags table, an abbreviation table, or a
    /// table whose columns the conversion displaced.
    Keyless,
    /// The conversion captions no table of that number.
    Unknown,
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Agrees => "the table agrees",
            Self::Absent => "absent from the table",
            Self::Denied => "denied, and the table states it",
            Self::Keyless => "the table states no entries",
            Self::Unknown => "no such table",
        })
    }
}

/// One key attributed to one table by one sentence.
#[derive(Debug, Clone)]
pub struct Attribution {
    /// The table the sentence names.
    pub table: u16,
    /// The key it attributes to it, without the solidus.
    pub key: String,
    /// What the standard says about the pair.
    pub verdict: Verdict,
    /// The tables that do state the key, for the correction a defect needs.
    pub elsewhere: Vec<u16>,
    /// Where the sentence is: `doc/conformance/ledger.toml:123 (§12.5.6.19, partial)` or
    /// `crates/pdf-model/src/view.rs:88`.
    pub location: String,
    /// The sentence, whole, because the verdict is a reading list and not a finding.
    pub sentence: String,
    /// Whether the sentence narrates a correction — the shape that quotes the number it retired.
    pub kind: Kind,
}

impl Attribution {
    /// Whether exactly one other table states the key.
    ///
    /// The signature of every defect this sweep has found: a number one out from the right one,
    /// with the right one unambiguous. A key stated by four tables is usually prose about a
    /// dictionary several tables extend.
    #[must_use]
    pub fn unambiguous(&self) -> bool {
        self.elsewhere.len() == 1
    }
}

/// What one run read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Every attribution the run read, in the order it read them.
    pub attributions: Vec<Attribution>,
    /// How many sentences named a table at all, attributing a key or not.
    pub sentences: usize,
}

impl Report {
    /// The attributions carrying one verdict.
    #[must_use]
    pub fn reaching(&self, verdict: Verdict) -> Vec<&Attribution> {
        self.attributions
            .iter()
            .filter(|attribution| attribution.verdict == verdict)
            .collect()
    }

    /// The suspects, sharpest first: a standing claim above a correction, and an unambiguous key
    /// above one several tables state. A denial the table contradicts is a suspect too, and it
    /// sorts with the rest.
    #[must_use]
    pub fn suspects(&self) -> Vec<&Attribution> {
        let mut suspects = self.reaching(Verdict::Absent);
        suspects.extend(self.reaching(Verdict::Denied));
        suspects.sort_by_key(|attribution| {
            (
                attribution.kind == Kind::Correction,
                !attribution.unambiguous(),
                attribution.elsewhere.is_empty(),
                attribution.table,
            )
        });
        suspects
    }
}

/// Runs the sweep over the ledger's notes, the tree's comments and this project's prose.
///
/// `sources` are the Rust files under [`crate::SOURCE_ROOTS`] and `documents` the Markdown under
/// `doc/`, each with its text. Two directories are read by nothing here, for the reasons
/// [`retired::NOT_SWEPT`] and [`crate::NOT_SCANNED`] give: a round's own record is not another
/// round's to correct, and this checker's own prose quotes the wrong numbers as examples.
#[must_use]
pub fn sweep(
    tables: &Tables,
    ledger: &Ledger,
    sources: &[(PathBuf, String)],
    documents: &[(PathBuf, String)],
) -> Report {
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
        if shown.starts_with(retired::NOT_SWEPT) {
            continue;
        }
        for (line, block) in retired::paragraphs(text) {
            places.push((format!("{shown}:{line}"), block));
        }
    }

    let mut report = Report::default();
    for (location, block) in &places {
        for sentence in crate::unread::sentences(block) {
            if !sentence.contains("Table ") {
                continue;
            }
            report.sentences = report.sentences.saturating_add(1);
            let kind = retired::kind_of(sentence);
            for claim in attributions_in(sentence) {
                let verdict = tables.judge(&claim);
                let elsewhere = tables
                    .stating(&claim.key)
                    .iter()
                    .copied()
                    .filter(|stating| *stating != claim.table)
                    .collect();
                report.attributions.push(Attribution {
                    table: claim.table,
                    key: claim.key,
                    verdict,
                    elsewhere,
                    location: location.clone(),
                    sentence: sentence.to_owned(),
                    kind,
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

/// Every key one sentence attributes to a numbered table.
///
/// The attribution is what makes a key a claim about a table rather than a key that happens to
/// share a sentence with one; the module documentation says which forms count and why the list is
/// short.
#[must_use]
pub fn attributions_in(sentence: &str) -> Vec<Claim> {
    let mut found = Vec::new();
    let mut rest = sentence;
    while let Some(at) = rest.find("Table ") {
        let after = rest.get(at.saturating_add("Table ".len())..).unwrap_or("");
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        rest = after;
        let Ok(table) = digits.parse::<u16>() else {
            continue;
        };
        let tail = after.get(digits.len()..).unwrap_or("");
        let Some((tail, reach)) = attributive(tail) else {
            continue;
        };
        let denied = denies(tail, reach);
        for key in keys_within(tail, reach) {
            found.push(Claim { table, key, denied });
        }
    }
    found
}

/// What one sentence says about one key and one table, before the standard is asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// The table the sentence names.
    pub table: u16,
    /// The key it attributes to it.
    pub key: String,
    /// Whether it attributes the key in order to say the table does **not** state it.
    ///
    /// "Table 119 gives a Type 0 dictionary no `/FontDescriptor`" is this project reading a table
    /// correctly and saying so, and a sweep that read it as a wrong number would print the tree's
    /// most careful sentences at it every round. It is not simply dropped, because the negation
    /// makes it a *checkable* claim in the other direction: a sentence denying an entry the table
    /// does state is as wrong as a sentence inventing one.
    pub denied: bool,
}

/// The words that turn an attribution into a denial.
///
/// Matched over the words before the key, within the same reach.
const DENIALS: [&str; 6] = ["no", "not", "never", "neither", "none", "nothing"];

/// Whether an attribution's reach denies the key rather than claiming it.
fn denies(tail: &str, reach: usize) -> bool {
    tail.split_whitespace()
        .take(reach)
        .take_while(|word| key_of(word).is_none())
        .any(|word| {
            let word = word.trim_matches(|c: char| !c.is_alphanumeric());
            DENIALS.contains(&word.to_ascii_lowercase().as_str())
        })
}

/// The text after an attribution and how far it reaches, or `None` where what follows the number
/// attributes nothing.
fn attributive(tail: &str) -> Option<(&str, usize)> {
    for possessive in ["'s", "\u{2019}s"] {
        if let Some(rest) = tail.strip_prefix(possessive) {
            return Some((rest, WINDOW));
        }
    }
    if let Some(rest) = tail.strip_prefix(',') {
        return Some((rest, WINDOW));
    }
    let mut words = tail.split_whitespace();
    let word = words.next()?.trim_matches(|c: char| !c.is_alphabetic());
    if ATTRIBUTIONS.contains(&word.to_ascii_lowercase().as_str()) {
        let at = tail.find(word)?.saturating_add(word.len());
        return tail.get(at..).map(|rest| (rest, VERB_WINDOW));
    }
    None
}

/// The keys within [`WINDOW`] words of an attribution, up to what ends its reach.
///
/// One attribution reaches one key, or a *list* of them — the shape a row listing five entries
/// under one table has. So the words before the first key are the adjectives the prose puts
/// there, and the words after it are only ever a list's own: anything else is the sentence
/// having moved on, which is what "Table 211's `/Base`, from the catalog's `/URI` dictionary"
/// does one comma later.
fn keys_within(tail: &str, reach: usize) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    let mut linked = false;
    let mut before = 0_usize;
    for word in tail.split_whitespace() {
        if keys.is_empty() {
            if before >= reach {
                break;
            }
            before = before.saturating_add(1);
        }
        // Another table, a parenthesis or a clause break is the sentence turning to something
        // else, and this project writes the correction to a wrong number in exactly that
        // position.
        if word.starts_with("Table") || ends_the_reach(word) {
            break;
        }
        if let Some(key) = key_of(word) {
            if !keys.is_empty() && !linked {
                // Two keys with nothing between them are a key and its *value* — `/Subtype
                // /Image` — and the second is not an entry of anything.
                break;
            }
            keys.push(key);
            linked = word.trim_end_matches(['`', '"', '\'']).ends_with(',');
            continue;
        }
        if !keys.is_empty() {
            if !continues_a_list(word) {
                break;
            }
            linked = true;
        }
    }
    keys
}

/// Whether a word closes the sentence's reach for an attribution.
///
/// A bracket is the other half of this: `[/None /None]` is an array *value* and its names are not
/// entries of anything, which is what a line ending's default looks like written down.
fn ends_the_reach(word: &str) -> bool {
    word.starts_with(['(', ')', ';', ':', '['])
        || word.ends_with([';', ':', ')'])
        || word.contains('—')
        || word.contains('(')
}

/// Whether a word between two keys is a list's own rather than the sentence moving on.
fn continues_a_list(word: &str) -> bool {
    let word = word.trim_matches(|c: char| !c.is_alphanumeric());
    word.is_empty() || matches!(word.to_ascii_lowercase().as_str(), "and" | "or" | "nor")
}

/// One word reduced to the key it names, or `None` where it names none.
///
/// A key is written with the solidus the standard prints, inside whatever backticks, quotation
/// marks and punctuation the prose puts around it.
fn key_of(word: &str) -> Option<String> {
    let word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '/');
    let key = word.strip_prefix('/')?;
    let key = key.trim_end_matches(|c: char| !c.is_alphanumeric());
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(key.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two tables of the conversion's own shape: one that states entries and one whose first
    /// column is a bit position, which is what a flags table looks like.
    const CONVERSION: &str = "\
Table 166 -Entries common to all annotation dictionaries

| Key      | Type   | Value                    |
|----------|--------|--------------------------|
| Subtype  | name   | (Required) The type.     |
| Contents | string | (Optional) The text.     |
| AP       | dict   | (Optional) An appearance.|

Table 191 -Additional entries specific to a widget annotation

| Key | Type | Value               |
|-----|------|---------------------|
| H   | name | (Optional) A mode.  |

Table 22 -User access permissions

| Bit position | Meaning        |
|--------------|----------------|
| 3            | (Security ...) |
";

    /// One claim, written the way this project writes one.
    fn claim(table: u16, key: &str) -> Claim {
        Claim {
            table,
            key: key.to_owned(),
            denied: false,
        }
    }

    #[test]
    fn a_possessive_attributes_the_key_after_it() {
        assert_eq!(
            attributions_in("Table 191's `/H` is the highlighting mode."),
            vec![claim(191, "H")]
        );
    }

    /// A row listing several entries under one table is several claims.
    #[test]
    fn a_list_under_one_table_is_one_claim_apiece() {
        assert_eq!(
            attributions_in("Table 172's `/Subj`, `/RC` and `/IRT` reach a pane"),
            vec![claim(172, "Subj"), claim(172, "RC"), claim(172, "IRT")]
        );
    }

    /// Two keys with nothing between them are a key and its value, and a value is an entry of
    /// nothing — the shape `/Subtype /Image` has wherever this tree names a stream's role.
    #[test]
    fn a_key_next_to_a_key_is_its_value() {
        assert_eq!(
            attributions_in("An image `XObject`, Table 87's `/Subtype /Image`."),
            vec![claim(87, "Subtype")]
        );
    }

    /// The sentence moving on ends the reach even inside the window: the key after the comma
    /// belongs to the dictionary the sentence has turned to.
    #[test]
    fn a_sentence_that_moves_on_attributes_nothing_further() {
        assert_eq!(
            attributions_in("Table 211's `/Base`, from the catalog's `/URI` dictionary."),
            vec![claim(211, "Base")]
        );
    }

    /// A denial is a claim in the other direction, and this project writes several a round.
    #[test]
    fn a_denial_is_read_as_one() {
        let found = attributions_in("Table 119 gives a Type 0 dictionary no `/FontDescriptor`");
        assert_eq!(
            found,
            vec![Claim {
                table: 119,
                key: "FontDescriptor".to_owned(),
                denied: true,
            }]
        );
    }

    /// The commonest false positive there is, and the reason [`ATTRIBUTIONS`] is short: the
    /// sentence says the table is *inside* the entry rather than that it states it.
    #[test]
    fn a_table_named_without_attributing_claims_nothing() {
        assert_eq!(
            attributions_in("Table 227 is the flags inside `/Ff`"),
            Vec::new()
        );
        assert_eq!(
            attributions_in("Table 405 is the value of the `/OPI` Tables 87 and 93 state"),
            Vec::new()
        );
    }

    /// A verb attributes as a possessive does — the form `pdf-font`'s comment used for the
    /// four-hundred-and-eighty-ninth's `/FontFile2` defect.
    #[test]
    fn a_verb_attributes_too() {
        assert_eq!(
            attributions_in("Table 124 defines `/FontFile2` for a CIDFont"),
            vec![claim(124, "FontFile2")]
        );
    }

    /// The sentence turning to another table ends the first one's reach and starts the second's,
    /// which is what a correction looks like: the retired number, then the right one, each judged
    /// on its own.
    #[test]
    fn another_table_ends_the_reach_and_begins_its_own() {
        assert_eq!(
            attributions_in("Table 193's `/FixedPrint`, which is Table 194's `/H`"),
            vec![claim(193, "FixedPrint"), claim(194, "H")]
        );
    }

    #[test]
    fn a_keys_table_is_read_and_a_flags_table_is_keyless() {
        let tables = Tables::read(CONVERSION);
        assert_eq!(tables.captioned(), 3);
        assert_eq!(tables.keyed(), 2);
        assert_eq!(tables.judge(&claim(166, "AP")), Verdict::Agrees);
        assert_eq!(tables.judge(&claim(191, "AP")), Verdict::Absent);
        assert_eq!(tables.judge(&claim(22, "P")), Verdict::Keyless);
        assert_eq!(tables.judge(&claim(999, "P")), Verdict::Unknown);
        assert_eq!(
            tables.title(191),
            Some("Additional entries specific to a widget annotation")
        );
    }

    /// A denial is judged the other way round, which is what makes it worth reading rather than
    /// dropping: the table agreeing is nothing and the table contradicting it is a defect.
    #[test]
    fn a_denial_is_judged_the_other_way_round() {
        let tables = Tables::read(CONVERSION);
        let denied = |table, key: &str| Claim {
            table,
            key: key.to_owned(),
            denied: true,
        };
        assert_eq!(tables.judge(&denied(191, "AP")), Verdict::Agrees);
        assert_eq!(tables.judge(&denied(166, "AP")), Verdict::Denied);
    }

    /// The answer a suspect needs, and the sentence a correction is written from: the
    /// five-hundred-and-twenty-fifth's finding was `/AP` under Table 177, and `/AP` is Table
    /// 166's.
    #[test]
    fn a_suspect_carries_the_table_that_does_state_the_key() {
        let tables = Tables::read(CONVERSION);
        assert_eq!(tables.stating("AP"), [166]);
        assert_eq!(tables.stating("Nothing"), []);
    }
}
