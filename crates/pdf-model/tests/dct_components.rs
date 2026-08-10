//! How many components a `DCTDecode` codestream has, and what says so.
//!
//! ISO 32000-2 §7.4.8 puts the number in one place:
//!
//! > The values of these parameters, which include the dimensions of the image and the number of
//! > components per sample, are entirely under the control of the encoder and shall be stored in
//! > the encoded data.
//!
//! Table 13's `/ColorTransform` is a *different* fact, and the clause states it in terms of the
//! first — "If the image has three colour components, RGB values shall be transformed to YCbCr
//! before encoding and from YCbCr to RGB after decoding" — so the marker that carries it says
//! nothing about how many components there are. Adobe's APP14 spells transform 0 as "no
//! transformation", and `zune-jpeg` maps that marker to `CMYK` provisionally and corrects it to
//! `RGB` only once it has read the frame. A reader that took the provisional value for the
//! component count asked a three-channel codestream for four channels, which is not a conversion
//! the decoder has — so every such image was lost whole.
//!
//! **The witnesses are real and are not in this repository**: 21 images over four documents of a
//! 4000-document `SafeDocs` sample, every one of them a three-component frame carrying an Adobe
//! APP14 marker with transform 0. They are named by archive and digest in ADR 0266 rather than
//! committed, and the fixtures here are *generated*, which is what a test owes when its witness
//! is somebody else's crawled web page.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and every index below is into \
              a buffer this file built at a size it also states"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;

/// A baseline 8×8 JPEG of three components, every sample mid-grey.
///
/// Written out rather than encoded, because the smallest thing that exercises the defect is a
/// frame header and one MCU: with an identity quantisation table, a DC difference of zero and an
/// immediate end-of-block, every coefficient is zero and JPEG's level shift makes each of the
/// three channels 128. Nothing here depends on the sample values — what is under test is which
/// *count* the decoder is asked for — but a known constant is what lets the test say the samples
/// arrived rather than merely that nothing failed.
///
/// `transform` is the Adobe APP14 colour-transform code, or `None` for a codestream carrying no
/// APP14 marker at all.
fn three_component_jpeg(transform: Option<u8>) -> Vec<u8> {
    let mut out = vec![0xFF, 0xD8];

    if let Some(code) = transform {
        // APP14: length, "Adobe", version, two flag words, and the transform code.
        out.extend_from_slice(&[0xFF, 0xEE, 0x00, 0x0E]);
        out.extend_from_slice(b"Adobe");
        out.extend_from_slice(&[0x00, 0x64, 0x00, 0x00, 0x00, 0x00, code]);
    }

    // One 8-bit quantisation table, all ones, so that a coefficient survives unchanged.
    out.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x43, 0x00]);
    out.extend_from_slice(&[1u8; 64]);

    // SOF0: 8 bits, 8×8, three components each at 1×1 sampling and quantisation table 0.
    out.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x08, 0x00, 0x08, 0x03]);
    for id in 1u8..=3 {
        out.extend_from_slice(&[id, 0x11, 0x00]);
    }

    // Two Huffman tables, DC 0 and AC 0, each holding two codes of two bits: symbol 0 is `00`
    // and symbol 1 is `01`. Symbol 0 is DC category zero and the AC end-of-block, which is all
    // one all-zero block needs.
    for class in [0x00u8, 0x10] {
        out.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x15, class]);
        out.extend_from_slice(&[0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend_from_slice(&[0x00, 0x01]);
    }

    // SOS over all three components, both tables 0, spectral selection 0..=63.
    out.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x0C, 0x03]);
    for id in 1u8..=3 {
        out.extend_from_slice(&[id, 0x00]);
    }
    out.extend_from_slice(&[0x00, 0x3F, 0x00]);

    // Three blocks of `DC category 0` then `end of block`, which is `00` six times: twelve zero
    // bits, padded to a byte boundary with ones as ISO/IEC 10918-1 requires.
    out.extend_from_slice(&[0x00, 0x0F]);

    out.extend_from_slice(&[0xFF, 0xD9]);
    out
}

/// A one-page PDF whose page is one image `XObject`, drawn over the whole page.
///
/// Built as bytes rather than through the string helper the other fixtures use, because a
/// codestream is not text and a `String` cannot hold one.
fn pdf_with_image(codestream: &[u8], colour_space: &str) -> Vec<u8> {
    let content = b"q 8 0 0 8 0 0 cm /Im0 Do Q";
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 8 8] \
          /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content);
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);
    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 8 /BitsPerComponent 8 \
         /ColorSpace {colour_space} /Filter /DCTDecode /Length {} >>\nstream\n",
        codestream.len()
    )
    .into_bytes();
    image.extend_from_slice(codestream);
    image.extend_from_slice(b"\nendstream");
    objects.push(image);

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

/// The samples the page's one image carries, or the reports that stopped it being drawn.
fn first_sample(bytes: Vec<u8>) -> Result<(u8, u8, u8), String> {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    if !interpretation.is_complete() {
        return Err(format!("{:?}", interpretation.unsupported));
    }
    let image = interpretation
        .display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            pdf_render::Command::Image { image, .. } => Some(image.clone()),
            _ => None,
        })
        .expect("the page draws one image");
    let placed = image.at(pdf_render::Transform::IDENTITY);
    Ok((placed.data[0], placed.data[1], placed.data[2]))
}

/// Adobe's transform 0 is "No transformation" and says nothing about the component count.
///
/// This is the defect a 4000-document sample found in the four-hundred-and-thirtieth session:
/// the marker was read as the component count, so a three-channel frame was asked for four
/// channels and the whole image was reported `malformed` instead of drawn.
#[test]
fn a_three_component_frame_marked_transform_zero_is_three_components() {
    let sample = first_sample(pdf_with_image(&three_component_jpeg(Some(0)), "/DeviceRGB"))
        .expect("a three-component codestream with an Adobe APP14 marker draws");
    assert_eq!(
        sample,
        (128, 128, 128),
        "every coefficient is zero, so JPEG's level shift makes each channel 128"
    );
}

/// Transform 2 on a *three*-component frame is the same mistake one step further on.
///
/// §7.4.8's Table 13 describes transform 2 for four components — YCCK — and a decoder that
/// believed the marker over the frame would run this tree's four-channel `YCCK → CMYK`
/// conversion across three-channel pixels, which walks past every pixel boundary rather than
/// refusing. Files spell this: `zune-jpeg` reads such a frame as `YCbCr`, and so must we.
#[test]
fn a_three_component_frame_marked_transform_two_is_not_converted_as_ycck() {
    let sample = first_sample(pdf_with_image(&three_component_jpeg(Some(2)), "/DeviceRGB"))
        .expect("a three-component codestream marked transform 2 draws");
    assert_eq!(
        sample,
        (128, 128, 128),
        "the four-channel conversion must not have run over three channels"
    );
}

/// The control: the same codestream with no APP14 marker at all, which never had the defect.
///
/// Without it the two cases above would only show that *something* changed, rather than that
/// the marker was what decided.
#[test]
fn the_same_frame_without_an_app14_marker_decodes_identically() {
    let sample = first_sample(pdf_with_image(&three_component_jpeg(None), "/DeviceRGB"))
        .expect("a three-component codestream with no Adobe marker draws");
    assert_eq!(sample, (128, 128, 128));
}
