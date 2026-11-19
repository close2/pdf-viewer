//! The pages that hold a fault of their content stream the target refuses, found so that
//! `as-if-printed` can photograph them (`doc/adr/2516`, owner answer A694 item 1).
//!
//! # Why a page, and which pages
//!
//! ADR 0816's fence is that the converter never edits a producer's content stream, so a rule a
//! content stream fails (an operator the base standard does not define, a hexadecimal string with a
//! byte that is no digit, an inline image's `/I`) has no repair on the page. What a company does
//! with such a letter is print it and scan the print: the page survives as the marks the window
//! draws, and the fault is not in them. The page is photographed by `fonts::photographed`
//! and the requirement stops being failed because the content stream that failed it is no longer
//! in the output.
//!
//! This module only says **which pages**. A requirement is covered when every place the validator
//! kept for it names a page and the list is whole; one that reports an object, or a list past the
//! validator's bound, is left to its refusal, because the page that reaches the object is not
//! known from the report. The output is held to the target afterwards (`super::hold_the_output_to_the_target`),
//! so a page missed here makes the file refused rather than wrong.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::Report;
use pdf_archive::report::Outcome;
use pdf_archive::survey::Survey;
use pdf_model::Pages;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Object, ObjectId};

/// The operator row, whose pages the survey names exactly.
pub(super) const OPERATORS: &str = "graphics/only-operators-the-base-standard-defines";

/// The rows a page's content stream fails and that nothing repairs on the page, besides
/// [`OPERATORS`] (ISO 19005-2 section 6.2.2, ISO 19005-4 section 6.2.2): an inline image's
/// interpolation, a hexadecimal string holding a byte that is no digit, a glyph procedure whose
/// width the procedure contradicts, an `/ActualText` holding a Private Use character, a bound
/// name that is not UTF-8 (ISO 19005-2 section 6.1.8), and a transfer function other than the
/// identity, stated by a graphics state's `/TR` or `/TR2` or by a halftone (ISO 19005-2 section
/// 6.2.5, ISO 19005-4 section 6.2.5).
pub(super) const ROWS: [&str; 9] = [
    OPERATORS,
    "graphics/inline-image-interpolation-is-off",
    "file-structure/hexadecimal-string-holds-only-digits",
    "fonts/type3-glyph-procedures-state-their-width",
    "fonts/actual-text-states-no-private-use",
    "file-structure/bound-names-are-valid-utf8",
    TRANSFER,
    SECOND_TRANSFER,
    HALFTONE_TRANSFER,
];

/// What the failed fenced rows hold on the document's pages.
#[derive(Debug, Default)]
pub(super) struct Fenced {
    /// The pages (zero-based) each failed row holds a fault on, keyed by page.
    pub(super) pages: BTreeMap<usize, BTreeSet<&'static str>>,
    /// The spellings of the operators no standard defines that the content streams used: the one
    /// kind of operator the interpreter reports and the photograph may still be taken over, since
    /// no reader draws anything for it.
    pub(super) operators: BTreeSet<String>,
}

/// The pages each failed fenced row holds a fault on.
///
/// A row whose pages are not all known is absent from every value, so the requirement keeps the
/// refusal it had. Where `drawn` is given, `super::as_drawn` wrote the number this reader draws
/// in place of every run it salvages in the streams it could read, so an operator that is such a
/// run, in such a stream, holds no page: its page converts as text (`doc/adr/2638`). A run in a
/// stream it could not read is still in the output, and its page is photographed (ADR 2803).
///
/// **An operator in a stream an annotation's appearance reaches holds no page either**, and its
/// row keeps the refusal (ADR 2803): a photograph replaces the page's `/Contents` and
/// `/Resources` and leaves its `/Annots`, so ISO 32000-2 §12.5.5's appearance stream, and every
/// form, pattern, glyph procedure and soft mask group it names, is still in the output with the
/// operator in it.
pub(super) fn pages(
    document: &Document,
    input: &Report,
    survey: &Survey,
    drawn: Option<&super::as_drawn::Drawn>,
    type3_restated: bool,
) -> Fenced {
    let mut found = Fenced::default();
    let mut annotations_reach: Option<BTreeSet<ObjectId>> = None;
    for judgement in input.failures() {
        let Some(row) = ROWS.iter().copied().find(|row| *row == judgement.id) else {
            continue;
        };
        let pages: BTreeSet<usize> = if row == OPERATORS {
            let unwritten = || {
                survey.unlisted_operators().iter().filter(|unlisted| {
                    !drawn.is_some_and(|drawn| {
                        drawn.answers(&unlisted.spelling) && !drawn.left(document, unlisted)
                    })
                })
            };
            let reached =
                annotations_reach.get_or_insert_with(|| reached_from_annotations(document));
            if unwritten().any(|unlisted| {
                unlisted
                    .place
                    .object
                    .is_some_and(|object| reached.contains(&object))
            }) {
                continue;
            }
            for unlisted in unwritten() {
                found
                    .operators
                    .insert(String::from_utf8_lossy(&unlisted.spelling).into_owned());
            }
            unwritten().map(|unlisted| unlisted.page).collect()
        } else {
            let Outcome::Failed { places, total } = &judgement.outcome else {
                continue;
            };
            if let Some(applies) = transfer_function_of(row) {
                let reached =
                    annotations_reach.get_or_insert_with(|| reached_from_annotations(document));
                match transfer_pages(document, judgement, reached, applies) {
                    Some(pages) => pages,
                    None => continue,
                }
            } else if row == BOUND_NAMES {
                let reached =
                    annotations_reach.get_or_insert_with(|| reached_from_annotations(document));
                match bound_name_pages(document, judgement, reached) {
                    Some(pages) => pages,
                    None => continue,
                }
            } else if row == TYPE3_WIDTHS {
                // Restated in the procedures where the profile granted that loss, which keeps the
                // pages' text and takes their place (`super::type3_widths`, `doc/adr/2907`).
                if type3_restated {
                    continue;
                }
                let reached =
                    annotations_reach.get_or_insert_with(|| reached_from_annotations(document));
                match type3_pages(document, judgement, reached) {
                    Some(pages) => pages,
                    None => continue,
                }
            } else {
                let named: Option<BTreeSet<usize>> =
                    places.iter().map(|finding| finding.place.page).collect();
                match named {
                    Some(named) if places.len() == *total => named,
                    _ => continue,
                }
            }
        };
        for page in pages {
            found.pages.entry(page).or_default().insert(row);
        }
    }
    found
}

/// The row whose places name a font, a `CIDFont` or a font descriptor where the name a font is
/// bound by is not UTF-8 (ISO 19005-2 section 6.1.8).
const BOUND_NAMES: &str = "file-structure/bound-names-are-valid-utf8";

/// The pages a bound-name row's faults are on, or `None` where they are not all known.
///
/// A name inside a content stream is reported at its page. A font's name is reported at the
/// object stating it — the font, its `CIDFont` or its descriptor — and where the file embeds the
/// program the name is that program's, so renaming it would make the dictionary name a font the
/// program does not (`doc/pdf-a-conversion-limits.md` section 4.7). A photograph of every page
/// whose resources reach the font takes the font out of the output with those resources, the way
/// a print carries the glyphs and not their names (`doc/adr/3127`). Only a font's parts are
/// answered so: a colourant or a structure type is named from places a page's photograph does
/// not take. A font an annotation's appearance reaches keeps the refusal (ADR 2803).
fn bound_name_pages(
    document: &Document,
    judgement: &pdf_archive::Judgement,
    annotations_reach: &BTreeSet<ObjectId>,
) -> Option<BTreeSet<usize>> {
    let Outcome::Failed { places, total } = &judgement.outcome else {
        return None;
    };
    if places.len() != *total {
        return None;
    }
    let mut pages = BTreeSet::new();
    let mut holders = BTreeSet::new();
    for finding in places {
        match (finding.place.object, finding.place.page) {
            (Some(holder), _) if is_a_font_part(document, holder) => {
                holders.insert(holder);
            }
            (None, Some(page)) => {
                pages.insert(page);
            }
            _ => return None,
        }
    }
    if holders.is_empty() {
        return Some(pages);
    }
    if with_font_parts(document, annotations_reach.clone())
        .intersection(&holders)
        .next()
        .is_some()
    {
        return None;
    }
    let mut unreached = holders.clone();
    for (index, page) in Pages::new(document).every().into_iter().enumerate() {
        let Some(page) = page else {
            continue;
        };
        let mut queue = Vec::new();
        resource_entries(document, &page.resources, &mut queue);
        let reached = with_font_parts(document, reached_from(document, queue));
        if holders.iter().any(|holder| reached.contains(holder)) {
            pages.insert(index);
            unreached.retain(|holder| !reached.contains(holder));
        }
    }
    unreached.is_empty().then_some(pages)
}

/// Whether `object` is a font dictionary or a font descriptor, Table 109's and Table 120's `/Type`.
fn is_a_font_part(document: &Document, object: ObjectId) -> bool {
    document.get(object).as_dict().is_some_and(|dict| {
        document
            .get_key(dict, "Type")
            .as_name()
            .is_some_and(|name| matches!(name.as_bytes(), b"Font" | b"FontDescriptor"))
    })
}

/// `reached` with each font's parts: a Type 0 font's descendant `CIDFont` (§9.7.6.1's
/// `/DescendantFonts`) and every font's descriptor, which the resource walk does not follow.
fn with_font_parts(document: &Document, mut reached: BTreeSet<ObjectId>) -> BTreeSet<ObjectId> {
    let mut fonts: Vec<ObjectId> = reached.iter().copied().collect();
    while let Some(font) = fonts.pop() {
        let Some(dict) = document.get(font).as_dict().cloned() else {
            continue;
        };
        let descendants = document.get_key(&dict, "DescendantFonts");
        let mut parts: Vec<Object> = document
            .resolve(&descendants)
            .as_array()
            .map_or_default(<[Object]>::to_vec);
        parts.extend(dict.get("FontDescriptor").cloned());
        for part in parts {
            if let Some(id) = part.as_reference()
                && reached.insert(id)
            {
                fonts.push(id);
            }
        }
    }
    reached
}

/// The row whose places name a Type 3 font rather than a page (ISO 19005-4 section 6.2.10.5).
const TYPE3_WIDTHS: &str = "fonts/type3-glyph-procedures-state-their-width";

/// The pages a Type 3 width row's faults are on, or `None` where they are not all known.
///
/// ISO 19005-4 section 6.2.10.5's glyph procedure width is reported at its font, which no page
/// holds, so the pages are the ones whose resources reach the font; the validator keeps every
/// object it named past the places it lists (`pdf_archive::Named`), and a font written directly
/// in a resource dictionary is reported at its page. A font an annotation's appearance reaches
/// keeps the refusal, as an operator there does, because a photograph leaves `/Annots` (ADR
/// 2803, ADR 2847).
fn type3_pages(
    document: &Document,
    judgement: &pdf_archive::Judgement,
    annotations_reach: &BTreeSet<ObjectId>,
) -> Option<BTreeSet<usize>> {
    let Outcome::Failed { places, total } = &judgement.outcome else {
        return None;
    };
    let named = &judgement.named;
    if named.cut_short() || (named.elsewhere() && places.len() != *total) {
        return None;
    }
    let mut pages: BTreeSet<usize> = places
        .iter()
        .filter(|finding| finding.place.object.is_none())
        .map(|finding| finding.place.page)
        .collect::<Option<_>>()?;
    for &font in named.objects() {
        let reaching = pages_reaching(document, font);
        if annotations_reach.contains(&font) || reaching.is_empty() {
            return None;
        }
        pages.extend(reaching);
    }
    Some(pages)
}

/// The row whose places name a graphics state stating `/TR` (ISO 19005-2 section 6.2.5, ISO
/// 19005-4 section 6.2.5).
const TRANSFER: &str = "graphics/no-transfer-function-in-a-graphics-state";

/// The row whose places name a graphics state stating a `/TR2` other than `Default` (the same
/// sections).
const SECOND_TRANSFER: &str = "graphics/second-transfer-function-is-default";

/// The row whose places name the graphics state (`HT`) or the type 5 halftone (a colourant's
/// name) whose halftone states a `TransferFunction` where ISO 32000-2 §10.6.5.6 requires none, or
/// omits one it requires (ISO 19005-2 section 6.2.5; ISO 19005-4 section 6.2.5 as its erratum
/// leaves it).
const HALFTONE_TRANSFER: &str = "graphics/halftone-transfer-function-only-where-required";

/// Whether one finding of a transfer row names a function this reader applies (`Some(true)`), the
/// name `Identity` (`Some(false)`), or a place the photograph cannot answer (`None`).
type Applies = fn(&Document, &pdf_archive::Finding) -> Option<bool>;

/// The reading of a transfer row's findings, where `row` is one.
fn transfer_function_of(row: &str) -> Option<Applies> {
    match row {
        TRANSFER => Some(|document, finding| state_applies(document, finding, "TR")),
        SECOND_TRANSFER => Some(|document, finding| state_applies(document, finding, "TR2")),
        HALFTONE_TRANSFER => Some(halftone_applies),
        _ => None,
    }
}

/// Whether a graphics state's `key` is a function other than the identity.
///
/// ISO 32000-2 §8.4.5's Table 57: for `TR`, "\[t]he name Identity may be used to represent the
/// Identity function", and `TR2` is "\[s]ame as TR except that the value may also be the name
/// Default"; the validator reports no `Default`, so every other value is a function.
fn state_applies(document: &Document, finding: &pdf_archive::Finding, key: &str) -> Option<bool> {
    let state = finding.place.object?;
    let identity = document
        .get(state)
        .as_dict()
        .and_then(|dict| dict.get(key))
        .and_then(Object::as_name)
        .is_some_and(|name| name.as_bytes() == b"Identity");
    Some(!identity)
}

/// Whether the halftone a finding names states a function other than the identity, or omits the
/// one it is required to state.
///
/// The finding names the object holding the halftone and the key it is held under: `HT` on a
/// graphics state, a colourant's name on a type 5 halftone. ISO 32000-2 §10.5's second bullet has
/// the halftone's function "override the corresponding one specified by the current transfer
/// function parameter in the graphics state", so a function there is applied as a state's is. A
/// component omitting the function §10.6.5.6 requires of it is drawn as this reader draws it, and
/// the photograph is that drawing, so it is answered as a function is: nothing in the file says
/// what the omitted function would be, and the photograph claims nothing about it.
fn halftone_applies(document: &Document, finding: &pdf_archive::Finding) -> Option<bool> {
    let holder = document.get(finding.place.object?);
    let key = finding.place.name.as_deref()?;
    let halftone = document.resolve(holder.as_dict()?.get(key)?);
    let dict = match &halftone {
        Object::Dictionary(dict) => dict,
        Object::Stream(stream) => &stream.dict,
        _ => return None,
    };
    let identity = dict
        .get("TransferFunction")
        .and_then(Object::as_name)
        .is_some_and(|name| name.as_bytes() == b"Identity");
    Some(!identity)
}

/// The pages a transfer row's graphics states or halftones are reached from, or `None` where the
/// photograph does not answer the row.
///
/// ISO 32000-2 §10.5 has every processor apply the function — "the PDF processor shall apply the
/// transfer function after performing any needed conversions between colour spaces" — so a print
/// carries it, and the page this reader draws with it applied is what a print scanned back in
/// shows (`doc/adr/1964` section 3, `doc/adr/2889`, `doc/adr/2893`). The pages are the ones whose
/// resources reach an object stating a function other than the identity, as `reads` reads it.
/// Where every one names `Identity` the removal moves no colour and answers the row itself, so
/// nothing is photographed; an `Identity` any other page reaches would be left in the output by a
/// photograph of these, so the row keeps that answer's refusal. An object an annotation's
/// appearance reaches keeps it too, as a Type 3 font does (ADR 2847), and so does a place that
/// names no object.
fn transfer_pages(
    document: &Document,
    judgement: &pdf_archive::Judgement,
    annotations_reach: &BTreeSet<ObjectId>,
    reads: Applies,
) -> Option<BTreeSet<usize>> {
    let Outcome::Failed { places, total } = &judgement.outcome else {
        return None;
    };
    if places.len() != *total {
        return None;
    }
    let mut functions: BTreeSet<ObjectId> = BTreeSet::new();
    let mut identity: BTreeSet<ObjectId> = BTreeSet::new();
    for finding in places {
        let holder = finding.place.object?;
        if reads(document, finding)? {
            functions.insert(holder);
        } else {
            identity.insert(holder);
        }
    }
    if functions.is_empty() {
        return None;
    }
    let mut pages = BTreeSet::new();
    for holder in &functions {
        let reaching = pages_reaching(document, *holder);
        if annotations_reach.contains(holder) || reaching.is_empty() {
            return None;
        }
        pages.extend(reaching);
    }
    for holder in &identity {
        if annotations_reach.contains(holder)
            || !pages_reaching(document, *holder).is_subset(&pages)
        {
            return None;
        }
    }
    Some(pages)
}

/// The pages (zero-based) whose resources reach `object`, through every form `XObject`, pattern,
/// font and soft mask group below them: the pages a photograph has to take for the object to
/// leave the output with their resources.
fn pages_reaching(document: &Document, object: ObjectId) -> BTreeSet<usize> {
    Pages::new(document)
        .every()
        .into_iter()
        .enumerate()
        .filter_map(|(index, page)| {
            let page = page?;
            let mut queue = Vec::new();
            resource_entries(document, &page.resources, &mut queue);
            reached_from(document, queue)
                .contains(&object)
                .then_some(index)
        })
        .collect()
}

/// How many objects [`reached_from_annotations`] visits before it stops; past it the walk keeps
/// what it found, and an operator in a stream it did not reach is photographed over and still
/// held to the target afterwards, which refuses a file the photograph left failing.
const MOST_REACHED: usize = 1 << 16;

/// Every stream an annotation's appearance reaches: the appearance streams themselves (§12.5.5's
/// Table 168) and, through each one's `/Resources`, every form `XObject`, pattern, Type 3 glyph
/// procedure and soft mask group below them.
fn reached_from_annotations(document: &Document) -> BTreeSet<ObjectId> {
    let mut queue: Vec<Object> = Vec::new();
    for page in Pages::new(document).every().into_iter().flatten() {
        queue.extend(super::rewrite::appearance_streams(document, &page.dict));
    }
    reached_from(document, queue)
}

/// Every object `queue`'s references reach through `/Resources`, glyph procedures and soft mask
/// groups, bounded at [`MOST_REACHED`].
fn reached_from(document: &Document, mut queue: Vec<Object>) -> BTreeSet<ObjectId> {
    let mut reached = BTreeSet::new();
    while let Some(next) = queue.pop() {
        if reached.len() >= MOST_REACHED {
            break;
        }
        let Some(id) = next.as_reference() else {
            continue;
        };
        if !reached.insert(id) {
            continue;
        }
        let object = document.get(id);
        let dict = match &object {
            Object::Stream(stream) => &stream.dict,
            Object::Dictionary(dict) => dict,
            _ => continue,
        };
        // A type 5 halftone's components (ISO 32000-2 §10.6.5.6's Table 132), where this object
        // is one: the halftone row names the type 5 halftone, and its components are below it.
        if document.get_key(dict, "HalftoneType").as_integer() == Some(5) {
            queue.extend(dict.iter().map(|(_, component)| component.clone()));
        }
        // A Type 3 font's glyph procedures, where this object is one.
        if let Some(procedures) = document.get_key(dict, "CharProcs").as_dict() {
            queue.extend(procedures.iter().map(|(_, procedure)| procedure.clone()));
        }
        let resources = document.get_key(dict, "Resources");
        if let Some(resources) = resources.as_dict() {
            resource_entries(document, resources, &mut queue);
        }
    }
    reached
}

/// The entries of one resource dictionary the walk follows: its forms, patterns and fonts, and the
/// soft mask groups its graphics states name.
fn resource_entries(document: &Document, resources: &Dictionary, queue: &mut Vec<Object>) {
    for category in ["XObject", "Pattern", "Font"] {
        if let Some(entries) = document.get_key(resources, category).as_dict() {
            queue.extend(entries.iter().map(|(_, entry)| entry.clone()));
        }
    }
    if let Some(states) = document.get_key(resources, "ExtGState").as_dict() {
        // The states themselves, for the transfer rows that name them, and each state's
        // halftone (Table 57's `HT`), for the halftone row that names a type 5 halftone.
        queue.extend(states.iter().map(|(_, state)| state.clone()));
        for (_, state) in states.iter() {
            let state = document.resolve(state);
            queue.extend(state.as_dict().and_then(|state| state.get("HT").cloned()));
            let group = state
                .as_dict()
                .map(|state| document.get_key(state, "SMask"))
                .and_then(|mask| mask.as_dict().and_then(|mask| mask.get("G").cloned()));
            queue.extend(group);
        }
    }
}
