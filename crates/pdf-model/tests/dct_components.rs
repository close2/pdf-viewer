//! What a `DCTDecode` codestream states about itself, and who is believed when the dictionary
//! disagrees.
//!
//! ISO 32000-2 §7.4.8 puts both facts in one place:
//!
//! > The values of these parameters, which include the dimensions of the image and the number of
//! > components per sample, are entirely under the control of the encoder and shall be stored in
//! > the encoded data.
//!
//! The component count is the first half of that sentence's subject and the dimensions are the
//! second; the tests below are in that order.
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
//!
//! **The third fact is what the samples *mean*, and it is the dictionary's rather than the
//! codestream's** — §7.4.9's precedence for JPEG 2000, and for a JPEG the plain reading that
//! nothing in ISO/IEC 10918 states a PDF colour space. The last test is that one: §8.6.6.3 makes
//! an `Indexed` sample "an index into the colour table", and this route scaled it into 0 to 1
//! first, which sent every index of a 256-entry table onto its two darkest entries.

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
    jpeg(3, transform)
}

/// The same frame with one component, whose single sample is likewise 128.
///
/// A greyscale JPEG is what a scanner writes, and it is the frame an `Indexed` colour space is
/// stated over: one eight-bit component per sample *is* an index into a 256-entry table.
fn one_component_jpeg() -> Vec<u8> {
    jpeg(1, None)
}

/// An 8×8 baseline JPEG of `components` components, every coefficient zero.
fn jpeg(components: u8, transform: Option<u8>) -> Vec<u8> {
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

    // SOF0: 8 bits, 8×8, each component at 1×1 sampling and quantisation table 0. Its length
    // is the eight fixed bytes plus three per component.
    let frame_header = 8 + 3 * u16::from(components);
    out.extend_from_slice(&[0xFF, 0xC0]);
    out.extend_from_slice(&frame_header.to_be_bytes());
    out.extend_from_slice(&[0x08, 0x00, 0x08, 0x00, 0x08, components]);
    for id in 1..=components {
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

    // SOS over every component, both tables 0, spectral selection 0..=63. Six fixed bytes plus
    // two per component.
    let scan_header = 6 + 2 * u16::from(components);
    out.extend_from_slice(&[0xFF, 0xDA]);
    out.extend_from_slice(&scan_header.to_be_bytes());
    out.push(components);
    for id in 1..=components {
        out.extend_from_slice(&[id, 0x00]);
    }
    out.extend_from_slice(&[0x00, 0x3F, 0x00]);

    // One block per component of `DC category 0` then `end of block`, which is `00` twice: four
    // zero bits apiece, padded to a byte boundary with ones as ISO/IEC 10918-1 requires.
    let bits = 4 * usize::from(components);
    out.resize(out.len() + bits / 8, 0x00);
    if bits % 8 != 0 {
        out.push(0x0F);
    }

    out.extend_from_slice(&[0xFF, 0xD9]);
    out
}

/// A one-page PDF whose page is one image `XObject`, drawn over the whole page.
///
/// `stated` is what the image dictionary's `/Width` and `/Height` say, which is a separate
/// statement from the codestream's own and need not agree with it.
///
/// Built as bytes rather than through the string helper the other fixtures use, because a
/// codestream is not text and a `String` cannot hold one.
fn pdf_with_image(codestream: &[u8], colour_space: &str, stated: (u32, u32)) -> Vec<u8> {
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
    let (stated_width, stated_height) = stated;
    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width {stated_width} /Height {stated_height} \
         /BitsPerComponent 8 /ColorSpace {colour_space} /Filter /DCTDecode /Length {} >>\n\
         stream\n",
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

/// The page's one image if it was drawn, and everything the interpreter reported about it.
///
/// The two are separate answers because they are separate facts: an image can be drawn *and*
/// reported, which is what a codestream contradicting its dictionary is.
fn interpret_one(bytes: Vec<u8>) -> (Option<pdf_render::ImageSource>, String) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let drawn = interpretation
        .display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            pdf_render::Command::Image { image, .. } => Some(image.clone()),
            _ => None,
        });
    (drawn, format!("{:?}", interpretation.unsupported))
}

/// The page's one image, or the reports that stopped it being drawn.
fn first_image(bytes: Vec<u8>) -> Result<pdf_render::ImageSource, String> {
    match interpret_one(bytes) {
        (Some(image), reported) if reported == "[]" => Ok(image),
        (_, reported) => Err(reported),
    }
}

/// The samples the page's one image carries, or the reports that stopped it being drawn.
fn first_sample(bytes: Vec<u8>) -> Result<(u8, u8, u8), String> {
    let source = first_image(bytes)?;
    let placed = source.at(pdf_render::Transform::IDENTITY);
    Ok((placed.data[0], placed.data[1], placed.data[2]))
}

/// Adobe's transform 0 is "No transformation" and says nothing about the component count.
///
/// This is the defect a 4000-document sample found in the four-hundred-and-thirtieth session:
/// the marker was read as the component count, so a three-channel frame was asked for four
/// channels and the whole image was reported `malformed` instead of drawn.
#[test]
fn a_three_component_frame_marked_transform_zero_is_three_components() {
    let sample = first_sample(pdf_with_image(
        &three_component_jpeg(Some(0)),
        "/DeviceRGB",
        (8, 8),
    ))
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
    let sample = first_sample(pdf_with_image(
        &three_component_jpeg(Some(2)),
        "/DeviceRGB",
        (8, 8),
    ))
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
    let sample = first_sample(pdf_with_image(
        &three_component_jpeg(None),
        "/DeviceRGB",
        (8, 8),
    ))
    .expect("a three-component codestream with no Adobe marker draws");
    assert_eq!(sample, (128, 128, 128));
}

/// The grid a `DCTDecode` image is built on is the codestream's, not the dictionary's.
///
/// The same clause the module quotes says the dimensions are "entirely under the control of the
/// encoder and shall be stored in the encoded data", and a decoder has nowhere else to read them
/// from. A dictionary that states something different has contradicted the data rather than
/// described it, and the samples still land where §8.9.5.1 puts every image whatever its
/// resolution: "the unit square of user space, bounded by user coordinates (0, 0) and (1, 1),
/// corresponds to the boundary of the image in image space".
///
/// This tree refused the image outright until the five-hundred-and-fifth session, where
/// `pdfCabinetOfHorrors/veraPDFHiResChangedHeight.pdf` — a valid file with one digit of its
/// `/Height` altered on purpose — lost a whole photograph over one row in 1227.
///
/// **The contradiction is still reported**, and both halves are asserted here because either one
/// alone would be a different decision: drawn and silent is what a page showing one red sample
/// where 200×100 were described would be, and reported without drawing is what this session
/// found.
#[test]
fn a_dictionary_that_contradicts_the_frames_dimensions_does_not_cost_the_image() {
    let (drawn, reported) = interpret_one(pdf_with_image(
        &three_component_jpeg(None),
        "/DeviceRGB",
        (8, 9),
    ));
    let image = drawn.expect("a dictionary stating one row more than the frame holds still draws");
    let pdf_render::ImageSource::Decoded(decoded) = &image else {
        panic!("a DCTDecode image is decoded rather than deferred to the device scale");
    };
    assert_eq!(
        (decoded.width, decoded.height),
        (8, 8),
        "the frame states 8x8 and the samples are on that grid, whatever /Height says"
    );
    let placed = image.at(pdf_render::Transform::IDENTITY);
    assert_eq!(
        (placed.data[0], placed.data[1], placed.data[2]),
        (128, 128, 128),
        "and the samples are the frame's, unshifted by the row that does not exist"
    );
    assert!(
        reported.contains("the JPEG frame is 8x8 where the dictionary says 8x9"),
        "the page says what it drew instead of what the dictionary described, and it said \
         {reported}"
    );
}

/// A frame this tree will not build a raster for, so that the case above is a *reading* rather
/// than a decoder that stopped checking.
///
/// §7.4.9 states for `JPXDecode` the constraint §7.4.8 does not state for `DCTDecode` — "Width
/// and Height shall match the corresponding width and height values in the JPEG 2000 data" — and
/// that mismatch is still a refusal, in [`pdf_model::image`]'s JPEG 2000 arm. What is checked
/// here is the other bound the codestream's grid now carries alone: a frame stating no samples
/// is not an image, and reaching the sample loops with it would answer with an empty raster
/// instead of saying so. `zune-jpeg` answers this one first today — "Image width or height is
/// set to zero, cannot continue" — which is why the assertion is on the report reaching the
/// page rather than on which layer wrote it.
#[test]
fn a_frame_stating_no_samples_is_refused_rather_than_drawn_empty() {
    let mut codestream = three_component_jpeg(None);
    // SOF0's height, big-endian, at the two bytes after the marker, its length and the sample
    // precision. `three_component_jpeg` writes `FF C0 00 11 08 <height> <width> …`.
    let sof = codestream
        .windows(2)
        .position(|pair| pair == [0xFF, 0xC0])
        .expect("the fixture writes one SOF0 marker");
    codestream[sof + 5] = 0;
    codestream[sof + 6] = 0;
    let reported = first_image(pdf_with_image(&codestream, "/DeviceRGB", (8, 8)))
        .expect_err("a frame of no rows is refused");
    assert!(
        reported.contains("malformed image"),
        "the refusal reaches the page rather than being silent, and it was {reported}"
    );
}

/// An `Indexed` colour space over a `DCTDecode` frame reads each sample as an index.
///
/// §8.6.6.3, whose sentence is the whole of the rule:
///
/// > A PDF reader shall treat each sample value as an index into the colour table and shall use
/// > the colour value it finds there.
///
/// The other four image routes obey it — `unpack` builds its palette from `Decode`, whose default
/// range for an `Indexed` space is the index range itself, and the JPEG 2000 route scales by one
/// — and this one divided every sample by 255 before the lookup. That sends the whole of a
/// 256-entry table onto entries 0 and 1, so a scan whose samples sit near 250 draws in the two
/// *darkest* colours the palette states. The witness is a crawled Hewlett-Packard scan whose
/// palette is a grey ramp: this tree drew it as a solid black page at ink 253.8 of 255 where
/// `poppler`, `mupdf` and `ghostscript` agree on 8.9 to 9.2 (session 613).
///
/// The fixture's sample is 128, so the defect and the fix name different entries of the table:
/// 128 ÷ 255 rounds to index 1, which is red here, and the index itself is 128, which is not.
#[test]
fn an_indexed_space_over_a_jpeg_reads_the_sample_as_an_index() {
    // hival 128 and 129 entries of three components, all black but the two the test names.
    let mut table = vec![0u8; 129 * 3];
    table[3..6].copy_from_slice(&[255, 0, 0]);
    table[128 * 3..128 * 3 + 3].copy_from_slice(&[0, 128, 255]);
    let mut hex = String::with_capacity(table.len() * 2);
    for byte in &table {
        let _ = write!(hex, "{byte:02X}");
    }
    let space = format!("[/Indexed /DeviceRGB 128 <{hex}>]");

    let sample = first_sample(pdf_with_image(&one_component_jpeg(), &space, (8, 8)))
        .expect("a greyscale JPEG under an Indexed space draws");
    assert_eq!(
        sample,
        (0, 128, 255),
        "sample 128 selects table entry 128; entry 1 is what dividing by 255 selects"
    );
}
