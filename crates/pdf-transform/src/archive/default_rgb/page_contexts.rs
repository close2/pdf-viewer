//! A closed page/Form footprint for a part-4 page output-intent override.
//!
//! ISO 19005-4 section 6.2.3 makes a page's PDF/A output intent the current one on that page,
//! and its section 6.2.4.3 admits `DeviceRGB` where a device-independent `DefaultRGB` is set,
//! the remapping ISO 32000-2 §8.6.5.6 defines:
//!
//! > When a device colour space is selected, the ColorSpace subdictionary of the current
//! > resource dictionary (see 7.8.3, "Resource dictionaries") is checked for the presence of an
//! > entry designating a corresponding default colour space
//!
//! The writer places that default in every resource dictionary, so every resource context is
//! proved first, including Forms the survey deduplicates. Unsupported consumers remain refused
//! (ADR 2230).
use super::super::{decision::Because, prepare::SRGB};
use pdf_archive::survey::{DeviceFamily, Route, SpaceKind, Survey};
use pdf_syntax::{
    Document,
    object::{Dictionary, Object, ObjectId},
};
use std::collections::BTreeSet;

const CONTEXT: Because = Because::NotBuiltYet(
    "the page-level output intent and all resource contexts cannot be proved safe for the new DefaultRGB route",
);
const LIMIT: usize = 65_536;

/// The pages whose current intent has another family; indexes agree with Survey.
#[derive(Debug)]
pub(in crate::archive) struct PageContexts {
    non_rgb: BTreeSet<usize>,
    pages: usize,
}

impl PageContexts {
    /// Caller admits only part 4 with an actually prepared shipped sRGB profile.
    pub(super) fn of(document: &Document) -> Result<Option<Self>, Because> {
        let pages = pdf_model::Pages::new(document).every();
        let mut non_rgb = BTreeSet::new();
        for (index, page) in pages.iter().enumerate() {
            let page = page.as_ref().ok_or(CONTEXT)?;
            if page_intent(document, &page.dict)? {
                non_rgb.insert(index);
            }
        }
        if non_rgb.is_empty() {
            return Ok(None);
        }
        let catalog = document.catalog().map_err(|_| CONTEXT)?;
        if catalog.get("OCProperties").is_some() {
            return Err(CONTEXT);
        }
        let mut walked = 0_usize;
        let mut remaining = 1_048_576_usize;
        for page in &pages {
            let page = page.as_ref().ok_or(CONTEXT)?;
            if let Some(annotations) = page.dict.get("Annots") {
                let resolved = document.resolve(annotations);
                if !matches!(resolved.as_array(), Some(items) if items.is_empty()) {
                    return Err(CONTEXT);
                }
            }
            // The writer uses this same inherited value. Independently prove the
            // parent chain first: its permissive fallback is not proof of absence.
            page_group(document, &page.dict)?;
            let (bytes, issues) = page.content_with_report(document);
            if !issues.is_empty() {
                return Err(CONTEXT);
            }
            content(&bytes, &mut remaining)?;
            let resources = resources_of_page(document, &page.dict)?;
            footprint(document, resources, &mut walked, &mut remaining)?;
        }
        Ok(Some(Self {
            non_rgb,
            pages: pages.len(),
        }))
    }

    pub(super) fn selected(&self, survey: &Survey) -> Result<(), Because> {
        if survey.stopped_short()
            || !survey.missing_resources().is_empty()
            || !survey.unlisted_operators().is_empty()
            || !survey.inline_images().is_empty()
        {
            return Err(CONTEXT);
        }
        let mut needed = false;
        for group in survey.group_spaces() {
            if group.page >= self.pages
                || (group.kind == SpaceKind::Device(DeviceFamily::Rgb)
                    && group.default.in_force.is_none())
            {
                return Err(CONTEXT);
            }
        }
        for colour in survey.device_colours() {
            if colour.page >= self.pages {
                return Err(CONTEXT);
            }
            if colour.family != DeviceFamily::Rgb {
                continue;
            }
            // An inserted explicit default must not shadow a different fallback.
            if colour.default.in_force.is_some() && colour.default.explicit.is_none() {
                return Err(CONTEXT);
            }
            if colour.default.in_force.is_some() {
                continue;
            }
            if colour.via != Route::Direct
                || !matches!(
                    colour.blending,
                    None | Some(SpaceKind::Independent(Some(
                        DeviceFamily::Gray | DeviceFamily::Cmyk
                    )))
                )
            {
                return Err(CONTEXT);
            }
            needed |= self.non_rgb.contains(&colour.page);
        }
        if needed { Ok(()) } else { Err(CONTEXT) }
    }
}

/// No page intent uses the shipped catalog fallback. A present array is proved in full.
///
/// The array is ISO 32000-2 Table 31's page `/OutputIntents` and each entry §14.11.5's Table
/// 401 dictionary. ISO 19005-4 section 6.2.3 is read entry by entry: no `/DestOutputProfileRef`, a
/// `GTS_PDFA1` entry with a `/DestOutputProfile`, every entry naming one indirect profile, and
/// that profile a `prtr` or `mntr` class in `GRAY`, `RGB ` or `CMYK`.
fn page_intent(document: &Document, page: &Dictionary) -> Result<bool, Because> {
    let Some(value) = page.get("OutputIntents") else {
        return Ok(false);
    };
    let resolved = document.resolve(value);
    let entries = resolved.as_array().ok_or(CONTEXT)?;
    let mut destination = None;
    let mut pdfa = false;
    for entry in entries {
        let entry = document.resolve(entry);
        let dict = entry.as_dict().ok_or(CONTEXT)?;
        if dict.get("DestOutputProfileRef").is_some() {
            return Err(CONTEXT);
        }
        let current = dict.get("DestOutputProfile").and_then(Object::as_reference);
        if dict.get("DestOutputProfile").is_some() && current.is_none() {
            return Err(CONTEXT);
        }
        if let Some(current) = current {
            if destination.is_some_and(|held| held != current) {
                return Err(CONTEXT);
            }
            destination = Some(current);
        }
        if document
            .get_key(dict, "S")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"GTS_PDFA1")
        {
            if current.is_none() {
                return Err(CONTEXT);
            }
            pdfa = true;
        }
    }
    if !pdfa {
        return Err(CONTEXT);
    }
    let Object::Stream(profile) = document.get(destination.ok_or(CONTEXT)?) else {
        return Err(CONTEXT);
    };
    let bytes = decoded(document, &profile)?;
    let stated = pdf_model::icc::Identification::read(&bytes).ok_or(CONTEXT)?;
    if !matches!(&stated.class, b"prtr" | b"mntr")
        || pdf_model::icc::Profile::parse(&bytes).is_none()
    {
        return Err(CONTEXT);
    }
    let (components, other) = match &stated.space {
        b"GRAY" => (1, true),
        b"CMYK" => (4, true),
        b"RGB " if bytes.as_ref() == SRGB => (3, false),
        _ => return Err(CONTEXT),
    };
    if profile.dict.get("N").and_then(Object::as_integer) != Some(components) {
        return Err(CONTEXT);
    }
    Ok(other)
}

/// The page's resource dictionary as ISO 32000-2 §7.7.3.4 finds it: the page's own, else the
/// first an ancestor states, "used in its entirety".
fn resources_of_page(document: &Document, page: &Dictionary) -> Result<Object, Because> {
    let mut node = page.clone();
    let mut seen = BTreeSet::new();
    loop {
        if let Some(resources) = node.get("Resources") {
            if document.resolve(resources).as_dict().is_none() {
                return Err(CONTEXT);
            }
            return Ok(resources.clone());
        }
        let Some(parent) = node.get("Parent") else {
            return Ok(Object::Dictionary(Dictionary::new()));
        };
        let id = parent.as_reference().ok_or(CONTEXT)?;
        if !seen.insert(id) || seen.len() > 256 {
            return Err(CONTEXT);
        }
        node = document.get(id).as_dict().cloned().ok_or(CONTEXT)?;
    }
}

/// Each shared Form is checked under every path. Ancestors distinguish cycles from sharing.
///
/// A Form that states no `/Resources` is walked under the resources it was invoked with. For a
/// Form the page invokes that is ISO 32000-2 §7.8.3's rule for a form written to an earlier
/// version, whose resources "shall be inherited from the resource dictionary of the page on
/// which they are used"; for a nested one it is the stricter proof, because `defaults` holds every
/// dictionary on every path to the same `DefaultRGB`.
fn footprint(
    document: &Document,
    resources: Object,
    walked: &mut usize,
    remaining: &mut usize,
) -> Result<(), Because> {
    let mut pending = vec![(resources, BTreeSet::<ObjectId>::new())];
    while let Some((resources, ancestors)) = pending.pop() {
        *walked = walked.checked_add(1).ok_or(CONTEXT)?;
        if *walked > LIMIT {
            return Err(CONTEXT);
        }
        let resolved = document.resolve(&resources);
        let dict = resolved.as_dict().ok_or(CONTEXT)?;
        defaults(document, dict)?;
        for key in ["ExtGState", "Font", "Pattern", "Shading", "Properties"] {
            if dict.get(key).is_some() {
                return Err(CONTEXT);
            }
        }
        let Some(objects) = dict.get("XObject") else {
            continue;
        };
        let objects = document.resolve(objects);
        let objects = objects.as_dict().ok_or(CONTEXT)?;
        for (_, object) in objects.iter() {
            let id = object.as_reference().ok_or(CONTEXT)?;
            let mut path = ancestors.clone();
            if !path.insert(id) || path.len() >= 8 {
                return Err(CONTEXT);
            }
            let Object::Stream(form) = document.get(id) else {
                return Err(CONTEXT);
            };
            if document
                .get_key(&form.dict, "Subtype")
                .as_name()
                .is_none_or(|name| name.as_bytes() != b"Form")
                || ["Ref", "OPI", "OC", "Group"]
                    .iter()
                    .any(|key| form.dict.get(key).is_some())
            {
                return Err(CONTEXT);
            }
            let bytes = decoded(document, &form)?;
            content(&bytes, remaining)?;
            let next = form
                .dict
                .get("Resources")
                .cloned()
                .unwrap_or_else(|| resources.clone());
            pending.push((next, path));
        }
    }
    Ok(())
}

/// A lexical gate, not another interpreter: only path geometry, literal device
/// colour selection and ordinary Form invocation can occur in this new route.
///
/// The operators are drawn from seven of ISO 32000-2 §8.2's Table 50 categories — general and
/// special graphics state, path construction, path painting, clipping paths, colour, and the
/// category holding `Do` — each narrowed to the operators named below; `cs`, `scn`, `sh`, `gs`
/// and every text operator are absent.
fn content(bytes: &[u8], remaining: &mut usize) -> Result<(), Because> {
    if bytes.len() > *remaining {
        return Err(CONTEXT);
    }
    *remaining = remaining.checked_sub(bytes.len()).ok_or(CONTEXT)?;
    let mut lexer = pdf_syntax::Lexer::new(bytes);
    while let Some(token) = lexer.next_token() {
        if lexer.salvaged().is_some() {
            return Err(CONTEXT);
        }
        match token {
            pdf_syntax::Token::Real(value) if value.is_finite() => {}
            pdf_syntax::Token::Integer(_)
            | pdf_syntax::Token::Name(_)
            | pdf_syntax::Token::Keyword(
                b"q" | b"Q" | b"cm" | b"w" | b"J" | b"j" | b"M" | b"m" | b"l" | b"c" | b"v" | b"y"
                | b"h" | b"re" | b"S" | b"s" | b"f" | b"F" | b"f*" | b"B" | b"B*" | b"b" | b"b*"
                | b"n" | b"W" | b"W*" | b"g" | b"G" | b"rg" | b"RG" | b"k" | b"K" | b"Do",
            ) => {}
            _ => return Err(CONTEXT),
        }
    }
    Ok(())
}

/// No named colour spaces or nonidentity inherited defaults enter this route.
/// Identical defaults make explicit insertion and fallback equivalent on every
/// resource path, including paths the survey deduplicates.
///
/// The one name admitted is ISO 32000-2 §8.6.5.6's `DefaultRGB`, as the shipped sRGB profile.
fn defaults(document: &Document, resources: &Dictionary) -> Result<(), Because> {
    let Some(value) = resources.get("ColorSpace") else {
        return Ok(());
    };
    let value = document.resolve(value);
    let spaces = value.as_dict().ok_or(CONTEXT)?;
    for (name, value) in spaces.iter() {
        if name.as_bytes() != b"DefaultRGB" {
            return Err(CONTEXT);
        }
        let profile = icc(document, value, 3)?;
        if profile.as_slice() != SRGB {
            return Err(CONTEXT);
        }
    }
    Ok(())
}

/// Only a page's supported independent gray target is admitted. Forms cannot
/// change blending context, so all paths on one page have this same target.
///
/// The group is ISO 32000-2 §11.4.7's page group, its attributes §11.6.6's, and its `/CS` the
/// blending space ISO 19005-4 section 6.2.4.3 names as the other licence for a device colour.
fn page_group(document: &Document, page: &Dictionary) -> Result<(), Because> {
    let Some(value) = page.get("Group") else {
        return Ok(());
    };
    let value = document.resolve(value);
    let group = value.as_dict().ok_or(CONTEXT)?;
    if document
        .get_key(group, "S")
        .as_name()
        .is_none_or(|name| name.as_bytes() != b"Transparency")
    {
        return Err(CONTEXT);
    }
    let bytes = icc(document, group.get("CS").ok_or(CONTEXT)?, 1)?;
    // A one-component LUT may trace a tinted axis. Only the existing neutral
    // tone-curve route supports this proof; channel count alone is insufficient.
    if !pdf_model::icc::has_tag(&bytes, *b"kTRC")
        || [*b"A2B0", *b"A2B1", *b"A2B2"]
            .into_iter()
            .any(|tag| pdf_model::icc::has_tag(&bytes, tag))
    {
        return Err(CONTEXT);
    }
    let parsed = pdf_model::icc::Profile::parse(&bytes).ok_or(CONTEXT)?;
    let space = pdf_model::colour::ColourSpace::Icc {
        profile: std::sync::Arc::new(parsed),
    };
    if pdf_model::colour::GreyRoute::of(&space).is_none() {
        return Err(CONTEXT);
    }
    Ok(())
}

/// An ISO 32000-2 §8.6.5.5 `ICCBased` space of `components` channels, admitted only where
/// Table 65's `/Range` and `/Alternate` are absent, so the profile alone states the colour.
fn icc(document: &Document, value: &Object, components: i64) -> Result<Vec<u8>, Because> {
    let value = document.resolve(value);
    let array = value.as_array().ok_or(CONTEXT)?;
    if array.len() != 2
        || array[0]
            .as_name()
            .is_none_or(|name| name.as_bytes() != b"ICCBased")
    {
        return Err(CONTEXT);
    }
    let id = array[1].as_reference().ok_or(CONTEXT)?;
    let Object::Stream(stream) = document.get(id) else {
        return Err(CONTEXT);
    };
    if stream.dict.get("N").and_then(Object::as_integer) != Some(components)
        || stream.dict.get("Range").is_some()
        || stream.dict.get("Alternate").is_some()
    {
        return Err(CONTEXT);
    }
    let bytes = decoded(document, &stream)?;
    let stated = pdf_model::icc::Identification::read(&bytes).ok_or(CONTEXT)?;
    if !matches!((components, &stated.space), (1, b"GRAY") | (3, b"RGB ")) {
        return Err(CONTEXT);
    }
    if pdf_model::icc::Profile::parse(&bytes).is_none() {
        return Err(CONTEXT);
    }
    Ok(bytes.to_vec())
}

/// Decoding partial bytes is not proof that every source consumer was inspected.
fn decoded(
    document: &Document,
    stream: &pdf_syntax::Stream,
) -> Result<std::sync::Arc<[u8]>, Because> {
    let decoded = document
        .decoded_stream_data_reported(stream)
        .map_err(|_| CONTEXT)?;
    if decoded.damage.is_some() {
        return Err(CONTEXT);
    }
    Ok(decoded.data)
}
