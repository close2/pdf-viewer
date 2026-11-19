//! ISO 32000-2 §7.10.5.3's errors a type 4 program meets only where §8.7.4.5.2's type 1 grid is
//! drawn reach the reader after the draw, and not before it (ADRs 2848, 2946).
//!
//! §8.7.4.5.2: a type 1 shading's colour is "the result of a function" at "every point in the
//! domain", so the display list carries the function and whatever draws the page evaluates it on
//! the grid its device needs — after the page's interpretation, and so after its reports were
//! worded. §7.10.5.3: "It shall be an error for the number of remaining operands to differ from
//! the number of output variables specified by Range". The program below leaves two operands for
//! an `x` above one half and three otherwise, against `/Range`'s three: it errs for part of its
//! domain only, which no single point the interpretation evaluates need reach.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    reason = "test code: the offsets are within a fixture written in this file, and a helper that \
              finds no page or no raster fails the test that called it"
)]

use std::fmt::Write as _;

use pdf_render::Rasterizer as _;
use render_cpu::CpuRasterizer;
use viewer_core::{
    Answer, Command, DocumentId, Event, Printing, Query, RenderRequest, Rendered, Sheet, Viewer,
    Zoom,
};

/// The page side, in points and so in pixels at a scale of one.
const SIDE: u32 = 100;

/// The program: `x y` in, `y` dropped; above one half, two ones (one operand short of `/Range`'s
/// three), and otherwise `x x x`.
const ERRS_ABOVE_HALF: &str = "{ pop dup 0.5 gt { pop 1 1 } { dup dup } ifelse }";

/// A one-page document whose page paints a type 1 shading over itself with `program`.
fn document(program: &str) -> Vec<u8> {
    let content = "/Sh sh";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {SIDE} {SIDE}] \
         /Resources << /Shading << /Sh 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /ShadingType 1 /ColorSpace /DeviceRGB /Domain [0 1 0 1] \
         /Matrix [{SIDE} 0 0 {SIDE} 0 0] /Function 6 0 R >>\nendobj\n\
         6 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1] /Range [0 1 0 1 0 1] /Length {} >>\n\
         stream\n{program}\nendstream\nendobj\n",
        content.len(),
        program.len(),
    );
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Opens `program`'s document, and answers the viewer and the request for its page one.
fn opened(program: &str) -> (Viewer, RenderRequest) {
    let mut viewer = Viewer::new(SIDE, SIDE, 1.0);
    let request = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: document(program).into(),
            password: None,
            fragment: None,
        })
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .expect("opening asks for page one, which is what interprets it");
    (viewer, request)
}

/// The notes each `Event::Reported` of `events` carried about page one.
fn reported(events: impl Iterator<Item = Event>) -> Vec<String> {
    events
        .filter_map(|event| match event {
            Event::Reported {
                page: Some(0),
                notes,
                ..
            } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect()
}

/// What `Query::Reports` keeps for page one.
fn kept(viewer: &Viewer) -> Vec<String> {
    let Answer::Reports(pages) = viewer.query(Query::Reports) else {
        panic!("a document is open, so the reports are answered");
    };
    pages
        .iter()
        .filter(|page| page.page == 0)
        .flat_map(|page| page.notes.iter().cloned())
        .collect()
}

/// Whether `note` is the sentence `viewer-core` words a §7.10.5.3 fault in.
fn is_a_function_fault(note: &str) -> bool {
    note.starts_with("the document's function met an error for some inputs")
}

/// Draws the request on the processor, as a tier 1 host does before it answers.
fn drawn(request: &RenderRequest) -> Rendered {
    Rendered::Raster(
        CpuRasterizer::new()
            .rasterize(&request.list, request.target)
            .expect("a 100-pixel page draws"),
    )
}

/// A host that draws and then answers: nothing is said of the program before the draw, the fault
/// is said with the answer after it and kept with the page's notes, and a second draw at another
/// magnification says it no second time.
#[test]
fn a_type_1_grids_fault_is_said_after_the_draw_and_once() {
    let (mut viewer, request) = opened(ERRS_ABOVE_HALF);
    assert!(
        !kept(&viewer).iter().any(|note| is_a_function_fault(note)),
        "nothing has drawn the grid, so nothing has met the fault: {:?}",
        kept(&viewer)
    );

    let rendered = drawn(&request);
    let said = reported(viewer.handle(Command::RenderReady {
        token: request.token,
        rendered,
    }));
    assert_eq!(said.len(), 1, "the draw met one fault: {said:?}");
    assert!(is_a_function_fault(&said[0]), "{said:?}");
    assert!(
        kept(&viewer).contains(&said[0]),
        "what was said is kept with the page's notes: {:?}",
        kept(&viewer)
    );

    let zoomed: Vec<RenderRequest> = viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(2.0),
            at: None,
        })
        .filter_map(|event| match event {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .collect();
    assert!(!zoomed.is_empty(), "a new magnification is drawn again");
    for request in zoomed {
        let rendered = drawn(&request);
        let again = reported(viewer.handle(Command::RenderReady {
            token: request.token,
            rendered,
        }));
        assert!(
            !again.iter().any(|note| is_a_function_fault(note)),
            "the same fault is not said again: {again:?}"
        );
    }
}

/// A host that answers before it draws — a confined worker's window, drawn after the page was
/// answered `Rendered::Listed` — says nothing with the answer, and the fault with the
/// `Command::Declined` it sends once the draw is over, empty though that is.
#[test]
fn a_draw_after_the_answer_is_told_by_an_empty_decline() {
    let (mut viewer, request) = opened(ERRS_ABOVE_HALF);
    let answered = reported(viewer.handle(Command::RenderReady {
        token: request.token,
        rendered: Rendered::Listed,
    }));
    assert!(answered.is_empty(), "nothing is drawn yet: {answered:?}");

    let _ = drawn(&request);
    let said = reported(viewer.handle(Command::Declined {
        document: request.document,
        page: 0,
        declined: Vec::new(),
    }));
    assert_eq!(said.len(), 1, "the draw met one fault: {said:?}");
    assert!(is_a_function_fault(&said[0]), "{said:?}");
}

/// The control: a program whose two arms both leave `/Range`'s three operands meets no error
/// anywhere, and its draw says nothing (trap 11 — the report fires only on a recorded fault).
#[test]
fn a_program_whose_arms_agree_says_nothing_after_its_draw() {
    let (mut viewer, request) = opened("{ pop dup 0.5 gt { pop 1 1 1 } { dup dup } ifelse }");
    let rendered = drawn(&request);
    let said = reported(viewer.handle(Command::RenderReady {
        token: request.token,
        rendered,
    }));
    assert!(said.is_empty(), "no input errs: {said:?}");
    assert!(kept(&viewer).is_empty(), "{:?}", kept(&viewer));
}

/// A printed sheet is drawn by the host after `Query::PrintPage` answered its reports, at the
/// job's resolution and on a reading of its own, so its grid's fault is asked of the page the
/// answer carried once the sheet is drawn: nothing before the draw, the fault after it, worded as
/// the screen words it (ADR 3122).
#[test]
fn a_printed_sheets_grid_fault_is_asked_after_the_sheet_is_drawn() {
    let (mut viewer, _) = opened(ERRS_ABOVE_HALF);
    let _: Vec<Event> = viewer
        .handle(Command::Print(Printing::Start(Sheet {
            media: Some([0.0, 0.0, 595.0, 842.0]),
            scale: 150.0 / 72.0,
            page_scale: 1.0,
        })))
        .collect();
    let Answer::PrintPage(printed) = viewer.query(Query::PrintPage(0)) else {
        panic!("the operation was granted, so page one prints");
    };
    assert!(
        !printed.reports.iter().any(|note| is_a_function_fault(note)),
        "nothing has drawn the sheet's grid: {:?}",
        printed.reports
    );
    assert_eq!(printed.faults_after_drawing(), Vec::<String>::new());

    let target = printed
        .target
        .expect("a 100-point page fits the sheet's budget");
    let _ = CpuRasterizer::new()
        .rasterize(&printed.list, target)
        .expect("the sheet draws");
    let said = printed.faults_after_drawing();
    assert_eq!(said.len(), 1, "the sheet's draw met one fault: {said:?}");
    assert!(is_a_function_fault(&said[0]), "{said:?}");
}
