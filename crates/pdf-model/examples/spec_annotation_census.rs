//! What the annotations in a document carry, by subtype — and what its structure tree gives.
//!
//! Written for one question, in `doc/todo/48`: `doc/md/` is a Markdown conversion of the fourteen
//! specification PDFs under `doc/`, the conversion **ignored annotations**, and the conformance
//! gate verifies every rustdoc quotation against the result. If those annotations are errata or
//! committee notes, a quotation of body text can quote a sentence a note has corrected; if they
//! are links and bookmarks, nothing has been lost and the item closes with the count written
//! down. Nothing decides that but counting, so this counts.
//!
//! Three groups of numbers, per document and then in total:
//!
//! - **How many annotations, by §12.5.6 subtype.** Table 171's list is not enumerated here — the
//!   `/Subtype` name is reported as the file states it, because a subtype outside the table is
//!   itself a finding.
//! - **What they carry.** Table 166's `/Contents`, Table 172's `/T`, `/RC`, `/Subj`, `/M` and
//!   `/CreationDate`, and §12.5.6.14's `/Popup` — read through `pdf_model::popup::popups`, so
//!   that what is counted is what a reader would be shown rather than what the file states.
//!   Distinct `/T` and `/Subj` values are printed in full: an erratum names its author and its
//!   subject, and a bookmark names neither.
//! - **What §14.7 gives**, since the proposal's second argument is that a tagged document needs
//!   no layout converter's inference. `/MarkInfo`, `/StructTreeRoot`, and — the number that
//!   decides whether that argument holds — how much of the tree `structure::Tree::walk` returns
//!   before its `MAX_CHILDREN` bound stops it, which is what `logical_order`, `logical_text` and
//!   `logical_range` are all built on.
//!
//! Every annotation of every page is read, not a prefix: an erratum on page 900 is the case this
//! exists to find. That costs about a minute over ISO 32000-2's 1023 pages, which is why this is
//! an example a person runs and not a gate — `doc/todo/48` is explicit that the gate must not
//! parse a 1023-page PDF on every run.
//!
//! ```sh
//! cargo run --release -p pdf-model --example spec_annotation_census -- doc/*.pdf
//! ```
//!
//! **What it prints is derived from documents this project may not redistribute** (ADR 0187), so
//! its output belongs beside them under `.gitignore` rather than in a committed file. The counts
//! are facts about the documents and are quotable; the text it samples is the specification's.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Object};

/// Table 172's entries that make an annotation a person's remark rather than a document's link.
///
/// `/T` is "[t]he text label that shall be displayed in the title bar of the annotation's popup
/// window when open and active" — in practice the author — and `/Subj` is "[t]ext representing a
/// short description of the subject being addressed by the annotation". A review comment states
/// them; a link states neither, because Table 176 gives a link neither entry.
const MARKUP_LABELS: [&str; 2] = ["T", "Subj"];

/// One document's counters.
#[derive(Default)]
struct Census {
    /// How many pages were walked.
    pages: usize,
    /// How many entries in every `/Annots` array together.
    annotations: usize,
    /// `/Subtype` name to count, as the file states it.
    subtypes: BTreeMap<String, usize>,
    /// Of those, how many state Table 166's `/Contents` with something in it.
    contents: usize,
    /// The total length in characters of every `/Contents` — how much text a reader would want.
    contents_chars: usize,
    /// How many state `/RC`, Table 172's rich text (ADR 0224).
    rich: usize,
    /// How many state one of [`MARKUP_LABELS`], and the distinct values of each.
    labels: BTreeMap<&'static str, BTreeSet<String>>,
    /// How many state `/M`, and how many `/CreationDate`.
    dates: BTreeMap<&'static str, usize>,
    /// The distinct years those dates state, which say whether a file was annotated after it was
    /// published.
    years: BTreeSet<i32>,
    /// How many state §12.5.6.14's `/Popup`.
    popup_entry: usize,
    /// How many popup windows `pdf_model::popup::popups` would show, and how many carry text.
    popup_windows: usize,
    /// Of those windows, how many have text in them.
    popup_windows_with_text: usize,
    /// A few `/Contents` values, so that a reader can see what kind of remark they are.
    samples: Vec<String>,
}

impl Census {
    /// Walks every page of one document into the counters.
    fn count(&mut self, document: &Document) {
        let pages = pdf_model::Pages::new(document);
        let view = pdf_model::view::ViewState::of(document);
        self.pages = self.pages.saturating_add(pages.len());
        for index in 0..pages.len() {
            let Some(page) = pages.get(index) else {
                continue;
            };
            for window in pdf_model::popup::popups(document, &page, &view) {
                self.popup_windows = self.popup_windows.saturating_add(1);
                if window.text.is_some() {
                    self.popup_windows_with_text = self.popup_windows_with_text.saturating_add(1);
                }
            }
            let Some(list) = document
                .get_key(&page.dict, "Annots")
                .as_array()
                .map(<[Object]>::to_vec)
            else {
                continue;
            };
            for entry in &list {
                let resolved = document.resolve(entry);
                if let Some(annotation) = resolved.as_dict() {
                    self.count_annotation(document, annotation);
                }
            }
        }
    }

    /// One annotation dictionary.
    fn count_annotation(&mut self, document: &Document, annotation: &Dictionary) {
        self.annotations = self.annotations.saturating_add(1);
        let subtype = document
            .get_key(annotation, "Subtype")
            .as_name()
            .map_or_else(
                || "(no /Subtype)".to_owned(),
                |name| String::from_utf8_lossy(name.as_bytes()).into_owned(),
            );
        let counter = self.subtypes.entry(subtype).or_default();
        *counter = counter.saturating_add(1);
        if let Some(text) = text(document, annotation, "Contents") {
            self.contents = self.contents.saturating_add(1);
            self.contents_chars = self.contents_chars.saturating_add(text.chars().count());
            if self.samples.len() < 8 {
                self.samples.push(flatten(&text));
            }
        }
        if text(document, annotation, "RC").is_some() {
            self.rich = self.rich.saturating_add(1);
        }
        for key in MARKUP_LABELS {
            if let Some(value) = text(document, annotation, key) {
                self.labels.entry(key).or_default().insert(flatten(&value));
            }
        }
        for key in ["M", "CreationDate"] {
            let value = document.get_key(annotation, key);
            let Object::String(bytes) = &value else {
                continue;
            };
            let counter = self.dates.entry(key).or_default();
            *counter = counter.saturating_add(1);
            if let Some(date) =
                pdf_syntax::Date::parse(&pdf_syntax::text_string::text_string(bytes))
            {
                self.years.insert(date.year);
            }
        }
        if document.get_key(annotation, "Popup").as_dict().is_some() {
            self.popup_entry = self.popup_entry.saturating_add(1);
        }
    }

    /// This document's block of the report.
    fn report(&self, name: &str, structure: &Structure) {
        println!("{name}");
        println!(
            "  {} page(s), {} annotation(s)",
            self.pages, self.annotations
        );
        let subtypes: Vec<String> = self
            .subtypes
            .iter()
            .map(|(subtype, count)| format!("{subtype}={count}"))
            .collect();
        println!("  subtypes: {}", subtypes.join(" "));
        println!(
            "  /Contents {} ({} chars)  /RC {}  /Popup {}  popup windows {} ({} with text)",
            self.contents,
            self.contents_chars,
            self.rich,
            self.popup_entry,
            self.popup_windows,
            self.popup_windows_with_text
        );
        let dates: Vec<String> = self
            .dates
            .iter()
            .map(|(key, count)| format!("/{key}={count}"))
            .collect();
        let years: Vec<String> = self.years.iter().map(i32::to_string).collect();
        println!(
            "  dates: {}  years: {}",
            if dates.is_empty() {
                "none".to_owned()
            } else {
                dates.join(" ")
            },
            if years.is_empty() {
                "none".to_owned()
            } else {
                years.join(",")
            }
        );
        for key in MARKUP_LABELS {
            let values = self.labels.get(key).map_or(0, BTreeSet::len);
            println!("  distinct /{key}: {values}");
            for value in self.labels.get(key).into_iter().flatten().take(12) {
                println!("    {value}");
            }
        }
        println!("  {structure}");
        for sample in &self.samples {
            println!("  sample /Contents: {sample}");
        }
    }
}

/// What §14.7 says about one document, and how much of it this tree's reader returns.
struct Structure {
    /// Table 353's `/MarkInfo /Marked`.
    marked: bool,
    /// Whether the catalog states a `/StructTreeRoot` this tree can open.
    tree: bool,
    /// How many items `structure::Tree::walk` returned.
    walked: usize,
    /// Whether that number is the reader's `MAX_CHILDREN` bound rather than the tree's size.
    bounded: bool,
    /// How many pages state §14.7.5.4's `/StructParents`, so that `ParentTree::for_page` — the
    /// per-page route, which the bound above does not reach — has something to read.
    pages_with_parents: usize,
}

impl Structure {
    /// Reads one document's structure summary.
    fn of(document: &Document) -> Self {
        let mark = pdf_model::structure::MarkInfo::read(document);
        let tree = pdf_model::structure::Tree::of(document);
        let walked = tree
            .as_ref()
            .map_or(0, |tree| tree.walk(document).items.len());
        let pages = pdf_model::Pages::new(document);
        let mut pages_with_parents = 0_usize;
        for index in 0..pages.len() {
            let Some(page) = pages.get(index) else {
                continue;
            };
            if document
                .get_key(&page.dict, "StructParents")
                .as_integer()
                .is_some()
            {
                pages_with_parents = pages_with_parents.saturating_add(1);
            }
        }
        Self {
            marked: mark.marked,
            tree: tree.is_some(),
            // `walk` stops at 65 536 items; a tree at exactly that size is reported as bounded,
            // which errs towards saying the reader saw less than the file holds.
            bounded: walked >= 65_536,
            walked,
            pages_with_parents,
        }
    }
}

impl std::fmt::Display for Structure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "structure: /Marked {} /StructTreeRoot {} walk {}{} pages with /StructParents {}",
            self.marked,
            self.tree,
            self.walked,
            if self.bounded { " (BOUNDED)" } else { "" },
            self.pages_with_parents
        )
    }
}

fn main() {
    let mut totals = Census::default();
    let mut documents = 0_usize;
    for path in std::env::args().skip(1) {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                println!("{path}: unreadable: {error}");
                continue;
            }
        };
        let document = match Document::open(bytes) {
            Ok(document) => document,
            Err(error) => {
                println!("{path}: will not open: {error}");
                continue;
            }
        };
        documents = documents.saturating_add(1);
        let mut census = Census::default();
        census.count(&document);
        let structure = Structure::of(&document);
        census.report(path.rsplit('/').next().unwrap_or(&path), &structure);
        totals.merge(&census);
    }
    println!("--");
    println!("{documents} document(s)");
    totals.report(
        "TOTAL",
        &Structure {
            marked: false,
            tree: false,
            walked: 0,
            bounded: false,
            pages_with_parents: 0,
        },
    );
}

impl Census {
    /// Adds one document's counters into the running total.
    ///
    /// The label sets are unioned rather than summed: two documents whose annotations carry the
    /// same author state one author between them, and a total that counted it twice would say
    /// something no document says.
    fn merge(&mut self, other: &Self) {
        self.pages = self.pages.saturating_add(other.pages);
        self.annotations = self.annotations.saturating_add(other.annotations);
        for (subtype, count) in &other.subtypes {
            let counter = self.subtypes.entry(subtype.clone()).or_default();
            *counter = counter.saturating_add(*count);
        }
        self.contents = self.contents.saturating_add(other.contents);
        self.contents_chars = self.contents_chars.saturating_add(other.contents_chars);
        self.rich = self.rich.saturating_add(other.rich);
        for (key, values) in &other.labels {
            self.labels.entry(key).or_default().extend(values.clone());
        }
        for (key, count) in &other.dates {
            let counter = self.dates.entry(key).or_default();
            *counter = counter.saturating_add(*count);
        }
        self.years.extend(other.years.iter().copied());
        self.popup_entry = self.popup_entry.saturating_add(other.popup_entry);
        self.popup_windows = self.popup_windows.saturating_add(other.popup_windows);
        self.popup_windows_with_text = self
            .popup_windows_with_text
            .saturating_add(other.popup_windows_with_text);
    }
}

/// One text string entry, decoded, with an empty value treated as absent.
fn text(document: &Document, annotation: &Dictionary, key: &str) -> Option<String> {
    let value = document.get_key(annotation, key);
    let bytes: Vec<u8> = match &value {
        Object::String(bytes) => bytes.to_vec(),
        Object::Stream(stream) => document.decoded_stream_data(stream)?.to_vec(),
        _ => return None,
    };
    let decoded = pdf_syntax::text_string::text_string(&bytes);
    (!decoded.trim().is_empty()).then_some(decoded)
}

/// A value on one line, short enough for a terminal.
fn flatten(value: &str) -> String {
    let flat: String = value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    flat.chars().take(160).collect()
}
