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
//! # The three things a `§` can be
//!
//! A `§` with nothing in front of it is a clause of ISO 32000-2, and that is the population the
//! gate checks. The other two are told apart by the document named before the sign, because a
//! section number resolves in *whatever document it belongs to* and checking it against the
//! wrong one passes in silence:
//!
//! - [`ForeignCitation`] — another standard's, which is a finding. `RFC 3986 §5.2` reads
//!   correctly and lands on a clause ISO 32000-2 has.
//! - [`ProjectSection`] — one of this project's own documents', which is not a finding but is
//!   not a clause either. `` `doc/todo/02` §2 `` is how this tree has cited itself for hundreds
//!   of sessions, and every one of those read as a clause citation until the
//!   nine-hundred-and-seventy-seventh session.
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
/// A `§` with no document in front of it means "a clause of ISO 32000-2" — that is what makes
/// every one of them checkable — and the failure this catches is not a typo but a *readable*
/// citation of something else. `RFC 3986 §5.2` reads correctly to a person and checks as ISO
/// 32000-2's §5.2, which exists, so it passes in silence while pointing at another document
/// entirely. The first one arrived in the eightieth session with §12.6.4.8's URI resolution, and
/// one of its four spellings landed on a real clause.
///
/// **This is another *standard's* section**, and it is a finding. A section of one of this
/// project's own documents is [`ProjectSection`], which is not: the two are separated because a
/// gate that reported `doc/todo/02` §2 would be reporting the spelling this tree has written
/// several hundred times.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignCitation {
    /// The document named before the `§`, as written.
    pub document: String,
    /// The 1-based line it appears on.
    pub line: usize,
}

/// A `§` naming a section of one of this project's own documents.
///
/// **This tree writes `§` for its own documents' sections, and the scanner could not see it.**
/// `` `doc/todo/02` §2 ``, `doc/oracle-and-corpus.md §3d` and `doc/PLAN.md` §5a are how the
/// project has cited itself for hundreds of sessions; every one of them read as an ISO 32000-2
/// citation until the nine-hundred-and-seventy-seventh session, and passed
/// `every_citation_names_a_clause_that_exists` by landing on a clause that exists.
/// [`ForeignCitation`]'s failure, with our own documents in the place of another standard —
/// except that where `RFC 3986 §5.2` is a *mistake* to report, these are correct writing to
/// classify.
///
/// Two shapes reach here, and only the first of them names a document:
///
/// - **A document in front of the sign**: a `doc/` path, a Markdown file name, `ADR NNNN`, or
///   one of this project's own RFCs, which are numbered with a leading zero (`RFC 0002`) where
///   the IETF's are not (`RFC 3986`).
/// - **A number ISO 32000-2 cannot have.** A letter-suffixed section — `§3a`, `§5a`, `§2b` — is
///   provably not a clause of the standard, whose numbered headings are digits and full stops,
///   with an annex's opening letter the only letter in any of them. Such a `§` is *somebody's*
///   section and this line does not say whose, which is what [`Self::document`] being `None`
///   records. Nothing can check it, and a rule that guessed which document a `§3a` twelve lines
///   below `doc/oracle-and-corpus.md` belonged to would be guessing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSection {
    /// The document named immediately before the `§`, or `None` where the line names none.
    pub document: Option<String>,
    /// The section as written: `2`, `5.2`, `3a`.
    pub section: String,
    /// The 1-based line it appears on.
    pub line: usize,
}

impl ProjectSection {
    /// The clause of ISO 32000-2 this section number would be checked against, if anything
    /// checked it against ISO 32000-2.
    ///
    /// **This is the measurement of the defect rather than a use of it.** A section of one of
    /// our own documents is merely *wrong* to read as a clause; it is dangerous when the number
    /// it carries is also a clause the standard has, because then the reader resolves it, the
    /// gate passes, and the coverage instrument counts a citation of a clause nobody cited.
    /// `doc/todo/03` §9 and `doc/todo/11` §6 are the shape: clause 9 is text and clause 6 is
    /// conformance, and both rows are in the ledger's population.
    ///
    /// The number is read the way the citation reader reads one — the opening letter of an annex
    /// or a digit, then digits and full stops — so `3a` answers for clause 3, which is what it
    /// was counted as.
    #[must_use]
    pub fn would_resolve_as(&self) -> Option<ClauseNumber> {
        let mut characters = self.section.chars();
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
        digits.trim_end_matches('.').parse().ok()
    }
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
    /// Every `§` attached to another *standard*, which is a finding.
    pub foreign: Vec<ForeignCitation>,
    /// Every `§` naming a section of one of this project's own documents, which is not.
    pub sections: Vec<ProjectSection>,
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
        // A code span holding nothing but the sign is the *character* being named rather than a
        // citation — "every `§` in this file is an ISO 32000-2 number" is a sentence about the
        // convention, and there is no number after it to read.
        //
        // Until the nine-hundred-and-seventy-seventh session nothing outside `tools/conformance`
        // had written that sentence, and this crate is the one the scan does not read. Then four
        // `pdf-archive` module headers said it in one round — a file whose clause numbers resolve
        // in two editions of the standard has to say which edition it means — and each was
        // reported as a citation the checker could not read. Correct writing about the convention
        // is not a malformed citation.
        if after.starts_with('`') && line.get(..position).is_some_and(|up| up.ends_with('`')) {
            continue;
        }
        // A `§` that belongs to another document is not a citation of this one, and checking
        // its number against ISO 32000-2's clauses is how it would pass unnoticed. Another
        // standard's section is a finding; one of this project's own documents' sections is
        // what this tree writes several hundred times and is classified rather than reported.
        match another_document(line.get(..position).unwrap_or_default()) {
            Some(Named::Standard(document)) => {
                scan.foreign.push(ForeignCitation {
                    document,
                    line: line_number,
                });
                continue;
            }
            Some(Named::Ours(document)) => {
                scan.sections.push(ProjectSection {
                    document: Some(document),
                    section: section_at(after),
                    line: line_number,
                });
                continue;
            }
            None => {}
        }
        // A number ISO 32000-2 cannot have is not a clause of it, whatever stands in front of
        // the sign. The standard's numbered headings are digits and full stops, with an annex's
        // opening letter the only letter any of them carries — so `§3a` and `§5a` are sections
        // of a document this line does not name, and reading them as clauses 3 and 5 is the
        // silent pass `ProjectSection` exists to end.
        let section = section_at(after);
        if section.contains(|character: char| character.is_ascii_lowercase()) {
            scan.sections.push(ProjectSection {
                document: None,
                section,
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

/// The designation a `§` is followed by, as written: `7.4.9`, `3a`, `Q`.
///
/// Everything a section number is made of — letters, digits and full stops — and a trailing
/// full stop dropped, because a citation ends a sentence as often as not (`§7.4.9.`). Empty
/// where the sign is followed by none of those, which is what [`MalformedCitation`] records.
fn section_at(after: &str) -> String {
    let designation: String = after
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '.')
        .collect();
    designation.trim_end_matches('.').to_owned()
}

/// Which document a `§` or a `Table` belongs to, and whose it is.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Named {
    /// Another standard. A `§` after one is a finding: its section number would otherwise be
    /// checked against ISO 32000-2's clauses and pass by landing on one.
    Standard(String),
    /// One of this project's own documents. A `§` after one is this tree's own spelling for a
    /// section of it, and is classified rather than reported.
    Ours(String),
}

/// The document a `§` or a `Table` belongs to, from the text before it on the same line.
///
/// Two families are recognised, and the caller does different things with them:
///
/// - **Another standard**, by the shape every such citation has: an acronym and a number —
///   `RFC 3986 §5.2.2`, `ISO 15076-1 §6`, `ISO/TS 32002 Table 3`. ISO 32000-2 itself is
///   deliberately not one, because naming the standard before its own clause number is exactly
///   what `CLAUDE.md` principle 5 asks for.
/// - **One of this project's own documents**: a `doc/` path (`doc/todo/58`), a Markdown file
///   name in any case (`doc/oracle-and-corpus.md`, `QUORRA_FEEDBACK.md`), an `ADR NNNN`, or one
///   of this project's own RFCs — which are numbered with a leading zero, `RFC 0002`, where the
///   IETF's are not.
///
/// # What counts as *immediately before*, and why the rule is not looser
///
/// **Immediately** is what keeps this honest, and it is why one rule serves both callers: a
/// sentence that merely mentions another standard puts a word between the two — `ISO/TS 32002
/// amends Table 21` reads *amends* here and is ISO 32000-2's table, correctly.
///
/// What the rule had to learn in the nine-hundred-and-seventy-seventh session is that a name is
/// often *wrapped* rather than bare, and that wrapping is not distance: this tree writes
/// `` `doc/todo/02` §2 `` with backticks round the path, and the word in front of the sign was
/// therefore `` `doc/todo/02` `` and matched nothing. So a wrapper is removed — a backtick, a
/// quotation mark or an emphasis marker at either end, and an *opening* bracket — and everything
/// else is left where it is:
///
/// - **A trailing comma, full stop or closing bracket is the sentence moving on.** `(ADR 0044),
///   §12.5.6.5` is a citation of the standard with an ADR mentioned before it, and the comma is
///   the whole of the evidence for that. Trimming punctuation blindly — which the file-name arm
///   used to do — would have taken several hundred genuine clause citations out of the checked
///   population, because this tree ends a parenthetical in front of a `§` constantly.
/// - **A possessive is not adjacency either, and the tree proves it twice.**
///   `` `doc/conformance/ledger.toml`'s §8.7.4.1 row `` and `` `examples/absence_audit`'s §10.7.5
///   block `` both name a file that *discusses* a clause of the standard. `X §N` is a section of
///   X; `X's §N` is X's treatment of the standard's §N, and reading the second as the first
///   would lose a real citation.
fn another_document(before: &str) -> Option<Named> {
    let before = before.trim_end();
    let mut words = before.split_whitespace().rev();
    let number = unwrapped(words.next()?)?;

    // A document of this project's own, named in one word: a path under `doc/`, or a Markdown
    // file name with or without one.
    //
    // **The upper-case stem this arm used to require was a test of how the author spelled the
    // name**, twice over. `doc/` is not upper case, which the three-hundred-and-ninety-first
    // session found and fixed by dropping the directory; `oracle-and-corpus` is not upper case
    // either, and eleven citations of that document's sections were still being checked against
    // ISO 32000-2's clauses when this was written. A file name is a file name in any case.
    let leaf = number.rsplit('/').next().unwrap_or(number);
    let under_doc = number.split('/').next() == Some("doc") && number.contains('/');
    if leaf
        .strip_suffix(".md")
        .is_some_and(|stem| !stem.is_empty())
    {
        return Some(Named::Ours(leaf.to_owned()));
    }
    // A `doc/` path whose leaf carries no extension is one of this project's documents named
    // without its suffix — `doc/todo/58`, `doc/adr/0984`. One that carries a *different*
    // extension is a file rather than a document with sections, and `doc/conformance/ledger.toml`
    // is the tree's own counter-example: the `§` after it is the standard's.
    if under_doc && !leaf.contains('.') {
        return Some(Named::Ours(number.to_owned()));
    }

    // **A number has to have a digit in it and an acronym a letter**, and neither test said so
    // until the eight-hundred-and-thirty-second session: the character sets are permissive
    // because `ISO/IEC` needs the solidus and `32000-2` the hyphen, and `all` over a permissive
    // set is satisfied by a string made of nothing else. So `///` passed as an acronym — every
    // character is a solidus — and `-` passed as a number, which made the two words in front of
    // a wrapped doc comment's `Table` into a document called `/// -`. It was latent on the `§`
    // side for the whole of that arm's life and needed a bare number before the sign to show;
    // the `Table` caller reached it on the first run, twice.
    let acronym = unwrapped(words.next()?)?;
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
    // This project's own ADRs and RFCs are numbered in four digits with a leading zero, and the
    // IETF's RFCs are not — `RFC 0002` is `doc/rfc/0002`, `RFC 3986` is the IETF's. The zero is
    // the whole discriminator, and it is a property of the numbering rather than a convention
    // this round invented: every file under `doc/rfc/` and `doc/adr/` is `0NNN`.
    if acronym == "ADR" || (acronym == "RFC" && number.starts_with('0')) {
        return Some(Named::Ours(format!("{acronym} {number}")));
    }
    Some(Named::Standard(format!("{acronym} {number}")))
}

/// One word with its symmetric wrapper removed, or `None` if what is left is not a name.
///
/// A name in this tree's prose is routinely wrapped — `` `doc/todo/02` ``, `"RFC 3986"`,
/// `(ISO 15076-1` — and a wrapper is not distance from the sign that follows. What is *not*
/// removed is a separator: a trailing comma, semicolon, colon or full stop, or a closing
/// bracket nothing in this word opened, each of which means the sentence moved on before the
/// `§` (see [`another_document`]). A possessive is a separator for the same reason.
fn unwrapped(word: &str) -> Option<&str> {
    // A possessive first, because it ends in a letter and would otherwise read as part of the
    // name: `` `ledger.toml`'s §8.7.4.1 `` is the standard's clause, discussed in that file.
    if word.ends_with("'s") || word.ends_with('\u{2019}') || word.ends_with("\u{2019}s") {
        return None;
    }
    let inner = word
        .trim_start_matches(['(', '[', '{', '`', '"', '*', '\'', '\u{201c}', '\u{2018}'])
        .trim_end_matches(['`', '"', '*', '\'', '\u{201d}', '\u{2019}']);
    // A *closing* bracket is deliberately not trimmed, where a backtick or a quotation mark is.
    // An emphasis or a code span can open on an earlier word — this tree writes `` `RFC 3986` ``
    // with the opening backtick in front of the acronym — so a trailing one is a wrapper. A
    // closing bracket says the parenthetical ended before the sign did, which is the difference
    // between `(ADR 0044) §12.5.6.5` and a section of ADR 0044.
    inner
        .chars()
        .next_back()
        .is_some_and(|character| character.is_ascii_alphanumeric())
        .then_some(inner)
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
        //
        // **Only a standard's**, where the `§` side takes this project's own documents too. A
        // table is captioned by whatever document prints it and a section sign is not, so
        // `ISO/TS 32002 Table 3` is another standard's table while `doc/todo/02 §2` is a section
        // of ours; and no comment in this tree names one of our documents in front of a `Table`,
        // so there is no evidence here on which to decide what that would mean.
        if let Some(Named::Standard(document)) =
            another_document(comment.get(..position).unwrap_or_default())
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

    /// A project document's own file name before a `§` marks the citation as that document's
    /// section — not a silent pass against whichever ISO clause the number happens to land on.
    #[test]
    fn a_project_documents_file_name_before_a_section_is_not_a_citation() {
        let source = format!("{DOC} measured, RENDER_LIBRARY.md {SECTION}4.5 says, not assumed\n");
        let scan = scan(&source);
        assert!(scan.citations.is_empty(), "{:?}", scan.citations);
        assert_eq!(
            scan.sections
                .first()
                .map(|section| (section.document.as_deref(), section.section.as_str())),
            Some((Some("RENDER_LIBRARY.md"), "4.5"))
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
            scan.sections
                .first()
                .and_then(|section| section.document.as_deref()),
            Some("QUORRA_FEEDBACK.md")
        );
    }

    /// The spelling this tree writes several hundred times, and the one the scanner could not
    /// see: a backticked path, with the backticks between the name and the sign.
    ///
    /// Every one of these read as a clause citation until the nine-hundred-and-seventy-seventh
    /// session — and `doc/todo/03` §9, `doc/todo/02` §7 and `doc/todo/11` §6 landed on clauses
    /// 9, 7 and 6, which are in the ledger's population, so the citation fed the coverage
    /// instrument as well as passing the gate.
    #[test]
    fn a_backticked_project_document_before_a_section_is_that_documents_section() {
        let source = format!(
            "{DOC} the gates of `doc/todo/02` {SECTION}2 walk, and `doc/todo/03` {SECTION}9\n"
        );
        let scan = scan(&source);
        assert!(scan.citations.is_empty(), "{:?}", scan.citations);
        assert!(scan.foreign.is_empty(), "{:?}", scan.foreign);
        assert_eq!(
            scan.sections
                .iter()
                .map(|section| (section.document.as_deref(), section.section.as_str()))
                .collect::<Vec<(Option<&str>, &str)>>(),
            vec![(Some("doc/todo/02"), "2"), (Some("doc/todo/03"), "9"),]
        );
    }

    /// A letter-suffixed number is not a clause of ISO 32000-2, whatever stands before it.
    ///
    /// The standard's numbered headings are digits and full stops, with an annex's opening
    /// letter the only letter in any of them — so `§3a` is somebody's section and the line does
    /// not say whose. Reading it as clause 3 is the silent pass; guessing the document is the
    /// thing this declines to do.
    #[test]
    fn a_letter_suffixed_section_is_not_a_clause_and_names_no_document() {
        let scan = scan(&format!("// it sat at the head of {SECTION}3a's ranking\n"));
        assert!(scan.citations.is_empty(), "{:?}", scan.citations);
        assert_eq!(
            scan.sections
                .first()
                .map(|section| (section.document.as_deref(), section.section.as_str())),
            Some((None, "3a"))
        );
    }

    /// The negative half of the pair above, and the reason the rule is adjacency rather than
    /// proximity: a clause of the standard keeps its citation when a project document is
    /// *mentioned* in front of it.
    ///
    /// Both lines are the tree's own. A parenthetical that ends in a comma, and a possessive —
    /// `X's §N` is X's treatment of the standard's clause, where `X §N` is a section of X.
    /// Without this half the change would take several hundred genuine citations out of the
    /// checked population and the gate would still be green.
    #[test]
    fn a_document_mentioned_before_a_clause_does_not_take_its_citation_away() {
        let source = format!(
            "{DOC} for the `View` event (ADR 0044), {SECTION}12.5.6.5's appearance\n\
             {DOC} `doc/conformance/ledger.toml`'s {SECTION}8.7.4.1 row has claimed since\n\
             {DOC} `examples/absence_audit`'s {SECTION}10.7.5 block counts the documents\n\
             {DOC} ADR 0253 and ADR 0728.) {SECTION}14.7.5.2's integer\n"
        );
        let scan = scan(&source);
        assert!(scan.sections.is_empty(), "{:?}", scan.sections);
        assert_eq!(
            scan.citations
                .iter()
                .map(|citation| citation.number.to_string())
                .collect::<Vec<String>>(),
            vec!["12.5.6.5", "8.7.4.1", "10.7.5", "14.7.5.2"]
        );
    }

    /// This project's own RFCs are numbered with a leading zero and the IETF's are not, which is
    /// the whole discriminator between a section of `doc/rfc/0002` and a finding.
    #[test]
    fn our_rfc_numbering_is_ours_and_the_ietfs_is_another_standard() {
        let source = format!(
            "{DOC} the serializer RFC 0002 {SECTION}10 defines, resolved by `RFC 3986` \
             {SECTION}5.2.2\n"
        );
        let scan = scan(&source);
        assert!(scan.citations.is_empty(), "{:?}", scan.citations);
        assert_eq!(
            scan.sections
                .first()
                .and_then(|section| section.document.as_deref()),
            Some("RFC 0002")
        );
        assert_eq!(
            scan.foreign
                .first()
                .map(|foreign| foreign.document.as_str()),
            Some("RFC 3986"),
            "a wrapper is not distance: backticks round a standard's name still name it"
        );
    }

    /// A code span holding nothing but the sign names the character, and a sentence about the
    /// convention is not a malformed citation.
    ///
    /// Four `pdf-archive` module headers wrote "every `§` in this file is an ISO 32000-2 number"
    /// in the nine-hundred-and-seventy-seventh session, because their clause numbers resolve in
    /// two editions of the standard and the header has to say which one it means. Each was
    /// reported as a citation this checker could not read.
    #[test]
    fn a_code_span_holding_the_sign_is_the_character_and_not_a_citation() {
        let source = format!(
            "{DOC} Every `{SECTION}` here is an ISO 32000-2 number, and this walk serves\n\
             {DOC} see {SECTION} above\n"
        );
        let scan = scan(&source);
        assert!(scan.citations.is_empty(), "{:?}", scan.citations);
        assert_eq!(
            scan.malformed.len(),
            1,
            "the bare sign is still a finding: {:?}",
            scan.malformed
        );
    }

    /// A `doc/` path is one of this project's documents; a *file* under `doc/` that carries
    /// another extension is not a document with sections, and the tree's own counter-example is
    /// the ledger.
    #[test]
    fn a_doc_path_is_a_document_and_a_file_under_doc_is_not() {
        let source = format!(
            "{DOC} `doc/todo/58` {SECTION}5 owed this crate a perf floor\n\
             {DOC} doc/conformance/ledger.toml {SECTION}8.7.4.1 is the standard's clause\n"
        );
        let scan = scan(&source);
        assert_eq!(
            scan.sections
                .iter()
                .map(|section| section.document.as_deref())
                .collect::<Vec<Option<&str>>>(),
            vec![Some("doc/todo/58")]
        );
        assert_eq!(
            scan.citations
                .iter()
                .map(|citation| citation.number.to_string())
                .collect::<Vec<String>>(),
            vec!["8.7.4.1"]
        );
    }
}
