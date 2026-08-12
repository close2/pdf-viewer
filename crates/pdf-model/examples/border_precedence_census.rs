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
//!   ones.
//!
//! Both are counted **only where an annotation states no `/AP`**, because §12.5.2 hands a stored
//! appearance the whole job and a border this crate never constructs cannot be misplaced by it.
//!
//! ```sh
//! cargo run --release -p pdf-model --example border_precedence_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_syntax::{Document, Object};

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
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.seen = self.seen.saturating_add(other.seen);
        self.constructed = self.constructed.saturating_add(other.constructed);
        self.both = self.both.saturating_add(other.both);
        self.both_with_radius = self.both_with_radius.saturating_add(other.both_with_radius);
        for (style, count) in &other.styles {
            let held = self.styles.entry(style.clone()).or_default();
            *held = held.saturating_add(*count);
        }
    }
}

fn main() {
    let mut total = Counts::default();
    let mut opened = 0_usize;
    let mut lines: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        let counts = document_counts(&document);
        if counts.both > 0 || counts.styles.keys().any(|style| style != "S (or default)") {
            lines.push(format!(
                "  {name}: {} constructed, {} state both (radius {}), styles {:?}",
                counts.constructed, counts.both, counts.both_with_radius, counts.styles
            ));
        }
        total.absorb(&counts);
    }

    println!("{opened} document(s) opened, {} annotation(s)", total.seen);
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
    for line in &lines {
        println!("{line}");
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
            }
        }
    }
    counts
}
