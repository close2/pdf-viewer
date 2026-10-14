//! The window, and everything that crosses [`viewer_core`]'s boundary to fill it.
//!
//! One `Viewer`, one `render-cpu` rasteriser, and GTK4 widgets on the other side. The loop is the
//! one `viewer-ui` and `tests/headless.rs` already run — a command in, the events it produced out,
//! and the ones asking for work answered until nothing is left — because the vocabulary is what is
//! being tested and a host that invented its own scheduling would be testing that instead.
//!
//! # Where the pixels go
//!
//! A [`gtk4::Overlay`] whose main child is a [`gtk4::Fixed`]. The page is a [`gtk4::Picture`]
//! inside the `Fixed`, moved to the origin [`viewer_core::Query::Frame`] reports; §12.7's
//! controls are `Fixed` children at their widgets' own rectangles. On top of both is a
//! [`gtk4::DrawingArea`] that cannot be targeted by input and draws nothing but chrome —
//! the selection, the focus ring — in the platform's own colour.
//!
//! That third layer is also where the viewport's *size* comes from: `GtkDrawingArea::resize` is
//! the only size signal GTK4 gives application code without subclassing a widget, and subclassing
//! is a larger thing to take on than one signal is worth.
//!
//! **`#![forbid(unsafe_code)]` is not what rules subclassing out** — checked by writing the
//! subclass rather than by reading the attribute (ADR 0623). `#[glib::object_subclass]` expands to
//! `unsafe impl` and `unsafe` blocks, but the `unsafe_code` lint does not fire on a proc-macro's
//! expansion, so a `GObject` implementing `gtk4::Accessible` compiles in this crate today with the
//! `forbid` untouched. The reason not to subclass is a judgement about cost, which is a different
//! kind of claim from a compiler-enforced impossibility — and stating the second where only the
//! first is true is how a floor gets written down that nobody re-checks (trap 17, and ADR 0508's
//! rule paying a third time).
//!
//! # The modules
//!
//! This file is the [`Host`] and the state it keeps between two callbacks; what it does is in its
//! children, each named by what it is (ADR 1906). The launch is `opening`, the loop `react`, the
//! drawing thread's side `drawing`, and the pictures a frame puts up `stand_in`; §12.4.4's clock
//! is `presentation`; the window's widgets, title and chrome are `window`, and what a person does
//! to it is `input`. The strip of documents is `tabs`, the six panels `panels`, the find bar
//! `find`; the controls over the page are `forms`, the popups `notes`, and a note's rich text
//! `rich_text`. What the window asks a person is `questions`, the levels menu `restrictions`, what
//! a document sends or fetches `requests`, and what is written out `saving` and `printing`.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

use gtk4::glib;
use pdf_model::view::WidgetAppearances;
use render_cpu::CpuRasterizer;
use viewer_core::{Command, DocumentId, Event, Viewer};

use crate::controls::Placed;
use viewer_host::panel::{self, Miniatures};
use viewer_host::trace::{Topic, Trace};

mod drawing;
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
mod rich_text;
mod saving;
mod stand_in;
mod tabs;
#[cfg(test)]
mod tests;
mod window;

use notes::{NoteEditor, PlacedPopup};
use window::build_window;

/// The identity this host gives the one document it opens.
const DOCUMENT: DocumentId = DocumentId(1);

/// What a window does with the word a person pressed on one of `Host::put_a_question`'s two
/// buttons.
///
/// Boxed and reference-counted because the dialogue outlives the call that built it: GTK holds the
/// closure in two places — each button, and the handler for a window closed without pressing
/// either — and neither may capture the `Host`, so what each captures is this and the weak handle
/// beside it.
type Answered = Rc<dyn Fn(&mut Host, bool)>;

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

/// The document named on the command line, opening — and its first page interpreted — on a thread
/// of its own while GTK comes up (ADR 1539).
///
/// **The launch path's overlap, in this window's shape.** What GTK needs before a first allocation
/// — the display, the theme, the window, its realisation — is 90 to 140 ms under Xvfb, and none of
/// it needs the document; the open and page one's interpretation need none of it. So the open is
/// started before `Application::run`, on a `Viewer` with no viewport, through the same commands
/// [`Host::open_document`] sends, and [`viewer_core::Viewer::anticipate`] interprets the page it
/// stands at (ADR 1531). The first allocation joins it and its `Resize` goes straight to the
/// render. The core is still single-threaded: it is made on that thread and moved back, which is
/// `viewer-core`'s rule 4 as `quorra` keeps it.
#[derive(Debug)]
pub struct Opening {
    /// What was named.
    path: PathBuf,
    /// The open file, held here too for a password's second open.
    bytes: pdf_syntax::FileBytes,
    /// Annex O's fragment, as the command line carried it.
    fragment: Option<String>,
    /// The thread, until the first allocation joins it — a [`viewer_host::Loading`], so that page
    /// one publishes how far it has got (ADR 1811).
    thread: viewer_host::Loading<(Viewer, Vec<Event>)>,
    /// Page one drawn while it is read on that thread (ADRs 1855, 1867): told the window's size on
    /// the first allocation, looked at while the bar is up, and handed to the drawing at the join.
    ahead: viewer_host::Ahead,
}

impl Opening {
    /// Opens the file and starts the thread that opens the document in it.
    ///
    /// `reader` is the reader's three policy words, which become their commands on that thread:
    /// reading an anchor or a target directory is not needed to show page one (`CLAUDE.md`
    /// principle 2), and the commands go before the document because a policy applied halfway
    /// through is not a policy (ADR 1580).
    ///
    /// # Errors
    ///
    /// [`HostError::Unreadable`] where the file named cannot be read.
    pub fn start(
        path: &Path,
        fragment: Option<String>,
        widget_appearances: WidgetAppearances,
        settings: viewer_host::Settings,
        reader: viewer_host::ReaderWords,
    ) -> Result<Self, HostError> {
        // Open on disk rather than read whole: the core reads what page one needs through the
        // handle, and the file's size stops being the launch's cost (ADR 0809).
        let bytes =
            pdf_syntax::FileBytes::on_disk(path).map_err(|error| HostError::Unreadable {
                path: path.to_owned(),
                error: error.to_string(),
            })?;
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
        let thread = viewer_host::Loading::start(move || {
            // No viewport: the window does not exist yet, and the core renders nothing into one
            // with no extent.
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
            path: path.to_owned(),
            bytes,
            fragment,
            thread,
            ahead,
        })
    }
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
/// One statement for the open at launch, on [`Opening`]'s thread, and every open after it, on
/// this one. A policy applied halfway through is not a policy: §6.3.2.2's instruction is a
/// property of *this host* rather than of the document, which is why it is not part of `Open` — a
/// host that changes its mind sends it again and the page is rebuilt — and `Restrict` is the
/// reader's answer to what the *file* asserts, which `CLAUDE.md` says is always the reader's to
/// give.
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

/// What the chrome layer draws, kept apart from the [`Host`] on purpose.
///
/// GTK draws from its own main loop, so a draw function that reached into the host would borrow
/// it at a moment nothing else chose. This is the one piece of state both sides touch, it is
/// written only by [`Host::refresh`] and read only by the draw function, and keeping it separate
/// is what makes that provable rather than argued.
#[derive(Debug, Default)]
struct Chrome {
    /// §12.5.6.10's target: what is selected, in device pixels of the viewport.
    selection: Vec<[f32; 8]>,
    /// Every occurrence of the find bar's string on this page, in the same pixels.
    ///
    /// Beside the selection rather than folded into it, because the two say different things and
    /// a person needs both at once: these are *where else* the word is and the selection is
    /// *which one you are on*. Drawn first and fainter, so the current occurrence reads as the
    /// one the platform's own selection colour is over.
    matches: Vec<[f32; 8]>,
    /// Annex O's `highlight`: the rectangles the URI's fragment named, in the same pixels.
    ///
    /// Table Annex O.4 leaves what one looks like to a processor — "[t]he nature of the
    /// highlighting is implementation-dependent" — and this platform's honest answer is the same
    /// one it gives a match: the theme's own colour at an alpha of its own, because GTK exposes no
    /// second colour to ask for.
    highlights: Vec<[f32; 8]>,
    /// §12.5.1's ring, in the same pixels.
    focus: Option<[f32; 8]>,
    /// §12.9's traced path — the points a person has put down while measuring — in the same
    /// pixels.
    ///
    /// **The rubber band, which is chrome and not a message.** The clause states no state for a
    /// viewer to be in, so the points are the host's (`viewer_host::Measuring`) and what they look
    /// like is this platform's; `viewer_core::Query::Measure` answers what they *mean* and never
    /// sees a pixel of this. Drawn over everything else, because it is the one shape a person is
    /// making rather than one the document or a search put there (ADR 1264).
    measuring: Vec<[f32; 2]>,
    /// Device pixels per logical pixel, because GTK draws in logical ones.
    scale: f64,
}

/// What one of [`Tab`](viewer_host::panel::Tab)'s panels turned out to hold.
///
/// Two shapes rather than one, because §12.3.4's is genuinely not a tree: five of the six panels
/// are a [`panel::PanelRow`] apiece and the sixth is a page count whose pictures are fetched a row
/// at a time. Collapsing them would mean either a row type carrying a picture nobody else has or a
/// list of miniatures decoded before anybody looked at one, and `CLAUDE.md` section 2 forbids the second.
#[derive(Debug)]
enum Panel {
    /// A tree of rows.
    Rows(Vec<panel::PanelRow>),
    /// §12.3.4: how many pages there are. The miniatures come from [`Host::page_sink`].
    Pages(usize),
}

/// The widgets, held so that the loop can put things in them.
#[derive(Debug)]
struct Ui {
    /// The toplevel.
    window: gtk4::ApplicationWindow,
    /// The page and the form controls.
    fixed: gtk4::Fixed,
    /// §12.5.6.14's popup windows, in a layer of their own over the page.
    ///
    /// Not in [`Ui::fixed`], and the reason is in `build_window`: a `GtkFixed` measures its
    /// children and a window the document put beside the page would then decide how wide the
    /// window is.
    popups: gtk4::Fixed,
    /// The pixels of every page Table 29's arrangement is showing, one widget apiece.
    ///
    /// Grown when a layout puts more pages on the screen and never shrunk — a `GtkPicture` with
    /// no paintable and `set_visible(false)` costs a hidden widget, and destroying and rebuilding
    /// them on every scroll would churn the widget tree at scroll speed.
    pictures: Vec<gtk4::Picture>,
    /// The layer that measures the viewport and draws the interactive chrome.
    chrome: gtk4::DrawingArea,
    /// Where each of [`viewer_host::Tab`]'s panels goes, in [`viewer_host::Tab::ALL`]'s order.
    ///
    /// A slot apiece rather than six named fields, because the list of panels is
    /// `viewer_host::Tab` and a second list here would be a second thing to keep level with the
    /// other two hosts. [`Tab::index`](viewer_host::panel::Tab::index) is what addresses one.
    slots: Vec<gtk4::Box>,
    /// The panels' notebook — Table 29's "any other window", and what the page mode opens.
    tabs: gtk4::Notebook,
    /// The strip of open documents, one notebook page apiece.
    ///
    /// A real [`gtk4::Notebook`] rather than a row of buttons, because a notebook is what this
    /// platform's tab strip *is* — keyboard traversal, the overflow arrows, the theme's own
    /// spacing — and the one thing it wants that this window cannot give it is a separate widget
    /// tree per page. There is one view here and it holds whichever document is in front, so
    /// every page but the current one is an empty [`gtk4::Box`] and [`Ui::split`] is moved into
    /// the page that becomes current. Reparenting on a tab change costs one allocation and
    /// happens when a person clicks; keeping a panel tree, a page picture and a set of controls
    /// per document would cost them all the time (ADR 1264).
    ///
    /// Its own tabs are hidden while one document is open, so a window that opened one file looks
    /// exactly as it did before this existed.
    documents: gtk4::Notebook,
    /// The splitter it sits in, because full screen takes the notebook *out* rather than hiding
    /// it: a `GtkPaned` keeps a position that was set, and a hidden child would leave the space.
    split: gtk4::Paned,
    /// The page's own half of that splitter, held across the times it is not in it.
    ///
    /// Table 158's `/Direction` `N` takes it out — "[t]he entire window region shall be dedicated
    /// to the file navigation view" — the same way full screen takes the notebook out, and for
    /// the same reason: a `GtkPaned` keeps a position that was set (ADR 1252).
    page_area: gtk4::Overlay,
    /// The header bar's own buttons, which are this host's tool bar: §12.2's `/HideToolbar`.
    ///
    /// The *buttons* rather than the bar, because GTK4 puts them in the same widget as the window
    /// controls and a document asking for no tool bar has not asked for no close button. Full
    /// screen takes the whole titlebar, which is Table 29's separate sentence.
    tool_buttons: Vec<gtk4::Button>,
    /// The restrictions menu, which is §12.2's `/HideMenubar` finally naming something here.
    ///
    /// Separate from [`Ui::tool_buttons`] because a `GtkMenuButton` is not a `GtkButton` and
    /// because the two flags are different sentences: a document asking for no tool bar has not
    /// asked for the reader's own policy to be unreachable. Its contents are built when it is
    /// opened and never before — `CLAUDE.md` section 2's rule, and the levels it ticks are a
    /// function of a policy that can change between one opening and the next.
    menu: gtk4::MenuButton,
    /// The separator and the status label, which are §12.2's `/HideWindowUI`.
    status_bar: gtk4::Box,
    /// Trap 5's channel reaching a person: what the page could not draw, and what was refused.
    status: gtk4::Label,
    /// How far page one has got while the document is still opening past `WARN`, and hidden
    /// otherwise (ADR 1811).
    loading: gtk4::ProgressBar,
    /// The find bar, which is a real [`gtk4::SearchBar`] and not a rectangle this host draws.
    find: gtk4::SearchBar,
    /// The string in it.
    find_entry: gtk4::SearchEntry,
}

/// Everything this window holds about the document **in front**, and nothing about the others.
///
/// A struct rather than a row of fields on [`Host`] because this window now holds more than one
/// document: `viewer_host::Documents` parks one of these beside every tab that is not in front and
/// swaps the whole of it in one move when the person changes tabs. A field that must not leak
/// between two documents therefore cannot be left behind by forgetting it at a call site — either
/// all of it travels or none of it does, and the compiler names a field added here and forgotten
/// there (ADR 1264).
///
/// What is **not** here is as deliberate: the window's restriction levels, `--links=`,
/// `--remote-documents=`, §10.8.3's simulation, §6.3.2.2's delegation, Table 29's full screen and
/// this window's wish for a panel are all facts about the *window* and apply to every document it
/// shows, which is `viewer_core::Command::Restrict`'s own rule for a host-supplied value. Nor are
/// the controls, the popup windows, the placed pictures or the magnification every control fits
/// at: each is rebuilt from a query on the repaint that follows a switch, so parking one would be
/// keeping a second answer to a question the core answers.
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
    /// What is shared with the other two windows is `viewer_host::Measuring` — when a press is a
    /// point, and what the answer says — and what is GTK's is the press and the status label
    /// (ADR 1191).
    measuring: viewer_host::Measuring,
    /// Table 29's arrangement, as this window last asked for it.
    ///
    /// Kept because `l` *cycles*: the value in force is the viewer's, and a host that wanted to
    /// know it without remembering would have to ask a question this vocabulary does not have —
    /// which is the right answer, because `Query::Opening` says what the *document* asked for and
    /// that is a different sentence.
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
            directory: path.parent().map(Path::to_owned),
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

/// One document, one window, and the loop between them.
///
/// `struct_excessive_bools` is asking for a state machine, and there is no state here to make one
/// of: each flag is an independent fact about a *different* clause's window — whether the document
/// is open, whether it is unsaved, whether the first frame has been reported, whether the reader
/// asked for Table 29's panel. Packing four unrelated sentences into one enumeration would make a
/// state out of their product, which is fifteen states nobody wrote down. The same argument
/// `viewer_host::Chrome` is written under.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about one window, each read in one place and none a state"
)]
pub struct Host {
    /// The state machine every host on this boundary drives.
    pub(crate) viewer: Viewer,
    /// Everything this window holds about the document in front.
    showing: Showing,
    /// Which page this window has told the person is taking a long time to draw.
    ///
    /// **The state Escape's third row depends on** (`viewer_host::Waiting`), held rather than
    /// asked for twice because it decides two different things: what the status bar is saying, and
    /// what the key means while it says it. `viewer_host::Drawing::overlong` is where it comes
    /// from and the drawing poll is the only thing that reads it, so a window at rest never asks.
    warned: Option<usize>,
    /// What the find bar is looking for, kept because the *page's* highlights are asked for on
    /// every repaint while the document-wide search is a plan inside `viewer-core`.
    needle: String,
    /// How many pages a search still has to read, which is what puts a `Find::Continue` on the
    /// idle queue after each repaint. Zero when nothing is being searched for.
    ///
    /// A count rather than a flag because it is also what the status line says: a person watching
    /// a thousand-page document wants to see it come down.
    pages_left: usize,
    /// Where the pointer last was over the page, in logical pixels of the page area.
    ///
    /// GTK puts no position in a scroll event, so the point a Control + wheel zoom keeps still is
    /// the last one the motion controller reported; `None` before the pointer has entered.
    pointer_at: Option<(f64, f64)>,
    /// The travel a Control + wheel gesture has made and not yet spent on a zoom step.
    zoom_wheel: viewer_host::ZoomWheel,
    /// The documents this window has open, in the order the strip shows them.
    ///
    /// One of these until something opens a second — a `/NewWindow true` on a remote go-to, or a
    /// second file named on the command line — and the notebook hides its own tabs until then, so
    /// a window showing one document looks exactly as it did (ADR 1264).
    documents: viewer_host::Documents<Showing>,
    /// The documents a reader named that are still to open beside this one — the command line's
    /// later paths and the files chosen with Ctrl + O — one at a time (`viewer_host::Arrivals`).
    arrivals: viewer_host::Arrivals,
    /// Whether the panels are to be built again when the pump now running has finished.
    ///
    /// A tab brought to the front sends `Command::Focus` on the pump's queue, and every panel is an
    /// answer about the *focused* document — so building them before that command has run would
    /// fill the new tab's panels from the document that was in front.
    panels_due: bool,
    /// Tier 1's worker for the pictures this window makes *for itself* — §12.3.4's miniatures and
    /// §12.4.4.1's two transition faces.
    ///
    /// **The page is no longer drawn here**, which is ADR 0668: a display list this program did not
    /// write is drawn on [`Host::drawing`]'s thread, so that a page written to be expensive cannot
    /// take the window with it. What is left on this thread is bounded by *this host's* own asking
    /// — a miniature is a `/Thumb` at panel size and a face is one whole-viewport draw per page
    /// turn while a presentation is running — rather than by what a document chose to state.
    rasterizer: CpuRasterizer,
    /// The page rasteriser, on a thread of its own.
    ///
    /// `viewer_host::Drawing` is the whole arrangement, shared with `viewer-qt`: what is GTK's is
    /// the one-shot below that asks it whether anything has landed.
    drawing: viewer_host::Drawing,
    /// A small picture of each page drawn lately, shown where a page arrives on the screen with
    /// no frame until its own lands — `viewer_host::stand_in::Retained`, shared with `viewer-qt`
    /// (ADR 1866).
    retained: viewer_host::stand_in::Retained,
    /// The pages past the whole-page bound, drawn as the window shows them (ADR 1892) — shared
    /// with `viewer-qt`.
    oversized: viewer_host::oversize::Oversized,
    /// The one-shot waiting to ask [`Host::drawing`] whether a page has finished.
    ///
    /// **`None` is a window that wakes for this exactly never**, which is the same shape
    /// [`Host::armed`] has one clause over: `pump_drawing` re-arms only while `Drawing::interval`
    /// answers `Some`, and it answers `None` the moment the thread is idle.
    drawing_armed: Option<glib::SourceId>,
    /// The dialogue a script's question stands on, while it is up: which document asked, the
    /// window, and whether it has been answered — what [`Event::ScriptQuestionWithdrawn`] takes
    /// down, marked answered first so that closing it does not answer the script twice (ADR 1643).
    script_dialogue: Option<(DocumentId, gtk4::Window, Rc<Cell<bool>>)>,
    /// The launch timeline.
    pub(crate) trace: Trace,
    /// The widgets.
    ui: Ui,
    /// What the chrome layer draws.
    chrome: Rc<RefCell<Chrome>>,
    /// Set while this host is writing values into its own controls, so that the write is not
    /// mistaken for a person typing.
    suppress: Rc<Cell<bool>>,
    /// This host, so that a dialogue's callback can reach it after the borrow that opened it ends.
    me: Weak<RefCell<Self>>,
    /// Device pixels per logical pixel, from the display GTK put the window on.
    scale: i32,
    /// §12.3.4's miniatures, decoded when a row is drawn and bounded by [`viewer_host::Miniatures`].
    ///
    /// Beside the host's own fields rather than among them, because the closure a `GtkListView`
    /// binds a row with reaches it while the host may itself be borrowed — the same reason
    /// [`Host::chrome`] is an [`Rc<RefCell<Chrome>>`].
    miniatures: Rc<RefCell<Miniatures<gtk4::gdk::MemoryTexture>>>,
    /// §12.3.6's preview pictures, decoded when a collection row is bound and bounded by
    /// [`viewer_host::panel::Previews`].
    ///
    /// Empty for every document that states no `/Collection`, and for every collection presented
    /// as a tree, a details view or a tile view: only Table 160's `FilmStrip`, `FreeForm` and
    /// `Linear` are made of pictures of the attachments, and a preview is an embedded document
    /// opened (ADR 1251).
    previews: Rc<RefCell<panel::Previews<gtk4::gdk::MemoryTexture>>>,
    /// Whether the document has been opened yet, which waits for the first allocation.
    opened: bool,
    /// The controls over the page, and which fields they are for.
    placed: Vec<Placed>,
    /// §12.5.6.14's open popup windows, as the widgets placed for them.
    popups: Vec<PlacedPopup>,
    /// The answer those widgets were built from, so that a repaint that changes nothing rebuilds
    /// nothing. `viewer_core::PopupWindow` is `PartialEq`, which is what makes the comparison the
    /// whole test.
    popups_shown: Vec<viewer_core::PopupWindow>,
    /// The note whose window a person is typing into, and the editor standing in that window's
    /// place (ADR 1726).
    note_editor: Option<NoteEditor>,
    /// The note whose window the press now held went down on, which its release does not reach
    /// the page for either.
    pressed_a_note: Option<pdf_syntax::ObjectId>,
    /// The keys the core was last told of (ADR 1771).
    keys_told: pdf_model::view::Keys,
    /// Whether the press now held went down on one of §12.7's controls placed over the page, so
    /// that the core is told of its release however far the pointer has moved (ADR 1752).
    pressed_a_control: bool,
    /// Whether the page's drag began on such a control, and is therefore that control's press.
    dragging_a_control: bool,
    /// Whether the pointer was last over §12.5.6.5's activation region.
    ///
    /// Kept so that `Query::LinkAt` changes the cursor when the answer changes rather than on
    /// every motion event: `gdk_surface_set_cursor` is a round trip to the display server and a
    /// pointer moved across a page produces hundreds of them a second.
    over_link: bool,
    /// Whether the first frame has been reported, so that the launch line is printed once.
    presented: bool,
    /// §14.7's tree on AT-SPI, brought up after the first frame and never before it.
    ///
    /// **`None` until [`Host::attend`] has run once**, which is `CLAUDE.md`'s startup rule: the
    /// bridge spawns a thread that connects to the session bus, and page one may not wait behind
    /// a D-Bus round trip for a screen reader that is probably not there (ADR 0623).
    pub(crate) accessibility: Option<viewer_accessibility::Bridge>,
    /// The page and viewport last published to it, and where the viewport sat in the window, so
    /// that a tree is not rebuilt per frame.
    pub(crate) spoken: Option<(viewer_accessibility::Showing, (f32, f32))>,
    /// Set from `accesskit_unix`'s own thread when a client asks for something.
    ///
    /// The only value in this host that crosses a thread boundary, and it carries no payload on
    /// purpose: the request itself is read back on the main thread through
    /// `Bridge::requested`, against the tree the client actually walked.
    pub(crate) access_pending: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// How often the drain below is waking, so that the source is re-armed only when it changes.
    pub(crate) access_interval: Option<i32>,
    /// The source draining that flag, at the interval the bridge asks for.
    pub(crate) access_draining: Option<glib::SourceId>,
    /// §6.3.2.2: who draws §12.7's widgets — this host's controls, or the document's own pictures.
    ///
    /// [`WidgetAppearances::Delegated`] unless the command line asked otherwise, because this host
    /// *does* place a real control over every widget and leaving the appearance underneath is the
    /// duplication ADR 0244 photographed. `--draw-widget-appearances` restores §6.3.2.2's default,
    /// which is what taking the two pictures side by side needs.
    widget_appearances: WidgetAppearances,
    /// How much of what the document asserts over its reader this window obeys.
    ///
    /// The other policy value beside the one above, and the same kind of thing: a fact about what
    /// *this reader* has been asked to do rather than about the file.
    /// What `--restrictions=` asked for until the menu says otherwise, and what the open document
    /// departs from that in — two scopes, because a level set to catch one file would otherwise
    /// catch every file this window opens afterwards (ADR 1145). The word
    /// [`viewer_host::IGNORE_RESTRICTIONS`] turns them all off, and this program refused it as an
    /// unknown option while telling every person who hit a refusal to use it (ADR 0604).
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
    /// answers that have come back — looked at on GTK's main loop while one is out (ADR 1291).
    submitter: viewer_host::submit::Submitter,
    /// §10.8.3: whether this window has asked for the separation simulation.
    ///
    /// Held here rather than asked of the core, because it is a preference a person toggles and
    /// `viewer-core` holds none: this is the window's record of the answer it last sent, so the
    /// toggle knows which way to go. `--separations=` sets what it starts at (ADR 1228).
    separations: bool,
    /// The magnification at which every control on this page would fit its `/Rect`, where they do
    /// not fit now.
    ///
    /// **ADR 0245's third decision, and it needed no message.** `viewer_host::ControlFit` computes
    /// it from what `Query::Fields` already answers and what GTK already says a control's minimum
    /// is; `w` sends it as `Zoom::Scale`. It is offered rather than applied, because a viewer that
    /// magnified a page by itself because a form is on it would be answering a question nobody
    /// asked — which gesture asks for it is chrome, and chrome is a host's (rule 5).
    fit_magnification: Option<f32>,
    /// Whether the reader wants the panel of three trees on the screen.
    ///
    /// **A wish, beside Table 29's *permission***. `Presenting::chrome`'s `other_windows` says
    /// whether the clause allows a panel at all and this says whether the person asked for one;
    /// `apply_chrome` shows it only where both are true, so leaving full screen puts back what the
    /// reader had rather than what the document last permitted. `o` is the key, in all three hosts
    /// since ADR 0526.
    panel_wanted: bool,
    /// Table 29's `/PageMode /FullScreen`, §12.2's chrome flags, and the way back out.
    ///
    /// **The window §12.4.4's presentation had never had** (ADR 0470). Which sentence this window
    /// is obeying is `viewer_host::Presenting`, shared with the other two hosts; what is GTK's is
    /// `GtkWindow::fullscreen` and which widget each of Table 147's three flags names.
    presenting: viewer_host::Presenting,
    /// §12.4.4.1's clock, while a presentation is running.
    ///
    /// **`None` is a window with no timer armed at all**, rather than a timer that wakes to find
    /// nothing to do: `pump_presentation` re-arms a one-shot only while this is `Some`, so a
    /// reader who is not presenting pays nothing. The decision inside it — how often, what a tick
    /// carries, when Table 164's `/D` has run out — is `viewer_host::Clock`, shared with the other
    /// two hosts.
    clock: Option<viewer_host::Clock>,
    /// The one-shot waiting to turn the clock, so that it can be moved when the interval changes.
    ///
    /// One source at a time, removed and re-armed rather than repeating: a transition wants a
    /// frame every sixtieth of a second and a still page wants a tick every tenth, and a repeating
    /// source cannot change its mind.
    armed: Option<glib::SourceId>,
    /// When this window last told the core that time passed for a document's script timers, which
    /// [`Self::armed`] also wakes for while no presentation does (ADR 1702).
    script_ticker: viewer_host::script_timers::Ticker,
    /// A transition named by the core, waiting for the page it moves *to* to be rendered.
    ///
    /// §12.4.4.1's transition is one *to* a page, and the core settles after the command that
    /// turned it: the events arrive as page change, transition, render request. Beginning the
    /// effect here would animate the page being left against itself.
    arming: Option<pdf_model::navigation::Transition>,
    /// The page on the screen, as the list that drew it, the target the core asked for and the
    /// whole pixel the window shows it at — the face a transition would move *from*
    /// (`viewer_host::clock::face_as_shown`).
    ///
    /// Kept only while a presentation is running, because it costs a `Query::PageGeometry` per
    /// page render and nothing outside §12.4.4 asks for it. The list is shared rather than
    /// copied: `RenderRequest::list` is an `Arc` precisely so that a host may keep one.
    shown: Option<(
        std::sync::Arc<pdf_render::DisplayList>,
        pdf_render::TargetSpec,
        (f32, f32),
    )>,
    /// The viewport in device pixels, which is the rectangle a transition's frames are drawn in.
    pub(crate) viewport: (u32, u32),
    /// The document opening on [`Opening`]'s thread, until the first allocation joins it — or,
    /// where it is not ready inside `viewer_host::drawing::WARN`, until the tick that finds it
    /// ready (ADR 1811).
    anticipated: Option<viewer_host::Loading<(Viewer, Vec<Event>)>>,
    /// The latest viewport the window was given while the document was still opening, which the
    /// join tells the core rather than the first one.
    awaiting: Option<Command>,
    /// Page one being drawn while it is read on the opening thread, until the join hands it to
    /// [`Self::drawing`] (ADRs 1855, 1867).
    ahead: Option<viewer_host::Ahead>,
    /// The window as the core was last told it, which a page turn's drawing ahead draws into
    /// (ADR 1871); `None` before the first allocation.
    extent: Option<viewer_host::loading::Window>,
    /// The last look at page one as it was read, shown until the page has a frame of its own.
    read_look: Option<viewer_host::loading::Look>,
    /// The launch document's outline and placed page tree, being read on a thread of their own
    /// from the join until they are handed back ([`viewer_core::Preparation`], ADR 1553).
    preparing: Option<std::sync::mpsc::Receiver<viewer_core::Prepared>>,
}

/// The next of Table 29's six arrangements, in the order that table states them.
///
/// A host's choice and not a clause's: the standard states the six and says nothing about moving
/// between them, because moving between them is a user interface.
/// How wide the panel is, in logical pixels, when it is showing.
///
/// Named because Table 29's `FullScreen` needs to put the divider back where it was: hiding the
/// notebook is not enough on its own, since a `GtkPaned` keeps a position that was set.
///
/// **Three hundred until `viewer_host::Tab` made it six panels**, and the screen is what raised it:
/// six tab labels and a tree do not share three hundred logical pixels. The tabs went down the
/// side at the same time, which is where the rest of the answer is.
const PANEL_WIDTH: i32 = 380;

/// How far §12.5.6.14's text sits from the edge of its window, in logical pixels.
///
/// A choice, and written down as one: the clause states a rectangle and not one word about what a
/// window looks like inside it. The other two hosts make the same choice in their own units.
const POPUP_PADDING: i32 = 5;

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

impl Host {
    /// Builds the window and everything hanging off it, and shows it.
    ///
    /// The document is opening on `opening`'s thread, and the first allocation joins it: the
    /// viewport's size decides the resolution page one is rasterised at, and a page drawn at a
    /// guessed size would be drawn twice (ADR 1539).
    #[must_use]
    pub fn open(
        app: &gtk4::Application,
        opening: Opening,
        widget_appearances: WidgetAppearances,
        settings: viewer_host::Settings,
        trace: Trace,
    ) -> Rc<RefCell<Self>> {
        let Opening {
            path,
            bytes,
            fragment,
            thread,
            ahead,
        } = opening;
        let path = path.as_path();
        trace.say(
            Topic::Launch,
            format_args!("opened {} bytes of {} on disk", bytes.len(), path.display()),
        );
        Rc::new_cyclic(|me| {
            let chrome = Rc::new(RefCell::new(Chrome {
                scale: 1.0,
                ..Chrome::default()
            }));
            let ui = build_window(app, path, me, &chrome);
            trace.say(Topic::Launch, format_args!("window built"));
            RefCell::new(Self {
                viewer: Viewer::new(1, 1, 1.0),
                showing: Showing::new(path.to_owned(), bytes, fragment),
                // One document, and the strip hides its own tabs for it — the launch path opens
                // exactly the one document it always has, under exactly the name it always had
                // (ADR 1264).
                documents: viewer_host::Documents::new(
                    DOCUMENT,
                    viewer_host::documents::label(path),
                ),
                warned: None,
                needle: String::new(),
                pages_left: 0,
                pointer_at: None,
                zoom_wheel: viewer_host::ZoomWheel::default(),
                arrivals: viewer_host::Arrivals::new(),
                panels_due: false,
                rasterizer: CpuRasterizer::new(),
                // No thread yet, and none until a page needs one: `CLAUDE.md` section 2's rule
                // that nothing page one does not need happens before page one.
                drawing: viewer_host::Drawing::new(),
                retained: viewer_host::stand_in::Retained::default(),
                oversized: viewer_host::oversize::Oversized::default(),
                drawing_armed: None,
                script_dialogue: None,
                trace,
                ui,
                chrome,
                suppress: Rc::new(Cell::new(false)),
                me: me.clone(),
                scale: 1,
                miniatures: Rc::new(RefCell::new(Miniatures::new())),
                previews: Rc::new(RefCell::new(panel::Previews::new())),
                opened: false,
                placed: Vec::new(),
                popups: Vec::new(),
                popups_shown: Vec::new(),
                note_editor: None,
                keys_told: pdf_model::view::Keys::default(),
                pressed_a_note: None,
                pressed_a_control: false,
                dragging_a_control: false,
                over_link: false,
                presented: false,
                accessibility: None,
                spoken: None,
                access_pending: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                access_interval: None,
                access_draining: None,
                widget_appearances,
                restrictions: standing(settings),
                links: settings.links,
                remote_documents: settings.remote_documents,
                submitter: viewer_host::submit::Submitter::new(),
                separations: settings.separations,
                fit_magnification: None,
                // The panel is what this window opens with, and `o` is what takes it away.
                panel_wanted: true,
                // Table 147's and Table 29's own defaults, replaced by what the catalog states
                // the moment the document opens.
                presenting: viewer_host::Presenting::default(),
                clock: None,
                armed: None,
                script_ticker: viewer_host::script_timers::Ticker::default(),
                arming: None,
                shown: None,
                viewport: (1, 1),
                anticipated: Some(thread),
                awaiting: None,
                ahead: Some(ahead),
                extent: None,
                read_look: None,
                preparing: None,
            })
        })
    }

    /// This host, weakly, for a callback that has to reach it after the borrow that armed it ends.
    pub(crate) fn me(&self) -> Weak<RefCell<Self>> {
        self.me.clone()
    }
}

/// Runs `what` against the host, or says why it could not.
///
/// A callback that arrives while the host is already borrowed is a callback GTK raised from
/// inside one of this host's own writes — dropping it is right, and saying so is trap 5.
pub(crate) fn with(me: &Weak<RefCell<Host>>, what: impl FnOnce(&mut Host)) {
    let Some(host) = me.upgrade() else {
        return;
    };
    match host.try_borrow_mut() {
        Ok(mut host) => what(&mut host),
        Err(_) => eprintln!("note: the host was busy and an action was dropped"),
    }
}
