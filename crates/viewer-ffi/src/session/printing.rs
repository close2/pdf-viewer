//! The granted print operation and its retained page raster.

use super::{PrintedPage, Session};
use crate::events::Events;
use crate::status::Status;
use pdf_render::Rasterizer as _;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, Query};

impl Session {
    /// How large one page of the print operation is, in default user space units, before it is
    /// drawn — `viewer_core::Query::PrintSize`.
    ///
    /// §12.2, Table 147's `/PickTrayByPDFSize` states "whether the PDF page size shall be used to
    /// select the input paper tray", so a caller honouring it chooses each sheet from this and
    /// states it with `quorra_print_placed` before drawing the page onto it (ADR 2919).
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no print operation is running or the document has no such page.
    pub fn print_size(&self, page: usize) -> Result<(f32, f32), Status> {
        match self.viewer.query(Query::PrintSize(page)) {
            Answer::PrintSize(size) => Ok((size.width, size.height)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// One page of the print operation `Session::print` began, rasterised on the processor.
    ///
    /// **The rasteriser is here for [`super::rasterise`]'s reason**: `viewer_core::Query::PrintPage`
    /// answers with a display list, which is clauses 8 and 9 in a data structure and not a thing
    /// to put in a header, and what a C caller wants from a printed page is pixels. It is the CPU
    /// backend rather than a device because that backend is this project's correctness oracle,
    /// which is RFC 0004's whole argument for rendering the printed page ourselves.
    ///
    /// The pixels wait here until [`Session::print_page_copy`] takes them.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no print operation is running or the document has no such page,
    /// and [`Status::RenderRefused`] where this page's marks will not go onto a raster at the
    /// job's resolution — the reason is among the page's reports either way.
    pub fn print_page(&mut self, page: usize) -> Result<(u32, u32, usize), Status> {
        let Answer::PrintPage(printed) = self.viewer.query(Query::PrintPage(page)) else {
            return Err(Status::NoAnswer);
        };
        let Some(target) = printed.target else {
            self.printed = None;
            return Err(Status::RenderRefused);
        };
        let raster = CpuRasterizer::new()
            .rasterize(&printed.list, target)
            .map_err(|_| Status::RenderRefused)?;
        // A type 1 shading's grid is evaluated in that draw, after the reports were worded, so
        // what its programs met is asked of the page now (ADR 3122).
        let faults = printed.faults_after_drawing();
        let mut reports = printed.reports;
        reports.extend(faults);
        let size = (raster.width, raster.height, raster.data.len());
        self.printed = Some(PrintedPage {
            page,
            raster,
            reports,
        });
        Ok(size)
    }

    /// Copies the page [`Session::print_page`] drew into a buffer the caller owns.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no page has been drawn or the one drawn is a different page, and
    /// [`Status::BufferTooSmall`] where `into` is shorter than that call said.
    pub fn print_page_copy(&self, page: usize, into: &mut [u8]) -> Result<usize, Status> {
        let Some(printed) = self.printed.as_ref().filter(|held| held.page == page) else {
            return Err(Status::NoAnswer);
        };
        let data = &printed.raster.data;
        let Some(room) = into.get_mut(..data.len()) else {
            return Err(Status::BufferTooSmall);
        };
        room.copy_from_slice(data);
        Ok(data.len())
    }

    /// What could not be drawn on the page [`Session::print_page`] drew, one sentence apiece.
    ///
    /// Trap 5's channel for a printed page: a page that goes to the printer with something
    /// missing says so, rather than coming out short in silence.
    #[must_use]
    pub fn printed_reports(&self) -> &[String] {
        self.printed
            .as_ref()
            .map_or(&[] as &[String], |printed| &printed.reports)
    }

    /// §7.6.4.2's bit 3 asked as an operation, and the print intent if it is granted.
    #[must_use]
    pub fn print(&mut self, printing: viewer_core::Printing) -> Events {
        self.handle(Command::Print(printing))
    }
}
