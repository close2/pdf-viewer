//! Specification audit of ISO 32000-2 §12.5.3: a link whose flags forbid interaction is not
//! activated by a click. Finding `2175-B02` of `doc/reviews/2175-specification-audit-12-5.md`.
//!
//! Table 167, bit 2 `Hidden`: "If set, do not render the annotation or allow it to interact with
//! the user"; bit 6 `NoView`: "do not render the annotation on the screen or allow it to interact
//! with the user"; bit 7 `ReadOnly`: "do not allow the annotation to interact with the user … should
//! not respond to mouse clicks". A click is §12.5.6.5's activation, which `interact::activate`
//! answers from `pdf_model::link::at` with none of the three bits read.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "a broken fixture must fail where it is built; device points are small sums"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, PointerAction, Query, Viewer};

fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    writeln!(out, "xref\n0 {size}").expect("string write");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        writeln!(out, "{offset:010} 00000 n ").expect("string write");
    }
    writeln!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF"
    )
    .expect("string write");
    out.into_bytes()
}

/// Two pages; page one's whole area is a link to page two whose `/F` is `flags`.
fn pdf(flags: i64) -> Vec<u8> {
    assemble(&format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Contents 5 0 R /Annots [6 0 R] >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Contents 5 0 R >>\nendobj\n\
         5 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Link /Rect [10 10 190 190] /F {flags} \
         /Border [0 0 0] /Dest [4 0 R /Fit] >>\nendobj\n"
    ))
}

/// Whether a click in the middle of page one turns to page two, and what `Query::LinkAt` says
/// there first.
fn click(flags: i64) -> (bool, bool) {
    let mut viewer = Viewer::new(200, 200, 1.0);
    let opened: Vec<_> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: pdf(flags).into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { pages: 2, .. })),
        "the fixture opens with two pages"
    );
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("an open page has geometry");
    };
    let point = (
        geometry.origin.0 + 100.0 * geometry.scale,
        geometry.origin.1 + 100.0 * geometry.scale,
    );
    let over_link = matches!(viewer.query(Query::LinkAt(point)), Answer::Link(true));
    let mut turned = false;
    for action in [PointerAction::Pressed, PointerAction::Released] {
        turned |= viewer
            .handle(Command::Pointer { at: point, action })
            .any(|event| matches!(event, Event::PageChanged { index: 1, .. }));
    }
    (over_link, turned)
}

#[test]
fn a_link_that_may_not_interact_is_not_activated_spec_audit() {
    assert_eq!(
        click(4),
        (true, true),
        "control: a printable, shown link turns the page"
    );
    // Hidden, NoView and ReadOnly, each beside Print.
    assert_eq!(
        [click(4 | 2), click(4 | 32), click(4 | 64)],
        [(false, false); 3],
        "Hidden, NoView, ReadOnly"
    );
}
