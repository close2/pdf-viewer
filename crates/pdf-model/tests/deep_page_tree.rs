//! §7.7.3.1's page tree at depths no balanced tree reaches.
//!
//! §7.7.3.1 lets a producer use "any form of tree structure", so a tree that hangs each
//! intermediate node under the last is valid however deep it is, and its `/Kids` order is the
//! page order. These fixtures are such chains, with the two pages named in the reverse of their
//! object numbers, so that a reader falling back to a scan of `/Type /Page` declarations — which
//! answers in ascending object number — gives the wrong order and says it scanned.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixtures whose arithmetic is bounded by the depths the tests pass"
)]

use std::fmt::Write as _;

use pdf_model::Pages;
use pdf_syntax::Document;

/// A catalog (1), a chain of `depth` page-tree nodes from 5 upwards with the root first, and two
/// pages (3 and 4) named `[4 3]` by the last node.
fn chain(depth: u32) -> Vec<u8> {
    let first_node = 5_u32;
    let last_node = first_node + depth - 1;
    let mut objects: Vec<(u32, String)> = vec![
        (1, format!("<< /Type /Catalog /Pages {first_node} 0 R >>")),
        (
            3,
            format!("<< /Type /Page /Parent {last_node} 0 R /MediaBox [0 0 10 10] >>"),
        ),
        (
            4,
            format!("<< /Type /Page /Parent {last_node} 0 R /MediaBox [0 0 20 20] >>"),
        ),
    ];
    for node in first_node..=last_node {
        let parent = if node == first_node {
            String::new()
        } else {
            format!(" /Parent {} 0 R", node - 1)
        };
        let kids = if node == last_node {
            "[4 0 R 3 0 R]".to_owned()
        } else {
            format!("[{} 0 R]", node + 1)
        };
        objects.push((
            node,
            format!("<< /Type /Pages{parent} /Kids {kids} /Count 2 >>"),
        ));
    }
    objects.sort_by_key(|(number, _)| *number);
    let size = last_node + 1;
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = vec![None; usize::try_from(size).expect("a small count")];
    for (number, body) in &objects {
        offsets[usize::try_from(*number).expect("a small number")] = Some(out.len());
        let _ = writeln!(out, "{number} 0 obj\n{body}\nendobj");
    }
    let xref = out.len();
    let _ = writeln!(out, "xref\n0 {size}");
    for offset in &offsets {
        match offset {
            Some(offset) => {
                let _ = writeln!(out, "{offset:010} 00000 n ");
            }
            None => out.push_str("0000000000 65535 f \n"),
        }
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The pages' object numbers in the order the reader gives them, and whether it scanned.
fn order(depth: u32) -> (Vec<u32>, Option<usize>) {
    let document = Document::open(chain(depth)).expect("the exact-xref fixture opens");
    assert!(!document.was_recovered(), "the fixture's xref is exact");
    let pages = Pages::new(&document);
    let numbers = (0..pages.len())
        .map(|index| {
            pages
                .get(index)
                .and_then(|page| page.id)
                .expect("each page has its object")
                .number
        })
        .collect();
    (numbers, pages.found_by_scanning())
}

/// Two levels: the control, which any reader walks.
#[test]
fn a_shallow_chain_keeps_its_order() {
    assert_eq!(order(2), (vec![4, 3], None));
}

/// A thousand levels, past the bound the walks once had and inside the one they have: read as
/// the tree, in `/Kids` order, on an ordinary test thread's stack.
#[test]
fn a_thousand_level_tree_keeps_its_order() {
    assert_eq!(order(1000), (vec![4, 3], None));
}

/// Past the bound, the tree is not read as a tree with fewer pages: both pages are still found,
/// and the reader says it found them by scanning rather than presenting the scan as the tree.
#[test]
fn a_tree_past_the_bound_is_scanned_and_says_so() {
    let (numbers, scanned) = order(1100);
    assert_eq!(numbers.len(), 2, "both pages are found");
    assert!(scanned.is_some(), "the scan is reported: {numbers:?}");
}
