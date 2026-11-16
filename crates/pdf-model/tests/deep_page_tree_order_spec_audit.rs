//! Source-derived boundary witness for §7.7.3.1's “any form of tree structure” reader requirement.
//! A valid tree at the implementation boundary must retain its /Kids page sequence.
#![expect(clippy::panic, reason = "test fixture preconditions must fail visibly")]

use pdf_model::Pages;
use pdf_syntax::Document;

fn tree_with_page_leaf_depth(leaf_depth: usize) -> Vec<u8> {
    assert!((1..=128).contains(&leaf_depth));
    let catalog = 1_u32;
    let root = 2_u32;
    let pages = [3_u32, 4_u32];
    let [first_page, second_page] = pages;
    let mut next_id = 5_u32;
    let mut nodes = vec![root];
    for _ in 1..leaf_depth {
        nodes.push(next_id);
        next_id = next_id.saturating_add(1);
    }
    let terminal = nodes
        .last()
        .copied()
        .unwrap_or_else(|| panic!("root exists"));
    let mut objects: Vec<(u32, String)> = Vec::new();
    objects.push((catalog, format!("<< /Type /Catalog /Pages {root} 0 R >>")));
    for (index, node) in nodes.iter().copied().enumerate() {
        let parent = nodes.get(index.wrapping_sub(1)).copied();
        let parent_entry = parent.map_or(String::new(), |id| format!(" /Parent {id} 0 R"));
        let kids = if node == terminal {
            // Tree order [4, 3] deliberately disagrees with object-number scan order [3, 4].
            format!("[{second_page} 0 R {first_page} 0 R]")
        } else {
            let child = nodes
                .get(index.saturating_add(1))
                .copied()
                .unwrap_or_else(|| panic!("nonterminal has child"));
            format!("[{child} 0 R]")
        };
        objects.push((
            node,
            format!("<< /Type /Pages{parent_entry} /Kids {kids} /Count 2 >>"),
        ));
    }
    let red = b"1 0 0 rg 0 0 10 10 re f\n";
    let blue = b"0 0 1 rg 0 0 10 10 re f\n";
    let red_stream = next_id;
    let blue_stream = next_id.saturating_add(1);
    objects.push((first_page, format!(
        "<< /Type /Page /Parent {terminal} 0 R /MediaBox [0 0 111 111] /Resources << >> /Contents {red_stream} 0 R >>"
    )));
    objects.push((second_page, format!(
        "<< /Type /Page /Parent {terminal} 0 R /MediaBox [0 0 222 222] /Resources << >> /Contents {blue_stream} 0 R >>"
    )));
    objects.push((
        red_stream,
        format!(
            "<< /Length {} >>\nstream\n{}endstream",
            red.len(),
            String::from_utf8_lossy(red)
        ),
    ));
    objects.push((
        blue_stream,
        format!(
            "<< /Length {} >>\nstream\n{}endstream",
            blue.len(),
            String::from_utf8_lossy(blue)
        ),
    ));
    objects.sort_by_key(|(id, _)| *id);

    let size = blue_stream.saturating_add(1);
    let mut bytes = b"%PDF-2.0\n".to_vec();
    let mut offsets = vec![0_usize; size as usize];
    for (id, body) in objects {
        offsets[id as usize] = bytes.len();
        bytes.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
    bytes.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        if *offset == 0 {
            bytes.extend_from_slice(b"0000000000 00000 f \n");
        } else {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root {catalog} 0 R >>\nstartxref\n{xref}\n%%EOF\n")
            .as_bytes(),
    );
    bytes
}

fn page_sequence(leaf_depth: usize) -> (Vec<u32>, Vec<Vec<u8>>, Option<usize>) {
    let document = Document::open(tree_with_page_leaf_depth(leaf_depth))
        .unwrap_or_else(|e| panic!("exact-xref PDF opens: {e:?}"));
    assert!(!document.was_recovered(), "fixture must use its exact xref");
    let pages = Pages::new(&document);
    let mut ids = Vec::new();
    let mut contents = Vec::new();
    for index in 0..pages.len() {
        let page = pages
            .get(index)
            .unwrap_or_else(|| panic!("page sequence entry"));
        ids.push(
            page.id
                .unwrap_or_else(|| panic!("page object identity"))
                .number,
        );
        let (content, issues) = page.content_with_report(&document);
        assert!(
            issues.is_empty(),
            "page {index} content should be intact: {issues:?}"
        );
        contents.push(content);
    }
    (ids, contents, pages.found_by_scanning())
}

#[test]
fn depth_64_control_keeps_kids_order_and_page_content() {
    // Pages are at depth 64, the current bound; tree order remains [4,3], red follows blue.
    let (ids, content, scan) = page_sequence(64);
    assert_eq!(ids, [4, 3], "scan fallback: {scan:?}");
    assert_eq!(
        content,
        [
            b"0 0 1 rg 0 0 10 10 re f\n\n".to_vec(),
            b"1 0 0 rg 0 0 10 10 re f\n\n".to_vec()
        ]
    );
    assert_eq!(scan, None);
}

#[test]
fn valid_depth_65_tree_keeps_kids_order_and_corresponding_page_content() {
    // Current traversal bounds can route this valid shape through a disclosed object scan.
    // The scan signal is included in assertion diagnostics; source-defined order is still [4,3].
    let (ids, content, scan) = page_sequence(65);
    assert_eq!(ids, [4, 3], "found_by_scanning={scan:?}");
    assert_eq!(
        content,
        [
            b"0 0 1 rg 0 0 10 10 re f\n\n".to_vec(),
            b"1 0 0 rg 0 0 10 10 re f\n\n".to_vec()
        ],
        "found_by_scanning={scan:?}"
    );
}
