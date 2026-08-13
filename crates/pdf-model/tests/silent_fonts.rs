//! A font whose program draws nothing must say so.
//!
//! `Interpretation::is_complete` is the claim that the interpreter drew everything the page
//! asked for, and trap 1 is the standing warning about how little that can mean. This is the
//! narrowest version of it: a page of text whose font program answers **every** code with no
//! outline draws a blank page and, until the hundred-and-ninety-third session, reported
//! `unsupported: []` while doing it.
//!
//! `issue13316_reduced.pdf` is the witness — 200×50 points, one `Tj` of nine codes through an
//! embedded `TrueType` program, and `0 commands` — and the condition is the one ADR 0152 wrote
//! for a substituted face, applied where the code had been applying something else: **no code
//! reached an outline**. What keeps it from firing on ordinary pages is that a code reading
//! back as whitespace is not counted at all: a space is *meant* to be blank, and counting one
//! took the corpus's incomplete documents from 79 to 109.
//!
//! The tests are against real documents, which is trap 4's rule: a hand-built font program with
//! no outlines would be built by the same reading of the format the code under test uses.

#![expect(
    clippy::panic,
    reason = "test code: a document that stops opening should fail loudly, naming itself"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::Document;

/// Page one's interpretation, or `None` when the corpus submodule is not checked out.
fn page_one(name: &str) -> Option<pdf_model::Interpretation> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).unwrap_or_else(|e| panic!("{name} does not open: {e}"));
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .unwrap_or_else(|| panic!("{name} has no page one"));
    Some(pdf_model::interpret(&document, &page))
}

/// Every report the interpretation carries, as one string.
fn reports(interpretation: &pdf_model::Interpretation) -> String {
    format!("{:?}", interpretation.unsupported)
}

/// A page whose only text draws nothing says so, and names the font.
#[test]
fn a_font_that_draws_none_of_its_codes_is_reported() {
    let Some(interpretation) = page_one("issue13316_reduced.pdf") else {
        return;
    };
    assert!(
        interpretation.display_list.commands().is_empty(),
        "the page still draws nothing: that is the defect this reports, not one it fixes"
    );
    let said = reports(&interpretation);
    assert!(
        said.contains("/F1") && said.contains("no outline for any"),
        "a blank page of text must not be silent: {said}"
    );
}

/// And a page whose text draws normally says nothing about its fonts.
///
/// The discriminating half. `tracemonkey.pdf` is fourteen pages of dense embedded text with
/// spaces in every line — the exact shape that a report counting *blank* glyphs as missing
/// marks would fire on, and it fired on thirty such documents before the whitespace codes were
/// excluded.
#[test]
fn a_page_of_ordinary_text_reports_nothing_about_its_fonts() {
    let Some(interpretation) = page_one("tracemonkey.pdf") else {
        return;
    };
    let said = reports(&interpretation);
    assert!(
        !said.contains("no outline for any"),
        "a page that draws its text must not report: {said}"
    );
    assert!(interpretation.is_complete(), "and it is complete: {said}");
}

/// A code whose glyph the font *contains* and draws as empty is not a mark missed.
///
/// `pr12564.pdf` was the largest single contributor to the corpus's silent missing-glyph count
/// — 26 of 62 — and not one of them is a missing mark: every one is code 35 through `/TT3`,
/// whose glyph 1 the program contains and describes with no contours, which is how an sfnt
/// stores a space. What made it look like a loss is the `/ToUnicode`, which reads that code
/// back as `#`, so the whitespace exemption in front of the count could not see it — and
/// `pdftotext` renders the page as `1101#Strayer#Drive`, which is the same statement from
/// outside. §9.6.5.4's routes ended at a glyph; what that glyph draws is the program's to say.
#[test]
fn a_code_whose_font_contains_an_empty_glyph_is_counted_apart_from_a_missing_mark() {
    let Some(interpretation) = page_one("pr12564.pdf") else {
        return;
    };
    assert_eq!(
        (
            interpretation.codes_without_a_glyph,
            interpretation.codes_reaching_a_blank_glyph
        ),
        (0, 26),
        "all 26 reach a glyph the program describes as empty"
    );
    assert!(
        interpretation.is_complete(),
        "and the page is drawn: {}",
        reports(&interpretation)
    );
}

/// And a code that reaches no glyph, or `.notdef`, still counts as one.
///
/// `issue14821.pdf` is the corpus's other half of the same branch and both ends of it appear on
/// one page. Five of its codes are `Identity-H` CIDs whose `loca` entries are empty by the
/// glyph table's own statement — the program contains those glyphs and draws them as nothing.
/// Three are ASCII codes in a nonsymbolic `TrueType` subset whose `(3, 1)` `cmap` maps all
/// three to glyph 0 and whose `post` is version 3.0 with no names at all, so every route
/// §9.6.5.4 states ends at `.notdef`: those three are text the reader loses.
#[test]
fn a_code_that_reaches_notdef_is_still_a_mark_missed() {
    let Some(interpretation) = page_one("issue14821.pdf") else {
        return;
    };
    assert_eq!(
        (
            interpretation.codes_without_a_glyph,
            interpretation.codes_reaching_a_blank_glyph
        ),
        (3, 5),
        "three codes reach .notdef and five reach an empty glyph"
    );
}

/// A page that draws its text and can name none of it says how much it lost.
///
/// `french_diacritics.pdf` is the sharpest case in the corpus and it was refused deliberately and
/// **silently** until the four-hundred-and-seventy-sixth session. A pdfTeX Type 3 font whose
/// `/Differences` names the Latin-1 accented letters `/a192`, `/a194`, `/a196` …, which is the
/// character code in decimal and is the producer's own label: §9.10.2's first method has no
/// `/ToUnicode` to read, its second looks the name up in "the Adobe Glyph List and Adobe Glyph
/// List for New Fonts" and neither holds it, its third is for composite fonts, and the closing
/// permission is taken only for 0x21–0x7E — which is why the `1` at code 49 comes back and the
/// twenty-eight accented codes do not. `doc/todo/21` §5 has the reading; ADR 0311 has why the
/// clause reaches no further.
///
/// The page is **right**: all twenty-nine glyphs mark it, nothing is reported, and the picture
/// agrees with every reference. What was missing was any statement that a reader gets one
/// character of it.
#[test]
fn a_page_whose_codes_no_method_can_name_says_how_many() {
    let Some(interpretation) = page_one("french_diacritics.pdf") else {
        return;
    };
    assert_eq!(
        (
            interpretation.glyphs,
            interpretation.codes_without_a_character.total(),
            interpretation.text.trim()
        ),
        (29, 28, "1"),
        "the page draws every glyph and §9.10.2 names one of its twenty-nine codes"
    );
    assert!(
        interpretation.is_complete(),
        "and it is a *readback* refusal rather than a drawing one: {}",
        reports(&interpretation)
    );
}

/// The same count on a page that reads back nothing at all.
///
/// `complex_ttf_font.pdf` is the archetype the text gate has carried since the sixty-third
/// session — 527 glyphs on the page and nothing but the placement pass's own inferred breaks out
/// of it — and the discriminating half of the test above: a page with no text and a page whose
/// text nothing can name produce the same readback and are not the same page. This count is the
/// only thing that tells them apart, and the readback being *whitespace* rather than empty is
/// what makes it the sharper illustration: a rule that asked the buffer would call all 616 of
/// these spaces.
#[test]
fn a_page_that_reads_back_nothing_is_distinguishable_from_a_page_with_no_text() {
    let Some(interpretation) = page_one("complex_ttf_font.pdf") else {
        return;
    };
    assert!(
        interpretation.text.trim().is_empty() && interpretation.glyphs == 527,
        "527 glyphs and a readback of nothing but separators: {} glyphs, {:?}",
        interpretation.glyphs,
        interpretation.text
    );
    assert_eq!(
        interpretation.codes_without_a_character.total(),
        616,
        "and every code the page showed is one §9.10.2 could not name"
    );
}

/// A `ZapfDingbats` page reads back its dingbats, and none of its codes is left unnamed.
///
/// The other side of the two tests above, and the reason the census `doc/todo/21` §5 asked for was
/// worth taking: 114 of the corpus's 1342 unnamed codes were this font, whose glyph names the
/// Adobe Glyph List does not hold and whose characters ISO 32000-2 prints itself in Table D.6
/// (ADR 0318). **Both halves of the defect are here**, and only one of them was visible as a
/// count: codes 191 and up read back as nothing at all, and codes 0x21 to 0x7E read back as
/// *ASCII* — §9.10.2's closing permission taken for a font whose set the standard documents, so
/// the page's first dingbat came back as `!`.
///
/// The document is a specimen sheet, which is what makes it the witness: beside each dingbat it
/// prints, in Helvetica, the name and the Unicode value that dingbat has — so the page states the
/// expected answer next to the glyph, and the assertion below reads both out of one line. That
/// agreement is **evidence** and not the source: the table these characters come from is ISO
/// 32000-2's own Annex D.6, and a specimen sheet published in 2000 agreeing with it is what
/// `CLAUDE.md` principle 5 says such agreement is worth.
#[test]
fn a_dingbats_page_reads_back_the_characters_annex_d6_states() {
    let Some(interpretation) = page_one("ZapfDingbats.pdf") else {
        return;
    };
    assert_eq!(
        interpretation.codes_without_a_character.total(),
        0,
        "every code of the page is one Annex D.6 names"
    );
    for (dingbat, name, scalar) in [
        ('\u{2701}', "a1", 0x2701_u32),
        ('\u{2723}', "a30", 0x2723),
        ('\u{2792}', "a158", 0x2792),
    ] {
        let stated = format!("{dingbat} {name} [x{scalar:04X}]");
        assert!(
            interpretation.text.contains(&stated),
            "the page prints its own answer beside the glyph, and we read back {stated:?}: {:?}",
            interpretation.text.get(..400)
        );
    }
    assert!(
        !interpretation.text.contains("! a1 "),
        "and no dingbat reads back as the ASCII byte of its code"
    );
}
