//! The window's whole state, and the questions it answers about itself.
//!
//! [`App`] is what winit hands every event to, so every other module in this program is an `impl`
//! block on it: the fields are here because a field read in two modules has to be declared in
//! one, and the methods here are the ones that belong to no single feature — the window's extent,
//! the title bar, and the lists a document states once and cannot change.

use std::path::PathBuf;
use std::sync::Arc;

use pdf_render::TargetSpec;
use viewer_core::{Answer, Command, Event, Query, Viewer};
use viewer_ui::chrome::{About, Chrome, FindBar, Sidebar};

use crate::arguments::Backend;
use crate::presentation::Presentation;
use crate::surface::State;
use crate::timing::{FrameLog, Launch};
use crate::trace::Trace;
use crate::typing::{Choosing, Drawing, Typing};

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about a window, each read in one place: whether a button is \
              down, which modifiers are held, whether the core has been told about the last \
              frame, whether anything is unsaved, and what this run asked of the graphics stack"
)]
pub(crate) struct App {
    /// Everything about documents, pages and clicks.
    pub(crate) viewer: Viewer,
    /// The file's name, for the title bar.
    pub(crate) title: String,
    /// The file itself, kept because §7.6.4.1's prompt has to open it again with a password.
    ///
    /// The path rather than the bytes: a host that held a copy of every document it had failed
    /// to open would be holding a copy of every document.
    pub(crate) path: PathBuf,
    /// The document this window is showing when it came out of another one — Annex O's `ef`.
    ///
    /// The bytes rather than a path, which is the exception [`Self::path`]'s rule needs and not a
    /// contradiction of it: §7.11.4 puts an embedded file *inside* the document, so there is no
    /// path to re-read when §7.6.4.1's prompt has to open it again. One file's worth, and only
    /// while a fragment's `ef` has opened one — `None` for every window that opened a file from
    /// the command line.
    pub(crate) embedded: Option<Vec<u8>>,
    /// Annex O's fragment identifier, kept for the same reason as the path.
    ///
    /// A document that asked for a password and got one is opened a second time, and the URI that
    /// named it said `#page=5` both times.
    pub(crate) fragment: Option<String>,
    /// The directory the open document is in, where one can be named.
    ///
    /// The whole of this program's answer to "which files may a document ask for". §12.7.6.4's
    /// import-data action carries a file specification the *document* wrote, so honouring it
    /// unrestricted would let a PDF read any path this process can — and the clause states no
    /// policy, because a policy is a property of the processor. See [`App::supply`].
    pub(crate) directory: Option<PathBuf>,
    /// What the title bar says about the page, from the last `PageChanged`.
    pub(crate) caption: String,
    /// The renders `viewer-core` has asked for, one per page of Table 29's arrangement, in page
    /// order — kept so an expose can redraw them.
    ///
    /// **A list since ADR 0442.** `Command::Layout`'s five continuous and two-page values put
    /// several pages in one window and the core asks for one render apiece; a host holding the
    /// newest would draw a column with a hole in it. What takes an entry *out* is
    /// [`App::arrangement`]: `Query::PageGeometry` answers `Answer::None` for a page the
    /// arrangement no longer shows, which is the only thing this host needed in order to follow a
    /// scroll and is why obeying the clause added no message to the boundary.
    pub(crate) requests: Vec<viewer_core::RenderRequest>,
    /// The tokens of those requests the core has not yet been told about.
    ///
    /// One per page rather than one flag, because an arrangement has one outstanding request per
    /// page on the screen and the token is what says which page an answer is about. Drained when
    /// a frame reaches the window.
    pub(crate) unacknowledged: Vec<viewer_core::RenderToken>,
    /// The page and placement last put on the screen, for §12.4.4's transition to draw from.
    ///
    /// **The page being left, exactly where it was.** A transition is a picture between two
    /// pages, and by the time `Event::Transition` arrives the core has already moved on — so the
    /// outgoing page is this, kept at the moment it was presented rather than reconstructed
    /// afterwards from a geometry that now describes the page arriving.
    pub(crate) presented: Option<(Arc<pdf_render::DisplayList>, TargetSpec)>,
    /// §12.4.4's presentation, while `p` has one running. `None` is a window reading a document.
    pub(crate) presentation: Option<Presentation>,
    /// Table 29's `/PageMode /FullScreen`, §12.2's chrome flags, and the way back out.
    ///
    /// **The window a presentation had never had.** Which of Table 147's flags this window is
    /// obeying and which page mode §12.2 says to come back to are decided by
    /// [`viewer_host::Presenting`], shared with the other two hosts because none of it is a fact
    /// about winit; what is this host's is `set_fullscreen` and the key that asks for it. ADR 0470.
    pub(crate) presenting: viewer_host::Presenting,
    /// Table 29's arrangement as this window last asked for it — what `l` cycles from.
    ///
    /// The *document's* value until somebody presses the key, because §7.7.2 states the layout a
    /// document opens in and says nothing about what a reader chooses afterwards.
    pub(crate) layout: pdf_model::viewer_preferences::PageLayout,
    /// A transition named but not yet begun, waiting for the arriving page's own display list.
    ///
    /// See [`App::arm_transition`]: the core settles after the page turn, so the render request
    /// for the page being moved to arrives one event *after* the transition that names it.
    pub(crate) arming: Option<pdf_model::navigation::Transition>,
    /// Where the pointer last was, in device pixels.
    ///
    /// `winit` reports movement and clicks as separate events, so a click needs the position
    /// remembered from the last `CursorMoved` — the click itself carries none.
    pub(crate) cursor: (f64, f64),
    /// What to say about what is happening, from `--trace`.
    ///
    /// A viewer that will not draw a page has to be able to say how far it got, and the four
    /// steps between a key press and a frame — command, interpretation, draw, present — are
    /// invisible from outside the process. This makes them visible, in order, with a duration
    /// apiece and a clock; the *last line printed* is the step that did not finish.
    pub(crate) trace: Trace,
    /// Every frame this run has drawn, for the summary printed when it exits.
    pub(crate) frames: FrameLog,
    /// Whether the window is showing a reprojection, and what it would take to draw the next one.
    ///
    /// **A view change whose frame takes most of a second used to move nothing at all.** This is
    /// the state behind [`crate::stale`]: the view the last real frame drew, what it cost, and
    /// whether what is on the screen right now is the real thing. It lives here rather than in
    /// `viewer-core` because the knowledge is a presenter's — when a frame was asked for, when it
    /// arrived, and what the last one cost — and because nothing that judges a picture may ever
    /// be able to reach an approximate one (`doc/todo/37` rule 2, ADR 0378).
    pub(crate) stale: crate::stale::Stale,
    /// How often this window may present, and when it next may.
    ///
    /// **`doc/todo/36`**: the presenter is a clock rather than a reaction, and this is the clock.
    /// It is the surface's own refresh rate from the moment there is a window (`resumed`), and
    /// `doc/todo/36`'s floor of 60 Hz until then and on a display that states none. Nothing wakes
    /// for it — `about_to_wait` arms it only while a frame is owed — so an idle window costs
    /// exactly what it did before this field existed.
    pub(crate) cadence: crate::cadence::Cadence,
    /// Whether to draw with `render-cpu` rather than the graphics device, from `--cpu`.
    ///
    /// The same rasteriser the reference oracle is built on, and the same one that draws a page
    /// the device refuses. As a *flag* it is a diagnostic a person can pull without a debugger:
    /// if a page appears under `--cpu` and not without it, the difference is the device.
    ///
    /// **And it now decides whether there is a device at all.** A run with this flag creates no
    /// instance, selects no adapter and makes no device, and presents through
    /// [`viewer_ui::software::SoftwareSurface`] instead — so it is also the answer to a driver
    /// that will not load, which is what the project owner needed it to be and what it was not.
    /// ADR 0221.
    pub(crate) processor: bool,
    /// How many whole pages this window retains a low-resolution picture of, from
    /// `--proxy-pages`.
    ///
    /// **A host's setting and it could not be anything else** (`doc/todo/37`, ADR 0443). The
    /// pictures are stand-ins, so rule 2 keeps everything that makes one inside this binary: a
    /// count that became a `Command` or a field of a boundary type would make a wrong picture
    /// visible to the tree that judges pictures. It sits beside [`Self::processor`] and
    /// [`Self::backend`] for the same reason those do.
    pub(crate) proxy_pages: usize,
    /// Which coverage lane draws the page, from `--coverage`: the magnification
    /// policy, or one lane pinned for a measuring session.
    pub(crate) coverage: crate::arguments::CoverageChoice,
    /// The lane the last asked frame used — the sticky half of
    /// [`crate::surface::lane_for`], so a chrome-only ask keeps its replay.
    pub(crate) lane: Option<quorra_gpu::Coverage>,
    /// Magnifications the atlas has drawn (linear parts, by bits, newest last, at most
    /// eight) — the revisit half of [`crate::surface::lane_for`].
    pub(crate) atlas_saw: Vec<[u32; 4]>,
    /// The settled frame's resolution factor, from `--supersample`: 1 off, 2 on.
    ///
    /// A host's setting for [`Self::proxy_pages`]'s reason: what it changes is how a
    /// picture is *presented*, which is this binary's alone, and nothing a gate could
    /// link to. The device path reads it; `--cpu` ignores it, because the composing
    /// thread draws at window resolution by construction.
    pub(crate) supersample: u32,
    /// The driver stack asked for, from `--backend` or from
    /// [`DEFAULT_BACKEND`](crate::arguments::DEFAULT_BACKEND).
    ///
    /// Kept past `main` for one job: `resumed` needs it to say what was asked for when no adapter
    /// matched, and to decide between refusing and falling back.
    pub(crate) backend: Option<Backend>,
    /// Whether a person named [`App::backend`] or this program defaulted to it.
    ///
    /// **A backend a person named is honoured or refused; one this program chose gives way.** A
    /// machine that has no adapter for the flag is a question with an answer — "this machine has
    /// no such adapter, here is what it has" — and answering it beats starting on a stack the
    /// person was trying to avoid. A machine that has no adapter for the *default* is a machine
    /// this project guessed wrong about, and it gets a note and every backend.
    pub(crate) backend_asked_for: bool,
    /// Whether the button is down, which is what separates a move from a drag.
    pub(crate) dragging: bool,
    /// Whether Ctrl is held, which is what separates a wheel scroll from a wheel zoom.
    ///
    /// `winit` reports a modifier change as its own event and puts nothing in the wheel's, so a
    /// host that wants to know has to remember. Ctrl + wheel is a convention rather than a
    /// clause, and it is the one every desktop viewer has converged on. ADR 0166.
    pub(crate) control: bool,
    /// Whether shift is held, which is the only thing that distinguishes §12.5.1's tab from
    /// its shift-tab: winit reports one key for both.
    pub(crate) shift: bool,
    /// A touchpad's accumulated pixels, spent one zoom step at a time.
    ///
    /// A wheel notch arrives as a line and a touchpad's pinch as a stream of pixels; sixteen
    /// pixels is one of this program's own text rows and means nothing to a magnification, so
    /// the pixels are counted up and a step taken per `WHEEL_ZOOM_PIXELS` rather than per event.
    pub(crate) pinch: f32,
    /// Whether anything a person did is unsaved.
    pub(crate) dirty: bool,
    /// §7.6.4.1's attempts, counted by [`viewer_host::Asking`] so that three hosts count alike.
    pub(crate) asking: viewer_host::Asking,
    /// §7.6.4.1's prompt, over the page — this host's own, because it has no toolkit to ask.
    pub(crate) password: viewer_ui::chrome::PasswordCard,
    /// Why there is no document, where there is none — `Event::OpenFailed`, or a page tree with no
    /// leaves. **Two `std::process::exit(1)` calls until the seven-hundred-and-fourth session**,
    /// which is `viewer_host::keys`' Escape-quits finding again: this host left the process where
    /// the other two said the sentence and stayed up.
    pub(crate) refused: viewer_ui::chrome::Refusal,
    /// Whether this window has ever put chrome up over no page at all.
    ///
    /// What it decides is what an *empty* overlay list means on a window with no page:
    /// before the first such frame it means the launch path — present nothing, so that no blank
    /// frame goes up in front of page one — and after it, take the chrome away. See
    /// [`App::without_a_page`](crate::surface).
    pub(crate) drawn_without_a_page: bool,
    /// Which document the prompt is about, from [`viewer_core::Event::PasswordRequired`].
    ///
    /// Kept because the card is answered on a later turn of the event loop than the one that put
    /// it up: the identity is the *host's* by construction (`Command::Open` names it), so it is
    /// carried rather than re-derived.
    pub(crate) locked: Option<viewer_core::DocumentId>,
    /// The fonts this program draws its own text with, or why it cannot.
    ///
    /// An `Option` because a build whose compiled-in faces will not parse must still show the
    /// document: the panel is chrome and the page is the point. The refusal is printed once.
    pub(crate) chrome: Option<Chrome>,
    /// The three lists a document keeps about itself, as this program draws them.
    pub(crate) panel: Sidebar,
    /// `/NOTICE`, over the page, which is the About panel the owner asked for.
    pub(crate) about: About,
    /// The find bar, and what a search has said. Opened with `/`.
    pub(crate) find: FindBar,
    /// How many pages the search in progress still has to read, zero when there is none.
    ///
    /// What makes the next frame ask for another step: `viewer-core` reads one page per
    /// `Find::Continue` and this host pumps them from its own event loop, so the window keeps
    /// drawing while a thousand-page document is searched. A count rather than a flag because it
    /// is also what the bar says.
    pub(crate) pages_left: usize,
    /// When the find bar's progress count was last repainted, for
    /// [`SEARCH_PROGRESS`](crate::find::SEARCH_PROGRESS).
    ///
    /// A clock, in a host, which is where `doc/ui-boundary.md` puts one: rule 3 leaves
    /// `viewer-core` without one precisely so that a host can spend its own.
    pub(crate) searched_at: Option<std::time::Instant>,
    /// §12.3.3's outline and §7.11.4's embedded files, taken once when the document opened.
    ///
    /// Copied out of the queries rather than asked for per frame, and not for speed: both are
    /// properties of an immutable document that no edit reaches, so a copy taken at open cannot
    /// go stale — which is exactly not true of §8.11's layers, whose whole point is that a click
    /// changes them, so those are asked for every time.
    ///
    /// (This note read "`Answer::Outline` borrows the viewer, and a panel that is about to send
    /// it a command cannot be holding a borrow of it" until ADR 0247 made that answer owned. The
    /// reason above is the one that survives, and it is the one that was always the stronger.)
    pub(crate) outline: pdf_model::outline::Outline,
    /// §7.11.4's embedded files, likewise.
    pub(crate) attachments: Vec<pdf_model::attachment::Attachment>,
    /// §12.4.3's article threads, likewise: `Query::Articles` reads them on demand and the list
    /// belongs to a document that no edit reaches.
    pub(crate) articles: Vec<pdf_model::article::Thread>,
    /// §12.3.5's collection, where the catalog states one — read once, like the rest.
    ///
    /// `None` for every document anyone has opened. Where it is `Some`, the files tab draws
    /// §12.3.5.2's folder tree and the schema's columns instead of a flat list, and §12.3.5.1's
    /// resolved `/D` decides which of its rows is the document the file says to open on.
    pub(crate) collection: Option<(
        pdf_model::collection::Collection,
        pdf_model::collection::Initial,
    )>,
    /// §14.3.3's Table 349, likewise.
    pub(crate) information: pdf_model::metadata::Information,
    /// §14.3.2's metadata stream, read — `None` where the catalog names none.
    ///
    /// **Was `metadata_stream: bool` until the two-hundred-and-ninety-fourth session**, when
    /// `pdf_model::xmp` gave `viewer-core` something to answer with. The three states matter to a
    /// host and only to a host: a document that states no metadata and one whose metadata this
    /// program could not read get two different sentences in the properties tab.
    pub(crate) metadata: Option<Result<pdf_model::xmp::Xmp, pdf_model::xmp::XmpError>>,
    /// The field a person is typing into: the point on the page that named it, and where in its
    /// value the next character goes.
    ///
    /// **The host keeps the point, not the text.** §12.7.5.3's `DoNotScroll` makes a field take
    /// only as much of a value as fits its rectangle (ADR 0197), so a buffer of what had been
    /// typed would diverge from the field on the first character past the edge — while a point is
    /// a place, and the field does not move. Every keystroke re-asks `Query::FieldAt` for the
    /// value the *document* now holds and sends that value plus one character back, which makes
    /// divergence impossible rather than unlikely.
    ///
    /// **The caret is an offset and nothing more.** Where it *is* on the screen is
    /// `Query::Caret`'s answer, because the place the next character will be drawn is
    /// §12.7.4.3's arithmetic and not this host's — the same division `Query::Selection` and
    /// `Query::Focus` already draw. The offset is clamped to the value after every edit, which is
    /// what keeps it inside a value the field truncated.
    ///
    /// `None` is a host that is not typing, which is every host until somebody clicks a field.
    pub(crate) typing: Option<Typing>,
    /// §12.7.5.4's choice field whose options are on the screen, if one is.
    ///
    /// **The tier-2 counterpart of a `GtkDropDown` and a `QComboBox`**, and it is state rather
    /// than a widget for the reason [`Choosing`] gives: what is kept is which field was pressed,
    /// and where its options go on the screen is derived from `Query::Fields` for every frame. A
    /// host that draws its own chrome has to hold this; a host that places somebody else's
    /// widgets has the toolkit hold it. ADR 0596.
    pub(crate) choosing: Option<Choosing>,
    /// What Ctrl + C and Ctrl + X took out of a field — or `c` took off the page — for Ctrl + V
    /// to put back.
    ///
    /// **The program's own, beside the platform's rather than instead of it** (ADR 0519). What a
    /// field's three keys mean is the *viewer's* question — which characters, which is the range
    /// `Query::FieldSelection` draws and `Edit::SetField` replaces, and nothing about it crosses
    /// the boundary, which is ADR 0225's finding. Where those characters *go* is the platform's,
    /// and [`App::platform_clipboard`] is now that end: this string is what Ctrl + V pastes back,
    /// and every copy that fills it also offers it to the session.
    ///
    /// Kept rather than read back from the platform, and deliberately: `arboard`'s X11 backend
    /// reads a clipboard by asking whoever owns the selection and waiting for them, so a paste
    /// that went to the platform would put another program's response time inside a keystroke.
    pub(crate) clipboard: String,
    /// The session's clipboard, connected the first time somebody copies and never at startup.
    ///
    /// **Not a `String` and not this host's**: X11's `CLIPBOARD` selection, Wayland's data device,
    /// `NSPasteboard` and `OpenClipboard` are four platforms' answers to one question, and
    /// `viewer_ui::clipboard` is where the one this program builds against is named. `viewer-core`
    /// has no command for it and needs none — `Query::Selection` and `Query::LogicalSelection`
    /// already answer with the text, which is `doc/todo/30`'s claim tested for the seventh time.
    pub(crate) platform_clipboard: viewer_ui::clipboard::Clipboard,
    /// §12.5.6.6's rectangle while a person is drawing one, in the page viewport's pixels.
    ///
    /// **`Some` only between `f` and the release that ends the drag**, which is a mode a highlight
    /// does not need: a free text annotation's geometry is a box a person drew rather than text
    /// they swept, so there is nothing on the page for a press to mean until they have drawn it.
    /// `f` arms it, the next press records the first corner, and the release sends
    /// `Edit::FreeText` with both. Which key, and that there is a mode at all, are this host's —
    /// the standard describes the annotation and says nothing about how a person comes to make
    /// one.
    pub(crate) drawing: Option<Drawing>,
    /// §12.3.4's tab: one entry per page, with its label and its decoded thumbnail.
    ///
    /// **Empty until that tab is first shown**, which is principle 2 with a clause behind it:
    /// §12.3.4's NOTE says thumbnails "are not required, and can be included for some pages and
    /// not for others", so building this list means decoding every miniature a document carries,
    /// and a document opens at a page rather than at a contact sheet. Filled once and kept,
    /// because a thumbnail is a property of an immutable document — the same argument the
    /// outline and the attachments are cached under, and exactly not the layers'.
    pub(crate) pages: viewer_host::Miniatures<pdf_render::Image>,
    /// How many rows §12.3.4's tab has, which is the document's page count.
    pub(crate) page_count: usize,
    pub(crate) state: Option<State>,
    /// The thread opening the document, until the window and the device have been brought up.
    ///
    /// `None` from the moment it is joined, which is the first thing `resumed` does after the
    /// presenter exists — so every later event, command and query sees an ordinary `Viewer` and
    /// nothing else in this file knows a thread was involved.
    pub(crate) opening: Option<std::thread::JoinHandle<(Viewer, Vec<Event>)>>,
    /// The thread creating the graphics instance, which is 80% of what bring-up blocks for.
    ///
    /// **`None` under `--cpu`, where it was never spawned** — the one place in this program that
    /// distinguishes "the instance is not ready yet" from "there will not be one", and both read
    /// as `None` here only because `resumed` asks `processor` first.
    pub(crate) instancing: Option<std::thread::JoinHandle<quorra_gpu::wgpu::Instance>>,
    /// The launch path's milestones, printed once under `--trace` when the first frame lands.
    pub(crate) launch: Launch,
    /// §14.7's structure on AT-SPI, once there is a page to put there.
    ///
    /// **`None` until the first frame has been presented**, and that is `CLAUDE.md`'s startup
    /// rule rather than laziness for its own sake: bringing this up creates a thread and a D-Bus
    /// connection, and page one may not wait behind either. Once it exists it costs nothing per
    /// page — `accesskit_unix` keeps every adapter inactive until an assistive technology is
    /// actually present, so publishing a page a screen reader is not reading is a lock and a
    /// clone. ADR 0214.
    pub(crate) accessibility: Option<viewer_accessibility::Bridge>,
    /// The page and viewport last handed to it, so that one is not rebuilt per frame.
    ///
    /// A scroll and a zoom leave the structure alone; a page turn replaces it, and a resize moves
    /// every rectangle in it. Those are the two things this remembers, and everything else a
    /// person does costs the bridge nothing.
    pub(crate) spoken: Option<viewer_accessibility::Showing>,
    /// What wakes this event loop from a thread that is not in it.
    ///
    /// **The one thing in this program that arrives from outside the window system.**
    /// `accesskit_unix` runs its D-Bus connection on a thread of its own, so a client's action
    /// request is handed to [`App::act`] through a queue rather than through winit — and a window
    /// resting in `ControlFlow::Wait` would not come round to read it. This is winit's only answer
    /// to that, and it is the whole of what the user event carries.
    ///
    /// `None` before the loop exists, which is every path that never gets one: the constructor
    /// runs before `EventLoop::new`, and a bridge given a waker that does nothing still publishes
    /// its tree.
    pub(crate) waker: Option<winit::event_loop::EventLoopProxy<()>>,
}

impl App {
    /// The window's extent in device pixels and its scale factor, once there is a window.
    pub(crate) fn window(&self) -> Option<(u32, u32, f32)> {
        let state = self.state.as_ref()?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = state.window.scale_factor() as f32;
        Some((state.size.0, state.size.1, scale))
    }

    /// How many device pixels down the left edge the panel occupies.
    ///
    /// **The page's viewport is the window less this.** A panel drawn *over* the page would
    /// hide part of it and leave the core centring the page behind the panel; telling the core
    /// about the smaller viewport instead is what makes a fitted page fit what is visible. It
    /// also means every coordinate crossing the boundary — a pointer going in, a selection quad
    /// coming out — is offset by exactly this and nothing else.
    pub(crate) fn inset(&self) -> u32 {
        self.window()
            .map_or(0, |(_, _, scale)| self.panel.inset(scale))
    }

    /// Tells the core how much of the window is the page's, after the panel appeared or went.
    pub(crate) fn resize_page(&mut self) {
        let Some((width, height, scale)) = self.window() else {
            return;
        };
        self.dispatch(Command::Resize {
            width: width.saturating_sub(self.inset()).max(1),
            height,
            scale,
        });
        self.redraw();
    }

    /// Whether the pointer is over the panel rather than over the page.
    pub(crate) fn over_panel(&self) -> bool {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        edge > 0.0 && at(self.cursor).0 < edge
    }

    /// A window point in the page's own viewport, which begins where the panel ends.
    pub(crate) fn on_page(&self, cursor: (f64, f64)) -> (f32, f32) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        let (x, y) = at(cursor);
        (x - edge, y)
    }

    /// Asks the window to draw again, where there is one.
    pub(crate) fn redraw(&self) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }

    /// Puts the caption in the title bar.
    pub(crate) fn retitle(&self) {
        if let Some(state) = self.state.as_ref() {
            let mark = if self.dirty { "• " } else { "" };
            state
                .window
                .set_title(&format!("{mark}{} — {}", self.named(), self.caption));
        }
    }

    /// What the title bar calls the document — §12.2's `/DisplayDocTitle`.
    ///
    /// Table 147: "[a] flag specifying whether the window's title bar should display the
    /// document title taken from the `dc:title` entry of the XMP metadata stream … If false, the
    /// title bar should instead display the name of the PDF file containing the document."
    ///
    /// **The clause is obeyed as written since the two-hundred-and-ninety-fourth session.** It
    /// names `dc:title` and nothing else, and `pdf_model::xmp` reads it, so that is what a
    /// document asking for its title gets. §14.3.3's `/Info /Title` is the *fallback* now rather
    /// than the substitution: it is used where the document states no metadata stream, where the
    /// stream states no `dc:title`, or where the stream could not be read — and the last of those
    /// three is printed, because it is the only one where this program failed at something.
    ///
    /// Table 349's NOTE 1 is why the fallback is a reading rather than a guess: "[t]he `dc:title`
    /// entry in the document's metadata stream **can be used to represent** the document's
    /// title." Measured over the corpus, 93 documents state a title in both places and one
    /// disagrees, so the ranking is what decides a single file (ADR 0186).
    pub(crate) fn named(&self) -> &str {
        if !self.display_doc_title() {
            return &self.title;
        }
        let stated = self
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.as_ref().ok())
            .and_then(pdf_model::xmp::Xmp::title)
            .or(self.information.title.as_deref());
        stated
            .filter(|title| !title.is_empty())
            .unwrap_or(&self.title)
    }

    /// Table 147's `/DisplayDocTitle`, **default false**.
    pub(crate) fn display_doc_title(&self) -> bool {
        matches!(
            self.viewer.query(Query::Preferences),
            Answer::Preferences(preferences) if preferences.display_doc_title
        )
    }

    /// Adds what the pages on the screen could not draw to the title bar.
    ///
    /// A count rather than the list: a page may report dozens of items and a title bar that
    /// scrolls off the screen tells a person less than a number does. The items themselves are
    /// printed, in the core's own words.
    ///
    /// **Asked rather than counted from the event**, since Table 29's arrangements were obeyed:
    /// the events arrive one page at a time as each is interpreted, so a title written from the
    /// last of them would say what one page of a column of four could not draw. `Query::Reports`
    /// answers for every page on the screen, which is what the window is showing.
    pub(crate) fn retitle_incomplete(&self) {
        let Answer::Reports(pages) = self.viewer.query(Query::Reports) else {
            return;
        };
        let items: usize = pages.iter().map(|page| page.notes.len()).sum();
        if items == 0 {
            return;
        }
        let named = pages.iter().filter(|page| !page.notes.is_empty()).count();
        if let Some(state) = self.state.as_ref() {
            state.window.set_title(&format!(
                "{} — {} — incomplete: {items} item(s) not drawn on {named} page(s) on screen",
                self.title, self.caption
            ));
        }
    }

    /// Takes the lists a document cannot change, once, when it opens.
    ///
    /// §12.3.3's outline, §7.11.4's embedded files and §12.4.3's article threads — all three
    /// properties of an immutable document, so what the panel holds is a copy that cannot go
    /// stale. §8.11's layers are *not* here for exactly that reason.
    pub(crate) fn gather(&mut self) {
        if let Answer::Outline(outline) = self.viewer.query(Query::Outline) {
            self.outline = outline;
        }
        if let Answer::Attachments(files) = self.viewer.query(Query::Attachments) {
            self.attachments = files;
        }
        if let Answer::Articles(threads) = self.viewer.query(Query::Articles) {
            self.articles = threads;
        }
        self.collection = match self.viewer.query(Query::Collection) {
            Answer::Collection {
                collection,
                initial,
            } => Some((collection, initial)),
            _ => None,
        };
        if let Answer::Properties {
            information,
            metadata,
        } = self.viewer.query(Query::Properties)
        {
            self.information = information;
            self.metadata = metadata;
        }
        // §12.2 names XMP's `dc:title` and this program now reads it; see `named`. What is left
        // to say out loud is the case where it *could not* — a document that asks for its title
        // and whose metadata stream this reader refused, which is the one situation where the
        // fallback to §14.3.3's `/Info /Title` is still a substitution rather than the clause.
        if let Some(Err(error)) = self.metadata.as_ref() {
            println!("note: this document's §14.3.2 metadata stream could not be read: {error}");
            if self.display_doc_title() {
                println!(
                    "note: it also asks for its title in the title bar (§12.2's \
                     /DisplayDocTitle), which names XMP's dc:title; §14.3.3's /Info /Title is \
                     shown instead"
                );
            }
        }
        self.retitle();
        self.obey_page_mode();
        let layers = self.layers().len();
        if !self.outline.items.is_empty()
            || !self.attachments.is_empty()
            || !self.articles.is_empty()
            || layers > 0
        {
            println!(
                "{}: {} outline item(s), {layers} layer entr(ies), {} embedded file(s), {} \
                 article thread(s) — press o for the panel",
                self.title,
                self.outline.visible_count(),
                self.attachments.len(),
                self.articles.len()
            );
        }
    }

    /// Table 29: opens the window and the panel the document asks for.
    ///
    /// §7.7.2's `/PageMode` is "how the document shall be displayed when opened", and until the
    /// hundred-and-seventieth session this program had no panel for any of its answers to name.
    /// Four of the six were obeyed from the two-hundred-and-sixty-sixth, when §12.3.4's panel
    /// arrived; **`FullScreen` is the fifth and is obeyed since ADR 0470** — the note this used to
    /// print, that full screen "is chrome this program does not have", was true for four hundred
    /// sessions and is now the window `p` opens. `UseNone` asks for nothing and is the sixth.
    ///
    /// **`/PageLayout` is obeyed here as it stands**, and this host asks the viewer for nothing.
    ///
    /// The six-hundred-and-sixth session left this asking the viewer *back* for `SinglePage`,
    /// because a tier-2 surface drew exactly one `Arc<DisplayList>` per frame. It draws the
    /// arrangement now (ADR 0442), so what is left is the value `l` cycles **from** — the core
    /// read the catalog for itself when the document opened, and a host that started its cycle at
    /// `SinglePage` would move the first press onto what the document already asked for.
    fn obey_page_mode(&mut self) {
        use pdf_model::viewer_preferences::PageLayout;
        let Answer::Opening(opening) = self.viewer.query(Query::Opening) else {
            return;
        };
        let Answer::Preferences(preferences) = self.viewer.query(Query::Preferences) else {
            return;
        };
        self.presenting = viewer_host::Presenting::opening(opening, &preferences);
        self.report_unobeyable_chrome();
        self.show_page_mode(opening.mode);
        self.layout = opening.layout;
        if opening.layout != PageLayout::SinglePage {
            println!(
                "note: this document opens in the {:?} page layout (§7.7.2) — press l for the \
                 next of Table 29's six",
                opening.layout
            );
        }
    }

    /// Shows the document the way one of Table 29's six page modes asks for.
    ///
    /// Two callers, and that is the point of it being a function: the document opening, where
    /// §7.7.2 states "how the document shall be displayed when opened", and full screen ending,
    /// where §12.2's `/NonFullScreenPageMode` states "how to display the document on exiting
    /// full-screen mode". One mapping, so the two sentences cannot drift apart.
    ///
    /// `FullScreen` cannot arrive from the second caller: `pdf_model`'s reader refuses that name
    /// for `/NonFullScreenPageMode`, because it is that entry's own condition.
    pub(crate) fn show_page_mode(&mut self, mode: pdf_model::viewer_preferences::PageMode) {
        use pdf_model::viewer_preferences::PageMode;
        // Which panel each name opens is `viewer_host::Tab`'s, shared with the other two hosts
        // since `doc/todo/30`'s item 4; what is left here is `FullScreen`, which is not a panel.
        if let Some(tab) = viewer_host::Tab::of_page_mode(mode) {
            self.panel.show(tab);
        } else if matches!(mode, PageMode::FullScreen) {
            self.begin_presentation();
        }
    }

    /// The next of Table 29's six arrangements, which is what `l` asks for.
    ///
    /// The *order* is `viewer_host::next_layout`, shared with the other two hosts because what
    /// the next arrangement is is not a fact about a toolkit; which key says so is this host's.
    pub(crate) fn cycle_layout(&mut self) {
        self.layout = viewer_host::next_layout(self.layout);
        println!("page layout: {:?} (§7.7.2)", self.layout);
        self.dispatch(Command::Layout(self.layout));
    }

    /// Whether anything on the page is selected.
    ///
    /// Asked before §12.5.6.10's markup, which is defined over selected text: the core does
    /// nothing when there is nothing to mark up, and a person who pressed a key and saw no change
    /// has been told nothing at all.
    pub(crate) fn has_selection(&self) -> bool {
        matches!(self.viewer.query(Query::Selection),
            Answer::Selected(selection) if !selection.quads.is_empty())
    }
}

/// A window position as the device pixels `viewer-core` speaks in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a window coordinate is a small number of pixels"
)]
pub(crate) fn at(cursor: (f64, f64)) -> (f32, f32) {
    (cursor.0 as f32, cursor.1 as f32)
}
