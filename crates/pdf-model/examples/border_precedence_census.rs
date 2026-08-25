//! §12.5.4's borders, counted where this tree actually constructs one.
//!
//! Two questions, both about the sentences that decide where a border's ink lands:
//!
//! - **Precedence.** Table 166 says "[i]f an annotation dictionary includes the BS entry, then
//!   the Border entry is ignored", which Errata Collection 3 Issue #287 sharpens to *shall be
//!   ignored*. An annotation carrying both describes its border twice, and the half that is
//!   ignored includes `/Border`'s first two elements — the corner radii, which are the one part
//!   of the entry `/BS` has no equivalent for.
//! - **Placement.** "If present, the border shall be drawn completely inside the annotation
//!   rectangle", which is a statement about every style in Table 168 and not only the rectangular
//!   ones. **This census counts what states a border and cannot say where the ink went**, because
//!   that is a fact about a raster rather than about a dictionary; `border_overhang_census` is the
//!   one that measures it, over this tree's render and a reference's together.
//!
//! Both are counted **only where an annotation states no `/AP`**, because §12.5.2 hands a stored
//! appearance the whole job and a border this crate never constructs cannot be misplaced by it.
//!
//! ```sh
//! cargo run --release -p pdf-model --example border_precedence_census              # curated
//! cargo run --release -p pdf-model --example border_precedence_census -- --pdfjs
//! cargo run --release -p pdf-model --example border_precedence_census -- --crawl   # CC-MAIN-2021-31
//! cargo run --release -p pdf-model --example border_precedence_census -- <file.pdf>...
//! ```
//!
//! **The three scopes are the six-hundred-and-eighty-sixth session's**, and they are here for the
//! reason ADR 0490 gives: this row's negatives were measured over "the 964 openable documents"
//! before `CC-MAIN-2021-31` was on the disk, and a negative decays when the population grows. Run
//! the control beside the crawl rather than instead of it — the old sentence is usually right
//! about its own population, which is exactly why nothing in the tree could see it.
//!
//! **What is counted alongside each total is the *subtype*.** §12.5.4 states which subtypes'
//! `/BS` is a border at all — "[s]uch dictionaries may also be used to specify the width and dash
//! pattern for the lines drawn by line, square, circle, and ink annotations" — so a corner radius
//! on an ink annotation and one on a link are different findings, and a count that added them
//! would retire the sharper half of the claim with the wider one (ADR 0516).

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Object};
use rayon::prelude::*;

/// How many witnessing document names are printed per finding before the list is truncated.
///
/// The curated population is small enough to print whole and the crawl is not; the shape
/// `absence_audit` settled on, for the same reason it settled on it.
const MAX_NAMED: usize = 12;

/// Which population a run is over.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// The pdf.js corpus alone — "the 974", which this row's own sentences are about.
    PdfJs,
    /// That, the four `doc/corpora/` submodules, and this project's own fixtures.
    Curated,
    /// The `SafeDocs` `CC-MAIN-2021-31` crawl under `corpus-cache/`, and nothing else.
    Crawl,
    /// Whatever files the command line named.
    Named,
}

/// Every PDF this census measures over, in the scope asked for.
fn corpus(scope: Scope, named: &[String]) -> Vec<PathBuf> {
    if scope == Scope::Named {
        return named.iter().map(PathBuf::from).collect();
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    let roots: &[&str] = match scope {
        Scope::PdfJs => &["doc/pdf.js/test/pdfs"],
        Scope::Curated => &["doc/pdf.js/test/pdfs", "doc/corpora", "doc/corpora-own"],
        Scope::Crawl => &["corpus-cache/safedocs/cc-main-2021-31"],
        Scope::Named => &[],
    };
    for relative in roots {
        collect(&root.join(relative), &mut files);
    }
    files.sort();
    files.dedup();
    files
}

/// Every `.pdf` under one directory, recursively.
fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        {
            into.push(path);
        }
    }
}

/// What one annotation contributes to the census.
#[derive(Default)]
struct Counts {
    /// Annotations seen at all.
    seen: usize,
    /// Annotations stating no `/AP`, which are the ones a construction reaches.
    constructed: usize,
    /// Of those, the ones stating both `/Border` and `/BS`.
    both: usize,
    /// Of those, the ones whose ignored `/Border` states a non-zero corner radius.
    both_with_radius: usize,
    /// Of the constructed ones, per Table 168 border style name.
    styles: BTreeMap<String, usize>,
    /// The subtypes carrying a non-zero ignored corner radius, which is the sharper claim.
    radius_on: BTreeMap<String, usize>,
    /// The subtypes carrying Table 168's `B` or `I`, which this tree draws as `S` and reports.
    bevelled_on: BTreeMap<String, usize>,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.seen = self.seen.saturating_add(other.seen);
        self.constructed = self.constructed.saturating_add(other.constructed);
        self.both = self.both.saturating_add(other.both);
        self.both_with_radius = self.both_with_radius.saturating_add(other.both_with_radius);
        merge(&mut self.styles, &other.styles);
        merge(&mut self.radius_on, &other.radius_on);
        merge(&mut self.bevelled_on, &other.bevelled_on);
    }

    /// Whether this document says anything the summary lines would not already imply.
    fn is_a_witness(&self) -> bool {
        self.both > 0 || self.styles.keys().any(|style| style != "S (or default)")
    }
}

/// Adds one distribution's counts into another.
fn merge(into: &mut BTreeMap<String, usize>, from: &BTreeMap<String, usize>) {
    for (key, count) in from {
        let held = into.entry(key.clone()).or_default();
        *held = held.saturating_add(*count);
    }
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let named: Vec<String> = arguments
        .iter()
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .collect();
    let scope = if !named.is_empty() {
        Scope::Named
    } else if arguments.iter().any(|a| a == "--crawl") {
        Scope::Crawl
    } else if arguments.iter().any(|a| a == "--pdfjs") {
        Scope::PdfJs
    } else {
        Scope::Curated
    };

    let files = corpus(scope, &named);
    eprintln!("{} PDF(s) in the population", files.len());

    let measured: Vec<(String, Counts)> = files
        .par_iter()
        .filter_map(|path| {
            let bytes = std::fs::read(path).ok()?;
            let document = Document::open(bytes).ok()?;
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            Some((name, document_counts(&document)))
        })
        .collect();

    let mut total = Counts::default();
    for (_, counts) in &measured {
        total.absorb(counts);
    }

    println!(
        "{} document(s) opened, {} annotation(s)",
        measured.len(),
        total.seen
    );
    println!(
        "  {} state no /AP, so a border is constructed",
        total.constructed
    );
    println!("  {} of those state both /Border and /BS", total.both);
    println!(
        "  {} of those state a non-zero /Border corner radius that Table 166 ignores",
        total.both_with_radius
    );
    println!("  border styles among the constructed: {:?}", total.styles);
    println!("  a non-zero ignored radius sits on: {:?}", total.radius_on);
    println!("  Table 168's B and I sit on: {:?}", total.bevelled_on);

    let witnesses: Vec<&(String, Counts)> = measured
        .iter()
        .filter(|(_, counts)| counts.is_a_witness())
        .collect();
    println!("  {} document(s) witness one of the above", witnesses.len());
    for (name, counts) in witnesses.iter().take(MAX_NAMED) {
        println!(
            "    {name}: {} constructed, {} state both (radius {}), styles {:?}",
            counts.constructed, counts.both, counts.both_with_radius, counts.styles
        );
    }
    if witnesses.len() > MAX_NAMED {
        println!(
            "    … and {} more",
            witnesses.len().saturating_sub(MAX_NAMED)
        );
    }
}

/// Walks every annotation on every page of one document.
fn document_counts(document: &Document) -> Counts {
    let mut counts = Counts::default();
    let pages = pdf_model::Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let entry = document.get_key(&page.dict, "Annots");
        let Some(list) = entry.as_array() else {
            continue;
        };
        for item in list {
            let object = document.resolve(item);
            let Some(annotation) = object.as_dict() else {
                continue;
            };
            counts.seen = counts.seen.saturating_add(1);
            if !matches!(document.get_key(annotation, "AP"), Object::Null) {
                continue;
            }
            counts.constructed = counts.constructed.saturating_add(1);
            let subtype = document
                .get_key(annotation, "Subtype")
                .as_name()
                .map_or_else(
                    || "(no /Subtype)".to_owned(),
                    |name| format!("/{}", String::from_utf8_lossy(name.as_bytes())),
                );
            let border = document.get_key(annotation, "Border");
            let style = document.get_key(annotation, "BS");
            let name = style.as_dict().map_or_else(
                || "S (or default)".to_owned(),
                |dict| {
                    document.get_key(dict, "S").as_name().map_or_else(
                        || "S (or default)".to_owned(),
                        |name| String::from_utf8_lossy(name.as_bytes()).into_owned(),
                    )
                },
            );
            if name == "B" || name == "I" {
                let held = counts
                    .bevelled_on
                    .entry(format!("{name} on {subtype}"))
                    .or_default();
                *held = held.saturating_add(1);
            }
            let held = counts.styles.entry(name).or_default();
            *held = held.saturating_add(1);
            let (Some(array), Some(_)) = (border.as_array(), style.as_dict()) else {
                continue;
            };
            counts.both = counts.both.saturating_add(1);
            if array.iter().take(2).any(
                |item| matches!(document.resolve(item).as_number(), Some(value) if value != 0.0),
            ) {
                counts.both_with_radius = counts.both_with_radius.saturating_add(1);
                let held = counts.radius_on.entry(subtype).or_default();
                *held = held.saturating_add(1);
            }
        }
    }
    counts
}
