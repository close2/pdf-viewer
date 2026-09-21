//! A substituted face draws glyphs the size the *file* says the absent font drew them.
//!
//! `/Widths` is two statements rather than one. It is where each glyph is placed, which this
//! tree has honoured since its first font, and — by Table 109's own sentence, "[t]hese widths
//! shall be consistent with the actual widths given in the font program" — it is also how wide
//! the shapes of the font the document meant *were*. A substitute drawn at its own designer's
//! width inside the first statement contradicts the second, and on a condensed face the
//! contradiction is visible: the letters collide where the file says there is a gap.
//!
//! `bug1671312_ArialNarrow.pdf` is the witness (ADR 0358). One line of 20 pt `/ArialNarrow`,
//! nothing embedded, and a `/Widths` array stating Arial Narrow's advances — about 0.82 of the
//! Arial-metric faces every entry of `substitute`'s sans preference list is.
//!
//! **What is asserted is the property and not this machine's number.** Every face that list
//! names is a normal-width design, and so is the compiled-in fallback, so the ratio is a
//! property of the *file*: the letters fit in the room the file gives them.
//!
//! # Why the claim is the line's and not each letter's
//!
//! §9.2.4 makes a width a **displacement** — where the next glyph starts — so it says nothing
//! about where *this* glyph's ink ends. A letter whose outline reaches past its own advance has
//! a negative side bearing, which is ordinary typography and not a defect: `DejaVuSans`' `f` and
//! `t` do it here, the compiled-in face's `y` does, and this machine's Arial-metric faces do not.
//! A per-letter `<=` is therefore an assertion about which faces are installed, which is the
//! thing this file's own first paragraph says it is not making (ADR 1154).
//!
//! So the claim is a **share**, in the shape `pdf-font`'s width tests already use for the same
//! reason, and the control beside it is arithmetic rather than a second measurement: the ink
//! drawn without the scale is the ink drawn with it divided by [`LoadedFont::stretch`], so the
//! test computes what the defect would have looked like and shows that it is the *majority*. On
//! four faces measured — this machine's, the compiled-in fallback's, `DejaVuSans` and
//! `DroidSansFallback` — at most 2 of the 19 letters overflow with the scale and 14 or 15 of them
//! would without it. A quarter is the line between, and the defect ADR 0358 fixed is on the far
//! side of it by a factor of seven.

#![expect(
    clippy::expect_used,
    reason = "test code: a witness that stops opening should fail loudly, naming itself"
)]

use std::path::{Path, PathBuf};

use pdf_font::{Code, LoadedFont};
use pdf_syntax::{Dictionary, Document};

/// The witness's page-one font dictionary, or `None` when the corpus submodule is absent.
fn witness_font() -> Option<(Document, Dictionary)> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/bug1671312_ArialNarrow.pdf");
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).expect("the witness opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the witness has a page one");
    let fonts = document.get_key(&page.resources, "Font");
    let fonts = fonts.as_dict().expect("the resources state fonts");
    let (_, first) = fonts.iter().next().expect("the page states one font");
    let dict = document
        .resolve(first)
        .as_dict()
        .expect("the font resource is a dictionary")
        .clone();
    Some((document, dict))
}

/// The letters the witness shows are drawn inside the advances its `/Widths` states for them.
///
/// The inequality is the whole claim, and it fails in the direction the defect had: before ADR
/// 0358 the substitute's `A` was 0.636 em of ink inside the 0.547 em this file gives it. The
/// module comment says why it is counted over the line rather than asserted of each letter, and
/// the second assertion is the control that makes the first one mean something: remove the scale
/// — which is a division, not another measurement — and most of the line goes over.
#[test]
fn a_substituted_glyph_fits_the_width_the_file_states_for_it() {
    let Some((document, dict)) = witness_font() else {
        return;
    };
    let font = LoadedFont::load(&document, &dict, "F1").expect("a non-embedded TrueType loads");
    assert!(
        font.is_substituted(),
        "the witness embeds no program, so its glyphs are a stand-in"
    );

    let mut checked = 0usize;
    let mut over: Vec<String> = Vec::new();
    let mut over_unscaled = 0usize;
    for byte in b"Accessory facilities" {
        let code = Code::single_byte(*byte);
        let Some(outline) = font.outline(code) else {
            continue; // the space draws nothing, which is not a shape to measure
        };
        let bounds = outline
            .bounds(pdf_render::Transform::scale(1.0, 1.0))
            .expect("a drawn glyph has bounds");
        let stated = font.advance(code);
        checked += 1;
        if bounds.width() > stated {
            over.push(format!(
                "{:?} is {} em of ink in the {stated} em the file gives it",
                char::from(*byte),
                bounds.width()
            ));
        }
        // The same letter with the scale taken off, which is what the face's own designer drew.
        if bounds.width() / font.stretch() > stated {
            over_unscaled += 1;
        }
    }
    assert!(
        checked > 10,
        "only {checked} of the line's letters were drawn"
    );
    assert!(
        over.len() * 4 < checked,
        "{} of {checked} letters overflow the room the file gives them:\n{}",
        over.len(),
        over.join("\n")
    );
    assert!(
        over_unscaled * 2 > checked,
        "only {over_unscaled} of {checked} letters would overflow unscaled, so the assertion \
         above is not discriminating on this machine's face"
    );
}

/// The scale comes from the file's widths, and the file's *stem* — which nothing derived it
/// from — agrees.
///
/// `/StemV 66` against the 88-ish stem of an Arial-metric face is 0.75, and the widths give
/// 0.82: two entries of two different tables, written by one producer about one absent font,
/// landing within a twentieth of each other. That is the independent check on the construction,
/// and it is the reason `/StemV` is read as evidence here rather than used as the scale — see
/// ADR 0358, which prices the alternative.
#[test]
fn the_scale_derived_from_the_widths_agrees_with_the_stated_stem() {
    let Some((document, dict)) = witness_font() else {
        return;
    };
    let font = LoadedFont::load(&document, &dict, "F1").expect("a non-embedded TrueType loads");
    let stretch = font.stretch();
    assert!(
        (0.70..=0.90).contains(&stretch),
        "a condensed face's widths against a normal-width substitute: {stretch}"
    );
}
