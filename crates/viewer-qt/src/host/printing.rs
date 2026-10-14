//! RFC 0004's print path: the grant, the dialogue's job and paper, each page drawn on the
//! processor and painted by the C++ side (ADR 1180; filed here by ADR 1908).

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, Printing, Query};

use crate::bridge::ffi::{QtPrintCell, QtPrintJob};

use super::Host;

/// What the grant said about a print job, for the dialogue that opens on it.
pub(super) struct PrintJob {
    /// How many pages the document has.
    pages: usize,
    /// The page's own extent in §8.3.2.3's points, after §7.7.3.3's `/Rotate`.
    page: (f32, f32),
    /// §7.6.4.2's Table 22 bit 12, as the core answered it.
    fidelity: viewer_core::Fidelity,
    /// §12.2's Table 147, as the dialogue's opening state.
    defaults: viewer_host::PrintDefaults,
}

/// One printed page, rasterised and waiting for the C++ side to paint it.
pub(super) struct PrintedPage {
    /// Its pixels, or `None` where the page could not be drawn — the sentences say why.
    raster: Option<pdf_render::Raster>,
    /// What could not be drawn on it.
    reports: Vec<String>,
}

impl Host {
    /// The grant, on a window whose printing is a picture of what would print.
    ///
    /// §12.5.3's bit 3, §8.11.4.5's `Print` event and §12.5.6.22's sheet are already in force on
    /// every page this window draws, which is RFC 0004 §6's preview. The spool is ADR 1180's
    /// deferral and is named here rather than left as a key that appears to do nothing.
    pub(super) fn showing_what_would_print(
        &mut self,
        pages: usize,
        fidelity: viewer_core::Fidelity,
    ) {
        self.printing = true;
        let page = match self.viewer.query(Query::PageGeometry(0)) {
            Answer::Geometry(geometry) => (geometry.page.width, geometry.page.height),
            _ => (0.0, 0.0),
        };
        let defaults = match self.viewer.query(Query::Preferences) {
            Answer::Preferences(preferences) => viewer_host::PrintDefaults::of(&preferences),
            _ => viewer_host::PrintDefaults::of(
                &pdf_model::viewer_preferences::ViewerPreferences::default(),
            ),
        };
        if let Some(note) = defaults.enforcement_note() {
            self.say(note);
        }
        // §7.6.4.2's Table 22 bit 12, said before the dialogue rather than after a destination was
        // chosen — the same moment `viewer-gtk` says it, because it is the same sentence.
        if let Some(note) = viewer_host::printing::destination_refused(fidelity) {
            self.say(note);
        }
        self.print = Some(PrintJob {
            pages,
            page,
            fidelity,
            defaults,
        });
        self.update.print_dialogue = true;
        self.say(&format!(
            "printing, over {pages} page(s) — the pages on the screen are what would print; \
             press the key again to stop"
        ));
    }

    /// The job [`QtUpdate::print_dialogue`] announced, as a dialogue opens on it.
    ///
    /// [`QtUpdate::print_dialogue`]: crate::bridge::ffi::QtUpdate::print_dialogue
    pub(crate) fn print_job(&self) -> QtPrintJob {
        let Some(job) = self.print.as_ref() else {
            return empty_print_job();
        };
        let range = job.defaults.page_range.first().copied().unwrap_or((0, 0));
        QtPrintJob {
            pages: job.pages,
            page_width: job.page.0,
            page_height: job.page.1,
            degraded: job.fidelity == viewer_core::Fidelity::Degraded,
            refusal: viewer_host::printing::destination_refused(job.fidelity)
                .unwrap_or_default()
                .to_owned(),
            enforcement: job
                .defaults
                .enforcement_note()
                .unwrap_or_default()
                .to_owned(),
            copies: narrow(job.defaults.copies.unwrap_or_default()),
            duplex: match job.defaults.duplex {
                None => -1,
                Some(pdf_model::viewer_preferences::Duplex::Simplex) => 0,
                Some(pdf_model::viewer_preferences::Duplex::FlipLongEdge) => 1,
                Some(pdf_model::viewer_preferences::Duplex::FlipShortEdge) => 2,
            },
            // One-based, as Table 147's own NOTE insists, and the first pair only: a
            // `QPrintDialog` states one range and the entry may state several, so what a dialogue
            // opens on is the first of them and a person changes it (ADR 1180).
            from_page: narrow(range.0),
            to_page: narrow(range.1),
            no_scaling: job.defaults.scaling
                == pdf_model::viewer_preferences::PrintScaling::NoScaling,
        }
    }

    /// The paper and the arrangement a person chose in that dialogue — `Printing::Paper`.
    ///
    /// **This is where §12.5.6.22's matrix B stops being the identity in this program**: a page
    /// shrunk onto the sheet or placed in an n-up cell states the factor it was scaled by, and a
    /// fixed print watermark is drawn immune to it (ADR 1204).
    pub(crate) fn print_paper(
        &mut self,
        paper_width: f32,
        paper_height: f32,
        dpi: f32,
        scaling: u8,
        per_sheet: u8,
    ) {
        let Some(job) = self.print.as_ref() else {
            return;
        };
        let sheet = viewer_host::printing::placed(
            (paper_width, paper_height),
            job.page,
            scale_mode(scaling),
            pages_per_sheet(per_sheet),
            viewer_host::printing::scale_at(job.fidelity, Some(dpi)),
        );
        self.dispatch(Command::Print(Printing::Paper(sheet)));
    }

    /// Where each page of one sheet is painted, for the arrangement `print_paper` was given.
    pub(crate) fn print_cells(
        &self,
        paper_width: f32,
        paper_height: f32,
        scaling: u8,
        per_sheet: u8,
    ) -> Vec<QtPrintCell> {
        let Some(job) = self.print.as_ref() else {
            return Vec::new();
        };
        let per_sheet = pages_per_sheet(per_sheet);
        (0..per_sheet.count())
            .filter_map(|index| {
                viewer_host::printing::cell(
                    (paper_width, paper_height),
                    job.page,
                    scale_mode(scaling),
                    per_sheet,
                    index,
                )
            })
            .map(|cell| QtPrintCell {
                x: cell.origin.0,
                y: cell.origin.1,
                width: cell.extent.0,
                height: cell.extent.1,
            })
            .collect()
    }

    /// Draws one page of the job on the processor — the backend this project's oracle is.
    ///
    /// `[width, height]` in samples, `[0, 0]` where the page is not this document's to print or
    /// its marks will not go onto a raster at the job's resolution. Every such refusal is a
    /// sentence in [`Self::print_reports`] rather than a blank sheet (trap 5).
    pub(crate) fn print_page(&mut self, page: usize) -> Vec<u32> {
        self.printed = None;
        let Answer::PrintPage(printed) = self.viewer.query(Query::PrintPage(page)) else {
            return vec![0, 0];
        };
        let (target, list, mut reports) = (printed.target, printed.list, printed.reports);
        let Some(target) = target else {
            self.printed = Some(PrintedPage {
                raster: None,
                reports,
            });
            return vec![0, 0];
        };
        match CpuRasterizer::new().rasterize(&list, target) {
            Ok(raster) => {
                let size = vec![raster.width, raster.height];
                self.printed = Some(PrintedPage {
                    raster: Some(raster),
                    reports,
                });
                size
            }
            Err(refusal) => {
                reports.push(format!(
                    "page {} did not print: {refusal}",
                    page.saturating_add(1)
                ));
                self.printed = Some(PrintedPage {
                    raster: None,
                    reports,
                });
                vec![0, 0]
            }
        }
    }

    /// That page's pixels, row-major RGBA8 with no padding — the layout every raster here has.
    pub(crate) fn print_page_pixels(&self) -> &[u8] {
        self.printed
            .as_ref()
            .and_then(|printed| printed.raster.as_ref())
            .map_or(&[], |raster| raster.data.as_slice())
    }

    /// What could not be drawn on it, one sentence apiece.
    pub(crate) fn print_reports(&self) -> Vec<String> {
        self.printed
            .as_ref()
            .map(|printed| printed.reports.clone())
            .unwrap_or_default()
    }

    /// The end of the operation — §8.11.4.5's "then all groups shall revert to their prior
    /// states". Harmless where none is running, which is what lets the window send it from
    /// wherever its job actually ended.
    pub(crate) fn print_finish(&mut self) {
        self.dispatch(Command::Print(Printing::Finish));
        self.printing = false;
        self.print = None;
        self.printed = None;
    }
}

/// A job that is not running, which is what a window gets if it asks at any other moment.
fn empty_print_job() -> QtPrintJob {
    QtPrintJob {
        pages: 0,
        page_width: 0.0,
        page_height: 0.0,
        degraded: false,
        refusal: String::new(),
        enforcement: String::new(),
        copies: 0,
        duplex: -1,
        from_page: 0,
        to_page: 0,
        no_scaling: false,
    }
}

/// RFC 0004 §6's scale modes, as the bridge numbers them: 0 actual size, 1 shrink, 2 fit.
///
/// Anything else is the default, because a number this side does not know is a dialogue saying
/// nothing rather than asking for something strange.
fn scale_mode(code: u8) -> viewer_host::printing::Scaling {
    match code {
        0 => viewer_host::printing::Scaling::ActualSize,
        2 => viewer_host::printing::Scaling::FitToPage,
        _ => viewer_host::printing::Scaling::ShrinkToFit,
    }
}

/// RFC 0004 §6's 1, 2 and 4, as the bridge numbers them — the count itself.
fn pages_per_sheet(count: u8) -> viewer_host::printing::PagesPerSheet {
    match count {
        2 => viewer_host::printing::PagesPerSheet::Two,
        4 => viewer_host::printing::PagesPerSheet::Four,
        _ => viewer_host::printing::PagesPerSheet::One,
    }
}

/// A one-based page number or a copy count as the `i32` a `QPrintDialog` takes.
///
/// A number that does not fit is `0`, which is this bridge's "the document stated none": a
/// document asking for four billion copies has said nothing a dialogue can open on.
fn narrow(value: i64) -> i32 {
    i32::try_from(value).unwrap_or_default()
}
