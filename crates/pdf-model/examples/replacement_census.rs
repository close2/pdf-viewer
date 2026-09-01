//! Whether re-placing §12.5.3's annotations produces the page a whole interpretation does, over
//! every document of a corpus rather than over the eight pages a test can afford.
//!
//! `content::replace` restores the interpreter's state at the seam before the annotation pass and
//! runs that pass again at another magnification (ADR 0777). `tests/replacement.rs` is the guard
//! that runs every build, and its population is a sample of ISO 32000-2's own pages — a document
//! whose every font draws, whose pages state no attachment and which therefore cannot exercise
//! several of the fields the seam carries. This is the wide population: every first page of a
//! corpus that §12.5.3 makes view-dependent, compared field by field.
//!
//! ```sh
//! cargo run --profile gates -p pdf-model --example replacement_census -- <directory>…
//! ```
//!
//! With no directory it reads `doc/pdf.js/test/pdfs`, which is what every ratchet in this tree is
//! measured over. It prints the population it judged as well as the disagreements, because a
//! sweep that found nothing and a sweep that read nothing print the same thing otherwise
//! (trap 25).

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the census"
)]

use std::path::{Path, PathBuf};

use pdf_model::content::{FontCache, Interpretation};
use pdf_model::page::Pages;
use pdf_model::view::ViewState;
use pdf_syntax::Document;

/// The magnifications each page is re-placed at, none of them the one it was interpreted at.
const MAGNIFICATIONS: [Option<f32>; 3] = [Some(0.4), Some(3.25), None];

fn main() {
    let mut roots: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if roots.is_empty() {
        roots.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs"));
    }
    let (mut documents, mut judged, mut disagreed) = (0_usize, 0_usize, 0_usize);
    let mut unpaired = 0_usize;
    for root in &roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            println!("{} is not a directory", root.display());
            continue;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|kind| kind == "pdf"))
            .collect();
        paths.sort();
        for path in paths {
            documents = documents.saturating_add(1);
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Ok(document) = Document::open(bytes) else {
                continue;
            };
            let pages = Pages::new(&document);
            let fonts = FontCache::new();
            for index in 0..pages.len() {
                let Some(page) = pages.get(index) else {
                    continue;
                };
                let mut state = ViewState::of(&document);
                state.set_magnification(Some(1.0));
                let (first, replacement) =
                    pdf_model::content::interpret_replaceable(&document, &page, &state, &fonts);
                // The seam's own condition, checked on the wide population rather than asserted:
                // a page §12.5.3 makes view-dependent and that kept nothing to replace from would
                // re-interpret for ever, silently and correctly, which is the shape no gate reads.
                if first.view_dependent != replacement.is_some() {
                    unpaired = unpaired.saturating_add(1);
                    println!(
                        "{} page {}: view_dependent {} but replacement {}",
                        path.display(),
                        index.saturating_add(1),
                        first.view_dependent,
                        replacement.is_some(),
                    );
                }
                let Some(replacement) = replacement else {
                    continue;
                };
                for magnification in MAGNIFICATIONS {
                    let mut moved = ViewState::of(&document);
                    moved.set_magnification(magnification);
                    let replaced =
                        pdf_model::content::replace(&document, &page, &moved, &fonts, &replacement);
                    let whole =
                        pdf_model::content::interpret_with_fonts(&document, &page, &moved, &fonts);
                    judged = judged.saturating_add(1);
                    for field in differences(&replaced, &whole) {
                        disagreed = disagreed.saturating_add(1);
                        println!(
                            "{} page {} at {magnification:?}: {field}",
                            path.display(),
                            index.saturating_add(1),
                        );
                    }
                }
            }
        }
    }
    println!(
        "{documents} document(s) read, {judged} comparison(s), {disagreed} disagreement(s), \
         {unpaired} page(s) where the seam's condition and the clause's answer disagree"
    );
}

/// Which fields of the two interpretations differ, named.
///
/// The destructure is exhaustive for `Interpreter::checkpoint`'s reason: a field added to
/// [`Interpretation`] stops this compiling until somebody has said whether the replacement
/// reproduces it.
fn differences(replaced: &Interpretation, whole: &Interpretation) -> Vec<&'static str> {
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
    let mut found = Vec::new();
    let mut differs = |same: bool, field: &'static str| {
        if !same {
            found.push(field);
        }
    };
    differs(display_list == &whole.display_list, "display list");
    differs(view_dependent == &whole.view_dependent, "view_dependent");
    differs(unsupported == &whole.unsupported, "reports");
    differs(text == &whole.text, "readback");
    differs(glyphs == &whole.glyphs, "glyphs");
    differs(
        codes_without_a_glyph == &whole.codes_without_a_glyph,
        "codes with no glyph",
    );
    differs(
        codes_reaching_a_blank_glyph == &whole.codes_reaching_a_blank_glyph,
        "codes reaching a blank glyph",
    );
    differs(
        codes_without_a_vertical_form == &whole.codes_without_a_vertical_form,
        "codes with no vertical form",
    );
    differs(
        codes_without_a_character == &whole.codes_without_a_character,
        "codes §9.10.2 could not name",
    );
    differs(described == &whole.described, "§14.9's spans");
    differs(artifacts == &whole.artifacts, "§14.8.2.2's spans");
    differs(
        inferred_separators == &whole.inferred_separators,
        "inferred separators",
    );
    differs(marked == &whole.marked, "§14.7.5.2's spans");
    differs(
        format!("{associated_files:?}") == format!("{:?}", whole.associated_files),
        "§14.13.5's associated files",
    );
    differs(language == &whole.language, "§14.9.2.3's /Lang");
    differs(text_layer == &whole.text_layer, "where the codes sit");
    found
}
