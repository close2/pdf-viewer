//! §12.4.3's article threads, listed and followed with no window at all.
//!
//! The clause reads the structure and makes the way in a permission:
//!
//! > Interactive PDF processors may provide navigation facilities to allow the user to follow a
//! > thread from one bead to the next.
//!
//! `pdf_model::article` has read the structure since the two-hundred-and-fifty-fifth session and
//! §12.6.4.7's thread *action* has followed it since; what this crate owes is the other way in —
//! a person choosing a thread from a list, which is what [`Query::Articles`] and
//! [`Command::Activate`] are.
//!
//! **Not one of the 974 corpus documents states an article thread**, so the fixture is written
//! here. Trap 8's converse: a corpus finds what documents contain and not what the standard says,
//! and clause 12's display half is in scope without exclusions.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "test code: the object numbers below are the small integers this file wrote"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, PageTarget, Query, Viewer};

/// The document this file drives: two pages, one thread, three beads.
///
/// §12.4.3's own EXAMPLE shape — a story that starts on page one and continues on page two — with
/// Table 162's `/F` and `/I` and Table 163's `/T`, `/N`, `/V`, `/P` and `/R` spelled in full. The
/// ring is closed at both ends, which the clause requires and which is what makes a thread walk
/// stop by returning rather than by running out.
fn two_page_thread() -> Vec<u8> {
    let objects: [&str; 9] = [
        "<< /Type /Catalog /Pages 2 0 R /Threads [6 0 R] >>",
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 800] /Contents 5 0 R /B [7 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 800] /Contents 5 0 R /B [8 0 R 9 0 R] >>",
        "<< /Length 0 >>\nstream\n\nendstream",
        "<< /Type /Thread /F 7 0 R /I << /Title (Man Bites Dog) >> >>",
        "<< /Type /Bead /T 6 0 R /N 8 0 R /V 9 0 R /P 3 0 R /R [40 600 360 760] >>",
        "<< /T 6 0 R /N 9 0 R /V 7 0 R /P 4 0 R /R [40 400 360 560] >>",
        "<< /T 6 0 R /N 7 0 R /V 8 0 R /P 4 0 R /R [40 100 360 300] >>",
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    out.into_bytes()
}

/// A viewer with that document open at page one.
fn opened() -> Viewer {
    let mut viewer = Viewer::new(400, 800, 1.0);
    let opened = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: two_page_thread(),
            password: None,
            fragment: None,
        })
        .any(|event| matches!(event, Event::Opened { .. }));
    assert!(opened, "the fixture is a valid PDF");
    viewer
}

/// The list a panel draws, in the `/Threads` array's own order.
#[test]
fn the_threads_a_document_states_are_answered_with_their_titles() {
    let viewer = opened();
    let Answer::Articles(threads) = viewer.query(Query::Articles) else {
        panic!("Query::Articles answers with a list");
    };
    assert_eq!(threads.len(), 1);
    // Table 162's `/I` is §14.3.3's Table 349 by another name, and `/Title` is what a list shows.
    assert_eq!(threads[0].title.as_deref(), Some("Man Bites Dog"));
    assert_eq!(
        threads[0].beads.len(),
        3,
        "the ring is walked once and closes"
    );
}

/// Activating a thread follows it, and lands on Table 163's `/R` rather than on the page.
///
/// The panel sends the *object*, exactly as §12.3.3's outline does, and the document decides what
/// that means — here by composing §12.6.4.7's own thread action, which is the action a file writes
/// to do the same job. Landing on the rectangle is the clause's point: an article exists because
/// its pieces are not physically sequential, so "follow the thread" has to mean the bead.
#[test]
fn activating_a_thread_goes_to_its_first_bead() {
    let mut viewer = opened();
    // Away from page one first, so that the jump is visible as a change rather than as a
    // coincidence: the first bead is on page one.
    let _ = viewer.handle(Command::GoTo(PageTarget::Index(1))).count();
    assert!(matches!(
        viewer.query(Query::CurrentPage),
        Answer::Page { index: 1, .. }
    ));

    let thread = pdf_syntax::ObjectId::new(6, 0);
    let events: Vec<Event> = viewer.handle(Command::Activate(thread)).collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::PageChanged { .. })),
        "following a thread turned the page: {events:?}"
    );
    assert!(matches!(
        viewer.query(Query::CurrentPage),
        Answer::Page { index: 0, .. }
    ));

    // And the view is framed on the bead, which is what distinguishes "went to the bead" from
    // "went to the page the bead is on". `/R` is [40 600 360 760], 320 wide by 160 tall, and
    // Table 149's `/FitR` magnifies "just enough to fit the rectangle … entirely within the
    // window" — so a 400 x 800 viewport lands on 400/320 and not on 800/160, because the smaller
    // of the two is what fits both.
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("page one is the one showing");
    };
    assert!(
        (geometry.scale - 1.25).abs() < 0.01,
        "the bead's 320-wide rectangle in a 400-wide viewport is 1.25, not {}",
        geometry.scale
    );
}

/// A document that states no threads answers with an empty list rather than with nothing.
///
/// The distinction a panel needs: an empty list is a fact about the file, and a panel that could
/// not tell it from a query it failed to ask would say the same thing about both.
#[test]
fn a_document_with_no_threads_answers_with_an_empty_list() {
    let mut viewer = Viewer::new(400, 800, 1.0);
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf"),
    );
    let Ok(bytes) = bytes else {
        println!("skipped: doc/specifications.zip is not unpacked");
        return;
    };
    let opened = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes,
            password: None,
            fragment: None,
        })
        .any(|event| matches!(event, Event::Opened { .. }));
    assert!(opened);
    let Answer::Articles(threads) = viewer.query(Query::Articles) else {
        panic!("Query::Articles always answers with a list");
    };
    assert!(threads.is_empty());
}
