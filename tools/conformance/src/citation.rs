//! Finding the citations, quotations and table references a Rust source file makes of the
//! standard.
//!
//! `CLAUDE.md` principle 5 asks that every item implementing a normative requirement cite
//! its clause, and that a load-bearing sentence appear as a rustdoc blockquote under that
//! clause number. This module is the reader for both halves of that convention.
//!
//! # Why a citation is scanned everywhere and a quotation only in doc comments
//!
//! A wrong clause number is wrong wherever it is written — in a doc comment, in a `//` note
//! beside a branch, in a test's name for a group of pages — so [`scan`] looks for `§` on
//! every line. A *quotation*, on the other hand, is checked for being the standard's own
//! words, and this tree quotes plenty of other things: an error message, a warning another
//! renderer printed, a phrase from the ICC specification, a rhetorical aside. Marking the
//! checkable ones is what the blockquote form is for, so only a blockquote inside a doc
//! comment is treated as a claim about ISO 32000-2.
//!
//! # How a quotation finds its clause
//!
//! By the nearest `§` citation before it in the same doc comment. That is how the
//! convention already reads on the page —
//!
//! ```text
//! /// ISO 32000-2 §7.7.3.3 defines the crop box:
//! ///
//! /// > the region to which the contents of the page shall be clipped
//! ```
//!
//! — and a blockquote with no citation before it is a finding rather than a default: an
//! unattributed quotation is exactly the thing that cannot be checked.

use std::path::{Path, PathBuf};

use crate::clause::ClauseNumber;

/// A `§` reference to a clause, as it appears in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    /// The clause number cited.
    pub number: ClauseNumber,
    /// The 1-based line it appears on.
    pub line: usize,
}

/// A `§` that is not followed by a clause number.
///
/// Kept rather than skipped, because a citation this reader cannot parse is a citation
/// nothing checks — which is the condition the whole module exists to end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedCitation {
    /// What followed the `§`, up to 24 characters of it.
    pub text: String,
    /// The 1-based line it appears on.
    pub line: usize,
}

/// A rustdoc blockquote, claiming to be the standard's own words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quotation {
    /// The quoted text, with the `///` and `>` markers removed and the lines joined.
    pub text: String,
    /// The 1-based line the blockquote starts on.
    pub line: usize,
    /// The clause it is attributed to: the nearest citation before it in the same comment.
    pub clause: Option<ClauseNumber>,
}

/// A `§` that belongs to a document other than ISO 32000-2.
///
/// `§` in this tree means "a clause of ISO 32000-2" — that is what makes every one of them
/// checkable — and the failure this catches is not a typo but a *readable* citation of
/// something else. `RFC 3986 §5.2` reads correctly to a person and checks as ISO 32000-2's
/// §5.2, which exists, so it passes in silence while pointing at another document entirely.
/// The first one arrived in the eightieth session with §12.6.4.8's URI resolution, and one of
/// its four spellings landed on a real clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignCitation {
    /// The document named before the `§`, as written.
    pub document: String,
    /// The 1-based line it appears on.
    pub line: usize,
}

/// A table that belongs to a document other than ISO 32000-2.
///
/// [`ForeignCitation`] one level down, and the two are not symmetrical. A `§` in this tree
/// *means* a clause of ISO 32000-2, so naming another document in front of one is a finding;
/// the word `Table` means nothing of the kind, and `ISO/TS 32002 Table 3` is correct writing
/// — a table is named with the standard that captions it or with none at all, and the second
/// case is the one that means ours.
///
/// **What is not correct is checking such a reference against ISO 32000-2's captions**, which
/// is what happened for as long as [`TableReference`] existed: ISO 32000-2 has a Table 3 and a
/// Table 4, so twenty-one references to ISO/TS 32002's supported ECDSA and `EdDSA` curves passed
/// the gate as the escape sequences in literal strings and the examples of literal names, and
/// the listing that prints a title beside every cited number printed those two titles. It is
/// exactly [`ForeignCitation`]'s failure — a reference that reads correctly to a person and
/// resolves, in silence, in the wrong document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignTable {
    /// The document named before the word `Table`, as written.
    pub document: String,
    /// The designation that document captions the table with.
    pub designation: String,
    /// The 1-based line it appears on.
    pub line: usize,
}

/// A reference to one of the standard's numbered tables.
///
/// Table numbers are the half of a citation nothing checked until the thirteenth session,
/// and one was already wrong: `§9.3.6 Table 106` had been copied into four comments, two
/// tests and a written report, and Table 106 is the text-*positioning* operators. A clause
/// number that names nothing is caught by the parser; a table number that names the wrong
/// table looks exactly like a right one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableReference {
    /// The table number named.
    pub table: u16,
    /// The 1-based line it appears on.
    pub line: usize,
    /// The clause it is attributed to, or `None` if none is in scope to attribute it to.
    pub clause: Option<ClauseNumber>,
}

/// A reference to one of the standard's tables by whatever its caption designates it.
///
/// **A table number is not always a number**, and until the eight-hundred-and-twentieth session
/// nothing here could say so: [`TableReference`] parses the digits after `Table ` and stops, so
/// `Table Annex O.3`, `Table D.2` and `Table 125a` are read as no reference at all. The standard
/// captions tables four ways — a bare integer, an integer with a letter (`Table 125a`), an
/// annex's letter and number (`Table D.2`), and the two in Annex O, which are captioned
/// `Table Annex O.3` and `Table Annex O.4`.
///
/// The two populations are kept apart rather than merged because they answer different
/// questions. [`Scan::tables`] is what the conformance gate checks against
/// [`crate::clause::ClauseIndex::table_title`]: a bare number is the only designation
/// `table_title` takes. This one is *every* designation the tree cites, which is what an
/// instrument asking **which tables does this tree stand on** needs — and the first such
/// instrument is `spec-errata renumbered`, for an erratum that renumbers a table by striking
/// its caption.
///
/// **Both populations are ISO 32000-2's**, and neither was until the
/// eight-hundred-and-thirty-second session: a reference [`ForeignTable`] claims is in neither,
/// because it belongs to a document this tree has no conversion of and cannot check a number
/// against. That is the one thing about `Scan::tables` that has changed since it was written,
/// and it changed because the alternative is a gate that answers about the wrong standard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableDesignation {
    /// The designation as the standard's caption writes it: `104`, `125a`, `D.2`, `Annex O.3`.
    pub designation: String,
    /// The 1-based line it appears on.
    pub line: usize,
    /// The clause it is attributed to, or `None` if none is in scope to attribute it to.
    pub clause: Option<ClauseNumber>,
}

/// Everything one source file says about the standard.
#[derive(Debug, Clone, Default)]
pub struct Scan {
    /// Every `§` citation, in file order.
    pub citations: Vec<Citation>,
    /// Every `§` that could not be read as a citation.
    pub malformed: Vec<MalformedCitation>,
    /// Every `§` attached to a document that is not ISO 32000-2.
    pub foreign: Vec<ForeignCitation>,
    /// Every rustdoc blockquote, in file order.
    pub quotations: Vec<Quotation>,
    /// Every `Table N` reference of ISO 32000-2's, in file order.
    pub tables: Vec<TableReference>,
    /// Every `Table <designation>` reference of ISO 32000-2's, in file order — the wider
    /// population.
    pub designations: Vec<TableDesignation>,
    /// Every `Table` reference attached to a document that is not ISO 32000-2.
    pub foreign_tables: Vec<ForeignTable>,
}

/// Reads one source file's citations and quotations.
#[must_use]
pub fn scan(source: &str) -> Scan {
    let mut scan = Scan::default();

    // The state of the doc comment currently being read: the last clause cited within it,
    // and the blockquote being accumulated. Both end when the comment does, so that a
    // citation cannot attribute a quotation attached to a different item.
    let mut cited: Option<ClauseNumber> = None;
    let mut quoting: Option<Quotation> = None;
    let mut fenced = false;

    for (index, line) in source.lines().enumerate() {
        let line_number = index.saturating_add(1);
        read_citations(line, line_number, &mut scan);
        read_tables(line, line_number, cited.as_ref(), &mut scan);

        let doc = doc_comment_body(line);
        if let Some(body) = doc {
            // A fenced example inside a doc comment is illustration, not documentation of
            // this item: this module's own comment shows the convention it reads, and would
            // otherwise be scanned as a claim about the standard.
            if body.trim_start().starts_with("```") {
                fenced = !fenced;
            }
            if fenced {
                if let Some(quotation) = quoting.take() {
                    scan.quotations.push(quotation);
                }
                continue;
            }
            if let Some(quoted) = body.trim_start().strip_prefix('>') {
                let quotation = quoting.get_or_insert_with(|| Quotation {
                    text: String::new(),
                    line: line_number,
                    clause: cited.clone(),
                });
                if !quotation.text.is_empty() {
                    quotation.text.push(' ');
                }
                quotation.text.push_str(quoted.trim());
                continue;
            }
            if let Some(last) = scan
                .citations
                .iter()
                .rfind(|citation| citation.line == line_number)
            {
                cited = Some(last.number.clone());
            }
        }

        // A blockquote ends at the first line that does not continue it, and a doc comment's
        // attribution ends with the comment.
        if let Some(quotation) = quoting.take() {
            scan.quotations.push(quotation);
        }
        if doc.is_none() {
            cited = None;
        }
    }
    if let Some(quotation) = quoting.take() {
        scan.quotations.push(quotation);
    }

    scan
}

/// Reads the citations and table references in plain prose, outside any source file.
///
/// The conformance ledger's notes are the reason this exists. They are the densest prose about
/// the standard this project has — 823 rows, most of them naming clauses and tables — and until
/// the eighty-second session **nothing checked a word of it**: [`scan`] reads Rust sources, and
/// the ledger is TOML. Three table numbers in it were wrong on the first run, all three by
/// naming ISO 32000-1's number for a table ISO 32000-2 renumbered.
///
/// Quotations are deliberately not read. A ledger note quotes the standard constantly and also
/// quotes this project's own past conclusions, and it has no blockquote syntax to tell the two
/// apart — so a checker here would either report the second kind or have to guess.
#[must_use]
pub fn scan_prose(text: &str) -> Scan {
    let mut scan = Scan::default();
    for (index, line) in text.lines().enumerate() {
        let line_number = index.saturating_add(1);
        read_citations(line, line_number, &mut scan);
        // `read_tables` reads the comment part of a source line; prose is all comment.
        let commented = format!("//{line}");
        read_tables(&commented, line_number, None, &mut scan);
    }
    scan
}

/// The text of a doc comment line, or `None` if the line is not one.
fn doc_comment_body(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix("///")
        .or_else(|| trimmed.strip_prefix("//!"))
}

/// Reads every `§` on one line into `scan`.
fn read_citations(line: &str, line_number: usize, scan: &mut Scan) {
    for (position, _) in line.match_indices('\u{a7}') {
        let after = line
            .get(position.saturating_add('\u{a7}'.len_utf8())..)
            .unwrap_or_default();
        // `§{clause}` is a format string building a citation at runtime, which is how this
        // project's reports name clauses. There is no number here to check.
        if after.starts_with('{') {
            continue;
        }
        // A `§` that belongs to another document is not a citation of this one, and checking
        // its number against ISO 32000-2's clauses is how it would pass unnoticed.
        if let Some(document) = another_document(line.get(..position).unwrap_or_default()) {
            scan.foreign.push(ForeignCitation {
                document,
                line: line_number,
            });
            continue;
        }
        // An annex's number opens with its letter — `§K.2`, `§Q` — and everything after it
        // is a clause number like any other. Only the *first* character may be a letter.
        let mut characters = after.chars();
        let mut digits = String::new();
        if let Some(opening) = characters
            .next()
            .filter(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
        {
            digits.push(opening);
        }
        digits.extend(
            characters.take_while(|character| character.is_ascii_digit() || *character == '.'),
        );
        // A citation at the end of a sentence carries the full stop: `§7.4.9.` is `§7.4.9`.
        let number = digits.trim_end_matches('.');
        match number.parse::<ClauseNumber>() {
            Ok(number) => scan.citations.push(Citation {
                number,
                line: line_number,
            }),
            Err(_) => scan.malformed.push(MalformedCitation {
                text: after.chars().take(24).collect(),
                line: line_number,
            }),
        }
    }
}

/// The other document a `§` or a `Table` belongs to, from the text before it on the same line.
///
/// Recognised by the shape every such citation has: an acronym and a number, immediately
/// before the section sign or the word — `RFC 3986 §5.2.2`, `ISO 15076-1 §6`,
/// `ISO/TS 32002 Table 3`. ISO 32000-2 itself is deliberately *not* foreign, because naming
/// the standard before its own clause number is exactly what `CLAUDE.md` principle 5 asks for.
///
/// **Immediately** is what keeps it honest, and it is why one rule serves both callers: a
/// sentence that merely mentions another standard puts a word between the two — `ISO/TS 32002
/// amends Table 21` reads *amends* here and is ISO 32000-2's table, correctly.
///
/// Nothing wider is attempted. A checker that guessed at every phrase before a `§` would
/// report the sentences that merely mention another standard, and this one has to be right
/// every time to be worth having.
fn another_document(before: &str) -> Option<String> {
    let before = before.trim_end();
    let mut words = before.split_whitespace().rev();
    let number = words.next()?;

    // A document that is a *file* of this project names itself in one word — an
    // upper-case stem and the `.md` suffix, immediately before the sign. Such a
    // `§` used to slip through whenever its number landed on a real clause
    // (QUORRA_FEEDBACK.md section 2's own citation did); this arm turns it into
    // a named finding, whose message teaches the "FILE.md section N" spelling
    // the tree writes for every document that is not the standard.
    //
    // **The directory in front of the name is dropped first, and until the
    // three-hundred-and-ninety-first session it was not** — `doc/` is not upper
    // case, so a citation written with a path passed this arm for the whole of its
    // life. There were eight in the tree and six of them named QUORRA_FEEDBACK.md,
    // which is the very document the paragraph above cites as the case this exists
    // to catch. A stem test that a lower-case path defeats is a test of how the
    // author spelled the path.
    let file = number.trim_matches(['(', '"', '`', ')', ',', ';']);
    let file = file.rsplit('/').next().unwrap_or(file);
    if let Some(stem) = file.strip_suffix(".md")
        && !stem.is_empty()
        && stem.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
    {
        return Some(file.to_owned());
    }

    // **A number has to have a digit in it and an acronym a letter**, and neither test said so
    // until the eight-hundred-and-thirty-second session: the character sets are permissive
    // because `ISO/IEC` needs the solidus and `32000-2` the hyphen, and `all` over a permissive
    // set is satisfied by a string made of nothing else. So `///` passed as an acronym — every
    // character is a solidus — and `-` passed as a number, which made the two words in front of
    // a wrapped doc comment's `Table` into a document called `/// -`. It was latent on the `§`
    // side for the whole of that arm's life and needed a bare number before the sign to show;
    // the `Table` caller reached it on the first run, twice.
    let name = words.next()?;
    let acronym = name.trim_start_matches(['(', '"', '`']);
    if !number
        .chars()
        .all(|character| character.is_ascii_digit() || character == '-')
        || !number.contains(|character: char| character.is_ascii_digit())
        || !acronym.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '/'
        })
        || !acronym.contains(|character: char| character.is_ascii_uppercase())
        || acronym.len() < 2
    {
        return None;
    }
    if acronym == "ISO" && number == "32000-2" {
        return None;
    }
    Some(format!("{acronym} {number}"))
}

/// Reads every `Table N` in one line's comment into `scan`.
///
/// Only comments are read. `Table` occurs in this tree's own identifiers — `CodeTable` is
/// one — and a checker that reported those would be a checker people learn to ignore.
///
/// A table is attributed to a clause cited on the same line, which is how this tree usually
/// writes one (`§9.6.4 Table 111`), and otherwise to the clause the enclosing doc comment
/// last cited. A reference with neither is left unattributed: there is nothing to check it
/// against, and inventing an attribution would produce a verdict about a clause nobody named.
///
/// A reference another standard's name stands in front of goes to [`Scan::foreign_tables`] and
/// to neither of the two ISO 32000-2 populations, for the reason [`ForeignTable`] states.
fn read_tables(line: &str, line_number: usize, cited: Option<&ClauseNumber>, scan: &mut Scan) {
    let Some(comment) = line.find("//").and_then(|at| line.get(at..)) else {
        return;
    };

    // A clause named on this line is the one the reference belongs to; the running
    // attribution only applies where this line names none.
    let here = scan
        .citations
        .iter()
        .rfind(|citation| citation.line == line_number)
        .map(|citation| citation.number.clone())
        .or_else(|| cited.cloned());

    for (position, _) in comment.match_indices("Table ") {
        // `CodeTable 3` is an identifier followed by a number, not a citation.
        if comment
            .get(..position)
            .unwrap_or_default()
            .ends_with(|character: char| character.is_alphanumeric())
        {
            continue;
        }
        let rest = comment
            .get(position.saturating_add("Table ".len())..)
            .unwrap_or_default();

        // A table another standard captions is that standard's, and checking its designation
        // against ISO 32000-2's captions is how it passes unnoticed — the same sentence
        // `read_citations` writes over a `§`, and the same rule, `another_document`. Both
        // populations below are about ISO 32000-2, so this leaves them before they are reached
        // rather than filtering afterwards.
        if let Some(document) = another_document(comment.get(..position).unwrap_or_default())
            && let Some(designation) = designation_at(rest)
        {
            scan.foreign_tables.push(ForeignTable {
                document,
                designation,
                line: line_number,
            });
            continue;
        }

        // The numbered population, which the conformance gate checks: the digits that open the
        // designation, and nothing where there are none.
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(table) = digits.parse::<u16>() {
            scan.tables.push(TableReference {
                table,
                line: line_number,
                clause: here.clone(),
            });
        }

        if let Some(designation) = designation_at(rest) {
            scan.designations.push(TableDesignation {
                designation,
                line: line_number,
                clause: here.clone(),
            });
        }
    }
}

/// The table designation `text` opens with, if it opens with one.
///
/// The four caption shapes the standard uses, read off `doc/md/`'s own caption lines: `104`,
/// `125a`, `D.2` and `Annex O.3`. What separates a designation from the ordinary English after
/// the word *Table* is that it **carries a digit** — which is the whole of the rule, and it is
/// enough: this tree writes `Table N`, `Table NNN`, `Table structure` and `Table or` in prose
/// about tables in general, and not one of them has one.
///
/// A trailing full stop is dropped because a citation ends a sentence as often as not
/// (`Table D.4.`), and a designation never ends in one.
pub(crate) fn designation_at(text: &str) -> Option<String> {
    // `Annex ` is part of the caption rather than a word before it: the standard captions Annex
    // O's two tables `Table Annex O.3` and `Table Annex O.4`, so a designation that dropped the
    // prefix would name a table no caption has.
    let (prefix, body) = text
        .strip_prefix("Annex ")
        .map_or(("", text), |body| ("Annex ", body));
    let designation: String = body
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '.')
        .collect();
    let designation = designation.trim_end_matches('.');
    if !designation.contains(|character: char| character.is_ascii_digit()) {
        return None;
    }
    Some(format!("{prefix}{designation}"))
}

/// Every `.rs` file under `roots`, in a stable order.
///
/// # Errors
///
/// If a directory cannot be read. A checker that silently skipped an unreadable directory
/// would report a clean tree for a tree it had not looked at.
pub fn rust_sources(roots: &[PathBuf]) -> std::io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for root in roots {
        collect(root, &mut found)?;
    }
    found.sort();
    Ok(found)
}

fn collect(directory: &Path, found: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            // Build outputs are not sources, and are large enough to matter.
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            collect(&path, found)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two markers these fixtures are made of are built rather than written, so that a
    /// file full of deliberately malformed citations and unattributed quotations is not
    /// itself a finding when the gate scans the tree. It is the one file where that matters,
    /// and it is cheaper than teaching the scanner to recognise its own test data.
    const SECTION: char = '\u{a7}';
    const DOC: &str = "///";

    fn number(text: &str) -> ClauseNumber {
        text.parse().unwrap()
    }

    #[test]
    fn a_citation_is_read_wherever_it_appears() {
        let source =
            format!("// {SECTION}8.9.6.2 in a plain comment\nfn f() {{}} // and {SECTION}7.4.9.\n");
        let scan = scan(&source);
        assert_eq!(
            scan.citations,
            vec![
                Citation {
                    number: number("8.9.6.2"),
                    line: 1
                },
                Citation {
                    number: number("7.4.9"),
                    line: 2
                },
            ]
        );
    }

    #[test]
    fn a_section_sign_with_no_number_is_a_finding_not_a_skip() {
        let scan = scan(&format!("{DOC} see {SECTION} above\n"));
        assert!(scan.citations.is_empty());
        assert_eq!(scan.malformed.len(), 1);
    }

    #[test]
    fn a_blockquote_takes_the_clause_cited_before_it() {
        let source = format!(
            "{DOC} ISO 32000-2 {SECTION}7.7.3.3 defines the crop box:\n\
             {DOC}\n\
             {DOC} > the region to which the contents\n\
             {DOC} > shall be clipped\n\
             {DOC}\n\
             {DOC} and that is what a viewer shows.\n\
             pub fn f() {{}}\n"
        );
        let scan = scan(&source);
        assert_eq!(scan.quotations.len(), 1);
        let quotation = scan.quotations.first().unwrap();
        assert_eq!(
            quotation.text,
            "the region to which the contents shall be clipped"
        );
        assert_eq!(quotation.clause, Some(number("7.7.3.3")));
        assert_eq!(quotation.line, 3);
    }

    /// Attribution must not survive the comment it was made in, or a quotation would take
    /// the clause number of whatever item happened to be written above it.
    #[test]
    fn a_citation_does_not_attribute_the_next_item_s_quotation() {
        let source = format!(
            "{DOC} {SECTION}7.4.9 is here.\n\
             pub fn f() {{}}\n\
             \n\
             {DOC} > an unattributed quotation\n\
             pub fn g() {{}}\n"
        );
        let scan = scan(&source);
        assert_eq!(scan.quotations.len(), 1);
        assert_eq!(scan.quotations.first().unwrap().clause, None);
    }

    #[test]
    fn a_table_reference_is_read_from_a_comment_and_not_from_code() {
        let source = format!(
            "{DOC} {SECTION}9.3.6 Table 104 gives the eight modes.\n\
             struct CodeTable 3;\n\
             let x = \"Table 999\";\n"
        );
        let scan = scan(&source);
        assert_eq!(
            scan.tables,
            vec![TableReference {
                table: 104,
                line: 1,
                clause: Some(number("9.3.6")),
            }],
            "only the comment's reference is a citation: `CodeTable 3` is an identifier and \
             a string literal is data"
        );
    }

    /// A reference nothing attributes is kept, because the alternative is dropping it.
    #[test]
    fn a_table_reference_outside_any_citation_is_unattributed_rather_than_skipped() {
        let scan = scan("// Table 87's default is false.\n");
        assert_eq!(scan.tables.len(), 1);
        assert_eq!(scan.tables.first().unwrap().clause, None);
    }

    /// The four caption shapes the standard uses, and the tree's own prose about tables in
    /// general, which carries no digit and is therefore not a designation.
    #[test]
    fn a_designation_is_every_caption_shape_and_the_numbered_population_is_unchanged() {
        let scan = scan(
            "// Table 104's modes, Table 125a, Table D.2 and Table Annex O.3's `ef`.\n\
             // A citation ending a sentence writes Table D.4.\n\
             // In general a Table N is cited as Table NNN, whatever the Table structure.\n",
        );
        assert_eq!(
            scan.designations
                .iter()
                .map(|reference| reference.designation.as_str())
                .collect::<Vec<&str>>(),
            vec!["104", "125a", "D.2", "Annex O.3", "D.4"],
            "the designation is the whole caption, `Annex ` included, and a full stop that \
             ends the sentence is not part of it"
        );
        assert_eq!(
            scan.tables
                .iter()
                .map(|reference| reference.table)
                .collect::<Vec<u16>>(),
            vec![104, 125],
            "the numbered population the gate checks is exactly what it was: the digits that \
             open a designation, and nothing where there are none"
        );
    }

    #[test]
    fn two_blockquotes_in_one_comment_are_two_quotations() {
        let source = format!(
            "{DOC} {SECTION}9.6.5.4 says two things.\n\
             {DOC}\n\
             {DOC} > the first\n\
             {DOC}\n\
             {DOC} and also\n\
             {DOC}\n\
             {DOC} > the second\n\
             pub fn f() {{}}\n"
        );
        let scan = scan(&source);
        assert_eq!(scan.quotations.len(), 2);
        assert_eq!(scan.quotations.get(1).unwrap().text, "the second");
    }

    /// A fenced example shows what a doc comment looks like; it is not one. This module's
    /// own comment contains exactly that, and was scanned as two unattributed quotations
    /// until the fence was understood.
    #[test]
    fn a_fenced_example_is_illustration_rather_than_a_quotation() {
        let source = format!(
            "{DOC} {SECTION}9.6.5.4 shows the convention:\n\
             {DOC}\n\
             {DOC} ```text\n\
             {DOC} > a quotation in an example\n\
             {DOC} ```\n\
             {DOC}\n\
             {DOC} > a real one\n\
             pub fn f() {{}}\n"
        );
        let scan = scan(&source);
        assert_eq!(scan.quotations.len(), 1);
        assert_eq!(scan.quotations.first().unwrap().text, "a real one");
    }

    /// A `§` that belongs to another document is not a citation of this one.
    ///
    /// The case that made this necessary: `RFC 3986 §5.2` is *correct writing* about the
    /// document §12.6.4.8 defers to, and ISO 32000-2 has a §5.2 of its own — so the citation
    /// checker read it, found the clause, and said nothing.
    #[test]
    fn a_section_sign_after_another_documents_name_is_not_a_citation() {
        let source = format!("{DOC} resolved by RFC 3986 {SECTION}5.2.2's algorithm\n");
        let scan = scan(&source);
        assert!(scan.citations.is_empty(), "{:?}", scan.citations);
        assert_eq!(
            scan.foreign
                .first()
                .map(|foreign| foreign.document.as_str()),
            Some("RFC 3986")
        );
    }

    /// ISO 32000-2's own name before its own clause number is the convention, not a finding.
    #[test]
    fn the_standards_own_name_before_a_clause_is_a_citation() {
        let source = format!("{DOC} ISO 32000-2 {SECTION}9.6.5.4 names five routes\n");
        let scan = scan(&source);
        assert!(scan.foreign.is_empty());
        assert_eq!(scan.citations.len(), 1);
    }

    /// A project document's own file name before a `§` marks the citation as another
    /// document's — a finding with the file's name on it, not a silent pass against
    /// whichever ISO clause the number happens to land on.
    #[test]
    fn a_project_documents_file_name_before_a_section_is_not_a_citation() {
        let source = format!("{DOC} measured, RENDER_LIBRARY.md {SECTION}4.5 says, not assumed\n");
        let scan = scan(&source);
        assert!(scan.citations.is_empty(), "{:?}", scan.citations);
        assert_eq!(
            scan.foreign
                .first()
                .map(|foreign| foreign.document.as_str()),
            Some("RENDER_LIBRARY.md")
        );
    }

    /// A table another standard captions is not one of ISO 32000-2's, and the failure it
    /// caused is the same one the `§` arm above exists for, one level down.
    ///
    /// `ISO/TS 32002 Table 3` is *correct writing* — that is where the supported ECDSA curves
    /// are — and ISO 32000-2 has a Table 3 too, the escape sequences in literal strings. So the
    /// reference resolved, the gate passed, and the listing that prints a title beside every
    /// number printed the wrong document's table.
    #[test]
    fn a_table_after_another_documents_name_is_not_one_of_the_standards() {
        let scan =
            scan("// One of ISO/TS 32002 Table 3's curves, and ISO/IEC 15444-1 Table A.19.\n");
        assert!(scan.tables.is_empty(), "{:?}", scan.tables);
        assert!(scan.designations.is_empty(), "{:?}", scan.designations);
        assert_eq!(
            scan.foreign_tables
                .iter()
                .map(|table| (table.document.as_str(), table.designation.as_str()))
                .collect::<Vec<(&str, &str)>>(),
            vec![("ISO/TS 32002", "3"), ("ISO/IEC 15444-1", "A.19")],
            "both populations the gate checks are ISO 32000-2's, and neither of these is"
        );
    }

    /// A permissive character set is satisfied by a string made of nothing but its punctuation,
    /// and both of `another_document`'s were.
    ///
    /// `///` is every character a solidus, which `ISO/IEC` needs; `-` is every character a
    /// hyphen, which `32000-2` needs. A doc comment wrapping between a standard's name and its
    /// table put those two words in front of the word, and the reference went to a document
    /// called `/// -`.
    #[test]
    fn a_comment_marker_is_not_an_acronym_and_a_hyphen_is_not_a_number() {
        let scan = scan(
            "/// - Table 172 is the annotation flags.\n\
             /// something written about ISO/TS\n\
             /// 32002 Table 3, wrapped between the two.\n",
        );
        assert!(scan.foreign_tables.is_empty(), "{:?}", scan.foreign_tables);
        assert_eq!(
            scan.tables
                .iter()
                .map(|reference| reference.table)
                .collect::<Vec<u16>>(),
            vec![172, 3]
        );
    }

    /// The standard's own name before its own table is the convention, and a sentence that
    /// merely mentions another standard is not a foreign reference at all.
    #[test]
    fn the_standards_own_table_survives_a_neighbouring_documents_name() {
        let scan = scan(
            "// ISO 32000-2 Table 109 lets a standard-14 dictionary omit `/Widths`.\n\
             // ISO/TS 32002 amends Table 21 rather than captioning it.\n",
        );
        assert!(scan.foreign_tables.is_empty(), "{:?}", scan.foreign_tables);
        assert_eq!(
            scan.tables
                .iter()
                .map(|reference| reference.table)
                .collect::<Vec<u16>>(),
            vec![109, 21],
            "`immediately before` is the whole rule: one word between the two names and the \
             table is the standard's"
        );
    }

    /// The directory in front of the file name does not excuse it.
    ///
    /// This is the same finding as the test above and it is separate because the arm above
    /// missed it for the whole of its life: the stem is checked for upper case, and `doc/` is
    /// not upper case, so every citation written with a path passed. Seven of the eight in the
    /// tree named `QUORRA_FEEDBACK.md`, which is the document the arm's own comment cites as
    /// the case it exists to catch.
    #[test]
    fn a_path_in_front_of_a_project_documents_name_does_not_excuse_it() {
        let source = format!("{DOC} the shape doc/QUORRA_FEEDBACK.md {SECTION}12 asked for\n");
        let scan = scan(&source);
        assert!(scan.citations.is_empty(), "{:?}", scan.citations);
        assert_eq!(
            scan.foreign
                .first()
                .map(|foreign| foreign.document.as_str()),
            Some("QUORRA_FEEDBACK.md")
        );
    }
}
