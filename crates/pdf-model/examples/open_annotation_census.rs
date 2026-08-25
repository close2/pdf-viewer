//! §12.5.6.4's `/Open`, counted against §12.5.6.14's.
//!
//! Two entries carry the name `Open` and they are statements about two different objects:
//!
//! - Table 175 puts it on the **text annotation** — "[a] flag specifying whether the annotation
//!   shall initially be displayed open" — and §12.5.6.4 says what an open text annotation does:
//!   "when open, it shall display a popup window containing the text of the note".
//! - Table 186 puts it on the **popup annotation** — "whether the popup annotation shall
//!   initially be displayed open".
//!
//! Neither says the window shall be *closed*; each states a condition under which it is open. So
//! the population this counts is the one where the two differ: a text annotation asking for its
//! window whose popup does not ask for itself.
//!
//! ```sh
//! cargo run --release -p pdf-model --example open_annotation_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_syntax::{Dictionary, Document, Object};

/// What one document contributes to the census.
#[derive(Default)]
struct Counts {
    /// Annotations seen at all.
    seen: usize,
    /// `/Subtype /Text` annotations.
    text: usize,
    /// Of those, the ones stating Table 175's `/Open true`.
    text_open: usize,
    /// Of those, the ones naming a popup through Table 172's `/Popup`.
    text_open_with_popup: usize,
    /// Of those, the ones whose popup already states Table 186's `/Open true`.
    text_open_popup_open: usize,
    /// Popup annotations stating Table 186's `/Open true`.
    popup_open: usize,
    /// Annotations of another subtype stating an `/Open` no table gives them.
    open_elsewhere: usize,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.seen = self.seen.saturating_add(other.seen);
        self.text = self.text.saturating_add(other.text);
        self.text_open = self.text_open.saturating_add(other.text_open);
        self.text_open_with_popup = self
            .text_open_with_popup
            .saturating_add(other.text_open_with_popup);
        self.text_open_popup_open = self
            .text_open_popup_open
            .saturating_add(other.text_open_popup_open);
        self.popup_open = self.popup_open.saturating_add(other.popup_open);
        self.open_elsewhere = self.open_elsewhere.saturating_add(other.open_elsewhere);
    }
}

fn main() {
    let mut total = Counts::default();
    let mut opened = 0_usize;
    let mut lines: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let counts = document_counts(&document);
        // **`popup_open` is in this condition because it was in the totals and in nothing else.**
        // The census counted popups stating Table 186's `/Open true` and named no document
        // holding one, so a reader who wanted to *look* at an open window had a number and no
        // file — which is how a reason in `tools/state.sh` came to say "seven of the corpus's
        // documents state an open one" where the population is seven windows on far fewer.
        if counts.text_open > 0 || counts.open_elsewhere > 0 || counts.popup_open > 0 {
            let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
            lines.push(format!(
                "  {name}: {} /Text with /Open true ({} with a /Popup, {} whose popup is \
                 already open), {} popup(s) with /Open true, {} /Open on another subtype",
                counts.text_open,
                counts.text_open_with_popup,
                counts.text_open_popup_open,
                counts.popup_open,
                counts.open_elsewhere
            ));
        }
        total.absorb(&counts);
    }

    println!("{opened} document(s) opened, {} annotation(s)", total.seen);
    println!("  {} of subtype /Text", total.text);
    println!(
        "  {} of those state Table 175's /Open true",
        total.text_open
    );
    println!(
        "  {} of those name a /Popup, {} of which already state Table 186's /Open true",
        total.text_open_with_popup, total.text_open_popup_open
    );
    println!(
        "  {} popup(s) state Table 186's /Open true on their own",
        total.popup_open
    );
    println!(
        "  {} annotation(s) of another subtype state an /Open no table gives them",
        total.open_elsewhere
    );
    for line in &lines {
        println!("{line}");
    }
}

/// Walks every annotation on every page of one document.
fn document_counts(document: &Document) -> Counts {
    let mut counts = Counts::default();
    let pages = pdf_model::Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let entry = document.get_key(&page.dict, "Annots");
        let Some(list) = entry.as_array() else {
            continue;
        };
        for item in list {
            let object = document.resolve(item);
            let Some(annotation) = object.as_dict() else {
                continue;
            };
            counts.seen = counts.seen.saturating_add(1);
            let subtype = document.get_key(annotation, "Subtype");
            let subtype = subtype.as_name().map(|name| name.as_bytes().to_vec());
            let open = is_open(document, annotation);
            match subtype.as_deref() {
                Some(b"Text") => {
                    counts.text = counts.text.saturating_add(1);
                    if open {
                        counts.text_open = counts.text_open.saturating_add(1);
                        if let Some(popup) = popup_of(document, annotation) {
                            counts.text_open_with_popup =
                                counts.text_open_with_popup.saturating_add(1);
                            if is_open(document, &popup) {
                                counts.text_open_popup_open =
                                    counts.text_open_popup_open.saturating_add(1);
                            }
                        }
                    }
                }
                Some(b"Popup") => {
                    if open {
                        counts.popup_open = counts.popup_open.saturating_add(1);
                    }
                }
                _ => {
                    if open {
                        counts.open_elsewhere = counts.open_elsewhere.saturating_add(1);
                    }
                }
            }
        }
    }
    counts
}

/// Whether this dictionary states `/Open true`.
fn is_open(document: &Document, dict: &Dictionary) -> bool {
    document.get_key(dict, "Open") == Object::Boolean(true)
}

/// Table 172's `/Popup`, resolved.
fn popup_of(document: &Document, annotation: &Dictionary) -> Option<Dictionary> {
    document.get_key(annotation, "Popup").as_dict().cloned()
}
