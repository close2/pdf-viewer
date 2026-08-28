//! How many documents state a form field dictionary with no `/T`, and where in the tree it is.
//!
//! ISO 32000-2 §12.7.4.1 Table 226 prints `/T` as
//!
//! > (Required) The partial field name (see 12.7.4.2, "Field names").
//!
//! and Errata Collection 3's Issue #28 strikes the requirement level and writes *Optional* in its
//! place — because §12.7.4.2 already describes the entry's absence:
//!
//! > A field dictionary that does not have a partial field name ( T entry) of its own shall not be
//! > considered a field but simply a Widget annotation.
//!
//! So the cell and the paragraph contradicted each other, and the erratum takes the contradiction
//! out on the cell's side. What that changes for a reader is which files are conforming: a
//! dictionary with no `/T` is one now, so what the paragraph says about it decides a page rather
//! than being a repair to a malformed file.
//!
//! The paragraph's rule is *relative* — such a dictionary belongs to the field its ancestors name.
//! At the root of `/AcroForm /Fields` there are no ancestors, so a root entry stating no `/T`
//! belongs to no field at all, and two of them belong to no field *together*. That is the
//! population this census counts, in four widths:
//!
//! - every dictionary reached from `/Fields` that states no `/T`, wherever it is;
//! - those with no `/T` anywhere on the path from a root of `/Fields` down to them, which are the
//!   ones §12.7.4.2 leaves nameless;
//! - documents where two or more *leaves* are nameless in that sense, which is where a reader
//!   keying widgets by their field's name has two annotations under one key and no field;
//! - and of the nameless roots, the ones stating no `/Parent` either. §12.7.3 makes `/Fields`
//!   "[a]n array of references to the document's root fields (those with no ancestors in the field
//!   hierarchy)", so a `/Parent` on one of them is the file contradicting that — and it is also
//!   the file saying which field the entry belongs to, which
//!   [`pdf_model::view::widgets_by_field_name`] takes as a recovery. This last count is therefore
//!   the population that recovery cannot reach.
//!
//! It walks `/Fields` with `pdf_syntax` alone rather than through
//! [`pdf_model::view::widgets_by_field_name`], because a census whose predicate is the code under
//! test measures the code rather than the corpus (`doc/HANDOVER.md` trap 8).
//!
//! ```sh
//! cargo run --release -p pdf-model --example unnamed_field_census -- <file.pdf>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_syntax::{Document, Object, ObjectId};

/// The bound `pdf_model::view::widgets_by_field_name` walks the field tree under, so that this
/// counts what that walk would reach rather than what an unbounded one would.
const MAX_FIELD_DEPTH: usize = 32;

/// What one document's field tree says about the question.
#[derive(Default)]
struct Finding {
    /// Dictionaries reached from `/Fields` that state no `/T` of their own.
    untitled: usize,
    /// Of those, the ones with no `/T` on the whole path from a root of `/Fields`.
    nameless: usize,
    /// Of the nameless, the ones that are leaves — no `/Kids`, so a widget in their own right.
    nameless_leaves: usize,
    /// Whether a nameless dictionary is itself an entry of `/Fields`.
    nameless_at_root: bool,
    /// Of the nameless entries of `/Fields`, the ones stating no `/Parent` — so no ancestry a
    /// reader could recover a name from.
    nameless_root_orphans: usize,
}

fn main() {
    let mut opened = 0_usize;
    let mut with_form = 0_usize;
    let mut with_untitled = 0_usize;
    let mut with_nameless = 0_usize;
    let mut with_nameless_root = 0_usize;
    let mut with_orphan_root = 0_usize;
    let mut colliding = 0_usize;
    let mut lines: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let Some(finding) = examine(&document) else {
            continue;
        };
        with_form = with_form.saturating_add(1);
        if finding.untitled == 0 {
            continue;
        }
        with_untitled = with_untitled.saturating_add(1);
        if finding.nameless > 0 {
            with_nameless = with_nameless.saturating_add(1);
        }
        if finding.nameless_at_root {
            with_nameless_root = with_nameless_root.saturating_add(1);
        }
        if finding.nameless_root_orphans > 0 {
            with_orphan_root = with_orphan_root.saturating_add(1);
        }
        if finding.nameless_leaves > 1 {
            colliding = colliding.saturating_add(1);
        }
        lines.push(format!(
            "  {path}: {} without /T, {} of them nameless{}, {} of those leaves, \
             {} nameless in /Fields with no /Parent",
            finding.untitled,
            finding.nameless,
            if finding.nameless_at_root {
                " (one at the root of /Fields)"
            } else {
                ""
            },
            finding.nameless_leaves,
            finding.nameless_root_orphans
        ));
    }

    println!("{opened} document(s) opened");
    println!("  {with_form} with an /AcroForm stating a /Fields array");
    println!("  {with_untitled} with a field dictionary stating no /T");
    println!("    of which {with_nameless} have one no ancestor names either");
    println!("    of which {with_nameless_root} state one directly in /Fields");
    println!("      of which {with_orphan_root} state no /Parent on it either");
    println!("  {colliding} with two or more nameless leaves, which share one empty name");
    for line in &lines {
        println!("{line}");
    }
}

/// Walks one document's field tree. `None` where the document states no `/AcroForm /Fields`.
fn examine(document: &Document) -> Option<Finding> {
    let catalog = document.catalog().ok()?;
    let form = document.get_key(&catalog, "AcroForm");
    let form = form.as_dict()?;
    let fields = document.get_key(form, "Fields");
    let fields = fields.as_array().map(<[Object]>::to_vec)?;

    let mut finding = Finding::default();
    let mut seen = std::collections::BTreeSet::new();
    for field in &fields {
        walk(document, field, false, true, &mut seen, 0, &mut finding);
    }
    Some(finding)
}

/// Descends one node, recording what it says about its own name.
///
/// `named` is whether anything on the path from `/Fields` to this node stated a `/T`, which is
/// what decides whether §12.7.4.2 leaves this dictionary with a fully qualified name at all.
fn walk(
    document: &Document,
    node: &Object,
    named: bool,
    is_root: bool,
    seen: &mut std::collections::BTreeSet<ObjectId>,
    depth: usize,
    finding: &mut Finding,
) {
    if depth > MAX_FIELD_DEPTH {
        return;
    }
    let Some(id) = node.as_reference() else {
        return;
    };
    if !seen.insert(id) {
        return;
    }
    let resolved = document.get(id);
    let Some(dict) = resolved.as_dict() else {
        return;
    };

    let states_title = matches!(document.get_key(dict, "T"), Object::String(_));
    if !states_title {
        finding.untitled = finding.untitled.saturating_add(1);
        if !named {
            finding.nameless = finding.nameless.saturating_add(1);
            finding.nameless_at_root |= is_root;
            if is_root && document.get_key(dict, "Parent").as_dict().is_none() {
                finding.nameless_root_orphans = finding.nameless_root_orphans.saturating_add(1);
            }
        }
    }
    let named = named || states_title;

    let kids = document.get_key(dict, "Kids");
    match kids.as_array().map(<[Object]>::to_vec) {
        Some(kids) if !kids.is_empty() => {
            for kid in &kids {
                walk(
                    document,
                    kid,
                    named,
                    false,
                    seen,
                    depth.saturating_add(1),
                    finding,
                );
            }
        }
        _ => {
            if !named {
                finding.nameless_leaves = finding.nameless_leaves.saturating_add(1);
            }
        }
    }
}
