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
                    self.count_annotation(document, annotation);
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
    fn count_annotation(&mut self, document: &Document, annotation: &Dictionary) {
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
        let Some(normal) = appearances
            .as_dict()
            .map(|dict| document.get_key(dict, "N"))
        else {
            return;
        };
        let Object::Stream(stream) = &normal else {
            return;
        };
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
