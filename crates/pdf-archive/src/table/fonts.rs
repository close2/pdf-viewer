//! Clause 6.2.11 of ISO 19005-2 and clause 6.2.10 of ISO 19005-4: fonts.
//!
//! The largest tranche and the one PDF/A exists for. Both parts open it by saying what the
//! whole subclause is *for* — that the same glyphs come back in fifty years, and that the
//! characters behind them can still be recovered — and both then say that a requirement binds
//! every font in the file, including one shown only in text rendering mode 3, unless it says
//! otherwise. Neither of those two opening subclauses (ISO 19005-2 §6.2.11.1,
//! ISO 19005-4 §6.2.10.1) constrains a document by itself, so neither is a row here; what they
//! do is fix the population every row below is about, which is why they are named in this
//! paragraph rather than left out.
//!
//! # Where the two parts differ, and it is not where one would guess
//!
//! Most of the subclause is shared word for word. Five places are not:
//!
//! - **Part 2 makes the Unicode rules a `shall` for Levels A and U only** (§6.2.11.7.1 says a
//!   Level B writer may ignore §6.2.11.7). **Part 4 states the same rules as `should`** —
//!   §6.2.10.7 and the first half of §6.2.10.8 — so they bind no part 4 target at all, and the
//!   rows carrying them cite part 2 alone.
//! - **Part 4 keeps two `shall`s inside those two subclauses**: the values in a `ToUnicode`
//!   `CMap` that *is* present, and that an `ActualText` entry states no private-use character.
//!   The second has no counterpart in part 2 whatsoever, which makes it the one font rule that
//!   is stricter in part 4 than in part 2.
//! - **Part 2 requires `CharSet` and `CIDSet`, where present, to be complete** (§6.2.11.4.2).
//!   Part 4's §6.2.10.4.2 dropped both: it now permits subsetting and states no requirement.
//! - **Part 4's font-metrics subclause grew two rules** (§6.2.10.5): a Type 3 font's `d0`/`d1`
//!   operands, and a vertical composite font's `DW2`/`W2`. Part 2 §6.2.11.5 states neither.
//! - **The two font-metrics subclauses bind different glyphs**, and this list said four for as
//!   long as it missed it. Part 2 §6.2.11.5 asks that the dictionary and the program agree for a
//!   font that is embedded and used for rendering, and says nothing about *which* of its glyphs;
//!   part 4 §6.2.10.5 narrows the same rule to the glyphs referenced for rendering and exempts
//!   those referenced only in mode 3. So part 4 is the *weaker* of the two here — the reverse of
//!   the direction the entry above records — and a predicate that judged every stated width
//!   would report a part 4 file for a width it is not asked about.
//!
//! The `cmap`-subtable rules of §6.2.11.6 and §6.2.10.6 also differ in their detail — part 2
//! wants one or more non-symbolic subtables, part 4 names (3,1) or (1,0) — but they are one
//! rule stated twice and are cited as such.
//!
//! # Two populations, and why they are not the same one
//!
//! Almost every rule here is about *a font dictionary*, so its population is **every
//! `/Type /Font` dictionary the cross-reference table reaches** — the same bound
//! `super::file_structure` takes, for the same reason: both parts exempt an indirect object no
//! cross-reference section names. What that population includes and should not is a font
//! object no page ever selects, and what it misses is a font dictionary written directly inside
//! a resource dictionary rather than as an object of its own.
//!
//! Embedding is the exception, because its clause defines its own population: a font is used
//! when a content stream references one of its glyphs, and one shown only in rendering mode 3
//! is exempt. That answer cannot be read off a dictionary, so [`selected_fonts`] walks the
//! content streams for it.
//!
//! # The rule every predicate in this file keeps
//!
//! **A predicate reports only what it has established, and stays silent where it cannot
//! decide.** Several of these clauses turn on a fact that lives inside the embedded font
//! program — an Adobe Glyph List name, a `cmap` subtable, a glyph's advance. `pdf-font` is the
//! reader for those and this crate now depends on it (`Cargo.toml` says why that keeps the
//! crate's design rather than bending it), so the rules that were unanswerable for want of it
//! are answered here.
//!
//! **All of the ones this paragraph used to list are answered, and by the same move each time:
//! the fact was inside `pdf-font` rather than on its surface, so `pdf-font` publishes it.** A
//! glyph's advance as the *program* states it is [`pdf_font::LoadedFont::program_advance`] —
//! where [`pdf_font::LoadedFont::advance`] is the *dictionary's*, which is the very number
//! §6.2.11.5 / §6.2.10.5 compare it against — and an sfnt's `cmap` subtables, which
//! §6.2.11.6 / §6.2.10.6 name by platform and encoding ID, are
//! [`pdf_font::LoadedFont::program_cmap_subtables`]. None of them is a second font reader here,
//! which is the condition this crate's dependency on `pdf-font` was taken under.
//!
//! The last four arrived together, because §6.2.11.7's and §6.2.11.4.2's rules turn on them
//! (ADR 0924):
//!
//! - **A `ToUnicode` `CMap`'s value set** is [`pdf_font::tounicode::ToUnicode::mappings`], which
//!   hands back what the file *said* rather than answering per code.
//!   [`to_unicode_values_are_usable`] used to ask the map about every code a font's code space
//!   holds, which was exact for no font: ISO 32000-2 §9.7.6.2 lets a code be one to four bytes
//!   and the walk stopped at two.
//! - **Which glyph name a simple font's encoding selected** is
//!   [`pdf_font::LoadedFont::selected_glyph_name`]. §6.2.11.7.2's second exemption is about the
//!   *name*, and every reading of a code — `text`, `naming_gap` — has by then taken §9.10.2's
//!   closing permission to choose a character where its methods fail, which hides exactly the
//!   fonts the exemption does not cover.
//!
//! §6.2.11.4.2's two rules came off `Unchecked` the same way and in the same ADR: what a
//! *program* contains, rather than what a code reaches, is
//! [`pdf_font::LoadedFont::program_glyph_names`] and
//! [`pdf_font::LoadedFont::program_character_identifiers`]. A `/CharSet` or `/CIDSet` claims to
//! list the program's whole set including the glyphs nothing draws, so a reader that answers per
//! code cannot check the claim at all.
//!
//! Where such a fact decides *whether the rule applies at all*, the requirement is
//! [`Check::Unchecked`] with the reason; where it decides only *some* of the cases, the
//! predicate checks the cases it can and its doc comment names the ones it declines. What no
//! predicate here does is guess, because a validator that invents a failure is worse than one
//! that admits a gap.

use std::collections::BTreeSet;

use pdf_font::cmap::CMap;
use pdf_font::encoding;
use pdf_font::encoding::SymbolicEncoding;
use pdf_font::tounicode::{Mapping, ToUnicode};
use pdf_font::{LoadedFont, NOTDEF_GLYPH};
use pdf_syntax::{Dictionary, Document, Lexer, Name, Object, ObjectId, Token, text_string};

use crate::Examination;
use crate::finding::{Findings, Where};
use crate::requirement::{Applies, Check, Clauses, Requirement};
use crate::survey::SelectedFont;
use crate::target::{Level, Part};

/// The rows this module contributes, which `super::TRANCHES` concatenates.
pub(super) static REQUIREMENTS: &[Requirement] = &[
    Requirement {
        id: "fonts/font-programs-conform-to-their-own-specifications",
        asks: "Every font and font program in the file shall conform to the base standard's \
               font clauses and to the format specifications those clauses refer to.",
        clauses: Clauses::both("6.2.11.2", "6.2.10.2"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "judging a font program against its own format specification is a validator of \
             sfnt, CFF and Type 1 that nothing in this tree is; `pdf-font` reads those formats \
             to draw from them and reports what it could not use, not whether the program is \
             well-formed",
        ),
    },
    Requirement {
        id: "fonts/cid-system-info-agrees-with-the-cmap",
        asks: "A composite font whose encoding is not one of the identity CMaps shall have the \
               same registry and ordering in the CIDFont and in the CMap, and the CIDFont's \
               supplement shall be at least the CMap's.",
        clauses: Clauses::both("6.2.11.3.1", "6.2.10.3.1"),
        applies: Applies::Always,
        check: Check::Implemented(cid_system_info_agrees_with_the_cmap),
    },
    Requirement {
        id: "fonts/cid-to-gid-map-present",
        asks: "An embedded Type 2 CIDFont shall state a CIDToGIDMap entry that is either a \
               stream or the name Identity.",
        clauses: Clauses::both("6.2.11.3.2", "6.2.10.3.2"),
        applies: Applies::Always,
        check: Check::Implemented(cid_to_gid_map_present),
    },
    Requirement {
        id: "fonts/cmap-embedded-or-predefined",
        asks: "A CMap used in the file shall either be one the base standard predefines or be \
               embedded in the file as a CMap stream.",
        clauses: Clauses::both("6.2.11.3.3", "6.2.10.3.3"),
        applies: Applies::Always,
        check: Check::Implemented(cmap_embedded_or_predefined),
    },
    Requirement {
        id: "fonts/embedded-cmap-states-its-own-write-mode",
        asks: "An embedded CMap's WMode entry shall be the same integer as the write mode the \
               CMap program itself states.",
        clauses: Clauses::both("6.2.11.3.3", "6.2.10.3.3"),
        applies: Applies::Always,
        check: Check::Implemented(embedded_cmap_states_its_own_write_mode),
    },
    Requirement {
        id: "fonts/cmap-uses-only-predefined-cmaps",
        asks: "A CMap shall not build on any CMap other than the ones the base standard \
               predefines.",
        clauses: Clauses::both("6.2.11.3.3", "6.2.10.3.3"),
        applies: Applies::Always,
        check: Check::Implemented(cmap_uses_only_predefined_cmaps),
    },
    Requirement {
        id: "fonts/font-programs-embedded",
        asks: "The font program of every font a content stream renders shall be embedded in \
               the file, the fourteen standard fonts included.",
        clauses: Clauses::both("6.2.11.4.1", "6.2.10.4.1"),
        applies: Applies::Always,
        check: Check::Implemented(font_programs_embedded),
    },
    Requirement {
        id: "fonts/font-programs-embeddable-without-permission",
        asks: "Only a font program that may lawfully be embedded for unlimited, universal \
               rendering shall be used.",
        clauses: Clauses::both("6.2.11.4.1", "6.2.10.4.1"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "this is a fact about a licence rather than about the file; a font program's \
             embedding bits are evidence of what its vendor asserts and not of what the \
             copyright holder permits, and neither part makes those bits the test",
        ),
    },
    Requirement {
        id: "fonts/embedded-programs-define-every-glyph-shown",
        asks: "An embedded font program shall contain a definition for every glyph the file \
               renders from it.",
        clauses: Clauses::both("6.2.11.4.1", "6.2.10.4.1"),
        applies: Applies::Always,
        check: Check::Implemented(embedded_programs_define_every_glyph_shown),
    },
    Requirement {
        id: "fonts/charset-lists-every-glyph-in-the-program",
        asks: "Where an embedded Type 1 font's descriptor states a CharSet string, it shall \
               name every glyph the font program contains and not only the glyphs the file \
               uses.",
        clauses: Clauses::only_two("6.2.11.4.2"),
        applies: Applies::Always,
        check: Check::Implemented(charset_lists_every_glyph_in_the_program),
    },
    Requirement {
        id: "fonts/cidset-lists-every-cid-in-the-program",
        asks: "Where an embedded CIDFont's descriptor states a CIDSet stream, it shall mark \
               every CID the font program contains and not only the CIDs the file uses.",
        clauses: Clauses::only_two("6.2.11.4.2"),
        applies: Applies::Always,
        check: Check::Implemented(cidset_lists_every_cid_in_the_program),
    },
    Requirement {
        id: "fonts/widths-agree-with-the-program",
        asks: "The glyph widths the font dictionary states shall agree with the embedded font \
               program's own, to within a thousandth of a text-space unit.",
        clauses: Clauses::both("6.2.11.5", "6.2.10.5"),
        applies: Applies::Always,
        check: Check::Implemented(widths_agree_with_the_program),
    },
    Requirement {
        id: "fonts/type3-glyph-procedures-state-their-width",
        asks: "A Type 3 font used for rendering shall have d0 or d1 operands in each glyph \
               procedure that agree with the width the font dictionary states.",
        clauses: Clauses::only_four("6.2.10.5"),
        applies: Applies::Always,
        check: Check::Implemented(type3_glyph_procedures_state_their_width),
    },
    Requirement {
        id: "fonts/vertical-metrics-agree-with-the-program",
        asks: "Where a composite font is rendered in vertical writing mode and its program \
               carries vertical metrics, those shall agree with the DW2 and W2 entries.",
        clauses: Clauses::only_four("6.2.10.5"),
        applies: Applies::Always,
        check: Check::Implemented(vertical_metrics_agree_with_the_program),
    },
    Requirement {
        id: "fonts/non-symbolic-truetype-program-maps-every-code",
        asks: "A non-symbolic TrueType font used for rendering shall have a cmap subtable in \
               its embedded program through which every needed glyph lookup can be made.",
        clauses: Clauses::both("6.2.11.6", "6.2.10.6"),
        applies: Applies::Always,
        check: Check::Implemented(non_symbolic_truetype_program_maps_every_code),
    },
    Requirement {
        id: "fonts/non-symbolic-truetype-uses-a-standard-encoding",
        asks: "A non-symbolic TrueType font shall name MacRomanEncoding or WinAnsiEncoding, \
               either as its Encoding entry or as the BaseEncoding of its encoding dictionary.",
        clauses: Clauses::both("6.2.11.6", "6.2.10.6"),
        applies: Applies::Always,
        check: Check::Implemented(non_symbolic_truetype_uses_a_standard_encoding),
    },
    Requirement {
        id: "fonts/non-symbolic-truetype-differences-are-listed-names",
        asks: "A non-symbolic TrueType font shall state a Differences array only when every \
               name in it is in the Adobe Glyph List.",
        clauses: Clauses::both("6.2.11.6", "6.2.10.6"),
        applies: Applies::Always,
        check: Check::Implemented(non_symbolic_truetype_differences_are_listed_names),
    },
    Requirement {
        id: "fonts/non-symbolic-truetype-differences-need-the-unicode-cmap",
        asks: "A non-symbolic TrueType font shall state a Differences array only when its \
               embedded program carries the Microsoft Unicode (3,1) cmap subtable.",
        clauses: Clauses::both("6.2.11.6", "6.2.10.6"),
        applies: Applies::Always,
        check: Check::Implemented(non_symbolic_truetype_differences_need_the_unicode_cmap),
    },
    Requirement {
        id: "fonts/symbolic-truetype-states-no-encoding",
        asks: "A symbolic TrueType font shall not state an Encoding entry in its font \
               dictionary.",
        clauses: Clauses::both("6.2.11.6", "6.2.10.6"),
        applies: Applies::Always,
        check: Check::Implemented(symbolic_truetype_states_no_encoding),
    },
    Requirement {
        id: "fonts/symbolic-truetype-program-has-a-usable-cmap",
        asks: "A symbolic TrueType font's embedded program shall carry a cmap subtable of the \
               kind its part names.",
        clauses: Clauses::both("6.2.11.6", "6.2.10.6"),
        applies: Applies::Always,
        check: Check::Implemented(symbolic_truetype_program_has_a_usable_cmap),
    },
    Requirement {
        id: "fonts/truetype-codes-reach-glyphs-by-the-standard-route",
        asks: "A TrueType font that is rendered shall let every character code reach its glyph \
               by the base standard's own procedure, without a mapping the reader invents.",
        clauses: Clauses::both("6.2.11.6", "6.2.10.6"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "`pdf-font` takes two tiers past §9.6.5.4's own steps for a code its glyph \
             names do not reach — offering the code as a character, then as a glyph index — \
             which is exactly the reader-chosen mapping this rule forbids, and it reports \
             neither: `NamingGap` and `uncovered_character` are about reading text back \
             rather than about which tier drew it",
        ),
    },
    Requirement {
        id: "fonts/to-unicode-present",
        asks: "Every font shall carry a ToUnicode CMap mapping its referenced codes to Unicode, \
               unless it falls under one of the clause's four exemptions.",
        clauses: Clauses::only_two("6.2.11.7.2"),
        applies: Applies::FromLevel(Level::U),
        check: Check::Implemented(to_unicode_present),
    },
    Requirement {
        id: "fonts/to-unicode-values-are-usable",
        asks: "Every Unicode value a ToUnicode CMap states shall be greater than zero and \
               shall be neither U+FEFF nor U+FFFE.",
        clauses: Clauses::both("6.2.11.7.2", "6.2.10.7"),
        applies: Applies::FromLevel(Level::U),
        check: Check::Implemented(to_unicode_values_are_usable),
    },
    Requirement {
        id: "fonts/actual-text-covers-private-use-characters",
        asks: "A character mapped into the Unicode Private Use Area shall be covered by an \
               ActualText entry, alone or as part of a sequence.",
        clauses: Clauses::only_two("6.2.11.7.3"),
        applies: Applies::FromLevel(Level::A),
        check: Check::Implemented(actual_text_covers_private_use_characters),
    },
    Requirement {
        id: "fonts/actual-text-states-no-private-use",
        asks: "An ActualText entry shall not itself contain a character from the Unicode \
               Private Use Area.",
        clauses: Clauses::only_four("6.2.10.8"),
        applies: Applies::Always,
        check: Check::Implemented(actual_text_states_no_private_use),
    },
    Requirement {
        id: "fonts/no-notdef-glyph-shown",
        asks: "No text-showing operator in any content stream shall reference the .notdef \
               glyph, whatever the rendering mode.",
        clauses: Clauses::both("6.2.11.8", "6.2.10.9"),
        applies: Applies::Always,
        check: Check::Implemented(no_notdef_glyph_shown),
    },
];

// --------------------------------------------------------------------------------------------
// The population every dictionary-shaped rule is about.
// --------------------------------------------------------------------------------------------

/// Visits every `/Type /Font` dictionary the cross-reference table reaches.
///
/// Bounded by that table for the reason `super::file_structure::for_each_stream` gives:
/// ISO 19005-2 §6.1.4 and ISO 19005-4 §6.1.4 exempt an indirect object no cross-reference
/// section names, so this is the population the requirements bind. A font dictionary written
/// directly inside a resource dictionary has no object of its own and is not reached; see the
/// module documentation for why that under-reports rather than mis-reports.
fn for_each_font(exam: &Examination<'_>, mut visit: impl FnMut(ObjectId, &Dictionary)) {
    let document = exam.document;
    for (id, object) in exam.objects() {
        if let Object::Dictionary(dict) = object
            && named(document.get_key(dict, "Type").as_name()).as_deref() == Some("Font")
        {
            visit(*id, dict);
        }
    }
}

/// A name as a string a report can print.
fn named(name: Option<&Name>) -> Option<String> {
    name.map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
}

/// One font dictionary's `/Subtype`.
fn subtype(document: &Document, font: &Dictionary) -> Option<String> {
    named(document.get_key(font, "Subtype").as_name())
}

/// The descendant `CIDFont` of a composite font, with the object it is where it is one.
///
/// ISO 32000-2 §9.7.6.2 makes `/DescendantFonts` "a one-element array", so the first element is
/// the whole of it.
fn descendant(document: &Document, font: &Dictionary) -> Option<(Option<ObjectId>, Dictionary)> {
    let Object::Array(fonts) = document.get_key(font, "DescendantFonts") else {
        return None;
    };
    let first = fonts.first()?;
    let id = first.as_reference();
    match document.resolve(first) {
        Object::Dictionary(dict) => Some((id, dict)),
        _ => None,
    }
}

/// A font dictionary's `/FontDescriptor`.
fn descriptor(document: &Document, font: &Dictionary) -> Option<Dictionary> {
    match document.get_key(font, "FontDescriptor") {
        Object::Dictionary(dict) => Some(dict),
        _ => None,
    }
}

/// Whether a font descriptor carries an embedded font program.
///
/// ISO 32000-2 §9.9's Table 128 gives the three keys — `/FontFile` for Type 1, `/FontFile2` for
/// TrueType, `/FontFile3` for the CFF and `OpenType` shapes — and a descriptor states at most
/// one of them.
fn carries_a_program(descriptor: &Dictionary) -> bool {
    ["FontFile", "FontFile2", "FontFile3"]
        .iter()
        .any(|key| descriptor.get(key).is_some())
}

/// Whether the descriptor's flags say the font is symbolic, where they say anything at all.
///
/// ISO 32000-2 §9.8.2's Table 121 puts `Symbolic` at bit position 3 and `Nonsymbolic` at bit
/// position 6 and requires that exactly one of them be set; the clause then settles which one a
/// reader is to trust:
///
/// > A PDF processor should always check the Symbolic flag to determine whether the state is
/// > Symbolic or NonSymbolic.
///
/// `None` where there is no descriptor or no `/Flags`, because a rule that starts "for all
/// symbolic TrueType fonts" cannot be applied to a font whose file never said which it is.
fn is_symbolic(document: &Document, font: &Dictionary) -> Option<bool> {
    let flags = descriptor(document, font)
        .map(|descriptor| document.get_key(&descriptor, "Flags"))?
        .as_integer()?;
    Some(flags & 0b100 != 0)
}

/// The encoding a simple font names, whether directly or through an encoding dictionary.
///
/// ISO 32000-2 §9.6.5.1 lets `/Encoding` be a predefined encoding's name or a dictionary whose
/// `/BaseEncoding` names one, and every rule below that mentions a named encoding means either
/// spelling.
fn base_encoding_name(document: &Document, font: &Dictionary) -> Option<String> {
    match document.get_key(font, "Encoding") {
        Object::Name(name) => named(Some(&name)),
        Object::Dictionary(dict) => named(document.get_key(&dict, "BaseEncoding").as_name()),
        _ => None,
    }
}

// --------------------------------------------------------------------------------------------
// 6.2.11.3 / 6.2.10.3 — composite fonts.
// --------------------------------------------------------------------------------------------

/// ISO 32000-2 §9.7.5.2's Table 116, the `CMaps` a file may name instead of embedding.
///
/// Held as the standard prints it rather than as the set of `CMap` files Adobe publishes, which
/// is `CLAUDE.md` principle 5 at work: the rule says "except those listed in" the table, so the
/// table is the exemption and anything else is a `CMap` the file has to embed.
///
/// **The part 2 caveat is the one `crate::Part::Two` states generally.** Part 2's rule names
/// ISO 32000-1's Table 118, and this tree carries ISO 32000-2; the two lists agree name for
/// name as far as the later edition can show, and a name the earlier table listed and this one
/// does not would be reported here as unembedded when it is exempt.
static PREDEFINED_CMAPS: &[&str] = &[
    "GB-EUC-H",
    "GB-EUC-V",
    "GBpc-EUC-H",
    "GBpc-EUC-V",
    "GBK-EUC-H",
    "GBK-EUC-V",
    "GBKp-EUC-H",
    "GBKp-EUC-V",
    "GBK2K-H",
    "GBK2K-V",
    "UniGB-UCS2-H",
    "UniGB-UCS2-V",
    "UniGB-UTF16-H",
    "UniGB-UTF16-V",
    "B5pc-H",
    "B5pc-V",
    "HKscs-B5-H",
    "HKscs-B5-V",
    "ETen-B5-H",
    "ETen-B5-V",
    "ETenms-B5-H",
    "ETenms-B5-V",
    "CNS-EUC-H",
    "CNS-EUC-V",
    "UniCNS-UCS2-H",
    "UniCNS-UCS2-V",
    "UniCNS-UTF16-H",
    "UniCNS-UTF16-V",
    "83pv-RKSJ-H",
    "90ms-RKSJ-H",
    "90ms-RKSJ-V",
    "90msp-RKSJ-H",
    "90msp-RKSJ-V",
    "90pv-RKSJ-H",
    "Add-RKSJ-H",
    "Add-RKSJ-V",
    "EUC-H",
    "EUC-V",
    "Ext-RKSJ-H",
    "Ext-RKSJ-V",
    "H",
    "V",
    "UniJIS-UCS2-H",
    "UniJIS-UCS2-V",
    "UniJIS-UCS2-HW-H",
    "UniJIS-UCS2-HW-V",
    "UniJIS-UTF16-H",
    "UniJIS-UTF16-V",
    "KSC-EUC-H",
    "KSC-EUC-V",
    "KSCms-UHC-H",
    "KSCms-UHC-V",
    "KSCms-UHC-HW-H",
    "KSCms-UHC-HW-V",
    "KSCpc-EUC-H",
    "UniKS-UCS2-H",
    "UniKS-UCS2-V",
    "UniKS-UTF16-H",
    "UniKS-UTF16-V",
    "Identity-H",
    "Identity-V",
];

/// ISO 19005-2 §6.2.11.3.1, ISO 19005-4 §6.2.10.3.1.
///
/// The clause exempts the identity `CMaps` outright and otherwise asks the two `CIDSystemInfo`
/// dictionaries to describe one character collection, which is ISO 32000-2 §9.7.3's own
/// condition:
///
/// > In order for a CIDFont and a CMap to be compatible, their Registry and Ordering values
/// > shall be the same.
///
/// on top of which the PDF/A clause adds that the `CIDFont`'s supplement is at least the `CMap`'s,
/// so that every CID the `CMap` can produce exists in the font.
///
/// **Only an embedded `CMap` is compared, and the reason is no longer that the answer is out of
/// reach.** A predefined `CMap`'s `CIDSystemInfo` is stated by the `CMap` program rather than by
/// the file — and this tree carries those programs: `data/cmaps` holds Adobe's 239 files, every
/// one of which states its own `/Registry`, `/Ordering` and `/Supplement`, and `pdf_font::predefined`
/// already reads them for their mappings. So the fact is here, in the strongest form there is:
/// a predefined `CMap` *is* its program, which is why this answer does not depend on which
/// edition of the base standard happens to print a table of them.
///
/// What is not settled is whether the clause should be *applied* to it. PDF Association issue
/// #77 — determining the supplement of a predefined `CMap` — is open and parked, and a great many
/// conforming files state a descendant supplement below the `CMap`'s. Reporting them all is a
/// decision about a contested reading rather than a gap in this tree's data, so it is the project
/// owner's to take. Until then a font naming a predefined `CMap` is passed over, and this
/// paragraph records that the material for the other choice is already on disk.
fn cid_system_info_agrees_with_the_cmap(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_font(exam, |id, font| {
        if subtype(document, font).as_deref() != Some("Type0") {
            return;
        }
        let Object::Stream(cmap) = document.get_key(font, "Encoding") else {
            return;
        };
        let (Object::Dictionary(stated), Some((_, cid_font))) = (
            document.get_key(&cmap.dict, "CIDSystemInfo"),
            descendant(document, font),
        ) else {
            return;
        };
        let Object::Dictionary(used) = document.get_key(&cid_font, "CIDSystemInfo") else {
            return;
        };
        for key in ["Registry", "Ordering"] {
            let (Some(one), Some(other)) = (
                document
                    .get_key(&stated, key)
                    .as_string()
                    .map(<[u8]>::to_vec),
                document.get_key(&used, key).as_string().map(<[u8]>::to_vec),
            ) else {
                continue;
            };
            if one != other {
                findings.record(
                    Where::object(id).named(key),
                    format!("the CMap and its CIDFont state different {key} strings"),
                );
            }
        }
        let (Some(stated), Some(used)) = (
            document.get_key(&stated, "Supplement").as_integer(),
            document.get_key(&used, "Supplement").as_integer(),
        ) else {
            return;
        };
        if used < stated {
            findings.record(
                Where::object(id).named("Supplement"),
                format!(
                    "the CIDFont's supplement {used} is below the CMap's {stated}, so the CMap \
                     can name CIDs the font does not have"
                ),
            );
        }
    });
}

/// ISO 19005-2 §6.2.11.3.2, ISO 19005-4 §6.2.10.3.2.
///
/// The base standard makes `/CIDToGIDMap` optional with a default of `Identity`; both parts of
/// ISO 19005 make it required for an embedded Type 2 `CIDFont`, so an absent entry is a failure
/// here even though it is legal in the base document.
fn cid_to_gid_map_present(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_font(exam, |id, font| {
        if subtype(document, font).as_deref() != Some("CIDFontType2") {
            return;
        }
        if !descriptor(document, font).is_some_and(|descriptor| carries_a_program(&descriptor)) {
            return;
        }
        match document.get_key(font, "CIDToGIDMap") {
            Object::Stream(_) => {}
            Object::Name(name) if name.as_bytes() == b"Identity" => {}
            Object::Null => findings.record(
                Where::object(id).named("CIDToGIDMap"),
                "an embedded Type 2 CIDFont states no CIDToGIDMap",
            ),
            other => findings.record(
                Where::object(id).named("CIDToGIDMap"),
                format!(
                    "an embedded Type 2 CIDFont's CIDToGIDMap is {}, which is neither a stream \
                     nor the name Identity",
                    other.type_name()
                ),
            ),
        }
    });
}

/// ISO 19005-2 §6.2.11.3.3, ISO 19005-4 §6.2.10.3.3.
fn cmap_embedded_or_predefined(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_font(exam, |id, font| {
        if subtype(document, font).as_deref() != Some("Type0") {
            return;
        }
        let Object::Name(name) = document.get_key(font, "Encoding") else {
            // A stream is an embedded CMap and satisfies the rule; anything else is a font the
            // base standard already rejects, and saying so is not this requirement's job.
            return;
        };
        let Some(name) = named(Some(&name)) else {
            return;
        };
        if !PREDEFINED_CMAPS.contains(&name.as_str()) {
            findings.record(
                Where::object(id).named(name.clone()),
                format!(
                    "a composite font names the CMap {name}, which the base standard does not \
                     predefine and the file does not embed"
                ),
            );
        }
    });
}

/// ISO 19005-2 §6.2.11.3.3, ISO 19005-4 §6.2.10.3.3, second paragraph.
///
/// Read over every `CMap` stream the cross-reference table holds, rather than only the ones a
/// composite font names, because a chain of `/UseCMap` entries puts the offending reference in
/// a `CMap` the font never mentions.
///
/// **What it checks is the dictionary's `/UseCMap`, not the program's `usecmap` operator.** A
/// `CMap` program can name its parent in its own syntax, and reading that would mean parsing the
/// program — see `fonts/embedded-cmap-states-its-own-write-mode` for why this crate does not.
fn cmap_uses_only_predefined_cmaps(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        let Object::Stream(stream) = document.get(id) else {
            continue;
        };
        if named(document.get_key(&stream.dict, "Type").as_name()).as_deref() != Some("CMap") {
            continue;
        }
        match document.get_key(&stream.dict, "UseCMap") {
            Object::Null => {}
            Object::Name(name) => {
                let Some(name) = named(Some(&name)) else {
                    continue;
                };
                if !PREDEFINED_CMAPS.contains(&name.as_str()) {
                    findings.record(
                        Where::object(id).named(name.clone()),
                        format!("an embedded CMap builds on {name}, which is not predefined"),
                    );
                }
            }
            _ => findings.record(
                Where::object(id).named("UseCMap"),
                "an embedded CMap builds on another embedded CMap rather than a predefined one",
            ),
        }
    }
}

/// ISO 19005-2 §6.2.11.3.3, ISO 19005-4 §6.2.10.3.3 — the sentence about `/WMode`.
///
/// A `CMap` states its writing mode twice: once as `/WMode` in the stream dictionary and once
/// as a `/WMode … def` in the `CMap` program. ISO 32000-2 §9.7.5.3's Table 118 already requires
/// the two to agree —
///
/// > The value of this entry shall be the same as the value of WMode in the CMap file.
///
/// — and both parts of ISO 19005 restate it, which is why it is a row here: a PDF/A verdict has
/// to name the ISO 19005 clause a file failed, not the base standard's.
///
/// **A program that states no `/WMode` states 0**, because Table 118 gives the entry a default
/// of 0 and §9.7.5.1 makes 0 the horizontal mode every `CMap` has unless it says otherwise. So a
/// dictionary claiming 1 over a silent program is the mismatch this clause is about, not a
/// silence to be excused.
///
/// The population is every `CMap` stream the cross-reference table reaches, for the reason
/// [`cmap_uses_only_predefined_cmaps`] gives: a `CMap` a font reaches only through a `/UseCMap`
/// chain is embedded in the file just the same.
fn embedded_cmap_states_its_own_write_mode(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        let Object::Stream(stream) = document.get(id) else {
            continue;
        };
        if named(document.get_key(&stream.dict, "Type").as_name()).as_deref() != Some("CMap") {
            continue;
        }
        let Some(bytes) = document.decoded_stream_data(&stream) else {
            // A stream this tree cannot decode is not a stream whose write mode disagrees.
            continue;
        };
        let stated = document
            .get_key(&stream.dict, "WMode")
            .as_integer()
            .unwrap_or(0);
        let programs = i64::from(CMap::parse(&bytes, None).wmode());
        if stated != programs {
            findings.record(
                Where::object(id).named("WMode"),
                format!(
                    "an embedded CMap's dictionary states WMode {stated} and its program states \
                     {programs}"
                ),
            );
        }
    }
}

// --------------------------------------------------------------------------------------------
// 6.2.11.4 / 6.2.10.4 — embedding, and the content walk that decides who it binds.
// --------------------------------------------------------------------------------------------

/// ISO 19005-2 §6.2.11.4.1, ISO 19005-4 §6.2.10.4.1 — the rule the whole standard is for.
///
/// The population is [`crate::survey::Survey`]'s: a font a content stream shows in a rendering mode
/// other than 3. Both parts state the definition themselves — a font is used when a glyph of it
/// is referenced from a content stream — and both exempt mode 3 in a note, so a font dictionary
/// that is merely present in a resource dictionary is not what this rule is about and is not
/// reported.
///
/// A Type 3 font is passed over because it has no font program to embed: ISO 32000-2 §9.6.4
/// makes its glyphs content streams inside the font dictionary, which are in the file by
/// construction.
fn font_programs_embedded(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for used in exam.survey().fonts() {
        if !used.rendered {
            continue;
        }
        let Some(subtype) = subtype(document, &used.dict) else {
            continue;
        };
        if subtype == "Type3" {
            continue;
        }
        let (holder_id, holder) = if subtype == "Type0" {
            match descendant(document, &used.dict) {
                Some((id, dict)) => (id.or(used.id), dict),
                None => continue,
            }
        } else {
            (used.id, used.dict.clone())
        };
        let place = holder_id.map_or_else(
            || Where::page(used.page).named(used.name.clone()),
            |id| Where::object(id).named(used.name.clone()),
        );
        match descriptor(document, &holder) {
            None => findings.record(
                place,
                format!(
                    "the font {} is rendered and has no font descriptor, so it embeds no font \
                     program — the fourteen standard fonts have no exemption from this",
                    used.name
                ),
            ),
            Some(descriptor) if !carries_a_program(&descriptor) => findings.record(
                place,
                format!(
                    "the font {} is rendered and its descriptor states no FontFile, FontFile2 \
                     or FontFile3",
                    used.name
                ),
            ),
            Some(_) => {}
        }
    }
}

// --------------------------------------------------------------------------------------------
// 6.2.11.4.2 — subset embedding, which part 4 dropped and part 2 alone still states.
// --------------------------------------------------------------------------------------------

/// The glyph name the base standard requires a `/CharSet` string to leave out.
///
/// §9.8.1's Table 122, on `/CharSet`:
///
/// > The name .notdef shall be omitted; it shall exist in the font subset.
///
/// So a program's set and a conforming `/CharSet`'s set differ by this one name by the base
/// standard's own instruction, and subtracting it is not a licence taken here.
const OMITTED_FROM_CHARSET: &str = ".notdef";

/// The names a `/CharSet` string lists, read as the PDF name syntax §9.8.1 requires it to be.
///
/// > The names in this string shall be in PDF syntax - that is, each name preceded by a slash
/// > (/).
///
/// So the string's *contents* are lexed rather than split on a byte: a name may carry `#`
/// escapes, and the lexer is what resolves them. An empty result is `None` rather than an empty
/// set — a string this crate could read nothing out of is not a claim that the font has no
/// glyphs, and treating it as one would report every font whose `/CharSet` is written in a form
/// not read here.
fn charset_names(bytes: &[u8]) -> Option<BTreeSet<String>> {
    let mut lexer = Lexer::new(bytes);
    let mut names = BTreeSet::new();
    while let Some(token) = lexer.next_token() {
        if let Token::Name(name) = token {
            names.insert(String::from_utf8_lossy(&name).into_owned());
        }
    }
    (!names.is_empty()).then_some(names)
}

/// ISO 19005-2 §6.2.11.4.2, second requirement, and part 2 only: ISO 19005-4 §6.2.10.4.2 dropped
/// both of this subclause's rules and states none.
///
/// The clause makes normative what §9.8.1's Table 122 describes: a `/CharSet` present in an
/// embedded Type 1 font's descriptor names every glyph **in the program**, not merely the glyphs
/// the file uses. So the comparison is against
/// [`LoadedFont::program_glyph_names`], which answers for the program's own charset, less
/// [`OMITTED_FROM_CHARSET`].
///
/// # What is passed over, and why each would otherwise accuse a sound file
///
/// - **A font with no `/CharSet`.** The rule is conditional on the entry being present.
/// - **A substituted font, or one `pdf-font` refuses.** The names would be this machine's.
/// - **A font whose program is an sfnt.** It keys glyphs by index and has no charset to compare;
///   `/CharSet` is "meaningful only in Type 1 fonts" by Table 122's own words.
/// - **A `/CharSet` this crate read no name out of.** See [`charset_names`].
///
/// The direction of the comparison is one way on purpose: a name the program has and the string
/// omits is the fault the clause names. A name the string has and the program does not is a
/// different fault, which this clause does not state.
fn charset_lists_every_glyph_in_the_program(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_font(exam, |id, font| {
        if !matches!(
            subtype(document, font).as_deref(),
            Some("Type1" | "MMType1")
        ) {
            return;
        }
        let Some(descriptor) = descriptor(document, font) else {
            return;
        };
        let Some(stated) = document
            .get_key(&descriptor, "CharSet")
            .as_string()
            .and_then(charset_names)
        else {
            return;
        };
        let Ok(loaded) = LoadedFont::load(document, font, "CharSet") else {
            return;
        };
        let Some(present) = loaded.program_glyph_names() else {
            return;
        };
        let missing: Vec<String> = present
            .into_iter()
            .filter(|name| name != OMITTED_FROM_CHARSET && !stated.contains(name))
            .collect();
        if missing.is_empty() {
            return;
        }
        findings.record(
            Where::object(id).named("CharSet"),
            format!(
                "the CharSet string omits {} of the font program's glyph names, including /{}",
                missing.len(),
                missing.first().map_or("", String::as_str)
            ),
        );
    });
}

/// ISO 19005-2 §6.2.11.4.2, third requirement, and part 2 only.
///
/// The `CIDFont` counterpart of [`charset_lists_every_glyph_in_the_program`]: a `/CIDSet` present
/// in an embedded `CIDFont`'s descriptor marks every CID the program defines rather than only the
/// ones the file uses. §9.8.3.1's Table 124 gives the stream's shape, and it is read here as the
/// table states it:
///
/// > The stream's data shall be organised as a table of bits indexed by CID. The bits shall be
/// > stored in bytes with the high-order bit first.
///
/// # The one assumption, and where it is refused
///
/// A `CIDFontType2`'s program has glyph indices rather than CIDs, and §9.7.4.2's `/CIDToGIDMap`
/// is what relates the two. [`LoadedFont::program_character_identifiers`] answers for the
/// identity map, which is that entry's default, so a `CIDFont` stating a *stream* map is passed
/// over here rather than judged against an assumption its file contradicts.
///
/// Also passed over: a descendant with no `/CIDSet`, a substituted or unreadable font, and a
/// `CIDFontType0` whose CFF is name-keyed rather than CID-keyed — the last because its glyphs
/// carry names and not CIDs, so there is no set of the clause's kind to compare.
fn cidset_lists_every_cid_in_the_program(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_font(exam, |id, font| {
        if subtype(document, font).as_deref() != Some("Type0") {
            return;
        }
        let Some((_, cid_font)) = descendant(document, font) else {
            return;
        };
        // The identity map is Table 117's default, so an absent entry is it; anything else is a
        // stream this crate would have to invert before the comparison meant anything.
        match document.get_key(&cid_font, "CIDToGIDMap") {
            Object::Null => {}
            Object::Name(name) if name.as_bytes() == b"Identity" => {}
            _ => return,
        }
        let Some(descriptor) = descriptor(document, &cid_font) else {
            return;
        };
        let Object::Stream(stream) = document.get_key(&descriptor, "CIDSet") else {
            return;
        };
        let Some(bits) = document.decoded_stream_data(&stream) else {
            return;
        };
        let Ok(loaded) = LoadedFont::load(document, font, "CIDSet") else {
            return;
        };
        let Some(present) = loaded.program_character_identifiers() else {
            return;
        };
        // Table 124: bits indexed by CID, high-order bit of the first byte being CID 0.
        let marked = |cid: u16| {
            let at = usize::from(cid) / 8;
            let bit = 7u32.saturating_sub(u32::from(cid) % 8);
            bits.get(at).is_some_and(|byte| byte & (1u8 << bit) != 0)
        };
        let missing: Vec<u16> = present.into_iter().filter(|cid| !marked(*cid)).collect();
        if missing.is_empty() {
            return;
        }
        findings.record(
            Where::object(id).named("CIDSet"),
            format!(
                "the CIDSet stream leaves {} of the font program's CIDs unmarked, including \
                 CID {}",
                missing.len(),
                missing.first().copied().unwrap_or_default()
            ),
        );
    });
}

// --------------------------------------------------------------------------------------------
// The two rules about a *code*: 6.2.11.4.1 / 6.2.10.4.1's last requirement, and
// 6.2.11.8 / 6.2.10.9.
// --------------------------------------------------------------------------------------------

/// One selected font loaded from **its own embedded program**, or nothing.
///
/// Every condition below is here to keep a report honest rather than to be thorough, and each
/// one is a way this check could otherwise fail a conforming file:
///
/// - **A font with no embedded program is passed over.** `pdf-font` substitutes a system face for
///   one, and the glyphs a substitute happens to have are a fact about this machine. Whether the
///   program should have been embedded is [`font_programs_embedded`]'s question, asked once.
/// - **A substituted font is passed over** even when a program *is* embedded, because
///   substitution means the embedded one could not be read — and a program this tree cannot read
///   is not a program whose glyphs are missing.
/// - **A font `pdf-font` refuses outright is passed over**, for the same reason: the refusal is a
///   statement about this reader.
/// - **A Type 3 font is passed over**, because ISO 32000-2 §9.6.4 makes its glyphs content
///   streams in the font dictionary; it has no program for a glyph to be absent from.
fn font_with_its_own_program(document: &Document, used: &SelectedFont) -> Option<LoadedFont> {
    let subtype = subtype(document, &used.dict)?;
    if subtype == "Type3" {
        return None;
    }
    let holder = if subtype == "Type0" {
        descendant(document, &used.dict)?.1
    } else {
        used.dict.clone()
    };
    if !descriptor(document, &holder).is_some_and(|descriptor| carries_a_program(&descriptor)) {
        return None;
    }
    let font = LoadedFont::load(document, &used.dict, &used.name).ok()?;
    (!font.is_substituted()).then_some(font)
}

/// Where a selected font's fault is reported, by object where it has one.
fn font_place(used: &SelectedFont) -> Where {
    used.id.map_or_else(
        || Where::page(used.page).named(used.name.clone()),
        Where::object,
    )
}

/// ISO 19005-2 §6.2.11.8, ISO 19005-4 §6.2.10.9.
///
/// A code whose glyph selection lands on glyph 0 has referenced `.notdef`, which is what both
/// parts forbid from any text-showing operator. ISO 32000-2 states glyph 0's meaning twice —
/// §9.6.5.2 substitutes it where an encoding names a glyph the program does not have, and
/// §9.7.6.3 substitutes "the glyph for CID 0 (which shall be present)" where no glyph exists for
/// a CID — so the two routes agree on where a failed selection ends.
///
/// **Every rendering mode, including 3**, which is the clause's own wording and is why this reads
/// [`SelectedFont::shown`] rather than filtering it by [`SelectedFont::rendered`]. The corpus
/// carries a file that is a failure for exactly that reason.
///
/// A code that reaches *no* glyph is not reported here. That answer means `pdf-font`'s selection
/// route ended nowhere rather than at glyph 0 — a shortfall of this reader is possible there, and
/// a rule that named the file for it would be reporting our gap as the producer's fault.
fn no_notdef_glyph_shown(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for used in exam.survey().fonts() {
        let Some(font) = font_with_its_own_program(document, used) else {
            continue;
        };
        // One finding per code rather than per occurrence: a page that shows the same code a
        // thousand times has one fault in it.
        let mut reported = BTreeSet::new();
        for text in &used.shown {
            for code in font.decode(text) {
                if font.glyph_index(code) != Some(NOTDEF_GLYPH) || !reported.insert(code.value()) {
                    continue;
                }
                findings.record(
                    font_place(used).named(used.name.clone()),
                    format!(
                        "a text-showing operator draws code {} in the font {}, which its \
                         embedded program maps to the .notdef glyph",
                        code.value(),
                        used.name
                    ),
                );
            }
        }
    }
}

/// ISO 19005-2 §6.2.11.4.1, ISO 19005-4 §6.2.10.4.1 — the requirement after embedding itself.
///
/// Subsetting is permitted by both parts, and this is the sentence that bounds it: whatever the
/// file draws, the embedded program has to define. So the population is the codes a content
/// stream showed, and the test is whether the font's own selection route reaches a glyph for
/// each of them.
///
/// **A code reaching glyph 0 is the same failure**, because ISO 32000-2 §9.6.5.2 and §9.7.6.3
/// both put a *failed* selection there; the two rules therefore report the same file, under the
/// two clauses that each state a requirement it breaks.
///
/// # The two places this is less exact than the clause
///
/// - **Mode 3 is excused per font rather than per string.** The clause exempts a glyph referenced
///   only in rendering mode 3, and [`SelectedFont`] records the mode against the font rather than
///   against each string, so a font drawn in both modes has all of its codes judged. Recording
///   the mode per string in [`crate::survey`] is what would close it.
/// - **A font whose strings did not fit the survey's budget is passed over**
///   ([`SelectedFont::shown_complete`]), because this rule asks something of *every* code shown
///   and a prefix cannot answer it.
fn embedded_programs_define_every_glyph_shown(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for used in exam.survey().fonts() {
        if !used.rendered || !used.shown_complete {
            continue;
        }
        let Some(font) = font_with_its_own_program(document, used) else {
            continue;
        };
        let mut reported = BTreeSet::new();
        for text in &used.shown {
            for code in font.decode(text) {
                if font
                    .glyph_index(code)
                    .is_some_and(|glyph| glyph != NOTDEF_GLYPH)
                    || !reported.insert(code.value())
                {
                    continue;
                }
                findings.record(
                    font_place(used).named(used.name.clone()),
                    format!(
                        "the font {} is rendered with code {}, and its embedded program defines \
                         no glyph for it",
                        used.name,
                        code.value()
                    ),
                );
            }
        }
    }
}

// --------------------------------------------------------------------------------------------
// 6.2.10.5 — font metrics, in the one shape that can be read without the font program.
// --------------------------------------------------------------------------------------------

/// What both parts mean by consistent: a difference of no more than a thousandth of a
/// text-space unit (ISO 19005-2 §6.2.11.5, ISO 19005-4 §6.2.10.5).
const CONSISTENT: f64 = 0.001;

/// How many tokens of a glyph procedure are read looking for its first operator.
///
/// ISO 32000-2 §9.6.4 requires `d0` or `d1` to be the procedure's *first* operator, so a
/// procedure that has not reached one within this many tokens has not got one where the clause
/// puts it. The bound is what keeps a hostile `/CharProcs` stream from being read to its end
/// once per glyph.
const GLYPH_PROCEDURE_TOKENS: usize = 64;

/// The horizontal scale of a Type 3 font's `/FontMatrix`, which maps glyph space to text space.
///
/// ISO 32000-2 §9.6.4 makes the matrix required and arbitrary for a Type 3 font — that is the
/// whole of what distinguishes it from the 1/1000 every other font type is fixed at — so the
/// tolerance has to be applied after it rather than before. `None` where the entry is absent or
/// unreadable, because a rule stated in text-space units cannot be applied to a font whose
/// mapping into text space the file never gave.
fn font_matrix_scale(document: &Document, font: &Dictionary) -> Option<f64> {
    let matrix = document.get_key(font, "FontMatrix");
    document.resolve(matrix.as_array()?.first()?).as_number()
}

/// The codes a Type 3 font's `/Differences` array assigns a glyph name to.
///
/// ISO 32000-2 §9.6.4 makes `/Encoding` required for a Type 3 font and requires its
/// `/Differences` array to state the font's whole encoding, so this is the code-to-procedure
/// table rather than a partial view of one. §9.6.5.1 gives the array its shape: a code, then the
/// names of the glyphs at that code and the codes after it.
fn type3_encoding(document: &Document, font: &Dictionary) -> Vec<(i64, String)> {
    let mut out = Vec::new();
    let encoding = document.get_key(font, "Encoding");
    let Some(encoding) = encoding.as_dict() else {
        return out;
    };
    let differences = document.get_key(encoding, "Differences");
    let Some(items) = differences.as_array() else {
        return out;
    };
    let mut code = 0_i64;
    for item in items {
        match document.resolve(item) {
            Object::Integer(next) => code = next,
            Object::Name(name) => {
                if let Some(name) = named(Some(&name)) {
                    out.push((code, name));
                }
                code = code.saturating_add(1);
            }
            _ => {}
        }
    }
    out
}

/// The `w_x` operand of a glyph procedure's `d0` or `d1`, with which of the two it was.
///
/// ISO 32000-2 §9.6.4's Table 111 gives both operators the same first two operands — the glyph's
/// horizontal and vertical displacement in glyph space — so the width is the first number either
/// way, and `d1` differs only by the bounding box after it.
///
/// `None` where the procedure's first operator is neither, which is a base-standard fault rather
/// than this clause's, and where it states no operands at all.
fn glyph_procedure_width(bytes: &[u8]) -> Option<(&'static str, f64)> {
    let mut lexer = Lexer::new(bytes);
    let mut first: Option<f64> = None;
    for _ in 0..GLYPH_PROCEDURE_TOKENS {
        match lexer.next_token()? {
            // An operand no `i32` can hold is not a width; the procedure is read no further
            // rather than having the *next* number taken for its first one.
            Token::Integer(value) => {
                if first.is_none() {
                    first = Some(f64::from(i32::try_from(value).ok()?));
                }
            }
            Token::Real(value) => first = first.or(Some(value)),
            Token::Keyword(b"d0") => return Some(("d0", first?)),
            Token::Keyword(b"d1") => return Some(("d1", first?)),
            Token::Keyword(_) => return None,
            _ => {}
        }
    }
    None
}

/// ISO 19005-4 §6.2.10.5, second paragraph — part 4's own addition, which part 2 states nowhere.
///
/// A Type 3 font states each glyph's width twice: in the font dictionary's `/Widths` array, and
/// in the `d0` or `d1` operator its glyph procedure has to open with. The clause requires the two
/// to agree to within [`CONSISTENT`] of a text-space unit, which is why the `/FontMatrix` is
/// applied to the difference: both numbers are in the font's own glyph space, and only the matrix
/// says how large a unit of that is.
///
/// **The population is the fonts a content stream rendered**, which is the clause's own
/// condition, so a Type 3 font that is merely present in a resource dictionary is not reported.
///
/// A glyph whose procedure opens with something other than `d0` or `d1` is passed over: that is
/// ISO 32000-2 §9.6.4 being broken rather than this clause, and reporting it here would file the
/// fault under the wrong requirement.
fn type3_glyph_procedures_state_their_width(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for used in exam.survey().fonts() {
        if !used.rendered || subtype(document, &used.dict).as_deref() != Some("Type3") {
            continue;
        }
        let (Some(scale), Some(first)) = (
            font_matrix_scale(document, &used.dict),
            document.get_key(&used.dict, "FirstChar").as_integer(),
        ) else {
            continue;
        };
        let widths = document.get_key(&used.dict, "Widths");
        let procedures = document.get_key(&used.dict, "CharProcs");
        let (Some(widths), Some(procedures)) = (widths.as_array(), procedures.as_dict()) else {
            continue;
        };
        for (code, name) in type3_encoding(document, &used.dict) {
            let Some(stated) = usize::try_from(code.saturating_sub(first))
                .ok()
                .and_then(|at| widths.get(at))
                .and_then(|width| document.resolve(width).as_number())
            else {
                continue;
            };
            let Some(procedure) = procedures.get(name.as_str()) else {
                continue;
            };
            let Object::Stream(procedure) = document.resolve(procedure) else {
                continue;
            };
            let Some(bytes) = document.decoded_stream_data(&procedure) else {
                continue;
            };
            let Some((operator, drawn)) = glyph_procedure_width(&bytes) else {
                continue;
            };
            if ((stated - drawn) * scale).abs() > CONSISTENT {
                findings.record(
                    used.id.map_or_else(
                        || Where::page(used.page).named(name.clone()),
                        |id| Where::object(id).named(name.clone()),
                    ),
                    format!(
                        "the Type 3 glyph /{name} at code {code} is {stated} wide in the font's \
                         Widths array and {drawn} wide in its own {operator} operator"
                    ),
                );
            }
        }
    }
}

/// ISO 19005-2 §6.2.11.5, first paragraph; ISO 19005-4 §6.2.10.5, first paragraph.
///
/// A file states every glyph's advance twice — once in the font dictionary, where §9.6.2.1's
/// `/Widths` or §9.7.4.3's `/W` puts it, and once inside the embedded program — and both parts
/// require the two to agree to within [`CONSISTENT`]. ISO 32000-2 §9.6.2.1's Table 109 states
/// the same obligation on the producer:
///
/// > These widths shall be consistent with the actual widths given in the font program.
///
/// [`pdf_font::LoadedFont::advance`] is the first statement and
/// [`pdf_font::LoadedFont::program_advance`] the second; the second was added to `pdf-font` for
/// this rule, because asking `advance` twice would have compared the dictionary with itself and
/// passed every file.
///
/// # The population, and why one predicate serves two clauses that differ
///
/// The parts disagree about *which* glyphs, and part 4 is the weaker: §6.2.11.5 says only
/// "embedded … and used for rendering" and puts no condition on the glyph, while §6.2.10.5
/// narrows it to the glyphs referenced for rendering and exempts those referenced only in text
/// rendering mode 3. This checks the narrower population — the codes the survey saw shown, in a
/// font a content stream rendered — which is exactly part 4's and a subset of part 2's. Part 2
/// is therefore under-reported by the glyphs no page draws, which is the direction of error this
/// module always takes.
///
/// **One gap inside that, named rather than papered over.**
/// [`SelectedFont::rendered`](crate::survey::SelectedFont::rendered) is a fact about the *font*
/// — whether any text-showing operator ran with it outside mode 3 — and
/// [`SelectedFont::shown`](crate::survey::SelectedFont::shown) does not say which mode each byte
/// string was drawn in. So a part 4 font drawn in both modes has its mode-3 codes judged too,
/// where the clause exempts them. Closing it needs the survey to keep the two populations apart;
/// until it does, the exposure is a font that is rendered *and* draws a different, inconsistent
/// glyph invisibly.
///
/// # What is passed over, and why each one would otherwise accuse a sound file
///
/// [`font_with_its_own_program`] supplies half the answer: no program, a substituted face, a font
/// `pdf-font` refuses, or a Type 3 font (whose own rule is
/// [`type3_glyph_procedures_state_their_width`]). The other half is
/// [`pdf_font::LoadedFont::program_advance`]'s own contract — its `None` is never a finding here
/// — and two of its reasons matter to this rule in particular:
///
/// - **A code whose glyph selection lands on `.notdef`.** Its advance is a statement about a
///   character the program does not have, not about this one — and a file that shows it is
///   already failing [`no_notdef_glyph_shown`], under the clause that is about it.
/// - **A code the program states no advance for.** That is a fact about this reader — a bare
///   Type 1 program's `hsbw`, a repaired CFF — and naming the file for it would report our gap
///   as the producer's fault.
fn widths_agree_with_the_program(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for used in exam.survey().fonts() {
        if !used.rendered {
            continue;
        }
        let Some(font) = font_with_its_own_program(document, used) else {
            continue;
        };
        let mut reported = BTreeSet::new();
        for text in &used.shown {
            for code in font.decode(text) {
                if !reported.insert(code.value()) {
                    continue;
                }
                let Some(program) = font.program_advance(code) else {
                    continue;
                };
                let stated = font.advance(code);
                if f64::from(stated - program).abs() <= CONSISTENT {
                    continue;
                }
                findings.record(
                    font_place(used).named(used.name.clone()),
                    format!(
                        "the font {} draws code {}, which its dictionary makes {} wide and its \
                         embedded program {} wide",
                        used.name,
                        code.value(),
                        stated,
                        program
                    ),
                );
            }
        }
    }
}

/// ISO 19005-4 §6.2.10.5, third paragraph — part 4's second addition, absent from part 2.
///
/// A composite font shown in writing mode 1 has its vertical metrics in §9.7.4.3's `/DW2` and
/// `/W2`, and an embedded `OpenType` program may state the same quantity in its `vmtx` table.
/// Where both exist the clause requires them to agree to within [`CONSISTENT`].
///
/// The condition is threefold and each part is the clause's own: the font is composite, it is
/// *rendered* in vertical writing mode — which ISO 32000-2 §9.7.5.1 makes the `CMap`'s `/WMode`,
/// so [`pdf_font::LoadedFont::is_vertical`] is the question — and the program states vertical
/// metrics at all. The third is where most fonts leave: a face never meant to be set vertically
/// carries no `vmtx`, and [`pdf_font::LoadedFont::program_vertical_advance`] answers `None`,
/// which is an absence of a statement rather than a disagreement with one.
///
/// # Only the displacement is compared, and the position vector deliberately is not
///
/// §9.7.4.3 gives a vertical glyph two quantities: the displacement `w1`, whose vertical
/// component `/DW2`'s second number and `/W2`'s first state, and the position vector `v`, which
/// `/DW2`'s first number and `/W2`'s remaining two state. `vmtx` states the first outright, as an
/// advance height. It does not state the second: a program's vertical origin is `VORG` where the
/// font has one and otherwise the top side bearing plus the glyph's own `yMax`, which is a
/// quantity derived from two tables rather than one the program asserts. Comparing a derivation
/// against the file's assertion would report a font for arithmetic this crate chose, so the
/// clause's "information about vertical metrics" is read here as the one the program states.
/// The cost is that a `/DW2` or `/W2` whose *position* disagrees is not reported.
fn vertical_metrics_agree_with_the_program(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for used in exam.survey().fonts() {
        if !used.rendered {
            continue;
        }
        let Some(font) = font_with_its_own_program(document, used) else {
            continue;
        };
        if !font.is_vertical() {
            continue;
        }
        let mut reported = BTreeSet::new();
        for text in &used.shown {
            for code in font.decode(text) {
                if !reported.insert(code.value()) {
                    continue;
                }
                let Some(program) = font.program_vertical_advance(code) else {
                    continue;
                };
                let (displacement, _) = font.vertical_metrics(code);
                let stated = displacement[1];
                if f64::from(stated - program).abs() <= CONSISTENT {
                    continue;
                }
                findings.record(
                    font_place(used).named(used.name.clone()),
                    format!(
                        "the font {} draws code {} downwards, and its DW2 or W2 entry displaces \
                         the glyph by {} where its embedded program's vmtx states {}",
                        used.name,
                        code.value(),
                        stated,
                        program
                    ),
                );
            }
        }
    }
}

// --------------------------------------------------------------------------------------------
// 6.2.11.6 / 6.2.10.6 — character encodings.
// --------------------------------------------------------------------------------------------

/// ISO 19005-2 §6.2.11.6, ISO 19005-4 §6.2.10.6, second paragraph.
///
/// Applied to a TrueType font whose descriptor says it is non-symbolic. A font whose file
/// states no descriptor or no `/Flags` is passed over, because the rule's own subject is
/// undecided there.
fn non_symbolic_truetype_uses_a_standard_encoding(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_font(exam, |id, font| {
        if subtype(document, font).as_deref() != Some("TrueType")
            || is_symbolic(document, font) != Some(false)
        {
            return;
        }
        match base_encoding_name(document, font).as_deref() {
            Some("MacRomanEncoding" | "WinAnsiEncoding") => {}
            Some(other) => findings.record(
                Where::object(id).named(other.to_owned()),
                format!(
                    "a non-symbolic TrueType font's encoding is {other} rather than \
                     MacRomanEncoding or WinAnsiEncoding"
                ),
            ),
            None => findings.record(
                Where::object(id).named("Encoding"),
                "a non-symbolic TrueType font names neither MacRomanEncoding nor \
                 WinAnsiEncoding",
            ),
        }
    });
}

/// ISO 19005-2 §6.2.11.6, ISO 19005-4 §6.2.10.6, fourth paragraph.
fn symbolic_truetype_states_no_encoding(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_font(exam, |id, font| {
        if subtype(document, font).as_deref() != Some("TrueType")
            || is_symbolic(document, font) != Some(true)
        {
            return;
        }
        if font.get("Encoding").is_some() {
            findings.record(
                Where::object(id).named("Encoding"),
                "a symbolic TrueType font states an Encoding entry",
            );
        }
    });
}

/// ISO 19005-2 §6.2.11.6, ISO 19005-4 §6.2.10.6, third paragraph.
///
/// The clause permits a non-symbolic `TrueType` font to state a `/Differences` array only under
/// two conditions at once: every name in it is in the Adobe Glyph List, **and** the embedded
/// program carries the (3, 1) Microsoft Unicode `cmap` subtable. Failing either is failing the
/// rule, so the first condition alone decides a font that fails it.
///
/// **This is the first condition; the second is
/// [`non_symbolic_truetype_differences_need_the_unicode_cmap`]**, which is a row of its own
/// because the two bind different populations — a glyph name is readable from the font
/// dictionary, and the subtable is not.
///
/// # Which list of names, and why the wider one
///
/// [`pdf_font::encoding::text_for`] is the Adobe Glyph List Specification's own algorithm — a
/// variant suffix dropped, an underscore-joined name mapped component by component — rather than
/// a membership test against the list's literal rows. It answers for strictly more names than
/// the bare list does, so a font it declines is one no reading of the clause excuses, and the
/// error is in the direction this module always takes it.
fn non_symbolic_truetype_differences_are_listed_names(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    for_each_font(exam, |id, font| {
        if subtype(document, font).as_deref() != Some("TrueType")
            || is_symbolic(document, font) != Some(false)
        {
            return;
        }
        let encoding = document.get_key(font, "Encoding");
        let Some(encoding) = encoding.as_dict() else {
            return;
        };
        let differences = document.get_key(encoding, "Differences");
        let Some(items) = differences.as_array() else {
            return;
        };
        for item in items {
            let Object::Name(name) = document.resolve(item) else {
                continue;
            };
            let Some(name) = named(Some(&name)) else {
                continue;
            };
            if encoding::text_for(&name).is_none() {
                findings.record(
                    Where::object(id).named(name.clone()),
                    format!(
                        "a non-symbolic TrueType font's Differences array names /{name}, which \
                         the Adobe Glyph List does not hold"
                    ),
                );
            }
        }
    });
}

// --------------------------------------------------------------------------------------------
// The `cmap` subtable rules of 6.2.11.6 / 6.2.10.6, where the two parts ask different things.
// --------------------------------------------------------------------------------------------

/// The Microsoft Symbol subtable, ISO 32000-2 §9.6.5.4's (3, 0).
const MICROSOFT_SYMBOL: (u16, u16) = (3, 0);

/// The Microsoft Unicode subtable, §9.6.5.4's (3, 1).
const MICROSOFT_UNICODE: (u16, u16) = (3, 1);

/// The Macintosh Roman subtable, §9.6.5.4's (1, 0).
const MACINTOSH_ROMAN: (u16, u16) = (1, 0);

/// The `cmap` subtables a rendered simple `TrueType` font's own program carries.
///
/// The population every rule below shares, and it is narrower than the clause's in one way that
/// is written down rather than hidden: it is the fonts a content stream *selected*, because
/// [`font_with_its_own_program`] is what loads a program without standing a substitute in for it
/// and it is handed a [`SelectedFont`]. A `TrueType` font dictionary that no page uses is
/// therefore not asked. Both parts' first and fourth paragraphs are about a font "used for
/// rendering" anyway; what this misses is the general sentence of ISO 19005-2 §6.2.11.1 and
/// ISO 19005-4 §6.2.10.1, which extends a requirement to fonts shown only in mode 3.
///
/// `None` — nothing to judge — for every font whose program this tree could not read as an sfnt,
/// which is the module's standing rule: a program this reader declines is not a program whose
/// subtables are missing.
fn rendered_truetype_subtables<'a>(
    exam: &'a Examination<'a>,
    symbolic: bool,
) -> impl Iterator<Item = (&'a SelectedFont, Vec<(u16, u16)>)> {
    let document = exam.document;
    exam.survey().fonts().filter_map(move |used| {
        if !used.rendered
            || subtype(document, &used.dict).as_deref() != Some("TrueType")
            || is_symbolic(document, &used.dict) != Some(symbolic)
        {
            return None;
        }
        let font = font_with_its_own_program(document, used)?;
        Some((used, font.program_cmap_subtables()?))
    })
}

/// ISO 19005-2 §6.2.11.6, ISO 19005-4 §6.2.10.6, first paragraph.
///
/// **The one place in this subclause where the two parts genuinely ask different questions**, so
/// this is one of the few predicates that reads [`Examination::target`]:
///
/// - **Part 4** names the two subtables it will accept, Microsoft Unicode (3, 1) or Macintosh
///   Roman (1, 0), and requires the program to carry at least one of them.
/// - **Part 2** asks instead for one or more *non-symbolic* `cmap` entries. §9.6.5.4 makes
///   exactly one of the subtables it names symbolic — Microsoft Symbol (3, 1)'s companion (3, 0),
///   which the same clause's first rule reaches only when the font descriptor's symbolic flag is
///   set — so a non-symbolic entry is read here as any entry that is not (3, 0). A program
///   carrying nothing but the symbolic subtable is what the paragraph excludes, and it is what
///   this reports.
///
/// The clause's closing condition — that the subtables be such that all necessary glyph lookups
/// can be carried out — is **not** checked, and that is deliberate rather than an omission.
/// Whether a lookup succeeds is [`embedded_programs_define_every_glyph_shown`]'s question, asked
/// per code under the clause that is about it; what this paragraph adds beyond that is the
/// requirement on the *inventory*, which is what a file has to satisfy before any lookup is
/// possible at all.
fn non_symbolic_truetype_program_maps_every_code(exam: &Examination<'_>, findings: &mut Findings) {
    for (used, subtables) in rendered_truetype_subtables(exam, false) {
        let acceptable = match exam.target.part() {
            Part::Four => {
                subtables.contains(&MICROSOFT_UNICODE) || subtables.contains(&MACINTOSH_ROMAN)
            }
            Part::Two => subtables.iter().any(|entry| *entry != MICROSOFT_SYMBOL),
        };
        if acceptable {
            continue;
        }
        findings.record(
            font_place(used).named(used.name.clone()),
            format!(
                "the non-symbolic TrueType font {} is rendered, and its embedded program's cmap \
                 carries {}",
                used.name,
                subtables_as_words(&subtables)
            ),
        );
    }
}

/// ISO 19005-2 §6.2.11.6, ISO 19005-4 §6.2.10.6, fourth paragraph, second half.
///
/// The first half — that a symbolic `TrueType` font state no `/Encoding` — is
/// [`symbolic_truetype_states_no_encoding`], which needs no font program and so binds every font
/// dictionary rather than only the rendered ones. This is the half that reads the program, and
/// here too the parts differ:
///
/// - **Part 4** names Microsoft Symbol (3, 0) or Macintosh Roman (1, 0) and requires one of them.
/// - **Part 2** accepts either of two shapes: a `cmap` carrying exactly one subtable, whatever it
///   is, or one carrying at least the Microsoft Symbol (3, 0) subtable. The first is the older
///   convention for a symbolic face — a font with one encoding has no ambiguity about which
///   subtable a code goes to — and it is stated in the clause rather than inferred.
fn symbolic_truetype_program_has_a_usable_cmap(exam: &Examination<'_>, findings: &mut Findings) {
    for (used, subtables) in rendered_truetype_subtables(exam, true) {
        let acceptable = match exam.target.part() {
            Part::Four => {
                subtables.contains(&MICROSOFT_SYMBOL) || subtables.contains(&MACINTOSH_ROMAN)
            }
            Part::Two => subtables.len() == 1 || subtables.contains(&MICROSOFT_SYMBOL),
        };
        if acceptable {
            continue;
        }
        findings.record(
            font_place(used).named(used.name.clone()),
            format!(
                "the symbolic TrueType font {} is rendered, and its embedded program's cmap \
                 carries {}",
                used.name,
                subtables_as_words(&subtables)
            ),
        );
    }
}

/// ISO 19005-2 §6.2.11.6, ISO 19005-4 §6.2.10.6, third paragraph, second condition.
///
/// The paragraph permits a `/Differences` array only where *both* conditions hold, and
/// [`non_symbolic_truetype_differences_are_listed_names`] is the first of them. This is the
/// second: the embedded program has to carry the Microsoft Unicode (3, 1) subtable, which both
/// parts name identically and neither qualifies.
///
/// It is a second predicate rather than a second loop inside the first because the two
/// conditions have different populations. A glyph name can be judged from the font dictionary
/// alone, so that one binds every dictionary the cross-reference table reaches; this one needs
/// the program, so it binds the rendered fonts [`rendered_truetype_subtables`] can load. Merging
/// them would have quietly narrowed the first to the second's population.
fn non_symbolic_truetype_differences_need_the_unicode_cmap(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    for (used, subtables) in rendered_truetype_subtables(exam, false) {
        let encoding = document.get_key(&used.dict, "Encoding");
        let Some(encoding) = encoding.as_dict() else {
            continue;
        };
        if document
            .get_key(encoding, "Differences")
            .as_array()
            .is_none()
            || subtables.contains(&MICROSOFT_UNICODE)
        {
            continue;
        }
        findings.record(
            font_place(used).named(used.name.clone()),
            format!(
                "the non-symbolic TrueType font {} states a Differences array, and its embedded \
                 program's cmap carries {} rather than the Microsoft Unicode (3, 1) subtable",
                used.name,
                subtables_as_words(&subtables)
            ),
        );
    }
}

/// A `cmap` inventory as a sentence a person can act on.
///
/// The pairs are printed as §9.6.5.4 writes them, and an empty `cmap` is said in words rather
/// than shown as an empty list, because "no subtable at all" is the finding in that case.
fn subtables_as_words(subtables: &[(u16, u16)]) -> String {
    if subtables.is_empty() {
        return "no subtable at all".to_owned();
    }
    let named: Vec<String> = subtables
        .iter()
        .map(|(platform, encoding)| format!("({platform}, {encoding})"))
        .collect();
    named.join(", ")
}

// --------------------------------------------------------------------------------------------
// 6.2.11.7 / 6.2.10.7 and 6.2.10.8 — Unicode, which is where the parts separate.
// --------------------------------------------------------------------------------------------

/// The character collections ISO 19005-2 §6.2.11.7.2 and ISO 19005-4 §6.2.10.7 exempt.
///
/// **The union of the two lists, deliberately.** Part 2 names Adobe-Korea1 and part 4 names
/// Adobe-KR. Exempting on either keeps the check from reporting a font one of the parts would
/// have excused — the module's standing direction of error.
///
/// **The reason recorded here used to be that a predicate could not know which part it was
/// judging, and that has not been true since [`Examination::target`] arrived**; the rules under
/// §6.2.11.6 / §6.2.10.6 read it. So the union is a choice rather than a limit, and it is the
/// looser of the two available answers: narrowing it to the part's own list would report a
/// part 2 file whose `CIDFont` states Adobe-KR, an ordering ISO 19005-2 predates. Whether that
/// is the right reading of §6.2.11.7.2 is a question for the clause, and it is open.
static EXEMPT_ORDERINGS: &[&str] = &["GB1", "CNS1", "Japan1", "Korea1", "KR"];

/// Whether a glyph name is in either list ISO 19005's second `/ToUnicode` exemption names.
///
/// The clause's two lists are the Adobe Glyph List and the set of named characters in the Symbol
/// font, which ISO 32000-2's Annex D prints; [`pdf_font::encoding::text_for`] answers for the
/// first (by the list's own algorithm, so a ligature or a variant suffix resolves) and
/// [`SymbolicEncoding::Symbol`] carries the second.
fn name_is_in_either_list(name: &str) -> bool {
    encoding::text_for(name).is_some() || SymbolicEncoding::Symbol.character_for(name).is_some()
}

/// Whether a Type 1, multiple-master or Type 3 font draws a glyph whose name is in neither list.
///
/// This is the second exemption of ISO 19005-2 §6.2.11.7.2 being **ruled out**, and only that:
/// `false` means the exemption stands or could not be settled, and the two are deliberately the
/// same answer here. The exemption asks about "the glyphs referenced", so the population is the
/// codes the content streams showed ([`crate::survey::SelectedFont::shown`]) rather than the
/// codes the encoding could reach.
///
/// # How the name is obtained, and why the two subtypes take different routes
///
/// - **A Type 3 font's names are in the file.** ISO 32000-2 §9.6.4 requires its `/Encoding` to
///   state the whole encoding in a `/Differences` array, so the array *is* the code-to-name table.
/// - **A Type 1 font's may not be.** With no `/Encoding` the names are the font program's own
///   built-in encoding, which is `pdf-font`'s to read.
///   [`LoadedFont::selected_glyph_name`] is that table, and it is the *name* rather than a
///   reading of it — which is the distinction this predicate turns on.
///
/// **It used to ask [`LoadedFont::naming_gap`] instead, and that was wrong for a reason worth
/// keeping.** `naming_gap` answers what §9.10.2 made of a code, and the clause ends by
/// permitting a processor to choose a character where its three methods fail. `pdf-font` takes
/// that permission — a Type 1 code whose name is `integraldisplay` comes back as the character
/// the *code* would be in ASCII — so the gap is `None` and the unlisted name has been hidden by
/// the very sentence that says nothing could name it. ISO 19005-2 §6.2.11.7.2's exemption is
/// about the name, not about whether a reader found something to say, so the name is what is
/// asked for. The Symbol set is consulted beside the Adobe Glyph List because the clause names
/// both and §9.10.2 names only the first.
///
/// `.notdef` is passed over: it is §9.6.5.2's substitute for a glyph the font does not have
/// rather than a glyph the content referenced, and drawing it is ISO 19005-2 §6.2.11.8's
/// subject — [`no_notdef_glyph_shown`] reports it there, and reporting it here as well would
/// state one fault twice under two clauses.
///
/// A substituted font answers for the substitute's names rather than the file's, so it is passed
/// over — the same guard [`font_with_its_own_program`] applies, for the same reason.
fn references_an_unlisted_glyph_name(exam: &Examination<'_>, id: ObjectId, subtype: &str) -> bool {
    let document = exam.document;
    let Some(used) = exam
        .survey()
        .fonts()
        .find(|used| used.id == Some(id) && used.shown_complete)
    else {
        // A font no content stream selected references no glyph, so nothing rules the
        // exemption out; a font whose strings overran the survey's budget is not one whose
        // referenced names are all known.
        return false;
    };
    if subtype == "Type3" {
        let names: std::collections::BTreeMap<i64, String> =
            type3_encoding(document, &used.dict).into_iter().collect();
        return used.shown.iter().flatten().any(|byte| {
            names
                .get(&i64::from(*byte))
                .is_some_and(|name| !name_is_in_either_list(name))
        });
    }
    let Ok(font) = LoadedFont::load(document, &used.dict, &used.name) else {
        return false;
    };
    if font.is_substituted() {
        return false;
    }
    used.shown.iter().any(|text| {
        font.decode(text).into_iter().any(|code| {
            font.selected_glyph_name(code)
                .is_some_and(|name| name != ".notdef" && !name_is_in_either_list(name))
        })
    })
}

/// ISO 19005-2 §6.2.11.7.2, and part 2 only: ISO 19005-4 §6.2.10.7 states the same rule with
/// `should`, which binds nobody.
///
/// # Which fonts this reports, and which it will not
///
/// The clause excuses four kinds of font, and all four are ruled out here rather than three:
/// [`references_an_unlisted_glyph_name`] settles the second — a Type 1 or Type 3 font all of
/// whose referenced glyph names are in the Adobe Glyph List or the Symbol set — for the fonts
/// whose names this tree can establish, and answers "exempt" for the rest. So what is reported
/// is still only the fonts that are *certainly* not exempt: a composite font over a character
/// collection the clause does not name, a symbolic TrueType font with no predefined encoding,
/// and a Type 1 or Type 3 font that draws a glyph neither list holds.
///
/// A `CIDFont` is skipped outright: ISO 32000-2 §9.10.3 puts `/ToUnicode` on the Type 0 font
/// dictionary, and a descendant is not a font a content stream can select.
fn to_unicode_present(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_font(exam, |id, font| {
        let Some(subtype) = subtype(document, font) else {
            return;
        };
        if font.get("ToUnicode").is_some() {
            return;
        }
        if matches!(
            base_encoding_name(document, font).as_deref(),
            Some("MacRomanEncoding" | "MacExpertEncoding" | "WinAnsiEncoding")
        ) {
            return;
        }
        match subtype.as_str() {
            // A TrueType font is exempt when it is non-symbolic, and a font whose descriptor
            // never said which it is settles nothing either way.
            "TrueType" => {
                if is_symbolic(document, font) != Some(true) {
                    return;
                }
            }
            // A composite font is exempt over four character collections, so the ordering has
            // to be legible before the exemption can be ruled out.
            "Type0" => {
                let Some((_, cid_font)) = descendant(document, font) else {
                    return;
                };
                let Object::Dictionary(info) = document.get_key(&cid_font, "CIDSystemInfo") else {
                    return;
                };
                let Some(ordering) = document
                    .get_key(&info, "Ordering")
                    .as_string()
                    .map(<[u8]>::to_vec)
                else {
                    return;
                };
                if EXEMPT_ORDERINGS
                    .iter()
                    .any(|exempt| exempt.as_bytes() == ordering)
                {
                    return;
                }
            }
            // The clause names Type 1 and Type 3; a multiple-master font is a Type 1 font
            // with interpolated designs (ISO 32000-2 §9.6.3) and its glyph names are Type 1
            // names, so it takes the same route.
            "Type1" | "MMType1" | "Type3" => {
                if !references_an_unlisted_glyph_name(exam, id, &subtype) {
                    return;
                }
            }
            // A descendant CIDFont is not a font a content stream selects, and a subtype the
            // base standard does not define is not one this clause describes.
            _ => return,
        }
        findings.record(
            Where::object(id).named("ToUnicode"),
            format!("a {subtype} font outside the clause's exemptions states no ToUnicode CMap"),
        );
    });
}

/// The three Unicode values ISO 19005-2 §6.2.11.7.2 and ISO 19005-4 §6.2.10.7 forbid.
///
/// Zero because the clause asks for values greater than it, and the other two because they are
/// the byte-order mark and its byte-swapped twin: a `/ToUnicode` destination is UTF-16BE
/// (ISO 32000-2 §9.10.3), so either of those is a producer having written the mark where the
/// character belonged.
static UNUSABLE_VALUES: [char; 3] = ['\u{0}', '\u{FEFF}', '\u{FFFE}'];

/// ISO 19005-2 §6.2.11.7.2's last sentence, ISO 19005-4 §6.2.10.7's last sentence.
///
/// Both parts state this one as a `shall` — part 4 conditions it on a `/ToUnicode` being present
/// at all, which is the same population, since a font without one states no values. It is the
/// one rule of §6.2.10.7 that binds a part 4 file, the rest of that subclause being `should`.
///
/// # Reading the values off, which is what the clause asks
///
/// The rule is about every value the `CMap` **states**, so the map is read rather than
/// interrogated: [`ToUnicode::mappings`] hands back each statement in the form the producer
/// wrote it, and a `beginbfrange` span is one statement rather than up to sixty-five thousand.
/// A span is judged by arithmetic — a forbidden value falls in it exactly when it lies between
/// the span's first scalar and that scalar plus the span's width — so a `<0000> <FFFF>` line
/// costs three comparisons and the code that maps to the offending value is still named.
///
/// **This used to ask the map about every code a font's code space holds**, 256 or 65 536 of
/// them under a document-wide bound, which was exact for no font: §9.7.6.2 lets a code be one
/// to four bytes, so the space is four billion wide and the walk stopped at two bytes. Reading
/// the statements covers three- and four-byte codes as well, and needs no budget, because the
/// number of statements is what `ToUnicode`'s own parse limits already bound.
///
/// A descendant `CIDFont` is passed over with the rest: §9.10.3 puts `/ToUnicode` on the Type 0
/// dictionary, so a `/ToUnicode` on a descendant is not a map any code reaches.
fn to_unicode_values_are_usable(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_font(exam, |id, font| {
        let Object::Stream(stream) = document.get_key(font, "ToUnicode") else {
            return;
        };
        let Some(bytes) = document.decoded_stream_data(&stream) else {
            // A CMap this tree cannot decode is not a CMap whose values are unusable.
            return;
        };
        let map = ToUnicode::parse(&bytes);
        // One finding per offending value rather than per code, because a `bfrange` spanning a
        // hundred codes onto U+0000 is one thing the producer did wrong and not a hundred.
        let mut reported = [false; UNUSABLE_VALUES.len()];
        for mapping in map.mappings() {
            match mapping {
                Mapping::Single { code, text } => {
                    for character in text.chars() {
                        report_unusable_value(id, code, character, &mut reported, findings);
                    }
                }
                Mapping::Span { low, high, first } => {
                    // The span runs `first` upwards, one scalar per code, so a forbidden value
                    // is stated exactly when it lies within that many steps of the first.
                    let last = first.saturating_add(high.saturating_sub(low));
                    for &character in &UNUSABLE_VALUES {
                        let value = u32::from(character);
                        if value < first || value > last {
                            continue;
                        }
                        let code = low.saturating_add(value.saturating_sub(first));
                        report_unusable_value(id, code, character, &mut reported, findings);
                    }
                }
            }
        }
    });
}

/// Records one forbidden value the once, whichever code and statement reached it.
///
/// `reported` is per font rather than per document, so two fonts stating U+FEFF are two faults;
/// the same font stating it for a hundred codes is one.
fn report_unusable_value(
    id: ObjectId,
    code: u32,
    character: char,
    reported: &mut [bool; UNUSABLE_VALUES.len()],
    findings: &mut Findings,
) {
    let Some(at) = UNUSABLE_VALUES.iter().position(|it| *it == character) else {
        return;
    };
    let Some(seen) = reported.get_mut(at) else {
        return;
    };
    if *seen {
        return;
    }
    *seen = true;
    findings.record(
        Where::object(id).named("ToUnicode"),
        format!(
            "a ToUnicode CMap maps code {code} to U+{:04X}, which the clause names as a \
             placeholder rather than a usable value",
            u32::from(character)
        ),
    );
}

/// Unicode's three Private Use Areas, as inclusive scalar-value bounds.
///
/// The same three ranges [`is_private_use`] matches, in the form a `beginbfrange` span has to be
/// intersected with: a span states its values arithmetically, so the question "does this span
/// reach the area" is answered by arithmetic rather than by visiting each code.
static PRIVATE_USE_AREAS: [(u32, u32); 3] = [
    (0xE000, 0xF8FF),
    (0xF_0000, 0xF_FFFD),
    (0x10_0000, 0x10_FFFD),
];

/// Whether a character is in one of Unicode's three Private Use Areas.
const fn is_private_use(character: char) -> bool {
    matches!(character,
        '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}')
}

/// The Private Use Area codes one font's `/ToUnicode` states, and where the first of each is.
///
/// Returns the character code and the private-use scalar it stands for, one pair per statement
/// the `CMap` makes that reaches the area — a `beginbfrange` span contributes the first of its
/// codes that does, rather than every one, because a span onto the area is one thing the
/// producer wrote.
fn private_use_codes(map: &ToUnicode) -> Vec<(u32, u32)> {
    /// Bounds what one font contributes, so a `CMap` mapping the whole area cannot fill a report.
    const MAX_PER_FONT: usize = 64;

    let mut out = Vec::new();
    for mapping in map.mappings() {
        if out.len() >= MAX_PER_FONT {
            break;
        }
        match mapping {
            Mapping::Single { code, text } => {
                if let Some(character) = text.chars().find(|it| is_private_use(*it)) {
                    out.push((code, u32::from(character)));
                }
            }
            Mapping::Span { low, high, first } => {
                let last = first.saturating_add(high.saturating_sub(low));
                if let Some(value) = PRIVATE_USE_AREAS
                    .iter()
                    .filter_map(|(area_low, area_high)| {
                        let start = first.max(*area_low);
                        (start <= last.min(*area_high)).then_some(start)
                    })
                    .min()
                {
                    out.push((low.saturating_add(value.saturating_sub(first)), value));
                }
            }
        }
    }
    out
}

/// Whether the document states an `ActualText` entry anywhere this crate can reach one.
///
/// ISO 32000-2 §14.9.4 puts the entry in two places, and both are read: a structure element's
/// dictionary or a marked-content property list, the latter either an object of its own or
/// written straight into a `BDC` operator's operands. So this asks the objects a cross-reference
/// section names *and* [`crate::survey::Survey::names_actual_text`] — the second is the only
/// route to an inline property list, and the corpus's passing witnesses are mostly of that shape.
///
/// # The measurement that moved the second half onto the survey
///
/// This used to decode every content stream again and lex it, and `examples/cost.rs` said what
/// that was worth: [`actual_text_states_no_private_use`] cost about 180 ms of object walk plus
/// 118 ms of content on ISO 32000-2's 1023-page specification, which made it the dearest single
/// requirement in a 2.5 s part 4 report — for a second decode of streams the survey had already
/// decoded. The survey now records the two facts as it goes and this reads them, which takes the
/// content half to nothing.
///
/// **Deliberately an over-approximation.** It answers "does the file say `ActualText` at all",
/// not "does it say it about the right character", and it exists so that
/// [`actual_text_covers_private_use_characters`] can report the case where the answer is *no* —
/// the one case in which no span analysis is needed to be certain.
fn states_any_actual_text(exam: &Examination<'_>) -> bool {
    /// How deep a direct dictionary is followed. An indirect one is an object of its own and is
    /// reached by the outer walk, so this only has to cover what a producer writes inline.
    const MAX_DEPTH: u32 = 8;

    fn in_object(object: &Object, depth: u32) -> bool {
        if depth >= MAX_DEPTH {
            return false;
        }
        match object {
            Object::Dictionary(dict) => in_dictionary(dict, depth),
            Object::Stream(stream) => in_dictionary(&stream.dict, depth),
            Object::Array(items) => items
                .iter()
                .any(|item| in_object(item, depth.saturating_add(1))),
            _ => false,
        }
    }

    fn in_dictionary(dict: &Dictionary, depth: u32) -> bool {
        dict.get("ActualText").is_some()
            || dict
                .iter()
                .any(|(_, value)| in_object(value, depth.saturating_add(1)))
    }

    if exam
        .objects()
        .iter()
        .any(|(_, object)| in_object(object, 0))
    {
        return true;
    }
    exam.survey().names_actual_text()
}

/// ISO 19005-2 §6.2.11.7.3, and part 2 Level A only: part 4 states no counterpart to it.
///
/// A character a font maps into the Private Use Area means nothing on its own — the area is by
/// definition unassigned — so the clause requires an `ActualText` entry saying what it stands
/// for, either for that character or for a sequence containing it.
///
/// # What this reports, and the half it declines
///
/// Two facts make the clause decidable, and this crate has one of them:
///
/// - **Which shown codes map into the area** is read off the font's own `/ToUnicode` with
///   [`ToUnicode::mappings`], against the codes [`crate::survey::SelectedFont::shown`] recorded —
///   every rendering mode included, which is what the clause says in as many words.
/// - **Whether a given `ActualText` covers a given character** needs the `BDC`/`EMC` spans and
///   which string was drawn inside which, and `crate::survey` records neither.
///
/// So the predicate reports the case the second fact is not needed for: a private-use character
/// is shown and the file states **no** `ActualText` at all, where no span analysis can make one
/// cover it. Where the file does state one somewhere, this stays silent — including where the
/// entry covers some other character, which is a real failure this cannot yet tell from a real
/// pass. The corpus has a witness of each, and the silent one is named here rather than counted
/// as met.
///
/// A font whose `shown` set overran the survey's budget is still read: the question is whether
/// *any* private-use character was shown, so a prefix can establish it and can only under-report.
fn actual_text_covers_private_use_characters(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let mut offences: Vec<(ObjectId, u32, u32)> = Vec::new();
    for used in exam.survey().fonts() {
        let Some(id) = used.id else {
            continue;
        };
        let Object::Stream(stream) = document.get_key(&used.dict, "ToUnicode") else {
            continue;
        };
        let Some(bytes) = document.decoded_stream_data(&stream) else {
            continue;
        };
        let private = private_use_codes(&ToUnicode::parse(&bytes));
        if private.is_empty() {
            continue;
        }
        let Ok(font) = LoadedFont::load(document, &used.dict, &used.name) else {
            continue;
        };
        let shown: BTreeSet<u32> = used
            .shown
            .iter()
            .flat_map(|text| font.decode(text))
            .map(pdf_font::Code::value)
            .collect();
        offences.extend(
            private
                .into_iter()
                .filter(|(code, _)| shown.contains(code))
                .map(|(code, value)| (id, code, value)),
        );
    }
    // Asked only once something was found, because it decodes every content stream the document
    // has and a document with no private-use character owes nothing for the answer.
    if offences.is_empty() || states_any_actual_text(exam) {
        return;
    }
    for (id, code, value) in offences {
        findings.record(
            Where::object(id).named("ToUnicode"),
            format!(
                "code {code} is shown and maps to U+{value:04X}, which is in the Unicode Private \
                 Use Area, and the file states no ActualText entry anywhere"
            ),
        );
    }
}

/// ISO 19005-4 §6.2.10.8, last sentence — the one font rule part 4 states and part 2 does not.
///
/// The surrounding paragraph recommends an `ActualText` for a private-use character with
/// `should`, so it binds nothing; this sentence is a `shall` and says the replacement text may
/// not itself be private-use, which would make the substitution circular.
///
/// ISO 32000-2 §14.9.4 puts the entry in two places and both are read: a structure element's
/// dictionary, and a marked-content property list — which may be an object of its own or written
/// straight into the `BDC` operator's operands. The second form is why
/// [`crate::survey::Survey::inline_actual_texts`] is read as well as the objects; a corpus
/// witness of each shape exists, and reading only the objects passed the inline one.
fn actual_text_states_no_private_use(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        let dict = match document.get(id) {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => stream.dict.clone(),
            _ => continue,
        };
        if dict.get("ActualText").is_none() {
            continue;
        }
        let Some(bytes) = document
            .get_key(&dict, "ActualText")
            .as_string()
            .map(<[u8]>::to_vec)
        else {
            continue;
        };
        if let Some(character) = text_string(&bytes).chars().find(|it| is_private_use(*it)) {
            findings.record(
                Where::object(id).named("ActualText"),
                format!(
                    "an ActualText entry contains U+{:04X}, which is in the Unicode Private Use \
                     Area",
                    u32::from(character)
                ),
            );
        }
    }
    for (page, bytes) in exam.survey().inline_actual_texts() {
        if let Some(character) = text_string(bytes).chars().find(|it| is_private_use(*it)) {
            findings.record(
                Where::page(*page).named("ActualText"),
                format!(
                    "an ActualText entry written into a marked-content operator contains \
                     U+{:04X}, which is in the Unicode Private Use Area",
                    u32::from(character)
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Examination;
    use crate::{Flavour, Target};
    use std::fmt::Write as _;

    use pdf_font::tounicode::ToUnicode;
    use pdf_syntax::Document;

    use super::{
        Findings, actual_text_states_no_private_use, charset_names,
        cid_system_info_agrees_with_the_cmap, cid_to_gid_map_present, cmap_embedded_or_predefined,
        embedded_cmap_states_its_own_write_mode, font_programs_embedded, glyph_procedure_width,
        is_private_use, non_symbolic_truetype_differences_are_listed_names,
        non_symbolic_truetype_uses_a_standard_encoding, private_use_codes,
        symbolic_truetype_states_no_encoding, to_unicode_present, to_unicode_values_are_usable,
        type3_glyph_procedures_state_their_width,
    };

    /// Wraps numbered objects in a header, a cross-reference table and a trailer.
    ///
    /// The same shape `tests/verdict.rs` uses, kept here because a predicate that walks the
    /// cross-reference table needs a document whose table is real.
    fn document(body: &str) -> Document {
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for object in body.split_inclusive("endobj\n") {
            offsets.push(out.len());
            out.push_str(object);
        }
        let xref_at = out.len();
        let size = offsets.len().saturating_add(1);
        let _ = writeln!(out, "xref\n0 {size}");
        out.push_str("0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("the fixture is a valid PDF")
    }

    /// A one-page document whose page draws `content` with the font objects in `extra`.
    fn page_with(content: &str, extra: &str) -> Document {
        let length = content.len();
        document(&format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length {length} >>\nstream\n{content}\nendstream\nendobj\n{extra}"
        ))
    }

    fn findings(
        document: &Document,
        predicate: fn(&Examination<'_>, &mut Findings),
    ) -> Vec<String> {
        let mut found = Findings::default();
        let exam = Examination::new(document, Target::Four(Flavour::Plain));
        predicate(&exam, &mut found);
        found
            .kept()
            .iter()
            .map(|finding| finding.what.clone())
            .collect()
    }

    #[test]
    fn a_rendered_font_with_no_program_is_reported_and_a_mode_three_one_is_not() {
        let helvetica = "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n";

        let drawn = page_with("BT /F1 12 Tf (hi) Tj ET", helvetica);
        assert_eq!(
            findings(&drawn, font_programs_embedded).len(),
            1,
            "a standard-14 font shown normally has no exemption"
        );

        let invisible = page_with("BT /F1 12 Tf 3 Tr (hi) Tj ET", helvetica);
        assert!(
            findings(&invisible, font_programs_embedded).is_empty(),
            "a font shown only in rendering mode 3 is exempt from embedding"
        );

        let clipped = page_with("BT /F1 12 Tf 7 Tr (hi) Tj ET", helvetica);
        assert_eq!(
            findings(&clipped, font_programs_embedded).len(),
            1,
            "mode 7 adds the glyphs to the clipping path, so it is not mode 3's exemption"
        );

        let never_shown = page_with("BT /F1 12 Tf ET", helvetica);
        assert!(
            findings(&never_shown, font_programs_embedded).is_empty(),
            "a font no text-showing operator ran with is not a font the file uses"
        );
    }

    #[test]
    fn the_rendering_mode_is_restored_with_the_graphics_state() {
        let helvetica = "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n";
        // `Tr 3` is set inside a `q`/`Q` pair, so the show operator after `Q` is back in mode 0.
        let restored = page_with("q 3 Tr Q BT /F1 12 Tf (hi) Tj ET", helvetica);
        assert_eq!(
            findings(&restored, font_programs_embedded).len(),
            1,
            "Q restored the rendering mode the file set before q"
        );
    }

    #[test]
    fn a_font_reached_only_through_a_form_xobject_is_still_used() {
        let inner = "BT /F9 12 Tf (hi) Tj ET";
        let inner_length = inner.len();
        let outer = "/X1 Do";
        let outer_length = outer.len();
        let drawn = document(&format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Resources << /XObject << /X1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length {outer_length} >>\nstream\n{outer}\nendstream\nendobj\n\
             5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] \
             /Resources << /Font << /F9 6 0 R >> >> /Length {inner_length} >>\n\
             stream\n{inner}\nendstream\nendobj\n\
             6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n"
        ));
        assert_eq!(
            findings(&drawn, font_programs_embedded).len(),
            1,
            "the page draws nothing itself; the font is selected inside the form it invokes"
        );
    }

    #[test]
    fn a_cid_font_without_a_cid_to_gid_map_is_reported_only_when_it_is_embedded() {
        let embedded = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /CIDFontType2 /FontDescriptor 3 0 R >>\nendobj\n\
             3 0 obj\n<< /Type /FontDescriptor /FontFile2 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n",
        );
        assert_eq!(findings(&embedded, cid_to_gid_map_present).len(), 1);

        let not_embedded = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /CIDFontType2 /FontDescriptor 3 0 R >>\nendobj\n\
             3 0 obj\n<< /Type /FontDescriptor >>\nendobj\n",
        );
        assert!(
            findings(&not_embedded, cid_to_gid_map_present).is_empty(),
            "the clause binds embedded Type 2 CIDFonts"
        );

        let identity = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /CIDFontType2 /FontDescriptor 3 0 R \
             /CIDToGIDMap /Identity >>\nendobj\n\
             3 0 obj\n<< /Type /FontDescriptor /FontFile2 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n",
        );
        assert!(findings(&identity, cid_to_gid_map_present).is_empty());
    }

    #[test]
    fn an_unlisted_cmap_name_is_reported_and_a_listed_one_is_not() {
        let unlisted = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /Type0 /Encoding /Nonesuch-H >>\nendobj\n",
        );
        assert_eq!(findings(&unlisted, cmap_embedded_or_predefined).len(), 1);

        let listed = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /Type0 /Encoding /Identity-H >>\nendobj\n",
        );
        assert!(findings(&listed, cmap_embedded_or_predefined).is_empty());
    }

    #[test]
    fn a_cid_system_info_is_compared_only_against_an_embedded_cmap() {
        let mismatched = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /Type0 /Encoding 3 0 R \
             /DescendantFonts [4 0 R] >>\nendobj\n\
             3 0 obj\n<< /Type /CMap /Length 0 \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 4 >> >>\n\
             stream\n\nendstream\nendobj\n\
             4 0 obj\n<< /Type /Font /Subtype /CIDFontType0 \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 2 >> >>\nendobj\n",
        );
        let found = findings(&mismatched, cid_system_info_agrees_with_the_cmap);
        assert_eq!(
            found.len(),
            2,
            "the orderings differ and the supplement is below the CMap's: {found:?}"
        );

        let predefined = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /Type0 /Encoding /UniJIS-UCS2-H \
             /DescendantFonts [3 0 R] >>\nendobj\n\
             3 0 obj\n<< /Type /Font /Subtype /CIDFontType0 \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 0 >> >>\nendobj\n",
        );
        assert!(
            findings(&predefined, cid_system_info_agrees_with_the_cmap).is_empty(),
            "a predefined CMap's own CIDSystemInfo is not in the file, so nothing is compared"
        );
    }

    #[test]
    fn the_truetype_encoding_rules_turn_on_the_symbolic_flag() {
        // Bit position 3 is the Symbolic flag, so 4 is symbolic and 32 (bit 6) is not.
        let symbolic_with_encoding = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /TrueType /FontDescriptor 3 0 R \
             /Encoding /WinAnsiEncoding >>\nendobj\n\
             3 0 obj\n<< /Type /FontDescriptor /Flags 4 >>\nendobj\n",
        );
        assert_eq!(
            findings(
                &symbolic_with_encoding,
                symbolic_truetype_states_no_encoding
            )
            .len(),
            1
        );
        assert!(
            findings(
                &symbolic_with_encoding,
                non_symbolic_truetype_uses_a_standard_encoding
            )
            .is_empty(),
            "the non-symbolic rule does not reach a symbolic font"
        );

        let non_symbolic_odd_encoding = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /TrueType /FontDescriptor 3 0 R \
             /Encoding /MacExpertEncoding >>\nendobj\n\
             3 0 obj\n<< /Type /FontDescriptor /Flags 32 >>\nendobj\n",
        );
        assert_eq!(
            findings(
                &non_symbolic_odd_encoding,
                non_symbolic_truetype_uses_a_standard_encoding
            )
            .len(),
            1
        );

        let no_flags = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /TrueType /Encoding /MacExpertEncoding >>\nendobj\n",
        );
        assert!(
            findings(&no_flags, non_symbolic_truetype_uses_a_standard_encoding).is_empty(),
            "a font that never said whether it is symbolic is not judged by either rule"
        );
    }

    #[test]
    fn to_unicode_is_asked_of_the_fonts_whose_exemption_can_be_settled() {
        let identity_composite = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /Type0 /Encoding /Identity-H \
             /DescendantFonts [3 0 R] >>\nendobj\n\
             3 0 obj\n<< /Type /Font /Subtype /CIDFontType2 \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> >>\n\
             endobj\n",
        );
        assert_eq!(
            findings(&identity_composite, to_unicode_present).len(),
            1,
            "an Identity ordering is not one of the four collections the clause names"
        );

        let japanese = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /Type0 /Encoding /UniJIS-UCS2-H \
             /DescendantFonts [3 0 R] >>\nendobj\n\
             3 0 obj\n<< /Type /Font /Subtype /CIDFontType0 \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 6 >> >>\nendobj\n",
        );
        assert!(findings(&japanese, to_unicode_present).is_empty());

        let type1 = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Nonesuch >>\nendobj\n",
        );
        assert!(
            findings(&type1, to_unicode_present).is_empty(),
            "the Adobe Glyph List exemption cannot be settled here, so a Type 1 font is passed \
             over"
        );
    }

    #[test]
    fn a_cmap_whose_two_write_modes_disagree_is_reported() {
        /// A `CMap` stream whose program states `wmode` and whose dictionary states `stated`.
        fn cmap(stated: i64, wmode: u8) -> Document {
            let program = format!("/CMapName /Test def\n/WMode {wmode} def\n");
            let length = program.len();
            document(&format!(
                "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                 2 0 obj\n<< /Type /CMap /CMapName /Test /WMode {stated} /Length {length} >>\n\
                 stream\n{program}\nendstream\nendobj\n"
            ))
        }

        assert_eq!(
            findings(&cmap(1, 0), embedded_cmap_states_its_own_write_mode).len(),
            1
        );
        assert_eq!(
            findings(&cmap(0, 1), embedded_cmap_states_its_own_write_mode).len(),
            1
        );
        assert!(findings(&cmap(1, 1), embedded_cmap_states_its_own_write_mode).is_empty());

        // ISO 32000-2 §9.7.5.3, Table 118: `/WMode` defaults to 0, so a program that states
        // none states 0 and a dictionary that states none agrees with it.
        let silent = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /CMap /CMapName /Test /Length 0 >>\nstream\n\nendstream\nendobj\n",
        );
        assert!(findings(&silent, embedded_cmap_states_its_own_write_mode).is_empty());
    }

    #[test]
    fn a_glyph_procedures_width_is_the_first_operand_of_its_first_operator() {
        assert_eq!(glyph_procedure_width(b"750 0 d0\n"), Some(("d0", 750.0)));
        assert_eq!(
            glyph_procedure_width(b"1000 0 0 0 750 750 d1\n0 0 750 750 re\nf\n"),
            Some(("d1", 1000.0))
        );
        assert_eq!(
            glyph_procedure_width(b"0 0 750 750 re f"),
            None,
            "a procedure whose first operator is neither d0 nor d1 breaks ISO 32000-2 §9.6.4 \
             rather than this clause"
        );
        assert_eq!(glyph_procedure_width(b"d0"), None, "d0 states two operands");
    }

    /// A one-page document drawing `a` and `b` in a Type 3 font whose glyphs are `wide` wide.
    fn type3_page(widths: &str, wide: u32) -> Document {
        let procedure = format!("{wide} 0 0 0 750 750 d1\n0 0 750 750 re\nf");
        let length = procedure.len();
        let content = "BT /F1 12 Tf (ab) Tj ET";
        let content_length = content.len();
        document(&format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length {content_length} >>\nstream\n{content}\nendstream\nendobj\n\
             5 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750] \
             /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 6 0 R /Encoding 7 0 R \
             /FirstChar 97 /LastChar 98 /Widths {widths} >>\nendobj\n\
             6 0 obj\n<< /square 8 0 R /triangle 8 0 R >>\nendobj\n\
             7 0 obj\n<< /Type /Encoding /Differences [97 /square /triangle] >>\nendobj\n\
             8 0 obj\n<< /Length {length} >>\nstream\n{procedure}\nendstream\nendobj\n"
        ))
    }

    #[test]
    fn a_type3_glyphs_two_stated_widths_are_compared_in_text_space() {
        assert!(
            findings(
                &type3_page("[1000 1000]", 1000),
                type3_glyph_procedures_state_their_width
            )
            .is_empty()
        );

        let found = findings(
            &type3_page("[0 0]", 1000),
            type3_glyph_procedures_state_their_width,
        );
        assert_eq!(found.len(), 2, "both glyphs disagree: {found:?}");

        // The FontMatrix maps glyph space to text space, and the tolerance is a thousandth of a
        // text-space unit — so with the usual 0.001 matrix a difference of one glyph unit is
        // exactly at the limit and is not a failure.
        assert!(
            findings(
                &type3_page("[1000 1000]", 1001),
                type3_glyph_procedures_state_their_width
            )
            .is_empty()
        );
    }

    #[test]
    fn a_differences_name_outside_the_adobe_glyph_list_is_reported() {
        /// A non-symbolic `TrueType` font whose `/Differences` names one glyph.
        fn with(name: &str) -> Document {
            document(&format!(
                "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                 2 0 obj\n<< /Type /Font /Subtype /TrueType /FontDescriptor 3 0 R \
                 /Encoding << /BaseEncoding /WinAnsiEncoding \
                 /Differences [96 /{name}] >> >>\nendobj\n\
                 3 0 obj\n<< /Type /FontDescriptor /Flags 32 >>\nendobj\n"
            ))
        }

        assert_eq!(
            findings(
                &with("grav"),
                non_symbolic_truetype_differences_are_listed_names
            )
            .len(),
            1
        );
        assert!(
            findings(
                &with("grave"),
                non_symbolic_truetype_differences_are_listed_names
            )
            .is_empty()
        );

        let symbolic = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /TrueType /FontDescriptor 3 0 R \
             /Encoding << /Differences [96 /grav] >> >>\nendobj\n\
             3 0 obj\n<< /Type /FontDescriptor /Flags 4 >>\nendobj\n",
        );
        assert!(
            findings(
                &symbolic,
                non_symbolic_truetype_differences_are_listed_names
            )
            .is_empty(),
            "the clause's subject is the non-symbolic fonts"
        );
    }

    #[test]
    fn the_three_placeholder_unicode_values_are_reported_and_others_are_not() {
        /// A font whose `/ToUnicode` maps code 65 to `value`, written as UTF-16BE hex.
        fn mapping(value: &str) -> Document {
            let program = format!("1 beginbfchar\n<41> <{value}>\nendbfchar\n");
            let length = program.len();
            document(&format!(
                "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                 2 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Test \
                 /ToUnicode 3 0 R >>\nendobj\n\
                 3 0 obj\n<< /Length {length} >>\nstream\n{program}\nendstream\nendobj\n"
            ))
        }

        for placeholder in ["0000", "feff", "fffe"] {
            assert_eq!(
                findings(&mapping(placeholder), to_unicode_values_are_usable).len(),
                1,
                "U+{placeholder} is one of the three the clause names"
            );
        }
        assert!(findings(&mapping("0041"), to_unicode_values_are_usable).is_empty());
    }

    /// A `bfrange` states its values by arithmetic, so the rule is judged by arithmetic: the
    /// span below runs U+FEFD, U+FEFE, U+FEFF, and only the third is one the clause forbids.
    #[test]
    fn a_span_that_runs_over_a_forbidden_value_is_reported_at_the_code_that_reaches_it() {
        let program = "1 beginbfrange\n<10> <20> <fefd>\nendbfrange\n";
        let length = program.len();
        let subject = document(&format!(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Test /ToUnicode 3 0 R >>\nendobj\n\
             3 0 obj\n<< /Length {length} >>\nstream\n{program}\nendstream\nendobj\n"
        ));
        let found = findings(&subject, to_unicode_values_are_usable);
        assert_eq!(found.len(), 1, "one statement, one fault");
        assert!(
            found[0].contains("code 18"),
            "and it names the code that reaches U+FEFF, which is 0x10 plus two: {}",
            found[0]
        );
    }

    /// ISO 32000-2 §9.7.6.2 lets a character code be up to four bytes. Reading the statements
    /// covers those; sweeping a guessed code space, which is what this rule used to do, did not.
    #[test]
    fn a_forbidden_value_stated_for_a_four_byte_code_is_reported() {
        let program = "1 beginbfchar\n<00A10001> <0000>\nendbfchar\n";
        let length = program.len();
        let subject = document(&format!(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /Test /ToUnicode 3 0 R >>\nendobj\n\
             3 0 obj\n<< /Length {length} >>\nstream\n{program}\nendstream\nendobj\n"
        ));
        assert_eq!(findings(&subject, to_unicode_values_are_usable).len(), 1);
    }

    /// §9.8.1's Table 122 makes a `/CharSet` string PDF name syntax rather than a delimited
    /// list, so it is lexed: `#` escapes resolve, and a string nothing could be read out of is
    /// no claim at all rather than a claim that the font has no glyphs.
    #[test]
    fn a_charset_string_is_read_as_names_and_an_unreadable_one_is_not_an_empty_set() {
        let names = charset_names(b"/slash/C/S/e").expect("four names");
        assert_eq!(names.len(), 4);
        assert!(names.contains("slash") && names.contains("C"));
        assert_eq!(
            charset_names(b"/a#20b").map(|it| it.into_iter().collect::<Vec<_>>()),
            Some(vec!["a b".to_owned()]),
            "the lexer resolves the escape the table's syntax allows"
        );
        assert!(charset_names(b"").is_none());
        assert!(
            charset_names(b"slash C S").is_none(),
            "no slashes, no names"
        );
    }

    /// The codes a `CMap` sends into the Private Use Area, which is what ISO 19005-2
    /// §6.2.11.7.3 is about. A span is answered by its first offending code rather than all of
    /// them, because one `bfrange` line is one thing the producer wrote.
    #[test]
    fn the_private_use_codes_are_read_off_the_statements() {
        let plain = ToUnicode::parse(b"1 beginbfchar\n<41> <0041>\nendbfchar\n");
        assert!(private_use_codes(&plain).is_empty());

        let single = ToUnicode::parse(b"1 beginbfchar\n<01> <e020>\nendbfchar\n");
        assert_eq!(private_use_codes(&single), vec![(1, 0xE020)]);

        // A surrogate pair, which is how a `CMap` states a value outside the basic plane:
        // U+10016D is in Supplementary Private Use Area-B.
        let supplementary = ToUnicode::parse(b"1 beginbfchar\n<03> <DBC0DD6D>\nendbfchar\n");
        assert_eq!(private_use_codes(&supplementary), vec![(3, 0x10_016D)]);

        // A span running U+EFFF8 upwards, which crosses into Supplementary Private Use Area-A
        // at U+F0000: the first code that reaches it is eight past the span's low code, and it
        // is the only one reported, because one `bfrange` line is one statement.
        let span = ToUnicode::parse(b"1 beginbfrange\n<10> <20> <DB7FDFF8>\nendbfrange\n");
        assert_eq!(span.mappings().count(), 1);
        assert_eq!(private_use_codes(&span), vec![(0x18, 0xF_0000)]);

        let missing = ToUnicode::parse(b"1 beginbfrange\n<10> <20> <0041>\nendbfrange\n");
        assert!(private_use_codes(&missing).is_empty());
    }

    #[test]
    fn a_placeholder_inside_a_bfrange_is_reported_once_rather_than_per_code() {
        // A range of sixteen codes onto U+FFF8 upwards, so U+FFFE falls inside it. The producer
        // wrote one range, so the finding is one.
        let program = "1 beginbfrange\n<10> <1f> <fff8>\nendbfrange\n";
        let length = program.len();
        let document = document(&format!(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Test /ToUnicode 3 0 R >>\nendobj\n\
             3 0 obj\n<< /Length {length} >>\nstream\n{program}\nendstream\nendobj\n"
        ));
        assert_eq!(findings(&document, to_unicode_values_are_usable).len(), 1);
    }

    #[test]
    fn a_private_use_character_inside_actual_text_is_reported() {
        // UTF-16BE with the byte-order mark ISO 32000-2 §7.9.2.2 requires, holding U+E000.
        let offending = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /StructElem /ActualText <FEFFE000> >>\nendobj\n",
        );
        assert_eq!(
            findings(&offending, actual_text_states_no_private_use).len(),
            1
        );

        let plain = document(
            "1 0 obj\n<< /Type /Catalog >>\nendobj\n\
             2 0 obj\n<< /Type /StructElem /ActualText (ff) >>\nendobj\n",
        );
        assert!(findings(&plain, actual_text_states_no_private_use).is_empty());
    }

    #[test]
    fn the_private_use_areas_are_the_three_the_standard_defines() {
        assert!(is_private_use('\u{E000}'));
        assert!(is_private_use('\u{F8FF}'));
        assert!(is_private_use('\u{F0000}'));
        assert!(is_private_use('\u{10FFFD}'));
        assert!(!is_private_use('A'));
        assert!(!is_private_use('\u{F900}'));
        assert!(!is_private_use('\u{EFFFF}'));
    }
}
