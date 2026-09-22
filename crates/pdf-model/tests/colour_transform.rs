//! Table 13's `/ColorTransform`, which is the one `DCTDecode` parameter ISO 32000-2 §7.4.8 puts
//! in the filter's parameter dictionary rather than in the encoded data.
//!
//! > However, in one instance, the parameter need not be present in the encoded data but shall be
//! > specified in the filter parameter dictionary; see "Table 13 -Optional parameter for the
//! > DCTDecode filter".
//!
//! The entry ranks three cases against Adobe's `APP14` marker segment, and the tests below are in
//! that order:
//!
//! > If the encoding algorithm has inserted the Adobe-defined marker code in the encoded data
//! > indicating the ColorTransform value, then the colours shall be transformed, or not, after the
//! > DCT decoding has been performed according to the value provided in the encoded data and the
//! > value of this dictionary entry shall be ignored. If the Adobe-defined marker code in the
//! > encoded data indicating the ColorTransform value is not present then the value specified in
//! > this dictionary entry will be used. If the Adobe-defined marker code (APP14) in the encoded
//! > data indicating the ColorTransform value is not present and this dictionary entry is not
//! > present in the filter dictionary then the default value of ColorTransform shall be 1 if the
//! > image has three components and 0 otherwise.
//!
//! **What the entry asks for is stated in terms of the frame's component count**, which is why the
//! fixtures come in threes and fours:
//!
//! > 1 If the image has three colour components, RGB values shall be transformed to YCbCr before
//! > encoding and from YCbCr to RGB after decoding. If the image has four components, CMYK values
//! > shall be transformed to YCbCrK before encoding and from YCbCrK to CMYK after decoding. This
//! > option shall be ignored if the image has one or two colour components.
//!
//! **Two of the values name operations the marker also names**, and that is what most of these
//! tests measure: the entry's 1 over four components and the marker's transform 2 are the same
//! sentence of Table 13, and the entry's 0 and the marker's 0 are its "No transformation". So a
//! fixture pair that differs only in *where* the value is written must draw the same pixels, and
//! that equivalence is derivable from the clause alone — no arithmetic of ISO/IEC 10918-1, which
//! this project does not hold, is assumed by it.
//!
//! The absolute pixel values are asserted where the clause's own transform decides them: a
//! three-component frame in `/DeviceRGB`, where §8.9.5.2's default `/Decode` is the identity and
//! the samples the filter delivers are the pixels.
//!
//! **Where the entry is written is half of what the clause says**, and two of these tests are
//! that half. §7.4.1 puts a filter's parameters in one place — "These optional parameters shall
//! be specified by the DecodeParms entry in the stream's dictionary" — so the same name written
//! as a direct key of the image dictionary is not this parameter and the clause's default
//! governs instead.
//!
//! **And three are about the frame's component identifiers**, which `zune-jpeg` reads as saying
//! the samples are already transformed. A marker outranks them, an entry outranks them, and where
//! neither is present they decide — which is this tree's one departure from Table 13, held here as
//! a fixture rather than as a sentence. ADRs 1177, 1183.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and every index below is into \
              a buffer this file built at a size it also states"
)]
#![expect(
    clippy::doc_markdown,
    reason = "verbatim quotations: §7.4.1 and Table 13 spell DecodeParms and ColorTransform \
              without backticks"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;

/// The sample value every component of a one-component-per-block frame is built around.
///
/// The three below are a `YCbCr` triple chosen so that the transform's answer clamps in no
/// channel, which is what makes the transformed and untransformed readings tell each other apart
/// by more than rounding.
const CHANNELS: [u8; 4] = [128, 64, 192, 100];

/// An 8×8 baseline JPEG whose components carry the stated sample values and nothing else.
///
/// Written out rather than encoded, because the smallest thing that exercises Table 13 is a frame
/// header, one block per component, and — in half the fixtures — an Adobe `APP14` segment. Each
/// block holds a DC coefficient and an immediate end-of-block, so the inverse DCT makes every
/// sample of that component the same value: a DC-only block is flat at `DC / 8`, and ISO/IEC
/// 10918-1's level shift puts the zero of that scale at 128.
///
/// `transform` is the Adobe `APP14` colour-transform code, or `None` for a codestream carrying no
/// such segment at all — which is the only case in which Table 13's entry is read.
fn jpeg(samples: &[u8], transform: Option<u8>) -> Vec<u8> {
    jpeg_of(samples, transform, false)
}

/// The same frame with its components identified by the ASCII letters `R`, `G` and `B`.
///
/// No clause of ISO 32000-2 and none of ISO/IEC 10918-1 gives a component identifier any meaning;
/// the convention is `libjpeg`'s, and `zune-jpeg` reads it as saying the samples are already
/// transformed. The three tests that use this are about which of the two — the identifiers, or
/// Table 13 — decides, and the clause's answer differs by case.
fn rgb_identified_jpeg(samples: &[u8], transform: Option<u8>) -> Vec<u8> {
    jpeg_of(samples, transform, true)
}

/// The generator both of the above call, with the component identifiers chosen.
fn jpeg_of(samples: &[u8], transform: Option<u8>, spell_rgb: bool) -> Vec<u8> {
    let components = u8::try_from(samples.len()).expect("a frame of at most 255 components");
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

    // SOF0: 8 bits, 8×8, each component at 1×1 sampling and quantisation table 0. Its length is
    // the eight fixed bytes plus three per component.
    let frame_header = 8 + 3 * u16::from(components);
    out.extend_from_slice(&[0xFF, 0xC0]);
    out.extend_from_slice(&frame_header.to_be_bytes());
    out.extend_from_slice(&[0x08, 0x00, 0x08, 0x00, 0x08]);
    out.push(components);
    for (index, id) in (1..=components).enumerate() {
        let id = if spell_rgb { b"RGB"[index] } else { id };
        out.extend_from_slice(&[id, 0x11, 0x00]);
    }

    // The DC table is ISO/IEC 10918-1 Annex K's luminance one, whose twelve values are the twelve
    // magnitude categories a DC difference can fall in; the AC table holds one code, the
    // end-of-block, which is all an otherwise empty block needs.
    out.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x1F, 0x00]);
    out.extend_from_slice(&DC_BITS);
    out.extend_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    out.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x15, 0x10]);
    out.extend_from_slice(&[0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&[0x00, 0x01]);

    // SOS over every component, both tables 0, spectral selection 0..=63. Six fixed bytes plus
    // two per component.
    let scan_header = 6 + 2 * u16::from(components);
    out.extend_from_slice(&[0xFF, 0xDA]);
    out.extend_from_slice(&scan_header.to_be_bytes());
    out.push(components);
    for (index, id) in (1..=components).enumerate() {
        let id = if spell_rgb { b"RGB"[index] } else { id };
        out.extend_from_slice(&[id, 0x00]);
    }
    out.extend_from_slice(&[0x00, 0x3F, 0x00]);

    let mut bits = Bits::default();
    for sample in samples {
        // One block per component per MCU, and one MCU in an 8×8 frame, so each component's DC
        // difference is its whole DC coefficient: nothing precedes it to predict from.
        bits.dc((i32::from(*sample) - 128) * 8);
        // The end-of-block, whose symbol is 0 in the AC table written above.
        bits.push(0b00, 2);
    }
    bits.finish(&mut out);

    out.extend_from_slice(&[0xFF, 0xD9]);
    out
}

/// The `BITS` list of ISO/IEC 10918-1 Annex K's luminance DC Huffman table: one two-bit code,
/// five three-bit codes, then one of each length up to nine.
const DC_BITS: [u8; 16] = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];

/// An entropy-coded segment under construction, with ISO/IEC 10918-1's byte stuffing.
#[derive(Default)]
struct Bits {
    /// The bits written so far, most significant first, one per entry.
    written: Vec<u8>,
}

impl Bits {
    /// Appends the `length` low bits of `code`, most significant first.
    fn push(&mut self, code: u32, length: u32) {
        for index in (0..length).rev() {
            self.written
                .push(u8::try_from((code >> index) & 1).expect("one bit"));
        }
    }

    /// Appends one DC difference: its magnitude category's Huffman code, then the difference
    /// itself in that many bits, which is how ISO/IEC 10918-1 section F.1.2.1 codes one.
    ///
    /// A negative difference is written as its one's complement in the same width, which is what
    /// makes the category enough to read it back.
    fn dc(&mut self, difference: i32) {
        let category = u32::BITS - difference.unsigned_abs().leading_zeros();
        let (code, length) = canonical(DC_BITS, category)
            .expect("the table above holds every category written here");
        self.push(code, length);
        if category > 0 {
            let value = if difference > 0 {
                difference
            } else {
                difference + (1 << category) - 1
            };
            self.push(
                u32::try_from(value).expect("a non-negative width-limited value"),
                category,
            );
        }
    }

    /// Flushes the segment into `out`, padded to a byte boundary with one bits and with every
    /// `FF` byte followed by the stuffed zero the standard requires.
    fn finish(&mut self, out: &mut Vec<u8>) {
        while !self.written.len().is_multiple_of(8) {
            self.written.push(1);
        }
        for byte in self.written.chunks_exact(8) {
            let packed = byte.iter().fold(0u8, |acc, bit| (acc << 1) | bit);
            out.push(packed);
            if packed == 0xFF {
                out.push(0x00);
            }
        }
    }
}

/// The canonical Huffman code a symbol's index gets from a `BITS` list, as ISO/IEC 10918-1
/// section C.2 generates one: codes are assigned in increasing length, left to right.
fn canonical(bits: [u8; 16], symbol: u32) -> Option<(u32, u32)> {
    let mut code = 0u32;
    let mut index = 0u32;
    for (offset, count) in bits.iter().enumerate() {
        let length = u32::try_from(offset).expect("sixteen lengths") + 1;
        for _ in 0..*count {
            if index == symbol {
                return Some((code, length));
            }
            index += 1;
            code += 1;
        }
        code <<= 1;
    }
    None
}

/// A one-page PDF whose page is one image `XObject`, drawn over the whole page.
///
/// `entries` goes into the image dictionary beside the required ones, which is where every
/// spelling of `/ColorTransform` these tests compare is written.
fn pdf_with_image(codestream: &[u8], colour_space: &str, entries: &str) -> Vec<u8> {
    let content = b"q 8 0 0 8 0 0 cm /Im0 Do Q";
    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 8 8] \
          /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
    ];
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content);
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);
    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 8 \
         /BitsPerComponent 8 /ColorSpace {colour_space} {entries} /Filter /DCTDecode \
         /Length {} >>\nstream\n",
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

/// The top-left pixel of the page's one image, as red, green and blue.
fn first_pixel(bytes: Vec<u8>) -> (u8, u8, u8) {
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
    let placed = source.at(pdf_render::Transform::IDENTITY);
    (placed.data[0], placed.data[1], placed.data[2])
}

/// The three-component fixture with the given `APP14` code and the given dictionary entries.
fn three(transform: Option<u8>, entries: &str) -> (u8, u8, u8) {
    first_pixel(pdf_with_image(
        &jpeg(&CHANNELS[..3], transform),
        "/DeviceRGB",
        entries,
    ))
}

/// The four-component fixture, whose samples §8.9.5.1 leaves `/DeviceCMYK` to interpret.
fn four(transform: Option<u8>, entries: &str) -> (u8, u8, u8) {
    first_pixel(pdf_with_image(
        &jpeg(&CHANNELS, transform),
        "/DeviceCMYK",
        entries,
    ))
}

/// A `/DecodeParms` dictionary stating `/ColorTransform`.
fn parms(value: i32) -> String {
    format!("/DecodeParms << /ColorTransform {value} >>")
}

/// `/ColorTransform 0` is "No transformation", so the frame's components are the pixels.
///
/// The fixture's components are 128, 64 and 192, and in `/DeviceRGB` under Table 88's identity
/// `/Decode` that is exactly what the page must show.
#[test]
fn a_three_component_frame_whose_entry_says_zero_is_not_transformed() {
    assert_eq!(three(None, &parms(0)), (128, 64, 192));
}

/// `/ColorTransform 1` is "from YCbCr to RGB after decoding", whose answer is arithmetic.
///
/// ITU-T T.871 states the inverse §7.4.8 defers to Adobe Technical Note #5116 for, and on the
/// fixture's 128, 64, 192 it gives 128 + 1.402 × 64 = 217.7, 128 − 0.344136 × −64 − 0.714136 × 64
/// = 104.3, and 128 + 1.772 × −64 = 14.6. A decoder's own rounding of that is its own, so the
/// assertion allows one count in each channel and the *separation* from the untransformed
/// reading — 90 counts in green — is what discriminates.
#[test]
fn a_three_component_frame_whose_entry_says_one_is_transformed_to_rgb() {
    let (red, green, blue) = three(None, &parms(1));
    assert!(red.abs_diff(218) <= 1, "red was {red}");
    assert!(green.abs_diff(104) <= 1, "green was {green}");
    assert!(blue.abs_diff(15) <= 1, "blue was {blue}");
}

/// With no `APP14` and no entry, "the default value of ColorTransform shall be 1 if the image has
/// three components".
#[test]
fn a_three_component_frame_with_neither_takes_the_clauses_default_of_one() {
    assert_eq!(three(None, ""), three(None, &parms(1)));
}

/// The entry's 0 and the marker's 0 are Table 13's same "No transformation".
#[test]
fn a_three_component_entry_of_zero_matches_the_marker_spelling_of_it() {
    assert_eq!(three(None, &parms(0)), three(Some(0), ""));
}

/// The entry's 1 and the marker's 1 are Table 13's same `YCbCr` → `RGB`.
#[test]
fn a_three_component_entry_of_one_matches_the_marker_spelling_of_it() {
    assert_eq!(three(None, &parms(1)), three(Some(1), ""));
}

/// "the value of this dictionary entry shall be ignored" where the marker is present.
///
/// Both directions, because an implementation that read the entry *instead* of the marker would
/// pass a test of one of them.
#[test]
fn a_marked_three_component_frame_ignores_a_contradicting_entry() {
    assert_eq!(three(Some(0), &parms(1)), three(Some(0), ""));
    assert_eq!(three(Some(1), &parms(0)), three(Some(1), ""));
}

/// With no `APP14` and no entry the default "shall be […] 0 otherwise", so four components arrive
/// as they were stored.
///
/// `/DeviceCMYK` then interprets them, which is §8.9.5.1's division of labour and not this
/// clause's — so what this asserts is that the entry's 0 changes nothing, and the test below it
/// is where the two values part.
#[test]
fn a_four_component_frame_with_neither_takes_the_clauses_default_of_zero() {
    assert_eq!(four(None, ""), four(None, &parms(0)));
    assert_eq!(four(None, ""), four(Some(0), ""));
}

/// `/ColorTransform 1` over four components is "from YCbCrK to CMYK after decoding", which is the
/// operation Adobe's marker spells as transform 2.
///
/// So the entry's 1 must draw what the marker's 2 draws, and neither may draw what the
/// untransformed reading draws.
#[test]
fn a_four_component_entry_of_one_is_the_markers_transform_two() {
    let transformed = four(None, &parms(1));
    assert_eq!(transformed, four(Some(2), ""));
    assert_ne!(transformed, four(None, &parms(0)));
}

/// "This option shall be ignored if the image has one or two colour components."
///
/// A greyscale frame is what a scanner writes, and an entry of either value on one leaves it
/// exactly as it was.
#[test]
fn a_one_component_frame_ignores_the_entry() {
    let grey = first_pixel(pdf_with_image(
        &jpeg(&CHANNELS[..1], None),
        "/DeviceGray",
        "",
    ));
    for value in [0, 1] {
        assert_eq!(
            first_pixel(pdf_with_image(
                &jpeg(&CHANNELS[..1], None),
                "/DeviceGray",
                &parms(value),
            )),
            grey
        );
    }
}

/// `/DecodeParms` may be the array form, one entry per filter, and the parameter is still this
/// filter's.
///
/// §7.4's own words: the array "shall have one entry for each filter", and a chain of one has one
/// — which is the spelling the crawl's deciding population is written in as often as not.
#[test]
fn the_array_form_of_decode_parms_states_the_same_entry() {
    assert_eq!(
        first_pixel(pdf_with_image(
            &jpeg(&CHANNELS[..3], None),
            "/DeviceRGB",
            "/DecodeParms [<< /ColorTransform 0 >>]",
        )),
        three(None, &parms(0))
    );
}

/// The same name written as a direct key of the image dictionary is not Table 13's entry.
///
/// §7.4.1 puts a filter's parameters in one place — "These optional parameters shall be specified
/// by the DecodeParms entry in the stream's dictionary" — and §7.4.8 says the same of this one.
/// A producer who writes it elsewhere has written no parameter, so the clause's default governs:
/// 1 for three components, which is the transformed reading. The population is real and it is
/// larger than the one the entry decides (ADR 1177), so this is pinned rather than assumed.
#[test]
fn the_name_written_outside_the_parameter_dictionary_is_not_the_entry() {
    assert_eq!(three(None, "/ColorTransform 0"), three(None, ""));
}

/// The three-component fixture whose component identifiers spell `R`, `G`, `B`.
fn spelt(transform: Option<u8>, entries: &str) -> (u8, u8, u8) {
    first_pixel(pdf_with_image(
        &rgb_identified_jpeg(&CHANNELS[..3], transform),
        "/DeviceRGB",
        entries,
    ))
}

/// A marker present is the marker obeyed, whatever the component identifiers say.
///
/// "the colours shall be transformed, or not, after the DCT decoding has been performed according
/// to the value provided in the encoded data" — so a frame whose identifiers spell `R`, `G`, `B`
/// beside an `APP14` stating transform 1 is transformed, and beside one stating 0 is not. The
/// identifiers are `libjpeg`'s convention and no standard gives them a meaning; the marker is
/// what this clause reads. ADR 1183.
#[test]
fn the_marker_outranks_the_component_identifiers() {
    assert_eq!(spelt(Some(1), ""), three(Some(1), ""));
    assert_eq!(spelt(Some(0), ""), three(Some(0), ""));
}

/// And with no marker, the entry does: it is "the value specified in this dictionary entry".
#[test]
fn the_entry_outranks_the_component_identifiers() {
    assert_eq!(spelt(None, &parms(1)), three(None, &parms(1)));
    assert_eq!(spelt(None, &parms(0)), three(None, &parms(0)));
}

/// With neither, the identifiers decide — and that is this tree's one departure from Table 13.
///
/// The clause's default here is 1, and this tree does not apply it: a frame declaring `R`, `G`,
/// `B` and carrying neither the marker nor the entry is read as already transformed, which is
/// what `poppler`, `mupdf` and `hayro` also do and what `ghostscript` does not. The cost is one
/// corpus page, held by name in `oracle.rs`'s `AMBIGUOUS_JPEG_COMPONENT_IDS`, and ADR 1183 is the
/// argument. This test exists so that the departure is a fixture rather than a sentence, and so
/// that a later round changing it is told.
#[test]
fn with_neither_the_component_identifiers_decide_which_is_the_one_departure() {
    assert_eq!(spelt(None, ""), (128, 64, 192));
    assert_ne!(spelt(None, ""), three(None, ""));
}

/// Table 13 describes two values and no others, so an entry stating a third states no code.
///
/// The clause's default then governs, which is the same answer §7.4.1's misplacement gets: a
/// producer who has written something this parameter does not admit has not written this
/// parameter. ADR 1183.
#[test]
fn an_entry_outside_the_two_values_the_table_describes_leaves_the_default() {
    for value in [-1, 2, 7] {
        assert_eq!(three(None, &parms(value)), three(None, ""));
    }
}

/// The same, over four components, where the two values name the transform Table 13 calls 1.
///
/// `APP14` transform 2 beside `/ColorTransform 0` is transformed and `APP14` transform 0 beside
/// `/ColorTransform 1` is not, because in both the marker is present and "the value of this
/// dictionary entry shall be ignored".
#[test]
fn a_marked_four_component_frame_ignores_a_contradicting_entry() {
    assert_eq!(four(Some(2), &parms(0)), four(Some(2), ""));
    assert_eq!(four(Some(0), &parms(1)), four(Some(0), ""));
    assert_ne!(four(Some(2), ""), four(Some(0), ""));
}
