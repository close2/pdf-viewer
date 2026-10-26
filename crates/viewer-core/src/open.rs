//! One open document, and everything the viewer derives from it.
//!
//! Split from [`crate::Viewer`] because the viewer's own job is the *set* of them and the one
//! viewport they share, while everything here is per document: which page is showing, how large
//! it is drawn, and the pixels that showed it last.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use pdf_model::content::{Interpretation, Placed};
use pdf_model::outline::Outline;
use pdf_model::page_label::PageLabels;
use pdf_model::view::ViewState;
use pdf_model::{Page, Pages};
use pdf_render::{DisplayList, Raster, TargetSpec};
use pdf_syntax::Document;

use crate::command::Zoom;
use crate::viewer::RenderToken;
use pdf_model::action::{EmbeddedGoTo, ImportData};
use pdf_model::view::Pointer;
use pdf_syntax::ObjectId;

mod deferred;
mod destinations;
mod edits;
mod fragments;
mod interpretation;
mod magnification;
mod pages;

pub(crate) use magnification::raster_extent;

/// The zoom a document opens at, where its `/OpenAction` states none.
///
/// Where one does, [`Open::pending_views`] carries it and `apply_view` replaces this on the first
/// settled frame — which is why this is still a mode rather than a number: a document with no
/// opinion should follow the window, and one with an opinion should be obeyed.
const INITIAL_ZOOM: Zoom = Zoom::FitPage;

/// How far one [`Zoom::In`] or [`Zoom::Out`] step moves.
///
/// A quarter larger each press, which is the step every viewer this project's owner uses has
/// converged on. Nothing derives it; it is a choice, and it is written down as one. Public because a
/// host that magnifies by part of a step — a high-resolution wheel's eighth of a notch — states the
/// magnification it wants as a power of it (ADR 2274).
pub const ZOOM_STEP: f32 = 1.25;

/// The smallest and largest magnification a person may reach.
///
/// A bound rather than a limit on stupidity: the pixel budget already refuses a render that is
/// too large, and without this a held-down key would walk the scale into the range where an
/// `f32` transform stops resolving a pixel.
const ZOOM_RANGE: (f32, f32) = (0.02, 64.0);

/// One document, open.
#[derive(Debug)]
pub(crate) struct Open {
    /// The file. Immutable, by rule 1, for as long as it is open.
    ///
    /// **Shared rather than owned, so that a host may read what page one does not need on a
    /// thread of its own** — [`crate::Preparation`] holds a second handle, and `Document` is
    /// `Sync` (ADR 0260). The core itself still runs on one thread (rule 4): what crosses is a
    /// handle to an immutable file, and what comes back is a value (ADR 1543).
    pub(crate) document: Arc<Document>,
    /// What §12.6.4's actions and §8.11's layer switches have changed since it opened.
    pub(crate) view: ViewState,
    /// §12.4.2's labelling ranges, read once when the document opened.
    ///
    /// Once rather than per page turn: the tree is a handful of ranges and reading it costs one
    /// walk, where doing it per page would put a number-tree walk on every arrow key.
    pub(crate) labels: PageLabels,
    /// §12.3.3's outline, read the first time something asks for it and never on the launch path.
    ///
    /// **Not read when the document opens, because page one does not need it** — `CLAUDE.md`
    /// principle 2's "\[a]nything not needed to show page one is deferred until first use". It is
    /// 35.5 M instructions on ISO 32000-2's 988 items, and the title bar's §12.3.3 section, which
    /// was the reason it was read at open, also needs [`Self::page_indices`] — 65.1 M more, the
    /// whole page tree. [`crate::Preparation`] is how a host has both read off the thread that
    /// draws; a host that does not ask has them read at first use instead — `Query::Outline`, or
    /// the first page turn's section (ADR 1543).
    ///
    /// A [`OnceLock`] rather than a cell, as [`Self::page_indices`] and [`Self::about`] are, so
    /// that the document's state may be read from the thread that draws page one while another
    /// reads it (ADR 1855): the state is consulted there and changed nowhere.
    pub(crate) outline: OnceLock<Outline>,
    /// §7.7.3's whole page tree, placed once: which page each object in it is.
    ///
    /// **The largest single item in a large document's open, and it was being paid again on
    /// every page turn.** [`Outline::section_at`] needs an item's destination resolved to a page
    /// number, and resolving many of them one at a time is a tree walk apiece (ADR 0058), so it
    /// builds this map — 1023 pages of ISO 32000-2, about six milliseconds. The document is
    /// immutable, so the map is a function of the file and the same map every time; held here,
    /// the walk is paid once for as long as the document is open rather than once per arrow key.
    ///
    /// A [`OnceLock`] rather than a field of [`Self::around`], because a document with no
    /// outline never needs it at all — and because the walk is exactly the "full page-tree walk"
    /// `CLAUDE.md` principle 2 says is not on the launch path. Nothing on that path builds it:
    /// the opening announcement states a section only where the outline has already been read
    /// (ADRs 0890, 1543).
    pub(crate) page_indices: OnceLock<BTreeMap<ObjectId, usize>>,
    /// How many pages, counting §12.7.8.3.3's imported template pages after the document's own.
    pub(crate) page_count: usize,
    /// Which page is showing, zero-based.
    ///
    /// **Under an arrangement that shows more than one page it is the *current* page**, which is
    /// two things at once and deliberately so: the page every question about "the page being
    /// shown" is answered for, and the page whose row the scroll is measured from. Table 29's
    /// continuous layouts move it as the scroll crosses a row boundary
    /// ([`crate::layout::settle_scroll`]) and nothing on the screen shifts when they do.
    pub(crate) page_index: usize,
    /// Table 29's `/PageLayout`, as it now stands.
    ///
    /// Read from the catalog when the document opens — the entry says the layout "shall be used
    /// when the document is opened", which is an initial state and not a permanent one — and
    /// changed afterwards by [`crate::Command::Layout`], because a reader who cannot leave the
    /// arrangement a file asked for is a reader a file has taken over.
    pub(crate) layout: pdf_model::viewer_preferences::PageLayout,
    /// How large the page is drawn.
    pub(crate) zoom: Zoom,
    /// Whether RFC 0008 section 6.5's open sequence has run, which [`crate::Command::Presented`]
    /// asks once for each document (ADR 1602).
    pub(crate) presented: bool,
    /// What this document's reader answered about its scripts, and the runner standing in until
    /// they did (RFC 0008 section 6.3, ADR 1616).
    pub(crate) consent: crate::scripting::Consent,
    /// How many of the view state's script sentences have gone out as [`crate::Event::Reported`].
    pub(crate) scripts_said: usize,
    /// §12.3.2.1's other two items, waiting for a viewport and a display list to be applied to.
    ///
    /// A destination states a page, a location and a magnification; the page is a property of the
    /// document and is applied at once, while the other two need to know how large the window is
    /// and — for `/FitB`, `/FitBH` and `/FitBV` — what the page's contents cover. Both are known
    /// in `Viewer::settle` and nowhere earlier, so the view waits here until then. ADR 0162.
    ///
    /// **Several, and in order, because Annex O made it possible to state more than one.** A URI's
    /// fragment may say `zoom=150,0,792&view=FitH,500`, and §O.2 requires that both be "processed
    /// and (if required) executed from left to right"; applying them in sequence is what makes
    /// Table 149's null — "the current value of that parameter shall be retained unchanged" — mean
    /// what it says about the value the previous parameter left. Table 29's `/OpenAction` puts at
    /// most one here, so nothing that existed before this list can tell the difference.
    pub(crate) pending_views: Vec<pdf_model::destination::View>,
    /// How far the page is scrolled under the viewport, in device pixels.
    ///
    /// Positive means the content has moved up and left — the top-left of the raster is off the
    /// top-left of the viewport — which is what scrolling down and right does.
    pub(crate) scroll: (f32, f32),
    /// The page point a zoom about the pointer was pointed at, held while the steps go on about
    /// the same point and nothing else moves the page (ADR 2275).
    pub(crate) pointed: Option<magnification::Pointed>,
    /// How many times this document's page has been interpreted.
    ///
    /// What makes a render request stale for a reason other than the page or the resolution: an
    /// edit, a layer switch or a pointer over a rollover appearance all rebuild the display list
    /// *at the same page and the same target*, and without this the scheduler would see a frame
    /// that matched and leave the old picture on the screen.
    pub(crate) revision: u64,
    /// How many times this document's ink has changed — [`crate::RenderRequest::ink`].
    ///
    /// **Not [`Self::revision`], and the difference is the whole of what it is for.** That one
    /// counts *interpretations*, so it moves when a page is interpreted again after leaving the
    /// arrangement and coming back — the same commands, a new number. This one moves only when
    /// [`Self::stale`] says the ink is no longer what it was, which is the one place that decides
    /// it. Two interpretations under the same value of this are the same picture of the same page.
    pub(crate) ink: u64,
    /// Every page Table 29's arrangement puts in the viewport, in page order.
    ///
    /// **One entry rather than a cache of them, and the bound is the view's rather than
    /// invented.** A cache would need a bound and an eviction rule, and `/PageLayout` supplies both
    /// without anybody choosing a number: what is kept is exactly what is on the screen, and what
    /// is evicted is what has scrolled off it. `crate::layout::MOST` is the only figure, and it
    /// bounds the *arrangement* rather than a cache.
    ///
    /// `SinglePage` — Table 29's default and every document that states nothing — puts one entry
    /// here, which is what this crate has always held.
    pub(crate) on_screen: Vec<OnScreen>,
    /// How long the page showing has been shown, in seconds — §12.4.4.1's `/Dur` clock.
    ///
    /// Accumulated from [`crate::Command::Tick`] and reset by every page change, because the
    /// clause makes the duration a property of *this* page rather than of the presentation.
    pub(crate) shown_for: f32,
    /// §12.4.4.2's current navigation node, as a position in this page's `/Next` chain.
    ///
    /// > An interactive PDF processor shall maintain a current navigation node.
    ///
    /// `None` is the clause's own other state — "(Otherwise, there is no current node.)" — and is
    /// what every document has while nothing is presenting, because NOTE 3 respects the nodes
    /// "only when in presentation mode". `crate::presentation` is the whole of what moves it.
    pub(crate) node: Option<usize>,
    /// How long the current navigation node has been shown, in seconds — Table 165's `/Dur` clock.
    ///
    /// A second clock beside [`Self::shown_for`] because the standard states two maxima: a page's
    /// `/Dur` advances the page and a node's advances the node.
    pub(crate) node_shown_for: f32,
    /// §8.11's group states as presentation mode found them (§12.4.4.2 NOTE 2).
    ///
    /// `None` while nothing is presenting *and* for a presenting document with no `/OCProperties`,
    /// which §8.11.4.2 makes decisive — and the two are the same answer here, because restoring
    /// "no optional content" onto a document that has none changes nothing. What says whether a
    /// presentation is running is the viewer's own mode, never this field.
    pub(crate) saved_groups: Option<pdf_model::optional_content::OptionalContent>,
    /// Which annotation the pointer is interacting with, and how (§12.5.5).
    ///
    /// Kept beside [`Self::view`] rather than read back out of it because the question asked
    /// here is "has this changed", and the answer decides whether the page has to be
    /// interpreted again. Set only for an annotation that *has* the appearance in question:
    /// re-interpreting a page because the cursor crossed a link with one appearance stream
    /// would put 2 000 M instructions on a mouse move.
    pub(crate) pointer: Option<(ObjectId, Pointer)>,
    /// The *link* a press went down on, if the button is still down.
    ///
    /// §12.5.6.5's activation region and nothing wider, because what a release here decides is
    /// whether §12.6's action chain runs. [`Self::pressed_on`] is the same question for
    /// §12.5.1's activation, which belongs to every annotation.
    pub(crate) pressed: Option<ObjectId>,
    /// The annotation a press went down on, whatever its subtype.
    ///
    /// Two fields for two clauses: this one is what §12.5.1 calls activating an annotation — "the
    /// user activates the annotation by clicking it" — and a click is a press and a release on
    /// one thing. Kept apart from [`Self::pressed`] because a link under a stamp is two
    /// annotations and the two clauses answer differently about which one was clicked.
    pub(crate) pressed_on: Option<ObjectId>,
    /// Which annotation the pointer is inside, for §12.6.3's `/E` and `/X`.
    ///
    /// Separate from `pointer` above, which is §12.5.5's *appearance* state and is filtered to
    /// annotations whose picture can change. Table 197's enter and exit are about the cursor
    /// crossing an activation region and say nothing about appearances, so an annotation with
    /// one `/AP` and an `/AA /E` still raises the event.
    pub(crate) inside: Option<ObjectId>,
    /// Which widget holds the input focus, for §12.6.3's `/Fo` and `/Bl`.
    ///
    /// **A press inside a widget's active area gives it the focus, and a press anywhere else
    /// takes it away.** §12.6.3 says what happens *when* an annotation "receives the input
    /// focus" and nothing whatever about how it comes to; that is an interface's business, and
    /// this is the choice every pointing interface makes — the same shape as "a press dragged
    /// off a link does not activate it", which this crate already decides for itself and says so.
    ///
    /// Widgets only *for the events*, because Table 197 says so of both entries. The focus itself
    /// moves through every annotation the page has, because §12.5.1 says "the annotations on a
    /// page" without qualification.
    ///
    /// **And it moves by keyboard as well**: `Command::Focused` is that key's message and
    /// `pdf_model::tab_order` is Table 31's `/Tabs`, all five values.
    pub(crate) focus: Option<ObjectId>,
    /// §12.7.6.4's import, waiting for the host to supply the file.
    pub(crate) importing: Option<ImportData>,
    /// §12.6.4.4's embedded go-to, suspended at Table 204's `/F` until the root document arrives.
    ///
    /// The whole action rather than what is left of it, because the rest of the walk is the
    /// action's own `/T` and its `/D` is read in whatever document the walk ends in. Nothing
    /// about the walk changes across the pause, which is why it can be *held* rather than
    /// re-derived: `pdf_model::action::MAX_TARGET_DEPTH` bounded the path when the action was
    /// read, and the limits the target's children are decoded under are this document's.
    pub(crate) resuming: Option<EmbeddedGoTo>,
    /// §12.6.4.3's remote go-to, suspended at Table 203's `/F` until the document arrives.
    ///
    /// The whole action rather than its destination, for `resuming`'s reason: both of the
    /// entries that name a page — `/D` and `/SD` — are read in the document that has not arrived
    /// yet, and neither means anything in this one. ADR 1227.
    pub(crate) opening: Option<pdf_model::action::RemoteGoTo>,
    /// §12.6.4.7's thread action, suspended at Table 209's `/F` until the document arrives.
    ///
    /// The whole action rather than what is left of it, for `opening`'s reason: Table 209's `/D`
    /// and `/B` are read in the document that has not arrived yet — its `/Threads` array, its
    /// thread titles, its bead indices — and none of them means anything in this one. ADR 1239.
    pub(crate) threading: Option<pdf_model::action::ThreadJump>,
    /// An edit the document restricts, held until the person answers `Event::Asking`.
    ///
    /// The *ask* level's whole state: resolved already, for [`Done`]'s reason — what goes ahead
    /// on a `yes` is what was asked for at the moment it was asked — and one at a time, because
    /// a second question while the first stands is a person who moved on. `Command::Answer` takes
    /// it, and a `no` drops it without a word.
    pub(crate) asking: Option<Held>,
    /// The sheet of a print operation that has been granted, or nothing.
    ///
    /// **`Some` is §8.11.4.5's "duration of the print operation"**, and the two things that end
    /// it are `crate::Printing::Finish` and the document closing. While it stands,
    /// [`Self::view`]'s purpose is `Purpose::Print` and its paper is this sheet, so every page
    /// of this document — the ones a printer asks for and the ones on the screen — is interpreted
    /// for paper. `crate::Query::PrintPage` answers nothing without it, which is what keeps
    /// §7.6.4.2's bit 3 a question `crate::Command::Print` asked rather than one a readback
    /// walked around. ADR 1180.
    pub(crate) printing: Option<crate::Sheet>,
    /// What this document departs from the window's restriction levels in.
    ///
    /// **The scope a viewer-wide policy could not express** (ADR 1145): a reader who wants to be
    /// asked before text leaves *this* file has said nothing about the next file they open, and a
    /// window-wide level set for one document is a level every other document then answers to.
    /// `crate::RestrictionOverride::NONE` until [`crate::Command::Restrict`] says otherwise, so a
    /// document that was never overridden is the window's policy exactly.
    ///
    /// **It lives here, which is what makes it end when the document does.** A second document
    /// opened in the same window has its own `Open` and therefore its own nothing, which is ADR
    /// 0604's rule for the window read the other way round: the window keeps its policy for its
    /// whole life, and a departure from it keeps the document's.
    pub(crate) restrictions: crate::RestrictionOverride,
    /// Everything a person has changed, in the order they changed it.
    ///
    /// The log `CLAUDE.md`'s rule 1 asks for: the document is immutable, so an edit is an entry
    /// here and never a change to the file. Undo and redo *are* this log — which is why they
    /// belong in the core rather than being reimplemented by every host — and §7.5.6's
    /// incremental update is a pure function of it.
    pub(crate) log: Vec<Done>,
    /// How many entries of [`Self::log`] are in effect.
    ///
    /// Everything before the cursor has been applied; everything after it has been undone and
    /// may be redone. A new edit truncates the tail, which is what a single log with a cursor
    /// means and what every editor does.
    pub(crate) cursor: usize,
    /// Where [`Self::cursor`] stood when this document was last saved.
    ///
    /// The two together are what "unsaved" means, and the cursor alone is not: §7.5.6's update
    /// writes every edit before the cursor, so a document saved and not touched again owes the
    /// file nothing however long its log is. Recording the position rather than a flag is what
    /// makes an undo back to the saved state clean and a redo past it dirty again, which is the
    /// same reason the log has a cursor at all.
    pub(crate) saved_at: usize,
    /// What is selected: two [`Spot`]s, each a page and an offset into that page's readback.
    ///
    /// **The page is here because Table 29's continuous layouts show several pages**, and it is on
    /// *both* ends because a drag may cross a row boundary. With one page on the screen the page a
    /// selection belongs to is always the current one, so a page turn could end a selection; but
    /// a scroll that crosses a row boundary changes the current page without a person having done
    /// anything to
    /// what they had selected, so the rule is the honest one: a selection lives while the pages
    /// it covers are on the screen.
    pub(crate) selection: Option<Chosen>,
    /// A selection to make once the page it names has been interpreted, and which page that is.
    ///
    /// **Only a search sets this**, and it exists because the two rules about a selection and a
    /// page turn are both right and disagree here: a range is into *one page's* readback, so
    /// `settle` ends a selection when the page turns, and a search's occurrence is a range into
    /// the page it is turning **to**. The one is a stale statement and the other is the point of
    /// the command, and the difference is which side of the turn it was made on — which is what
    /// this field records. The same shape [`Self::pending_views`] uses for §12.3.2.1's
    /// magnification, and for the same reason: it waits for the page.
    pub(crate) pending_selection: Option<Chosen>,
    /// The document-wide search in progress, if one is.
    ///
    /// Kept beside the selection rather than inside it because a search is a *plan* — which pages
    /// are still to be read, and from where — while the selection is the one occurrence it has
    /// arrived at. `None` between searches, which is every moment nobody is looking for anything.
    pub(crate) searching: Option<crate::search::Searching>,
    /// Table Annex O.3's `ef`, waiting to be handed to the host: §7.7.4's tree key it named, and
    /// the rest of the fragment that belongs to the file rather than to this document.
    ///
    /// The same shape [`Self::searching`] has and for the same reason — a fragment parameter whose
    /// work is not this type's — except that this one is finished as the document opens rather
    /// than pumped: the bytes are already in the file, so `Viewer::open` takes this and pushes
    /// `Event::Extracted`. `None` for every fragment that names no embedded file, which is every
    /// fragment anybody writes.
    pub(crate) opening_file: Option<OpeningFile>,
    /// Table Annex O.4's `highlight`, one entry per time the fragment stated it.
    ///
    /// A *list* because §O.2 makes a fragment a sequence of parameters and states no rule against
    /// two of these, and because each is measured against the page that was showing when it was
    /// read — `#page=3&highlight=…&page=7&highlight=…` names a rectangle on each of two pages, and
    /// a single field could only have kept the last. Emptied by nothing: the annex says a document
    /// is *opened* with the rectangle highlighted, so it is a property of how this document was
    /// opened rather than of what a person is doing, and nothing in this vocabulary asks for it
    /// to be taken away.
    pub(crate) highlights: Vec<Highlighted>,
    /// The readback of pages this document has already been read, under a byte budget.
    ///
    /// What a document-wide search costs is interpreting pages nobody is looking at, and ADR 0250
    /// threw each one away as soon as the needle had been compared against it. This keeps them —
    /// the readback alone, never the display list — so that a second search over the same ground
    /// is a string comparison rather than a thousand interpretations. `crate::readback::BUDGET`
    /// is the ceiling and [`Self::stale`] is the one place that empties it.
    pub(crate) readbacks: crate::readback::Readbacks,
    /// §9.6's fonts this document's pages have already loaded, under a byte budget.
    ///
    /// **Beside the document rather than inside `interpret`**, which is where ADR 0256 put the
    /// readback and for the same reason: `pdf_model::content::interpret_with_fonts` is still a
    /// pure function of a document, a page and a view state, and a cache changes what that costs
    /// rather than what it says. What is different from the readback is the *direction* — this
    /// one is an input to the interpretation rather than an output of it — which is why it is
    /// passed in rather than filled from the answer. `pdf_model::FONT_BUDGET` is the ceiling.
    ///
    /// **[`Self::stale`] deliberately does not empty it.** Everything that makes a display list
    /// stale is a move of the *view state* — a layer switched, a value typed, an appearance under
    /// the pointer — and a loaded font is a function of the document alone, so a font that was
    /// right before the layer switch is right after it. Dropping it there would make every layer
    /// switch re-read the page's font programs for no change in any answer.
    ///
    /// **Shared rather than owned** so that a page turn's read can run on a host's thread while
    /// this document goes on answering on its own ([`crate::ReadOut`], ADR 2344); the cache locks
    /// per lookup and per insertion, never for an interpretation.
    pub(crate) fonts: Arc<pdf_model::FontCache>,
    /// ISO 32000-2 §8.10.4's target documents, as [`crate::Command::References`] supplied them.
    ///
    /// Empty for every document a host said nothing about, which is every document by default.
    /// Shared with every other open document rather than parsed per document: the files are the
    /// *reader's* policy, and re-opening them once per tab would parse one PDF several times to
    /// reach the same answer. Replaced wholesale when the command arrives again.
    pub(crate) references: Arc<pdf_model::reference::Supply>,
    /// How many objects lost to a damaged object stream this document has already been told about.
    ///
    /// §7.5.7's losses are discovered when an object inside such a stream is first asked for,
    /// which is *after* the document opened — nothing expands an object stream until something
    /// needs one, and `CLAUDE.md`'s startup rule is why. So the sentence cannot be said once at
    /// open and be true; it is said whenever the count grows, and this is what stops it being
    /// said twice. Host state deliberately: putting it in the page's report would make
    /// `pdf_model::interpret` depend on which pages had been read before it. ADR 0366.
    pub(crate) losses_said: usize,
    /// Whether the reader's refusal to scan this file has been said.
    ///
    /// The same shape as [`Self::losses_said`] for the same reason: a scan is what the reader
    /// runs when the cross-reference table is disproved by an object it names, and that can
    /// happen at any page — so `pdf_syntax::Document::scan_refused` becomes `Some` whenever it
    /// does, and the sentence is said once, when it first is. ADR 0812.
    pub(crate) scan_refusal_said: bool,
    /// What the *document* says about itself, worded the first time somebody asks for it.
    ///
    /// **Off the launch path on purpose, and the reason is a measurement.** [`crate::notes::about`]
    /// answers clauses about the file rather than about any page, and §12.8's is the
    /// expensive one: a signed document's report reads the signed byte ranges and digests them
    /// twice, which on `doc/pdf.js/test/pdfs/xfa_filled_imm1344e.pdf` — three megabytes, one page —
    /// took a `Command::Open` from 99 KiB and 26 read calls to 1304 KiB and 118, and from 1.80 M
    /// instructions to 47.6 M. `CLAUDE.md` principle 2 says "\[a]nything not needed to show page one
    /// is deferred until first use", and none of those eight sentences is needed to draw a page.
    ///
    /// So [`crate::Command::Report`] is what asks, and this is what makes asking twice cost once.
    /// ADR 1044.
    pub(crate) about: OnceLock<crate::notes::About>,
}

/// One end of a selection: a page, and a byte offset into that page's readback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Spot {
    /// The page whose readback the offset is into, zero-based.
    ///
    /// **First, and that is what makes the derived ordering the document's own.** Two spots
    /// compare by page and then by offset, which is the order the pages are read in and the
    /// order [`Chosen::ordered`] needs; deriving it is what stops a second opinion about
    /// "before" from existing.
    pub(crate) page: usize,
    /// How far into that page's readback, in bytes.
    pub(crate) offset: usize,
}

/// What is selected: where the drag anchored, and where its moving end is now.
///
/// Anchor first, then where the pointer is now — in that order rather than sorted, because a
/// selection dragged backwards is a selection, and which end moves is the difference between
/// extending it and starting again.
///
/// **Both ends name a page**, and that is the whole of what a selection crossing a page boundary
/// needs in this crate. Each offset is into *one* page's readback, because the standard offers no
/// offset a selection could be a range of. ISO 32000-2 §9.4.1 says of the text matrix, the text
/// line matrix and the text
/// rendering matrix that they
///
/// > may be specified only within a text object and shall not persist from one text object to the
/// > next
///
/// so text has no continuous position even between two text objects of *one* page, let alone
/// across pages, and §12.4.2's page indices are the only sequence there is. So this is a pair of
/// `(page, offset)` rather than one number, and a pair composes across a boundary where a number
/// could not exist. Table 29's continuous arrangements put several pages on the screen at once,
/// so a drag from one to the next is an ordinary gesture rather than an exotic one, and refusing
/// it would leave the second page's half of a paragraph unselectable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Chosen {
    /// Where the drag anchored.
    pub(crate) from: Spot,
    /// Where its moving end is.
    pub(crate) to: Spot,
}

impl Chosen {
    /// A range inside one page, which is what a search finds and what `Selection::All` is.
    pub(crate) const fn in_page(page: usize, range: (usize, usize)) -> Self {
        Self {
            from: Spot {
                page,
                offset: range.0,
            },
            to: Spot {
                page,
                offset: range.1,
            },
        }
    }

    /// The two ends in the order the pages are read in, rather than in the order they were made.
    pub(crate) fn ordered(self) -> (Spot, Spot) {
        if self.from <= self.to {
            (self.from, self.to)
        } else {
            (self.to, self.from)
        }
    }

    /// Whether anything at all is selected.
    ///
    /// A press makes an empty selection so that the drag after it has an anchor, and an empty
    /// selection highlights nothing and is not one a person can see.
    pub(crate) fn empty(self) -> bool {
        self.from == self.to
    }
}

/// One page of Table 29's arrangement, as far as it has got towards the screen.
///
/// Everything that used to be four fields of [`Open`] describing *the* page — its object, its
/// interpretation, the render outstanding for it and the pixels a host handed back — with the
/// page itself beside them, because an arrangement has several and each is at a different stage.
#[derive(Debug)]
pub(crate) struct OnScreen {
    /// Which page, zero-based.
    pub(crate) page: usize,
    /// The page itself, kept rather than looked up again.
    ///
    /// **`Pages::get` is a walk of the page tree**, and on ISO 32000-2's thousandth page that is
    /// 3.8 ms. Hit-testing a link asks for the page on every pointer move and the geometry is
    /// asked for on every frame, so looking it up each time put several milliseconds of tree
    /// walking on paths a person drives with a mouse (ADR 0124).
    ///
    /// Kept *beside* [`Self::interpreted`] rather than inside it, and that is the whole subtlety:
    /// the display list is thrown away whenever the page's ink changes — a layer switched, a
    /// value typed, §12.5.5's appearance following the pointer — and the *page* has not changed
    /// at any of those. A cache tied to the display list's lifetime would be empty exactly when
    /// a press asks what it landed on, which is how the first version of this failed.
    pub(crate) object: Page,
    /// Where its raster's top-left corner sits in the viewport, in device pixels.
    ///
    /// Recomputed by `Viewer::settle` from [`crate::layout::place`] every time anything moves, so
    /// that a question asked between two commands is answered against the arrangement on screen.
    pub(crate) origin: (f32, f32),
    /// The raster this page occupies at the current magnification, in device pixels.
    ///
    /// Kept beside the origin because the two together are what a hit test needs, and because
    /// recomputing it per pointer move would be [`raster_extent`] on a path a person drives with
    /// a mouse.
    pub(crate) raster: (u32, u32),
    /// Its drawing commands, once it has been interpreted.
    pub(crate) interpreted: Option<Interpreted>,
    /// What §12.5.3's annotations could be re-placed from, for a page the clause is about.
    ///
    /// `None` for every page whose display list the magnification cannot change, which is all but
    /// the pages carrying a `NoZoom` annotation. Kept **beside** [`Self::interpreted`] rather than
    /// inside it, because the two have different lifetimes and that is the whole point of it: a
    /// zoom throws the display list away and keeps this, which is what turns a wheel notch from an
    /// interpretation of the page into a re-run of its annotations (ADR 0777).
    ///
    /// **It is dropped by [`Open::stale`] and only there.** The content half was interpreted
    /// against `Open::view` as it then was, so everything but the magnification is baked into it —
    /// and `stale` is this crate's one funnel for the view state's ink having moved.
    pub(crate) replaceable: Option<pdf_model::content::Replacement>,
    /// Which interpretation of this page [`Self::interpreted`] holds.
    ///
    /// What makes a frame stale for a reason other than the resolution: an edit, a layer switch
    /// or a pointer over a rollover appearance all rebuild the display list at the same page and
    /// the same target. Taken from [`Open::revision`], which counts interpretations across the
    /// document, so that two pages never collide on a number.
    pub(crate) revision: u64,
    /// The resolution and revision the host last drew, whichever tier it is.
    ///
    /// Separate from [`Self::frame`] because a tier-2 host hands back no pixels: it draws onto
    /// its own surface and says so, and without this the scheduler would see nothing on the
    /// screen and ask for the same frame again, for ever.
    pub(crate) shown: Option<(TargetSpec, u64)>,
    /// The pixels a tier-1 host handed back, row-major RGBA with no padding.
    ///
    /// What they were drawn *into* is [`Self::shown`], which is what makes them stale when the
    /// magnification changes — one statement rather than two that have to agree.
    pub(crate) frame: Option<Raster>,
    /// The render that is outstanding for this page, if one is.
    pub(crate) pending: Option<Pending>,
    /// The magnification, as its bits, and the interpretation this page was last refused at —
    /// so that a refusal is said once for the view it is about, and again only when the view or
    /// the page changes rather than at every command that settles (ADR 1892).
    pub(crate) refused: Option<(u32, u64)>,
}

/// §7.11.4's file Annex O's `ef` parameter named, and the fragment that belongs to it.
///
/// Two fields rather than two state fields on [`Open`], because they are one instruction: Table
/// Annex O.3 says the processor "shall open the embedded file … identified by name" and then that
/// "\[a]ny remaining parameters after this parameter apply to the selected embedded file", so a name
/// without its remainder would open the right file at the wrong place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpeningFile {
    /// §7.7.4's `/EmbeddedFiles` key the fragment named, as text.
    pub(crate) name: String,
    /// The rest of the URI's fragment, undecoded — `pdf_model::fragment::Fragment` stopped there.
    ///
    /// `None` where `ef` was the last parameter: the file is opened and nothing is said about
    /// where in it.
    pub(crate) fragment: Option<String>,
}

/// One rectangle ISO 32000-2 Annex O's `highlight` parameter named, and the page it named it on.
///
/// Table Annex O.4: "Open the document with the specified rectangle highlighted." The rectangle is
/// kept in **default user space**, normalised, rather than in device pixels: the window it will be
/// drawn in has not been resized yet when a fragment is read, and a magnification or a scroll
/// afterwards must not move it off the ink it points at. `Viewer::query` maps it the same way
/// §12.5.1's focus ring and §12.5.6.14's popups are mapped, through the one arithmetic ADR 0118
/// keeps in one place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Highlighted {
    /// Which page, zero-based — the one the fragment had selected when it stated the rectangle.
    ///
    /// §O.2's left-to-right rule is what makes this a page rather than a document-wide rectangle:
    /// each row of Table Annex O.4 measures "from the top left corner of the page", and which page
    /// that is depends on the parameters before it — the same dependence the annex spells out for
    /// `comment` in a NOTE.
    pub(crate) page: usize,
    /// `[x0, y0, x1, y1]` in default user space, normalised so that the first corner is the lower
    /// left as the page's own coordinates count.
    pub(crate) rect: [f32; 4],
}

/// One thing a person did, **resolved**, as the log records it.
///
/// Not [`crate::Edit`], and the difference is the whole reason this type exists: an `Edit` is
/// what a host *asks for* and this is what was done. `Edit::Markup` names its target as "what is
/// selected", which is a fact about the moment the command arrived — a replay after the selection
/// moved would mark up something else — so [`crate::Viewer`] resolves it to the page and the
/// quadrilaterals before it reaches the log. Undo and redo replay this, and a replay has to
/// produce what happened rather than what was requested.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Done {
    /// §12.7.4's value, by the field's fully qualified name.
    SetField {
        /// §12.7.4.2's name.
        field: String,
        /// The value: characters, §12.7.5.4's chosen options, or nothing.
        value: pdf_model::view::Entered,
    },
    /// §12.7.4.3's commit of what was typed into a field: Table 199's `/K` in its commit form,
    /// then `/V`, then Table 224's `/CO` (ADR 1592).
    ///
    /// An entry of its own because a replay rebuilds the view from the log alone: a commit left
    /// out of it would come undone at the next edit to any field, and the formatted value with it.
    /// The outcome is a function of the entries before it, so a replay reaches the same one — an
    /// accepted value accepted again, a refused one put back again.
    CommitField {
        /// §12.7.4.2's name.
        field: String,
    },
    /// §12.7.5.3's file-select control, filled with the file a person chose.
    ///
    /// The bytes are shared rather than copied, for [`Done::Attach`]'s reason: the log, an undo
    /// and a redo hold this one file and a replay hands it to `ViewState::choose_file` again.
    ChooseFile {
        /// §12.7.4.2's name.
        field: String,
        /// Table 231 bit 21's "pathname of a file", which is what the field's text becomes.
        pathname: String,
        /// The contents §12.7.6.2 submits.
        bytes: pdf_model::attachment::filing::Payload,
        /// Table 44's media type, where the host knew one.
        mime: Option<String>,
    },
    /// §12.5.6.10's markup, over quadrilaterals in **default user space**.
    Markup {
        /// The pages it was added to, in page order, each with the quadrilaterals of its own
        /// part of the selection: `Page::id` and Table 182's `/QuadPoints`, one entry per run of
        /// a line.
        ///
        /// **A list because a selection crosses page boundaries**, and the standard rather than
        /// convenience is what makes it a list. ISO 32000-2
        /// §12.5.2:
        ///
        /// > A given annotation dictionary shall be referenced from the Annots array of only one
        /// > page.
        ///
        /// and Table 182 states §12.5.6.10's `/QuadPoints` "in default user space", which is a
        /// page's. So a mark-up over a selection spanning two pages **shall** be two annotations
        /// and cannot be one shared between them — and they are one `Done` rather than two so that
        /// the gesture undoes in one, which is what a person who made one drag means by undo.
        pages: Vec<(ObjectId, Vec<[f32; 8]>)>,
        /// Which of the four.
        kind: pdf_model::view::Markup,
        /// Table 166's `/C`.
        colour: [f32; 3],
    },
    /// §12.5.6.6's free text annotation, over a rectangle in **default user space**.
    FreeText {
        /// The page it was added to, which is `Page::id`.
        page: ObjectId,
        /// Table 166's `/Rect`, mapped out of the viewport the drag was measured in.
        rect: [f32; 4],
        /// The colour of the text, which Table 177's `/DA` carries.
        colour: [f32; 3],
    },
    /// §12.5.6.6's `/Contents`, on an annotation an earlier entry of this log added **or on one
    /// the file itself states**.
    ///
    /// **The object is stable across a replay**, which is what lets the log name one at all:
    /// `ViewState::add_free_text` allocates from the document's own highest number plus however
    /// many annotations have been added, and a replay adds the same ones in the same order. The
    /// file's own annotation needs no such argument — its object number *is* its identity.
    SetFreeText {
        /// The annotation.
        annotation: ObjectId,
        /// What it now says.
        text: String,
    },
    /// §12.5.6.4's note, retyped: Table 166's `/Contents` of a text annotation the file states.
    SetNoteText {
        /// The note.
        annotation: ObjectId,
        /// What it now says.
        text: String,
    },
    /// §7.11.4's file, put into one of §7.11.4.1's homes — the page and the rectangle resolved,
    /// where the home is a page, out of the viewport the point was measured in.
    ///
    /// The bytes are shared rather than copied, because the log, an undo and a redo all hold
    /// this one file and a replay hands it to `ViewState::attach` again.
    Attach {
        /// What, and where.
        filing: pdf_model::view::Filing,
    },
    /// §7.11.4's file, taken out by the name it is filed under.
    Detach {
        /// The `/EmbeddedFiles` key.
        name: String,
    },
}

impl Done {
    /// Whether applying or unapplying this entry moves the list `Query::Attachments` answers.
    ///
    /// What `Event::AttachmentsChanged` is sent for, and a file attached to a *page* counts:
    /// the list does not show it, but the page does, and a host listing files beside a page it
    /// draws is owed the same signal either way.
    pub(crate) fn moves_attachments(&self) -> bool {
        matches!(self, Self::Attach { .. } | Self::Detach { .. })
    }
}

/// What [`Open::commit`] changed that a host has an event for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Committed {
    /// Whether [`Open::dirty`] answers differently now.
    pub(crate) dirty_changed: bool,
    /// Whether the entry moves the list `Query::Attachments` answers.
    pub(crate) attachments: bool,
}

/// An operation held for `Command::Answer`, which is the *ask* level's whole state.
///
/// Resolved before it is held, both ways round, for [`Done`]'s reason: what goes ahead on a `yes`
/// is what was asked for at the moment it was asked, and a selection or a field value that moved
/// while the question stood would otherwise silently change what the person agreed to.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Held {
    /// An edit, resolved.
    Edit(Done),
    /// A document read and not yet processed — §12.11.6, held before anything else opening one
    /// does (ADR 1167).
    ///
    /// The one held thing that is not an operation on a document already open, which is why it
    /// carries what the open still needs: Annex O's fragment, whose parameters §O.2.2 puts
    /// *after* the document's own open parameters and which therefore has not been applied yet.
    Process {
        /// The fragment identifier `crate::Command::Open` carried, if it carried one.
        fragment: Option<String>,
    },
    /// A print operation, with the sheet it was asked about — `crate::Command::Print`.
    ///
    /// The sheet is held for [`Self::Copy`]'s reason: under `crate::RestrictionLevel::Ask` the
    /// answer arrives after the person has read the question, and what goes ahead on a `yes` is
    /// the job they were asked about rather than whatever a dialogue says now.
    Print(crate::Sheet),
    /// §7.6.4.2's bit 12, asked of a print the reader's level for bit 3 already let through —
    /// `crate::Command::Print`.
    ///
    /// **The one held question whose `no` is an answer rather than a cancellation**, and Table
    /// 22 is why: bit 3 clear withholds printing and bit 12 clear withholds only the fidelity,
    /// with the job going ahead "limited to a low- level representation of the appearance". So a
    /// person who declines here has asked for a degraded print rather than for no print, and
    /// `Viewer::answer` begins one. ADR 1203.
    PrintFaithfully(crate::Sheet),
    /// A conversion to ISO 19005, with the target and profile it was asked about —
    /// `crate::Command::Archive`, held for [`Self::Print`]'s reason (ADR 1954).
    Archive(crate::Archiving),
    /// A media clip §13.2.2 called viable, held before it is handed over — Table 286's `/TF`
    /// asked as `pdf_model::restriction::Operation::HandOverMedia` (ADR 1961).
    Media {
        /// Table 285's `/CT`.
        content_type: Option<String>,
        /// The clip's data, decoded when the question was asked.
        media: crate::Media,
    },
    /// A copy, with the text already taken in both of §14.8.2.5's orders — `crate::Command::Copy`.
    Copy {
        /// §14.8.2.5's logical content order, where the structure tree reached the whole
        /// selection.
        logical: Option<String>,
        /// The same characters in page content order.
        page_order: String,
    },
}

/// A page, interpreted.
#[derive(Debug)]
/// Which page it is of is [`OnScreen::page`], which is what holds it.
pub(crate) struct Interpreted {
    /// Its drawing commands, resolution-independent and shared with whatever is drawing them.
    pub(crate) list: Arc<DisplayList>,
    /// What could not be drawn, already worded.
    pub(crate) reports: Vec<String>,
    /// What could not be *read*: the per-code counts `Query::Readback` answers with.
    ///
    /// Kept rather than recomputed for the reason `placed` is kept — it is only knowable while
    /// the page is being interpreted — and it costs three words per page rather than a vector.
    pub(crate) shortfall: pdf_model::content::Shortfall,
    /// The page's text, in the order the content stream showed it.
    pub(crate) text: String,
    /// Where each of that text's character codes sits on the page.
    ///
    /// Kept rather than rebuilt because a selection is dragged: a pointer move asks this
    /// question sixty times a second, and re-interpreting the page to answer it would be
    /// 2 000 M instructions a frame.
    pub(crate) placed: Vec<Placed>,
    /// §14.7.5.2's marked-content spans over that readback, for §14.9's tree.
    ///
    /// Kept for the same reason `placed` is and at a far smaller cost: a page's structure tree
    /// is asked for when a screen reader attaches and again on every page change, and the map
    /// from a `/MCID` to the text it produced is only knowable during interpretation. Empty for
    /// the 885 of 974 corpus documents that tag nothing.
    pub(crate) marked: Vec<pdf_model::content::MarkedSpan>,
    /// §14.8.2.2's artifact spans over that readback, which a copy in logical order leaves out
    /// (ADR 1948). Empty on every page that marks none, which is nearly all of them.
    pub(crate) artifacts: Vec<pdf_model::content::ArtifactSpan>,
    /// §14.7.5.3's annotations the structure tree claims, with what each read back, which a copy
    /// in logical order places where the tree names them (ADR 1970). Empty on every untagged page.
    pub(crate) objects: Vec<pdf_model::content::ObjectSpan>,
    /// §14.9's `/Alt`, `/E` and `/Lang` spans, kept beside them for the same reason.
    pub(crate) described: Vec<pdf_model::accessibility::Described>,
    /// The catalog's `/Lang`, which §14.9.2.3 makes the document-wide default.
    pub(crate) language: Option<String>,
    /// Whether this page has an annotation §12.5.3's `NoZoom` makes depend on the magnification.
    ///
    /// The whole point of `NeedsRender` carrying a display list is that a zoom re-rasterises
    /// without re-interpreting, and `NoZoom` is the one thing in the standard that breaks it.
    /// This is what keeps the breakage where the clause put it: 51 of the 974 corpus documents
    /// carry such an annotation and the other 923 never re-interpret on a zoom.
    pub(crate) view_dependent: bool,
    /// The conditions a draw of this reading has declined and the page has been told of, each
    /// once (`Command::Declined`, ADR 2290). Empty on nearly every page.
    pub(crate) declined: Vec<String>,
}

/// A render the host has not answered yet.
#[derive(Debug)]
pub(crate) struct Pending {
    /// What the answer must carry to be believed.
    pub(crate) token: RenderToken,
    /// Which page it is of.
    pub(crate) page: usize,
    /// What it is being drawn into.
    pub(crate) target: TargetSpec,
    /// Which interpretation of that page it is of.
    pub(crate) revision: u64,
}

impl Open {
    /// Opens a document and reads the three things every page turn would otherwise re-read.
    ///
    /// `limits` are [`pdf_syntax::Limits::DEFAULT`] for a document a host named, and the source's
    /// for one a §12.6.4.3 or §12.6.4.7 action reached and a person then unlocked, which is held
    /// to the bounds of the document the action was in, as the jump made without a prompt is.
    ///
    /// # Errors
    ///
    /// Whatever `pdf_syntax` says about the file, including §7.6.4.1's
    /// [`pdf_syntax::SyntaxError::PasswordRequired`], which the caller turns into a prompt
    /// rather than into a failure.
    pub(crate) fn new(
        bytes: impl Into<pdf_syntax::FileBytes>,
        password: Option<&crate::Secret>,
        limits: pdf_syntax::Limits,
    ) -> Result<Self, pdf_syntax::SyntaxError> {
        Ok(Self::around(Self::read(bytes, password, limits)?))
    }

    /// Reads a file under a password, without deriving anything a page turn needs.
    ///
    /// Apart from [`Self::new`] for §12.7.8's named page, whose file is read after §7.6.4.1's
    /// prompt and drawn into another document rather than shown (ADR 1335).
    ///
    /// # Errors
    ///
    /// [`Self::new`]'s.
    pub(crate) fn read(
        bytes: impl Into<pdf_syntax::FileBytes>,
        password: Option<&crate::Secret>,
        limits: pdf_syntax::Limits,
    ) -> Result<Document, pdf_syntax::SyntaxError> {
        Document::open_with_password(
            bytes,
            limits,
            // The one call in this crate that reads a password, which is what
            // `Secret::reveal` is named to make visible at its call site. §7.6.4.1's default
            // user password is the empty string, so a document opened without one asks with it.
            password.map_or("", crate::Secret::reveal),
        )
    }

    /// Everything the viewer derives from a document, read once.
    ///
    /// Separate from [`Self::new`] because §12.6.4.4's embedded go-to produces a `Document` that
    /// was never a file: it comes out of §7.11.4's embedded file stream inside the document
    /// already open, and everything below it is the same.
    pub(crate) fn around(document: Document) -> Self {
        let document = Arc::new(document);
        let pages = Pages::new(&document);
        let page_count = pages.len();
        // Read here, unlike the outline, because the opening announcement names page one by its
        // §12.4.2 label — the caption beside page one needs it — and it is one number tree of a
        // handful of ranges: 1.6 M instructions on ISO 32000-2, against the outline's 35.5 M.
        let labels = PageLabels::read(&document);
        // §12.3.2.1: "the optional OpenAction entry in a document's catalog dictionary may
        // specify a destination that shall be displayed when the document is opened." Table 29
        // states the other half — an absent or unresolvable entry means the top of the first
        // page — which is what `unwrap_or(0)` is, and why nothing is reported here.
        let open_action =
            pdf_model::destination::Destination::open_action(&document).and_then(|destination| {
                let index = destination.page_index(&document, &pages)?;
                (index < page_count).then_some((index, destination.view))
            });
        let page_index = open_action.map_or(0, |(index, _)| index);
        let open_view = open_action.map(|(_, view)| view);
        drop(pages);
        let view = ViewState::of(&document);
        // Table 29: the layout "shall be used when the document is opened". One name off the
        // catalog dictionary, which is already resolved — no page is read for it and no tree is
        // walked, so it costs nothing on the launch path.
        let layout = pdf_model::viewer_preferences::Opening::read(&document).layout;
        Self {
            document,
            view,
            labels,
            outline: OnceLock::new(),
            page_indices: OnceLock::new(),
            page_count,
            page_index,
            layout,
            zoom: INITIAL_ZOOM,
            presented: false,
            consent: crate::scripting::Consent::default(),
            scripts_said: 0,
            pending_views: open_view.into_iter().collect(),
            scroll: (0.0, 0.0),
            pointed: None,
            on_screen: Vec::new(),
            shown_for: 0.0,
            node: None,
            node_shown_for: 0.0,
            saved_groups: None,
            revision: 0,
            ink: 0,
            pointer: None,
            pressed: None,
            pressed_on: None,
            inside: None,
            focus: None,
            importing: None,
            resuming: None,
            opening: None,
            threading: None,
            asking: None,
            printing: None,
            restrictions: crate::RestrictionOverride::NONE,
            log: Vec::new(),
            cursor: 0,
            saved_at: 0,
            selection: None,
            pending_selection: None,
            searching: None,
            opening_file: None,
            highlights: Vec::new(),
            readbacks: crate::readback::Readbacks::default(),
            fonts: Arc::new(pdf_model::FontCache::new()),
            references: Arc::new(pdf_model::reference::Supply::none()),
            losses_said: 0,
            scan_refusal_said: false,
            about: OnceLock::new(),
        }
    }
}

/// What one page of an arrangement is interpreted into, with its reports already worded.
pub(crate) struct Read {
    /// The interpretation itself.
    pub(crate) interpretation: Interpretation,
    /// What it could not draw, as sentences.
    pub(crate) reports: Vec<String>,
    /// The page object, so that the caller need not walk the page tree for it again.
    pub(crate) object: Page,
    /// What §12.5.3's annotations can be re-placed from, where this read made one.
    ///
    /// **`None` means "nothing new", not "throw the old one away"**: a read that *used* a
    /// replacement did not make a second one, and the one it used is still good — so the caller
    /// installs a `Some` and leaves a `None` alone. [`Open::stale`] is what discards them.
    pub(crate) replaceable: Option<pdf_model::content::Replacement>,
}

/// A page's first read, from the parts of [`Open`] it needs and no others — for a read run on a
/// thread of its own while the document's state is consulted on another (ADR 1855). The same
/// read [`interpret`] makes of a page that holds no replacement yet, which page one at launch
/// never does.
pub(crate) fn first_read(
    document: &Arc<Document>,
    page: Page,
    view: &ViewState,
    fonts: &pdf_model::FontCache,
    references: &pdf_model::reference::Supply,
) -> Read {
    let (interpretation, replaceable) =
        pdf_model::content::interpret_replaceable(document, &page, view, fonts, references);
    let reports = interpretation
        .unsupported
        .iter()
        .map(crate::report::describe)
        .collect();
    Read {
        interpretation,
        reports,
        object: page,
        replaceable,
    }
}

/// The interpretation of a page, with its reports already worded.
///
/// **`replace` where there is something to replace from, and a whole interpretation otherwise**,
/// and the two produce the same [`Interpretation`] — which is a property rather than a hope:
/// `pdf_model::content::replace` restores the interpreter's state at the seam before §12.5.3's
/// annotation pass and runs that pass again, and `pdf-model`'s
/// `a_replaced_page_is_the_page_it_would_have_been_interpreted_as` compares the two field by field
/// over every corpus page the clause is about. ADR 0777.
pub(crate) fn interpret(open: &Open, index: usize) -> Option<Read> {
    let page = open.page(index)?;
    let replacement = open
        .on(index)
        .and_then(|on_screen| on_screen.replaceable.as_ref());
    let (interpretation, replaceable) = match replacement {
        Some(replacement) => (
            pdf_model::content::replace(
                &open.document,
                &page,
                &open.view,
                &open.fonts,
                &open.references,
                replacement,
            ),
            None,
        ),
        None => pdf_model::content::interpret_replaceable(
            &open.document,
            &page,
            &open.view,
            &open.fonts,
            &open.references,
        ),
    };
    let reports = interpretation
        .unsupported
        .iter()
        .map(crate::report::describe)
        .collect();
    Some(Read {
        interpretation,
        reports,
        object: page,
        replaceable,
    })
}

/// What a document reads after page one, packaged for a thread the host chooses.
///
/// **`CLAUDE.md` principle 2 defers it and `viewer-core`'s rule 4 forbids the core a thread of
/// its own, and this is how both are kept.** §12.3.3's outline and §7.7.3's whole page tree,
/// placed, are what the title bar's section needs and what page one does not: 100.6 M of the
/// 185.1 M instructions a 1023-page open executed before they left it. A host takes this from
/// [`crate::Viewer::preparation`] once page one is on its way, calls [`Self::run`] on any thread —
/// a handle to the immutable file is all it holds — and hands what comes back to
/// [`crate::Viewer::prepared`]. A host that never asks loses nothing but the timing: the first
/// use reads both, on the thread that asked (ADR 1543).
#[derive(Debug)]
pub struct Preparation {
    /// The document this was taken for, so that an answer names its document (trap 104).
    pub(crate) id: crate::DocumentId,
    /// The file, shared with the [`Open`] it came from.
    pub(crate) document: Arc<Document>,
}

impl Preparation {
    /// Reads the outline and places the page tree. The whole of the work, and none of it the
    /// core's state: it touches only the file, so it may run beside anything the core does.
    #[must_use]
    pub fn run(self) -> Prepared {
        let pages = Pages::new(&self.document);
        let outline = Outline::read(&self.document, &pages);
        let indices = pages.indices();
        drop(pages);
        Prepared {
            id: self.id,
            document: self.document,
            outline,
            indices,
        }
    }
}

/// What [`Preparation::run`] read, waiting to be handed to [`crate::Viewer::prepared`].
#[derive(Debug)]
pub struct Prepared {
    /// The document it was read for.
    pub(crate) id: crate::DocumentId,
    /// The file it was read from: an answer for a document since closed, or for another file
    /// opened under the same id, is told apart by this rather than by the id alone.
    pub(crate) document: Arc<Document>,
    /// §12.3.3's outline.
    pub(crate) outline: Outline,
    /// §7.7.3's page tree, placed.
    pub(crate) indices: BTreeMap<ObjectId, usize>,
}

#[cfg(test)]
mod tests {
    use super::magnification::content_box;
    use super::raster_extent;
    use pdf_render::{DisplayList, Point, Rect, Size, TargetSpec};

    /// §12.3.2.2's bounding box is cut to the page, and the clause says so in the sentence
    /// after the one every implementation quotes.
    ///
    /// The display list is not clipped to the box the page is displayed in — `interpret` puts
    /// that box at the origin — so ink outside it reaches `content_bounds` and would otherwise
    /// decide a `/FitB`'s magnification. Three cases: a box inside the page is its own answer,
    /// a box hanging off every side becomes the page's, and a box *entirely* outside leaves
    /// the forms with nothing, which is the same fallback as a page covering nothing.
    #[test]
    fn a_bounding_box_is_cut_to_the_page_it_is_on() {
        let page = Size {
            width: 200.0,
            height: 100.0,
        };
        let box_ = |x0: f32, y0: f32, x1: f32, y1: f32| {
            Some(Rect {
                min: Point::new(x0, y0),
                max: Point::new(x1, y1),
            })
        };
        assert_eq!(
            content_box(box_(10.0, 20.0, 30.0, 40.0), page),
            box_(10.0, 20.0, 30.0, 40.0),
            "a bounding box inside the page is the bounding box"
        );
        assert_eq!(
            content_box(box_(-50.0, -10.0, 260.0, 130.0), page),
            box_(0.0, 0.0, 200.0, 100.0),
            "every side outside the page takes the page's side instead"
        );
        assert_eq!(
            content_box(box_(210.0, 20.0, 260.0, 40.0), page),
            None,
            "ink entirely off the page leaves the /FitB forms the page box"
        );
        assert_eq!(content_box(None, page), None);
    }

    /// The extent this crate computes and the extent a frame is drawn at must be the
    /// same number, at every magnification — not merely rounded the same way.
    ///
    /// They are two arithmetics over the same inputs: `TargetSpec::for_page` multiplies in
    /// `f64` and `ceil`s, and so, now, does [`raster_extent`]. The sweep includes the
    /// magnifications where an `f32` product would land on the wrong side of an integer —
    /// A4's 595.276 is not representable, and 40× of it is where the two used to part —
    /// because everything this crate places over a frame (the focus ring, selection quads,
    /// the scroll clamp) is positioned from the first and drawn over the second. ADR 0118.
    #[test]
    fn the_page_extent_agrees_with_the_target_a_frame_is_drawn_at() {
        let a4 = Size {
            width: 595.276,
            height: 841.89,
        };
        let list = DisplayList::new(a4);
        for magnification in [
            0.02,
            0.333_333_3,
            1.0,
            1.000_000_1,
            1.9008,
            4.0,
            7.3,
            23.15,
            40.0,
            63.999,
            64.0,
        ] {
            let target = TargetSpec::for_page(&list, magnification, u64::MAX)
                .expect("every magnification here is inside MAX_EXTENT");
            assert_eq!(
                raster_extent(a4, magnification),
                (target.width, target.height),
                "at {magnification}x the two arithmetics have to land on one integer"
            );
        }
    }
}
