//! A Type 0 font naming no `CIDFont` is given one that draws nothing at the width this reader
//! moved past each code by (ADR 3109), and refused by name where this reader did otherwise.

use super::*;

const ROW: &str = "fonts/descendant-fonts-holds-one-cidfont";

/// A page showing two codes under `/F1`, a Type 0 font over `encoding` whose `/DescendantFonts`
/// is `[ null ]`, between two glyphs of `/F2`, an embedded `LiberationSans` under a `CIDFontType2`
/// whose CID 36 is its `A`, with a character spacing in force and no positioning operator between.
///
/// ISO 32000-2 §9.4.4 places the second `A` after the first's width, the two codes' and three
/// character spacings, so where the middle string goes is a mark on the page.
fn between_two_glyphs(target: Target, encoding: &str, middle: &str) -> Vec<u8> {
    let to_unicode = "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
                      /CMapName /Test def\n/CMapType 2 def\n\
                      1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
                      1 beginbfchar\n<0024> <0041>\nendbfchar\n\
                      endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";
    Conforming {
        target,
        resources: "/Font << /F1 6 0 R /F2 7 0 R >> /ColorSpace << /CS0 [/CalGray << \
                    /WhitePoint [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            format!(
                "/CS0 cs 0 sc BT 3 Tc /F2 12 Tf 10 100 Td <0024> Tj /F1 12 Tf <{middle}> Tj \
                 /F2 12 Tf <0024> Tj ET"
            )
            .into_bytes(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /Type0 /BaseFont /NUKKLY+Absent /Encoding /{encoding} \
                 /DescendantFonts [ null ] /ToUnicode 10 0 R >>"
            ),
            "<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding /Identity-H \
             /DescendantFonts [8 0 R] /ToUnicode 10 0 R >>"
                .to_owned(),
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
             /FontDescriptor 9 0 R /DW 1000 /W [36 [667]] /CIDToGIDMap /Identity >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
             /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 /FontFile2 11 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![
            stream(
                &format!("/Length {}", to_unicode.len()),
                to_unicode.as_bytes(),
            ),
            stream(
                &format!("/Length {}", LIBERATION_SANS.len()),
                LIBERATION_SANS,
            ),
        ],
        ..match target {
            Target::Two(_) => Conforming::part_two(),
            Target::Four(_) => Conforming::default(),
        }
    }
    .build()
}

/// Page 1 as this reader draws it, on the processor, as RGBA bytes.
fn pixels(bytes: &[u8]) -> Vec<u8> {
    use pdf_render::{Rasterizer as _, TargetSpec};
    let document = Document::open(bytes.to_vec()).expect("an exact-xref fixture");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let state = pdf_model::view::ViewState::of(&document);
    let list = pdf_model::content::interpret_with(&document, &page, &state).display_list;
    let target = TargetSpec::for_page(&list, 2.0, 1 << 22).expect("a bounded target");
    render_cpu::CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("CPU pixels")
        .data
}

#[test]
fn a_type0_font_naming_no_cidfont_draws_and_moves_as_this_reader_did() {
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let source = between_two_glyphs(target, "Identity-H", "00240024");
        // The control: where the middle string is drawn is a mark, because the second `A` is
        // placed after it (§9.4.4) — so the comparison below would see a font that moved
        // otherwise.
        let unmoved = between_two_glyphs(target, "Identity-H", "");
        assert_ne!(pixels(&source), pixels(&unmoved), "{target:?}");

        let (report, output) = convert(&source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, ROW),
                Decision::Stated {
                    rewrite: Rewrite::SubstituteFontProgram,
                    ..
                }
            ),
            "{target:?}: {:?}",
            decision(&report, ROW)
        );
        let output = written(&report, output);
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target:?}"
        );
        // ISO 32000-2 §9.4.4 with Table 115's default width: each code of the font the window
        // drew nothing for moved the pen 1000 thousandths of an em plus the spacing, and the
        // written font draws nothing and moves the same.
        assert_eq!(pixels(&output), pixels(&source), "{target:?}");

        // A48's condition: the empty CIDs are reported with the font, and the file's own
        // history names the clause.
        let substituted = &conversion(&report).substituted;
        let written_for = substituted
            .iter()
            .find(|font| font.resource == "F1")
            .expect("the font is reported");
        assert!(written_for.descendant_written, "{written_for:?}");
        assert_eq!(written_for.empty_cids, vec![36]);
        assert!(written_for.losses.is_empty(), "{written_for:?}");
        let history = String::from_utf8_lossy(&output).into_owned();
        assert!(history.contains("6.2.11.8"), "the history names the clause");

        // The written font: one CIDFontType2 at the default width, every CID sent to a glyph of
        // its own that is not `.notdef`.
        let document = Document::open(output).expect("the output opens");
        let page = pdf_model::Pages::new(&document).get(0).expect("one page");
        let fonts = document.get_key(&page.resources, "Font");
        let font = document.get_key(fonts.as_dict().expect("fonts"), "F1");
        let font = font.as_dict().expect("a font");
        let descendants = document.get_key(font, "DescendantFonts");
        let descendant = document.resolve(
            descendants
                .as_array()
                .and_then(<[pdf_syntax::Object]>::first)
                .expect("one descendant"),
        );
        let descendant = descendant.as_dict().expect("a CIDFont");
        assert_eq!(
            document
                .get_key(descendant, "Subtype")
                .as_name()
                .map(|name| name.as_bytes().to_vec()),
            Some(b"CIDFontType2".to_vec())
        );
        assert_eq!(document.get_key(descendant, "DW").as_number(), Some(1000.0));
        assert!(
            matches!(
                document.get_key(font, "ToUnicode"),
                pdf_syntax::Object::Stream(_)
            ),
            "the producer's ToUnicode is kept"
        );
        let loaded = pdf_font::LoadedFont::load(&document, font, "F1").expect("the font loads");
        for code in loaded.decode(&[0x00, 0x24, 0xff, 0xff]) {
            let glyph = loaded.glyph_index(code).expect("a glyph");
            assert_ne!(glyph, pdf_font::NOTDEF_GLYPH, "{code:?}");
            assert!((loaded.advance(code) - 1.0).abs() < 1e-3, "{code:?}");
        }
    }
}

#[test]
fn a_type0_font_naming_no_cidfont_over_another_cmap_stays_refused_by_name() {
    // `UniGB-UCS2-H` is one of Table 116's CMaps, whose codes this route does not read as their
    // own CIDs, so it is not the font the route writes.
    let target = Target::Two(Level::B);
    let source = between_two_glyphs(target, "UniGB-UCS2-H", "00240024");
    let (report, output) = convert(&source, target, Authorisations::default());
    let Decision::Refused(Because::NotBuiltYet(because)) = decision(&report, ROW) else {
        panic!("{:?}", decision(&report, ROW));
    };
    assert!(
        because.contains("no CIDFont as its descendant"),
        "{because}"
    );
    assert!(output.is_none(), "and nothing is written");
}
