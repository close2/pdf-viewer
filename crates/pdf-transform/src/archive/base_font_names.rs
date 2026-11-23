//! A font's `/BaseFont` written as something other than a name, restated as the name its embedded
//! program states (ADR 3395).
//!
//! ISO 32000-2 §9.6.2.1's Table 109 gives `/BaseFont` the type *name* and the value "The
//! PostScript language name of the font", and §9.6.3 says where a TrueType font's comes from: the
//! program's `name` table, where it states a PostScript name. ISO 19005-2 section 6.2.11.2 and ISO
//! 19005-4 section 6.2.10.2 hold the font to that clause (`pdf_archive`'s
//! `fonts/base-font-is-a-name`). A producer that wrote the entry as a string still embedded the
//! program, and the program says what the name is, so the name written is the program's and not
//! this converter's: the clause decides it, and the string is no name to keep.
//!
//! **Only the characters a PostScript name is made of are kept.** The OpenType specification
//! restricts name ID 6 to printable ASCII, codes 33 to 126 without the ten delimiters, and
//! `pdf_font::embed::postscript_name` reads it so. `issue1350`'s programs state the name as
//! `AACADT+` followed by `ArialMT` with a NUL before each letter, the same bytes as its string;
//! the NULs are outside the record's own alphabet and no name may hold one (§7.3.5), so what is
//! written is `AACADT+ArialMT`. That is a reading of a malformed record and it is said as one:
//! the bytes kept are the program's, in its order, and nothing is added.
//!
//! Table 120 makes the descriptor's `/FontName` the same as the font's `/BaseFont`, so a
//! descriptor whose `/FontName` is no name either is given the same name. What stays refused, by
//! name: a font written inside another object, a Type 0 font (Table 119 composes its name from
//! its descendant's), a font whose program is not an `sfnt` this tree reads or states no
//! PostScript name, and a descriptor that is not an object of its own.

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use super::decision::Because;

/// The requirement this module answers, by the identifier the validator reports it under.
pub(super) const ROW: &str = "fonts/base-font-is-a-name";

/// Why a font's `/BaseFont` is not restated.
const NOT_RESTATED: &str = "a font dictionary states BaseFont as something other than the name \
     ISO 32000-2 \u{a7}9.6.2.1's Table 109 requires, and this conversion restates it only as the \
     PostScript name an embedded TrueType or OpenType program states (\u{a7}9.6.3), for a font \
     that is an object of its own and not a Type 0 font; this font is not that";

/// The names to write, each keyed by the object it goes into, in the source's numbering.
#[derive(Debug, Default)]
pub(super) struct Restated {
    /// Each font's `/BaseFont`.
    pub(super) base_fonts: BTreeMap<ObjectId, Name>,
    /// Each font descriptor's `/FontName`, where it was no name either.
    pub(super) font_names: BTreeMap<ObjectId, Name>,
}

/// Every font [`ROW`] failed at, with the name its program states, or why one has none.
///
/// # Errors
///
/// [`NOT_RESTATED`] where any failing font is outside what the module comment lists; a
/// requirement half answered is a requirement refused.
pub(super) fn restated(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<Restated, Because> {
    let (named, elsewhere) = super::sites::objects_named(input, &[ROW])?;
    if elsewhere {
        return Err(Because::NotBuiltYet(NOT_RESTATED));
    }
    let mut restated = Restated::default();
    for id in named {
        let resolved = document.get(id);
        let Object::Dictionary(font) = &resolved else {
            return Err(Because::NotBuiltYet(NOT_RESTATED));
        };
        if document
            .get_key(font, "Subtype")
            .as_name()
            .map(Name::as_bytes)
            == Some(&b"Type0"[..])
        {
            return Err(Because::NotBuiltYet(NOT_RESTATED));
        }
        let Some(Object::Reference(descriptor_id)) = font.get("FontDescriptor") else {
            return Err(Because::NotBuiltYet(NOT_RESTATED));
        };
        let descriptor = document.get(*descriptor_id);
        let Object::Dictionary(descriptor) = &descriptor else {
            return Err(Because::NotBuiltYet(NOT_RESTATED));
        };
        let name = program_name(document, descriptor).ok_or(Because::NotBuiltYet(NOT_RESTATED))?;
        if document.get_key(descriptor, "FontName").as_name().is_none() {
            restated.font_names.insert(*descriptor_id, name.clone());
        }
        restated.base_fonts.insert(id, name);
    }
    Ok(restated)
}

/// The PostScript name the descriptor's embedded `sfnt` program states, kept to the characters
/// a PostScript name is made of; `None` where there is no such program or it states none.
fn program_name(document: &Document, descriptor: &Dictionary) -> Option<Name> {
    let program = match document.get_key(descriptor, "FontFile2") {
        Object::Stream(stream) => stream,
        _ => match document.get_key(descriptor, "FontFile3") {
            Object::Stream(stream)
                if document
                    .get_key(&stream.dict, "Subtype")
                    .as_name()
                    .map(Name::as_bytes)
                    == Some(&b"OpenType"[..]) =>
            {
                stream
            }
            _ => return None,
        },
    };
    let bytes = document.decoded_stream_data(&program)?;
    stated_name(&bytes).map(|name| Name::new(name.as_bytes()))
}

/// Name ID 6 of an `sfnt` program, the characters outside the record's own alphabet left out, or
/// `None` where the program states none or none of its characters is in that alphabet.
fn stated_name(program: &[u8]) -> Option<String> {
    use skrifa::MetadataProvider as _;
    let face = skrifa::FontRef::new(program).ok()?;
    let stated = face
        .localized_strings(skrifa::string::StringId::POSTSCRIPT_NAME)
        .english_or_first()?;
    let kept: String = stated
        .chars()
        .filter(|c| c.is_ascii_graphic() && !"()<>[]{}/%".contains(*c))
        .collect();
    (!kept.is_empty()).then_some(kept)
}

#[cfg(test)]
mod tests {
    use super::stated_name;

    /// A program that is no `sfnt` states no name, so nothing is restated from it.
    #[test]
    fn a_program_that_is_no_sfnt_states_no_name() {
        assert_eq!(stated_name(b"%!PS-AdobeFont-1.0: Times-Roman"), None);
        assert_eq!(stated_name(&[]), None);
    }
}
