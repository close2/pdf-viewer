//! Complete-PDF consumer regressions for §8.9.6 image masking.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "fixtures are controlled complete PDFs; malformed data should fail loudly"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;
const WHITE: [u8; 4] = [255, 255, 255, 255];
const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

fn stream_body(dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut body = format!("<< {dictionary} /Length {} >>\nstream\n", data.len()).into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(b"\nendstream");
    body
}

/// A one-page PDF with parent image object 5 and optional image objects beginning at 6.
fn pdf(
    image_dictionary: &str,
    image_data: &[u8],
    content: &str,
    extra_images: &[(&str, &[u8])],
) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 60 40] \
           /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream_body("", content.as_bytes()),
        stream_body(
            &format!("/Type /XObject /Subtype /Image {image_dictionary}"),
            image_data,
        ),
    ];
    for (dictionary, data) in extra_images {
        objects.push(stream_body(
            &format!("/Type /XObject /Subtype /Image {dictionary}"),
            data,
        ));
    }
    assemble(&objects)
}

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, body) in (1_usize..).zip(objects) {
        offsets.push(out.len());
        out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    out
}

fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    render_allowing_diagnostics(bytes, false)
}

fn render_allowing_diagnostics(bytes: Vec<u8>, allow_diagnostics: bool) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("valid complete one-page PDF");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let state = pdf_model::view::ViewState::of(&document);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        allow_diagnostics || interpretation.is_complete(),
        "supported complete fixture: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("target fits");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported rasterization")
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let at = ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

fn assert_gray(pixel: [u8; 4], expected: u8, what: &str) {
    for channel in &pixel[..3] {
        assert!(
            channel.abs_diff(expected) <= 2,
            "{what}: expected gray {expected}, got {pixel:?}"
        );
    }
    assert_eq!(pixel[3], 255, "{what}: opaque sample");
}

#[test]
fn a_colour_key_compares_raw_samples_before_a_nonidentity_decode_array() {
    let content = "q 40 0 0 20 10 10 cm /Im Do Q";
    let samples = [32, 192];
    let unmasked = render(pdf(
        "/Width 2 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8 /Decode [1 0]",
        &samples,
        content,
        &[],
    ));
    let keyed = render(pdf(
        "/Width 2 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8 \
         /Decode [1 0] /Mask [32 32]",
        &samples,
        content,
        &[],
    ));

    // Raw 32 is the key and must disappear although /Decode maps it to roughly 223. Raw 192
    // maps to about 63 and lies outside the range, so it remains. The unmasked positive control
    // confirms both decoded values independently of the key.
    assert_gray(
        pixel(&unmasked, 20, 20),
        223,
        "raw 32 decoded through [1 0]",
    );
    assert_gray(
        pixel(&unmasked, 40, 20),
        63,
        "raw 192 decoded through [1 0]",
    );
    assert_eq!(
        pixel(&keyed, 20, 20),
        WHITE,
        "raw key value 32 is transparent"
    );
    assert_gray(
        pixel(&keyed, 40, 20),
        63,
        "raw non-key value 192 remains visible",
    );
}

#[test]
fn a_coarser_explicit_stencil_spans_the_base_images_unit_square() {
    let content = "q 40 0 0 20 10 10 cm /Im Do Q";
    let base = [255, 0, 0].repeat(8); // 4x2 solid red base image.
    let unmasked = render(pdf(
        "/Width 4 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8",
        &base,
        content,
        &[],
    ));
    let masked = render(pdf(
        "/Width 4 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask 6 0 R",
        &base,
        content,
        &[(
            "/Width 2 /Height 1 /ImageMask true /BitsPerComponent 1",
            &[0b0100_0000],
        )],
    ));

    // The two stencil samples cover the left and right halves of the whole user-space square.
    // Under default /Decode, zero marks and one leaves the existing white page untouched.
    for y in [12, 27] {
        assert_eq!(
            pixel(&unmasked, 45, y),
            RED,
            "positive control paints full width"
        );
        assert_eq!(
            pixel(&masked, 15, y),
            RED,
            "first coarse-mask cell marks left half"
        );
        assert_eq!(
            pixel(&masked, 25, y),
            RED,
            "left half remains painted across base rows"
        );
        assert_eq!(
            pixel(&masked, 35, y),
            WHITE,
            "second coarse-mask cell clears right half"
        );
        assert_eq!(
            pixel(&masked, 45, y),
            WHITE,
            "mask cell spans the full right half"
        );
    }
}

#[test]
fn an_image_soft_mask_overrides_a_colour_key_mask() {
    let content = "q 40 0 0 20 10 10 cm /Im Do Q";
    let image = [255, 0, 0, 0, 0, 255]; // red, blue
    let no_soft_mask = render(pdf(
        "/Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 \
         /Mask [255 255 0 0 0 0]",
        &image,
        content,
        &[],
    ));
    let with_soft_mask = render(pdf(
        "/Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 \
         /Mask [255 255 0 0 0 0] /SMask 6 0 R",
        &image,
        content,
        &[(
            "/Width 2 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8",
            &[255, 255],
        )],
    ));

    // The no-SMask control proves the color key would hide red. EC3 #215's effective Table 87
    // /SMask entry, consistent with §11.6.4.3, makes the opaque soft mask win over that key.
    assert_eq!(
        pixel(&no_soft_mask, 20, 20),
        WHITE,
        "color key alone hides red"
    );
    assert_eq!(
        pixel(&no_soft_mask, 40, 20),
        BLUE,
        "color key leaves blue visible"
    );
    assert_eq!(
        pixel(&with_soft_mask, 20, 20),
        RED,
        "soft mask overrides key for red"
    );
    assert_eq!(
        pixel(&with_soft_mask, 40, 20),
        BLUE,
        "soft mask leaves blue visible"
    );
}

#[test]
fn masked_regions_preserve_existing_coloured_page_marks() {
    let content = "0 1 0 rg 0 0 60 40 re f 1 0 0 rg q 40 0 0 20 10 10 cm /Im Do Q";
    let green = [0, 255, 0, 255];
    // BitsPerComponent may be omitted on an ImageMask. The absent case must still read bits,
    // while the marked and untouched regions distinguish a dropped image or white erasure.
    let stencil = render(pdf(
        "/Width 2 /Height 1 /ImageMask true",
        &[0b0100_0000],
        content,
        &[],
    ));
    assert_eq!(pixel(&stencil, 20, 20), RED);
    assert_eq!(pixel(&stencil, 40, 20), green);
    let explicit = render(pdf(
        "/Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask 6 0 R",
        &[255, 0, 0, 255, 0, 0],
        content,
        &[(
            "/Width 2 /Height 1 /ImageMask true /BitsPerComponent 1",
            &[0b0100_0000],
        )],
    ));
    assert_eq!(pixel(&explicit, 20, 20), RED);
    assert_eq!(pixel(&explicit, 40, 20), green);
    let keyed = render(pdf(
        "/Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask [255 255 0 0 0 0]",
        &[255, 0, 0, 0, 0, 255],
        content,
        &[],
    ));
    assert_eq!(pixel(&keyed, 20, 20), green);
    assert_eq!(pixel(&keyed, 40, 20), BLUE);
    for raster in [stencil, explicit, keyed] {
        assert_eq!(
            pixel(&raster, 5, 20),
            green,
            "outside the image stays untouched"
        );
    }
}

#[test]
fn ec3_image_mask_ignores_its_own_mask_and_colour_space_entries() {
    // EC3 #215, Table87 ImageMask/Mask entries, requires these forbidden producer entries to
    // be ignored if present. Diagnostics are permitted here; only an altered image is a defect.
    let content = "0 1 0 rg 0 0 60 40 re f 1 0 0 rg q 40 0 0 20 10 10 cm /Im Do Q";
    for extra in ["/ColorSpace /DeviceRGB", "/Mask [0 1]", "/Mask 6 0 R"] {
        let image = render_allowing_diagnostics(
            pdf(
                &format!("/Width 2 /Height 1 /ImageMask true /BitsPerComponent 1 {extra}"),
                &[0b0100_0000],
                content,
                &[(
                    "/Width 2 /Height 1 /ImageMask true /BitsPerComponent 1",
                    &[0b1100_0000],
                )],
            ),
            true,
        );
        assert_eq!(
            pixel(&image, 20, 20),
            RED,
            "{extra} must not suppress the stencil mark"
        );
        assert_eq!(
            pixel(&image, 40, 20),
            [0, 255, 0, 255],
            "{extra} must not paint the clear sample"
        );
    }
}

#[test]
fn an_image_mask_with_more_than_one_bit_per_component_reports_inconsistency() {
    let content = "1 0 0 rg q 40 0 0 20 10 10 cm /Im Do Q";
    let control = render(pdf(
        "/Width 2 /Height 1 /ImageMask true /BitsPerComponent 1",
        &[0b0100_0000],
        content,
        &[],
    ));
    assert_eq!(pixel(&control, 20, 20), RED);
    assert_eq!(pixel(&control, 40, 20), WHITE);
    // Unlike Mask/ColorSpace, EC3 does not say to ignore this entry. Table87 requires 1 when
    // present, and §8.9.5.1 requires an error for inconsistent entries of the base image.
    let bytes = pdf(
        "/Width 2 /Height 1 /ImageMask true /BitsPerComponent 2",
        &[0b0100_0000],
        content,
        &[],
    );
    let document = Document::open(bytes).expect("complete PDF with one deliberate inconsistency");
    assert!(!document.was_recovered());
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let result = pdf_model::interpret(&document, &page);
    assert!(
        !result.is_complete(),
        "ImageMask true with BitsPerComponent 2 must report an error: {:?}",
        result.unsupported
    );
}
