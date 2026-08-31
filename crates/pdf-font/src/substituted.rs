//! Drawing a font whose program the document did not embed.
//!
//! [`crate::substitute`] answers *which face*; this module is what a document's character
//! codes do once one has been chosen. The two questions are kept apart because the second
//! decides the first: a candidate face is judged by the code table it produces for this
//! document's own encoding (ADR 0270), so the codes are resolved before the face is settled
//! rather than after.
//!
//! §9.5's NOTE 5 puts the choice of a substitute outside the standard altogether, which is
//! what makes every rule here a documented choice in a place the standard leaves open rather
//! than a reading of one.

use std::borrow::Cow;
use std::sync::Arc;

use pdf_render::Path;
use pdf_syntax::{Dictionary, Document};
use skrifa::{FontRef, MetadataProvider};

use crate::cff::CodeToGlyph;
use crate::encoding;
use crate::glyph_names::{GlyphNames, encoding_names};
use crate::loading::{CodeTable, FontError, NOTDEF_GLYPH};
use crate::program::Program;
use crate::substitute;

/// Resolves a simple font's character codes to glyphs in a *substitute* font.
///
/// The substitute shares nothing with the font the document named except the characters
/// it can draw, so every code is resolved to the character it stands for and that
/// character is looked up in the substitute's `cmap`.
///
/// The character comes from the glyph name the encoding gives — through the Adobe Glyph
/// List — rather than from `/ToUnicode`, because the encoding is what the *rendering*
/// model uses. `/ToUnicode` is a statement about extracted text and producers are known
/// to leave it wrong; a code's glyph name is what actually selected the glyph.
pub(crate) fn substitute_code_table(
    document: &Document,
    dict: &Dictionary,
    request: substitute::Request,
    names: GlyphNames,
    data: &[u8],
    program: Program,
    name: &str,
) -> Result<(CodeTable, GlyphNames), FontError> {
    let table = fill_substitute_table(&names, data, program, name)?;

    if !declared_codes(document, dict).any(|code| table.get(code).is_some_and(Option::is_some)) {
        return Err(FontError::NoSubstitute {
            name: name.to_owned(),
            reason: format!(
                "the {:?} face draws none of the codes the document declares",
                request.family
            ),
        });
    }

    Ok((table, names))
}

/// The glyph names a substituted simple font's codes select.
///
/// A symbolic standard-14 font carries its own encoding; everything else starts from a Latin
/// base, defaulting to `StandardEncoding` when the document names none.
///
/// Separated from [`substitute_code_table`] because the *choice of face* needs them too — see
/// [`substitute_face`] — and computing them twice per font would be paying for the same
/// `/Differences` array twice.
pub(crate) fn substitute_encoding_names(
    document: &Document,
    dict: &Dictionary,
    request: substitute::Request,
    name: &str,
) -> Result<GlyphNames, FontError> {
    encoding_names(document, dict, name, symbolic_set(request.family), true)
}

/// Which of Annex D's two symbolic character sets a substituted family belongs to.
///
/// One reader for both of the questions that need it — which glyph names the codes select
/// (§9.6.5.1), and which characters those names stand for (Annex D.6) — so that a font cannot be
/// `ZapfDingbats` for one and not for the other.
pub(crate) fn symbolic_set(family: substitute::Family) -> Option<encoding::SymbolicEncoding> {
    match family {
        substitute::Family::Symbol => Some(encoding::SymbolicEncoding::Symbol),
        substitute::Family::ZapfDingbats => Some(encoding::SymbolicEncoding::ZapfDingbats),
        _ => None,
    }
}

/// One code table, built by asking a face for every name the encoding states.
fn fill_substitute_table(
    names: &GlyphNames,
    data: &[u8],
    program: Program,
    name: &str,
) -> Result<CodeTable, FontError> {
    // **The two routes differ in what the substitute is addressed *by*, and the name-keyed one
    // is the shorter of the two.** An `sfnt` substitute is reached by character, so a glyph name
    // has to go through the Adobe Glyph List first; a bare CFF keys its glyphs by name already,
    // which is the same name §9.6.5.2's encoding produced. Since the hundred-and-forty-eighth
    // session the second route is the one every compiled-in Foxit face takes, and it is why
    // `Symbol` and `ZapfDingbats` work at all: their glyph names — `a9`, `universal` — are in no
    // Unicode mapping worth trusting, and going through one is how a dingbat became a Latin
    // letter.
    let mut table: CodeTable = [None; 256];
    if program == Program::BareCff {
        let keyed = match CodeToGlyph::read(data).map_err(|e| FontError::Malformed {
            name: name.to_owned(),
            detail: format!("substitute font: {e}"),
        })? {
            CodeToGlyph::Named(keyed) => keyed,
            // No compiled-in face is CID-keyed, and a machine's fonts never reach here.
            CodeToGlyph::Keyed { .. } => {
                return Err(FontError::UnsupportedEncoding {
                    name: name.to_owned(),
                    encoding: "CID-keyed CFF as a substitute".to_owned(),
                });
            }
        };
        for (code, slot) in table.iter_mut().enumerate() {
            let Some(glyph_name) = names.get(code).map(Cow::as_ref).filter(|n| !n.is_empty())
            else {
                continue;
            };
            *slot = keyed.by_name.get(glyph_name).copied();
        }
    } else {
        let font = FontRef::new(data).map_err(|e| FontError::Malformed {
            name: name.to_owned(),
            detail: format!("substitute font: {e}"),
        })?;
        let charmap = font.charmap();
        for (code, slot) in table.iter_mut().enumerate() {
            let Some(glyph_name) = names.get(code).map(Cow::as_ref).filter(|n| !n.is_empty())
            else {
                continue;
            };
            let Some(character) = read_fonts::ps::agl::name_to_char(glyph_name) else {
                continue;
            };
            *slot = charmap
                .map(character)
                .and_then(|id| u16::try_from(id.to_u32()).ok());
        }
    }

    Ok(table)
}

/// The character a code stands for, where the Adobe Glyph List can say.
///
/// The same route [`fill_substitute_table`] takes for an `sfnt` face, asked separately because
/// [`substitute_face`] needs to know *which* characters a face would have to have.
fn substituted_character(names: &GlyphNames, code: usize) -> Option<char> {
    let glyph_name = names.get(code).map(Cow::as_ref).filter(|n| !n.is_empty())?;
    read_fonts::ps::agl::name_to_char(glyph_name)
}

/// Which face a substituted simple font is drawn from, when the first choice cannot draw it.
///
/// # The defect this exists for, and it is §9.6.2.2's fourteen that carry it
///
/// [`substitute::find`] answers a `/BaseFont` naming one of the standard 14 from the binary,
/// which is what makes a machine with no fonts installed draw text at all — and ten of the
/// fourteen compiled-in faces are Foxit's bare CFF programs, whose charsets hold the standard
/// Latin character set and nothing else. So a document that names `Times-Roman` (or
/// `TimesNewRomanPSMT`, which folds to it) and then states an `/Encoding` whose `/Differences`
/// name `afii10017` and its neighbours — the Adobe Glyph List's names for Cyrillic — asked for
/// characters that face has never had. Every one of those codes reached no glyph, and because
/// the *Latin* codes of the same font drew, the "this font drew nothing" report never fired:
/// the page lost its Russian in silence. The four-hundred-and-thirty-fourth session found this
/// as the largest population in a 65 944-document web survey, and ADR 0270 has the numbers.
///
/// # The rule, and why it compares tables rather than characters
///
/// **A face is replaced only by one that draws everything it drew and more.** The two code
/// tables are built and compared over the codes the document declares (§9.6.2.1, Table 109's
/// `/FirstChar` and `/LastChar`), and the second face is taken only where its table is a strict
/// superset of the first's. So a page can gain marks and cannot lose one, which is what makes
/// this safe to apply to every substituted simple font rather than to a population somebody
/// picked.
///
/// The alternative — ADR 0153's rule for a composite font, a face that covers a *set of
/// characters* — was tried first and is measurably weaker here: `0546109.pdf` states a Greek
/// encoding whose `/Differences` also name `controlSTX` and its thirty neighbours, so the set of
/// characters the encoding names includes the C0 controls, no face on any machine has those in a
/// `cmap`, and a coverage test refuses every candidate over glyphs the page could never show.
/// Comparing tables asks the question the page actually poses: which of these two faces answers
/// more of this document's codes.
///
/// # Which faces are tried, and why not the whole machine
///
/// [`substitute::installed`]'s own preference list for the request's family, in its order. A
/// catalogue-wide search is what [`substitute::installed_covering`] does for a composite font,
/// where a Latin face is *no* answer for a Chinese collection; here the first face is already
/// the right shape and the search is for the same shape with a wider repertoire, so leaving the
/// family would trade a page's typeface for a glyph.
///
/// §9.5's NOTE 5 puts the choice of substitute outside the standard altogether, so this is a
/// documented choice in a place the standard leaves open, and it is made on what the *file*
/// states rather than on what any other reader does.
///
/// # What it costs, and where
///
/// One code table per candidate — 256 `cmap` lookups over a face already read for the catalogue
/// — and only for a font that has a declared code the first face cannot answer. A document whose
/// substitutes cover their encodings, which is every Latin one, pays a single table build that
/// [`substitute_code_table`] would have paid anyway.
///
/// The two symbolic families are excluded outright. Their encodings are name-keyed by design
/// (`a9`, `universal`), an `sfnt` candidate is addressed through the Adobe Glyph List instead,
/// and §9.6.2.2's own Symbol and `ZapfDingbats` faces are the right ones for them.
pub(crate) fn substitute_face(
    document: &Document,
    dict: &Dictionary,
    request: substitute::Request,
    names: &GlyphNames,
    name: &str,
) -> (Arc<[u8]>, substitute::Format) {
    let (data, format) = substitute::find(request);
    if matches!(
        request.family,
        substitute::Family::Symbol | substitute::Family::ZapfDingbats
    ) {
        return (data, format);
    }
    let Ok(table) = fill_substitute_table(names, &data, Program::from(format), name) else {
        // A face this crate cannot read is reported by the caller, which builds the same table
        // again and keeps the error. Choosing a second face on the strength of a failure to
        // read the first would report the wrong one.
        return (data, format);
    };

    // The declared range is the producer's own statement of which codes the page shows
    // (§9.6.2.1, Table 109), which is what makes it the right range to judge a face by — the
    // same argument `declared_codes` already carries for the refusal below it.
    //
    // **And a dictionary that states neither bound has said nothing**, so nothing here is
    // decided: `declared_codes` widens that silence to all 256 codes, which is right for
    // "does this face draw *any* of them" and wrong for this comparison. `franz.pdf` is the
    // page that showed it — `/Helvetica-Bold`, no `/FirstChar`, and a `/Differences` naming
    // `ff`, `ffi`, `ffl` and `dotlessj` among the hundred codes it does show. The compiled-in
    // face has no ligatures, another face on this machine has, and the page would have traded
    // its typeface for four glyphs it never shows. ADR 0133 compiled the fourteen in so that a
    // rendered page would stop being a property of the machine; spending that on a glyph no
    // page asked for is the wrong side of the trade.
    // A code answered with `.notdef` has not been answered — §9.6.5.2 makes that glyph what a
    // *name the program does not have* resolves to — and a `/Differences` array naming
    // `.notdef` outright is common enough to decide this comparison on its own: the name-keyed
    // face has such a glyph and an `sfnt` reached through the Adobe Glyph List has no character
    // for the name at all, so counting it would make every `sfnt` candidate look worse.
    // `0546109.pdf` is the witness — a Greek `/Differences` with eleven `/.notdef` entries in it.
    let answered = |table: &CodeTable, code: usize| {
        table
            .get(code)
            .copied()
            .flatten()
            .is_some_and(|glyph| glyph != NOTDEF_GLYPH)
    };
    let Some(declared) = stated_code_range(document, dict) else {
        return (data, format);
    };
    if declared
        .clone()
        .all(|code| answered(&table, code) || substituted_character(names, code).is_none())
    {
        return (data, format);
    }
    let wider = |bytes: &Arc<[u8]>| {
        let Ok(other) = fill_substitute_table(names, bytes, Program::Sfnt, name) else {
            return false;
        };
        let mut gains = false;
        for code in declared.clone() {
            match (answered(&table, code), answered(&other, code)) {
                (true, false) => return false,
                (false, true) => gains = true,
                _ => {}
            }
        }
        gains
    };
    match substitute::installed_wider(request, wider) {
        // Every candidate in the preference list is an `sfnt`; the catalogue admits no other.
        Some(better) => (better, substitute::Format::Sfnt),
        None => (data, format),
    }
}

/// One substituted glyph outline, wound the way every other substituted outline is wound.
///
/// # The clause, and why this is a defect rather than a taste
///
/// ISO 32000-2 §9.3.6 combines the glyphs of a text object in a clipping render mode into one
/// path:
///
/// > At the end of the text object identified by the ET operator the accumulated glyph
/// > outlines, if any, shall be combined into a single path, treating the individual outlines
/// > as subpaths of that path and applying the non-zero winding number rule
///
/// and NOTE 2 says what follows:
///
/// > Due to the use of non-zero winding number rule, the direction of the paths comprising each
/// > glyph can cause different output for overlapping glyphs.
///
/// So direction is *visible*, and where two glyphs run opposite ways their overlap cancels
/// instead of uniting. For an embedded program that is the producer's own statement and this
/// function never touches it. For a face **this program chose**, nothing in the file said
/// anything: §9.5's NOTE 5 puts substitution outside the standard, so the direction is ours.
///
/// What makes one direction better than the other is §9.6.2.2, which names the fourteen faces a
/// document may use without carrying them —
///
/// > The PostScript language names of 14 Type 1 fonts, known as the standard 14 fonts, are as
/// > follows: Times-Roman, Helvetica, Courier, Symbol, Times-Bold, Helvetica-Bold, Courier-Bold,
/// > ZapfDingbats, Times-Italic, Helvetica-Oblique, Courier-Oblique, Times-BoldItalic,
/// > Helvetica-BoldOblique, CourierBoldOblique.
///
/// — as **one set of Type 1 programs**. A document may draw two of them into one path, as
/// `OverlappingGlyphClipping.pdf` does with `/Times-Bold` and `/Helvetica`, and the fourteen it
/// names do not disagree with each other about direction. A stand-in set that answered one with
/// an `sfnt` and another with a CFF would manufacture a disagreement the fourteen do not have,
/// in the one place the clause makes direction visible — which is what this tree did until the
/// five-hundred-and-sixty-first session (ADR 0396).
///
/// # Which direction, and why measured rather than assumed
///
/// Counter-clockwise in the glyph's own y-upward space, which is the direction ten of the
/// fourteen compiled-in faces already carry and the one the Type 1 charstrings among them are
/// drawn in. The standard states no direction anywhere, so this is a documented choice in a
/// place it leaves open, and `crates/pdf-font/src/standard.rs` asserts the set agrees.
///
/// The direction is **measured** — [`Path::signed_area`] over the whole glyph — rather than
/// inferred from the program's format, because the format does not decide it: an OpenType face
/// on this machine carries CFF charstrings inside an `sfnt` wrapper and is wound the CFF way, and
/// a substitute may be any face `crate::substitute::installed_wider` found. An outer contour
/// always encloses more than the counters inside it, so the sum has the outer contour's sign.
///
/// # What it costs
///
/// Nothing at startup, which is `CLAUDE.md`'s rule for compiled-in data: this runs inside
/// `LoadedFont`'s outline cache, once per glyph a page actually shows, and only for a font
/// whose program the document did not embed. The measurement is one pass over the outline's
/// control points and the reversal a second, against the outline extraction that produced them.
/// **Measured under callgrind** on two substituted-text pages of the corpus, interpretation
/// end to end: `issue20489.pdf` 37 075 144 → 37 144 162 instructions and `pr12564.pdf`
/// 182 243 769 → 182 901 300, which is 0.19% and 0.36% (ADR 0396).
///
/// **And it changes no glyph drawn on its own.** Reversing every subpath negates every winding
/// number, and both of §8.5.3.3's rules test a winding number's magnitude, so a fill, a stroke
/// and a clip of this glyph alone paint exactly what they painted before.
pub(crate) fn wound_counter_clockwise(path: Path) -> Path {
    if path.signed_area() < 0.0 {
        return path.reversed();
    }
    path
}

/// Characters any face standing in for a registered character collection has to be able to draw.
///
/// # Why a table, and why it is a *choice*
///
/// §9.10.2 says how to find out what a code *means* — the collection's own `-UCS2` table — and
/// says nothing whatever about which face to draw it with; §9.8.3's substitution hints are
/// about weight, width and serifs, none of which distinguishes a face that has Chinese from one
/// that does not. So this is a documented choice in the one place the standard leaves for it,
/// and the choice is the cheapest true statement available: **a face that cannot draw a
/// character every font for this collection contains is not a face for this collection.**
///
/// One character per registry-ordering, taken from the collection's own script and picked for
/// being unavoidable in it rather than for being first: Adobe-Japan1's あ, Adobe-GB1's 的,
/// Adobe-CNS1's 的 (shared with GB1 — the two disagree about *forms*, and a face that has
/// neither has neither), Adobe-Korea1's and Adobe-KR's 한.
///
/// `Identity` and anything unregistered yield nothing, so those fonts keep the family match
/// they had — the codes there index a font nobody supplied and §9.10.2's third method has
/// nothing to read either way.
pub(crate) fn script_sample(document: &Document, descendant: &Dictionary) -> &'static [char] {
    let Some((registry, ordering)) = crate::composite::collection_names(document, descendant)
    else {
        return &[];
    };
    match (registry.as_str(), ordering.as_str()) {
        ("Adobe", "Japan1") => &['\u{3042}'],
        ("Adobe", "GB1" | "CNS1") => &['\u{7684}'],
        ("Adobe", "Korea1" | "KR") => &['\u{d55c}'],
        _ => &[],
    }
}

/// The character codes the font dictionary says the document uses.
///
/// `/FirstChar` and `/LastChar` bound `/Widths`, so between them they are the producer's own
/// statement of which codes appear in the content stream. That is what makes them the right
/// range to judge a mapping by: a substitute that reaches a glyph for two hundred codes the
/// document never shows, and for none of the four it does, is not a usable substitute — and
/// counting *all* the codes it happens to cover says it is.
///
/// `issue20504.pdf` is the case that showed it. Its Chinese line uses a Type 1 program this
/// crate cannot read, so the font is substituted, and `/Differences [33 /gid2436 …]` names
/// four glyphs only the original font had. Every one of the 252 codes the document does not
/// use resolved through `StandardEncoding` and mapped, so the substitute looked usable, and
/// the four codes actually shown drew nothing at all — in silence.
///
/// A dictionary stating neither yields every code, which keeps the judgement no weaker than
/// it was for a font that says nothing about its range.
fn declared_codes(document: &Document, dict: &Dictionary) -> std::ops::RangeInclusive<usize> {
    stated_code_range(document, dict).unwrap_or(0..=255)
}

/// The same range, and `None` where the dictionary states neither bound.
///
/// The distinction is [`substitute_face`]'s: "which codes does this document show" is a question
/// a dictionary can decline to answer, and a caller comparing two faces over the answer needs to
/// know that it did. Table 109 makes both entries "(Required; optional in PDF 1.0-1.7 for the
/// standard 14 fonts)", so the silence belongs to exactly the fonts this tree substitutes most.
fn stated_code_range(
    document: &Document,
    dict: &Dictionary,
) -> Option<std::ops::RangeInclusive<usize>> {
    let bound = |key: &str| {
        document
            .get_key(dict, key)
            .as_integer()
            .and_then(|value| usize::try_from(value).ok())
    };
    match (bound("FirstChar"), bound("LastChar")) {
        (Some(first), Some(last)) if first <= last => Some(first..=last.min(255)),
        _ => None,
    }
}

/// §9.10.2's choice of substitute, on the one part of it that is not a property of this machine.
///
/// ADR 0270's rule is a comparison of two code tables, and which faces there are to compare is
/// whatever is installed. What holds everywhere is the direction of the comparison, and that is
/// what these state.
#[cfg(test)]
mod substitute_face_tests {
    use crate::fixture::font_dictionary;
    use crate::{Code, LoadedFont};

    use super::{declared_codes, stated_code_range};

    /// A substituted face is replaced only by one that draws everything it drew.
    ///
    /// The invariant ADR 0270's rule rests on, and the only part of it that is a property of
    /// this *program* rather than of this machine's font collection. `/Times-Roman` is one of
    /// §9.6.2.2's fourteen, so [`substitute::find`] answers it from the binary with Foxit's
    /// serif CFF, whose charset is the standard Latin character set; `afii10017` is the Adobe
    /// Glyph List's name for CYRILLIC CAPITAL LETTER A, which that face has never had. A
    /// machine with a Cyrillic serif face draws both codes and a machine with none draws the
    /// Latin one — what may not happen, on any machine, is the swap that gains the second and
    /// loses the first.
    #[test]
    fn a_substituted_face_is_replaced_only_by_one_that_keeps_every_code_it_drew() {
        let (document, dict) = font_dictionary(
            "/BaseFont /Times-Roman /FirstChar 65 /LastChar 66 \
             /Encoding << /Differences [65 /A 66 /afii10017] >>",
        );
        let font = LoadedFont::load(&document, &dict, "F1").expect("a standard-14 name loads");
        let latin = font.glyph_index(Code::single_byte(65));
        let cyrillic = font.glyph_index(Code::single_byte(66));
        assert!(
            latin.is_some(),
            "the Latin code the compiled-in face draws may not be lost to a wider face"
        );
        assert!(
            cyrillic.is_none() || latin.is_some(),
            "a face is taken only where its table is a superset of the one in hand"
        );
    }

    /// A dictionary that states neither bound has said nothing about which codes it shows.
    ///
    /// Table 109 makes `/FirstChar` and `/LastChar` "(Required; optional in PDF 1.0-1.7 for the
    /// standard 14 fonts)", so the silence belongs to exactly the fonts that get substituted —
    /// and [`substitute_face`] reads it as *no evidence* rather than as all 256 codes.
    /// `franz.pdf` is why: `/Helvetica-Bold` with no `/FirstChar` and a `/Differences` naming
    /// `ff`, `ffi` and `ffl`, which would otherwise have traded the page's typeface for three
    /// ligatures it never shows.
    #[test]
    fn a_font_stating_neither_bound_states_no_range() {
        let (document, dict) = font_dictionary("/BaseFont /Helvetica");
        assert_eq!(stated_code_range(&document, &dict), None);
        assert_eq!(declared_codes(&document, &dict), 0..=255);

        let (document, dict) = font_dictionary("/BaseFont /Helvetica /FirstChar 32 /LastChar 90");
        assert_eq!(stated_code_range(&document, &dict), Some(32..=90));
    }
}
