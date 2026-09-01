//! How many documents state a page count this reader cannot produce a page for, and why.
//!
//! ISO 32000-2 §7.7.3.2, Table 30's `/Count` cell, makes the entry redundant and the `Kids`
//! arrays "which definitively determines the number of descendant pages" — so where a tree
//! yields no page at all, `pdf_model::Pages` scans the file for Table 31's `/Type /Page`
//! declaration and, finding none, leaves `/Count` standing as the one statement anybody has
//! made about the number (ADR 0782). That state — **a positive `len()` over a `get(0)` of
//! `None`** — is a document this program opens, counts pages for, and cannot show. This census
//! is its population and its causes.
//!
//! **The predicate is deliberately this reader's**, which is the one shape of census
//! `doc/traps/parsers-and-streams.md` trap 8 permits: the question is not what the standard
//! says but what share of the files that exist this program fails on, and that is a question
//! only the program can answer. **The classification underneath it is not**: every cause below
//! is read off the file's own bytes with [`pdf_syntax::Lexer`] and [`pdf_syntax::Parser`],
//! never through `Pages`, so a change to the recovery moves the first column and leaves the
//! rest of the row where it was.
//!
//! The causes, which are the reason for the run:
//!
//! - **no object declares a page at all** — the file states a count over objects that are not
//!   in it, and nothing here can be recovered from;
//! - **an object's bytes declare a page and the object does not parse**, split by *where* the
//!   parse stops: before the dictionary opens (`obj` is followed by something that is not
//!   `<<`, so there is no dictionary to take a prefix of) or inside the body (some number of
//!   entries were read before the damage, which is the prefix a recovery could take);
//! - **an object parses to a page dictionary** and the tree still yields nothing, which would
//!   be a defect of the scan rather than of the file.
//!
//! ```sh
//! cargo run --profile gates -p pdf-model --example standing_count_census -- <file-or-directory>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::path::{Path, PathBuf};

use rayon::prelude::{IntoParallelRefIterator as _, ParallelIterator as _};

use pdf_syntax::{Document, Lexer, Limits, Parser, Token};

/// Where reading one object stated to be a page stops.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reading {
    /// The object parses, and what it parses to is a dictionary.
    Whole,
    /// The `obj` keyword does not stand alone: a regular byte is glued to it.
    ///
    /// §7.2.3's regular characters run up to the next delimiter or white-space, so
    /// `obj\xbc<<` lexes as one keyword `obj\xbc` and the header is not a header at all.
    /// Counted separately because it is the one cause where the damage is *outside* the
    /// object's value, so no prefix of the value exists to be asked about.
    GluedKeyword,
    /// `obj` is followed by something that is not `<<`: no dictionary opened.
    NoDictionary,
    /// The dictionary opened and a value inside it could not be read.
    ///
    /// The number is how many entries were complete before that, which is the prefix
    /// §7.3.7's "sequence of key-value pairs enclosed in double angle brackets" offers.
    DamagedBody(usize),
    /// The dictionary opened and the input ended before `>>`.
    Truncated(usize),
}

/// What one object stated to be a page carries, and how far it reads.
struct Page {
    /// This object's number, for the per-document line.
    number: u32,
    /// How far the object reads.
    reading: Reading,
    /// Whether the entries read before the damage include `/Contents`.
    ///
    /// The entry that decides whether a recovered prefix would draw anything: Table 31 makes
    /// `/Contents` optional, so a prefix without it is a page and an empty one.
    prefix_has_contents: bool,
    /// Whether they include `/MediaBox`, which §7.7.3.4 also lets an ancestor state.
    prefix_has_media_box: bool,
}

/// What one document says about the question.
#[derive(Default)]
struct Finding {
    /// The document opened at all.
    opened: bool,
    /// What `Pages::len()` answers.
    declared: usize,
    /// Whether `Pages::get(0)` answers a page.
    has_first_page: bool,
    /// Objects whose bytes declare Table 31's `/Type /Page`, whether or not they parse.
    pages: Vec<Page>,
}

impl Finding {
    /// The population this census is about: a count standing over no page at all.
    fn standing(&self) -> bool {
        self.opened && self.declared > 0 && !self.has_first_page
    }

    /// Objects declaring a page whose reading is `kind`.
    fn count(&self, kind: Reading) -> usize {
        self.pages
            .iter()
            .filter(|page| page.reading == kind)
            .count()
    }

    /// Objects declaring a page whose dictionary opened and then failed.
    fn damaged(&self) -> impl Iterator<Item = (&Page, usize)> {
        self.pages.iter().filter_map(|page| match page.reading {
            Reading::DamagedBody(entries) | Reading::Truncated(entries) => Some((page, entries)),
            Reading::Whole | Reading::NoDictionary | Reading::GluedKeyword => None,
        })
    }
}

/// Every finding added up, which is what the run prints.
#[derive(Default)]
struct Totals {
    /// Files read off the disk, and those that opened.
    read: usize,
    /// As above.
    opened: usize,
    /// Documents whose `/Count` stands over no first page.
    standing: usize,
    /// Of those: documents where no object's bytes declare a page at all.
    nothing_declared: usize,
    /// Of those: documents where an object declares a page and parses whole.
    parses_whole: usize,
    /// Of those: documents where an object declares a page and no dictionary opens.
    no_dictionary: usize,
    /// Of those: documents where a page object's `obj` keyword has a regular byte glued to it.
    glued_keyword: usize,
    /// Of those: documents where a page's dictionary opens and a value inside it fails.
    damaged_body: usize,
    /// Damaged page dictionaries in all, and the entries their prefixes carry.
    damaged_objects: usize,
    /// As above.
    prefix_entries: usize,
    /// Damaged page dictionaries whose prefix already carries `/Contents`.
    prefix_with_contents: usize,
    /// Damaged page dictionaries whose prefix already carries `/MediaBox`.
    prefix_with_media_box: usize,
    /// Damaged page dictionaries whose prefix carries no entry at all.
    empty_prefix: usize,
}

impl Totals {
    /// Adds one document's finding.
    fn add(&mut self, finding: &Finding) {
        self.read = self.read.saturating_add(1);
        if !finding.opened {
            return;
        }
        self.opened = self.opened.saturating_add(1);
        if !finding.standing() {
            return;
        }
        self.standing = self.standing.saturating_add(1);
        if finding.pages.is_empty() {
            self.nothing_declared = self.nothing_declared.saturating_add(1);
        }
        if finding.count(Reading::Whole) > 0 {
            self.parses_whole = self.parses_whole.saturating_add(1);
        }
        if finding.count(Reading::NoDictionary) > 0 {
            self.no_dictionary = self.no_dictionary.saturating_add(1);
        }
        if finding.count(Reading::GluedKeyword) > 0 {
            self.glued_keyword = self.glued_keyword.saturating_add(1);
        }
        let mut damaged = 0_usize;
        for (page, entries) in finding.damaged() {
            damaged = damaged.saturating_add(1);
            self.damaged_objects = self.damaged_objects.saturating_add(1);
            self.prefix_entries = self.prefix_entries.saturating_add(entries);
            if entries == 0 {
                self.empty_prefix = self.empty_prefix.saturating_add(1);
            }
            if page.prefix_has_contents {
                self.prefix_with_contents = self.prefix_with_contents.saturating_add(1);
            }
            if page.prefix_has_media_box {
                self.prefix_with_media_box = self.prefix_with_media_box.saturating_add(1);
            }
        }
        if damaged > 0 {
            self.damaged_body = self.damaged_body.saturating_add(1);
        }
    }

    /// Prints the summary, one claim per line.
    fn print(&self) {
        println!(
            "standing-count census: {} document(s) read, {} opened, {} state a page count with no \
             first page",
            self.read, self.opened, self.standing
        );
        println!(
            "  of those {}: {} have no object whose bytes declare /Type /Page, {} have one that \
             parses whole, {} have one whose `obj` keyword has a regular byte glued to it, {} \
             have one whose `obj` is not followed by `<<`, {} have one whose dictionary opens \
             and then fails",
            self.standing,
            self.nothing_declared,
            self.parses_whole,
            self.glued_keyword,
            self.no_dictionary,
            self.damaged_body
        );
        println!(
            "  {} damaged page dictionar(ies) in all, carrying {} complete entr(ies) before the \
             damage ({} of them none at all); {} already carry /Contents and {} /MediaBox",
            self.damaged_objects,
            self.prefix_entries,
            self.empty_prefix,
            self.prefix_with_contents,
            self.prefix_with_media_box
        );
    }
}

fn main() {
    let files = collect(std::env::args().skip(1).map(PathBuf::from));
    let findings: Vec<(PathBuf, Finding)> = files
        .par_iter()
        .map(|path| (path.clone(), examine(path)))
        .collect();

    let mut totals = Totals::default();
    let mut lines: Vec<String> = Vec::new();
    for (path, finding) in &findings {
        totals.add(finding);
        if !finding.standing() {
            continue;
        }
        let mut causes: Vec<String> = Vec::new();
        for page in &finding.pages {
            causes.push(match page.reading {
                Reading::Whole => format!("{} parses whole", page.number),
                Reading::GluedKeyword => format!("{}'s `obj` keyword is glued", page.number),
                Reading::NoDictionary => format!("{} opens no dictionary", page.number),
                Reading::DamagedBody(entries) => {
                    format!("{} damaged after {entries} entr(ies)", page.number)
                }
                Reading::Truncated(entries) => {
                    format!("{} truncated after {entries} entr(ies)", page.number)
                }
            });
        }
        if causes.is_empty() {
            causes.push("no object declares a page".to_owned());
        }
        lines.push(format!(
            "  {}: /Count {} and no first page — {}",
            path.display(),
            finding.declared,
            causes.join(", ")
        ));
    }

    lines.sort();
    for line in &lines {
        println!("{line}");
    }
    totals.print();
}

/// Reads one document and answers the census's questions about it.
fn examine(path: &Path) -> Finding {
    let Ok(bytes) = std::fs::read(path) else {
        return Finding::default();
    };
    let Ok(document) = Document::open(bytes.clone()) else {
        return Finding::default();
    };
    let pages = pdf_model::Pages::new(&document);
    let mut finding = Finding {
        opened: true,
        declared: pages.len(),
        has_first_page: pages.get(0).is_some(),
        pages: Vec::new(),
    };
    if !finding.standing() {
        return finding;
    }
    // Only now, and off the bytes: §7.5.4's table is the thing that may be broken, so the
    // objects are found the way `xref::rebuild` finds them — by their own headers, which §7.3.10
    // puts next to the bytes they describe.
    for (number, body, glued) in object_bodies(&bytes) {
        if let Some(page) = read_page(&bytes, number, body, glued) {
            finding.pages.push(page);
        }
    }
    finding
}

/// Every `N G obj` header in the file: its number, the offset just past `obj`, and whether a
/// regular byte is glued to the keyword.
///
/// A byte scan rather than a table read, for the reason above. `Parser::parse_indirect_object`
/// is not used because it parses the *object*, and what is wanted here is the offset before it —
/// and because it rejects the glued keyword outright, which is one of the causes being counted.
fn object_bodies(bytes: &[u8]) -> Vec<(u32, usize, bool)> {
    let mut found = Vec::new();
    let mut at = 0usize;
    while let Some(hit) = bytes.get(at..).and_then(|rest| find(rest, b"obj")) {
        let keyword = at.saturating_add(hit);
        at = keyword.saturating_add(3);
        // `obj` has to be a keyword of its own, so what precedes it is white-space and what
        // follows is not a regular character — `endobj` and a name ending in `obj` are both
        // things a file contains.
        if bytes
            .get(keyword.saturating_sub(1))
            .is_none_or(|&byte| !pdf_syntax::lexer::is_whitespace(byte))
        {
            continue;
        }
        let mut header = Lexer::at(bytes, header_start(bytes, keyword));
        let (Some(Token::Integer(number)), Some(Token::Integer(_)), Some(Token::Keyword(word))) = (
            header.next_token(),
            header.next_token(),
            header.next_token(),
        ) else {
            continue;
        };
        if !word.starts_with(b"obj") {
            continue;
        }
        if let Ok(number) = u32::try_from(number) {
            // Past `obj` rather than past the keyword the lexer read, so that a glued byte is
            // the first thing the body's reader sees rather than something already swallowed.
            found.push((number, keyword.saturating_add(3), word.len() != 3));
        }
    }
    found
}

/// Where to start lexing so that the two integers before `obj` are the next two tokens.
///
/// Back over the white-space, the generation's digits, the white-space and the number's digits;
/// anything else stops the walk and the caller's lexer then fails to see a header, which is the
/// right answer for `obj` appearing anywhere else.
fn header_start(bytes: &[u8], keyword: usize) -> usize {
    let mut at = keyword;
    for expect_digits in [false, true, false, true] {
        while at > 0
            && bytes.get(at.saturating_sub(1)).is_some_and(|&byte| {
                if expect_digits {
                    byte.is_ascii_digit()
                } else {
                    pdf_syntax::lexer::is_whitespace(byte)
                }
            })
        {
            at = at.saturating_sub(1);
        }
    }
    at
}

/// Reads the object body at `body`, answering only where it declares Table 31's `/Type /Page`.
///
/// The declaration is looked for in the *entries this reads*, so an object whose damage falls
/// before its `/Type` is not counted — which is deliberate and is the conservative direction:
/// a census of what a prefix recovery could reach must not count an object whose prefix does
/// not say it is a page.
fn read_page(bytes: &[u8], number: u32, body: usize, glued: bool) -> Option<Page> {
    let mut lexer = Lexer::at(bytes, body);
    if glued || lexer.next_token() != Some(Token::DictOpen) {
        // Before deciding there is no dictionary, ask whether this object claims to be a page
        // at all — otherwise every stream and every array in the file would answer here.
        let rest = bytes.get(body..)?;
        let extent = rest.get(..2048).unwrap_or(rest);
        return declares_a_page_in_bytes(extent).then_some(Page {
            number,
            reading: if glued {
                Reading::GluedKeyword
            } else {
                Reading::NoDictionary
            },
            prefix_has_contents: false,
            prefix_has_media_box: false,
        });
    }

    let mut entries = 0usize;
    let mut is_page = false;
    let mut has_contents = false;
    let mut has_media_box = false;
    let reading = loop {
        match lexer.next_token() {
            None => break Reading::Truncated(entries),
            Some(Token::DictClose) => break Reading::Whole,
            Some(Token::Name(key)) => {
                let mut parser = Parser::at(bytes, lexer.position(), Limits::DEFAULT);
                let Ok(value) = parser.parse_object() else {
                    break Reading::DamagedBody(entries);
                };
                lexer.seek(parser.position());
                entries = entries.saturating_add(1);
                match key.as_slice() {
                    b"Type" => {
                        is_page = value
                            .as_name()
                            .is_some_and(|name| name.as_bytes() == b"Page");
                    }
                    b"Contents" => has_contents = true,
                    b"MediaBox" => has_media_box = true,
                    _ => {}
                }
            }
            // A non-name where a key belongs, which `Parser::parse_dictionary_body` skips too.
            Some(_) => {}
        }
    };
    is_page.then_some(Page {
        number,
        reading,
        prefix_has_contents: has_contents,
        prefix_has_media_box: has_media_box,
    })
}

/// Whether a run of bytes states `/Type` and `/Page` close together.
///
/// Only for the object whose dictionary never opened, where there are no entries to ask. It is
/// a heuristic and is labelled as one: it decides which objects a *cause* is printed for, never
/// what the recovery does.
fn declares_a_page_in_bytes(extent: &[u8]) -> bool {
    let Some(at) = find(extent, b"/Type") else {
        return false;
    };
    let after = extent.get(at.saturating_add(5)..).unwrap_or_default();
    let after = after.get(..32).unwrap_or(after);
    find(after, b"/Page").is_some_and(|hit| {
        after
            .get(hit.saturating_add(5))
            .is_none_or(|&byte| !byte.is_ascii_alphanumeric())
    })
}

/// The first offset of `needle` in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Every file named, and every `.pdf` under every directory named.
fn collect(paths: impl Iterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for path in paths {
        if path.is_dir() {
            walk(&path, &mut found);
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Adds every `.pdf` under `directory`, depth first.
fn walk(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, found);
        } else if path.extension().is_some_and(|kind| kind == "pdf") {
            found.push(path);
        }
    }
}
