//! Independent public pixel witness for the §11.6.5.2 Filter/Decode-before-Matte ordering.
//!
//! The source image and mask use their own valid non-default Decode arrays. The source's
//! preblend equation is arranged with exact rational sample relationships, then compared with
//! an unblended original and a no-Matte decoy.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small hand-built PDF fixtures and in-bounds raster samples"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 20;

fn matte_decode_fixture(parent_sample: u8, matte: bool) -> Vec<u8> {
    let content = b"0 g 0 0 40 40 re f 40 0 0 40 0 0 cm /Im Do";
    let image = stream_object(
        5,
        "/Type /XObject /Subtype /Image /Width 1 /Height 1 \
         /ColorSpace /DeviceGray /BitsPerComponent 8 /Decode [0.25 0.75] /SMask 6 0 R",
        &[parent_sample],
    );
    let matte_entry = if matte { " /Matte [0.25]" } else { "" };
    let mask = stream_object(
        6,
        &format!(
            "/Type /XObject /Subtype /Image /Width 1 /Height 1 \
             /ColorSpace /DeviceGray /BitsPerComponent 8 /Decode [0 0.5]{matte_entry}"
        ),
        &[128],
    );
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", content),
        image,
        mask,
    ];
    assemble(&objects)
}

fn stream_object(number: usize, dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut object = format!(
        "{number} 0 obj\n<< {dictionary} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream\nendobj\n");
    object
}

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for object in objects {
        offsets.push(bytes.len());
        bytes.extend_from_slice(object);
    }
    let xref_at = bytes.len();
    let size = objects.len() + 1;
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes.extend_from_slice(trailer.as_bytes());
    bytes
}

fn center(bytes: Vec<u8>) -> [u8; 4] {
    let document = Document::open(bytes).expect("valid exact-xref PDF fixture");
    assert!(!document.was_recovered(), "fixture uses its exact xref");
    assert!(!document.cross_reference_chain_was_cut());
    assert_eq!(document.cross_reference_entries_lost(), 0);
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture interprets completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, PIXEL_BUDGET).expect("bounded target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported CPU raster");
    let row = raster.height - 1 - 20;
    let index = ((row * raster.width + 20) * 4) as usize;
    [
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
        raster.data[index + 3],
    ]
}

#[test]
fn matte_inverts_parent_samples_after_both_decode_arrays() {
    // Parent sample 64 / 255 decoded by [1/4, 3/4] is
    // m + alpha * (c - m) = 1/4 + (64/255) * (1/2) = 95.75/255.
    // Mask sample 128 / 255 decoded by [0, 1/2] is alpha = 64/255. With m=1/4,
    // inversion recovers c=3/4. The independent unblended control stores c=3/4 as
    // sample 255 under the same parent Decode. Both should paint 3/4 * 64/255 over black: 48.
    // Without Matte, the decoded preblended sample instead paints 24.
    let decoded_preblend_without_matte = center(matte_decode_fixture(64, false));
    assert!(
        (22..=26).contains(&decoded_preblend_without_matte[0]),
        "Decode is applied before compositing; expected the uncorrected preblend near 24, got {decoded_preblend_without_matte:?}"
    );
    assert_eq!(
        decoded_preblend_without_matte[1],
        decoded_preblend_without_matte[0]
    );
    assert_eq!(
        decoded_preblend_without_matte[2],
        decoded_preblend_without_matte[0]
    );
    assert_eq!(decoded_preblend_without_matte[3], 255);

    let unblended_positive_control = center(matte_decode_fixture(255, false));
    assert!(
        (46..=50).contains(&unblended_positive_control[0]),
        "the literal unblended source is 0.75 at alpha 64/255: {unblended_positive_control:?}"
    );
    assert_eq!(unblended_positive_control[1], unblended_positive_control[0]);
    assert_eq!(unblended_positive_control[2], unblended_positive_control[0]);
    assert_eq!(unblended_positive_control[3], 255);

    let matte_corrected = center(matte_decode_fixture(64, true));
    assert!(
        (46..=50).contains(&matte_corrected[0]),
        "Matte inversion must recover the original 0.75 source after Decode, got {matte_corrected:?}"
    );
    assert_eq!(matte_corrected[1], matte_corrected[0]);
    assert_eq!(matte_corrected[2], matte_corrected[0]);
    assert_eq!(matte_corrected[3], 255);
    assert!(
        matte_corrected[0].abs_diff(unblended_positive_control[0]) <= 1,
        "preblended+Matte and the original unblended sample must agree: {matte_corrected:?} vs {unblended_positive_control:?}"
    );
}
