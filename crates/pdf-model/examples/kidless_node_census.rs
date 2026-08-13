//! How many documents state a page tree node with no `/Kids`, and what is beside it.
//!
//! ISO 32000-2 §7.7.3.2 Table 30 makes `/Kids` **required** of a page tree node:
//!
//! > ( Required ) An array of indirect references to the immediate children of this node. The
//! > children shall only be page objects or other page tree nodes.
//!
//! and `/Count` beside it, as the number of leaf nodes descended from the node — an entry the
//! same table's own NOTE calls redundant, because the `/Kids` arrays determine it.
//!
//! So a dictionary that states `/Type /Pages` and no `/Kids` has said in one breath that it is
//! a node and that it has no children, which is a contradiction the file has to be read
//! against. `pdf_model::page` treats *any* dictionary without `/Kids` as a leaf, deliberately
//! — trusting `/Type` instead would drop pages from the files that omit it — and the cost of
//! that is a node drawn as though it were a page.
//!
//! Which reading is right is a question about the population, and this census is the
//! population: how many documents have such a node, whether it is the root of the tree, and
//! whether the file also holds an object declaring `/Type /Page` for §7.7.3.2's tree to be
//! recovered from. It reads the tree with `pdf_syntax` alone rather than through
//! [`pdf_model::Pages`], because a census whose predicate is the code under test measures the
//! code rather than the corpus (`doc/HANDOVER.md` trap 8).
//!
//! ```sh
//! cargo run --release -p pdf-model --example kidless_node_census -- <file.pdf>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// The same bounds `pdf_model::page` walks the tree under, so that this counts what that walk
/// would reach rather than what an unbounded one would.
const MAX_TREE_DEPTH: usize = 64;
/// As above: a `/Kids` cycle is what stops this walk, not the tree's size.
const MAX_NODES_VISITED: usize = 100_000;

/// What one document's page tree says about the question.
#[derive(Default)]
struct Finding {
    /// Nodes reached with no `/Kids` array that state `/Type /Pages`.
    typed: usize,
    /// Nodes reached with no `/Kids` array that state a `/Count`, whatever their `/Type`.
    counted: usize,
    /// Whether the root of the tree is one of them.
    at_root: bool,
    /// Objects in the file declaring `/Type /Page`, which is what a recovery would find.
    declared_pages: usize,
}

fn main() {
    let mut opened = 0_usize;
    let mut with_typed = 0_usize;
    let mut with_counted = 0_usize;
    let mut with_typed_root = 0_usize;
    let mut recoverable = 0_usize;
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
        if finding.typed == 0 && finding.counted == 0 {
            continue;
        }
        if finding.typed > 0 {
            with_typed = with_typed.saturating_add(1);
            if finding.at_root {
                with_typed_root = with_typed_root.saturating_add(1);
            }
            if finding.declared_pages > 0 {
                recoverable = recoverable.saturating_add(1);
            }
        }
        if finding.counted > 0 {
            with_counted = with_counted.saturating_add(1);
        }
        lines.push(format!(
            "  {path}: {} /Type /Pages without /Kids{}, {} stating /Count without /Kids, \
             {} object(s) declaring /Type /Page",
            finding.typed,
            if finding.at_root { " (the root)" } else { "" },
            finding.counted,
            finding.declared_pages
        ));
    }

    println!("{opened} document(s) opened");
    println!("  {with_typed} with a /Type /Pages node stating no /Kids");
    println!("    of which {with_typed_root} at the root of the tree");
    println!("    of which {recoverable} hold an object declaring /Type /Page");
    println!("  {with_counted} with a node stating /Count and no /Kids, whatever its /Type");
    for line in &lines {
        println!("{line}");
    }
}

/// Walks one document's page tree, counting the nodes that state no `/Kids`.
fn examine(document: &Document) -> Finding {
    let mut finding = Finding {
        declared_pages: declared_pages(document),
        ..Finding::default()
    };
    let Ok(catalog) = document.catalog() else {
        return finding;
    };
    let root = document.get_key(&catalog, "Pages");
    let Some(root) = root.as_dict() else {
        return finding;
    };
    let mut visited = 0_usize;
    walk(document, root, 0, &mut visited, &mut finding, true);
    finding
}

/// Descends the tree, recording every node that states no `/Kids`.
fn walk(
    document: &Document,
    node: &Dictionary,
    depth: usize,
    visited: &mut usize,
    finding: &mut Finding,
    is_root: bool,
) {
    if depth > MAX_TREE_DEPTH || *visited > MAX_NODES_VISITED {
        return;
    }
    *visited = visited.saturating_add(1);

    let kids = document.get_key(node, "Kids");
    let Some(kids) = kids.as_array() else {
        let typed = document
            .get_key(node, "Type")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Pages");
        let counts = !matches!(document.get_key(node, "Count"), Object::Null);
        if typed {
            finding.typed = finding.typed.saturating_add(1);
            finding.at_root |= is_root;
        }
        if counts {
            finding.counted = finding.counted.saturating_add(1);
        }
        return;
    };

    for kid in kids {
        let kid = document.resolve(kid);
        let Some(kid) = kid.as_dict() else { continue };
        walk(
            document,
            kid,
            depth.saturating_add(1),
            visited,
            finding,
            false,
        );
    }
}

/// How many objects in the file declare `/Type /Page`.
///
/// Table 31 makes `/Type` required of a page object, so this is what `pdf_model::page`'s
/// recovery scan would find where the tree yields nothing.
fn declared_pages(document: &Document) -> usize {
    let mut found = 0_usize;
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId::new(number, 0));
        let Some(dict) = object.as_dict() else {
            continue;
        };
        if document
            .get_key(dict, "Type")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Page")
        {
            found = found.saturating_add(1);
        }
    }
    found
}
