//! §12.5.3's annotation pass, run again over a kept content half, is the whole page.
//!
//! `content::replace` exists so that a wheel notch on a page carrying a `NoZoom` annotation costs
//! the annotations rather than the page (`doc/todo/46`, ADR 0777). What makes that safe is not the
//! argument for it but this: the interpretation it produces is the interpretation a whole
//! re-interpretation would have produced, **field by field**, on real pages of a real document.
//!
//! The document is ISO 32000-2 itself, which carries such an annotation on a third of its 1023
//! pages — trap 4's rule, and the corpus this project opens most.

use std::path::PathBuf;

use pdf_model::content::{FontCache, Interpretation};
use pdf_model::page::Pages;
use pdf_model::view::ViewState;
use pdf_syntax::Document;

/// The document every arm reads, and the one `doc/todo/46` is about.
const SPECIFICATION: &str = "doc/ISO_32000-2_sponsored_EC3.pdf";

/// Where it is, from the manifest rather than from the working directory.
fn specification() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(SPECIFICATION)
}

/// How many pages of each kind the arms below read, so that they are a few seconds rather than a
/// few minutes.
///
/// Spread across the document rather than taken from its front, because the standard's front
/// matter, its clause bodies and its annexes carry different annotations and a run over pages 1
/// to 8 would be a run over its table of contents.
const SAMPLE: usize = 8;

/// Table 167's `NoZoom`, which is bit position 4 and therefore the value 8.
///
/// Read straight off `/Annots` rather than by interpreting the page, for `examples/zoom_cost`'s
/// reason: what is wanted is the population the clause is about, and the flag is what decides it.
const FLAG_NO_ZOOM: i64 = 8;

/// The pages of a document whose annotations set that flag, and the pages whose do not.
fn split(document: &Document) -> (Vec<usize>, Vec<usize>) {
    let pages = Pages::new(document);
    let (mut dependent, mut plain) = (Vec::new(), Vec::new());
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let annotations = document.get_key(&page.dict, "Annots");
        let no_zoom = annotations.as_array().is_some_and(|entries| {
            entries.iter().any(|entry| {
                document
                    .resolve(entry)
                    .as_dict()
                    .map(|dict| document.get_key(dict, "F"))
                    .and_then(|flags| flags.as_integer())
                    .is_some_and(|flags| flags & FLAG_NO_ZOOM != 0)
            })
        });
        if no_zoom {
            dependent.push(index);
        } else {
            plain.push(index);
        }
    }
    (spread(&dependent), spread(&plain))
}

/// [`SAMPLE`] of a population, taken from across it rather than from its front.
fn spread(found: &[usize]) -> Vec<usize> {
    let stride = found.len().div_ceil(SAMPLE).max(1);
    found.iter().copied().step_by(stride).take(SAMPLE).collect()
}

/// The magnifications the replacement is asked for, none of them the one the page was interpreted
/// at.
///
/// §12.5.3's adjustment is a scale of 1/magnification about the annotation's corner, so a value
/// under 1 and a value over it exercise the two directions, and `None` — nobody has said — is the
/// third answer the clause has and the one every gate in this tree interprets at.
const MAGNIFICATIONS: [Option<f32>; 3] = [Some(0.4), Some(3.25), None];

#[test]
fn a_replaced_page_is_the_page_it_would_have_been_interpreted_as() {
    let Ok(bytes) = std::fs::read(specification()) else {
        panic!("{SPECIFICATION} is not unpacked; see doc/environment.md");
    };
    let document = Document::open(bytes).expect("the specification opens");
    let pages = Pages::new(&document);
    let fonts = FontCache::new();
    let (dependent, _) = split(&document);
    let mut compared = 0_usize;
    for index in dependent {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let mut state = ViewState::of(&document);
        state.set_magnification(Some(1.0));
        let (_, replacement) =
            pdf_model::content::interpret_replaceable(&document, &page, &state, &fonts);
        let Some(replacement) = replacement else {
            continue;
        };
        for magnification in MAGNIFICATIONS {
            let mut moved = ViewState::of(&document);
            moved.set_magnification(magnification);
            let replaced =
                pdf_model::content::replace(&document, &page, &moved, &fonts, &replacement);
            let whole = pdf_model::content::interpret_with_fonts(&document, &page, &moved, &fonts);
            agree(&replaced, &whole, index, magnification);
            compared = compared.saturating_add(1);
        }
    }
    // A run that compared nothing passes for the wrong reason, which is trap 25's shape: the
    // population is derived from the file, so an empty one is a claim about the file and has to
    // fail rather than be silent.
    assert!(
        compared >= MAGNIFICATIONS.len(),
        "no page of {SPECIFICATION} was read as view-dependent, so nothing was compared"
    );
}

#[test]
fn a_page_no_annotation_makes_view_dependent_keeps_nothing_to_replace_from() {
    let Ok(bytes) = std::fs::read(specification()) else {
        panic!("{SPECIFICATION} is not unpacked; see doc/environment.md");
    };
    let document = Document::open(bytes).expect("the specification opens");
    let pages = Pages::new(&document);
    let fonts = FontCache::new();
    let (_, plain_pages) = split(&document);
    let mut plain = 0_usize;
    for index in plain_pages {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let mut state = ViewState::of(&document);
        state.set_magnification(Some(1.0));
        let (interpretation, replacement) =
            pdf_model::content::interpret_replaceable(&document, &page, &state, &fonts);
        assert_eq!(
            interpretation.view_dependent,
            replacement.is_some(),
            "page {} keeps a replacement exactly where §12.5.3 makes it depend on the \
             magnification",
            index.saturating_add(1)
        );
        if !interpretation.view_dependent {
            plain = plain.saturating_add(1);
        }
    }
    assert!(
        plain > 0,
        "no plain page was read, so the arm proves nothing"
    );
}

/// Every field of one interpretation against the other's, with no `..` in the destructure.
///
/// Exhaustive on purpose and for `Interpreter::checkpoint`'s reason one crate over: a field added
/// to [`Interpretation`] stops this compiling until somebody has said whether the replacement
/// reproduces it, and `..` would let a field be added that the seam silently loses.
fn agree(replaced: &Interpretation, whole: &Interpretation, index: usize, at: Option<f32>) {
    let page = index.saturating_add(1);
    let where_ = format!("page {page} at magnification {at:?}");
    let Interpretation {
        display_list,
        view_dependent,
        unsupported,
        text,
        glyphs,
        codes_without_a_glyph,
        codes_reaching_a_blank_glyph,
        codes_without_a_vertical_form,
        codes_without_a_character,
        described,
        artifacts,
        inferred_separators,
        marked,
        associated_files,
        language,
        text_layer,
    } = replaced;
    assert_eq!(display_list, &whole.display_list, "display list, {where_}");
    assert_eq!(view_dependent, &whole.view_dependent, "NoZoom, {where_}");
    assert_eq!(unsupported, &whole.unsupported, "reports, {where_}");
    assert_eq!(text, &whole.text, "readback, {where_}");
    assert_eq!(glyphs, &whole.glyphs, "glyphs, {where_}");
    assert_eq!(
        codes_without_a_glyph, &whole.codes_without_a_glyph,
        "codes with no glyph, {where_}"
    );
    assert_eq!(
        codes_reaching_a_blank_glyph, &whole.codes_reaching_a_blank_glyph,
        "codes reaching a blank glyph, {where_}"
    );
    assert_eq!(
        codes_without_a_vertical_form, &whole.codes_without_a_vertical_form,
        "codes with no vertical form, {where_}"
    );
    assert_eq!(
        codes_without_a_character, &whole.codes_without_a_character,
        "codes §9.10.2 could not name, {where_}"
    );
    assert_eq!(described, &whole.described, "§14.9's spans, {where_}");
    assert_eq!(artifacts, &whole.artifacts, "§14.8.2.2's spans, {where_}");
    assert_eq!(
        inferred_separators, &whole.inferred_separators,
        "inferred separators, {where_}"
    );
    assert_eq!(marked, &whole.marked, "§14.7.5.2's spans, {where_}");
    // §14.13.5's attachments state no `PartialEq` — an embedded file is bytes and a specification
    // — so the comparison is of what they print, which is total over the same fields.
    assert_eq!(
        format!("{associated_files:?}"),
        format!("{:?}", whole.associated_files),
        "§14.13.5's associated files, {where_}"
    );
    assert_eq!(language, &whole.language, "§14.9.2.3's /Lang, {where_}");
    assert_eq!(
        text_layer, &whole.text_layer,
        "where the codes sit, {where_}"
    );
    // Not a field, and the one thing a caller reads that is derived from several of them.
    assert_eq!(
        format!("{:?}", replaced.shortfall()),
        format!("{:?}", whole.shortfall()),
        "the readback's shortfall, {where_}"
    );
}
