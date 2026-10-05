//! ISO 32000-2 §I.1, the annex's one instruction to a processor, run through the viewer.
//!
//! > PDF has been designed to enable users to view everything in the document that the PDF
//! > processor understands and to enable the PDF processor to ignore or inform the user about
//! > objects not understood. The decision whether to ignore or inform the user is made on a
//! > feature-by-feature basis, at the discretion of the PDF processor.
//!
//! The annex leaves the choice to the processor and asks that it be made per feature, so what a
//! fixture can derive from it is three things: everything understood is still shown, an object
//! not understood is either passed over or said, and which of the two is a property of the
//! feature. This tree's decision is the ledger's vocabulary — `reported` is *inform* and `silent`
//! is a gate held at zero — with one feature on the *ignore* side by the annex's own next
//! subclause: I.3 says processors that do not know a dictionary entry "behave as if they were not
//! there". So a page that names a filter no clause of §7.4 defines is drawn as far
//! as it can be and *said*, and a page that carries an entry no table defines is drawn whole and
//! says nothing.

#![expect(
    clippy::panic,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing, and its offsets are within a fixture written in this file"
)]

use std::fmt::Write as _;

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, Query, Rendered, Viewer};

/// A one-page document: `page_extra` in the page dictionary, `content` its stream, `objects`
/// after it, numbered on from 6.
fn document(page_extra: &str, content: &str, objects: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 20] \
         /Resources << /XObject << /Im0 6 0 R >> >> /Contents 4 0 R {page_extra} >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< >>\nendobj\n\
         {objects}",
        content.len() + 1
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

/// A 1×1 grey image under `filter`, its one sample byte stored as written.
fn image(filter: &str) -> String {
    format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray \
         /BitsPerComponent 8 {filter} /Length 1 >>\nstream\n\x00\nendstream\nendobj\n"
    )
}

/// The left half of the page filled black, the right half the image: so the page shows
/// something this tree understands whichever way the image goes.
const CONTENT: &str = "0 g 0 0 20 20 re f q 20 0 0 20 20 0 cm /Im0 Do Q";

/// Opens `bytes`, draws the first page on the CPU, and answers with what the viewer said about
/// it and the drawn pixels' grey levels at the centre of each half.
fn open_and_draw(bytes: Vec<u8>) -> (Vec<String>, [u8; 2]) {
    let mut viewer = Viewer::new(40, 20, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    let request = events
        .iter()
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .expect("opening a document asks for its first page");
    let raster = CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("the CPU backend draws a fill and an image");
    let at = |x: usize, y: usize| {
        let width = raster.width as usize;
        raster.data[(y * width + x) * 4]
    };
    let (width, height) = (raster.width as usize, raster.height as usize);
    let sampled = [at(width / 4, height / 2), at(width * 3 / 4, height / 2)];
    let _ = viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(raster),
        })
        .count();

    let Answer::Reports(reports) = viewer.query(Query::Reports) else {
        panic!("a page that was interpreted answers with its reports");
    };
    let notes = reports
        .iter()
        .flat_map(|page| page.notes.iter().cloned())
        .collect();
    (notes, sampled)
}

/// A filter no clause of §7.4 defines is an object not understood, and this tree *informs*: the
/// page is drawn as far as it is understood — the black half — and a sentence names the image by
/// its resource name and says it was not drawn. The image is not guessed at, which is the annex's
/// "view everything … that the PDF processor understands" and not more.
///
/// The assertion is on what I.1 asks — that the user is told which object — and not on the
/// sentence's wording of *why*, which the annex leaves to the processor.
#[test]
fn an_object_not_understood_is_said_and_what_is_understood_is_still_drawn() {
    let (notes, [understood, _]) = open_and_draw(document(
        "",
        CONTENT,
        &image("/Filter /FilterThisStandardDoesNotDefine"),
    ));
    assert_eq!(understood, 0, "the fill this tree understands is drawn");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("Im0") && note.contains("not drawn")),
        "the object not understood is named to the user: {notes:?}"
    );
}

/// The same page with the image's data stated plainly, and an entry no table of ISO 32000-2
/// defines on the page dictionary: Annex I.3's case of a feature a processor ignores, behaving
/// "as if they were not there". The page is drawn whole — the image's sample 0 is black — and
/// nothing is said, which is the per-feature half of I.1's sentence: the same viewer informs
/// about one object it does not understand and ignores another.
#[test]
fn an_entry_not_understood_is_passed_over_in_silence() {
    let (notes, drawn) = open_and_draw(document(
        "/EntryThisStandardDoesNotDefine 42",
        CONTENT,
        &image(""),
    ));
    assert_eq!(drawn, [0, 0], "both the fill and the image are drawn");
    assert!(
        notes.is_empty(),
        "an unknown entry is ignored, not reported: {notes:?}"
    );
}
