//! What a *name-keyed* font program offers a simple font's character codes.
//!
//! ISO 32000-2 §9.6.5.2 describes one algorithm for two file formats, and §9.6.2.1's NOTE 1
//! is why it can: a CFF is "an alternative, more compact but functionally equivalent
//! representation of a Type 1 font program". Both key their glyph descriptions by *name*,
//! both carry a built-in encoding from codes to those names, and the clause's rules —
//! `/Differences` over a base encoding, the base being the program's own when the program is
//! embedded — are stated once for both.
//!
//! So this is the shape `cff.rs` and `type1.rs` both produce and [`simple_code_table`]
//! consumes. Neither reader appears in it: what a Type 1 program's eexec encryption or a
//! CFF's INDEX structures look like is their own business, and the clause's business is only
//! that a code has a name and a name has a glyph.

use std::borrow::Cow;
use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document};

use crate::glyph_names::{GlyphNames, encoding_names};
use crate::loading::{CodeTable, FontError};

/// A name-keyed program's own statements about its glyphs and its codes.
#[derive(Debug)]
pub struct NameKeyed {
    /// Glyph index by glyph name, taken from the program's charset.
    pub by_name: BTreeMap<Box<str>, u16>,
    /// Glyph index by character code, taken from the encoding the program itself carries.
    ///
    /// §9.6.5.1's Table 112 makes this the *base* encoding whenever the font program is
    /// embedded and the `/Encoding` dictionary names no `/BaseEncoding`, so it is what a
    /// `/Differences` array describes differences from rather than only a fallback for a
    /// code nothing else reached.
    pub builtin: Box<[Option<u16>; 256]>,
    /// The glyph name the built-in encoding gives each character code.
    ///
    /// The same mapping as [`Self::builtin`], carried through the charset instead of
    /// stopping at the glyph index. Nothing about *drawing* needs it — `builtin` selects the
    /// glyph directly — but a code's glyph name is what a document with no `/ToUnicode`
    /// means by that code, so text extraction and [`crate::LoadedFont::code_for`] would
    /// otherwise lose every code the PDF encoding left to the program.
    pub builtin_names: Box<[Option<Box<str>>; 256]>,
}

impl NameKeyed {
    /// Builds the mapping from a program's charset and its built-in encoding.
    ///
    /// `by_glyph` is the charset: one name per glyph, in glyph order. Inverting it here
    /// rather than in each reader is what keeps the tie-break in one place — a name two
    /// glyphs share resolves to the *lower* glyph, matching the order a charset assigns
    /// them, and a `BTreeMap`'s iteration order is what makes that true rather than a
    /// comment claiming it.
    #[must_use]
    pub fn new(by_glyph: &BTreeMap<u16, Box<str>>, builtin: Box<[Option<u16>; 256]>) -> Self {
        let mut by_name = BTreeMap::new();
        for (glyph, name) in by_glyph {
            by_name.entry(name.clone()).or_insert(*glyph);
        }
        let builtin_names = Box::new(std::array::from_fn(|code| {
            builtin
                .get(code)
                .copied()
                .flatten()
                .and_then(|glyph| by_glyph.get(&glyph).cloned())
        }));
        Self {
            by_name,
            builtin,
            builtin_names,
        }
    }
}

/// Resolves a simple font's character codes to glyphs in a bare CFF program.
///
/// This is the specification's encoding algorithm for a font whose glyphs are keyed by
/// name (ISO 32000-2 §9.6.5.2): choose a base encoding, layer `/Differences` over it, and
/// resolve the resulting glyph names through the font's charset.
///
/// # The base encoding of an embedded program is the program's own
///
/// §9.6.5.1's Table 112 states the default in three cases, and only the last two turn on
/// the Symbolic flag:
///
/// > For a font program that is embedded in the PDF file, the default base encoding shall
/// > be the font program's built-in encoding, as described in 9.6.5, "Character encoding"
/// > and further elaborated in the subclauses on specific font types.
///
/// A bare CFF only reaches this function by having been embedded, so the first sentence
/// decides every font that gets here and the flag decides none of them. An earlier version
/// asked the flag anyway and gave a nonsymbolic font `StandardEncoding`, which is the rule
/// for a font this crate would be *substituting* — so a code the document left to the
/// program drew whatever `StandardEncoding` puts there instead of what the program does.
/// It is invisible on almost every document, because a CFF's built-in encoding usually
/// *is* `StandardEncoding`; over the whole corpus it moves one code of one font.
///
/// # Why an unresolved name is not retried against the font's own encoding
///
/// When the encoding names a glyph the font does not have, this leaves the code with no
/// glyph rather than falling back to whatever the font's built-in encoding puts there.
/// The fallback is tempting because it fills the page, and wrong because a subset font's
/// built-in encoding is arbitrary: it would draw *a* glyph, confidently, and not the one
/// the document asked for. A blank is a visible defect; a wrong letter is an invisible
/// one. That is not in tension with the rule above: the built-in encoding is the base
/// *before* `/Differences` renames a code, never a second chance after a name failed.
pub(crate) fn simple_code_table(
    document: &Document,
    dict: &Dictionary,
    program: &NameKeyed,
    name: &str,
) -> Result<(CodeTable, GlyphNames), FontError> {
    let NameKeyed {
        by_name,
        builtin,
        builtin_names,
    } = program;

    // No fall-back to StandardEncoding: an unnamed code belongs to the program's own
    // encoding, which is what Table 112 makes the base here.
    let mut names = encoding_names(document, dict, name, None, false)?;

    let mut table: CodeTable = [None; 256];
    for (code, slot) in table.iter_mut().enumerate() {
        match names.get(code).map(Cow::as_ref).filter(|n| !n.is_empty()) {
            // The encoding named a glyph. If the font does not have it, the code has no
            // glyph, and that is final — see the note above this function.
            Some(glyph_name) => *slot = by_name.get(glyph_name).copied(),
            // The encoding said nothing, so the font's own encoding applies.
            None => *slot = builtin.get(code).copied().flatten(),
        }
    }

    // Every code the base encoding answered is now drawn by the right glyph and named by
    // nothing, and a font with no `/ToUnicode` has only the name to say what a code means.
    // Taken from the charset rather than from a Latin table, so the name is the program's
    // own statement about the glyph the code just selected.
    for (code, slot) in names.iter_mut().enumerate() {
        if let Some(builtin_name) = builtin_names
            .get(code)
            .and_then(Option::as_deref)
            .filter(|_| slot.is_empty())
        {
            *slot = Cow::Owned(builtin_name.to_owned());
        }
    }

    if table.iter().all(Option::is_none) {
        // The font loaded and the encoding resolved, and between them they addressed not
        // one glyph. Reporting beats rendering an entirely blank page in silence.
        return Err(FontError::Malformed {
            name: name.to_owned(),
            detail: "no character code maps to a glyph".to_owned(),
        });
    }

    Ok((table, names))
}

/// ISO 32000-2 §9.6.5.1's Table 112, on a name-keyed program: which encoding is the *base*.
///
/// The rule is a sentence rather than an algorithm, and it is the sentence a corpus cannot
/// test. Table 112 makes an **embedded** font program's own built-in encoding the default
/// base, and only a font whose program is *not* embedded falls back to `StandardEncoding`
/// or to the Symbolic flag. Nearly every real CFF's built-in encoding *is*
/// `StandardEncoding`, so reading the wrong sentence is invisible on all 974 corpus
/// documents but one, and on that one it moves a single code.
///
/// So the fixtures state the two encodings *differently* and ask which one answered. The
/// CFF is a value rather than a font program: what is under test is the choice of base,
/// which happens entirely in [`simple_code_table`], and building a byte-level CFF here
/// would test `read-fonts` instead.
#[cfg(test)]
mod cff_encoding_tests {
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    use super::{NameKeyed, simple_code_table};
    use crate::fixture::font_dictionary;

    /// The glyphs the fixture font holds, by the names its charset gives them.
    const ALPHA: u16 = 11;
    const BETA: u16 = 12;
    /// The charset also names a glyph `A`, which is what `StandardEncoding` asks for at 65.
    const LATIN_A: u16 = 13;

    /// A name-keyed CFF whose built-in encoding disagrees with `StandardEncoding`.
    ///
    /// Code 65 is `A` in `StandardEncoding` and `alpha` in this font; code 66 is `B`, which
    /// the charset does not have at all, and `beta` in this font. So each code distinguishes
    /// the two bases, and in opposite ways: one would draw the wrong glyph and the other
    /// would draw nothing.
    fn fixture() -> NameKeyed {
        let by_name: BTreeMap<Box<str>, u16> = [
            ("alpha".into(), ALPHA),
            ("beta".into(), BETA),
            ("A".into(), LATIN_A),
        ]
        .into_iter()
        .collect();
        let mut builtin = Box::new([None; 256]);
        builtin[65] = Some(ALPHA);
        builtin[66] = Some(BETA);
        let builtin_names = Box::new(std::array::from_fn(|code| match code {
            65 => Some("alpha".into()),
            66 => Some("beta".into()),
            _ => None,
        }));
        NameKeyed {
            by_name,
            builtin,
            builtin_names,
        }
    }

    /// Resolves the fixture font's codes under the `/Encoding` entries given.
    fn resolve(entries: &str) -> (crate::loading::CodeTable, crate::glyph_names::GlyphNames) {
        let (document, dict) = font_dictionary(entries);
        simple_code_table(&document, &dict, &fixture(), "F1").expect("the fixture resolves")
    }

    /// With no `/Encoding`, every code is the program's own — which is the uncontested half.
    #[test]
    fn a_font_with_no_encoding_entry_is_its_own() {
        let (table, names) = resolve("");

        assert_eq!(table[65], Some(ALPHA));
        assert_eq!(table[66], Some(BETA));
        assert_eq!(names[65], Cow::Borrowed("alpha"));
    }

    /// `/Differences` alone describes differences from the *program's* encoding.
    ///
    /// The rule that was wrong: a code the array does not mention keeps the built-in
    /// glyph. Reading `StandardEncoding` as the base instead would draw `LATIN_A` at 65 —
    /// a plausible wrong letter — and nothing at all at 66.
    #[test]
    fn differences_without_a_base_encoding_layer_over_the_programs_own() {
        let (table, names) = resolve("/Encoding << /Type /Encoding /Differences [66 /A] >>");

        assert_eq!(table[66], Some(LATIN_A), "the array names this code");
        assert_eq!(table[65], Some(ALPHA), "and says nothing about this one");
        assert_eq!(names[65], Cow::Borrowed("alpha"));
        assert_eq!(names[66], Cow::Borrowed("A"));
    }

    /// A named `/BaseEncoding` is still the base, and the program's encoding is not consulted.
    ///
    /// Which is what keeps the rule above from being a licence: the document may state the
    /// base, and when it does, a code it leaves undefined is undefined rather than the
    /// font's own. Code 66 is `B`, which this charset lacks, so it reaches no glyph.
    #[test]
    fn a_named_base_encoding_still_wins() {
        let (table, _) = resolve("/Encoding << /BaseEncoding /WinAnsiEncoding >>");

        assert_eq!(table[65], Some(LATIN_A));
        assert_eq!(table[66], None);
    }

    /// An `/Encoding` name Table 109 does not permit leaves the font its own encoding.
    ///
    /// ISO 32000-2 §9.6.2.1, Table 109, of `/Encoding`:
    ///
    /// > ( Optional ) A specification of the font's character encoding if different from its
    /// > built-in encoding. The value of Encoding shall be either the name of a predefined
    /// > encoding ( MacRomanEncoding, MacExpertEncoding , or WinAnsiEncoding , as described in
    /// > Annex D, "Character sets and encodings") or an encoding dictionary
    ///
    /// The entry is optional and the same cell says what its absence means, so a value the
    /// table does not permit has said nothing — and this must draw exactly what
    /// `a_font_with_no_encoding_entry_is_its_own` draws. `bug859204.pdf` writes `/Encoding
    /// /NULL` and lost its whole page to a refusal.
    ///
    /// The second half is the control, and it is what keeps the first from being a licence:
    /// **every name the table permits is a name this crate has a table for**, so a font naming
    /// one means it and gets it. `MacExpertEncoding` used to be the exception and used to be
    /// refused by name; Annex D.4's table arrived and it is not.
    ///
    /// The two lists still answer different questions — *may a font say this* and *does this
    /// crate have the table* — and this asserts that they currently give the same four answers,
    /// which is what makes [`crate::FontError::UnsupportedEncoding`] a branch with no member
    /// rather than a report that fires. A later edition adding a fifth permitted name would
    /// fail here rather than start refusing fonts quietly.
    #[test]
    fn an_encoding_name_the_table_does_not_permit_is_no_encoding_at_all() {
        let (table, names) = resolve("/Encoding /NULL");
        let (own, own_names) = resolve("");
        assert_eq!(table, own);
        assert_eq!(names[65], own_names[65]);
        assert_eq!(table[65], Some(ALPHA));

        for name in crate::glyph_names::PERMITTED_ENCODING_NAMES {
            assert!(
                pdf_font_encoding_has(name),
                "{} is permitted and has no table",
                String::from_utf8_lossy(name)
            );
        }
    }

    /// Whether [`crate::encoding::BaseEncoding`] has a table for this name.
    fn pdf_font_encoding_has(name: &[u8]) -> bool {
        crate::encoding::BaseEncoding::by_name(name).is_some()
    }

    /// The font descriptor's flags decide nothing here, which is the whole finding.
    ///
    /// Table 112 asks the Symbolic flag only for a font whose program is *not* embedded,
    /// and a bare CFF reaches this crate by having been embedded. Stated as a test because
    /// the previous reading was defensible from the same table's last sentence.
    #[test]
    fn the_symbolic_flag_changes_nothing_for_an_embedded_program() {
        let symbolic = resolve("/FontDescriptor << /Flags 4 >>").0;
        let nonsymbolic = resolve("/FontDescriptor << /Flags 32 >>").0;

        assert_eq!(symbolic[65], Some(ALPHA));
        assert_eq!(symbolic, nonsymbolic);
    }
}
