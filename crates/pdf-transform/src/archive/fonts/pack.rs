//! The CJK pack: one CID-keyed program per Adobe character collection, downloaded beside this
//! program, and the route that embeds a subset of one into a composite font the file did not
//! embed (owner answer A569, ADRs 2302 and 2303).
//!
//! # Why a program of the collection answers the font, and only such a program
//!
//! ISO 32000-2 §9.7.2:
//!
//! > A character collection is an ordered set of glyphs. The order of the glyphs in the character
//! > collection shall determine the CID number for each glyph.
//!
//! So a CID of `Adobe-Japan1` names the same glyph in every font of that collection, and a
//! program whose glyph N is the collection's CID N can stand in for a producer's program that was
//! never embedded. `tools/cjk-pack.py` builds those programs from Source Han Sans, placing a glyph
//! only through a mapping Adobe publishes for the collection; `doc/cjk-pack.md` is the layout.
//! An `Adobe-Identity` font is not this route's: its CIDs are the glyph order of the producer's
//! own program (§9.7.4.2), and [`super::rekeyed`] answers it through its `/ToUnicode` (ADR 2378).
//!
//! # What reaches the output
//!
//! A §9.9.2 subset of the pack program, written bare under Table 124's `CIDFontType0C` row: the
//! subsetter keeps each shown glyph at its CID through the program's charset, so the content
//! stream is untouched. The descendant becomes a `CIDFontType0` where it said `CIDFontType2`,
//! because Table 124 puts a CFF program under that subtype and no program was there for the old
//! subtype to describe; its `/BaseFont` and the descriptor's `/FontName` become the subset's name,
//! which Table 117 requires of a `CIDFontType0` and §9.9.2 of a subset. A descendant that states
//! no descriptor, writes one inside itself, or names one that already holds a subset at other
//! widths is given a descriptor object of its own (`home_of`, ADR 2461). The Type 0 font's own
//! `/BaseFont` is the producer's statement of the face they meant and is kept (Table 119 asks the
//! concatenation with a "should").
//!
//! The face drawn is the pack's and not the producer's, which is ADR 2162's named loss
//! `machine-face`: `as-if-printed` discards it and `as-shown` stops. A shown CID the pack has no
//! glyph for is refused under both, because that is a mark lost.
//!
//! # Where the pack comes from
//!
//! **The caller's**, as every other input of a conversion is: `apply` opens no path. The binary
//! looks beside its executable and where `--cjk-pack` points, and never fetches anything; this
//! module verifies what it is handed against the digests below, so a program that is not the one
//! this tree's `tools/cjk-pack.py` writes is refused as not this pack rather than embedded.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use pdf_archive::survey::SelectedFont;
use pdf_font::LoadedFont;
use pdf_font::substitute::Format;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};
use sha2::{Digest as _, Sha256};
use skrifa::MetadataProvider as _;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};

use super::super::decision::{Because, Loss, Losses};
use super::super::prepare::Spare;
use super::{
    Embedding, FaceAuthority, MetricRoute, PROGRAM_NOT_ENCODED, SubstitutedFont, Substitutes,
    TWO_WIDTHS_FOR_ONE_GLYPH, Table124, base_font, disagrees, name_of, program_stream,
    subtype_dictionary,
};

/// The notice every pack program carries, verbatim: the family's own `LICENSE.txt`.
pub(super) const LICENCE: &str = include_str!("../../../../../data/cjk-pack/LICENSE.txt");

/// One of the character collections the pack holds a program for.
///
/// §9.7.5.2 requires a processor to support `Adobe-CNS1-7`, `Adobe-GB1-5`, `Adobe-Japan1-7` and
/// `Adobe-KR-9`; `Adobe-Korea1` is here as well because every Korean `CMap` Table 116 names maps
/// codes to its CIDs rather than to `Adobe-KR`'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Collection {
    /// `Adobe-Japan1`, supplement 7.
    Japan1,
    /// `Adobe-GB1`, supplement 5.
    Gb1,
    /// `Adobe-CNS1`, supplement 7.
    Cns1,
    /// `Adobe-Korea1`, supplement 2.
    Korea1,
    /// `Adobe-KR`, supplement 9.
    Kr,
}

impl Collection {
    /// Every collection the pack holds, in the order `SHA256SUMS` lists them.
    pub const ALL: [Self; 5] = [Self::Japan1, Self::Gb1, Self::Cns1, Self::Korea1, Self::Kr];

    /// The collection a `/CIDSystemInfo`'s `/Ordering` names under the `Adobe` registry.
    #[must_use]
    pub fn from_ordering(ordering: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|collection| collection.ordering() == ordering)
    }

    /// Table 114's `/Ordering`.
    #[must_use]
    pub const fn ordering(self) -> &'static str {
        match self {
            Self::Japan1 => "Japan1",
            Self::Gb1 => "GB1",
            Self::Cns1 => "CNS1",
            Self::Korea1 => "Korea1",
            Self::Kr => "KR",
        }
    }

    /// The program's file name inside the pack directory.
    #[must_use]
    pub const fn file(self) -> &'static str {
        match self {
            Self::Japan1 => "Adobe-Japan1.otf",
            Self::Gb1 => "Adobe-GB1.otf",
            Self::Cns1 => "Adobe-CNS1.otf",
            Self::Korea1 => "Adobe-Korea1.otf",
            Self::Kr => "Adobe-KR.otf",
        }
    }

    /// The SHA-256 of the program `tools/cjk-pack.py` writes, which is how this program knows the
    /// bytes it was handed are the pack's: a pack of another build is refused as not this one, and
    /// CI holds the pack it publishes to these before it uploads it (A596, ADR 2386).
    const fn digest(self) -> &'static str {
        match self {
            Self::Japan1 => "ba07bdf0a11c3566de98dd4dc59710b452a198b58aaafd72b8bd807082066076",
            Self::Gb1 => "e5ad2631a81f3f80bb0611aba2f05d797e39bef5740e3a4613d7123b93588b96",
            Self::Cns1 => "1022435a520cbda14c59234b73605d8370b4ebf7ac380718c871885ef10fe859",
            Self::Korea1 => "67e78f3d70173a7ebfdbfac2e24b1f15cba95d8471041b3dc2d914634b50cbfc",
            Self::Kr => "77ddb2f4bf9e7fb2b8f71a047f36bf9b6fccc180abaddb63b11d558edd3ef6e1",
        }
    }

    /// Where this collection sits in [`Collection::ALL`].
    fn index(self) -> usize {
        match self {
            Self::Japan1 => 0,
            Self::Gb1 => 1,
            Self::Cns1 => 2,
            Self::Korea1 => 3,
            Self::Kr => 4,
        }
    }
}

/// What reads a pack file for the conversion: the caller's, so that `apply` opens no path.
pub trait PackFiles: std::fmt::Debug + Send + Sync {
    /// The bytes of the named file of the pack, or `None` where there is no such file.
    fn read(&self, file: &str) -> Option<Vec<u8>>;
}

/// Why the pack did not supply a collection's program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackGap {
    /// The file is not where the caller looked.
    Absent,
    /// The file is there and is not the program this tree's builder writes.
    NotThisPack,
}

/// The CJK pack a conversion may embed from, each program read and verified on first use.
#[derive(Debug, Clone)]
pub struct CjkPack {
    files: Arc<dyn PackFiles>,
    loaded: Arc<[Loaded; 5]>,
}

/// One collection's program once it was asked for: its verified bytes, or why there are none.
type Loaded = OnceLock<Result<Arc<[u8]>, PackGap>>;

/// Two packs are one pack when they read through the same reader: a plan's equality is a question
/// about what its caller handed in, and comparing bytes would read a pack nobody asked for yet.
impl PartialEq for CjkPack {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.files, &other.files)
    }
}

impl CjkPack {
    /// A pack read through `files`, nothing of it read yet.
    #[must_use]
    pub fn new(files: Arc<dyn PackFiles>) -> Self {
        Self {
            files,
            loaded: Arc::new(std::array::from_fn(|_| OnceLock::new())),
        }
    }

    /// The collection's program, read once and held to its digest.
    ///
    /// # Errors
    ///
    /// [`PackGap::Absent`] where the caller's reader has no such file, and
    /// [`PackGap::NotThisPack`] where its bytes are not the ones `tools/cjk-pack.py` writes.
    pub fn program(&self, collection: Collection) -> Result<Arc<[u8]>, PackGap> {
        self.loaded[collection.index()]
            .get_or_init(|| {
                let bytes = self.files.read(collection.file()).ok_or(PackGap::Absent)?;
                if hex(&Sha256::digest(&bytes)) == collection.digest() {
                    Ok(Arc::from(bytes))
                } else {
                    Err(PackGap::NotThisPack)
                }
            })
            .clone()
    }
}

/// Lower-case hexadecimal, the form `SHA256SUMS` and [`Collection::digest`] write.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        // Writing into a `String` cannot fail.
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// The collection a composite font's descendant names, where the pack is the route for it.
///
/// `None` sends the font back to the refusal it had: no collection the pack holds, or an encoding
/// this route does not read the CIDs through.
pub(super) fn collection_of(
    document: &Document,
    font: &Dictionary,
    descendant: &Dictionary,
) -> Option<Collection> {
    let info = document.get_key(descendant, "CIDSystemInfo");
    let info = document.resolve(&info).as_dict()?.clone();
    let registry = document.get_key(&info, "Registry");
    let ordering = document.get_key(&info, "Ordering");
    let text = |object: &Object| -> Option<String> {
        match document.resolve(object) {
            Object::String(value) => Some(String::from_utf8_lossy(&value).into_owned()),
            _ => None,
        }
    };
    if text(&registry)? != "Adobe" {
        return None;
    }
    let collection = Collection::from_ordering(&text(&ordering)?)?;
    // **A predefined `CMap` and not an identity one.** §9.7.5.2: "The Identity-H and Identity-V
    // CMaps shall not be used with a non-embedded font. Only standardized character sets may be
    // used." A file that does so states codes that are no collection's CIDs by the clause's own
    // reading, so it keeps its refusal; an embedded `CMap` stream is not read by this route.
    let encoding = name_of(document, font, "Encoding")?;
    (encoding != "Identity-H"
        && encoding != "Identity-V"
        && pdf_font::predefined::exists(&encoding))
    .then_some(collection)
}

/// Embeds a subset of the pack's program for `collection` into one composite font.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] naming the condition met — the pack absent or not this tree's, a
/// shown CID the program has no glyph for, a descendant that is no object, a descriptor that is
/// no dictionary — and [`Because::TheFence`] where the dictionary asks two widths of one CID.
pub(super) fn embed(
    document: &Document,
    used: &SelectedFont,
    descendant: &Dictionary,
    collection: Collection,
    pack: Option<&CjkPack>,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    let program = program_of(pack, collection)?;
    let descendant_at = descendant_object(document, &used.dict)
        .ok_or(Because::NotBuiltYet(NO_DESCENDANT_OBJECT))?;
    let encoding = name_of(document, &used.dict, "Encoding").unwrap_or_default();
    let cmap = pdf_font::predefined::cmap(&encoding).ok_or(Because::NotBuiltYet(NO_CMAP))?;
    let font = LoadedFont::metrics_only(document, &used.dict, &used.name)
        .ok_or(Because::NotBuiltYet(NO_CMAP))?;
    let mut widths = shown_widths(&font, &cmap, used)?;
    let home = home_of(
        document,
        descendant,
        descendant_at,
        collection,
        &mut widths,
        spare,
        substitutes,
    )?;
    let at = home.program;
    let (bytes, name, route) = written_program(&program, collection, &widths)?;

    let row = Table124 {
        key: "FontFile3",
        subtype: Some("CIDFontType0C"),
    };
    let mut stream = program_stream(subtype_dictionary(row).as_ref(), &bytes, false)
        .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
    super::notices::attach_notice(
        document,
        LICENCE,
        &mut stream,
        spare,
        &mut substitutes.written,
    )?;
    substitutes.written.insert(at, stream);
    let embedding = Embedding { key: row.key, at };
    match &home.own {
        None => {
            substitutes.at.insert(home.descriptor, embedding);
        }
        Some(template) => {
            let mut own = match template {
                Template::Copied(stated) => stated.clone(),
                Template::Built => built_descriptor(&bytes, &program, collection)?,
            };
            embedding.write_into(&mut own);
            own.insert(
                Name::new(&b"FontName"[..]),
                Object::Name(Name::new(name.as_bytes())),
            );
            substitutes
                .written
                .insert(home.descriptor, Object::Dictionary(own));
        }
    }
    rename(
        document,
        descendant,
        descendant_at,
        &home,
        &name,
        substitutes,
    );
    substitutes.packed.insert(
        home.descriptor,
        Packed {
            collection,
            widths,
            at,
            own: home.own,
            descendants: home.descendants,
        },
    );
    substitutes.done.push(SubstitutedFont {
        resource: used.name.clone(),
        requested: base_font(document, &used.dict),
        face: format!("QuorraCJK-{}", collection.ordering()),
        authority: FaceAuthority::Shipped,
        route,
        described: None,
        losses: Losses::default().with(Loss::MachineFace),
        fitting: None,
        empty_glyphs: Vec::new(),
        font: super::selected(used),
    });
    Ok(())
}

/// What the pack route wrote into one descriptor: the collection, every CID shown through it with
/// its width, the stream object holding the subset, what the descriptor was made from where this
/// conversion wrote it, and the descendants that name it.
#[derive(Debug)]
pub(super) struct Packed {
    collection: Collection,
    widths: BTreeMap<u16, f32>,
    at: ObjectId,
    own: Option<Template>,
    descendants: BTreeSet<ObjectId>,
}

/// Where one font's subset is written: the descriptor that names it, the stream object holding
/// it, and — where the descriptor is one this conversion writes — what it is made from.
struct Home {
    descriptor: ObjectId,
    program: ObjectId,
    own: Option<Template>,
    descendants: BTreeSet<ObjectId>,
}

/// What a descriptor this conversion writes for a descendant is made from.
#[derive(Debug, Clone)]
enum Template {
    /// The descriptor the descendant states, written inside it or shared with a font another
    /// subset was written for: its entries are the producer's and are carried.
    Copied(Dictionary),
    /// No descriptor at all: every entry Table 120 requires is read off the written program.
    Built,
}

/// The descriptor one descendant's subset goes into (`doc/adr/2461`).
///
/// ISO 32000-2 Table 117 requires a `CIDFont`'s `/FontDescriptor` and makes it an indirect
/// reference, and §9.9 embeds a program by a key of that descriptor, which holds one program. So
/// the subset goes into the descriptor the descendant names where that descriptor is an object no
/// other program was written into, or was written into by this route for the same collection at
/// the same widths — one subset of every CID the fonts reaching it show. **Otherwise the
/// descendant is given a descriptor object of its own**: where it states none, where it writes
/// one inside itself, and where the one it names already holds a program another font's widths
/// were held to. Two `CIDFont`s whose `/W` give one CID two advances cannot share a program,
/// since a program states one advance per glyph (§9.2.4); each is a font of its own, and its
/// descriptor is too.
///
/// # Errors
///
/// [`Because::TheFence`] where a descendant met again asks two widths of one CID, and
/// [`Because::NotBuiltYet`] where a descriptor is no dictionary or no object is spare.
fn home_of(
    document: &Document,
    descendant: &Dictionary,
    descendant_at: ObjectId,
    collection: Collection,
    widths: &mut BTreeMap<u16, f32>,
    spare: &mut Spare,
    substitutes: &Substitutes,
) -> Result<Home, Because> {
    // A descendant met again on another page: its own descriptor, widened.
    if let Some((descriptor, earlier)) = substitutes
        .packed
        .iter()
        .find(|(_, earlier)| earlier.own.is_some() && earlier.descendants.contains(&descendant_at))
    {
        widen(widths, &earlier.widths)?;
        return Ok(Home {
            descriptor: *descriptor,
            program: earlier.at,
            own: earlier.own.clone(),
            descendants: earlier.descendants.clone(),
        });
    }
    let template = match descendant.get("FontDescriptor") {
        None | Some(Object::Null) => Template::Built,
        Some(Object::Dictionary(inline)) => Template::Copied(inline.clone()),
        Some(Object::Reference(stated)) => {
            let stated = *stated;
            match substitutes.packed.get(&stated) {
                Some(earlier)
                    if earlier.collection == collection && agrees(widths, &earlier.widths) =>
                {
                    widen(widths, &earlier.widths)?;
                    let mut descendants = earlier.descendants.clone();
                    descendants.insert(descendant_at);
                    return Ok(Home {
                        descriptor: stated,
                        program: earlier.at,
                        own: None,
                        descendants,
                    });
                }
                None if !substitutes.at.contains_key(&stated)
                    && !substitutes.described.contains_key(&stated) =>
                {
                    let program = spare
                        .take(document)
                        .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
                    return Ok(Home {
                        descriptor: stated,
                        program,
                        own: None,
                        descendants: BTreeSet::from([descendant_at]),
                    });
                }
                _ => {}
            }
            let resolved = document.resolve(&Object::Reference(stated));
            Template::Copied(
                resolved
                    .as_dict()
                    .ok_or(Because::NotBuiltYet(NOT_A_DESCRIPTOR))?
                    .clone(),
            )
        }
        Some(_) => return Err(Because::NotBuiltYet(NOT_A_DESCRIPTOR)),
    };
    let descriptor = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
    let program = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
    Ok(Home {
        descriptor,
        program,
        own: Some(template),
        descendants: BTreeSet::from([descendant_at]),
    })
}

/// Whether every CID both width lists name has one advance in both.
fn agrees(widths: &BTreeMap<u16, f32>, earlier: &BTreeMap<u16, f32>) -> bool {
    widths.iter().all(|(cid, width)| {
        earlier
            .get(cid)
            .is_none_or(|held| !disagrees(*held, *width))
    })
}

/// `widths` widened by every CID an earlier meeting showed.
///
/// # Errors
///
/// [`Because::TheFence`] where the two give one CID two advances.
fn widen(widths: &mut BTreeMap<u16, f32>, earlier: &BTreeMap<u16, f32>) -> Result<(), Because> {
    for (cid, width) in earlier {
        match widths.entry(*cid) {
            Entry::Vacant(slot) => {
                slot.insert(*width);
            }
            Entry::Occupied(held) if disagrees(*held.get(), *width) => {
                return Err(Because::TheFence(TWO_WIDTHS_FOR_ONE_GLYPH));
            }
            Entry::Occupied(_) => {}
        }
    }
    Ok(())
}

/// A descriptor for a descendant that states none, every entry Table 120 requires read off the
/// subset written (`subset`) or the pack's own program (`program`).
///
/// ISO 32000-2 Table 120 makes `/Type`, `/FontName`, `/Flags`, `/FontBBox`, `/ItalicAngle`,
/// `/Ascent`, `/Descent` and `/StemV` required, and `/CapHeight` "Required for fonts that have
/// Latin characters"; `/FontName` and the program's key are written by the caller. Each value is
/// the definition's own measurement:
///
/// - `/Flags` is Table 121's `Symbolic` alone: a collection's program "contains glyphs outside the
///   Standard Latin character set", and the pack's faces are upright, sans-serif and
///   proportional.
/// - `/FontBBox` encloses every glyph of the subset — "the shape that would result if all of the
///   glyphs of the font were placed with their origins coincident and then filled" — `.notdef`
///   included, since the font holds it.
/// - `/Ascent` and `/Descent` are the highest and lowest points of the subset's glyphs but
///   `.notdef`, which no shown code reaches.
/// - `/CapHeight` is the top of the collection's `H`, which every collection the pack holds has:
///   stated whether or not the page shows Latin text, since Table 120 asks it of a font that has
///   Latin characters and the program does.
/// - `/ItalicAngle` is the program's `post` table's, and `/StemV` is 0, Table 120's "unknown
///   stem thickness": a CID-keyed program states its stems per Font DICT, not once.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] where a program does not draw, or the subset's glyphs reach no point
/// below the baseline, which Table 120's `/Descent` "shall be a negative number" cannot state.
fn built_descriptor(
    subset: &[u8],
    program: &[u8],
    collection: Collection,
) -> Result<Dictionary, Because> {
    let unmeasured = || Because::NotBuiltYet(DESCRIPTOR_NOT_MEASURED);
    let glyphs = pdf_font::cff::glyph_count(subset).map_err(|_| unmeasured())?;
    let mut all = Bounds::default();
    let mut shown = Bounds::default();
    for glyph in 0..glyphs {
        let glyph = u16::try_from(glyph).map_err(|_| unmeasured())?;
        let mut pen = Bounds::default();
        pdf_font::cff::draw(subset, glyph, &mut pen).map_err(|_| unmeasured())?;
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
    let face = skrifa::FontRef::new(program).map_err(|_| unmeasured())?;
    let italic_angle = face
        .metrics(Size::unscaled(), LocationRef::default())
        .italic_angle;
    let cap_height = capital_height(&face, collection).ok_or_else(unmeasured)?;
    let number = |value: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a glyph-space measurement in thousandths of an em, rounded to the whole \
                      unit a descriptor states it in and clamped far inside i64"
        )]
        Object::Integer(value.round().clamp(-1.0e6, 1.0e6) as i64)
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
        Object::Real(f64::from(italic_angle)),
    );
    dict.insert(Name::new(&b"Ascent"[..]), number(ascent));
    dict.insert(Name::new(&b"Descent"[..]), number(descent));
    dict.insert(Name::new(&b"CapHeight"[..]), number(cap_height));
    dict.insert(Name::new(&b"StemV"[..]), Object::Integer(0));
    Ok(dict)
}

/// The top of the collection's first CID for `H`, through the collection's own `Adobe-*-UCS2`
/// table (§9.10.2's third method), in the pack's program, whose glyph N is CID N.
fn capital_height(face: &skrifa::FontRef<'_>, collection: Collection) -> Option<f32> {
    let table = pdf_font::predefined::cid_to_unicode("Adobe", collection.ordering())?;
    let glyphs = face.outline_glyphs();
    let cid = (1..u32::from(u16::MAX)).find(|cid| table.char_for(*cid) == Some('H'))?;
    let mut pen = Bounds::default();
    glyphs
        .get(skrifa::GlyphId::new(cid))?
        .draw(
            DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
            &mut pen,
        )
        .ok()?;
    pen.rectangle().map(|[_, _, _, top]| top)
}

/// The extent a pen was drawn over, in the program's units.
#[derive(Debug, Default)]
struct Bounds(Option<[f32; 4]>);

impl Bounds {
    fn point(&mut self, x: f32, y: f32) {
        self.0 = Some(match self.0 {
            None => [x, y, x, y],
            Some([x0, y0, x1, y1]) => [x0.min(x), y0.min(y), x1.max(x), y1.max(y)],
        });
    }

    fn include(&mut self, other: &Self) {
        if let Some([x0, y0, x1, y1]) = other.0 {
            self.point(x0, y0);
            self.point(x1, y1);
        }
    }

    /// `[x_min, y_min, x_max, y_max]`, where anything was drawn.
    fn rectangle(&self) -> Option<[f32; 4]> {
        self.0
    }
}

/// The on-curve and control points bound a glyph's outline, which is all a bounding box and
/// Table 120's heights need: a curve stays inside its control points' hull.
impl OutlinePen for Bounds {
    fn move_to(&mut self, x: f32, y: f32) {
        self.point(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.point(x, y);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.point(cx, cy);
        self.point(x, y);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.point(cx0, cy0);
        self.point(cx1, cy1);
        self.point(x, y);
    }
    fn close(&mut self) {}
}

/// The subset of the pack's `program` keeping each of `widths`' CIDs, its advances the
/// dictionary's, with its §9.9.2 name and the metric route it took.
fn written_program(
    program: &[u8],
    collection: Collection,
    widths: &BTreeMap<u16, f32>,
) -> Result<(Vec<u8>, String, MetricRoute), Because> {
    let shown: BTreeSet<u16> = widths.keys().copied().collect();
    let glyphs = pdf_font::embedding::glyph_count(program, Format::Sfnt)
        .ok_or(Because::NotBuiltYet(PACK_NOT_THIS_PACK))?;
    if shown.iter().any(|cid| *cid >= glyphs) {
        return Err(Because::NotBuiltYet(PACK_LACKS_A_CID));
    }
    let subset = pdf_font::embed::for_embedding(program, &shown)
        .map_err(|_| Because::NotBuiltYet(PACK_NOT_SUBSET))?;
    match &subset.outlines {
        pdf_font::embed::Outlines::CompactCid { system }
            if system.registry == b"Adobe"
                && system.ordering == collection.ordering().as_bytes() => {}
        _ => return Err(Because::NotBuiltYet(PACK_NOT_SUBSET)),
    }

    // **A glyph with no outline is a mark lost unless the collection makes the CID a space.** The
    // builder leaves a CID it had no published mapping for as an empty glyph, and §9.10.2's third
    // method — the collection's own `Adobe-*-UCS2` table — says which empty glyphs are spaces.
    let spaces = pdf_font::predefined::cid_to_unicode("Adobe", collection.ordering());
    let mut by_glyph: BTreeMap<u16, f32> = BTreeMap::new();
    let mut route = MetricRoute::FaceMetrics;
    for (cid, width) in widths {
        let glyph = *subset
            .glyphs
            .get(cid)
            .ok_or(Because::NotBuiltYet(PACK_LACKS_A_CID))?;
        if !draws(&subset.program, glyph)
            && !spaces
                .as_ref()
                .and_then(|table| table.char_for(u32::from(*cid)))
                .is_some_and(char::is_whitespace)
        {
            return Err(Because::NotBuiltYet(PACK_LACKS_A_CID));
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
    // §9.2.4 makes the dictionary's numbers what positions every glyph, so the program written is
    // held to them before anything is attached.
    super::proves(&bytes, Format::BareCff, &by_glyph, &BTreeMap::new())?;
    Ok((bytes, subset.name, route))
}

/// The collection's program, or the refusal that names why the pack did not supply it.
fn program_of(pack: Option<&CjkPack>, collection: Collection) -> Result<Arc<[u8]>, Because> {
    let pack = pack.ok_or(Because::NotBuiltYet(PACK_ABSENT))?;
    pack.program(collection).map_err(|gap| match gap {
        PackGap::Absent => Because::NotBuiltYet(PACK_ABSENT),
        PackGap::NotThisPack => Because::NotBuiltYet(PACK_NOT_THIS_PACK),
    })
}

/// The names Table 117 and §9.9.2 ask of a `CIDFontType0` holding a subset: the descendant's
/// `/BaseFont` and the descriptor's `/FontName` are the program's own, and the descendant's
/// `/Subtype` is the one Table 124 puts a CFF program under; a descendant given a descriptor of
/// its own is pointed at it.
fn rename(
    document: &Document,
    descendant: &Dictionary,
    descendant_at: ObjectId,
    home: &Home,
    subset: &str,
    substitutes: &mut Substitutes,
) {
    let name = Object::Name(Name::new(subset.as_bytes()));
    let mut gains = Dictionary::new();
    gains.insert(Name::new(&b"BaseFont"[..]), name.clone());
    if name_of(document, descendant, "Subtype").as_deref() != Some("CIDFontType0") {
        gains.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(&b"CIDFontType0"[..])),
        );
    }
    if home.own.is_some() {
        // The descriptor this conversion wrote carries its `/FontName` itself; the descendant
        // is pointed at it, which Table 117's indirect reference asks.
        gains.insert(
            Name::new(&b"FontDescriptor"[..]),
            Object::Reference(home.descriptor),
        );
        substitutes.described.insert(descendant_at, gains);
        return;
    }
    substitutes.described.insert(descendant_at, gains);
    let mut named = Dictionary::new();
    named.insert(Name::new(&b"FontName"[..]), name);
    substitutes.described.insert(home.descriptor, named);
}

/// The CIDs the page shows, each with the width the dictionary gives it.
///
/// §9.7.4.3's `/W` and `/DW` are indexed by CID, and §9.2.4 makes them what positions the glyph,
/// so two codes reaching one CID at two widths ask a program to state two advances for one glyph.
fn shown_widths(
    font: &LoadedFont,
    cmap: &pdf_font::cmap::CMap,
    used: &SelectedFont,
) -> Result<BTreeMap<u16, f32>, Because> {
    let mut widths: BTreeMap<u16, f32> = BTreeMap::new();
    for text in used.shown.keys() {
        for code in font.decode(text) {
            let cid = cmap
                .cid(code)
                .and_then(|cid| u16::try_from(cid).ok())
                .filter(|cid| *cid != pdf_font::NOTDEF_GLYPH)
                .ok_or(Because::NotBuiltYet(PACK_LACKS_A_CID))?;
            let stated = font.advance(code);
            match widths.entry(cid) {
                Entry::Vacant(slot) => {
                    slot.insert(stated);
                }
                Entry::Occupied(held) if disagrees(*held.get(), stated) => {
                    return Err(Because::TheFence(TWO_WIDTHS_FOR_ONE_GLYPH));
                }
                Entry::Occupied(_) => {}
            }
        }
    }
    Ok(widths)
}

/// Whether a bare CFF program's glyph draws anything.
fn draws(program: &[u8], glyph: u16) -> bool {
    /// Whether any segment was drawn.
    struct Drew(bool);
    impl OutlinePen for Drew {
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
    let mut drew = Drew(false);
    pdf_font::cff::draw(program, glyph, &mut drew).is_ok() && drew.0
}

/// The object the `/DescendantFonts` array names, where it names one rather than holding it.
fn descendant_object(document: &Document, dict: &Dictionary) -> Option<ObjectId> {
    let array = document.get_key(dict, "DescendantFonts");
    document.resolve(&array).as_array()?.first()?.as_reference()
}

/// Why a collection's font is refused where the caller handed no pack, or the pack has no file.
const PACK_ABSENT: &str = "this document renders a composite font it does not embed whose \
     descendant names one of the Adobe character collections ISO 32000-2 §9.7.5.2 requires, and \
     the CJK pack that holds a program of that collection is not installed. ISO 32000-2 §9.7.2 \
     makes a collection's order what numbers its glyphs, so the pack's program for the \
     collection carries the glyph each CID names; it is a separate download, placed in a \
     directory named cjk-pack beside this program or named with --cjk-pack <dir> \
     (doc/cjk-pack.md), and this program never fetches it on its own. --font \
     <base-font>=<path> naming the producer's own program resolves it as well";

/// Why a pack file whose bytes are not this tree's is refused.
const PACK_NOT_THIS_PACK: &str = "this document renders a composite font of one of the Adobe \
     character collections, and the CJK pack file for that collection is not the program this \
     version of the converter was built to embed: its SHA-256 differs from the one this program \
     carries. A program whose provenance cannot be checked is not written into an archive; \
     installing the pack built for this version (doc/cjk-pack.md) resolves it";

/// Why a font whose shown CID the pack has no glyph for is refused.
const PACK_LACKS_A_CID: &str = "this document shows a CID of its composite font that the CJK \
     pack's program for the collection has no glyph for — CID 0, which ISO 32000-2 §9.7.6.3 \
     makes the analogue of .notdef, a CID past the collection's last, or one no mapping Adobe \
     publishes for the collection reaches in the family the pack is built from. Embedding the \
     program would show an empty glyph where the page shows a mark, so it is not embedded";

/// Why a pack program the subsetter could not keep CID-keyed is refused.
const PACK_NOT_SUBSET: &str = "the CJK pack's program for this collection could not be written \
     as a CID-keyed subset whose charset keeps each shown glyph at its CID, which is the only \
     form in which the content stream's codes still reach the glyphs ISO 32000-2 §9.7.4.2 says \
     they name";

/// Why a font whose `CMap` this route cannot read is refused.
const NO_CMAP: &str = "this document renders a composite font of an Adobe character collection \
     whose Encoding names a predefined CMap this reader could not read, so which CIDs the page \
     shows is not a question it can answer";

/// Why a descendant whose `/FontDescriptor` is neither absent nor a dictionary is refused.
const NOT_A_DESCRIPTOR: &str = "this composite font's descendant CIDFont states a \
     FontDescriptor that is not a dictionary, and a program is embedded by a key of the font \
     descriptor ISO 32000-2's Table 120 describes. There is no descriptor here to embed into, and \
     none to carry into one written for it";

/// Why a descriptor written for a descendant that states none is refused.
const DESCRIPTOR_NOT_MEASURED: &str = "this composite font's descendant CIDFont states no \
     FontDescriptor, which ISO 32000-2's Table 117 requires, and the descriptor written for it \
     could not be measured off the CJK pack's program: its glyphs did not draw, or reach no point \
     below the baseline, and Table 120 requires a Descent that is a negative number";

/// Why a descendant written inline is refused.
const NO_DESCENDANT_OBJECT: &str = "this composite font's DescendantFonts array holds the \
     CIDFont dictionary itself rather than a reference to one, and embedding the CJK pack's \
     program rewrites that dictionary's Subtype and BaseFont. There is no object here to write \
     those entries into";

#[cfg(test)]
mod tests {
    use super::{CjkPack, Collection, PackFiles, PackGap};
    use std::sync::Arc;

    /// A reader holding one file under one name.
    #[derive(Debug)]
    struct One(&'static str, Vec<u8>);

    impl PackFiles for One {
        fn read(&self, file: &str) -> Option<Vec<u8>> {
            (file == self.0).then(|| self.1.clone())
        }
    }

    #[test]
    fn every_ordering_the_pack_holds_names_its_own_collection_and_no_other() {
        for collection in Collection::ALL {
            assert_eq!(
                Collection::from_ordering(collection.ordering()),
                Some(collection)
            );
        }
        assert_eq!(Collection::from_ordering("Identity"), None);
        assert_eq!(Collection::from_ordering("Japan2"), None);
    }

    #[test]
    fn a_missing_file_is_absent_and_other_bytes_are_not_this_pack() {
        let empty = CjkPack::new(Arc::new(One("nothing", Vec::new())));
        assert_eq!(empty.program(Collection::Japan1), Err(PackGap::Absent));
        let wrong = CjkPack::new(Arc::new(One("Adobe-GB1.otf", b"not a font".to_vec())));
        assert_eq!(wrong.program(Collection::Gb1), Err(PackGap::NotThisPack));
    }

    #[test]
    fn the_notice_is_the_open_font_licence_with_the_reserved_name_the_rename_avoids() {
        assert!(super::LICENCE.contains("SIL OPEN FONT LICENSE Version 1.1"));
        assert!(super::LICENCE.contains("Reserved Font\nName 'Source'"));
    }

    /// A reader of a pack directory on disk.
    #[derive(Debug)]
    struct Directory(std::path::PathBuf);

    impl PackFiles for Directory {
        fn read(&self, file: &str) -> Option<Vec<u8>> {
            std::fs::read(self.0.join(file)).ok()
        }
    }

    /// A built pack against what this file carries (A596): the `SHA256SUMS` `tools/cjk-pack.py`
    /// wrote lists each program under the digest compiled in here and the licence under the
    /// notice's, in [`Collection::ALL`]'s order, and every file holds the bytes its line names.
    /// CI's `cjk-pack` job builds the pack from the Source Han files fetched by digest and runs
    /// this before it publishes the zip, so a builder or a source that writes other bytes stops
    /// the publication rather than shipping a pack this program refuses (ADR 2386).
    #[test]
    #[ignore = "needs a built CJK pack, which is not part of a checkout: QUORRA_CJK_PACK=<dir> \
                names it (doc/cjk-pack.md), and CI's cjk-pack job runs it"]
    fn a_built_pack_holds_the_programs_whose_digests_this_file_carries() {
        use super::{LICENCE, hex};
        use sha2::{Digest as _, Sha256};

        let directory = std::path::PathBuf::from(
            std::env::var_os("QUORRA_CJK_PACK")
                .expect("QUORRA_CJK_PACK names the directory the pack was built into"),
        );
        let read = |file: &str| {
            std::fs::read(directory.join(file))
                .unwrap_or_else(|error| panic!("{}: {error}", directory.join(file).display()))
        };
        let sums = String::from_utf8(read("SHA256SUMS")).expect("SHA256SUMS is ASCII");
        let listed: Vec<(String, String)> = sums
            .lines()
            .map(|line| {
                let (digest, file) = line
                    .split_once("  ")
                    .unwrap_or_else(|| panic!("not sha256sum's format: {line:?}"));
                (file.to_owned(), digest.to_owned())
            })
            .collect();
        let mut carried: Vec<(String, String)> = Collection::ALL
            .into_iter()
            .map(|collection| (collection.file().to_owned(), collection.digest().to_owned()))
            .collect();
        carried.push((
            "LICENSE.txt".to_owned(),
            hex(&Sha256::digest(LICENCE.as_bytes())),
        ));
        assert_eq!(
            listed, carried,
            "the built pack is not the one pack.rs names"
        );
        for (file, digest) in &listed {
            assert_eq!(&hex(&Sha256::digest(read(file))), digest, "{file}");
        }
        let pack = CjkPack::new(Arc::new(Directory(directory)));
        for collection in Collection::ALL {
            assert!(pack.program(collection).is_ok(), "{}", collection.file());
        }
    }
}
