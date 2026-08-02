//! A `TrueType` Collection embedded where ISO 32000-2 §9.9 states a font program.
//!
//! Table 127 makes `/FontFile2` "a TrueType font program"; a collection is a *container* of
//! several, introduced by a `ttcf` header rather than by a table directory, so a file embedding
//! one is malformed. Two of the pdf.js corpus's first pages do it, and until the
//! hundred-and-fifty-seventh session both drew no text at all and said `Invalid sfnt version
//! 0x74746366` — which is `ttcf` in hexadecimal.
//!
//! `pdf_font::collection` chooses the face the descriptor's own `/FontName` names and copies it
//! out as a standalone `sfnt`. **The tests are against the real documents**, which is trap 4's
//! rule and matters here more than usual: a hand-built collection would be built by the same
//! reading of the format that the code under test uses.

#![expect(
    clippy::panic,
    reason = "test code: a font that stops loading should fail loudly, naming the document"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::{Dictionary, Document};

/// Opens a corpus document, or `None` when the submodule is not checked out.
fn corpus_document(name: &str) -> Option<Document> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    let bytes = std::fs::read(path).ok()?;
    Some(Document::open(bytes).unwrap_or_else(|e| panic!("{name} does not open: {e}")))
}

/// Page one's `/Font` resource of that name.
fn page_one_font(document: &Document, file: &str, resource: &str) -> Dictionary {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .unwrap_or_else(|| panic!("{file} has no page one"));
    let resources = document.get_key(&page.dict, "Resources");
    let resources = resources
        .as_dict()
        .unwrap_or_else(|| panic!("{file} page one has no /Resources"));
    let fonts = document.get_key(resources, "Font");
    let fonts = fonts
        .as_dict()
        .unwrap_or_else(|| panic!("{file} page one has no /Font resources"));
    let font = document.get_key(fonts, resource);
    font.as_dict()
        .unwrap_or_else(|| panic!("{file} page one has no /{resource}"))
        .clone()
}

/// The two corpus documents that embed a collection, loaded and drawn from.
///
/// What says the face was really extracted is that a code produces an *outline*: the container
/// parses as nothing at all, so a reader that had passed the raw bytes through would fail at
/// `FontRef::new` rather than produce a blank glyph.
#[test]
fn a_collection_embedded_as_a_font_program_yields_glyphs() {
    for (file, resource) in [("issue13193.pdf", "F1"), ("issue9262_reduced.pdf", "F1")] {
        let Some(document) = corpus_document(file) else {
            println!("skipped: the doc/pdf.js submodule is not checked out");
            return;
        };
        let dict = page_one_font(&document, file, resource);
        let font = pdf_font::LoadedFont::load(&document, &dict, resource)
            .unwrap_or_else(|e| panic!("{file}'s /{resource} is a collection and must load: {e}"));

        let drawn = (0_u32..=255)
            .filter(|code| {
                u8::try_from(*code).is_ok_and(|byte| {
                    font.outline(pdf_font::Code::single_byte(byte))
                        .is_some_and(|path| !path.is_empty())
                })
            })
            .count();
        assert!(
            drawn > 8,
            "{file}: only {drawn} of 256 codes reached a glyph outline, so the face was not \
             extracted from the collection"
        );
    }
}

/// The face is the one the document asked for, and it is not the first in the container.
///
/// `issue13193.pdf` embeds a collection of `Cambria` and `CambriaMath` and its descriptor names
/// `DCWGQU+CambriaMath` — so §9.6.4's subset prefix has to come off and the *second* face has to
/// be chosen. **Face zero would be the wrong face**, which is what makes this document worth a
/// test of its own rather than a line in the first one.
///
/// What distinguishes the two without comparing pictures is a table: `CambriaMath` carries
/// `MATH` and `Cambria` does not.
#[test]
fn the_face_the_descriptor_names_is_the_face_extracted() {
    let file = "issue13193.pdf";
    let Some(document) = corpus_document(file) else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let dict = page_one_font(&document, file, "F1");
    // §9.7.3 puts a composite font's descriptor on the *descendant*, so both places are
    // looked at rather than the one this document happened to use.
    let descendant = document.get_key(&dict, "DescendantFonts");
    let descendant = descendant
        .as_array()
        .and_then(|array| array.first())
        .map(|first| document.resolve(first));
    let descriptor = descendant
        .as_ref()
        .and_then(|font| font.as_dict())
        .map_or_else(
            || document.get_key(&dict, "FontDescriptor"),
            |font| document.get_key(font, "FontDescriptor"),
        );
    let descriptor = descriptor.as_dict().expect("the font has a descriptor");
    let stream = document.get_key(descriptor, "FontFile2");
    let stream = stream.as_stream().expect("the descriptor embeds a program");
    let embedded = document
        .decoded_stream_data(stream)
        .expect("/FontFile2 decodes");
    assert_eq!(
        embedded.get(..4),
        Some(b"ttcf".as_slice()),
        "this test rests on {file} embedding a collection"
    );

    let has_math = |bytes: &[u8]| {
        read_fonts::FontRef::new(bytes)
            .expect("extract produces a readable sfnt")
            .table_data(read_fonts::types::Tag::new(b"MATH"))
            .is_some()
    };
    let wanted = pdf_font::collection::extract(&embedded, Some("DCWGQU+CambriaMath"))
        .expect("the collection yields the face the descriptor names");
    let first = pdf_font::collection::extract(&embedded, None)
        .expect("the collection yields its first face");

    assert!(
        has_math(&wanted),
        "the extracted face has no MATH table, so it is not CambriaMath"
    );
    assert!(
        !has_math(&first),
        "face zero has a MATH table, so this document no longer distinguishes the two and the \
         test proves nothing"
    );
}
