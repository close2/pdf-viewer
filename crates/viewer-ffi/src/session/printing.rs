//! The granted print operation and its retained page raster.

use super::{PrintedPage, Session};
use crate::events::Events;
use crate::status::Status;
use pdf_render::Rasterizer as _;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, Query};

impl Session {
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
        let (target, reports) = (printed.target, printed.reports);
        let Some(target) = target else {
            self.printed = None;
            return Err(Status::RenderRefused);
        };
        let raster = CpuRasterizer::new()
            .rasterize(&printed.list, target)
            .map_err(|_| Status::RenderRefused)?;
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
