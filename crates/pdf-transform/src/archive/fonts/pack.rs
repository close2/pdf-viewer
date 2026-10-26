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
//! own program (§9.7.4.2), and what may answer it is the owner's open `doc/questions/Q570`.
//!
//! # What reaches the output
//!
//! A §9.9.2 subset of the pack program, written bare under Table 124's `CIDFontType0C` row: the
//! subsetter keeps each shown glyph at its CID through the program's charset, so the content
//! stream is untouched. The descendant becomes a `CIDFontType0` where it said `CIDFontType2`,
//! because Table 124 puts a CFF program under that subtype and no program was there for the old
//! subtype to describe; its `/BaseFont` and the descriptor's `/FontName` become the subset's name,
//! which Table 117 requires of a `CIDFontType0` and §9.9.2 of a subset. The Type 0 font's own
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
    /// bytes it was handed are the pack's (`doc/questions/Q596` asks how this is versioned).
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
/// shown CID the program has no glyph for, a descendant or descriptor that is no object — and
/// [`Because::TheFence`] where the dictionary asks two widths of one CID.
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
    let descriptor = descendant
        .get("FontDescriptor")
        .and_then(Object::as_reference)
        .ok_or(Because::NotBuiltYet(NO_DESCRIPTOR_OBJECT))?;
    let descendant_at = descendant_object(document, &used.dict)
        .ok_or(Because::NotBuiltYet(NO_DESCENDANT_OBJECT))?;
    let encoding = name_of(document, &used.dict, "Encoding").unwrap_or_default();
    let cmap = pdf_font::predefined::cmap(&encoding).ok_or(Because::NotBuiltYet(NO_CMAP))?;
    let font = LoadedFont::metrics_only(document, &used.dict, &used.name)
        .ok_or(Because::NotBuiltYet(NO_CMAP))?;
    let mut widths = shown_widths(&font, &cmap, used)?;

    // **One subset per descriptor, of every CID any font reaching it shows.** The survey meets a
    // font once per page that selects it, and two Type 0 dictionaries may share one descendant;
    // each meeting widens the one program rather than writing a second over the first.
    let at = match substitutes.packed.get(&descriptor) {
        Some(earlier) if earlier.collection == collection => {
            for (cid, width) in &earlier.widths {
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
            earlier.at
        }
        Some(_) => return Err(Because::NotBuiltYet(SHARED_DESCRIPTOR)),
        None if substitutes.at.contains_key(&descriptor)
            || substitutes.described.contains_key(&descriptor) =>
        {
            return Err(Because::NotBuiltYet(SHARED_DESCRIPTOR));
        }
        None => spare
            .take(document)
            .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?,
    };
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
    substitutes
        .at
        .insert(descriptor, Embedding { key: row.key, at });
    rename(
        document,
        descendant,
        descendant_at,
        descriptor,
        &name,
        substitutes,
    );
    substitutes.packed.insert(
        descriptor,
        Packed {
            collection,
            widths,
            at,
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
    });
    Ok(())
}

/// What the pack route wrote into one descriptor: the collection, every CID shown through it with
/// its width, and the stream object holding the subset.
#[derive(Debug)]
pub(super) struct Packed {
    collection: Collection,
    widths: BTreeMap<u16, f32>,
    at: ObjectId,
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
/// `/Subtype` is the one Table 124 puts a CFF program under.
fn rename(
    document: &Document,
    descendant: &Dictionary,
    descendant_at: ObjectId,
    descriptor: ObjectId,
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
    substitutes.described.insert(descendant_at, gains);
    let mut named = Dictionary::new();
    named.insert(Name::new(&b"FontName"[..]), name);
    substitutes.described.insert(descriptor, named);
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

/// Why a descendant whose descriptor is written inline is refused.
const NO_DESCRIPTOR_OBJECT: &str = "this composite font's descendant CIDFont states its \
     FontDescriptor as a dictionary written inside itself rather than as the indirect reference \
     ISO 32000-2's Table 115 requires, and a program is embedded by writing a FontFile3 key into a \
     descriptor object. There is no object here to write into";

/// Why a descendant written inline is refused.
const NO_DESCENDANT_OBJECT: &str = "this composite font's DescendantFonts array holds the \
     CIDFont dictionary itself rather than a reference to one, and embedding the CJK pack's \
     program rewrites that dictionary's Subtype and BaseFont. There is no object here to write \
     those entries into";

/// Why a descriptor another route or another collection already wrote into is refused.
const SHARED_DESCRIPTOR: &str = "this composite font's FontDescriptor is shared with a font that \
     was given a program of another collection, or by another route, and one descriptor holds \
     one program; separate descendant descriptors are not built yet, so the font is not written";

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
}
