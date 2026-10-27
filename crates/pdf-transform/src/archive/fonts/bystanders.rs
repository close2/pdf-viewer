//! A font that names a descriptor this conversion renamed, and that nothing in the conversion
//! touched, keeps a descriptor stating its own name.
//!
//! ISO 32000-2 §9.8.1 (Table 120) makes a descriptor's `/FontName` the name its fonts state:
//!
//! > This name shall be the same as the value of BaseFont in the font or CIDFont dictionary that
//! > refers to this font descriptor.
//!
//! The re-keying route (ADR 2378) and the CJK pack's (ADR 2302) embed a face under the descriptor
//! the font names and rename it, `/FontName` and the font's `/BaseFont` together, for the face
//! they embed. Another font dictionary that names the same descriptor object — a descendant the
//! pages never show, or one an earlier page of a form shows — is not part of that decision: its
//! `/BaseFont` is the producer's and stays, so after the rename its descriptor would name a font
//! it does not describe. **The descriptor it keeps is a copy of the one the producer wrote**,
//! unembedded as the producer left it, which is the font it always was; ISO 19005-2 section
//! 6.2.11.4.1 asks a program of the fonts a document renders, and this one is rendered by no page
//! (`doc/adr/2476`).
//!
//! The walk is over every object of the document rather than over the fonts the pages name,
//! because Table 120 speaks of every dictionary that refers to the descriptor; it runs only when
//! a route renamed a descriptor, so a document no such route touched pays nothing. A font written
//! directly inside another dictionary has no object number to point at the copy from, and keeps
//! the renamed descriptor: the same limit `fonts::gains` meets for a font that is not an object.

use std::collections::BTreeMap;

use pdf_syntax::Document;
use pdf_syntax::object::{Name, Object, ObjectId};

use super::super::decision::Because;
use super::super::prepare::Spare;
use super::Substitutes;

/// Points every font dictionary that names a descriptor this conversion renamed, and that the
/// conversion left its own `/BaseFont`, at a copy of the descriptor as the producer wrote it.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] where no object number is spare for a copy.
pub(super) fn keep_their_own_descriptors(
    document: &Document,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    let renamed: Vec<ObjectId> = substitutes
        .described
        .iter()
        .filter(|(_, gains)| gains.get("FontName").is_some())
        .map(|(descriptor, _)| *descriptor)
        .collect();
    if renamed.is_empty() {
        return Ok(());
    }
    let mut copies: BTreeMap<ObjectId, ObjectId> = BTreeMap::new();
    for number in 1..document.xref().next_object_number() {
        let font_at = ObjectId::new(number, 0);
        let font = document.get(font_at);
        let Some(descriptor) = font
            .as_dict()
            .and_then(|font| font.get("FontDescriptor"))
            .and_then(Object::as_reference)
        else {
            continue;
        };
        if !renamed.contains(&descriptor) || the_conversion_names(substitutes, font_at) {
            continue;
        }
        let copy = if let Some(copy) = copies.get(&descriptor) {
            *copy
        } else {
            let at = copy_of(document, descriptor, spare, substitutes)?;
            copies.insert(descriptor, at);
            at
        };
        substitutes
            .described
            .entry(font_at)
            .or_default()
            .insert(Name::new(&b"FontDescriptor"[..]), Object::Reference(copy));
    }
    Ok(())
}

/// The producer's descriptor, written again as an object of its own, and where.
fn copy_of(
    document: &Document,
    descriptor: ObjectId,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<ObjectId, Because> {
    let producers = document
        .get(descriptor)
        .as_dict()
        .cloned()
        .ok_or(Because::NotBuiltYet(super::DESCRIPTOR_NOT_AN_OBJECT))?;
    let at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
    substitutes
        .written
        .insert(at, Object::Dictionary(producers));
    Ok(at)
}

/// Whether the conversion gives this font dictionary a `/BaseFont` or a descriptor of its own:
/// the fonts a route embedded into, whose names it chose.
fn the_conversion_names(substitutes: &Substitutes, font: ObjectId) -> bool {
    substitutes.described.get(&font).is_some_and(|gains| {
        gains.get("BaseFont").is_some() || gains.get("FontDescriptor").is_some()
    })
}
