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
    Answer, Command, DocumentId, Edit, Event, Find, FindDirection, PageTarget, PointerAction,
    Query, RenderRequest, Rendered, Selection, Viewer, Zoom,
};

use crate::answers::{Collection, Matches, Miniature, Popups, Structure};
use crate::events::Events;
use crate::form::Form;
use crate::kinds::{
    BoundaryKind, DirectionKind, DuplexKind, LayoutKind, PageModeKind, PreferenceKey,
    PrintScalingKind, ShortfallKind,
};
use crate::panels::{Outline, Panel};
use crate::shapes::Quads;
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
    ///
    /// The password arrives as a [`viewer_core::Secret`] rather than a `String` so that the text
    /// the caller's NUL-terminated bytes became **moves** into it: a conversion here would leave a
    /// second copy in a buffer nothing clears. What the *caller's* memory holds is the caller's,
    /// and the header says so.
    #[must_use]
    pub fn open(
        &mut self,
        id: u64,
        bytes: Vec<u8>,
        password: Option<viewer_core::Secret>,
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
    /// `pdfv_event_searched` reports nothing remaining — the same loop it already runs for
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
    pub fn frame_info(&self, frame: usize) -> Result<FrameInfo, Status> {
        match self.viewer.query(Query::Frame) {
            Answer::Frame(frames) => frames.get(frame).map_or(Err(Status::NoAnswer), |frame| {
                Ok(FrameInfo {
                    page: frame.page,
                    width: frame.raster.width,
                    height: frame.raster.height,
                    format: frame.raster.format,
                    bytes: frame.raster.data.len(),
                    origin: frame.origin,
                })
            }),
            _ => Err(Status::NoAnswer),
        }
    }

    /// How many frames the viewer is holding — Table 29's arrangement, counted.
    ///
    /// **The entry point a C caller could not have deduced**, and the reason it exists rather than
    /// a changed `pdfv_frame_info` alone: a C consumer cannot fail to compile, so a `/PageLayout`
    /// putting a second page on the screen has to be something a caller *asks* about. Zero for a
    /// tier-2 host, which hands no pixels back, and one for the `SinglePage` every document that
    /// says nothing about it opens in.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        match self.viewer.query(Query::Frame) {
            Answer::Frame(frames) => frames.len(),
            _ => 0,
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
    pub fn frame_copy(&self, frame: usize, into: &mut [u8]) -> Result<usize, Status> {
        let Answer::Frame(frames) = self.viewer.query(Query::Frame) else {
            return Err(Status::NoAnswer);
        };
        let Some(frame) = frames.get(frame) else {
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

    // ---------------------------------------------------------------------------------------
    // The pointer, the selection and §12.5.1's focus.
    // ---------------------------------------------------------------------------------------

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

    /// The selected text, as the pages read back.
    ///
    /// **Pages**, since a drag across Table 29's continuous arrangements may cross a page
    /// boundary: what comes back is then each page's part in page order, joined by a newline
    /// (ADR 0444). Owned here whichever it is, because nothing of the viewer may cross this
    /// boundary borrowed — and the copy is the one the caller was going to make anyway.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn selection_text(&self) -> Result<String, Status> {
        match self.viewer.query(Query::Selection) {
            Answer::Selected(selected) => Ok(selected.text.into_owned()),
            _ => Err(Status::NoAnswer),
        }
    }

    /// The text a *copy* should carry, and which of §14.8.2.5's two orders it is in.
    ///
    /// **The gap `tools/state.sh hosts` found, closed with the decision rather than the
    /// ingredients** (ADR 0519). [`Self::selection_text`] answers in page content order, because
    /// that is what a selection is measured in and what the quadrilaterals are drawn from;
    /// §14.8.2.5.1 says the two orders "should coincide" and does not require it, so a copy off a
    /// page whose producer wrote its columns out of order wants the other one. A C caller could
    /// not ask for it at all: `Query::LogicalSelection` reached no entry point.
    ///
    /// Handing over both answers and letting the caller choose was the alternative, and it is the
    /// worse one — it would make a fifth consumer re-derive a reading of the standard that
    /// [`viewer_host::copied`] already states for the three windowed hosts. So this answers with
    /// what those hosts put on their clipboards, and says which order it is so that a caller who
    /// disagrees can still tell.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, or where nothing is selected — the
    /// second because a copy with nothing to copy must not empty a caller's clipboard.
    pub fn copy_text(&self) -> Result<(String, viewer_host::ContentOrder), Status> {
        // Owned before the second question, because both answers borrow the viewer.
        let page_order = match self.viewer.query(Query::Selection) {
            Answer::Selected(selected) => selected.text.into_owned(),
            _ => return Err(Status::NoAnswer),
        };
        let logical = match self.viewer.query(Query::LogicalSelection) {
            Answer::LogicalSelection(text) => Some(text),
            _ => None,
        };
        viewer_host::copied(logical, &page_order)
            .map(|copied| (copied.text, copied.order))
            .ok_or(Status::NoAnswer)
    }

    /// The shapes covering what is selected, in device pixels of the viewport.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn selection_quads(&self) -> Result<Quads, Status> {
        match self.viewer.query(Query::Selection) {
            Answer::Selected(selected) => Ok(Quads::new(selected.quads)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// Annex O's highlighted rectangles on the page being shown.
    ///
    /// Table Annex O.4's `highlight`, which a URI's fragment states and this program cannot draw
    /// for a caller: "[t]he nature of the highlighting is implementation-dependent", so the shapes
    /// cross and the colour is the caller's. Empty where the fragment named none, which is why
    /// this cannot fail for an open document.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn highlight_quads(&self) -> Result<Quads, Status> {
        match self.viewer.query(Query::Highlight) {
            Answer::Highlighted(quads) => Ok(Quads::new(quads)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// Which annotation holds §12.5.1's focus, and where it is on the screen.
    ///
    /// Named for the *annotation* rather than for the query, because [`Self::focus`] is already
    /// [`Command::Focus`] — a document being made the one commands apply to, which is a different
    /// thing entirely and was here first.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where nothing is focused or it states no usable `/Rect`.
    pub fn focused_annotation(&self) -> Result<((u32, u16), [f32; 8]), Status> {
        match self.viewer.query(Query::Focus) {
            Answer::Focus { object, quad } => Ok(((object.number, object.generation), quad)),
            _ => Err(Status::NoAnswer),
        }
    }

    // ---------------------------------------------------------------------------------------
    // §12.7's form, and the four edits.
    // ---------------------------------------------------------------------------------------

    /// §12.7's fields with a widget on the page being shown.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused. A page with no widget on it answers an
    /// empty form, which is an answer rather than a refusal.
    pub fn fields(&self) -> Result<Form, Status> {
        match self.viewer.query(Query::Fields) {
            Answer::Fields(fields) => Ok(Form::of(&fields)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// What the field at a point is called: §12.7.4.2's qualified name and §14.9.3's shown one.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no field is there.
    pub fn field_at(&self, at: (f32, f32)) -> Result<(String, String), Status> {
        match self.viewer.query(Query::FieldAt(at)) {
            Answer::Field { name, .. } => Ok((name.qualified.clone(), name.shown().to_owned())),
            _ => Err(Status::NoAnswer),
        }
    }

    /// Where the caret sits in the field at a point, given how far into the value it is.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] in the cases [`viewer_core::Query::Caret`] answers `None` in.
    /// Answered as `[from_x, from_y, to_x, to_y]`: a caret has no width, so what crosses is two
    /// points, and a flat array is what the entry point writes out anyway.
    pub fn caret(&self, at: (f32, f32), offset: usize) -> Result<[f32; 4], Status> {
        match self.viewer.query(Query::Caret { at, offset }) {
            Answer::Caret { from, to } => Ok([from.0, from.1, to.0, to.1]),
            _ => Err(Status::NoAnswer),
        }
    }

    /// How far into a field's value a point inside it is, in bytes — the caret's inverse.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] in the cases [`Self::caret`] answers it in.
    pub fn offset(&self, at: (f32, f32), point: (f32, f32)) -> Result<usize, Status> {
        match self.viewer.query(Query::Offset { at, point }) {
            Answer::Offset(offset) => Ok(offset),
            _ => Err(Status::NoAnswer),
        }
    }

    /// The shapes covering a byte range of a field's value, one per line it touches.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] in the cases [`Self::caret`] answers it in.
    pub fn field_selection(&self, at: (f32, f32), from: usize, to: usize) -> Result<Quads, Status> {
        match self.viewer.query(Query::FieldSelection { at, from, to }) {
            Answer::FieldSelection(quads) => Ok(Quads::new(quads)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.5.6.6: the free text annotation **this session added** at a point, and what it says.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no such annotation covers the point — including for every free
    /// text annotation the file itself states, which nothing in this vocabulary can change.
    pub fn free_text_at(&self, at: (f32, f32)) -> Result<((u32, u16), String), Status> {
        match self.viewer.query(Query::FreeTextAt { at }) {
            Answer::FreeText { annotation, text } => {
                Ok(((annotation.number, annotation.generation), text))
            }
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.7.4: put a value into a field, by the name §12.7.4.2 gives it.
    #[must_use]
    pub fn set_field(&mut self, field: String, value: pdf_model::view::Entered) -> Events {
        self.handle(Command::Edit(Edit::SetField { field, value }))
    }

    /// §12.5.6.10: mark up what is selected, in one of the clause's four ways.
    #[must_use]
    pub fn markup(&mut self, kind: pdf_model::view::Markup, colour: [f32; 3]) -> Events {
        self.handle(Command::Edit(Edit::Markup { kind, colour }))
    }

    /// §12.5.6.6: put an empty free text annotation over a rectangle a person drew.
    #[must_use]
    pub fn free_text(&mut self, from: (f32, f32), to: (f32, f32), colour: [f32; 3]) -> Events {
        self.handle(Command::Edit(Edit::FreeText { from, to, colour }))
    }

    /// §12.5.6.6: say what a free text annotation this session added says.
    #[must_use]
    pub fn set_free_text(&mut self, number: u32, generation: u16, text: String) -> Events {
        self.handle(Command::Edit(Edit::SetFreeText {
            annotation: pdf_syntax::ObjectId::new(number, generation),
            text,
        }))
    }

    /// §7.11.4: put a file into the document, in one of §7.11.4.1's two homes.
    #[must_use]
    pub fn attach(
        &mut self,
        bytes: Vec<u8>,
        name: String,
        description: Option<String>,
        mime: Option<String>,
        home: viewer_core::AttachHome,
    ) -> Events {
        self.handle(Command::Edit(Edit::Attach {
            bytes: bytes.into(),
            name,
            description,
            mime,
            home,
        }))
    }

    /// §7.11.4: take an embedded file out of the document, by the name it is filed under.
    #[must_use]
    pub fn detach(&mut self, name: String) -> Events {
        self.handle(Command::Edit(Edit::Detach { name }))
    }

    /// The person's answer to [`viewer_core::Event::Asking`].
    #[must_use]
    pub fn answer(&mut self, document: u64, proceed: bool) -> Events {
        self.handle(Command::Answer {
            document: DocumentId(document),
            proceed,
        })
    }

    /// Undo the last edit.
    #[must_use]
    pub fn undo(&mut self) -> Events {
        self.handle(Command::Undo)
    }

    /// Redo the last undone edit.
    #[must_use]
    pub fn redo(&mut self) -> Events {
        self.handle(Command::Redo)
    }

    /// Whether anything has been edited since the document opened.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn dirty(&self) -> Result<bool, Status> {
        match self.viewer.query(Query::Dirty) {
            Answer::Dirty(dirty) => Ok(dirty),
            _ => Err(Status::NoAnswer),
        }
    }

    // ---------------------------------------------------------------------------------------
    // Bytes out: §7.5.6's incremental update and §7.11.4's embedded files.
    // ---------------------------------------------------------------------------------------

    /// Write §7.5.6's incremental update for everything the log holds.
    ///
    /// The bytes come back on [`viewer_core::Event::Saved`], because rule 2 gives this crate no
    /// filesystem and where a file lands is the caller's decision.
    #[must_use]
    pub fn save(&mut self) -> Events {
        self.handle(Command::Save)
    }

    /// §7.11.4: take an embedded file's bytes out of the document.
    #[must_use]
    pub fn extract(&mut self, name: String) -> Events {
        self.handle(Command::Extract { name })
    }

    /// The bytes the viewer asked for with [`viewer_core::Event::NeedsFile`], or a refusal.
    #[must_use]
    pub fn supply(&mut self, purpose: viewer_core::Purpose, bytes: Option<Vec<u8>>) -> Events {
        self.handle(Command::Supply { purpose, bytes })
    }

    // ---------------------------------------------------------------------------------------
    // The other two panels, and §8.11's switch.
    // ---------------------------------------------------------------------------------------

    /// §8.11.4.3's `/Order`, flattened.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused or it states no optional content.
    pub fn layers(&self) -> Result<Panel, Status> {
        match self.viewer.query(Query::Layers) {
            Answer::Layers(layers) => Ok(Panel::of_layers(&layers)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §7.11.4's embedded files, listed rather than extracted.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused or it states no `/EmbeddedFiles` tree.
    pub fn attachments(&self) -> Result<Panel, Status> {
        match self.viewer.query(Query::Attachments) {
            Answer::Attachments(attachments) => Ok(Panel::of_attachments(&attachments)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §8.11: switch an optional content group on or off.
    #[must_use]
    pub fn set_group(&mut self, number: u32, generation: u16, on: bool) -> Events {
        self.handle(Command::SetGroup {
            group: pdf_syntax::ObjectId::new(number, generation),
            on,
        })
    }

    // ---------------------------------------------------------------------------------------
    // The three policy values, and §12.4.4's clock.
    // ---------------------------------------------------------------------------------------

    /// §12.4.4.1's clock, driven by the caller, because rule 3 leaves this crate none.
    #[must_use]
    pub fn tick(&mut self, millis: u32) -> Events {
        self.handle(Command::Tick { millis })
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

    /// How much of what a document asserts about its reader this viewer obeys.
    #[must_use]
    pub fn restrict(&mut self, level: viewer_core::RestrictionLevel) -> Events {
        self.handle(Command::Restrict(level))
    }

    /// §6.3.2.2's "unless otherwise instructed": who draws §12.7's widget appearances.
    #[must_use]
    pub fn delegate(&mut self, appearances: pdf_model::view::WidgetAppearances) -> Events {
        self.handle(Command::Delegate(appearances))
    }

    /// How many pages on the screen this viewer has anything to say about.
    ///
    /// **The entry point a C caller could not have deduced**, and it exists for
    /// [`Self::frame_count`]'s reason said again: a C consumer cannot fail to compile, so a
    /// `/PageLayout` putting a second page on the screen has to be something a caller *asks*
    /// about. Zero where no document is focused or no page has been interpreted, and one for the
    /// `SinglePage` every document that says nothing about it opens in.
    #[must_use]
    pub fn reported_pages(&self) -> usize {
        match self.viewer.query(Query::Reports) {
            Answer::Reports(pages) => pages.len(),
            _ => 0,
        }
    }

    /// Which page one of those entries is about, zero-based.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, [`Status::OutOfRange`] for an entry
    /// past [`Self::reported_pages`].
    pub fn reported_page(&self, entry: usize) -> Result<usize, Status> {
        let Answer::Reports(pages) = self.viewer.query(Query::Reports) else {
            return Err(Status::NoAnswer);
        };
        pages
            .get(entry)
            .map(|page| page.page)
            .ok_or(Status::OutOfRange)
    }

    /// Everything one page on the screen could not draw.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, [`Status::OutOfRange`] for an entry
    /// past [`Self::reported_pages`].
    pub fn reports(&self, entry: usize) -> Result<Vec<String>, Status> {
        let Answer::Reports(pages) = self.viewer.query(Query::Reports) else {
            return Err(Status::NoAnswer);
        };
        pages
            .get(entry)
            .map(|page| page.notes.to_vec())
            .ok_or(Status::OutOfRange)
    }

    // ---------------------------------------------------------------------------------------
    // The other half of the queries — `doc/todo/30` item 5, the seven-hundred-and-ninth session.
    //
    // Eleven `Query` variants reached no symbol at all. Every one of them is below, and
    // `tests/every_query_reaches_the_abi.rs` is what keeps that true: a variant added to
    // `viewer-core` fails to compile in a `match` there, which is the mechanism
    // `PDFV_EVENT_KIND_COUNT` already is for events and which nothing was for a question.
    // ---------------------------------------------------------------------------------------

    /// Every occurrence of a string on the page being shown, as shapes to draw over it.
    ///
    /// **This page, and [`Self::find_start`] is the document.** The two are one search asked
    /// twice: this answers from a readback that already exists, in microseconds, so a find bar may
    /// ask it on every repaint; the other interprets pages nobody is looking at and takes one step
    /// at a time. A caller that had only the second could run a search and not draw a match, which
    /// is what this ABI was until now.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn find(&self, needle: &str) -> Result<Matches, Status> {
        match self.viewer.query(Query::Find(needle)) {
            Answer::Found(occurrences) => Ok(Matches::new(occurrences)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// Table 29's `/PageMode` and `/PageLayout`: what the catalogue asks of the window opening it.
    ///
    /// Both, in one call, because §7.7.2 states them as one instruction to a window and a caller
    /// obeys them together — one chooses which panel is open and the other how the pages are laid
    /// out. [`Self::preferences`] is Table 147, which is a different table and a separate call for
    /// that reason.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn opening(&self) -> Result<(PageModeKind, LayoutKind), Status> {
        match self.viewer.query(Query::Opening) {
            Answer::Opening(opening) => Ok((
                PageModeKind::of(opening.mode),
                LayoutKind::of(opening.layout),
            )),
            _ => Err(Status::NoAnswer),
        }
    }

    /// One entry of §12.2's Table 147, as a number.
    ///
    /// See [`PreferenceKey`] for why this is one keyed accessor rather than nineteen symbols or a
    /// struct. Every entry Table 147 gives a default answers that default; the three it leaves
    /// genuinely open answer [`Status::NoAnswer`], because "the document says nothing" and "the
    /// document says the default" are different facts and only the first leaves the choice to the
    /// caller.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused or the entry is one Table 147 leaves
    /// open and the document states none; [`Status::WrongKind`] for `/PrintPageRange`, which is a
    /// list and is read with [`Self::preference_ranges`].
    pub fn preference(&self, key: PreferenceKey) -> Result<i64, Status> {
        let Answer::Preferences(preferences) = self.viewer.query(Query::Preferences) else {
            return Err(Status::NoAnswer);
        };
        let boolean = |value: bool| Ok(i64::from(value));
        match key {
            PreferenceKey::HideToolbar => boolean(preferences.hide_toolbar),
            PreferenceKey::HideMenubar => boolean(preferences.hide_menubar),
            PreferenceKey::HideWindowUi => boolean(preferences.hide_window_ui),
            PreferenceKey::FitWindow => boolean(preferences.fit_window),
            PreferenceKey::CenterWindow => boolean(preferences.center_window),
            PreferenceKey::DisplayDocTitle => boolean(preferences.display_doc_title),
            PreferenceKey::EnforcePrintScaling => boolean(preferences.enforce_print_scaling),
            PreferenceKey::NonFullScreenPageMode => Ok(i64::from(
                PageModeKind::of(preferences.non_full_screen_page_mode).code(),
            )),
            PreferenceKey::Direction => {
                Ok(i64::from(DirectionKind::of(preferences.direction) as u32))
            }
            PreferenceKey::ViewArea => {
                Ok(i64::from(BoundaryKind::of(preferences.view_area) as u32))
            }
            PreferenceKey::ViewClip => {
                Ok(i64::from(BoundaryKind::of(preferences.view_clip) as u32))
            }
            PreferenceKey::PrintArea => {
                Ok(i64::from(BoundaryKind::of(preferences.print_area) as u32))
            }
            PreferenceKey::PrintClip => {
                Ok(i64::from(BoundaryKind::of(preferences.print_clip) as u32))
            }
            PreferenceKey::PrintScaling => Ok(i64::from(PrintScalingKind::of(
                preferences.print_scaling,
            ) as u32)),
            PreferenceKey::Duplex => preferences
                .duplex
                .map(|duplex| i64::from(DuplexKind::of(duplex) as u32))
                .ok_or(Status::NoAnswer),
            PreferenceKey::PickTrayByPdfSize => preferences
                .pick_tray_by_pdf_size
                .map(i64::from)
                .ok_or(Status::NoAnswer),
            PreferenceKey::NumCopies => preferences.num_copies.ok_or(Status::NoAnswer),
            // Named and refused rather than absent, which is trap 5 in the small: a key that did
            // not exist would look like a table this build had not read, and this one says which
            // function to ask instead.
            PreferenceKey::PrintPageRange => Err(Status::WrongKind),
        }
    }

    /// Table 147's `/PrintPageRange`: "the page numbers used to initialize the print dialog box".
    ///
    /// The one entry of that table that is a list, which is why it is not [`Self::preference`]'s.
    /// Pairs, because the entry is "an array of integers … [t]he first and last pages in a
    /// sub-range", and an empty list is a document that states none.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn preference_ranges(&self) -> Result<Vec<(i64, i64)>, Status> {
        match self.viewer.query(Query::Preferences) {
            Answer::Preferences(preferences) => Ok(preferences.print_page_range),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §14.3.3's Table 349 and §14.3.2's metadata stream, as a panel would show them.
    ///
    /// **`viewer_host::property_rows`, unchanged**, which is what makes this two lines rather than
    /// a second reading of two tables: both are shown rather than merged, because Table 349's text
    /// entries each carry a NOTE pointing at an XMP counterpart, §12.2 ranks `dc:title` above
    /// `/Title` and nothing ranks the rest, and §14.3.4 leaves the disagreement "at the discretion
    /// of the PDF processor". A panel that merged them would hide a disagreement rather than
    /// resolve one, and this ABI is not the place that decision changes.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn properties(&self) -> Result<Panel, Status> {
        match self.viewer.query(Query::Properties) {
            Answer::Properties {
                information,
                metadata,
            } => Ok(Panel::of_rows(&viewer_host::property_rows(
                &information,
                metadata.as_ref(),
            ))),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.4.3's article threads, as a panel would list them.
    ///
    /// Each row acts through `pdfv_activate` on the thread's object, which is the same message an
    /// outline row sends and for the same reason: the *document* decides what activating a thing
    /// means, and `viewer_core::interact` composes §12.6.4.7's thread action out of it so that
    /// following a thread lands on Table 163's `/R` rather than on the page the first bead sits on.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn articles(&self) -> Result<Panel, Status> {
        match self.viewer.query(Query::Articles) {
            Answer::Articles(threads) => Ok(Panel::of_rows(&viewer_host::article_rows(&threads))),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.4.2's label for one page, where the document states one.
    ///
    /// **A page at a time, and separate from [`Self::thumbnail`] on purpose.** A caller drawing a
    /// page list needs a name per row and a picture only for the rows it is showing; folding the
    /// two into one call would make listing a thousand pages decode a thousand images, which is
    /// exactly the launch-path defect the seven-hundred-and-fourth session found in the host that
    /// drew its own rows.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused or the page states no label, which is
    /// most pages of most documents — the clause makes the index what identifies a page and the
    /// label an addition, so a caller falls back to the number rather than to nothing.
    pub fn page_label(&self, index: usize) -> Result<String, Status> {
        match self.viewer.query(Query::PageLabel(index)) {
            Answer::Label(label) => Ok(label),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.3.4's thumbnail for one page, decoded.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, where the page states no `/Thumb` — the
    /// clause's own NOTE says they "are not required, and can be included for some pages and not
    /// for others" — or where this reader could not decode the one it states, which is reported
    /// through the same channel any other undecodable image is.
    pub fn thumbnail(&self, index: usize) -> Result<Miniature, Status> {
        match self.viewer.query(Query::Thumbnail(index)) {
            Answer::Thumbnail(thumbnail) => Ok(Miniature::new(thumbnail)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// How many pages on the screen have a readback shortfall to report.
    ///
    /// [`Self::reported_pages`]'s counterpart, and one entry per page for the same reason: a
    /// column shows several pages and a caller given one page's counts for four would be silent
    /// about three of them.
    #[must_use]
    pub fn readback_pages(&self) -> usize {
        match self.viewer.query(Query::Readback) {
            Answer::Readback(pages) => pages.len(),
            _ => 0,
        }
    }

    /// Which page one of those entries is about, zero-based.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, [`Status::OutOfRange`] for an entry past
    /// [`Self::readback_pages`].
    pub fn readback_page(&self, entry: usize) -> Result<usize, Status> {
        let Answer::Readback(pages) = self.viewer.query(Query::Readback) else {
            return Err(Status::NoAnswer);
        };
        pages
            .get(entry)
            .map(|page| page.page)
            .ok_or(Status::OutOfRange)
    }

    /// One of §9.10.2's counts for one page on the screen.
    ///
    /// **Not a report, and [`viewer_core::Query::Readback`] states the reading**: §9.10.2's own
    /// closing sentence is "there is no way to determine what the character code represents", so a
    /// code that route ends at is an answer the standard states rather than something this program
    /// failed to do. What a caller does with it is the thing a person needs — say that a search
    /// found nothing on a page whose text cannot be read, or that a copied selection is short.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, [`Status::OutOfRange`] for an entry past
    /// [`Self::readback_pages`].
    pub fn readback_count(&self, entry: usize, which: ShortfallKind) -> Result<usize, Status> {
        let Answer::Readback(pages) = self.viewer.query(Query::Readback) else {
            return Err(Status::NoAnswer);
        };
        pages
            .get(entry)
            .map(|page| which.of(&page.shortfall))
            .ok_or(Status::OutOfRange)
    }

    /// §12.5.6.14's open popup windows on the page being shown.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused. An **empty list** where the page has
    /// none open, which is a different answer and is why the two are told apart.
    pub fn popups(&self) -> Result<Popups, Status> {
        match self.viewer.query(Query::Popups) {
            Answer::Popups(windows) => Ok(Popups::new(&windows)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §14.7's logical structure for every page the arrangement is showing.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused. A page with an **empty** node list is an
    /// untagged page, which is an answer rather than a silence.
    pub fn structure(&self) -> Result<Structure, Status> {
        match self.viewer.query(Query::AccessibilityTree) {
            Answer::Accessibility(pages) => Ok(Structure::new(pages)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.3.5's portable collection, where the catalogue states one.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused or it states no collection, which is
    /// every document in this project's corpora.
    pub fn collection(&self) -> Result<Collection, Status> {
        match self.viewer.query(Query::Collection) {
            Answer::Collection {
                collection,
                initial,
            } => Ok(Collection::new(&collection, &initial)),
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
        let before = session.frame_info(0).expect("a frame was handed back");
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
        let after = session.frame_info(0).expect("a frame was handed back");
        assert_eq!(after.page, 1);

        let mut pixels = vec![0u8; after.bytes];
        assert_eq!(session.frame_copy(0, &mut pixels), Ok(after.bytes));
        assert!(
            pixels.iter().any(|byte| *byte != 0),
            "a drawn page is not all zeroes"
        );
        // And the two-call idiom's refusal, which is what a caller that sized its buffer from a
        // stale `frame_info` would get instead of a partial page.
        let mut small = vec![0u8; after.bytes.saturating_sub(1)];
        assert_eq!(
            session.frame_copy(0, &mut small),
            Err(Status::BufferTooSmall)
        );
    }

    /// A viewer with no document answers `NoAnswer` rather than zero.
    #[test]
    fn nothing_open_is_an_answer_and_not_a_count() {
        let session = Session::new(100, 100, 1.0);
        assert_eq!(session.page_count(), Err(Status::NoAnswer));
        assert_eq!(session.current_page(), Err(Status::NoAnswer));
        assert!(session.frame_info(0).is_err());
        assert!(session.outline().is_err());
    }

    /// §14.7's per-character offsets and boxes reach a C caller, and the invariant holds.
    ///
    /// **The half of the accessibility answer that crossed nowhere until this test existed.**
    /// `PDFV_ELEMENT_NAME` is one string for a whole paragraph, so a client building AT-SPI's
    /// `org.a11y.atspi.Text` had the tree and the extents and no way to answer
    /// `GetCharacterExtents`. What is asserted here is the property that interface rests on: the
    /// character byte counts sum to the line's own length, so an offset into the text and an index
    /// into the characters convert into each other without either side guessing.
    #[test]
    fn a_tagged_pages_lines_cross_with_a_place_for_every_character_code() {
        let mut session = Session::new(800, 1000, 1.0);
        let events = session.open(1, note(), None, None);
        // The tree is built from the page's interpretation, so the page has to have been drawn.
        let asked = (0..events.len())
            .find(|index| events.kind(*index) == Ok(EventKind::NeedsRender))
            .expect("opening a document asks for its first page");
        let request = events.render_request(asked).expect("that is the kind");
        let raster = rasterise(&request).expect("the note's first page draws");
        drop(session.render_ready_raster(&request, raster));

        let structure = session.structure().expect("the note is tagged");
        let (_, nodes) = structure.page(0).expect("one entry per page on the screen");
        let mut lines = 0_usize;
        let mut characters = 0_usize;
        for node in 0..nodes {
            let count = structure.lines(0, node).expect("in range");
            for line in 0..count {
                let (text, codes) = structure.line(0, node, line).expect("in range");
                let mut bytes = 0_usize;
                for character in 0..codes {
                    let (produced, box_) = structure
                        .character(0, node, line, character)
                        .expect("in range");
                    bytes += produced;
                    assert!(box_[2] >= box_[0] && box_[3] >= box_[1], "a normalised box");
                    characters += 1;
                }
                assert_eq!(
                    bytes,
                    text.len(),
                    "the codes account for every byte of the line"
                );
                lines += 1;
            }
        }
        assert!(lines > 0, "a tagged page's elements drew text");
        assert!(characters > 0, "and every line is made of character codes");
        // Out of range in each of the three directions, rather than a zero that reads like an
        // answer — which is what the rest of this ABI does and what a caller walks by.
        assert_eq!(structure.lines(0, nodes), Err(Status::OutOfRange));
        assert_eq!(structure.line(0, 0, usize::MAX), Err(Status::OutOfRange));
        assert_eq!(
            structure.character(0, 0, 0, usize::MAX),
            Err(Status::OutOfRange)
        );
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
