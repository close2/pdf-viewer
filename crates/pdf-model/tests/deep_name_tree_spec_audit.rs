//! Valid indirect §7.9.6 tree: public destination lookup distinguishes byte prefixes and
//! composed/decomposed multibyte keys while preserving the existing ASCII controls.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref fixture and explicit public-reader expectations"
)]

use std::io::Write as _;

use pdf_model::{Pages, destination::Destination};
use pdf_syntax::{Document, Object};

fn pdf() -> Vec<u8> {
    // The leaf keys are sorted by their encoded bytes:
    // A < a < a-NUL < b < e+combining-acute < z < composed-é.
    // Every intermediate and leaf has exact Limits; Kids and destination values are indirect.
    let objects: [&[u8]; 19] = [
        b"<< /Type /Catalog /Pages 2 0 R /Names << /Dests 5 0 R >> >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        b"<< /Kids [6 0 R 11 0 R] >>",
        b"<< /Limits [<41> <6100>] /Kids [7 0 R] >>",
        b"<< /Limits [<41> <6100>] /Names [<41> 8 0 R <61> 9 0 R <6100> 10 0 R] >>",
        b"[3 0 R /Fit]",
        b"[4 0 R /Fit]",
        b"[3 0 R /Fit]",
        b"<< /Limits [<62> <C3A9>] /Kids [12 0 R 13 0 R 14 0 R 15 0 R] >>",
        b"<< /Limits [<62> <62>] /Names [<62> 16 0 R] >>",
        b"<< /Limits [<65CC81> <65CC81>] /Names [<65CC81> 17 0 R] >>",
        b"<< /Limits [<7A> <7A>] /Names [<7A> 18 0 R] >>",
        b"<< /Limits [<C3A9> <C3A9>] /Names [<C3A9> 19 0 R] >>",
        b"[4 0 R /Fit]",
        b"[4 0 R /Fit]",
        b"[3 0 R /Fit]",
        b"[3 0 R /Fit]",
    ];

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
fn named_destination_lookup_descends_valid_indirect_nodes_and_uses_exact_byte_keys() {
    let bytes = pdf();
    let document = Document::open(bytes).expect("generated PDF parses");
    assert!(!document.was_recovered(), "all xref offsets are exact");
    assert!(
        !document.cross_reference_chain_was_cut(),
        "the intact classic xref is not being accepted through chain truncation"
    );
    let pages = Pages::new(&document);

    // Preserve the original ASCII behavior and add distinct direct-byte targets for every
    // extension/canonical-equivalence case. These expectations are literal destination values.
    for (key, expected_page) in [
        (b"A".as_slice(), 0),
        (b"a", 1),
        (b"a\0", 0),
        (b"b", 1),
        (b"e\xCC\x81", 1),
        (b"z", 0),
        (b"\xC3\xA9", 0),
    ] {
        let destination = Destination::read(&document, &Object::String(key.to_vec().into()))
            .expect("key is present in a valid leaf");
        assert_eq!(
            destination.page_index(&document, &pages),
            Some(expected_page),
            "{key:?}"
        );
    }
    for absent in [b"B".as_slice(), b"aa", b"e", b"\xC3\x88"] {
        assert!(
            Destination::read(&document, &Object::String(absent.to_vec().into())).is_none(),
            "absent distinct bytes must not match: {absent:?}"
        );
    }
}
