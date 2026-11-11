//! Annex I's mandatory attempt to read a PDF whose version is newer than the reader supports.

use std::fmt::Write as _;

use pdf_render::Rasterizer;
use pdf_syntax::{Document, Version};
use render_cpu::CpuRasterizer;
use viewer_core::{Command, DocumentId, Event, Rendered, Viewer};

fn newer_pdf_with_a_classic_xref() -> Vec<u8> {
    let mut bytes = b"%PDF-2.1\n".to_vec();
    let objects = [
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".as_slice(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".as_slice(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources << >> /Contents 4 0 R >>\nendobj\n".as_slice(),
        b"4 0 obj\n<< /Length 19 >>\nstream\n0 g 0 0 20 20 re f\nendstream\nendobj\n".as_slice(),
    ];
    let mut offsets = Vec::with_capacity(objects.len());
    for object in objects {
        offsets.push(bytes.len());
        bytes.extend_from_slice(object);
    }

    let xref = bytes.len();
    let mut table = String::from("xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(&mut table, "{offset:010} 00000 n ");
    }
    let _ = writeln!(
        &mut table,
        "trailer\n<< /Size 5 /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <FFEEDDCCBBAA99887766554433221100>] >>\nstartxref\n{xref}\n%%EOF"
    );
    bytes.extend_from_slice(table.as_bytes());
    bytes
}

#[test]
fn a_newer_version_with_a_valid_xref_is_opened_and_its_page_is_drawn() {
    let bytes = newer_pdf_with_a_classic_xref();
    let parsed = Document::open(bytes.clone()).expect("the strict-xref fixture is readable");
    assert_eq!(
        parsed.version(),
        Some(Version { major: 2, minor: 1 }),
        "the fixture states a version newer than this reader supports"
    );
    assert!(
        !parsed.was_recovered(),
        "the fixture's xref is usable as written"
    );

    let mut viewer = Viewer::new(100, 100, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();

    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { pages: 1, .. })),
        "the reader attempts to open the 2.1 file and finds its one page: {opened:?}"
    );
    let request = opened
        .iter()
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .expect("opening asks for the first page to be rendered");
    let raster = CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("the supported page content is rendered");
    let center =
        ((raster.height as usize / 2) * raster.width as usize + raster.width as usize / 2) * 4;
    assert_eq!(
        &raster.data[center..center + 3],
        &[0, 0, 0],
        "the explicit black fill covers the page centre"
    );
    let _rendered: Vec<Event> = viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(raster),
        })
        .collect();

    let report: Vec<Event> = viewer.handle(Command::Report).collect();
    assert!(
        report.iter().any(|event| matches!(event,
            Event::Reported { notes, .. }
            if notes.iter().any(|note| note.contains("newer than the 2.0"))
        )),
        "the newer-version warning is still given when the host asks for the file report: {report:?}"
    );
}
