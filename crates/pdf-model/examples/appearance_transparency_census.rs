//! §12.5.5's transparency sentences, counted: what an annotation's appearance says about the
//! group it is composited as, and what the annotation says about how that group meets the page.
//!
//! The clause states two things and this tree reads one of them:
//!
//! > Starting with PDF 1.4, an annotation appearance may include transparency. If the
//! > appearance's stream dictionary does not contain a Group entry, it shall be treated as a
//! > non-isolated, non-knockout transparency group. Otherwise, the isolated and knockout values
//! > specified in the group dictionary (see 11.6.6, "Transparency group XObjects") shall be used.
//!
//! > The transparency group shall be composited with a backdrop consisting of the page content
//! > along with any previously painted annotations, using the values of the BM , ca and CA
//! > entries in the annotation dictionary (see "Table 166 -Entries common to all annotation
//! > dictionaries") and a soft mask of None .
//!
//! §11.4.4's NOTE 5 is what makes the first sentence's *default* case free: compositing objects as
//! a group is the same as compositing them separately when the group is non-isolated with its
//! parent's knockout attribute and "the Normal blend mode is used, and the shape and opacity
//! inputs are always 1.0" at the composite with the backdrop. So the population that ranks the
//! requirement is the one where a condition of that note fails — an appearance stating a
//! `/Group` of its own, or an annotation stating a `/BM` that is not `Normal`.
//!
//! ```sh
//! cargo run --release -p pdf-model --example appearance_transparency_census              # curated
//! cargo run --release -p pdf-model --example appearance_transparency_census -- --pdfjs
//! cargo run --release -p pdf-model --example appearance_transparency_census -- --crawl
//! cargo run --release -p pdf-model --example appearance_transparency_census -- <file.pdf>...
//! ```
//!
//! The three scopes are `border_precedence_census`'s and are here for ADR 0490's reason: a
//! negative measured over one population decays when the population grows.

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
const MAX_NAMED: usize = 12;

/// Which population a run is over.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// The pdf.js corpus alone, which is what every gate in this tree walks.
    PdfJs,
    /// That, the `doc/corpora/` submodules, and this project's own fixtures.
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

/// What one document contributes to the census.
#[derive(Default)]
struct Counts {
    /// Annotations seen at all.
    seen: usize,
    /// Of those, the ones whose `/AP` resolves to at least one appearance stream.
    stored: usize,
    /// Of the stored appearance streams, the ones stating a `/Group` of any subtype.
    group: usize,
    /// Of those, the ones whose group subtype is `/Transparency`, which §11.6.6 makes the
    /// only one that means anything.
    transparency: usize,
    /// Of those, the ones stating `/I true`: §11.4.5's isolated group.
    isolated: usize,
    /// Of those, the ones stating `/K true`: §11.4.6's knockout group.
    knockout: usize,
    /// Of those, the ones stating a `/CS` blending colour space of their own (§11.6.6).
    group_space: usize,
    /// Annotations stating a `/BM` at all, by the name they state.
    blend: BTreeMap<String, usize>,
    /// Of the annotations with a stored appearance, the ones stating a `/BM` that is not
    /// `Normal` — where the group's composite with the page differs from painting its
    /// elements one at a time.
    blended_stored: usize,
    /// Of those, the subtypes they sit on.
    blended_on: BTreeMap<String, usize>,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.seen = self.seen.saturating_add(other.seen);
        self.stored = self.stored.saturating_add(other.stored);
        self.group = self.group.saturating_add(other.group);
        self.transparency = self.transparency.saturating_add(other.transparency);
        self.isolated = self.isolated.saturating_add(other.isolated);
        self.knockout = self.knockout.saturating_add(other.knockout);
        self.group_space = self.group_space.saturating_add(other.group_space);
        self.blended_stored = self.blended_stored.saturating_add(other.blended_stored);
        merge(&mut self.blend, &other.blend);
        merge(&mut self.blended_on, &other.blended_on);
    }

    /// Whether this document says anything the summary lines would not already imply.
    fn is_a_witness(&self) -> bool {
        self.transparency > 0 || self.blended_stored > 0
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
    let scope = if named.is_empty() {
        if arguments.iter().any(|a| a == "--crawl") {
            Scope::Crawl
        } else if arguments.iter().any(|a| a == "--pdfjs") {
            Scope::PdfJs
        } else {
            Scope::Curated
        }
    } else {
        Scope::Named
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
    println!("  {} state at least one appearance stream", total.stored);
    println!("  {} of those streams state a /Group", total.group);
    println!(
        "  {} of those are /S /Transparency: {} isolated, {} knockout, {} with a /CS",
        total.transparency, total.isolated, total.knockout, total.group_space
    );
    println!("  /BM names stated by annotations: {:?}", total.blend);
    println!(
        "  {} annotation(s) state a non-Normal /BM beside a stored appearance, on {:?}",
        total.blended_stored, total.blended_on
    );

    let witnesses: Vec<&(String, Counts)> = measured
        .iter()
        .filter(|(_, counts)| counts.is_a_witness())
        .collect();
    println!("  {} document(s) witness one of the above", witnesses.len());
    for (name, counts) in witnesses.iter().take(MAX_NAMED) {
        println!(
            "    {name}: {} transparency group(s) ({} isolated, {} knockout), {} blended",
            counts.transparency, counts.isolated, counts.knockout, counts.blended_stored
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
            let streams = appearance_streams(document, annotation);
            if streams.is_empty() {
                continue;
            }
            counts.stored = counts.stored.saturating_add(1);
            for stream in &streams {
                group_counts(document, stream, &mut counts);
            }
            let mode = document.get_key(annotation, "BM");
            let Some(name) = mode.as_name() else {
                continue;
            };
            let name = String::from_utf8_lossy(name.as_bytes()).into_owned();
            let held = counts.blend.entry(name.clone()).or_default();
            *held = held.saturating_add(1);
            if name == "Normal" || name == "Compatible" {
                continue;
            }
            counts.blended_stored = counts.blended_stored.saturating_add(1);
            let subtype = document
                .get_key(annotation, "Subtype")
                .as_name()
                .map_or_else(
                    || "(no /Subtype)".to_owned(),
                    |name| format!("/{}", String::from_utf8_lossy(name.as_bytes())),
                );
            let held = counts.blended_on.entry(subtype).or_default();
            *held = held.saturating_add(1);
        }
    }
    counts
}

/// Every appearance stream `/AP` names, whether directly or through a state subdictionary.
///
/// Table 170's three entries each hold "a single appearance stream or an appearance
/// subdictionary", so the population is every stream the file could show for this annotation
/// rather than the one `/AS` selects today.
fn appearance_streams(
    document: &Document,
    annotation: &pdf_syntax::Dictionary,
) -> Vec<pdf_syntax::Dictionary> {
    let appearance = document.get_key(annotation, "AP");
    let Some(appearance) = appearance.as_dict() else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for state in ["N", "R", "D"] {
        match document.get_key(appearance, state) {
            Object::Stream(stream) => found.push(stream.dict.clone()),
            Object::Dictionary(states) => {
                for (_, value) in states.iter() {
                    if let Object::Stream(stream) = document.resolve(value) {
                        found.push(stream.dict.clone());
                    }
                }
            }
            _ => {}
        }
    }
    found
}

/// Counts what one appearance stream's dictionary says about its transparency group.
fn group_counts(document: &Document, stream: &pdf_syntax::Dictionary, counts: &mut Counts) {
    let group = document.get_key(stream, "Group");
    let Some(group) = group.as_dict() else {
        return;
    };
    counts.group = counts.group.saturating_add(1);
    let subtype = document.get_key(group, "S");
    if subtype.as_name().map(|name| name.as_bytes().to_vec()) != Some(b"Transparency".to_vec()) {
        return;
    }
    counts.transparency = counts.transparency.saturating_add(1);
    if matches!(document.get_key(group, "I"), Object::Boolean(true)) {
        counts.isolated = counts.isolated.saturating_add(1);
    }
    if matches!(document.get_key(group, "K"), Object::Boolean(true)) {
        counts.knockout = counts.knockout.saturating_add(1);
    }
    if !matches!(document.get_key(group, "CS"), Object::Null) {
        counts.group_space = counts.group_space.saturating_add(1);
    }
}
