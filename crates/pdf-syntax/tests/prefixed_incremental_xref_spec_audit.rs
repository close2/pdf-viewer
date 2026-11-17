//! §7.5.2 offsets begin at the PDF marker, including after a prefix; §7.5.6 keeps deletions.

use std::fmt::Write as _;

use pdf_syntax::{Document, Object, ObjectId};

fn fixture(prefix_len: usize, delete: bool) -> Vec<u8> {
    let mut body = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources <<>> >>",
        "(original object)",
    ] {
        offsets.push(body.len());
        let _ = writeln!(body, "{} 0 obj\n{object}\nendobj", offsets.len());
    }
    let original_xref = body.len();
    body.push_str("xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(body, "{offset:010} 00000 n ");
    }
    let _ = write!(
        body,
        "trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{original_xref}\n%%EOF\n"
    );
    if delete {
        let last_xref = body.len();
        // §7.5.4 permits free entries at generation65535 outside the reusable free list.
        // Incremental subsections do not redefine object0. No old object is rewritten.
        let _ = write!(
            body,
            "xref\n4 1\n0000000000 65535 f \ntrailer\n<< /Size 5 /Root 1 0 R /Prev {original_xref} >>\nstartxref\n{last_xref}\n%%EOF\n"
        );
    }
    let mut bytes = vec![b'x'; prefix_len];
    bytes.extend_from_slice(body.as_bytes());
    bytes
}

#[test]
fn a_long_header_prefix_does_not_resurrect_an_object_deleted_by_the_latest_section() {
    for prefix_len in [0, 1000, 2048] {
        let original = Document::open(fixture(prefix_len, false)).expect("original document opens");
        assert_eq!(
            original.get(ObjectId::new(4, 0)),
            Object::String(b"original object".to_vec().into()),
            "positive control at prefix length {prefix_len}"
        );
        let updated = Document::open(fixture(prefix_len, true)).expect("updated document opens");
        let catalog = updated.catalog().expect("the catalog remains reachable");
        assert!(updated.get_key(&catalog, "Pages").as_dict().is_some());
        assert_eq!(
            updated.get(ObjectId::new(4, 0)),
            Object::Null,
            "latest xref frees object4; a {prefix_len}-byte prefix cannot revive old bytes (recovered={})",
            updated.was_recovered()
        );
    }
}
