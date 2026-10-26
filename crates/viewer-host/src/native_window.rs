//! The half of a native window that is not its toolkit's: the state both toolkit hosts keep about
//! the documents they show, and the methods both answer alike (ADR 1920).
//!
//! `viewer-gtk` and `viewer-qt` each drive one [`Viewer`] behind a window their toolkit draws, and
//! most of what either does with it names no toolkit type: opening the next document waiting,
//! taking the drawing thread back, asking for the part of a zoomed page the viewport shows,
//! saying how a search stands, arming §12.4.4.1's transition. Those were written twice, line for
//! line, and the second copy is where two hosts stop agreeing — this crate's own test, applied to
//! a window's methods rather than to a mapping.
//!
//! Three things make a method a toolkit's and not this module's, and the trait is exactly those
//! three plus the borrow of the state: **what the window says** (a status label in GTK, an update
//! flag the C++ side reads in Qt), **how a command reaches the loop** (GTK's pump on a `VecDeque`,
//! Qt's drain), and **how the page area is drawn again** (GTK's refresh, Qt's frame flag) — with
//! the one deferral each toolkit makes differently, GTK's idle callback and Qt's flag read on the
//! next drain. Everything else a moved method needs is a field of
//! [`WindowState`], which is why a field is here: a method both hosts share reads it.
//!
//! **The event match is here too**, once and exhaustive: [`NativeWindow::react`] hands each event
//! to one method, a provided one where both toolkits answer alike and a required one where the
//! answer is a toolkit's — a dialogue, a clipboard, a panel. So the required methods past the
//! first six are exactly the events a toolkit answers its own way, and an event added to the
//! vocabulary fails to compile here, then in both hosts if its answer is theirs (ADR 1972).
//!
//! **And the launch**: [`Launch`] starts the document opening on a thread of its own before the
//! toolkit is up, and [`NativeWindow::sized`] takes the viewport's sizes, the first of which joins
//! it — each toolkit's part a typed parameter rather than a step left in the host (ADR 2172).
//!
//! What is *not* here is as deliberate. The widgets, the controls placed over the page, the
//! popups, §14.7's tree and the loop's own `run` stay each host's: `run` publishes what changed to
//! the accessibility bridge, whose crate this one does not depend on (ADR 1920 §3).

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use pdf_model::view::WidgetAppearances;
use viewer_core::{
    Answer, Command, DocumentId, Event, Find, FindDirection, Purpose, Query, Viewer,
};

use crate::trace::{Topic, Trace};

mod launch;
mod placing;
mod requests;

pub use launch::{FirstSize, Joining, Launch, Refusals, Sizing};
pub use placing::{Extents, NOT_CENTRED, NOT_CENTRED_TITLE, Placement};
pub use requests::Asked;

/// The identity a native window gives the one document it opens with.
pub const DOCUMENT: DocumentId = DocumentId(1);

/// The window's standing levels, from what the command line set (ADR 1145).
#[must_use]
pub fn standing(settings: crate::Settings) -> crate::Restrictions {
    crate::Restrictions::new(settings.restrictions)
        .with(crate::ActLevel::EmbeddedDocuments(
            settings.embedded_documents,
        ))
        .with(crate::ActLevel::Submissions(settings.submissions))
        .with(crate::ActLevel::Scripts(settings.scripts))
        .with(crate::ActLevel::LaunchedDocuments(
            settings.launched_documents,
        ))
}

/// What an open sends, in the order it sends it: every policy before the document.
///
/// One statement for the open at launch, on the opening thread, and every open after it, on the
/// window's. A policy applied halfway through is not a policy: §6.3.2.2's instruction is a
/// property of *this host* rather than of the document, which is why it is not part of `Open` — a
/// host that changes its mind sends it again and the page is rebuilt (ADR 0245) — and `Restrict`
/// is the reader's answer to what the *file* asserts, which `CLAUDE.md` says is always the
/// reader's to give.
#[must_use]
pub fn opening_commands(
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
        // A page zoomed past its window is drawn as much as the window shows of it, as `quorra`
        // draws every page, and one past the whole-page bound is not refused (ADRs 1892, 1930).
        Command::Oversize(viewer_core::Oversize::Listed),
        Command::Open {
            id: DOCUMENT,
            bytes,
            password,
            fragment,
        },
    ])
}

/// Everything one tab of a native window holds about the document in it.
///
/// A struct rather than a row of fields on the window because a window holds more than one
/// document: [`crate::Documents`] parks one of these beside every tab that is not in front and
/// swaps the whole of it in one move when the person changes tabs. A field that must not leak
/// between two documents therefore cannot be left behind by forgetting it at a call site — either
/// all of it travels or none of it does, and the compiler names a field added here and forgotten
/// there (ADR 1264).
///
/// What is **not** here is as deliberate: the window's restriction levels, `--links=`,
/// `--remote-documents=`, §10.8.3's simulation, §6.3.2.2's delegation, Table 29's full screen and
/// the window's wish for a panel are all facts about the *window* and apply to every document it
/// shows, which is `viewer_core::Command::Restrict`'s own rule for a host-supplied value. Nor are
/// the controls, the popup windows, the placed pictures or the magnification every control fits
/// at: each is rebuilt from a query on the repaint that follows a switch, so parking one would be
/// keeping a second answer to a question the core answers.
#[derive(Debug)]
pub struct Showing {
    /// Where the bytes came from, which is what rule 2 makes a host's business and not the core's.
    pub path: PathBuf,
    /// The directory §12.7.6.4's policy resolves against.
    pub directory: Option<PathBuf>,
    /// The file, kept because §7.6.4.1's second attempt opens it again with a password.
    pub bytes: pdf_syntax::FileBytes,
    /// Annex O's fragment, where the host was given one.
    pub fragment: Option<String>,
    /// §7.6.4.1's attempts, counted by [`crate::Asking`] so that three hosts count alike.
    pub asking: crate::Asking,
    /// Whether the open document still owes what it says about *itself*.
    ///
    /// Asked for once the first frame is on the screen rather than as part of opening, because
    /// §12.8's answer digests the signed part of the file and `CLAUDE.md` principle 2 keeps that
    /// off a launch. [`crate::report::Due`] holds the rule for every host. ADR 1044.
    pub report_due: crate::report::Due,
    /// Whether anything is unsaved.
    pub dirty: bool,
    /// What the title bar says about the page.
    pub caption: String,
    /// What the title bar calls the document where §12.2's `/DisplayDocTitle` asks for its own
    /// title and it states one ([`crate::documents::document_title`]); `None` for the file's name.
    pub titled: Option<String>,
    /// §12.9's measuring: whether a press on the page is a point, and the points so far.
    ///
    /// What is shared with the other windows is [`crate::Measuring`] — when a press is a point,
    /// and what the answer says — and what is a toolkit's is the press and the status line
    /// (ADR 1191).
    pub measuring: crate::Measuring,
    /// Table 29's arrangement, as this window last asked for it.
    ///
    /// Kept because `l` *cycles*: the value in force is the viewer's, and a host that wanted to
    /// know it without remembering would have to ask a question this vocabulary does not have —
    /// which is the right answer, because `Query::Opening` says what the *document* asked for and
    /// that is a different sentence.
    pub layout: pdf_model::viewer_preferences::PageLayout,
    /// What this document departs from the window's restriction levels in.
    ///
    /// The one half of [`crate::Restrictions`] that belongs to a file rather than to the window:
    /// `viewer_core` already keeps the departures beside the document they are about (ADR 1145),
    /// so the menu's ticks travel with the tab and nothing is sent when one changes.
    pub departures: viewer_core::RestrictionOverride,
    /// Whether Table 29's `/PageMode` is still to be obeyed, the first time this tab is in front.
    ///
    /// A document opened behind the one showing is not displayed, and `/PageMode` is "how the
    /// document shall be displayed when opened" — so its full screen and its panel wait for the
    /// pump that first brings it to the front (ADR 1303).
    pub catalog_due: bool,
    /// The active presentation state parked by hosts that restore it with this document.
    /// `None` means its catalog has not supplied a remembered state yet. The active window's
    /// `presenting` remains authoritative until `WindowState::park_presentation` copies it here.
    parked_presentation: Option<crate::Presenting>,
    /// The pages a draw of which left marks to the library's converter, which the title names
    /// (ADR 2290); the sentences themselves are the core's, said once per page and condition.
    pub declined: std::collections::BTreeSet<usize>,
    /// Whether this document asked for §12.2's `/CenterWindow` on a platform that places every
    /// window itself, which the title names ([`NOT_CENTRED_TITLE`], ADR 2308).
    pub uncentred: bool,
}

impl Showing {
    /// A document this window has just been handed, before anything has been read from it.
    #[must_use]
    pub fn new(path: PathBuf, bytes: pdf_syntax::FileBytes, fragment: Option<String>) -> Self {
        Self {
            directory: path.parent().map(Path::to_owned),
            path,
            bytes,
            fragment,
            asking: crate::Asking::new(),
            report_due: crate::report::Due::default(),
            dirty: false,
            caption: String::new(),
            titled: None,
            measuring: crate::Measuring::default(),
            layout: pdf_model::viewer_preferences::PageLayout::SinglePage,
            departures: viewer_core::RestrictionOverride::NONE,
            catalog_due: false,
            parked_presentation: None,
            declined: std::collections::BTreeSet::new(),
            uncentred: false,
        }
    }
}

/// The toolkit-free half of a native window's state: the viewer, the documents it shows, the
/// drawing thread and what it holds, the search, the presentation's clock, and the submissions out.
///
/// Each field is one that a method of [`NativeWindow`] reads; the fields only one toolkit's code
/// reads stay on that toolkit's `Host`, beside its widgets.
#[derive(Debug)]
pub struct WindowState {
    /// The state machine every host on this boundary drives.
    pub viewer: Viewer,
    /// Everything this window holds about the document in front.
    pub showing: Showing,
    /// The documents this window has open, in the order the strip shows them.
    ///
    /// One of these until something opens a second — a `/NewWindow true` on a remote go-to, or a
    /// second file named on the command line — and the strip is hidden until then, so a window
    /// showing one document looks exactly as it did (ADR 1264).
    pub documents: crate::Documents<Showing>,
    /// The documents a reader named that are still to open beside this one — the command line's
    /// later paths and the files chosen with Ctrl + O — one at a time ([`crate::Arrivals`]).
    pub arrivals: crate::Arrivals,
    /// The page rasteriser, on a thread of its own.
    ///
    /// [`crate::Drawing`] is the whole arrangement; what is a toolkit's is the timer that asks it
    /// whether anything has landed.
    pub drawing: crate::Drawing,
    /// The lane beside the drawing thread, where the window has one: the graphics device, read
    /// back (ADR 2070). `None` draws every page on the drawing thread.
    pub lane: Option<Box<dyn crate::drawing::PageLane>>,
    /// A small picture of each page drawn lately, shown where a page arrives on the screen with
    /// no frame until its own lands (ADR 1866).
    pub retained: crate::stand_in::Retained,
    /// The pages zoomed past their window, drawn as it shows them (ADRs 1892, 1930).
    pub oversized: crate::oversize::Oversized,
    /// Which page this window has told the person is taking a long time to draw.
    ///
    /// **The state Escape's third row depends on** ([`crate::Waiting`]), held rather than asked
    /// for twice because it decides two different things: what the status line is saying, and
    /// what the key means while it says it. [`crate::Drawing::overlong`] is where it comes from
    /// and the drawing poll is the only thing that reads it, so a window at rest never asks.
    warned: Option<usize>,
    /// The launch timeline.
    pub trace: Trace,
    /// What the find bar is looking for, kept because the *page's* highlights are asked for on
    /// every repaint while the document-wide search is a plan inside `viewer-core`.
    pub needle: String,
    /// How many pages a search still has to read. Zero when nothing is being searched for.
    ///
    /// A count rather than a flag because it is also what the status line says: a person watching
    /// a thousand-page document wants to see it come down.
    pub pages_left: usize,
    /// §12.4.4.1's clock, while a presentation is running.
    ///
    /// **`None` is a window with no timer armed at all**, rather than a timer that wakes to find
    /// nothing to do. The decision inside it — how often, what a tick carries, when Table 164's
    /// `/D` has run out — is [`crate::Clock`], shared with the other hosts.
    pub clock: Option<crate::Clock>,
    /// What turns [`Self::clock`] and a document's script timers on the toolkit's tick, and
    /// whether the window is hidden or owes a document change's synchronisation (ADR 2300).
    pub pump: crate::presentation_pump::Pump,
    /// A transition named by the core, waiting for the page it moves *to* to be rendered.
    ///
    /// §12.4.4.1's transition is one *to* a page, and the core settles after the command that
    /// turned it: the events arrive as page change, transition, render request. Beginning the
    /// effect here would animate the page being left against itself.
    pub arming: Option<pdf_model::navigation::Transition>,
    /// The page on the screen, as the list that drew it, the target the core asked for and the
    /// whole pixel the window shows it at — the face a transition would move *from*
    /// ([`crate::clock::face_as_shown`]).
    ///
    /// Kept only while a presentation is running, because it costs a `Query::PageGeometry` per
    /// page render and nothing outside §12.4.4 asks for it. The list is shared rather than
    /// copied: `RenderRequest::list` is an `Arc` precisely so that a host may keep one.
    pub shown: Option<(
        std::sync::Arc<pdf_render::DisplayList>,
        pdf_render::TargetSpec,
        (f32, f32),
    )>,
    /// The viewport in device pixels, which is the rectangle a transition's frames are drawn in.
    pub viewport: (u32, u32),
    /// The window as the core was last told it, which a page turn's drawing ahead draws into
    /// (ADR 1871); `None` before the first allocation.
    pub extent: Option<crate::loading::Window>,
    /// Page one being drawn while it is read on the opening thread, until the join hands it to
    /// [`WindowState::drawing`] (ADRs 1855, 1867).
    pub ahead: Option<crate::Ahead>,
    /// The last look at page one as it was read, shown until the page has a frame of its own.
    pub read_look: Option<crate::loading::Look>,
    /// The document opening on its own thread, until the first allocation joins it — or, where
    /// it is not ready inside `crate::drawing::WARN`, until the tick that finds it ready (ADR
    /// 1811).
    pub anticipated: Option<crate::Loading<(Viewer, Vec<Event>)>>,
    /// A page turn's read, out on a thread of its own until the window takes it in (ADR 2344).
    pub turning: Option<crate::loading::Turning>,
    /// The commands that came after a second turn while a turn's read was out, in the order they
    /// came, for the window to hand on once the read is in (ADR 2344).
    pub behind_the_turn: VecDeque<Command>,
    /// The latest viewport the window was given while the document was still opening, which the
    /// join tells the core rather than the first one.
    pub awaiting: Option<Command>,
    /// Whether the viewport has had a size, the first of which joins the open (ADR 2172).
    pub sized: bool,
    /// Whether the first frame has been reported, so that the launch line is printed once.
    pub presented: bool,
    /// Table 29's `/PageMode /FullScreen`, §12.2's chrome flags, and the way back out.
    ///
    /// **The window §12.4.4's presentation had never had** (ADR 0470). Which sentence the window
    /// is obeying is [`crate::Presenting`]'s; what is a toolkit's is how its window goes full
    /// screen and which widget each of Table 147's three flags names.
    pub presenting: crate::Presenting,
    /// §12.7.6.2's submissions on the wire, each on a `submit-form` thread of its own, and the
    /// answers that have come back — looked at on the toolkit's loop while one is out (ADR 1291).
    pub submitter: crate::submit::Submitter,
    /// The conversion to ISO 19005 `--archive` chose, or the person chose in the window, which
    /// the archive key asks for; `None` until one is chosen (ADRs 1954, 1966).
    pub archive: Option<viewer_core::Archiving>,
    /// The password each open document was opened with, which a conversion opens the file again
    /// with (ADR 1967).
    pub kept: crate::password::Kept,
    /// The conversions running, each on an `archive` thread of its own, looked for on the timer
    /// that looks for a server's answers (ADR 1954).
    pub converter: crate::archiving::Converter,
    /// §6.3.2.2: who draws §12.7's widgets — the toolkit's controls, or the document's own
    /// pictures.
    ///
    /// [`WidgetAppearances::Delegated`] unless the command line asked otherwise, because both
    /// toolkit hosts place a real control over every widget and leaving the appearance underneath
    /// is the duplication ADR 0244 photographed. `--draw-widget-appearances` restores §6.3.2.2's
    /// default, which is what taking the two pictures side by side needs.
    pub widget_appearances: WidgetAppearances,
    /// How much of what the document asserts over its reader this window obeys: a fact about
    /// what *this reader* has been asked to do rather than about the file.
    ///
    /// One level per operation, every one of them [`viewer_core::RestrictionLevel::Off`] unless
    /// [`crate::RESTRICTIONS`] or [`crate::IGNORE_RESTRICTIONS`] was on the command line (ADRs
    /// 0604, 1144), until the menu says otherwise. **Two scopes**: the window's levels, and what
    /// the open document departs from them in — a level set to catch one file would otherwise
    /// catch every file this window opens afterwards (ADR 1145).
    pub restrictions: crate::Restrictions,
    /// §10.8.3: whether this window has asked for the separation simulation.
    ///
    /// Held here rather than asked of the core, because it is a preference a person toggles and
    /// `viewer-core` holds none: this is the window's record of the answer it last sent, so the
    /// toggle knows which way to go. `--separations=` sets what it starts at (ADR 1228).
    pub separations: bool,
    /// §12.6.4.8: what this window does when a link asks for a URI, per `--links=`.
    ///
    /// The other direction from [`Self::restrictions`], and that is why it is a second value
    /// rather than a seventh operation of the first: what a document *asserts over its reader* is
    /// one subject, and what a document asks **this machine to start** is the other.
    /// [`crate::Links`] holds `CLAUDE.md`'s four levels over the second (ADR 1155).
    pub links: crate::Links,
    /// §12.6.4.3: what this window does when an action names another file, per
    /// `--remote-documents=`.
    ///
    /// A third value for a third subject, on the entry above's argument: starting another program
    /// on a URL and opening a PDF beside this one in this reader are two decisions, and one word
    /// for both would make each of them mean the other (ADR 1227).
    pub remote_documents: crate::RemoteDocuments,
}

impl WindowState {
    /// What the title bar says after the page's caption: ADR 1831's sentence, where a page was
    /// drawn on the processor rather than the device, the pages whose marks a draw left to the
    /// library's converter (ADR 2290), and a `/CenterWindow` the platform would not carry out (ADR
    /// 2308) — each after a dash, and nothing where none holds.
    #[must_use]
    pub fn title_notes(&self) -> String {
        let mut notes = String::new();
        if let Some(lane) = self.lane.as_ref().and_then(|lane| lane.title()) {
            notes.push_str(" — ");
            notes.push_str(lane);
        }
        if let Some(declined) = crate::status::declined_pages(&self.showing.declined) {
            notes.push_str(" — ");
            notes.push_str(&declined);
        }
        if self.showing.uncentred {
            notes.push_str(" — ");
            notes.push_str(NOT_CENTRED_TITLE);
        }
        notes
    }

    /// Saves the active document's current state before its `Showing` is parked or replaced.
    pub fn park_presentation(&mut self) {
        if self.showing.parked_presentation.is_some() {
            self.showing.parked_presentation = Some(self.presenting);
        }
    }

    /// Restores the newly focused document's state without replaying its catalog or panel commands.
    /// The adopting host must synchronize core mode, clock, transient frames and effective chrome.
    pub fn restore_presentation(&mut self) {
        self.presenting = self.showing.parked_presentation.unwrap_or_default();
    }

    /// A window's state before its document has been read: a viewer with no viewport yet, one tab
    /// for the file at `path` under [`DOCUMENT`], and no thread started for anything but `ahead`,
    /// page one's drawing beside the opening thread's read. `widget_appearances` and `settings` are
    /// what the command line asked of the window, which every open sends before its document.
    #[must_use]
    pub fn new(
        path: &Path,
        bytes: pdf_syntax::FileBytes,
        fragment: Option<String>,
        trace: Trace,
        ahead: crate::Ahead,
        widget_appearances: WidgetAppearances,
        settings: crate::Settings,
    ) -> Self {
        Self {
            viewer: Viewer::new(1, 1, 1.0),
            showing: Showing::new(path.to_owned(), bytes, fragment),
            // One document, and the strip hides itself for it — the launch path opens exactly the
            // one document it always has, under exactly the name it always had (ADR 1264).
            documents: crate::Documents::new(DOCUMENT, crate::documents::label(path)),
            arrivals: crate::Arrivals::new(),
            // No thread yet, and none until a page needs one: `CLAUDE.md` section 2's rule that
            // nothing page one does not need happens before page one.
            drawing: crate::Drawing::new(),
            lane: None,
            retained: crate::stand_in::Retained::default(),
            oversized: crate::oversize::Oversized::default(),
            warned: None,
            trace,
            needle: String::new(),
            pages_left: 0,
            clock: None,
            pump: crate::presentation_pump::Pump::default(),
            arming: None,
            shown: None,
            viewport: (1, 1),
            extent: None,
            ahead: Some(ahead),
            read_look: None,
            anticipated: None,
            turning: None,
            behind_the_turn: VecDeque::new(),
            awaiting: None,
            sized: false,
            presented: false,
            presenting: crate::Presenting::default(),
            submitter: crate::submit::Submitter::new(),
            archive: None,
            kept: crate::password::Kept::new(),
            converter: crate::archiving::Converter::new(),
            widget_appearances,
            restrictions: standing(settings),
            separations: settings.separations,
            links: settings.links,
            remote_documents: settings.remote_documents,
        }
    }

    /// The same state, with the conversion `--archive` chose for the archive key (ADR 1954).
    #[must_use]
    pub fn archiving(self, archive: Option<viewer_core::Archiving>) -> Self {
        Self { archive, ..self }
    }

    /// The same state, with the document opening on its own thread (ADR 1811).
    #[must_use]
    pub fn opening(self, anticipated: crate::Loading<(Viewer, Vec<Event>)>) -> Self {
        Self {
            anticipated: Some(anticipated),
            ..self
        }
    }

    /// Whether anything this window runs on a thread of its own is still out, so that the
    /// timer looking for it keeps running: a server's answer, or a conversion.
    #[must_use]
    pub const fn looking(&self) -> bool {
        self.submitter.interval().is_some() || self.converter.interval().is_some()
    }

    /// When the window should look at its pages again: the drawing thread's interval or the
    /// lane's beside it, the sooner, and `None` while neither has anything out (ADR 2070) — and
    /// [`crate::drawing::POLL`] while a turn's read is out, which the same look takes in (ADR
    /// 2344).
    #[must_use]
    pub fn drawing_interval(&self) -> Option<std::time::Duration> {
        let lane = self.lane.as_ref().and_then(|lane| lane.interval());
        let turning = self.turning.as_ref().map(|_| crate::drawing::POLL);
        [self.drawing.interval(), lane, turning]
            .into_iter()
            .flatten()
            .min()
    }
}

/// A native window: what its toolkit does, and — provided — what both toolkit hosts do alike.
///
/// The first six required methods are the three things a toolkit does differently inside the
/// shared methods — what is said, how a command reaches the loop, and how the page area is asked
/// to be drawn again — the deferral of the next open, and the borrow of the [`WindowState`]. The
/// rest are what [`Self::react`] hands to a toolkit, because each answers the event its own way
/// (ADR 1972). Every provided method was written twice, nearly or exactly alike, before it was
/// written here (ADR 1908 §3, ADR 1920).
pub trait NativeWindow {
    /// The toolkit-free half of this window's state.
    fn state(&self) -> &WindowState;

    /// The toolkit-free half of this window's state, to change.
    fn state_mut(&mut self) -> &mut WindowState;

    /// Puts a sentence where the person reads it, and on standard output; an empty one is nothing.
    fn say(&mut self, what: &str);

    /// Runs one command through the viewer and answers everything it produced.
    fn dispatch(&mut self, command: Command);

    /// Opens the next document waiting once the loop now running has finished.
    fn later_open_the_next(&mut self);

    /// Has the page area drawn again from what the window now holds.
    fn repaint(&mut self);

    /// Puts the caption in the title bar, from [`Showing::caption`] and the document's name.
    fn retitle(&mut self);

    /// A document opened: the first, one beside it, or one a reader named on its way to a tab.
    fn opened(&mut self, document: DocumentId, pages: usize, queue: &mut VecDeque<Command>);

    /// §7.6.4.1's prompt, for the document in front or for one on its way to a tab.
    ///
    /// §7.6.4.1: "the interactive PDF processor should prompt for a password". The prompt is a
    /// window, and a window is a host's — which is the whole reason this event exists rather than
    /// a refusal. How many times to ask is `crate::password`'s, because the clause states no
    /// number and three hosts held three copies of the same three. A document on its way to a tab
    /// of its own counts its own prompts, because the count in the window's fields is the one in
    /// front's.
    fn password_required(&mut self, document: DocumentId);

    /// §12.6.4.8's URI, resolved against this document's own location where the action left it
    /// partial, then opened, declined or put to the person by the one policy three windows share
    /// — `CLAUDE.md`'s four levels over the act of starting another program on a string the
    /// *document* chose (ADRs 1079, 1155). What the toolkit owns is the dialogue.
    fn follow_the_link(&mut self, uri: &str) {
        requests::link(self, uri);
    }

    /// §12.7.6.2's submission, composed by `viewer-core`; whether this machine transmits it is
    /// `crate::may_submit`'s one answer rather than the window's (ADRs 1062, 1291).
    fn submit(&mut self, document: DocumentId, submission: pdf_model::submission::Submission) {
        requests::submit(self, document, submission);
    }

    /// A file the document named and the core cannot read for itself.
    ///
    /// Three of the five purposes are asked at one of four levels and §12.7.6.4's own file is
    /// not, and the difference is what each does: an import puts another file's *values* into
    /// the document being read, while a remote go-to, a thread in another file and a named page
    /// each make this program parse a second PDF — which is the act a person may want to be
    /// asked about. `crate::under_remote_documents` is the division, written once rather than in
    /// each of three windows (ADRs 1227, 1239). An import named by an absolute URI is fetched
    /// rather than read, under the level a submission is sent at (ADR 1527).
    fn needs_file(
        &mut self,
        document: DocumentId,
        purpose: Purpose,
        name: String,
        beside: bool,
        queue: &mut VecDeque<Command>,
    ) {
        requests::needs_file(self, document, purpose, &name, beside, queue);
    }

    /// A signature policy's published copy, under the level the menu holds — the one answer
    /// `crate::policy::may_fetch_signature_policy` gives (ADR 1738). Declined, it is said;
    /// fetched, its answer is looked for as a submission's is and a bound PDF opens beside.
    fn fetch_policy(&mut self, document: DocumentId, policy: crate::PublishedPolicy) {
        requests::fetch_policy(self, document, policy);
    }

    /// §7.11.4's file, or §O.2.1's, taken out of the document: written, or opened beside it.
    fn extracted(
        &mut self,
        document: DocumentId,
        asked: viewer_core::Extraction,
        name: &str,
        bytes: Vec<u8>,
        fragment: Option<String>,
    ) {
        requests::extracted(self, document, asked, name, bytes, fragment);
    }

    /// §7.6.4.2's bit 3 granted: the print this toolkit offers, with the fidelity the core
    /// decided (ADR 1180).
    fn print(&mut self, pages: usize, fidelity: viewer_core::Fidelity);

    /// Text a copy produced, put where this toolkit's clipboard is.
    fn to_clipboard(&mut self, text: String);

    /// Arms whatever looks for a thread's answer on this toolkit's loop, where nothing was being
    /// looked for until now: a server's, or a conversion's report (ADRs 1291, 1954).
    fn look_for_answers(&mut self);

    /// `CLAUDE.md`'s *ask* level over an operation the document restricts, held as
    /// [`Asked::Restricted`]: the words are `crate::restriction`'s and the dialogue the toolkit's
    /// (ADR 1145).
    fn ask_whether_to_proceed(
        &mut self,
        document: DocumentId,
        operation: pdf_model::restriction::Operation,
        notes: &[String],
    ) {
        self.ask(
            Asked::Restricted {
                document,
                operation,
            },
            &crate::asked(operation, notes),
        );
    }

    /// RFC 0008 section 6.3's one question per document before its scripts run, held as
    /// [`Asked::Scripts`] (ADR 1616).
    fn ask_whether_to_run_scripts(&mut self, document: DocumentId, script: &str, first_line: &str) {
        self.ask(
            Asked::Scripts { document },
            &crate::asked_to_run_scripts(script, first_line),
        );
    }

    /// One question at `CLAUDE.md`'s *ask* level, put to the person now — or held while the window
    /// presents full screen and put once it has left (`crate::presentation_pump`, ADR 2309).
    fn ask(&mut self, about: Asked, words: &crate::Question) {
        crate::PresentationPump::put_up(
            self,
            crate::presentation_pump::Held::Question {
                about,
                words: words.clone(),
            },
        );
    }

    /// Puts one question at `CLAUDE.md`'s *ask* level in front of the person, in this toolkit's
    /// dialogue, and hands what they answered — a dismissed dialogue answers `false` — to
    /// [`Self::answer_the_question`] with the same [`Asked`] (ADR 2105). Reached through
    /// [`Self::ask`].
    fn put_the_question(&mut self, about: Asked, words: &crate::Question);

    /// Whether a question is already waiting for its answer in a dialogue a second one would take
    /// the place of; `false` for a toolkit that puts each question in a window of its own.
    fn question_waiting(&self) -> bool;

    /// What the person answered to the question [`Self::put_the_question`] put, carried out the
    /// way every window carries it out (ADR 2105).
    fn answer_the_question(&mut self, about: Asked, proceed: bool) {
        requests::answer(self, about, proceed);
        // A toolkit with one dialogue puts up a second held question only once the first is
        // answered (ADR 2309).
        crate::PresentationPump::put_up_what_was_held(self);
    }

    /// RFC 0008 section 4.2's `app.alert` and `app.response`, in a dialogue of its own (ADR 1628).
    fn ask_for_a_script(&mut self, document: DocumentId, question: viewer_core::ScriptQuestion);

    /// A script's question whose wait ran out, taken down without answering it again (ADR 1643).
    fn withdraw_script_question(&mut self, document: DocumentId);

    /// `app.beep`: the display's one system sound, whichever of the five was asked for (ADR 1702).
    fn beep(&mut self, document: DocumentId, sound: pdf_model::view::Sound);

    /// §7.11.4's list moved: the panel that shows it is built again from the same answer.
    fn attachments_changed(&mut self);

    /// The bar a page still being read showed, handed over to the page being drawn (ADR 1825):
    /// while a draw is shown as it stands the bar carries how much of the page's drawing there is,
    /// and it goes when the page lands.
    fn show_the_drawing(&mut self);

    /// Obeys Table 147's `/FitWindow` and `/CenterWindow` at a frame, while
    /// [`crate::Presenting::place`] says it is owed (ADR 1429).
    fn fit_the_window(&mut self);

    /// Table 147's `/CenterWindow`, asked for by the document opening in front: obeyed where the
    /// toolkit can place its own window, and said where it cannot (trap 5, ADR 1429).
    fn asked_to_centre(&mut self);

    /// Table 147's `/FitWindow` and `/CenterWindow` answered to the window's own `extents` at
    /// `scale` device pixels per logical pixel, for a window that resizes and moves itself by
    /// request: `None` where nothing is owed. The decision `viewer-qt` and the C window share
    /// (`placing`, ADR 2308).
    fn place_the_window(&mut self, scale: f32, extents: Extents) -> Option<Placement> {
        let placement = placing::place(self, scale, extents);
        // Said on the launch topic, because the act is the toolkit's and comes later: a drive
        // reading the window's position reads it against this line (ADR 2345).
        self.state().trace.say(
            Topic::Launch,
            format_args!("placing the window, measured {extents:?}: {placement:?}"),
        );
        placement
    }

    /// Shows the document opening in front the way Table 29's `/PageMode` asks, with the chrome
    /// [`WindowState::presenting`] now says it may have.
    fn show_the_page_mode(&mut self, mode: pdf_model::viewer_preferences::PageMode);

    /// Puts the window in the state [`WindowState::presenting`] now says — full screen or not,
    /// and the chrome it may show — after a change of the document in front brought another
    /// document's presentation forward (`crate::presentation_pump`, ADR 2300).
    fn apply_the_chrome(&mut self);

    /// Runs a queue of commands the way [`Self::dispatch`] runs one: until nothing is left, then
    /// the window is refreshed.
    fn drain(&mut self, queue: VecDeque<Command>);

    /// Reads what page one does not need beside it, on a thread of its own: §12.3.3's outline and
    /// §7.7.3's page tree, placed, which the open no longer reads (ADRs 1543, 1553).
    fn prepare(&mut self);

    /// Puts how far page one has got on the status line, while the document is still opening.
    fn show_the_opening(&mut self);

    /// Takes down the bar a document still opening put up, answering whether there was one to
    /// take down.
    fn take_down_the_bar(&mut self) -> bool;

    /// Does what one event asks.
    ///
    /// **One match for both toolkit windows, and exhaustive**, so that an event added to the
    /// vocabulary fails to compile here, once; where its answer is a toolkit's, the method this
    /// arm calls is a required one and both hosts then fail until each supplies it. Every arm is
    /// one line handing the event to a method (habit 103), and the clause each answers is cited
    /// at the method (ADR 1972).
    fn react(&mut self, event: Event, queue: &mut VecDeque<Command>) {
        match event {
            Event::Opened { document, pages } => self.opened(document, pages, queue),
            Event::OpenFailed { document, reason } => self.open_failed(document, &reason),
            // A dialogue a document raises waits while the window presents (ADR 2309).
            Event::PasswordRequired { document } => crate::PresentationPump::put_up(
                self,
                crate::presentation_pump::Held::Password { document },
            ),
            // A file held for a §12.7.8 named page's password is read and put down again, and the
            // name it was asked under is closed so the window stops holding it (ADR 1335).
            Event::Closed(document) if self.state().arrivals.is(document) => {
                self.given_up(document);
            }
            // Two events with nothing to do here, for two different reasons. A close drops
            // everything derived from the document, and the tab it had is taken down by the
            // window's own `close_document`, which asked for it; damage is what a tier-1 host
            // repaints from `Query::Frame`, and the repaint at the end of every pump has already
            // been scheduled by the time this arrives.
            Event::Closed(_) | Event::Damage(_) => {}
            Event::PageChanged {
                index,
                label,
                of,
                section,
                ..
            } => self.turned(index, label.as_deref(), of, section.as_deref()),
            Event::NeedsRender(request) => self.needs_render(request),
            Event::OpenUri { uri, .. } => self.follow_the_link(&uri),
            Event::Submit {
                document,
                submission,
            } => self.submit(document, *submission),
            Event::NeedsFile {
                document,
                purpose,
                name,
                beside,
            } => self.needs_file(document, purpose, name, beside, queue),
            Event::SignaturePoliciesPublished { document, policies } => {
                self.fetch_policies(document, policies);
            }
            // §12.4.4.1: played since the window was given a clock, and named where it is not.
            Event::Transition { transition, .. } => self.arm_transition(transition),
            Event::Extracted {
                document,
                asked,
                name,
                bytes,
                fragment,
            } => self.extracted(document, asked, &name, bytes, fragment),
            Event::Saved { bytes, .. } => self.write_saved(&bytes),
            Event::Dirty { dirty, .. } => self.dirtied(dirty),
            Event::Searched {
                found,
                remaining,
                wrapped,
                ..
            } => self.searched(found, remaining, wrapped),
            Event::Reported { page, notes, .. } => self.reported(page, &notes),
            Event::Refused { notes, .. } => self.refused(&notes),
            Event::Warned { notes, .. } => self.warned(&notes),
            Event::Printing {
                pages, fidelity, ..
            } => self.print(pages, fidelity),
            // Table 22's bit 11 granted: the conversion runs on a thread of its own (ADR 1954).
            granted @ Event::Archiving { .. } => self.convert(granted),
            Event::Copied {
                logical,
                page_order,
                ..
            } => self.copied(logical, &page_order),
            Event::Asking {
                document,
                operation,
                notes,
            } => self.ask_whether_to_proceed(document, operation, &notes),
            Event::AskingToRunScripts {
                document,
                script,
                first_line,
            } => self.ask_whether_to_run_scripts(document, &script, &first_line),
            Event::ScriptAsking { document, question } => crate::PresentationPump::put_up(
                self,
                crate::presentation_pump::Held::Script { document, question },
            ),
            // Withdrawn before anybody saw it, it leaves the hold and is said withdrawn as one on
            // the screen would be.
            Event::ScriptQuestionWithdrawn { document }
                if self.state_mut().pump.withdraw_held_script(document) =>
            {
                let name = self.state().documents.label_of(document);
                self.say(&crate::script_asks::withdrawn(&name));
            }
            Event::ScriptQuestionWithdrawn { document } => self.withdraw_script_question(document),
            Event::Beep { document, sound } => self.beep(document, sound),
            Event::Console { document, request } => self.console(document, request),
            Event::Media {
                document,
                content_type,
                media,
            } => self.media(document, content_type.as_deref(), &media),
            Event::AttachmentsChanged { .. } => self.attachments_changed(),
        }
    }

    /// A page turn: the title bar, and what the pages now on the screen could not draw.
    ///
    /// §12.4.2: "Page labels and page indices need not coincide". Both are shown, because a
    /// label alone cannot say "of 320".
    fn turned(&mut self, index: usize, label: Option<&str>, of: usize, section: Option<&str>) {
        self.state_mut().showing.caption = crate::documents::caption(index, label, of, section);
        self.retitle();
        self.restate();
    }

    /// Says again what the pages now on the screen could not draw.
    ///
    /// **`Query::Reports` is where a host that cleared its status bar finds it again**, and under
    /// Table 29's continuous arrangements it is the only way to be right: the events came page by
    /// page as each was interpreted, and a scroll that brings a page back does not interpret it
    /// again. The wording is `crate::status`'s, so that three hosts say one sentence.
    fn restate(&mut self) {
        let Answer::Reports(pages) = self.state().viewer.query(Query::Reports) else {
            return;
        };
        self.say(&crate::status::on_screen(&pages));
    }

    /// A page the core asks to have drawn, **handed to the drawing thread rather than drawn on the
    /// toolkit's** (ADR 0668): `rasterize` inside the toolkit's loop would let a page written to
    /// draw for 27.6 s take the window with it — no repaint, no key, and no thread from which
    /// `pdf_render::Interrupt` could be raised. What comes back arrives in the toolkit's
    /// `take_the_drawn`.
    ///
    /// The rows the window shows are drawn first, and the rest of the page after (ADR 1868). A
    /// page zoomed past its window is held, and drawn as the window shows it once the commands are
    /// run (ADRs 1892, 1930).
    fn needs_render(&mut self, request: viewer_core::RenderRequest) {
        self.state_mut().retained.asked(&request);
        let viewport = self.state().viewport;
        if !self.state_mut().oversized.asked(&request, viewport) {
            // The lane beside the drawing thread is offered the page first, unless the drawing
            // thread is about to finish it from the page drawn while it was read (ADR 2070).
            let request = if self.state().drawing.finishes_ahead(&request) {
                Some(request)
            } else {
                let shown = self.shown_of(&request);
                match self.state_mut().lane.as_mut() {
                    Some(lane) => lane.offer(request, shown),
                    None => Some(request),
                }
            };
            if let Some(request) = request {
                let seen = self.seen_of(&request);
                self.state_mut().drawing.ask_showing(request, seen);
            }
        }
    }

    /// What the lane beside the drawing thread has finished, with what it said and the requests it
    /// handed back given to the drawing thread (ADR 2070) — empty for a window with no lane.
    fn take_the_lane(&mut self) -> Vec<crate::Finished> {
        let settle = (!self.state().presented).then_some(crate::drawing::SETTLE);
        let title = self
            .state()
            .lane
            .as_ref()
            .and_then(|lane| lane.title().map(str::to_owned));
        let Some(handed) = self
            .state_mut()
            .lane
            .as_mut()
            .map(|lane| lane.collect(settle))
        else {
            return Vec::new();
        };
        for traced in &handed.traced {
            self.state()
                .trace
                .say(Topic::Frames, format_args!("{traced}"));
        }
        for said in &handed.said {
            self.say(said);
        }
        if self.state().lane.as_ref().and_then(|lane| lane.title()) != title.as_deref() {
            self.retitle();
        }
        for request in handed.back {
            let seen = self.seen_of(&request);
            self.state_mut().drawing.ask_showing(request, seen);
        }
        // A window drawn ahead of its page answers nothing: it is the request's picture until the
        // page lands (ADR 2322), and nothing else repaints for it.
        let ahead = !handed.windows.is_empty();
        for (finished, window) in handed.windows {
            self.state().trace.say(
                Topic::Frames,
                format_args!(
                    "page {} window {}x{} at {},{} drawn on the graphics device ahead of its page \
                     in {:?}, waited {:?}",
                    finished.request.page.saturating_add(1),
                    window.width,
                    window.height,
                    window.x,
                    window.y,
                    finished.cost,
                    finished.waited
                ),
            );
            if let Some(viewer_core::Rendered::Raster(raster)) = finished.outcome {
                self.state_mut()
                    .oversized
                    .ahead(&finished.request, window, raster);
            }
        }
        if ahead {
            self.repaint();
        }
        for finished in &handed.finished {
            self.state().trace.say(
                Topic::Frames,
                format_args!(
                    "page {} drawn on the graphics device and read back {}x{} in {:?}",
                    finished.request.page.saturating_add(1),
                    finished.request.target.width,
                    finished.request.target.height,
                    finished.cost
                ),
            );
        }
        handed.finished
    }

    /// §12.8.3.4.4's policies, whose published copies are fetched under the same level a form is
    /// sent at and opened beside only where a copy is the one the signer signed (ADR 1738): each
    /// one asked of [`Self::fetch_policy`].
    fn fetch_policies(&mut self, document: DocumentId, policies: Vec<crate::PublishedPolicy>) {
        for policy in policies {
            self.fetch_policy(document, policy);
        }
    }

    /// Table 22's bit 11 granted: the conversion starts on a thread of its own, and its report
    /// is looked for on the toolkit's loop, armed here where nothing was being looked for (ADR
    /// 1954).
    fn convert(&mut self, granted: Event) {
        let first = !self.state().looking();
        self.start_archiving(granted);
        if first && self.state().looking() {
            self.look_for_answers();
        }
    }

    /// Whether the document has changes a save would write, in the title bar's mark.
    fn dirtied(&mut self, dirty: bool) {
        self.state_mut().showing.dirty = dirty;
        self.retitle();
    }

    /// `CLAUDE.md`: a document's restrictions are the reader's to set, and it shall always be
    /// possible to turn them off. The sentence is `crate::refused` rather than a window's own
    /// because it names the word the argument parser takes (ADR 0604).
    fn refused(&mut self, notes: &[String]) {
        self.say(&crate::refused(notes));
    }

    /// The *warn* level of `CLAUDE.md`'s four (ADR 0814): a sentence after an edit that went
    /// ahead. *Ask* is [`Self::ask_whether_to_proceed`], in the modal shape §7.6.4.1's password
    /// already had (ADR 1145).
    fn warned(&mut self, notes: &[String]) {
        self.say(&crate::warned(notes));
    }

    /// What a copy produced, on the clipboard, and how much of it in which order.
    fn copied(&mut self, logical: Option<String>, page_order: &str) {
        if let Some(copied) = crate::copied(logical, page_order) {
            self.say(&format!(
                "copied {} characters in {}",
                copied.text.chars().count(),
                copied.order
            ));
            self.to_clipboard(copied.text);
        }
    }

    /// A script's console request: this window's console is its log, so it is said (ADR 1771).
    fn console(&mut self, document: DocumentId, request: pdf_model::view::ConsoleRequest) {
        let said =
            crate::script_timers::console(&self.state().documents.label_of(document), request);
        self.say(&said);
    }

    /// §13.2.4.2's clip handed over: no player is started yet, and the window says so (ADR 1961).
    fn media(
        &mut self,
        document: DocumentId,
        content_type: Option<&str>,
        media: &viewer_core::Media,
    ) {
        let said = crate::handed_media(
            &self.state().documents.label_of(document),
            content_type,
            media,
        );
        self.say(&said);
    }

    /// Takes whatever the drawing thread has finished and answers the viewer for it.
    ///
    /// **A `Finished` whose outcome is `None` is answered to nobody**, which is trap 20 and is why
    /// the type carries an `Option` rather than a `Rendered`: `Rendered::Failed` would record the
    /// page as answered for and stop the scheduler ever asking again, which freezes a page this
    /// window merely chose not to finish drawing.
    ///
    /// **Before the first frame this waits, and after it this only asks** — ADR 0678. A poll
    /// cannot be dispatched while the toolkit's loop is inside its own first frame, and that frame
    /// is the expensive one, so page one used to wait the whole of it out for an answer that was
    /// ready in three milliseconds. A window with nothing on the screen has no frame to spoil and
    /// no input to lose, and `crate::drawing::SETTLE` bounds the whole of what it may spend.
    fn take_the_drawn(&mut self, queue: &mut VecDeque<Command>) {
        self.take_the_turn(queue);
        let laned = self.take_the_lane();
        let drawn = if self.state().presented {
            self.state_mut().drawing.collect()
        } else {
            self.state_mut().drawing.settle(crate::drawing::SETTLE)
        };
        if let Some(lane) = self.state_mut().lane.as_mut() {
            for finished in drawn.iter().filter(|finished| finished.outcome.is_some()) {
                lane.processor_drew(&finished.request, finished.cost);
            }
        }
        let drawn: Vec<crate::Finished> = laned.into_iter().chain(drawn).collect();
        // A draw that ended takes the bar it was showing with it.
        if !drawn.is_empty() {
            self.show_the_drawing();
        }
        for finished in drawn {
            self.state().trace.say(
                Topic::Frames,
                format_args!(
                    "page {} {} {}x{} in {:?}, waited {:?}",
                    finished.request.page.saturating_add(1),
                    if finished.outcome.is_some() {
                        "rasterised"
                    } else {
                        "abandoned after"
                    },
                    finished.request.target.width,
                    finished.request.target.height,
                    finished.cost,
                    finished.waited
                ),
            );
            self.declined_in(&finished.request, finished.declined, queue);
            let Some(rendered) = finished.outcome else {
                continue;
            };
            // The page's own frame, so the window that stood in for it since a notch back is let
            // go (ADR 1930).
            self.state_mut().oversized.answered_whole(&finished.request);
            if let viewer_core::Rendered::Raster(raster) = &rendered {
                let began = std::time::Instant::now();
                if self.state_mut().retained.landed(&finished.request, raster) {
                    self.state().trace.say(
                        Topic::Frames,
                        format_args!(
                            "page {} retained, {} held, in {:?}",
                            finished.request.page.saturating_add(1),
                            self.state().retained.len(),
                            began.elapsed()
                        ),
                    );
                }
            }
            queue.push_back(Command::RenderReady {
                token: finished.request.token,
                rendered,
            });
            self.answered(&finished.request, queue);
        }
        self.take_the_windows(queue);
        self.mind_a_long_draw();
    }

    /// What a draw of `request`'s page declined, handed to the viewer, which says each condition
    /// once (`Command::Declined`), and named in the title (ADR 2290).
    fn declined_in(
        &mut self,
        request: &viewer_core::RenderRequest,
        declined: Vec<viewer_core::Declined>,
        queue: &mut VecDeque<Command>,
    ) {
        if declined.is_empty() {
            return;
        }
        queue.push_back(Command::Declined {
            document: request.document,
            page: request.page,
            declined,
        });
        if self.state_mut().showing.declined.insert(request.page) {
            self.retitle();
        }
    }

    /// What follows the viewer's being answered for `request`, by its whole frame or by the first
    /// window drawn of it (ADR 1930).
    fn answered(&mut self, request: &viewer_core::RenderRequest, queue: &mut VecDeque<Command>) {
        // The reader has their page, so the document may now be asked what it says about
        // itself — §12.8's signatures above all, whose answer digests the signed part of the
        // file. Once per opened document; `crate::report::Due` is the rule. ADR 1044.
        if self.state_mut().showing.report_due.after_a_frame() {
            queue.push_back(Command::Report);
            // And its open sequence, at the same moment and for the same reason: after the
            // first present, never in front of it (ADR 1602).
            queue.push_back(Command::Presented);
        }
        // Table 147's `/FitWindow` and `/CenterWindow` are about the first displayed page's
        // size, which is known now and not before (ADR 1429).
        if self.state().presenting.owes_placing() {
            self.fit_the_window();
        }
        // §12.4.4.1: the page a transition moves *to* is the one whose list has just
        // arrived, so this is where an armed one can begin. The face is kept whether or not
        // anything is presenting — it is an `Arc` and a target, and no rasterisation — because
        // §12.6.4.15's transition needs a page to move *from* and states no mode (ADR 1216).
        self.face_arrived(request);
    }

    /// Takes the windows the drawing has finished, and answers the viewer for the first of each
    /// request — the host took its list (ADR 1892).
    fn take_the_windows(&mut self, queue: &mut VecDeque<Command>) {
        let windows = self.state_mut().drawing.windows();
        // A window that lands after its request was answered is a scroll's, and nothing else
        // repaints for it: the look that found it does, as a glimpse is repainted for. A window
        // abandoned changes no picture, so it asks for none (ADR 1972).
        let repaint = windows
            .iter()
            .any(|(finished, _)| finished.outcome.is_some());
        for (mut finished, window) in windows {
            self.state().trace.say(
                Topic::Frames,
                format_args!(
                    "page {} window {}x{} at {},{} {} in {:?}, waited {:?}",
                    finished.request.page.saturating_add(1),
                    window.width,
                    window.height,
                    window.x,
                    window.y,
                    if finished.outcome.is_some() {
                        "rasterised"
                    } else {
                        "abandoned"
                    },
                    finished.cost,
                    finished.waited
                ),
            );
            if finished.outcome.is_none() {
                self.state_mut()
                    .oversized
                    .abandoned(&finished.request, window);
                continue;
            }
            let declined = std::mem::take(&mut finished.declined);
            self.declined_in(&finished.request, declined, queue);
            let request = finished.request.clone();
            if let Some((token, rendered)) = self.state_mut().oversized.landed(finished, window) {
                queue.push_back(Command::RenderReady { token, rendered });
                self.answered(&request, queue);
            }
        }
        if repaint {
            self.repaint();
        }
    }

    /// Puts §12.7.6.2's request on a `submit-form` thread of its own, and the answer is looked
    /// for on the toolkit's loop, at `crate::submit::LOOK`, while one is out.
    fn send_form(
        &mut self,
        document: DocumentId,
        submission: pdf_model::submission::Submission,
        warned: Option<String>,
    ) {
        self.say(&crate::policy::submission_note(&submission, None));
        let first = !self.state().looking();
        if let Err(sentence) = self
            .state_mut()
            .submitter
            .send(document, submission, warned, None)
        {
            self.say(&sentence);
            return;
        }
        if first {
            self.look_for_answers();
        }
    }

    /// Puts a policy's GET on a `fetch-policy` thread of its own, looked for as a submission's
    /// answer is.
    fn start_policy_fetch(
        &mut self,
        document: DocumentId,
        policy: crate::PublishedPolicy,
        warned: Option<String>,
    ) {
        self.say(&crate::policy::policy_fetch_note(&policy));
        let first = !self.state().looking();
        if let Err(sentence) = self
            .state_mut()
            .submitter
            .fetch_policy(document, policy, warned, None)
        {
            self.say(&sentence);
            return;
        }
        if first {
            self.look_for_answers();
        }
    }

    /// Table 29's two display entries, and §12.2's chrome flags with them.
    ///
    /// `/PageLayout` is the *viewer's* to apply — it read the catalog when the document opened —
    /// and what the window needs from it is the value `l` cycles from, so that the first press
    /// moves off what the document asked for rather than back onto it. `/PageMode` is "how the
    /// document shall be displayed when opened", which is a panel for three of its six names,
    /// nothing for `UseNone`, and since ADR 0470 a full-screen window for `FullScreen`.
    ///
    /// `in_front` is false for a document opened behind the one showing. Its layout is its own tab's
    /// and is taken, but its `/PageMode` is "how the document shall be displayed when opened" and it
    /// is not displayed: the window's full screen, presentation and panel stay the front
    /// document's, so a presentation running when a second document arrives keeps its full screen
    /// (ADR 1303).
    fn obey_the_catalog(&mut self, queue: &mut VecDeque<Command>, in_front: bool) {
        let Answer::Opening(opening) = self.state().viewer.query(Query::Opening) else {
            return;
        };
        self.state_mut().showing.layout = opening.layout;
        if opening.layout != pdf_model::viewer_preferences::PageLayout::SinglePage {
            self.say(&format!(
                "this document opens in the {:?} page layout (§7.7.2)",
                opening.layout
            ));
        }
        if !in_front {
            return;
        }
        if let Answer::Preferences(preferences) = self.state().viewer.query(Query::Preferences) {
            self.state_mut().presenting = crate::Presenting::opening(opening, &preferences);
            let presenting = self.state().presenting;
            self.state_mut().showing.parked_presentation = Some(presenting);
            // Trap 5: a document asking for something and getting silence. Two of Table 147's
            // three flags name widgets the window hides on request, and the third names one it
            // keeps on purpose — the menu holding this reader's own restriction levels
            // (`crate::restriction::NOT_THE_DOCUMENTS_TO_HIDE`).
            if preferences.hide_menubar {
                self.say(crate::restriction::NOT_THE_DOCUMENTS_TO_HIDE);
            }
            if preferences.center_window {
                self.asked_to_centre();
            }
            // §12.2's `/DisplayDocTitle`: the title bar shows the document's own title, by the rule
            // the other two windows take too. The metadata stream is read only for a document
            // that asks, so a document that does not costs its opening nothing here.
            let titled = if preferences.display_doc_title {
                match self.state().viewer.query(Query::Properties) {
                    Answer::Properties {
                        information,
                        metadata,
                    } => crate::documents::document_title(&information, metadata.as_ref()),
                    _ => None,
                }
            } else {
                None
            };
            self.state_mut().showing.titled = titled;
            self.retitle();
        }
        // Queued rather than dispatched: this runs inside `react`, and `dispatch` would start a
        // second pump under the first. `queue` is what that parameter is for.
        if self.state().presenting.full_screen() {
            self.start_the_clock();
            let mode = self.state().presenting.mode();
            queue.push_back(Command::Present(mode));
        }
        self.show_the_page_mode(opening.mode);
    }

    /// One look at a document still opening: joins it where it is ready, and otherwise moves the
    /// bar. Answers whether to look again.
    fn mind_the_opening(&mut self) -> bool {
        if !self
            .state()
            .anticipated
            .as_ref()
            .is_some_and(crate::Loading::ended)
        {
            self.show_the_opening();
            self.show_the_read();
            return self.state().anticipated.is_some();
        }
        // A bar that never went up is not taken down with a sentence: a page read between
        // `GLIMPSE_AFTER` and `WARN` was shown as it was read and said nothing (ADR 1811).
        if self.take_down_the_bar() {
            self.say(crate::loading::READ);
        }
        if let Some(resize) = self.state_mut().awaiting.take() {
            self.take_the_opening(resize);
        }
        false
    }

    /// §7.6.4.1: opens the document, with a password where one has been supplied.
    fn open_document(&mut self, password: Option<viewer_core::Secret>) {
        if let Some(password) = &password {
            self.state_mut().kept.keep(DOCUMENT, password);
        }
        let state = self.state();
        let commands = opening_commands(
            state.restrictions.window(),
            state.widget_appearances,
            state.separations,
            state.showing.bytes.clone(),
            password,
            state.showing.fragment.clone(),
        );
        self.drain(commands);
    }

    /// The viewport took a size, in device pixels — the first joins the open — under what
    /// `sizing` says this toolkit does (ADRs 1539, 1811, 1855, 2172). Answers what is left for the
    /// toolkit to do.
    fn sized(&mut self, width: u32, height: u32, scale: f32, sizing: Sizing) -> Joining {
        launch::sized(self, width, height, scale, sizing)
    }

    /// Takes in the document the opening thread opened, and tells it its viewport.
    fn take_the_opening(&mut self, resize: Command) {
        // Page one as it was drawn while it was read goes to the drawing, whose first frame of it
        // is finished from that drawing rather than drawn (ADR 1855).
        if let Some(ahead) = self.state_mut().ahead.take() {
            self.state_mut().drawing.draw_ahead(ahead);
        }
        let joined = self
            .state_mut()
            .anticipated
            .take()
            .map(crate::Loading::join);
        let Some(Ok((viewer, opened))) = joined else {
            if joined.is_some() {
                eprintln!("the thread opening the document panicked; opening it here instead");
            }
            self.dispatch(resize);
            self.open_document(None);
            return;
        };
        self.state_mut().viewer = viewer;
        self.state().trace.say(
            Topic::Launch,
            format_args!("document joined, {} event(s)", opened.len()),
        );
        // Asked before the open's events are answered, so that the panels built from them do not
        // read the outline on this thread first (ADR 1553).
        self.prepare();
        let resized: Vec<Event> = self.state_mut().viewer.handle(resize).collect();
        let mut queue = VecDeque::new();
        for event in opened.into_iter().chain(resized) {
            self.react(event, &mut queue);
        }
        self.drain(queue);
    }

    /// §7.6.4.1: a person typed a password, or dismissed the prompt with nothing in it.
    ///
    /// Exhaustive over `Supplied` on purpose, which is what holds three hosts level.
    fn password_supplied(&mut self, document: DocumentId, password: viewer_core::Secret) {
        match crate::password::supplied(password) {
            // The document the prompt was about, from its own bytes: one on its way to a tab —
            // a file named, an embedded file §O.2.1's `ef` opened, a file §12.6.4.3 reached —
            // or the first, which is the only other document a prompt is put about (ADR 1332).
            crate::Supplied::Open(secret) if self.state().arrivals.is(document) => {
                self.open_arriving(Some(secret));
            }
            crate::Supplied::Open(secret) if document == DOCUMENT => {
                self.open_document(Some(secret));
            }
            crate::Supplied::Open(_) => {
                self.say(&crate::documents::not_asked(document));
            }
            crate::Supplied::Cancelled => {
                self.say(crate::password::CANCELLED);
                self.cancelled(document);
            }
        }
    }

    /// Puts page one up as it stands while it is read, where a newer look at it has been drawn
    /// since the last (ADRs 1855, 1867), for the window to show until the page has a frame of its
    /// own. Nothing is answered to the core, which has no document yet; the frame that answers is
    /// the first.
    fn show_the_read(&mut self) {
        let Some(look) = self.state().ahead.as_ref().and_then(crate::Ahead::look) else {
            return;
        };
        if self
            .state()
            .read_look
            .as_ref()
            .is_some_and(|shown| shown.serial == look.serial)
        {
            return;
        }
        let commands = look.commands;
        let first = self.state().read_look.is_none();
        self.state_mut().read_look = Some(look);
        self.repaint();
        self.state().trace.say(
            Topic::Frames,
            format_args!("page 1 shown as it is read, {commands} commands drawn"),
        );
        if first {
            self.state().trace.say(
                Topic::Launch,
                format_args!(
                    "page shown as it is read at {:?}",
                    self.state().trace.since_start()
                ),
            );
        }
    }

    /// One page's refusals, where the window says things, with the page named.
    ///
    /// **The page is named because under a column it has to be**: `Event::Reported` has always
    /// carried which page it is about, and a host that dropped it was attributing one page's
    /// refusals to whichever page a person happened to be looking at. `None` is what the
    /// *document* says about itself before any page is drawn, and belongs to no page.
    fn reported(&mut self, page: Option<usize>, notes: &[String]) {
        let notes = notes.join("; ");
        match page {
            Some(page) => self.say(&format!("page {}: {notes}", page.saturating_add(1))),
            None => self.say(&notes),
        }
    }

    /// §7.5.6's update, written beside the document rather than over it.
    ///
    /// Rule 2 in one method: the core produced the bytes and the host owns the filesystem.
    /// Overwriting somebody's document is a decision this program has not been given.
    fn write_saved(&mut self, bytes: &[u8]) {
        let path = self.state().showing.path.with_extension("edited.pdf");
        match std::fs::write(&path, bytes) {
            Ok(()) => self.say(&format!(
                "saved {} bytes to {}",
                bytes.len(),
                path.display()
            )),
            Err(error) => self.say(&format!("cannot write {}: {error}", path.display())),
        }
    }

    /// §7.11.4's file, written beside the document.
    ///
    /// The name is the document's own words and not a path — §7.11.4's note is that `/F` is "a
    /// platform-dependent encoding" — so only its last component is used and only beside the
    /// document, which is the same policy §12.7.6.4's import takes in the other direction.
    fn write_extracted(&mut self, asked: viewer_core::Extraction, name: &str, bytes: &[u8]) {
        // §O.2.1's own sentence, decided once for all three hosts in `crate::policy`: a URI that
        // named a file is not a person who asked for one.
        if let Err(refusal) = crate::may_write_extracted(asked) {
            self.say(&refusal);
            return;
        }
        // Cloned rather than borrowed, because the sentences below are said while it is held.
        let Some(directory) = self.state().showing.directory.clone() else {
            self.say("cannot write the attachment: the document is not in a known directory");
            return;
        };
        let single = Path::new(name).file_name().unwrap_or(name.as_ref());
        let path = directory.join(single);
        match std::fs::write(&path, bytes) {
            Ok(()) => self.say(&format!(
                "wrote {} bytes to {}",
                bytes.len(),
                path.display()
            )),
            Err(error) => self.say(&format!("cannot write {}: {error}", path.display())),
        }
    }

    /// ADR 0668's second half: takes the drawing thread back from a page the arrangement has
    /// stopped showing.
    ///
    /// The *rule* is [`crate::Drawing`]'s and the same in both native windows; what this supplies
    /// is the one fact that type cannot ask for itself, because it holds no viewer: a page Table
    /// 29's arrangement does not show has no place on the screen, so `Query::PageGeometry`
    /// answers `Answer::None` for it.
    fn take_the_thread_back(&mut self) {
        let Some(page) = self.state().drawing.inside() else {
            return;
        };
        // A draw for a tab behind this one is still wanted, and the geometry below is the front's.
        if self
            .state()
            .drawing
            .inside_document()
            .is_some_and(|document| document != self.state().documents.focused())
        {
            return;
        }
        let shown = matches!(
            self.state().viewer.query(Query::PageGeometry(page)),
            Answer::Geometry(_)
        );
        if self.state_mut().drawing.superseded(shown) {
            self.state().trace.say(
                Topic::Frames,
                format_args!(
                    "page {} left the arrangement while it was being drawn, so the draw was \
                     abandoned",
                    page.saturating_add(1)
                ),
            );
        }
    }

    /// Asks the drawing for the part of each page zoomed past its window that the viewport shows
    /// and nothing drawn holds — after a zoom to such a page, and after a scroll out of the window
    /// drawn of it (ADRs 1892, 1930).
    fn ask_for_windows(&mut self) {
        let state = self.state_mut();
        let viewer = &state.viewer;
        let due = state.oversized.due(
            state.documents.focused(),
            |page| match viewer.query(Query::PageGeometry(page)) {
                Answer::Geometry(geometry) => Some(geometry),
                _ => None,
            },
            state.viewport,
        );
        for (request, window) in due {
            state.trace.say(
                Topic::Frames,
                format_args!(
                    "page {} asked as the window shows it, {}x{} at {},{} of {}x{}",
                    request.page.saturating_add(1),
                    window.width,
                    window.height,
                    window.x,
                    window.y,
                    request.target.width,
                    request.target.height
                ),
            );
            state.drawing.ask_window(request, window);
        }
    }

    /// Tells the person about a draw that has outlasted [`crate::drawing::WARN`], and takes the
    /// sentence back when it ends.
    ///
    /// **The *warn* half of the owner's "warn the user and allow the user to abort, however don't
    /// block"**, on the poll the window already runs while something is being drawn — so nothing
    /// new wakes for it and a window at rest asks nothing. The decision, the duration and the
    /// wording are all this crate's; what is a toolkit's is the status line.
    ///
    /// It says something only when the answer *changes*, which is not tidiness: the poll runs
    /// every millisecond, and a status line rewritten a thousand times a second is one nobody can
    /// read.
    fn mind_a_long_draw(&mut self) {
        let overlong = self.state().drawing.overlong();
        if overlong == self.state().warned {
            return;
        }
        let was = self.state_mut().warned.take();
        self.state_mut().warned = overlong;
        match (was, overlong) {
            (_, Some(page)) => self.say(&crate::still_drawing(Some(page))),
            // A page that finished on its own: the sentence offered a key it no longer has a job
            // for, so it is withdrawn rather than left standing.
            (Some(page), None) => self.say(&crate::drew_after_all(Some(page))),
            (None, None) => {}
        }
    }

    /// Whether this window has offered the key that stops a draw — [`crate::Waiting`].
    ///
    /// Read off the state's `warned` rather than asked of the drawing again, and the difference
    /// is the point: what decides Escape's meaning is whether the window has *said* something, not
    /// whether a clock has passed. A key that changed meaning a millisecond before the sentence
    /// appeared would be the guess the sentence exists to remove.
    fn waiting(&self) -> crate::Waiting {
        if self.state().warned.is_some() {
            crate::Waiting::Warned
        } else {
            crate::Waiting::Nothing
        }
    }

    /// Stops the draw the sentence above offered to stop — [`crate::WindowAct::AbortDrawing`].
    ///
    /// Nothing is reported to the viewer for it (trap 20), so the page keeps whatever it was
    /// showing and is drawn again the next time the view changes, which is what the sentence says.
    fn stop_the_long_draw(&mut self) {
        let stopped = self.state_mut().drawing.abandon();
        self.state_mut().warned = None;
        if let Some(page) = stopped {
            self.say(&crate::stopped_drawing(Some(page)));
        }
    }

    /// The part of `request`'s page the viewport shows, where that is less than the page: what
    /// the lane beside the drawing thread may draw ahead of the page (ADR 2322).
    fn shown_of(&self, request: &viewer_core::RenderRequest) -> Option<crate::oversize::Window> {
        if request.document != self.state().documents.focused() {
            return None;
        }
        let Answer::Geometry(geometry) =
            self.state().viewer.query(Query::PageGeometry(request.page))
        else {
            return None;
        };
        crate::oversize::shown(request.target, geometry, self.state().viewport).filter(|window| {
            (window.width, window.height) != (request.target.width, request.target.height)
        })
    }

    /// The rows of a request's target this window shows, for the drawing to draw first — `None`
    /// for a page of a document behind the one in front, which the arrangement does not place
    /// (ADR 1868).
    fn seen_of(&self, request: &viewer_core::RenderRequest) -> Option<std::ops::Range<u32>> {
        if request.document != self.state().documents.focused() {
            return None;
        }
        match self.state().viewer.query(Query::PageGeometry(request.page)) {
            Answer::Geometry(geometry) => {
                crate::drawing::seen(request.target, geometry, self.state().viewport.1)
            }
            _ => None,
        }
    }

    /// One command through the viewer — a page turn through [`Self::turn`] — answering what it
    /// raised, or `None` for a command that waits behind a turn's read and is handed on by
    /// [`Self::take_the_turn`].
    ///
    /// **A second turn waits for the first turn's read, and so does every command after it**, in
    /// the order they came (ADR 2344). The core has no way to stop a read part-way, so a turn
    /// handed out while another page is read would read both at once on the same cores; and a
    /// command let past a turn that waits would be handled before it, out of the order it was
    /// given. A command that comes while the read is out and no turn waits is handled at once —
    /// a frame answered, a zoom, a tick — and the core leaves the page being read unread for it.
    fn through_the_viewer(&mut self, command: Command) -> Option<Vec<Event>> {
        let turn = matches!(command, Command::GoTo(_));
        if self.state().turning.is_some() && (turn || !self.state().behind_the_turn.is_empty()) {
            self.state().trace.say(
                Topic::Events,
                format_args!(
                    "{} waits for page {}'s read",
                    crate::trace::described(&command, 60),
                    self.state()
                        .turning
                        .as_ref()
                        .map_or(0, |turning| turning.page().saturating_add(1))
                ),
            );
            self.state_mut().behind_the_turn.push_back(command);
            return None;
        }
        Some(if turn {
            self.turn(command)
        } else {
            self.state_mut().viewer.handle(command).collect()
        })
    }

    /// A page turn, with the page it brings on read on a thread of its own and drawn while it is
    /// read (ADRs 1871, 1886, 2344).
    ///
    /// `quorra`'s `App::turn`, in this window, with the read off the toolkit's thread: the turn is
    /// handled with the read left to this window ([`Viewer::handle_unread`]) and the read handed
    /// out ([`Viewer::read_out`]) to a thread [`crate::Ahead::read_out`] spawns, the page drawn
    /// beside it; the handler returns at once, so the window paints the page's retained picture
    /// while it is read, and [`Self::take_the_turn`] takes the read in when it ends, after which
    /// the frame is finished by [`WindowState::drawing`] from that drawing where the request is
    /// for that list at that target. Nothing is drawn ahead until the read has run
    /// `GLIMPSE_AFTER`. Where the core hands out no read — an arrangement that brings on more than
    /// one unread page — the page is read here as before, beside the same drawing (trap 154's
    /// arrangement), and before the first allocation there is no target to draw into, and the
    /// turn is handled whole.
    fn turn(&mut self, command: Command) -> Vec<Event> {
        let Some(window) = self.state().extent else {
            return self.state_mut().viewer.handle(command).collect();
        };
        let mut events: Vec<Event> = self.state_mut().viewer.handle_unread(command).collect();
        let ahead = crate::Ahead::page_sized();
        ahead.window_is(window);
        let (viewport, scale) = (window.viewport(), window.scale);
        if let Some(read_out) = self.state_mut().viewer.read_out(viewport, scale) {
            self.state().trace.say(
                Topic::Frames,
                format_args!(
                    "page {} read off the toolkit thread",
                    read_out.page().saturating_add(1)
                ),
            );
            self.state_mut().turning = Some(ahead.read_out(read_out));
            return events;
        }
        events.extend(ahead.anticipate(&mut self.state_mut().viewer));
        events.extend(self.state_mut().viewer.read_the_rest());
        self.state_mut().drawing.draw_ahead(ahead);
        events
    }

    /// Takes in a turn's read once it has ended — [`Self::turn`]'s second half, on the look the
    /// drawing already gets — answers what it raised, and hands on the commands that waited for
    /// it (ADR 2344).
    fn take_the_turn(&mut self, queue: &mut VecDeque<Command>) {
        if !self
            .state()
            .turning
            .as_ref()
            .is_some_and(crate::loading::Turning::ended)
        {
            return;
        }
        let Some(turning) = self.state_mut().turning.take() else {
            return;
        };
        let (page, read) = (turning.page(), turning.since());
        let (mut events, ahead) = turning.land(&mut self.state_mut().viewer);
        events.extend(self.state_mut().viewer.read_the_rest());
        self.state_mut().drawing.draw_ahead(ahead);
        self.state().trace.say(
            Topic::Frames,
            format_args!(
                "page {} read off the toolkit thread in {read:?}, {} event(s)",
                page.saturating_add(1),
                events.len()
            ),
        );
        for event in events {
            self.react(event, queue);
        }
        let behind = std::mem::take(&mut self.state_mut().behind_the_turn);
        queue.extend(behind);
    }

    /// A step of the search reported. Says what it found, and how much is left to read.
    fn searched(&mut self, found: Option<viewer_core::Found>, remaining: usize, wrapped: bool) {
        self.state_mut().pages_left = remaining;
        match found {
            Some(found) => self.say(&format!(
                "found {:?} on page {}{}",
                self.state().needle,
                found.page.saturating_add(1),
                if wrapped { " (wrapped)" } else { "" }
            )),
            None if remaining == 0 => {
                self.say(&format!(
                    "{:?} is not in this document",
                    self.state().needle
                ));
            }
            None => self.say(&format!(
                "searching for {:?}: {remaining} page(s) left",
                self.state().needle
            )),
        }
    }

    /// The next occurrence anywhere in the document, or the previous one.
    fn find(&mut self, backward: bool) {
        if self.state().needle.is_empty() {
            return;
        }
        let needle = self.state().needle.clone();
        self.dispatch(Command::Find(Find::Start {
            needle,
            direction: if backward {
                FindDirection::Backward
            } else {
                FindDirection::Forward
            },
        }));
    }

    /// Asks for what is selected on the page to be copied, which `Event::Copied` answers with the
    /// text for the toolkit's clipboard (ADR 0519).
    ///
    /// Which of §14.8.2.5's two orders the text is in is *not* a window's decision and is
    /// [`crate::copied`]: the same question in all three windowed hosts, and the third copy of it
    /// is where two of them would stop agreeing.
    fn copy_selection(&mut self) {
        // **A command rather than two questions**: §7.6.4.2's bit 5 restricts taking text out of
        // the document, and a readback can be neither refused, asked about nor warned of. The two
        // answers arrive together on `Event::Copied` and the choice between them is unchanged
        // (ADR 1144).
        if !matches!(
            self.state().viewer.query(Query::Selection),
            Answer::Selected(_)
        ) {
            self.say("nothing on the page is selected to copy");
            return;
        }
        self.dispatch(Command::Copy);
    }

    /// One more point of §12.9's measurement, and what the path comes to now.
    ///
    /// The points are this window's — a rubber band is chrome — and the arithmetic is the
    /// document's, which is the split `viewer_core::Query::Measure` exists for: Table 267's
    /// conversions and §12.9.2's formatting are a reading of the file, and no host holds either.
    fn measure(&mut self, at: (f32, f32)) {
        if !self.state_mut().showing.measuring.point(at) {
            return;
        }
        let points = self.state().showing.measuring.points().to_vec();
        let (traced, located) = match self.state().viewer.query(Query::Measure(&points)) {
            Answer::Measured { traced, located } => (Some(traced), located),
            _ => (None, None),
        };
        self.say(&crate::measuring::said(
            points.len(),
            traced.as_ref(),
            located.as_ref(),
        ));
    }

    /// Starts §12.4.4.1's clock, which is what makes this a presentation rather than a big page.
    fn start_the_clock(&mut self) {
        if self.state().clock.is_none() {
            self.state_mut().clock = Some(crate::Clock::started(std::time::Instant::now()));
        }
    }

    /// The viewport a transition's frames are shaped in, in this window's own device pixels.
    #[expect(
        clippy::cast_precision_loss,
        reason = "window dimensions are far below f32's exact integer range"
    )]
    fn viewport_rect(&self) -> pdf_render::Rect {
        pdf_render::Rect::from_corners(
            pdf_render::Point::new(0.0, 0.0),
            pdf_render::Point::new(
                self.state().viewport.0 as f32,
                self.state().viewport.1 as f32,
            ),
        )
    }

    /// Takes `transition` to be drawn when the page it moves *to* arrives, or says why it is not.
    ///
    /// Armed rather than begun: `Viewer::handle` settles after the command that turned the page,
    /// so the events arrive as page change, transition, render request, and the arriving page's
    /// list is in the last of the three. §12.4.4.1's transition is one *to* a page, so waiting for
    /// that page's own request is the clause's order as well as this host's.
    fn arm_transition(&mut self, transition: pdf_model::navigation::Transition) {
        if !crate::Clock::shapes(&transition, self.viewport_rect()) {
            // The core has already said *why* through `Event::Reported`; a second sentence here
            // would say it twice.
            return;
        }
        // §12.6.4.15 states no mode: a processor "shall render the state of the page viewing area
        // as it exists after completion of the previous action and display it using a transition
        // specified in the action dictionary". So a window that is not presenting gets a clock of
        // its own for the one effect and drops it when it ends (ADR 1216).
        if self.state().clock.is_none() {
            self.state_mut().clock =
                Some(crate::Clock::for_one_transition(std::time::Instant::now()));
        }
        self.state_mut().arming = Some(transition);
    }

    /// Keeps the page just rendered as a transition's face, and begins one that was armed.
    ///
    /// Two page rasterisations happen here and none per frame: the page being left and the page
    /// arriving, each at its own size and put where the window shows it. A transition that
    /// re-rasterised per frame would pay a page's interpretation sixty times a second for the
    /// length of it.
    ///
    /// Only the page of the document in front is a face: a document opening behind it draws its
    /// first page too, and that page is not the one a presented page turn leaves (ADR 1303).
    fn face_arrived(&mut self, request: &viewer_core::RenderRequest) {
        if request.document != self.state().documents.focused() {
            return;
        }
        // At the whole pixel the window shows the page at, so that a face is the settled page
        // pixel for pixel and the transition's last frame meets it without a seam (ADR 1865).
        let origin = match self.state().viewer.query(Query::PageGeometry(request.page)) {
            Answer::Geometry(geometry) => crate::stand_in::whole_pixel(geometry.origin),
            _ => (0.0, 0.0),
        };
        let arriving = (std::sync::Arc::clone(&request.list), request.target, origin);
        let Some(transition) = self.state_mut().arming.take() else {
            self.state_mut().shown = Some(arriving);
            return;
        };
        let leaving = self.state_mut().shown.replace(arriving.clone());
        let Some((list, target, at)) = leaving else {
            self.say(&format!(
                "transition: {:?} was named with no page to move from, so the page is shown at \
                 once",
                transition.style
            ));
            return;
        };
        let began = std::time::Instant::now();
        let (Some(outgoing), Some(incoming)) = (
            crate::clock::face_as_shown(&list, target, at, self.state().viewport),
            crate::clock::face_as_shown(&arriving.0, arriving.1, arriving.2, self.state().viewport),
        ) else {
            self.say(&format!(
                "transition: {:?} was named but the pages behind it would not rasterise, so the \
                 page is shown at once",
                transition.style
            ));
            return;
        };
        self.state().trace.say(
            Topic::Frames,
            format_args!(
                "TRANSITION {:?} over {} s: two {}x{} pages rasterised in {:?}",
                transition.style,
                transition.duration,
                self.state().viewport.0,
                self.state().viewport.1,
                began.elapsed()
            ),
        );
        if let Some(clock) = self.state_mut().clock.as_mut() {
            clock.begin(transition, outgoing, incoming, std::time::Instant::now());
        }
    }

    /// A document that could not be opened, said by the name it arrived under — a tab on its way
    /// in, or the one in front — and the next one waiting started.
    fn open_failed(&mut self, document: DocumentId, reason: &str) {
        if let Some(arriving) = self.state_mut().arrivals.settle(document) {
            self.say(&crate::cannot_open(
                &crate::documents::label(&arriving.named.path),
                reason,
            ));
        } else {
            self.say(&crate::cannot_open(
                &crate::documents::label(&self.state().showing.path),
                reason,
            ));
        }
        self.later_open_the_next();
    }

    /// Every server's answer that has arrived: imported, opened beside, or said (ADR 1291).
    fn take_the_answers(&mut self) {
        for returned in self.state_mut().submitter.collect() {
            let (document, url) = (returned.document, returned.url.clone());
            match returned.reply() {
                crate::submit::Reply::Import {
                    answers,
                    format,
                    bytes,
                    note,
                } => {
                    self.say(&note);
                    self.dispatch(Command::Respond {
                        document,
                        answers,
                        source: url,
                        format,
                        bytes,
                    });
                }
                crate::submit::Reply::Document { bytes, note } => {
                    match crate::submit::keep_answer(&bytes) {
                        Ok(path) => {
                            self.say(&format!("{note}; kept at {}", path.display()));
                            self.state_mut()
                                .arrivals
                                .wait(crate::Named::file(path), false);
                            self.open_the_next();
                        }
                        Err(sentence) => self.say(&format!("{note}; {sentence}")),
                    }
                }
                crate::submit::Reply::Say(note) => self.say(&note),
            }
        }
        self.take_the_conversions();
    }

    /// The archive key: the conversion chosen already, or the choice put in front of the person
    /// (ADR 1966).
    fn ask_to_archive(&mut self) {
        match crate::archiving::pressed(self.state().archive.as_ref()) {
            crate::archiving::Pressed::Convert(command) => self.dispatch(command),
            crate::archiving::Pressed::Choose(choosing) => self.offer_to_archive(choosing),
        }
    }

    /// The chooser again, in a window that has chosen: it starts on the choice the window holds
    /// (ADR 2001), the next conversion is to what is chosen on it, and closing it leaves the choice
    /// the window held (ADR 1985).
    fn choose_the_archive_again(&mut self) {
        let choosing = crate::archiving::Choosing::starting_on(self.state().archive.as_ref());
        self.offer_to_archive(choosing);
    }

    /// Puts the six targets and the shipped profiles in front of the person, in this toolkit's
    /// dialogue; the answer is [`Self::archive_chosen`], or nothing where it was closed.
    fn offer_to_archive(&mut self, choosing: crate::archiving::Choosing);

    /// The person chose a target and a profile: the window keeps the choice for its life, says
    /// so, and asks for the conversion (ADR 1966).
    fn archive_chosen(&mut self, asked: viewer_core::Archiving) {
        self.say(&crate::archiving::remembered(&asked));
        self.state_mut().archive = Some(asked.clone());
        self.dispatch(Command::Archive(asked));
    }

    /// `Event::Archiving`, the grant: the conversion starts on a thread of its own, looked for on
    /// the timer a server's answer is (ADR 1954). Any other event is not a grant and starts
    /// nothing; a thread that could not be started is said.
    fn start_archiving(&mut self, granted: Event) {
        let Event::Archiving {
            document,
            bytes,
            asked,
        } = granted
        else {
            return;
        };
        self.say(&crate::archiving::starting(&asked));
        let path = self.state().showing.path.clone();
        let password = self.state().kept.lend(document);
        if let Err(sentence) = self
            .state_mut()
            .converter
            .start(document, path, bytes, asked, password, None)
        {
            self.say(&sentence);
        }
    }

    /// Every conversion that has finished: its report's lines said in order, and the file written
    /// beside the document it was read from.
    fn take_the_conversions(&mut self) {
        for (path, converted) in self.state_mut().converter.collect() {
            for line in &converted.lines {
                self.say(line);
            }
            let written = crate::archiving::write_beside(&path, &converted);
            self.say(&written);
        }
    }

    /// §O.2.1's embedded document, on its way to a tab of its own beside the one holding it.
    fn open_embedded(
        &mut self,
        document: DocumentId,
        name: &str,
        bytes: Vec<u8>,
        fragment: Option<String>,
    ) {
        self.say(&crate::opening_embedded(name, fragment.as_deref()));
        let named =
            crate::Named::embedded(self.state().showing.directory.as_deref(), name, fragment);
        let behind = document != self.state().documents.focused();
        self.state_mut()
            .arrivals
            .wait_held(named, bytes.into(), behind);
        self.later_open_the_next();
    }

    /// A name held out for a document Table 203's `/NewWindow true` would open beside this one.
    ///
    /// **An offer rather than a request**, which is `viewer_core::Command::Beside`'s own shape: the
    /// name is used only where the action states the entry, and one this window offers and nothing
    /// opens under is simply skipped. It is offered per supplied file so that the reserve is never
    /// stale — a name held across two clicks would have the second document open under the first's
    /// identity, which `Command::Open`'s rule makes a *replacement* of a tab somebody is reading.
    ///
    /// The file waits beside the name in [`crate::Arrivals`], so that a document which opens under
    /// it gets its tab and one that asks for §7.6.4.1's password is asked about as itself
    /// (ADR 1332).
    fn offer_a_name(
        &mut self,
        path: &Path,
        read: &[u8],
        purpose: Purpose,
        queue: &mut VecDeque<Command>,
    ) {
        let state = self.state_mut();
        let name = state
            .arrivals
            .offer(&mut state.documents, path.to_owned(), read, purpose);
        queue.push_back(Command::Beside(Some(name)));
    }

    /// How many pages this window keeps a small picture of, per
    /// [`crate::stand_in::RETAINED_PAGES_WORD`]; zero turns the whole of it off (ADR 1866).
    fn retain_pages(&mut self, extent: usize) {
        self.state_mut().retained = crate::stand_in::Retained::new(extent);
    }

    /// The command line's later paths, each to open as a tab behind the first once page one is up.
    fn open_behind(&mut self, named: Vec<crate::Named>) {
        for one in named {
            self.state_mut().arrivals.wait(one, true);
        }
    }

    /// Starts the next document waiting to open, where none is on its way.
    ///
    /// `viewer_core::Command::Open` under the name [`crate::Arrivals`] reserved, so the core hands
    /// the document every answer this reader has given exactly as it did the first
    /// (`Viewer::adopt`, ADR 1263); the tab is added when `Event::Opened` names it.
    fn open_the_next(&mut self) {
        if let Some(declined) = self.state_mut().arrivals.declined() {
            self.dispatch(declined);
        }
        loop {
            let state = self.state_mut();
            let Some(arriving) = state.arrivals.start(&mut state.documents) else {
                return;
            };
            let id = arriving.id;
            let fragment = arriving.named.fragment.clone();
            match arriving.bytes() {
                Ok(bytes) => {
                    self.dispatch(Command::Open {
                        id,
                        bytes,
                        password: None,
                        fragment,
                    });
                    return;
                }
                Err(sentence) => {
                    self.say(&sentence);
                    self.state_mut().arrivals.settle(id);
                }
            }
        }
    }

    /// §7.6.4.1's second attempt, for the document on its way to a tab rather than the one in
    /// front.
    fn open_arriving(&mut self, password: Option<viewer_core::Secret>) {
        let Some((id, bytes, fragment)) = self.state().arrivals.current().map(|arriving| {
            (
                arriving.id,
                arriving.bytes(),
                arriving.named.fragment.clone(),
            )
        }) else {
            return;
        };
        if let Some(password) = &password {
            self.state_mut().kept.keep(id, password);
        }
        match bytes {
            Ok(bytes) => self.dispatch(Command::Open {
                id,
                bytes,
                password,
                fragment,
            }),
            Err(sentence) => {
                self.say(&sentence);
                self.given_up(id);
            }
        }
    }

    /// A document on its way to a tab that will not arrive — cancelled, refused, declined — and the
    /// next one waiting, which may now start. Nothing for any other name.
    fn given_up(&mut self, id: DocumentId) {
        if self.state_mut().arrivals.settle(id).is_some() || id == DOCUMENT {
            self.later_open_the_next();
        }
    }

    /// [`Self::given_up`], for §7.6.4.1's prompt cancelled or out of attempts: a file a document
    /// named is declined to the core as well, where the next arrival starts (ADR 1345).
    fn cancelled(&mut self, id: DocumentId) {
        self.state_mut().kept.forget(id);
        if self.state_mut().arrivals.cancel(id) || id == DOCUMENT {
            self.later_open_the_next();
        }
    }

    /// What to call the document: the title it states where §12.2 asks for it, else the file's
    /// own name.
    fn named(&self) -> String {
        self.state()
            .showing
            .titled
            .clone()
            .unwrap_or_else(|| crate::documents::label(&self.state().showing.path))
    }

    /// What the title bar says about the page, which is what the window is called after the name.
    fn caption(&self) -> &str {
        &self.state().showing.caption
    }
}
