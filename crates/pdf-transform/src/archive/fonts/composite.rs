//! The program an operator supplies for a composite font, and where ISO 32000-2 §9.7.4.2 puts it.
//!
//! A composite font is not a simple one with wider codes, and three of this module's decisions
//! follow from that rather than from anything about supplying a face:
//!
//! - **The program goes into the *descendant* `CIDFont`'s descriptor.** §9.7.4.2 makes a CID an
//!   index into the glyphs of the font that defined it, and Table 115 makes the descendant's
//!   `/FontDescriptor` the one describing the `CIDFont` — so a `/FontFile` written into the Type 0
//!   dictionary's own descriptor would describe nothing, there being none.
//! - **The advances are §9.7.4.3's `/W` and `/DW`**, indexed by CID, rather than a `/Widths`
//!   array indexed by code. What is restated is still the program, for §9.2.4's reason: the
//!   dictionary is what positions the glyphs.
//! - **The `CMap` is what turns a code into a CID**, so which CIDs the page shows is a fact about
//!   the encoding. This build takes the two the standard defines outright — `Identity-H` and
//!   `Identity-V`, where the CID is the code — and refuses the rest by name, because for any
//!   other `CMap` the CIDs belong to a character collection and whether the supplied program
//!   defines *that* collection is a question the file does not answer.
//!
//! # What is refused, and why each refusal is a clause rather than a gap
//!
//! - **A program whose Top DICT uses `CIDFont` operators.** §9.7.4.2 makes such a program identify
//!   its character collection with a `CIDSystemInfo` the clause says should be copied into the
//!   PDF `CIDFont` dictionary, and the CIDs then reach glyphs through the program's charset.
//!   Embedding one under a dictionary whose `/CIDSystemInfo` names a different collection leaves the
//!   file asserting a collection its program does not define — and correcting `/CIDSystemInfo` is
//!   what `doc/pdf-a-mitigations.md` keeps as *none* for
//!   `fonts/cid-system-info-agrees-with-the-cmap` (`doc/adr/1188`). So the programs this embeds
//!   are the ones whose CIDs *are* glyph indices, which §9.7.4.2 states for a `CFF ` Top DICT
//!   without `CIDFont` operators and Table 115 states as `/CIDToGIDMap` `Identity`.
//! - **A `/CIDToGIDMap` that is a stream, or a name other than `Identity`.** §9.7.4.2: a map
//!   beside a font program that is *not* embedded "shall be ignored, since it is not meaningful
//!   to refer to glyph indices in an external font program". Embedding a program turns that
//!   entry on, and a table written for a program nobody had is not evidence about the one the
//!   operator named.
//! - **A page set in vertical writing mode whose supplied program states vertical metrics.** ISO
//!   19005-4 section 6.2.10.5 requires those to agree with `/DW2` and `/W2`, and this embeds the
//!   program before anything has restated them.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::sync::Arc;

use pdf_archive::survey::SelectedFont;
use pdf_font::substitute::Format;
use pdf_font::{Code, LoadedFont};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Object, ObjectId};

use super::super::decision::Because;
use super::super::prepare::Spare;
use super::{
    Embedding, FaceAuthority, MetricRoute, PROGRAM_NOT_ENCODED, SubstitutedFont, Substitutes,
    TWO_WIDTHS_FOR_ONE_GLYPH, base_font, descendant, disagrees, key_for, name_of, no_row,
    program_stream, subtype_dictionary,
};

/// Embeds the program an operator named into one composite font's descendant `CIDFont`.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] naming which of this module's conditions the document met, and
/// [`Because::TheFence`] where the dictionary asks two widths of one glyph — a document no
/// rewrite of a program can satisfy, since a program states one advance per glyph.
pub(super) fn embed(
    document: &Document,
    used: &SelectedFont,
    spare: &mut Spare,
    supplied: &BTreeMap<String, Arc<[u8]>>,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    let requested = base_font(document, &used.dict);
    let descendant_dict =
        descendant(document, &used.dict).ok_or(Because::NotBuiltYet(NO_DESCENDANT_CIDFONT))?;
    // **Either name reaches the program**, because §9.7.6.2 lets the two differ: the Type 0
    // dictionary's `/BaseFont` and the descendant's are both the PostScript name of the same
    // font, and an operator has only the one the report showed them.
    let named = super::named_program(supplied, &requested)
        .or_else(|| super::named_program(supplied, &base_font(document, &descendant_dict)));
    let Some((named, program)) = named else {
        return Err(Because::NotBuiltYet(super::COMPOSITE_NOT_SUBSTITUTED));
    };
    let encoding = name_of(document, &used.dict, "Encoding").unwrap_or_default();
    if encoding != "Identity-H" && encoding != "Identity-V" {
        return Err(Because::NotBuiltYet(CMAP_IS_NOT_IDENTITY));
    }
    let subtype = name_of(document, &descendant_dict, "Subtype").unwrap_or_default();
    let descriptor = descendant_dict
        .get("FontDescriptor")
        .and_then(Object::as_reference)
        .ok_or(Because::NotBuiltYet(NO_DESCRIPTOR_OBJECT_TO_EMBED_INTO))?;
    let format = pdf_font::standard::program_format(program)
        .ok_or(Because::NotBuiltYet(super::SUPPLIED_FACE_NOT_READ))?;
    if pdf_font::embedding::cff_keying(program, format) == Some(pdf_font::embedding::Keying::ByCid)
    {
        return Err(Because::NotBuiltYet(SUPPLIED_PROGRAM_IS_CID_KEYED));
    }
    let row = key_for(&subtype, format, program)
        .ok_or_else(|| Because::NotBuiltYet(no_row(&subtype, format, program)))?;
    let identity_map = needs_an_identity_map(document, &used.dict, &descendant_dict, &subtype)?;

    // **The metrics and the CMap, not a face.** What embedding a program needs from the font
    // dictionary is where one code ends and the next begins and what §9.7.4.3's `/W` and `/DW`
    // say the displacement is — both of which [`LoadedFont::metrics_only`] reads without a glyph
    // anywhere. Loading the font outright would ask a second question, whether this reader can
    // *draw* the composite font as the file stands, and §9.7.5.2 makes the answer to that no for
    // every `Identity` encoding whose program is missing — which is the whole population here.
    let font = LoadedFont::metrics_only(document, &used.dict, &used.name)
        .ok_or(Because::NotBuiltYet(NO_CMAP_TO_READ_THE_CODES_BY))?;
    if font.is_vertical() && states_vertical_metrics(program, format) {
        return Err(Because::NotBuiltYet(SUPPLIED_VERTICAL_NOT_RESTATED));
    }
    let (widths, route) = shown_widths(&font, used, program, format)?;

    // **§9.9.1, and it is the one table a supplied program loses**: "If used with a CIDFont
    // dictionary, the "cmap" table is not needed and shall not be present, since the mapping from
    // character codes to glyph descriptions is provided separately." The mapping provided
    // separately is the `/CIDToGIDMap` written below, so nothing a reader consults goes with it.
    let mut bytes = program.to_vec();
    if row.key == "FontFile2" {
        bytes = pdf_font::embedding::without_tables(&bytes, &[*b"cmap"])
            .ok_or(Because::NotBuiltYet(CMAP_NOT_REMOVED))?;
    }
    if route == MetricRoute::RestatedProgram {
        bytes = pdf_font::restate::with_widths(&bytes, format, &widths)
            .map_err(|_| Because::NotBuiltYet(super::FACE_NOT_RESTATABLE))?;
    }
    // The same proof the simple route takes, and for the same reason: §9.2.4 makes the
    // dictionary's numbers what positions every glyph, so a program whose advances do not read
    // back as the file's numbers would move the page's text without saying so.
    super::proves(&bytes, format, &widths, &BTreeMap::new())?;

    let at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
    let stream = program_stream(
        subtype_dictionary(row).as_ref(),
        &bytes,
        row.key == "FontFile2",
    )
    .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
    substitutes.written.insert(at, stream);
    substitutes
        .at
        .insert(descriptor, Embedding { key: row.key, at });
    if let Some(descendant_at) = identity_map {
        substitutes.identity_cid_to_gid.insert(descendant_at);
    }
    substitutes.done.push(SubstitutedFont {
        resource: used.name.clone(),
        requested,
        face: named,
        authority: FaceAuthority::Operator,
        route,
    });
    Ok(())
}

/// Which descendant object this conversion has to write a `/CIDToGIDMap` `Identity` into.
///
/// Table 115 makes the entry required of a Type 2 `CIDFont` with an embedded font program, so a
/// dictionary that stated none while its program was elsewhere owes one the moment a program is
/// written in — and `Identity` is what the CIDs of an `Identity` `CMap` were already being read as.
/// `Ok(None)` where nothing is owed: a `CIDFontType0`, which Table 115 does not ask this of, and
/// a dictionary that already states `Identity`.
fn needs_an_identity_map(
    document: &Document,
    dict: &Dictionary,
    descendant: &Dictionary,
    subtype: &str,
) -> Result<Option<ObjectId>, Because> {
    if subtype != "CIDFontType2" {
        return Ok(None);
    }
    match document.get_key(descendant, "CIDToGIDMap") {
        Object::Name(map) if map == "Identity" => Ok(None),
        Object::Null => descendant_object(document, dict)
            .map(Some)
            .ok_or(Because::NotBuiltYet(NO_DESCENDANT_OBJECT_TO_WRITE_INTO)),
        _ => Err(Because::NotBuiltYet(CID_TO_GID_MAP_IS_THE_PRODUCERS)),
    }
}

/// The object the `/DescendantFonts` array names, where it names one rather than holding it.
fn descendant_object(document: &Document, dict: &Dictionary) -> Option<ObjectId> {
    let array = document.get_key(dict, "DescendantFonts");
    document.resolve(&array).as_array()?.first()?.as_reference()
}

/// The width the dictionary states for each glyph the page shows, and which metric route that is.
///
/// Keyed by glyph index, which under an `Identity` `CMap` and a program whose CIDs are its glyph
/// indices is the CID itself: §9.7.4.2, of a `CFF ` Top DICT without `CIDFont` operators, "The CIDs
/// shall be used directly as GID values", and Table 115, of `/CIDToGIDMap` `Identity`, "the
/// mapping between CIDs and glyph indices is the identity mapping".
fn shown_widths(
    font: &LoadedFont,
    used: &SelectedFont,
    program: &[u8],
    format: Format,
) -> Result<(BTreeMap<u16, f32>, MetricRoute), Because> {
    let glyphs = pdf_font::embedding::glyph_count(program, format)
        .ok_or(Because::NotBuiltYet(super::SUPPLIED_FACE_NOT_READ))?;
    let mut widths: BTreeMap<u16, f32> = BTreeMap::new();
    let mut route = MetricRoute::FaceMetrics;
    for text in used.shown.keys() {
        for code in font.decode(text) {
            let glyph = identity_glyph(code, glyphs)
                .ok_or(Because::NotBuiltYet(SUPPLIED_PROGRAM_MISSES_A_CID))?;
            let stated = font.advance(code);
            match widths.entry(glyph) {
                Entry::Vacant(slot) => {
                    slot.insert(stated);
                }
                Entry::Occupied(held) if disagrees(*held.get(), stated) => {
                    return Err(Because::TheFence(TWO_WIDTHS_FOR_ONE_GLYPH));
                }
                Entry::Occupied(_) => {}
            }
            if pdf_font::restate::advance(program, format, glyph)
                .is_none_or(|own| disagrees(stated, own))
            {
                route = MetricRoute::RestatedProgram;
            }
        }
    }
    Ok((widths, route))
}

/// The glyph an `Identity` `CMap`'s code selects, where the supplied program has one for it.
///
/// `None` for a CID past the program's last glyph and for CID 0, which §9.7.6.3 makes the
/// analogue of `.notdef` — ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid
/// showing that glyph, so a page that shows it is refused rather than given a program with a hole
/// where its text was.
fn identity_glyph(code: Code, glyphs: u16) -> Option<u16> {
    let glyph = u16::try_from(code.value()).ok()?;
    (glyph != pdf_font::NOTDEF_GLYPH && glyph < glyphs).then_some(glyph)
}

/// Whether a supplied program states vertical metrics at all.
fn states_vertical_metrics(program: &[u8], format: Format) -> bool {
    format == Format::Sfnt
        && pdf_font::embedding::sfnt_tables(program).is_some_and(|tables| tables.carries(b"vmtx"))
}

/// Why a composite font whose codes this reader cannot split is not supplied.
const NO_CMAP_TO_READ_THE_CODES_BY: &str = "this document renders a composite font whose \
     Encoding names a CMap this reader cannot read, so where one character code ends and the \
     next begins is not a question it can answer — and which CIDs the page shows is what decides \
     whether a supplied program covers it";

/// Why a composite font whose `CMap` is not one of the two `Identity` ones is not supplied.
const CMAP_IS_NOT_IDENTITY: &str = "this document renders a composite font whose Encoding is \
     neither Identity-H nor Identity-V, and the program named with --font is not embedded into \
     it. ISO 32000-2 §9.7.5 makes the CMap what turns a code into a CID and §9.7.4.2 makes a CID \
     an index into the glyphs of the font that defined a character collection, so for any other \
     CMap the codes this page shows name glyphs of that collection — and whether the supplied \
     program defines the same collection is a fact neither the file nor the file name states. \
     Under an Identity CMap the CID is the code and the question does not arise, which is why \
     those two are reached";

/// Why a composite font this reader cannot find a descendant for is not supplied.
const NO_DESCENDANT_CIDFONT: &str = "this document renders a composite font whose \
     DescendantFonts array names no CIDFont dictionary this reader can read. ISO 32000-2 §9.7.6.2 \
     makes that array exactly one element long and §9.7.4.2 puts the font program in the \
     descendant's own descriptor, so there is nowhere for a supplied program to go";

/// Why a descendant whose descriptor is written inline is not supplied.
const NO_DESCRIPTOR_OBJECT_TO_EMBED_INTO: &str = "this composite font's descendant CIDFont \
     states its FontDescriptor as a dictionary written inside itself rather than as the indirect \
     reference ISO 32000-2's Table 115 requires, and a program is embedded by writing a FontFile \
     key into a descriptor object. There is no object here to write into";

/// Why a descendant written inline is not given a `/CIDToGIDMap`.
const NO_DESCENDANT_OBJECT_TO_WRITE_INTO: &str = "this composite font's DescendantFonts array \
     holds the CIDFont dictionary itself rather than a reference to one, and ISO 32000-2's Table \
     115 makes CIDToGIDMap required of a Type 2 CIDFont with an embedded font program. There is \
     no object here to write that entry into";

/// Why a program keyed by CID is not supplied.
const SUPPLIED_PROGRAM_IS_CID_KEYED: &str = "the program named with --font for this composite \
     font has a Top DICT that uses CIDFont operators, so ISO 32000-2 §9.7.4.2 makes its own \
     CIDSystemInfo the character collection its CIDs belong to and its charset the route from a \
     CID to a glyph. Embedding it under a dictionary whose CIDSystemInfo states a different \
     collection would leave the file asserting a collection its program does not define, and \
     correcting a CIDSystemInfo is not something this converter does. Naming a program whose CIDs \
     are its glyph indices resolves it, which is what §9.7.4.2 says of a CFF Top DICT without \
     CIDFont operators and what Table 115 says of CIDToGIDMap Identity";

/// Why a descendant whose `/CIDToGIDMap` is the producer's own is not supplied.
const CID_TO_GID_MAP_IS_THE_PRODUCERS: &str = "this composite font's descendant CIDFont states a \
     CIDToGIDMap of its own. ISO 32000-2 §9.7.4.2 says that where the font program is not \
     embedded the entry shall be ignored, since it is not meaningful to refer to glyph indices in \
     an external font program — so this file's map is a table no reader has used, written for a \
     program that was never here, and embedding a program would turn it on over glyphs it was not \
     written for. This conversion embeds a program under the identity mapping or not at all";

/// Why a program that does not cover the CIDs a page shows is not supplied.
const SUPPLIED_PROGRAM_MISSES_A_CID: &str = "the program named with --font has no glyph for \
     every CID this document shows: a code of its Identity CMap is the CID itself, and one of \
     them is past the program's last glyph or is CID 0, which ISO 32000-2 §9.7.6.3 makes the \
     analogue of .notdef. Embedding it would show that glyph, which ISO 19005-2 section 6.2.11.8 \
     and ISO 19005-4 section 6.2.10.9 forbid, so the program is not embedded rather than embedded \
     with holes where the document's text was";

/// Why a vertically set composite font with a supplied program is not embedded.
const SUPPLIED_VERTICAL_NOT_RESTATED: &str = "this document sets a composite font down the page \
     and the program named with --font states vertical metrics of its own. ISO 19005-4 section \
     6.2.10.5 requires those to agree with the DW2 and W2 entries this file states, and this \
     conversion restates a supplied program's advances across the line and not yet down it, so \
     embedding the program would produce a file failing a requirement it was converted for";

/// Why a program whose `cmap` could not be dropped is not supplied.
const CMAP_NOT_REMOVED: &str = "ISO 32000-2 §9.9.1 requires that a TrueType program embedded \
     under a CIDFont dictionary carry no cmap table, since the mapping from character codes to \
     glyph descriptions is provided separately, and the table directory of the program named with \
     --font could not be written again without it";
