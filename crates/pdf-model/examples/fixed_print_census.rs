//! §12.5.6.22's watermark annotations, and how many of them state Table 193's `/FixedPrint`.
//!
//! Written for one question, in `doc/todo/25`: the entry states a placement — the annotation's
//! rectangle transformed by Table 194's `/Matrix` and translated by `/H` and `/V` percentages of
//! the media — and until the nine-hundred-and-forty-second session this reader placed a watermark
//! on `/Rect` like any other annotation. Whether a corpus page can *rank* that placement, or
//! whether the fixture for it has to be hand-built, is a question about a population rather than
//! about the clause, and `CLAUDE.md` says the population is counted rather than assumed.
//!
//! What is counted, and why each number is here:
//!
//! - **Watermark annotations**, and the documents stating one. The subtype is what Table 193's
//!   row is conditioned on: "shall be Watermark for a watermark annotation".
//! - **Those stating `/FixedPrint`**, which is what makes the placement different at all — Table
//!   193 makes it optional, and "[i]f this entry is not present, the annotation shall be drawn
//!   without any special consideration for the dimensions of the target media".
//! - **What each fixed print dictionary states.** `/Matrix`, `/H` and `/V` are all optional with
//!   stated defaults (the identity, 0 and 0), so a dictionary stating none of the three asks for
//!   the placement the file's own `/Rect` already gives, and would rank nothing.
//! - **Whether the page's `/MediaBox` starts at the origin**, because that is the one term of the
//!   placement §8.3.2.3's NOTE 1 makes optional: "the origin of default user space always
//!   corresponds to the lower-left corner of the output medium … it is not required". A corpus
//!   whose media boxes all start at (0, 0) cannot tell a reader that adds the media's own corner
//!   from one that does not.
//!
//! Every page is walked rather than the first: a watermark is drawn wherever it is stated.
//!
//! ```sh
//! cargo run --release -p pdf-model --example fixed_print_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_syntax::{Dictionary, Document, Object};

/// Everything this census keeps.
#[derive(Default)]
struct Census {
    /// Documents opened.
    documents: usize,
    /// Documents stating at least one watermark annotation.
    with_watermark: usize,
    /// Watermark annotations altogether.
    watermarks: usize,
    /// Of those, the ones stating Table 193's `/FixedPrint` as a dictionary.
    fixed_print: usize,
    /// Of those, the ones stating Table 194's `/Matrix`.
    with_matrix: usize,
    /// Of those, the ones stating Table 194's `/H` or `/V` as something other than zero.
    with_translation: usize,
    /// Of those, the ones on a page whose `/MediaBox` does not start at the origin.
    off_origin_media: usize,
    /// Of the watermarks, the ones carrying an appearance stream under Table 170's `/N`.
    with_appearance: usize,
    /// The documents stating a watermark, so that a round has a page to look at.
    named: Vec<String>,
}

impl Census {
    /// Walks every page of one document into the counters.
    fn count(&mut self, name: &str, document: &Document) {
        self.documents = self.documents.saturating_add(1);
        let pages = pdf_model::Pages::new(document);
        let mut here = 0usize;
        for index in 0..pages.len() {
            let Some(page) = pages.get(index) else {
                continue;
            };
            let annotations = document.get_key(&page.dict, "Annots");
            let Some(list) = annotations.as_array().map(<[Object]>::to_vec) else {
                continue;
            };
            for entry in &list {
                let resolved = document.resolve(entry);
                let Some(annotation) = resolved.as_dict() else {
                    continue;
                };
                if document
                    .get_key(annotation, "Subtype")
                    .as_name()
                    .is_some_and(|subtype| subtype.as_bytes() == b"Watermark")
                {
                    self.count_annotation(document, annotation, page.media_box);
                    here = here.saturating_add(1);
                }
            }
        }
        if here > 0 {
            self.with_watermark = self.with_watermark.saturating_add(1);
            self.named.push(format!("{name}={here}"));
        }
    }

    /// One watermark annotation, on a page whose media box is `media`.
    fn count_annotation(&mut self, document: &Document, annotation: &Dictionary, media: [f32; 4]) {
        self.watermarks = self.watermarks.saturating_add(1);
        if let Some(appearances) = document.get_key(annotation, "AP").as_dict()
            && matches!(document.get_key(appearances, "N"), Object::Stream(_))
        {
            self.with_appearance = self.with_appearance.saturating_add(1);
        }
        let fixed = document.get_key(annotation, "FixedPrint");
        let Some(fixed) = fixed.as_dict() else {
            return;
        };
        self.fixed_print = self.fixed_print.saturating_add(1);
        if document.get_key(fixed, "Matrix").as_array().is_some() {
            self.with_matrix = self.with_matrix.saturating_add(1);
        }
        let translation = |key: &'static str| document.get_key(fixed, key).as_number();
        if translation("H").is_some_and(|value| value != 0.0)
            || translation("V").is_some_and(|value| value != 0.0)
        {
            self.with_translation = self.with_translation.saturating_add(1);
        }
        if media[0] != 0.0 || media[1] != 0.0 {
            self.off_origin_media = self.off_origin_media.saturating_add(1);
        }
    }

    /// The measurement, which is this program's whole output.
    fn report(&self) {
        println!(
            "{} document(s) opened, {} stating a watermark annotation",
            self.documents, self.with_watermark
        );
        println!(
            "  {} watermark annotation(s), {} carrying an /AP /N stream",
            self.watermarks, self.with_appearance
        );
        println!("  {} state Table 193's /FixedPrint", self.fixed_print);
        println!(
            "  of those: {} state /Matrix, {} state a non-zero /H or /V, {} sit on a page whose \
             /MediaBox does not start at the origin",
            self.with_matrix, self.with_translation, self.off_origin_media
        );
        println!("  documents: {}", self.named.join(" "));
    }
}

fn main() {
    let mut census = Census::default();
    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        census.count(&name, &document);
    }
    census.report();
}
