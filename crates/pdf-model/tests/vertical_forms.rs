//! A vertical `CMap` names different CIDs, and a substituted face has to draw different glyphs.
//!
//! ISO 32000-2 §9.7.5.1 makes the writing mode a property of the `CMap` and its NOTE says what
//! follows from that:
//!
//! > Writing mode is specified as part of the CMap because, in some cases, different shapes are
//! > used when writing horizontally and vertically. In such cases, the horizontal and vertical
//! > variants of a CMap specify different CIDs for a given character code.
//!
//! `doc/corpora/pdf-differences/VerticalText/VerticalText.pdf` is the PDF Association's witness
//! for it: `/Encoding /Identity-V` over a non-embedded `CIDFontType0` of `Adobe-Japan1`, so the
//! two-byte codes in the content stream *are* Adobe-Japan1 CIDs and the producer has written the
//! vertical ones — 7911 for 「, 7888 for 。 — beside ordinary kanji and kana. §9.7.4.2 leaves a
//! substitute reachable only by character, and the collection's `Adobe-Japan1-UCS2` table sends
//! both of U+300C's CIDs to the same character, so before this route existed the page drew its
//! brackets lying on their sides and its full stops in the middle of the column.
//!
//! # What is asserted, and what is this machine's
//!
//! **The collection half is not machine-dependent at all** and is asserted in
//! `pdf_font::predefined`'s own tests: which CID is the vertical form of which character is two
//! compiled-in files of Adobe's, and holds everywhere.
//!
//! What is here is the half that needs a *face*: which face stands in is §9.5's NOTE 5 and this
//! machine's font catalogue. So nothing below names a glyph. **One descendant is read twice, once
//! under each identity `CMap`**, and what is asserted is which of its CIDs the writing mode
//! changes: the vertical forms and nothing else. Two readings of one dictionary differ in the
//! writing mode alone, which is trap 8's rule for a property no single document states.
//!
//! `LoadedFont::face_states_vertical_forms` is the skip condition, and it answers from the chosen
//! face's `GSUB` rather than from whether the route changed anything: a skip read off the output
//! of the thing under test is trap 13, and would have made this file green with the whole route
//! deleted. Calibrated that way — with `downward` forced to `None` the run does not skip, it
//! fails on the first pair.

#![expect(
    clippy::expect_used,
    reason = "test code: a witness that stops opening should fail loudly, naming itself"
)]

use std::path::Path;

use pdf_font::cmap::CMap;
use pdf_font::{Code, LoadedFont};
use pdf_syntax::{Dictionary, Document, Name, Object};

/// The pairs the witness's page shows: the character, the vertical CID the producer wrote, and
/// the horizontal CID the same character has in the same collection.
///
/// Every number is `UniJIS-UCS2-V`'s and `UniJIS-UCS2-H`'s for that character, and the first of
/// each pair is what the content stream contains.
const PAIRS: [(char, u16, u16); 4] = [
    ('\u{300c}', 7911, 686), // 「 LEFT CORNER BRACKET
    ('\u{300d}', 7912, 687), // 」 RIGHT CORNER BRACKET
    ('\u{3001}', 7887, 634), // 、 IDEOGRAPHIC COMMA
    ('\u{3002}', 7888, 635), // 。 IDEOGRAPHIC FULL STOP
];

/// Characters the collection gives one CID under either writing mode, so nothing may rotate
/// them: 縦 (2382), 書 (2427) and き (854), all three on the witness's first column.
const UNROTATED: [u16; 3] = [2382, 2427, 854];

/// The witness's Japanese font dictionary, or `None` when the corpus submodule is absent.
fn witness_font() -> Option<(Document, Dictionary)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/corpora/pdf-differences/VerticalText/VerticalText.pdf");
    let bytes = std::fs::read(&path).ok()?;
    let document = Document::open(bytes).expect("the witness opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the witness has a page one");
    let fonts = document.get_key(&page.resources, "Font");
    let fonts = fonts.as_dict().expect("the resources state fonts");
    let japanese = document
        .get_key(fonts, "Japanese")
        .as_dict()
        .expect("the page states a /Japanese font")
        .clone();
    Some((document, japanese))
}

/// The two-byte code an identity `CMap` makes of a CID (Table 116: `Identity-H` "maps 2-byte
/// character codes ranging from 0 to 65,535").
fn code(cid: u16) -> Code {
    CMap::identity().next_code(&cid.to_be_bytes())
}

#[test]
fn only_the_collections_vertical_forms_are_drawn_differently_downwards() {
    let Some((document, dict)) = witness_font() else {
        println!("skipped: doc/corpora/pdf-differences is not checked out");
        return;
    };
    let downward = LoadedFont::load(&document, &dict, "Japanese")
        .expect("a non-embedded Adobe-Japan1 CIDFontType0 is substituted rather than refused");
    assert!(
        downward.is_substituted(),
        "the witness embeds no program, so the face is this machine's"
    );
    if !downward.face_states_vertical_forms() {
        println!(
            "skipped: the face this machine chose for Adobe-Japan1 states no vert or vrt2 feature"
        );
        return;
    }

    // The same descendant, the same face, writing mode 0.
    let mut horizontal_dict = dict.clone();
    let _ = horizontal_dict.insert(
        Name::new(b"Encoding".to_vec()),
        Object::Name(Name::new(b"Identity-H".to_vec())),
    );
    let upright = LoadedFont::load(&document, &horizontal_dict, "Japanese")
        .expect("the same descendant under the horizontal identity CMap loads");

    for (character, vertical, horizontal) in PAIRS {
        assert_ne!(
            downward.glyph_index(code(vertical)),
            upright.glyph_index(code(vertical)),
            "CID {vertical} is {character:?}'s vertical form, so the writing mode decides it"
        );
        assert_eq!(
            downward.glyph_index(code(horizontal)),
            upright.glyph_index(code(horizontal)),
            "CID {horizontal} is {character:?}'s horizontal form, which the producer may write \
             in a vertical CMap and which nothing here may rotate"
        );
    }

    for cid in UNROTATED {
        assert_eq!(
            downward.glyph_index(code(cid)),
            upright.glyph_index(code(cid)),
            "CID {cid} is the same CID in both of the collection's Unicode CMaps, so it has no \
             vertical form to substitute"
        );
        assert!(
            downward.glyph_index(code(cid)).is_some(),
            "CID {cid} is on the witness's first column and the face draws it"
        );
    }
}

/// A vertical form the producer chose is drawn, or counted, or the character is missing entirely
/// — exactly one of the three, on any machine.
///
/// **This is the instrument's own calibration and it needs no skip**, which is what makes it
/// different from the test above. Which face stands in for a non-embedded Adobe-Japan1 font is
/// §9.5's NOTE 5 and this machine's catalogue, so *which* of the three arms fires is a fact about
/// the machine — but that exactly one fires is a fact about the code, and it is the whole of what
/// ADR 0764 built: a face with no vertical form and a face with no glyph at all were one silence
/// with one number under it, and they are now two.
///
/// The three, in the order the routes run:
///
/// 1. the face has no glyph for the character at all, which is `uncovered_character` and is
///    already counted (ADR 0152, `Interpretation::codes_without_a_glyph`'s neighbours);
/// 2. the face states the form, so the glyph the vertical reading reaches differs from the one
///    the horizontal reading of the same CID reaches;
/// 3. the face has the glyph and states no form for it, which is
///    `unsupplied_vertical_form` and `Shortfall::without_a_vertical_form`.
///
/// **Calibrated against the defect rather than assumed** (trap 13): with `VerticalForms::read`
/// made to return an empty map — a face with no `vert` or `vrt2`, which is what every Latin face
/// is — this machine's run moves from arm 2 to arm 3 for all four pairs and the count rises by
/// one per code the page shows. ADR 0764 records both runs.
#[test]
fn a_vertical_form_is_drawn_or_counted_and_never_both() {
    let Some((document, dict)) = witness_font() else {
        println!("skipped: doc/corpora/pdf-differences is not checked out");
        return;
    };
    let downward = LoadedFont::load(&document, &dict, "Japanese")
        .expect("a non-embedded Adobe-Japan1 CIDFontType0 is substituted rather than refused");
    let mut horizontal_dict = dict.clone();
    let _ = horizontal_dict.insert(
        Name::new(b"Encoding".to_vec()),
        Object::Name(Name::new(b"Identity-H".to_vec())),
    );
    let upright = LoadedFont::load(&document, &horizontal_dict, "Japanese")
        .expect("the same descendant under the horizontal identity CMap loads");

    for (character, vertical, _) in PAIRS {
        let code = code(vertical);
        let absent = downward.uncovered_character(code).is_some();
        let supplied = downward.glyph_index(code) != upright.glyph_index(code);
        let counted = downward.unsupplied_vertical_form(code).is_some();
        assert_eq!(
            usize::from(absent) + usize::from(supplied) + usize::from(counted),
            1,
            "CID {vertical} is {character:?}'s vertical form, and the face either cannot draw \
             the character ({absent}), draws the form ({supplied}), or draws it upright and is \
             counted for it ({counted}) — exactly one"
        );
        assert_eq!(
            downward.unsupplied_vertical_form(code),
            (!absent && !supplied).then_some(character),
            "the count names the character whose form was lost, and nothing else"
        );
    }

    // The other side of the condition, which is what stops the count from meaning "a vertical
    // page": a CID the collection gives one form has none to lose, whatever the face states.
    for cid in UNROTATED {
        assert_eq!(
            downward.unsupplied_vertical_form(code(cid)),
            None,
            "CID {cid} is the same CID in both of the collection's Unicode CMaps, so no form of \
             it was chosen and none can be missing"
        );
    }
}
