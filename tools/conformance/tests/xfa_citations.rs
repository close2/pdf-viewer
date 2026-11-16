//! Every citation of XFA 3.3 names a page the text has, under the heading it says, and none quotes
//! it.
//!
//! Not a conformance question of ISO 32000-2; it lives here for `state_sections.rs`'s reason —
//! this is the crate whose gates read the repository's own files.
//!
//! # Why the checker reads XFA 3.3
//!
//! §K.2 names XFA 3.3 as the format of an interactive form's XFA resource, and this tree cites it
//! by page hundreds of times: the static half's behaviours, `FormCalc`, rich text, and the barcode
//! symbologies `crates/xfa-barcode` encodes. `doc/questions/A840` put the text into
//! `doc/specifications.zip` with the first test that derives an expected value from it, and asked
//! that the conformance checker read it. It cannot read it the way it reads ISO 32000-2, by
//! clause and quotation, for two reasons the text itself gives:
//!
//! - **It has no numbered clauses.** The PDF carries no structure tree, so its conversion
//!   (`doc/md/XFA-3_3.md`) is in content order with one `<!-- page N (N) -->` marker per page and
//!   the running head of each page — the chapter and the section title — left in the text. A
//!   citation is therefore *chapter, section title, page* (RFC 0011 section 1), and the page is
//!   the part that can be checked.
//! - **It is never quoted.** Its notice grants no right to reproduce its prose, so it is held as
//!   XFDF 3.0 is: cited and paraphrased, never with a `§` and never in a `>` blockquote
//!   (`doc/third-party-data.md`, ADR 0187).
//!
//! So three things are checked, each against the conversion: every `XFA 3.3 … page N` (or
//! `pages N–M`) in a comment names a page the text has; where the citation names a section title in
//! italics before the page, that title's words are on a cited page or on the page before the first
//! — a section that began a page earlier and runs on is cited by the pages the rule is on, as
//! `pdf-formcalc`'s *Simple Expressions*, pages 1061–1062, is; and no blockquote follows an
//! attribution to XFA 3.3. ADR 2877.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              the gate prints the population it judged, which is what makes its verdict readable"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The conversion of XFA 3.3 the checker reads, unpacked from `doc/specifications.zip`.
const XFA: &str = "doc/md/XFA-3_3.md";

/// How far after `XFA 3.3`, in bytes, a page number is still that citation's page: a chapter and
/// a section title fit, and a page named two sentences later does not.
const REACH: usize = 160;

/// The text of each page of XFA 3.3, keyed by its printed folio.
struct Pages(BTreeMap<u32, String>);

impl Pages {
    fn read(root: &Path) -> Self {
        let text = fs::read_to_string(root.join(XFA)).expect(
            "doc/md/XFA-3_3.md is in doc/specifications.zip; unpack it as doc/environment.md says",
        );
        Self::parse(&text)
    }

    fn parse(text: &str) -> Self {
        let mut pages = BTreeMap::new();
        let mut current: Option<(u32, String)> = None;
        for line in text.lines() {
            if let Some(folio) = folio(line) {
                if let Some((number, body)) = current.take() {
                    pages.insert(number, body);
                }
                current = Some((folio, String::new()));
            } else if let Some((_, body)) = current.as_mut() {
                body.push_str(line);
                body.push('\n');
            }
        }
        if let Some((number, body)) = current {
            pages.insert(number, body);
        }
        Self(pages)
    }

    fn holds_words(&self, page: u32, title: &str) -> bool {
        let Some(body) = self.0.get(&page) else {
            return false;
        };
        let body = words(body);
        words(title).iter().all(|word| body.contains(word))
    }
}

/// The printed folio a page marker names: `<!-- page 440 (440) -->` is page 440.
fn folio(line: &str) -> Option<u32> {
    let inner = line.strip_prefix("<!-- page ")?.strip_suffix(" -->")?;
    let printed = inner.split_once('(')?.1.strip_suffix(')')?;
    printed.trim().parse().ok()
}

/// The lower-case words of a text, for comparing a title with a page whatever its typography.
fn words(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// One citation of XFA 3.3 by page, as a comment writes it.
#[derive(Debug, PartialEq, Eq)]
struct Citation {
    /// The pages it names: one, or the two ends of a range.
    pages: Vec<u32>,
    /// The section title it names in italics before the page, if any.
    title: Option<String>,
}

/// The citations of XFA 3.3 by page in one comment paragraph.
fn citations(paragraph: &str) -> Vec<Citation> {
    let mut found = Vec::new();
    for (start, _) in paragraph.match_indices("XFA 3.3") {
        let after = paragraph.get(start.saturating_add(7)..).unwrap_or_default();
        let window = after
            .char_indices()
            .take_while(|(index, _)| *index < REACH)
            .map(|(_, character)| character)
            .collect::<String>();
        let window = window.split("XFA 3.3").next().unwrap_or_default();
        let Some(at) = page_word(window) else {
            continue;
        };
        let (before, rest) = window.split_at(at);
        let pages = numbers(rest);
        if pages.is_empty() {
            continue;
        }
        found.push(Citation {
            pages,
            title: italic(before),
        });
    }
    found
}

/// Where `page ` or `pages ` followed by a digit starts in a window, if it does.
fn page_word(window: &str) -> Option<usize> {
    let mut from = 0usize;
    while let Some(offset) = window.get(from..)?.find("page") {
        let at = from.saturating_add(offset);
        let rest = window.get(at..)?;
        let rest = rest
            .strip_prefix("pages ")
            .or_else(|| rest.strip_prefix("page "));
        if rest.is_some_and(|rest| rest.starts_with(|character: char| character.is_ascii_digit())) {
            return Some(at);
        }
        from = at.saturating_add(4);
    }
    None
}

/// The page numbers at the head of `page N`, `pages N–M` or `pages N-M`.
fn numbers(text: &str) -> Vec<u32> {
    let rest = text
        .strip_prefix("pages ")
        .or_else(|| text.strip_prefix("page "))
        .unwrap_or(text);
    let first: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let Ok(first) = first.parse::<u32>() else {
        return Vec::new();
    };
    let mut pages = vec![first];
    let tail = rest.trim_start_matches(|character: char| character.is_ascii_digit());
    let tail = tail
        .strip_prefix('–')
        .or_else(|| tail.strip_prefix('-'))
        .or_else(|| tail.strip_prefix(" to "));
    if let Some(tail) = tail {
        let second: String = tail.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(second) = second.parse::<u32>() {
            pages.push(second);
        }
    }
    pages
}

/// The last single-asterisk italic span in a text: `chapter 12, *Framing*, ` names `Framing`.
fn italic(text: &str) -> Option<String> {
    let mut spans = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('*') {
        let after = rest.get(open.saturating_add(1)..)?;
        if after.starts_with('*') {
            rest = after.trim_start_matches('*');
            continue;
        }
        let close = after.find('*')?;
        spans.push(after.get(..close)?.to_owned());
        rest = after.get(close.saturating_add(1)..)?;
    }
    spans.pop().filter(|span| !span.trim().is_empty())
}

/// The paragraphs of comment text in one source, each with the line it starts on: a run of comment
/// lines, ended by a line that is not one or by a comment line with nothing on it.
fn paragraphs(source: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for (number, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("//") {
            let rest = rest
                .strip_prefix('/')
                .or_else(|| rest.strip_prefix('!'))
                .unwrap_or(rest)
                .trim();
            if rest.is_empty() {
                found.extend(current.take());
                continue;
            }
            let (_, text) =
                current.get_or_insert_with(|| (number.saturating_add(1), String::new()));
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(rest);
        } else if let Some(done) = current.take() {
            found.push(done);
        }
    }
    found.extend(current);
    found
}

/// The lines of a source where a blockquote follows an attribution to XFA 3.3: the nearest comment
/// line before it that is not itself a quotation names `XFA 3.3` and no `§`.
fn quoted_lines(source: &str) -> Vec<usize> {
    let mut found = Vec::new();
    let mut attributed_to_xfa = false;
    let mut quoting = false;
    for (number, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("//") else {
            attributed_to_xfa = false;
            quoting = false;
            continue;
        };
        let rest = rest
            .strip_prefix('/')
            .or_else(|| rest.strip_prefix('!'))
            .unwrap_or(rest)
            .trim();
        if rest.starts_with('>') {
            if attributed_to_xfa && !quoting {
                found.push(number.saturating_add(1));
            }
            quoting = true;
            continue;
        }
        quoting = false;
        if rest.contains('§') {
            attributed_to_xfa = false;
        } else if rest.contains("XFA 3.3") {
            attributed_to_xfa = true;
        } else if !rest.is_empty() {
            attributed_to_xfa = false;
        }
    }
    found
}

/// What one source says wrongly about XFA 3.3, and how many citations it made.
fn sweep(pages: &Pages, at: &str, source: &str, findings: &mut Vec<String>) -> usize {
    let mut cited = 0usize;
    for (line, paragraph) in paragraphs(source) {
        for citation in citations(&paragraph) {
            cited = cited.saturating_add(1);
            for page in &citation.pages {
                if !pages.0.contains_key(page) {
                    findings.push(format!(
                        "{at}:{line}: XFA 3.3 has no page {page} (its folios run {}–{})",
                        pages.0.keys().next().copied().unwrap_or_default(),
                        pages.0.keys().next_back().copied().unwrap_or_default()
                    ));
                }
            }
            if let (Some(title), Some(first), Some(last)) = (
                &citation.title,
                citation.pages.first(),
                citation.pages.last(),
            ) && pages.0.contains_key(first)
                && !(first.saturating_sub(1)..=*last).any(|page| pages.holds_words(page, title))
            {
                findings.push(format!(
                    "{at}:{line}: XFA 3.3 pages {first}–{last} and the page before them do not \
                     carry the title *{title}*"
                ));
            }
        }
    }
    for line in quoted_lines(source) {
        findings.push(format!(
            "{at}:{line}: a blockquote attributed to XFA 3.3, whose notice grants no right to \
             reproduce its prose — paraphrase it and cite the page (doc/third-party-data.md)"
        ));
    }
    cited
}

fn rust_sources(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("a directory entry is readable").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name.starts_with('.') || name == "target" {
            continue;
        }
        if path.is_dir() {
            rust_sources(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

#[test]
fn every_citation_of_xfa_3_3_names_a_page_it_has_and_none_quotes_it() {
    let root = conformance::workspace_root();
    let pages = Pages::read(&root);
    assert!(
        pages.0.len() > 1_000,
        "doc/md/XFA-3_3.md read as {} pages; the conversion has one marker per page of 1 584",
        pages.0.len()
    );
    let own = Path::new(file!())
        .file_name()
        .expect("this test's own path names a file");
    let mut sources = Vec::new();
    for directory in conformance::roots::source_roots(&root).expect("the workspace's source roots")
    {
        rust_sources(&root.join(directory), &mut sources);
    }
    sources.sort();
    let mut findings = Vec::new();
    let mut cited = 0usize;
    for path in &sources {
        let is_this_file = path.file_name() == Some(own)
            && path.starts_with(root.join("tools").join("conformance").join("tests"));
        if is_this_file {
            continue;
        }
        let source = fs::read_to_string(path).expect("a source file of the tree is readable");
        let relative = path.strip_prefix(&root).unwrap_or(path);
        cited = cited.saturating_add(sweep(
            &pages,
            &relative.display().to_string(),
            &source,
            &mut findings,
        ));
    }
    println!(
        "{} source(s), {cited} citation(s) of XFA 3.3 by page, read against {} pages",
        sources.len(),
        pages.0.len()
    );
    assert!(
        cited > 0,
        "the sweep found no citation of XFA 3.3 — a clean answer over an empty population is a \
         sentence about the sweep"
    );
    assert!(findings.is_empty(), "\n  {}", findings.join("\n  "));
}

/// The sweep's own functions, run over a page that does not exist, a title on the wrong page, a
/// quotation, and a citation that is right — the first three found, the last passed (trap 13).
#[test]
fn the_sweep_finds_planted_citations_and_passes_a_true_one() {
    let pages = Pages::parse(
        "<!-- page 427 (427) -->\nChapter 12, Using Barcodes Pre-Processing of Barcode Data 427\n\
         <!-- page 428 (428) -->\nChapter 12, Using Barcodes Framing 428\nbody\n",
    );
    let planted = "\
/// The checksum keywords are XFA 3.3's, chapter 12, *Framing*, page 1600.
fn a() {}
/// The framing is XFA 3.3 chapter 12, *Legends*, page 428.
fn b() {}
/// XFA 3.3 page 428 says:
///
/// > Some barcode types support the inclusion of supplemental data
fn c() {}
";
    let mut findings = Vec::new();
    let cited = sweep(&pages, "planted.rs", planted, &mut findings);
    assert_eq!(cited, 3, "{findings:?}");
    assert_eq!(findings.len(), 3, "{findings:?}");
    assert!(findings.first().is_some_and(|f| f.contains("no page 1600")));
    assert!(findings.get(1).is_some_and(|f| f.contains("*Legends*")));
    assert!(findings.get(2).is_some_and(|f| f.contains("blockquote")));

    let true_one = "\
/// Framing is XFA 3.3 chapter 12, *Framing*, page 428, and pages 427–428 run it.
fn d() {}
";
    let mut findings = Vec::new();
    assert_eq!(sweep(&pages, "true.rs", true_one, &mut findings), 1);
    assert!(findings.is_empty(), "{findings:?}");
}
