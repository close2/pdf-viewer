//! How many pages state no usable `/MediaBox` anywhere in their ancestry.
//!
//! ISO 32000-2 §7.7.3.3 Table 31 makes the entry **required** and **inheritable**:
//!
//! > ( Required; inheritable ) A rectangle (see 7.9.5, "Rectangles"), expressed in default user
//! > space units, that shall define the boundaries of the physical medium on which the page shall
//! > be displayed or printed (see 14.11.2, "Page boundaries").
//!
//! and §7.7.3.4 says where an inheritable required entry may be written instead:
//!
//! > If such an attribute is omitted from a page object, its value shall be inherited from an
//! > ancestor node in the page tree. If the attribute is a required one, a value shall be
//! > supplied in an ancestor node.
//!
//! So a page with no `/MediaBox` on itself and none on any ancestor is a file that broke both
//! sentences, and the standard states no recovery for it. `pdf_model::Page` substitutes
//! `Page::DEFAULT_MEDIA_BOX`, which is a **choice** rather than a reading — and the size of the
//! population that choice decides is what this counts.
//!
//! **Three** ways to reach it, counted apart because they are different mistakes by the producer:
//! no ancestor states the entry at all; one states it and the array is not four finite numbers
//! (§7.9.5); or one states four finite numbers enclosing no area. The third is the one §7.9.5
//! makes easy to miss, because it says outright that the rectangle *type* admits such a value —
//!
//! > NOTE Rectangles can have a width of zero or height of zero.
//!
//! — so the array is a rectangle, and the entry that is not satisfied is Table 31's: a rectangle
//! of zero extent does not "define the boundaries of the physical medium", and a page measured
//! against one has no area for any of its marks to land in. §14.11.2's other four boxes have
//! handled the empty case since they were read, because each of them *defaults* to a larger box
//! and can fall back to it; the media box is the one they all default to and the only one with
//! nothing behind it.
//!
//! It reads the tree with `pdf_syntax` alone rather than through [`pdf_model::Pages`], because a
//! census whose predicate is the code under test measures the code rather than the corpus
//! (`doc/HANDOVER.md` trap 8).
//!
//! ```sh
//! cargo run --release -p pdf-model --example media_box_census -- <file.pdf>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_syntax::{Dictionary, Document, Object};

/// The same bounds `pdf_model::page` walks the tree under, so that this counts what that walk
/// would reach rather than what an unbounded one would.
const MAX_TREE_DEPTH: usize = 64;
/// As above: a `/Kids` cycle is what stops this walk, not the tree's size.
const MAX_NODES_VISITED: usize = 100_000;

/// What one document's page tree says about the question.
#[derive(Default)]
struct Finding {
    /// Leaves reached by the walk.
    pages: usize,
    /// Leaves with no `/MediaBox` on themselves or any ancestor.
    absent: usize,
    /// Leaves whose nearest stated `/MediaBox` is not four finite numbers.
    unreadable: usize,
    /// Leaves whose nearest stated `/MediaBox` is four finite numbers enclosing no area.
    empty: usize,
    /// Whether the *first* leaf is one of the two above, which is the page every gate draws.
    first_page: bool,
    /// Whether any such leaf states `/Contents`, so that the guessed box places real marks.
    with_contents: usize,
    /// Such leaves that state one of §14.11.2's *other* four boxes.
    ///
    /// The question behind the substitution: does the file state a rectangle somewhere that a
    /// reader could take the page's extent from instead of guessing? Table 31 defaults the
    /// crop box to the media box and §14.11.2.1 intersects the other three with it, so the
    /// arrow runs the wrong way and none of them is defined without one — but a producer that
    /// wrote a `/CropBox` and no `/MediaBox` has still said how big its page is, and a
    /// substitution that ignored it would be discarding the file's own words.
    with_another_box: usize,
}

fn main() {
    let mut opened = 0_usize;
    let mut documents_absent = 0_usize;
    let mut documents_unreadable = 0_usize;
    let mut documents_empty = 0_usize;
    let mut documents_first_page = 0_usize;
    let mut pages = 0_usize;
    let mut pages_absent = 0_usize;
    let mut pages_unreadable = 0_usize;
    let mut pages_empty = 0_usize;
    let mut pages_with_contents = 0_usize;
    let mut pages_with_another_box = 0_usize;
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
        pages = pages.saturating_add(finding.pages);
        if finding.absent == 0 && finding.unreadable == 0 && finding.empty == 0 {
            continue;
        }
        pages_absent = pages_absent.saturating_add(finding.absent);
        pages_unreadable = pages_unreadable.saturating_add(finding.unreadable);
        pages_empty = pages_empty.saturating_add(finding.empty);
        pages_with_contents = pages_with_contents.saturating_add(finding.with_contents);
        pages_with_another_box = pages_with_another_box.saturating_add(finding.with_another_box);
        if finding.absent > 0 {
            documents_absent = documents_absent.saturating_add(1);
        }
        if finding.unreadable > 0 {
            documents_unreadable = documents_unreadable.saturating_add(1);
        }
        if finding.empty > 0 {
            documents_empty = documents_empty.saturating_add(1);
        }
        if finding.first_page {
            documents_first_page = documents_first_page.saturating_add(1);
        }
        lines.push(format!(
            "  {path}: {} of {} page(s) with no usable /MediaBox ({} absent, {} unreadable, \
             {} empty), {} of them stating /Contents{}",
            finding
                .absent
                .saturating_add(finding.unreadable)
                .saturating_add(finding.empty),
            finding.pages,
            finding.absent,
            finding.unreadable,
            finding.empty,
            finding.with_contents,
            if finding.first_page {
                ", including page one"
            } else {
                ""
            }
        ));
    }

    println!("{opened} document(s) opened, {pages} page(s) walked");
    println!("  {pages_absent} page(s) with no /MediaBox anywhere in the ancestry");
    println!("  {pages_unreadable} page(s) whose nearest /MediaBox is not four finite numbers");
    println!("  {pages_empty} page(s) whose nearest /MediaBox encloses no area");
    println!("  {pages_with_contents} of those page(s) state /Contents");
    println!("  {pages_with_another_box} of those page(s) state another of §14.11.2's boxes");
    println!("  {documents_absent} document(s) with at least one of the first kind");
    println!("  {documents_unreadable} document(s) with at least one of the second");
    println!("  {documents_empty} document(s) with at least one of the third");
    println!("  {documents_first_page} document(s) where page one is one of them");
    for line in &lines {
        println!("{line}");
    }
}

/// Walks one document's page tree, carrying the nearest stated `/MediaBox` down it.
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
    walk(
        document,
        root,
        State::default(),
        0,
        &mut visited,
        &mut finding,
    );
    finding
}

/// What the ancestors said about `/MediaBox`, carried down the tree.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum State {
    /// No ancestor stated the entry.
    #[default]
    Absent,
    /// The nearest ancestor stating it wrote something that is not four finite numbers.
    Unreadable,
    /// The nearest ancestor stating it wrote four finite numbers enclosing no area.
    Empty,
    /// The nearest ancestor stating it wrote a rectangle with area.
    Usable,
}

/// Descends the tree, classifying each leaf by what its ancestry stated.
fn walk(
    document: &Document,
    node: &Dictionary,
    state: State,
    depth: usize,
    visited: &mut usize,
    finding: &mut Finding,
) {
    if depth > MAX_TREE_DEPTH || *visited > MAX_NODES_VISITED {
        return;
    }
    *visited = visited.saturating_add(1);

    let state = match document.get_key(node, "MediaBox") {
        Object::Null => state,
        stated => classify(document, &stated),
    };

    let kids = document.get_key(node, "Kids");
    let Some(kids) = kids.as_array() else {
        // A dictionary with no `/Kids` is where `pdf_model::page`'s walk stops, so it is a
        // page for this count whatever its `/Type` says.
        finding.pages = finding.pages.saturating_add(1);
        if state == State::Usable {
            return;
        }
        let first = finding.pages == 1;
        finding.first_page |= first;
        match state {
            State::Unreadable => finding.unreadable = finding.unreadable.saturating_add(1),
            State::Empty => finding.empty = finding.empty.saturating_add(1),
            State::Absent | State::Usable => finding.absent = finding.absent.saturating_add(1),
        }
        if !matches!(document.get_key(node, "Contents"), Object::Null) {
            finding.with_contents = finding.with_contents.saturating_add(1);
        }
        if ["CropBox", "BleedBox", "TrimBox", "ArtBox"]
            .iter()
            .any(|key| !matches!(document.get_key(node, key), Object::Null))
        {
            finding.with_another_box = finding.with_another_box.saturating_add(1);
        }
        return;
    };

    for kid in kids {
        let kid = document.resolve(kid);
        let Some(kid) = kid.as_dict() else { continue };
        walk(
            document,
            kid,
            state,
            depth.saturating_add(1),
            visited,
            finding,
        );
    }
}

/// Which of the three kinds a stated `/MediaBox` is.
///
/// §7.9.5's rectangle is four numbers, each finite; Table 31 then asks that this one bound a
/// physical medium, which one of zero extent does not. So the two questions are asked in that
/// order and answered separately.
fn classify(document: &Document, object: &Object) -> State {
    let Some(items) = object.as_array() else {
        return State::Unreadable;
    };
    if items.len() < 4 {
        return State::Unreadable;
    }
    let mut corners = [0f64; 4];
    for (slot, item) in corners.iter_mut().zip(items) {
        match document.resolve(item).as_number() {
            Some(number) if number.is_finite() => *slot = number,
            _ => return State::Unreadable,
        }
    }
    // The corners may be given in either order (§7.9.5), so the extent is the absolute
    // difference rather than a signed one.
    if (corners[2] - corners[0]).abs() > 0.0 && (corners[3] - corners[1]).abs() > 0.0 {
        State::Usable
    } else {
        State::Empty
    }
}
