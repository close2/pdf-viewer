//! How far the corpus's substituted faces are from the widths their documents state.
//!
//! The instrument ADR 0358 was decided on. A substituted simple font is drawn at a horizontal
//! scale derived from the file's own `/Widths` against the chosen face's own advances
//! (§9.6.2.1's Table 109: "[t]hese widths shall be consistent with the actual widths given in
//! the font program"), and the two questions that decides are *how many* fonts that reaches
//! and *how far* it moves them. A rule that moved every substituted font by a tenth would be a
//! different proposition from one that leaves the metric-compatible ones alone and answers the
//! condensed ones, and only a count can tell those apart.
//!
//! ```sh
//! cargo run --release -p pdf-model --example substitute_stretch_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! Fonts are taken from each document's first page, which is the page the oracle judges. A
//! font that fails to load is skipped and counted: its shapes are not this census's subject.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout)]
#![allow(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus, each bounded by the fonts of one page"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use pdf_font::LoadedFont;
use pdf_syntax::{Dictionary, Document};

/// One printed line: a document's widest departure, its name, and the fonts behind it.
type Row = (f32, String, Vec<(String, f32)>);

/// The font dictionaries a document's first page names.
fn first_page_fonts(document: &Document) -> Vec<(String, Dictionary)> {
    let Some(page) = pdf_model::Pages::new(document).get(0) else {
        return Vec::new();
    };
    let fonts = document.get_key(&page.resources, "Font");
    let Some(fonts) = fonts.as_dict() else {
        return Vec::new();
    };
    fonts
        .iter()
        .filter_map(|(name, value)| {
            let dict = document.resolve(value).as_dict()?.clone();
            Some((String::from_utf8_lossy(name.as_bytes()).into_owned(), dict))
        })
        .collect()
}

fn main() {
    let files: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let mut loaded = 0usize;
    let mut substituted = 0usize;
    let mut moved = 0usize;
    let mut documents_moved = BTreeMap::new();
    // The distribution in twentieths, so that "metric-compatible" and "condensed" separate.
    let mut buckets: BTreeMap<i32, usize> = BTreeMap::new();

    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        for (font_name, dict) in first_page_fonts(&document) {
            let Ok(font) = LoadedFont::load(&document, &dict, &font_name) else {
                continue;
            };
            loaded += 1;
            if !font.is_substituted() {
                continue;
            }
            substituted += 1;
            let stretch = font.stretch();
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a stretch is bounded by the rule that produced it"
            )]
            let bucket = (stretch * 20.0).round() as i32;
            *buckets.entry(bucket).or_default() += 1;
            // Every departure, however small: a page's raster moves at a thousandth as surely
            // as at a fifth, and a round explaining which pages moved needs the whole list.
            if (stretch - 1.0).abs() > f32::EPSILON {
                moved += 1;
                documents_moved
                    .entry(name.clone())
                    .or_insert_with(Vec::new)
                    .push((font_name, stretch));
            }
        }
    }

    println!(
        "{} documents, {loaded} first-page fonts loaded",
        files.len()
    );
    println!("  substituted: {substituted}");
    println!("  drawn at a scale other than 1: {moved}");
    println!(
        "  documents with at least one such font: {}",
        documents_moved.len()
    );
    println!("  distribution, in twentieths of the face's own width:");
    for (bucket, count) in &buckets {
        println!("    {:.2}: {count}", f64::from(*bucket) / 20.0);
    }
    println!("  every document with a moved font, widest departure first:");
    let mut rows: Vec<Row> = documents_moved
        .into_iter()
        .map(|(document, fonts)| {
            let worst = fonts
                .iter()
                .map(|(_, stretch)| (stretch - 1.0).abs())
                .fold(0.0_f32, f32::max);
            (worst, document, fonts)
        })
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, document, fonts) in rows {
        let listed: Vec<String> = fonts
            .iter()
            .map(|(name, stretch)| format!("/{name} {stretch:.4}"))
            .collect();
        println!("    {document}: {}", listed.join(", "));
    }
}
