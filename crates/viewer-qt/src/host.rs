//! One document, one viewer, and everything the Qt side asks it for.
//!
//! The loop is the one `viewer-ui`, `tests/headless.rs` and `viewer-gtk` already run — a command
//! in, the events it produced out, and the ones asking for work answered until nothing is left —
//! because the vocabulary is what is being tested and a host that invented its own scheduling
//! would be testing that instead.
//!
//! # No Qt type appears in this module
//!
//! That is not a style rule; it is what makes the crate's `unsafe` position defensible. Every
//! `unsafe` token in `viewer-qt` is inside one macro expansion in `crate::bridge`, and it stays
//! there because the Rust half of the host never touches a `QObject`. What crosses is values:
//! rows, controls, quadrilaterals, a frame's dimensions and a borrowed slice of its pixels. Every
//! `pub(crate)` method on [`Host`] is something the C++ side calls, and each is a `Command` or a
//! question; none of them knows what a widget is.
//!
//! # Where the state lives, and why it is a plain `Box`
//!
//! C++ owns this struct for the whole life of `QApplication::exec` and calls into it from Qt's
//! signal handlers, one at a time, on the one thread Qt runs its event loop on. There is no path
//! from here back into Qt, so there is no re-entrancy to guard: `viewer-gtk` needs
//! `Rc<RefCell<Host>>` and a `try_borrow_mut` because `gtk4-rs` callbacks are `'static` closures
//! the Rust side installs, and this host needs neither because the callbacks are C++ lambdas.
//! Two ownership models, one vocabulary — ADR 0246.
//!
//! # One struct, its methods filed by concern
//!
//! [`Host`] is one state machine and this module holds it, the document in front
//! ([`Showing`]) and the constructor; each child module is one concern's `impl Host` block with
//! the free items only that concern uses, so a method is found by what it is about (ADR 1908):
//! [`opening`] the launch, [`react`] the loop, [`drawing`] the drawing thread, [`stand_in`] what
//! the page area paints, [`presentation`] §12.4.4's clock and transitions, [`window`] the title,
//! the status line and the window's placement, [`input`] the keys and the pointer, [`tabs`] the
//! strip of open documents, [`faces`] the machine's faces, [`panels`] the six panels, [`find`] the
//! find bar, [`forms`] §12.7's placed controls, [`notes`] §12.5.6.14's popups, [`questions`] the
//! dialogues a person answers, [`restrictions`] the levels menu, and [`printing`] RFC 0004's print
//! path. What a document asks of this machine is carried out by `viewer_host::native_window` for
//! every window, and this one holds the dialogue (ADR 2105).

mod archiving;
mod drawing;
pub use drawing::device_lane;
mod faces;
mod find;
mod forms;
mod input;
mod notes;
mod opening;
mod panels;
mod presentation;
mod printing;
mod questions;
mod react;
mod restrictions;
mod stand_in;
mod tabs;
#[cfg(test)]
mod tests;
mod window;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use pdf_model::view::WidgetAppearances;
use render_cpu::CpuRasterizer;
use viewer_core::{Command, DocumentId, FormField};
use viewer_host::native_window::{
    Asked, DOCUMENT, Launch, NativeWindow, Refusals, Showing, WindowState,
};
use viewer_host::panel::Tab;
use viewer_host::trace::{Topic, Trace};

use crate::bridge::ffi::{QtQuad, QtUpdate};

pub(crate) use notes::{leader_cycles, reached_stop, rule_pieces, rule_thickness};

use forms::Placement;
use panels::Flat;
use printing::{PrintJob, PrintedPage};

/// What the contents panel says while the launch document's outline is read beside page one
/// (ADR 1553): a sentence about this window, since the document's answer is not in yet.
const READING_THE_OUTLINE: &str = "Reading the outline.";

/// The window this host asks Qt for, in logical pixels.
///
/// The same 1000×1100 `viewer-gtk` asks for, for the same reason: two hosts whose windows are the
/// same size produce launch numbers that can be read side by side.
const WINDOW: (i32, i32) = (1000, 1100);

/// Why the host could not start.
#[derive(Debug, thiserror::Error)]
pub enum HostError {
    /// The file named on the command line could not be read.
    #[error("cannot read {path}: {error}")]
    Unreadable {
        /// What was named.
        path: PathBuf,
        /// What the operating system said.
        error: String,
    },
}

/// How many panels this window has, which is `viewer_host::Tab`'s own count.
///
/// §12.3.3's outline, §12.3.4's miniatures, §8.11.4.3's `/Order`, §7.11.4's embedded files,
/// §12.4.3's threads and §14.3.3's information. Five of the six are answers of five different
/// types that `viewer_host::panel` turns into one row shape, so the only thing left to distinguish
/// is which panel a row is in and it crosses the bridge as an index into
/// [`viewer_host::Tab::ALL`]. The sixth is §12.3.4's and holds no rows at all — its pictures are
/// asked for a page at a time by [`Host::page_row`] — so its slot here stays empty and the C++
/// side puts a `QListView` where the other five have a `QTreeView`.
const PANELS: usize = Tab::ALL.len();

/// One document, one viewer, and the loop between them.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about one window, each read in one place and none a state"
)]
pub struct Host {
    /// The toolkit-free half of this window's state, shared with the other toolkit host
    /// (`viewer_host::native_window`, ADR 1920).
    pub(crate) state: WindowState,
    /// The machine's faces for characters of a tab's or a panel row's label the compiled-in face
    /// lacks, asked by file off this thread (ADRs 1406, 1418), and how many of them the window has
    /// registered with Qt.
    ///
    /// Qt's own fallback goes by *family*, and on a machine where the face covering a label shares
    /// its family name with Latin faces it picks one of those and draws boxes; the file the covering
    /// search names, registered with `QFontDatabase`, is what the tab strip and the panels are then
    /// set in.
    faces: viewer_host::machine_faces::MachineFaces,
    /// How many of [`Self::faces`] the C++ side has taken.
    faces_taken: usize,
    /// Which document the password prompt now on its way to the screen is about.
    ///
    /// The prompt is one `QDialog` and the answer comes back as a bare string, so this is how the
    /// answer finds a document that is not yet in the strip.
    prompting: DocumentId,
    /// Whether the next of [`WindowState::arrivals`] may start, which C++ answers with
    /// [`Host::arrive`].
    arrival_due: bool,
    /// Whether the panels are to be built again when the pump now running has finished, because
    /// a `Command::Focus` on its queue has moved the front and every panel is an answer about the
    /// focused document.
    panels_due: bool,
    /// Whether the strip of documents has changed since C++ last asked.
    documents_moved: bool,
    /// Whether the last document was closed, so the window should go with it.
    closing: bool,
    /// Tier 1's worker for the pictures this window makes *for itself* — §12.3.4's miniatures and
    /// §12.4.4.1's two transition faces.
    ///
    /// **The page is no longer drawn here**, which is ADR 0668: a display list this program did not
    /// write is drawn on [`WindowState::drawing`]'s thread, so that a page written to be expensive
    /// cannot take `QApplication::exec` with it. What is left on this thread is bounded by *this
    /// host's* own asking rather than by what a document chose to state.
    rasterizer: CpuRasterizer,
    /// Device pixels per logical pixel, from the screen Qt put the window on.
    scale: f32,
    /// Whether the reader wants the panel of three trees on the screen.
    ///
    /// **A wish, beside Table 29's *permission***. [`Host::chrome`] shows the panel only where the
    /// clause allows one and the person asked for one, so leaving full screen puts back what the
    /// reader had. `o` is the key, in all three hosts since ADR 0526.
    panel_shown: bool,
    /// Whether §8.11.4.5's print operation is running, which is what this window shows while it is.
    ///
    /// RFC 0004 §6's preview is on every page while it stands — Table 167's bit 3 applied to the
    /// annotations, §8.11.4.5's `Print` event to the layers, §12.5.6.22's watermarks against the
    /// sheet — and the same key ends it. A build with `QtPrintSupport` opens a dialogue on top of
    /// it and ends the operation when the job is over (ADR 1203).
    printing: bool,
    /// What the grant said about the job, for the dialogue that opens on it.
    ///
    /// `None` where none is running. Read once rather than queried per dialogue field, because
    /// every one of §12.2's Table 147 entries is a statement about the moment the dialogue opens.
    print: Option<PrintJob>,
    /// ISO 19005's conversion being chosen, for the dialogue `QtUpdate::archive_dialogue`
    /// announced; `None` where none is (ADR 1966).
    archive_choice: Option<viewer_host::archiving::Choosing>,
    /// The page the print job last asked for, rasterised and waiting for the C++ side.
    ///
    /// One at a time, for `viewer_ffi::Session`'s reason: an A4 page at 300 dots per inch is 35
    /// megabytes, and a print job draws a page, spools it and moves on.
    printed: Option<PrintedPage>,
    /// `CLAUDE.md`'s *ask* level: what the standing question is about, and what it says.
    ///
    /// One at a time, which is `viewer_core::Command::Answer`'s own rule — a second question
    /// replaces the first — and `None` while nothing is outstanding, so that a dialogue closed
    /// twice answers once.
    question: Option<(Asked, String)>,
    /// RFC 0008 section 4.2's `app.alert` or `app.response` a script is waiting on: which
    /// document's, what that document is called, and what it asks — taken once, by the answer,
    /// so that a dialogue closed twice answers once (ADR 1628).
    script_question: Option<(DocumentId, String, viewer_core::ScriptQuestion)>,
    /// What to put above the entry, worded when the prompt is asked for and read once by C++.
    prompt: String,
    /// The most recent sentence for a person.
    message: String,
    /// The three trees.
    trees: [Vec<Flat>; PANELS],
    /// The fields of the page being shown, as `Query::Fields` last answered.
    fields: Vec<FormField>,
    /// One entry per control, in the order the C++ side placed them.
    placed: Vec<Placement>,
    /// §12.5.6.14's open windows, as `Query::Popups` last answered.
    ///
    /// Kept rather than asked for twice: `refresh` compares this against the new answer to decide
    /// whether the C++ side has to rebuild the widgets, and `popups` then hands over what the
    /// comparison was made on. `viewer_core::PopupWindow` is `PartialEq`, which is the whole test.
    popups_shown: Vec<viewer_core::PopupWindow>,
    /// Whether the pointer was last over §12.5.6.5's activation region.
    ///
    /// Kept so that the cursor is changed when the answer changes rather than on every motion
    /// event, which is the same economy `viewer-gtk` makes for the same reason.
    over_link: bool,
    /// The keys the core was last told of (ADR 1771).
    keys_told: pdf_model::view::Keys,
    /// What has changed since the C++ side last asked.
    update: QtUpdate,
    /// §14.7's tree on AT-SPI, brought up after the first frame and never before it (ADR 0623).
    pub(crate) accessibility: Option<viewer_accessibility::Bridge>,
    /// The page and viewport last published to it, so that a tree is not rebuilt per frame.
    pub(crate) spoken: Option<viewer_accessibility::Showing>,
    /// Where the window is on the screen, as Qt last reported it: the frame, then the contents.
    ///
    /// **The half of AT-SPI's geometry `viewer-gtk` has no answer for.** It is kept here rather
    /// than handed straight on because a `moveEvent` arrives before the first paint and the
    /// adapter does not exist until after it.
    pub(crate) window_at: Option<crate::access::WindowPlace>,
    /// Whether this window has the keyboard, as Qt last reported it — kept for `window_at`'s
    /// reason: the window is activated before the first paint, and the adapter does not exist
    /// until after it (ADR 1565).
    pub(crate) window_active: bool,
    /// The field and widget an assistive technology's click aimed the keyboard at, until C++
    /// asks for it after the drain (ADR 1566).
    pub(crate) aimed: Option<(pdf_model::view::FieldName, pdf_syntax::ObjectId)>,
    /// Where the page area's top-left corner sits in the window's contents, in device pixels, as Qt
    /// last reported it — the document node's transform, so a node is placed where the page is
    /// rather than at the contents' corner (ADR 1528).
    pub(crate) page_at: (f32, f32),
    /// Set from `accesskit_unix`'s own thread when a client asks for something.
    pub(crate) access_pending: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// The travel a Control + wheel gesture has made and not yet spent on a zoom step.
    zoom_wheel: viewer_host::ZoomWheel,
    /// The magnification at which every control on this page would fit its `/Rect`, where they do
    /// not fit now.
    ///
    /// **ADR 0245's third decision, on the far side of a bridge.** `viewer_host::ControlFit`
    /// computes it from what `Query::Fields` answers and what Qt says a control's minimum is, and
    /// the second of those two is the reason this host was the last to have it: a
    /// `minimumSizeHint` is a C++ value, so the numbers have to cross before the arithmetic can
    /// happen. `w` sends the answer as `Zoom::Scale`, as it does in `viewer-gtk`; it is offered
    /// rather than applied, because a viewer that magnified a page by itself because a form is on
    /// it would be answering a question nobody asked (rule 5).
    fit_magnification: Option<f32>,
    /// When this window last told the core that time passed for a document's script timers, which
    /// the same `QTimer` wakes for while no presentation does (ADR 1702).
    script_ticker: viewer_host::script_timers::Ticker,
    /// The frame of a transition in flight, which is what this window shows instead of a page.
    ///
    /// A whole `Raster` rather than a display list, because what crosses this bridge is pixels:
    /// `frame_pixels` hands the C++ side a borrowed slice and a transition frame has to be
    /// somewhere for that borrow to point at.
    playing: Option<pdf_render::Raster>,
    /// Whether the bar of a document still opening past `WARN` has been said on the terminal,
    /// which it is once, the moment it goes up.
    bar_said: bool,
    /// The launch document's outline and placed page tree, being read on a thread of their own
    /// from the join until they are handed back ([`viewer_core::Preparation`], ADR 1553).
    preparing: Option<std::sync::mpsc::Receiver<viewer_core::Prepared>>,
    /// §14.8.2.5's text between the key that copied it and the C++ side taking it to `QClipboard`.
    ///
    /// Empty at every other moment, because `take_clipboard` clears it: this is a hand-over and
    /// not a second clipboard. `viewer-ui` keeps its string for a different reason — it is what
    /// Ctrl + V pastes into a field — and this host pastes with `QLineEdit`'s own binding, so
    /// there is nothing here to keep.
    clipboard: String,
}

impl std::fmt::Debug for Host {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Host")
            .field("path", &self.state.showing.path)
            .field("dirty", &self.state.showing.dirty)
            .field("placed", &self.placed.len())
            .finish_non_exhaustive()
    }
}

/// One `[x0, y0, … x3, y3]` as the bridge carries it.
fn quad(corners: [f32; 8]) -> QtQuad {
    QtQuad {
        x0: corners[0],
        y0: corners[1],
        x1: corners[2],
        y1: corners[3],
        x2: corners[4],
        y2: corners[5],
        x3: corners[6],
        y3: corners[7],
    }
}

/// Every flag clear, which is what `take_update` leaves behind.
fn nothing_changed() -> QtUpdate {
    QtUpdate {
        frame: false,
        panels: false,
        controls: false,
        chrome: false,
        popups: false,
        cursor: false,
        title: false,
        status: false,
        password: false,
        question: false,
        script_question: false,
        menu: false,
        window: false,
        clipboard: false,
        find_bar: false,
        notices: false,
        print_dialogue: false,
        archive_dialogue: false,
        documents: false,
        choose_document: false,
        placement: false,
        beep: false,
    }
}

/// A colour component in `0.0..=1.0` as the 0..=255 level a `QColor` takes.
///
/// Shared by `pdf_render::SURROUND` and Table 166's `/C`, because two roundings of one convention
/// is exactly the kind of thing that comes to differ by a level and be argued about.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a colour component in 0..=1 scaled by 255"
)]
fn level(component: f32) -> u8 {
    (component.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// A colour as Qt's `QRgb` takes it, `0xRRGGBB`.
fn packed(colour: pdf_render::Color) -> u32 {
    u32::from(level(colour.r)) << 16 | u32::from(level(colour.g)) << 8 | u32::from(level(colour.b))
}

/// What to call the document in a title bar: its file name, or the whole path where it has none.
fn named(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

impl Host {
    /// Reads the document's bytes and builds the host around them.
    ///
    /// **The document opens here, on a thread of its own, and page one is interpreted there**
    /// (ADR 1539): Qt's application, its window and the first resize take longer than the open
    /// does, and need nothing of it. The thread's `Viewer` has no viewport, so nothing is drawn
    /// until the first resize joins it — the viewport's size decides the resolution page one is
    /// rasterised at, and a page drawn at a guessed size would be drawn twice. The sequence is
    /// [`Launch`], which `viewer-gtk` and the C window start alike, so the launch timelines
    /// measure the same thing (ADR 2172).
    ///
    /// `reader` is the reader's policy words, which become their commands on that thread and
    /// before the document (ADR 1580); what a named directory refused is said on the terminal.
    ///
    /// # Errors
    ///
    /// [`HostError::Unreadable`] where the file named cannot be read.
    pub fn open(
        path: &Path,
        fragment: Option<String>,
        widget_appearances: WidgetAppearances,
        settings: viewer_host::Settings,
        reader: viewer_host::ReaderWords,
        trace: Trace,
    ) -> Result<Self, HostError> {
        // Open on disk rather than read whole: the core reads what page one needs through the
        // handle, and the file's size stops being the launch's cost (ADR 0809).
        let bytes =
            pdf_syntax::FileBytes::on_disk(path).map_err(|error| HostError::Unreadable {
                path: path.to_owned(),
                error: error.to_string(),
            })?;
        trace.say(
            Topic::Launch,
            format_args!("opened {} bytes of {} on disk", bytes.len(), path.display()),
        );
        let launch = Launch::start(
            bytes,
            fragment,
            widget_appearances,
            settings,
            reader,
            Refusals::Terminal,
        );
        Ok(Self {
            state: launch.state(path, trace),
            // No thread and nothing read until a label holds a character the compiled-in face
            // lacks, and then off this thread (ADR 1406).
            faces: viewer_host::machine_faces::MachineFaces::new(),
            faces_taken: 0,
            prompting: DOCUMENT,
            arrival_due: false,
            panels_due: false,
            documents_moved: false,
            closing: false,
            rasterizer: CpuRasterizer::new(),
            scale: 1.0,
            // The panel is what this window opens with, and `o` is what takes it away.
            panel_shown: true,
            printing: false,
            print: None,
            archive_choice: None,
            printed: None,
            question: None,
            script_question: None,
            prompt: String::new(),
            message: String::new(),
            trees: std::array::from_fn(|_| Vec::new()),
            fields: Vec::new(),
            placed: Vec::new(),
            popups_shown: Vec::new(),
            over_link: false,
            keys_told: pdf_model::view::Keys::default(),
            update: nothing_changed(),
            accessibility: None,
            spoken: None,
            window_at: None,
            window_active: false,
            aimed: None,
            page_at: (0.0, 0.0),
            access_pending: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zoom_wheel: viewer_host::ZoomWheel::default(),
            fit_magnification: None,
            script_ticker: viewer_host::script_timers::Ticker::default(),
            playing: None,
            bar_said: false,
            preparing: None,
            // Table 147's and Table 29's own defaults, replaced by what the catalog states the
            // moment the document opens.
            clipboard: String::new(),
        })
    }
}

/// What Qt does inside the methods the two toolkit hosts share: the update flags the C++ side reads
/// for the status line and the frame, the drain, and a flag the next drain reads for the next open
/// (ADR 1920).
///
/// Each forwards to this host's own method of the same name, which a path through the type
/// reaches before the trait's: inherent methods are looked up first.
impl NativeWindow for Host {
    fn state(&self) -> &WindowState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut WindowState {
        &mut self.state
    }

    fn say(&mut self, what: &str) {
        Self::say(self, what);
    }

    fn dispatch(&mut self, command: Command) {
        Self::dispatch(self, command);
    }

    fn later_open_the_next(&mut self) {
        Self::later_open_the_next(self);
    }

    fn repaint(&mut self) {
        self.update.frame = true;
    }

    fn offer_to_archive(&mut self, choosing: viewer_host::archiving::Choosing) {
        self.offer_the_archive_choice(choosing);
    }

    fn opened(&mut self, document: DocumentId, pages: usize, queue: &mut VecDeque<Command>) {
        Self::opened(self, document, pages, queue);
    }

    fn password_required(&mut self, document: DocumentId) {
        Self::password_required(self, document);
    }

    fn put_the_question(&mut self, about: Asked, words: &viewer_host::Question) {
        Self::put_the_question(self, about, words);
    }

    /// One dialogue, so one question at a time.
    fn question_waiting(&self) -> bool {
        self.question.is_some()
    }

    fn withdraw_script_question(&mut self, document: DocumentId) {
        Self::withdraw_script_question(self, document);
    }

    fn beep(&mut self, document: DocumentId, sound: pdf_model::view::Sound) {
        Self::beep(self, document, sound);
    }

    fn retitle(&mut self) {
        self.update.title = true;
    }

    /// Nothing to push: the C++ side reads the bar from [`Host::loading_permille`] whenever it
    /// reads the status line, and a draw that has ended has no share to show (ADR 1825).
    fn show_the_drawing(&mut self) {}

    /// A flag: the window asks `place_window` with its own extents, which Rust does not know, and
    /// obeys `/CenterWindow` there too (ADR 1429).
    fn fit_the_window(&mut self) {
        self.update.placement = true;
    }

    /// Nothing here: `place_window` centres the window at the frame that fits it, where its own
    /// extents are known (ADR 1429).
    fn asked_to_centre(&mut self) {}

    /// The window's chrome and panel are the C++ side's, read from [`Host::chrome`] on the flag,
    /// and full screen is said here, where `viewer-gtk` says it as it shows the mode.
    fn show_the_page_mode(&mut self, _mode: pdf_model::viewer_preferences::PageMode) {
        if self.state.presenting.full_screen() {
            self.say("presenting full screen (§7.7.2's FullScreen page mode) — Escape comes back");
        }
        self.update.window = true;
    }

    fn drain(&mut self, queue: VecDeque<Command>) {
        Self::drain(self, queue);
    }

    fn prepare(&mut self) {
        Self::prepare(self);
    }

    fn show_the_opening(&mut self) {
        Self::show_the_opening(self);
    }

    /// The bar is the C++ side's, drawn from [`Host::loading_permille`], which has nothing to show
    /// once the document is joined; what is this side's is whether it was ever said.
    fn take_down_the_bar(&mut self) -> bool {
        self.bar_said
    }

    fn print(&mut self, pages: usize, fidelity: viewer_core::Fidelity) {
        self.showing_what_would_print(pages, fidelity);
    }

    /// A flag for the C++ side, which owns the clipboard, because Rust never calls a Qt object.
    fn to_clipboard(&mut self, text: String) {
        self.clipboard = text;
        self.update.clipboard = true;
    }

    /// Nothing to arm: the timer [`Host::drawing_wait`] sets looks for a server's answer and a
    /// conversion's report whenever one is out (ADRs 1291, 1954).
    fn look_for_answers(&mut self) {}

    fn ask_for_a_script(&mut self, document: DocumentId, question: viewer_core::ScriptQuestion) {
        Self::ask_for_a_script(self, document, question);
    }

    /// Every panel is rebuilt from the answers they were built from, the files tab's among them.
    fn attachments_changed(&mut self) {
        self.build_panels();
    }
}
