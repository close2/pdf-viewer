//! ISO 32000-2 Annex O's fragment identifiers, applied to real documents.
//!
//! `pdf_model::fragment`'s own tests are about the grammar, and their fragments are written by
//! hand because a fragment identifier cannot come from a corpus document — it arrives with the
//! *request*. This file is the other half: what a fragment then names is looked up in a file
//! somebody else wrote, and every expected value below is derivable from that file's own objects,
//! quoted in the comment above the test.

#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::path::{Path, PathBuf};

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, Query, Rendered, Viewer};

/// The document these tests open.
const DOCUMENT: DocumentId = DocumentId(1);

/// A corpus document's bytes, or `None` when the submodule is not checked out.
fn corpus_bytes(name: &str) -> Option<Vec<u8>> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    std::fs::read(path).ok()
}

/// Opens a corpus document with a fragment identifier, into an 800 × 1000 viewport.
///
/// `None` where the corpus is not checked out, which every test here says out loud rather than
/// passing in silence.
fn opened(name: &str, fragment: &str) -> Option<(Viewer, Vec<Event>)> {
    let bytes = corpus_bytes(name)?;
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: Some(fragment.to_owned()),
        })
        .collect();
    Some((viewer, events))
}

/// The page showing, zero-based.
fn page(viewer: &Viewer) -> usize {
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("an open document is showing a page");
    };
    index
}

/// What the viewer said about the document, with no page number attached.
fn notes(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Reported {
                page: None, notes, ..
            } => Some(notes.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Draws whatever was asked for, which is what makes a pending view apply.
///
/// §12.3.2.1's location and magnification wait for a viewport *and* a display list — the `/FitB`
/// forms are measured against the page's contents — so a fragment's `zoom`, `view` and `viewrect`
/// are applied on the first settled frame and not before.
fn settle(viewer: &mut Viewer, events: &[Event]) {
    let mut outstanding: Vec<viewer_core::RenderRequest> = events
        .iter()
        .filter_map(|event| match event {
            Event::NeedsRender(request) => Some(request.clone()),
            _ => None,
        })
        .collect();
    // A view applied on the first frame changes the magnification, which asks for another.
    for _ in 0..4 {
        let Some(request) = outstanding.pop() else {
            break;
        };
        let raster = CpuRasterizer::new()
            .rasterize(&request.list, request.target)
            .expect("the CPU backend draws this page");
        let answered: Vec<Event> = viewer
            .handle(Command::RenderReady {
                token: request.token,
                rendered: Rendered::Raster(raster),
            })
            .collect();
        outstanding.extend(answered.iter().filter_map(|event| match event {
            Event::NeedsRender(request) => Some(request.clone()),
            _ => None,
        }));
    }
}

/// The page's place on the screen, which is where a `zoom` or a `viewrect` becomes visible.
fn geometry(viewer: &Viewer, index: usize) -> viewer_core::PageGeometry {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(index)) else {
        panic!("the page on the screen has a geometry");
    };
    geometry
}

/// Table Annex O.3's `page`: "the PDF processor shall open the document to the specified page",
/// counting from one where this crate counts from zero.
///
/// `vertical.pdf` has three pages — `/Type /Pages /Count 3 /Kids [3 0 R 9 0 R 12 0 R]`.
#[test]
fn a_fragment_opens_the_page_it_names() {
    let Some((viewer, _)) = opened("vertical.pdf", "page=3") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    assert_eq!(page(&viewer), 2, "the third page, one-based in the URI");
}

/// A page the document does not have. The annex states no outcome, so the one thing that must not
/// happen is silence: a URI that named page 99 and opened page one has misled its reader.
#[test]
fn a_page_the_document_does_not_have_is_named_rather_than_guessed() {
    let Some((viewer, events)) = opened("vertical.pdf", "page=99") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    assert_eq!(page(&viewer), 0);
    let notes = notes(&events);
    assert!(
        notes.iter().any(|note| note.contains("page 99")),
        "{notes:?}"
    );
}

/// Table Annex O.3's `nameddest`: "the PDF processor shall open the document to the page referred
/// to by the named destination."
///
/// `vertical.pdf`'s name tree is `/Names [(Doc-Start) 15 0 R (page.1) 16 0 R (page.2) 17 0 R
/// (page.3) 18 0 R]`; object 17 is `[9 0 R /XYZ 10.98 309.69]`, and the page tree's `/Kids [3 0 R
/// 9 0 R 12 0 R]` makes `9 0 R` the second page. So `page.2` is index 1, from the file's own
/// objects rather than from anything this program decided.
#[test]
fn a_named_destination_opens_the_page_the_document_files_it_under() {
    let Some((viewer, events)) = opened("vertical.pdf", "nameddest=page.2") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    assert_eq!(page(&viewer), 1);
    assert_eq!(notes(&events), Vec::<String>::new(), "nothing to report");
}

/// A name the document does not define. §12.3.2.4's lookup answers nothing, and so does this.
#[test]
fn a_destination_the_document_does_not_define_is_named() {
    let Some((viewer, events)) = opened("vertical.pdf", "nameddest=page.9") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    assert_eq!(page(&viewer), 0);
    let notes = notes(&events);
    assert!(
        notes.iter().any(|note| note.contains("page.9")),
        "{notes:?}"
    );
}

/// §O.2's left-to-right rule, turned into behaviour by the `comment` parameter's own NOTE:
/// "[u]nless the page on which the comment resides has been selected prior to the comment
/// parameter, the comment will not be selected."
///
/// `file_pdfjs_test.pdf` has five pages — `/Kids [45 0 R 1 0 R 4 0 R 7 0 R 10 0 R]` — and object
/// 119 is a `/Subtype /FreeText` annotation carrying `/NM (b075c192-c2d9-44f2-a1db-ade732673d99)`
/// and `/P 10 0 R`, which is the *fifth* page. So the same two parameters in the two orders must
/// give two different answers, and the annex says which.
#[test]
fn a_comment_is_looked_for_on_the_page_chosen_before_it() {
    const NAME: &str = "b075c192-c2d9-44f2-a1db-ade732673d99";
    let Some((mut after, events)) =
        opened("file_pdfjs_test.pdf", &format!("page=5&comment={NAME}"))
    else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    assert_eq!(page(&after), 4);
    // Settled first: `Query::Focus` answers with the annotation's rectangle *on the screen*, so
    // it needs a frame to measure against — the ring is drawn by the host, in device pixels.
    settle(&mut after, &events);
    let Answer::Focus { object, .. } = after.query(Query::Focus) else {
        panic!("the comment on the page the fragment selected is the focused annotation");
    };
    assert_eq!(
        object,
        pdf_syntax::ObjectId::new(119, 0),
        "object 119 is the annotation carrying that /NM"
    );

    let (mut before, events) = opened("file_pdfjs_test.pdf", &format!("comment={NAME}&page=5"))
        .expect("the same document");
    assert_eq!(page(&before), 4, "the page parameter still applies");
    settle(&mut before, &events);
    assert!(
        matches!(before.query(Query::Focus), Answer::None),
        "the comment was looked for on page one, where it is not"
    );
    let notes = notes(&events);
    assert!(notes.iter().any(|note| note.contains(NAME)), "{notes:?}");
}

/// Table Annex O.4's `zoom` is a percentage — "a value of 100 would correspond to a zoom of 100%"
/// — where Table 149's `/XYZ`, which the `view` parameter carries, states the same magnification
/// as a factor. Both spellings are in this annex, and both have to land on the same place.
#[test]
fn a_zoom_is_a_percentage_and_a_view_is_a_factor() {
    let Some((mut percent, events)) = opened("vertical.pdf", "zoom=200") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    settle(&mut percent, &events);
    assert!(
        (geometry(&percent, 0).scale - 2.0).abs() < 0.001,
        "{}",
        geometry(&percent, 0).scale
    );

    let (mut factor, events) = opened("vertical.pdf", "view=XYZ,,,2").expect("the same document");
    settle(&mut factor, &events);
    assert!(
        (geometry(&factor, 0).scale - 2.0).abs() < 0.001,
        "{}",
        geometry(&factor, 0).scale
    );
}

/// §O.2.2's coordinates are default user space's units measured from the page's *top left*, and
/// this is the test that can tell the two origins apart.
///
/// `vertical.pdf`'s pages are `/MediaBox [0 0 249.45 321.02]`. A rectangle 0,0 → 124.7 × 160.5 is
/// the page's top-left quarter, so it fits at min(800 ÷ 124.7, 1000 ÷ 160.5) = 6.23 and the
/// page's own top-left corner lands at the window's — an origin of zero in both directions.
/// Measured from the *bottom* left instead, the same numbers would name the bottom-left quarter
/// and the page would be scrolled down, which is a negative origin.
#[test]
fn a_view_rectangle_is_measured_from_the_top_left_corner_of_the_page() {
    let Some((mut viewer, events)) = opened("vertical.pdf", "viewrect=0,0,124.7,160.5") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    settle(&mut viewer, &events);
    let geometry = geometry(&viewer, 0);
    assert!(
        (geometry.scale - 1000.0 / 160.5).abs() < 0.01,
        "the shorter of the two fits: {}",
        geometry.scale
    );
    assert!(
        geometry.origin.1.abs() < 0.5,
        "the page's top edge is at the window's: {:?}",
        geometry.origin
    );
}

/// Trap 5, in Annex O's own words. A parameter this program cannot carry out is named, and the
/// rest of the fragment still runs: §O.2's rule is that the parameters are executed in order, not
/// that one of them can cancel the others.
#[test]
fn a_parameter_this_program_refuses_is_named_and_the_others_still_run() {
    let Some((viewer, events)) = opened("vertical.pdf", "highlight=1,2,3,4&page=2") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    assert_eq!(
        page(&viewer),
        1,
        "the page parameter after it still applied"
    );
    let notes = notes(&events);
    assert!(
        notes.iter().any(|note| note.contains("highlight")),
        "{notes:?}"
    );
}

/// Table Annex O.3's `ef`: "[a]ny remaining parameters after this parameter apply to the selected
/// embedded file."
///
/// So a fragment this program stops at is a fragment it must stop *reading*: applying the `page`
/// after an `ef` would take a person to page three of the wrong document. The refusal does not
/// depend on the file — opening an embedded file is a host's decision — which is why this uses a
/// document with no embedded files at all and still expects both statements.
#[test]
fn an_embedded_file_stops_the_parameters_after_it() {
    let Some((viewer, events)) = opened("vertical.pdf", "ef=data.xml&page=3") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    assert_eq!(page(&viewer), 0, "page three was never applied");
    let notes = notes(&events);
    assert!(notes.iter().any(|note| note.contains("`ef`")), "{notes:?}");
    assert!(
        notes.iter().any(|note| note.contains("1 parameter(s)")),
        "{notes:?}"
    );
}

/// A parameter neither table defines — `pagemode` is one of the ones other readers have — and
/// Annex O's own `structelem` naming nothing.
///
/// The second is the annex's stated outcome rather than a failure: "[i]f no content is contained
/// within the hierarchy of the structure element or structID does not match a structure element,
/// the first page in the document shall be identified." **No corpus document has an `/IDTree`**,
/// so the half of `structelem` that finds one is tested in `pdf_model::destination` against a
/// document written for it; this is the half a real file can reach.
#[test]
fn a_parameter_no_table_defines_and_an_identifier_that_matches_nothing_are_both_named() {
    let Some((viewer, events)) = opened("vertical.pdf", "page=2&pagemode=bookmarks&structelem=x")
    else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    assert_eq!(page(&viewer), 0, "the annex sends structelem to page one");
    let notes = notes(&events);
    assert!(
        notes.iter().any(|note| note.contains("pagemode")),
        "{notes:?}"
    );
    assert!(
        notes.iter().any(|note| note.contains("structure element")),
        "{notes:?}"
    );
}

/// A document opened without a fragment is the document opening as it always did.
#[test]
fn no_fragment_changes_nothing() {
    let Some(bytes) = corpus_bytes("vertical.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .collect();
    assert_eq!(page(&viewer), 0);
    assert_eq!(notes(&events), Vec::<String>::new());
}
