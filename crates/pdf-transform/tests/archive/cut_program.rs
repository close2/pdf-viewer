//! A font program whose decode stops short is answered as an unembedded font is: the front is
//! dropped and the shipped face takes the font's place under its descriptor's names and its
//! dictionary's widths (`doc/adr/2636`), and a program decoded whole is kept. This reader drew
//! none of the cut program's glyphs, so every glyph the page shows in that face is emptied and the
//! page draws the nothing the window drew (`doc/adr/2830`).

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

/// The page as the window draws it, on the processor at 72 dpi: what "the output draws what the
/// window drew" is measured on (trap 1).
fn pixels(bytes: &[u8]) -> pdf_render::Raster {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("a first page");
    pdf_transform::render::render_page(
        &document,
        &page,
        &pdf_model::view::ViewState::of(&document),
        &pdf_model::content::FontCache::new(),
        &mut render_cpu::CpuRasterizer::new(),
        pdf_transform::render::Drawing {
            size: pdf_transform::render::Sizing::Dpi(72.0),
            page_box: None,
            annotations: true,
        },
        &Budget::default(),
    )
    .expect("the page draws")
    .raster
}

/// Whether `output` draws, pixel for pixel, what `source` drew, and `source` drew a page with no
/// mark on it: the font this file shows is its only content.
fn draws_the_nothing_drawn(source: &[u8], output: &[u8]) -> bool {
    let before = pixels(source);
    let blank = before.data.iter().all(|byte| *byte == 0xFF);
    blank && pixels(output).data == before.data
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
fn a_program_cut_mid_stream_is_dropped_and_the_font_draws_the_nothing_the_window_drew() {
    // ISO 32000-2 §9.9 embeds a program "as data contained in a PDF stream object", and a stream
    // cut inside a stored block whose LEN runs past the bytes holds only the front of one. The
    // checker fails it as it fails a font naming none (ADR 2619); the converter drops the front
    // and embeds the face a font naming no program is drawn from (ADR 2636), every glyph the page
    // shows in it emptied, since this reader drew the font from its widths alone and marked
    // nothing (ADRs 1094, 2830).
    let whole = zlib_stored(LIBERATION_SANS);
    let cut = whole.get(..4096).expect("the face is far longer");
    let source = a_font_whose_program_is(cut);
    let target = Target::Two(Level::B);
    assert_eq!(holds(&source, target).verdict(), Verdict::Fails);
    let (report, output) = convert(&source, target, the_unreadable_program_authorised());
    let output = written(&report, output);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let substituted = &conversion(&report).substituted;
    assert_eq!(substituted.len(), 1, "{substituted:?}");
    let entry = substituted.first().expect("one entry");
    let dropped = entry.cut_program.expect("the entry names the cut program");
    assert_eq!(dropped.key, "FontFile2");
    assert_eq!(dropped.damage, Some(pdf_syntax::filter::Damage::Truncated));
    assert_eq!(
        entry.losses.iter().map(Loss::word).collect::<Vec<_>>(),
        ["unreadable-program"]
    );
    assert_ne!(embedded_program(&output), cut.to_vec());
    // The page draws nothing, pixel for pixel, where the shipped face whole would draw "Hi".
    assert!(draws_the_nothing_drawn(&source, &output));
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
        let (report, output) = convert(&source, target, the_unreadable_program_authorised());
        let output = written(&report, output);
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        let entry = conversion(&report).substituted.first().cloned();
        let dropped = entry.and_then(|entry| entry.cut_program);
        assert_eq!(dropped.and_then(|cut| cut.damage), Some(damage));
        assert!(draws_the_nothing_drawn(&source, &output));
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
    let (report, output) = convert(&source, target, the_unreadable_program_authorised());
    let output = written(&report, output);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let entry = conversion(&report).substituted.first().cloned();
    let dropped = entry.and_then(|entry| entry.cut_program);
    assert_eq!(
        dropped.map(|cut| (cut.key, cut.damage)),
        Some(("FontFile2", None))
    );
    assert!(draws_the_nothing_drawn(&source, &output));
    let text = conversion(&report).render();
    assert!(
        text.contains("decodes whole and is not a program"),
        "{text}"
    );
}

/// The one loss a font this reader drew as nothing costs, and no other.
fn the_unreadable_program_authorised() -> Authorisations {
    Authorisations {
        unreadable_program: true,
        ..Authorisations::default()
    }
}

#[test]
fn a_cut_program_is_refused_where_its_loss_is_withheld_and_the_face_losses_change_nothing() {
    // The program written states that the text draws nothing, where the source left the face to
    // the reader (ISO 32000-2 §9.1, §9.8.1): a loss of its own, which the face losses that once
    // answered a cut program do not authorise (ADR 2830).
    let whole = zlib_stored(LIBERATION_SANS);
    let source = a_font_whose_program_is(whole.get(..4096).expect("the face is far longer"));
    let target = Target::Two(Level::B);
    let (report, output) = convert(&source, target, the_face_losses_authorised());
    assert!(output.is_none(), "{}", conversion(&report).render());
}

#[test]
fn which_shipped_profiles_admit_an_unreadable_program() {
    // `as-shown` keeps what the window showed, and the window showed nothing; `as-if-printed` and
    // `state-every-loss` carry the font site's whole bundle; every other shipped profile stops.
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        for shipped in pdf_transform::profiles::SHIPPED {
            let config = pdf_transform::archive::Configuration::read(shipped.text, target)
                .expect("it reads");
            let admits = matches!(
                shipped.name,
                "as-shown" | "as-if-printed" | "state-every-loss"
            );
            assert_eq!(
                config
                    .authorisations(target)
                    .grants(Loss::UnreadableProgram),
                admits,
                "{} at {target}",
                shipped.name
            );
        }
    }
}

/// An `Identity-H` composite font over a `CIDFontType2` whose `/FontFile2` is `program`, showing
/// three codes a `/ToUnicode` names, as `issue11651`'s `NotoSans-Regular` is.
fn an_identity_font_whose_program_is(program: &[u8]) -> Vec<u8> {
    let cmap = "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
                /CMapName /Test def\n/CMapType 2 def\n\
                1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
                3 beginbfchar\n<0024> <0041>\n<0025> <0042>\n<0030> <0041>\nendbfchar\n\
                endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 48 Tf 10 100 Td <002400250030> Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding /Identity-H \
             /DescendantFonts [7 0 R] /ToUnicode 10 0 R >>"
                .to_owned(),
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans /CIDSystemInfo \
             << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 8 0 R \
             /DW 1000 /W [36 [667 667] 48 [500]] >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
             /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 /FontFile2 9 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![
            stream(
                &format!(
                    "/Filter /FlateDecode /Length {} /Length1 {}",
                    program.len(),
                    LIBERATION_SANS.len()
                ),
                program,
            ),
            stream(&format!("/Length {}", cmap.len()), cmap.as_bytes()),
        ],
        ..Conforming::part_two()
    }
    .build()
}

#[test]
fn a_composite_font_whose_program_is_cut_sends_every_shown_cid_to_a_glyph_drawing_nothing() {
    // `issue11651`: the window drew the composite font from its `CMap` and `/W` alone (ADR 1094),
    // and the route that would embed a face for its `/ToUnicode` characters drew "AB A" where the
    // window drew nothing. Every shown CID goes to an empty glyph at its `/W` width under a
    // `/CIDToGIDMap` stream (§9.7.4.2), so the page is the window's (ADR 2830). The descendant
    // states no map, as `issue11651`'s does not, so `fonts/cid-to-gid-map-present`'s `Identity`
    // would reach glyphs the subset does not hold if it were written over the stream.
    let whole = zlib_stored(LIBERATION_SANS);
    let source =
        an_identity_font_whose_program_is(whole.get(..4096).expect("the face is far longer"));
    let target = Target::Two(Level::B);
    assert_eq!(holds(&source, target).verdict(), Verdict::Fails);
    let (report, output) = convert(&source, target, the_unreadable_program_authorised());
    let output = written(&report, output);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let entry = conversion(&report)
        .substituted
        .first()
        .cloned()
        .expect("the font is substituted");
    assert_eq!(entry.cut_program.map(|cut| cut.key), Some("FontFile2"));
    assert_eq!(
        entry.losses.iter().map(Loss::word).collect::<Vec<_>>(),
        ["unreadable-program"]
    );
    assert_eq!(entry.empty_cids, [36, 37, 48]);
    assert!(draws_the_nothing_drawn(&source, &output));
    let document = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let font = document.get(ObjectId::new(6, 0));
    let descendant = document
        .resolve(&document.get_key(font.as_dict().expect("the font"), "DescendantFonts"))
        .as_array()
        .and_then(|fonts| fonts.first().cloned())
        .expect("one descendant");
    let descendant = document.resolve(&descendant);
    let map = document.get_key(descendant.as_dict().expect("a dictionary"), "CIDToGIDMap");
    assert!(
        matches!(map, pdf_syntax::Object::Stream(_)),
        "the map is the substitution's stream: {map:?}"
    );
}
