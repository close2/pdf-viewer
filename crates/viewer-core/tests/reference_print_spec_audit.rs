//! Public reference-XObject print-path policy control.
//!
//! Table 95 files are offered through the same `Command::References` a real host sends. This
//! exercises the public Viewer print path and asserts pixels in `PrintPage`, rather than relying on
//! the lower-level `pdf_model::content::interpret_importing` tests. §8.10.4.2 permits either
//! imported content or proxy on paper; this locks the product's chosen behavior (import when
//! supplied, proxy without a supply), not a universal normative choice.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test fixture is exact-xref and bounded to 10x10"
)]

use std::fmt::Write as _;

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{
    Answer, Command, DocumentId, Event, Printing, Query, ReferenceFiles, Rendered, Sheet, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);
const TARGET_ID: (&str, &str) = (
    "00112233445566778899aabbccddeeff",
    "ffeeddccbbaa99887766554433221100",
);

fn page(colour: &str) -> Vec<u8> {
    let content = format!("{colour} 0 0 10 10 re f");
    pdf(
        &format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
            content.len() + 1
        ),
        Some(TARGET_ID),
    )
}

fn containing() -> Vec<u8> {
    let proxy = "1 0 0 rg 0 0 10 10 re f";
    let content = "/Fm Do";
    pdf(
        &format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
         /Resources << /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] \
         /Ref << /F << /Type /Filespec /F (target.pdf) >> /Page 0 \
         /ID [<{}> <{}>] >> /Length {} >>\nstream\n{proxy}\nendstream\nendobj\n",
            content.len() + 1,
            TARGET_ID.0,
            TARGET_ID.1,
            proxy.len() + 1
        ),
        None,
    )
}

fn pdf(objects: &str, id: Option<(&str, &str)>) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in objects.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let id = id.map_or_else(String::new, |(first, second)| {
        format!(" /ID [<{first}> <{second}>]")
    });
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R{id} >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn settle(viewer: &mut Viewer, mut events: Vec<Event>) {
    for _ in 0..8 {
        let mut more = Vec::new();
        for event in events {
            let Event::NeedsRender(request) = event else {
                continue;
            };
            let raster = CpuRasterizer::new()
                .rasterize(&request.list, request.target)
                .expect("opening render");
            more.extend(viewer.handle(Command::RenderReady {
                token: request.token,
                rendered: Rendered::Raster(raster),
            }));
        }
        if !more
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_)))
        {
            return;
        }
        events = more;
    }
    panic!("fixture did not settle within eight renders");
}

fn printed_center(viewer: &Viewer) -> [u8; 4] {
    let Answer::PrintPage(printed) = viewer.query(Query::PrintPage(0)) else {
        panic!("the granted job has page zero");
    };
    let target = printed.target.expect("10x10 page at unit scale");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&printed.list, target)
        .expect("the print display list rasterizes");
    let offset = ((5 * raster.width + 5) * 4) as usize;
    [
        raster.data[offset],
        raster.data[offset + 1],
        raster.data[offset + 2],
        raster.data[offset + 3],
    ]
}

fn print(with_target: bool) -> [u8; 4] {
    let mut viewer = Viewer::new(100, 100, 1.0);
    if with_target {
        let target = page("0 0 1 rg");
        let parsed = pdf_syntax::Document::open(target.clone()).expect("target fixture parses");
        assert!(!parsed.was_recovered(), "target uses exact xref offsets");
        let files = ReferenceFiles {
            files: vec![("target.pdf".to_owned(), target)],
            source: "test's explicit target supply".to_owned(),
        };
        let events: Vec<Event> = viewer.handle(Command::References(files)).collect();
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::OpenFailed { .. }))
        );
    }
    let containing = containing();
    let parsed = pdf_syntax::Document::open(containing.clone()).expect("containing fixture parses");
    assert!(
        !parsed.was_recovered(),
        "containing fixture uses exact xref offsets"
    );
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: containing.into(),
            password: None,
            fragment: None,
        })
        .collect();
    settle(&mut viewer, events);
    let events: Vec<Event> = viewer
        .handle(Command::Print(Printing::Start(Sheet {
            media: Some([0.0, 0.0, 10.0, 10.0]),
            scale: 1.0,
            page_scale: 1.0,
        })))
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Printing { .. }))
    );
    printed_center(&viewer)
}

#[test]
fn print_uses_the_supplied_reference_page_and_the_proxy_when_no_target_was_supplied() {
    assert_eq!(
        print(true),
        [0, 0, 255, 255],
        "paper uses target content when the host supplied the named target"
    );
    assert_eq!(
        print(false),
        [255, 0, 0, 255],
        "without a target the paper uses the proxy"
    );
}
