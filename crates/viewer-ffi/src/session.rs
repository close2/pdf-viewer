//! The viewer, wrapped so that every entry point has one safe function to call.
//!
//! [`crate::abi`]'s job is to turn raw pointers into these arguments and nothing else; every
//! decision about what a command *means* is here, in safe Rust, where the rest of the tree's
//! rules apply unchanged.
//!
//! **Nothing in this module re-decides anything `viewer-core` says.** A method here is a
//! [`viewer_core::Command`] built from C-shaped arguments, or a [`viewer_core::Query`] whose
//! answer is copied out of the borrow before it is handed on.

use pdf_render::Rasterizer as _;
use pdf_render::{Raster, RasterFormat};
use render_cpu::CpuRasterizer;
use viewer_core::{
    Answer, Command, DocumentId, Event, PageTarget, Query, RenderRequest, Rendered, Viewer, Zoom,
};

use crate::events::Events;
use crate::panels::Outline;
use crate::status::Status;

/// One viewer, and the whole of what a `pdfv_viewer *` points at.
///
/// A plain wrapper with no interior mutability, because C owns it: every entry point takes it as
/// `*mut` and the caller is the one holding it. That is the ownership `viewer-qt` found to be
/// simpler than `viewer-gtk`'s `Rc<RefCell<…>>` (ADR 0246, section 3), arrived at here for the same
/// reason — the callbacks are the caller's, not ours.
#[derive(Debug)]
pub struct Session {
    /// The state machine.
    viewer: Viewer,
}

impl Session {
    /// A viewer for a viewport of this size.
    ///
    /// The scale is device pixels per logical pixel, which is 2.0 on a doubled display.
    /// [`Viewer::new`] takes it because a page must be rasterised at the resolution it will be
    /// shown at.
    #[must_use]
    pub fn new(width: u32, height: u32, scale: f32) -> Self {
        Self {
            viewer: Viewer::new(width, height, scale),
        }
    }

    /// Sends one command and collects everything it produced.
    ///
    /// The one place a command reaches the viewer, so that "the borrow ends before the caller
    /// sees anything" is a property of one function rather than of every entry point.
    fn handle(&mut self, command: Command) -> Events {
        Events::new(self.viewer.handle(command).collect::<Vec<Event>>())
    }

    /// §7.6.4.1 and Annex O: opens a document from bytes the caller owns.
    #[must_use]
    pub fn open(
        &mut self,
        id: u64,
        bytes: Vec<u8>,
        password: Option<String>,
        fragment: Option<String>,
    ) -> Events {
        self.handle(Command::Open {
            id: DocumentId(id),
            bytes,
            password,
            fragment,
        })
    }

    /// Closes a document and forgets everything derived from it.
    #[must_use]
    pub fn close(&mut self, id: u64) -> Events {
        self.handle(Command::Close(DocumentId(id)))
    }

    /// Makes an already-open document the one commands apply to.
    #[must_use]
    pub fn focus(&mut self, id: u64) -> Events {
        self.handle(Command::Focus(DocumentId(id)))
    }

    /// The viewport changed size, or moved to a display with a different scale.
    #[must_use]
    pub fn resize(&mut self, width: u32, height: u32, scale: f32) -> Events {
        self.handle(Command::Resize {
            width,
            height,
            scale,
        })
    }

    /// Shows another page.
    #[must_use]
    pub fn go_to(&mut self, target: PageTarget) -> Events {
        self.handle(Command::GoTo(target))
    }

    /// Changes the magnification, holding the viewport's centre still.
    ///
    /// The point [`Command::Zoom`] can hold still is deliberately not in this ABI yet: it is a
    /// wheel zoom's anchor, and a caller that has one has a pointer, which is the next thing this
    /// crate owes. `None` is the centre, which is what a keyboard `+` means (ADR 0166).
    #[must_use]
    pub fn zoom(&mut self, zoom: Zoom) -> Events {
        self.handle(Command::Zoom { zoom, at: None })
    }

    /// Moves the page under the viewport by a device-pixel delta.
    #[must_use]
    pub fn scroll(&mut self, dx: f32, dy: f32) -> Events {
        self.handle(Command::Scroll { dx, dy })
    }

    /// §12.3.3: activates an object the caller is showing outside the page — an outline row.
    ///
    /// The object rather than a page number, because the clause says clicking an item makes the
    /// processor "jump to a destination **or** trigger an action associated with the item", and
    /// which of the two it is belongs to the document (ADR 0144).
    #[must_use]
    pub fn activate(&mut self, number: u32, generation: u16) -> Events {
        self.handle(Command::Activate(pdf_syntax::ObjectId::new(
            number, generation,
        )))
    }

    /// Hands back the pixels a request asked for.
    ///
    /// The request carries the token, which is how a token that is opaque in Rust stays out of
    /// the header entirely: a caller returns what it was given. A stale one is dropped by the
    /// viewer, which is what stops a page turned mid-render from being overwritten by the frame
    /// the previous page produced.
    #[must_use]
    pub fn render_ready_raster(&mut self, request: &RenderRequest, raster: Raster) -> Events {
        self.handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(raster),
        })
    }

    /// Says that a request could not be drawn, and why.
    ///
    /// Reported rather than swallowed, which is [`Rendered::Failed`]'s own reason: a viewer that
    /// silently shows the previous page when a render fails is telling a person something false
    /// about the document.
    #[must_use]
    pub fn render_ready_failed(&mut self, request: &RenderRequest, why: String) -> Events {
        self.handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Failed(why),
        })
    }

    /// How many pages the focused document has.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn page_count(&self) -> Result<usize, Status> {
        match self.viewer.query(Query::PageCount) {
            Answer::Count(count) => Ok(count),
            _ => Err(Status::NoAnswer),
        }
    }

    /// Which page is showing, and how many there are.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn current_page(&self) -> Result<(usize, usize), Status> {
        match self.viewer.query(Query::CurrentPage) {
            Answer::Page { index, of, .. } => Ok((index, of)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// Where a page sits on the screen and how large it is drawn.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] for a page that is not the one showing: the geometry is a property of
    /// the view rather than of the page.
    pub fn page_geometry(&self, page: usize) -> Result<viewer_core::PageGeometry, Status> {
        match self.viewer.query(Query::PageGeometry(page)) {
            Answer::Geometry(geometry) => Ok(geometry),
            _ => Err(Status::NoAnswer),
        }
    }

    /// What the viewer is holding, without the pixels: enough to size a buffer for them.
    ///
    /// The first half of C's two-call idiom, and the reason no pointer into the viewer is ever
    /// handed out. Answers the page, the extent, the pixel layout, how many bytes
    /// [`Self::frame_copy`] will write, and where the raster's top-left corner sits.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where the viewer holds no frame — before the first render, and for a
    /// tier-2 host that presented rather than handing pixels back.
    pub fn frame_info(&self) -> Result<FrameInfo, Status> {
        match self.viewer.query(Query::Frame) {
            Answer::Frame(frame) => Ok(FrameInfo {
                page: frame.page,
                width: frame.raster.width,
                height: frame.raster.height,
                format: frame.raster.format,
                bytes: frame.raster.data.len(),
                origin: frame.origin,
            }),
            _ => Err(Status::NoAnswer),
        }
    }

    /// Copies the frame into a buffer the caller owns.
    ///
    /// **One copy, which is what `doc/ui-boundary.md` prices tier 1 at** — the same copy
    /// `gdk::MemoryTexture` and `QImage` make, into a buffer the caller already has because it is
    /// about to upload it. Lending a pointer instead would save the copy and hand a C program a
    /// lifetime it has no way to keep.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where the viewer holds no frame, and [`Status::BufferTooSmall`] where
    /// `into` is shorter than [`FrameInfo::bytes`] said.
    pub fn frame_copy(&self, into: &mut [u8]) -> Result<usize, Status> {
        let Answer::Frame(frame) = self.viewer.query(Query::Frame) else {
            return Err(Status::NoAnswer);
        };
        let data = &frame.raster.data;
        let Some(room) = into.get_mut(..data.len()) else {
            return Err(Status::BufferTooSmall);
        };
        room.copy_from_slice(data);
        Ok(data.len())
    }

    /// §12.3.3's outline, flattened into rows with a depth on each.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused or it states no outline.
    pub fn outline(&self) -> Result<Outline, Status> {
        match self.viewer.query(Query::Outline) {
            Answer::Outline(outline) => Ok(Outline::of(&outline)),
            _ => Err(Status::NoAnswer),
        }
    }
}

/// What the viewer is holding, without the pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameInfo {
    /// Which page these pixels are of, zero-based.
    pub page: usize,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The pixel layout, which is what a caller blits under.
    pub format: RasterFormat,
    /// How many bytes [`Session::frame_copy`] writes.
    pub bytes: usize,
    /// Where the raster's top-left corner sits in the viewport, in device pixels.
    pub origin: (f32, f32),
}

/// Draws a request with the processor rasteriser.
///
/// **This is why this crate depends on `render-cpu` and it is the only reason.** A
/// [`viewer_core::Event::NeedsRender`] carries an `Arc<DisplayList>`, which is clauses 8 and 9 in
/// a data structure and is not a thing to put in a header; what a C caller wants from it is
/// pixels. The request stays an opaque handle it may move to a thread of its own, and this is
/// what that thread calls.
///
/// # Errors
///
/// [`Status::RenderRefused`] where the rasteriser declined — a coverage or budget refusal, which
/// `CLAUDE.md` requires be reported rather than approximated.
pub fn rasterise(request: &RenderRequest) -> Result<Raster, Status> {
    // `Rasterizer` is the trait `rasterize` is on, and it takes `&mut self` because a backend may
    // keep scratch buffers between frames. One rasteriser per call is what a C caller's own
    // thread wants anyway: the handle it was given is the only state the drawing needs.
    CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .map_err(|_| Status::RenderRefused)
}

#[cfg(test)]
mod tests {
    use viewer_core::PageTarget;

    use super::{Session, rasterise};
    use crate::kinds::EventKind;
    use crate::status::Status;

    /// The five-page application note, which every host in this tree is measured on.
    fn note() -> Vec<u8> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf"),
        )
        .expect("the application note is in the tree")
    }

    /// The whole path a C caller takes, in Rust: open, render, turn a page, ask, copy pixels.
    ///
    /// The same sequence `c/open_a_page.c` performs through the ABI. Having it here as well is
    /// what makes a failure legible: this one says the *vocabulary* broke, and that one says the
    /// pointers did.
    #[test]
    fn a_document_opens_draws_turns_a_page_and_hands_over_its_pixels() {
        let mut session = Session::new(800, 1000, 1.0);
        let events = session.open(1, note(), None, None);
        let asked = (0..events.len())
            .find(|index| events.kind(*index) == Ok(EventKind::NeedsRender))
            .expect("opening a document asks for its first page");
        let request = events.render_request(asked).expect("that is the kind");
        let raster = rasterise(&request).expect("the note's first page draws");
        drop(session.render_ready_raster(&request, raster));

        assert_eq!(session.page_count(), Ok(5));
        assert_eq!(session.current_page(), Ok((0, 5)));
        let before = session.frame_info().expect("a frame was handed back");
        assert_eq!(before.page, 0);
        assert!(
            before.bytes > 0 && before.bytes == before.width as usize * before.height as usize * 4
        );

        // A page turn, and the frame that follows it is of the page that was turned to.
        let turned = session.go_to(PageTarget::Next);
        let asked = (0..turned.len())
            .find(|index| turned.kind(*index) == Ok(EventKind::NeedsRender))
            .expect("a page turn asks for the new page");
        let request = turned.render_request(asked).expect("that is the kind");
        let raster = rasterise(&request).expect("the note's second page draws");
        drop(session.render_ready_raster(&request, raster));
        assert_eq!(session.current_page(), Ok((1, 5)));
        let after = session.frame_info().expect("a frame was handed back");
        assert_eq!(after.page, 1);

        let mut pixels = vec![0u8; after.bytes];
        assert_eq!(session.frame_copy(&mut pixels), Ok(after.bytes));
        assert!(
            pixels.iter().any(|byte| *byte != 0),
            "a drawn page is not all zeroes"
        );
        // And the two-call idiom's refusal, which is what a caller that sized its buffer from a
        // stale `frame_info` would get instead of a partial page.
        let mut small = vec![0u8; after.bytes.saturating_sub(1)];
        assert_eq!(session.frame_copy(&mut small), Err(Status::BufferTooSmall));
    }

    /// A viewer with no document answers `NoAnswer` rather than zero.
    #[test]
    fn nothing_open_is_an_answer_and_not_a_count() {
        let session = Session::new(100, 100, 1.0);
        assert_eq!(session.page_count(), Err(Status::NoAnswer));
        assert_eq!(session.current_page(), Err(Status::NoAnswer));
        assert!(session.frame_info().is_err());
        assert!(session.outline().is_err());
    }

    /// §12.3.3's outline crosses owned, which is ADR 0247's second amendment from the C side.
    #[test]
    fn the_outline_crosses_as_rows_with_a_depth_and_an_object_on_each() {
        let mut session = Session::new(800, 1000, 1.0);
        drop(session.open(1, note(), None, None));
        let outline = session.outline().expect("the note has an outline");
        assert_eq!(outline.len(), 14, "the note's outline is fourteen rows");
        assert_eq!(outline.depth(0), Ok(0), "the first row is at the top level");
        assert!(
            (0..outline.len()).any(|row| outline.depth(row) == Ok(1)),
            "the note's outline has children"
        );
        for row in 0..outline.len() {
            assert!(!outline.title(row).expect("in range").is_empty());
        }
        assert_eq!(outline.title(outline.len()), Err(Status::OutOfRange));
    }
}
