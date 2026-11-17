//! Public-consumer geometry and saved-output controls for a same-document `/APRef`.
//!
//! It uses a page-tree page with inherited font resources, a crop box smaller than its media
//! box, and /Rotate 90. Four colored corner cells make the Table 31 rotation and §12.5.5
//! appearance transform visible as pixels. A crop-exterior cell is placed where an incorrect
//! MediaBox-as-BBox implementation would leak into the widget.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "integration fixture uses fixed in-bounds raster coordinates and exact PDF objects"
)]

use std::fmt::Write as _;

use pdf_model::{Pages, view::ViewState};
use pdf_render::{Medium, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn target_pdf() -> Vec<u8> {
    let content = concat!(
        "1 0 0 rg 5 5 20 20 re f\n",   // lower-left red
        "0 1 0 rg 75 5 20 20 re f\n",  // lower-right green
        "0 0 1 rg 5 75 20 20 re f\n",  // upper-left blue
        "1 1 0 rg 75 75 20 20 re f\n", // upper-right yellow
        "0 1 1 rg 105 5 10 10 re f\n", // outside CropBox: should be clipped
        "BT /InheritedOnly 10 Tf 0 g 30 40 Td (inherited-font) Tj ET\n",
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> /Names << /Pages << /Names [(stamp) 9 0 R] >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R 9 0 R] /Count 2 /Resources << /Font << /InheritedOnly 7 0 R >> >> >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /Annots [5 0 R] >>".to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Rect [40 40 140 140] /F 4 /FT /Btn /Ff 65536 /T (press) /AP << /N 8 0 R >> >>".to_owned(),
        "null".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /InheritedOnlyetica /Encoding /WinAnsiEncoding >>".to_owned(),
        "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << >> /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 120 120] /CropBox [0 0 100 100] /Rotate 90 /Group << /S /Transparency /CS /DeviceRGB /I true >> /Contents 10 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{content}endstream", content.len()),
    ];
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        writeln!(pdf, "{} 0 obj\n{object}\nendobj", index + 1).expect("String write");
    }
    let xref = pdf.len();
    writeln!(pdf, "xref\n0 {}\n0000000000 65535 f ", offsets.len() + 1).expect("String write");
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").expect("String write");
    }
    write!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("String write");
    pdf.into_bytes()
}

fn fdf() -> Document {
    Document::open(
        b"%FDF-1.2\n1 0 obj\n<< /FDF << /Fields [ << /T (press) /APRef << /N << /Name (stamp) >> >> >> ] >> >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n".to_vec(),
    )
    .expect("valid minimal FDF")
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    assert!(
        x < raster.width && y < raster.height,
        "sample is inside raster"
    );
    let row = raster.height - 1 - y;
    let i = ((row * raster.width + x) * 4) as usize;
    [
        raster.data[i],
        raster.data[i + 1],
        raster.data[i + 2],
        raster.data[i + 3],
    ]
}

fn check_pixels(raster: &pdf_render::Raster) {
    assert_eq!((raster.width, raster.height), (200, 200));
    // Clockwise90: (x,y)->(y,-x). BBox [0,-100,100,0] maps to Rect [40,40,140,140].
    assert_eq!(pixel(raster, 55, 125), [255, 0, 0, 255]);
    assert_eq!(pixel(raster, 55, 55), [0, 255, 0, 255]);
    assert_eq!(pixel(raster, 125, 125), [0, 0, 255, 255]);
    assert_eq!(pixel(raster, 125, 55), [255, 255, 0, 255]);
    // Crop-exterior cyan maps below the widget; an incorrect MediaBox BBox moves it inside.
    assert_eq!(
        pixel(raster, 50, 30)[3],
        0,
        "crop-exterior marker is clipped"
    );
    assert_eq!(
        pixel(raster, 50, 50),
        [0, 255, 0, 255],
        "MediaBox-as-BBox control"
    );
    assert_eq!(
        pixel(raster, 35, 60)[3],
        0,
        "left of widget remains transparent"
    );
    assert_eq!(
        pixel(raster, 145, 60)[3],
        0,
        "right of widget remains transparent"
    );
}

#[test]
fn apref_page_rotation_crop_and_inherited_resources_reach_the_widget_pixels() {
    let document = Document::open(target_pdf()).expect("valid exact-xref target PDF");
    assert!(!document.was_recovered(), "fixture xref offsets are exact");
    let mut view = ViewState::of(&document);
    let data = pdf_model::forms_data::FormsData::read(&fdf()).expect("FDF root");
    let outcome = view.import(&document, &data);
    assert_eq!(outcome.widgets, 1);
    assert!(outcome.refused.is_empty(), "{:?}", outcome.refused);

    let page = Pages::new(&document).get(0).expect("target page");
    let interpretation = pdf_model::content::interpret_with(&document, &page, &view);
    assert!(
        interpretation.is_complete(),
        "unsupported: {:?}",
        interpretation.unsupported
    );
    assert!(
        interpretation.text.contains("inherited-font"),
        "{}",
        interpretation.text
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20).expect("target");
    let raster = CpuRasterizer::new()
        .with_medium(Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("CPU raster");

    check_pixels(&raster);

    // A same-document page stays referential while live; the saved appearance stream must
    // preserve the same transform and resources in the target document's incremental update.
    let written = view.save(&document).expect("appearance can be written");
    let saved = Document::open(written.bytes).expect("saved update is a PDF");
    assert!(!saved.was_recovered(), "saved xref offsets are exact");
    // Separate page-/Group transfer control: Table 31 page Group must follow the source page
    // marks into the generated Form XObject. Assert the serialized dictionary itself; generic
    // annotations.rs tests separately establish what the downstream appearance Group means.
    let widget_object = saved.get(pdf_syntax::ObjectId {
        number: 5,
        generation: 0,
    });
    let widget = widget_object.as_dict().expect("widget dictionary");
    let appearances = saved.get_key(widget, "AP");
    let appearances = appearances.as_dict().expect("saved /AP dictionary");
    let normal = saved.get_key(appearances, "N");
    let normal = normal.as_stream().expect("saved normal appearance stream");
    let group_object = saved.get_key(&normal.dict, "Group");
    let group = group_object
        .as_dict()
        .expect("page /Group is carried to the form");
    assert_eq!(
        saved
            .get_key(group, "S")
            .as_name()
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"Transparency"[..])
    );
    assert_eq!(
        saved
            .get_key(group, "CS")
            .as_name()
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"DeviceRGB"[..])
    );
    assert!(matches!(
        saved.get_key(group, "I"),
        pdf_syntax::Object::Boolean(true)
    ));
    let saved_page = Pages::new(&saved).get(0).expect("saved target page");
    let saved_interpretation =
        pdf_model::content::interpret_with(&saved, &saved_page, &ViewState::of(&saved));
    assert!(
        saved_interpretation.is_complete(),
        "{:?}",
        saved_interpretation.unsupported
    );
    assert!(saved_interpretation.text.contains("inherited-font"));
    let saved_target =
        TargetSpec::for_page(&saved_interpretation.display_list, 1.0, 1 << 20).expect("target");
    let saved_raster = CpuRasterizer::new()
        .with_medium(Medium::NONE)
        .rasterize(&saved_interpretation.display_list, saved_target)
        .expect("saved CPU raster");
    check_pixels(&saved_raster);
}
