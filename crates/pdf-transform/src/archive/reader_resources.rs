//! The resources the reader used, written where ISO 19005 section 6.2.2 looks for them.
//!
//! ISO 19005-2 and ISO 19005-4 section 6.2.2 ask two things of a content stream's resources: the
//! dictionary associated with a stream shall be explicit, and shall define every name the stream
//! uses (the second sentence is part 4's, which `TechNote 0010` A002 reads into part 2). The
//! producer's file fails them in two ways this module answers without touching a content-stream
//! byte (`doc/adr/0816`):
//!
//! - **A form, a tiling pattern or an annotation appearance that states no `/Resources`.** ISO
//!   32000-2 §7.8.3 hands such a stream the resources of the page it is drawn on, and this reader
//!   draws it so (`doc/adr/0255`). [`assign`] writes that dictionary into the stream's own
//!   dictionary: the reference where the page's is an object, a copy where it is direct.
//! - **A name no dictionary defines.** The reader draws nothing for it and says so. [`supply`]
//!   defines the name as an empty resource of its category — a form with no marks, a graphics
//!   state with no parameters — which draws the nothing the reader drew (`doc/adr/2500`).
//!
//! Both read the content-stream survey's own record of which stream looked up which name, so the
//! population is the one the validator judged. **A shape neither can carry out is a refusal**
//! naming it, never a partial fix (`doc/adr/0947`).

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::{MissingInStream, Survey};
use pdf_model::Pages;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};

use super::decision::Because;
use super::prepare::Spare;
use super::sites::inherited_resources;

/// What the survey calls the walk of a page's own `/Contents`.
const PAGE_CONTENT: &str = "the page content";

/// Why a Type 3 glyph procedure's resources are not written.
const GLYPH_PROCEDURE: &str = "a Type 3 glyph procedure states no Resources of its own, and \
     ISO 32000-2 \u{a7}7.8.3 hands it the Type 3 font dictionary's before the page's. The \
     dictionary that would receive the entry is the font's, which several pages and several \
     glyph procedures share, and this conversion does not yet follow a glyph procedure to its font";

/// Why a form reached from pages with different resources is not given one dictionary.
const PAGES_DIFFER: &str = "a form XObject, tiling pattern or appearance that states no \
     Resources is reached from pages whose resource dictionaries differ, so each page resolves \
     its names to other objects and no one dictionary answers for all of them; the elegant \
     remedy is a copy of the form for each page, which is owed";

/// Why a page whose resources are inherited as a direct dictionary is not edited.
const INHERITED_DIRECT: &str = "a page names a resource its resources do not define, and the \
     dictionary it inherits is written directly inside an ancestor of the page tree, shared by \
     every page beneath it and not an object of its own to add the entry to";

/// Why a font name is not defined where the text shown under it was more than glyphs.
const FONT_MOVES: &str = "a text-showing operator ran with a font name no resource dictionary \
     defines, and moved the text position by more than a width (a character or word spacing, a \
     TJ adjustment, the line a ' or \" starts) or added its glyphs to the clipping path. This \
     reader drew none of that text and moved by its own amounts, which a font that draws nothing \
     and has every width zero would not reproduce; the refusal is conservative, and the loss can \
     be stated on an appended page under the profile";

/// Why a font was not defined: the survey stopped before it had read every text-showing operator.
const SURVEY_STOPPED: &str = "the content walk stopped at one of its bounds, so the names it \
     found missing and the streams that state no resources are not all of them";

/// Why a name in a category with no empty resource is not defined.
const NO_EMPTY_RESOURCE: &str = "a content stream names a pattern, a shading or a colour space \
     that no resource dictionary defines. ISO 32000-2 has no empty value of those categories \
     (a pattern that paints nothing and a shading that covers nothing are inventions of what the \
     producer meant), so nothing is defined for the name";

/// Why a name looked up by a stream this conversion cannot follow is not defined.
const UNFOLLOWED_STREAM: &str = "a content stream that is neither a page's contents nor an \
     object of its own (a soft mask's group) names a resource its resources do not define, and \
     the dictionary that would receive the entry is not one this conversion can address";

/// Why a form's resources were not found among the objects `assign` wrote.
const NO_SHARED_DICTIONARY: &str = "a form XObject, tiling pattern or appearance that states no \
     Resources names a resource its page's direct resource dictionary does not define, and the \
     object that dictionary would be written as for it was not written";

/// Why no object number was left for the empty form or the empty font.
const NO_SPARE_OBJECT: &str = "no object number was free for the empty form XObject or the \
     empty font that stands for a name nothing defines";

/// Where an entry goes inside the object being rewritten.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Place {
    /// The object's own direct `/Resources` dictionary, which an assignment may have just written.
    OwnResources,
    /// The object is the resources dictionary.
    Itself,
    /// The object is the category's table of names.
    Table,
}

/// One name defined as an empty resource.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Addition {
    /// The resource category: `XObject` or `ExtGState`.
    pub(super) category: &'static str,
    /// The name the content stream used.
    pub(super) name: Vec<u8>,
    /// The empty resource.
    pub(super) value: Object,
}

/// What one object is given.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Edit {
    /// The object states no `/Resources`; this is the dictionary the reader read it against.
    Assign(Object),
    /// A name defined as an empty resource, in the place [`Place`] says.
    Define(Place, Addition),
}

/// The edits one rewrite makes, per object, and the objects it adds.
#[derive(Debug, Default)]
pub(super) struct ReaderResources {
    /// The edits per object, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, Vec<Edit>>,
    /// The empty form and the empty font, where a name in `XObject` or `Font` is defined.
    pub(super) written: Vec<(ObjectId, Object)>,
    /// The objects an [`Edit::Assign`] was filed against, so that one is filed once.
    assigned: BTreeSet<ObjectId>,
    /// The object written for each distinct direct resource dictionary a form is given, by the
    /// dictionary's text, so that pages stating equal dictionaries share one object.
    shared_by_content: BTreeMap<String, ObjectId>,
    /// The object a page's direct dictionary was written as, by page index.
    shared_by_page: BTreeMap<usize, ObjectId>,
}

impl ReaderResources {
    /// Files an edit against an object.
    ///
    /// **No search of the edits already filed**: a hostile stream can name tens of thousands of
    /// missing resources, and a search per name would make the filing quadratic. The rewrite
    /// writes a name that is already there once and no more, so a name filed twice costs a
    /// comparison and nothing else.
    fn file(&mut self, id: ObjectId, edit: Edit) {
        if matches!(edit, Edit::Assign(_)) && !self.assigned.insert(id) {
            return;
        }
        self.at.entry(id).or_default().push(edit);
    }
}

/// The dictionary the reader read a page's content against, and where it lives.
enum Location {
    /// An object of its own.
    Object(ObjectId),
    /// Written directly in the page dictionary of this object.
    InPage(ObjectId, Dictionary),
    /// Written directly in an ancestor of the page.
    Inherited(Dictionary),
    /// Written directly on the page and given an object of its own for the forms that state none
    /// (the dictionary is that object's content as `assign` wrote it).
    Shared(ObjectId, Dictionary),
}

impl ReaderResources {
    /// The object a direct resource dictionary is written as for the forms of page `page`, once
    /// for each distinct dictionary.
    ///
    /// ISO 32000-2 §7.8.3 lets a resource dictionary be an indirect object any number of streams
    /// refer to. A copy of the dictionary in each form would make the output as large as the
    /// dictionary times the forms (`doc/adr/2500`).
    fn share(
        &mut self,
        document: &Document,
        spare: &mut Spare,
        page: usize,
        dict: &Dictionary,
    ) -> Result<ObjectId, Because> {
        if let Some(id) = self.shared_by_page.get(&page) {
            return Ok(*id);
        }
        let text = format!("{dict:?}");
        let id = if let Some(id) = self.shared_by_content.get(&text) {
            *id
        } else {
            let id = spare
                .take(document)
                .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
            self.written.push((id, Object::Dictionary(dict.clone())));
            self.shared_by_content.insert(text, id);
            id
        };
        self.shared_by_page.insert(page, id);
        Ok(id)
    }
}

/// Where page `index`'s resources live, `None` where the page states a value that is not a
/// dictionary or names an object that is not one.
fn location_of(document: &Document, pages: &Pages<'_>, index: usize) -> Option<Location> {
    let page = pages.get(index)?;
    match page.dict.get("Resources") {
        Some(Object::Reference(id)) => document.get(*id).as_dict().map(|_| Location::Object(*id)),
        Some(Object::Dictionary(dict)) => Some(Location::InPage(page.id?, dict.clone())),
        _ => match inherited_resources(document, &page.dict) {
            Object::Reference(id) => document.get(id).as_dict().map(|_| Location::Object(id)),
            Object::Dictionary(dict) => Some(Location::Inherited(dict)),
            _ => None,
        },
    }
}

/// Every form, pattern and appearance that names resources and states none, given the dictionary
/// its page states.
///
/// ISO 19005-2 section 6.2.2 and ISO 19005-4 section 6.2.2 require the association to be explicit;
/// ISO 32000-2 §7.8.3 is where the reader's own search ends, at the page's resources, so writing
/// that dictionary changes no name's resolution (`doc/adr/0255`, `doc/adr/2500`).
pub(super) fn assign(
    document: &Document,
    survey: &Survey,
    spare: &mut Spare,
) -> Result<ReaderResources, Because> {
    if survey.stopped_short() {
        return Err(Because::NotBuiltYet(SURVEY_STOPPED));
    }
    let pages = Pages::new(document);
    let mut found = ReaderResources::default();
    let mut given: BTreeMap<ObjectId, Object> = BTreeMap::new();
    for opened in survey.opened_streams() {
        if !opened.referenced || opened.own_resources {
            continue;
        }
        let Some(object) = opened.object else {
            // A page's own contents are the page rewrite's; a soft mask's group is held resolved.
            continue;
        };
        if opened.what == "a Type 3 glyph procedure" {
            return Err(Because::NotBuiltYet(GLYPH_PROCEDURE));
        }
        let value = if let Some(id) = found.shared_by_page.get(&opened.page) {
            Object::Reference(*id)
        } else {
            match location_of(document, &pages, opened.page) {
                None => return Err(Because::NotBuiltYet(INHERITED_DIRECT)),
                Some(Location::Object(id) | Location::Shared(id, _)) => Object::Reference(id),
                Some(Location::InPage(_, dict) | Location::Inherited(dict)) => {
                    Object::Reference(found.share(document, spare, opened.page, &dict)?)
                }
            }
        };
        match given.get(&object) {
            Some(earlier) if *earlier != value => return Err(Because::NotBuiltYet(PAGES_DIFFER)),
            Some(_) => {}
            None => {
                given.insert(object, value.clone());
                found.file(object, Edit::Assign(value));
            }
        }
    }
    Ok(found)
}

/// The shared empty objects a run has written so far.
#[derive(Default)]
struct Empties {
    /// The form that stands for a name in `XObject`.
    form: Option<ObjectId>,
    /// The font that stands for a name in `Font`.
    font: Option<ObjectId>,
}

impl Empties {
    /// The empty form, written on first use.
    fn form(
        &mut self,
        document: &Document,
        spare: &mut Spare,
        written: &mut Vec<(ObjectId, Object)>,
    ) -> Result<ObjectId, Because> {
        if let Some(id) = self.form {
            return Ok(id);
        }
        let id = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        written.push((id, empty_form()));
        self.form = Some(id);
        Ok(id)
    }

    /// The empty font and its one glyph procedure, written on first use.
    fn font(
        &mut self,
        document: &Document,
        spare: &mut Spare,
        written: &mut Vec<(ObjectId, Object)>,
    ) -> Result<ObjectId, Because> {
        if let Some(id) = self.font {
            return Ok(id);
        }
        let procedure = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        let id = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        written.push((procedure, empty_glyph_procedure()));
        written.push((id, empty_font(procedure)));
        self.font = Some(id);
        Ok(id)
    }
}

/// The empty value of a category, for the three that have one.
///
/// ISO 32000-2 §8.4.5 Table 57 lists every entry of a graphics state parameter dictionary as
/// optional, so an empty one sets no parameter, which is what a `gs` naming nothing set. A form
/// `XObject` is clipped to its `/BBox` (§8.10.2, Table 93), so one whose box has no area marks
/// nothing wherever it is invoked. A Type 3 font (§9.6.4) whose one glyph procedure paints
/// nothing and states a width of zero draws nothing and, with the character and word spacing
/// zero, moves nothing (see [`FONT_MOVES`] for where it would).
fn empty_resource(
    document: &Document,
    category: &'static str,
    name: &[u8],
    spare: &mut Spare,
    empties: &mut Empties,
    written: &mut Vec<(ObjectId, Object)>,
) -> Result<Object, Because> {
    match category {
        "ExtGState" => Ok(Object::Dictionary(Dictionary::new())),
        "XObject" => Ok(Object::Reference(empties.form(document, spare, written)?)),
        // One of §9.6.2.2's fourteen is drawn by this reader from its own faces without a
        // resource, so what it drew is a font of that name and not an empty one.
        "Font" if is_standard(name) => Ok(standard_font(name)),
        "Font" => Ok(Object::Reference(empties.font(document, spare, written)?)),
        _ => Err(Because::NotBuiltYet(NO_EMPTY_RESOURCE)),
    }
}

/// Whether `name` is one of ISO 32000-2 §9.6.2.2's fourteen, which the reader draws unaided.
fn is_standard(name: &[u8]) -> bool {
    std::str::from_utf8(name).is_ok_and(pdf_font::standard::is_standard_name)
}

/// A Type 1 font dictionary naming one of the fourteen, as the reader drew it.
fn standard_font(name: &[u8]) -> Object {
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Font"[..])),
    );
    dict.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"Type1"[..])),
    );
    dict.insert(
        Name::new(&b"BaseFont"[..]),
        Object::Name(Name::new(name.to_vec())),
    );
    Object::Dictionary(dict)
}

/// The glyph name every code of the empty font selects.
const EMPTY_GLYPH: &[u8] = b"empty";

/// The one glyph procedure of the empty font: width zero, nothing painted.
///
/// ISO 32000-2 §9.6.4's Table 111 permits `d0` only as the first operator of a glyph description,
/// and its width has to agree with the font's `/Widths`; `0 0 d0` names zero on both axes.
fn empty_glyph_procedure() -> Object {
    let data = b"0 0 d0\n".to_vec();
    let mut dict = Dictionary::new();
    dict.insert(Name::new(&b"Length"[..]), Object::Integer(7));
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    }))
}

/// A Type 3 font whose every code selects [`empty_glyph_procedure`] at width zero.
///
/// ISO 32000-2 §9.6.4's Table 110 asks a Type 3 font for `/FontBBox`, `/FontMatrix`,
/// `/CharProcs`, `/Encoding`, `/FirstChar`, `/LastChar` and `/Widths`; all 256 codes are given a
/// width so that none depends on a default.
fn empty_font(procedure: ObjectId) -> Object {
    let name = |text: &[u8]| Name::new(text);
    let mut dict = Dictionary::new();
    dict.insert(name(b"Type"), Object::Name(name(b"Font")));
    dict.insert(name(b"Subtype"), Object::Name(name(b"Type3")));
    dict.insert(
        name(b"FontBBox"),
        Object::Array(vec![Object::Integer(0); 4]),
    );
    dict.insert(
        name(b"FontMatrix"),
        Object::Array(vec![
            Object::Real(0.001),
            Object::Integer(0),
            Object::Integer(0),
            Object::Real(0.001),
            Object::Integer(0),
            Object::Integer(0),
        ]),
    );
    let mut procedures = Dictionary::new();
    procedures.insert(name(EMPTY_GLYPH), Object::Reference(procedure));
    dict.insert(name(b"CharProcs"), Object::Dictionary(procedures));
    let mut encoding = Dictionary::new();
    encoding.insert(name(b"Type"), Object::Name(name(b"Encoding")));
    let mut differences = vec![Object::Integer(0)];
    differences.extend(std::iter::repeat_n(Object::Name(name(EMPTY_GLYPH)), 256));
    encoding.insert(name(b"Differences"), Object::Array(differences));
    dict.insert(name(b"Encoding"), Object::Dictionary(encoding));
    dict.insert(name(b"FirstChar"), Object::Integer(0));
    dict.insert(name(b"LastChar"), Object::Integer(255));
    dict.insert(
        name(b"Widths"),
        Object::Array(vec![Object::Integer(0); 256]),
    );
    Object::Dictionary(dict)
}

/// A form `XObject` that marks nothing.
fn empty_form() -> Object {
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"XObject"[..])),
    );
    dict.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"Form"[..])),
    );
    dict.insert(
        Name::new(&b"BBox"[..]),
        Object::Array(vec![Object::Integer(0); 4]),
    );
    dict.insert(
        Name::new(&b"Resources"[..]),
        Object::Dictionary(Dictionary::new()),
    );
    dict.insert(Name::new(&b"Length"[..]), Object::Integer(0));
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: Vec::new().into(),
        decryption_failed: false,
    }))
}

/// Every name a content stream used that its resources did not define, defined as an empty
/// resource of its category.
///
/// ISO 19005-4 section 6.2.2 (and part 2's by `TechNote 0010` A002) asks the dictionary to define
/// every name; the reader drew nothing for the name, and an empty resource draws the same
/// (`doc/adr/2500`).
pub(super) fn supply(
    document: &Document,
    survey: &Survey,
    spare: &mut Spare,
    assigned: Option<&ReaderResources>,
) -> Result<ReaderResources, Because> {
    if survey.stopped_short() {
        return Err(Because::NotBuiltYet(SURVEY_STOPPED));
    }
    let pages = Pages::new(document);
    let mut found = ReaderResources::default();
    let mut empties = Empties::default();
    let shared = assigned.map(|assigned| &assigned.shared_by_page);
    let moving: BTreeSet<(usize, &[u8], Option<ObjectId>)> = survey
        .moving_shows()
        .iter()
        .map(|show| (show.page, show.name.as_slice(), show.object))
        .collect();
    // Where each (page, stream, category) is defined, worked out once: the survey holds at most
    // `MAX_OBSERVATIONS` misses, and a dictionary read and copied for each would make a stream
    // naming many of them quadratic in its size.
    let mut targets: BTreeMap<Holder, (ObjectId, Place)> = BTreeMap::new();
    for miss in survey.missing_in_streams() {
        if miss.category == "Font"
            && !is_standard(&miss.name)
            && moving.contains(&(miss.page, miss.name.as_slice(), miss.object))
        {
            return Err(Because::NotBuiltYet(FONT_MOVES));
        }
        let value = empty_resource(
            document,
            miss.category,
            &miss.name,
            spare,
            &mut empties,
            &mut found.written,
        )?;
        let key = (miss.page, miss.object, miss.own_resources, miss.category);
        let (holder, place) = if let Some(target) = targets.get(&key) {
            target.clone()
        } else {
            let target = target_of(document, &pages, miss, shared, &mut found)?;
            targets.insert(key, target.clone());
            target
        };
        let addition = Addition {
            category: miss.category,
            name: miss.name.clone(),
            value,
        };
        found.file(holder, Edit::Define(place, addition));
    }
    Ok(found)
}

/// Which page, stream, whether its own resources, and category a definition is made for.
type Holder = (usize, Option<ObjectId>, bool, &'static str);

/// The object a miss's definition is filed against, and where inside it the name goes.
///
/// A page given a copy of an inherited dictionary is filed an [`Edit::Assign`] on the way.
fn target_of(
    document: &Document,
    pages: &Pages<'_>,
    miss: &MissingInStream,
    shared: Option<&BTreeMap<usize, ObjectId>>,
    found: &mut ReaderResources,
) -> Result<(ObjectId, Place), Because> {
    let page_content = miss.object.is_none() && miss.what == PAGE_CONTENT;
    if miss.object.is_none() && !page_content {
        return Err(Because::NotBuiltYet(UNFOLLOWED_STREAM));
    }
    if miss.what == "a Type 3 glyph procedure" {
        return Err(Because::NotBuiltYet(GLYPH_PROCEDURE));
    }
    let location = if miss.own_resources {
        own_location(document, pages, miss.object, miss.page)?
    } else {
        let location = location_of(document, pages, miss.page)
            .ok_or(Because::NotBuiltYet(INHERITED_DIRECT))?;
        match (location, miss.object) {
            (Location::Object(id), _) => Location::Object(id),
            // The stream is given the object `assign` wrote for the page's dictionary, and the
            // entry goes into that object.
            (Location::InPage(_, dict) | Location::Inherited(dict), Some(_)) => {
                let object = shared
                    .and_then(|shared| shared.get(&miss.page))
                    .ok_or(Because::NotBuiltYet(NO_SHARED_DICTIONARY))?;
                Location::Shared(*object, dict)
            }
            (Location::Shared(..), _) => return Err(Because::NotBuiltYet(INHERITED_DIRECT)),
            (location @ Location::InPage(..), None) => location,
            // The page states none and inherits a dictionary written in an ancestor, shared by
            // every page beneath it. The page is given its own copy (§7.7.3.4 puts that
            // dictionary in force for it already) and the entry goes into the copy.
            (Location::Inherited(dict), None) => {
                let page = pages
                    .get(miss.page)
                    .and_then(|page| page.id)
                    .ok_or(Because::NotBuiltYet(INHERITED_DIRECT))?;
                found.file(page, Edit::Assign(Object::Dictionary(dict.clone())));
                Location::InPage(page, dict)
            }
        }
    };
    place_in(document, &location, miss.category)
}

/// The holder and the dictionary a stream that states its own resources was read against.
fn own_location(
    document: &Document,
    pages: &Pages<'_>,
    object: Option<ObjectId>,
    page: usize,
) -> Result<Location, Because> {
    let (holder, dict) = if let Some(id) = object {
        let dict = match document.get(id) {
            Object::Stream(stream) => stream.dict.clone(),
            Object::Dictionary(dict) => dict,
            _ => return Err(Because::NotBuiltYet(UNFOLLOWED_STREAM)),
        };
        (id, dict)
    } else {
        let page = pages
            .get(page)
            .ok_or(Because::NotBuiltYet(UNFOLLOWED_STREAM))?;
        let id = page.id.ok_or(Because::NotBuiltYet(UNFOLLOWED_STREAM))?;
        (id, page.dict)
    };
    match dict.get("Resources") {
        Some(Object::Reference(id)) => Ok(Location::Object(*id)),
        Some(Object::Dictionary(resources)) => Ok(Location::InPage(holder, resources.clone())),
        _ => Err(Because::NotBuiltYet(UNFOLLOWED_STREAM)),
    }
}

/// The object that holds a category's table of names, and the place in it.
fn place_in(
    document: &Document,
    location: &Location,
    category: &str,
) -> Result<(ObjectId, Place), Because> {
    match location {
        Location::Object(resources) => {
            let table = document
                .get(*resources)
                .as_dict()
                .and_then(|dict| dict.get(category).cloned());
            match table {
                Some(Object::Reference(table)) => {
                    require_dictionary(document, table)?;
                    Ok((table, Place::Table))
                }
                _ => Ok((*resources, Place::Itself)),
            }
        }
        Location::InPage(owner, dict) => match dict.get(category) {
            Some(Object::Reference(table)) => {
                require_dictionary(document, *table)?;
                Ok((*table, Place::Table))
            }
            _ => Ok((*owner, Place::OwnResources)),
        },
        Location::Shared(owner, dict) => match dict.get(category) {
            Some(Object::Reference(table)) => {
                require_dictionary(document, *table)?;
                Ok((*table, Place::Table))
            }
            _ => Ok((*owner, Place::Itself)),
        },
        Location::Inherited(_) => Err(Because::NotBuiltYet(INHERITED_DIRECT)),
    }
}

/// A table object has to be a dictionary for an entry to be added to it.
fn require_dictionary(document: &Document, id: ObjectId) -> Result<(), Because> {
    if document.get(id).as_dict().is_some() {
        Ok(())
    } else {
        Err(Because::NotBuiltYet(UNFOLLOWED_STREAM))
    }
}
