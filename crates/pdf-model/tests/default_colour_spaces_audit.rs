//! Reader regressions for §8.6.5.6's current-resource lookup in a Form `XObject`.
//!
//! Complete PDF fixtures compare nested default substitution with absence controls.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a broken fixture or out-of-raster sample must fail loudly; coordinates are bounded"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;

fn fixture(form_resources: &str) -> Vec<u8> {
    let page_content = "1 0 0 rg 0 50 100 50 re f /Fm Do";
    let form_content = "1 0 0 rg 0 0 100 50 re f";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 50] \
         /Resources << {form_resources} >> /Length {} >>\nstream\n{form_content}\nendstream",
        form_content.len().saturating_add(1)
    );
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{page_content}\nendstream",
            page_content.len().saturating_add(1)
        ),
        form,
    ];

    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(bodies.len());
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(bytes.len());
        writeln!(bytes, "{} 0 obj\n{body}\nendobj", index + 1)
            .expect("writing to String cannot fail");
    }
    let xref = bytes.len();
    writeln!(bytes, "xref\n0 {}\n0000000000 65535 f ", bodies.len() + 1)
        .expect("writing to String cannot fail");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("writing to String cannot fail");
    }
    writeln!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF",
        bodies.len() + 1
    )
    .expect("writing to String cannot fail");
    bytes.into_bytes()
}

fn plain_fixture(resources: &str, content: &str, extra_objects: &[String]) -> Vec<u8> {
    let mut bodies = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] \
             /Resources << {resources} >> /Contents 4 0 R >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len().saturating_add(1)
        ),
    ];
    bodies.extend(extra_objects.iter().cloned());

    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(bodies.len());
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(bytes.len());
        writeln!(bytes, "{} 0 obj\n{body}\nendobj", index + 1)
            .expect("writing to String cannot fail");
    }
    let xref = bytes.len();
    writeln!(bytes, "xref\n0 {}\n0000000000 65535 f ", bodies.len() + 1)
        .expect("writing to String cannot fail");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("writing to String cannot fail");
    }
    writeln!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF",
        bodies.len() + 1
    )
    .expect("writing to String cannot fail");
    bytes.into_bytes()
}

fn raster(bytes: Vec<u8>) -> pdf_render::backend::Raster {
    let document = Document::open(bytes).expect("complete PDF fixture");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture draws completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("bounded target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("CPU rasterizer supports the fixture")
}

fn pixel(image: &pdf_render::backend::Raster, x: u32, y: u32) -> [u8; 4] {
    let index = (y * image.width + x) as usize * 4;
    image
        .data
        .get(index..index + 4)
        .expect("sample coordinate is inside the raster")
        .try_into()
        .expect("four channels per raster pixel")
}

#[test]
fn a_form_uses_its_own_default_rgb_for_its_content_stream() {
    // The same red operator paints both the page and a Form `XObject`. The page has no default;
    // the form's explicit resources define a valid three-component CalRGB DefaultRGB whose
    // first basis vector maps red strongly to green. §8.6.5.6 says the current resource
    // dictionary governs the operation, so only the form's red changes. This detects using the
    // invoking page's resources for Form content, a route distinct from painting a shading that
    // carries its own /ColorSpace entry.
    let cal_rgb = "/ColorSpace << /DefaultRGB [/CalRGB << /WhitePoint [0.9505 1 1.089] \
                  /Matrix [0.3576 0.7152 0.1192 0.4124 0.2126 0.0193 \
                  0.1805 0.0722 0.9505] >>] >>";

    // Positive control first: with no default in the form, its red and the page's red remain
    // the same device colour. The page mark also proves the fixture paints outside the form.
    let unchanged = raster(fixture(""));
    assert_eq!(
        pixel(&unchanged, 50, 25),
        [255, 0, 0, 255],
        "page control is red"
    );
    assert_eq!(
        pixel(&unchanged, 50, 75),
        [255, 0, 0, 255],
        "form control is red"
    );

    let remapped = raster(fixture(cal_rgb));
    assert_eq!(
        pixel(&remapped, 50, 25),
        [255, 0, 0, 255],
        "the page lacks DefaultRGB and stays in DeviceRGB"
    );
    let form_pixel = pixel(&remapped, 50, 75);
    assert!(
        form_pixel[0] < 40 && form_pixel[1] > 215 && form_pixel[2] < 40,
        "the form's own DefaultRGB makes its red mark green: {form_pixel:?}"
    );
}

#[test]
fn default_rgb_remaps_the_device_base_of_an_indexed_space() {
    // §8.6.5.6 explicitly includes an Indexed space's base. A one-entry red lookup table is
    // the indexed value; DefaultRGB changes the base interpretation to the valid CalRGB space
    // used by the Form test above. Without the entry the table value is red, providing a
    // positive absence control.
    let indexed = "[/Indexed /DeviceRGB 0 <FF0000>]".to_owned();
    let resources_without = "/ColorSpace << /Ix 5 0 R >>";
    let resources_with = format!(
        "/ColorSpace << /Ix 5 0 R /DefaultRGB [{CAL_RGB}] >>",
        CAL_RGB = "/CalRGB << /WhitePoint [0.9505 1 1.089] \
                   /Matrix [0.3576 0.7152 0.1192 0.4124 0.2126 0.0193 \
                   0.1805 0.0722 0.9505] >>"
    );
    let content = "/Ix cs 0 scn 0 0 20 20 re f";

    let unchanged = raster(plain_fixture(
        resources_without,
        content,
        std::slice::from_ref(&indexed),
    ));
    assert_eq!(
        pixel(&unchanged, 10, 10),
        [255, 0, 0, 255],
        "base is DeviceRGB"
    );

    let remapped = raster(plain_fixture(&resources_with, content, &[indexed]));
    let mapped = pixel(&remapped, 10, 10);
    assert!(
        mapped[0] < 40 && mapped[1] > 215 && mapped[2] < 40,
        "the Indexed entry's red base value uses DefaultRGB: {mapped:?}"
    );
}

#[test]
fn default_rgb_remaps_the_underlying_space_of_an_uncoloured_tiling_pattern() {
    // §8.6.5.6 explicitly includes the underlying space of Pattern. The cell's paint is a
    // stencil; the parent supplies red in the Pattern base, which DefaultRGB remaps to green.
    let cell = "0 0 10 10 re f";
    let pattern = format!(
        "<< /Type /Pattern /PatternType 1 /PaintType 2 /TilingType 1 /BBox [0 0 10 10] \
         /XStep 10 /YStep 10 /Resources << >> /Length {} >>\nstream\n{cell}\nendstream",
        cell.len().saturating_add(1)
    );
    let pattern_space = "[/Pattern /DeviceRGB]".to_owned();
    let resources_without = "/Pattern << /P0 5 0 R >> /ColorSpace << /PS 6 0 R >>";
    let resources_with = format!(
        "/Pattern << /P0 5 0 R >> /ColorSpace << /PS 6 0 R /DefaultRGB [{CAL_RGB}] >>",
        CAL_RGB = "/CalRGB << /WhitePoint [0.9505 1 1.089] \
                   /Matrix [0.3576 0.7152 0.1192 0.4124 0.2126 0.0193 \
                   0.1805 0.0722 0.9505] >>"
    );
    let content = "/PS cs 1 0 0 /P0 scn 0 0 20 20 re f";
    let objects = [pattern, pattern_space];

    let unchanged = raster(plain_fixture(resources_without, content, &objects));
    assert_eq!(
        pixel(&unchanged, 10, 10),
        [255, 0, 0, 255],
        "base is DeviceRGB"
    );

    let remapped = raster(plain_fixture(&resources_with, content, &objects));
    let mapped = pixel(&remapped, 10, 10);
    assert!(
        mapped[0] < 40 && mapped[1] > 215 && mapped[2] < 40,
        "uncoloured pattern's supplied red uses DefaultRGB: {mapped:?}"
    );
}

#[test]
fn a_selected_devicen_alternate_uses_the_current_default_rgb() {
    let program = "{ pop pop 1 0 0 }";
    let tint = format!(
        "<< /FunctionType 4 /Domain [0 1 0 1] /Range [0 1 0 1 0 1] /Length {} >>\nstream\n{program}\nendstream",
        program.len().saturating_add(1)
    );
    let space = "[/DeviceN [/SpotA /SpotB] /DeviceRGB 6 0 R]".to_owned();
    let objects = [space, tint];
    let content = "/Ink cs 0.25 0.75 scn 0 0 20 20 re f";
    let bare = "/ColorSpace << /Ink 5 0 R >>";
    let remapped = "/ColorSpace << /Ink 5 0 R /DefaultRGB [/CalRGB \
        << /WhitePoint [0.9505 1 1.089] /Matrix [0.3576 0.7152 0.1192 \
        0.4124 0.2126 0.0193 0.1805 0.0722 0.9505] >>] >>";
    assert_eq!(
        pixel(&raster(plain_fixture(bare, content, &objects)), 10, 10),
        [255, 0, 0, 255],
        "without a default the selected alternate's red is device red"
    );
    let mapped = pixel(&raster(plain_fixture(remapped, content, &objects)), 10, 10);
    assert!(
        mapped[0] < 40 && mapped[1] > 215 && mapped[2] < 40,
        "the selected DeviceN alternate is remapped: {mapped:?}"
    );
}

#[test]
fn selecting_a_defaulted_device_cmyk_keeps_the_device_initial_components() {
    // The default's DeviceN initial vector is [1 1 1 1], while selecting DeviceCMYK
    // initializes [0 0 0 1]. This transform makes those distinguishable: cyan zero is
    // red, cyan one is black. Compare implicit selection with explicit operands and
    // with the same DeviceN space selected as its own family.
    let program = "{ pop pop pop 1 exch sub 0 0 }";
    let tint = format!(
        "<< /FunctionType 4 /Domain [0 1 0 1 0 1 0 1] /Range [0 1 0 1 0 1] /Length {} >>\nstream\n{program}\nendstream",
        program.len().saturating_add(1)
    );
    let default = "[/DeviceN [/Cyan /Magenta /Yellow /Black] /DeviceRGB 6 0 R]".to_owned();
    let objects = [default, tint];
    let resources = "/ColorSpace << /DefaultCMYK 5 0 R /Own 5 0 R >>";
    for (content, expected) in [
        (
            "/DeviceCMYK cs 0 0 0 1 scn 0 0 20 20 re f",
            [255, 0, 0, 255],
        ),
        ("/Own cs 0 0 20 20 re f", [0, 0, 0, 255]),
        ("/DeviceCMYK cs 0 0 20 20 re f", [255, 0, 0, 255]),
    ] {
        assert_eq!(
            pixel(&raster(plain_fixture(resources, content, &objects)), 10, 10),
            expected,
            "selection and initial components: {content}"
        );
    }
}
