//! A font name ISO 19005 binds to UTF-8, restated where this conversion embeds the program it names
//! (`doc/adr/2846`).
//!
//! # What the clause asks, and of which names
//!
//! ISO 19005-2 section 6.1.8 and ISO 19005-4 section 6.1.7 hold font names, colourant names and
//! structure type names to valid UTF-8 once their number-sign escapes are expanded. A colourant
//! name is what a press and every other colour space naming the same ink find the separation by,
//! and a structure type name is what the role map and every element of that type are keyed by,
//! so neither changes here. A font name is different in one case, and this module is that case.
//!
//! # Why a name the embedding writes is the name the clauses ask for
//!
//! Where the file embeds no program for a font and this conversion embeds one, the routes that do
//! it (`super::pack`, `super::machine_cid`, `super::rekeyed`) already write the descendant's
//! `/BaseFont` and the descriptor's `/FontName` as the embedded program's own name, because ISO
//! 32000-2 §9.7.4's Table 115 makes a `CIDFont`'s `/BaseFont` its program's PostScript name, and
//! §9.8.1's Table 120 makes the descriptor's `/FontName` the same as that `/BaseFont`. The producer's
//! name was a request for a face; the face this conversion embedded answers it, and the request
//! is kept in the report and the file's history as the font's `requested` name.
//!
//! That leaves the Type 0 font above the descendant. §9.7.6.1's Table 119 gives its `/BaseFont`
//! a value of its own:
//!
//! > If the descendant is a Type 0 CIDFont, this name should be the concatenation of the CIDFont's
//! > BaseFont name, a hyphen, and the CMap name given in the Encoding entry (or the CMapName entry
//! > in the CMap). If the descendant is a Type 2 CIDFont, this name should be the same as the
//! > CIDFont's BaseFont name.
//!
//! and its NOTE says the name is "In principle, ... an arbitrary name, since there is no font
//! program associated directly with a Type 0 font dictionary". So where the descendant's name is
//! now the program's, the Type 0 font's is restated as the table composes it from that name, and
//! no processor computes anything different from the change.
//!
//! # What stays refused
//!
//! A place the output no longer reaches is not held to anything, because the rewriter writes what
//! it reaches from the catalog: a descriptor a route replaced with one of its own, and a font only
//! the resources of a page photographed for another reason named (`super::photographed`, ADR 2516),
//! go with what replaced them.
//!
//! A font name on a font whose program the file embeds itself (the program's own name is then the
//! one Table 115 asks for, and it is the invalid one), a font name no route rewrote, a font name
//! written inside another object, and every colourant and structure type name: the row is answered
//! only where every place it names is an object whose font names the output holds valid, and the
//! output is held to the target afterwards (`super::super::hold_the_output_to_the_target`).

use std::collections::BTreeSet;

use pdf_archive::report::Outcome;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::Substitutes;

/// The requirement this module answers, by the identifier the validator reports it under.
pub(in crate::archive) const ROW: &str = "file-structure/bound-names-are-valid-utf8";

/// The two entries a font or a font descriptor states a font name in.
const FONT_NAME_KEYS: [&str; 2] = ["BaseFont", "FontName"];

/// Restates each failing Type 0 font's `/BaseFont` from its descendant's new name, and records
/// whether every place [`ROW`] failed at now holds valid names.
pub(in crate::archive) fn restate(
    document: &Document,
    input: &pdf_archive::Report,
    substitutes: &mut Substitutes,
) {
    substitutes.font_names_restated = false;
    let Some(judgement) = input.failures().find(|judgement| judgement.id == ROW) else {
        return;
    };
    let Outcome::Failed { places, total } = &judgement.outcome else {
        return;
    };
    // A list cut at the validator's bound names objects this module never sees.
    if places.len() != *total {
        return;
    }
    let Some(objects): Option<Vec<ObjectId>> =
        places.iter().map(|finding| finding.place.object).collect()
    else {
        return;
    };
    for &id in &objects {
        if let Some(name) = type0_name(document, id, substitutes) {
            substitutes
                .described
                .entry(id)
                .or_default()
                .insert(Name::new(&b"BaseFont"[..]), Object::Name(name));
        }
    }
    let reached = reached(document, substitutes);
    substitutes.font_names_restated = objects.iter().all(|&id| {
        names_are_valid(&effective(document, id, substitutes)) || !reached.contains(&id)
    });
}

/// Every object the output reaches from the trailer's `/Root` and `/Info`, read as the output
/// writes it: an object this conversion wrote in place of the source's, the source's with the
/// entries the routes gave it, and a photographed page without what its photograph replaced.
///
/// The rewriter writes what it reaches from the catalog, so a place the walk does not reach — a
/// descriptor a route replaced with one of its own, a font only a photographed page's resources
/// named — is in no object of the output, and holds no name to be valid.
fn reached(document: &Document, substitutes: &Substitutes) -> BTreeSet<ObjectId> {
    let trailer = document.trailer();
    let mut pending: Vec<Object> = ["Root", "Info"]
        .into_iter()
        .filter_map(|key| trailer.get(key).cloned())
        .collect();
    let mut seen = BTreeSet::new();
    while let Some(object) = pending.pop() {
        match object {
            Object::Reference(id) => {
                if !seen.insert(id) {
                    continue;
                }
                let mut written = effective(document, id, substitutes);
                if let Some(photograph) = substitutes.photographs.get(&id) {
                    for key in super::photographed::PHOTOGRAPHED_AWAY {
                        written.remove(key);
                    }
                    pending.push(Object::Reference(photograph.contents));
                    pending.push(Object::Dictionary(photograph.resources.clone()));
                }
                pending.extend(written.iter().map(|(_, value)| value.clone()));
                if let Some(Object::Array(items)) = substitutes.written.get(&id) {
                    pending.extend(items.iter().cloned());
                } else if let Object::Array(items) = document.get(id) {
                    pending.extend(items);
                }
            }
            Object::Dictionary(dict) => pending.extend(dict.iter().map(|(_, value)| value.clone())),
            Object::Array(items) => pending.extend(items),
            Object::Stream(stream) => {
                pending.extend(stream.dict.iter().map(|(_, value)| value.clone()));
            }
            _ => {}
        }
    }
    seen
}

/// The name Table 119 composes for the Type 0 font at `id`, where it states one that is not
/// UTF-8 and its descendant's name, as this conversion writes it, is.
fn type0_name(document: &Document, id: ObjectId, substitutes: &Substitutes) -> Option<Name> {
    // A font this conversion wrote whole is not one the gains reach.
    if substitutes.written.contains_key(&id) {
        return None;
    }
    let font = document.get(id);
    let font = font.as_dict()?;
    if document.get_key(font, "Subtype").as_name()?.as_bytes() != b"Type0"
        || is_valid(document.get_key(font, "BaseFont").as_name()?)
    {
        return None;
    }
    let descendants = document.get_key(font, "DescendantFonts");
    let descendant = descendants.as_array()?.first()?.as_reference()?;
    let written = effective(document, descendant, substitutes);
    let base = written.get("BaseFont")?.as_name()?;
    if !is_valid(base) {
        return None;
    }
    let composed = match written.get("Subtype").and_then(Object::as_name) {
        Some(subtype) if subtype.as_bytes() == b"CIDFontType0" => {
            let cmap = cmap_name(document, font)?;
            let mut bytes = base.as_bytes().to_vec();
            bytes.push(b'-');
            bytes.extend_from_slice(cmap.as_bytes());
            Name::new(bytes)
        }
        Some(subtype) if subtype.as_bytes() == b"CIDFontType2" => base.clone(),
        _ => return None,
    };
    is_valid(&composed).then_some(composed)
}

/// The `CMap` name Table 119 composes the Type 0 font's name with: the `/Encoding` entry's own
/// name, or the `/CMapName` of the `CMap` stream it names.
fn cmap_name(document: &Document, font: &Dictionary) -> Option<Name> {
    match document.get_key(font, "Encoding") {
        Object::Name(name) => Some(name),
        Object::Stream(cmap) => document.get_key(&cmap.dict, "CMapName").as_name().cloned(),
        _ => None,
    }
}

/// The dictionary the output writes at `id`: the object this conversion wrote in its place, or the
/// source's with the entries the font routes gave it.
fn effective(document: &Document, id: ObjectId, substitutes: &Substitutes) -> Dictionary {
    if let Some(written) = substitutes.written.get(&id) {
        return written.as_dict().cloned().unwrap_or_default();
    }
    let source = document.get(id);
    let mut dict = source.as_dict().cloned().unwrap_or_default();
    if let Some(gains) = substitutes.described.get(&id) {
        for (key, value) in gains.iter() {
            dict.insert(key.clone(), value.clone());
        }
    }
    dict
}

/// Whether `dict` is a font or a font descriptor whose font names are all valid UTF-8.
///
/// A dictionary stating neither entry is no font the clause's font names are read from, so a place
/// there is a colourant, a structure type or a font written inside another object, and the row
/// keeps its refusal for it.
fn names_are_valid(dict: &Dictionary) -> bool {
    let mut stated = FONT_NAME_KEYS
        .iter()
        .filter_map(|key| dict.get(key))
        .peekable();
    stated.peek().is_some() && stated.all(|value| value.as_name().is_some_and(is_valid))
}

/// ISO 19005-2 section 6.1.8's test: the name's bytes, its escapes already expanded by the parser,
/// are a valid UTF-8 sequence.
fn is_valid(name: &Name) -> bool {
    name.as_str().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(bytes: &[u8]) -> Object {
        Object::Name(Name::new(bytes))
    }

    fn dict(entries: &[(&str, Object)]) -> Dictionary {
        let mut dict = Dictionary::new();
        for (key, value) in entries {
            dict.insert(Name::new(key.as_bytes()), value.clone());
        }
        dict
    }

    #[test]
    fn a_font_name_is_valid_only_where_every_stated_name_is_utf8() {
        // ISO 19005-2 section 6.1.8: "Font names ... shall be valid UTF-8 character sequences".
        // GBK's bytes for the two characters of a Chinese face name are no UTF-8 sequence.
        let gbk = b"\xba\xda\xcc\xe5".as_slice();
        assert!(names_are_valid(&dict(&[(
            "BaseFont",
            name(b"ABCDEF+NotoSansCJKsc")
        )])));
        assert!(!names_are_valid(&dict(&[("BaseFont", name(gbk))])));
        assert!(!names_are_valid(&dict(&[
            ("BaseFont", name(b"Valid")),
            ("FontName", name(gbk)),
        ])));
        // A dictionary stating no font name is not one this module answers for.
        assert!(!names_are_valid(&dict(&[("Type", name(b"Font"))])));
    }
}
