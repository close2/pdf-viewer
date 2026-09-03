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
    jpeg_of(8, 8, components, transform)
}

/// The same codestream at a chosen frame size, which must be a multiple of eight in both axes.
///
/// Every component is sampled 1×1, so an MCU is one 8×8 block per component and the number of
/// them is fixed by the frame alone. Kept apart from [`jpeg`] because only one test states a
/// frame larger than a single MCU and the arithmetic is worth having in one place.
fn jpeg_of(width: u16, height: u16, components: u8, transform: Option<u8>) -> Vec<u8> {
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
    out.push(0x08);
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&width.to_be_bytes());
    out.push(components);
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

    // One block per component per MCU of `DC category 0` then `end of block`, which is `00`
    // twice: four zero bits apiece, padded to a byte boundary with ones as ISO/IEC 10918-1
    // requires.
    let blocks = (usize::from(width) / 8) * (usize::from(height) / 8);
    let bits = 4 * usize::from(components) * blocks;
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

/// A `Lab` space over a JPEG maps the sample onto the space's own component range.
///
/// The sibling of the `Indexed` test above, one arm along and from the same sentence. §8.9.5.2
/// maps an integer sample onto a component value, and Table 88 sets the ends of that map per
/// *space* where the dictionary states no `/Decode`: a `Lab` space's default is
/// `[0 100 amin amax bmin bmax]`, because §8.6.5.4 makes its first component a **percentage**
/// and its other two the space's own `/Range`. Dividing by 255 instead hands `Lab` a lightness
/// of at most 1 where 100 is white, so every colour in the image collapses onto black.
///
/// The frame here is every coefficient zero, which JPEG's level shift makes 128 in each of three
/// channels. Through Table 88 that is L\* 50.196 with both chromatic axes within four tenths of
/// neutral — mid grey by construction, since CIE L\* 50 *is* the middle of the lightness scale —
/// and through a division by 255 it is L\* 0.502, which is black. The band below separates those
/// two answers by ninety levels and nothing finer, so it pins the map rather than this tree's
/// white-point adaptation or its sRGB encoding.
///
/// **The witness is a crawled schoolbook page**: a 543×372 `/DCTDecode` photograph under
/// `[/Lab << /WhitePoint [.964203 1 .824905] /Range [-128 127 -128 127] >>]`, drawn as a solid
/// black rectangle with nothing reported, at **+32.097** of 255 where `pdftoppm`, `mutool` and
/// `gs` agree within 1.8 (session 631, ADR 0464).
#[test]
fn a_lab_space_over_a_jpeg_maps_the_sample_onto_the_spaces_own_range() {
    let space = "[/Lab << /WhitePoint [0.9505 1.0 1.089] >>]";
    let sample = first_sample(pdf_with_image(&three_component_jpeg(None), space, (8, 8)))
        .expect("a three-component JPEG under a Lab space draws");
    let (red, green, blue) = sample;
    assert!(
        (100..=140).contains(&red),
        "a sample of 128 is L* 50.196, which is mid grey; dividing by 255 gives L* 0.502, \
         which is black. Drew {sample:?}"
    );
    assert!(
        red.abs_diff(green) <= 3 && green.abs_diff(blue) <= 3,
        "both chromatic axes are within four tenths of neutral: {sample:?}"
    );
}

/// A frame taller than sixteen thousand rows is this crate's budget to refuse, not a decoder's.
///
/// §7.4.8 says where a frame's dimensions come from and puts no ceiling on them:
///
/// > The values of these parameters, which include the dimensions of the image and the number of
/// > components per sample, are entirely under the control of the encoder and shall be stored in
/// > the encoded data.
///
/// ISO/IEC 10918-1 allows sixteen bits for each, so a codestream may state up to 65535 rows, and
/// what bounds this tree is `pdf_model::image`'s own `MAX_SAMPLES` — an explicit budget with an
/// argument written beside it. `zune-jpeg`'s `DecoderOptions` carries an unrelated default of
/// 16384 in each axis, and it was reached first: a crawled full-page scan 28341 rows tall came
/// back as `Image height 28341 greater than height limit 16384` and the page was blank where
/// `pdftoppm`, `mutool` and `gs` agree on 84.2 of 255 (session 619).
///
/// 20000 rows is above that default and far below the budget, so the two answers differ and this
/// test can only pass for the right reason. Eight columns wide keeps the fixture at 2500 blocks.
#[test]
fn a_frame_past_the_decoders_default_dimension_limit_is_still_this_crates_budget() {
    let image = first_image(pdf_with_image(
        &jpeg_of(8, 20000, 1, None),
        "/DeviceGray",
        (8, 20000),
    ))
    .expect("a frame of 20000 rows is inside MAX_SAMPLES and decodes");
    let pdf_render::ImageSource::Decoded(decoded) = &image else {
        panic!("a DCTDecode image is decoded rather than deferred to the device scale");
    };
    assert_eq!(
        (decoded.width, decoded.height),
        (8, 20000),
        "the samples are on the grid §7.4.8 puts in the encoded data"
    );
}

/// The same 8×8 frame with its header's number of lines replaced by `header` and a `DNL`
/// marker segment stating `lines` at the end of its one scan.
///
/// ISO/IEC 10918-1 section B.2.5: the `DNL` segment defines or redefines the frame header's `Y` at the
/// end of the first scan, and `Y = 0` in the header means the count is the `DNL`'s to give.
fn one_component_jpeg_with_dnl(header: u16, lines: u16) -> Vec<u8> {
    let mut out = one_component_jpeg();
    // The frame header's `Y` follows `FF C0`, its two-byte length and the one-byte precision.
    let sof = out
        .windows(2)
        .position(|pair| pair == [0xFF, 0xC0])
        .expect("the fixture has a baseline frame header");
    out[sof + 5..sof + 7].copy_from_slice(&header.to_be_bytes());
    // `DNL` before `EOI`: marker, length 4, `NL`.
    let eoi = out.len() - 2;
    assert_eq!(&out[eoi..], &[0xFF, 0xD9]);
    let mut dnl = vec![0xFF, 0xDC, 0x00, 0x04];
    dnl.extend_from_slice(&lines.to_be_bytes());
    out.splice(eoi..eoi, dnl);
    out
}

/// A `DNL` marker defines the number of lines where the frame header left it open, and
/// redefines it where the header wrote a placeholder.
///
/// §7.4.8 puts the dimensions in the encoded data, and ISO/IEC 10918-1 lets the encoded data
/// state its number of lines in two places: the frame header, or a `DNL` segment at the end of
/// the first scan, which section B.2.5 of that standard says defines *or redefines* the header's `Y`.
/// A scanner that does not know the page length when it writes the header writes `0` or
/// `65535` there and the true count after the data. `zune-jpeg` reads the header alone, so
/// `poppler-61994-0.pdf`'s 2480 × 3486 letter was drawn as the top five per cent of a
/// 2480 × 65535 image over grey, where both reference renderers drew the letter; against the
/// header alone, the first case here draws an 8 × 65535 grid. ADR 0799.
#[test]
fn a_dnl_marker_defines_or_redefines_the_frames_number_of_lines() {
    for (header, why) in [
        (65535, "a placeholder the DNL redefines"),
        (0, "left open for the DNL to define"),
        (8, "already what the DNL states"),
    ] {
        let (drawn, reported) = interpret_one(pdf_with_image(
            &one_component_jpeg_with_dnl(header, 8),
            "/DeviceGray",
            (8, 8),
        ));
        let image = drawn.unwrap_or_else(|| panic!("the frame draws with Y {why}: {reported}"));
        let pdf_render::ImageSource::Decoded(decoded) = &image else {
            panic!("a DCTDecode image is decoded rather than deferred to the device scale");
        };
        assert_eq!(
            (decoded.width, decoded.height),
            (8, 8),
            "the DNL's eight lines are the frame's, with the header's Y {why}"
        );
        let placed = image.at(pdf_render::Transform::IDENTITY);
        assert_eq!(placed.data[0], 128, "and the samples are the scan's");
        assert_eq!(
            reported, "[]",
            "a frame whose lines agree with the dictionary reports nothing, with Y {why}"
        );
    }
    // A `DNL` at odds with the dictionary is reported with the number the encoded data states,
    // not the header's placeholder.
    let (drawn, reported) = interpret_one(pdf_with_image(
        &one_component_jpeg_with_dnl(65535, 8),
        "/DeviceGray",
        (8, 9),
    ));
    assert!(drawn.is_some());
    assert!(
        reported.contains("the JPEG frame is 8x8 where the dictionary says 8x9"),
        "the report names the DNL's count: {reported}"
    );
}
