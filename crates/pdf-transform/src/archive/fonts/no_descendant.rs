//! A Type 0 font that names no `CIDFont`, given one that draws nothing at the widths this reader
//! moved by (ADR 3109).
//!
//! ISO 32000-2 §9.7.6.1's Table 119 makes `/DescendantFonts` the one-element array naming the
//! `CIDFont`, and ISO 19005-2 section 6.2.11.2 and ISO 19005-4 section 6.2.10.2 hold a file to it.
//! A Type 0 font whose array holds `null`, or nothing, or no `CIDFont`, states no glyph selection
//! and no character collection, and this reader loads no glyph of it. **It still places the
//! codes** (ADR 1094): its `CMap` splits each string into codes, and each code moves the text
//! position by Table 115's default width, "\[t]he default width for glyphs in the `CIDFont`",
//! 1000, plus §9.4.4's spacings — so the window draws nothing and moves as a font of that width
//! would (`pdf_font::LoadedFont::metrics_only`, `pdf-model`'s `advance_through_a_refused_font`).
//!
//! **The absence is kept, never filled in** (`doc/questions/A48`). The font is given a
//! `CIDFontType2` whose program holds glyph 0 and one glyph that draws nothing
//! (`pdf_font::embed::for_embedding_with_empty_glyphs`, ADR 2824's construction), restated to the
//! advance the window moved by; `/DW` stating that advance; a `/CIDToGIDMap` stream sending every
//! CID to that glyph; and the collection `Adobe-Identity-0`, which an `Identity` `CMap` reads every
//! CID as (§9.7.5.2). It states no character: a `CIDFontType2` reaches a glyph by index alone
//! (§9.7.4.2), and what each code means is still the producer's `/ToUnicode`, which stays. The CIDs
//! the pages show are reported and recorded in the file's own `xmpMM:History`, naming the clause,
//! as A48 asks.
//!
//! **Why the page is the window's, operator for operator.** §9.4.4 moves the position after each
//! glyph by `tx = ((w0 - Tj/1000) × Tfs + Tc + Tw) × Th`, and the window applies the same
//! expression to a font it draws nothing for; `w0` is proved equal for every shown code before
//! the font is written, and the other four terms are the graphics state's, not the font's. A
//! clipping mode adds no outline in either case, and §9.3.6 then clips nothing: "If no glyphs are
//! shown or if the only glyphs shown have no outlines ... no clipping shall occur."

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::SelectedFont;
use pdf_font::substitute::{Family, Format, Request};
use pdf_font::{Code, LoadedFont};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object};

use super::super::decision::{Because, Losses};
use super::super::prepare::Spare;
use super::{
    Embedding, FaceAuthority, MetricRoute, NO_SPARE_OBJECT, PROGRAM_NOT_ENCODED, SubstitutedFont,
    Substitutes, base_font, name_of, program_stream, selected,
};

/// Why a Type 0 font that has no `CIDFont` is refused where this route does not reach it.
///
/// ISO 32000-2 §9.7.6.1's Table 119 makes `/DescendantFonts` the one-element array naming the
/// `CIDFont`, and §9.7.4.1 makes the `CIDFont` what carries the glyph selection, the widths and the
/// character collection (ADR 2433).
pub(in crate::archive) const NO_CIDFONT_TO_DESCEND_TO: &str = "a Type 0 font here has no CIDFont \
     as its descendant - its DescendantFonts is missing, is not the one-element array ISO 32000-2 \
     \u{a7}9.7.6.1's Table 119 requires, or holds something other than a CIDFont dictionary, a \
     null among them - so the file states no glyph selection, widths or character collection for \
     any code the font shows. This conversion writes a CIDFont that draws nothing only for a font \
     this reader draws nothing for and moves past at one width, written as an object of its own \
     over the Identity-H CMap, whose strings it holds whole; this font is not one";

/// The `CMap` whose two-byte codes are read as their own CIDs (§9.7.5.2), horizontally.
const IDENTITY_H: &str = "Identity-H";

/// Whether `used` is a Type 0 font that names no `CIDFont`, which this module answers.
pub(super) fn lacks_one(document: &Document, used: &SelectedFont) -> bool {
    if name_of(document, &used.dict, "Subtype").as_deref() != Some("Type0") {
        return false;
    }
    let array = document.get_key(&used.dict, "DescendantFonts");
    let first = document
        .resolve(&array)
        .as_array()
        .filter(|array| array.len() == 1)
        .and_then(|array| array.first().cloned());
    let Some(first) = first else {
        return true;
    };
    let resolved = document.resolve(&first);
    let Some(descendant) = resolved.as_dict() else {
        return true;
    };
    !matches!(
        name_of(document, descendant, "Subtype").as_deref(),
        Some("CIDFontType0" | "CIDFontType2")
    )
}

/// Writes the `CIDFont` that draws nothing for a Type 0 font naming none, at the advance the
/// window moved each of its codes by.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] with [`NO_CIDFONT_TO_DESCEND_TO`] where the font is not an object of
/// its own, is not over `Identity-H`, is one this reader draws a glyph of or places no code of,
/// shows strings the survey did not keep whole, or moves its codes by more than one width; and
/// where the program written for it cannot be built or proved.
pub(super) fn written(
    document: &Document,
    used: &SelectedFont,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    let refused = || Because::NotBuiltYet(NO_CIDFONT_TO_DESCEND_TO);
    let font_at = used.id.ok_or_else(refused)?;
    if name_of(document, &used.dict, "Encoding").as_deref() != Some(IDENTITY_H)
        || !used.shown_complete
        || super::drawn_by_the_window(document, used).is_some()
    {
        return Err(refused());
    }
    // The font the window places the codes by: the widths alone (ADR 1094).
    let placed = LoadedFont::metrics_only(document, &used.dict, &used.name).ok_or_else(refused)?;
    let advance = one_advance(&placed, used).ok_or_else(refused)?;

    let (face, _) = pdf_font::standard::face(Request {
        family: Family::SansSerif,
        bold: false,
        italic: false,
        standard: true,
    });
    let (subset, empty) = pdf_font::embed::for_embedding_with_empty_glyphs(
        face,
        &BTreeSet::from([pdf_font::NOTDEF_GLYPH]),
        1,
    )
    .map_err(|_| refused())?;
    let blank = *empty.first().ok_or_else(refused)?;
    let widths = BTreeMap::from([(blank, advance)]);
    let program = pdf_font::restate::with_widths(&subset.program, Format::Sfnt, &widths)
        .map_err(|_| Because::NotBuiltYet(super::FACE_NOT_RESTATABLE))?;
    super::proves(&program, Format::Sfnt, &widths, &BTreeMap::new())?;
    let name = substitutes
        .names
        .name_program(document, &subset.name, &program);
    let descriptor = descriptor(&program, &name).ok_or_else(refused)?;

    let mut take = || {
        spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))
    };
    let (cid_font_at, descriptor_at, program_at, map_at) = (take()?, take()?, take()?, take()?);
    let stream =
        program_stream(None, &program, true).ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
    let every_cid: BTreeMap<u16, u16> = (0..=u16::MAX).map(|cid| (cid, blank)).collect();
    let map = super::rekeyed::cid_to_gid_map(&every_cid)?;
    let mut descriptor = descriptor;
    Embedding {
        key: "FontFile2",
        at: program_at,
    }
    .write_into(&mut descriptor);
    let font_name = Object::Name(Name::new(name.as_bytes()));
    substitutes.written.insert(program_at, stream);
    substitutes.written.insert(map_at, map);
    substitutes
        .written
        .insert(descriptor_at, Object::Dictionary(descriptor));
    substitutes.written.insert(
        cid_font_at,
        Object::Dictionary(cid_font(
            font_name.clone(),
            advance,
            (descriptor_at, map_at),
        )),
    );
    // Table 119: a Type 2 descendant's Type 0 font is named as the descendant is.
    let mut gains = Dictionary::new();
    gains.insert(
        Name::new(&b"DescendantFonts"[..]),
        Object::Array(vec![Object::Reference(cid_font_at)]),
    );
    gains.insert(Name::new(&b"BaseFont"[..]), font_name);
    substitutes.described.insert(font_at, gains);
    substitutes.done.push(SubstitutedFont {
        resource: used.name.clone(),
        requested: base_font(document, &used.dict),
        face: pdf_font::embed::postscript_name(face),
        authority: FaceAuthority::Shipped,
        route: MetricRoute::RestatedProgram,
        described: None,
        losses: Losses::default(),
        fitting: None,
        empty_glyphs: Vec::new(),
        unreached_glyphs: Vec::new(),
        empty_cids: shown_cids(used),
        descendant_written: true,
        twin_codes: Vec::new(),
        cut_program: None,
        font: selected(used),
    });
    Ok(())
}

/// The one advance, in ems, the window moves every code the pages show by, or `None` where two
/// codes move by two, which a font naming no `/W` does not do.
///
/// A font whose strings hold no code is moved by nothing, and is written at Table 115's default
/// width, the one the window would place a code by.
fn one_advance(placed: &LoadedFont, used: &SelectedFont) -> Option<f32> {
    let mut advances = used
        .shown
        .keys()
        .flat_map(|text| placed.decode(text))
        .map(|code: Code| placed.advance(code));
    let Some(first) = advances.next() else {
        return Some(TABLE_115_DEFAULT_WIDTH);
    };
    advances
        .all(|other| !super::disagrees(first, other))
        .then_some(first)
}

/// Table 115's `/DW` default, "1000", in ems.
const TABLE_115_DEFAULT_WIDTH: f32 = 1.0;

/// The CIDs the pages show under `Identity-H`, each code its own CID (§9.7.5.2): two bytes,
/// high-order first, with a trailing odd byte read as no code.
fn shown_cids(used: &SelectedFont) -> Vec<u16> {
    let cids: BTreeSet<u16> = used
        .shown
        .keys()
        .flat_map(|text| {
            text.as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_be_bytes(*pair))
        })
        .collect();
    cids.into_iter().collect()
}

/// Table 115's `CIDFont`: Type 2, `Adobe-Identity-0`, every CID `advance` ems wide.
fn cid_font(
    name: Object,
    advance: f32,
    (descriptor, map): (pdf_syntax::object::ObjectId, pdf_syntax::object::ObjectId),
) -> Dictionary {
    let named = |value: &[u8]| Object::Name(Name::new(value));
    let mut info = Dictionary::new();
    info.insert(
        Name::new(&b"Registry"[..]),
        Object::String(b"Adobe".to_vec().into()),
    );
    info.insert(
        Name::new(&b"Ordering"[..]),
        Object::String(b"Identity".to_vec().into()),
    );
    info.insert(Name::new(&b"Supplement"[..]), Object::Integer(0));
    let mut dict = Dictionary::new();
    dict.insert(Name::new(&b"Type"[..]), named(b"Font"));
    dict.insert(Name::new(&b"Subtype"[..]), named(b"CIDFontType2"));
    dict.insert(Name::new(&b"BaseFont"[..]), name);
    dict.insert(Name::new(&b"CIDSystemInfo"[..]), Object::Dictionary(info));
    dict.insert(
        Name::new(&b"FontDescriptor"[..]),
        Object::Reference(descriptor),
    );
    dict.insert(
        Name::new(&b"DW"[..]),
        Object::Real(f64::from(advance) * 1000.0),
    );
    dict.insert(Name::new(&b"CIDToGIDMap"[..]), Object::Reference(map));
    dict
}

/// Table 120's descriptor of the written program, measured off it: the bounding box of its one
/// drawing glyph, and the face's own ascent, descent and capital height, since the glyph a CID
/// reaches draws nothing to measure.
fn descriptor(program: &[u8], name: &str) -> Option<Dictionary> {
    use skrifa::MetadataProvider as _;
    use skrifa::instance::{LocationRef, Size};
    use skrifa::outline::DrawSettings;

    let face = skrifa::FontRef::new(program).ok()?;
    let metrics = face.metrics(Size::unscaled(), LocationRef::default());
    if metrics.units_per_em == 0 {
        return None;
    }
    let scale = 1000.0 / f32::from(metrics.units_per_em);
    let mut pen = super::pack::Bounds::default();
    face.outline_glyphs()
        .get(skrifa::GlyphId::new(u32::from(pdf_font::NOTDEF_GLYPH)))?
        .draw(
            DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
            &mut pen,
        )
        .ok()?;
    let bbox = pen.rectangle()?;
    let number = |value: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a glyph-space measurement in thousandths of an em, rounded to the whole \
                      unit a descriptor states it in and clamped far inside i64"
        )]
        Object::Integer((value * scale).round().clamp(-1.0e6, 1.0e6) as i64)
    };
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"FontDescriptor"[..])),
    );
    dict.insert(
        Name::new(&b"FontName"[..]),
        Object::Name(Name::new(name.as_bytes())),
    );
    // Symbolic: the program's glyphs are reached by index, and its character set is no standard
    // Latin one (Table 121).
    dict.insert(Name::new(&b"Flags"[..]), Object::Integer(1 << 2));
    dict.insert(
        Name::new(&b"FontBBox"[..]),
        Object::Array(bbox.iter().map(|side| number(*side)).collect()),
    );
    dict.insert(Name::new(&b"ItalicAngle"[..]), Object::Integer(0));
    dict.insert(Name::new(&b"Ascent"[..]), number(metrics.ascent));
    dict.insert(Name::new(&b"Descent"[..]), number(metrics.descent));
    dict.insert(
        Name::new(&b"CapHeight"[..]),
        number(metrics.cap_height.unwrap_or(metrics.ascent)),
    );
    dict.insert(Name::new(&b"StemV"[..]), Object::Integer(0));
    Some(dict)
}
