//! ISO 32000-2 §7.9.4's date strings, over every one the corpus contains.
//!
//! §7.9.4 states a grammar exactly and states no recovery, so this is the shape of gate this
//! project's habits call the strongest kind: **a clause that states an algorithm is a clause
//! that can audit a corpus.** Every `/CreationDate`, `/ModDate` and annotation or signature `/M`
//! in all 974 documents is a producer's independent attempt at the same grammar, and running the
//! parser over all of them checks the reader against a hundred producers at once.
//!
//! What it is *not* is a check that a date is correct. Nothing here can know whether a file's
//! `/CreationDate` is when the file was created. What it checks is conformance, in both
//! directions: the count may only rise, so a parser that starts rejecting valid dates fails, and
//! the named non-conforming strings are listed so that a parser quietly starting to *accept* one
//! is visible in the diff.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_syntax::{Date, Document, Object, ObjectId};

/// The lowest number of conforming date strings this corpus may yield.
///
/// A ratchet in one direction only, like every other count in this tree: it rises when the
/// parser learns something the clause permits, and a fall is a regression. It is not the *total*
/// — see `MAX_NON_CONFORMING` for the other end.
const MIN_CONFORMING: usize = 1514;

/// The most date-shaped strings this corpus may hold that are not §7.9.4 dates.
///
/// Measured at 31 over 22 distinct strings, out of 1545 date-shaped strings, every one of them a producer breaking a rule the
/// clause states in as many words. The list is in the test's own output; the four kinds are:
/// an offset minute outside `mm (00-59)` — `+00'112'` and four other values, 26 of the 31, and
/// plainly one broken producer; two strings with no `D:` prefix at all; a month of `00`; and a
/// thirteen-digit numeric part, which no prefix of the layout can be.
const MAX_NON_CONFORMING: usize = 31;

/// The pdf.js corpus, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    files.sort();
    Some(files)
}

/// Whether a string is *trying* to be a date, so that the denominator is not every string in the
/// corpus.
///
/// Two shapes: §7.9.4's own `D:` prefix, and eight leading digits, which is what the two
/// corpus strings that omit the prefix have. Deliberately wider than the clause — a denominator
/// that only counts conforming strings would report 100% by construction.
fn looks_like_a_date(text: &str) -> bool {
    text.starts_with("D:") || (text.len() >= 8 && text.bytes().take(8).all(|b| b.is_ascii_digit()))
}

/// Every `/M`, `/CreationDate` and `/ModDate` in every object of every document.
///
/// The three keys the standard gives a date value that a *reader* meets: §12.5.2's annotation
/// modification time, §12.8's signing time (both `/M`), and Table 45's two on an embedded file.
/// Walking every object rather than every annotation is what makes this a census: a document
/// information dictionary's dates are counted too, and they are the same grammar.
#[test]
#[ignore = "walks every object of the whole corpus"]
fn every_date_string_in_the_corpus_is_measured_against_the_clause() {
    let Some(files) = corpus() else {
        println!("the pdf.js submodule is not checked out; skipping");
        return;
    };

    let (mut total, mut conforming) = (0usize, 0usize);
    let mut refused: BTreeMap<String, usize> = BTreeMap::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let count = u32::try_from(document.xref().len()).unwrap_or(u32::MAX);
        for number in 1..=count {
            let object = document.get(ObjectId::new(number, 0));
            let dict = match &object {
                Object::Dictionary(dict) => dict.clone(),
                Object::Stream(stream) => stream.dict.clone(),
                _ => continue,
            };
            for key in ["M", "CreationDate", "ModDate"] {
                let Some(stated) = dict.get(key) else {
                    continue;
                };
                let Object::String(bytes) = document.resolve(stated) else {
                    continue;
                };
                let text = pdf_syntax::text_string(&bytes);
                if !looks_like_a_date(&text) {
                    continue;
                }
                total = total.saturating_add(1);
                if Date::parse(&text).is_some() {
                    conforming = conforming.saturating_add(1);
                } else {
                    *refused.entry(text).or_default() += 1;
                }
            }
        }
    }

    let mut listed: Vec<(&String, &usize)> = refused.iter().collect();
    listed.sort_by_key(|(text, count)| (std::cmp::Reverse(**count), (*text).clone()));
    for (text, count) in &listed {
        println!("  {count:3} × {text:?}");
    }
    let non_conforming = total.saturating_sub(conforming);
    // Per ten thousand and then printed with a decimal point, so the share needs no float and
    // no cast: 1514 of 1545 is 9799, shown as 97.99%.
    let share = conforming
        .saturating_mul(10_000)
        .checked_div(total)
        .unwrap_or_default();
    println!(
        "{total} date strings in {} documents: {conforming} conform to §7.9.4 ({}.{:02}%), \
         {non_conforming} do not, over {} distinct strings",
        files.len(),
        share / 100,
        share % 100,
        refused.len()
    );

    assert!(
        conforming >= MIN_CONFORMING,
        "{conforming} date strings parse, down from {MIN_CONFORMING} — the ratchet only rises"
    );
    assert!(
        non_conforming <= MAX_NON_CONFORMING,
        "{non_conforming} date strings do not parse, up from {MAX_NON_CONFORMING}"
    );
}
