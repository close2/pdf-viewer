//! An `Adobe-Identity` composite font the file does not embed, given a shipped face's glyphs chosen
//! by its own `/ToUnicode` values and numbered by the document's CIDs (owner answer A570, ADR
//! 2378).
//!
//! # Why the file is invalid as it stands, and what this writes down instead
//!
//! ISO 32000-2 §9.7.5.2:
//!
//! > The Identity-H and Identity-V CMaps shall not be used with a non-embedded font.
//!
//! Under `Identity` ordering a CID is an index into the glyphs of the program that defined it
//! (§9.7.4.2), and that program is not in the file, so no face can be ordered by those numbers.
//! The viewer draws such a page all the same (ADR 0270): each code is taken to the character the
//! font's `/ToUnicode` states for it (§9.10.2's first method), and that character to a face's
//! glyph. This module writes the same selection into the file — the face is one this program
//! ships, and the character is the producer's own statement — so the archive shows what the
//! window guessed and invents nothing of its own. What the file supplies is the character; what
//! the build supplies is the face; the positions stay the document's, because the content stream,
//! the `/Encoding` and §9.7.4.3's `/W` and `/DW` are untouched and the program's advances are
//! restated to them.
//!
//! # The face
//!
//! **The shipped face for the descendant's name first** — `pdf_font::standard::face`, the face
//! ADR 2162's route embeds for a simple font of the same name — where it has a glyph for every
//! character the page shows; **the CJK pack's program otherwise** (A569), for the first collection
//! whose `Uni*-UTF32-H` `CMap` — the mapping Adobe publishes from Unicode to the collection's CIDs,
//! and the one the pack is built through (`doc/cjk-pack.md`) — reaches every character. Which
//! collection is tried first is a documented choice, because nothing in an `Identity` font names a
//! script: Table 122's `/Lang` where the descriptor states one, then the script the characters
//! themselves are in (kana Japanese, Hangul Korean), then §9.7.5.2's own order of the four.
//!
//! # How the glyphs are numbered by the document's CIDs
//!
//! - **A `glyf` face** is embedded whole under `/FontFile2`, and Table 115's `/CIDToGIDMap` stream
//!   sends each CID to the glyph its character selects — the stream §9.7.4.2 gives a Type 2
//!   `CIDFont` with an embedded program for exactly this.
//! - **A `CFF ` face** is written as a §9.9.2 subset under Table 124's `CIDFontType0C` row whose
//!   charset gives each kept glyph the document's CID (`pdf_font::embed::renumbered`), because a
//!   Type 0 `CIDFont` reaches its glyphs through the charset and has no `/CIDToGIDMap`. A glyph two
//!   CIDs show is written twice, once under each.
//!
//! **The `/ToUnicode` is kept as it is**: the CIDs did not change, so it still states, code by
//! code, the value each glyph was keyed from.
//!
//! # The losses, and why both are named
//!
//! `machine-face` at composite scale — the shapes are a shipped face's rather than the face the
//! window drew on this machine — and `glyphs-from-to-unicode`, because which glyph each mark shows
//! is inferred from an extraction aid, which a producer may have written for searching rather than
//! for drawing. `as-if-printed` discards both and `as-shown` stops on them (A570, ADR 2162).
//!
//! # What refuses, each by a sentence
//!
//! An encoding other than `Identity-H`; a shown code the `/ToUnicode` does not state, states as a
//! sequence or as a value ISO 19005-2 section 6.2.11.7.2 names unusable; CID 0; a character no
//! shipped face and no pack program reaches, or reaches as an empty glyph that is no space; two
//! CIDs one `glyf` glyph would have to show at two advances; a descendant or descriptor that is no
//! object, or a descriptor another route wrote into.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::SelectedFont;
use pdf_font::LoadedFont;
use pdf_font::substitute::{Format, Request};
use pdf_font::tounicode::ToUnicode;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;
use skrifa::MetadataProvider as _;

use super::super::COMPRESSION_LEVEL;
use super::super::decision::{Because, Loss, Losses};
use super::super::prepare::Spare;
use super::super::to_unicode::UNUSABLE;
use super::pack::{CjkPack, Collection, LICENCE};
use super::{
    Embedding, FaceAuthority, MetricRoute, PROGRAM_NOT_ENCODED, SubstitutedFont, Substitutes,
    TWO_WIDTHS_FOR_ONE_GLYPH, Table124, base_font, disagrees, name_of, program_stream,
    subtype_dictionary,
};

/// Whether this route answers the font: an `Adobe-Identity` descendant, a `/ToUnicode` stream.
///
/// Every other composite font the file does not embed keeps the refusal it had, because this
/// route's whole evidence is the producer's map from codes to characters.
pub(super) fn applies(document: &Document, font: &Dictionary, descendant: &Dictionary) -> bool {
    is_adobe_identity(document, descendant)
        && matches!(document.get_key(font, "ToUnicode"), Object::Stream(_))
}

/// Whether a descendant `CIDFont` states the `Adobe-Identity` character collection, whose CIDs
/// index only the program that defined them (§9.7.4.2).
pub(super) fn is_adobe_identity(document: &Document, descendant: &Dictionary) -> bool {
    let info = document.get_key(descendant, "CIDSystemInfo");
    let Some(info) = document.resolve(&info).as_dict().cloned() else {
        return false;
    };
    let text = |key: &str| match document.get_key(&info, key) {
        Object::String(value) => Some(value.to_vec()),
        _ => None,
    };
    text("Registry").as_deref() == Some(b"Adobe")
        && text("Ordering").as_deref() == Some(b"Identity")
}

/// What this route wrote into one descriptor: every CID shown through it with its character and
/// width, the objects it wrote and the `CIDFont`s that show it, so that a font met again on another
/// page widens the one program rather than writing a second.
#[derive(Debug)]
pub(super) struct Rekeyed {
    shown: BTreeMap<u16, (char, f32)>,
    at: ObjectId,
    map_at: Option<ObjectId>,
    /// Every `CIDFont` object that shows the descriptor's program: Table 120 makes the descriptor's
    /// `/FontName` the `/BaseFont` of each, so a program widened for a later font restates the
    /// name, and the face's other gains, on all of them (`doc/adr/2486`).
    descendants: Vec<ObjectId>,
}

/// The face the characters are drawn from.
enum Face {
    /// One of the faces compiled into this program.
    Shipped(&'static [u8], Format),
    /// The CJK pack's program for a collection.
    Pack(std::sync::Arc<[u8]>, Collection),
}

impl Face {
    /// The face's name, for the report.
    fn describe(&self) -> String {
        match self {
            Self::Shipped(program, format) => face_name(program, *format),
            Self::Pack(_, collection) => format!("QuorraCJK-{}", collection.ordering()),
        }
    }
}

/// Attaches the notice a face's licence asks of every copy: the Foxit faces' BSD text where the
/// shipped face is one (`super::notices::attach`), the pack's OFL always.
fn noticed(
    document: &Document,
    face: &Face,
    stream: &mut Object,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    match face {
        Face::Shipped(program, _) => {
            super::notices::attach(document, program, stream, spare, &mut substitutes.written)
        }
        Face::Pack(..) => super::notices::attach_notice(
            document,
            LICENCE,
            stream,
            spare,
            &mut substitutes.written,
        ),
    }
}

/// Embeds the shipped face's glyphs, chosen by the font's `/ToUnicode`, into its descendant.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] naming the condition met, and [`Because::TheFence`] where one glyph of
/// a `glyf` face would have to state two advances.
pub(super) fn embed(
    document: &Document,
    used: &SelectedFont,
    descendant: &Dictionary,
    pack: Option<&CjkPack>,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    if name_of(document, &used.dict, "Encoding").as_deref() != Some("Identity-H") {
        return Err(Because::NotBuiltYet(NOT_IDENTITY_H));
    }
    let descriptor = descendant
        .get("FontDescriptor")
        .and_then(Object::as_reference)
        .ok_or(Because::NotBuiltYet(NO_DESCRIPTOR_OBJECT))?;
    let descendant_at = descendant_object(document, &used.dict)
        .ok_or(Because::NotBuiltYet(NO_DESCENDANT_OBJECT))?;
    let mut shown = shown(document, used)?;
    let earlier = substitutes.rekeyed.remove(&descriptor);
    if earlier.is_none()
        && (substitutes.at.contains_key(&descriptor)
            || substitutes.described.contains_key(&descriptor))
    {
        return Err(Because::NotBuiltYet(SHARED_DESCRIPTOR));
    }
    if let Some(earlier) = &earlier {
        shown.extend(earlier.shown.iter().map(|(cid, held)| (*cid, *held)));
    }
    let characters: BTreeSet<char> = shown.values().map(|(character, _)| *character).collect();
    let descriptor_dict = document.get_key(descendant, "FontDescriptor");
    let descriptor_dict = document.resolve(&descriptor_dict).as_dict().cloned();
    let request = Request::derive(document, descendant, descriptor_dict.as_ref());
    let face = chosen_face(
        document,
        request,
        descriptor_dict.as_ref(),
        &characters,
        pack,
    )?;
    let at = match &earlier {
        Some(earlier) => earlier.at,
        None => spare
            .take(document)
            .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?,
    };
    let (row, mut stream, gains, map, route) = match &face {
        Face::Shipped(program, Format::Sfnt) if has_glyf(program) => {
            let (bytes, map, route) = whole_true_type(program, &shown)?;
            let stream = program_stream(subtype_dictionary(TRUE_TYPE).as_ref(), &bytes, true)
                .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
            let mut gains = Dictionary::new();
            if name_of(document, descendant, "Subtype").as_deref() != Some("CIDFontType2") {
                gains.insert(
                    Name::new(&b"Subtype"[..]),
                    Object::Name(Name::new(&b"CIDFontType2"[..])),
                );
            }
            (TRUE_TYPE, stream, gains, Some(map), route)
        }
        Face::Shipped(program, format) => compact(document, descendant, program, *format, &shown)?,
        Face::Pack(program, collection) => {
            let glyphs = pack_glyphs(program, *collection, &shown)?;
            compact_from(document, descendant, program, Format::Sfnt, &glyphs, &shown)?
        }
    };
    noticed(document, &face, &mut stream, spare, substitutes)?;
    substitutes.written.insert(at, stream);
    substitutes
        .at
        .insert(descriptor, Embedding { key: row.key, at });
    let mut gains = gains;
    let held = earlier.as_ref().and_then(|earlier| earlier.map_at);
    let map_at = placed_map(document, map, held, &mut gains, spare, substitutes)?;
    if let Some(name) = gains.get("BaseFont").cloned() {
        let mut named = Dictionary::new();
        named.insert(Name::new(&b"FontName"[..]), name);
        substitutes.described.insert(descriptor, named);
    }
    let mut descendants = earlier
        .map(|earlier| earlier.descendants)
        .unwrap_or_default();
    if !descendants.contains(&descendant_at) {
        descendants.push(descendant_at);
    }
    restate_on_each(document, &descendants, &gains, descendant, substitutes)?;
    substitutes.rekeyed.insert(
        descriptor,
        Rekeyed {
            shown,
            at,
            map_at,
            descendants,
        },
    );
    substitutes.done.push(SubstitutedFont {
        resource: used.name.clone(),
        requested: base_font(document, &used.dict),
        face: face.describe(),
        authority: FaceAuthority::Shipped,
        route,
        described: None,
        losses: Losses::one(Loss::MachineFace).with(Loss::GlyphsFromToUnicode),
        fitting: None,
        empty_glyphs: Vec::new(),
        font: super::selected(used),
    });
    Ok(())
}

/// Gives every `CIDFont` that shows the descriptor's program the gains the face brings, so that
/// each states the `/BaseFont` the descriptor now carries (Table 120).
fn restate_on_each(
    document: &Document,
    descendants: &[ObjectId],
    gains: &Dictionary,
    reference: &Dictionary,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    for sharing in descendants {
        let own = document.get(*sharing);
        let own = own
            .as_dict()
            .ok_or(Because::NotBuiltYet(NO_DESCENDANT_OBJECT))?;
        substitutes
            .described
            .insert(*sharing, gains_for(document, own, gains, reference));
    }
    Ok(())
}

/// What the face gives the `CIDFont` `own` that shares a descriptor with `reference`: the gains
/// computed for `reference`, with `/Subtype` stated only where `own` does not already state the
/// one the face's program needs.
///
/// A `CIDFont` that shows a program sits under the subtype Table 124's row for that program names
/// (`/CIDFontType2` for `/FontFile2`, `/CIDFontType0` for `/FontFile3`), so the subtype is the
/// one part of the gains that depends on the dictionary they are written into.
fn gains_for(
    document: &Document,
    own: &Dictionary,
    gains: &Dictionary,
    reference: &Dictionary,
) -> Dictionary {
    let mut restated = gains.clone();
    let needed = match gains.get("Subtype") {
        Some(Object::Name(name)) => Some(name.as_bytes().to_vec()),
        _ => name_of(document, reference, "Subtype").map(String::into_bytes),
    };
    let stated = name_of(document, own, "Subtype").map(String::into_bytes);
    match needed {
        Some(needed) if stated.as_deref() != Some(needed.as_slice()) => {
            restated.insert(
                Name::new(&b"Subtype"[..]),
                Object::Name(Name::new(needed.as_slice())),
            );
        }
        _ => {
            restated.remove("Subtype");
        }
    }
    restated
}

/// Writes the `/CIDToGIDMap` stream a `glyf` face needs at the object a font met earlier already
/// holds, or at a new one, and names it in the descendant's gains.
fn placed_map(
    document: &Document,
    map: Option<Object>,
    held: Option<ObjectId>,
    gains: &mut Dictionary,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<Option<ObjectId>, Because> {
    match (map, held) {
        (Some(map), held) => {
            let map_at = match held {
                Some(map_at) => map_at,
                None => spare
                    .take(document)
                    .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?,
            };
            substitutes.written.insert(map_at, map);
            gains.insert(Name::new(&b"CIDToGIDMap"[..]), Object::Reference(map_at));
            Ok(Some(map_at))
        }
        (None, None) => Ok(None),
        // The first page's characters were all a `glyf` face's and a later page's are not: the
        // map already written would describe a program no longer there.
        (None, Some(_)) => Err(Because::NotBuiltYet(FACE_CHANGED)),
    }
}

/// Table 124's row for a TrueType program under a `CIDFontType2`.
const TRUE_TYPE: Table124 = Table124 {
    key: "FontFile2",
    subtype: None,
};

/// Table 124's row for a bare CID-keyed CFF program under a `CIDFontType0`.
const COMPACT: Table124 = Table124 {
    key: "FontFile3",
    subtype: Some("CIDFontType0C"),
};

/// Every CID the page shows, with the character the `/ToUnicode` states for it and its width.
///
/// Under `Identity-H` the code is the CID (Table 116), and the `/ToUnicode` is keyed by code, so
/// each shown CID's value is read straight off the producer's map.
fn shown(document: &Document, used: &SelectedFont) -> Result<BTreeMap<u16, (char, f32)>, Because> {
    let Object::Stream(stream) = document.get_key(&used.dict, "ToUnicode") else {
        return Err(Because::NotBuiltYet(VALUE_NOT_USABLE));
    };
    let data = document
        .decoded_stream_data(&stream)
        .ok_or(Because::NotBuiltYet(VALUE_NOT_USABLE))?;
    let map = ToUnicode::parse(&data);
    let font = LoadedFont::metrics_only(document, &used.dict, &used.name)
        .ok_or(Because::NotBuiltYet(NOT_READ))?;
    let mut shown = BTreeMap::new();
    for text in used.shown.keys() {
        for code in font.decode(text) {
            let cid = u16::try_from(code.value())
                .ok()
                .filter(|cid| *cid != pdf_font::NOTDEF_GLYPH)
                .ok_or(Because::NotBuiltYet(CID_ZERO))?;
            let character = map
                .states(code.value())
                .then(|| map.char_for(code.value()))
                .flatten()
                .filter(|character| !UNUSABLE.contains(character))
                .ok_or(Because::NotBuiltYet(VALUE_NOT_USABLE))?;
            shown.insert(cid, (character, font.advance(code)));
        }
    }
    Ok(shown)
}

/// The face that draws every character, in the module documentation's order.
fn chosen_face(
    document: &Document,
    request: Request,
    descriptor: Option<&Dictionary>,
    characters: &BTreeSet<char>,
    pack: Option<&CjkPack>,
) -> Result<Face, Because> {
    let (program, format) = pdf_font::standard::face(request);
    if characters
        .iter()
        .all(|character| shipped_glyph(program, format, *character).is_some())
    {
        return Ok(Face::Shipped(program, format));
    }
    let pack = pack.ok_or(Because::NotBuiltYet(NO_FACE_DRAWS))?;
    for collection in collections(document, descriptor, characters) {
        let Ok(program) = pack.program(collection) else {
            continue;
        };
        if characters
            .iter()
            .all(|character| collection_cid(collection, *character).is_some())
        {
            return Ok(Face::Pack(program, collection));
        }
    }
    Err(Because::NotBuiltYet(NO_FACE_DRAWS))
}

/// The collections to try, in the order the module documentation gives.
fn collections(
    document: &Document,
    descriptor: Option<&Dictionary>,
    characters: &BTreeSet<char>,
) -> Vec<Collection> {
    use pdf_font::substitute::Language;
    match descriptor.and_then(|descriptor| pdf_font::substitute::language(document, descriptor)) {
        Some(Language::Japanese) => return vec![Collection::Japan1],
        Some(Language::Chinese) => return vec![Collection::Gb1, Collection::Cns1],
        Some(Language::Korean) => return vec![Collection::Kr, Collection::Korea1],
        _ => {}
    }
    let any = |low: u32, high: u32| {
        characters
            .iter()
            .any(|character| (low..=high).contains(&u32::from(*character)))
    };
    if any(0x3040, 0x30ff) {
        return vec![Collection::Japan1];
    }
    if any(0xac00, 0xd7af) || any(0x1100, 0x11ff) || any(0x3130, 0x318f) {
        return vec![Collection::Kr, Collection::Korea1];
    }
    // §9.7.5.2 names the four in this order: Adobe-CNS1, Adobe-GB1, Adobe-Japan1, Adobe-KR.
    vec![
        Collection::Cns1,
        Collection::Gb1,
        Collection::Japan1,
        Collection::Kr,
    ]
}

/// The `Uni*-UTF32-H` `CMap` the pack builder placed each of a collection's glyphs through.
const fn utf32_cmap(collection: Collection) -> &'static str {
    match collection {
        Collection::Japan1 => "UniJIS-UTF32-H",
        Collection::Gb1 => "UniGB-UTF32-H",
        Collection::Cns1 => "UniCNS-UTF32-H",
        Collection::Korea1 => "UniKS-UTF32-H",
        Collection::Kr => "UniAKR-UTF32-H",
    }
}

/// The collection's CID for a character, through the mapping Adobe publishes for it.
fn collection_cid(collection: Collection, character: char) -> Option<u16> {
    let cmap = pdf_font::predefined::cmap(utf32_cmap(collection))?;
    let code = cmap.next_code(&u32::from(character).to_be_bytes());
    cmap.cid(code)
        .and_then(|cid| u16::try_from(cid).ok())
        .filter(|cid| *cid != 0)
}

/// The glyph a shipped face gives a character: its Unicode `cmap` for an sfnt, the Adobe Glyph
/// List's name for it in a bare CFF's charset.
fn shipped_glyph(program: &[u8], format: Format, character: char) -> Option<u16> {
    let glyph = match format {
        Format::Sfnt => {
            let font = skrifa::FontRef::new(program).ok()?;
            u16::try_from(font.charmap().map(character)?.to_u32()).ok()?
        }
        Format::BareCff => {
            let pdf_font::cff::CodeToGlyph::Named(table) =
                pdf_font::cff::CodeToGlyph::read(program).ok()?
            else {
                return None;
            };
            let name = pdf_font::encoding::glyph_name(character)?;
            *table.by_name.get(name.as_str())?
        }
        // A format this route has no reader for draws nothing it could check.
        _ => return None,
    };
    (glyph != 0).then_some(glyph)
}

/// A shipped face's PostScript name, for the report: an sfnt's `name` table's, a bare CFF's Name
/// INDEX entry.
fn face_name(program: &[u8], format: Format) -> String {
    use skrifa::raw::FontRead as _;
    match format {
        Format::Sfnt => skrifa::FontRef::new(program)
            .ok()
            .and_then(|font| {
                font.localized_strings(skrifa::string::StringId::POSTSCRIPT_NAME)
                    .english_or_first()
                    .map(|name| name.chars().collect())
            })
            .unwrap_or_default(),
        Format::BareCff => skrifa::raw::tables::cff::Cff::read(skrifa::raw::FontData::new(program))
            .ok()
            .and_then(|cff| {
                cff.names()
                    .get(0)
                    .map(|name| String::from_utf8_lossy(name).into_owned())
            })
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Whether an sfnt carries `glyf` outlines.
fn has_glyf(program: &[u8]) -> bool {
    skrifa::FontRef::new(program).is_ok_and(|font| {
        font.table_data(skrifa::raw::types::Tag::new(b"glyf"))
            .is_some()
    })
}

/// A `glyf` face written whole, its advances the dictionary's, with the `/CIDToGIDMap` stream
/// sending each shown CID to the glyph its character selects.
fn whole_true_type(
    program: &'static [u8],
    shown: &BTreeMap<u16, (char, f32)>,
) -> Result<(Vec<u8>, Object, MetricRoute), Because> {
    let mut by_glyph: BTreeMap<u16, f32> = BTreeMap::new();
    let mut map: BTreeMap<u16, u16> = BTreeMap::new();
    let mut route = MetricRoute::FaceMetrics;
    for (cid, (character, width)) in shown {
        let glyph = shipped_glyph(program, Format::Sfnt, *character)
            .ok_or(Because::NotBuiltYet(NO_FACE_DRAWS))?;
        if !draws_or_spaces(program, Format::Sfnt, glyph, *character) {
            return Err(Because::NotBuiltYet(NO_FACE_DRAWS));
        }
        match by_glyph.get(&glyph) {
            Some(held) if disagrees(*held, *width) => {
                return Err(Because::TheFence(TWO_WIDTHS_FOR_ONE_GLYPH));
            }
            _ => {
                by_glyph.insert(glyph, *width);
            }
        }
        if pdf_font::restate::advance(program, Format::Sfnt, glyph)
            .is_none_or(|own| disagrees(*width, own))
        {
            route = MetricRoute::RestatedProgram;
        }
        map.insert(*cid, glyph);
    }
    let bytes = if route == MetricRoute::RestatedProgram {
        let modified = pdf_font::restate::with_widths(program, Format::Sfnt, &by_glyph)
            .map_err(|_| Because::NotBuiltYet(super::FACE_NOT_RESTATABLE))?;
        super::renamed_modified(&modified, Format::Sfnt)?
    } else {
        program.to_vec()
    };
    super::proves(&bytes, Format::Sfnt, &by_glyph, &BTreeMap::new())?;
    Ok((bytes, cid_to_gid_map(&map)?, route))
}

/// Table 115's `/CIDToGIDMap` stream: the glyph index for CID *c* is the two-byte value in bytes
/// 2*c* and 2*c* + 1, high-order byte first, and every CID nothing names is left at glyph 0.
fn cid_to_gid_map(map: &BTreeMap<u16, u16>) -> Result<Object, Because> {
    let highest = map.keys().next_back().copied().unwrap_or(0);
    let bytes: Vec<u8> = (0..=highest)
        .flat_map(|cid| map.get(&cid).copied().unwrap_or(0).to_be_bytes())
        .collect();
    let encoded =
        flate_encode(&bytes, COMPRESSION_LEVEL).ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    Ok(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: encoded.into(),
        decryption_failed: false,
    })))
}

/// The written pieces of a `CFF ` route: its Table 124 row, the program stream, what the
/// descendant gains, no map, and the metric route.
type Written = (Table124, Object, Dictionary, Option<Object>, MetricRoute);

/// A shipped `CFF ` face's subset, renumbered by the document's CIDs.
fn compact(
    document: &Document,
    descendant: &Dictionary,
    program: &[u8],
    format: Format,
    shown: &BTreeMap<u16, (char, f32)>,
) -> Result<Written, Because> {
    let glyphs = shown
        .iter()
        .map(|(cid, (character, _))| {
            shipped_glyph(program, format, *character)
                .map(|glyph| (*cid, glyph))
                .ok_or(Because::NotBuiltYet(NO_FACE_DRAWS))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    compact_from(document, descendant, program, format, &glyphs, shown)
}

/// The pack program's glyph for each shown CID: glyph *N* is the collection's CID *N*.
fn pack_glyphs(
    program: &[u8],
    collection: Collection,
    shown: &BTreeMap<u16, (char, f32)>,
) -> Result<BTreeMap<u16, u16>, Because> {
    shown
        .iter()
        .map(|(cid, (character, _))| {
            collection_cid(collection, *character)
                .map(|glyph| (*cid, glyph))
                .ok_or(Because::NotBuiltYet(NO_FACE_DRAWS))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
        .and_then(|glyphs| {
            let count = pdf_font::embedding::glyph_count(program, Format::Sfnt)
                .ok_or(Because::NotBuiltYet(NO_FACE_DRAWS))?;
            if glyphs.values().any(|glyph| *glyph >= count) {
                return Err(Because::NotBuiltYet(NO_FACE_DRAWS));
            }
            Ok(glyphs)
        })
}

/// A `CFF ` program's subset whose charset numbers each glyph by the document's CID, its
/// advances the dictionary's, proved, with what the descendant and descriptor gain.
fn compact_from(
    document: &Document,
    descendant: &Dictionary,
    program: &[u8],
    format: Format,
    glyphs: &BTreeMap<u16, u16>,
    shown: &BTreeMap<u16, (char, f32)>,
) -> Result<Written, Because> {
    let subset = pdf_font::embed::renumbered(program, format, glyphs)
        .map_err(|_| Because::NotBuiltYet(NOT_SUBSET))?;
    let mut by_glyph: BTreeMap<u16, f32> = BTreeMap::new();
    let mut route = MetricRoute::FaceMetrics;
    for (cid, (character, width)) in shown {
        let glyph = *subset
            .glyphs
            .get(cid)
            .ok_or(Because::NotBuiltYet(NOT_SUBSET))?;
        if !draws_or_spaces(&subset.program, Format::BareCff, glyph, *character) {
            return Err(Because::NotBuiltYet(NO_FACE_DRAWS));
        }
        if pdf_font::restate::advance(&subset.program, Format::BareCff, glyph)
            .is_none_or(|own| disagrees(*width, own))
        {
            route = MetricRoute::RestatedProgram;
        }
        by_glyph.insert(glyph, *width);
    }
    let bytes = if route == MetricRoute::RestatedProgram {
        pdf_font::restate::with_widths(&subset.program, Format::BareCff, &by_glyph)
            .map_err(|_| Because::NotBuiltYet(super::FACE_NOT_RESTATABLE))?
    } else {
        subset.program.clone()
    };
    super::proves(&bytes, Format::BareCff, &by_glyph, &BTreeMap::new())?;
    let stream = program_stream(subtype_dictionary(COMPACT).as_ref(), &bytes, false)
        .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
    let mut gains = Dictionary::new();
    gains.insert(
        Name::new(&b"BaseFont"[..]),
        Object::Name(Name::new(subset.name.as_bytes())),
    );
    if name_of(document, descendant, "Subtype").as_deref() != Some("CIDFontType0") {
        gains.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(&b"CIDFontType0"[..])),
        );
    }
    Ok((COMPACT, stream, gains, None, route))
}

/// Whether a glyph draws anything, or its character is one a blank glyph is right for.
fn draws_or_spaces(program: &[u8], format: Format, glyph: u16, character: char) -> bool {
    /// Whether any segment was drawn.
    struct Drew(bool);
    impl skrifa::outline::OutlinePen for Drew {
        fn move_to(&mut self, _: f32, _: f32) {}
        fn line_to(&mut self, _: f32, _: f32) {
            self.0 = true;
        }
        fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {
            self.0 = true;
        }
        fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
            self.0 = true;
        }
        fn close(&mut self) {}
    }
    if character.is_whitespace() {
        return true;
    }
    let mut drew = Drew(false);
    let drawn = match format {
        Format::BareCff => pdf_font::cff::draw(program, glyph, &mut drew).is_ok(),
        Format::Sfnt => skrifa::FontRef::new(program).is_ok_and(|font| {
            font.outline_glyphs()
                .get(skrifa::GlyphId::new(u32::from(glyph)))
                .is_some_and(|outline| {
                    outline
                        .draw(
                            skrifa::outline::DrawSettings::unhinted(
                                skrifa::instance::Size::unscaled(),
                                skrifa::instance::LocationRef::default(),
                            ),
                            &mut drew,
                        )
                        .is_ok()
                })
        }),
        _ => false,
    };
    drawn && drew.0
}

/// The object the `/DescendantFonts` array names, where it names one rather than holding it.
fn descendant_object(document: &Document, dict: &Dictionary) -> Option<ObjectId> {
    let array = document.get_key(dict, "DescendantFonts");
    document.resolve(&array).as_array()?.first()?.as_reference()
}

/// Why an encoding other than `Identity-H` keeps the refusal.
const NOT_IDENTITY_H: &str = "this document renders an Adobe-Identity composite font it does not \
     embed under Identity-V. Re-keying its glyphs from its ToUnicode is built for Identity-H, \
     where the window draws each character upright; a vertical font's forms are a further question \
     of which glyph the window chose, so the font is not written";

/// Why a shown code without a usable `/ToUnicode` value keeps the refusal.
const VALUE_NOT_USABLE: &str = "this document renders an Adobe-Identity composite font it does \
     not embed, and a code the page shows has no single usable character in the font's ToUnicode \
     CMap: it is not stated, it is stated as a sequence, or it is a value ISO 19005-2 section \
     6.2.11.7.2 names unusable. The glyph for such a code could be chosen only by inventing which \
     character it was, so the font is not re-keyed; --font <base-font>=<path> naming the \
     producer's own program resolves it";

/// Why a shown CID 0 keeps the refusal.
const CID_ZERO: &str = "this document shows CID 0 of an Adobe-Identity composite font it does not \
     embed, which ISO 32000-2 §9.7.4.2 reserves for the glyph a missing character draws, so no \
     character's glyph may be numbered by it";

/// Why a character no face reaches keeps the refusal.
const NO_FACE_DRAWS: &str = "this document renders an Adobe-Identity composite font it does not \
     embed, and a character its ToUnicode CMap states for a shown code is drawn by neither the face \
     this program ships for the font's name nor any installed CJK pack program (doc/cjk-pack.md), \
     or is drawn by an empty glyph that is no space. Embedding it would show nothing where the \
     page shows a mark, so it is not written";

/// Why the font did not load.
const NOT_READ: &str = "this document renders an Adobe-Identity composite font whose encoding and \
     widths this reader could not read, so which CIDs the page shows is not a question it can \
     answer";

/// Why a subset the writer could not make keeps the refusal.
const NOT_SUBSET: &str = "the face chosen for this Adobe-Identity composite font could not be \
     written as a CID-keyed subset whose charset numbers each glyph by the document's CID, which is \
     the only form in which a CIDFontType0 reaches them (ISO 32000-2 §9.7.4.2)";

/// Why a descriptor another route wrote into keeps the refusal.
const SHARED_DESCRIPTOR: &str = "this Adobe-Identity composite font's FontDescriptor is shared \
     with a font another route gave a program, and one descriptor holds one program";

/// Why a descriptor whose face changed between pages keeps the refusal.
const FACE_CHANGED: &str = "this Adobe-Identity composite font shows characters on a later page \
     that the face chosen for its first page's characters does not draw, and the CIDToGIDMap \
     written for that face would describe a program no longer embedded";

/// Why a descendant whose descriptor is written inline is refused.
const NO_DESCRIPTOR_OBJECT: &str = "this Adobe-Identity composite font's descendant CIDFont \
     writes its FontDescriptor inside itself rather than as the indirect reference ISO 32000-2's \
     Table 115 requires, and a program is embedded by writing a key into a descriptor object";

/// Why a descendant written inline is refused.
const NO_DESCENDANT_OBJECT: &str = "this Adobe-Identity composite font's DescendantFonts array \
     holds the CIDFont dictionary itself rather than a reference to one, and re-keying it writes \
     that dictionary's CIDToGIDMap or Subtype and BaseFont";
