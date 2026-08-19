//! §12.5.6.6's annotations as the corpus states them, and what Table 167 says about editing one.
//!
//! Written for one question, in `doc/todo/33`: this program can add a free text annotation and
//! type into it, and it could not touch one the *file* states. Whether that is worth building is a
//! question about how many such annotations exist and what they carry, and three of the four
//! numbers below decide the shape of the answer rather than its size:
//!
//! - **How many** free text annotations there are, and in how many documents.
//! - **How many carry an appearance stream.** §12.5.5 has a reader draw the stored stream, so an
//!   annotation whose `/Contents` changed and whose `/AP` did not is one that goes on showing the
//!   producer's text. The count says whether the regeneration path is the common case or the rare
//!   one.
//! - **What Table 167 says about each.** Bit 10 `LockedContents` is "do not allow the contents of
//!   the annotation to be modified by the user", and bit 8 `Locked` is the one that sounds like it
//!   and is not — its own row ends "this flag does not restrict changes to the annotation's
//!   contents". Bit 7 `ReadOnly` is "do not allow the annotation to interact with the user". A
//!   count of each says which of the three a corpus document would actually exercise.
//! - **How many state Table 177's `/CL`**, the callout line — drawn since the
//!   four-hundred-and-ninety-fourth session (ADR 0329), with this count (0 of 73) as the reason
//!   its fixtures are hand-built pairs rather than corpus pages.
//! - **What each says about its border**, which is a fifth question and the one trap 11 asks.
//!   §12.5.4's "[i]f neither the Border nor the BS entry is present, the border shall be drawn as
//!   a solid line with a width of 1 point" is a `shall` that fires on a file which said nothing,
//!   so the population of a border report is *not* "documents stating `/BS`". The split below is
//!   the clause's own three cases — a stated `/BS`, a stated `/Border`, and neither — counted
//!   only over the annotations an appearance is *constructed* for, because §12.5.2 has a reader
//!   "ignore the values of the C, IC, Border, BS, BE …" wherever an appearance stream exists.
//!
//! Every page is walked rather than the first: a note is typed into by a click, and a click can be
//! on page 40.
//!
//! ```sh
//! cargo run --release -p pdf-model --example free_text_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_syntax::{Dictionary, Document, Object};

/// Table 167 bit 7, "do not allow the annotation to interact with the user".
const FLAG_READ_ONLY: i64 = 1 << 6;

/// Table 167 bit 8, which restricts an annotation's *properties* and says so.
const FLAG_LOCKED: i64 = 1 << 7;

/// Table 167 bit 10, "do not allow the contents of the annotation to be modified by the user".
const FLAG_LOCKED_CONTENTS: i64 = 1 << 9;

/// Everything this census keeps.
#[derive(Default)]
struct Census {
    /// Documents opened.
    documents: usize,
    /// Documents stating at least one free text annotation.
    with_free_text: usize,
    /// Free text annotations altogether.
    annotations: usize,
    /// Of those, the ones carrying an appearance stream under Table 170's `/N`.
    with_appearance: usize,
    /// Of those, the ones whose appearance stream holds §12.7.4.3's `/Tx` marked content.
    with_marked_text: usize,
    /// Of those, the ones stating Table 166's `/Contents` with something in it.
    with_contents: usize,
    /// Table 167 bit 7.
    read_only: usize,
    /// Table 167 bit 8.
    locked: usize,
    /// Table 167 bit 10.
    locked_contents: usize,
    /// Table 177's `/CL`, the callout line.
    callout: usize,
    /// Of the annotations with no `/AP` `/N` stream — the ones an appearance is constructed for —
    /// those stating Table 177's `/BS`.
    constructed_style: usize,
    /// Of the same population, those stating Table 166's `/Border` and no `/BS`.
    constructed_border: usize,
    /// Of the same population, those stating neither, where §12.5.4's one-point default fires.
    constructed_default: usize,
    /// Of the same population, those whose border width works out greater than zero — which is
    /// what a border report actually matches.
    constructed_bordered: usize,
    /// The documents that hold one of those, so that a round can look at the page.
    bordered: Vec<String>,
    /// The documents that state one, so that a round has somewhere to click.
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
                    .is_some_and(|subtype| subtype.as_bytes() == b"FreeText")
                {
                    self.count_annotation(name, document, annotation);
                    here = here.saturating_add(1);
                }
            }
        }
        if here > 0 {
            self.with_free_text = self.with_free_text.saturating_add(1);
            self.named.push(format!("{name}={here}"));
        }
    }

    /// One free text annotation.
    fn count_annotation(&mut self, name: &str, document: &Document, annotation: &Dictionary) {
        self.annotations = self.annotations.saturating_add(1);
        let flags = document
            .get_key(annotation, "F")
            .as_integer()
            .unwrap_or_default();
        if flags & FLAG_READ_ONLY != 0 {
            self.read_only = self.read_only.saturating_add(1);
        }
        if flags & FLAG_LOCKED != 0 {
            self.locked = self.locked.saturating_add(1);
        }
        if flags & FLAG_LOCKED_CONTENTS != 0 {
            self.locked_contents = self.locked_contents.saturating_add(1);
        }
        if document
            .get_key(annotation, "CL")
            .as_array()
            .is_some_and(|line| !line.is_empty())
        {
            self.callout = self.callout.saturating_add(1);
        }
        let contents = document.get_key(annotation, "Contents");
        if matches!(&contents, Object::String(bytes) if !bytes.is_empty()) {
            self.with_contents = self.with_contents.saturating_add(1);
        }
        let appearances = document.get_key(annotation, "AP");
        let normal = appearances
            .as_dict()
            .map(|dict| document.get_key(dict, "N"));
        let Some(Object::Stream(stream)) = normal else {
            // §12.5.2 has a reader "ignore the values of the C, IC, Border, BS, BE …" wherever an
            // appearance stream exists, so the border question below is only asked of the
            // annotations this program constructs an appearance for.
            self.count_border(name, document, annotation);
            return;
        };
        let stream = &stream;
        self.with_appearance = self.with_appearance.saturating_add(1);
        // §12.7.4.3 has a regenerating processor "replace the existing contents of the appearance
        // stream from … BMC to the matching EMC", and §12.5.6.6 sends this subtype to that
        // subclause — so whether the region exists decides whether the rewrite is a splice or an
        // append, and both are the clause's own two cases.
        if let Some(data) = document.decoded_stream_data(stream)
            && data.windows(3).any(|window| window == b"BMC")
        {
            self.with_marked_text = self.with_marked_text.saturating_add(1);
        }
    }

    /// §12.5.4's three cases for one annotation an appearance is constructed for.
    ///
    /// The precedence is Table 166's — "[i]f an annotation dictionary includes the BS entry, then
    /// the Border entry is ignored" — and the width is Table 168's `/W` (default 1), Table 166's
    /// third element (default 1), or §12.5.4's sentence for a file that states neither.
    fn count_border(&mut self, name: &str, document: &Document, annotation: &Dictionary) {
        let width = if let Some(style) = document.get_key(annotation, "BS").as_dict() {
            self.constructed_style = self.constructed_style.saturating_add(1);
            document.get_key(style, "W").as_number().unwrap_or(1.0)
        } else if let Some(border) = document.get_key(annotation, "Border").as_array() {
            self.constructed_border = self.constructed_border.saturating_add(1);
            border
                .get(2)
                .and_then(|item| document.resolve(item).as_number())
                .unwrap_or(1.0)
        } else {
            self.constructed_default = self.constructed_default.saturating_add(1);
            1.0
        };
        if width > 0.0 {
            self.constructed_bordered = self.constructed_bordered.saturating_add(1);
            if self.bordered.last().map(String::as_str) != Some(name) {
                self.bordered.push(name.to_owned());
            }
        }
    }

    /// The measurement, which is this program's whole output.
    fn report(&self) {
        println!(
            "{} document(s) opened, {} stating a free text annotation",
            self.documents, self.with_free_text
        );
        println!("  {} free text annotation(s)", self.annotations);
        println!(
            "  {} state /Contents with something in it",
            self.with_contents
        );
        println!(
            "  {} carry an /AP /N stream, {} of which hold a BMC marked-content region",
            self.with_appearance, self.with_marked_text
        );
        println!(
            "  Table 167: {} ReadOnly (bit 7), {} Locked (bit 8), {} LockedContents (bit 10)",
            self.read_only, self.locked, self.locked_contents
        );
        println!("  {} state Table 177's /CL callout line", self.callout);
        println!(
            "  of the {} with no /AP /N stream: {} state /BS, {} state /Border only, {} state \
             neither (§12.5.4's one-point default)",
            self.constructed_style
                .saturating_add(self.constructed_border)
                .saturating_add(self.constructed_default),
            self.constructed_style,
            self.constructed_border,
            self.constructed_default
        );
        println!(
            "  {} of those work out to a border width above zero, in {} document(s): {}",
            self.constructed_bordered,
            self.bordered.len(),
            self.bordered.join(" ")
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
