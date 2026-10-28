//! Where each of the lossless rewrites has to reach, read off the validator's own findings.
//!
//! # The class this file is for
//!
//! `doc/pdf-a-mitigations.md` section 13.3 names a class of refusal it calls **owed, not
//! optional**: a requirement refused today although the right answer loses nothing, because
//! nobody has written the rewrite. Every preparation here answers one of those. What they share
//! is a shape — the standard, or a clause of it, says exactly what the corrected value is, and
//! the only question left is *which objects* it goes into.
//!
//! # Why the population is the validator's findings rather than a walk of this converter's
//!
//! [`super::to_unicode`]'s reason, and it is the one that keeps `CLAUDE.md` principle 5 true of
//! this file: `pdf_archive` is the reading of ISO 19005, so which dictionary is a graphics state
//! and which annotation is exempt is its judgement rather than a second reading made here. A
//! findings list is capped (`pdf_archive::Findings`), so a document failing in more places than
//! the cap is one whose rewrite is incomplete — and `doc/adr/0947`'s third stage refuses to write
//! a file that does not conform, which is exactly the net that case needs.
//!
//! # And why a shape the rewrite cannot take is a refusal rather than a partial fix
//!
//! Three of these requirements fail in two shapes, one of which has an answer in the standard and
//! one of which has none — an array of blend modes against a bare name, an appearance
//! subdictionary with an `/AS` against one without, a page's content stream against a form
//! `XObject`'s. `doc/adr/0947`'s first rule is that a decision which cannot be carried out is a
//! refusal rather than a plan, so a document failing in the shape with no answer is refused by
//! the preparation, with the reason the refusal carries.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::Survey;
use pdf_archive::{Outcome, Part, Target};
use pdf_font::LoadedFont;
use pdf_model::Pages;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};
use pdf_syntax::{Document, Version};

use super::decision::Because;
use super::prepare::Spare;
use super::signatures::Site;

/// The objects one rewrite touches.
///
/// A set rather than a list: a requirement can be reported at one object twice — `/CharSet` and
/// `/CIDSet` in one descriptor, two findings for one annotation — and the rewrite is applied to
/// the object once whatever the findings said.
#[derive(Debug, Default)]
pub(super) struct Sites {
    /// The objects to rewrite, in the source's numbering.
    pub(super) at: BTreeSet<ObjectId>,
    /// The objects among them that are a resource dictionary's `ExtGState` table written as an
    /// object of its own, whose entries are therefore graphics states one level down. Filled only
    /// by [`rendering_intents`].
    pub(super) tables: BTreeSet<ObjectId>,
}

/// Every object one requirement's findings name, and whether any place named no object.
///
/// The second half is what separates the two shapes of three of these requirements: a finding
/// whose place is a page rather than an object is inside a content stream, and this verb carries
/// those byte for byte.
pub(super) fn objects_named(
    input: &pdf_archive::Report,
    requirements: &[&str],
) -> (BTreeSet<ObjectId>, bool) {
    let mut objects = BTreeSet::new();
    let mut elsewhere = false;
    for judgement in input.failures() {
        if !requirements.contains(&judgement.id) {
            continue;
        }
        let Outcome::Failed { places, .. } = &judgement.outcome else {
            continue;
        };
        for finding in places {
            match finding.place.object {
                Some(id) => {
                    objects.insert(id);
                }
                None => elsewhere = true,
            }
        }
    }
    (objects, elsewhere)
}

/// The four rendering intents §8.6.5.8 defines, which ISO 19005-2 section 6.2.6 restricts a file
/// to.
pub(super) const RENDERING_INTENTS: [&[u8]; 4] = [
    b"RelativeColorimetric",
    b"AbsoluteColorimetric",
    b"Perceptual",
    b"Saturation",
];

/// The name §8.6.5.8 says a processor uses for any other.
pub(super) const RELATIVE_COLORIMETRIC: &[u8] = b"RelativeColorimetric";

/// Why an inline image's rendering intent is not restated.
const INLINE_IMAGE_INTENT: &str = "a rendering intent this file states is an inline image's \
     Intent entry, which is written inside the content stream that draws the image. §8.6.5.8 \
     says what a processor does with a name it does not recognise, so the answer is known — but \
     writing it down means editing the producer's page, and ADR 0816's fence is where that \
     stops. The same name in a graphics state's RI entry or an image XObject's Intent entry is \
     an object and is restated";

/// Every graphics state and image `XObject` whose rendering intent is to be restated.
///
/// ISO 19005-2 section 6.2.6. The population is the validator's, and the one shape it reports
/// that this rewrite cannot reach is an inline image — whose `/Intent` is content-stream bytes.
///
/// **The validator's list is a prefix when the document states more places than it keeps**
/// (`pdf_archive`'s `Findings`), and a rewrite gated on a prefix restates the first few and leaves
/// the rest failing. Then the sites are found structurally, by the validator's own rule for what
/// a graphics state is (§8.4.5's `ExtGState` table, or a dictionary typed `ExtGState`) and for
/// which dictionary is an image (§8.9.5.1): [`GraphicsStateReach`]. An object that is neither
/// is not a site, so a `/RI` that is not a graphics state's (a push button's `/MK` states one in
/// Table 192, as its rollover icon) is left as the producer wrote it (`doc/adr/2535`).
pub(super) fn rendering_intents(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<Sites, Because> {
    const REQUIREMENT: &str = "graphics/rendering-intent-entries-name-one-of-four";
    let (mut at, in_content) = objects_named(input, &[REQUIREMENT]);
    if in_content {
        return Err(Because::TheFence(INLINE_IMAGE_INTENT));
    }
    let mut tables = BTreeSet::new();
    if reports_a_prefix(input, REQUIREMENT) {
        let reach = GraphicsStateReach::of(document);
        at.extend(reach.sites);
        tables = reach.tables;
    }
    Ok(Sites { at, tables })
}

/// The objects that can state a graphics state's `/RI` or an image's `/Intent`, found by walking
/// the document the way `pdf_archive` does.
#[derive(Default)]
struct GraphicsStateReach {
    /// Every object that is, or holds directly, a graphics state or an image dictionary.
    sites: BTreeSet<ObjectId>,
    /// The `ExtGState` tables that are objects of their own.
    tables: BTreeSet<ObjectId>,
}

impl GraphicsStateReach {
    /// Walks every cross-referenced object once, references not followed.
    fn of(document: &Document) -> Self {
        let mut reach = Self::default();
        for number in document.xref().object_numbers() {
            let id = ObjectId::new(number, 0);
            reach.walk(document, id, &document.get(id), 0);
        }
        reach
    }

    /// One object's own structure and the dictionaries written inside it.
    fn walk(&mut self, document: &Document, id: ObjectId, object: &Object, depth: usize) {
        if depth >= MAX_ENTRY_DEPTH {
            return;
        }
        let deeper = depth.saturating_add(1);
        let dict = match object {
            Object::Array(items) => {
                for item in items {
                    self.walk(document, id, item, deeper);
                }
                return;
            }
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => return,
        };
        let named = |key: &str, value: &[u8]| {
            document
                .get_key(dict, key)
                .as_name()
                .is_some_and(|name| name.as_bytes() == value)
        };
        if named("Type", b"ExtGState") || named("Subtype", b"Image") {
            self.sites.insert(id);
        }
        if dict.get("ExtGState").is_some() {
            self.sites.insert(id);
            if let Some(table) = dict.get("ExtGState").and_then(Object::as_reference) {
                self.tables.insert(table);
                self.sites.insert(table);
            }
            if let Object::Dictionary(table) = document.get_key(dict, "ExtGState") {
                self.sites
                    .extend(table.iter().filter_map(|(_, entry)| entry.as_reference()));
            }
        }
        for (_, value) in dict.iter() {
            self.walk(document, id, value, deeper);
        }
    }
}

/// Whether the validator named fewer places for one requirement than the document states.
fn reports_a_prefix(input: &pdf_archive::Report, requirement: &str) -> bool {
    input.failures().any(|judgement| {
        judgement.id == requirement
            && matches!(&judgement.outcome, Outcome::Failed { places, total } if *total > places.len())
    })
}

/// Why a stream naming a filter that is no abbreviation is not rewritten.
const FILTER_NOT_DECODED: &str = "a stream in this file names a filter that is neither one of \
     the base standard's standard filters nor the abbreviation ISO 32000-2 8.9.7's Table 92 \
     gives one, so this tree does not decode it and cannot say what the stream holds. \
     Re-encoding it under a standard filter would need its decoded bytes, and naming a standard \
     filter over bytes it did not encode would be a statement about them nobody has made";

/// Whether every failing filter name is one [`super::rewrite::FILTER_ABBREVIATIONS`] spells out.
///
/// ISO 19005-2 section 6.1.7.2, ISO 19005-4 section 6.1.6.2. The findings carry the name; the
/// rewrite finds the streams itself, structurally, so a shortened finding list costs nothing
/// here — any stream it did not reach is still in the output's own verdict.
pub(super) fn abbreviated_filters(input: &pdf_archive::Report) -> Result<(), Because> {
    for judgement in input.failures() {
        if judgement.id != "file-structure/stream-filters-are-standard" {
            continue;
        }
        let Outcome::Failed { places, .. } = &judgement.outcome else {
            continue;
        };
        for finding in places {
            let abbreviation = finding.place.name.as_deref().is_some_and(|name| {
                super::rewrite::FILTER_ABBREVIATIONS
                    .iter()
                    .any(|(short, _)| *short == name.as_bytes())
            });
            if !abbreviation {
                return Err(Because::NotBuiltYet(FILTER_NOT_DECODED));
            }
        }
    }
    Ok(())
}

/// Why a blend mode written as a bare name the standard does not define is not resolved.
const BARE_BLEND_MODE: &str = "this file sets a blend mode ISO 32000-2 does not define, written \
     as a bare name. §8.4.1's Table 57 says what a reader takes from an *array* of names and \
     that answer is written down; for a name on its own it says only that the value shall be one \
     of the standard modes, with no sentence about one that is not. So writing a mode in its \
     place would be choosing how these marks composite with what is under them, which \
     doc/questions/A48 forbids, and removing the entry does not state Normal either, because a \
     graphics state parameter dictionary sets only what it names";

/// The two requirements whose subject is a `/BM` entry.
const BLEND_MODE_REQUIREMENTS: [&str; 2] = [
    "graphics/graphics-state-blend-modes-are-defined",
    "graphics/annotation-blend-modes-are-defined",
];

/// Every dictionary whose `/BM` array is to be restated as `Normal`.
///
/// ISO 19005-2 section 6.2.10, ISO 19005-4 section 6.2.9, and the split
/// `doc/pdf-a-mitigations.md` section 4.6 makes: the array half has the standard's own answer
/// behind it and the bare-name half has none, so a document failing in the second shape is
/// refused rather than half-corrected.
pub(super) fn blend_modes(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<Sites, Because> {
    let (named, in_content) = objects_named(input, &BLEND_MODE_REQUIREMENTS);
    if in_content {
        return Err(Because::NotBuiltYet(BARE_BLEND_MODE));
    }
    let mut at = BTreeSet::new();
    for id in named {
        let mut shapes = BlendModes::default();
        shapes.of(document, &document.get(id), 0);
        if shapes.bare {
            return Err(Because::NotBuiltYet(BARE_BLEND_MODE));
        }
        if !shapes.array {
            // The validator recorded a failure this walk does not find, which is a disagreement
            // about where the entry is rather than about what it says. Refusing is the only
            // honest answer: writing nothing and calling the requirement answered would leave
            // `doc/adr/0947`'s third stage to discover it.
            return Err(Because::NotBuiltYet(BARE_BLEND_MODE));
        }
        at.insert(id);
    }
    Ok(Sites {
        at,
        ..Sites::default()
    })
}

/// Which shapes of failing `/BM` one object holds.
///
/// The two are answered together and by one walk, because the *object* is what the validator
/// names and a single object can hold several graphics states — a resource dictionary written
/// directly inside a page is the standing case, and it is why neither question can be asked of
/// the object's top-level dictionary alone.
#[derive(Debug, Default)]
struct BlendModes {
    /// Whether an array naming no defined mode is anywhere inside it.
    array: bool,
    /// Whether a bare name the standard does not define is anywhere inside it.
    bare: bool,
}

impl BlendModes {
    /// One value's own tree, references not followed.
    ///
    /// Not followed because the validator does not follow them either: a graphics state written
    /// as its own object is reported at that object and gets its own answer.
    fn of(&mut self, document: &Document, value: &Object, depth: usize) {
        if depth >= MAX_ENTRY_DEPTH {
            return;
        }
        let deeper = depth.saturating_add(1);
        let dict = match value {
            Object::Array(items) => {
                for item in items {
                    self.of(document, item, deeper);
                }
                return;
            }
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => return,
        };
        if dict.get("BM").is_some() {
            match document.get_key(dict, "BM") {
                Object::Array(names) => {
                    if !names
                        .iter()
                        .any(|name| is_blend_mode(&document.resolve(name)))
                    {
                        self.array = true;
                    }
                }
                other => {
                    if !is_blend_mode(&other) {
                        self.bare = true;
                    }
                }
            }
        }
        for (_, item) in dict.iter() {
            self.of(document, item, deeper);
        }
    }
}

/// How deep inside one object an entry these rewrites are about is looked for.
///
/// `pdf_archive`'s own `MAX_DEPTH` for the same walk, so that neither crate reaches a dictionary
/// the other passed over.
pub(super) const MAX_ENTRY_DEPTH: usize = 32;

/// The blend modes §11.3.5's Tables 134 and 135 define, `Compatible` among them.
pub(super) const BLEND_MODES: [&[u8]; 17] = [
    b"Normal",
    b"Compatible",
    b"Multiply",
    b"Screen",
    b"Overlay",
    b"Darken",
    b"Lighten",
    b"ColorDodge",
    b"ColorBurn",
    b"HardLight",
    b"SoftLight",
    b"Difference",
    b"Exclusion",
    b"Hue",
    b"Saturation",
    b"Color",
    b"Luminosity",
];

/// Whether a value names one of them.
pub(super) fn is_blend_mode(value: &Object) -> bool {
    value
        .as_name()
        .is_some_and(|name| BLEND_MODES.contains(&name.as_bytes()))
}

/// The dictionary of an object, whether it is a dictionary or a stream.
pub(super) fn dictionary_of(object: &Object) -> Option<&Dictionary> {
    match object {
        Object::Dictionary(dict) => Some(dict),
        Object::Stream(stream) => Some(&stream.dict),
        _ => None,
    }
}

/// Why an appearance subdictionary with no `/AS` is not collapsed.
const NO_APPEARANCE_STATE: &str = "this annotation's normal appearance is a subdictionary of \
     appearance states, which ISO 19005 admits only for a button widget, and the annotation \
     states no AS entry selecting one of them. §12.5.2's Table 166 makes AS what selects the \
     applicable stream, so where it is there the collapse writes down the reader's own choice — \
     and where it is not, nothing in the file says which state the document is in, and choosing \
     one is doc/questions/A48's forbidden half";

/// Why an appearance dictionary with no normal appearance is not given one.
const NO_NORMAL_APPEARANCE: &str = "this annotation's appearance dictionary states no N entry, \
     which ISO 32000-2's Table 170 requires as the annotation's normal appearance and which ISO \
     19005 requires to be the one entry such a dictionary holds. The file states no appearance to \
     move there, and writing one would be drawing the annotation for its producer";

/// Why an `/AS` naming no entry of its subdictionary is not collapsed.
const STATE_NOT_HELD: &str = "this annotation's normal appearance is a subdictionary of \
     appearance states and its AS entry names a state the subdictionary does not hold. Table \
     166 makes AS what selects the applicable stream, and here it selects none, so the file states \
     no appearance for the state the annotation is in; choosing another of its states is \
     doc/questions/A48's forbidden half";

/// Why an appearance the validator reported in a shape no annotation has is not collapsed.
const NOT_AN_APPEARANCE: &str = "the validator reported a normal appearance that this \
     conversion does not find as an annotation dictionary whose N is a stream or a subdictionary \
     of streams - the finding names no object, the object is no dictionary, or N or the entry its \
     AS selects is some other kind of object - so there is no stream for the collapse to name";

/// Why a normal appearance written directly inside its dictionary is not given an object.
const NO_NUMBER_FOR_THE_APPEARANCE: &str = "this annotation's normal appearance stream is \
     written directly inside a dictionary rather than as an object of its own, and no unused \
     object number was found to give it one, which the subdictionary written in its place has to \
     refer to";

/// Why a check box's or a radio button's single normal appearance stream is not wrapped.
const A_STATE_WOULD_BE_A_VALUE: &str = "this widget belongs to a check box or a radio button \
     field and its normal appearance is a single stream, where ISO 19005 requires a button \
     widget's N to be a subdictionary of appearance states. §12.7.5.2.3 makes a check box's \
     states Off and the name of its on state, and a radio button's the same, so the one name \
     such a subdictionary would hold says which value the field has — and the file states no \
     appearance for that value. A push button has no value, and its single stream is written as \
     a subdictionary holding it alone (doc/adr/2319)";

/// The appearance state a push button's single normal appearance is written under.
///
/// ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 require a button widget's `/N` to be
/// a subdictionary, `TechNote 0010`'s A012 a push button's included, and §12.5.2's Table 166
/// requires the `/AS` that selects from it. §12.7.5.2.2 gives a push button no value, so nothing
/// in the file or the standard names its one state; the name chosen is §12.5.5's own word for the
/// appearance it holds, and it decides nothing a reader draws, since `/AS` selects that one
/// stream and no other entry refers to it (`doc/adr/2319`).
pub(super) const PUSH_BUTTON_STATE: &[u8] = b"Normal";

/// §12.7.4.1's field flags, the push-button bit: Table 229's bit position 17.
const PUSH_BUTTON_FLAG: i64 = 1 << 16;

/// A field entry on this dictionary or inherited up its `/Parent` chain (§12.7.4.1), or `Null`.
fn inherited_field_entry(document: &Document, dict: &Dictionary, key: &str) -> Object {
    let mut current = dict.clone();
    for _ in 0..MAX_ENTRY_DEPTH {
        let value = document.get_key(&current, key);
        if !matches!(value, Object::Null) {
            return value;
        }
        let parent = document.get_key(&current, "Parent");
        let Some(parent) = parent.as_dict() else {
            return Object::Null;
        };
        current = parent.clone();
    }
    Object::Null
}

/// The appearances an annotation's own `/AS` entry selects.
///
/// ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3.
#[derive(Debug, Default)]
pub(super) struct AppearanceStates {
    /// The stream each annotation's `/AP` `/N` is to name, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, ObjectId>,
    /// The streams this conversion copies out of a subdictionary that held them directly.
    pub(super) written: Vec<(ObjectId, Object)>,
    /// The push buttons whose single normal appearance stream is written as a subdictionary
    /// holding it alone under [`PUSH_BUTTON_STATE`], each with that stream, in the source's
    /// numbering.
    pub(super) wrapped: BTreeMap<ObjectId, ObjectId>,
}

/// Every annotation whose normal appearance is to be collapsed to the state it selects.
pub(super) fn appearance_states(
    document: &Document,
    input: &pdf_archive::Report,
    spare: &mut Spare,
) -> Result<AppearanceStates, Because> {
    let (named, elsewhere) = objects_named(input, &["annotations/normal-appearance-shape"]);
    if elsewhere {
        return Err(Because::NotBuiltYet(NOT_AN_APPEARANCE));
    }
    let mut found = AppearanceStates::default();
    for id in named {
        let resolved = document.get(id);
        let annotation = dictionary_of(&resolved).ok_or(Because::NotBuiltYet(NOT_AN_APPEARANCE))?;
        let states = document.get_key(annotation, "AP");
        let normal = states
            .as_dict()
            .and_then(|appearance| appearance.get("N"))
            .cloned()
            // §7.3.10 reads a reference to an undefined object as one to null, and §7.3.7 an
            // entry whose value is null as no entry, so an `N` naming object 0 states none.
            .filter(|normal| !matches!(document.resolve(normal), Object::Null))
            .ok_or(Because::NotBuiltYet(NO_NORMAL_APPEARANCE))?;
        let button = document
            .get_key(annotation, "Subtype")
            .as_name()
            .is_some_and(|subtype| subtype.as_bytes() == b"Widget")
            && inherited_field_entry(document, annotation, "FT")
                .as_name()
                .is_some_and(|kind| kind.as_bytes() == b"Btn");
        if button && matches!(document.resolve(&normal), Object::Stream(_)) {
            let flags = inherited_field_entry(document, annotation, "Ff")
                .as_integer()
                .unwrap_or(0);
            if flags & PUSH_BUTTON_FLAG == 0 {
                return Err(Because::NotBuiltYet(A_STATE_WOULD_BE_A_VALUE));
            }
            let stream = match &normal {
                Object::Reference(at) => *at,
                direct => {
                    let at = spare
                        .take(document)
                        .ok_or(Because::NotBuiltYet(NO_NUMBER_FOR_THE_APPEARANCE))?;
                    found.written.push((at, direct.clone()));
                    at
                }
            };
            found.wrapped.insert(id, stream);
            continue;
        }
        let states = document.resolve(&normal);
        let states = states
            .as_dict()
            .ok_or(Because::NotBuiltYet(NOT_AN_APPEARANCE))?;
        // §7.3.5 resolves a name by its bytes, so the lookup is byte-exact: an `/AS` whose name
        // is not valid UTF-8 selects the entry spelled with those same bytes and nothing else.
        let state = document
            .get_key(annotation, "AS")
            .as_name()
            .ok_or(Because::NotBuiltYet(NO_APPEARANCE_STATE))?
            .clone();
        let selected = states
            .get_by_name(&state)
            .ok_or(Because::NotBuiltYet(STATE_NOT_HELD))?
            .clone();
        // §12.5.5's subdictionary holds appearance streams, and a stream is an indirect object,
        // so the usual shape is a reference this rewrite can simply name. A subdictionary whose
        // entry is a stream some other object already holds is copied to an object of its own
        // rather than being left unreferenced.
        let stream = match &selected {
            Object::Reference(at) if matches!(document.get(*at), Object::Stream(_)) => *at,
            Object::Stream(_) => {
                let at = spare
                    .take(document)
                    .ok_or(Because::NotBuiltYet(NO_NUMBER_FOR_THE_APPEARANCE))?;
                found.written.push((at, selected.clone()));
                at
            }
            _ => return Err(Because::NotBuiltYet(NOT_AN_APPEARANCE)),
        };
        found.at.insert(id, stream);
    }
    Ok(found)
}

/// The `/Resources` each page with none of its own is to be given.
#[derive(Debug, Default)]
pub(super) struct PageResources {
    /// The value to write, per page object, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, Object>,
}

/// Every page that is to be given the resources dictionary it already inherits.
///
/// ISO 19005-2 section 6.2.2, ISO 19005-4 section 6.2.2, as `TechNote 0010`'s A003 reads the
/// phrase *explicitly associated*.
pub(super) fn page_resources(document: &Document, input: &pdf_archive::Report) -> PageResources {
    let mut pages: BTreeSet<usize> = BTreeSet::new();
    for judgement in input.failures() {
        if judgement.id != "graphics/content-streams-have-an-explicit-resources-dictionary" {
            continue;
        }
        let Outcome::Failed { places, .. } = &judgement.outcome else {
            continue;
        };
        for finding in places {
            // An object is a form `XObject`, a tiling pattern or a Type 3 glyph procedure, which
            // `super::reader_resources::assign` answers; only a page's own `/Contents` is
            // reported at a page with no object beside it.
            if finding.place.object.is_some() {
                continue;
            }
            let Some(page) = finding.place.page else {
                continue;
            };
            pages.insert(page);
        }
    }
    let tree = Pages::new(document);
    let mut found = PageResources::default();
    for index in pages {
        let Some(page) = tree.get(index) else {
            continue;
        };
        let Some(id) = page.id else {
            continue;
        };
        if page.dict.get("Resources").is_some() {
            continue;
        }
        found
            .at
            .insert(id, inherited_resources(document, &page.dict));
    }
    found
}

/// How far up §7.7.3.2's page tree a `/Resources` entry is looked for.
///
/// `pdf_syntax::Limits::DEFAULT`'s `max_depth`, which is the depth the parser admitted the tree
/// at: a `/Parent` chain longer than that is one no page in the document was read through.
const MAX_INHERITANCE_DEPTH: usize = 256;

/// The `/Resources` value §7.7.3.4's inheritance puts in force for a page stating none.
///
/// The **reference** where the page inherits one by reference, so that the dictionary is not
/// copied and every page sharing it goes on sharing it; a copy of the dictionary where it is
/// inherited directly; and an empty dictionary where nothing above the page states one at all,
/// which is the smallest value that makes the association explicit without changing what any
/// name resolves to.
pub(super) fn inherited_resources(document: &Document, page: &Dictionary) -> Object {
    let mut node = page.clone();
    for _ in 0..MAX_INHERITANCE_DEPTH {
        let Some(parent) = node.get("Parent").and_then(Object::as_reference) else {
            break;
        };
        let Some(above) = document.get(parent).as_dict().cloned() else {
            break;
        };
        match above.get("Resources") {
            Some(reference @ Object::Reference(_)) => return reference.clone(),
            Some(Object::Dictionary(resources)) => return Object::Dictionary(resources.clone()),
            _ => node = above,
        }
    }
    Object::Dictionary(Dictionary::new())
}

/// Why a descriptor's incomplete `/CharSet` or `/CIDSet` could not be removed.
const NO_DESCRIPTOR: &str = "a font this file embeds states a CharSet or CIDSet that does not \
     describe the whole of its own program, and the descriptor holding it is not an object this \
     conversion can rewrite — it is written inside the font dictionary rather than beside it, so \
     removing the entry would mean rewriting a value this walk does not reach";

/// The descriptor entries to remove.
#[derive(Debug, Default)]
pub(super) struct DescriptorSets {
    /// Which keys go, per font descriptor object, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, BTreeSet<&'static str>>,
}

/// Every font descriptor whose incomplete subset description is to be removed.
///
/// ISO 19005-2 section 6.2.11.4.2. The findings name the **font**, so the descriptor is found
/// from it — through the descendant `CIDFont` for the `/CIDSet` row, since §9.7.4.1 puts a
/// composite font's descriptor on the `CIDFont` rather than on the Type 0 dictionary.
///
/// **A findings list the validator shortened is answered from the fonts themselves**: every
/// `/Type /Font` object the cross-reference table names whose own descriptor states the key — the
/// population the requirement is checked over, a `CIDFont` among them as an object of its own.
/// Removing the entry from a descriptor whose set was complete loses nothing either, by the same
/// sentence (the entry is optional in both editions of the base standard and its absence means
/// what the subset tag says), so reaching more descriptors than failed costs nothing; reaching
/// fewer left the output failing at the rest (`doc/adr/1978` section 5).
pub(super) fn descriptor_sets(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<DescriptorSets, Because> {
    let mut found = DescriptorSets::default();
    for judgement in input.failures() {
        let key = match judgement.id {
            "fonts/charset-lists-every-glyph-in-the-program" => "CharSet",
            "fonts/cidset-lists-every-cid-in-the-program" => "CIDSet",
            _ => continue,
        };
        let Outcome::Failed { places, total } = &judgement.outcome else {
            continue;
        };
        if places.len() != *total {
            every_descriptor_stating(document, key, &mut found);
        }
        for finding in places {
            let Some(font) = finding.place.object else {
                return Err(Because::NotBuiltYet(NO_DESCRIPTOR));
            };
            let at = descriptor_object(document, font, key == "CIDSet")
                .ok_or(Because::NotBuiltYet(NO_DESCRIPTOR))?;
            found.at.entry(at).or_default().insert(key);
        }
    }
    Ok(found)
}

/// Every font descriptor object a `/Type /Font` object names that states `key`.
fn every_descriptor_stating(document: &Document, key: &'static str, found: &mut DescriptorSets) {
    for number in document.xref().object_numbers() {
        let font = document.get(ObjectId::new(number, 0));
        let Some(font) = dictionary_of(&font) else {
            continue;
        };
        if document
            .get_key(font, "Type")
            .as_name()
            .is_none_or(|name| name.as_bytes() != b"Font")
        {
            continue;
        }
        let Some(at) = font.get("FontDescriptor").and_then(Object::as_reference) else {
            continue;
        };
        if document
            .get(at)
            .as_dict()
            .is_some_and(|descriptor| descriptor.get(key).is_some())
        {
            found.at.entry(at).or_default().insert(key);
        }
    }
}

/// The object a font's `/FontDescriptor` is, through the descendant `CIDFont` where asked for.
fn descriptor_object(document: &Document, font: ObjectId, descendant: bool) -> Option<ObjectId> {
    let resolved = document.get(font);
    let mut dict = dictionary_of(&resolved)?.clone();
    if descendant {
        // §9.7.1: a Type 0 font's `/DescendantFonts` is a one-element array holding the CIDFont.
        let kids = document.get_key(&dict, "DescendantFonts");
        let kid = kids.as_array().and_then(|array| array.first().cloned())?;
        dict = document.resolve(&kid).as_dict().cloned()?;
    }
    dict.get("FontDescriptor").and_then(Object::as_reference)
}

/// Why a `/CIDToGIDMap` is not written, and why a stated one is not replaced.
/// Why a `CIDFont` written directly inside another object is not given the entry.
///
/// `pdf_archive` reports a font with no object of its own at no object (ADR 2413), and lists
/// where each such font stating no entry is written; this rewrite writes into the one place that
/// list names, the `/DescendantFonts` array of a Type 0 font object (ADR 2432).
const CID_TO_GID_MAP_IN_ANOTHER_OBJECT: &str = "an embedded Type 2 CIDFont here is written \
     directly inside another object rather than as an object of its own, and either it states a \
     CIDToGIDMap that is neither a stream nor the name Identity, or what holds it is not the \
     DescendantFonts array written in a Type 0 font that is an object of its own - the one place \
     ISO 32000-2 \u{a7}9.7.6.1 allows that this rewrite writes the entry into";

const CID_TO_GID_MAP_NOT_DERIVED: &str = "an embedded Type 2 CIDFont here states no \
     CIDToGIDMap, or states one that is neither a stream nor the name Identity, and which of \
     those two it is decides the answer together with the file's own version. ISO 32000-1's table \
     gives Identity as this entry's default, so an *absent* entry is written as Identity wherever \
     that edition is the one describing the file: at a part 2 target, which ISO 19005-2 section \
     5.1 builds on that edition, and at a part 4 target when the file is labelled PDF 1.7 or earlier, which \
     ISO 32000-2 \u{a7}I.2 makes the statement that ISO 32000-1 describes it. ISO 32000-2's Table \
     115 requires the entry and states no default, so for a file labelled PDF 2.0, or labelled \
     with no version at all, the same write would assert a mapping the file's own base document \
     does not supply, and deriving the map from the embedded program is a reading of the font \
     this converter does not make. An entry that is *there* and is neither shape is the \
     producer's statement, and replacing it would change which glyph every code selects";

/// The first version ISO 32000-2 describes; every earlier one is ISO 32000-1's (§I.2).
const FIRST_VERSION_WITHOUT_THE_DEFAULT: Version = Version { major: 2, minor: 0 };

/// The embedded Type 2 `CIDFont`s that gain `/CIDToGIDMap` `/Identity`, by where each is written.
#[derive(Debug, Default)]
pub(super) struct CidToGidSites {
    /// The `CIDFont`s that are objects of their own, in the source's numbering.
    pub(super) at: BTreeSet<ObjectId>,
    /// The Type 0 font objects whose `/DescendantFonts` array writes the `CIDFont` directly.
    pub(super) descendant_in: BTreeSet<ObjectId>,
}

/// Every embedded Type 2 `CIDFont` that is to be given `/CIDToGIDMap` `/Identity`.
///
/// ISO 19005-2 section 6.2.11.3.2, ISO 19005-4 section 6.2.10.3.2. Only where the entry is absent
/// altogether, and only where ISO 32000-1 — whose table gives the entry its `Identity` default —
/// is the edition describing the file: always at a part 2 target, and at a part 4 one when the
/// file's own label is below PDF 2.0. ISO 32000-2 §I.2:
///
/// > A PDF file is labelled with the version number from the PDF standard that describes the set of functionality used by the PDF file.
///
/// So relabelling such a file as PDF 2.0 is a translation of what it said, and the absent entry
/// said `Identity`; the write carries that into the edition that has no default (ADR 2246). The
/// label is [`Document::version`]'s — the header raised by any catalog's `/Version` — and a file
/// that states none never said which edition describes it.
///
/// **Where the dictionary is written does not change what its absent entry said**, so a `CIDFont`
/// written directly in its Type 0 font's `/DescendantFonts` array, as §9.7.6.1 allows, is given
/// the entry there under the same two conditions (ADR 2432). The row reports such a font at no
/// object (ADR 2413); `pdf_archive` lists where each one is, and every finding at no object has to
/// be one the list places, or the whole requirement is refused rather than half answered.
pub(super) fn cid_to_gid_maps(
    document: &Document,
    input: &pdf_archive::Report,
    target: Target,
) -> Result<CidToGidSites, Because> {
    let described_by_the_first_edition = match target.part() {
        Part::Two => true,
        Part::Four => document
            .version()
            .is_some_and(|version| version < FIRST_VERSION_WITHOUT_THE_DEFAULT),
    };
    if !described_by_the_first_edition {
        return Err(Because::NotBuiltYet(CID_TO_GID_MAP_NOT_DERIVED));
    }
    let (named, elsewhere) = objects_named(input, &[CID_TO_GID_MAP_PRESENT]);
    let mut descendant_in = BTreeSet::new();
    if elsewhere {
        let placed = pdf_archive::direct_cid_fonts_stating_no_cid_to_gid_map(document, target);
        if placed.len() != findings_at_no_object(input, CID_TO_GID_MAP_PRESENT) {
            return Err(Because::NotBuiltYet(CID_TO_GID_MAP_IN_ANOTHER_OBJECT));
        }
        for holder in placed {
            descendant_in
                .insert(holder.ok_or(Because::NotBuiltYet(CID_TO_GID_MAP_IN_ANOTHER_OBJECT))?);
        }
    }
    let mut at = BTreeSet::new();
    for id in named {
        let resolved = document.get(id);
        let dict =
            dictionary_of(&resolved).ok_or(Because::NotBuiltYet(CID_TO_GID_MAP_NOT_DERIVED))?;
        if !document.get_key(dict, "CIDToGIDMap").is_null() {
            return Err(Because::NotBuiltYet(CID_TO_GID_MAP_NOT_DERIVED));
        }
        at.insert(id);
    }
    Ok(CidToGidSites { at, descendant_in })
}

/// The requirement [`cid_to_gid_maps`] answers.
const CID_TO_GID_MAP_PRESENT: &str = "fonts/cid-to-gid-map-present";

/// How many of one requirement's findings name no object.
fn findings_at_no_object(input: &pdf_archive::Report, requirement: &str) -> usize {
    input
        .failures()
        .filter(|judgement| judgement.id == requirement)
        .filter_map(|judgement| match &judgement.outcome {
            Outcome::Failed { places, .. } => Some(places),
            _ => None,
        })
        .flatten()
        .filter(|finding| finding.place.object.is_none())
        .count()
}

/// Why an `/Order` array could not be completed.
const ORDER_NOT_REACHABLE: &str = "an optional content configuration here states an Order array \
     that does not reference every group the file lists in OCProperties/OCGs, and the \
     configuration is not in a place this conversion can rewrite — the catalog states it, or the \
     OCProperties dictionary holding it, in a shape whose object this walk does not reach";

/// The `/Order` arrays to complete, and where each one goes.
#[derive(Debug, Default)]
pub(super) struct CompletedOrders {
    /// The `/Order` each configuration object is to state, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, Vec<Object>>,
    /// The `/OCProperties` dictionary to replace wholesale, where the catalog states one.
    pub(super) properties: Option<(ObjectId, Dictionary)>,
    /// The `/OCProperties` value the catalog is to state, where it states one directly.
    pub(super) in_catalog: Option<Dictionary>,
}

/// How deep an `/Order` array's nesting is followed.
///
/// §8.11.4.3 lets the array nest to show sublayers; a bound rather than a reading, for
/// `CLAUDE.md` principle 3's reason, and past it the groups a deeper array references are read as
/// unreferenced — which appends them and leaves the file conforming rather than failing.
const MAX_ORDER_DEPTH: usize = 64;

/// Every optional content configuration whose `/Order` is to gain the groups it left out.
///
/// ISO 19005-2 section 6.9, ISO 19005-4 section 6.10. A structural read rather than a findings
/// one, because the correction is arithmetic over two arrays the file states — the groups and the
/// order — and the finding says only which group was missing from which configuration.
pub(super) fn completed_orders(document: &Document) -> Result<CompletedOrders, Because> {
    let Ok(catalog) = document.catalog() else {
        return Err(Because::NotBuiltYet(ORDER_NOT_REACHABLE));
    };
    let Some(properties) = document
        .get_key(&catalog, "OCProperties")
        .as_dict()
        .cloned()
    else {
        return Err(Because::NotBuiltYet(ORDER_NOT_REACHABLE));
    };
    let groups: Vec<Object> = document
        .get_key(&properties, "OCGs")
        .as_array()
        .map(<[Object]>::to_vec)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.as_reference().is_some())
        .collect();
    let mut found = CompletedOrders::default();
    let mut rebuilt = properties.clone();
    let mut any_direct = false;
    for key in ["D", "Configs"] {
        let Some(entry) = properties.get(key) else {
            continue;
        };
        let replacement = match (key, entry) {
            ("D", entry) => one_configuration(document, entry, &groups, &mut found),
            (_, Object::Array(items)) => {
                let mut out = Vec::with_capacity(items.len());
                let mut changed = false;
                for item in items {
                    match one_configuration(document, item, &groups, &mut found) {
                        Some(value) => {
                            out.push(value);
                            changed = true;
                        }
                        None => out.push(item.clone()),
                    }
                }
                changed.then_some(Object::Array(out))
            }
            (_, entry) => {
                // A `/Configs` stated as another object. Every configuration it names by
                // reference is rewritten in its own object; one written directly inside an array
                // this walk does not reach is what the refusal is for.
                let resolved = document.resolve(entry);
                let items = resolved
                    .as_array()
                    .map(<[Object]>::to_vec)
                    .unwrap_or_default();
                for item in &items {
                    if one_configuration(document, item, &groups, &mut found).is_some() {
                        return Err(Because::NotBuiltYet(ORDER_NOT_REACHABLE));
                    }
                }
                None
            }
        };
        if let Some(value) = replacement {
            rebuilt.insert(Name::new(key.as_bytes()), value);
            any_direct = true;
        }
    }
    if any_direct {
        match catalog.get("OCProperties") {
            Some(Object::Reference(at)) => found.properties = Some((*at, rebuilt)),
            Some(Object::Dictionary(_)) => found.in_catalog = Some(rebuilt),
            _ => return Err(Because::NotBuiltYet(ORDER_NOT_REACHABLE)),
        }
    }
    Ok(found)
}

/// The `/Name` each optional content configuration stating none is given, and where it is.
///
/// The three places [`AutomaticStates`] reaches, for its reason: a configuration that is an
/// object of its own, one written directly inside an `/OCProperties` object, and one inside an
/// `/OCProperties` the catalog states directly. The rewrite edits the dictionary each is in after
/// the `/Order` completion and the `/AS` removal, so the three compose (`doc/adr/1952`).
#[derive(Debug, Default)]
pub(super) struct ConfigurationNames {
    /// Each configuration that is an object of its own, and its new name.
    pub(super) at: BTreeMap<ObjectId, String>,
    /// Each configuration written directly inside `/OCProperties`: `None` is `/D`, `Some(i)` the
    /// `i`-th entry of `/Configs`.
    pub(super) direct: BTreeMap<Option<usize>, String>,
    /// The `/OCProperties` object holding the direct ones, where it is an object.
    pub(super) properties: Option<ObjectId>,
    /// Whether the catalog states that `/OCProperties` directly.
    pub(super) in_catalog: bool,
}

impl ConfigurationNames {
    /// Every name written, in the order the configurations were met: `/D`, then `/Configs`.
    pub(super) fn written(&self) -> impl Iterator<Item = &String> {
        self.direct.values().chain(self.at.values())
    }
}

/// Every optional content configuration stating no `/Name`, each given `template`'s wording.
///
/// ISO 19005-2 section 6.9, ISO 19005-4 section 6.10. `{n}` becomes the first number from 1 that
/// no configuration's name already is — compared as text, which is how the validator compares
/// them — so the names are unique among themselves and among the producer's. `None` where the
/// file states two configurations of one name, which this does not answer, or a configuration
/// needing a name inside a `/Configs` array that is an object of its own, which this walk does not
/// rewrite; the requirement keeps its refusal in both.
pub(super) fn configuration_names(
    document: &Document,
    template: &str,
) -> Option<ConfigurationNames> {
    let catalog = document.catalog().ok()?;
    let properties = document
        .get_key(&catalog, "OCProperties")
        .as_dict()
        .cloned()?;
    // Each configuration in the order the requirement meets them, with where it is written.
    let mut configurations: Vec<(Option<ObjectId>, Option<usize>, Dictionary)> = Vec::new();
    if let Some(entry) = properties.get("D")
        && let Some(configuration) = document.resolve(entry).as_dict()
    {
        configurations.push((entry.as_reference(), None, configuration.clone()));
    }
    match properties.get("Configs") {
        Some(Object::Array(items)) => {
            for (index, item) in items.iter().enumerate() {
                if let Some(configuration) = document.resolve(item).as_dict() {
                    configurations.push((item.as_reference(), Some(index), configuration.clone()));
                }
            }
        }
        // A `/Configs` that is an object of its own: a configuration it names by reference is
        // named in its own object, and one written directly inside it is past what this walk
        // rewrites — `usize::MAX` marks it, and the loop below refuses where one needs a name.
        Some(entry) => {
            let resolved = document.resolve(entry);
            for item in resolved.as_array().unwrap_or_default() {
                if let Some(configuration) = document.resolve(item).as_dict() {
                    configurations.push((
                        item.as_reference(),
                        Some(usize::MAX),
                        configuration.clone(),
                    ));
                }
            }
        }
        None => {}
    }
    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut unnamed = Vec::new();
    for (at, place, configuration) in configurations {
        match document.get_key(&configuration, "Name").as_string() {
            Some(bytes) => {
                if !used.insert(pdf_syntax::text_string(bytes)) {
                    return None;
                }
            }
            None if at.is_none() && place == Some(usize::MAX) => return None,
            None => unnamed.push((at, place)),
        }
    }
    if unnamed.is_empty() {
        return None;
    }
    let mut found = ConfigurationNames::default();
    let mut number = 0usize;
    for (at, place) in unnamed {
        let name = loop {
            number = number.saturating_add(1);
            let candidate = template.replacen("{n}", &number.to_string(), 1);
            if !used.contains(&candidate) {
                break candidate;
            }
        };
        used.insert(name.clone());
        match at {
            Some(id) => {
                found.at.insert(id, name);
            }
            None => {
                found.direct.insert(place, name);
            }
        }
    }
    if !found.direct.is_empty() {
        match catalog.get("OCProperties") {
            Some(Object::Reference(at)) => found.properties = Some(*at),
            Some(Object::Dictionary(_)) => found.in_catalog = true,
            _ => return None,
        }
    }
    Some(found)
}

/// Why an `/AS` entry could not be removed.
const AUTOMATIC_STATE_NOT_REACHABLE: &str = "an optional content configuration here states the AS \
     entry ISO 19005-2 section 6.9 forbids, and the configuration is not in a place this \
     conversion can rewrite \u{2014} the catalog states no OCProperties dictionary, or a Configs \
     array is an object of its own holding the configuration directly inside it, where this walk \
     rewrites dictionaries rather than arrays";

/// The optional content configurations whose `/AS` entry goes, and where each one is.
///
/// Three places rather than one, because §8.11.4.3's configuration dictionaries may be written at
/// any of them: an object of their own, directly inside an `/OCProperties` that is an object, or
/// directly inside an `/OCProperties` the catalog itself states. The rewrite reaches each by
/// editing the dictionary it is in, which is what lets it compose with
/// [`completed_orders`]' edits to the same dictionaries rather than overwriting them.
#[derive(Debug, Default)]
pub(super) struct AutomaticStates {
    /// Each configuration that is an object of its own and states `/AS`.
    pub(super) at: BTreeSet<ObjectId>,
    /// The `/OCProperties` object holding a configuration that states `/AS` directly inside it.
    pub(super) properties: Option<ObjectId>,
    /// Whether the catalog states such an `/OCProperties` dictionary directly.
    pub(super) in_catalog: bool,
}

/// Every optional content configuration that states the `/AS` ISO 19005-2 section 6.9 forbids.
///
/// A structural read rather than a findings one, for [`completed_orders`]' reason: the edit is the
/// removal of one key, and a findings list is capped where a document's configurations are not.
pub(super) fn automatic_states(document: &Document) -> Result<AutomaticStates, Because> {
    let Ok(catalog) = document.catalog() else {
        return Err(Because::NotBuiltYet(AUTOMATIC_STATE_NOT_REACHABLE));
    };
    let Some(properties) = document
        .get_key(&catalog, "OCProperties")
        .as_dict()
        .cloned()
    else {
        return Err(Because::NotBuiltYet(AUTOMATIC_STATE_NOT_REACHABLE));
    };
    let mut found = AutomaticStates::default();
    let mut any_direct = false;
    for key in ["D", "Configs"] {
        let Some(entry) = properties.get(key) else {
            continue;
        };
        match (key, entry) {
            ("D", entry) => any_direct |= one_automatic_state(document, entry, &mut found),
            (_, Object::Array(items)) => {
                for item in items {
                    any_direct |= one_automatic_state(document, item, &mut found);
                }
            }
            (_, entry) => {
                // A `/Configs` stated as another object. A configuration it names by reference is
                // rewritten in its own object; one written directly inside that array is what the
                // refusal is for, because this walk edits dictionaries and the array is neither.
                let resolved = document.resolve(entry);
                for item in resolved
                    .as_array()
                    .map(<[Object]>::to_vec)
                    .unwrap_or_default()
                {
                    if one_automatic_state(document, &item, &mut found) {
                        return Err(Because::NotBuiltYet(AUTOMATIC_STATE_NOT_REACHABLE));
                    }
                }
            }
        }
    }
    if any_direct {
        match catalog.get("OCProperties") {
            Some(Object::Reference(at)) => found.properties = Some(*at),
            Some(Object::Dictionary(_)) => found.in_catalog = true,
            _ => return Err(Because::NotBuiltYet(AUTOMATIC_STATE_NOT_REACHABLE)),
        }
    }
    Ok(found)
}

/// One configuration entry: `true` where it states `/AS` and is written directly.
///
/// A configuration reached by reference is recorded in [`AutomaticStates::at`] and the entry that
/// names it is left exactly as its producer wrote it.
fn one_automatic_state(document: &Document, entry: &Object, found: &mut AutomaticStates) -> bool {
    let resolved = document.resolve(entry);
    let Some(configuration) = resolved.as_dict() else {
        return false;
    };
    if configuration.get("AS").is_none() {
        return false;
    }
    match entry.as_reference() {
        Some(at) => {
            found.at.insert(at);
            false
        }
        None => true,
    }
}

/// One configuration entry: the completed value where it is direct, and `None` otherwise.
///
/// A configuration reached by reference is recorded in [`CompletedOrders::at`] and the entry that
/// names it is left exactly as its producer wrote it.
fn one_configuration(
    document: &Document,
    entry: &Object,
    groups: &[Object],
    found: &mut CompletedOrders,
) -> Option<Object> {
    let resolved = document.resolve(entry);
    let configuration = resolved.as_dict()?;
    let order = document.get_key(configuration, "Order");
    let order = order.as_array()?;
    let mut referenced = BTreeSet::new();
    order_references(order, 0, &mut referenced);
    let missing: Vec<Object> = groups
        .iter()
        .filter(|group| {
            group
                .as_reference()
                .is_some_and(|id| !referenced.contains(&id))
        })
        .cloned()
        .collect();
    if missing.is_empty() {
        return None;
    }
    let mut completed = order.to_vec();
    completed.extend(missing);
    if let Some(at) = entry.as_reference() {
        found.at.insert(at, completed);
        return None;
    }
    let mut out = configuration.clone();
    out.insert(Name::new(&b"Order"[..]), Object::Array(completed));
    Some(Object::Dictionary(out))
}

/// Every optional content group an `/Order` array references, however deeply nested.
///
/// The entries are read as the array states them rather than resolved, which is what the
/// validator's own reading of this rule does: §8.11.4.3 makes the array's elements optional
/// content group dictionaries and arrays, and a group is named by reference.
fn order_references(order: &[Object], depth: usize, into: &mut BTreeSet<ObjectId>) {
    if depth >= MAX_ORDER_DEPTH {
        return;
    }
    for entry in order {
        match entry {
            Object::Reference(id) => {
                into.insert(*id);
            }
            Object::Array(nested) => {
                order_references(nested, depth.saturating_add(1), into);
            }
            _ => {}
        }
    }
}

/// Why a deprecated XMP packet header attribute could not be cut out.
const PACKET_HEADER_NOT_CUTTABLE: &str = "an XMP packet in this file states the deprecated bytes \
     or encoding attribute in its header, and the header is not one this conversion can edit — \
     the finding names no object, or the stream it names holds no readable packet, or the \
     attribute is written in a shape whose value has no delimiter to cut to. Both attributes \
     describe the packet's own framing, so removing one loses nothing a reader of the packet \
     uses; what is refused here is guessing where one ends";

/// Every metadata stream whose packet header is to lose a deprecated attribute.
///
/// ISO 19005-2 section 6.6.2.1, ISO 19005-4 section 6.7.2.1. The findings name the stream, and
/// both attributes are reported at it — so a packet stating both is one object here rather than
/// two, which is what [`Sites`] is a set for.
pub(super) fn packet_headers(input: &pdf_archive::Report) -> Result<Sites, Because> {
    let (at, elsewhere) = objects_named(input, &["metadata/xmp-packet-header-attributes"]);
    if elsewhere {
        return Err(Because::NotBuiltYet(PACKET_HEADER_NOT_CUTTABLE));
    }
    Ok(Sites {
        at,
        ..Sites::default()
    })
}

/// Why a TrueType font's encoding entry is left as its producer wrote it.
const TRUETYPE_ENCODING_NOT_PROVEN: &str = "a TrueType font in this file states an encoding \
     ISO 19005 does not admit for it, and the entry cannot be changed without changing what a \
     code draws. §9.6.5.4 makes the encoding decide which cmap subtable a code is looked up \
     through, so this conversion proves the change first: the font is loaded as the file states \
     it and again as it would be written, and every code its content streams actually showed has \
     to reach the same glyph both ways. This font either failed that comparison, or embeds no \
     program of its own — in which case what its codes draw is not a fact about the document at \
     all — or showed more text than the survey kept, so that *every* is a word nothing here can \
     say";

/// Whether two readings of one font dictionary select the same glyph for every code shown.
///
/// **The codes the content streams drew, rather than every code a byte could be**, and the
/// choice is this tree's own precedent read twice over: `super::fonts` restates the advances of
/// the glyphs a font *showed*, and `super::to_unicode` derives a `CMap` over the codes a content
/// stream *showed*. A code no operator ever passed to this font draws nothing, so no mark moves
/// when the glyph it would have reached changes — and holding the rewrite to all 256 would
/// refuse almost every real font, because a full Latin face resolves the upper half of
/// `StandardEncoding` and `WinAnsiEncoding` to different glyphs by construction.
///
/// `None` where the survey kept only a prefix of what was drawn: `shown_complete` is
/// `pdf_archive`'s own warning that a rule asking whether *every* shown code is sound may not
/// read this list, and this is exactly such a rule.
fn selects_the_same_glyphs(
    shown: &[Vec<u8>],
    complete: bool,
    before: &LoadedFont,
    after: &LoadedFont,
) -> Option<bool> {
    if !complete {
        return None;
    }
    Some(shown.iter().all(|text| {
        before
            .decode(text)
            .into_iter()
            .all(|code| before.glyph_index(code) == after.glyph_index(code))
    }))
}

/// The distinct strings one font object drew, and whether that list is all of them.
///
/// A font the survey never reached drew nothing, which is a complete list of no strings: the
/// rewrite is then free, because no operator in the file passes a code to this font at all.
fn shown_by(survey: &Survey, font: ObjectId) -> (Vec<Vec<u8>>, bool) {
    let mut strings = Vec::new();
    let mut complete = true;
    for selected in survey.fonts() {
        if selected.id != Some(font) {
            continue;
        }
        complete &= selected.shown_complete;
        strings.extend(selected.shown.keys().cloned());
    }
    (strings, complete)
}

/// One font dictionary, loaded as this file states it, for a comparison to be made against.
///
/// `None` where the font cannot be read at all, and where it carries **no program of its own**:
/// a font whose face this program chose has no appearance the document determines, so a glyph
/// comparison over it would be comparing this reader's substitution with itself rather than
/// proving anything about the file.
pub(super) fn loaded_as_stated(document: &Document, dict: &Dictionary) -> Option<LoadedFont> {
    let font = LoadedFont::load(document, dict, "").ok()?;
    (!font.is_substituted()).then_some(font)
}

/// Every symbolic TrueType font whose `/Encoding` is to be removed.
///
/// ISO 19005-2 section 6.2.11.6, ISO 19005-4 section 6.2.10.6. Which fonts are symbolic is the
/// validator's reading of the descriptor's flags rather than a second one made here; what this
/// adds is the proof that the entry can go without moving a mark.
pub(super) fn symbolic_truetype_encodings(
    document: &Document,
    input: &pdf_archive::Report,
    survey: &Survey,
) -> Result<Sites, Because> {
    let (named, elsewhere) = objects_named(input, &["fonts/symbolic-truetype-states-no-encoding"]);
    if elsewhere {
        return Err(Because::NotBuiltYet(TRUETYPE_ENCODING_NOT_PROVEN));
    }
    let mut at = BTreeSet::new();
    for id in named {
        let resolved = document.get(id);
        let dict =
            dictionary_of(&resolved).ok_or(Because::NotBuiltYet(TRUETYPE_ENCODING_NOT_PROVEN))?;
        let before = loaded_as_stated(document, dict)
            .ok_or(Because::NotBuiltYet(TRUETYPE_ENCODING_NOT_PROVEN))?;
        let mut candidate = dict.clone();
        candidate.remove("Encoding");
        let after = LoadedFont::load(document, &candidate, "")
            .map_err(|_| Because::NotBuiltYet(TRUETYPE_ENCODING_NOT_PROVEN))?;
        let (shown, complete) = shown_by(survey, id);
        if selects_the_same_glyphs(&shown, complete, &before, &after) != Some(true) {
            return Err(Because::NotBuiltYet(TRUETYPE_ENCODING_NOT_PROVEN));
        }
        at.insert(id);
    }
    Ok(Sites {
        at,
        ..Sites::default()
    })
}

/// The two names ISO 19005 admits as a non-symbolic TrueType font's encoding, in the order they
/// are tried.
///
/// An order rather than a preference: a font that passes the proof under either name draws the
/// same glyphs under both, so which one is written is settled by nothing and is therefore settled
/// here.
const STANDARD_TRUETYPE_ENCODINGS: [&[u8]; 2] = [b"WinAnsiEncoding", b"MacRomanEncoding"];

/// The `/Encoding` value each non-symbolic TrueType font is to state.
#[derive(Debug, Default)]
pub(super) struct StandardEncodings {
    /// The value to write, per font object, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, Object>,
}

/// Every non-symbolic TrueType font that is to be given one of §9.6.5.1's two admitted names.
///
/// ISO 19005-2 section 6.2.11.6, ISO 19005-4 section 6.2.10.6. **The value written keeps the
/// producer's own dictionary where they wrote one**: §9.6.5.1 lets `/Encoding` be a name or a
/// dictionary whose `/BaseEncoding` names one, and this requirement is about which name — so a
/// `/Differences` array beside it is left exactly as it was.
pub(super) fn standard_truetype_encodings(
    document: &Document,
    input: &pdf_archive::Report,
    survey: &Survey,
) -> Result<StandardEncodings, Because> {
    let (named, elsewhere) = objects_named(
        input,
        &["fonts/non-symbolic-truetype-uses-a-standard-encoding"],
    );
    if elsewhere {
        return Err(Because::NotBuiltYet(TRUETYPE_ENCODING_NOT_PROVEN));
    }
    let mut found = StandardEncodings::default();
    for id in named {
        let resolved = document.get(id);
        let dict =
            dictionary_of(&resolved).ok_or(Because::NotBuiltYet(TRUETYPE_ENCODING_NOT_PROVEN))?;
        let before = loaded_as_stated(document, dict)
            .ok_or(Because::NotBuiltYet(TRUETYPE_ENCODING_NOT_PROVEN))?;
        let (shown, complete) = shown_by(survey, id);
        let written = STANDARD_TRUETYPE_ENCODINGS
            .into_iter()
            .find_map(|name| {
                let value = standard_encoding_value(document, dict, name);
                let mut candidate = dict.clone();
                candidate.insert(Name::new(&b"Encoding"[..]), value.clone());
                let after = LoadedFont::load(document, &candidate, "").ok()?;
                (selects_the_same_glyphs(&shown, complete, &before, &after) == Some(true))
                    .then_some(value)
            })
            .ok_or(Because::NotBuiltYet(TRUETYPE_ENCODING_NOT_PROVEN))?;
        found.at.insert(id, written);
    }
    Ok(found)
}

/// The `/Encoding` value a font stating this base encoding would have.
///
/// A bare name where the font states none or states one, and the producer's own encoding
/// dictionary with its `/BaseEncoding` restated where they wrote a dictionary — which is the
/// shape §9.6.5.1 gives the two spellings.
fn standard_encoding_value(document: &Document, font: &Dictionary, base: &[u8]) -> Object {
    match document.get_key(font, "Encoding") {
        Object::Dictionary(mut stated) => {
            stated.insert(
                Name::new(&b"BaseEncoding"[..]),
                Object::Name(Name::new(base)),
            );
            Object::Dictionary(stated)
        }
        _ => Object::Name(Name::new(base)),
    }
}

/// Why a Type 3 font's `.notdef` glyph is not renamed.
const TYPE3_NOTDEF_NOT_RENAMED: &str = "a Type 3 font in this file shows a code its encoding \
     names .notdef, which ISO 19005 section 6.2.11.8 forbids a text-showing operator to \
     reference. Its glyph is the producer's own procedure, so this conversion gives that \
     procedure another name and changes no mark; here the font's encoding or glyph procedures \
     could not be read as ISO 32000-2 \u{a7}9.6.4 states them, or the finding names no font \
     object";

/// The `/Encoding` and `/CharProcs` each Type 3 font is to state with its `.notdef` renamed.
#[derive(Debug, Default)]
pub(super) struct Type3Renames {
    /// Per font object, in the source's numbering, the two values to write.
    pub(super) at: BTreeMap<ObjectId, (Object, Object)>,
}

/// Every Type 3 font whose `.notdef` glyph is to be given another name.
///
/// ISO 19005-2 section 6.2.11.8, ISO 19005-4 section 6.2.10.9: no text-showing operator references
/// the `.notdef` glyph. ISO 32000-2 §9.6.4 keys a Type 3 font's glyph descriptions by name in
/// `/CharProcs` and selects them through the encoding's `/Differences`, so the glyph a code draws
/// is the procedure of the name the encoding gives it — and giving that procedure another name, in
/// both places, draws the same marks while referencing no `.notdef`. ADR 0816's fence is about a
/// *program*, whose `.notdef` is glyph 0 by position and cannot be renamed; a Type 3 font has no
/// program, and a font the requirement names that is not one keeps the fence's refusal.
///
/// The new name is the first of `notdef`, `notdef.1`, `notdef.2`, … that neither dictionary
/// already uses. No glyph name in a Type 3 font is read by any rule but its own encoding's, and
/// §9.10.2's text methods find nothing for either name in the Adobe Glyph List, so what the code
/// extracts to is unchanged.
pub(super) fn type3_notdef_renames(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<Type3Renames, Because> {
    let refused = Because::NotBuiltYet(TYPE3_NOTDEF_NOT_RENAMED);
    let (named, elsewhere) = objects_named(input, &["fonts/no-notdef-glyph-shown"]);
    if elsewhere {
        return Err(refused);
    }
    let mut found = Type3Renames::default();
    for id in named {
        let resolved = document.get(id);
        let dict = dictionary_of(&resolved).ok_or(refused)?;
        if document
            .get_key(dict, "Subtype")
            .as_name()
            .map(Name::as_bytes)
            != Some(&b"Type3"[..])
        {
            return Err(Because::TheFence(
                super::decision::THE_GLYPH_IS_NOT_IN_THE_PROGRAM,
            ));
        }
        let Object::Dictionary(mut encoding) = document.get_key(dict, "Encoding") else {
            return Err(refused);
        };
        let Object::Array(differences) = document.get_key(&encoding, "Differences") else {
            return Err(refused);
        };
        let procedures = document.get_key(dict, "CharProcs");
        let mut procedures = procedures.as_dict().cloned().ok_or(refused)?;
        let differences: Vec<Object> = differences
            .iter()
            .map(|item| document.resolve(item))
            .collect();
        let taken = |candidate: &str| {
            procedures.get(candidate).is_some()
                || differences.iter().any(|item| {
                    item.as_name()
                        .is_some_and(|name| name.as_bytes() == candidate.as_bytes())
                })
        };
        // One more candidate than there are names in the two places, so one of them is free.
        let bound = procedures.len().saturating_add(differences.len());
        let fresh = std::iter::once("notdef".to_owned())
            .chain((1..=bound).map(|n| format!("notdef.{n}")))
            .find(|candidate| !taken(candidate))
            .ok_or(refused)?;
        let renamed: Vec<Object> = differences
            .into_iter()
            .map(|item| match item.as_name() {
                Some(name) if name.as_bytes() == b".notdef" => {
                    Object::Name(Name::new(fresh.as_bytes()))
                }
                _ => item,
            })
            .collect();
        encoding.insert(Name::new(&b"Differences"[..]), Object::Array(renamed));
        if let Some(procedure) = procedures.remove(".notdef") {
            procedures.insert(Name::new(fresh.as_bytes()), procedure);
        }
        found.at.insert(
            id,
            (Object::Dictionary(encoding), Object::Dictionary(procedures)),
        );
    }
    Ok(found)
}

/// Why a code that reaches its glyph by a mapping of this reader's choosing is left so.
///
/// The half of ISO 19005-2 section 6.2.11.6 this conversion does not reach.
/// [`standard_route_names`] says where it does.
const STANDARD_ROUTE_NOT_PROVEN: &str = "a TrueType font in this file shows a character code \
     that reaches its glyph only through a mapping ISO 32000-2 \u{a7}9.6.5.4 leaves to the \
     processor, and ISO 19005 requires the subclause's own route. This conversion names such a \
     code's glyph in the encoding's Differences array where the file itself says which glyph it \
     is \u{2014} a non-symbolic font, a ToUnicode stating the code's one character, and a (3, 1) \
     subtable in the program mapping that character to the glyph this reader drew \u{2014} and \
     then proves it: the font is loaded again with the names written, and every code its \
     content streams showed has to reach the glyph it reached before, by the subclause's own \
     steps. This font did not meet one of those conditions, failed the proof, also states a base \
     encoding ISO 19005 does not admit, or showed more text than the survey kept. Where this \
     conversion can also prove it, the same row is met by stating the route in the program's \
     cmap instead, and it did not do that for this file either";

/// The `/Encoding` value each non-symbolic TrueType font is to state so that every code it shows
/// reaches its glyph by §9.6.5.4's own steps.
#[derive(Debug, Default)]
pub(super) struct StandardRouteNames {
    /// The value to write, per font object, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, Object>,
}

/// Every TrueType font whose shown codes are to be named in a `/Differences` array.
///
/// ISO 19005-2 section 6.2.11.6, ISO 19005-4 section 6.2.10.6: a rendered TrueType font's codes
/// reach glyphs by ISO 32000-2 §9.6.5.4's route, never by a mapping the reader chose. For a
/// non-symbolic font that route runs code, glyph name, Adobe Glyph List, (3, 1) subtable — so a
/// code whose glyph the program's (3, 1) subtable maps from some character is reached by the
/// route once the encoding names that character's glyph. Which character is the file's own
/// statement, the `/ToUnicode` (§9.10.3), and it is the one `pdf_font` drew the code through
/// (ADR 1943); ISO 32000-2 Annex D.2's note on the duplicate space code is the standard's own
/// example of a `/Differences` entry doing exactly this.
///
/// **The program is not touched here**: §9.6.5.1 makes the encoding a font dictionary entry, and
/// writing a name there changes which glyph a code selects only where the proof below would
/// fail. The names written are the Adobe Glyph List's, and the program carries the (3, 1)
/// subtable by construction, which is the subclause's third paragraph met. Where this
/// preparation refuses, the row is met by the restated `cmap` instead
/// (`super::fonts::truetype_cmaps_restated`, `doc/adr/2520`).
///
/// # The proof
///
/// [`SymbolicTrueTypeEncodingRemoved`](super::rewrite::Rewrite)'s, made stricter: the font is
/// loaded with the names written, and every code a content stream showed must select the glyph it
/// selected before **and** must no longer need a mapping of the reader's choosing. A font failing
/// either is refused with [`STANDARD_ROUTE_NOT_PROVEN`] rather than written.
pub(super) fn standard_route_names(
    document: &Document,
    input: &pdf_archive::Report,
    survey: &Survey,
) -> Result<StandardRouteNames, Because> {
    let refused = Because::NotBuiltYet(STANDARD_ROUTE_NOT_PROVEN);
    let (named, elsewhere) = objects_named(
        input,
        &["fonts/truetype-codes-reach-glyphs-by-the-standard-route"],
    );
    // A font whose base encoding is also being restated would have two values written into one
    // entry; the two proofs are each made against the font as the file states it.
    let (restated, _) = objects_named(
        input,
        &["fonts/non-symbolic-truetype-uses-a-standard-encoding"],
    );
    if elsewhere || !named.is_disjoint(&restated) {
        return Err(refused);
    }
    let mut found = StandardRouteNames::default();
    for id in named {
        let resolved = document.get(id);
        let dict = dictionary_of(&resolved).ok_or(refused)?;
        let before = loaded_as_stated(document, dict).ok_or(refused)?;
        let (shown, complete) = shown_by(survey, id);
        if !complete {
            return Err(refused);
        }
        let mut differences: BTreeMap<u32, String> = BTreeMap::new();
        for text in &shown {
            for code in before.decode(text) {
                if !before.glyph_needed_a_reader_chosen_mapping(code) {
                    continue;
                }
                let character = before.stated_character(code).ok_or(refused)?;
                let name = pdf_font::encoding::glyph_name(character).ok_or(refused)?;
                differences.insert(code.value(), name);
            }
        }
        let value = with_differences(document, dict, &differences).ok_or(refused)?;
        let mut candidate = dict.clone();
        candidate.insert(Name::new(&b"Encoding"[..]), value.clone());
        let after = LoadedFont::load(document, &candidate, "").map_err(|_| refused)?;
        let by_the_route = shown.iter().all(|text| {
            before.decode(text).into_iter().all(|code| {
                before.glyph_index(code) == after.glyph_index(code)
                    && !after.glyph_needed_a_reader_chosen_mapping(code)
            })
        });
        if !by_the_route {
            return Err(refused);
        }
        found.at.insert(id, value);
    }
    Ok(found)
}

/// The font's `/Encoding` with these codes' names stated in its `/Differences` array.
///
/// A name becomes a dictionary whose `/BaseEncoding` is that name; a dictionary keeps every entry
/// its producer wrote, and its array is rewritten from the codes it already named with these laid
/// over them. Each code is written as its own run, `[code /name]`, which §9.6.5.1's array grammar
/// reads the same as any longer run. `None` for a font stating no encoding, or one this does not
/// read as a name or a dictionary: such a font takes §9.6.5.4's symbolic route, which a
/// `/Differences` array does not reach.
fn with_differences(
    document: &Document,
    font: &Dictionary,
    named: &BTreeMap<u32, String>,
) -> Option<Object> {
    let mut encoding = match document.get_key(font, "Encoding") {
        Object::Name(base) => {
            let mut fresh = Dictionary::new();
            fresh.insert(Name::new(&b"BaseEncoding"[..]), Object::Name(base));
            fresh
        }
        Object::Dictionary(stated) => stated,
        _ => return None,
    };
    let mut codes: BTreeMap<u32, Object> = BTreeMap::new();
    if let Object::Array(stated) = document.get_key(&encoding, "Differences") {
        let mut next: Option<u32> = None;
        for item in stated {
            match document.resolve(&item) {
                Object::Integer(start) => next = u32::try_from(start).ok(),
                Object::Name(name) => {
                    let code = next?;
                    codes.insert(code, Object::Name(name));
                    next = code.checked_add(1);
                }
                _ => return None,
            }
        }
    }
    for (code, name) in named {
        codes.insert(*code, Object::Name(Name::new(name.as_bytes())));
    }
    let mut array = Vec::new();
    for (code, name) in codes {
        array.push(Object::Integer(i64::from(code)));
        array.push(name);
    }
    encoding.insert(Name::new(&b"Differences"[..]), Object::Array(array));
    Some(Object::Dictionary(encoding))
}

/// Why an `OutputIntents` array whose entries name genuinely different profiles is not collapsed.
///
/// The half of ISO 19005-2 section 6.2.3's sentence this rewrite does not reach, and the reason
/// is the clause's own: the entries name *different* destinations, so making them name one
/// throws the loser's away. The array is what tells a reader which device the file's colours
/// were prepared for, and choosing between two such statements is the operator's call rather
/// than this converter's.
const DESTINATION_PROFILES_DIFFER: &str = "two entries of this file's OutputIntents array name \
     destination profiles that are not the same bytes, and ISO 19005 section 6.2.3 requires one \
     object. Pointing both at one profile therefore discards a destination its producer stated — \
     which press or display the other standard's readers were to refer this file's colours to — \
     and nothing in the file says which of the two it meant. Where the two objects carry the \
     same profile, sharing one of them loses nothing and is done";

/// Why a destination profile written as a direct object is not shared.
const DESTINATION_PROFILE_NOT_A_REFERENCE: &str = "an entry of this file's OutputIntents array \
     states a DestOutputProfile that is not an indirect reference. \u{a7}7.3.8.1 requires every \
     stream to be an indirect object — ISO 32000-1:2008, 7.3.8.1 states the same sentence, so it \
     binds a part 2 target as well — so this value is not the ICC profile stream section 6.2.3 \
     requires, and there is no object for the other entries to share. What the entry was meant to \
     name is not in the file";

/// Why an output intent stating a profile reference and no profile of its own keeps its refusal.
const REFERENCE_IS_THE_ONLY_PROFILE: &str = "an output intent here names its destination profile \
     through DestOutputProfileRef alone, which puts the profile outside the file, and embeds no \
     DestOutputProfile beside it. Removing the reference would leave the intent naming no profile \
     at all, so the profile has to come into the file first: that is the external-data case of \
     doc/pdf-a-conversion-limits.md section 2.3, and it is not built for an output intent";

/// Why a profile reference listing an embedded file is not removed.
const REFERENCE_HOLDS_AN_EMBEDDED_FILE: &str = "an output intent here states a \
     DestOutputProfileRef whose URLs array lists an embedded file specification, so the profile \
     it names is a file this document carries. ISO 32000-2 Table 402 admits both kinds of entry; \
     removing the reference would leave that file named by nothing, which loses it from the \
     archive rather than restating anything";

/// Why a profile reference that is not an output intent's own entry is not reached.
const REFERENCE_NOT_AN_OBJECT: &str = "an output intent here states DestOutputProfileRef and is \
     written as a direct dictionary inside an OutputIntents array, so there is no object this \
     conversion's rewrite reaches it in";

/// The two rows that forbid an output intent's `DestOutputProfileRef`.
const PROFILE_REFERENCE_REQUIREMENTS: [&str; 2] = [
    "graphics/no-destination-profile-reference",
    "graphics/no-destination-profile-reference-in-a-pdfx-output-intent",
];

/// Every output intent whose `DestOutputProfileRef` goes (`doc/adr/2083`).
///
/// ISO 19005-4 section 6.2.3 forbids the key in every output intent and ISO 19005-2 section 6.2.3
/// in a PDF/X one. The population is the two rows' findings, each an output intent object stating
/// the key. **Removed only where the same intent embeds a `DestOutputProfile`**: then the profile
/// the intent's colours are referred to is in the file, ISO 32000-2 Table 401 gives the reference
/// nothing a processor must do with it, and this reader reads no reference at all. Three shapes are
/// refused by name: an intent whose reference is its only profile, a reference listing an embedded
/// file, and an intent written as a direct dictionary.
pub(super) fn profile_references(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<Sites, Because> {
    let (at, elsewhere) = objects_named(input, &PROFILE_REFERENCE_REQUIREMENTS);
    if elsewhere {
        return Err(Because::NotBuiltYet(REFERENCE_NOT_AN_OBJECT));
    }
    for id in &at {
        let Object::Dictionary(intent) = document.get(*id) else {
            return Err(Because::NotBuiltYet(REFERENCE_NOT_AN_OBJECT));
        };
        let Some(reference) = intent.get("DestOutputProfileRef") else {
            return Err(Because::NotBuiltYet(REFERENCE_NOT_AN_OBJECT));
        };
        if intent.get("DestOutputProfile").is_none() {
            return Err(Because::NotBuiltYet(REFERENCE_IS_THE_ONLY_PROFILE));
        }
        let reference = document.resolve(reference);
        let urls = reference
            .as_dict()
            .map(|reference| document.get_key(reference, "URLs"));
        let lists_a_file = urls
            .as_ref()
            .and_then(Object::as_array)
            .is_some_and(|urls| {
                urls.iter().any(|entry| {
                    document
                        .resolve(entry)
                        .as_dict()
                        .is_some_and(|spec| spec.get("EF").is_some())
                })
            });
        if lists_a_file {
            return Err(Because::NotBuiltYet(REFERENCE_HOLDS_AN_EMBEDDED_FILE));
        }
    }
    Ok(Sites {
        at,
        ..Sites::default()
    })
}

/// Why a destination profile whose bytes cannot be read is not shared.
const DESTINATION_PROFILE_NOT_DECODED: &str = "a destination profile this file's OutputIntents \
     array names cannot be decoded, so whether two entries carry the same profile cannot be \
     answered. ISO 19005 section 6.2.3 requires one object and this conversion shares one only \
     where it has proved the profiles identical";

/// The one destination profile object every entry of an `OutputIntents` array is to name.
///
/// ISO 19005-2 section 6.2.3, ISO 19005-4 section 6.2.3.
#[derive(Debug)]
pub(super) struct SharedProfile {
    /// The object every entry names afterwards, in the source's numbering.
    pub(super) profile: ObjectId,
    /// The output intent objects whose `/DestOutputProfile` is repointed.
    pub(super) at: BTreeSet<ObjectId>,
    /// The catalog's array, corrected, where an entry written directly inside it had to change.
    ///
    /// `None` where every entry that had to change is an object of its own, which is the usual
    /// shape; an output intent written as a direct dictionary in the array has nowhere else for
    /// the corrected value to go.
    pub(super) entries: Option<Vec<Object>>,
}

/// Every entry of the catalog's `OutputIntents` array that is to name one shared profile.
///
/// # Why the population is the array rather than the findings
///
/// The rule elsewhere in this file is that `pdf_archive`'s findings decide which objects a
/// rewrite touches. This one reads the array instead, and the difference is the requirement's
/// own: it is *about* the catalog's `OutputIntents` array, which is one key of the catalog, so
/// there is no second reading of the standard to make — `doc/pdf-a-mitigations.md` section
/// 13.3.1's warning is about the duplicate-profile rows, whose site is a content stream's choice
/// of colour space and can only be found by walking the pages again. The findings are still what
/// makes this run at all: nothing is prepared that no failed requirement asked for.
///
/// # What makes it lossless, and where it stops
///
/// ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3 require every entry that states a
/// `DestOutputProfile` to state *the same indirect object*. Where the objects differ but the
/// profile streams they hold are the same bytes, naming one of them says exactly what the file
/// said before: every entry still refers its colours to the profile it already referred them to,
/// and the object that goes carried a copy. Where the bytes differ the entries name two
/// destinations, one of them has to go, and that is a decision — [`DESTINATION_PROFILES_DIFFER`].
pub(super) fn shared_destination_profile(
    document: &Document,
    catalog: &Dictionary,
) -> Result<SharedProfile, Because> {
    let stated = super::prepare::output_intent_entries(document, catalog);
    let mut winner: Option<(ObjectId, std::sync::Arc<[u8]>, Dictionary)> = None;
    for entry in &stated {
        let Some(id) = destination_profile(document, entry)? else {
            continue;
        };
        let Object::Stream(stream) = document.get(id) else {
            return Err(Because::TheFence(DESTINATION_PROFILE_NOT_A_REFERENCE));
        };
        let data = document
            .decoded_stream_data(&stream)
            .ok_or(Because::NotBuiltYet(DESTINATION_PROFILE_NOT_DECODED))?;
        let shape = profile_shape(&stream.dict);
        match &winner {
            None => winner = Some((id, data, shape)),
            Some((first, bytes, first_shape)) => {
                if id != *first && (*data != **bytes || &shape != first_shape) {
                    return Err(Because::TheFence(DESTINATION_PROFILES_DIFFER));
                }
            }
        }
    }
    // Unreachable through `Prepared`, which prepares this only for a document the validator
    // failed on the requirement — and it fails only where two entries named two objects.
    let (profile, ..) = winner.ok_or(Because::NotBuiltYet(DESTINATION_PROFILE_NOT_DECODED))?;
    let mut at = BTreeSet::new();
    let mut entries = stated.clone();
    let mut corrected = false;
    for (index, entry) in stated.iter().enumerate() {
        let Some(id) = destination_profile(document, entry)? else {
            continue;
        };
        if id == profile {
            continue;
        }
        if let Some(intent) = entry.as_reference() {
            at.insert(intent);
        } else {
            // An output intent written as a direct dictionary inside the array: there is no
            // object of its own for the corrected value to go in, so the array is what changes.
            let Some(mut fixed) = document.resolve(entry).as_dict().cloned() else {
                continue;
            };
            fixed.insert(
                Name::new(&b"DestOutputProfile"[..]),
                Object::Reference(profile),
            );
            if let Some(slot) = entries.get_mut(index) {
                *slot = Object::Dictionary(fixed);
                corrected = true;
            }
        }
    }
    Ok(SharedProfile {
        profile,
        at,
        entries: corrected.then_some(entries),
    })
}

/// The profile object one entry of an `OutputIntents` array names, where it names one.
///
/// `Ok(None)` for an entry that states no `DestOutputProfile` at all, which the clause's own
/// wording exempts: the sentence binds the entries that state the key, not every entry.
fn destination_profile(document: &Document, entry: &Object) -> Result<Option<ObjectId>, Because> {
    let Some(intent) = document.resolve(entry).as_dict().cloned() else {
        return Ok(None);
    };
    let Some(value) = intent.get("DestOutputProfile") else {
        return Ok(None);
    };
    value
        .as_reference()
        .map(Some)
        .ok_or(Because::TheFence(DESTINATION_PROFILE_NOT_A_REFERENCE))
}

/// A profile stream's dictionary with the three keys that describe its *encoding* taken out.
///
/// Two streams carrying the same profile may hold it differently — one flate-encoded and one
/// not — and §7.3.8.2 makes `/Length`, `/Filter` and `/DecodeParms` the entries that say so. What
/// is left is what the stream asserts about the profile itself: §8.6.5.5's `/N` and `/Alternate`,
/// §14.11.5's `/Metadata`. Those have to agree, because an entry naming a profile with a
/// different `/N` is not naming the same thing.
fn profile_shape(dict: &Dictionary) -> Dictionary {
    let mut out = dict.clone();
    for key in ["Length", "Filter", "DecodeParms"] {
        out.remove(key);
    }
    out
}

/// Why a missing `/Colorants` entry is not synthesised.
///
/// `doc/pdf-a-mitigations.md` section 4.2 proposed deriving the entry from the `DeviceN` space's
/// own alternate space and tint transform, and section 13.3.1 records why the general case cannot
/// be done exactly: §7.10 gives a PDF function no way to call another, so a one-input transform
/// derived from an *N*-input one is a **sample** of the producer's rather than a restatement of
/// it. What this rewrite does instead is the one case that needs no derivation at all.
const COLORANT_NOT_DERIVABLE: &str = "a DeviceN or NChannel colour space here uses a spot \
     colourant its Colorants dictionary does not define, and this file states no Separation \
     array of its own for that colourant. Synthesising one means deriving a one-input tint \
     transform from the space's N-input one, which ISO 32000-2 §7.10 gives a PDF function no way \
     to express: the derived function can only sample the producer's, and an archive would then \
     carry an approximation of their transform written as though it were their definition. Where \
     the file does state a Separation array for the colourant, that array is the entry — ISO \
     19005 section 6.2.4.4 requires every Separation of one name in a file to state the same \
     alternate space and tint transform, so the entry is the producer's own and is written";

/// Why a colourant this file does define twice, differently, is not copied.
const COLOURANT_DEFINED_TWICE: &str = "this file states more than one Separation array for a \
     colourant whose Colorants entry is missing, and the arrays are not the same objects. ISO \
     19005 section 6.2.4.4 requires every Separation of one name to agree, so the file is already \
     failing that rule and there is no producer's definition for this rewrite to copy — choosing \
     between them is the question doc/pdf-a-mitigations.md section 4.2 puts to an operator";

/// Why the `DeviceN` space a finding names cannot be reached.
const COLORANTS_NOT_REACHABLE: &str = "a DeviceN colour space whose Colorants dictionary is \
     missing an entry is written in a place this rewrite cannot complete: the finding names no \
     object, or the space's Attributes or Colorants dictionary is an indirect object of its own, \
     which is not the object the failure was reported at. Writing into an object no finding named \
     would be this converter reading the file a second way";

/// The `/Colorants` entries a `DeviceN` colour space is to gain, per object.
///
/// ISO 19005-2 section 6.2.4.4, ISO 19005-4 section 6.2.4.4.
#[derive(Debug, Default)]
pub(super) struct ColorantEntries {
    /// Per object the findings named, the `Separation` array each missing colourant is to get.
    pub(super) at: BTreeMap<ObjectId, BTreeMap<Vec<u8>, Object>>,
}

/// Every missing `/Colorants` entry this file already states the answer to.
///
/// # Why the file's own `Separation` array is the entry rather than a derivation
///
/// ISO 19005-2 section 6.2.4.4 and ISO 19005-4 section 6.2.4.4 require every `Separation` array
/// in one file that names a given colourant — and the subclause names the ones written inside a
/// `Colorants` dictionary as being among them —
/// to state the same alternate space and the same tint transform, and require the comparison to
/// be made on the PDF objects rather than on what using them computes. So where the file states
/// one for this colourant, the entry that requirement admits is *that array* and nothing else:
/// there is no choice left to make, and the producer's own definition of the ink is written down
/// rather than a function this converter fitted to their N-input one.
///
/// Every other shape is refused, and [`COLORANT_NOT_DERIVABLE`] says why.
pub(super) fn colorant_entries(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<ColorantEntries, Because> {
    let separations = separation_arrays(document);
    let mut found = ColorantEntries::default();
    for judgement in input.failures() {
        if judgement.id != "graphics/spot-colourants-appear-in-the-colorants-dictionary" {
            continue;
        }
        let Outcome::Failed { places, .. } = &judgement.outcome else {
            continue;
        };
        for finding in places {
            let (Some(id), Some(colourant)) = (finding.place.object, finding.place.name.as_ref())
            else {
                return Err(Because::NotBuiltYet(COLORANTS_NOT_REACHABLE));
            };
            let colourant = colourant.clone().into_bytes();
            let array = separations.stated.get(&colourant).ok_or_else(|| {
                Because::NotBuiltYet(if separations.clashed.contains(&colourant) {
                    COLOURANT_DEFINED_TWICE
                } else {
                    COLORANT_NOT_DERIVABLE
                })
            })?;
            found
                .at
                .entry(id)
                .or_default()
                .insert(colourant, array.clone());
        }
    }
    // **The proof that the rewrite reaches every entry it promises.** The same placement the
    // rewriter performs is made here on a copy, and a colourant it could not place refuses the
    // document rather than leaving the output still failing the clause.
    for (id, entries) in &found.at {
        let mut copy = document.get(*id);
        let placed = match &mut copy {
            Object::Stream(stream) => {
                let mut dict = stream.dict.clone();
                place_colorants(document, &mut dict, entries, 0)
            }
            other => place_colorants_in(document, other, entries),
        };
        if entries.keys().any(|name| !placed.contains(name)) {
            return Err(Because::NotBuiltYet(COLORANTS_NOT_REACHABLE));
        }
    }
    Ok(found)
}

/// Every `Separation` colour space array the file's cross-referenced objects state, by colourant.
///
/// §8.6.6.4 shapes the array — the family name, the colourant name, the alternate space and the
/// tint transform — and ISO 19005 section 6.2.4.4 is what makes one per name the only answer:
/// a colourant two arrays define differently is a colourant this file has already failed on, and
/// [`COLOURANT_DEFINED_TWICE`] is what that is.
fn separation_arrays(document: &Document) -> Separations {
    let mut out: BTreeMap<Vec<u8>, Object> = BTreeMap::new();
    let mut clashed: BTreeSet<Vec<u8>> = BTreeSet::new();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId::new(number, 0));
        descend_for_separations(
            document,
            &object,
            0,
            &mut |name, array| match out.get(&name) {
                Some(held) if held == &array => {}
                Some(_) => {
                    clashed.insert(name);
                }
                None => {
                    out.insert(name, array);
                }
            },
        );
    }
    for name in &clashed {
        out.remove(name);
    }
    Separations {
        stated: out,
        clashed,
    }
}

/// What [`separation_arrays`] found: the colourants this file defines once, and those it defines
/// more than one way.
struct Separations {
    /// The one `Separation` array the file states for each colourant it agrees with itself about.
    stated: BTreeMap<Vec<u8>, Object>,
    /// The colourants the file defines two ways, which are already failing section 6.2.4.4.
    clashed: BTreeSet<Vec<u8>>,
}

/// One object's contribution to [`separation_arrays`].
fn descend_for_separations(
    document: &Document,
    object: &Object,
    depth: usize,
    found: &mut impl FnMut(Vec<u8>, Object),
) {
    if depth >= MAX_ENTRY_DEPTH {
        return;
    }
    let deeper = depth.saturating_add(1);
    match object {
        Object::Array(items) => {
            if let Some(name) = separation_colourant(document, items) {
                found(name, Object::Array(items.clone()));
            }
            for item in items {
                descend_for_separations(document, item, deeper, found);
            }
        }
        Object::Dictionary(dict) => {
            for (_, value) in dict.iter() {
                descend_for_separations(document, value, deeper, found);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                descend_for_separations(document, value, deeper, found);
            }
        }
        _ => {}
    }
}

/// The colourant a §8.6.6.4 `Separation` array names, where the array is one.
fn separation_colourant(document: &Document, items: &[Object]) -> Option<Vec<u8>> {
    if items.len() < 4 {
        return None;
    }
    if !is_name(document, items.first()?, b"Separation") {
        return None;
    }
    document
        .resolve(items.get(1)?)
        .as_name()
        .map(|name| name.as_bytes().to_vec())
}

/// Whether a value resolves to one particular name.
fn is_name(document: &Document, value: &Object, wanted: &[u8]) -> bool {
    document
        .resolve(value)
        .as_name()
        .is_some_and(|name| name.as_bytes() == wanted)
}

/// Writes each colourant's `Separation` array into every `DeviceN` space inside `dict` that uses
/// it, and answers which colourants were placed.
///
/// **References are not followed**, for the reason the rest of this file does not follow them:
/// the rewrite reaches the object the failure was reported at, and a dictionary written as its
/// own object is reported at that object. An `/Attributes` or `/Colorants` the space states
/// indirectly is therefore left alone, and [`colorant_entries`]'s proof turns that into a refusal.
pub(super) fn place_colorants(
    document: &Document,
    dict: &mut Dictionary,
    entries: &BTreeMap<Vec<u8>, Object>,
    depth: usize,
) -> BTreeSet<Vec<u8>> {
    let mut placed = BTreeSet::new();
    if depth >= MAX_ENTRY_DEPTH {
        return placed;
    }
    let deeper = depth.saturating_add(1);
    let inside: Vec<(Name, Object)> = dict
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    for (key, value) in inside {
        let mut held = value;
        if place_in_value(document, &mut held, entries, deeper, &mut placed) {
            dict.insert(key, held);
        }
    }
    placed
}

/// The same placement for an object that is not a dictionary.
///
/// A `DeviceN` colour space array written as its own object is reported *at* that object, so the
/// value the rewrite is handed is the array rather than a dictionary holding one.
pub(super) fn place_colorants_in(
    document: &Document,
    value: &mut Object,
    entries: &BTreeMap<Vec<u8>, Object>,
) -> BTreeSet<Vec<u8>> {
    let mut placed = BTreeSet::new();
    place_in_value(document, value, entries, 0, &mut placed);
    placed
}

/// One value inside a dictionary, and whether placing changed it.
fn place_in_value(
    document: &Document,
    value: &mut Object,
    entries: &BTreeMap<Vec<u8>, Object>,
    depth: usize,
    placed: &mut BTreeSet<Vec<u8>>,
) -> bool {
    match value {
        Object::Dictionary(dict) => {
            let inner = place_colorants(document, dict, entries, depth);
            let changed = !inner.is_empty();
            placed.extend(inner);
            changed
        }
        Object::Array(items) => {
            if depth >= MAX_ENTRY_DEPTH {
                return false;
            }
            let mut changed = place_in_devicen(document, items, entries, placed);
            for item in items {
                changed |= place_in_value(document, item, entries, depth.saturating_add(1), placed);
            }
            changed
        }
        _ => false,
    }
}

/// One `DeviceN` colour space array, given the entries its `/Colorants` dictionary is missing.
///
/// §8.6.6.5 shapes the array: the family name, the array of colourant names, the alternate space,
/// the tint transform, and an optional attributes dictionary whose `/Colorants` this writes into.
/// A space that states no attributes dictionary gains one holding nothing but `/Colorants`, which
/// is what Table 71 makes a valid attributes dictionary for a plain `DeviceN`.
fn place_in_devicen(
    document: &Document,
    items: &mut Vec<Object>,
    entries: &BTreeMap<Vec<u8>, Object>,
    placed: &mut BTreeSet<Vec<u8>>,
) -> bool {
    if items.len() < 4 || !is_name(document, &items[0], b"DeviceN") {
        return false;
    }
    let used: BTreeSet<Vec<u8>> = document
        .resolve(&items[1])
        .as_array()
        .map(|names| {
            names
                .iter()
                .filter_map(|entry| {
                    document
                        .resolve(entry)
                        .as_name()
                        .map(|name| name.as_bytes().to_vec())
                })
                .collect()
        })
        .unwrap_or_default();
    let wanted: Vec<(&Vec<u8>, &Object)> = entries
        .iter()
        .filter(|(name, _)| used.contains(*name))
        .collect();
    if wanted.is_empty() {
        return false;
    }
    if items.len() == 4 {
        items.push(Object::Dictionary(Dictionary::new()));
    }
    let Some(Object::Dictionary(attributes)) = items.get_mut(4) else {
        return false;
    };
    let mut colorants = match attributes.get("Colorants") {
        Some(Object::Dictionary(stated)) => stated.clone(),
        Some(_) => return false,
        None => Dictionary::new(),
    };
    for (name, array) in wanted {
        colorants.insert(Name::new(name.clone()), array.clone());
        placed.insert(name.clone());
    }
    attributes.insert(Name::new(&b"Colorants"[..]), Object::Dictionary(colorants));
    true
}

/// One `Separation` colour space array the document states, read as ISO 32000-2 §8.6.6.4 shapes it.
///
/// > It shall be a four-element array whose first element shall be the colour space family name
/// > Separation .
///
/// The two elements kept here are the ones ISO 19005-2 section 6.2.4.4 and ISO 19005-4 section
/// 6.2.4.4 require every array of one colourant to agree about.
#[derive(Debug, Clone)]
pub(super) struct SeparationUse {
    /// The colourant name, element 1, which is what the requirement groups by.
    pub(super) colourant: Vec<u8>,
    /// Element 2, the `alternateSpace`, exactly as the file wrote it.
    pub(super) alternate: Object,
    /// Element 3, the `tintTransform`, exactly as the file wrote it.
    pub(super) transform: Object,
}

/// Every `Separation` array the file's cross-referenced objects state, in object order.
///
/// **The whole file rather than the validator's findings, and that is the one place in this module
/// where it has to be.** A finding names the *later* array of a disagreeing pair; choosing which
/// definition an archive keeps is a choice among all of them, so a population of the findings alone
/// would be a choice among the losers. What is read here is §8.6.6.4's array shape and nothing of
/// ISO 19005 — which requirement failed, and where, stays `pdf_archive`'s.
pub(super) fn separation_uses(document: &Document) -> Vec<SeparationUse> {
    let mut found = Vec::new();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId::new(number, 0));
        collect_separation_uses(document, &object, 0, &mut found);
    }
    found
}

/// One object's contribution to [`separation_uses`].
fn collect_separation_uses(
    document: &Document,
    object: &Object,
    depth: usize,
    found: &mut Vec<SeparationUse>,
) {
    if depth >= MAX_ENTRY_DEPTH {
        return;
    }
    let deeper = depth.saturating_add(1);
    match object {
        Object::Array(items) => {
            if let Some(use_) = as_separation_use(document, items) {
                found.push(use_);
            }
            for item in items {
                collect_separation_uses(document, item, deeper, found);
            }
        }
        Object::Dictionary(dict) => {
            for (_, value) in dict.iter() {
                collect_separation_uses(document, value, deeper, found);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                collect_separation_uses(document, value, deeper, found);
            }
        }
        _ => {}
    }
}

/// An array read as §8.6.6.4's four-element `Separation`, or `None` where it is not one.
fn as_separation_use(document: &Document, items: &[Object]) -> Option<SeparationUse> {
    let colourant = separation_colourant(document, items)?;
    Some(SeparationUse {
        colourant,
        alternate: items.get(2)?.clone(),
        transform: items.get(3)?.clone(),
    })
}

/// Writes the agreed definition into every `Separation` array inside `dict` whose colourant is
/// named, and answers which colourants were reached.
///
/// The descent is [`place_colorants`]', and for the same reason: a colour space written inside a
/// page's resource dictionary is reported at the page, so the object is descended rather than
/// edited at its top level. References are not followed — an array written as its own object is
/// reached as that object, by [`agree_separations_in`].
pub(super) fn agree_separations(
    document: &Document,
    dict: &mut Dictionary,
    agreed: &BTreeMap<Vec<u8>, (Object, Object)>,
    depth: usize,
) -> BTreeSet<Vec<u8>> {
    let mut reached = BTreeSet::new();
    if depth >= MAX_ENTRY_DEPTH {
        return reached;
    }
    let deeper = depth.saturating_add(1);
    let inside: Vec<(Name, Object)> = dict
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    for (key, value) in inside {
        let mut held = value;
        if agree_in_value(document, &mut held, agreed, deeper, &mut reached) {
            dict.insert(key, held);
        }
    }
    reached
}

/// The same, for an object that is the `Separation` array itself.
pub(super) fn agree_separations_in(
    document: &Document,
    value: &mut Object,
    agreed: &BTreeMap<Vec<u8>, (Object, Object)>,
) -> BTreeSet<Vec<u8>> {
    let mut reached = BTreeSet::new();
    agree_in_value(document, value, agreed, 0, &mut reached);
    reached
}

/// One value, and whether agreeing changed it.
fn agree_in_value(
    document: &Document,
    value: &mut Object,
    agreed: &BTreeMap<Vec<u8>, (Object, Object)>,
    depth: usize,
    reached: &mut BTreeSet<Vec<u8>>,
) -> bool {
    match value {
        Object::Dictionary(dict) => {
            let inner = agree_separations(document, dict, agreed, depth);
            let changed = !inner.is_empty();
            reached.extend(inner);
            changed
        }
        Object::Array(items) => {
            if depth >= MAX_ENTRY_DEPTH {
                return false;
            }
            let mut changed = agree_one_separation(document, items, agreed, reached);
            for item in items {
                changed |= agree_in_value(document, item, agreed, depth.saturating_add(1), reached);
            }
            changed
        }
        _ => false,
    }
}

/// One array, given the definition its colourant's uses are to agree on.
///
/// Elements 2 and 3 are replaced together, because §8.6.6.4 makes them one statement: the
/// tintTransform "shall be called with the tint value and shall return the corresponding colour
/// component values", and "the number of components and the interpretation of their values shall
/// depend on the alternate colour space" — so a transform written beside another space is a
/// function whose output nothing reads.
fn agree_one_separation(
    document: &Document,
    items: &mut [Object],
    agreed: &BTreeMap<Vec<u8>, (Object, Object)>,
    reached: &mut BTreeSet<Vec<u8>>,
) -> bool {
    let Some(colourant) = separation_colourant(document, items) else {
        return false;
    };
    let Some((alternate, transform)) = agreed.get(&colourant) else {
        return false;
    };
    let mut changed = false;
    if let Some(slot) = items.get_mut(2)
        && !pdf_archive::same_parameter(document, slot, alternate)
    {
        *slot = alternate.clone();
        changed = true;
    }
    if let Some(slot) = items.get_mut(3)
        && !pdf_archive::same_parameter(document, slot, transform)
    {
        *slot = transform.clone();
        changed = true;
    }
    if changed {
        reached.insert(colourant);
    }
    changed
}

/// Where the interactive form dictionary sits, for the two rewrites that edit it.
///
/// ISO 19005-2 section 6.4.1 and ISO 19005-4 section 6.4.1 ask that Table 224's
/// `/NeedAppearances` be absent or false; ISO 19005-2 section 6.4.2 and ISO 19005-4 section 6.4.2
/// forbid its `/XFA`. §12.7.3 puts both keys in the dictionary the catalog's `/AcroForm` names,
/// which a producer may write as a reference or directly — [`Site`] is that difference, and the
/// rewrites edit whichever of the two the file states.
///
/// A catalog this tree cannot read, or one whose `/AcroForm` resolves to no dictionary, is
/// refused rather than passed over: the requirement failed at a form dictionary the validator
/// read, so a preparation that cannot find one has lost it rather than established there is none.
pub(super) fn interactive_form(document: &Document) -> Result<Site, Because> {
    let Ok(catalog) = document.catalog() else {
        return Err(Because::NotBuiltYet(FORM_NOT_REACHABLE));
    };
    let Some(entry) = catalog.get("AcroForm") else {
        return Err(Because::NotBuiltYet(FORM_NOT_REACHABLE));
    };
    if document.resolve(entry).as_dict().is_none() {
        return Err(Because::NotBuiltYet(FORM_NOT_REACHABLE));
    }
    Ok(Site::of(entry))
}

/// Why a form dictionary this rewrite cannot reach refuses the requirement.
const FORM_NOT_REACHABLE: &str = "this document fails an interactive form requirement whose \
     remedy edits the form dictionary, and the dictionary the catalog's AcroForm names is not one \
     this rewrite can reach — the catalog does not read, the entry is absent, or it resolves to \
     something that is not a dictionary";

/// Whether the document itself says the pages an XFA form shows are generated rather than held.
///
/// **The predicate is the file's own statement, not a reading of the XFA resource.** Annex K is
/// normative and states no test: it says the architecture exists because "the number of fields
/// needs to vary depending upon the quantity of entries", that "\[t]he introduction of dynamically
/// generated pages guided by schemas is a considerable departure from the fixed page model of PDF
/// files", and that "\[w]hether or not the pages generated by the processing of an XFA schema are
/// materialized as PDF pages is implementation dependent" — so the pages in the file settle
/// nothing either. The template that would settle it is XML this program does not parse, and
/// `CLAUDE.md` excludes XFA on the annex's own permission.
///
/// What the base standard does state is §7.7.2's Table 29, of the entry whose subject is this
/// very form:
///
/// > ( Optional; deprecated in PDF 2.0 ) A flag used to expedite the display of PDF documents
/// > containing XFA forms. It specifies whether the document shall be regenerated when the
/// > document is first opened. See Annex K, ' XFA forms ' . Default value: false .
///
/// A document stating it true says its own pages are to be regenerated before anybody reads them,
/// which is the dynamic case exactly; one stating it false or nothing states the default, and
/// what a reader shows is the pages the file holds. So `dynamic = "stop"` stops on the document's
/// own claim rather than on a guess about a resource nobody here reads (`doc/adr/1257`).
///
/// **The honest limit**: a producer whose form is dynamic and who wrote no `/NeedsRendering` has
/// stated the default, and nothing in the file distinguishes them from a static one. This reads
/// what the document says; it does not discover what the document omitted.
pub(super) fn xfa_form_is_dynamic(document: &Document) -> bool {
    document.catalog().is_ok_and(|catalog| {
        matches!(
            document.get_key(&catalog, "NeedsRendering"),
            Object::Boolean(true)
        )
    })
}

/// The file specifications a target's own embedded-file rule does not admit, and what names them.
///
/// ISO 19005-2 section 6.8 requires every embedded file to conform to a part of ISO 19005, and
/// ISO 19005-4 section 6.9 requires the same of a plain PDF/A-4 file. Neither offers anywhere to
/// put one that does not, so the `discard` this population is built for takes the whole file
/// specification out of the document — and **what is removed is every reference to it**, on
/// `doc/adr/1099`'s rule: the walk copies what the converted document reaches, so a specification
/// nothing names is a specification the output does not hold, and the embedded stream under it
/// goes the same way.
#[derive(Debug, Default)]
pub(super) struct EmbeddedFiles {
    /// The specifications themselves, for the report and for the reference removal to match on.
    pub(super) at: BTreeSet<ObjectId>,
    /// The outermost object on the path to each name-tree node whose `/Names` array names one.
    ///
    /// §7.9.6 lets a name tree's nodes be objects or be written directly inside their parent, and
    /// a producer mixes the two freely — so what the rewrite is given is the object it can reach
    /// and a descent through whatever that object states directly.
    pub(super) named_by: BTreeSet<ObjectId>,
    /// Whether the catalog itself is that outermost holder, its `/Names` written directly.
    pub(super) named_in_catalog: bool,
    /// The objects whose `/AF` array names one.
    pub(super) associated_by: BTreeSet<ObjectId>,
    /// Whether the catalog itself states the `/AF` array directly.
    pub(super) associated_in_catalog: bool,
    /// Each removed file's name, for the report to say what left.
    pub(super) names: Vec<String>,
}

/// Every embedded file the requirement failed at, with the entries that reach it.
pub(super) fn embedded_files(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<EmbeddedFiles, Because> {
    let (at, elsewhere) = objects_named(input, &EMBEDDED_FILE_ROWS);
    if elsewhere {
        // A leaf the producer wrote out in place has no object number, and this walk removes
        // references to objects. The name-tree node holding it is reachable, but the *finding*
        // does not say which one, so acting would be this converter guessing which entry the
        // validator meant.
        return Err(Because::NotBuiltYet(EMBEDDED_FILE_WRITTEN_IN_PLACE));
    }
    let mut found = EmbeddedFiles {
        at,
        ..EmbeddedFiles::default()
    };
    if found.at.is_empty() {
        return Ok(found);
    }
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        let object = document.get(id);
        let Some(dict) = object.as_dict() else {
            continue;
        };
        // §12.5.6.15's Table 184 makes a file attachment annotation's `/FS` required, and
        // §12.6.4.4's embedded `GoTo` action names a specification the same way. A specification
        // one of those still reaches survives the removal, and the requirement with it — so the
        // document is refused here rather than half-converted and caught by the output's verdict.
        if dict
            .get("FS")
            .and_then(Object::as_reference)
            .is_some_and(|named| found.at.contains(&named))
        {
            return Err(Because::NotBuiltYet(EMBEDDED_FILE_STILL_REACHED));
        }
        if names_one_of(document, dict, "AF", &found.at) {
            found.associated_by.insert(id);
        }
    }
    if let Ok(catalog) = document.catalog() {
        found.associated_in_catalog = matches!(catalog.get("AF"), Some(Object::Array(_)))
            && names_one_of(document, &catalog, "AF", &found.at);
        if let Some(entry) = catalog.get("Names") {
            let mut holders = BTreeSet::new();
            let mut in_catalog = false;
            holding(
                document,
                entry,
                None,
                0,
                &found.at,
                &mut holders,
                &mut in_catalog,
            );
            found.named_by = holders;
            found.named_in_catalog = in_catalog;
        }
    }
    for id in &found.at {
        let object = document.get(*id);
        let Some(specification) = object.as_dict() else {
            continue;
        };
        found
            .names
            .push(specification_file_name(document, specification));
    }
    found.names.sort_unstable();
    found.names.dedup();
    Ok(found)
}

/// Records the outermost object holding each name-tree node whose `/Names` names a removed file.
///
/// §7.9.6's tree is `/Kids` down to leaves stating `/Names`, and each node may be an object or be
/// written directly inside its parent. `owner` is the nearest object on the way down, which is
/// what the rewrite can reach; `None` means nothing but the catalog has been crossed, and the
/// removal is then the catalog's own.
fn holding(
    document: &Document,
    entry: &Object,
    owner: Option<ObjectId>,
    depth: usize,
    removed: &BTreeSet<ObjectId>,
    holders: &mut BTreeSet<ObjectId>,
    in_catalog: &mut bool,
) {
    if depth >= MAX_NAME_TREE_DEPTH {
        return;
    }
    let owner = entry.as_reference().or(owner);
    let resolved = document.resolve(entry);
    let Some(dict) = resolved.as_dict() else {
        return;
    };
    if names_one_of(document, dict, "Names", removed) {
        match owner {
            Some(id) => {
                holders.insert(id);
            }
            None => *in_catalog = true,
        }
    }
    // The name dictionary's own entry first, then the tree's internal nodes: one walk serves
    // both, because each is a dictionary whose children are reached by an entry of its own.
    for key in ["EmbeddedFiles", "Kids"] {
        match dict.get(key) {
            Some(Object::Array(kids)) => {
                for kid in kids {
                    holding(
                        document,
                        kid,
                        owner,
                        depth.saturating_add(1),
                        removed,
                        holders,
                        in_catalog,
                    );
                }
            }
            Some(entry) => holding(
                document,
                entry,
                owner,
                depth.saturating_add(1),
                removed,
                holders,
                in_catalog,
            ),
            None => {}
        }
    }
}

/// How deep a name tree this walk follows, so a producer's cycle ends.
pub(super) const MAX_NAME_TREE_DEPTH: usize = 64;

/// Whether a dictionary's array-valued entry references one of the removed specifications.
fn names_one_of(
    document: &Document,
    dict: &Dictionary,
    key: &str,
    removed: &BTreeSet<ObjectId>,
) -> bool {
    document.get_key(dict, key).as_array().is_some_and(|items| {
        items
            .iter()
            .filter_map(Object::as_reference)
            .any(|id| removed.contains(&id))
    })
}

/// What the report calls one removed file: §7.11.3's `/UF`, else its `/F`, else the words for one.
fn specification_file_name(document: &Document, specification: &Dictionary) -> String {
    for key in ["UF", "F"] {
        if let Some(bytes) = document.get_key(specification, key).as_string() {
            return pdf_syntax::text_string(bytes);
        }
    }
    "an embedded file whose specification states no name".to_owned()
}

/// The two rows one removal answers, one per part.
const EMBEDDED_FILE_ROWS: [&str; 2] = [
    "embedded-files/embedded-file-is-itself-pdfa",
    "embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile",
];

/// Why an embedded file whose specification a name tree holds in place is not removed.
const EMBEDDED_FILE_WRITTEN_IN_PLACE: &str = "this document writes a file specification directly \
     inside its EmbeddedFiles name tree rather than as an object of its own, and this verb \
     removes references to objects — so the entry the validator reported has no object number \
     for the removal to match on, and taking a guess at which name-tree entry it was would be \
     this converter deciding what the report meant";

/// Why an embedded file something else in the document still reaches is not removed.
const EMBEDDED_FILE_STILL_REACHED: &str = "an embedded file this target does not admit is named \
     by a file attachment annotation's FS entry or by an embedded GoTo action, which ISO 32000-2 \
     \u{a7}12.5.6.15's Table 184 makes required — so taking the specification out of the name \
     tree would leave it in the file, reached from a page, and the requirement failing. Removing \
     that annotation is a different loss, over the marks it draws, and it is not this one";
