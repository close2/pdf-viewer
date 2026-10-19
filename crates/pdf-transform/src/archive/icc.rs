//! An `ICCBased` space whose profile lacks a tag its ICC edition requires, answered by what this
//! reader drew through it (`doc/adr/1979`, `doc/adr/1990`, `doc/adr/2006`).
//!
//! # Two parts, two readings, one set of answers
//!
//! Both parts ask in section 6.2.4.2 for a profile that carries what an ICC edition requires, and
//! they choose the edition differently. ISO 19005-2 names four and a profile need satisfy one —
//! `graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires`, read for one profile by
//! `pdf_archive::icc_tag_shortfall`. ISO 19005-4 defers to ISO 32000-2 §8.6.5.5, which holds a
//! profile to the version its own header names — `graphics/icc-profiles-carry-the-tags-their-
//! version-requires`, read by `pdf_archive::icc_version_tag_shortfall`. [`Reading`] is the choice
//! between the two; everything after it is the same, because what a converter may do about a short
//! profile depends on what this reader drew rather than on which text found the tag missing.
//!
//! # Three kinds of profile, and three answers
//!
//! What the converter may do about a short profile is decided by whether this reader draws through
//! it, which `pdf_model::icc::Profile::parse` answers:
//!
//! - **Drawn through, short only of text tags.** `desc` and `cprt` are text no transform reads, so
//!   writing them in moves no colour — but what they say is not in the file. A `supply`: the
//!   operator's `description` and `copyright` keys, written as ICC.1:1998-09's `textDescriptionType`
//!   and `textType` into a version 2 profile and as ICC.1:2001-12's `multiLocalizedUnicodeType`
//!   into a later one, every byte of the colour data kept.
//! - **Not drawn through.** ISO 32000-2 §8.6.5.5's Table 65 gives the `Alternate` entry as the
//!   space "used in case the one specified in the stream data is not supported", and where it is
//!   omitted the device space of `N`. That is what the page already showed, so writing the
//!   alternate where the array was is a statement.
//! - **Drawn through, short of a tag its transform needs.** Refused by name: supplying a white
//!   point or a curve would be inventing the producer's colour.
//!
//! A file holding profiles of the first and second kinds together is refused as well, because the
//! two answers are two decisions — the operator's and the converter's — and this version carries
//! out one per requirement.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::DeviceFamily;
use pdf_archive::{IccTagShortfall, Part, Target};
use pdf_model::icc::{Profile, computed_id, stated_id};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;

use super::decision::Because;
use super::sites::MAX_ENTRY_DEPTH;

/// The part 2 requirement this file answers.
pub(super) const SITE: &str = "graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires";

/// The part 4 requirement this file answers, by the same three answers (`doc/adr/2006`).
pub(super) const SITE_FOUR: &str = "graphics/icc-profiles-carry-the-tags-their-version-requires";

/// Which text a profile's tags are held to: the part's own choice of ICC edition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reading {
    /// ISO 19005-2 section 6.2.4.2: any of the editions the part names will do.
    PermittedEdition,
    /// ISO 19005-4 section 6.2.4.2, through ISO 32000-2 §8.6.5.5: the edition the header names.
    OwnVersion,
}

impl Reading {
    /// The reading a target's part states.
    pub(super) const fn of(target: Target) -> Self {
        match target.part() {
            Part::Two => Self::PermittedEdition,
            Part::Four => Self::OwnVersion,
        }
    }

    /// The reading a requirement is, where it is one of the two this file answers.
    pub(super) fn of_site(site: &str) -> Option<Self> {
        match site {
            SITE => Some(Self::PermittedEdition),
            SITE_FOUR => Some(Self::OwnVersion),
            _ => None,
        }
    }

    /// The requirement the validator reports this reading's failures under.
    pub(super) const fn site(self) -> &'static str {
        match self {
            Self::PermittedEdition => SITE,
            Self::OwnVersion => SITE_FOUR,
        }
    }

    /// The tags this reading finds missing from one profile, by the row's own function.
    fn shortfall(self, data: &[u8]) -> Option<IccTagShortfall> {
        match self {
            Self::PermittedEdition => pdf_archive::icc_tag_shortfall(data),
            Self::OwnVersion => pdf_archive::icc_version_tag_shortfall(data),
        }
    }
}

/// Why a profile short of a transform tag is not answered.
const TRANSFORM_TAG_MISSING: &str = "an ICCBased colour space's profile is short of a tag its \
     colour transform needs - a white point, a curve, a matrix or a lookup table - and this \
     reader draws the space through the profile all the same. Supplying the tag would be \
     inventing the producer's colour, and falling back on the Alternate entry would move every \
     colour drawn through it, so the requirement is refused (doc/adr/1979)";

/// Why a profile short only of text tags is not answered without the operator.
const TEXT_NOT_SUPPLIED: &str = "an ICCBased colour space's profile lacks only a text tag - its \
     description or its copyright notice - which no colour transform reads, so writing one in \
     moves nothing on the page. What the text says is not in the file, so it is the operator's \
     to state: answer the site with remedy = \"supply\" and a `copyright` or `description` key \
     (doc/adr/1990)";

/// Why the two answers are not taken together.
const TWO_KINDS: &str = "this file holds an ICCBased profile this reader does not draw through \
     and another it draws through that lacks only a text tag. The first is answered by its \
     Alternate space and the second by text the operator supplies, and this version carries out \
     one answer per requirement, so neither is taken (doc/adr/1990)";

/// Why an unreadable profile in a default colour space is not replaced.
const IN_A_DEFAULT_SPACE: &str = "an ICCBased profile this reader does not draw through is the \
     value of a DefaultGray, DefaultRGB or DefaultCMYK entry, and writing its Alternate there \
     would make a device space substitute for itself, which ISO 32000-2 8.6.5.6 does not \
     describe. The requirement is refused (doc/adr/1990)";

/// Why an unreadable profile stating no space to fall back on is not answered.
const NO_ALTERNATE: &str = "an ICCBased profile this reader does not draw through states no \
     Alternate entry and no N of 1, 3 or 4, so ISO 32000-2 8.6.5.5's Table 65 names no space for \
     it, and the space this reader guessed is not one the file states. The requirement is \
     refused (doc/adr/1990)";

/// Why an alternate device space this file's output intent cannot license is not written.
const NO_INTENT_FOR_THE_ALTERNATE: &str = "an ICCBased profile this reader does not draw through \
     falls back on a device space, and writing that space puts device colour in the output where \
     the input had a managed space. ISO 19005-2 section 6.2.4.3 licenses DeviceRGB and DeviceCMYK \
     only through a PDF/A output intent of their own family, and the intent this conversion \
     holds or would write is of another, or none could be prepared (doc/adr/1990); ISO 19005-4 \
     section 6.2.4.3 states the same licence";

/// Why a failure this walk did not find is not answered.
const NOT_REACHED: &str = "the requirement failed at an ICCBased profile this conversion's walk \
     of the file's objects did not reach - one past its depth bound, or in an object of a \
     generation other than 0 - so a rewrite of the ones it found would leave the failure in \
     place (doc/adr/1990)";

/// Every short profile in the file, by the stream that holds it.
#[derive(Debug, Default)]
pub(super) struct ShortProfiles {
    /// Profiles this reader draws through that lack only text tags.
    pub(super) text_only: BTreeMap<ObjectId, IccTagShortfall>,
    /// Profiles this reader does not draw through, with the space Table 65 draws instead.
    pub(super) unreadable: BTreeMap<ObjectId, Object>,
    /// Whether a profile this reader draws through lacks a tag its transform needs.
    transform: bool,
    /// Whether an unreadable profile is the value of a `Default` colour space entry.
    in_a_default_space: bool,
    /// Whether an unreadable profile states neither an `Alternate` nor an `N` of 1, 3 or 4.
    no_alternate: bool,
}

impl ShortProfiles {
    /// Every `ICCBased` array in the file, its profile read once and judged by `reading`.
    pub(super) fn of(document: &Document, reading: Reading) -> Self {
        let mut out = Self::default();
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId::new(number, 0));
            out.visit(document, reading, &object, None, 0);
        }
        out
    }

    /// Whether this walk found nothing the requirement could have failed at.
    fn is_empty(&self) -> bool {
        self.text_only.is_empty()
            && self.unreadable.is_empty()
            && !self.transform
            && !self.no_alternate
    }

    /// Why the operator's text cannot answer the requirement, where it cannot.
    pub(super) fn supply_obstacle(&self) -> Option<Because> {
        if self.is_empty() {
            return Some(Because::NotBuiltYet(NOT_REACHED));
        }
        if self.transform {
            return Some(Because::NotBuiltYet(TRANSFORM_TAG_MISSING));
        }
        if !self.unreadable.is_empty() || self.no_alternate {
            return Some(Because::NotBuiltYet(TWO_KINDS));
        }
        None
    }

    /// Why the alternate route cannot answer the requirement, where it cannot.
    pub(super) fn alternate_obstacle(&self) -> Option<Because> {
        if self.is_empty() {
            return Some(Because::NotBuiltYet(NOT_REACHED));
        }
        if self.transform {
            return Some(Because::NotBuiltYet(TRANSFORM_TAG_MISSING));
        }
        if !self.text_only.is_empty() {
            return Some(Because::NotBuiltYet(if self.unreadable.is_empty() {
                TEXT_NOT_SUPPLIED
            } else {
                TWO_KINDS
            }));
        }
        if self.in_a_default_space {
            return Some(Because::NotBuiltYet(IN_A_DEFAULT_SPACE));
        }
        if self.no_alternate {
            return Some(Because::NotBuiltYet(NO_ALTERNATE));
        }
        None
    }

    /// One value, `key` being the dictionary entry it is the value of.
    fn visit(
        &mut self,
        document: &Document,
        reading: Reading,
        object: &Object,
        key: Option<&[u8]>,
        depth: usize,
    ) {
        if depth >= MAX_ENTRY_DEPTH {
            return;
        }
        let deeper = depth.saturating_add(1);
        match object {
            Object::Array(items) => {
                if let Some(at) = icc_profile_of(document, items) {
                    let in_default = key.is_some_and(is_default_space_key);
                    self.judge(document, reading, at, in_default);
                }
                for item in items {
                    self.visit(document, reading, item, None, deeper);
                }
            }
            Object::Dictionary(dict) => {
                for (name, value) in dict.iter() {
                    self.visit(document, reading, value, Some(name.as_bytes()), deeper);
                }
            }
            Object::Stream(stream) => {
                for (name, value) in stream.dict.iter() {
                    self.visit(document, reading, value, Some(name.as_bytes()), deeper);
                }
            }
            _ => {}
        }
    }

    /// One profile, judged by the requirement's own reading and by whether this reader draws it.
    ///
    /// *Drawn through* is `pdf_model::colour`'s own condition for an `ICCBased` space: the stream
    /// decodes whole and the profile parses. A profile decoded only as far as its damage is drawn
    /// in its alternate there, so it is an unreadable one here too.
    fn judge(&mut self, document: &Document, reading: Reading, at: ObjectId, in_default: bool) {
        let Object::Stream(stream) = document.get(at) else {
            return;
        };
        let Ok(decoded) = document.decoded_stream_data_reported(&stream) else {
            return;
        };
        let Some(shortfall) = reading.shortfall(&decoded.data) else {
            return;
        };
        if decoded.damage.is_some() || Profile::parse(&decoded.data).is_none() {
            self.in_a_default_space |= in_default;
            match alternate_of(document, &stream) {
                Some(alternate) => {
                    self.unreadable.insert(at, alternate);
                }
                None => self.no_alternate = true,
            }
        } else if shortfall.only_text() {
            self.text_only.insert(at, shortfall);
        } else {
            self.transform = true;
        }
    }
}

/// The profile stream an `[/ICCBased stream]` array names, where the array is one.
fn icc_profile_of(document: &Document, items: &[Object]) -> Option<ObjectId> {
    let family = document.resolve(items.first()?);
    if family.as_name()?.as_bytes() != b"ICCBased" {
        return None;
    }
    items.get(1)?.as_reference()
}

/// Whether a dictionary key is one of §8.6.5.6's three default colour space entries.
fn is_default_space_key(key: &[u8]) -> bool {
    matches!(key, b"DefaultGray" | b"DefaultRGB" | b"DefaultCMYK")
}

/// ISO 32000-2 §8.6.5.5's Table 65: the `Alternate` entry as written, or the device space of `N`.
///
/// `None` where neither is stated in a form the table admits, which leaves the profile with no
/// space this conversion could state for it.
fn alternate_of(document: &Document, stream: &Stream) -> Option<Object> {
    if let Some(alternate) = stream.dict.get("Alternate") {
        return Some(alternate.clone());
    }
    let device: &[u8] = match document.get_key(&stream.dict, "N").as_integer()? {
        1 => b"DeviceGray",
        3 => b"DeviceRGB",
        4 => b"DeviceCMYK",
        _ => return None,
    };
    Some(Object::Name(Name::new(device)))
}

/// What the alternate route writes: the space each unreadable profile's arrays become, and the
/// device families among them.
#[derive(Debug)]
pub(super) struct Alternates {
    /// The space each unreadable profile's `ICCBased` arrays are written as, by profile.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// The device families those spaces name.
    pub(super) families: BTreeSet<DeviceFamily>,
}

impl Alternates {
    /// The alternate route over this document under `reading`, or why it cannot be taken.
    pub(super) fn of(document: &Document, reading: Reading) -> Result<Self, Because> {
        let short = ShortProfiles::of(document, reading);
        if let Some(because) = short.alternate_obstacle() {
            return Err(because);
        }
        let families = device_families(document, &short.unreadable);
        Ok(Self {
            at: short.unreadable,
            families,
        })
    }

    /// Why the output intent in hand does not license these families, where it does not: grey
    /// takes an intent of any family, and RGB and CMYK one of their own (ISO 19005-2 section
    /// 6.2.4.3).
    pub(super) fn unlicensed_by(&self, intent: Option<DeviceFamily>) -> Option<Because> {
        let needs_one = self
            .families
            .iter()
            .any(|family| *family != DeviceFamily::Gray);
        match intent {
            None if self.families.is_empty() => None,
            None => Some(Because::NotBuiltYet(NO_INTENT_FOR_THE_ALTERNATE)),
            Some(held) if needs_one => self
                .families
                .iter()
                .any(|family| *family != DeviceFamily::Gray && *family != held)
                .then_some(Because::NotBuiltYet(NO_INTENT_FOR_THE_ALTERNATE)),
            Some(_) => None,
        }
    }
}

/// The device spaces the alternates name, which the output then uses where the input did not.
///
/// ISO 19005-2 section 6.2.4.3 licenses a device space through a PDF/A output intent, so a
/// conversion writing one needs an intent of that family (any family, for grey) — which is why
/// the preparation asks for the intent as the input's own device rows would have.
pub(super) fn device_families(
    document: &Document,
    alternates: &BTreeMap<ObjectId, Object>,
) -> BTreeSet<DeviceFamily> {
    let mut out = BTreeSet::new();
    for alternate in alternates.values() {
        families_in(document, alternate, 0, &mut out);
    }
    out
}

/// One value's contribution to [`device_families`]: a device name, or one inside an array such
/// as an `Indexed` space's base.
fn families_in(
    document: &Document,
    value: &Object,
    depth: usize,
    out: &mut BTreeSet<DeviceFamily>,
) {
    if depth >= MAX_ENTRY_DEPTH {
        return;
    }
    match &document.resolve(value) {
        Object::Name(name) => match name.as_bytes() {
            b"DeviceGray" => {
                out.insert(DeviceFamily::Gray);
            }
            b"DeviceRGB" => {
                out.insert(DeviceFamily::Rgb);
            }
            b"DeviceCMYK" => {
                out.insert(DeviceFamily::Cmyk);
            }
            _ => {}
        },
        Object::Array(items) => {
            for item in items {
                families_in(document, item, depth.saturating_add(1), out);
            }
        }
        _ => {}
    }
}

/// The value with every `ICCBased` array naming an unreadable profile written as its alternate.
///
/// Returns how many arrays were replaced. `value` itself is replaced where it is such an array,
/// which is the shape of a colour space written as an object of its own.
pub(super) fn write_alternates(
    document: &Document,
    value: &mut Object,
    alternates: &BTreeMap<ObjectId, Object>,
    depth: usize,
) -> usize {
    if depth >= MAX_ENTRY_DEPTH {
        return 0;
    }
    let deeper = depth.saturating_add(1);
    match value {
        Object::Array(items) => {
            if let Some(alternate) =
                icc_profile_of(document, items).and_then(|at| alternates.get(&at))
            {
                *value = alternate.clone();
                return 1;
            }
            items
                .iter_mut()
                .map(|item| write_alternates(document, item, alternates, deeper))
                .sum()
        }
        Object::Dictionary(dict) => write_alternates_in(document, dict, alternates, deeper),
        _ => 0,
    }
}

/// [`write_alternates`] over each entry of a dictionary, the entries it changed written back.
pub(super) fn write_alternates_in(
    document: &Document,
    dict: &mut Dictionary,
    alternates: &BTreeMap<ObjectId, Object>,
    depth: usize,
) -> usize {
    let mut written: usize = 0;
    let entries: Vec<(Name, Object)> = dict
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    for (name, mut value) in entries {
        let here = write_alternates(document, &mut value, alternates, depth);
        if here > 0 {
            dict.insert(name, value);
            written = written.saturating_add(here);
        }
    }
    written
}

/// The operator's two texts, for the tags ICC.1 calls the profile's description and copyright.
#[derive(Debug, Clone, Copy)]
pub(super) struct Texts<'a> {
    /// What a profile lacking `desc` is made to call itself.
    pub(super) description: Option<&'a str>,
    /// What a profile lacking `cprt` is made to state as its copyright information.
    pub(super) copyright: Option<&'a str>,
}

/// The profile stream with the missing text tags written in, or `None` where the operator stated
/// no text for one of them or the profile's tag table cannot be extended.
pub(super) fn supplied_profile(
    stream: &Stream,
    data: &[u8],
    shortfall: &IccTagShortfall,
    texts: Texts<'_>,
) -> Option<Object> {
    let version_2 = *data.get(8)? < 4;
    let mut tags: Vec<([u8; 4], Vec<u8>)> = Vec::new();
    if shortfall.description {
        let text = texts.description?;
        tags.push((
            *b"desc",
            if version_2 {
                text_description_type(text)?
            } else {
                multi_localized_unicode_type(text)?
            },
        ));
    }
    if shortfall.copyright {
        let text = texts.copyright?;
        tags.push((
            *b"cprt",
            if version_2 {
                text_type(text)?
            } else {
                multi_localized_unicode_type(text)?
            },
        ));
    }
    let profile = with_tags(data, &tags)?;
    let encoded = flate_encode(&profile, super::COMPRESSION_LEVEL)?;
    let mut dict = stream.dict.clone();
    dict.remove("DecodeParms");
    dict.remove("Length");
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: encoded.into(),
        decryption_failed: false,
    })))
}

/// The length of the profile header, ICC.1 section 6.1.
const HEADER: usize = 128;

/// Where the tag table's first entry starts, after its four-byte count, ICC.1 section 6.2.
const TABLE: usize = 132;

/// The bytes of one tag table entry: signature, offset and size, ICC.1 section 6.2.
const ENTRY: usize = 12;

/// The profile with tags appended: the table grown by one entry per tag, every existing tag's
/// offset moved by what the table grew, the new tags placed after the old data on the 4-byte
/// boundary ICC.1 section 6.2.2 requires, and the header's size and any stated profile ID
/// restated.
///
/// `None` for a profile whose table names data inside the table or past the profile's end — a
/// table this cannot move without guessing what the producer meant.
fn with_tags(data: &[u8], tags: &[([u8; 4], Vec<u8>)]) -> Option<Vec<u8>> {
    let size = usize::try_from(be32(data, 0)?).ok()?;
    let profile = data.get(..size)?;
    let count = usize::try_from(be32(profile, HEADER)?).ok()?;
    let table_end = HEADER
        .checked_add(4)?
        .checked_add(count.checked_mul(ENTRY)?)?;
    let grown = tags.len().checked_mul(ENTRY)?;
    let shift = u32::try_from(grown).ok()?;
    let mut out = Vec::with_capacity(size.checked_add(grown)?.checked_add(256)?);
    out.extend_from_slice(profile.get(..HEADER)?);
    out.extend_from_slice(
        &u32::try_from(count.checked_add(tags.len())?)
            .ok()?
            .to_be_bytes(),
    );
    for index in 0..count {
        let at = index.checked_mul(ENTRY)?.checked_add(TABLE)?;
        let entry = profile.get(at..at.checked_add(ENTRY)?)?;
        let offset = be32(entry, 4)?;
        let length = be32(entry, 8)?;
        let start = usize::try_from(offset).ok()?;
        if start < table_end || start.checked_add(usize::try_from(length).ok()?)? > size {
            return None;
        }
        out.extend_from_slice(entry.get(..4)?);
        out.extend_from_slice(&offset.checked_add(shift)?.to_be_bytes());
        out.extend_from_slice(&length.to_be_bytes());
    }
    // The appended entries' offsets are written once the data they point at has a place.
    let appended_entries = out.len();
    out.resize(out.len().checked_add(grown)?, 0);
    out.extend_from_slice(profile.get(table_end..)?);
    for (index, (signature, bytes)) in tags.iter().enumerate() {
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
        let offset = u32::try_from(out.len()).ok()?;
        let length = u32::try_from(bytes.len()).ok()?;
        let at = appended_entries.checked_add(index.checked_mul(ENTRY)?)?;
        let mut entry = [0u8; ENTRY];
        for (slot, byte) in entry.iter_mut().zip(
            signature
                .iter()
                .chain(&offset.to_be_bytes())
                .chain(&length.to_be_bytes()),
        ) {
            *slot = *byte;
        }
        out.get_mut(at..at.checked_add(ENTRY)?)?
            .copy_from_slice(&entry);
        out.extend_from_slice(bytes);
    }
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
    let total = u32::try_from(out.len()).ok()?;
    out.get_mut(0..4)?.copy_from_slice(&total.to_be_bytes());
    // ICC.1 section 7.2.18 makes a zero field mean no identifier was calculated, so only a
    // stated one is restated; leaving the old value would make the field name another profile.
    if stated_id(&out).is_some() {
        let id = computed_id(&out)?;
        out.get_mut(84..100)?.copy_from_slice(&id);
    }
    Some(out)
}

/// ICC.1:1998-09 section 6.5.17's `textType`: the signature, four reserved bytes, and 7-bit ASCII
/// ending in a zero byte.
fn text_type(text: &str) -> Option<Vec<u8>> {
    if !text.is_ascii() || text.contains('\0') {
        return None;
    }
    let mut out = b"text\0\0\0\0".to_vec();
    out.extend_from_slice(text.as_bytes());
    out.push(0);
    Some(out)
}

/// ICC.1:1998-09 section 6.5.16's `textDescriptionType`, holding its 7-bit ASCII description.
///
/// The section's own guidance for a description that is not localised is followed for the other
/// two: the Unicode language code and count are zero with no characters after them, and the
/// `ScriptCode` code and count are zero with the 67-byte Macintosh description zero-filled.
fn text_description_type(text: &str) -> Option<Vec<u8>> {
    if !text.is_ascii() || text.contains('\0') {
        return None;
    }
    let count = u32::try_from(text.len().checked_add(1)?).ok()?;
    let mut out = b"desc\0\0\0\0".to_vec();
    out.extend_from_slice(&count.to_be_bytes());
    out.extend_from_slice(text.as_bytes());
    out.push(0);
    // Unicode language code, Unicode count.
    out.extend_from_slice(&[0; 8]);
    // ScriptCode code, ScriptCode count, and the 67-byte localisable description.
    out.extend_from_slice(&[0; 3]);
    out.extend_from_slice(&[0; 67]);
    Some(out)
}

/// ICC.1:2001-12 section 6.5.14's `multiLocalizedUnicodeType`, holding one record.
///
/// The record's language and region are `en` and `US`: the clause asks for an ISO 639 language
/// code and an ISO 3166 region code and the operator's configuration states neither, so this is
/// a documented choice, the one a reader takes first when no other matches (`doc/adr/1990`).
fn multi_localized_unicode_type(text: &str) -> Option<Vec<u8>> {
    let units: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
    let length = u32::try_from(units.len()).ok()?;
    let mut out = b"mluc\0\0\0\0".to_vec();
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(&12u32.to_be_bytes());
    out.extend_from_slice(b"enUS");
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(&28u32.to_be_bytes());
    out.extend_from_slice(&units);
    Some(out)
}

/// The big-endian `u32` at `at`.
fn be32(data: &[u8], at: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(at..at.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::{multi_localized_unicode_type, text_description_type, text_type, with_tags};
    use pdf_model::icc::Identification;

    /// A version 2 display profile's header and a tag table of one `wtpt` entry, built here
    /// rather than shipped, since this crate redistributes no profile.
    fn profile_with_one_tag(major: u8) -> Vec<u8> {
        let mut out = vec![0u8; 128];
        out[8] = major;
        out[12..16].copy_from_slice(b"mntr");
        out[16..20].copy_from_slice(b"RGB ");
        out[20..24].copy_from_slice(b"XYZ ");
        out[36..40].copy_from_slice(b"acsp");
        out.extend_from_slice(&1u32.to_be_bytes());
        out.extend_from_slice(b"wtpt");
        out.extend_from_slice(&144u32.to_be_bytes());
        out.extend_from_slice(&20u32.to_be_bytes());
        out.extend_from_slice(b"XYZ \0\0\0\0");
        out.extend_from_slice(&[0, 0, 0xf6, 0xd6, 0, 1, 0, 0, 0, 0, 0xd3, 0x2d]);
        let size = u32::try_from(out.len()).unwrap_or_default();
        out[0..4].copy_from_slice(&size.to_be_bytes());
        out
    }

    #[test]
    fn a_supplied_copyright_reads_back_and_the_old_tag_moves_with_the_table() {
        let before = profile_with_one_tag(2);
        let Some(text) = text_type("Supplied by the operator") else {
            panic!("an ASCII text is a textType");
        };
        let Some(after) = with_tags(&before, &[(*b"cprt", text)]) else {
            panic!("a well-formed table takes a tag");
        };
        let read = Identification::read(&after);
        assert_eq!(
            read.and_then(|read| read.copyright).as_deref(),
            Some("Supplied by the operator")
        );
        // The white point is the same twenty bytes, twelve further on.
        assert_eq!(&after[156..176], &before[144..164]);
        assert_eq!(after.len() % 4, 0);
        assert_eq!(
            u32::from_be_bytes([after[0], after[1], after[2], after[3]]) as usize,
            after.len()
        );
    }

    #[test]
    fn a_version_four_profile_takes_both_texts_as_localised_unicode() {
        let before = profile_with_one_tag(4);
        let Some(description) = multi_localized_unicode_type("Profile") else {
            panic!("any text is a multiLocalizedUnicodeType");
        };
        let Some(copyright) = multi_localized_unicode_type("No copyright stated") else {
            panic!("any text is a multiLocalizedUnicodeType");
        };
        let Some(after) = with_tags(&before, &[(*b"desc", description), (*b"cprt", copyright)])
        else {
            panic!("a well-formed table takes two tags");
        };
        let Some(read) = Identification::read(&after) else {
            panic!("the result is a profile");
        };
        assert_eq!(read.description.as_deref(), Some("Profile"));
        assert_eq!(read.copyright.as_deref(), Some("No copyright stated"));
    }

    #[test]
    fn a_version_two_description_reads_back_and_a_non_ascii_one_is_refused() {
        let before = profile_with_one_tag(2);
        let Some(description) = text_description_type("Display") else {
            panic!("an ASCII text is a textDescriptionType");
        };
        let Some(after) = with_tags(&before, &[(*b"desc", description)]) else {
            panic!("a well-formed table takes a tag");
        };
        assert_eq!(
            Identification::read(&after)
                .and_then(|read| read.description)
                .as_deref(),
            Some("Display")
        );
        assert!(text_type("Zo\u{eb}").is_none());
        assert!(text_description_type("Zo\u{eb}").is_none());
    }

    #[test]
    fn a_table_naming_data_inside_itself_is_not_moved() {
        let mut before = profile_with_one_tag(2);
        before[136..140].copy_from_slice(&130u32.to_be_bytes());
        let Some(text) = text_type("x") else {
            panic!("an ASCII text is a textType");
        };
        assert!(with_tags(&before, &[(*b"cprt", text)]).is_none());
    }
}
