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
//! - **A `CFF ` face as a bare CID-keyed subset under `/FontFile3` `/CIDFontType0C`, the
//!   descendant a `CIDFontType0`** (Table 124, Table 117; `doc/adr/2828`). A Type 0 `CIDFont` has
//!   no `/CIDToGIDMap`: §9.7.4.2 has its CIDs reach glyphs through the program's own charset, so
//!   [`pdf_font::embed::renumbered`] writes the glyph the window drew for each shown CID under that
//!   CID, once for each CID that shows it. A CID the window drew nothing for has no glyph such a
//!   charset could name but `.notdef`, which ISO 19005-2 section 6.2.11.8 forbids a shown code to
//!   reach, so a `CFF ` face takes no font that shows one.
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
//! its `/ToUnicode`, an extraction aid (§9.10.2), so `glyphs-from-to-unicode` stays named, and
//! `as-shown` admits it here and nowhere else: the proof has shown the output draws what the
//! window drew (owner answer A832, `doc/adr/2829`). An embedded `CMap` stream, a collection file,
//! a face the window fitted to the widths, and a descriptor another route wrote into are not this
//! route's: each keeps the route it took before.

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

/// Table 124's row for a bare CID-keyed `CFF ` program under a `CIDFontType0`.
const COMPACT: Table124 = Table124 {
    key: "FontFile3",
    subtype: Some("CIDFontType0C"),
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
    let face = (super::machine_face_named(program), FaceAuthority::Machine);
    written_in(
        document,
        (used, descendant),
        (&font, program),
        (loss, face),
        spare,
        substitutes,
    )
}

/// A composite font the window drew as nothing, its program unreadable, embedded with a program
/// whose every shown CID draws that nothing at the dictionary's widths (`doc/adr/2830`).
///
/// ISO 32000-2 §9.9 counts a program whose data does not decode whole, or that is no program in
/// its format, as no program, and this reader drew the font from its `CMap` and its `/W` alone,
/// marking nothing (`doc/adr/1094`). So every shown CID goes to an empty glyph at its advance, as
/// [`written`] sends a CID the window drew nothing for (`doc/adr/2824`), appended to the
/// `.notdef` of the face this program ships for the descendant: a `glyf` program under a
/// `CIDFontType2`, reached by a `/CIDToGIDMap` stream (§9.7.4.2), proved by [`proved`] to draw no
/// outline and the window's metrics for every code the pages show. The `CMap`, `/CIDSystemInfo`
/// and `/ToUnicode` are the producer's, untouched. The loss is `unreadable-program`.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] where the font has no `CMap` this reader reads, its descendant is no
/// dictionary, the shipped face is not a `glyf` one, or the proof does not hold.
pub(super) fn drawn_as_nothing(
    document: &Document,
    used: &SelectedFont,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    let refused = || Because::NotBuiltYet(NOT_DRAWN_AS_NOTHING);
    let descendant = super::descendant(document, &used.dict).ok_or_else(refused)?;
    let font = LoadedFont::metrics_only(document, &used.dict, &used.name).ok_or_else(refused)?;
    let descriptor = document.get_key(&descendant, "FontDescriptor");
    let descriptor = document.resolve(&descriptor).as_dict().cloned();
    let (program, _) =
        pdf_font::standard::face(Request::derive(document, &descendant, descriptor.as_ref()));
    if pdf_font::embed::has_cff_outlines(program) {
        return Err(refused());
    }
    let face = (
        pdf_font::embed::postscript_name(program),
        FaceAuthority::Shipped,
    );
    written_in(
        document,
        (used, &descendant),
        (&font, program),
        (Losses::one(Loss::UnreadableProgram), face),
        spare,
        substitutes,
    )
    .map_err(|_| refused())
}

/// Writes `program`, the face `font` was drawn from, as the composite font's program: the subset
/// and its map, the descriptor it goes into, and the report's entry naming `face` and `loss`.
fn written_in(
    document: &Document,
    (used, descendant): (&SelectedFont, &Dictionary),
    (font, program): (&LoadedFont, &[u8]),
    (loss, (face, authority)): (Losses, (String, FaceAuthority)),
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    let descendant_at = super::rekeyed::descendant_object(document, &used.dict)
        .ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?;
    if substitutes.described.contains_key(&descendant_at) {
        return Err(Because::NotBuiltYet(NOT_THIS_ROUTES));
    }
    let shown = shown(document, used, font)?;
    let written = written(program, &shown)?;
    let home = home_of(document, descendant, substitutes, &written, program)?;
    let subtype = written.subtype();
    if written.cid_to_glyph.is_none() && descendant.get("CIDToGIDMap").is_some() {
        // Table 117 gives `/CIDToGIDMap` to a Type 2 `CIDFont` alone, and a gain can add a key
        // but not take one away, so a descendant stating one keeps the route it took before.
        return Err(Because::NotBuiltYet(NOT_THIS_ROUTES));
    }
    let mut gains = Dictionary::new();
    if name_of(document, descendant, "Subtype").as_deref() != Some(subtype) {
        gains.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(subtype.as_bytes())),
        );
    }
    gains.insert(
        Name::new(&b"BaseFont"[..]),
        Object::Name(Name::new(written.name.as_bytes())),
    );
    proved(document, used, font, descendant, &gains, &written, &home)?;

    let at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
    let map_at = match &written.cid_to_glyph {
        Some(map) => {
            let map_at = spare
                .take(document)
                .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
            substitutes
                .written
                .insert(map_at, super::rekeyed::cid_to_gid_map(map)?);
            Some(map_at)
        }
        None => None,
    };
    substitutes.written.insert(at, written.stream()?);
    let embedding = Embedding {
        key: written.table.key,
        at,
    };
    let font_name = Object::Name(Name::new(written.name.as_bytes()));
    match home {
        Home::Stated(descriptor) => {
            substitutes.at.insert(descriptor, embedding);
            let mut named = Dictionary::new();
            named.insert(Name::new(&b"FontName"[..]), font_name);
            substitutes.described.insert(descriptor, named);
        }
        Home::Own(mut own) => {
            let descriptor = spare
                .take(document)
                .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
            embedding.write_into(&mut own);
            own.insert(Name::new(&b"FontName"[..]), font_name);
            substitutes
                .written
                .insert(descriptor, Object::Dictionary(own));
            gains.insert(
                Name::new(&b"FontDescriptor"[..]),
                Object::Reference(descriptor),
            );
        }
    }
    if let Some(map_at) = map_at {
        gains.insert(Name::new(&b"CIDToGIDMap"[..]), Object::Reference(map_at));
    }
    substitutes.described.insert(descendant_at, gains);
    substitutes.done.push(SubstitutedFont {
        resource: used.name.clone(),
        requested: base_font(document, &used.dict),
        face,
        authority,
        route: written.route,
        described: None,
        losses: loss,
        fitting: None,
        empty_glyphs: Vec::new(),
        empty_cids: written.empty_cids,
        twin_codes: Vec::new(),
        cut_program: None,
        font: super::selected(used),
    });
    Ok(())
}

/// Where the program's descriptor stands: the object the descendant names, or one of the
/// descendant's own that this route writes (`doc/adr/2824`).
#[derive(Debug)]
enum Home {
    /// The descriptor object the descendant names, which no other route wrote a program into:
    /// the program goes into it, under Table 124's key.
    Stated(pdf_syntax::object::ObjectId),
    /// A descriptor object of the descendant's own, holding these entries and the program: the
    /// producer's, where the descendant writes its descriptor inside itself or names one another
    /// route already wrote a program into, and every entry Table 120 requires read off the
    /// program written, where it states none.
    Own(Dictionary),
}

impl Home {
    /// The descriptor's entries as the written font will hold them, before the program's key.
    fn entries(&self, document: &Document) -> Result<Dictionary, Because> {
        let mut entries = match self {
            Self::Stated(at) => document
                .resolve(&Object::Reference(*at))
                .as_dict()
                .cloned()
                .ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?,
            Self::Own(own) => own.clone(),
        };
        for key in ["FontFile", "FontFile2", "FontFile3", "CIDSet"] {
            entries.remove(key);
        }
        Ok(entries)
    }
}

/// The descriptor the program goes into (ADR 2461's rule, which the pack route keeps too).
///
/// ISO 32000-2 Table 117 requires a `CIDFont`'s `/FontDescriptor` and makes it an indirect
/// reference, and §9.9 embeds a program by a key of that descriptor, which holds one program. So
/// the program goes into the descriptor the descendant names where no other route wrote one into
/// it; otherwise the descendant is given a descriptor object of its own — where it states none,
/// where it writes one inside itself, and where the one it names already holds another program.
/// Every entry the producer stated is carried but the program's own (`/CIDSet` describes a
/// program this file did not carry); a descendant stating none is given the entries Table 120
/// requires, read off the program written ([`built_descriptor`]).
///
/// # Errors
///
/// [`Because::NotBuiltYet`] where the descriptor is no dictionary, or one cannot be measured.
fn home_of(
    document: &Document,
    descendant: &Dictionary,
    substitutes: &Substitutes,
    written: &Written,
    program: &[u8],
) -> Result<Home, Because> {
    let copied = |stated: &Object| -> Result<Home, Because> {
        let mut own = document
            .resolve(stated)
            .as_dict()
            .cloned()
            .ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?;
        for key in ["FontFile", "FontFile2", "FontFile3", "CIDSet"] {
            own.remove(key);
        }
        Ok(Home::Own(own))
    };
    match descendant.get("FontDescriptor") {
        None | Some(Object::Null) => Ok(Home::Own(built_descriptor(&written.program, program)?)),
        Some(Object::Reference(stated))
            if !substitutes.at.contains_key(stated)
                && !substitutes.described.contains_key(stated)
                && !substitutes.written.contains_key(stated) =>
        {
            Ok(Home::Stated(*stated))
        }
        Some(stated @ (Object::Reference(_) | Object::Dictionary(_))) => copied(stated),
        Some(_) => Err(Because::NotBuiltYet(NOT_THIS_ROUTES)),
    }
}

/// A descriptor for a descendant that states none, every entry ISO 32000-2 Table 120 requires
/// measured on the program written (`subset`) or stated by the face drawn (`program`).
///
/// Table 120 requires `/Type`, `/FontName`, `/Flags`, `/FontBBox`, `/ItalicAngle`, `/Ascent`,
/// `/Descent` and `/StemV`, and `/CapHeight` "for fonts that have Latin characters"; `/FontName`
/// and the program's key are written by the caller. Each value is in glyph space, a thousandth of
/// an em (§9.2.4), scaled from the program's own units:
///
/// - `/Flags` is Table 121's `Symbolic` alone: a collection's characters lie outside the Standard
///   Latin character set, the condition that flag states, and the font is reached by CID, not by
///   a standard encoding.
/// - `/FontBBox` encloses every glyph of the program written, `.notdef` included, which is the
///   definition's "all of the glyphs of the font … placed with their origins coincident".
/// - `/Ascent` and `/Descent` are the highest and lowest points of the glyphs but `.notdef`, which
///   no shown code reaches.
/// - `/CapHeight` is the face's own `OS/2` `sCapHeight`, stated whether or not the page shows Latin
///   text, since the face has Latin characters wherever it states one.
/// - `/ItalicAngle` is the face's `post` table's, and `/StemV` is 0, Table 120's "unknown stem
///   thickness": a `glyf` program states no stem width.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] where a program does not draw, its glyphs reach no point below the
/// baseline (Table 120's `/Descent` is negative), or the face states no capital height.
fn built_descriptor(subset: &[u8], program: &[u8]) -> Result<Dictionary, Because> {
    use skrifa::MetadataProvider as _;
    use skrifa::instance::{LocationRef, Size};
    use skrifa::outline::DrawSettings;
    use skrifa::raw::TableProvider as _;

    let unmeasured = || Because::NotBuiltYet(DESCRIPTOR_NOT_MEASURED);
    let written = skrifa::FontRef::new(subset).map_err(|_| unmeasured())?;
    let face = skrifa::FontRef::new(program).map_err(|_| unmeasured())?;
    let units = written
        .metrics(Size::unscaled(), LocationRef::default())
        .units_per_em;
    if units == 0 {
        return Err(unmeasured());
    }
    let scale = 1000.0 / f32::from(units);
    let outlines = written.outline_glyphs();
    let mut all = super::pack::Bounds::default();
    let mut shown = super::pack::Bounds::default();
    for glyph in 0..written.maxp().map_err(|_| unmeasured())?.num_glyphs() {
        let mut pen = super::pack::Bounds::default();
        if let Some(outline) = outlines.get(skrifa::GlyphId::new(u32::from(glyph))) {
            outline
                .draw(
                    DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                    &mut pen,
                )
                .map_err(|_| unmeasured())?;
        }
        all.include(&pen);
        if glyph != pdf_font::NOTDEF_GLYPH {
            shown.include(&pen);
        }
    }
    let (Some(bbox), Some([_, descent, _, ascent])) = (all.rectangle(), shown.rectangle()) else {
        return Err(unmeasured());
    };
    if descent >= 0.0 {
        return Err(unmeasured());
    }
    let stated = face.metrics(Size::unscaled(), LocationRef::default());
    let cap_height = stated.cap_height.ok_or_else(unmeasured)?;
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
    dict.insert(Name::new(&b"Flags"[..]), Object::Integer(1 << 2));
    dict.insert(
        Name::new(&b"FontBBox"[..]),
        Object::Array(bbox.iter().map(|side| number(*side)).collect()),
    );
    dict.insert(
        Name::new(&b"ItalicAngle"[..]),
        Object::Real(f64::from(stated.italic_angle)),
    );
    dict.insert(Name::new(&b"Ascent"[..]), number(ascent));
    dict.insert(Name::new(&b"Descent"[..]), number(descent));
    dict.insert(Name::new(&b"CapHeight"[..]), number(cap_height));
    dict.insert(Name::new(&b"StemV"[..]), Object::Integer(0));
    Ok(dict)
}

/// The machine's face the window drew `font` from, where it drew from one this route can write.
///
/// The same conditions as a simple font's (`super::machine_face`): a substitute that is not a
/// face this program ships, not a collection, and whose own `fsType` permits embedding — §9.9.1
/// makes preview-and-print the permission every embedded program carries. And one of this
/// route's own: the face drawn at its own width, since a program states the outline it holds and
/// not a fitting. A `glyf` face and a `CFF ` one are both this route's ([`written`]).
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

/// One shown CID: the glyph of the drawn face the window drew for it, or `None` where it drew
/// none, its horizontal advance and, in writing mode 1, its vertical displacement, both in ems.
#[derive(Debug, Clone, Copy)]
struct Shown {
    glyph: Option<u16>,
    width: f32,
    height: Option<f32>,
}

/// Every CID the pages show through `used`, with what the window drew for it.
///
/// The CID is the predefined `CMap`'s (§9.7.5.2); a code it maps to no CID, or to CID 0 — the
/// `CIDFont`'s own `.notdef` under §9.7.4.2 — is not this route's. A code the face has no glyph
/// for is drawn as nothing, at its advance, and is kept as a CID that draws nothing; one the
/// window drew a mark for without a glyph of the face's own is not this route's.
fn shown(
    document: &Document,
    used: &SelectedFont,
    font: &LoadedFont,
) -> Result<BTreeMap<u16, Shown>, Because> {
    let encoding = name_of(document, &used.dict, "Encoding").unwrap_or_default();
    // Table 116 states `Identity-H` and `Identity-V` by a rule, each two-byte code its own CID,
    // rather than as a file of mappings, so no predefined file carries them (`doc/adr/2828`).
    let cmap = match encoding.as_str() {
        "Identity-H" => pdf_font::cmap::CMap::identity(),
        "Identity-V" => pdf_font::cmap::CMap::identity_vertical(),
        name => pdf_font::predefined::cmap(name).ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?,
    };
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
                .filter(|glyph| *glyph != pdf_font::NOTDEF_GLYPH);
            if glyph.is_none() && font.outline(code).is_some() {
                return Err(Because::NotBuiltYet(NOT_THIS_ROUTES));
            }
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
/// subset, the metric route, and the CIDs sent to a glyph that draws nothing.
struct Written {
    program: Vec<u8>,
    name: String,
    /// The Table 124 row the program is filed under: [`TRUE_TYPE`] or [`COMPACT`].
    table: Table124,
    /// The `/CIDToGIDMap` a `glyf` program is reached through; `None` for a `CFF ` one, whose
    /// charset numbers its glyphs by CID.
    cid_to_glyph: Option<BTreeMap<u16, u16>>,
    route: MetricRoute,
    empty_cids: Vec<u16>,
}

impl Written {
    /// Table 117's `CIDFont` subtype for the program: a Type 2 for `glyf`, a Type 0 for `CFF `.
    fn subtype(&self) -> &'static str {
        if self.cid_to_glyph.is_some() {
            "CIDFontType2"
        } else {
            "CIDFontType0"
        }
    }

    /// The program as a font file stream, with Table 125's `/Length1` for a `/FontFile2` and
    /// `/Subtype` for a `/FontFile3`.
    fn stream(&self) -> Result<Object, Because> {
        program_stream(
            subtype_dictionary(self.table).as_ref(),
            &self.program,
            self.table.key == TRUE_TYPE.key,
        )
        .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))
    }
}

/// The §9.9.2 subset of the drawn face, its advances the dictionary's.
///
/// Two CIDs the window drew from one glyph at two advances cannot share it, since a program
/// states one advance per glyph (§9.2.4), and are not this route's.
///
/// A CID the window drew nothing for is sent to a glyph of the subset's that draws nothing, one
/// for each advance such CIDs are drawn at (`doc/adr/2824`). ISO 19005-2 section 6.2.11.8 and ISO
/// 19005-4 section 6.2.10.9 forbid a text-showing operator to reference `.notdef`, which glyph 0
/// is (§9.9.2), so the `/CIDToGIDMap` may not send the CID there; an empty glyph draws what the
/// window drew. It states no character: a `CIDFontType2`'s CID reaches a glyph by index alone
/// (§9.7.4.2), and what the CID means is still its collection's and the `/ToUnicode`'s, which
/// this route leaves untouched.
fn written(program: &[u8], shown: &BTreeMap<u16, Shown>) -> Result<Written, Because> {
    if pdf_font::embed::has_cff_outlines(program) {
        return compact(program, shown);
    }
    let glyphs: BTreeSet<u16> = shown.values().filter_map(|held| held.glyph).collect();
    let mut blanks: BTreeMap<(u32, Option<u32>), usize> = BTreeMap::new();
    for held in shown.values().filter(|held| held.glyph.is_none()) {
        let next = blanks.len();
        blanks.entry(blank_key(*held)).or_insert(next);
    }
    let count = u16::try_from(blanks.len()).map_err(|_| Because::NotBuiltYet(NOT_THIS_ROUTES))?;
    let (subset, empty) = pdf_font::embed::for_embedding_with_empty_glyphs(program, &glyphs, count)
        .map_err(|_| Because::NotBuiltYet(NOT_THIS_ROUTES))?;
    let mut cid_to_glyph = BTreeMap::new();
    let mut empty_cids = Vec::new();
    let mut widths: BTreeMap<u16, f32> = BTreeMap::new();
    let mut heights: BTreeMap<u16, f32> = BTreeMap::new();
    for (cid, held) in shown {
        let glyph = if let Some(drawn) = held.glyph {
            subset.glyphs.get(&drawn)
        } else {
            empty_cids.push(*cid);
            blanks
                .get(&blank_key(*held))
                .and_then(|index| empty.get(*index))
        };
        let glyph = *glyph.ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?;
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
        table: TRUE_TYPE,
        cid_to_glyph: Some(cid_to_glyph),
        route,
        empty_cids,
    })
}

/// The §9.9.2 subset of a drawn `CFF ` face, bare and CID-keyed, each shown CID's glyph written
/// under that CID by the charset (§9.7.4.2, Table 124's `CIDFontType0C` row), its advances the
/// dictionary's (`doc/adr/2828`).
///
/// A CID the window drew nothing for is not this route's: a charset reaches a glyph only by the
/// CID it names, so the CID would reach `.notdef`, which ISO 19005-2 section 6.2.11.8 and ISO
/// 19005-4 section 6.2.10.9 forbid. A bare `CFF ` program states no vertical metrics, so section
/// 6.2.11.5's agreement holds the horizontal advances alone.
fn compact(program: &[u8], shown: &BTreeMap<u16, Shown>) -> Result<Written, Because> {
    let by_cid = shown
        .iter()
        .map(|(cid, held)| held.glyph.map(|glyph| (*cid, glyph)))
        .collect::<Option<BTreeMap<u16, u16>>>()
        .ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?;
    let subset = pdf_font::embed::renumbered(program, Format::Sfnt, &by_cid)
        .map_err(|_| Because::NotBuiltYet(NOT_THIS_ROUTES))?;
    let mut widths: BTreeMap<u16, f32> = BTreeMap::new();
    for (cid, held) in shown {
        let glyph = *subset
            .glyphs
            .get(cid)
            .ok_or(Because::NotBuiltYet(NOT_THIS_ROUTES))?;
        widths.insert(glyph, held.width);
    }
    let mut bytes = subset.program;
    let mut route = MetricRoute::FaceMetrics;
    if widths.iter().any(|(glyph, width)| {
        pdf_font::restate::advance(&bytes, Format::BareCff, *glyph)
            .is_none_or(|own| disagrees(*width, own))
    }) {
        bytes = pdf_font::restate::with_widths(&bytes, Format::BareCff, &widths)
            .map_err(|_| Because::NotBuiltYet(super::FACE_NOT_RESTATABLE))?;
        route = MetricRoute::RestatedProgram;
    }
    super::proves(&bytes, Format::BareCff, &widths, &BTreeMap::new())?;
    Ok(Written {
        program: bytes,
        name: subset.name,
        table: COMPACT,
        cid_to_glyph: None,
        route,
        empty_cids: Vec::new(),
    })
}

/// The metrics an empty glyph is written at: the advance and the vertical displacement, as bits.
fn blank_key(held: Shown) -> (u32, Option<u32>) {
    (held.width.to_bits(), held.height.map(f32::to_bits))
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
    home: &Home,
) -> Result<(), Because> {
    let unproved = |_| Because::NotBuiltYet(NOT_PROVED);
    let mut assembly = Assembly::new(Vec::new());
    let program_at = assembly.add(written.stream()?).map_err(unproved)?;
    let map_at = match &written.cid_to_glyph {
        Some(map) => Some(
            assembly
                .add(super::rekeyed::cid_to_gid_map(map)?)
                .map_err(unproved)?,
        ),
        None => None,
    };
    let Object::Dictionary(mut descriptor) =
        super::described::resolved(document, &Object::Dictionary(home.entries(document)?), 4)
    else {
        return Err(Because::NotBuiltYet(NOT_PROVED));
    };
    descriptor.insert(
        Name::new(written.table.key.as_bytes()),
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
    if let Some(map_at) = map_at {
        cid_font.insert(Name::new(&b"CIDToGIDMap"[..]), Object::Reference(map_at));
    }
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
const NOT_THIS_ROUTES: &str = "the composite font is not drawn from a machine's face this route \
     can embed as the window drew it";

/// Why a descendant stating no descriptor is not given one.
const DESCRIPTOR_NOT_MEASURED: &str = "the composite font's descendant states no descriptor, and \
     the machine's face the window drew it from does not state every entry ISO 32000-2 Table 120 \
     requires of one: its glyphs reach no point below the baseline, or it states no capital height";

/// Why a composite font the window drew as nothing is not written drawing that nothing.
const NOT_DRAWN_AS_NOTHING: &str = "this viewer drew the composite font as nothing, its program \
     unreadable, and a program sending every shown CID to an empty glyph at the file's widths \
     could not be written and proved to draw that nothing";

/// Why a written font that did not draw what the window drew is not written.
const NOT_PROVED: &str = "the composite font written with the machine's face the window drew \
     does not draw, code by code, the outline and metrics the window drew";
