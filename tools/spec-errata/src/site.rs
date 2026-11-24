//! The second input: the errata pdf-issues.pdfa.org publishes, read from the site's own source.
//!
//! # Why a second input
//!
//! [`crate::read`] reads the errata *inside* a sponsored PDF — Errata Collection 3's strikeouts
//! and carets in `ISO_32000-2_sponsored_EC3.pdf`. That is one standard's one collection. The PDF
//! Association publishes the corrections the working groups agreed for fifteen documents on
//! pdf-issues.pdfa.org, and `CLAUDE.md` principle 5 makes such a correction the standard and the
//! printed text retired (ADR 3581). ISO 19005-4's corrections exist nowhere else: no PDF this
//! tree holds carries them as annotations, so an instrument that reads only annotations cannot
//! see that `doc/pdfa/` prints "greater" where part 4 now says "less" (pdf-issues #77, ADR 3582).
//!
//! `doc/errata/site/` holds the site's Jekyll source (ADR 3583). Each page is HTML inside a
//! Markdown file: a heading per clause (`<h4 id="H6.2.10.3">`), a `<p class="location">` stating
//! the instruction ("Change the last bullet as follows:"), and the passage with the retired words
//! in `<del>` and the new ones in `<ins>`, each naming its issue in `data-issue` and ISO's own
//! approval in `data-iso`. This module turns each change into a [`Note`], so that `check` and
//! `moved` ask the same questions of both inputs with the same comparison (ADR 3589).
//!
//! # What a change retires
//!
//! **The site strikes words, not sentences**, and most strikes are one word — `greater`, the `s`
//! of `https`. A one-word passage is below [`crate::MIN_WORDS`] and would be compared against
//! nothing, and a one-word passage compared against everything would be a coincidence per page.
//! So what a change retires is the *printed* text at the change: the struck words with
//! [`CONTEXT_WORDS`] printed words on each side. A quotation of the printed sentence through that
//! point contains it; a quotation that only shares the clause around it does not.
//!
//! **An insertion retires printed text only where it lands inside it.** `pdfa<ins>id</ins>:` makes
//! the printed `pdfa:conformance` wrong, and a word inserted between two printed words changes
//! the sentence they are in; a sentence, a NOTE or a row added at a boundary leaves every printed
//! word as it was and retires nothing — it is still a note, counted, and a heading it inserts is
//! still a move for `moved`.
//!
//! # What this is not
//!
//! The same as the crate: never read by `tools/conformance`, never a gate, and its output is
//! about this tree's quotations. The site's text is CC-BY-4.0 (`doc/errata/README.md`), so unlike
//! [`crate::markdown`]'s output it may be printed.

use std::collections::BTreeSet;
use std::path::Path;

use crate::{Error, Note, Origin, Role};

/// How many printed words on each side of a change are taken with it as the retired passage.
///
/// Three, so that a one-word strike with its context is seven words: past [`crate::MIN_WORDS`],
/// and short enough that a quotation containing it must have passed through the changed point.
pub const CONTEXT_WORDS: usize = 3;

/// Where the document a note is from is named, ahead of the page's path under `docs/`.
pub const SITE: &str = "pdf-issues.pdfa.org";

/// What a run of text is to the erratum it sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    /// Printed and kept.
    Plain,
    /// Printed and struck: inside `<del>`.
    Struck,
    /// Not printed: inside `<ins>`.
    Inserted,
}

/// One run of text with its mark and, for a change, the issues and approval it names.
#[derive(Debug, Clone)]
struct Piece {
    text: String,
    mark: Mark,
    issues: Vec<u32>,
    approved: bool,
}

/// One paragraph, list item, table row, caption or heading, as the page's HTML delimits it.
#[derive(Debug, Default)]
struct Block {
    pieces: Vec<Piece>,
    line: usize,
}

impl Block {
    fn changes(&self) -> bool {
        self.pieces.iter().any(|piece| piece.mark != Mark::Plain)
    }

    /// The block as printed (`Inserted` dropped) and as corrected (`Struck` dropped).
    fn texts(&self) -> (String, String) {
        let mut printed = String::new();
        let mut corrected = String::new();
        for piece in &self.pieces {
            if piece.mark != Mark::Inserted {
                printed.push_str(&piece.text);
            }
            if piece.mark != Mark::Struck {
                corrected.push_str(&piece.text);
            }
        }
        (collapsed(&printed), collapsed(&corrected))
    }
}

/// One change inside a block: struck words, inserted words or both, at one place in the printed
/// text.
#[derive(Debug, Default)]
struct Change {
    start: usize,
    end: usize,
    struck: String,
    inserted: String,
    issues: BTreeSet<u32>,
    approved: bool,
}

/// What the page is in the middle of, as the parser walks it.
#[derive(Debug, Default)]
struct Walk {
    /// The open `<del>` and `<ins>` elements, innermost last.
    marks: Vec<(Mark, Vec<u32>, bool)>,
    block: Block,
    /// The printed heading of the clause the walk is in.
    section: Option<String>,
    /// The `<p class="location">` instruction the walk is under.
    instruction: Option<String>,
    in_heading: bool,
    in_location: bool,
    location: String,
    /// The struck words of each note pushed so far, in order, for [`moved_text`].
    struck: Vec<String>,
    /// Every inserted word of the page, for [`moved_text`].
    inserted: String,
}

/// Every change on every page of the site's source under `root` (`doc/errata/site`).
///
/// # Errors
///
/// [`Error::Unreadable`] where `root/docs` or a page under it cannot be read.
pub fn read(root: &Path) -> Result<Vec<Note>, Error> {
    let docs = root.join("docs");
    let mut notes = Vec::new();
    for directory in sorted_entries(&docs)? {
        let Some(standard) = directory
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        // `_data` and the like are the site's machinery, not a standard's pages.
        if standard.starts_with('_') || !directory.is_dir() {
            continue;
        }
        let mut standard_notes: Vec<(Note, String)> = Vec::new();
        let mut standard_inserted = String::new();
        for page in sorted_entries(&directory)? {
            if page.extension().is_none_or(|extension| extension != "md") {
                continue;
            }
            let text = std::fs::read_to_string(&page)
                .map_err(|error| Error::Unreadable(page.clone(), error))?;
            let name = page
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            let (page, struck, inserted) = page_notes(&format!("{SITE}/{standard}/{name}"), &text);
            standard_notes.extend(page.into_iter().zip(struck));
            standard_inserted.push_str(&inserted);
        }
        notes.extend(moved_text(standard_notes, &standard_inserted));
    }
    Ok(notes)
}

/// The notes with what a *move* strikes taken out of what they retire.
///
/// The site states a move as the passage struck at its old place and inserted at its new one
/// ("Move the last paragraph to below the heading …"), so the struck words are still the
/// standard's, one place along. A struck passage of [`crate::MIN_WORDS`] words or more that the
/// same standard's pages insert again retires nothing; a quotation of it is a quotation of
/// current text. Shorter strikes are not tested this way — "the" is inserted somewhere on every
/// page.
fn moved_text(notes: Vec<(Note, String)>, inserted: &str) -> Vec<Note> {
    let inserted = conformance::prose::folded(inserted);
    notes
        .into_iter()
        .map(|(mut note, struck)| {
            if struck.split_whitespace().count() >= crate::MIN_WORDS
                && inserted.contains(&conformance::prose::folded(&struck))
            {
                note.covered = None;
            }
            note
        })
        .collect()
}

/// Whether a note came from a page of the site for one of ISO 19005's parts.
#[must_use]
pub fn is_pdfa(note: &Note) -> bool {
    note.origin == Origin::Site && note.document.contains("/19005-")
}

/// The issue numbers a note's `/Subj` (or the site's `data-issue`) names.
#[must_use]
pub fn issues_of(note: &Note) -> BTreeSet<u32> {
    let Some(subject) = note.subject.as_deref() else {
        return BTreeSet::new();
    };
    subject
        .split('#')
        .skip(1)
        .filter_map(|tail| {
            let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect()
}

fn sorted_entries(directory: &Path) -> Result<Vec<std::path::PathBuf>, Error> {
    let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(directory)
        .map_err(|error| Error::Unreadable(directory.to_owned(), error))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    Ok(entries)
}

/// The notes of one page, from its source text.
fn page_notes(document: &str, text: &str) -> (Vec<Note>, Vec<String>, String) {
    let body = without_front_matter(text);
    let offset_lines = text[..text.len().saturating_sub(body.len())]
        .lines()
        .count();
    let mut walk = Walk::default();
    let mut notes = Vec::new();
    let mut line = offset_lines.saturating_add(1);
    let mut rest = body;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("{{").or_else(|| rest.strip_prefix("{%")) {
            // Liquid, which the site's build expands and which states nothing about the text.
            let close = after
                .find(['}'])
                .map_or(after.len(), |at| at.saturating_add(2));
            let skipped = &rest[..close.saturating_add(2).min(rest.len())];
            line = line.saturating_add(skipped.matches('\n').count());
            rest = &rest[skipped.len()..];
            continue;
        }
        if rest.starts_with('<') {
            let Some(close) = rest.find('>') else {
                break;
            };
            let tag = &rest[1..close];
            let consumed = close.saturating_add(1);
            line = line.saturating_add(tag.matches('\n').count());
            rest = &rest[consumed..];
            let name = tag_name(tag);
            if let Some(skip) = skipped_element(&name, tag) {
                // Navigation, the issue pop-up and scripts: none of it is the standard's text.
                let end = rest
                    .find(skip)
                    .map_or(rest.len(), |at| at.saturating_add(skip.len()));
                line = line.saturating_add(rest[..end].matches('\n').count());
                rest = &rest[end..];
                continue;
            }
            tag_event(&mut walk, &mut notes, document, &name, tag, line);
            continue;
        }
        let next = rest
            .find(['<', '{', '\n'])
            .map_or(rest.len(), |at| if at == 0 { 1 } else { at });
        let chunk = &rest[..next];
        if chunk == "\n" {
            line = line.saturating_add(1);
            // A blank line in Markdown ends a paragraph, as `</p>` does in HTML.
            if rest[1..].trim_start_matches([' ', '\t']).starts_with('\n') && walk.marks.is_empty()
            {
                flush(&mut walk, &mut notes, document);
            }
            push_text(&mut walk, " ", line);
        } else {
            push_text(&mut walk, &decoded(chunk), line);
        }
        rest = &rest[next..];
    }
    flush(&mut walk, &mut notes, document);
    (notes, walk.struck, walk.inserted)
}

fn without_front_matter(text: &str) -> &str {
    let Some(after) = text.strip_prefix("---\n") else {
        return text;
    };
    after
        .find("\n---\n")
        .map_or(text, |end| &after[end.saturating_add(5)..])
}

fn tag_name(tag: &str) -> String {
    tag.trim_start_matches('/')
        .split(|character: char| character.is_whitespace() || character == '/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// The closing text to skip to, for an element whose content is not the standard's.
fn skipped_element(name: &str, tag: &str) -> Option<&'static str> {
    if tag.starts_with('/') {
        return None;
    }
    match name {
        "nav" => Some("</nav>"),
        "script" => Some("</script>"),
        "style" => Some("</style>"),
        "div" if tag.contains("fixedpopup") => Some("</div>"),
        _ => None,
    }
}

/// The value of one attribute of a tag, where it is quoted with `"`.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let at = tag.find(&format!("{name}=\""))?;
    let value = &tag[at.saturating_add(name.len()).saturating_add(2)..];
    value.find('"').map(|end| &value[..end])
}

fn tag_event(
    walk: &mut Walk,
    notes: &mut Vec<Note>,
    document: &str,
    name: &str,
    tag: &str,
    line: usize,
) {
    let closing = tag.starts_with('/');
    match name {
        "del" | "ins" => {
            if closing {
                walk.marks.pop();
            } else {
                let issues = attribute(tag, "data-issue").map_or_else(Vec::new, |list| {
                    list.split(',')
                        .filter_map(|issue| issue.trim().parse().ok())
                        .collect()
                });
                let approved =
                    attribute(tag, "data-iso").is_some_and(|value| value.contains("approved"));
                let mark = if name == "del" {
                    Mark::Struck
                } else {
                    Mark::Inserted
                };
                walk.marks.push((mark, issues, approved));
            }
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            flush(walk, notes, document);
            walk.in_heading = !closing;
        }
        "p" if !closing && tag.contains("class=\"location\"") => {
            flush(walk, notes, document);
            walk.in_location = true;
            walk.location.clear();
        }
        "p" | "li" | "tr" | "caption" | "table" | "ul" | "ol" | "div" | "dt" | "dd" | "dl"
        | "pre" | "blockquote" | "hr" | "thead" | "tbody" | "figure" | "figcaption" => {
            if walk.in_location && closing && name == "p" {
                walk.instruction = Some(collapsed(&walk.location));
                walk.in_location = false;
                walk.block = Block::default();
                return;
            }
            flush(walk, notes, document);
        }
        "td" | "th" | "br" => push_text(walk, " ", line),
        _ => {}
    }
}

fn push_text(walk: &mut Walk, text: &str, line: usize) {
    if walk.in_location {
        walk.location.push_str(text);
        return;
    }
    let (mark, issues, approved) =
        walk.marks
            .last()
            .cloned()
            .unwrap_or((Mark::Plain, Vec::new(), false));
    if walk.block.pieces.is_empty() {
        if text.trim().is_empty() {
            return;
        }
        walk.block.line = line;
    }
    if let Some(last) = walk.block.pieces.last_mut()
        && last.mark == mark
        && last.issues == issues
    {
        last.text.push_str(text);
        return;
    }
    walk.block.pieces.push(Piece {
        text: text.to_owned(),
        mark,
        issues,
        approved,
    });
}

/// Ends the block the walk is in: a heading becomes the section, and a block holding a change
/// becomes notes.
fn flush(walk: &mut Walk, notes: &mut Vec<Note>, document: &str) {
    let block = std::mem::take(&mut walk.block);
    if block.pieces.is_empty() {
        return;
    }
    let (printed, corrected) = block.texts();
    if walk.in_heading {
        if block.changes()
            && let Some(note) = heading_note(walk, document, &block, &printed, &corrected)
        {
            notes.push(note);
            walk.struck.push(String::new());
        }
        let heading = if printed.trim().is_empty() {
            corrected
        } else {
            printed
        };
        walk.section = Some(heading.trim().to_owned());
        walk.instruction = None;
        return;
    }
    if !block.changes() {
        return;
    }
    for (change, retired) in changes_of(&block) {
        notes.push(change_note(walk, document, block.line, &change, &retired));
        walk.struck.push(change.struck.clone());
        walk.inserted.push(' ');
        walk.inserted.push_str(&change.inserted);
    }
}

/// A heading whose number an erratum moves, as the instruction `moved` reads.
fn heading_note(
    walk: &Walk,
    document: &str,
    block: &Block,
    printed: &str,
    corrected: &str,
) -> Option<Note> {
    let number = |text: &str| -> Option<String> {
        let token = text.split_whitespace().next()?;
        (token.contains('.') && token.chars().all(|c| c.is_ascii_digit() || c == '.'))
            .then(|| token.trim_end_matches('.').to_owned())
    };
    let contents = match (number(printed), number(corrected)) {
        (Some(was), Some(now)) if was == now => return None,
        (Some(_), Some(_)) => format!("Renumber the heading {printed} as {corrected}"),
        (Some(_), None) => format!("Delete the heading {printed}"),
        (None, Some(_)) => format!("Insert the heading {corrected}"),
        (None, None) => return None,
    };
    let mut issues = BTreeSet::new();
    let mut approved = false;
    for piece in block
        .pieces
        .iter()
        .filter(|piece| piece.mark != Mark::Plain)
    {
        issues.extend(piece.issues.iter().copied());
        approved |= piece.approved;
    }
    let change = Change {
        issues,
        approved,
        ..Change::default()
    };
    let mut note = change_note(walk, document, block.line, &change, "");
    note.contents = Some(contents);
    "heading".clone_into(&mut note.subtype);
    Some(note)
}

/// The changes of one block, each at its place in the printed text.
///
/// Struck and inserted runs with nothing printed between them but space are one change — a
/// replacement, `<del>greater</del> <ins>less</ins>` — so that it is reported once.
fn changes_of(block: &Block) -> Vec<(Change, String)> {
    let mut printed = String::new();
    let mut changes: Vec<Change> = Vec::new();
    let mut open = false;
    for piece in &block.pieces {
        let text = &piece.text;
        match piece.mark {
            Mark::Plain => {
                if !text.trim().is_empty() {
                    open = false;
                }
                printed.push_str(text);
            }
            Mark::Struck | Mark::Inserted => {
                if !open {
                    changes.push(Change {
                        start: printed.len(),
                        end: printed.len(),
                        ..Change::default()
                    });
                    open = true;
                }
                let Some(change) = changes.last_mut() else {
                    continue;
                };
                change.issues.extend(piece.issues.iter().copied());
                change.approved |= piece.approved;
                if piece.mark == Mark::Struck {
                    if change.struck.is_empty() {
                        change.start = printed.len();
                    }
                    printed.push_str(text);
                    change.end = printed.len();
                    change.struck.push_str(text);
                } else {
                    change.inserted.push_str(text);
                }
            }
        }
    }
    changes
        .into_iter()
        .map(|change| {
            let retired = retired_window(&printed, &change);
            (change, retired)
        })
        .collect()
}

/// The printed text a change retires, or nothing where it retires none (see the module).
fn retired_window(printed: &str, change: &Change) -> String {
    let before = &printed[..change.start];
    let after = &printed[change.end..];
    if change.struck.trim().is_empty() {
        let inside = !before.trim().is_empty() && !after.trim().is_empty();
        let at_a_sentence =
            before.ends_with(char::is_whitespace) && before.trim_end().ends_with(['.', ';', ':']);
        // Words added at the end of a clause, ahead of its closing mark, extend it: every
        // printed word is still there, in order (#271's "(null entries shall not be present)").
        let at_a_clause_end = {
            let mut rest = after.trim_start().chars();
            matches!(rest.next(), Some('.' | ';' | ':' | ',' | ')'))
                && rest.next().is_none_or(char::is_whitespace)
        };
        if !inside || at_a_sentence || at_a_clause_end {
            return String::new();
        }
    }
    let from = start_of_last_words(before, CONTEXT_WORDS);
    let to = end_of_first_words(after, CONTEXT_WORDS);
    collapsed(&format!(
        "{}{}{}",
        &before[from..],
        &printed[change.start..change.end],
        &after[..to]
    ))
}

/// The byte offset where the last `count` words of `text` begin, or 0 where it has fewer.
fn start_of_last_words(text: &str, count: usize) -> usize {
    let mut starts: Vec<usize> = Vec::new();
    let mut previous_space = true;
    for (at, character) in text.char_indices() {
        if !character.is_whitespace() && previous_space {
            starts.push(at);
        }
        previous_space = character.is_whitespace();
    }
    starts
        .len()
        .checked_sub(count)
        .and_then(|index| starts.get(index).copied())
        .unwrap_or(0)
}

/// The byte offset where the first `count` words of `text` end, or its length where it has fewer.
fn end_of_first_words(text: &str, count: usize) -> usize {
    let mut seen = 0_usize;
    let mut in_word = false;
    for (at, character) in text.char_indices() {
        if character.is_whitespace() {
            if in_word {
                seen = seen.saturating_add(1);
                if seen == count {
                    return at;
                }
            }
            in_word = false;
        } else {
            in_word = true;
        }
    }
    text.len()
}

fn change_note(walk: &Walk, document: &str, line: usize, inner: &Change, retired: &str) -> Note {
    let subtype = match (
        inner.struck.trim().is_empty(),
        inner.inserted.trim().is_empty(),
    ) {
        (false, false) => "del+ins",
        (false, true) => "del",
        _ => "ins",
    };
    let mut states = BTreeSet::from(["site/published".to_owned()]);
    if inner.approved {
        states.insert("site/iso-approved".to_owned());
    }
    Note {
        document: document.to_owned(),
        page: line,
        section: walk.section.clone(),
        subject: (!inner.issues.is_empty()).then(|| {
            inner
                .issues
                .iter()
                .map(|issue| format!("Issue #{issue}"))
                .collect::<Vec<String>>()
                .join(", ")
        }),
        title: None,
        subtype: subtype.to_owned(),
        role: Role::Primary,
        change: None,
        contents: walk.instruction.clone(),
        covered: (!retired.is_empty()).then(|| retired.to_owned()),
        created: None,
        states,
        origin: Origin::Site,
    }
}

/// Runs of whitespace as one space, and none at either end.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// HTML's character references, as far as the site's pages use them.
fn decoded(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let Some(end) = tail.find(';').filter(|end| *end <= 10) else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let name = &tail[1..end];
        let character = match name {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            "Dagger" => Some('‡'),
            "trade" => Some('™'),
            "reg" => Some('®'),
            "alpha" => Some('α'),
            "times" => Some('×'),
            "permil" => Some('‰'),
            "minus" => Some('−'),
            "le" => Some('≤'),
            "ge" => Some('≥'),
            _ => name.strip_prefix('#').and_then(|number| {
                let value = match number.strip_prefix(['x', 'X']) {
                    Some(hex) => u32::from_str_radix(hex, 16).ok(),
                    None => number.parse().ok(),
                };
                value.and_then(char::from_u32)
            }),
        };
        if let Some(character) = character {
            out.push(character);
        } else {
            out.push_str(&tail[..=end]);
        }
        rest = &tail[end.saturating_add(1)..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "---\ntitle: x\n---\n\n<nav><ul><li>6.2</li></ul></nav>\n\
        <div class=\"fixedpopup\" id=\"issuelink\">Issue #xxxx</div>\n\
        <h5 id=\"H6.2.10.3.1\">6.2.10.3.1 General</h5>\n\
        <p class=\"location\">Change the last bullet as follows:</p>\n\
        <ul><li>the value of the <b>Supplement</b> key in the CIDSystemInfo dictionary of the \
        CIDFont shall be <del data-issue=\"77\">greater</del> <ins data-issue=\"77\">less</ins> \
        than or equal to the value</li></ul>\n\
        <h3 id=\"H6.7.3\">6.7.3 Version identification</h3>\n\
        <p class=\"location\">Change Table 2 as follows:</p>\n\
        <p>uses the namespace URI http<del data-issue=\"123\" data-iso=\"approved\">s</del>://www.aiim.org/pdfa/ns/id/. The required schema</p>\n\
        <p>NOTE 1 text. <ins data-issue=\"104\" data-iso=\"approved\">Note 2 Page transparency.</ins></p>\n\
        <h5 id=\"H14.8.4.7.4\">14.8.4.7.<del data-issue=\"133\">3</del><ins data-issue=\"133\">4</ins> Ruby</h5>\n";

    fn notes() -> Vec<Note> {
        page_notes("pdf-issues.pdfa.org/19005-4-2020/clause06.md", PAGE).0
    }

    /// #77's replacement is one change, under its clause and instruction, and what it retires
    /// is the printed sentence through "greater", long enough to compare.
    #[test]
    fn a_replacement_retires_the_printed_words_around_it() {
        let notes = notes();
        let supplement = notes
            .iter()
            .find(|note| issues_of(note).contains(&77))
            .expect("#77 is on the page");
        assert_eq!(supplement.subtype, "del+ins");
        assert_eq!(supplement.section.as_deref(), Some("6.2.10.3.1 General"));
        assert_eq!(
            supplement.contents.as_deref(),
            Some("Change the last bullet as follows:")
        );
        assert_eq!(
            supplement.covered.as_deref(),
            Some("CIDFont shall be greater than or equal")
        );
        assert!(supplement.retires_text());
        assert_eq!(
            notes
                .iter()
                .filter(|note| issues_of(note).contains(&77))
                .count(),
            1
        );
    }

    /// A strike inside a word takes the word with it: the printed `https` is what is retired.
    #[test]
    fn a_strike_inside_a_word_retires_the_word() {
        let notes = notes();
        let scheme = notes
            .iter()
            .find(|note| issues_of(note).contains(&123))
            .expect("#123 is on the page");
        assert_eq!(
            scheme.covered.as_deref(),
            Some("namespace URI https://www.aiim.org/pdfa/ns/id/. The required")
        );
        assert!(scheme.states.contains("site/iso-approved"));
    }

    /// Words added before a sentence's full stop extend it and retire nothing.
    #[test]
    fn an_insertion_before_the_full_stop_retires_nothing() {
        let page = "<p>The children shall only be page objects or other page tree nodes \
            <ins data-issue=\"271\">(null entries shall not be present)</ins>. And more.</p>\n";
        let (notes, _, _) = page_notes("x", page);
        assert_eq!(notes.len(), 1);
        assert!(!notes[0].retires_text());
    }

    /// A NOTE added after a sentence leaves every printed word as it was.
    #[test]
    fn an_insertion_after_a_sentence_retires_nothing() {
        let notes = notes();
        let note = notes
            .iter()
            .find(|note| issues_of(note).contains(&104))
            .expect("#104 is on the page");
        assert_eq!(note.subtype, "ins");
        assert!(!note.retires_text());
    }

    /// A heading whose number changes is a renumbering `moved` reads, with both numbers.
    #[test]
    fn a_renumbered_heading_is_structural() {
        let notes = notes();
        let heading = notes
            .iter()
            .find(|note| note.subtype == "heading")
            .expect("the heading is renumbered");
        let structural = crate::structural(std::slice::from_ref(heading));
        assert_eq!(structural.len(), 1);
        assert_eq!(structural[0].verbs, vec!["renumber"]);
        let named: Vec<String> = structural[0]
            .clauses
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(named, vec!["14.8.4.7.3", "14.8.4.7.4"]);
    }

    /// A paragraph struck at one place and inserted at another is moved, and still current.
    #[test]
    fn a_moved_passage_retires_nothing() {
        let page = "<p class=\"location\">Move the last paragraph as follows:</p>\n\
            <p><del data-issue=\"1\">All resources that are referenced shall be inherited.</del></p>\n\
            <p>Earlier text. <ins data-issue=\"1\">All resources that are referenced shall be \
            inherited.</ins></p>\n\
            <p>A sentence <del data-issue=\"2\">that this erratum struck out</del> here.</p>\n";
        let (notes, struck, inserted) = page_notes("x", page);
        assert_eq!(notes.len(), 3);
        let kept = moved_text(notes.into_iter().zip(struck).collect(), &inserted);
        let retiring: Vec<String> = kept
            .iter()
            .filter(|note| note.retires_text())
            .filter_map(|note| note.subject.clone())
            .collect();
        assert_eq!(retiring, vec!["Issue #2".to_owned()]);
    }

    #[test]
    fn character_references_are_decoded() {
        assert_eq!(
            decoded("a &lt;b&gt; &#x41;&amp;&nbsp;c &bogus; &"),
            "a <b> A& c &bogus; &"
        );
    }

    /// The held source reads, and #77's correction is on the PDF/A-4 page.
    #[test]
    fn the_held_site_reads() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/errata/site");
        let notes = read(&root).expect("the site's source is held in the repository");
        assert!(
            notes
                .iter()
                .any(|note| is_pdfa(note) && issues_of(note).contains(&77))
        );
        assert!(notes.iter().filter(|note| note.retires_text()).count() > 100);
    }
}
