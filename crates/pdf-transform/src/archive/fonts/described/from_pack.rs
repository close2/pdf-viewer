//! A standard font stated by name alone, whose shown codes the shipped face cannot all draw, given
//! the Latin-Greek-Cyrillic pack's face and a descriptor read off that face (`doc/adr/2938`).
//!
//! ISO 32000-2 §9.6.2.1's Table 109 lets one of §9.6.2.2's fourteen state no `/FontDescriptor`
//! and no `/Widths`; ISO 19005-2 section 6.2.11.4.1 asks an embedded program of it all the same,
//! which [`super::describe`] answers with the shipped face. That face holds the Standard Latin
//! set alone, so a `Times-Roman` showing a beta has had its page photographed. The pack's face for
//! the same style draws it as text, by the descriptor route's construction (`doc/adr/2934`): the
//! face's advances restated to the font's own Annex D metrics, the font read back and held to the
//! pack face's own glyphs, then cut to the glyphs the page shows and named by §9.9.2
//! (`doc/adr/2939`). What differs is that the descriptor is written here, measured off the pack
//! face as [`Measured`] measures a shipped one, and the widths Table 109 then requires are stated.
//!
//! A code the window advanced by its face's `.notdef`, because neither the published metrics nor
//! the shipped face name its glyph (`LoadedFont::advanced_by_notdef`), is drawn here by the pack
//! face's own glyph of that name, and the width stated for it is that glyph's own advance: the
//! rule the window applies — the width is the painted glyph's (ISO 32000-2 §9.2.4), the glyph
//! §9.6.5.2 selects by name — applied to the face this file draws (`doc/adr/2942`).
//!
//! Only the standard's layout is taken: a font outside the fourteen that states no descriptor is
//! laid out by this viewer's choice (ADR 2898) or by its own `/Widths` (ADR 3047), and it keeps
//! [`super::describe`]'s route.

use std::collections::BTreeMap;

use pdf_archive::survey::SelectedFont;
use pdf_font::standard::SuppliedFace;
use pdf_font::substitute::Format;
use pdf_font::{Code, LoadedFont};
use pdf_syntax::Document;
use pdf_syntax::object::{Name, Object};

use super::super::super::decision::{Because, Loss, Losses};
use super::super::super::prepare::Spare;
use super::super::lgc_pack::{self, LgcFace};
use super::super::{
    Face, FaceWidths, NO_SPARE_OBJECT, PROGRAM_NOT_ENCODED, SubstitutedFont, Substitutes, Table124,
    base_font, face_widths, gains, key_for, name_of, notices, program_stream, proves, selected,
    subtype_dictionary,
};
use super::{
    Described, Layout, Measured, PROGRAM_NOT_MEASURED, Restated, layout_of, pack_face_proved,
    pack_face_subset, restated_encoding, restated_entries, state_widths,
};

/// Embeds the pack's face for `used`, a standard font stated by name alone, with a descriptor of
/// its own; `Ok(false)` where the font is not one of the fourteen under the standard's layout, and
/// the caller takes [`super::describe`]'s route.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] where the face's advances cannot be restated or measured, the font as
/// written does not read back to the pack face's glyphs at the file's advances, or no object is
/// spare for the program or the descriptor.
pub(in crate::archive::fonts) fn described_from_pack(
    document: &Document,
    used: &SelectedFont,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
    (pack, wanted): (&SuppliedFace, LgcFace),
) -> Result<bool, Because> {
    let requested = base_font(document, &used.dict);
    let stated_subtype = name_of(document, &used.dict, "Subtype").unwrap_or_default();
    let layout = layout_of(document, used, &stated_subtype, &requested)?;
    if !matches!(layout, Layout::Standard) {
        return Ok(false);
    }
    let source = LoadedFont::load(document, &used.dict, &used.name)
        .map_err(|_| Because::NotBuiltYet(super::super::FONT_NOT_READ))?;
    let face = Face::Pack(pack.clone(), wanted);
    let FaceWidths {
        mut widths, route, ..
    } = face_widths(&source, &face, used, &[])?;
    let own = own_advances(&source, used, pack, &mut widths);
    let laid_out = |code: Code| {
        own.get(&code.value())
            .copied()
            .unwrap_or_else(|| source.advance(code))
    };
    let restated = pdf_font::restate::with_widths(&pack.program, Format::BareCff, &widths)
        .map_err(|_| Because::NotBuiltYet(super::super::FACE_NOT_RESTATABLE))?;
    proves(&restated, Format::BareCff, &widths, &BTreeMap::new())?;
    let row =
        key_for("Type1", &restated).ok_or(Because::NotBuiltYet(super::super::NO_SHIPPED_FORMAT))?;
    let measured =
        Measured::of_cff(&pack.program, true).ok_or(Because::NotBuiltYet(PROGRAM_NOT_MEASURED))?;
    let descriptor_at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    let font_name = requested.as_bytes().to_vec();
    let mut descriptor = measured.descriptor(&font_name, pack.request, None);
    // §9.6.5.1: a `Type1` font with no encoding takes the program's built-in one. The pack's is the
    // Standard Encoding, which is a standard Latin font's own; a dictionary of another kind, or one
    // naming none of the fourteen's own tables, states the names its source drew by.
    let loses_names =
        stated_subtype != "Type1" && document.get_key(&used.dict, "Encoding").is_null();
    let encoding = loses_names.then(|| restated_encoding(&source, None, &BTreeMap::new()));
    let restated_names = encoding.is_some();
    let mut entries = restated_entries(
        document,
        used,
        &Restated {
            descriptor_at,
            subtype: "Type1",
            stated_subtype: &stated_subtype,
            font_name: &font_name,
            layout: &layout,
        },
        encoding,
        &source,
    );
    state_widths(&mut entries, &laid_out);
    let whole = pack_face_proved(
        document,
        used,
        (&source, &|code| pack.glyph(code)),
        (&entries, &descriptor),
        row,
        &restated,
        &laid_out,
    )?;
    let program = match pack_face_subset(
        document,
        used,
        &source,
        (&entries, &descriptor),
        row,
        (&restated, &whole),
    )? {
        Some(subset) => {
            let name = Object::Name(Name::new(subset.name.as_bytes()));
            entries.insert(Name::new(&b"BaseFont"[..]), name.clone());
            descriptor.insert(Name::new(&b"FontName"[..]), name);
            subset.program
        }
        None => restated,
    };
    let program_at = program_written(document, row, &program, spare, substitutes)?;
    descriptor.insert(Name::new(row.key.as_bytes()), Object::Reference(program_at));
    substitutes
        .written
        .insert(descriptor_at, Object::Dictionary(descriptor));
    gains(document, used, entries, spare, substitutes)?;
    substitutes.done.push(SubstitutedFont {
        resource: used.name.clone(),
        requested,
        face: wanted.describe().to_owned(),
        authority: face.authority(),
        route,
        described: Some(if restated_names {
            Described::Type1EncodingRestated
        } else {
            Described::Type1
        }),
        // Neither the producer's face nor the one the window drew (`doc/adr/2934`).
        losses: Losses::one(Loss::MachineFace),
        fitting: None,
        empty_glyphs: Vec::new(),
        empty_cids: Vec::new(),
        descendant_written: false,
        twin_codes: Vec::new(),
        cut_program: None,
        font: selected(used),
    });
    Ok(true)
}

/// The pack face's own advance, in text-space units, for each shown code the window advanced by its
/// face's `.notdef` and the pack face draws a glyph of its own for, set into `widths` against that
/// glyph so that the face is restated to it (`doc/adr/2942`).
///
/// §9.6.5.2 selects a glyph by the name the encoding gives the code: the shipped face holds none of
/// that name and showed `.notdef`, at `.notdef`'s width; the pack face holds one, and §9.2.4's
/// width is "the distance the current text position shall move … when the glyph is painted", so
/// the glyph this file paints moves the text position by its own width.
fn own_advances(
    source: &LoadedFont,
    used: &SelectedFont,
    pack: &SuppliedFace,
    widths: &mut BTreeMap<u16, f32>,
) -> BTreeMap<u32, f32> {
    let mut own = BTreeMap::new();
    for code in used.shown.keys().flat_map(|text| source.decode(text)) {
        if !source.advanced_by_notdef(code) {
            continue;
        }
        let Some(glyph) = u8::try_from(code.value())
            .ok()
            .and_then(|byte| pack.glyph(byte))
            .filter(|glyph| *glyph != pdf_font::NOTDEF_GLYPH)
        else {
            continue;
        };
        let Some(advance) = pdf_font::restate::advance(&pack.program, Format::BareCff, glyph)
        else {
            continue;
        };
        widths.insert(glyph, advance);
        own.insert(code.value(), advance);
    }
    own
}

/// The program's stream, carrying the pack's licence in its `/Metadata` (`doc/adr/2934`), written
/// into an object of its own.
fn program_written(
    document: &Document,
    row: Table124,
    program: &[u8],
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<pdf_syntax::object::ObjectId, Because> {
    let program_at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    let mut stream = program_stream(subtype_dictionary(row).as_ref(), program, false)
        .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
    notices::attach_notice(
        document,
        lgc_pack::LICENCE,
        &mut stream,
        spare,
        &mut substitutes.written,
    )?;
    substitutes.written.insert(program_at, stream);
    Ok(program_at)
}
