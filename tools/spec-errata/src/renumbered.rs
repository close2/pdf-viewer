//! The sixth blindness closed: an erratum that renumbers a **table** by striking its caption.
//!
//! # The shape it exists for
//!
//! This crate's three existing questions all miss one kind of erratum, and the
//! eight-hundred-and-fifteenth session found it by reading:
//!
//! - [`crate::landings`] — `check` — compares *quotations this tree has written* against struck
//!   passages. Nothing quotes a table's caption, and the struck text here is two words, under
//!   [`crate::MIN_WORDS`]. Blind twice over.
//! - [`crate::structural`] — `moved` — wants one of [`crate::STRUCTURAL`]'s four verbs in the
//!   annotation's **own** `/Contents` and a **clause** number named there. A bare strike-and-caret
//!   pair carries no verb, and a table is not a clause. Blind.
//! - [`crate::applied`] asks whether a place recording an erratum has applied it, which
//!   presupposes somebody has recorded it.
//!
//! Issue #700 is the witness. It renumbers Annex O's two tables — `Table Annex O.3`'s
//! designation becomes *Annex O.1* and `Table Annex O.4`'s becomes *Annex O.2* — with a
//! `StrikeOut` over each caption's designation and a `Caret` carrying the replacement, and
//! nothing in this tree could print it while dozens of lines stood on the retired numbers.
//!
//! **The amended designations are written bare, and that is the convention rather than a
//! typographical preference.** What the erratum states is a strike over a *designation*, so the
//! designation is what a sentence about it names; writing either of them as a table would cite
//! a caption no reader can find, which is the paragraph below and, since the
//! eight-hundred-and-thirty-second session, a gate.
//!
//! **A renumbering is a class of erratum rather than a one-off**, which is the whole argument for
//! a command: the next caption strike would have been invisible in exactly the same way, and the
//! cost of finding it would have been another session's reading.
//!
//! # The predicate, and the two things that ground it
//!
//! A `StrikeOut` whose covered text is a table designation, paired through §12.5.6.2's `/IRT`
//! group with a `Caret` whose contents are another. **No verb**, because a bare pair carries none;
//! **no clause number**, because a table is not one.
//!
//! Neither half of that is enough on its own, and the first run said so out loud. What makes a
//! struck string a *table* designation is that **the conversion of that same document captions a
//! table with it** — a rule taking the shape alone would report every strike over a clause
//! number, a version number or a figure's, since all three are a letter, some digits and a full
//! stop. That grounding admits nine annotations of this collection and every one of them is an
//! **integer struck in body text**: Issue #124 correcting four array indices to be zero-based,
//! Issue #133 renumbering two NOTEs, Issue #527 a byte count in an LZW example. A bare `3` is a
//! table designation and an array index and a NOTE's number, and the erratum says which only by
//! where it is.
//!
//! So there is a second grounding and it is what ranks the report: **does the clause this
//! annotation is filed under caption that table?** ([`conformance::clause::ClauseIndex::captions_table`]).
//! A strike over a caption is inside the clause the caption belongs to; a strike over an array
//! index five hundred pages away is not. On the collection this separates the two annotations
//! of Issue #700 from all nine of the others.
//!
//! **It ranks rather than filters**, and that is deliberate. `emit` files an annotation under the
//! section §12.3.3's outline puts its *page* in, and ADR 0712's placement rule is that the
//! outline is one clause out often enough to have been written down six times — so a filter would
//! turn a placement artefact into a renumbering nobody ever sees, which is the blindness this
//! module exists to end. [`Rung::Elsewhere`] is counted and named, one line apiece.
//!
//! # What the ground is, and why it is not [`crate::standing_on`]'s
//!
//! `moved` counts what stands on a *clause* number, which
//! [`conformance::citation::Citation`]'s SECTION SIGN scanner already finds. A table is cited by
//! name — `Table Annex O.3's` — so the ground here is
//! [`conformance::citation::Scan::designations`], the population that module grew for this
//! command. The ledger is read as prose rather than as rows, because a table designation belongs
//! to a note's sentence rather than to a row's `clause` key.
//!
//! # What this does *not* do, and it is the standing answer rather than an omission
//!
//! **It renames nothing.** `doc/md/` is the published text every citation in this tree resolves
//! against, and the conformance gate refuses a designation ISO 32000-2 does not caption; a tree
//! citing the amended *Annex O.1* as a table would be citing a caption no reader can find. So
//! the published designations stay, the amendment is recorded in the ledger row, and this
//! command is what makes the ground findable from outside that row — which is exactly the
//! three-part answer `doc/errata-read.md` records for a clause number an erratum moves, with its
//! third part working for a table at last.
//!
//! **That sentence was a claim about an instrument and it was false when it was written**: the
//! gate refused a *number*, and a designation no `u16` can hold — which both of these are — was
//! not checked at all. It is true since the eight-hundred-and-thirty-second session, and this
//! module's own first paragraph was one of the five places in the tree that broke it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use conformance::clause::{ClauseIndex, ClauseIndexError, caption_of};

use crate::{Error, Note, Role};

/// How close a renumbering is to being one, which is the order the report prints them in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// The clause the annotation is filed under captions that very table.
    InSection,
    /// The document captions it, but somewhere else — so the struck digits are as likely to be
    /// an array index, a NOTE's number or an example's byte count.
    Elsewhere,
}

/// A table designation an erratum retires, and the one it writes in its place.
#[derive(Debug, Clone)]
pub struct Renumbering {
    /// The `StrikeOut` over the retired designation.
    pub note: Note,
    /// The designation the strikeout covers, as the caption writes it: `Annex O.3`.
    pub retired: String,
    /// The designation the paired caret writes instead: `Annex O.1`.
    pub replacement: String,
    /// The title the conversion gives the table the retired designation captions.
    ///
    /// This is what makes a hit readable in one line rather than in a page turn, and it is the
    /// same argument [`conformance::clause::ClauseIndex::table_title`] makes for printing a
    /// title beside a number.
    pub title: String,
    /// How close it is to being a renumbering.
    pub rung: Rung,
}

/// What this tree has standing on one retired table designation.
#[derive(Debug, Clone)]
pub struct Ground {
    /// The renumbering the places stand on the wrong side of.
    pub renumbering: Renumbering,
    /// Every citation of the retired designation in a comment under the source roots.
    pub citations: Vec<(PathBuf, usize)>,
    /// The same, in this project's own Markdown and in the ledger's notes.
    pub documents: Vec<(PathBuf, usize)>,
}

impl Ground {
    /// How many places stand on the retired designation altogether.
    #[must_use]
    pub fn places(&self) -> usize {
        self.citations.len().saturating_add(self.documents.len())
    }
}

/// Every erratum that renumbers a table by striking its caption, closest rung first.
///
/// `conversions` is the directory holding the Markdown conversion of each document — `doc/md/` —
/// which is where the caption a strikeout has to land on is read from. A document with no
/// conversion beside it, or one with no numbered headings at all, contributes nothing rather than
/// failing: the collection holds fourteen PDFs, five of which are association notes rather than
/// standards, and the caller may pass any one of them alone.
///
/// # Errors
///
/// [`Error::Unreadable`] where `conversions` cannot be walked, [`Error::Conversion`] where one of
/// its files cannot be read at all.
pub fn renumbered(notes: &[Note], conversions: &Path) -> Result<Vec<Renumbering>, Error> {
    let indexes = indexes(conversions)?;

    // The caret of each §12.5.6.2 group, so that a strikeout can find the replacement it was
    // written with. Keyed on `Note::change`, which is the group's primary annotation: the pair
    // is one erratum stated in two marks, and pairing on the *page* instead would join two
    // renumberings that happen to share one.
    let mut carets: BTreeMap<pdf_syntax::ObjectId, &Note> = BTreeMap::new();
    for note in notes {
        if note.subtype == "Caret"
            && note.role != Role::Reply
            && let Some(change) = note.change
        {
            carets.insert(change, note);
        }
    }

    let mut found = Vec::new();
    for note in notes {
        if note.subtype != "StrikeOut" || note.role == Role::Reply {
            continue;
        }
        let Some(retired) = note.covered.as_deref().map(str::trim) else {
            continue;
        };
        let Some(index) = indexes.get(note.document.as_str()) else {
            continue;
        };
        let Some(title) = index.designated_table_title(retired) else {
            continue;
        };
        let Some(replacement) = note
            .change
            .and_then(|change| carets.get(&change))
            .and_then(|caret| caret.contents.as_deref())
            .map(str::trim)
            .filter(|replacement| *replacement != retired)
            .and_then(designation_of)
        else {
            continue;
        };
        let rung = if section_of(note).is_some_and(|number| index.captions_table(&number, retired))
        {
            Rung::InSection
        } else {
            Rung::Elsewhere
        };
        found.push(Renumbering {
            note: note.clone(),
            retired: retired.to_owned(),
            replacement,
            title: title.to_owned(),
            rung,
        });
    }
    found.sort_by(|left, right| {
        left.rung
            .cmp(&right.rung)
            .then_with(|| left.note.page.cmp(&right.note.page))
    });
    Ok(found)
}

/// What the source roots, this project's documents and the ledger have standing on each.
///
/// # Errors
///
/// [`Error::Sources`] or [`Error::Documents`] where a tree cannot be walked,
/// [`Error::Unreadable`] for a file inside one.
pub fn standing_on(
    renumberings: &[Renumbering],
    ledger: &Path,
    roots: &[PathBuf],
    documents: &Path,
) -> Result<Vec<Ground>, Error> {
    let mut cited: Vec<(PathBuf, usize, String)> = Vec::new();
    let sources = conformance::citation::rust_sources(roots)
        .map_err(|error| Error::Sources(roots.first().cloned().unwrap_or_default(), error))?;
    for file in sources {
        let text = std::fs::read_to_string(&file)
            .map_err(|error| Error::Unreadable(file.clone(), error))?;
        for reference in conformance::citation::scan(&text).designations {
            cited.push((file.clone(), reference.line, reference.designation));
        }
    }

    // The ledger goes in beside the documents rather than in a column of its own. `moved` keeps
    // rows apart because a clause number *is* a row's key; a table designation is only ever a
    // sentence in a note, which is prose like any other.
    let mut written: Vec<(PathBuf, usize, String)> = Vec::new();
    let mut prose = conformance::prose::documents(documents)?;
    prose.push(ledger.to_owned());
    for file in prose {
        let text = std::fs::read_to_string(&file)
            .map_err(|error| Error::Unreadable(file.clone(), error))?;
        for reference in conformance::citation::scan_prose(&text).designations {
            written.push((file.clone(), reference.line, reference.designation));
        }
    }

    Ok(renumberings
        .iter()
        .map(|renumbering| Ground {
            renumbering: renumbering.clone(),
            citations: cited
                .iter()
                .filter(|(_, _, designation)| *designation == renumbering.retired)
                .map(|(file, line, _)| (file.clone(), *line))
                .collect(),
            documents: written
                .iter()
                .filter(|(_, _, designation)| *designation == renumbering.retired)
                .map(|(file, line, _)| (file.clone(), *line))
                .collect(),
        })
        .collect())
}

/// The clause index of every conversion under `conversions`, keyed by the PDF beside it.
///
/// The key is the **PDF's** file name, which is what [`Note::document`] carries, so that a
/// strikeout is only ever compared against the captions of the document it is in. Two standards
/// in this collection both caption a `Table 1`, and a strike over one is nothing to do with the
/// other.
fn indexes(conversions: &Path) -> Result<BTreeMap<String, ClauseIndex>, Error> {
    let mut found = BTreeMap::new();
    let entries = std::fs::read_dir(conversions)
        .map_err(|error| Error::Unreadable(conversions.to_owned(), error))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "md") {
            continue;
        }
        let Some(stem) = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
        else {
            continue;
        };
        match ClauseIndex::read(&path) {
            Ok(index) => {
                found.insert(format!("{stem}.pdf"), index);
            }
            // A conversion with no numbered heading is one of the association's notes rather than
            // a standard, and it captions no numbered table either. Skipped rather than refused,
            // because the caller passes `doc/*.pdf` and five of those are such notes.
            Err(ClauseIndexError::NotTheStandard { .. }) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(found)
}

/// The clause number the outline files this note under, if it files it under one.
///
/// A section is `O.2.1 PDF object identifiers` — the number, then the title — so the number is
/// its first word. A heading whose first word is not a clause number is the annex's own
/// (`Annex O`), which no strikeout is filed under on its own.
fn section_of(note: &Note) -> Option<conformance::clause::ClauseNumber> {
    note.section
        .as_deref()?
        .split_whitespace()
        .next()?
        .trim_end_matches('.')
        .parse()
        .ok()
}

/// `text` where the whole of it is one table designation, and `None` otherwise.
///
/// The replacement side of a renumbering, which cannot be grounded in a caption because the
/// caption it will have is exactly what the erratum is asking for. So the test is the shape —
/// [`conformance::clause::caption_of`]'s own, asked of a designation standing alone.
fn designation_of(text: &str) -> Option<String> {
    // `caption_of` reads a caption, so it is handed one: the designation with a title after it
    // that cannot be empty and cannot be mistaken for part of the number.
    let caption = format!("Table {text} -x");
    let (designation, _) = caption_of(&caption)?;
    (designation == text).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_replacement_is_a_designation_standing_alone() {
        assert_eq!(designation_of("Annex O.1").as_deref(), Some("Annex O.1"));
        assert_eq!(designation_of("125a").as_deref(), Some("125a"));
        assert_eq!(designation_of("D.2").as_deref(), Some("D.2"));
        assert_eq!(
            designation_of("Annex O.1 and Annex O.2"),
            None,
            "a replacement naming two tables is not one designation, and pairing it with a \
             single strikeout would print half an erratum as though it were the whole"
        );
        assert_eq!(
            designation_of("Optional"),
            None,
            "a requirement word carries no digit, and a strike over one is the erratum shape \
             `doc/todo/01` says is still read by eye"
        );
    }

    /// The ranking's own subject, on the two shapes the first run over Errata Collection 3
    /// produced: Issue #700's strike over `Annex O.3` inside §O.2.1, which captions that table,
    /// and Issue #124's strike over an array index of `3` in a clause that captions nothing.
    ///
    /// The conversion is written here rather than read, because what is being tested is the
    /// discriminator and not the standard.
    #[test]
    fn a_strike_inside_the_clause_that_captions_the_table_is_the_closer_rung() {
        let index = ClauseIndex::parse(
            "## O.2.1 PDF object identifiers\n\
             \n\
             Table Annex O.3 -PDF object identifiers\n\
             \n\
             ## 7.3.5 Name objects\n\
             \n\
             Table 3 -Escape sequences in literal strings\n\
             \n\
             ## 12.5.6.19 Redaction annotations\n\
             \n\
             The array's third element, at index 3, is the one meant.\n"
                .to_owned(),
        );
        assert!(
            index.captions_table(&"O.2.1".parse().expect("a clause number"), "Annex O.3"),
            "the caption is inside the clause the strike is filed under"
        );
        assert!(
            !index.captions_table(&"12.5.6.19".parse().expect("a clause number"), "3"),
            "an array index in a clause that captions no table is the far rung, and Table 3's \
             own caption five hundred pages away does not make it the near one"
        );
        assert_eq!(
            index.designated_table_title("3"),
            Some("Escape sequences in literal strings"),
            "the document does caption a Table 3, which is exactly why the caption alone \
             cannot decide this"
        );
    }
}
