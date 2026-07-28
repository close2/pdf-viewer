//! Renders one page with `hayro`, for the oracle's fourth opinion.
//!
//! ```text
//! pdfref-hayro <file.pdf> <page> <dpi> <out.png>
//! ```
//!
//! A separate program rather than a library call inside the harness, for the reason every
//! other reference is one: a corpus contains files written to make a reader loop, and a
//! thread cannot be cancelled. `pdfref::Reference::render_within` polls and kills, and that
//! only works on something with a process identifier.
//!
//! Found by `pdfref` next to the running executable, the same way `pdf-sandbox` finds its
//! worker and for the same reason: Cargo does not build one package's binaries when testing
//! another. See trap 10 in the handover.

#![forbid(unsafe_code)]
#![expect(
    clippy::print_stderr,
    reason = "a command-line tool: its diagnostics are its interface"
)]

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [pdf, page, dpi, out] = arguments.as_slice() else {
        eprintln!("usage: pdfref-hayro <file.pdf> <page> <dpi> <out.png>");
        return ExitCode::from(2);
    };

    let (Ok(page), Ok(dpi)) = (page.parse::<usize>(), dpi.parse::<f32>()) else {
        eprintln!("page must be a positive integer and dpi a number");
        return ExitCode::from(2);
    };

    match render(&PathBuf::from(pdf), page, dpi, &PathBuf::from(out)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("pdfref-hayro: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Renders `page` — one-based, as every other reference numbers them — at `dpi`.
fn render(pdf: &PathBuf, page: usize, dpi: f32, out: &PathBuf) -> Result<(), String> {
    use hayro::hayro_interpret::InterpreterSettings;
    use hayro::hayro_syntax::Pdf;
    use hayro::{RenderCache, RenderSettings, render};

    let bytes = std::fs::read(pdf).map_err(|error| format!("{}: {error}", pdf.display()))?;
    let document = Pdf::new(bytes).map_err(|error| format!("not a PDF: {error:?}"))?;

    let pages = document.pages();
    let index = page.checked_sub(1).ok_or("pages are numbered from one")?;
    let page = pages.get(index).ok_or_else(|| format!("no page {page}"))?;

    // The convention the harness uses for every renderer: 72 dpi is one pixel per PDF unit,
    // so the scale factor is dpi over 72.
    let scale = dpi / 72.0;
    let pixmap = render(
        page,
        &RenderCache::new(),
        &InterpreterSettings::default(),
        &RenderSettings {
            x_scale: scale,
            y_scale: scale,
            width: None,
            height: None,
            // White, because that is what a viewer shows and what the other three produce.
            // The default is transparent, which would compare a page against its own alpha.
            bg_color: hayro::vello_cpu::color::palette::css::WHITE,
        },
    );

    let png = pixmap
        .into_png()
        .map_err(|error| format!("encoding the render failed: {error}"))?;
    std::fs::write(out, png).map_err(|error| format!("{}: {error}", out.display()))
}
