//! How much of a page §6.3.2.2's instruction takes off it, over the corpus.
//!
//! A host that draws §12.7's controls itself asks for the page without the widgets' appearances
//! ([`pdf_model::view::WidgetAppearances`], ADR 0245). Two questions follow and neither can be
//! answered by reading the clause: how many real documents have anything to delegate at all, and
//! how much of a page it is when they do.
//!
//! One line per document that has a delegable widget on its first page — the fields, the widgets,
//! and the display list's length with the appearances and without — then a summary.
//!
//! ```sh
//! cargo run --release -p pdf-model --example delegated_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_model::content::interpret_with;
use pdf_model::view::{ViewState, WidgetAppearances};
use pdf_model::{Pages, form};
use pdf_syntax::Document;

fn main() {
    let (mut opened, mut with_widgets, mut widgets, mut removed) =
        (0_usize, 0_usize, 0_usize, 0_usize);
    let mut worst: Option<(usize, String)> = None;
    for path in std::env::args().skip(1) {
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let Some(page) = Pages::new(&document).get(0) else {
            continue;
        };
        let mut view = ViewState::of(&document);
        let delegable = form::delegated_widgets(&document, &page, &view);
        if delegable.is_empty() {
            continue;
        }
        let fields = form::fields(&document, &page, &view).len();
        let drawn = interpret_with(&document, &page, &view)
            .display_list
            .commands()
            .len();
        view.set_widget_appearances(WidgetAppearances::Delegated);
        let delegated = interpret_with(&document, &page, &view)
            .display_list
            .commands()
            .len();

        with_widgets = with_widgets.saturating_add(1);
        widgets = widgets.saturating_add(delegable.len());
        let gone = drawn.saturating_sub(delegated);
        removed = removed.saturating_add(gone);
        if worst.as_ref().is_none_or(|(most, _)| gone > *most) {
            worst = Some((gone, name.clone()));
        }
        println!(
            "{name}\t{fields} field(s)\t{} widget(s)\t{drawn} -> {delegated} command(s)",
            delegable.len()
        );
    }
    println!(
        "# {opened} document(s) opened; {with_widgets} have a delegable widget on page one, \
         {widgets} widget(s) in all, {removed} display-list command(s) removed"
    );
    if let Some((gone, name)) = worst {
        println!("# most removed from one page: {name}, {gone} command(s)");
    }
}
