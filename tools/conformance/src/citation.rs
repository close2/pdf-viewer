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
    /// Every `Table N` reference, in file order.
    pub tables: Vec<TableReference>,
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

/// The other document a `§` belongs to, from the text before it on the same line.
///
/// Recognised by the shape every such citation has: an acronym and a number, immediately
/// before the section sign — `RFC 3986 §5.2.2`, `ISO 15076-1 §6`. ISO 32000-2 itself is
/// deliberately *not* foreign, because naming the standard before its own clause number is
/// exactly what `CLAUDE.md` principle 5 asks for.
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

    let name = words.next()?;
    let acronym = name.trim_start_matches(['(', '"', '`']);
    if !number
        .chars()
        .all(|character| character.is_ascii_digit() || character == '-')
        || !acronym.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '/'
        })
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
        let digits: String = comment
            .get(position.saturating_add("Table ".len())..)
            .unwrap_or_default()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        let Ok(table) = digits.parse::<u16>() else {
            continue;
        };

        scan.tables.push(TableReference {
            table,
            line: line_number,
            clause: here.clone(),
        });
    }
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
