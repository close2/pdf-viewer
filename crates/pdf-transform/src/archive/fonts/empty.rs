//! An empty glyph where a substituted font's own encoding would reference `.notdef`.
//!
//! ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator to
//! reference the `.notdef` glyph in any rendering mode. A code reaches `.notdef` by the document's
//! own encoding wherever the encoding in force names no glyph for it, or names `.notdef` itself:
//! ISO 32000-2 §9.6.5 maps a simple font's code to a glyph name and the name to a glyph, and a
//! code with no name, or the name `.notdef`, has nothing else to reach. A code the line feed is
//! written as, under `WinAnsiEncoding`, is the commonest case; a `/Differences` array writing
//! `.notdef` over a code the page then shows is the other.
//!
//! **Where the file embeds no program, the conversion is choosing the program, and so it chooses
//! what such a code reaches.** `doc/pdf-a-conversion-limits.md` section 2.2 is the reading and
//! `doc/questions/A48` the permission: the code is given a real glyph that draws nothing, so the
//! page shows exactly what this reader showed — nothing, at the width the dictionary states — and
//! no `.notdef` is referenced. A48's condition is the line this module holds: the glyph is named
//! outside the Adobe Glyph List, so the code is given no character it did not have (§9.10.2's
//! glyph-name method finds none), and the conversion is reported per font and recorded in the
//! file's own `xmpMM:History` naming the clause. **It is the absence kept, never filled in**: a
//! code this reader draws something for is not one of these, and mapping the code to a plausible
//! glyph — a space for a line feed — would give it a meaning nobody wrote.
//!
//! **Only a Compact Font Format face can carry one.** Under a `Type1` dictionary a code reaches a
//! glyph by its name alone, so any name the program holds will do. Under a non-symbolic `TrueType`
//! dictionary — where a `glyf` face goes (§9.9's Table 124) — a name reaches a glyph only through
//! the character the Adobe Glyph List gives it (§9.6.5.4), and ISO 19005-2 and ISO 19005-4 section
//! 6.2.11.6 / 6.2.10.6 admit no `/Differences` name outside that list, so every name that could
//! reach an empty glyph there states a character the file never stated. That half is refused, on
//! the fence's side, with the sentence saying so.

use std::collections::BTreeMap;

use pdf_archive::survey::SelectedFont;
use pdf_font::cff::AddedGlyph;
use pdf_font::substitute::Format;
use pdf_font::{Code, LoadedFont};

use super::super::decision::Because;

/// The shown codes the document's encoding sends to `.notdef`, which this reader draws nothing
/// for, each with the advance the dictionary states for it in thousandths of an em.
///
/// `face_glyph` is the glyph the face would select for the code, so that a code a face *does*
/// reach by the encoding is never taken for one of these.
pub(super) fn codes(
    source: &LoadedFont,
    used: &SelectedFont,
    face_glyph: impl Fn(Code) -> Option<u16>,
) -> BTreeMap<u8, f32> {
    let mut codes = BTreeMap::new();
    for text in used.shown.keys() {
        for code in source.decode(text) {
            let Ok(byte) = u8::try_from(code.value()) else {
                continue;
            };
            let reached = face_glyph(code).is_some_and(|glyph| glyph != pdf_font::NOTDEF_GLYPH);
            if reaches_notdef(source, code) && !reached {
                codes.insert(byte, source.advance(code) * 1000.0);
            }
        }
    }
    codes
}

/// Whether the document's own encoding sends `code` to `.notdef` and this reader draws nothing
/// for it: the encoding in force names no glyph for the code, or names `.notdef` itself, and the
/// font as loaded has no outline with a mark in it there.
pub(super) fn reaches_notdef(source: &LoadedFont, code: Code) -> bool {
    let unnamed = source
        .selected_glyph_name(code)
        .is_none_or(|name| name.is_empty() || name == ".notdef");
    let drawn = source
        .outline(code)
        .is_some_and(|outline| !outline.commands().is_empty());
    unnamed && !drawn
}

/// The face's program with one empty glyph per distinct advance, and the name each code reaches.
///
/// One glyph per *advance* rather than per code, because ISO 19005-2 section 6.2.11.5 holds the
/// program's advance for a glyph to the width the dictionary states for every code reaching it:
/// two codes stating different widths cannot share one.
pub(super) fn written(
    program: &[u8],
    format: Format,
    codes: &BTreeMap<u8, f32>,
) -> Result<(Vec<u8>, BTreeMap<u8, String>), Because> {
    if format != Format::BareCff {
        return Err(Because::TheFence(NOTDEF_UNDER_A_TRUETYPE_DICTIONARY));
    }
    let units = pdf_font::cff::units_per_em(program)
        .map_err(|_| Because::NotBuiltYet(EMPTY_GLYPH_NOT_WRITTEN))?;
    let mut by_advance: BTreeMap<i64, Vec<u8>> = BTreeMap::new();
    for (code, thousandths) in codes {
        let exact = thousandths * units / 1000.0;
        let whole = exact.round();
        // The tolerance both parts state between a dictionary's width and its program's.
        if (whole - exact).abs() / units > super::CONSISTENT {
            return Err(Because::NotBuiltYet(EMPTY_GLYPH_NOT_WRITTEN));
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "an advance in font units, rounded and checked within the tolerance above"
        )]
        by_advance.entry(whole as i64).or_default().push(*code);
    }
    let advances: Vec<i64> = by_advance.keys().copied().collect();
    let (rebuilt, added) = pdf_font::cff::with_empty_glyphs(program, &advances)
        .map_err(|_| Because::NotBuiltYet(EMPTY_GLYPH_NOT_WRITTEN))?;
    let mut names = BTreeMap::new();
    for (codes, AddedGlyph { name, .. }) in by_advance.values().zip(added) {
        for code in codes {
            names.insert(*code, name.clone());
        }
    }
    Ok((rebuilt, names))
}

/// Whether the empty glyph is offered at `target`: at 2b and at part 4, never at Level U or A.
///
/// ISO 19005-2 section 6.2.11.7.2 asks, at Levels U and A, that every glyph a file shows map to a
/// Unicode value, and the empty glyph is the absence kept: it states none, and stating one would be
/// the character the file never wrote. Part 4's section 6.2.10.7 recommends the mapping rather than
/// requiring it, which is `doc/pdf-a-conversion-limits.md` section 2.1's reading for the same gap.
pub(super) fn offered(target: pdf_archive::Target) -> bool {
    !matches!(
        target.level(),
        Some(pdf_archive::Level::U | pdf_archive::Level::A)
    )
}

/// The codes, as the report and the file's history name them.
pub(super) fn listed(codes: &[u8]) -> String {
    codes
        .iter()
        .map(|code| format!("{code:#04x}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Why a code reaching `.notdef` is not given an empty glyph where the face is a `glyf` one.
pub(super) const NOTDEF_UNDER_A_TRUETYPE_DICTIONARY: &str = "this document renders a font it \
     does not embed and shows a code its own encoding names no glyph for, or names .notdef, so the \
     code reaches the .notdef glyph in any program — which ISO 19005-2 section 6.2.11.8 and ISO \
     19005-4 section 6.2.10.9 forbid. The face this program ships for it is a glyf one, which goes \
     under a non-symbolic TrueType dictionary (ISO 32000-2 \u{a7}9.9's Table 124), where a name \
     reaches a glyph only through the character the Adobe Glyph List gives it (\u{a7}9.6.5.4) and \
     section 6.2.11.6 admits no other name: every name that could reach an empty glyph there \
     states a character the file never stated, which doc/questions/A48 forbids. A face named with \
     --font does not change this, because the code reaches .notdef in that program too";

/// Why the empty glyph is not given at Level U or A.
pub(super) const NO_CHARACTER_AT_THIS_LEVEL: &str = "this document renders a font it does not \
     embed and shows a code its own encoding names no glyph for, or names .notdef, so the code \
     reaches the .notdef glyph — which ISO 19005-2 section 6.2.11.8 forbids. doc/questions/A48 \
     allows a glyph that draws nothing in its place, but that glyph states no character, and \
     section 6.2.11.7.2 asks at Levels U and A that every glyph shown map to one: the file never \
     said which character the code is, so no conforming file at this level can be made without \
     inventing it. The same conversion at PDF/A-2b or PDF/A-4 gives the code its empty glyph";

/// Why the empty glyph could not be written into a Compact Font Format face.
pub(super) const EMPTY_GLYPH_NOT_WRITTEN: &str = "this document shows a code its own encoding \
     sends to .notdef, and the empty glyph doc/questions/A48 allows in its place could not be \
     written into the face this program ships at the width the font dictionary states for it";

/// Why a code reaching `.notdef` is refused where the font states its own descriptor.
pub(super) const NOTDEF_BESIDE_A_DESCRIPTOR: &str = "this document renders a font it does not \
     embed and shows a code its own encoding names no glyph for, or names .notdef, so the code \
     reaches the .notdef glyph in any program — which ISO 19005-2 section 6.2.11.8 and ISO \
     19005-4 section 6.2.10.9 forbid. doc/questions/A48 allows a real glyph that draws nothing in \
     its place, and this converter writes one for a standard font stated by name alone; for a \
     font that states its own FontDescriptor it is not built yet. A face named with --font does \
     not change this, because the code reaches .notdef in that program too";
