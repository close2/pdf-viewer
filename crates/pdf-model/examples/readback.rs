//! What one page reads back as, which is §9.10.2's answer for every code it showed.
//!
//! The text gate measures *recall* — how many of `pdftotext`'s words this tree produces — and is
//! silent about the other direction. A rule that invents text would not move it, so a round that
//! adds one needs this: the readback itself, beside the reference's, for a person to read.
//!
//! ```sh
//! cargo run --release -p pdf-model --example readback -- file.pdf [page]
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the readback"
)]

fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        println!("usage: readback <file.pdf> [page]");
        return;
    };
    let page: usize = arguments
        .next()
        .and_then(|index| index.parse().ok())
        .unwrap_or(1);
    let Ok(bytes) = std::fs::read(&path) else {
        println!("cannot read {path}");
        return;
    };
    let Ok(document) = pdf_syntax::Document::open(bytes) else {
        println!("cannot open {path}");
        return;
    };
    let pages = pdf_model::Pages::new(&document);
    let Some(page) = pages.get(page.saturating_sub(1)) else {
        println!("no such page");
        return;
    };
    let interpretation = pdf_model::content::interpret(&document, &page);
    println!("{}", interpretation.text);
    // What the readback does *not* contain, which reading it cannot show: a page whose fonts
    // §9.10.2 cannot name prints an empty line and looks exactly like a page with no text.
    println!(
        "--- {} glyph(s) marked the page; {} code(s) §9.10.2 could not name",
        interpretation.glyphs, interpretation.codes_without_a_character
    );
}
