//! Public raster witness for ISO 32000-2 §11.7.4.5 Table 146's final row.
//!
//! The source says an explicit transparency group is not an elementary object and supplies
//! every colour component; the special compatible-overprinting blend therefore reverts to
//! Normal. This compares the group invocation with a direct ordinary paint, while a direct
//! elementary `DeviceCMYK` paint under the same OPM1 state is the discriminating negative control.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "tests fail loudly if the literal PDF fixture or its one-pixel witness is unavailable"
)]

use std::io::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

fn fixture(page_paint: &str) -> Vec<u8> {
    let page_stream = format!("{page_paint}\n");
    let form_stream = b"/Off gs 0.9 0 0 0 k 0 0 40 40 re f\n";
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Group << /S /Transparency /CS /DeviceCMYK >> \
          /Resources << /ExtGState << /Outer << /OP true /op true /OPM 1 >> >> \
          /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        format!(
            "<< /Length {} >>\nstream\n{}endstream",
            page_stream.len(),
            page_stream
        )
        .into_bytes(),
        {
            let mut body = format!(
                "<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 40 40] \
                 /Group << /S /Transparency /CS /DeviceCMYK /I true >> \
                 /Resources << /ExtGState << /Off << /OP false /op false /OPM 1 >> >> >> \
                 /Length {} >>\nstream\n",
                form_stream.len()
            )
            .into_bytes();
            body.extend_from_slice(form_stream);
            body.extend_from_slice(b"endstream");
            body
        },
    ];

    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        writeln!(&mut pdf, "{} 0 obj", index + 1).expect("write object header");
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    write!(&mut pdf, "xref\n0 {}\n", objects.len() + 1).expect("write xref header");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        writeln!(&mut pdf, "{offset:010} 00000 n ").expect("write xref entry");
    }
    write!(
        &mut pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("write trailer");
    pdf
}

fn centre(page_paint: &str) -> ([u8; 4], bool) {
    let document = Document::open(fixture(page_paint)).expect("valid exact-xref PDF 1.7");
    assert!(!document.was_recovered(), "fixture uses its exact xref");
    assert!(!document.cross_reference_chain_was_cut());
    assert_eq!(document.cross_reference_entries_lost(), 0);
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpreted = pdf_model::interpret(&document, &page);
    assert!(
        interpreted.is_complete(),
        "no chain cut or unsupported paint: {:?}",
        interpreted.unsupported
    );
    assert!(
        interpreted.display_list.blending().is_some(),
        "the page's four-component group is retained"
    );
    let list = &interpreted.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("40x40 raster target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("rasterize the literal witness");
    let at = ((20 * raster.width) + 20) as usize * 4;
    (
        [
            raster.data[at],
            raster.data[at + 1],
            raster.data[at + 2],
            raster.data[at + 3],
        ],
        interpreted.display_list.overprints(),
    )
}

#[test]
fn a_transparency_group_invocation_uses_the_group_row_not_elementary_opm1() {
    let backdrop = "0 0 0 1 k 0 0 40 40 re f";
    let grouped = centre(&format!("{backdrop} /Outer gs /Fm Do"));
    let ordinary_direct = centre(&format!("{backdrop} 0.9 0 0 0 k 0 0 40 40 re f"));
    let elementary_opm1 = centre(&format!("{backdrop} /Outer gs 0.9 0 0 0 k 0 0 40 40 re f"));
    let untouched = centre(backdrop);

    assert_eq!(
        grouped.0, ordinary_direct.0,
        "Table 146 group row is Cs in all columns"
    );
    assert_ne!(
        grouped.0, elementary_opm1.0,
        "the direct elementary first row keeps zero-tint backdrop channels under OPM1"
    );
    assert_ne!(
        grouped.0, untouched.0,
        "the region contains the opaque source paint"
    );
    assert_eq!(grouped.0[3], 255, "the source region is fully opaque");
    assert!(
        !grouped.1,
        "an explicit group paint does not select the elementary special overprint blend"
    );
    assert!(
        elementary_opm1.1,
        "the direct OPM1 elementary control does select it"
    );
}
