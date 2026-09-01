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
    /// The file carries no `N G obj` header for this object number at all.
    ///
    /// Only ever answered about an object some `/Kids` array *names*: §7.7.3.2 makes those
    /// references the tree, so a number named there and absent from the file is a page whose
    /// bytes are not in the document. Nothing can be recovered from it, and that is a
    /// different sentence from every other variant here, all of which are about bytes that
    /// are present and unreadable.
    Absent,
}

/// An object some `/Kids` array names and which does not resolve to a dictionary.
///
/// §7.7.3.2's `/Kids` is the page tree: "[t]he children shall only be page objects or other
/// page tree nodes". So an entry here is the file's own statement that this object is one of
/// those two, made in a place the object's own damage cannot reach — which is the question
/// [`Page`] cannot ask, because the recovery scan finds an object by its *own* `/Type /Page`
/// declaration and this population is exactly the objects that no longer make one.
struct Child {
    /// The object number the `/Kids` array names.
    number: u32,
    /// How far that object reads.
    reading: Reading,
    /// The keys of the entries read whole before the damage, in the order the file writes them.
    ///
    /// Printed rather than counted because what decides whether a prefix could honestly be
    /// taken for a page is *which* entries it holds: §7.7.3.4 makes `/Resources`, `/MediaBox`,
    /// `/CropBox` and `/Rotate` inheritable, so a page tree node states them legitimately and
    /// they discriminate nothing. Table 30's four entries and those four are the whole of what
    /// a node may say; anything else is Table 31's.
    keys: Vec<String>,
}

/// Table 31 entries that §7.7.3.4 does not make inheritable and Table 30 does not define.
///
/// A prefix holding one of these was written by a producer describing a *page*: a page tree
/// node has Table 30's four entries, and §7.7.3.4 adds the four inheritable ones to what it
/// may legitimately carry. These two are the ones the eleven documents of this population
/// actually witness; the list is short on purpose, because a name is only evidence here if the
/// standard puts it in one table and not the other.
const ONLY_A_PAGE_STATES: [&str; 2] = ["Contents", "Annots"];

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
    /// Objects a `/Kids` array names and which do not resolve to a dictionary.
    children: Vec<Child>,
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
            Reading::Whole | Reading::NoDictionary | Reading::GluedKeyword | Reading::Absent => {
                None
            }
        })
    }

    /// Whether some `/Kids` array names an object whose bytes are in the file and unreadable.
    fn names_a_damaged_child(&self) -> bool {
        self.children
            .iter()
            .any(|child| child.reading != Reading::Absent)
    }

    /// Whether such a child's prefix holds an entry only a page object states.
    fn a_childs_prefix_states_a_page_entry(&self) -> bool {
        self.children.iter().any(|child| {
            child
                .keys
                .iter()
                .any(|key| ONLY_A_PAGE_STATES.contains(&key.as_str()))
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
    /// Of the standing documents: those whose page tree names a child that does not resolve.
    names_an_unresolved_child: usize,
    /// Of those: the ones where every such child's bytes are absent from the file.
    every_child_absent: usize,
    /// Of those: the ones where such a child's prefix holds an entry only a page object states.
    childs_prefix_states_a_page_entry: usize,
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
        if !finding.children.is_empty() {
            self.names_an_unresolved_child = self.names_an_unresolved_child.saturating_add(1);
            if !finding.names_a_damaged_child() {
                self.every_child_absent = self.every_child_absent.saturating_add(1);
            }
            if finding.a_childs_prefix_states_a_page_entry() {
                self.childs_prefix_states_a_page_entry =
                    self.childs_prefix_states_a_page_entry.saturating_add(1);
            }
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
        println!(
            "  {} of the {} have a /Kids array naming an object that does not resolve: {} where \
             every such object's bytes are absent from the file, {} where such an object's \
             readable prefix holds an entry only a page object states ({})",
            self.names_an_unresolved_child,
            self.standing,
            self.every_child_absent,
            self.childs_prefix_states_a_page_entry,
            ONLY_A_PAGE_STATES.join(", ")
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
            causes.push(format!("{} {}", page.number, describe(page.reading)));
        }
        if causes.is_empty() {
            causes.push("no object declares a page".to_owned());
        }
        // The other half of the account, and the one a byte scan for `/Type /Page` cannot give:
        // what the page tree's own `/Kids` names, and how far each of those objects reads.
        for child in &finding.children {
            causes.push(format!(
                "/Kids names {} which {}{}",
                child.number,
                describe(child.reading),
                if child.keys.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", child.keys.join(" "))
                }
            ));
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

/// How far one object reads, in the words the per-document line uses.
fn describe(reading: Reading) -> String {
    match reading {
        Reading::Whole => "parses whole".to_owned(),
        Reading::GluedKeyword => "has a regular byte glued to its `obj` keyword".to_owned(),
        Reading::NoDictionary => "opens no dictionary".to_owned(),
        Reading::DamagedBody(entries) => format!("is damaged after {entries} entr(ies)"),
        Reading::Truncated(entries) => format!("is truncated after {entries} entr(ies)"),
        Reading::Absent => "is not in the file".to_owned(),
    }
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
        children: Vec::new(),
    };
    if !finding.standing() {
        return finding;
    }
    // Only now, and off the bytes: §7.5.4's table is the thing that may be broken, so the
    // objects are found the way `xref::rebuild` finds them — by their own headers, which §7.3.10
    // puts next to the bytes they describe.
    let bodies = object_bodies(&bytes);
    for &(number, body, glued) in &bodies {
        if let Some(page) = read_page(&bytes, number, body, glued) {
            finding.pages.push(page);
        }
    }
    for number in kids_named(&document) {
        // Only the ones that do not resolve: a child the reader can already read is not part of
        // the question, and where the tree yields no page there is at least one that cannot.
        if document
            .get(pdf_syntax::ObjectId {
                number,
                generation: 0,
            })
            .as_dict()
            .is_some()
        {
            continue;
        }
        let (reading, keys) = match bodies
            .iter()
            .find(|&&(candidate, _, _)| candidate == number)
        {
            Some(&(_, body, glued)) => read_entries(&bytes, body, glued),
            None => (Reading::Absent, Vec::new()),
        };
        finding.children.push(Child {
            number,
            reading,
            keys,
        });
    }
    finding
}

/// Every object number some `/Kids` array names, in the order the arrays state them.
///
/// Read through `Document` rather than off the bytes, and deliberately: this is the *tree*
/// asking, and a node that will not parse has stated nothing. Every document in this population
/// has a page tree node that reads whole and a child that does not, which is what makes the
/// question answerable at all.
fn kids_named(document: &Document) -> Vec<u32> {
    let mut named = Vec::new();
    for number in document.xref().object_numbers() {
        let object = document.get(pdf_syntax::ObjectId {
            number,
            generation: 0,
        });
        let Some(dict) = object.as_dict() else {
            continue;
        };
        let kids = document.get_key(dict, "Kids");
        let Some(kids) = kids.as_array() else {
            continue;
        };
        for kid in kids {
            if let pdf_syntax::Object::Reference(child) = kid
                && !named.contains(&child.number)
            {
                named.push(child.number);
            }
        }
    }
    named
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
    if glued || Lexer::at(bytes, body).next_token() != Some(Token::DictOpen) {
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

    let (reading, keys, declares) = read_body(bytes, body);
    declares.then_some(Page {
        number,
        reading,
        prefix_has_contents: keys.iter().any(|key| key == "Contents"),
        prefix_has_media_box: keys.iter().any(|key| key == "MediaBox"),
    })
}

/// How far the object body at `body` reads, and the keys of the entries read whole before that.
///
/// The two callers ask different questions of one reading, which is why it is one function:
/// [`read_page`] wants to know whether the prefix declares Table 31's `/Type /Page`, and the
/// `/Kids` walk wants to know what the prefix says about an object that declares nothing.
fn read_entries(bytes: &[u8], body: usize, glued: bool) -> (Reading, Vec<String>) {
    if glued {
        return (Reading::GluedKeyword, Vec::new());
    }
    if Lexer::at(bytes, body).next_token() != Some(Token::DictOpen) {
        return (Reading::NoDictionary, Vec::new());
    }
    let (reading, keys, _) = read_body(bytes, body);
    (reading, keys)
}

/// The shared reading of a dictionary body that has already opened.
fn read_body(bytes: &[u8], body: usize) -> (Reading, Vec<String>, bool) {
    let mut lexer = Lexer::at(bytes, body);
    lexer.next_token();
    let mut keys: Vec<String> = Vec::new();
    let mut declares = false;
    let reading = loop {
        match lexer.next_token() {
            None => break Reading::Truncated(keys.len()),
            Some(Token::DictClose) => break Reading::Whole,
            Some(Token::Name(key)) => {
                let mut parser = Parser::at(bytes, lexer.position(), Limits::DEFAULT);
                let Ok(value) = parser.parse_object() else {
                    break Reading::DamagedBody(keys.len());
                };
                lexer.seek(parser.position());
                if key.as_slice() == b"Type" {
                    declares = value
                        .as_name()
                        .is_some_and(|name| name.as_bytes() == b"Page");
                }
                keys.push(String::from_utf8_lossy(&key).into_owned());
            }
            // A non-name where a key belongs, which `Parser::parse_dictionary_body` skips too.
            Some(_) => {}
        }
    };
    (reading, keys, declares)
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
