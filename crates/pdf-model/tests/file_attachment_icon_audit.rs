//! `FileAttachment` icon defaults and stored-appearance precedence.
//!
//! Missing AP cases deliberately exercise the project's construction policy. Table187 recommends
//! the named artwork but gives no pixels; distinguishing the chosen drawings is a product guard.

#![expect(
    clippy::expect_used,
    reason = "invalid fixture or unsupported rendering must fail"
)]
use pdf_render::{Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;

fn raster(name: Option<&str>, stored: bool) -> pdf_render::Raster {
    let name = name.map_or_else(String::new, |n| format!("/Name /{n}"));
    let ap = if stored { "/AP << /N 6 0 R >>" } else { "" };
    let paint = "1 0 0 rg 0 0 20 20 re f";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 60 60] /Resources << >> /Annots [5 0 R] >>"
            .to_owned(),
        "null".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /FileAttachment /Rect [20 20 40 40] /F 4 /FS 7 0 R {name} {ap} >>"
        ),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Resources << >> /Length {} >>\nstream\n{paint}\nendstream",
            paint.len()
        ),
        "<< /Type /Filespec /F (payload.txt) /EF << /F 8 0 R >> >>".to_owned(),
        "<< /Type /EmbeddedFile /Length 1 >>\nstream\nx\nendstream".to_owned(),
    ];
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        writeln!(pdf, "{} 0 obj\n{object}\nendobj", index.saturating_add(1)).expect("object");
    }
    let xref = pdf.len();
    writeln!(
        pdf,
        "xref\n0 {}\n0000000000 65535 f ",
        offsets.len().saturating_add(1)
    )
    .expect("xref");
    for offset in &offsets {
        writeln!(pdf, "{offset:010} 00000 n ").expect("offset");
    }
    write!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len().saturating_add(1)
    )
    .expect("trailer");
    let document = pdf_syntax::Document::open(pdf.into_bytes()).expect("complete PDF");
    assert!(!document.was_recovered());
    let page = pdf_model::Pages::new(&document).get(0).expect("page");
    let result = pdf_model::interpret(&document, &page);
    assert!(result.is_complete(), "{:?}", result.unsupported);
    let target = TargetSpec::for_page(&result.display_list, 1.0, 1 << 20).expect("target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&result.display_list, target)
        .expect("raster")
}

#[test]
fn attachment_name_defaults_to_pushpin_and_the_chosen_four_icons_differ() {
    let pin = raster(Some("PushPin"), false);
    assert!(
        pin.data
            .chunks_exact(4)
            .any(|p| p.last().is_some_and(|a| *a > 0))
    );
    assert!(
        raster(None, false).data == pin.data,
        "Table187 absent Name defaults PushPin"
    );
    let mut figures = vec![pin];
    for name in ["Graph", "Paperclip", "Tag"] {
        let figure = raster(Some(name), false);
        assert!(
            figure
                .data
                .chunks_exact(4)
                .any(|p| p.last().is_some_and(|a| *a > 0)),
            "{name}"
        );
        assert!(
            figures.iter().all(|prior| prior.data != figure.data),
            "chosen {name} drawing has not collapsed to another icon"
        );
        figures.push(figure);
    }
}

#[test]
fn attachment_stored_appearance_wins_over_every_icon_name() {
    let stored = raster(Some("PushPin"), true);
    assert!(
        stored.data.chunks_exact(4).any(|p| p == [255, 0, 0, 255]),
        "red AP positive control"
    );
    assert!(
        stored.data != raster(Some("PushPin"), false).data,
        "construction control differs"
    );
    for name in [None, Some("Tag"), Some("UnspecifiedCustomIcon")] {
        assert!(
            raster(name, true).data == stored.data,
            "stored AP independent of Name {name:?}"
        );
    }
}
