//! Navigation, finding text and pointer actions on the current document.

use super::Session;
use crate::events::Events;
use crate::status::Status;
use viewer_core::{
    Answer, Command, Find, FindDirection, PageTarget, PointerAction, Query, Selection, Zoom,
};

impl Session {
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

    /// Puts the reader back at a view [`Self::view`] answered with.
    #[must_use]
    pub fn set_view(&mut self, view: viewer_core::Viewing) -> Events {
        self.handle(Command::View(view))
    }

    /// Annex O's `search`, and a find bar's *next*: one step of a document-wide search.
    ///
    /// Three calls rather than one, because [`viewer_core::Find`] is three verbs and a `bool` for
    /// each would be a caller's puzzle. This one starts, or starts again from what is selected;
    /// [`Self::find_continue`] reads one more page; [`Self::find_stop`] forgets the plan.
    ///
    /// A step reads **one page**, so a caller pumps `find_continue` until
    /// `quorra_event_searched` reports nothing remaining — the same loop it already runs for
    /// [`viewer_core::Event::NeedsRender`], and for the same reason: this ABI is not allowed to
    /// block a caller's event loop for the 5.84 s a thousand-page sweep costs.
    #[must_use]
    pub fn find_start(&mut self, needle: String, backward: bool) -> Events {
        self.handle(Command::Find(Find::Start {
            needle,
            direction: if backward {
                FindDirection::Backward
            } else {
                FindDirection::Forward
            },
        }))
    }

    /// Reads one more page of the search in progress.
    #[must_use]
    pub fn find_continue(&mut self) -> Events {
        self.handle(Command::Find(Find::Continue))
    }

    /// Forgets the search in progress. What closing a find bar sends.
    #[must_use]
    pub fn find_stop(&mut self) -> Events {
        self.handle(Command::Find(Find::Stop))
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

    /// Where the reader is looking: the page, the magnification and the scroll.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] when no document is focused.
    pub fn view(&self) -> Result<viewer_core::Viewing, Status> {
        match self.viewer.query(Query::View) {
            Answer::View(view) => Ok(view),
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

    /// The pointer moved, or a button went down or up, at a point in the viewport.
    #[must_use]
    pub fn pointer(&mut self, at: (f32, f32), action: PointerAction) -> Events {
        self.handle(Command::Pointer { at, action })
    }

    /// Select everything the page reads back as, or nothing.
    #[must_use]
    pub fn select(&mut self, selection: Selection) -> Events {
        self.handle(Command::Select(selection))
    }

    /// §12.5.1: move the input focus to the next or previous annotation on the page.
    #[must_use]
    pub fn focused(&mut self, moved: viewer_core::FocusMove) -> Events {
        self.handle(Command::Focused(moved))
    }

    /// Whether activating at this viewport point would follow a §12.5.6.5 link.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn link_at(&self, at: (f32, f32)) -> Result<bool, Status> {
        match self.viewer.query(Query::LinkAt(at)) {
            Answer::Link(link) => Ok(link),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.4.4: the caller has entered or left presentation mode.
    #[must_use]
    pub fn present(&mut self, mode: viewer_core::PresentationMode) -> Events {
        self.handle(Command::Present(mode))
    }

    /// Table 29's `/PageLayout`, as the person reading has chosen it.
    #[must_use]
    pub fn layout(&mut self, layout: pdf_model::viewer_preferences::PageLayout) -> Events {
        self.handle(Command::Layout(layout))
    }

    /// §8.11: switch an optional content group on or off.
    #[must_use]
    pub fn set_group(&mut self, number: u32, generation: u16, on: bool) -> Events {
        self.handle(Command::SetGroup {
            group: pdf_syntax::ObjectId::new(number, generation),
            on,
        })
    }
}
