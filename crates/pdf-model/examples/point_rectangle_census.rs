//! How many annotations state a `/Rect` covering no area, and what their subtype clause states.
//!
//! ISO 32000-2 §12.5.2's Table 166 makes `/Rect` required and, in the same table, frees a writer
//! from supplying an appearance dictionary for one shape of it:
//!
//! > Annotations where the value of the Rect key consists of an array where the value at index 1
//! > is equal to the value at index 3 and the value at index 2 is equal to the value at index 4.
//!
//! A point, in other words — and the standard says so about a *writer*, not about what a reader
//! then draws. For a subtype whose marks §12.5.5 places by scaling an appearance stream's `/BBox`
//! onto that rectangle there is nothing left to draw. For the six whose own clause states their
//! geometry "in default user space" instead — §12.5.6.7's `/L`, §12.5.6.9's `/Vertices`,
//! §12.5.6.10's `/QuadPoints`, §12.5.6.13's `/InkList` and §12.5.6.6's `/CL` — the marks are
//! stated whole and the rectangle is a bounding box the writer got wrong. ADR 0825 is that
//! reading; this census is its population.
//!
//! Three numbers per document and in total, over **every page** rather than page one:
//!
//! - annotations whose `/Rect` covers no area, by `/Subtype` as the file states it;
//! - how many of those state no appearance dictionary at all, which is the only case a
//!   construction is reached for;
//! - how many of *those* state the entry their own clause puts in default user space, which is
//!   the count of marks a reader can still make.
//!
//! It reads `/Annots` with `pdf_syntax` alone rather than through [`pdf_model::annotation`],
//! because a census whose predicate is the code under test measures the code rather than the
//! corpus (`doc/HANDOVER.md` trap 8).
//!
//! ```sh
//! cargo run --release -p pdf-model --example point_rectangle_census -- <file.pdf>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Object};

/// The bound `pdf_model::page` walks the page tree under, so that this reaches what that walk
/// would reach rather than what an unbounded one would.
const MAX_TREE_DEPTH: usize = 64;
/// As above: a `/Kids` cycle is what stops this walk, not the tree's size.
const MAX_NODES_VISITED: usize = 100_000;

/// The entry each subtype's clause states "in default user space", where it has one.
///
/// The same six [`pdf_model`]'s `appearance::bounded_by_rect` holds, listed here from the tables
/// rather than from that function, for trap 8's reason.
fn stated_in_user_space(subtype: &[u8]) -> Option<&'static str> {
    match subtype {
        b"Line" => Some("L"),
        b"Polygon" | b"PolyLine" => Some("Vertices"),
        b"Ink" => Some("InkList"),
        b"Highlight" | b"Underline" | b"StrikeOut" | b"Squiggly" => Some("QuadPoints"),
        b"FreeText" => Some("CL"),
        _ => None,
    }
}

/// What one document's annotations say about the question.
#[derive(Default)]
struct Finding {
    /// Annotations whose `/Rect` covers no area, by the `/Subtype` the file states.
    by_subtype: BTreeMap<String, usize>,
    /// How many of those state no `/AP` at all.
    without_appearance: usize,
    /// How many of those without an `/AP` state the entry their clause puts in user space.
    still_stated: usize,
}

fn main() {
    let mut opened = 0_usize;
    let mut with_any = 0_usize;
    let mut with_drawable = 0_usize;
    let mut totals: BTreeMap<String, usize> = BTreeMap::new();
    let mut without_appearance = 0_usize;
    let mut still_stated = 0_usize;
    let mut lines: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let finding = examine(&document);
        if finding.by_subtype.is_empty() {
            continue;
        }
        with_any = with_any.saturating_add(1);
        if finding.still_stated > 0 {
            with_drawable = with_drawable.saturating_add(1);
        }
        without_appearance = without_appearance.saturating_add(finding.without_appearance);
        still_stated = still_stated.saturating_add(finding.still_stated);
        let mut named: Vec<String> = Vec::new();
        for (subtype, count) in &finding.by_subtype {
            let total = totals.entry(subtype.clone()).or_default();
            *total = total.saturating_add(*count);
            named.push(format!("{count} {subtype}"));
        }
        lines.push(format!(
            "  {path}: {} of no area ({}), {} with no /AP, {} of those still stating their own \
             geometry",
            named.len(),
            named.join(", "),
            finding.without_appearance,
            finding.still_stated
        ));
    }

    println!("{opened} document(s) opened");
    println!("  {with_any} state an annotation whose /Rect covers no area");
    println!("    {without_appearance} such annotation(s) state no /AP at all");
    println!(
        "    {still_stated} of those state the entry their own clause puts in default user \
         space, over {with_drawable} document(s)"
    );
    for (subtype, count) in &totals {
        println!("      {count} /{subtype}");
    }
    for line in &lines {
        println!("{line}");
    }
}

/// Walks one document's pages, counting the annotations whose rectangle covers no area.
fn examine(document: &Document) -> Finding {
    let mut finding = Finding::default();
    let Ok(catalog) = document.catalog() else {
        return finding;
    };
    let root = document.get_key(&catalog, "Pages");
    let Some(root) = root.as_dict() else {
        return finding;
    };
    let mut visited = 0_usize;
    walk(document, root, 0, &mut visited, &mut finding);
    finding
}

/// Descends the page tree, reading each leaf's `/Annots`.
fn walk(
    document: &Document,
    node: &Dictionary,
    depth: usize,
    visited: &mut usize,
    finding: &mut Finding,
) {
    if depth > MAX_TREE_DEPTH || *visited > MAX_NODES_VISITED {
        return;
    }
    *visited = visited.saturating_add(1);

    let kids = document.get_key(node, "Kids");
    let Some(kids) = kids.as_array() else {
        page(document, node, finding);
        return;
    };
    for kid in kids {
        let kid = document.resolve(kid);
        let Some(kid) = kid.as_dict() else { continue };
        walk(document, kid, depth.saturating_add(1), visited, finding);
    }
}

/// Counts one page's annotations.
fn page(document: &Document, page: &Dictionary, finding: &mut Finding) {
    let annots = document.get_key(page, "Annots");
    let Some(annots) = annots.as_array() else {
        return;
    };
    for entry in annots {
        let entry = document.resolve(entry);
        let Some(annotation) = entry.as_dict() else {
            continue;
        };
        if !covers_no_area(document, annotation) {
            continue;
        }
        let subtype = document
            .get_key(annotation, "Subtype")
            .as_name()
            .map_or_else(
                || "(none)".to_owned(),
                |name| String::from_utf8_lossy(name.as_bytes()).into_owned(),
            );
        let seen = finding.by_subtype.entry(subtype.clone()).or_default();
        *seen = seen.saturating_add(1);
        if !matches!(document.get_key(annotation, "AP"), Object::Null) {
            continue;
        }
        finding.without_appearance = finding.without_appearance.saturating_add(1);
        let Some(entry) = stated_in_user_space(subtype.as_bytes()) else {
            continue;
        };
        if document
            .get_key(annotation, entry)
            .as_array()
            .is_some_and(|values| !values.is_empty())
        {
            finding.still_stated = finding.still_stated.saturating_add(1);
        }
    }
}

/// Whether `/Rect` states a rectangle of no width or no height.
///
/// The same condition `pdf_model::annotation::is_empty` applies, written from Table 166 rather
/// than taken from it, and wider than the `/AP` row's bullet in the same way: an array whose
/// pairs are equal is a point, and one axis collapsing is enough to leave nothing to scale onto.
fn covers_no_area(document: &Document, annotation: &Dictionary) -> bool {
    let rect = document.get_key(annotation, "Rect");
    let Some(values) = rect.as_array() else {
        return false;
    };
    let mut numbers = [0.0_f64; 4];
    if values.len() != 4 {
        return false;
    }
    for (slot, value) in numbers.iter_mut().zip(values) {
        let Some(number) = document.resolve(value).as_number() else {
            return false;
        };
        *slot = number;
    }
    let width = (numbers[2] - numbers[0]).abs();
    let height = (numbers[3] - numbers[1]).abs();
    width <= 0.0 || height <= 0.0
}
