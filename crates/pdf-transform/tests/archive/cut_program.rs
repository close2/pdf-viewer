//! A font program whose decode stops short is answered as an unembedded font is: the front is
//! dropped and the shipped face takes the font's place under its descriptor's names and its
//! dictionary's widths (`doc/adr/2636`), and a program decoded whole is kept. The window draws such
//! a font from the substitute face it draws a font naming no program from (`doc/adr/2870`), so the
//! face is written whole and the page draws what the window drew; only a font the window reaches no
//! substitute for, and draws from its widths alone, has every shown glyph emptied
//! (`doc/adr/2830`).

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
    a_font_flagged_and_encoded(program, 32, "/WinAnsiEncoding")
}

/// [`a_font_whose_program_is`] with the descriptor's `/Flags` and the font's `/Encoding` given.
fn a_font_flagged_and_encoded(program: &[u8], flags: u32, encoding: &str) -> Vec<u8> {
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
                 /Encoding {encoding} >>"
            ),
            format!(
                "<< /Type /FontDescriptor /FontName /LiberationSans /Flags {flags} \
                 /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
                 /CapHeight 716 /StemV 80 /FontFile2 8 0 R >>"
            ),
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

/// Whether `output` draws the text `source` drew, where `source` drew marks: the same ink, box for
/// box, to a pixel.
///
/// Not pixel for pixel: the window draws the source from the substitute face through the
/// substitution's route and the output's embedded program through the embedded one, and the two
/// differ in edge pixels of the same outlines (measured: 867 of 160 000 channel values, the glyphs
/// in the same place — looked at, trap 1). What is asserted is the thing a blank would fail: ink
/// where the window put ink, within one pixel of its box and a tenth of its amount.
fn draws_the_text_drawn(source: &[u8], output: &[u8]) -> bool {
    /// The bounding box of the non-white pixels and how many there are.
    fn ink(raster: &pdf_render::Raster) -> Option<([u32; 4], usize)> {
        let mut found: Option<[u32; 4]> = None;
        let mut count = 0;
        for (index, pixel) in (0_u32..).zip(raster.data.as_chunks::<4>().0) {
            if pixel[..3] == [0xFF, 0xFF, 0xFF] {
                continue;
            }
            count = usize::saturating_add(count, 1);
            let (Some(x), Some(y)) = (
                index.checked_rem(raster.width),
                index.checked_div(raster.width),
            ) else {
                continue;
            };
            found = Some(found.map_or([x, y, x, y], |[x0, y0, x1, y1]| {
                [x0.min(x), y0.min(y), x1.max(x), y1.max(y)]
            }));
        }
        found.map(|found| (found, count))
    }
    let (Some((before, drawn)), Some((after, written))) =
        (ink(&pixels(source)), ink(&pixels(output)))
    else {
        return false;
    };
    let near = before
        .iter()
        .zip(after)
        .all(|(before, after)| before.abs_diff(after) <= 1);
    near && drawn.abs_diff(written).saturating_mul(10) < drawn
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
fn a_program_cut_mid_stream_is_dropped_and_the_face_the_window_drew_is_written_whole() {
    // ISO 32000-2 §9.9 embeds a program "as data contained in a PDF stream object", and a stream
    // cut inside a stored block whose LEN runs past the bytes holds only the front of one. The
    // checker fails it as it fails a font naming none (ADR 2619); the converter drops the front
    // and embeds the face a font naming no program is drawn from (ADR 2636). §9.5's "information
    // that can be used to provide a substitute when the font program is not available" is what
    // the window draws "Hi" from (ADR 2870), so that face is written whole, under no loss of the
    // unreadable program's, and the page draws what the window drew.
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
    assert!(!dropped.drawn_as_nothing);
    assert!(
        !entry
            .losses
            .iter()
            .any(|loss| loss == Loss::UnreadableProgram),
        "{:?}",
        entry.losses
    );
    let (document, font) = described_font(&output);
    let shipped =
        pdf_font::standard::shipped_face(&document, &font, "F1").expect("a face is shipped");
    assert_eq!(embedded_program(&output), shipped.program.to_vec());
    // "Hi" in the substitute, pixel for pixel the window's.
    assert!(draws_the_text_drawn(&source, &output));
    let text = conversion(&report).render();
    assert!(text.contains("does not decode whole (Truncated)"), "{text}");
    assert!(text.contains("from a substitute face"), "{text}");
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
fn a_program_cut_inside_its_checksum_is_written_whole_again() {
    // The stream stops two bytes into RFC 1950's Adler-32, after every byte of the program, as
    // `F001u_3_7j`'s does: the decode stops short, so the checker fails it (ADR 2619), but the
    // front is the whole face, as many bytes as its `/Length1` states, and this reader draws from
    // it. The conversion writes those bytes again as a whole stream at the program's own object,
    // the producer's `/Length1` beside them, and every shape stays (ADR 3256).
    let whole = zlib_stored(LIBERATION_SANS);
    let cut = whole
        .get(..whole.len() - 2)
        .expect("the stream is longer than two bytes");
    let source = a_font_whose_program_is(cut);
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        assert_eq!(holds(&source, target).verdict(), Verdict::Fails);
        for profile in [the_face_losses_authorised(), Authorisations::default()] {
            let (report, output) = convert(&source, target, profile);
            let output = written(&report, output);
            assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
            assert!(
                conversion(&report).substituted.is_empty(),
                "no face replaces it: {:?}",
                conversion(&report).substituted
            );
            assert_eq!(embedded_program(&output), LIBERATION_SANS.to_vec());
            assert!(draws_the_text_drawn(&source, &output));
        }
    }

    // Where the stream states an extent the decode does not reach exactly, the bytes may not be
    // the program the producer described, so they are not written whole: this one, which this
    // reader does not load, takes ADR 2636's route instead. `/Length1` one more than the program,
    // spelled in as many digits so that no offset moves.
    let stated = format!("/Length1 {}", LIBERATION_SANS.len());
    let other = format!("/Length1 {}", LIBERATION_SANS.len() + 1);
    assert_eq!(stated.len(), other.len());
    let at = source
        .windows(stated.len())
        .position(|window| window == stated.as_bytes())
        .expect("the fixture states /Length1");
    let mut source = source;
    source[at..at + other.len()].copy_from_slice(other.as_bytes());
    let (report, output) = convert(&source, Target::Two(Level::B), the_face_losses_authorised());
    written(&report, output);
    let entry = conversion(&report).substituted.first().cloned();
    assert!(
        entry.and_then(|entry| entry.cut_program).is_some(),
        "{:?}",
        conversion(&report).substituted
    );
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
        assert!(draws_the_text_drawn(&source, &output));
        // The file's own history says what the file held: a program removed, never a font that
        // "was not embedded"; and of a program its checksum disowns, that it decoded to its end
        // rather than that the file held only its front (ADR 3338).
        let recorded = conversion(&report)
            .recorded
            .clone()
            .expect("the substitution is recorded");
        assert!(
            recorded.contains("carried a program this conversion removed")
                && !recorded.contains("was not embedded"),
            "{recorded}"
        );
        let front = recorded.contains("held only the front");
        assert_eq!(
            front,
            damage != pdf_syntax::filter::Damage::CheckValue,
            "{recorded}"
        );
        if damage == pdf_syntax::filter::Damage::CheckValue {
            assert!(recorded.contains("decodes to its end"), "{recorded}");
        }
    }
}

#[test]
fn a_symbolic_font_whose_program_its_checksum_disowns_is_restated_over_the_face_the_window_drew() {
    // `507676`'s Ghostscript `TTE…` fonts: symbolic, an `/Encoding` stating `/Differences`, and a
    // `/FontFile2` whose Adler-32 disagrees, which this reader does not load and the window draws
    // from the substitute face in its place (ADR 2870). Section 6.2.11.6's rows name such a font,
    // and its codes are restated over the face the substitution embeds, asked of the font the way
    // the window asks it (ADR 3326).
    let target = Target::Two(Level::B);
    let mut disowned = zlib_stored(LIBERATION_SANS);
    let last = disowned.len() - 1;
    disowned[last] ^= 0xFF;
    let source = a_font_flagged_and_encoded(
        &disowned,
        4,
        "<< /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [72 /H 105 /i] >>",
    );
    assert_eq!(holds(&source, target).verdict(), Verdict::Fails);
    let (report, output) = convert(&source, target, the_face_losses_authorised());
    let output = written(&report, output);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert!(draws_the_text_drawn(&source, &output));
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
    assert!(draws_the_text_drawn(&source, &output));
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
fn a_font_drawn_as_nothing_is_refused_where_its_loss_is_withheld_and_the_face_losses_change_nothing()
 {
    // The program written states that the text draws nothing, where the source left the face to
    // the reader (ISO 32000-2 §9.5, §9.8.1): a loss of its own, which the face losses do not
    // authorise (ADR 2830). The window draws nothing only where it reaches no substitute, which
    // is the composite font below with no `/ToUnicode` (§9.7.4.2, ADR 2870).
    let whole = zlib_stored(LIBERATION_SANS);
    let source = an_identity_font_whose_program_is(
        whole.get(..4096).expect("the face is far longer"),
        false,
    );
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
/// three codes a `/ToUnicode` names where `named` holds, as `issue11651`'s `NotoSans-Regular` is.
fn an_identity_font_whose_program_is(program: &[u8], named: bool) -> Vec<u8> {
    let cmap = "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
                /CMapName /Test def\n/CMapType 2 def\n\
                1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
                3 beginbfchar\n<0024> <0041>\n<0025> <0042>\n<0030> <0043>\nendbfchar\n\
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
            format!(
                "<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding /Identity-H \
                 /DescendantFonts [7 0 R] {} >>",
                if named { "/ToUnicode 10 0 R" } else { "" }
            ),
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
fn a_composite_font_whose_program_is_cut_and_reaches_no_substitute_draws_nothing() {
    // With no `/ToUnicode` and an `Adobe-Identity` collection, §9.7.4.2 leaves a substitute no
    // route to a glyph — "CIDs shall not participate in glyph selection" — so the window draws the
    // font from its `CMap` and `/W` alone (ADRs 1094, 2870). Every shown CID goes to an empty glyph
    // at its `/W` width under a `/CIDToGIDMap` stream (§9.7.4.2), so the page is the window's (ADR
    // 2830). The descendant states no map, so `fonts/cid-to-gid-map-present`'s `Identity` would
    // reach glyphs the subset does not hold if it were written over the stream.
    let whole = zlib_stored(LIBERATION_SANS);
    let source = an_identity_font_whose_program_is(
        whole.get(..4096).expect("the face is far longer"),
        false,
    );
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
    assert_eq!(
        entry.cut_program.map(|cut| (cut.key, cut.drawn_as_nothing)),
        Some(("FontFile2", true))
    );
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

#[test]
fn a_composite_font_whose_program_is_cut_takes_the_route_of_one_naming_none() {
    // `issue11651`: the composite font's `/ToUnicode` names each shown code, so §9.7.4.2's route to
    // a substitute is open and the window draws "ABA" from a face of this machine's, as it draws a
    // font naming no program (ADR 2870). The converter therefore takes the route such a font takes
    // (ADR 2636), and no glyph is emptied: the loss `unreadable-program` is not this font's.
    let whole = zlib_stored(LIBERATION_SANS);
    let source =
        an_identity_font_whose_program_is(whole.get(..4096).expect("the face is far longer"), true);
    let document = Document::open_with_limits(source.clone(), Limits::DEFAULT).expect("it opens");
    let font = document.get(ObjectId::new(6, 0));
    let font = font.as_dict().expect("the font");
    let dropped = pdf_font::without_program(&document, font).expect("a descriptor to copy");
    if !pdf_font::LoadedFont::machine_offers_a_substitute(&document, &dropped) {
        eprintln!("skipped: this machine offers no face a composite font can be addressed in");
        return;
    }
    let target = Target::Two(Level::B);
    let (report, output) = convert(&source, target, the_face_losses_authorised());
    let entry = conversion(&report)
        .substituted
        .first()
        .cloned()
        .unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(
        entry.cut_program.map(|cut| (cut.key, cut.drawn_as_nothing)),
        Some(("FontFile2", false))
    );
    assert!(entry.empty_cids.is_empty(), "{:?}", entry.empty_cids);
    assert!(
        !entry
            .losses
            .iter()
            .any(|loss| loss == Loss::UnreadableProgram),
        "{:?}",
        entry.losses
    );
    let output = written(&report, output);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert!(pixels(&output).data.iter().any(|byte| *byte != 0xFF));
    // Where the profile withholds `machine-face`, as `as-shown` does, the machine's face the window
    // drew the substitute from is the face written, held by its proof to the window's outlines and
    // advances: `issue11651` under `as-shown` (ADR 2871).
    let as_shown = Authorisations {
        glyphs_from_to_unicode: true,
        ..Authorisations::default()
    };
    let (report, output) = convert(&source, target, as_shown);
    let output = written(&report, output);
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert!(draws_the_text_drawn(&source, &output));
}
