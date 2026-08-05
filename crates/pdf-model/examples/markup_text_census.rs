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

fn main() {
    let mut documents = 0_usize;
    let mut with_markup = 0_usize;
    let mut markups = 0_usize;
    let mut with_popup = 0_usize;
    let mut with_contents = 0_usize;
    let mut with_rich = 0_usize;
    let mut rich_only = 0_usize;
    let mut rich_named: Vec<String> = Vec::new();
    let mut samples: Vec<String> = Vec::new();
    let mut windows = 0_usize;
    let mut windows_with_text = 0_usize;

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        let pages = pdf_model::Pages::new(&document);
        let view = pdf_model::view::ViewState::of(&document);
        let mut any = false;
        let mut rich_here = false;
        for index in 0..pages.len().min(MAX_PAGES) {
            let Some(page) = pages.get(index) else {
                continue;
            };
            // What the window would actually show, through the reader a host asks: the count
            // above is what the *file* states and this is what reaches a person.
            for window in pdf_model::popup::popups(&document, &page, &view) {
                windows = windows.saturating_add(1);
                if window.text.is_some() {
                    windows_with_text = windows_with_text.saturating_add(1);
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
                // §12.5.6.2's markup annotations are the ones Table 171 marks as having a popup;
                // the practical test is the entries themselves, because a file that states `/RC`
                // on a subtype the table does not list has still stated it.
                let contents = text(&document, annotation, "Contents");
                let rich = text(&document, annotation, "RC");
                let popup = document.get_key(annotation, "Popup").as_dict().is_some();
                if contents.is_none() && rich.is_none() && !popup {
                    continue;
                }
                any = true;
                markups = markups.saturating_add(1);
                if popup {
                    with_popup = with_popup.saturating_add(1);
                }
                if contents.is_some() {
                    with_contents = with_contents.saturating_add(1);
                }
                if let Some(rich) = rich {
                    with_rich = with_rich.saturating_add(1);
                    rich_here = true;
                    if contents.is_none() {
                        rich_only = rich_only.saturating_add(1);
                        if samples.len() < 6 {
                            samples.push(format!("{name}: {}", trimmed(&rich)));
                        }
                    }
                }
            }
        }
        if any {
            with_markup = with_markup.saturating_add(1);
        }
        if rich_here {
            rich_named.push(name);
        }
    }

    println!("{documents} document(s) opened, {with_markup} with a markup annotation");
    println!("  {markups} annotation(s) stating /Contents, /RC or a /Popup");
    println!("  {with_popup} state a /Popup");
    println!("  {with_contents} state /Contents");
    println!("  {with_rich} state Table 172's /RC");
    println!("  **{rich_only} state /RC and no /Contents** — what the XFA exclusion costs");
    println!(
        "  {windows} popup window(s) displayed, {windows_with_text} with text in them — which is \
         what a person sees"
    );
    println!("  documents stating /RC: {}", rich_named.join(" "));
    for sample in &samples {
        println!("  sample: {sample}");
    }
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
