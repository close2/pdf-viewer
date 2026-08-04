//! ISO 32000-2 §12.5.1's five navigation orders, on a page built to tell them apart.
//!
//! The clause states an algorithm per value, so this is the kind of check `dates.rs` and
//! `logical_order.rs` are: a fixture whose geometry, whose `/Annots` order and whose structure
//! tree all disagree, so that each of the five produces a *different* permutation and no two of
//! them can pass by accident.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot be built must fail loudly rather than pass by \
              doing nothing, and its byte offsets are computed from strings this file wrote"
)]

use pdf_model::tab_order::{Tabs, next, order, previous};
use pdf_syntax::{Document, ObjectId};

/// Four annotations on a 200 × 200 page, laid out so that every order differs.
///
/// ```text
///   (0,150)-(80,190)  B          (110,150)-(190,190)  A     <- top row
///   (0, 10)-(80, 50)  D          (110, 10)-(190, 50)  C     <- bottom row
/// ```
///
/// `/Annots` names them **A, B, C, D** — object 10, 11, 12, 13 — so the array order is neither
/// the row order (B A D C) nor the column order (B D A C). A and D are widgets and B and C are
/// links, so `W` separates them again; and the structure tree names D, C, B, A.
///
/// `tabs` is the `/Tabs` value to write, or `None` for a page that states none.
fn fixture(tabs: Option<&str>, rotate: i64) -> (Document, ObjectId) {
    use std::fmt::Write as _;
    let entry = tabs.map_or_else(String::new, |value| format!("/Tabs /{value} "));
    let rects = [
        ("Widget", 110.0, 150.0, 190.0, 190.0), // A, top right
        ("Link", 0.0, 150.0, 80.0, 190.0),      // B, top left
        ("Link", 110.0, 10.0, 190.0, 50.0),     // C, bottom right
        ("Widget", 0.0, 10.0, 80.0, 50.0),      // D, bottom left
    ];
    let mut annots = String::new();
    for (index, (subtype, x0, y0, x1, y1)) in rects.iter().enumerate() {
        let number = 10 + index;
        let _ = write!(
            annots,
            "{number} 0 obj\n<< /Type /Annot /Subtype /{subtype} \
             /Rect [{x0} {y0} {x1} {y1}] /F 4 >>\nendobj\n"
        );
    }
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Rotate {rotate} \
         {entry}/Annots [10 0 R 11 0 R 12 0 R 13 0 R] /Contents 4 0 R \
         /Resources << >> /StructParents 0 >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /StructTreeRoot /K [7 0 R] >>\nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /Sect /Pg 3 0 R /K \
         [<< /Type /OBJR /Obj 13 0 R /Pg 3 0 R >> << /Type /OBJR /Obj 12 0 R /Pg 3 0 R >> \
          << /Type /OBJR /Obj 11 0 R /Pg 3 0 R >> << /Type /OBJR /Obj 10 0 R /Pg 3 0 R >>] \
         >>\nendobj\n\
         8 0 obj\n<< >>\nendobj\n\
         9 0 obj\n<< >>\nendobj\n\
         {annots}"
    );
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    let mut cursor = out.len();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(cursor);
        cursor += object.len();
    }
    out.push_str(&body);
    let at = out.len();
    let size = offsets.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
    );
    let document = Document::open(out.into_bytes()).expect("the fixture is a valid file");
    (document, ObjectId::new(3, 0))
}

/// The order as letters, so an assertion reads like the clause.
fn letters(document: &Document, page_id: ObjectId) -> String {
    let pages = pdf_model::Pages::new(document);
    let page = pages.get(0).expect("one page");
    order(document, &page, page_id)
        .into_iter()
        .map(|id| match id.number {
            10 => 'A',
            11 => 'B',
            12 => 'C',
            13 => 'D',
            _ => '?',
        })
        .collect()
}

/// Each of Table 31's five values produces the permutation §12.5.1 describes.
#[test]
fn the_five_values_of_tabs_are_five_different_orders() {
    // `A`, and the page that states nothing, which this crate reads as `A` by a documented
    // choice: "[a]ll annotations shall be visited in the order in which they appear in the page
    // Annots array".
    let (document, page) = fixture(Some("A"), 0);
    assert_eq!(letters(&document, page), "ABCD");
    let (document, page) = fixture(None, 0);
    assert_eq!(
        letters(&document, page),
        "ABCD",
        "a page that states no /Tabs gets the array's order, which is this crate's choice"
    );

    // `R`: "[a]nnotations shall be visited in rows running horizontally across the page … The
    // first annotation that shall be visited is the first annotation in the topmost row."
    let (document, page) = fixture(Some("R"), 0);
    assert_eq!(letters(&document, page), "BADC");

    // `C`: "[a]nnotations shall be visited in columns running vertically up and down the page …
    // The first annotation that shall be visited is the one at the top of the first column."
    let (document, page) = fixture(Some("C"), 0);
    assert_eq!(letters(&document, page), "BDAC");

    // `S`: "[a]nnotations shall be visited in the order in which they appear in the structure
    // tree", which this fixture states backwards.
    let (document, page) = fixture(Some("S"), 0);
    assert_eq!(letters(&document, page), "DCBA");

    // `W`: "[w]idget annotations shall be visited in the order in which they appear in the page
    // Annots array, followed by other annotation types in row order." A and D are the widgets,
    // in array order; B and C are the rest, in row order.
    let (document, page) = fixture(Some("W"), 0);
    assert_eq!(letters(&document, page), "ADBC");

    // A value the clause does not define has not stated one, so it falls to the default.
    let (document, page) = fixture(Some("Z"), 0);
    assert_eq!(letters(&document, page), "ABCD");
    let pages = pdf_model::Pages::new(&document);
    let one = pages.get(0).expect("one page");
    assert_eq!(Tabs::read(&document, &one.dict), Tabs::Array);
}

/// "These descriptions assume the page is being viewed in the orientation specified by the
/// Rotate entry", so a rotated page has different rows.
#[test]
fn the_geometry_orders_are_taken_after_the_pages_own_rotation() {
    // At `/Rotate 90` the page turns clockwise on the screen, so what was the left column becomes
    // the top row: B and D — the two at x 0..80 — are now above A and C, and within that row the
    // one that was lowest in user space is leftmost.
    let (document, page) = fixture(Some("R"), 90);
    assert_eq!(letters(&document, page), "DBCA");

    // And the column order at 90° is the row order at 0° read the other way round, because the
    // two axes have swapped: what shared a row now shares a column.
    let (document, page) = fixture(Some("C"), 90);
    assert_eq!(letters(&document, page), "DCBA");

    // 180° turns the page over: the bottom row becomes the top one and each row reverses.
    let (document, page) = fixture(Some("R"), 180);
    assert_eq!(letters(&document, page), "CDAB");
}

/// Stepping through the order wraps, and an unknown starting point is the beginning.
#[test]
fn tab_and_shift_tab_wrap_around_the_page() {
    let (document, page) = fixture(Some("A"), 0);
    let pages = pdf_model::Pages::new(&document);
    let one = pages.get(0).expect("one page");
    let sequence = order(&document, &one, page);
    let (a, b, d) = (sequence[0], sequence[1], sequence[3]);

    assert_eq!(next(&sequence, None), Some(a), "tab with nothing focused");
    assert_eq!(next(&sequence, Some(a)), Some(b));
    assert_eq!(next(&sequence, Some(d)), Some(a), "and it wraps");
    assert_eq!(
        previous(&sequence, None),
        Some(d),
        "shift-tab with nothing focused starts at the end, which is what a person means"
    );
    assert_eq!(
        previous(&sequence, Some(a)),
        Some(d),
        "and wraps the other way"
    );
    assert_eq!(previous(&sequence, Some(b)), Some(a));

    // An identifier this page does not have is not a position in its order.
    assert_eq!(next(&sequence, Some(ObjectId::new(999, 0))), Some(a));
    assert_eq!(next(&[], None), None, "a page with no annotations");
}
