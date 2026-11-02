//! The sentence a `.notdef` row keeps where a font it names embeds a program the restatement does
//! not extend, naming that program's format (`doc/adr/2601`).
//!
//! ISO 32000-2 §9.9.1's Table 124 names a font program by the key that holds it: `FontFile` a Type 1
//! program in its original format, `FontFile2` a TrueType program, and `FontFile3` the format its
//! stream's `/Subtype` names — `Type1C`, `CIDFontType0C` or `OpenType`. The restatement
//! (`super::truetype_cmap`) gives a code on `.notdef` a glyph in a TrueType, a simple CFF, a
//! simple font's Type 1 and a CID-keyed CFF program; where the row is refused because a font it names holds another, the
//! sentence says which, rather than calling every program TrueType.

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name};

use super::super::sites::{dictionary_of, objects_named};
use super::truetype_cmap::{NOTDEF_NOT_GIVEN, NOTDEF_ROWS};
use super::{cff_notdef_glyph, cid_cff_notdef_glyph, cid_notdef_glyph};

/// Why the rows stay refused where a font they name embeds a Type 1 program the restatement does
/// not extend: one under an `MMType1` font, or one the descriptor does not name as a stream.
const IN_TYPE_1: &str = "this file shows a character code that reaches the .notdef glyph of an \
     embedded Type 1 program (a FontFile stream), and this conversion gives such a code a glyph of \
     its own only in a TrueType, simple CFF or CID-keyed CFF program, or a Type 1 program under a \
     Type1 font that names it as a stream, and only where every font the requirement names has \
     one: it does not extend this font's Type 1 program";

/// Why the rows stay refused where a font they name embeds an OpenType program.
const IN_OPENTYPE: &str = "this file shows a character code that reaches the .notdef glyph of an \
     embedded OpenType program (a FontFile3 stream of subtype OpenType), and this conversion gives \
     such a code a glyph of its own only in a TrueType, simple CFF or CID-keyed CFF program, and \
     only where every font the requirement names has one: it does not extend an OpenType program";

/// Why the rows stay refused where a font they name embeds no program.
const NOT_EMBEDDED: &str = "this file shows a character code that reaches the .notdef glyph of a \
     font that embeds no program, and this conversion gives such a code a glyph of its own only in \
     a TrueType, simple CFF or CID-keyed CFF program the file embeds, and only where every font the \
     requirement names has one";

/// Whether the restatement extends the program `dict`'s font embeds: a TrueType program under a
/// `TrueType` or `CIDFontType2` font, a simple CFF or Type 1 program under a `Type1` font, or a
/// CID-keyed CFF program.
pub(in crate::archive) fn extends(document: &Document, dict: &Dictionary) -> bool {
    document
        .get_key(dict, "Subtype")
        .as_name()
        .map(Name::as_bytes)
        == Some(&b"TrueType"[..])
        || cid_notdef_glyph::is_cid_truetype(document, dict)
        || cff_notdef_glyph::is_simple_cff(document, dict)
        || cff_notdef_glyph::is_simple_type1(document, dict)
        || cid_cff_notdef_glyph::is_cid_cff(document, dict)
}

/// The sentence the `.notdef` rows keep: the format of the first font they name whose program is
/// of a format the restatement never extends, or [`NOTDEF_NOT_GIVEN`] — which states the
/// restatement's conditions — where every program they name is TrueType or CFF, or the checker
/// named them only in part.
pub(in crate::archive) fn notdef_not_given(
    document: &Document,
    input: &pdf_archive::Report,
) -> &'static str {
    let Ok((named, _)) = objects_named(input, &NOTDEF_ROWS) else {
        return NOTDEF_NOT_GIVEN;
    };
    for id in named {
        let resolved = document.get(id);
        let Some(dict) = dictionary_of(&resolved) else {
            continue;
        };
        if let Some(sentence) = (!extends(document, dict))
            .then(|| format_of(document, dict))
            .flatten()
        {
            return sentence;
        }
    }
    NOTDEF_NOT_GIVEN
}

/// The sentence naming the program a font embeds by Table 124's key and subtype, where that
/// format is one the restatement never extends; `None` for a TrueType or CFF program, which
/// [`NOTDEF_NOT_GIVEN`]'s conditions decide.
fn format_of(document: &Document, dict: &Dictionary) -> Option<&'static str> {
    // A Type 0 font's program is its descendant CIDFont's (ISO 32000-2 §9.7.6.1, Table 119's
    // DescendantFonts: a one-element array).
    let descendant = document
        .get_key(dict, "DescendantFonts")
        .as_array()
        .and_then(|fonts| fonts.first().map(|font| document.resolve(font)));
    let font = descendant.as_ref().and_then(dictionary_of).unwrap_or(dict);
    let descriptor = document.get_key(font, "FontDescriptor");
    let Some(descriptor) = dictionary_of(&descriptor) else {
        return Some(NOT_EMBEDDED);
    };
    if descriptor.get("FontFile").is_some() {
        return Some(IN_TYPE_1);
    }
    let third = document.get_key(descriptor, "FontFile3");
    let opentype = dictionary_of(&third).is_some_and(|stream| {
        document
            .get_key(stream, "Subtype")
            .as_name()
            .map(Name::as_bytes)
            == Some(&b"OpenType"[..])
    });
    if opentype {
        Some(IN_OPENTYPE)
    } else if descriptor.get("FontFile2").is_some() || dictionary_of(&third).is_some() {
        None
    } else {
        Some(NOT_EMBEDDED)
    }
}

#[cfg(test)]
mod tests {
    use pdf_syntax::object::ObjectId;

    use super::*;

    /// A file with no cross-reference table, which the reader recovers by scanning: objects 3 to 7
    /// are fonts whose descriptors name each of Table 124's keys this module tells apart.
    fn fonts() -> Document {
        Document::open(
            b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type \
              /Pages /Kids [] /Count 0 >>\nendobj\n\
              3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /A /FontDescriptor 13 0 R >>\nendobj\n\
              13 0 obj\n<< /Type /FontDescriptor /FontName /A /FontFile 23 0 R >>\nendobj\n\
              23 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
              4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /B /FontDescriptor 14 0 R >>\nendobj\n\
              14 0 obj\n<< /Type /FontDescriptor /FontName /B /FontFile3 24 0 R >>\nendobj\n\
              24 0 obj\n<< /Subtype /OpenType /Length 0 >>\nstream\n\nendstream\nendobj\n\
              5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /C >>\nendobj\n\
              6 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /D /Encoding /Identity-H \
              /DescendantFonts [16 0 R] >>\nendobj\n\
              16 0 obj\n<< /Type /Font /Subtype /CIDFontType0 /BaseFont /D /FontDescriptor 13 0 R \
              >>\nendobj\n\
              7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /E /FontDescriptor 17 0 R >>\nendobj\n\
              17 0 obj\n<< /Type /FontDescriptor /FontName /E /FontFile2 23 0 R >>\nendobj\n\
              trailer\n<< /Root 1 0 R >>\n%%EOF\n"
                .to_vec(),
        )
        .expect("the fixture opens")
    }

    fn format_of_font(document: &Document, number: u32) -> Option<&'static str> {
        let font = document.get(ObjectId::new(number, 0));
        format_of(document, dictionary_of(&font).expect("a dictionary"))
    }

    #[test]
    fn each_program_is_named_by_table_124s_key_and_subtype() {
        let document = fonts();
        assert_eq!(format_of_font(&document, 3), Some(IN_TYPE_1));
        assert_eq!(format_of_font(&document, 4), Some(IN_OPENTYPE));
        assert_eq!(format_of_font(&document, 5), Some(NOT_EMBEDDED));
        // A Type 0 font is named by its descendant's program.
        assert_eq!(format_of_font(&document, 6), Some(IN_TYPE_1));
        // A TrueType program is one the restatement extends, so its own conditions decide.
        assert_eq!(format_of_font(&document, 7), None);
        for sentence in [IN_TYPE_1, IN_OPENTYPE, NOT_EMBEDDED, NOTDEF_NOT_GIVEN] {
            assert!(
                !sentence.contains("embedded TrueType program"),
                "{sentence}"
            );
        }
    }
}
