//! What a markup annotation offers §12.5.6.14's popup window to display.
//!
//! Table 172 gives a markup annotation two ways to say what its popup shows. Table 166's
//! `/Contents` is a plain text string and this tree draws it (ADR 0191); `/RC` is "[a] rich text
//! string … that shall be displayed in the popup window when the annotation is opened", in the
//! XFA rich-text format `CLAUDE.md` excludes by name — and NOTE 1 says the two are "expected" to
//! be textually equivalent where both are present.
//!
//! So the question that decides whether the exclusion costs anything is **how many annotations
//! state `/RC` and no `/Contents`**, and it had never been counted. This counts it, over every
//! page rather than page one: a popup is opened by a click, and a click can be on page 40.
//!
//! ```sh
//! cargo run --release -p pdf-model --example markup_text_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_syntax::{Dictionary, Document, Object};

/// How many pages of one document are walked, so that a thousand-page file cannot dominate.
const MAX_PAGES: usize = 200;

/// Every counter this census keeps, so that `main` walks and [`Census::report`] prints.
#[derive(Default)]
struct Census {
    documents: usize,
    with_markup: usize,
    markups: usize,
    with_popup: usize,
    with_contents: usize,
    with_rich: usize,
    rich_only: usize,
    rich_named: Vec<String>,
    samples: Vec<String>,
    windows: usize,
    windows_with_text: usize,
    /// Table 177 states `/RC` a second time, on a free text annotation, where it "shall be used
    /// to generate the appearance of the annotation" rather than a popup window's contents. That
    /// is a different `shall` with a different denominator, so it is counted apart (ADR 0224).
    free_text_rich: usize,
    free_text_rich_only: usize,
    free_text_rich_named: Vec<String>,
}

impl Census {
    /// Walks one document's first [`MAX_PAGES`] pages into the counters.
    fn count(&mut self, name: &str, document: &Document) {
        self.documents = self.documents.saturating_add(1);
        let pages = pdf_model::Pages::new(document);
        let view = pdf_model::view::ViewState::of(document);
        let mut any = false;
        let mut rich_here = false;
        for index in 0..pages.len().min(MAX_PAGES) {
            let Some(page) = pages.get(index) else {
                continue;
            };
            // What the window would actually show, through the reader a host asks: the count
            // above is what the *file* states and this is what reaches a person.
            for window in pdf_model::popup::popups(document, &page, &view) {
                self.windows = self.windows.saturating_add(1);
                if window.text.is_some() {
                    self.windows_with_text = self.windows_with_text.saturating_add(1);
                }
            }
            let annotations = document.get_key(&page.dict, "Annots");
            let Some(list) = annotations.as_array().map(<[Object]>::to_vec) else {
                continue;
            };
            for entry in &list {
                let resolved = document.resolve(entry);
                let Some(annotation) = resolved.as_dict() else {
                    continue;
                };
                if self.count_annotation(name, document, annotation) {
                    any = true;
                    rich_here |= text(document, annotation, "RC").is_some();
                }
            }
        }
        if any {
            self.with_markup = self.with_markup.saturating_add(1);
        }
        if rich_here {
            self.rich_named.push(name.to_owned());
        }
    }

    /// One annotation. `false` where it states nothing this census counts.
    fn count_annotation(
        &mut self,
        name: &str,
        document: &Document,
        annotation: &Dictionary,
    ) -> bool {
        // §12.5.6.2's markup annotations are the ones Table 171 marks as having a popup; the
        // practical test is the entries themselves, because a file that states `/RC` on a subtype
        // the table does not list has still stated it.
        let contents = text(document, annotation, "Contents");
        let rich = text(document, annotation, "RC");
        let popup = document.get_key(annotation, "Popup").as_dict().is_some();
        if contents.is_none() && rich.is_none() && !popup {
            return false;
        }
        self.markups = self.markups.saturating_add(1);
        if popup {
            self.with_popup = self.with_popup.saturating_add(1);
        }
        if contents.is_some() {
            self.with_contents = self.with_contents.saturating_add(1);
        }
        let subtype = document.get_key(annotation, "Subtype");
        let free_text =
            subtype.as_name().map(pdf_syntax::Name::as_bytes) == Some(b"FreeText".as_ref());
        if let Some(rich) = rich {
            self.with_rich = self.with_rich.saturating_add(1);
            if free_text {
                self.free_text_rich = self.free_text_rich.saturating_add(1);
                if !self.free_text_rich_named.iter().any(|seen| seen == name) {
                    self.free_text_rich_named.push(name.to_owned());
                }
            }
            if contents.is_none() {
                self.rich_only = self.rich_only.saturating_add(1);
                if free_text {
                    self.free_text_rich_only = self.free_text_rich_only.saturating_add(1);
                }
                if self.samples.len() < 6 {
                    self.samples.push(format!("{name}: {}", trimmed(&rich)));
                }
            }
        }
        true
    }

    /// The measurement, which is this program's whole output.
    fn report(&self) {
        println!(
            "{} document(s) opened, {} with a markup annotation",
            self.documents, self.with_markup
        );
        println!(
            "  {} annotation(s) stating /Contents, /RC or a /Popup",
            self.markups
        );
        println!("  {} state a /Popup", self.with_popup);
        println!("  {} state /Contents", self.with_contents);
        println!("  {} state Table 172's /RC", self.with_rich);
        println!(
            "  **{} state /RC and no /Contents** — what the XFA exclusion costs",
            self.rich_only
        );
        println!(
            "  {} popup window(s) displayed, {} with text in them — which is what a person sees",
            self.windows, self.windows_with_text
        );
        println!(
            "  of those, {} are free text annotations stating Table 177's /RC, **{} of them with \
             no /Contents** — what §12.5.6.6's own `shall` costs",
            self.free_text_rich, self.free_text_rich_only
        );
        println!(
            "  documents with a free text /RC: {}",
            self.free_text_rich_named.join(" ")
        );
        println!("  documents stating /RC: {}", self.rich_named.join(" "));
        for sample in &self.samples {
            println!("  sample: {sample}");
        }
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

/// One text string entry, decoded, empty treated as absent.
fn text(document: &Document, annotation: &Dictionary, key: &str) -> Option<String> {
    let value = document.get_key(annotation, key);
    let bytes: Vec<u8> = match &value {
        Object::String(bytes) => bytes.to_vec(),
        Object::Stream(stream) => document.decoded_stream_data(stream)?.to_vec(),
        _ => return None,
    };
    let decoded = pdf_syntax::text_string::text_string(&bytes);
    (!decoded.trim().is_empty()).then_some(decoded)
}

/// The first line of a value, for a sample line that has to fit on a terminal.
fn trimmed(value: &str) -> String {
    let flat: String = value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    flat.chars().take(150).collect()
}
