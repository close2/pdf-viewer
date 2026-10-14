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
//! dialogues a person answers, [`restrictions`] the levels menu, [`requests`] what a document asks
//! of this machine, [`saving`] §7.5.6's update and §7.11.4's files, and [`printing`] RFC 0004's
//! print path.

mod drawing;
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
mod requests;
mod restrictions;
mod saving;
mod stand_in;
mod tabs;
#[cfg(test)]
mod tests;
mod window;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use pdf_model::view::WidgetAppearances;
use render_cpu::CpuRasterizer;
use viewer_core::{Command, DocumentId, Event, FormField, Viewer};
use viewer_host::panel::Tab;
use viewer_host::trace::{Topic, Trace};

use crate::bridge::ffi::{QtQuad, QtUpdate};

pub(crate) use notes::{leader_cycles, reached_stop, rule_pieces, rule_thickness};

use forms::Placement;
use panels::Flat;
use printing::{PrintJob, PrintedPage};
use questions::Pending;

/// The identity this host gives the one document it opens.
const DOCUMENT: DocumentId = DocumentId(1);

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

/// The window's standing levels, from what the command line set (ADR 1145).
fn standing(settings: viewer_host::Settings) -> viewer_host::Restrictions {
    viewer_host::Restrictions::new(settings.restrictions)
        .with(viewer_host::ActLevel::EmbeddedDocuments(
            settings.embedded_documents,
        ))
        .with(viewer_host::ActLevel::Submissions(settings.submissions))
        .with(viewer_host::ActLevel::Scripts(settings.scripts))
}

/// What an open sends, in the order it sends it: every policy before the document.
///
/// One statement for the open at launch, on the opening thread, and every open after it, on this
/// one. A policy applied halfway through is not a policy: §6.3.2.2's instruction is a property of
/// *this host* rather than of the document (ADR 0245), and `Restrict` is the reader's answer to
/// what the *file* asserts, which `CLAUDE.md` says is always the reader's to give.
fn opening_commands(
    window: viewer_core::RestrictionPolicy,
    widget_appearances: WidgetAppearances,
    separations: bool,
    bytes: pdf_syntax::FileBytes,
    password: Option<viewer_core::Secret>,
    fragment: Option<String>,
) -> VecDeque<Command> {
    VecDeque::from([
        Command::Restrict(viewer_core::RestrictionScope::Window(window)),
        Command::Delegate(widget_appearances),
        // §10.8.3's simulation on the same argument: it decides what colour every mark of the
        // first interpretation is, so a preference applied after the page had been drawn would
        // have drawn the other picture first (ADR 1228).
        Command::Separations(separations),
        // A page past the whole-page bound is drawn as much as the window shows of it, as `quorra`
        // draws every page, rather than refused at the notch that crosses it (ADR 1892).
        Command::Oversize(viewer_core::Oversize::Listed),
        Command::Open {
            id: DOCUMENT,
            bytes,
            password,
            fragment,
        },
    ])
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

/// Everything this window holds about the document **in front**, and nothing about the others.
///
/// A struct rather than a row of fields on [`Host`] because this window now holds more than one
/// document: `viewer_host::Documents` parks one of these beside every tab that is not in front and
/// swaps the whole of it in one move when the person changes tabs. A field that must not leak
/// between two documents therefore cannot be left behind by forgetting it at a call site — either
/// all of it travels or none of it does (ADR 1264). `viewer-gtk` holds the same six things under
/// the same name, which is what makes the two hosts checkably level.
#[derive(Debug)]
struct Showing {
    /// Where the bytes came from, which is what rule 2 makes a host's business and not the core's.
    path: PathBuf,
    /// The directory §12.7.6.4's policy resolves against.
    directory: Option<PathBuf>,
    /// The file, kept because §7.6.4.1's second attempt opens it again with a password.
    bytes: pdf_syntax::FileBytes,
    /// Annex O's fragment, where the host was given one.
    fragment: Option<String>,
    /// §7.6.4.1's attempts, counted by [`viewer_host::Asking`] so that three hosts count alike.
    asking: viewer_host::Asking,
    /// Whether the open document still owes what it says about *itself*.
    ///
    /// Asked for once the first frame is on the screen rather than as part of opening, because
    /// §12.8's answer digests the signed part of the file and `CLAUDE.md` principle 2 keeps that
    /// off a launch. [`viewer_host::report::Due`] holds the rule for every host. ADR 1044.
    report_due: viewer_host::report::Due,
    /// Whether anything is unsaved.
    dirty: bool,
    /// What the title bar says about the page.
    caption: String,
    /// What the title bar calls the document where §12.2's `/DisplayDocTitle` asks for its own
    /// title and it states one ([`viewer_host::documents::document_title`]); `None` for the file's
    /// name.
    titled: Option<String>,
    /// §12.9's measuring: whether a press on the page is a point, and the points so far.
    ///
    /// `viewer_host::Measuring` is shared with the other two windows — when a press is a point,
    /// and what the answer says — and what is Qt's is the press and the status bar (ADR 1191).
    measuring: viewer_host::Measuring,
    /// Table 29's arrangement, as this window last asked for it — what `l` cycles from.
    layout: pdf_model::viewer_preferences::PageLayout,
    /// What this document departs from the window's restriction levels in.
    ///
    /// The one half of `viewer_host::Restrictions` that belongs to a file rather than to the
    /// window: `viewer_core` already keeps the departures beside the document they are about
    /// (ADR 1145), so the menu's ticks travel with the tab and nothing is sent when one changes.
    departures: viewer_core::RestrictionOverride,
    /// Whether Table 29's `/PageMode` is still to be obeyed, the first time this tab is in front.
    ///
    /// A document opened behind the one showing is not displayed, and `/PageMode` is "how the
    /// document shall be displayed when opened" — so its full screen and its panel wait for the
    /// pump that first brings it to the front (ADR 1303).
    catalog_due: bool,
}

impl Showing {
    /// A document this window has just been handed, before anything has been read from it.
    fn new(path: PathBuf, bytes: pdf_syntax::FileBytes, fragment: Option<String>) -> Self {
        Self {
            directory: path.parent().map(Path::to_path_buf),
            path,
            bytes,
            fragment,
            asking: viewer_host::Asking::new(),
            report_due: viewer_host::report::Due::default(),
            dirty: false,
            caption: String::new(),
            titled: None,
            measuring: viewer_host::Measuring::default(),
            layout: pdf_model::viewer_preferences::PageLayout::SinglePage,
            departures: viewer_core::RestrictionOverride::NONE,
            catalog_due: false,
        }
    }
}

/// One document, one viewer, and the loop between them.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about one window, each read in one place and none a state"
)]
pub struct Host {
    /// The state machine every host on this boundary drives.
    pub(crate) viewer: Viewer,
    /// Everything this window holds about the document in front.
    showing: Showing,
    /// The documents this window has open, in the order the strip shows them.
    ///
    /// One of these until something opens a second — a `/NewWindow true` on a remote go-to — and
    /// the `QTabBar` is hidden until then, so a window showing one document looks exactly as it
    /// did (ADR 1264).
    documents: viewer_host::Documents<Showing>,
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
    /// The documents a reader named that are still to open beside this one — the command line's
    /// later paths and the files chosen with Ctrl + O — one at a time (`viewer_host::Arrivals`).
    arrivals: viewer_host::Arrivals,
    /// Which document the password prompt now on its way to the screen is about.
    ///
    /// The prompt is one `QDialog` and the answer comes back as a bare string, so this is how the
    /// answer finds a document that is not yet in the strip.
    prompting: DocumentId,
    /// Whether the next of [`Host::arrivals`] may start, which C++ answers with [`Host::arrive`].
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
    /// write is drawn on [`Host::drawing`]'s thread, so that a page written to be expensive cannot
    /// take `QApplication::exec` with it. What is left on this thread is bounded by *this host's*
    /// own asking rather than by what a document chose to state.
    rasterizer: CpuRasterizer,
    /// The page rasteriser, on a thread of its own.
    ///
    /// `viewer_host::Drawing` is the whole arrangement, shared with `viewer-gtk`: what is Qt's is
    /// the `QTimer` that asks it whether anything has landed, and the interval for that is
    /// [`Host::drawing_wait`] — pulled rather than pushed, for this crate's own rule.
    drawing: viewer_host::Drawing,
    /// A small picture of each page drawn lately, shown where a page arrives on the screen with
    /// no frame until its own lands — `viewer_host::stand_in::Retained`, shared with `viewer-gtk`
    /// (ADR 1866).
    retained: viewer_host::stand_in::Retained,
    /// The pages past the whole-page bound, drawn as the window shows them (ADR 1892) — shared
    /// with `viewer-gtk`.
    oversized: viewer_host::oversize::Oversized,
    /// Which page this window has told the person is taking a long time to draw.
    ///
    /// **The state Escape's third row depends on** (`viewer_host::Waiting`), held rather than
    /// asked for twice because it decides two things: what the status bar is saying, and what the
    /// key means while it says it. The same field `viewer-gtk` holds, for the same reason and off
    /// the same poll — `viewer_host::Drawing::overlong` is the question and the wait is
    /// `viewer-host`'s.
    warned: Option<usize>,
    /// The launch timeline.
    pub(crate) trace: Trace,
    /// §6.3.2.2: who draws §12.7's widgets — this host's controls, or the document's own pictures.
    widget_appearances: WidgetAppearances,
    /// How much of what the document asserts over its reader this window obeys.
    ///
    /// The other policy value beside the one above, and the same kind of thing: a fact about what
    /// *this reader* has been asked to do rather than about the file.
    /// One level per operation, every one of them [`viewer_core::RestrictionLevel::Off`] unless
    /// [`viewer_host::RESTRICTIONS`] or [`viewer_host::IGNORE_RESTRICTIONS`] was on the command
    /// line — the first of which this program did not have, and the second of which it refused as
    /// an unknown option while telling every person who hit a refusal to use it (ADR 0604, ADR
    /// 1144).
    ///
    /// **Two scopes**: the window's levels, and what the open document departs from them in. A
    /// level set to catch one suspicious file would otherwise catch every file this window opens
    /// afterwards (ADR 1145).
    restrictions: viewer_host::Restrictions,
    /// §12.6.4.8: what this window does when a link asks for a URI, per `--links=`.
    ///
    /// The other direction from the entry above, and that is why it is a second value rather than
    /// a seventh operation of the first: what a document *asserts over its reader* is one subject,
    /// and what a document asks **this machine to start** is the other. `viewer_host::Links` holds
    /// `CLAUDE.md`'s four levels over the second (ADR 1155).
    links: viewer_host::Links,
    /// §12.6.4.3: what this window does when an action names another file, per
    /// `--remote-documents=`.
    ///
    /// A third value for a third subject, on the entry above's argument: starting another program
    /// on a URL and opening a PDF beside this one in this reader are two decisions, and one word
    /// for both would make each of them mean the other (ADR 1227).
    remote_documents: viewer_host::RemoteDocuments,
    /// §12.7.6.2's submissions on the wire, each on a `submit-form` thread of its own, and the
    /// answers that have come back — looked at on the drawing timer while one is out (ADR 1291).
    submitter: viewer_host::submit::Submitter,
    /// §10.8.3: whether this window has asked for the separation simulation.
    ///
    /// Held here rather than asked of the core, because it is a preference a person toggles and
    /// `viewer-core` holds none: this is the window's record of the answer it last sent, so the
    /// toggle knows which way to go. `--separations=` sets what it starts at (ADR 1228).
    separations: bool,
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
    question: Option<(Pending, String)>,
    /// RFC 0008 section 4.2's `app.alert` or `app.response` a script is waiting on: which
    /// document's, what that document is called, and what it asks — taken once, by the answer,
    /// so that a dialogue closed twice answers once (ADR 1628).
    script_question: Option<(DocumentId, String, viewer_core::ScriptQuestion)>,
    /// What to put above the entry, worded when the prompt is asked for and read once by C++.
    prompt: String,
    /// Whether the document has been opened yet, which waits for the first resize.
    opened: bool,
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
    /// Table 29's `/PageMode /FullScreen`, §12.2's chrome flags, and the way back out.
    ///
    /// **The window §12.4.4's presentation had never had** (ADR 0470). Which sentence this window
    /// is obeying is `viewer_host::Presenting`, shared with the other two hosts; what is Qt's is
    /// `QWidget::showFullScreen` and which widget each of Table 147's three flags names — and
    /// that half is in `cpp/window.cpp`, because the widgets are.
    presenting: viewer_host::Presenting,
    /// Whether the first frame has been reported, so that the launch line is printed once.
    presented: bool,
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
    /// What the find bar is looking for, kept because the page's highlights are asked for on
    /// every repaint while the document-wide search is a plan inside `viewer-core`.
    needle: String,
    /// How many pages a search still has to read. Zero when nothing is being searched for.
    pages_left: usize,
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
    /// §12.4.4.1's clock, while a presentation is running.
    ///
    /// **`None` is a window whose `QTimer` is stopped**, rather than one that fires to discover
    /// it has nothing to advance. The decision inside it — how often, what a tick carries, when
    /// Table 164's `/D` has run out — is `viewer_host::Clock`, shared with the other two hosts.
    clock: Option<viewer_host::Clock>,
    /// When this window last told the core that time passed for a document's script timers, which
    /// the same `QTimer` wakes for while no presentation does (ADR 1702).
    script_ticker: viewer_host::script_timers::Ticker,
    /// A transition named by the core, waiting for the page it moves *to* to be rendered.
    arming: Option<pdf_model::navigation::Transition>,
    /// The page on the screen, as the list that drew it, the target the core asked for and the
    /// whole pixel the window shows it at — the face a transition would move *from*
    /// (`viewer_host::clock::face_as_shown`). Kept only while presenting.
    shown: Option<(
        std::sync::Arc<pdf_render::DisplayList>,
        pdf_render::TargetSpec,
        (f32, f32),
    )>,
    /// The frame of a transition in flight, which is what this window shows instead of a page.
    ///
    /// A whole `Raster` rather than a display list, because what crosses this bridge is pixels:
    /// `frame_pixels` hands the C++ side a borrowed slice and a transition frame has to be
    /// somewhere for that borrow to point at.
    playing: Option<pdf_render::Raster>,
    /// The viewport in device pixels, which is the rectangle a transition's frames are drawn in.
    pub(crate) viewport: (u32, u32),
    /// The document opening on its own thread, until the first resize joins it (ADR 1539) — or,
    /// where it is not ready inside `viewer_host::drawing::WARN`, until the drawing timer's look
    /// that finds it ready (ADR 1811).
    anticipated: Option<viewer_host::Loading<(Viewer, Vec<Event>)>>,
    /// The latest viewport the window was given while the document was still opening, which the
    /// join tells the core rather than the first one.
    awaiting: Option<Command>,
    /// Page one being drawn while it is read on the opening thread, until the join hands it to
    /// [`Self::drawing`] (ADRs 1855, 1867).
    ahead: Option<viewer_host::Ahead>,
    /// The window as the core was last told it, which a page turn's drawing ahead draws into
    /// (ADR 1871); `None` before the first resize.
    extent: Option<viewer_host::loading::Window>,
    /// The last look at page one as it was read, shown until the page has a frame of its own.
    read_look: Option<viewer_host::loading::Look>,
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
            .field("path", &self.showing.path)
            .field("opened", &self.opened)
            .field("dirty", &self.showing.dirty)
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
    /// rasterised at, and a page drawn at a guessed size would be drawn twice. That is
    /// `viewer-gtk`'s shape too, so the two hosts' launch timelines measure the same thing.
    ///
    /// `reader` is the reader's three policy words, which become their commands on that thread
    /// and before the document, for `viewer-gtk`'s reasons (ADR 1580).
    ///
    /// # Errors
    ///
    /// [`HostError::Unreadable`] where the file named cannot be read.
    #[expect(
        clippy::too_many_lines,
        reason = "most of it is the host's initial state, one line a field; splitting the literal \
                  would put half of what a fresh window holds in another function"
    )]
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
        let commands = opening_commands(
            standing(settings).window(),
            widget_appearances,
            settings.separations,
            bytes.clone(),
            None,
            fragment.clone(),
        );
        let ahead = viewer_host::Ahead::page_sized();
        let drawing_ahead = ahead.clone();
        let anticipated = viewer_host::Loading::start(move || {
            // No viewport: the window does not exist yet, and the core renders nothing into one
            // with no extent. The core is made here and moved back, so it is still
            // single-threaded — `viewer-core`'s rule 4 as `quorra` keeps it (ADR 0182).
            let mut viewer = Viewer::new(0, 0, 1.0);
            let mut events = Vec::new();
            let supply = reader.commands();
            for command in supply.commands {
                events.extend(viewer.handle(command));
            }
            // Said where the window says everything else, and before the document so that a
            // person reading the verdicts that follow knows which files did not take (trap 5).
            for refusal in supply
                .refused
                .into_iter()
                .chain(viewer.reference_refusals().iter().map(ToString::to_string))
            {
                println!("note: {refusal}");
            }
            for command in commands {
                events.extend(viewer.handle(command));
            }
            // Page one read here as `Viewer::anticipate` reads it, and drawn beside the read on a
            // thread of its own once the read has run `GLIMPSE_AFTER` (ADR 1855): the read keeps
            // the thread the launch placed it on (trap 154).
            events.extend(drawing_ahead.anticipate(&mut viewer));
            (viewer, events)
        });
        Ok(Self {
            viewer: Viewer::new(1, 1, 1.0),
            showing: Showing::new(path.to_owned(), bytes, fragment),
            // One document, and the strip is hidden for it — the launch path opens exactly the
            // one document it always has, under exactly the name it always had (ADR 1264).
            documents: viewer_host::Documents::new(DOCUMENT, viewer_host::documents::label(path)),
            // No thread and nothing read until a label holds a character the compiled-in face
            // lacks, and then off this thread (ADR 1406).
            faces: viewer_host::machine_faces::MachineFaces::new(),
            faces_taken: 0,
            arrivals: viewer_host::Arrivals::new(),
            prompting: DOCUMENT,
            arrival_due: false,
            panels_due: false,
            documents_moved: false,
            closing: false,
            rasterizer: CpuRasterizer::new(),
            // No thread yet, and none until a page needs one: `CLAUDE.md` section 2's rule that
            // nothing page one does not need happens before page one.
            drawing: viewer_host::Drawing::new(),
            retained: viewer_host::stand_in::Retained::default(),
            oversized: viewer_host::oversize::Oversized::default(),
            warned: None,
            trace,
            widget_appearances,
            restrictions: standing(settings),
            links: settings.links,
            remote_documents: settings.remote_documents,
            submitter: viewer_host::submit::Submitter::new(),
            separations: settings.separations,
            scale: 1.0,
            // The panel is what this window opens with, and `o` is what takes it away.
            panel_shown: true,
            printing: false,
            print: None,
            printed: None,
            question: None,
            script_question: None,
            prompt: String::new(),
            opened: false,
            message: String::new(),
            trees: std::array::from_fn(|_| Vec::new()),
            fields: Vec::new(),
            placed: Vec::new(),
            popups_shown: Vec::new(),
            over_link: false,
            keys_told: pdf_model::view::Keys::default(),
            update: nothing_changed(),
            presented: false,
            accessibility: None,
            spoken: None,
            window_at: None,
            window_active: false,
            aimed: None,
            page_at: (0.0, 0.0),
            access_pending: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            needle: String::new(),
            pages_left: 0,
            zoom_wheel: viewer_host::ZoomWheel::default(),
            fit_magnification: None,
            clock: None,
            script_ticker: viewer_host::script_timers::Ticker::default(),
            arming: None,
            shown: None,
            playing: None,
            viewport: (1, 1),
            anticipated: Some(anticipated),
            awaiting: None,
            ahead: Some(ahead),
            extent: None,
            read_look: None,
            bar_said: false,
            preparing: None,
            // Table 147's and Table 29's own defaults, replaced by what the catalog states the
            // moment the document opens.
            presenting: viewer_host::Presenting::default(),
            clipboard: String::new(),
        })
    }
}
