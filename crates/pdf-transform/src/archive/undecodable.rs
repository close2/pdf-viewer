//! The streams whose filter no reader of the standard decodes, and the role each plays.
//!
//! ISO 19005-2 section 6.1.7.2 and ISO 19005-4 section 6.1.6.2 admit only ISO 32000-2's own
//! filters. A name that is an abbreviation of one is spelled out (`super::rewrite`, ADR 1952); a
//! name that is neither — a filter from outside the standard, or one a damaged file misspells —
//! leaves the stream's data unread, because ISO 32000-2 §7.4 makes the stated filters the only
//! route to it:
//!
//! > The filter or filters for a stream shall be specified by the Filter entry in the stream's
//! > dictionary (or the FFilter entry if the stream is external).
//!
//! and nothing in §7.3.8 or §7.4 lets a processor infer another (`doc/adr/2679`). This reader drew
//! the page without such a stream, so ADR 2490's R07 writes it as the reader treated it, absent,
//! and takes the loss `undecodable-stream`: what "absent" means is a property of the stream's
//! role, so the role is found here and each one is written as [`Role`] says (`doc/adr/2689`). A
//! stream whose role is none of these is refused by name: what drawing it absent would mean has
//! not been decided, and an ICC profile is refused by a sentence of its own until the alternate
//! its colour space is drawn as is written in its place.
use std::collections::BTreeSet;

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Object, ObjectId};

use super::decision::Because;
use super::rewrite::FILTER_ABBREVIATIONS;

/// The requirement whose failures name the streams.
const REQUIREMENT: &str = "file-structure/stream-filters-are-standard";

/// What a stream is to the reader, which decides what writing it absent means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// An image `XObject` (ISO 32000-2 §8.9.5): written as a form that draws nothing, at the same
    /// object, so that every `Do` naming it still names a defined resource and paints what the
    /// window painted, which is nothing.
    Image,
    /// A form `XObject` (§8.10): written as a form that draws nothing, for the same reason.
    Form,
    /// A stream a page's `/Contents` names (§7.8.2): written with no data, a content stream
    /// holding no operator, which is what the window drew from it.
    Content,
    /// A font program a font descriptor's `/FontFile`, `/FontFile2` or `/FontFile3` names
    /// (§9.9): left out, so that the font is one with no program and is carried by the routes
    /// every such font takes.
    FontProgram,
    /// An object stream (§7.5.7): not written, as the archive writes no object stream; the
    /// objects it holds were never read, and are §7.3.10's null wherever they are named.
    ObjectStream,
    /// A cross-reference stream (§7.5.8): not written, as the archive writes cross-reference
    /// information of its own; this reader found the objects through the rest of the file's.
    CrossReference,
}

impl Role {
    /// The word the report and the statement page name the role by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Form => "form",
            Self::Content => "content stream",
            Self::FontProgram => "font program",
            Self::ObjectStream => "object stream",
            Self::CrossReference => "cross-reference stream",
        }
    }

    /// What the archive writes in the stream's place, in the report's fixed words.
    #[must_use]
    pub const fn written(self) -> &'static str {
        match self {
            Self::Image | Self::Form => "written as a form that draws nothing",
            Self::Content => "written holding no operator",
            Self::FontProgram => "left out, and the font is carried as one with no program",
            Self::ObjectStream | Self::CrossReference => "not written",
        }
    }
}

/// One stream written absent: facts computed from the source, in the shape of
/// `RemovedPacket` (`doc/adr/2678`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndecodableStream {
    /// The stream, in the source's numbering.
    pub at: ObjectId,
    /// What it is to the reader.
    pub role: Role,
    /// Its stored length: the bytes between `stream` and `endstream`.
    pub length: usize,
    /// The filters its dictionary states, in order, in their written form.
    pub filters: Vec<String>,
    /// The names among them that no reader of the standard decodes.
    pub unknown: Vec<String>,
    /// The first page of the statement listing it, zero-based, where one does.
    pub stated_on: Option<usize>,
}

/// Why no stream of this kind is decoded: the decoder's reason, in fixed words.
pub(crate) const NOT_DECODED: &str = "its filters name one that is neither among ISO 32000-2 \
     section 7.4's standard filters nor an abbreviation Table 92 gives one, so no reader of the \
     standard decodes its data, and this viewer drew the page without it";

impl UndecodableStream {
    /// The sentence the report and `xmpMM:History` carry for it.
    #[must_use]
    pub fn sentence(&self) -> String {
        let stated = self
            .stated_on
            .map(|page| format!("; its removal is stated on page {}", page.saturating_add(1)))
            .unwrap_or_default();
        format!(
            "the {} object {} {} ({} bytes of stream data under {}) is {} (undecodable-stream): \
             {}{stated}",
            self.role.word(),
            self.at.number,
            self.at.generation,
            self.length,
            self.filters.join(" "),
            self.role.written(),
            NOT_DECODED,
        )
    }
}

/// Why a stream is not written absent where the file's own history cannot name it.
pub(crate) const NO_PLACE_TO_RECORD: &str = "writing a stream no reader decodes as absent names it in \
     this file's own xmpMM:History, and this document's XMP packet will not take that entry, so \
     the stream is not removed and no file is written (doc/adr/2689)";

/// The streams of one document written absent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Undecodable {
    /// Each stream, in object order.
    pub(crate) streams: Vec<UndecodableStream>,
}

impl Undecodable {
    /// The `xmpMM:History` parameters naming every stream written absent; `None` where none is.
    #[must_use]
    pub(crate) fn history(&self) -> Option<String> {
        (!self.streams.is_empty()).then(|| {
            let sentences: Vec<String> = self
                .streams
                .iter()
                .map(UndecodableStream::sentence)
                .collect();
            sentences.join("; ")
        })
    }

    /// The stream at `id`, where it is one written absent.
    #[must_use]
    pub(crate) fn at(&self, id: ObjectId) -> Option<&UndecodableStream> {
        self.streams.iter().find(|stream| stream.at == id)
    }
}

/// Whether `object` resolves to a stream whose filters name one that is neither standard nor an
/// abbreviation: one no reader of the standard decodes.
///
/// The font routes ask it of a descriptor's program before the validator's findings are read, so
/// that a program [`Role::FontProgram`] leaves out is one they carry as no program at all.
pub(super) fn decodes_through_no_known_filter(document: &Document, object: &Object) -> bool {
    let Object::Stream(stream) = document.resolve(object) else {
        return false;
    };
    let known = |name: &[u8]| {
        std::str::from_utf8(name).is_ok_and(pdf_archive::table::is_standard_filter)
            || FILTER_ABBREVIATIONS.iter().any(|(short, _)| *short == name)
    };
    match document.get_key(&stream.dict, "Filter") {
        Object::Name(name) => !known(name.as_bytes()),
        Object::Array(names) => names.iter().any(|name| {
            document
                .resolve(name)
                .as_name()
                .is_some_and(|name| !known(name.as_bytes()))
        }),
        _ => false,
    }
}

/// Why an ICC profile under such a filter keeps its refusal: the window drew its colour space as
/// §8.6.5.5's alternate, which the archive would have to state in its place, with the licence
/// ISO 19005 asks of a device space, and this version writes neither (ADR 2689).
const PROFILE_NOT_BUILT: &str = "an ICC profile stream in this file names a filter that is \
     neither one of the base standard's standard filters nor the abbreviation ISO 32000-2 \
     8.9.7's Table 92 gives one, so no reader of the standard decodes it, and this viewer draws \
     its ICCBased colour space as the alternate ISO 32000-2 8.6.5.5 gives it; writing that \
     alternate in the profile's place, with the licence ISO 19005 asks of a device colour space, \
     is not built yet (ADR 2689)";

/// Why a stream with such a filter keeps its refusal: its role is none [`Role`] names.
const ROLE_NOT_DECIDED: &str = "a stream in this file names a filter that is neither one of the \
     base standard's standard filters nor the abbreviation ISO 32000-2 8.9.7's Table 92 gives one, \
     so no reader of the standard decodes it; it is not an image, a form, a page's content \
     stream, a font program, an object stream or a cross-reference stream (nor an image another \
     names as its mask), the roles for which what this viewer drew without it is decided (ADR 2689), so what writing it absent would \
     mean is not decided and the conversion is refused";

/// The streams the requirement's findings name under a filter that is no abbreviation, each
/// with its role, or the refusal where one has a role [`Role`] does not name.
///
/// The population is the validator's findings (`super::sites`' reason); the role is read off
/// the stream's own dictionary first (§7.5.7's and §7.5.8's `/Type`, §8.9.5's and §8.10's
/// `/Subtype`) and otherwise off the objects that name it.
pub(super) fn survey(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<Undecodable, Because> {
    let mut named: BTreeSet<(ObjectId, String)> = BTreeSet::new();
    for judgement in input.failures() {
        if judgement.id != REQUIREMENT {
            continue;
        }
        let pdf_archive::Outcome::Failed { places, .. } = &judgement.outcome else {
            continue;
        };
        for finding in places {
            let Some(name) = finding.place.name.as_deref() else {
                continue;
            };
            let abbreviation = FILTER_ABBREVIATIONS
                .iter()
                .any(|(short, _)| *short == name.as_bytes());
            if let (false, Some(at)) = (abbreviation, finding.place.object) {
                named.insert((at, name.to_owned()));
            }
        }
    }
    if named.is_empty() {
        return Ok(Undecodable::default());
    }
    let holders = Holders::of(document);
    if named.iter().any(|(at, _)| holders.profiles.contains(at)) {
        return Err(Because::NotBuiltYet(PROFILE_NOT_BUILT));
    }
    let mut streams: Vec<UndecodableStream> = Vec::new();
    for (at, name) in named {
        if let Some(known) = streams.iter_mut().find(|stream| stream.at == at) {
            known.unknown.push(name);
            continue;
        }
        let Object::Stream(stream) = document.get(at) else {
            return Err(Because::NotBuiltYet(ROLE_NOT_DECIDED));
        };
        let role = role_of(document, &stream.dict, at, &holders)
            .ok_or(Because::NotBuiltYet(ROLE_NOT_DECIDED))?;
        streams.push(UndecodableStream {
            at,
            role,
            length: stream.data.len(),
            filters: super::prepare::filters_stated(document, &stream.dict),
            unknown: vec![name],
            stated_on: None,
        });
    }
    Ok(Undecodable { streams })
}

/// The streams the objects of the file name as a page's content or as a font's program.
struct Holders {
    contents: BTreeSet<ObjectId>,
    programs: BTreeSet<ObjectId>,
    /// The images an image names as its `/SMask` or `/Mask` (§8.9.6, §11.6.5.2): a form in
    /// that place is no mask, so such an image has no role here.
    masks: BTreeSet<ObjectId>,
    /// The streams an `/ICCBased` colour space names (§8.6.5.5).
    profiles: BTreeSet<ObjectId>,
}

impl Holders {
    /// Read off every object the cross-reference information places, as `super::font_files`
    /// reads its descriptors.
    fn of(document: &Document) -> Self {
        let mut holders = Self {
            contents: BTreeSet::new(),
            programs: BTreeSet::new(),
            masks: BTreeSet::new(),
            profiles: BTreeSet::new(),
        };
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId::new(number, 0));
            let Some(dict) = object.as_dict() else {
                continue;
            };
            profiles_named(dict, 0, &mut holders.profiles);
            if let Object::Stream(stream) = &object {
                profiles_named(&stream.dict, 0, &mut holders.profiles);
            }
            for key in ["SMask", "Mask"] {
                if let Some(id) = dict.get(key).and_then(Object::as_reference) {
                    holders.masks.insert(id);
                }
            }
            if name_is(document, dict, "Type", b"Page") {
                match dict.get("Contents") {
                    Some(Object::Reference(id)) => {
                        holders.contents.insert(*id);
                    }
                    Some(Object::Array(list)) => holders
                        .contents
                        .extend(list.iter().filter_map(Object::as_reference)),
                    _ => {}
                }
            }
            if name_is(document, dict, "Type", b"FontDescriptor") || dict.get("FontName").is_some()
            {
                for key in ["FontFile", "FontFile2", "FontFile3"] {
                    if let Some(id) = dict.get(key).and_then(Object::as_reference) {
                        holders.programs.insert(id);
                    }
                }
            }
        }
        holders
    }
}

/// How deep [`profiles_named`] looks: a resource dictionary's `/ColorSpace` subdictionary holds
/// the array two levels below the object, and a pattern's or a shading's one more.
const DEEPEST: usize = 4;

/// Every stream an `[/ICCBased …]` array written inside `dict` names, to [`DEEPEST`] levels.
fn profiles_named(dict: &Dictionary, depth: usize, out: &mut BTreeSet<ObjectId>) {
    if depth > DEEPEST {
        return;
    }
    for (_, value) in dict.iter() {
        match value {
            Object::Array(items) => {
                if items
                    .first()
                    .and_then(Object::as_name)
                    .is_some_and(|name| name.as_bytes() == b"ICCBased")
                    && let Some(id) = items.get(1).and_then(Object::as_reference)
                {
                    out.insert(id);
                }
            }
            Object::Dictionary(inner) => profiles_named(inner, depth.saturating_add(1), out),
            _ => {}
        }
    }
}

fn name_is(document: &Document, dict: &Dictionary, key: &str, wanted: &[u8]) -> bool {
    document
        .get_key(dict, key)
        .as_name()
        .is_some_and(|name| name.as_bytes() == wanted)
}

/// The role, from the stream's own dictionary and then from what names it; none where neither
/// says.
fn role_of(
    document: &Document,
    dict: &Dictionary,
    at: ObjectId,
    holders: &Holders,
) -> Option<Role> {
    if name_is(document, dict, "Type", b"ObjStm") {
        return Some(Role::ObjectStream);
    }
    if name_is(document, dict, "Type", b"XRef") {
        return Some(Role::CrossReference);
    }
    if holders.masks.contains(&at) {
        return None;
    }
    if name_is(document, dict, "Subtype", b"Image") {
        return Some(Role::Image);
    }
    if name_is(document, dict, "Subtype", b"Form") {
        return Some(Role::Form);
    }
    if holders.programs.contains(&at) {
        return Some(Role::FontProgram);
    }
    if holders.contents.contains(&at) {
        return Some(Role::Content);
    }
    None
}
