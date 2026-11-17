//! Existing valid multilevel §7.9.7 fixture, with generic pair order exposed separately.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref fixture and explicit public-reader expectations"
)]

use std::io::Write as _;

use pdf_model::{Pages, page_label::PageLabels};
use pdf_syntax::{Document, tree::number_pairs};

fn pdf() -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R /PageLabels 3 0 R >>".to_vec(),
        b"<< /Type /Pages /Count 11 /Kids [4 0 R 5 0 R 6 0 R 7 0 R 8 0 R 9 0 R 10 0 R 11 0 R 12 0 R 13 0 R 14 0 R] >>".to_vec(),
        b"<< /Kids [15 0 R 19 0 R] >>".to_vec(),
    ];
    for _ in 0..11 {
        objects.push(
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_vec(),
        );
    }
    objects.extend([
        b"<< /Limits [0 2] /Kids [16 0 R] >>".to_vec(),
        b"<< /Limits [0 2] /Nums [0 17 0 R 2 18 0 R] >>".to_vec(),
        b"<< /S /D /St 4 >>".to_vec(),
        b"<< /S /r >>".to_vec(),
        b"<< /Limits [10 10] /Kids [20 0 R] >>".to_vec(),
        b"<< /Limits [10 10] /Nums [10 21 0 R] >>".to_vec(),
        b"<< /S /A /St 26 >>".to_vec(),
    ]);
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let size = objects.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out
}

#[test]
fn page_labels_enumerate_numeric_ranges_from_valid_indirect_leaves() {
    let document = Document::open(pdf()).expect("generated PDF parses");
    assert!(
        !document.was_recovered(),
        "all classic-xref offsets are exact"
    );
    assert!(
        !document.cross_reference_chain_was_cut(),
        "the intact classic xref is not being accepted through chain truncation"
    );
    assert_eq!(Pages::new(&document).len(), 11);

    let catalog = document.catalog().expect("catalog resolves");
    let tree_object = document.get_key(&catalog, "PageLabels");
    let tree = tree_object
        .as_dict()
        .expect("PageLabels is a number-tree dictionary");
    let pairs = number_pairs(tree, &|object| document.resolve(object));
    assert_eq!(
        pairs.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
        [0, 2, 10],
        "generic number-tree traversal preserves the source's ascending numeric order"
    );

    // This consumer sorts range starts itself, so labels verify range semantics, not generic
    // number_pairs order. The assertion above is the independent traversal-order observation.
    let labels = PageLabels::read(&document);
    assert_eq!(labels.label(0).as_deref(), Some("4"));
    assert_eq!(labels.label(1).as_deref(), Some("5"));
    assert_eq!(labels.label(2).as_deref(), Some("i"));
    assert_eq!(labels.label(9).as_deref(), Some("viii"));
    assert_eq!(labels.label(10).as_deref(), Some("Z"));
}
