//! A font program whose decode stops short is answered as an unembedded font is: the front is
//! dropped and the shipped face takes the font's place under its descriptor's names and its
//! dictionary's widths (`doc/adr/2636`), and a program decoded whole is kept.

use super::*;

/// RFC 1950's zlib framing around RFC 1951's stored blocks of `data` — the encoding ISO 32000-2
/// §7.4.4.1 names for `FlateDecode` — so that a cut through it falls inside a block whose stated
/// length runs past the bytes.
pub(super) fn zlib_stored(data: &[u8]) -> Vec<u8> {
    // RFC 1950 section 2.2: CM 8, CINFO 7, and an FCHECK making the header a multiple of 31.
    let mut out = vec![0x78, 0x01];
    let mut blocks = data.chunks(u16::MAX.into()).peekable();
    while let Some(block) = blocks.next() {
        // RFC 1951 section 3.2.4: BFINAL, BTYPE 00, then LEN and its one's complement NLEN.
        out.push(u8::from(blocks.peek().is_none()));
        let len = u16::try_from(block.len()).expect("a chunk is at most 65535 bytes");
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    // RFC 1950 section 8.2: the Adler-32 of the uncompressed data, most significant byte first.
    // Both sums stay below 65521 before each addition, so neither addition can wrap.
    let (low, high) = data.iter().fold((1_u32, 0_u32), |(low, high), byte| {
        let low = low.wrapping_add(u32::from(*byte)) % 65521;
        (low, high.wrapping_add(low) % 65521)
    });
    out.extend_from_slice(&((high << 16) | low).to_be_bytes());
    out
}

/// [`a_truetype_font`]'s document showing "Hi", its `/FontFile2` the zlib stream `program` under
/// `FlateDecode`, `/Length1` the whole face's length.
fn a_font_whose_program_is(program: &[u8]) -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 48 Tf 10 100 Td (Hi) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 32 \
                 /LastChar 126 /Widths [{LIBERATION_SANS_ADVANCES}] /FontDescriptor 7 0 R \
                 /Encoding /WinAnsiEncoding >>"
            ),
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
             /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 /FontFile2 8 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(
            &format!(
                "/Filter /FlateDecode /Length {} /Length1 {}",
                program.len(),
                LIBERATION_SANS.len()
            ),
            program,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// The decoded `/FontFile2` of the converted font object 6.
fn embedded_program(output: &[u8]) -> Vec<u8> {
    let (document, font) = described_font(output);
    let descriptor = document
        .get_key(&font, "FontDescriptor")
        .as_dict()
        .cloned()
        .expect("the converted font has a descriptor");
    let pdf_syntax::Object::Stream(program) = document.get_key(&descriptor, "FontFile2") else {
        panic!("the converted descriptor states no FontFile2: {descriptor:?}");
    };
    document
        .decoded_stream_data(&program)
        .expect("the embedded program decodes")
        .to_vec()
}

#[test]
fn a_program_cut_mid_stream_is_dropped_and_the_shipped_face_draws_the_font() {
    // ISO 32000-2 §9.9 embeds a program "as data contained in a PDF stream object", and a stream
    // cut inside a stored block whose LEN runs past the bytes holds only the front of one. The
    // checker fails it as it fails a font naming none (ADR 2619); the converter drops the front
    // and embeds the face a font naming no program is drawn from (ADR 2636).
    let whole = zlib_stored(LIBERATION_SANS);
    let cut = whole.get(..4096).expect("the face is far longer");
    let source = a_font_whose_program_is(cut);
    let target = Target::Two(Level::B);
    assert_eq!(holds(&source, target).verdict(), Verdict::Fails);
    let (report, output) = convert(&source, target, the_face_losses_authorised());
    let output = written(&report, output);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let substituted = &conversion(&report).substituted;
    assert_eq!(substituted.len(), 1, "{substituted:?}");
    let entry = substituted.first().expect("one entry");
    let dropped = entry.cut_program.expect("the entry names the cut program");
    assert_eq!(dropped.key, "FontFile2");
    assert_eq!(dropped.damage, Some(pdf_syntax::filter::Damage::Truncated));
    let (document, font) = described_font(&output);
    let shipped =
        pdf_font::standard::shipped_face(&document, &font, "F1").expect("a face is shipped");
    assert_eq!(embedded_program(&output), shipped.program.to_vec());
    assert_ne!(embedded_program(&output), cut.to_vec());
    // The page draws the two glyphs in the substitute where it drew nothing from the front.
    assert_ne!(drawn(&output), drawn(&source));
    let text = conversion(&report).render();
    assert!(text.contains("does not decode whole (Truncated)"), "{text}");
}

#[test]
fn a_program_whole_with_one_byte_after_its_checksum_is_kept() {
    // The plant: the stream holds the whole zlib stream, its Adler-32 included, and one byte past
    // it, which RFC 1950 does not read. The decode does not stop short, so the program is
    // embedded and no substitution is routed.
    let mut whole = zlib_stored(LIBERATION_SANS);
    whole.push(b'\n');
    let source = a_font_whose_program_is(&whole);
    let target = Target::Two(Level::B);
    let (report, output) = convert(&source, target, the_face_losses_authorised());
    let output = written(&report, output);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert!(
        conversion(&report)
            .substituted
            .iter()
            .all(|entry| entry.cut_program.is_none()),
        "{:?}",
        conversion(&report).substituted
    );
    assert_eq!(embedded_program(&output), LIBERATION_SANS.to_vec());
}

#[test]
fn a_program_cut_inside_its_checksum_is_drawn_whole_and_refused_by_its_own_sentence() {
    // The stream stops two bytes into RFC 1950's Adler-32, after every byte of the program: the
    // decode stops short, so the checker fails it (ADR 2619), but the front is the whole face and
    // this reader draws from it. Dropping it would change the shapes the window drew, so the
    // route does not take it and the refusal says why (ADR 2636).
    let whole = zlib_stored(LIBERATION_SANS);
    let cut = whole
        .get(..whole.len() - 2)
        .expect("the stream is longer than two bytes");
    let source = a_font_whose_program_is(cut);
    let target = Target::Two(Level::B);
    assert_eq!(holds(&source, target).verdict(), Verdict::Fails);
    let (report, _) = convert(&source, target, the_face_losses_authorised());
    let Decision::Refused(Because::NotBuiltYet(because)) =
        decision(&report, "fonts/font-programs-embedded")
    else {
        panic!("{:#?}", conversion(&report).decided);
    };
    assert!(because.contains("loads and draws from"), "{because}");
}

#[test]
fn a_program_refused_outright_or_disowned_by_its_checksum_is_substituted() {
    // A stream whose zlib header is none hands over no program, and one whose Adler-32 disagrees
    // hands over bytes its own checksum disowns, which this reader does not load either: each is
    // a font naming no program, so the route takes it (ADR 2636).
    let target = Target::Two(Level::B);
    let mut refused = zlib_stored(LIBERATION_SANS);
    refused[0] = 0xFF;
    let mut disowned = zlib_stored(LIBERATION_SANS);
    let last = disowned.len() - 1;
    disowned[last] ^= 0xFF;
    for (program, damage) in [
        (refused, pdf_syntax::filter::Damage::Corrupt),
        (disowned, pdf_syntax::filter::Damage::CheckValue),
    ] {
        let source = a_font_whose_program_is(&program);
        assert_eq!(holds(&source, target).verdict(), Verdict::Fails);
        let (report, output) = convert(&source, target, the_face_losses_authorised());
        let output = written(&report, output);
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        let entry = conversion(&report).substituted.first().cloned();
        let dropped = entry.and_then(|entry| entry.cut_program);
        assert_eq!(dropped.and_then(|cut| cut.damage), Some(damage));
        assert_eq!(embedded_program(&output).len(), {
            let (document, font) = described_font(&output);
            pdf_font::standard::shipped_face(&document, &font, "F1")
                .expect("a face is shipped")
                .program
                .len()
        });
    }
}

#[test]
fn a_whole_program_that_is_not_one_in_its_format_is_substituted() {
    // `178360`'s `ICLJGJ+Glypha` decodes whole and every zero byte of it reads 0x20, so it is not
    // the program its key files it as and this reader loads none of it: the checker fails it as
    // a font naming no program and the route takes it (ADR 2636). The same face with its zero
    // bytes whole is the plant above, kept.
    let spaced: Vec<u8> = LIBERATION_SANS
        .iter()
        .map(|&byte| if byte == 0 { 0x20 } else { byte })
        .collect();
    let source = a_font_whose_program_is(&zlib_stored(&spaced));
    let target = Target::Two(Level::B);
    assert_eq!(holds(&source, target).verdict(), Verdict::Fails);
    let (report, output) = convert(&source, target, the_face_losses_authorised());
    let output = written(&report, output);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let entry = conversion(&report).substituted.first().cloned();
    let dropped = entry.and_then(|entry| entry.cut_program);
    assert_eq!(
        dropped.map(|cut| (cut.key, cut.damage)),
        Some(("FontFile2", None))
    );
    let (document, font) = described_font(&output);
    let shipped =
        pdf_font::standard::shipped_face(&document, &font, "F1").expect("a face is shipped");
    assert_eq!(embedded_program(&output), shipped.program.to_vec());
    let text = conversion(&report).render();
    assert!(
        text.contains("decodes whole and is not a program"),
        "{text}"
    );
}
