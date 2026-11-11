//! The machine's face the window drew a composite font from, embedded as that font's program
//! (`doc/adr/2807`).
//!
//! # What the window drew, and why that face is the one to embed
//!
//! A composite font the file does not embed is drawn by the window from a face of its own (ADR
//! 0270): each code goes through the font's `CMap` to a CID (ISO 32000-2 §9.7.6.2), the CID to the
//! character its collection's published map or the font's `/ToUnicode` states, and the character
//! to a glyph of the face — on this machine, for the documents that reach here, a face installed
//! on it rather than one this program ships. The CJK pack and the re-keying routes embed another
//! face, so the archive shows other shapes than the window did, which is the loss `machine-face`.
//! This route embeds the face that was drawn instead, as ADR 2773 does for a simple font, and only
//! where the profile withholds that loss (ADR 2774), so `as-if-printed` keeps the face every
//! machine would write.
//!
//! # How the program is written
//!
//! - **A `glyf` face under `/FontFile2`, the descendant a `CIDFontType2`** — Table 124's row for a
//!   TrueType program, and Table 117's subtype for a `CIDFont` whose glyphs it describes.
//! - **A §9.9.2 subset**, by [`pdf_font::embed::for_embedding`], which keeps every glyph a shown
//!   CID reaches, `.notdef`, and every component those name, and writes the face whole where its
//!   `fsType` forbids subsetting. A face of tens of thousands of glyphs is not carried whole for a
//!   page that shows a few.
//! - **A `/CIDToGIDMap` stream sending each shown CID to the glyph the window drew for it.**
//!   §9.7.4.2 makes that stream what maps a Type 2 `CIDFont`'s CIDs to the glyph indices of its
//!   embedded program, and ISO 19005-2 section 6.2.11.3.2 requires one for every Type 2 `CIDFont`
//!   used for rendering. The glyph is the window's own answer ([`LoadedFont::glyph_index`]), so a
//!   vertical form the window chose (§9.7.5.1's NOTE) is the glyph written.
//! - **Advances restated to the dictionary's `/W` and `/DW`** (§9.7.4.3), and the vertical ones to
//!   `/W2` and `/DW2` where the program states vertical metrics at all, which ISO 19005-2 section
//!   6.2.11.5 and ISO 19005-4 section 6.2.10.5 require to agree with the program.
//! - **The `CMap`, `/CIDSystemInfo` and `/ToUnicode` are the producer's, untouched**: the CIDs did
//!   not change, so ISO 19005-2 section 6.2.11.3.1's agreement between the `CMap` and the
//!   collection holds as it held, and a predefined `CMap` stays one Table 116 names (section
//!   6.2.11.3.3).
//!
//! # What is proved before anything is written
//!
//! The font as written is assembled into a document of its own, loaded by the reader the window
//! uses, and held to the window code by code: the same outline, bit for bit, for every code the
//! page shows, the same horizontal advance and the same vertical metrics, and a program of its
//! own rather than a substitute. A code the written font would draw otherwise keeps the route
//! the font took before, and with it the loss that route names.
//!
//! # The loss that is left, and the routes this does not answer
//!
//! A font of an Adobe character collection is drawn through the collection's own published map
//! from CIDs to characters, so nothing is left to name. An `Adobe-Identity` font is drawn through
//! its `/ToUnicode`, an extraction aid (§9.10.2), so `glyphs-from-to-unicode` stays named. An
//! embedded `CMap` stream, a face that is not `glyf`, a collection file, a face the window fitted
//! to the widths, and a descriptor another route wrote into are not this route's: each keeps the
//! route it took before.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::SelectedFont;
use pdf_font::LoadedFont;
use pdf_font::substitute::{Family, Format, Request};
use pdf_syntax::object::{Dictionary, Name, Object};
use pdf_syntax::serialize::{Assembly, Form, Options, serialize};
use pdf_syntax::{Document, Limits, Version};

use super::super::decision::{Because, Loss, Losses};
use super::super::prepare::Spare;
use super::{
    Embedding, FaceAuthority, MetricRoute, PROGRAM_NOT_ENCODED, SubstitutedFont, Substitutes,
    Table124, base_font, disagrees, name_of, program_stream, subtype_dictionary,
};

/// Table 124's row for a TrueType program under a `CIDFontType2`.
const TRUE_TYPE: Table124 = Table124 {
    key: "FontFile2",
    subtype: None,
};

/// Embeds the machine's face the window drew `used` from into its descendant `CIDFont`.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] naming why this route does not answer the font; the caller then takes
/// the route the font took before, which names the loss `machine-face`.
pub(super) fn embed(
    document: &Document,
    used: &SelectedFont,
    descendant: &Dictionary,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    let loss = if super::pack::collection_of(document, &used.dict, descendant).is_some() {
        Losses::default()
    } else if super::rekeyed::applies(document, &used.dict, descendant) {
        Losses::one(Loss::GlyphsFromToUnicode)
    } else {
        return Err(Because::NotBuiltYet(NOT_THIS_ROUTES));
    };
    let font = LoadedFont::load(document, &used.dict, &used.name)
        .map_err(|_| Because::NotBuiltYet(NOT_THIS_ROUTES))?;
    let program = drawn_face(document, descendant, &font)?;
    let descendant_at = super::rekeyed::descendant_object(document, &used.dict)
        .ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?;
    let descriptor = descendant
        .get("FontDescriptor")
        .and_then(Object::as_reference)
        .ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?;
    if substitutes.at.contains_key(&descriptor)
        || substitutes.described.contains_key(&descriptor)
        || substitutes.described.contains_key(&descendant_at)
    {
        return Err(Because::NotBuiltYet(NOT_THIS_ROUTES));
    }
    let shown = shown(document, used, &font)?;
    let written = written(program, &shown)?;
    let mut gains = Dictionary::new();
    if name_of(document, descendant, "Subtype").as_deref() != Some("CIDFontType2") {
        gains.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(&b"CIDFontType2"[..])),
        );
    }
    gains.insert(
        Name::new(&b"BaseFont"[..]),
        Object::Name(Name::new(written.name.as_bytes())),
    );
    proved(document, used, &font, descendant, &gains, &written)?;

    let at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
    let map_at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
    let stream = program_stream(
        subtype_dictionary(TRUE_TYPE).as_ref(),
        &written.program,
        true,
    )
    .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
    substitutes.written.insert(at, stream);
    substitutes.written.insert(
        map_at,
        super::rekeyed::cid_to_gid_map(&written.cid_to_glyph)?,
    );
    substitutes.at.insert(
        descriptor,
        Embedding {
            key: TRUE_TYPE.key,
            at,
        },
    );
    gains.insert(Name::new(&b"CIDToGIDMap"[..]), Object::Reference(map_at));
    let mut named = Dictionary::new();
    named.insert(
        Name::new(&b"FontName"[..]),
        Object::Name(Name::new(written.name.as_bytes())),
    );
    substitutes.described.insert(descriptor, named);
    substitutes.described.insert(descendant_at, gains);
    substitutes.done.push(SubstitutedFont {
        resource: used.name.clone(),
        requested: base_font(document, &used.dict),
        face: super::machine_face_named(program),
        authority: FaceAuthority::Machine,
        route: written.route,
        described: None,
        losses: loss,
        fitting: None,
        empty_glyphs: Vec::new(),
        twin_codes: Vec::new(),
        cut_program: None,
        font: super::selected(used),
    });
    Ok(())
}

/// The machine's face the window drew `font` from, where it drew from one this route can write.
///
/// The same conditions as a simple font's (`super::machine_face`): a substitute that is not a
/// face this program ships, not a collection, and whose own `fsType` permits embedding — §9.9.1
/// makes preview-and-print the permission every embedded program carries. And two of this
/// route's own: `glyf` outlines, which the `/CIDToGIDMap` reaches (a `CFF ` face would need its
/// charset renumbered instead), and the face drawn at its own width, since a program states the
/// outline it holds and not a fitting.
fn drawn_face<'a>(
    document: &Document,
    descendant: &Dictionary,
    font: &'a LoadedFont,
) -> Result<&'a [u8], Because> {
    let program = font
        .substitute_program()
        .ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?;
    let descriptor = document.get_key(descendant, "FontDescriptor");
    let descriptor = document.resolve(&descriptor).as_dict().cloned();
    let request = Request::derive(document, descendant, descriptor.as_ref());
    if is_shipped(program, request)
        || program.starts_with(b"ttcf")
        || !pdf_font::embed::may_be_embedded(program)
        || pdf_font::embed::has_cff_outlines(program)
        || font.stretch().to_bits() != 1.0_f32.to_bits()
    {
        return Err(Because::NotBuiltYet(NOT_THIS_ROUTES));
    }
    Ok(program)
}

/// Whether `program` is one of the faces this program ships, for any request.
fn is_shipped(program: &[u8], request: Request) -> bool {
    [
        Family::Serif,
        Family::SansSerif,
        Family::Monospace,
        Family::Symbol,
        Family::ZapfDingbats,
    ]
    .into_iter()
    .flat_map(|family| {
        [(false, false), (true, false), (false, true), (true, true)].map(|(bold, italic)| Request {
            family,
            bold,
            italic,
            ..request
        })
    })
    .any(|asked| pdf_font::standard::face(asked).0 == program)
}

/// One shown CID: the glyph of the drawn face the window drew for it, its horizontal advance and,
/// in writing mode 1, its vertical displacement, both in ems.
#[derive(Debug, Clone, Copy)]
struct Shown {
    glyph: u16,
    width: f32,
    height: Option<f32>,
}

/// Every CID the pages show through `used`, with what the window drew for it.
///
/// The CID is the predefined `CMap`'s (§9.7.5.2); a code it maps to no CID, or to CID 0 — the
/// `CIDFont`'s own `.notdef` under §9.7.4.2 — and a code the window drew no glyph for are not
/// this route's.
fn shown(
    document: &Document,
    used: &SelectedFont,
    font: &LoadedFont,
) -> Result<BTreeMap<u16, Shown>, Because> {
    let encoding = name_of(document, &used.dict, "Encoding").unwrap_or_default();
    let cmap =
        pdf_font::predefined::cmap(&encoding).ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?;
    let mut shown: BTreeMap<u16, Shown> = BTreeMap::new();
    for text in used.shown.keys() {
        for code in font.decode(text) {
            let cid = cmap
                .cid(code)
                .and_then(|cid| u16::try_from(cid).ok())
                .filter(|cid| *cid != 0)
                .ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?;
            let glyph = font
                .glyph_index(code)
                .filter(|glyph| *glyph != pdf_font::NOTDEF_GLYPH)
                .ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?;
            let height = font.is_vertical().then(|| font.vertical_metrics(code).0[1]);
            let this = Shown {
                glyph,
                width: font.advance(code),
                height,
            };
            match shown.get(&cid) {
                Some(held)
                    if held.glyph != this.glyph
                        || held.width.to_bits() != this.width.to_bits()
                        || held.height.map(f32::to_bits) != this.height.map(f32::to_bits) =>
                {
                    return Err(Because::NotBuiltYet(NOT_THIS_ROUTES));
                }
                _ => {
                    shown.insert(cid, this);
                }
            }
        }
    }
    Ok(shown)
}

/// The program as written: the subset, its name, the map from each shown CID to its glyph in the
/// subset, and the metric route.
struct Written {
    program: Vec<u8>,
    name: String,
    cid_to_glyph: BTreeMap<u16, u16>,
    route: MetricRoute,
}

/// The §9.9.2 subset of the drawn face, its advances the dictionary's.
///
/// Two CIDs the window drew from one glyph at two advances cannot share it, since a program
/// states one advance per glyph (§9.2.4), and are not this route's.
fn written(program: &[u8], shown: &BTreeMap<u16, Shown>) -> Result<Written, Because> {
    let glyphs: BTreeSet<u16> = shown.values().map(|held| held.glyph).collect();
    let subset = pdf_font::embed::for_embedding(program, &glyphs)
        .map_err(|_| Because::NotBuiltYet(NOT_THIS_ROUTES))?;
    let mut cid_to_glyph = BTreeMap::new();
    let mut widths: BTreeMap<u16, f32> = BTreeMap::new();
    let mut heights: BTreeMap<u16, f32> = BTreeMap::new();
    for (cid, held) in shown {
        let glyph = *subset
            .glyphs
            .get(&held.glyph)
            .ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?;
        cid_to_glyph.insert(*cid, glyph);
        for (map, value) in [(&mut widths, Some(held.width)), (&mut heights, held.height)] {
            let Some(value) = value else { continue };
            match map.get(&glyph) {
                Some(earlier) if disagrees(*earlier, value) => {
                    return Err(Because::NotBuiltYet(NOT_THIS_ROUTES));
                }
                _ => {
                    map.insert(glyph, value);
                }
            }
        }
    }
    let mut bytes = subset.program;
    let mut route = MetricRoute::FaceMetrics;
    if widths.iter().any(|(glyph, width)| {
        pdf_font::restate::advance(&bytes, Format::Sfnt, *glyph)
            .is_none_or(|own| disagrees(*width, own))
    }) {
        bytes = pdf_font::restate::with_widths(&bytes, Format::Sfnt, &widths)
            .map_err(|_| Because::NotBuiltYet(super::FACE_NOT_RESTATABLE))?;
        route = MetricRoute::RestatedProgram;
    }
    // A program stating no vertical metrics leaves section 6.2.11.5's agreement nothing to hold
    // to; one that states them is restated to the dictionary's.
    let states_vertical = heights.keys().next().is_some_and(|glyph| {
        pdf_font::restate::vertical_advance(&bytes, Format::Sfnt, *glyph).is_some()
    });
    if !states_vertical {
        heights.clear();
    } else if heights.iter().any(|(glyph, height)| {
        pdf_font::restate::vertical_advance(&bytes, Format::Sfnt, *glyph)
            .is_none_or(|own| disagrees(*height, own))
    }) {
        bytes = pdf_font::restate::with_vertical_advances(&bytes, Format::Sfnt, &heights)
            .map_err(|_| Because::NotBuiltYet(super::FACE_NOT_RESTATABLE))?;
        route = MetricRoute::RestatedProgram;
    }
    super::proves(&bytes, Format::Sfnt, &widths, &heights)?;
    Ok(Written {
        program: bytes,
        name: subset.name,
        cid_to_glyph,
        route,
    })
}

/// Writes the composite font as it will stand into a document of its own and holds it to the
/// window, code by code (ADR 2773's proof, at composite scale).
///
/// The written font must draw from its own program, and for every code the pages show draw the
/// window's outline bit for bit, at the window's advance and vertical metrics. The outline is
/// compared once wound one way, because the viewer winds a substitute's glyphs counter-clockwise
/// and leaves an embedded program's as drawn (ADR 0396), and §8.5.3.3's two rules paint a glyph
/// alike either way.
fn proved(
    document: &Document,
    used: &SelectedFont,
    source: &LoadedFont,
    descendant: &Dictionary,
    gains: &Dictionary,
    written: &Written,
) -> Result<(), Because> {
    let unproved = |_| Because::NotBuiltYet(NOT_PROVED);
    let mut assembly = Assembly::new(Vec::new());
    let program = program_stream(
        subtype_dictionary(TRUE_TYPE).as_ref(),
        &written.program,
        true,
    )
    .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
    let program_at = assembly.add(program).map_err(unproved)?;
    let map = super::rekeyed::cid_to_gid_map(&written.cid_to_glyph)?;
    let map_at = assembly.add(map).map_err(unproved)?;
    let stated = document.get_key(descendant, "FontDescriptor");
    let Object::Dictionary(mut descriptor) = super::described::resolved(document, &stated, 4)
    else {
        return Err(Because::NotBuiltYet(NOT_PROVED));
    };
    for key in ["FontFile", "FontFile2", "FontFile3", "CIDSet"] {
        descriptor.remove(key);
    }
    descriptor.insert(
        Name::new(TRUE_TYPE.key.as_bytes()),
        Object::Reference(program_at),
    );
    let descriptor_at = assembly
        .add(Object::Dictionary(descriptor))
        .map_err(unproved)?;
    let mut cid_font = Dictionary::new();
    for (key, value) in descendant.iter() {
        cid_font.insert(key.clone(), super::described::resolved(document, value, 4));
    }
    for (key, value) in gains.iter() {
        cid_font.insert(key.clone(), value.clone());
    }
    cid_font.insert(Name::new(&b"CIDToGIDMap"[..]), Object::Reference(map_at));
    cid_font.insert(
        Name::new(&b"FontDescriptor"[..]),
        Object::Reference(descriptor_at),
    );
    let cid_font_at = assembly
        .add(Object::Dictionary(cid_font))
        .map_err(unproved)?;
    let mut font = Dictionary::new();
    for (key, value) in used.dict.iter() {
        // `/ToUnicode` selects no glyph of an embedded program, and the `CMap` is a predefined
        // name, which the copy carries as it is.
        if key.as_bytes() != b"ToUnicode" {
            font.insert(key.clone(), super::described::resolved(document, value, 4));
        }
    }
    font.insert(
        Name::new(&b"DescendantFonts"[..]),
        Object::Array(vec![Object::Reference(cid_font_at)]),
    );
    let font_at = assembly.add(Object::Dictionary(font)).map_err(unproved)?;
    let mut pages = Dictionary::new();
    pages.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Pages"[..])),
    );
    pages.insert(Name::new(&b"Kids"[..]), Object::Array(Vec::new()));
    pages.insert(Name::new(&b"Count"[..]), Object::Integer(0));
    let pages_at = assembly.add(Object::Dictionary(pages)).map_err(unproved)?;
    let mut catalog = Dictionary::new();
    catalog.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Catalog"[..])),
    );
    catalog.insert(Name::new(&b"Pages"[..]), Object::Reference(pages_at));
    let root = assembly
        .add(Object::Dictionary(catalog))
        .map_err(unproved)?;
    assembly.set_root(root);
    let mut bytes = Vec::new();
    serialize(
        &assembly,
        Version { major: 1, minor: 7 },
        Options::new(Form::Table),
        &mut bytes,
    )
    .map_err(|_| Because::NotBuiltYet(NOT_PROVED))?;
    let reread = Document::open_with_limits(bytes, Limits::DEFAULT)
        .map_err(|_| Because::NotBuiltYet(NOT_PROVED))?;
    let dict = reread
        .get(font_at)
        .as_dict()
        .cloned()
        .ok_or(Because::NotBuiltYet(NOT_PROVED))?;
    let converted = LoadedFont::load(&reread, &dict, &used.name)
        .map_err(|_| Because::NotBuiltYet(NOT_PROVED))?;
    if converted.is_substituted() || !draws_as_drawn(used, source, &converted) {
        return Err(Because::NotBuiltYet(NOT_PROVED));
    }
    Ok(())
}

/// Whether `converted` draws every code the pages show through `used` as the window drew it from
/// `source`: the outline once wound one way, the advance and the vertical metrics bit for bit.
fn draws_as_drawn(used: &SelectedFont, source: &LoadedFont, converted: &LoadedFont) -> bool {
    for text in used.shown.keys() {
        for code in source.decode(text) {
            let same_outline = match (source.outline(code), converted.outline(code)) {
                (Some(before), Some(after)) => super::described::same_once_wound(&before, &after),
                (None, None) => true,
                _ => false,
            };
            let same_metrics = source.advance(code).to_bits() == converted.advance(code).to_bits()
                && bits(source.vertical_metrics(code)) == bits(converted.vertical_metrics(code));
            if !same_outline || !same_metrics {
                return false;
            }
        }
    }
    true
}

/// §9.7.4.3's `w1` and `v` as the bits of each number, compared exactly.
fn bits((displacement, position): ([f32; 2], [f32; 2])) -> [u32; 4] {
    [
        displacement[0].to_bits(),
        displacement[1].to_bits(),
        position[0].to_bits(),
        position[1].to_bits(),
    ]
}

/// Why a composite font is not this route's: the window drew it from no machine's face this
/// route can write, or the font is shaped in a way the module documentation lists.
const NOT_THIS_ROUTES: &str = "the composite font is not drawn from a machine's glyf face this \
     route can embed as the window drew it";

/// Why a written font that did not draw what the window drew is not written.
const NOT_PROVED: &str = "the composite font written with the machine's face the window drew \
     does not draw, code by code, the outline and metrics the window drew";
