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
use viewer_core::{Answer, Command, DocumentId, Event, Extraction, Find, Query, Rendered, Viewer};

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
            bytes: bytes.into(),
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

/// Trap 5, in Annex O's own words. A parameter this reader could not read is named, and the rest
/// of the fragment still runs: §O.2's rule is that the parameters are executed in order, not that
/// one of them can cancel the others.
///
/// **This test used to be about `highlight`**, which `Parameter::unhonoured` reported by name
/// until the five-hundred-and-twenty-second session carried it out (ADR 0357). What it is about is
/// the *channel*, so it now uses a parameter this annex does not define at all — `pagemode` is
/// another reader's, and a URI that mixes the two has still said something about this document.
#[test]
fn a_parameter_this_program_cannot_read_is_named_and_the_others_still_run() {
    let Some((viewer, events)) = opened("vertical.pdf", "pagemode=bookmarks&page=2") else {
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
        notes.iter().any(|note| note.contains("pagemode")),
        "{notes:?}"
    );
}

/// Table Annex O.4's `highlight` in §O.2.2, reported by name until the five-hundred-and-twenty-second
/// session and carried out since it (ADR 0357):
///
/// > Open the document with the specified rectangle highlighted. Each argument shall be an integer
/// > or floating point value representing the rectangle measured from the top left corner of the
/// > page. The nature of the highlighting is implementation-dependent.
///
/// The nature being implementation-dependent is why this crosses as geometry: `Query::Highlight`
/// answers with the rectangle in device pixels of the viewport and a host washes it in its own
/// colour, exactly as it does a selection.
///
/// **The expected pixels are derived rather than written down.** `vertical.pdf`'s pages are
/// `/MediaBox [0 0 249.45 321.02]` with no `/Rotate`, so the page's top-left corner in default
/// user space is (0, 321.02) and `Query::PageGeometry` says where that corner landed on the screen
/// and at what magnification. A rectangle measured from the *bottom* left would come back
/// `321.02 - top - height` further down, which is what this test can tell apart.
#[test]
fn a_highlighted_rectangle_is_measured_from_the_page_corner_and_crosses_as_a_quadrilateral() {
    let Some((mut viewer, events)) = opened("vertical.pdf", "highlight=10,60,20,80") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    settle(&mut viewer, &events);
    let geometry = geometry(&viewer, 0);
    let Answer::Highlighted(quads) = viewer.query(Query::Highlight) else {
        panic!("a fragment that named a rectangle has one to answer with");
    };
    let [quad] = quads.as_slice() else {
        panic!("one rectangle was named: {quads:?}");
    };
    let scale = geometry.scale;
    let (left, top) = (
        geometry.origin.0 + 10.0 * scale,
        geometry.origin.1 + 20.0 * scale,
    );
    let (right, bottom) = (
        geometry.origin.0 + 60.0 * scale,
        geometry.origin.1 + 80.0 * scale,
    );
    // Clockwise from the top-left as it appears on the screen, which is the form every other
    // quadrilateral this crate answers with takes.
    let expected = [left, top, right, top, right, bottom, left, bottom];
    for (corner, want) in quad.iter().zip(expected) {
        assert!(
            (corner - want).abs() < 0.5,
            "{quad:?} against {expected:?} at scale {scale}"
        );
    }
}

/// The rectangle belongs to the page the fragment had selected when it named it.
///
/// Every row of Table Annex O.4 measures "from the top left corner of the page", and §O.2 makes
/// the parameters run left to right — so `page=2&highlight=…` is a rectangle on page 2, and page 1
/// has nothing to draw. The same dependence the annex spells out for `comment` in a NOTE.
#[test]
fn a_highlighted_rectangle_belongs_to_the_page_it_was_named_on() {
    let Some((mut viewer, events)) = opened("vertical.pdf", "page=2&highlight=10,60,20,80") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    settle(&mut viewer, &events);
    assert_eq!(page(&viewer), 1, "the second page, one-based in the URI");
    let Answer::Highlighted(here) = viewer.query(Query::Highlight) else {
        panic!("the page it was named on has it");
    };
    assert_eq!(here.len(), 1, "{here:?}");

    let events: Vec<Event> = viewer
        .handle(Command::GoTo(viewer_core::PageTarget::Index(0)))
        .collect();
    settle(&mut viewer, &events);
    let Answer::Highlighted(elsewhere) = viewer.query(Query::Highlight) else {
        panic!("the answer is a list on every page, empty or not");
    };
    assert!(
        elsewhere.is_empty(),
        "page one was never highlighted: {elsewhere:?}"
    );
}

/// Table Annex O.4's `fdf` in §O.2.2, reported by name until the five-hundred-and-twenty-second
/// session (ADR 0357):
///
/// > Open the document and then import the data from the specified FDF or XFDF file. The URI shall
/// > be either a relative or absolute URI to an FDF or XFDF file.
///
/// **The fetch is the host's and always was**, which is what the old refusal described rather than
/// what stood in its way: this crate has no filesystem (`doc/ui-boundary.md`'s rule 2), so the name
/// crosses as `Event::NeedsFile` with the purpose §12.7.6.4's import action already uses, and a
/// host resolves it — against the document's own URI for a relative one, by its own policy for an
/// absolute one. This test *is* that host, which is the only way a headless one can be.
///
/// `form_two_pages.pdf` states a text field called `Text1` (§12.7.4.2's fully qualified name), and
/// the FDF below is §12.7.8.2's own construction — a `/FDF` dictionary whose `/Fields` array names
/// that field and gives it a `/V`.
#[test]
fn the_fdf_a_fragment_names_is_asked_for_and_imported() {
    let Some((mut viewer, events)) = opened("form_two_pages.pdf", "fdf=answers.fdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let asked: Vec<(viewer_core::Purpose, String)> = events
        .iter()
        .filter_map(|event| match event {
            Event::NeedsFile { purpose, name, .. } => Some((*purpose, name.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        asked,
        vec![(viewer_core::Purpose::ImportData, "answers.fdf".to_owned())],
        "the URI's fragment asked this host for the file, by the name it was written with"
    );

    // What the file itself says the field holds, so that the assertion below is about the import
    // rather than about a value that was there already.
    assert_ne!(value_of(&viewer, "Text1"), "Ada Lovelace");

    // §12.7.8.2's header, its `/FDF` dictionary and §12.7.8.2.4's trailer, which is all an FDF
    // file is. Written here rather than taken from the corpus for the reason the whole of this
    // annex's testing has: no document carries a fragment identifier, and none of the 964 carries
    // an FDF beside it either.
    let fdf: &[u8] = b"%FDF-1.2\n1 0 obj\n<< /FDF << /Fields \
        [ << /T (Text1) /V (Ada Lovelace) >> ] >> >>\nendobj\n\
        trailer\n<< /Root 1 0 R >>\n%%EOF\n";
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: viewer_core::Purpose::ImportData,
            bytes: Some(fdf.to_vec()),
        })
        .collect();
    settle(&mut viewer, &events);

    assert_eq!(
        value_of(&viewer, "Text1"),
        "Ada Lovelace",
        "§12.7.8's imported value is what the field says now"
    );
}

/// What a field of the form on the page being shown says now, by §12.7.4.2's qualified name.
fn value_of(viewer: &Viewer, field: &str) -> String {
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("this document has a form");
    };
    fields
        .iter()
        .find(|shown| shown.name.qualified == field)
        .expect("the field this test is about")
        .value
        .as_ref()
        .map(|value| value.text.clone())
        .unwrap_or_default()
}

/// A `fdf` naming something this program does not read is declined **by name**.
///
/// ISO 19444-1's XFDF is the same data in XML and would need an XML parser, which is a dependency
/// rather than a clause — the decision `interact::request_file` already takes for §12.7.6.4's
/// action, taken once for both by `pdf_model::action::data_format`. Trap 5: nothing is asked of
/// the host and a person is told why.
#[test]
fn an_xfdf_a_fragment_names_is_declined_by_name() {
    let Some((_, events)) = opened("form_two_pages.pdf", "fdf=answers.xfdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::NeedsFile { .. })),
        "nothing is asked for a format nothing reads"
    );
    let notes = notes(&events);
    assert!(
        notes
            .iter()
            .any(|note| note.contains("answers.xfdf") && note.contains("FDF")),
        "{notes:?}"
    );
}

/// Table Annex O.3's `ef` in §O.2.1, which is the annex's one `shall` about a file rather than a
/// view:
///
/// > When used as part of a PDF open parameter, the PDF processor shall open the embedded file
/// > contained within the EmbeddedFiles name tree identified by name .
///
/// `attachment.pdf`'s tree is `<</Names [(foo.txt) 15 0 R]>>`, so `foo.txt` is the key the annex
/// says to match, and the file specification's own `/F` is `foo.txt` as well — which is what
/// `Event::Extracted` reports, because Table 43's name is what a person would call the file. Its
/// contents are `bar baz \n` once §7.4's filters are undone, and checking those rather than a
/// length is what distinguishes an extraction from a still-deflated stream.
///
/// **Ten of the corpus's 964 documents carry an `/EmbeddedFiles` tree at all, with 23 files
/// between them** — the population this parameter can reach, counted rather than assumed.
///
/// What the annex leaves to a host is what "open" then means: "[s]ecurity should be strongly
/// considered when opening an embedded file … a PDF processor may choose to prompt the user or
/// even prevent opening of the file". The bytes crossing as `Event::Extracted` is exactly that
/// decision being handed over, and it is the channel `Command::Extract` and §12.5.6.15's
/// annotation already use.
#[test]
fn an_embedded_file_the_fragment_names_comes_out_of_the_document() {
    let Some((_, events)) = opened("attachment.pdf", "ef=foo.txt") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let extracted: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Extracted {
                asked, name, bytes, ..
            } => Some((*asked, name.clone(), bytes.clone())),
            _ => None,
        })
        .collect();
    let [(asked, name, bytes)] = extracted.as_slice() else {
        panic!("one file out of the document, not {extracted:?}");
    };
    assert_eq!(
        *asked,
        Extraction::Fragment,
        "a URI named it, and no host may write it as though a person had"
    );
    assert_eq!(name, "foo.txt", "Table 43's own name for the file");
    assert_eq!(
        String::from_utf8_lossy(bytes),
        "bar baz \n",
        "the file itself, with §7.4's filters undone"
    );
    let said = notes(&events);
    assert!(
        !said.iter().any(|note| note.contains("does not do")),
        "a parameter carried out is not refused: {said:?}"
    );

    // A name the tree does not hold is reported rather than swallowed, which is the same sentence
    // `Command::Extract` produces for the same mistake.
    let Some((_, events)) = opened("attachment.pdf", "ef=nothing.txt") else {
        return;
    };
    let said = notes(&events);
    assert!(
        said.iter()
            .any(|note| note.contains("embeds no file called")),
        "{said:?}"
    );
}

/// Table Annex O.3's `ef` again: "[a]ny remaining parameters after this parameter apply to the
/// selected embedded file."
///
/// That sentence is about the parameters *after* it, and this crate's whole part in it is to apply
/// none of them to *this* document and to hand them on undivided: opening a second document is a
/// `Command::Open` and therefore a host's (rule 2). So the `page` is not applied here — it would
/// take a person to page three of the wrong document — the `search` raises no `Event::Searched`,
/// and both leave in `Event::Extracted`'s fragment beside the bytes they are about (ADR 0431).
///
/// `issue17056.pdf` files a whole PDF under the tree key `destination-doc.pdf`, which is the case
/// that makes the sentence matter rather than a hypothetical one. The `search` is the witness
/// rather than the `page`: a search that had been applied to *this* document would have raised
/// `Event::Searched`, whatever number of pages the file turns out to have.
#[test]
fn an_embedded_file_carries_the_parameters_after_it() {
    let Some((viewer, events)) = opened(
        "issue17056.pdf",
        "ef=destination-doc.pdf&page=3&search=%22the%22",
    ) else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    assert_eq!(page(&viewer), 0, "page three was never applied");
    let notes = notes(&events);
    assert!(
        notes
            .iter()
            .any(|note| note.contains("the fragment continues `page=3&search=%22the%22`")),
        "{notes:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Searched { .. })),
        "nor was the search after it"
    );
    let carried = events.iter().find_map(|event| match event {
        Event::Extracted {
            fragment, bytes, ..
        } => Some((fragment.clone(), bytes.clone())),
        _ => None,
    });
    let Some((fragment, bytes)) = carried else {
        panic!("the file it named still came out");
    };
    assert_eq!(
        fragment.as_deref(),
        Some("page=3&search=%22the%22"),
        "the URI's own spelling, for the host to hand back to `Command::Open`"
    );
    // And the sentence is carried out by doing exactly that, which is what a host does with the
    // two of them: the page after `ef` is the *embedded* document's page.
    let mut second = Viewer::new(600, 800, 1.0);
    let opened: Vec<Event> = second
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "the embedded bytes are a document"
    );
    assert_eq!(
        page(&second),
        2,
        "page three of the file the parameters were about"
    );
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Searched { .. })),
        "and its search is the one that runs"
    );
}

/// A parameter neither table defines — `pagemode` is one of the ones other readers have — and
/// Annex O's own `structelem` naming nothing.
///
/// The second is the annex's stated outcome rather than a failure: "[i]f no content is contained
/// within the hierarchy of the structure element or structID does not match a structure element,
/// the first page in the document shall be identified." This is the half a real file reaches
/// *without* naming anything; the half that finds an element is tested in `pdf_model::destination`
/// against a document written for it.
///
/// **This comment used to justify that fixture with "no corpus document has an `/IDTree`", and
/// that was false when it was written**: 12 of the 974 state one. `Tree::element_by_id` carries
/// the count. ADR 0405.
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
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert_eq!(page(&viewer), 0);
    assert_eq!(notes(&events), Vec::<String>::new());
}

/// Table Annex O.4's `search`, carried out across the whole document. ISO 32000-2 §O.2.2:
///
/// > Open the document and search for one or more words, selecting the first matching word in the
/// > document.
///
/// **Started when the document opens and finished by the host**, which is the same division
/// `Event::NeedsRender` makes and is forced by the same two rules: this crate has no thread to
/// read a thousand pages on and nothing may block. So `Command::Open` answers with a
/// `Event::Searched` naming how many pages are to be read, and the host pumps `Find::Continue`.
///
/// The expected page is *derived* rather than written down: it is checked against `pdf_model`'s
/// own readback of every page of the same file.
#[test]
fn a_search_parameter_selects_the_first_matching_word_in_the_document() {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
    let bytes = std::fs::read(&path).expect("the application note is committed in doc/");
    let needle = "compensation";

    let document = pdf_syntax::Document::open(bytes.clone()).expect("the note opens");
    let pages = pdf_model::Pages::new(&document);
    let view = pdf_model::view::ViewState::of(&document);
    let first = (0..pages.len())
        .find(|index| {
            pages.get(*index).is_some_and(|page| {
                pdf_model::content::interpret_with(&document, &page, &view)
                    .text
                    .to_lowercase()
                    .contains(needle)
            })
        })
        .expect("the note is about black point compensation");

    let mut viewer = Viewer::new(800, 1000, 1.0);
    let mut events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: Some(format!("search=%22{needle}%22")),
        })
        .collect();
    // The note says the search is running rather than that it was refused, which is the sentence
    // `Parameter::unhonoured` used to produce for this parameter.
    let notes = notes(&events);
    assert!(
        notes.iter().any(|note| note.contains("which is running")),
        "{notes:?}"
    );

    let mut steps = 0_usize;
    let landed = loop {
        settle(&mut viewer, &events);
        steps = steps.saturating_add(1);
        let mut remaining = 0;
        let mut found = None;
        for event in &events {
            if let Event::Searched {
                found: at,
                remaining: left,
                wrapped,
                ..
            } = event
            {
                assert!(!wrapped, "the annex's search does not wrap");
                remaining = *left;
                found = *at;
            }
        }
        if let Some(found) = found {
            break Some(found);
        }
        assert!(remaining > 0, "the word is in this document");
        events = viewer.handle(Command::Find(Find::Continue)).collect();
    };
    let landed = landed.expect("an occurrence");
    assert_eq!(landed.page, first, "the first page that holds the word");
    assert_eq!(page(&viewer), first, "and that page is the one being shown");
    // One step for the open and one per page read after it.
    assert_eq!(steps, first.saturating_add(2), "{steps} step(s)");

    settle(&mut viewer, &events);
    let Answer::Selected(selected) = viewer.query(Query::Selection) else {
        panic!("the annex says the word is selected");
    };
    assert_eq!(selected.text.to_lowercase(), needle);
}
