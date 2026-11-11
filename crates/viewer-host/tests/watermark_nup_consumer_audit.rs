//! Consumer regression for the host/core watermark n-up handoff.
//!
//! This deliberately
//! calls the public host placement helpers and the public Viewer print API, then rasterizes the
//! returned print list; no copy of the page-placement algorithm is used as the implementation.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "the complete fixture, known-small raster and fixed finite grid must fail loudly"
)]

use std::io::Write as _;

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, Printing, Query, Rendered, Viewer};
use viewer_host::printing::{PagesPerSheet, Scaling, cell, placed};

const DOCUMENT: DocumentId = DocumentId(581);
const PAPER: (f32, f32) = (240.0, 180.0);
const PAGE: (f32, f32) = (100.0, 200.0);
const APPEARANCE: &str = "1 0 0 rg 0 0 10 10 re f";
const ERROR_TOLERANCE: f32 = 1.5;

#[derive(Debug, Clone, Copy)]
struct BoxOnPaper {
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
}

fn object_body(id: usize, body: Vec<u8>, objects: &mut Vec<Vec<u8>>) {
    assert_eq!(
        id,
        objects.len().saturating_add(1),
        "objects are sequential"
    );
    objects.push(body);
}

fn document() -> Vec<u8> {
    let mut objects = Vec::new();
    object_body(
        1,
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        &mut objects,
    );
    object_body(
        2,
        b"<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R 6 0 R] /Count 4 >>".to_vec(),
        &mut objects,
    );
    for index in 0..4_usize {
        let page = index.saturating_add(3);
        let content = index.saturating_add(7);
        let annotation = index.saturating_add(11);
        object_body(
            page,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 200] \
                 /Resources << >> /Contents {content} 0 R /Annots [{annotation} 0 R] >>"
            )
            .into_bytes(),
            &mut objects,
        );
    }
    for content in 7..=10_usize {
        object_body(
            content,
            b"<< /Length 0 >>\nstream\n\nendstream".to_vec(),
            &mut objects,
        );
    }
    for annotation in 11..=14_usize {
        object_body(
            annotation,
            b"<< /Type /Annot /Subtype /Watermark /Rect [10 10 30 30] \
              /F 4 /AP << /N 15 0 R >> /FixedPrint << /Type /FixedPrint \
              /Matrix [1 0 0 1 0 0] /H 0.375 /V 0.25 >> >>"
                .to_vec(),
            &mut objects,
        );
    }
    let appearance = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> \
         /Length {} >>\nstream\n{APPEARANCE}\nendstream",
        APPEARANCE.len()
    );
    object_body(15, appearance.into_bytes(), &mut objects);

    let mut output = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.into_iter().enumerate() {
        offsets.push(output.len());
        output.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        output.extend_from_slice(&object);
        output.extend_from_slice(b"\nendobj\n");
    }
    let xref = output.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(output, "xref\n0 {size}");
    output.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(output, "{offset:010} 00000 n ");
    }
    let _ = write!(
        output,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    output
}

fn settle(viewer: &mut Viewer, events: Vec<Event>) {
    for event in events {
        let Event::NeedsRender(request) = event else {
            continue;
        };
        let raster = CpuRasterizer::new()
            .rasterize(&request.list, request.target)
            .expect("the small screen fixture rasterizes");
        let more: Vec<_> = viewer
            .handle(Command::RenderReady {
                token: request.token,
                rendered: Rendered::Raster(raster),
            })
            .collect();
        settle(viewer, more);
    }
}

fn opened_viewer(sheet: viewer_core::Sheet) -> Viewer {
    let bytes = document();
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("the fixture is a complete PDF");
    assert!(!parsed.was_recovered(), "all xref offsets are exact");
    let mut viewer = Viewer::new(400, 300, 1.0);
    let opened: Vec<_> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    settle(&mut viewer, opened);

    let started: Vec<_> = viewer
        .handle(Command::Print(Printing::Start(sheet)))
        .collect();
    assert!(
        started
            .iter()
            .any(|event| matches!(event, Event::Printing { .. })),
        "the document grants the print operation"
    );
    settle(&mut viewer, started);
    viewer
}

fn red_bounds(
    viewer: &Viewer,
    page_index: usize,
    sheet: viewer_core::Sheet,
    per_sheet: PagesPerSheet,
) -> BoxOnPaper {
    let Answer::PrintPage(page) = viewer.query(Query::PrintPage(page_index)) else {
        panic!("the granted job hands out page {page_index}");
    };
    assert!(
        page.reports.is_empty(),
        "the print page is complete: {:?}",
        page.reports
    );
    let target = page.target.expect("each print target is small");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&page.list, target)
        .expect("the public PrintPage display list rasterizes");
    let mut pixels = None::<(u32, u32, u32, u32)>;
    for (index, pixel) in raster.data.chunks_exact(4).enumerate() {
        if pixel[0] < 200 || pixel[1] > 20 || pixel[2] > 20 || pixel[3] == 0 {
            continue;
        }
        let width = usize::try_from(raster.width).expect("small raster width");
        let x = u32::try_from(index % width).expect("x fits the raster");
        let y = u32::try_from(index / width).expect("y fits the raster");
        pixels = Some(match pixels {
            Some((min_x, min_y, max_x, max_y)) => {
                (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
            }
            None => (x, y, x, y),
        });
    }
    let (min_x, min_y, max_x, max_y) = pixels.expect("the red watermark appears on the print page");

    // The raster is row-major from its top-left corner. Convert the measured pixel box back to
    // PDF page coordinates using its actual dimensions, then compose through the host's Cell.
    let target_scale_x = raster.width as f32 / page.list.page_size.width;
    let target_scale_y = raster.height as f32 / page.list.page_size.height;
    let page_left = min_x as f32 / target_scale_x;
    let page_right = max_x.saturating_add(1) as f32 / target_scale_x;
    let page_bottom = page.list.page_size.height - max_y.saturating_add(1) as f32 / target_scale_y;
    let page_top = page.list.page_size.height - min_y as f32 / target_scale_y;
    let paper_cell = cell(
        PAPER,
        PAGE,
        Scaling::ShrinkToFit,
        per_sheet,
        page_index % per_sheet.count(),
    )
    .expect("the host places this page in its sheet cell");

    BoxOnPaper {
        left: paper_cell.origin.0 + sheet.page_scale * page_left,
        bottom: paper_cell.origin.1 + sheet.page_scale * page_bottom,
        right: paper_cell.origin.0 + sheet.page_scale * page_right,
        top: paper_cell.origin.1 + sheet.page_scale * page_top,
    }
}

fn assert_near(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= ERROR_TOLERANCE,
        "{what}: expected {expected}, got {actual}"
    );
}

fn assert_box(actual: BoxOnPaper, expected: BoxOnPaper) {
    assert_near(actual.left, expected.left, "left edge");
    assert_near(actual.bottom, expected.bottom, "bottom edge");
    assert_near(actual.right, expected.right, "right edge");
    assert_near(actual.top, expected.top, "top edge");
    assert_near(actual.right - actual.left, 20.0, "fixed physical width");
    assert_near(actual.top - actual.bottom, 20.0, "fixed physical height");
}

fn assert_inside(actual: BoxOnPaper, cell: (f32, f32, f32, f32)) {
    assert!(
        actual.left >= cell.0 - ERROR_TOLERANCE
            && actual.bottom >= cell.1 - ERROR_TOLERANCE
            && actual.right <= cell.2 + ERROR_TOLERANCE
            && actual.top <= cell.3 + ERROR_TOLERANCE,
        "watermark {actual:?} stays inside its cell {cell:?}"
    );
}

fn overlap(left: BoxOnPaper, right: BoxOnPaper) -> bool {
    left.left < right.right
        && right.left < left.right
        && left.bottom < right.top
        && right.bottom < left.top
}

#[test]
fn public_print_query_places_fixed_watermarks_inside_each_two_up_and_four_up_cell() {
    // Two-up on 240×180 paper: cells are 120×180, with the 100×200 page shrunk by 0.9.
    let two_sheet = placed(PAPER, PAGE, Scaling::ShrinkToFit, PagesPerSheet::Two, 1.0);
    assert_near(two_sheet.page_scale, 0.9, "two-up page scale");
    let two_viewer = opened_viewer(two_sheet);
    let two = [
        red_bounds(&two_viewer, 0, two_sheet, PagesPerSheet::Two),
        red_bounds(&two_viewer, 1, two_sheet, PagesPerSheet::Two),
    ];
    let two_expected = [
        BoxOnPaper {
            left: 45.0,
            bottom: 45.0,
            right: 65.0,
            top: 65.0,
        },
        BoxOnPaper {
            left: 165.0,
            bottom: 45.0,
            right: 185.0,
            top: 65.0,
        },
    ];
    let two_cells = [(0.0, 0.0, 120.0, 180.0), (120.0, 0.0, 240.0, 180.0)];
    for index in 0..2_usize {
        assert_box(two[index], two_expected[index]);
        assert_inside(two[index], two_cells[index]);
    }
    assert!(
        !overlap(two[0], two[1]),
        "two-up watermark boxes do not overlap"
    );

    // Four-up uses 120×90 cells and a 0.45 page scale. The same 20-point mark stays in each cell.
    let four_sheet = placed(PAPER, PAGE, Scaling::ShrinkToFit, PagesPerSheet::Four, 1.0);
    assert_near(four_sheet.page_scale, 0.45, "four-up page scale");
    let four_viewer = opened_viewer(four_sheet);
    let four = [
        red_bounds(&four_viewer, 0, four_sheet, PagesPerSheet::Four),
        red_bounds(&four_viewer, 1, four_sheet, PagesPerSheet::Four),
        red_bounds(&four_viewer, 2, four_sheet, PagesPerSheet::Four),
        red_bounds(&four_viewer, 3, four_sheet, PagesPerSheet::Four),
    ];
    let four_expected = [
        BoxOnPaper {
            left: 45.0,
            bottom: 112.5,
            right: 65.0,
            top: 132.5,
        },
        BoxOnPaper {
            left: 165.0,
            bottom: 112.5,
            right: 185.0,
            top: 132.5,
        },
        BoxOnPaper {
            left: 45.0,
            bottom: 22.5,
            right: 65.0,
            top: 42.5,
        },
        BoxOnPaper {
            left: 165.0,
            bottom: 22.5,
            right: 185.0,
            top: 42.5,
        },
    ];
    let four_cells = [
        (0.0, 90.0, 120.0, 180.0),
        (120.0, 90.0, 240.0, 180.0),
        (0.0, 0.0, 120.0, 90.0),
        (120.0, 0.0, 240.0, 90.0),
    ];
    for index in 0..4_usize {
        assert_box(four[index], four_expected[index]);
        assert_inside(four[index], four_cells[index]);
        for earlier in 0..index {
            assert!(
                !overlap(four[index], four[earlier]),
                "watermarks on distinct 4-up pages do not overlap"
            );
        }
    }
}
