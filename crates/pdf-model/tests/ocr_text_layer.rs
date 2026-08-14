//! A hollow embedded font under an invisible OCR layer, ISO 32000-2 §9.7.4.2 and Table 120.
//!
//! `OmniPage` CSDK 22 writes scanned pages whose invisible text layer (Table 104's mode 3)
//! uses
//! an embedded `CIDFontType2` that carries `head` and `hmtx` and **not one glyph outline** —
//! every `loca` entry empty — with a `/CIDToGIDMap` remapping stream. A reader that derives a
//! character's vertical extent from the glyph outlines collapses to zero-height boxes on such
//! a file: no selection overlay, a razor-thin hit line (old `PDFium`'s `FPDFText_GetCharBox`
//! did exactly that). ADR 0350 is the finding these tests pin: this tree's selection geometry
//! never asks the outlines — `glyph_quad` takes Table 120's `/Ascent`/`/Descent` through
//! `pdf_font::measured_extent`'s band (ADR 0216) — so nothing collapses, and these tests are
//! what keeps that true.
//!
//! Hand-built per trap 8: no corpus document carries the hollow-program-plus-stream shape
//! (`issue14821.pdf` has empty `loca` entries but no remapping stream, and every stream
//! witness has real outlines). The pair differs only in the rule under test — CSDK 20's shape
//! beside CSDK 22's, same text, same layout.
//!
//! `selection_geometry.rs` is the companion file: it pins the *band* — which descriptor pairs
//! are believed, and that Table 104's invisible modes place every glyph — over a simple font.
//! What is new here is the font whose program actively states zero extents for every glyph,
//! which is the one shape that separates a metrics-derived band from an outline-derived one.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly"
)]

use pdf_syntax::Document;
use test_scenes::{
    OCR_ASCENT, OCR_BASELINE, OCR_CID_COUNT, OCR_DESCENT, OCR_FIRST_WORD, OCR_FONT_SIZE,
    OCR_SECOND_WORD, OcrFont, ocr_advance_for_gid, ocr_gid_for_cid, scanned_ocr_pdf,
};

/// Table 104's invisible mode, the producer's own choice for an OCR layer.
const INVISIBLE: u8 = 3;

/// Page one of a fixture, interpreted.
fn interpreted(font: OcrFont, render_mode: u8) -> pdf_model::Interpretation {
    let document = Document::open(scanned_ocr_pdf(font, render_mode)).expect("the fixture opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    pdf_model::interpret(&document, &page)
}

/// The invisible layer's text extracts, exactly as it does from every other reader.
///
/// The CSDK 20 shape's font is *substituted* (§9.7.4.2: with no embedded program a
/// `/CIDToGIDMap` "shall be ignored" and the font is reached through what its codes mean), so
/// whether it loads depends on the machine's faces; the assertion is the equivalence — either
/// the font is reported by name, or the text is there. The embedded shape depends on nothing.
#[test]
fn the_invisible_layer_extracts_from_both_shapes() {
    for font in [OcrFont::HollowEmbedded, OcrFont::NotEmbedded] {
        let interpretation = interpreted(font, INVISIBLE);
        let extracted = interpretation.text.clone();
        let complete = extracted.contains(OCR_FIRST_WORD) && extracted.contains(OCR_SECOND_WORD);
        if font == OcrFont::HollowEmbedded {
            assert!(
                complete,
                "the hollow program embeds everything extraction needs: {extracted:?}"
            );
        } else {
            let reported = format!("{:?}", interpretation.unsupported).contains("F0");
            assert!(
                complete || reported,
                "a substituted font extracts or is reported by name: {extracted:?}, {:?}",
                interpretation.unsupported
            );
        }
    }
}

/// The selection band comes from the descriptor, so the hollow outlines cannot collapse it.
///
/// Every `text_layer` quadrilateral must be exactly the band Table 120 states — ascent 0.718,
/// descent −0.207, at 12 pt — sitting on the fixture's own baseline. The expected numbers are
/// derived from the file's statements, not from any renderer: §9.4.4 maps glyph space by the
/// font size, and `/Ascent` and `/Descent` are "the maximum height above the baseline" and
/// "the maximum depth below the baseline" (Table 120), in thousandths (§9.2.4).
#[test]
fn the_selection_band_is_the_descriptors_not_the_outlines() {
    let interpretation = interpreted(OcrFont::HollowEmbedded, INVISIBLE);
    assert_eq!(
        interpretation.text_layer.len(),
        usize::from(OCR_CID_COUNT),
        "one placed quadrilateral per shown CID"
    );

    let expected_top = OCR_BASELINE + OCR_ASCENT / 1000.0 * OCR_FONT_SIZE;
    let expected_bottom = OCR_BASELINE + OCR_DESCENT / 1000.0 * OCR_FONT_SIZE;
    for placed in &interpretation.text_layer {
        let quad = placed.quad;
        // Corners in order: (0, descent), (advance, descent), (advance, ascent), (0, ascent).
        let (bottom, top) = (quad[1], quad[7]);
        assert!(
            (top - expected_top).abs() < 0.01 && (bottom - expected_bottom).abs() < 0.01,
            "the band is {bottom}..{top}, Table 120 says {expected_bottom}..{expected_top}"
        );
        assert!(
            top - bottom > 11.0,
            "an 11.1 pt band cannot have collapsed to {}",
            top - bottom
        );
    }
}

/// A hollow font is counted in the blank band exactly where a mark is owed — and mode 3 owes
/// none.
///
/// ADR 0270's split: a code that reached a glyph the program contains and describes with no
/// contours is `codes_reaching_a_blank_glyph`, apart from a code that reached nothing. The
/// invisible layer never enters that accounting — Table 104's mode 3 neither fills nor
/// strokes, so no mark is owed and nothing is counted or reported; the same file with mode 0
/// owes every mark, counts all ten in the blank band (the stream mapped every CID to a glyph
/// the program *contains*), none in the absent band, and reports the font as drawing nothing.
#[test]
fn the_blank_glyph_band_counts_only_where_a_mark_is_owed() {
    let invisible = interpreted(OcrFont::HollowEmbedded, INVISIBLE);
    assert_eq!(invisible.glyphs, 0, "mode 3 draws nothing");
    assert_eq!(
        (
            invisible.codes_reaching_a_blank_glyph,
            invisible.codes_without_a_glyph
        ),
        (0, 0),
        "no mark is owed, so none is counted as missed"
    );
    assert!(
        invisible.unsupported.is_empty(),
        "an invisible layer over a drawn image is a complete page: {:?}",
        invisible.unsupported
    );

    let visible = interpreted(OcrFont::HollowEmbedded, 0);
    assert_eq!(
        (
            visible.codes_reaching_a_blank_glyph,
            visible.codes_without_a_glyph
        ),
        (usize::from(OCR_CID_COUNT), 0),
        "every CID reaches a glyph the program contains and describes as empty"
    );
    let said = format!("{:?}", visible.unsupported);
    assert!(
        said.contains("/F0") && said.contains("no outline for any"),
        "a visible page of hollow text must say the font drew nothing: {said}"
    );
}

/// §9.7.4.2: the remapping stream is read, and the identity would disagree measurably.
///
/// The `/W` widths are keyed by CID and the program's `hmtx` advances by glyph index — two
/// statements of one fact through separate structures, the same cross-check
/// `composite_fonts.rs` runs over the corpus — and the fixture's stream is the identity for
/// no CID it covers, so they agree only if Table 115's "2-byte value stored in bytes 2 × c
/// and 2 × c + 1" was actually read.
#[test]
fn the_remapping_stream_is_read_not_the_identity() {
    let document =
        Document::open(scanned_ocr_pdf(OcrFont::HollowEmbedded, INVISIBLE)).expect("opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("a page");
    let fonts = document.get_key(&page.resources, "Font");
    let fonts = fonts.as_dict().expect("the page names its font");
    let (_, entry) = fonts.iter().next().expect("/F0 is there");
    let resolved = document.resolve(entry);
    let dict = resolved.as_dict().expect("a font dictionary");
    let font = pdf_font::LoadedFont::load(&document, dict, "F0").expect("the hollow font loads");

    for cid in 1..=OCR_CID_COUNT {
        let bytes = cid.to_be_bytes();
        let codes = font.decode(&bytes);
        let code = *codes.first().expect("two bytes are one Identity-H code");
        assert_eq!(
            font.glyph_index(code),
            Some(ocr_gid_for_cid(cid)),
            "CID {cid} reaches the glyph the stream maps, not itself"
        );
        assert!(
            font.outline(code).is_none(),
            "every glyph is empty, so no code has an outline"
        );
        let advance = font.advance(code) * 1000.0;
        let stated = f32::from(ocr_advance_for_gid(ocr_gid_for_cid(cid)));
        assert!(
            (advance - stated).abs() < 0.5,
            "CID {cid}: /W says {advance}, hmtx through the stream says {stated}"
        );
    }
}
