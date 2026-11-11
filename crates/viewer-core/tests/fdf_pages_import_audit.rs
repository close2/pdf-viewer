//! Regression: a local FDF template page through the public viewer import and render path.
#![expect(
    clippy::expect_used,
    reason = "the exact-xref fixture and public route preconditions must fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{
    Answer, Command, DocumentId, Event, PageTarget, Purpose, Query, Rendered, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(71);

fn file(header: &str, root: &str, objects: &[String]) -> Vec<u8> {
    let mut out = String::from(header);
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let number = index.saturating_add(1);
        let _ = writeln!(out, "{number} 0 obj\n{object}\nendobj");
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root {root} /ID [<30313233343536373839616263646566> <66656463626139383736353433323130>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn source() -> Vec<u8> {
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [] >> /Names << /Templates << /Names [(stamp) 6 0 R] >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 120 120] /Resources << >> /Annots [4 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Link /Rect [5 5 30 30] /F 4 /A << /Type /Action /S /ImportData /F (pages.fdf) >> >>".to_owned(),
        "null".to_owned(),
        "<< /Type /Template /MediaBox [0 0 100 100] /Resources << >> /Contents 7 0 R >>".to_owned(),
        {
            let marks = "q 1 0 0 rg 10 10 60 60 re f Q\n";
            format!(
                "<< /Length {} >>\nstream\n{marks}endstream",
                marks.len()
            )
        },
    ];
    file("%PDF-2.0\n", "1 0 R", &objects)
}

fn fdf() -> Vec<u8> {
    let objects = [
        "<< /Version /1.3 /FDF << /Pages [<< /Templates [<< /TRef << /Name (stamp) >> >>] >>] >> >>".to_owned(),
    ];
    file("%FDF-1.3\n", "1 0 R", &objects)
}

fn render_request(events: &[Event], page: usize) -> viewer_core::RenderRequest {
    let mut requests = events.iter().filter_map(|event| match event {
        Event::NeedsRender(request) if request.page == page => Some(request),
        _ => None,
    });
    let request = requests.next().expect("the target page needs a render");
    assert!(requests.next().is_none(), "one request for page {page}");
    request.clone()
}

fn serve(viewer: &mut Viewer, request: &viewer_core::RenderRequest) -> Vec<Event> {
    let raster = CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("the complete page renders");
    viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(raster),
        })
        .collect()
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> (u8, u8, u8) {
    assert!(
        x < raster.width && y < raster.height,
        "sample ({x},{y}) in {}x{} raster",
        raster.width,
        raster.height
    );
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let index = usize::try_from(row.saturating_mul(raster.width).saturating_add(x))
        .expect("small raster")
        .saturating_mul(4);
    (
        raster.data[index],
        raster.data[index.saturating_add(1)],
        raster.data[index.saturating_add(2)],
    )
}

/// A local `/TRef` names the associated PDF's /Templates tree. The page is appended to the live
/// viewer after its existing page, is reachable through page navigation, and its marks render.
/// The final ordering is the implementation's documented choice; §12.7.8.3.3 does not prescribe
/// an insertion index. Persistence is also separate: ADR 2154 records that `ViewState` save does
/// not write appended pages into the target page tree.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "bounded positive fixture coordinates scale to small integer raster samples"
)]
#[test]
fn imported_local_template_is_counted_navigable_and_rendered_by_the_viewer() {
    let source_bytes = source();
    let source_document = pdf_syntax::Document::open(source_bytes.clone()).expect("source PDF");
    assert!(!source_document.was_recovered(), "exact source xref");
    let fdf_bytes = fdf();
    let fdf_document = pdf_syntax::Document::open(fdf_bytes.clone()).expect("FDF");
    assert!(!fdf_document.was_recovered(), "exact FDF xref");

    let mut viewer = Viewer::new(240, 240, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: source_bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(matches!(viewer.query(Query::PageCount), Answer::Count(1)));
    let original = render_request(&opened, 0);
    let _ = serve(&mut viewer, &original);
    let Answer::Frame(frames) = viewer.query(Query::Frame) else {
        panic!("the initial page frame is available");
    };
    assert_eq!(frames.len(), 1);
    assert_eq!(frames.first().expect("one frame").page, 0);
    assert_ne!(
        pixel(
            frames.first().expect("one frame").raster,
            frames.first().expect("one frame").raster.width / 2,
            frames.first().expect("one frame").raster.height / 2
        ),
        (255, 0, 0),
        "positive control: source page has no red template mark"
    );

    let asked: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    assert!(
        asked.iter().any(|event| matches!(event, Event::NeedsFile {
        purpose: Purpose::ImportData,
        name,
        ..
    } if name == "pages.fdf")),
        "the reachable ImportData link asks for its FDF: {asked:?}"
    );
    let imported: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::ImportData,
            bytes: Some(fdf_bytes),
        })
        .collect();
    assert!(
        matches!(viewer.query(Query::PageCount), Answer::Count(2)),
        "a successfully applied local template is included in the public page count: {imported:?}"
    );
    assert!(
        imported.iter().any(|event| matches!(event, Event::Reported { notes, .. }
            if notes.iter().any(|note| note.contains("1 template page(s) added") && note.contains("now has 2")))),
        "the import result names the page-count change: {imported:?}"
    );

    let turned: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Index(1))).collect();
    assert!(
        matches!(
            viewer.query(Query::CurrentPage),
            Answer::Page {
                index: 1,
                of: 2,
                ..
            }
        ),
        "the appended page is a navigation target: {turned:?}"
    );
    let request = render_request(&turned, 1);
    let _ = serve(&mut viewer, &request);
    let Answer::Frame(frames) = viewer.query(Query::Frame) else {
        panic!("the imported template frame is available");
    };
    assert_eq!(frames.len(), 1, "default SinglePage layout");
    assert_eq!(frames.first().expect("one frame").page, 1);
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(1)) else {
        panic!("the appended page has public geometry");
    };
    assert!(geometry.scale.is_finite() && geometry.scale > 0.0 && geometry.scale < 10.0);
    let (x, y) = (
        (35.0 * geometry.scale).round() as u32,
        (65.0 * geometry.scale).round() as u32,
    );
    assert_eq!(
        pixel(frames.first().expect("one frame").raster, x, y),
        (255, 0, 0),
        "interior of the template's PDF-space red rectangle is rendered"
    );
}
