//! Bare Type 1 font programs — ISO 32000-2 §9.9's `/FontFile` — draw the document's glyphs.
//!
//! # What this can check that the corpus gate cannot
//!
//! The corpus gate asks whether a page reports anything, and for nineteen sessions the
//! answer for these documents was *no* while they were being drawn in the wrong typeface.
//! An unreadable embedded program falls through to substitution, and substitution only
//! speaks when the face it found can address none of the codes the document declares — so a
//! page set in an embedded Type 1 font drew in some installed face, plausibly, in silence.
//! That is trap 5's failure mode with a fallback in front of it, and no count could see it.
//!
//! So this test does not ask whether the page is complete. It asks whether the glyphs on it
//! came from the program the file embedded, and it can only ask that because the two answers
//! are *distinguishable*: a substituted font is reached through what each code means, so its
//! outlines are a different face's, while the embedded program's are the document's own.
//! `Interpretation::glyphs` counts what marked the page and the extracted text says what it
//! said, and a document that embeds a Type 1 program and draws no glyph at all is exactly
//! the state `issue11150_reduced.pdf` was in.
//!
//! # Why the expectation is a count rather than a picture
//!
//! Type 1 charstrings are `read-fonts`' to interpret and the oracle is what judges the
//! resulting shapes — nineteen of these documents' pages moved from *contradicted* to
//! agreeing with the reference consensus when this landed, which is a stronger statement
//! about the outlines than any assertion here could make. What this file holds is the part
//! the oracle cannot: that the corpus still contains such documents, that they still parse,
//! and that they still put glyphs on the page.

#![expect(
    clippy::print_stdout,
    reason = "test code: the survey output is the point of the run"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Object};

/// Documents whose page one embeds a `/FontFile`, or `None` without the submodule.
///
/// Found by looking rather than by a list of names, because the property under test is
/// "the corpus contains these" and a hard-coded list would assert it into existence.
fn documents_embedding_type1() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut found: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .filter(|path| embeds_type1(path))
        .collect();
    found.sort();
    Some(found)
}

/// Whether page one names a font whose descriptor carries a `/FontFile`.
fn embeds_type1(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let Ok(document) = Document::open(bytes) else {
        return false;
    };
    let Some(page) = pdf_model::Pages::new(&document).get(0) else {
        return false;
    };
    let fonts = document.get_key(&page.resources, "Font");
    let Some(fonts) = fonts.as_dict() else {
        return false;
    };
    fonts.iter().any(|(_, value)| {
        let Some(dict) = document.resolve(value).as_dict().cloned() else {
            return false;
        };
        let descriptor = document.get_key(&dict, "FontDescriptor");
        descriptor.as_dict().is_some_and(|descriptor| {
            !matches!(document.get_key(descriptor, "FontFile"), Object::Null)
        })
    })
}

/// How many such documents the corpus is known to hold.
///
/// A ratchet in the direction that matters: it may rise, and a fall means either the corpus
/// shrank or `/FontFile` stopped being recognised.
const KNOWN_DOCUMENTS: usize = 57;

#[test]
fn an_embedded_type1_program_puts_glyphs_on_the_page() {
    let Some(documents) = documents_embedding_type1() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    assert!(
        documents.len() >= KNOWN_DOCUMENTS,
        "{} corpus documents embed a Type 1 program on page one, expected at least \
         {KNOWN_DOCUMENTS}",
        documents.len()
    );

    let mut blank = Vec::new();
    let mut drawn = 0usize;
    for path in &documents {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let bytes = std::fs::read(path).expect("a corpus file is readable");
        let document = Document::open(bytes).expect("a corpus file this test selected opens");
        let page = pdf_model::Pages::new(&document)
            .get(0)
            .expect("a page this test selected by reading its resources");
        let interpretation = pdf_model::interpret(&document, &page);
        println!(
            "  {name}: {} glyphs, {} unsupported",
            interpretation.glyphs,
            interpretation.unsupported.len()
        );
        if interpretation.glyphs == 0 {
            blank.push(name.into_owned());
        } else {
            drawn = drawn.saturating_add(1);
        }
    }

    // A document may legitimately name a font it never shows, so this is not "every one of
    // them", but a page that embeds a Type 1 program and marks nothing is the state the
    // whole feature exists to leave.
    assert!(
        blank.len() < documents.len() / 4,
        "{} of {} documents embedding a Type 1 program drew no glyph at all: {blank:?}",
        blank.len(),
        documents.len()
    );
    println!("{drawn} of {} documents drew glyphs", documents.len());
}

/// The one document whose glyphs cannot come from anywhere but the embedded program.
///
/// The survey above passes with `/FontFile` unread, because a substitute draws *something*
/// for most of these documents and the count cannot tell whose outlines they are.
/// `issue11150_reduced.pdf` can: its only font is `/Symbol` with `/Flags 4` and no
/// `/Widths`, its content stream is `[(q)521(q)(q)521(q)(q)521(q)] TJ`, and code `q` is
/// `theta` in Symbol's own encoding — a character the installed face this tree substitutes
/// with does not have. With the program unread it drew **nothing**, in silence, while four
/// reference renderers drew three thetas. Confirmed by reverting the feature.
#[test]
fn a_symbolic_type1_program_draws_what_no_substitute_could() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/issue11150_reduced.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let document = Document::open(bytes).expect("the document opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);

    assert_eq!(
        interpretation.unsupported,
        Vec::new(),
        "nothing about this page is unsupported"
    );
    assert_eq!(
        interpretation.glyphs, 6,
        "the content stream shows six codes and every one of them has a glyph"
    );
}
