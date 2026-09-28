//! §11.6.5.2's pre-blending is undone by one computation, whichever filter carried the samples.
//!
//! The clause states the inversion once — `c′ = m + α × (c − m)` run backwards, "performed
//! independently for each component" in the colour space the parent image's `/ColorSpace` names
//! — so a picture pre-blended with a matte draws the same pixels whether its
//! samples came out of a `DCTDecode` frame or were written raw. The fixture pair here is that
//! equivalence: the samples a flat `DCTDecode` frame decodes to are written again as a raw
//! image, both carry the same `/SMask` with the same `/Matte`, and the two rasters must be
//! byte-identical. The mask runs through every opacity the grid holds room for and the matte
//! sits between sample values, so a route that rounded differently, or carried the matte into
//! samples before undoing it, would show. ADR 1268.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and every index below is into \
              a buffer this file built at a size it also states"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;

/// An 8×8 baseline JPEG whose components are flat at the stated sample values, with an Adobe
/// `APP14` segment of transform 0 so that the samples are the components (§7.4.8, Table 13).
///
/// Each block holds a DC coefficient and an immediate end-of-block, so the inverse DCT makes
/// every sample of that component the same value; ISO/IEC 10918-1's level shift puts the zero of
/// that scale at 128 and a quantisation table of ones keeps the coefficient as written.
fn flat_jpeg(samples: &[u8]) -> Vec<u8> {
    let components = u8::try_from(samples.len()).expect("a frame of at most 255 components");
    let mut out = vec![0xFF, 0xD8];
    out.extend_from_slice(&[0xFF, 0xEE, 0x00, 0x0E]);
    out.extend_from_slice(b"Adobe");
    out.extend_from_slice(&[0x00, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00]);
    out.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x43, 0x00]);
    out.extend_from_slice(&[1u8; 64]);
    let frame_header = 8 + 3 * u16::from(components);
    out.extend_from_slice(&[0xFF, 0xC0]);
    out.extend_from_slice(&frame_header.to_be_bytes());
    out.extend_from_slice(&[0x08, 0x00, 0x08, 0x00, 0x08]);
    out.push(components);
    for id in 1..=components {
        out.extend_from_slice(&[id, 0x11, 0x00]);
    }
    out.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x1F, 0x00]);
    out.extend_from_slice(&DC_BITS);
    out.extend_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    out.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x15, 0x10]);
    out.extend_from_slice(&[0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&[0x00, 0x01]);
    let scan_header = 6 + 2 * u16::from(components);
    out.extend_from_slice(&[0xFF, 0xDA]);
    out.extend_from_slice(&scan_header.to_be_bytes());
    out.push(components);
    for id in 1..=components {
        out.extend_from_slice(&[id, 0x00]);
    }
    out.extend_from_slice(&[0x00, 0x3F, 0x00]);
    let mut bits: Vec<u8> = Vec::new();
    for sample in samples {
        dc(&mut bits, (i32::from(*sample) - 128) * 8);
        push(&mut bits, 0b00, 2);
    }
    while !bits.len().is_multiple_of(8) {
        bits.push(1);
    }
    for byte in bits.chunks_exact(8) {
        let packed = byte.iter().fold(0u8, |acc, bit| (acc << 1) | bit);
        out.push(packed);
        if packed == 0xFF {
            out.push(0x00);
        }
    }
    out.extend_from_slice(&[0xFF, 0xD9]);
    out
}

/// ISO/IEC 10918-1 Annex K's luminance DC `BITS` list.
const DC_BITS: [u8; 16] = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];

/// Appends the `length` low bits of `code`, most significant first.
fn push(bits: &mut Vec<u8>, code: u32, length: u32) {
    for index in (0..length).rev() {
        bits.push(u8::try_from((code >> index) & 1).expect("one bit"));
    }
}

/// Appends one DC difference as ISO/IEC 10918-1 section F.1.2.1 codes it.
fn dc(bits: &mut Vec<u8>, difference: i32) {
    let category = u32::BITS - difference.unsigned_abs().leading_zeros();
    let (mut code, mut index) = (0u32, 0u32);
    let mut found = None;
    for (offset, count) in DC_BITS.iter().enumerate() {
        let length = u32::try_from(offset).expect("sixteen lengths") + 1;
        for _ in 0..*count {
            if index == category && found.is_none() {
                found = Some((code, length));
            }
            index += 1;
            code += 1;
        }
        code <<= 1;
    }
    let (code, length) = found.expect("the table holds every category written here");
    push(bits, code, length);
    if category > 0 {
        let value = if difference > 0 {
            difference
        } else {
            difference + (1 << category) - 1
        };
        push(bits, u32::try_from(value).expect("non-negative"), category);
    }
}

/// The mask: all 64 samples distinct, from fully transparent to opaque.
fn mask() -> Vec<u8> {
    (0..64u16)
        .map(|index| u8::try_from(index * 4 + index / 16).expect("at most 255"))
        .collect()
}

/// A one-page PDF drawing one 8×8 image whose dictionary says `entries`, over `data`.
fn page(space: &str, entries: &str, data: &[u8], smask: bool, matte: &str) -> Vec<u8> {
    let content = b"8 0 0 8 0 0 cm /Im0 Do";
    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 8 8] \
          /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
    ];
    let stream = |dict: String, data: &[u8]| {
        let mut out = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
        out.extend_from_slice(data);
        out.extend_from_slice(b"\nendstream");
        out
    };
    objects.push(stream(String::new(), content));
    let smask_entry = if smask { " /SMask 6 0 R" } else { "" };
    objects.push(stream(
        format!(
            "/Type /XObject /Subtype /Image /Width 8 /Height 8 /BitsPerComponent 8 \
             /ColorSpace {space}{smask_entry} {entries}"
        ),
        data,
    ));
    objects.push(stream(
        format!(
            "/Type /XObject /Subtype /Image /Width 8 /Height 8 /BitsPerComponent 8 \
             /ColorSpace /DeviceGray /Matte {matte}"
        ),
        &mask(),
    ));
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    let size = objects.len() + 1;
    let mut tail = String::new();
    let _ = writeln!(tail, "xref\n0 {size}");
    tail.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(tail, "{offset:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(tail.as_bytes());
    out
}

/// The page's one image as the interpreter placed it: its RGBA bytes on its own grid.
fn drawn(bytes: Vec<u8>) -> Vec<u8> {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert_eq!(
        format!("{:?}", interpretation.unsupported),
        "[]",
        "the fixture is meant to draw"
    );
    let source = interpretation
        .display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            pdf_render::Command::Image { image, .. } => Some(image.clone()),
            _ => None,
        })
        .expect("the page draws its image");
    source.at(pdf_render::Transform::IDENTITY).data.to_vec()
}

/// The same pre-blended samples, once behind `DCTDecode` and once raw, give the same bytes.
fn routes_agree(space: &str, components: &[u8], matte: &str) {
    let jpeg = flat_jpeg(components);
    // The samples the codec delivers, read with no mask so nothing is undone yet.
    let plain = drawn(page(space, "/Filter /DCTDecode", &jpeg, false, matte));
    let width = components.len();
    let raw: Vec<u8> = plain
        .chunks_exact(4)
        .flat_map(|pixel| pixel[..width.min(3)].to_vec())
        .collect();
    assert!(
        width != 4,
        "a four-component frame's samples are not the raster's channels"
    );
    let through_the_codec = drawn(page(space, "/Filter /DCTDecode", &jpeg, true, matte));
    let written_raw = drawn(page(space, "", &raw, true, matte));
    assert_eq!(
        through_the_codec, written_raw,
        "a matte undone through the DCTDecode route and the raw route must be the same bytes"
    );
}

#[test]
fn a_matte_undone_through_the_dct_route_is_the_raw_route_s_bytes() {
    routes_agree("/DeviceRGB", &[100, 50, 200], "[0.3 0.5 0.1]");
}

#[test]
fn a_black_matte_undone_through_either_route_is_the_same_bytes() {
    routes_agree("/DeviceRGB", &[37, 128, 250], "[0 0 0]");
}

#[test]
fn a_grey_matte_undone_through_either_route_is_the_same_bytes() {
    routes_agree("/DeviceGray", &[90], "[0.77]");
}
