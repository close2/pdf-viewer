//! The cross-reference reader's entry budget (ADR 3257): free entries are held as runs, so a file
//! that states millions of them costs what its objects cost, and in-use entries are counted against
//! a named budget that refuses a file past it rather than reading it in part.
//!
//! Each fixture is built here from bytes, with every offset computed, so that what a test expects
//! follows from ISO 32000-2 §7.5.4, §7.5.6 and §7.5.8 rather than from another reader.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: a fixture that cannot be built must fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::serialize::flate_encode;
use pdf_syntax::{Document, Location, Object, SyntaxError};

/// A record of `/W [1 4 1]`: type, offset, generation (Table 18, type 1) or type 0 with zeros.
fn record(kind: u8, offset: usize, out: &mut Vec<u8>) {
    out.push(kind);
    out.extend_from_slice(&u32::try_from(offset).unwrap().to_be_bytes());
    out.push(0);
}

/// What one cross-reference stream section of a fixture states.
struct Section {
    /// `(first, count)` subsections, in `/Index`'s order.
    index: Vec<(u32, u32)>,
    /// One record per stated number, in `/Index`'s order: `Some(offset)` in use, `None` free.
    rows: Vec<Option<usize>>,
}

/// Appends a cross-reference stream numbered `number` stating `section`, and returns its offset.
fn append_xref_stream(
    out: &mut Vec<u8>,
    number: u32,
    section: &Section,
    size: u32,
    prev: Option<usize>,
) -> usize {
    let at = out.len();
    let mut rows = Vec::new();
    for row in &section.rows {
        match row {
            Some(offset) => record(1, *offset, &mut rows),
            None => record(0, 0, &mut rows),
        }
    }
    let data = flate_encode(&rows, 9).expect("deflate");
    let mut index = String::new();
    for (first, count) in &section.index {
        write!(index, "{first} {count} ").expect("a String takes every write");
    }
    let prev = prev.map_or_else(String::new, |prev| format!(" /Prev {prev}"));
    out.extend_from_slice(
        format!(
            "{number} 0 obj\n<< /Type /XRef /Size {size} /W [1 4 1] /Index [{}] /Root 1 0 R{prev} /Filter /FlateDecode /Length {} >>\nstream\n",
            index.trim_end(),
            data.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    at
}

/// Appends `number 0 obj body endobj` and returns its offset.
fn append_object(out: &mut Vec<u8>, number: u32, body: &str) -> usize {
    let at = out.len();
    out.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    at
}

/// Appends the catalog, page tree and page as objects 1 to 3, and returns their offsets.
fn append_page(out: &mut Vec<u8>) -> [usize; 3] {
    [
        append_object(out, 1, "<< /Type /Catalog /Pages 2 0 R >>"),
        append_object(out, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        append_object(
            out,
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] >>",
        ),
    ]
}

fn finish(out: &mut Vec<u8>, startxref: usize) {
    out.extend_from_slice(format!("startxref\n{startxref}\n%%EOF\n").as_bytes());
}

/// §7.5.4: a file states "one entry for each object number from 0 to the maximum object number
/// defined in the PDF file", and every in-use one names an object the reader must reach. Two
/// hundred thousand in-use entries, far past every file of the corpus this tree reads and well
/// inside the budget, are all read, each to the object its offset names.
#[test]
fn a_file_of_many_objects_reads_whole() {
    const OBJECTS: u32 = 200_000;
    let mut out = b"%PDF-1.7\n".to_vec();
    let page = append_page(&mut out);
    let mut rows = vec![None, Some(page[0]), Some(page[1]), Some(page[2])];
    for number in 4..4 + OBJECTS {
        rows.push(Some(append_object(&mut out, number, &format!("{number}"))));
    }
    let xref_number = 4 + OBJECTS;
    let size = xref_number + 1;
    let xref_at = out.len();
    rows.push(Some(xref_at));
    let at = append_xref_stream(
        &mut out,
        xref_number,
        &Section {
            index: vec![(0, size)],
            rows,
        },
        size,
        None,
    );
    finish(&mut out, at);

    let document = Document::open(out).expect("the file opens");
    let table = document.xref();
    assert_eq!(table.len(), usize::try_from(size - 1).unwrap());
    for number in [4, 5, 99_999, 3 + OBJECTS] {
        assert_eq!(
            document.get(pdf_syntax::ObjectId::new(number, 0)),
            Object::Integer(i64::from(number)),
            "object {number} is read from the offset its entry states"
        );
    }
}

/// §7.5.6: "the most recent copy of each object shall be the one accessed from the PDF file", a
/// removal included — and free entries are now runs, so the rule has to hold between a run and an
/// entry inside it. The original section states objects 4 to 9 in use and 10 to 99 free; the update
/// frees 5 to 7 in one run and puts object 50 inside the original's free run.
#[test]
fn a_free_run_and_an_entry_inside_another_keep_the_newest_statement() {
    let mut out = b"%PDF-1.7\n".to_vec();
    let page = append_page(&mut out);
    let mut rows = vec![None, Some(page[0]), Some(page[1]), Some(page[2])];
    for number in 4..10 {
        rows.push(Some(append_object(
            &mut out,
            number,
            &format!("({number})"),
        )));
    }
    rows.extend(std::iter::repeat_n(None, 90));
    let original_at = out.len();
    rows.push(Some(original_at));
    append_xref_stream(
        &mut out,
        100,
        &Section {
            index: vec![(0, 101)],
            rows,
        },
        101,
        None,
    );
    let fifty = append_object(&mut out, 50, "(fifty)");
    let update_at = out.len();
    append_xref_stream(
        &mut out,
        101,
        &Section {
            index: vec![(5, 3), (50, 1), (101, 1)],
            rows: vec![None, None, None, Some(fifty), Some(update_at)],
        },
        102,
        Some(original_at),
    );
    finish(&mut out, update_at);

    let document = Document::open(out).expect("the file opens");
    let table = document.xref();
    for number in [4, 8, 9, 50] {
        assert!(
            matches!(table.location(number), Some(Location::Offset(_))),
            "object {number} is in use in the newest statement about it"
        );
    }
    for number in [5, 6, 7, 10, 49, 51, 99] {
        assert_eq!(table.location(number), None, "object {number} is free");
        assert!(
            !table.declared_and_unread(number),
            "object {number} was described, as free"
        );
    }
    assert_eq!(table.highest_stated_number(), Some(101));
    assert_eq!(
        document.get(pdf_syntax::ObjectId::new(50, 0)),
        Object::String(b"fifty".to_vec().into())
    );
}

/// The budget's refusal: a stream stating one in-use entry more than the budget (`1 << 23`, ISO
/// 32000-1 Table C.1's 8 388 607 indirect objects and object 0) is refused by name, never read in
/// part and never rebuilt by a scan into a different document. The rows name one object, so the
/// file is a few kilobytes; it is not conforming, which is why it is the budget's case and not
/// §7.5.4's.
#[test]
fn a_file_past_the_entry_budget_is_refused_by_name() {
    const PAST_THE_BUDGET: usize = (1 << 23) + 1;
    let mut out = b"%PDF-1.7\n".to_vec();
    let page = append_page(&mut out);
    let mut rows = Vec::with_capacity(PAST_THE_BUDGET.saturating_mul(6));
    for _ in 0..PAST_THE_BUDGET {
        record(1, page[0], &mut rows);
    }
    let twice = flate_encode(&flate_encode(&rows, 9).expect("deflate"), 9).expect("deflate");
    drop(rows);
    let at = out.len();
    out.extend_from_slice(
        format!(
            "4 0 obj\n<< /Type /XRef /Size {PAST_THE_BUDGET} /W [1 4 1] /Root 1 0 R /Filter [/FlateDecode /FlateDecode] /Length {} >>\nstream\n",
            twice.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&twice);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    finish(&mut out, at);

    match Document::open(out) {
        Err(SyntaxError::LimitExceeded {
            at: refused_at,
            limit,
        }) => {
            assert_eq!(limit, "MAX_XREF_ENTRIES");
            assert_eq!(refused_at, at, "the refusal names the section");
        }
        other => panic!("expected the entry budget's refusal, got {other:?}"),
    }
}

/// The budget is of entries in use, not of records: a stream carrying more records than
/// `MAX_XREF_ENTRIES` is counted from its type bytes before anything is held, and read when the
/// count is within the budget (ADR 3257, ADR 3305). Three objects are in use and every other
/// number up to one past the budget is free, which §7.5.4 lets a file state; the file opens and
/// its three objects are where their entries say.
#[test]
fn a_stream_of_more_records_than_the_budget_whose_entries_are_free_is_read() {
    const PAST_THE_BUDGET: usize = (1 << 23) + 1;
    let mut out = b"%PDF-1.7\n".to_vec();
    let page = append_page(&mut out);
    let mut rows = Vec::with_capacity(PAST_THE_BUDGET.saturating_mul(6));
    record(0, 0, &mut rows);
    for offset in page {
        record(1, offset, &mut rows);
    }
    for _ in 4..PAST_THE_BUDGET {
        record(0, 0, &mut rows);
    }
    let data = flate_encode(&rows, 9).expect("deflate");
    drop(rows);
    let at = out.len();
    out.extend_from_slice(
        format!(
            "4 0 obj\n<< /Type /XRef /Size {PAST_THE_BUDGET} /W [1 4 1] /Root 1 0 R /Filter /FlateDecode /Length {} >>\nstream\n",
            data.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    finish(&mut out, at);

    let document = Document::open(out).expect("a stream within the budget of entries in use opens");
    assert_eq!(document.xref().len(), 3, "objects 1 to 3 are in use");
    for (number, offset) in (1..=3).zip(page) {
        assert_eq!(
            document.xref().location(number),
            Some(Location::Offset(offset)),
            "object {number} is where its entry says"
        );
    }
    assert!(
        document.catalog().is_ok(),
        "the catalog its trailer names is read"
    );
}
