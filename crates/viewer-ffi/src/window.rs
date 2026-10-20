//! `quorra_window`: a native window's toolkit-free half, driven from C (RFC 0010 section 3.2, ADR
//! 2088).
//!
//! The vocabulary layer — [`crate::Session`] and the entry points that take a `quorra_viewer *` —
//! hands a caller `viewer-core`'s messages and leaves the loop to it: which event opens a dialogue,
//! when the drawing thread is looked at, what the title says. Three windows answer those questions
//! alike because [`NativeWindow`] holds the answers once; a C host that wrote them again would be
//! the copy `doc/todo/30` warns of. This module is that trait implemented for a caller on the far
//! side of the ABI: [`Window`] holds a [`WindowState`] as `viewer-gtk` and `viewer-qt` do, and the
//! required methods a toolkit answers its own way leave a record in the window's [`Owing`] rather
//! than calling into the caller.
//!
//! # Records rather than callbacks
//!
//! RFC 0010 section 3.2 proposed a table of C function pointers the host fills. ADR 0247 refused
//! callbacks for this ABI on the ground that a caller answering one could call back into the
//! viewer while it is borrowed, and that ground holds here twice over: a `say` called from inside
//! [`NativeWindow::drain`] is a moment `&mut Window` is held, and in C# a callback that throws ends
//! the process. So each required method owes a record, and the caller takes the batch after the
//! entry point returns — the shape `viewer-qt` already has with its update flags, for the same
//! reason: Rust never calls into the toolkit (ADR 2088, RFC 0010 section 3.2a).
//!
//! # What the window does and what it hands back
//!
//! The window opens its document on a thread of its own and joins it at the first resize, draws
//! every page on the drawing thread, and asks to be looked at again on a tick whose interval it
//! states; a key goes through `viewer_host::keys` as every window's does, and the acts the shared
//! trait can carry out it carries out. What only a toolkit can do — the find bar, the panel, the
//! file chooser, a print dialogue — comes back as the `QUORRA_ACT_*` [`crate::ActKind`] names, as
//! `quorra_key_press` hands it back. The pointer, the find bar's search and the strip of open
//! documents are the window's too, and the panels' rows, the controls placed over the page and the
//! marks over it are read and committed through it (`shown`, ADR 2104).

mod owed;
mod shown;

use std::collections::VecDeque;
use std::path::Path;

use pdf_model::view::WidgetAppearances;
use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};
use viewer_host::native_window::{Asked, DOCUMENT, NativeWindow, Showing, WindowState};
use viewer_host::trace::{Topic, Trace};

pub use owed::{OwedKind, Owing};

use crate::keyboard::ActKind;
use crate::status::Status;
use owed::Owed;

/// One native window's toolkit-free half, and the whole of what a `quorra_window *` points at.
///
/// A plain struct with no interior mutability, for [`crate::Session`]'s reason: C owns it, every
/// entry point takes it as `*mut`, and nothing in this module calls back into the caller, so no
/// second borrow can begin while one is held.
#[derive(Debug)]
pub struct Window {
    /// The half every native window shares (ADR 1920).
    state: WindowState,
    /// What this window owes its host since the host last took it.
    owing: Owing,
    /// Device pixels per logical pixel, from the last resize.
    scale: f32,
    /// Whether the first resize has joined the document the opening thread opened.
    joined: bool,
    /// The sentence the status line now says.
    status: String,
    /// The one question at the *ask* level that is waiting for its answer: one at a time, which
    /// is `viewer_core::Command::Answer`'s own rule, so a second question replaces the first.
    question: Option<Asked>,
    /// Which document §7.6.4.1's prompt now out is about; `None` while no prompt is out, so that a
    /// password nobody asked for opens nothing.
    prompting: Option<DocumentId>,
    /// Whether the next document waiting may open once the call now running returns.
    arrival_due: bool,
    /// Whether the bar of a document still opening past `WARN` has been said, which it is once.
    bar_said: bool,
    /// The launch document's outline and placed page tree, read beside page one (ADR 1553).
    preparing: Option<std::sync::mpsc::Receiver<viewer_core::Prepared>>,
    /// The controls and popups the caller was last told of (ADR 2104).
    shown: shown::Shown,
}

impl Window {
    /// Reads `path`'s bytes from disk and starts opening the document on a thread of its own, at
    /// the levels and the policies a window with no command line holds (`viewer_host::Settings`'
    /// defaults, the reader's words unsaid).
    ///
    /// **Nothing is drawn until the first [`Self::resize`]**, which joins the open: the viewport
    /// decides the resolution page one is drawn at, which is `viewer-qt`'s shape and the reason
    /// the two launch timelines measure one thing (ADR 1539).
    ///
    /// # Errors
    ///
    /// [`Status::Unreadable`] where the file cannot be opened.
    pub fn open(path: &Path, fragment: Option<String>) -> Result<Self, Status> {
        let bytes = pdf_syntax::FileBytes::on_disk(path).map_err(|_| Status::Unreadable)?;
        let settings = viewer_host::Settings::default();
        let widget_appearances = WidgetAppearances::Delegated;
        let commands = viewer_host::native_window::opening_commands(
            viewer_host::native_window::standing(settings).window(),
            widget_appearances,
            settings.separations,
            bytes.clone(),
            None,
            fragment.clone(),
        );
        let ahead = viewer_host::Ahead::page_sized();
        let drawing_ahead = ahead.clone();
        let reader = viewer_host::ReaderWords::default();
        let anticipated = viewer_host::Loading::start(move || {
            // No viewport: the core renders nothing into one with no extent, and it is moved back
            // whole, so it is still single-threaded (ADR 0182).
            let mut viewer = Viewer::new(0, 0, 1.0);
            let mut events = Vec::new();
            for command in reader.commands().commands {
                events.extend(viewer.handle(command));
            }
            for command in commands {
                events.extend(viewer.handle(command));
            }
            events.extend(drawing_ahead.anticipate(&mut viewer));
            (viewer, events)
        });
        let trace = Trace::off(std::time::Instant::now());
        Ok(Self {
            state: WindowState::new(
                path,
                bytes,
                fragment,
                trace,
                ahead,
                widget_appearances,
                settings,
            )
            .opening(anticipated),
            owing: Owing::default(),
            scale: 1.0,
            joined: false,
            status: String::new(),
            question: None,
            prompting: None,
            arrival_due: false,
            bar_said: false,
            preparing: None,
            shown: shown::Shown::default(),
        })
    }

    /// Everything owed since the last call, leaving nothing owed.
    #[must_use]
    pub fn take_owed(&mut self) -> Owing {
        self.owing.take()
    }

    /// The title bar's text as it now stands.
    #[must_use]
    pub fn title(&self) -> String {
        let mark = if self.state.showing.dirty { "• " } else { "" };
        if self.state.showing.caption.is_empty() {
            return format!("{mark}{}", self.named());
        }
        format!("{mark}{} — {}", self.named(), self.state.showing.caption)
    }

    /// The status line's sentence as it now stands.
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// The viewport changed size, in device pixels; the first resize joins the open.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange64`] for an empty viewport or a scale that is not a finite positive
    /// number, and nothing changes.
    pub fn resize(&mut self, width: u32, height: u32, scale: f32) -> Result<(), Status> {
        if width == 0 || height == 0 || !scale.is_finite() || scale <= 0.0 {
            return Err(Status::OutOfRange64);
        }
        self.scale = scale;
        self.state.viewport = (width, height);
        let resize = Command::Resize {
            width,
            height,
            scale,
        };
        // Page one's drawing ahead draws into the target the first frame is asked at (ADR 1855).
        let window = viewer_host::loading::Window {
            width,
            height,
            inset: 0,
            scale,
        };
        self.state.extent = Some(window);
        if let Some(ahead) = self.state.ahead.as_ref() {
            ahead.window_is(window);
        }
        if !self.joined {
            self.joined = true;
            self.join_the_opening(resize);
        } else if self.state.awaiting.is_some() {
            // A document still opening past `WARN` is told the size it has when it is taken in.
            self.state.awaiting = Some(resize);
        } else {
            self.dispatch(resize);
        }
        self.settle();
        Ok(())
    }

    /// One look at everything running beside the window: the document still opening, a server's
    /// answer, the outline read beside page one, and the pages drawn — answered, and drawn again
    /// where anything landed.
    ///
    /// **A look that finds nothing owes nothing**, which is `viewer-qt`'s `drawing_pump` answer:
    /// a page being drawn is looked at about a thousand times a second.
    pub fn tick(&mut self) {
        if self.state.awaiting.is_some() {
            self.mind_the_opening();
        }
        self.take_the_answers();
        self.take_the_preparation();
        // Annex O's search, one page per tick, as `viewer-qt`'s zero-length timer steps it.
        if self.state.pages_left > 0 {
            self.dispatch(Command::Find(viewer_core::Find::Continue));
        }
        let mut queue = VecDeque::new();
        self.take_the_drawn(&mut queue);
        if queue.is_empty() {
            if self.state.drawing.glimpsed() {
                self.repaint();
            }
        } else {
            self.drain(queue);
        }
        self.settle();
    }

    /// When the window wants [`Self::tick`] again, in milliseconds, or `None` while nothing beside
    /// it is running — so a window showing a drawn page wakes for it exactly never.
    #[must_use]
    pub fn interval(&self) -> Option<u32> {
        let looking = self
            .state
            .submitter
            .interval()
            .or(self.state.converter.interval());
        let mut interval = match (self.state.drawing_interval(), looking) {
            (Some(drawing), Some(looking)) => Some(drawing.min(looking)),
            (drawing, looking) => drawing.or(looking),
        };
        if self.preparing.is_some() {
            let poll = viewer_host::drawing::POLL;
            interval = Some(interval.map_or(poll, |interval| interval.min(poll)));
        }
        if self.state.awaiting.is_some() {
            let tick = viewer_host::loading::TICK;
            interval = Some(interval.map_or(tick, |interval| interval.min(tick)));
        }
        // A search with pages left takes its next page as soon as the caller's loop is idle.
        if self.state.pages_left > 0 {
            interval = Some(std::time::Duration::ZERO);
        }
        interval.map(|interval| u32::try_from(interval.as_millis()).unwrap_or(u32::MAX))
    }

    /// A key, already one of `viewer_host::Key`'s, with the platform's *command* modifier.
    ///
    /// Decided by `viewer_host::decide`, as `quorra_key_press` and the three windows decide it; the
    /// acts [`NativeWindow`] carries out
    /// alike for every window are carried out here, and the rest are handed back.
    pub fn press(&mut self, key: viewer_host::Key, held: viewer_host::Modifiers) -> (ActKind, f32) {
        let mode = if self.state.presenting.full_screen() {
            viewer_host::Mode::Presenting
        } else {
            viewer_host::Mode::Reading
        };
        let waiting = self.waiting();
        // The page has the keyboard: a control the caller placed over it that has it keeps its own
        // keys, and does not pass them here.
        let page = viewer_host::Keyboard::Page;
        let answer = match viewer_host::decide(&self.state.viewer, key, held, mode, waiting, page) {
            viewer_host::Decided::Activate { annotation, .. } => {
                self.dispatch(Command::Activate(annotation));
                (ActKind::Nothing, 0.0)
            }
            viewer_host::Decided::Send(command) => {
                self.dispatch(command);
                (ActKind::Nothing, 0.0)
            }
            viewer_host::Decided::Say(sentence) => {
                self.say(&sentence);
                (ActKind::Nothing, 0.0)
            }
            viewer_host::Decided::Nothing => (ActKind::Nothing, 0.0),
            viewer_host::Decided::Window(act) => self.window_act(act),
        };
        self.settle();
        answer
    }

    /// The wheel or a scroll bar moved the view, in device pixels.
    pub fn scroll(&mut self, dx: f32, dy: f32) {
        self.dispatch(Command::Scroll { dx, dy });
        self.settle();
    }

    /// The pointer at `at`, in device pixels of the viewport, answering whether it is now over
    /// §12.5.6.5's activation region — the one cursor every window changes.
    ///
    /// §12.9's measuring takes a press before the text selection does, which is the one gesture it
    /// takes away (ADR 1191).
    pub fn pointer(&mut self, at: (f32, f32), action: viewer_core::PointerAction) -> bool {
        if self.state.showing.measuring.is_on() {
            if matches!(action, viewer_core::PointerAction::Pressed) {
                self.measure(at);
            }
        } else {
            self.dispatch(Command::Pointer { at, action });
        }
        let over = matches!(
            self.state.viewer.query(Query::LinkAt(at)),
            Answer::Link(true)
        );
        self.settle();
        over
    }

    /// Annex O's document-wide search for what the find bar holds, forwards or backwards; the
    /// pages after the first are read on the tick, one each (ADR 0250).
    pub fn find(&mut self, needle: &str, backward: bool) {
        needle.clone_into(&mut self.state.needle);
        NativeWindow::find(self, backward);
        self.settle();
    }

    /// The find bar closed: the search ends and its highlights go.
    pub fn find_stop(&mut self) {
        self.state.needle.clear();
        self.state.pages_left = 0;
        self.dispatch(Command::Find(viewer_core::Find::Stop));
        self.settle();
    }

    /// How many documents the window's strip holds, the one in front among them.
    #[must_use]
    pub fn document_count(&self) -> usize {
        self.state.documents.len()
    }

    /// What the strip calls the document at `index`, or `None` past the end.
    #[must_use]
    pub fn document_label(&self, index: usize) -> Option<&str> {
        self.state
            .documents
            .iter()
            .nth(index)
            .map(|(_, label)| label)
    }

    /// Where in the strip the document in front is.
    #[must_use]
    pub fn focused_document(&self) -> usize {
        self.state.documents.focused_index()
    }

    /// Brings the document at `index` in the strip to the front.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] for an index the strip does not hold.
    pub fn choose_document(&mut self, index: usize) -> Result<(), Status> {
        let id = self
            .state
            .documents
            .id_at(index)
            .ok_or(Status::OutOfRange)?;
        if self.take_the_front(id) {
            self.dispatch(Command::Focus(id));
            self.owing.owe(Owed::Panels);
        }
        self.settle();
        Ok(())
    }

    /// Opens the file at `path` beside the documents shown, in a tab of its own — a file chooser's
    /// answer — once the documents already waiting have opened ([`viewer_host::Arrivals`]).
    pub fn open_beside(&mut self, path: &Path) {
        self.state
            .arrivals
            .wait(viewer_host::Named::file(path.to_owned()), false);
        self.open_the_next();
        self.settle();
    }

    /// Closes the document in front and brings its neighbour forward, answering `true` where it
    /// was the last — closing the window, which is the caller's to do.
    pub fn close_document(&mut self) -> bool {
        let closing = self.state.documents.focused();
        self.state.showing.departures = self.state.restrictions.document();
        let last = match self.state.documents.close(closing, &mut self.state.showing) {
            viewer_host::Close::Last => true,
            viewer_host::Close::Unknown => false,
            viewer_host::Close::Done { state, focused } => {
                self.state
                    .restrictions
                    .depart(self.state.showing.departures);
                let label = viewer_host::documents::label(&state.path);
                drop(state);
                self.state.kept.forget(closing);
                self.drain(VecDeque::from([
                    Command::Close(closing),
                    Command::Focus(focused),
                ]));
                self.retitle();
                self.owing.owe(Owed::Panels);
                self.owing.owe(Owed::Strip);
                self.say(&viewer_host::documents::closed(
                    &label,
                    self.state.documents.len(),
                ));
                false
            }
        };
        self.settle();
        last
    }

    /// §7.6.4.1: the person typed a password for `document`, or closed the prompt (`None`).
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no prompt is out for that document.
    pub fn password(&mut self, document: u64, password: Option<String>) -> Result<(), Status> {
        let Some(prompted) = self.prompting.filter(|prompted| prompted.0 == document) else {
            return Err(Status::NoAnswer);
        };
        // Taken by the answer, so a prompt closed twice answers once.
        self.prompting = None;
        self.password_supplied(prompted, password.unwrap_or_default().into());
        self.settle();
        Ok(())
    }

    /// The person answered the one question at the *ask* level, carried out as every window
    /// carries it out (ADR 2105); taking the question is what makes a dialogue closed twice
    /// answer once.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no question is waiting.
    pub fn answer(&mut self, proceed: bool) -> Result<(), Status> {
        let about = self.question.take().ok_or(Status::NoAnswer)?;
        NativeWindow::answer_the_question(self, about, proceed);
        self.settle();
        Ok(())
    }

    /// The caller put the page area on the screen: the launch's first frame is counted from the
    /// first of these that shows a page, and the documents waiting behind it may start then and
    /// not one moment sooner (`CLAUDE.md` section 2).
    pub fn presented(&mut self) {
        if self.state.presented {
            return;
        }
        let shows = self.state.oversized.shows(self.state.documents.focused())
            || matches!(self.state.viewer.query(Query::Frame), Answer::Frame(frames) if !frames.is_empty());
        if shows {
            self.state.presented = true;
            self.state.read_look = None;
            self.state.trace.say(
                Topic::Launch,
                format_args!(
                    "first frame on the screen at {:?}",
                    self.state.trace.since_start()
                ),
            );
            self.later_open_the_next();
            self.settle();
        }
    }

    /// How many pictures the page area shows now.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frames().len()
    }

    /// Where one picture goes and how large it is shown: the raster's facts with its origin where
    /// the window shows it, and the extent it is shown at, which differs from the raster's for a
    /// frame from before a zoom (ADR 1843).
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] for an index the page area does not show.
    pub fn frame(&self, index: usize) -> Result<(crate::FrameInfo, (u32, u32)), Status> {
        let frames = self.frames();
        let placed = frames.get(index).ok_or(Status::NoAnswer)?;
        let raster = placed.frame.raster;
        Ok((
            crate::FrameInfo {
                page: placed.frame.page,
                width: raster.width,
                height: raster.height,
                format: raster.format,
                bytes: raster.data.len(),
                origin: placed.shown_at(),
            },
            placed.extent,
        ))
    }

    /// Copies one picture's pixels into a buffer the caller owns, the one copy tier 1 costs.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] for an index the page area does not show, and
    /// [`Status::BufferTooSmall`] where `into` is shorter than the picture, writing nothing.
    pub fn frame_copy(&self, index: usize, into: &mut [u8]) -> Result<usize, Status> {
        let frames = self.frames();
        let placed = frames.get(index).ok_or(Status::NoAnswer)?;
        let data = &placed.frame.raster.data;
        let room = into.get_mut(..data.len()).ok_or(Status::BufferTooSmall)?;
        room.copy_from_slice(data);
        Ok(data.len())
    }

    /// The pictures the page area shows: the viewer's frames, a page still being drawn as it
    /// stands, a frame from before a zoom scaled to where the page is now, and a page with no frame
    /// shown from its retained picture — the rules `viewer-gtk` and `viewer-qt` paint by (ADRs
    /// 1825, 1843, 1866, 1867, 1930).
    fn frames(&self) -> Vec<viewer_host::stand_in::Placed<'_>> {
        let geometry = |page| match self.state.viewer.query(Query::PageGeometry(page)) {
            Answer::Geometry(geometry) => Some(geometry),
            _ => None,
        };
        let focused = self.state.documents.focused();
        let frames = match self.state.viewer.query(Query::Frame) {
            Answer::Frame(frames) => frames,
            _ => Vec::new(),
        };
        if frames.is_empty()
            && !self.state.oversized.shows(focused)
            && self.state.drawing.partial().is_none()
            && (self.state.anticipated.is_some() || self.state.drawing_interval().is_some())
            && let Some(look) = self.state.read_look.as_ref()
        {
            return vec![look.placed()];
        }
        self.state.oversized.place(
            self.state.retained.fill(
                viewer_host::stand_in::placed(
                    viewer_host::drawing::over_frames(
                        frames,
                        self.state.drawing.partial(),
                        focused,
                        geometry,
                    ),
                    geometry,
                ),
                focused,
                geometry,
            ),
            self.state.drawing.partial(),
            focused,
            geometry,
        )
    }

    /// What every entry point does before it returns: the next document waiting starts, once the
    /// call that made it due has finished, which is `viewer-qt`'s flag read on the next drain.
    fn settle(&mut self) {
        while std::mem::take(&mut self.arrival_due) {
            self.open_the_next();
        }
    }

    /// The half of the key table that is a window's rather than a message: carried out where the
    /// shared trait carries it out for every window, and handed back where it is a toolkit's.
    ///
    /// Matched exhaustively, so that a binding added to `viewer_host::keys` fails to compile here
    /// as it does in the three windows.
    fn window_act(&mut self, act: viewer_host::WindowAct) -> (ActKind, f32) {
        use viewer_host::WindowAct;
        match act {
            WindowAct::ScrollBy(by) => self.dispatch(viewer_host::scroll_by(by, self.scale)),
            WindowAct::Copy => self.copy_selection(),
            WindowAct::Measure => {
                let on = self.state.showing.measuring.toggle();
                self.say(&viewer_host::measuring::switched(on));
            }
            WindowAct::Separations => {
                self.state.separations = !self.state.separations;
                self.dispatch(Command::Separations(self.state.separations));
                self.say(&viewer_host::separations_note(self.state.separations));
            }
            WindowAct::NextLayout => {
                self.state.showing.layout =
                    viewer_host::arrangement::next_layout(self.state.showing.layout);
                self.dispatch(Command::Layout(self.state.showing.layout));
                self.say(&format!(
                    "page layout: {:?} (§7.7.2)",
                    self.state.showing.layout
                ));
                self.restate();
            }
            WindowAct::Archive => self.ask_to_archive(),
            WindowAct::ChooseArchive => self.choose_the_archive_again(),
            WindowAct::AbortDrawing => self.stop_the_long_draw(),
            // A toolkit's: a widget shown, a dialogue opened, a tab strip's order, a window put
            // full screen — handed back as `quorra_key_press` hands them back.
            WindowAct::Find
            | WindowAct::Panel
            | WindowAct::Restrictions
            | WindowAct::Notices
            | WindowAct::Present
            | WindowAct::LeaveFullScreen
            | WindowAct::Print
            | WindowAct::FitControls
            | WindowAct::FreeText
            | WindowAct::NextDocument
            | WindowAct::CloseDocument
            | WindowAct::OpenDocument => return ActKind::of(act),
        }
        (ActKind::Nothing, 0.0)
    }

    /// One command through the viewer, with what it produced put back on the queue.
    fn run(&mut self, command: Command, queue: &mut VecDeque<Command>) {
        let supplied = matches!(command, Command::Supply { .. });
        let events: Vec<Event> = if matches!(command, Command::GoTo(_)) {
            self.turn(command)
        } else {
            self.state.viewer.handle(command).collect()
        };
        for event in events {
            self.react(event, queue);
        }
        // Every event the supply caused has been seen, so an offer nothing took is spent.
        if supplied {
            self.state.arrivals.supplied();
        }
    }

    /// The first resize: the document the opening thread opened is taken in, waiting no longer
    /// than `viewer_host::drawing::WARN` — or `GLIMPSE_AFTER` where page one is drawn while it is
    /// read — and left to its thread past that, joined on the tick that finds it ready (ADRs 1811,
    /// 1855).
    fn join_the_opening(&mut self, resize: Command) {
        let drawing_ahead = self.state.ahead.is_some();
        if self.state.anticipated.as_ref().is_some_and(|opening| {
            !if drawing_ahead {
                opening.wait_within(viewer_host::drawing::GLIMPSE_AFTER)
            } else {
                opening.wait()
            }
        }) {
            self.state.awaiting = Some(resize);
            self.show_the_opening();
            return;
        }
        self.take_the_opening(resize);
    }

    /// The launch document's outline and page tree, if the thread reading them has answered.
    fn take_the_preparation(&mut self) {
        let Some(answer) = self.preparing.as_ref() else {
            return;
        };
        let prepared = match answer.try_recv() {
            Ok(prepared) => Some(prepared),
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            // The thread ended without an answer; the first use reads it instead.
            Err(std::sync::mpsc::TryRecvError::Disconnected) => None,
        };
        self.preparing = None;
        let mut queue = VecDeque::new();
        if let Some(prepared) = prepared {
            let events: Vec<Event> = self.state.viewer.prepared(prepared).collect();
            for event in events {
                self.react(event, &mut queue);
            }
        }
        self.owing.owe(Owed::Panels);
        self.drain(queue);
    }

    /// A document that arrived under a name this window was holding out gets a tab of its own,
    /// and answers the document it gives the front back to where it was opened behind.
    fn opened_beside(&mut self, id: DocumentId) -> Option<DocumentId> {
        let arriving = self.state.arrivals.settle(id)?;
        let (path, behind, bytes) = (
            arriving.named.path.clone(),
            arriving.behind,
            arriving.bytes(),
        );
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(error) => {
                self.say(&format!(
                    "cannot keep {} open for a second attempt: {error}",
                    path.display()
                ));
                return None;
            }
        };
        let label = viewer_host::documents::label(&path);
        if self.state.documents.index_of(id).is_some() {
            self.state.documents.relabel(id, label);
            if id == self.state.documents.focused() {
                self.state.showing = Showing::new(path, bytes, None);
            }
            self.retitle();
            return None;
        }
        self.state
            .documents
            .add(id, label.clone(), Showing::new(path, bytes, None));
        self.owing.owe(Owed::Strip);
        if self.take_the_front(id) {
            self.owing.owe(Owed::Panels);
        }
        self.say(&viewer_host::documents::opened_beside(
            &label,
            self.state.documents.len(),
        ));
        behind
    }

    /// Brings a document's tab to the front, its departures from the window's levels with it
    /// (ADR 1145); `false` where it is already there or is not one of this window's.
    fn take_the_front(&mut self, id: DocumentId) -> bool {
        self.state.showing.departures = self.state.restrictions.document();
        if !self.state.documents.focus(id, &mut self.state.showing) {
            return false;
        }
        self.state
            .restrictions
            .depart(self.state.showing.departures);
        self.retitle();
        self.owing.owe(Owed::Strip);
        true
    }
}

impl NativeWindow for Window {
    fn state(&self) -> &WindowState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut WindowState {
        &mut self.state
    }

    /// Owed as a record, not printed: what a library's caller does with a sentence — a status
    /// bar, a log, both — is the caller's (RFC 0010 section 8, question 3).
    fn say(&mut self, what: &str) {
        if what.is_empty() {
            return;
        }
        what.clone_into(&mut self.status);
        self.owing.owe(Owed::Said(what.to_owned()));
    }

    fn dispatch(&mut self, command: Command) {
        self.drain(VecDeque::from([command]));
    }

    fn later_open_the_next(&mut self) {
        self.arrival_due = true;
    }

    fn repaint(&mut self) {
        self.owing.owe(Owed::Repaint);
    }

    fn retitle(&mut self) {
        let title = self.title();
        self.owing.owe(Owed::Title(title));
    }

    fn offer_to_archive(&mut self, _choosing: viewer_host::archiving::Choosing) {
        // The chooser is a dialogue of six targets and the shipped profiles, which this layer
        // does not yet put to its caller (RFC 0010 section 6, N2's second round): the choice it
        // starts on is said rather than made, so nothing converts that the person did not choose.
        self.say(
            "this window cannot offer the choice of an ISO 19005 conversion yet — the chooser \
             reaches quorra_window in RFC 0010's second N2 round",
        );
    }

    fn opened(&mut self, document: DocumentId, pages: usize, queue: &mut VecDeque<Command>) {
        let behind = self.opened_beside(document);
        if pages == 0 {
            self.say(&viewer_host::no_pages(&self.named()));
        }
        self.state.showing.asking.opened();
        self.state.restrictions.opened();
        self.state.showing.report_due.opened();
        self.obey_the_catalog(queue, behind.is_none());
        self.state.showing.catalog_due = behind.is_some();
        self.owing.owe(Owed::Panels);
        if let Some(front) = behind
            && self.take_the_front(front)
        {
            queue.push_back(Command::Focus(front));
        }
        if document != DOCUMENT || pages == 0 {
            self.later_open_the_next();
        }
    }

    fn password_required(&mut self, document: DocumentId) {
        if !viewer_host::documents::may_ask_about(&mut self.state.arrivals, document, DOCUMENT) {
            self.say(&viewer_host::documents::not_asked(document));
            return;
        }
        let (asked, about) = match self.state.arrivals.named_mut(document) {
            Some(arriving) => (
                arriving.asking.required(),
                viewer_host::documents::label(&arriving.named.path),
            ),
            None => (self.state.showing.asking.required(), self.named()),
        };
        match asked {
            viewer_host::Ask::Prompt { attempt, of } => {
                let words = viewer_host::password::prompt(&about, attempt, of);
                self.prompting = Some(document);
                self.owing.owe(Owed::Password {
                    document,
                    words: format!("{}\n{}", words.question, words.counted),
                });
            }
            viewer_host::Ask::Exhausted => {
                self.say(viewer_host::password::EXHAUSTED);
                self.cancelled(document);
            }
        }
    }

    fn print(&mut self, pages: usize, _fidelity: viewer_core::Fidelity) {
        // §7.6.4.2's bit 3 was granted, and this layer has no print path to its caller yet
        // (RFC 0010 section 6, N4): said, and the operation ended, so that §8.11.4.5's print
        // intent is not left standing over a page nobody prints.
        self.say(&format!(
            "{pages} page(s) would print, and this window cannot print yet — printing reaches \
             quorra_window in RFC 0010's N4"
        ));
        self.dispatch(Command::Print(viewer_core::Printing::Finish));
    }

    fn to_clipboard(&mut self, text: String) {
        self.owing.owe(Owed::Clipboard(text));
    }

    /// Nothing to arm: [`Window::interval`] includes a server's answer and a conversion's report
    /// whenever one is out (ADRs 1291, 1954).
    fn look_for_answers(&mut self) {}

    /// Held, and owed as one record: its two paragraphs joined by an empty line, as `viewer-qt`
    /// joins them for its one label; answered with `quorra_window_answer`.
    fn put_the_question(&mut self, about: Asked, words: &viewer_host::Question) {
        let text = format!("{}\n\n{}", words.reasons, words.choice);
        self.question = Some(about);
        self.owing.owe(Owed::Question(text));
    }

    /// One question at a time, as `viewer-qt`'s one dialogue holds it.
    fn question_waiting(&self) -> bool {
        self.question.is_some()
    }

    fn ask_for_a_script(&mut self, document: DocumentId, question: viewer_core::ScriptQuestion) {
        // `app.alert`'s buttons are the script's and an entry may stand under its text, which is
        // a dialogue this layer does not yet put to its caller: the question is said and answered
        // as a dialogue closed unanswered is, which the script is written to expect (ADR 1628).
        let name = self.state.documents.label_of(document);
        self.say(&viewer_host::script_asks::put(&name, &question));
        let answer = viewer_core::ScriptAnswer::dismissed(&question);
        self.say(&viewer_host::script_asks::answered(&answer, false));
        self.dispatch(Command::AnswerScript { document, answer });
    }

    fn withdraw_script_question(&mut self, document: DocumentId) {
        let name = self.state.documents.label_of(document);
        self.say(&viewer_host::script_asks::withdrawn(&name));
    }

    fn beep(&mut self, document: DocumentId, sound: pdf_model::view::Sound) {
        self.owing.owe(Owed::Beep);
        let name = self.state.documents.label_of(document);
        self.say(&viewer_host::script_timers::played(
            &name,
            sound,
            "the caller",
        ));
    }

    fn attachments_changed(&mut self) {
        self.owing.owe(Owed::Panels);
    }

    /// Nothing to push: the bar is the caller's, read from the status line it is owed.
    fn show_the_drawing(&mut self) {}

    fn fit_the_window(&mut self) {
        self.owing.owe(Owed::Place);
    }

    /// Owed with the placement: the caller centres the window where its own extents are known.
    fn asked_to_centre(&mut self) {
        self.owing.owe(Owed::Place);
    }

    fn show_the_page_mode(&mut self, _mode: pdf_model::viewer_preferences::PageMode) {
        if self.state.presenting.full_screen() {
            self.say("presenting full screen (§7.7.2's FullScreen page mode) — Escape comes back");
        }
        self.owing.owe(Owed::Chrome);
    }

    /// Runs commands until nothing is left, reacting to what each produces; a page that finished
    /// meanwhile answers the viewer, so the queue is drained again rather than the answer waiting
    /// for the next tick (ADR 0668).
    fn drain(&mut self, mut queue: VecDeque<Command>) {
        loop {
            while let Some(command) = queue.pop_front() {
                // Table 166's `/M` is read on the way to a save, at the one moment every window
                // reads it (ADR 1160).
                if let Some(clock) = viewer_host::modification::before(&command) {
                    self.run(clock, &mut queue);
                }
                self.run(command, &mut queue);
            }
            self.ask_for_windows();
            self.take_the_thread_back();
            self.take_the_drawn(&mut queue);
            if queue.is_empty() && std::mem::take(&mut self.state.showing.catalog_due) {
                self.obey_the_catalog(&mut queue, true);
            }
            if queue.is_empty() {
                break;
            }
        }
        // Idempotent and after every drain, which keeps the window a function of the viewer's
        // state rather than of the order events arrived in (ADR 0244 finding 5).
        self.repaint();
        self.owe_what_moved();
    }

    fn prepare(&mut self) {
        self.preparing = self.state.viewer.preparation().map(|preparation| {
            let (send, answer) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                // A window that has closed has nobody to give the answer to.
                let _ = send.send(preparation.run());
            });
            answer
        });
    }

    fn show_the_opening(&mut self) {
        let Some(reading) = self
            .state
            .anticipated
            .as_ref()
            .and_then(viewer_host::Loading::showing)
        else {
            return;
        };
        let sentence = viewer_host::loading::reading(reading);
        if sentence != self.status {
            self.bar_said = true;
            self.say(&sentence);
        }
    }

    fn take_down_the_bar(&mut self) -> bool {
        std::mem::take(&mut self.bar_said)
    }
}
