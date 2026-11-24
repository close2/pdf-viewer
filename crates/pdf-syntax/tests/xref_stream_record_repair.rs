//! A cross-reference stream record whose fields no location can hold is read as a free entry, and
//! that is logged as the repair `syntax/xref-stream-record` (ISO 32000-2 §7.5.8.3, ADR 3477) — found
//! in a pass of its own, only where `/W` makes such a record possible, so that the records of a
//! usual stream are read with no test for it (ADR 3561).
//!
//! Each fixture is built here from bytes, with every offset computed: Table 18's type 2 record
//! states "\[t]he object number of the object stream in which this object is stored", and a number
//! past `u32` names no object §7.3.10 allows.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot be built must fail loudly"
)]

use pdf_syntax::{Document, ObjectId};

/// The repair's stable name.
const RECORD: &str = "syntax/xref-stream-record";

/// A file of catalog, page tree and page, and a cross-reference stream of `/W [1 5 1]` whose last
/// record is type 2 naming the object stream `stream_number`.
fn a_file(stream_number: u64) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for body in [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
    ] {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", offsets.len()).as_bytes());
    }
    let xref_at = out.len();
    let five = |value: u64| value.to_be_bytes()[3..].to_vec();
    let mut rows = vec![0u8, 0, 0, 0, 0, 0, 0];
    for offset in offsets.iter().chain(std::iter::once(&xref_at)) {
        rows.push(1);
        rows.extend(five(u64::try_from(*offset).expect("an offset fits")));
        rows.push(0);
    }
    rows.push(2);
    rows.extend(five(stream_number));
    rows.push(0);
    out.extend_from_slice(
        format!(
            "4 0 obj\n<< /Type /XRef /Size 6 /W [1 5 1] /Root 1 0 R /Length {} >>\nstream\n",
            rows.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&rows);
    out.extend_from_slice(format!("\nendstream\nendobj\nstartxref\n{xref_at}\n%%EOF\n").as_bytes());
    out
}

fn logged(stream_number: u64) -> bool {
    let document = Document::open(a_file(stream_number)).expect("the file opens");
    assert!(
        document.get(ObjectId::new(3, 0)).as_dict().is_some(),
        "the page is read where its record puts it"
    );
    document
        .repair_classes()
        .iter()
        .any(|class| class.name == RECORD)
}

/// **A type 2 record naming an object stream past `u32` is read as free and logged**; the wrong
/// answer excluded is a five-byte `/W` read with no test at all, which the pass skipped for a usual
/// `/W` would be.
#[test]
fn a_record_no_location_holds_is_logged() {
    assert!(logged(1 << 33), "the record past u32 is a repair");
}

/// **The same stream naming an object stream inside `u32` logs nothing**: the pass finds only the
/// record the clause has no reading for.
#[test]
fn a_record_a_location_holds_is_not_a_repair() {
    assert!(!logged(7), "a record that fits is no repair");
}
