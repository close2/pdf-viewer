//! Table 169's cloudy border effect, counted where it is stated and where it is constructed.
//!
//! §12.5.4 gives four subtypes a `/BE` — "some annotations (square, circle, and polygon) may have
//! a BE entry", and "[b]eginning with PDF 1.6, free text annotations may also have a BE entry" —
//! and Table 169's `C` says the border "should be drawn as a series of convex curved line
//! segments in a manner that simulates the appearance of a cloud", with an `/I` intensity "in
//! the range 0 to 2". ADR 1057 chooses the geometry the table leaves open, and this is the
//! measurement it was written against: how many annotations state the effect at all, on which
//! subtypes, at which intensities, and how many of them a construction ever reaches — the ones
//! stating no `/AP`, since §12.5.2 hands a stored appearance the whole job.
//!
//! ```sh
//! cargo run --release -p pdf-model --example cloudy_border_census              # everything under doc/
//! cargo run --release -p pdf-model --example cloudy_border_census -- <file.pdf>...
//! ```
//!
//! A `/BE` on a subtype §12.5.4 does not give one to is counted under its own subtype, because
//! that is a finding rather than noise: Table 181 makes the entry "meaningful only for polygon
//! annotations", so a polyline's is a key the standard defines nothing for.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Object};
use rayon::prelude::*;

/// How many witnessing document names are printed before the list is truncated.
const MAX_NAMED: usize = 20;

/// Every PDF under `doc/`, or the files the command line named.
fn corpus(named: &[String]) -> Vec<PathBuf> {
    if !named.is_empty() {
        return named.iter().map(PathBuf::from).collect();
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc");
    let mut files = Vec::new();
    collect(&root, &mut files);
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

/// What one document contributes.
#[derive(Default)]
struct Counts {
    /// Annotations seen at all.
    seen: usize,
    /// Annotations stating `/BE /S /C`, per subtype.
    cloudy: BTreeMap<String, usize>,
    /// Of those, the ones stating no `/AP` — the ones a construction reaches — per subtype.
    constructed: BTreeMap<String, usize>,
    /// Of the cloudy ones, per `/I` value as the file spells it.
    intensities: BTreeMap<String, usize>,
    /// Of the cloudy ones, how many state an `/RD`.
    with_differences: usize,
}

impl Counts {
    fn absorb(&mut self, other: &Self) {
        self.seen = self.seen.saturating_add(other.seen);
        merge(&mut self.cloudy, &other.cloudy);
        merge(&mut self.constructed, &other.constructed);
        merge(&mut self.intensities, &other.intensities);
        self.with_differences = self.with_differences.saturating_add(other.with_differences);
    }

    fn is_a_witness(&self) -> bool {
        !self.cloudy.is_empty()
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
    let named: Vec<String> = std::env::args().skip(1).collect();
    let files = corpus(&named);
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
    println!("  /BE /S /C stated, by subtype: {:?}", total.cloudy);
    println!(
        "  of those, stating no /AP (a construction reaches them): {:?}",
        total.constructed
    );
    println!("  /I among the cloudy: {:?}", total.intensities);
    println!("  /RD among the cloudy: {}", total.with_differences);

    let witnesses: Vec<&(String, Counts)> = measured
        .iter()
        .filter(|(_, counts)| counts.is_a_witness())
        .collect();
    println!("  {} document(s) state a cloudy border", witnesses.len());
    for (name, counts) in witnesses.iter().take(MAX_NAMED) {
        println!(
            "    {name}: cloudy {:?}, constructed {:?}, /I {:?}",
            counts.cloudy, counts.constructed, counts.intensities
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
            let effect = document.get_key(annotation, "BE");
            let cloudy = effect.as_dict().is_some_and(|effect| {
                document
                    .get_key(effect, "S")
                    .as_name()
                    .is_some_and(|name| name.as_bytes() == b"C")
            });
            if !cloudy {
                continue;
            }
            let subtype = document
                .get_key(annotation, "Subtype")
                .as_name()
                .map_or_else(
                    || "(no /Subtype)".to_owned(),
                    |name| format!("/{}", String::from_utf8_lossy(name.as_bytes())),
                );
            let held = counts.cloudy.entry(subtype.clone()).or_default();
            *held = held.saturating_add(1);
            if matches!(document.get_key(annotation, "AP"), Object::Null) {
                let held = counts.constructed.entry(subtype).or_default();
                *held = held.saturating_add(1);
            }
            let intensity = effect
                .as_dict()
                .and_then(|effect| document.get_key(effect, "I").as_number())
                .map_or_else(|| "(absent)".to_owned(), |value| format!("{value}"));
            let held = counts.intensities.entry(intensity).or_default();
            *held = held.saturating_add(1);
            if !matches!(document.get_key(annotation, "RD"), Object::Null) {
                counts.with_differences = counts.with_differences.saturating_add(1);
            }
        }
    }
    counts
}
