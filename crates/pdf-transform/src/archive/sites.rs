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
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::decision::Because;
use super::prepare::Spare;

/// The objects one rewrite touches.
///
/// A set rather than a list: a requirement can be reported at one object twice — `/CharSet` and
/// `/CIDSet` in one descriptor, two findings for one annotation — and the rewrite is applied to
/// the object once whatever the findings said.
#[derive(Debug, Default)]
pub(super) struct Sites {
    /// The objects to rewrite, in the source's numbering.
    pub(super) at: BTreeSet<ObjectId>,
}

/// Every object one requirement's findings name, and whether any place named no object.
///
/// The second half is what separates the two shapes of three of these requirements: a finding
/// whose place is a page rather than an object is inside a content stream, and this verb carries
/// those byte for byte.
fn objects_named(input: &pdf_archive::Report, requirements: &[&str]) -> (BTreeSet<ObjectId>, bool) {
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
pub(super) fn rendering_intents(input: &pdf_archive::Report) -> Result<Sites, Because> {
    let (at, in_content) = objects_named(
        input,
        &["graphics/rendering-intent-entries-name-one-of-four"],
    );
    if in_content {
        return Err(Because::TheFence(INLINE_IMAGE_INTENT));
    }
    Ok(Sites { at })
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
    Ok(Sites { at })
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
     one is doc/questions/A48's forbidden half. The other shape this requirement fails in — an \
     appearance dictionary stating no N entry at all, or a button widget whose N is a single \
     stream — has no answer either";

/// The appearances an annotation's own `/AS` entry selects.
///
/// ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3.
#[derive(Debug, Default)]
pub(super) struct AppearanceStates {
    /// The stream each annotation's `/AP` `/N` is to name, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, ObjectId>,
    /// The streams this conversion copies out of a subdictionary that held them directly.
    pub(super) written: Vec<(ObjectId, Object)>,
}

/// Every annotation whose normal appearance is to be collapsed to the state it selects.
pub(super) fn appearance_states(
    document: &Document,
    input: &pdf_archive::Report,
    spare: &mut Spare,
) -> Result<AppearanceStates, Because> {
    let (named, elsewhere) = objects_named(input, &["annotations/normal-appearance-shape"]);
    if elsewhere {
        return Err(Because::NotBuiltYet(NO_APPEARANCE_STATE));
    }
    let mut found = AppearanceStates::default();
    for id in named {
        let resolved = document.get(id);
        let annotation =
            dictionary_of(&resolved).ok_or(Because::NotBuiltYet(NO_APPEARANCE_STATE))?;
        let states = document.get_key(annotation, "AP");
        let states = states
            .as_dict()
            .map(|appearance| document.get_key(appearance, "N"))
            .ok_or(Because::NotBuiltYet(NO_APPEARANCE_STATE))?;
        let states = states
            .as_dict()
            .ok_or(Because::NotBuiltYet(NO_APPEARANCE_STATE))?;
        // §7.3.5 resolves a name by its bytes, so the lookup is byte-exact: an `/AS` whose name
        // is not valid UTF-8 selects the entry spelled with those same bytes and nothing else.
        let state = document
            .get_key(annotation, "AS")
            .as_name()
            .ok_or(Because::NotBuiltYet(NO_APPEARANCE_STATE))?
            .clone();
        let selected = states
            .get_by_name(&state)
            .ok_or(Because::NotBuiltYet(NO_APPEARANCE_STATE))?
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
                    .ok_or(Because::NotBuiltYet(NO_APPEARANCE_STATE))?;
                found.written.push((at, selected.clone()));
                at
            }
            _ => return Err(Because::NotBuiltYet(NO_APPEARANCE_STATE)),
        };
        found.at.insert(id, stream);
    }
    Ok(found)
}

/// Why a form `XObject` with no resources dictionary is not given one.
const RESOURCES_ARE_THE_INVOCATION_S: &str = "a content stream here names resources of its own \
     and carries no Resources entry, and it is not a page. For a page the entry is the one \
     §7.7.3.4's inheritance already put in force, so copying it down resolves every name to the \
     object it already resolved to — that is written. For a form XObject, a tiling pattern or a \
     Type 3 glyph procedure the names resolve through whatever stream invoked it, and one \
     dictionary cannot answer for every invocation: a stream invoked from two pages whose \
     resources define the same name differently has no single correct answer, and writing one \
     would decide what it draws";

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
pub(super) fn page_resources(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<PageResources, Because> {
    let mut pages: BTreeSet<usize> = BTreeSet::new();
    for judgement in input.failures() {
        if judgement.id != "graphics/content-streams-have-an-explicit-resources-dictionary" {
            continue;
        }
        let Outcome::Failed { places, .. } = &judgement.outcome else {
            continue;
        };
        for finding in places {
            // An object is a form `XObject`, a tiling pattern or a Type 3 glyph procedure; only
            // a page's own `/Contents` is reported at a page with no object beside it.
            if finding.place.object.is_some() {
                return Err(Because::TheFence(RESOURCES_ARE_THE_INVOCATION_S));
            }
            let Some(page) = finding.place.page else {
                return Err(Because::TheFence(RESOURCES_ARE_THE_INVOCATION_S));
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
    Ok(found)
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
        let Outcome::Failed { places, .. } = &judgement.outcome else {
            continue;
        };
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

/// Why a `/CIDToGIDMap` is not written at a part 4 target, and why a stated one is not replaced.
const CID_TO_GID_MAP_NOT_DERIVED: &str = "an embedded Type 2 CIDFont here states no \
     CIDToGIDMap, or states one that is neither a stream nor the name Identity, and which of \
     those two it is decides the answer together with the target. ISO 19005-2 section 5.1 makes \
     a PDF/A-2 file one that adheres to ISO 32000-1, whose own table gives Identity as this \
     entry's default — so at a part 2 target an *absent* entry is written as Identity, which \
     restates what a reader of that edition already applies. ISO 32000-2's Table 121 requires \
     the entry and states no default, so the same write at a part 4 target would assert a \
     mapping its base document does not supply, and deriving the map from the embedded program \
     is a reading of the font this converter does not make. An entry that is *there* and is \
     neither shape is the producer's statement, and replacing it would change which glyph every \
     code selects";

/// Every embedded Type 2 `CIDFont` that is to be given `/CIDToGIDMap` `/Identity`.
///
/// ISO 19005-2 section 6.2.11.3.2. **Part 2 targets only**, and only where the entry is absent
/// altogether: both halves of that are in [`CID_TO_GID_MAP_NOT_DERIVED`].
pub(super) fn cid_to_gid_maps(
    document: &Document,
    input: &pdf_archive::Report,
    target: Target,
) -> Result<Sites, Because> {
    if target.part() != Part::Two {
        return Err(Because::NotBuiltYet(CID_TO_GID_MAP_NOT_DERIVED));
    }
    let (named, elsewhere) = objects_named(input, &["fonts/cid-to-gid-map-present"]);
    if elsewhere {
        return Err(Because::NotBuiltYet(CID_TO_GID_MAP_NOT_DERIVED));
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
    Ok(Sites { at })
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
    Ok(Sites { at })
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
fn loaded_as_stated(document: &Document, dict: &Dictionary) -> Option<LoadedFont> {
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
    Ok(Sites { at })
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
