//! A `CIDFont`'s `/CIDSystemInfo` restated as its `CMap`'s, where nothing this reader draws reads it.
//!
//! ISO 19005-2 section 6.2.11.3.1 and ISO 19005-4 section 6.2.10.3.1 require a composite font whose
//! encoding is not an identity `CMap` to name the same registry and ordering in its `CIDFont` and in
//! its `CMap`, with the `CIDFont`'s supplement at least the `CMap`'s. ISO 32000-2 §9.7.3 says what
//! the three entries are:
//!
//! > A character collection shall be uniquely identified by the Registry , Ordering , and
//! > Supplement entries in the CIDSystemInfo dictionary
//!
//! The clause is symmetric about registry and ordering, so the file alone does not say which side
//! its producer meant (`doc/pdf-a-mitigations.md`'s entry). **What decides it here is what this
//! reader reads** (ADR 2717). §9.7.4.2 selects a glyph of an embedded program without the entry: a
//! Type 2 `CIDFont` through its `/CIDToGIDMap`, a Type 0 one through the program's charset or
//! directly as the glyph index. This reader reads the `CIDFont`'s collection for three things and
//! no others (`pdf_font::composite::collection_names`): the meaning of a CID for §9.10.2's step
//! (b), the vertical forms of a collection that has them, and §9.8.3.3's glyph classes; and it
//! never reads the supplement, on §9.7.3's own words that it "shall not be used in determining
//! compatibility between character collections". So where the program is embedded, the
//! descriptor states no `/FD`, and neither the old nor the new registry and ordering is one this
//! reader holds a table or vertical forms for — or the two name the same collection — the
//! `CIDFont`'s entry is read by nothing this reader draws or extracts, and it is restated as the
//! `CMap`'s. Every other font keeps the refusal, by the sentence that says which condition it
//! failed.

use std::collections::BTreeMap;

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::decision::Because;

/// The requirement a restated collection answers.
const REQUIREMENT: &str = "fonts/cid-system-info-agrees-with-the-cmap";

/// Why a font the requirement failed at that is no object of its own keeps its refusal.
const NOT_AN_OBJECT: &str = "a composite font whose CIDFont and CMap name different character \
     collections is written directly inside another object, or is not a Type 0 font over an \
     embedded CMap and a CIDFont stating a CIDSystemInfo, so there is no font dictionary this \
     conversion restates the collection in";

/// Why a font whose collection this reader reads keeps its refusal.
const READ_BY_THIS_READER: &str = "a composite font's CIDFont and CMap name different character \
     collections, and this reader reads the CIDFont's: its program is not embedded, so the \
     collection is how a substitute is found; or the collection is one this reader holds a CID to \
     Unicode table or vertical forms for, or its descriptor states the FD glyph classes ISO \
     32000-2 section 9.8.3.3 names by collection. Restating it would change what a CID means \
     where it is read, which relabels the font's glyphs rather than restating what the file holds";

/// The `/CIDSystemInfo` each `CIDFont` states once its collection is restated.
#[derive(Debug, Default)]
pub(super) struct Restated {
    /// By the `CIDFont` that is an object of its own.
    pub(super) at: BTreeMap<ObjectId, Dictionary>,
    /// By the `Type0` font whose `/DescendantFonts` writes the `CIDFont` directly.
    pub(super) within: BTreeMap<ObjectId, Dictionary>,
}

/// Every `CIDFont` whose `/CIDSystemInfo` is restated as its `CMap`'s, or why none is.
///
/// The population is the validator's own: every Type 0 font object the requirement failed at,
/// and the refusal where any finding is at no object.
pub(super) fn restated(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<Restated, Because> {
    let (named, elsewhere) = super::sites::objects_named(input, &[REQUIREMENT])?;
    if elsewhere {
        return Err(Because::NotBuiltYet(NOT_AN_OBJECT));
    }
    let mut out = Restated::default();
    for id in named {
        let font = document.get(id);
        let font = font.as_dict().ok_or(Because::NotBuiltYet(NOT_AN_OBJECT))?;
        let Object::Stream(cmap) = document.get_key(font, "Encoding") else {
            return Err(Because::NotBuiltYet(NOT_AN_OBJECT));
        };
        let Object::Dictionary(stated) = document.get_key(&cmap.dict, "CIDSystemInfo") else {
            return Err(Because::NotBuiltYet(NOT_AN_OBJECT));
        };
        let descendants = document.get_key(font, "DescendantFonts");
        let first = descendants
            .as_array()
            .and_then(<[Object]>::first)
            .ok_or(Because::NotBuiltYet(NOT_AN_OBJECT))?;
        let resolved = document.resolve(first);
        let descendant = resolved
            .as_dict()
            .ok_or(Because::NotBuiltYet(NOT_AN_OBJECT))?;
        let Object::Dictionary(used) = document.get_key(descendant, "CIDSystemInfo") else {
            return Err(Because::NotBuiltYet(NOT_AN_OBJECT));
        };
        let vertical = matches!(document.get_key(&cmap.dict, "WMode").as_integer(), Some(1));
        if !unread(
            document,
            descendant,
            &collection(document, &used),
            &collection(document, &stated),
            vertical,
        ) {
            return Err(Because::NotBuiltYet(READ_BY_THIS_READER));
        }
        let restated = restatement(document, &stated);
        match first.as_reference() {
            Some(at) => out.at.insert(at, restated),
            None => out.within.insert(id, restated),
        };
    }
    Ok(out)
}

/// A collection's registry and ordering, as this reader reads them.
type Collection = Option<(String, String)>;

/// The registry and ordering a `/CIDSystemInfo` states, as text.
fn collection(document: &Document, info: &Dictionary) -> Collection {
    let text = |key: &str| {
        document
            .get_key(info, key)
            .as_string()
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    };
    Some((text("Registry")?, text("Ordering")?))
}

/// Whether nothing this reader draws or extracts reads the `CIDFont`'s collection, going from
/// `old` to `new` — the module's three readings, each ruled out.
fn unread(
    document: &Document,
    descendant: &Dictionary,
    old: &Collection,
    new: &Collection,
    vertical: bool,
) -> bool {
    let descriptor = document.get_key(descendant, "FontDescriptor");
    let Some(descriptor) = descriptor.as_dict() else {
        return false;
    };
    let embedded = ["FontFile", "FontFile2", "FontFile3"]
        .iter()
        .any(|key| matches!(document.get_key(descriptor, key), Object::Stream(_)));
    if !embedded || descriptor.get("FD").is_some() {
        return false;
    }
    if old == new {
        return true;
    }
    let read = |collection: &Collection| {
        collection.as_ref().is_some_and(|(registry, ordering)| {
            pdf_font::predefined::cid_to_unicode(registry, ordering).is_some()
                || (vertical && pdf_font::predefined::has_vertical_forms(registry, ordering))
        })
    };
    !read(old) && !read(new)
}

/// The `CMap`'s `/CIDSystemInfo`, written as a dictionary of its three entries' values.
fn restatement(document: &Document, stated: &Dictionary) -> Dictionary {
    let mut out = Dictionary::new();
    for key in ["Registry", "Ordering", "Supplement"] {
        let value = document.get_key(stated, key);
        if !value.is_null() {
            out.insert(Name::new(key.as_bytes()), value.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use pdf_archive::{Flavour, Target};
    use pdf_syntax::Document;

    use super::{Because, READ_BY_THIS_READER, restated};

    /// A one-page document showing one `Type0` font over an embedded `CMap` stating `cmap` as its
    /// collection, whose `CIDFontType2` descendant states `font` and files `program` in its
    /// descriptor (`""` for none).
    fn a_composite_font(cmap: &str, font: &str, program: &str) -> Document {
        let cmap_program = "/CIDInit /ProcSet findresource begin 12 dict begin begincmap \
             /CMapName /X def 1 begincodespacerange <00> <FF> endcodespacerange 1 begincidrange \
             <00> <FF> 0 endcidrange endcmap CMapName currentdict /CMap defineresource pop end end";
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 \
             0 R >> >> /Contents 4 0 R >>"
                .to_owned(),
            "<< /Length 22 >>\nstream\nBT /F1 12 Tf (h) Tj ET\nendstream".to_owned(),
            "<< /Type /Font /Subtype /Type0 /BaseFont /F /Encoding 6 0 R /DescendantFonts [7 0 \
             R] >>"
                .to_owned(),
            format!(
                "<< /Type /CMap /CMapName /X /CIDSystemInfo {cmap} /Length {} >>\nstream\n\
                 {cmap_program}\nendstream",
                cmap_program.len()
            ),
            format!(
                "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /F /CIDSystemInfo {font} \
                 /CIDToGIDMap /Identity /FontDescriptor 8 0 R >>"
            ),
            format!(
                "<< /Type /FontDescriptor /FontName /F /Flags 4 /FontBBox [0 0 1 1] /ItalicAngle 0 \
                 /Ascent 1 /Descent 0 /CapHeight 1 /StemV 1 {program} >>"
            ),
            "<< /Length 4 >>\nstream\n\0\x01\0\0\nendstream".to_owned(),
        ];
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let at = out.len();
        let size = objects.len().saturating_add(1);
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("the fixture opens")
    }

    const ADOBE_CNS: &str = "<< /Registry (Adobe) /Ordering (CNS) /Supplement 7 >>";
    const ADOBE_IDENTITY: &str = "<< /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>";

    #[test]
    fn an_embedded_fonts_collection_no_reading_reads_is_restated_as_its_cmaps() {
        // §9.7.4.2 selects an embedded Type 2 CIDFont's glyphs through its CIDToGIDMap, and
        // neither Adobe-Identity nor Adobe-CNS is a collection this reader holds a table for, so
        // the CIDFont is given the CMap's three entries, which the clause then finds identical.
        let document = a_composite_font(ADOBE_CNS, ADOBE_IDENTITY, "/FontFile2 9 0 R");
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        let restated = restated(&document, &input).expect("nothing reads the CIDFont's collection");
        let written = restated
            .at
            .values()
            .next()
            .expect("one CIDFont is restated");
        assert_eq!(
            restated.at.keys().map(|id| id.number).collect::<Vec<_>>(),
            vec![7]
        );
        assert_eq!(
            written
                .get("Ordering")
                .and_then(pdf_syntax::object::Object::as_string),
            Some(&b"CNS"[..])
        );
        assert_eq!(
            written
                .get("Supplement")
                .and_then(pdf_syntax::object::Object::as_integer),
            Some(7)
        );
    }

    #[test]
    fn a_font_whose_program_is_not_embedded_keeps_its_refusal() {
        // With no program, the collection is how this reader finds a substitute.
        let document = a_composite_font(ADOBE_CNS, ADOBE_IDENTITY, "");
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        assert!(matches!(
            restated(&document, &input),
            Err(Because::NotBuiltYet(why)) if why == READ_BY_THIS_READER
        ));
    }

    #[test]
    fn a_collection_this_reader_reads_keeps_its_refusal() {
        // Adobe-Japan1 has a CID to Unicode table this reader reads for §9.10.2's step (b), so
        // relabelling the font into it would change what its CIDs mean.
        let document = a_composite_font(
            "<< /Registry (Adobe) /Ordering (Japan1) /Supplement 6 >>",
            ADOBE_IDENTITY,
            "/FontFile2 9 0 R",
        );
        let input = pdf_archive::check(&document, Target::Four(Flavour::Plain));
        assert!(matches!(
            restated(&document, &input),
            Err(Because::NotBuiltYet(why)) if why == READ_BY_THIS_READER
        ));
    }
}
