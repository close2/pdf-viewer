//! One open document, and everything the viewer derives from it.
//!
//! Split from [`crate::Viewer`] because the viewer's own job is the *set* of them and the one
//! viewport they share, while everything here is per document: which page is showing, how large
//! it is drawn, and the pixels that showed it last.

use std::collections::BTreeMap;
use std::sync::Arc;

use pdf_model::content::{Interpretation, Placed};
use pdf_model::fragment::{Fragment, Parameter};
use pdf_model::outline::Outline;
use pdf_model::page_label::PageLabels;
use pdf_model::view::ViewState;
use pdf_model::{Page, Pages};
use pdf_render::{DisplayList, Point, Raster, Rect, Size, TargetSpec};
use pdf_syntax::Document;

use crate::command::Zoom;
use crate::viewer::{RenderToken, px};
use pdf_model::action::ImportData;
use pdf_model::view::Pointer;
use pdf_syntax::ObjectId;

/// The zoom a document opens at, where its `/OpenAction` states none.
///
/// Where one does, [`Open::pending_views`] carries it and `apply_view` replaces this on the first
/// settled frame — which is why this is still a mode rather than a number: a document with no
/// opinion should follow the window, and one with an opinion should be obeyed.
const INITIAL_ZOOM: Zoom = Zoom::FitPage;

/// How far one [`Zoom::In`] or [`Zoom::Out`] step moves.
///
/// A quarter larger each press, which is the step every viewer this project's owner uses has
/// converged on. Nothing derives it; it is a choice, and it is written down as one.
const ZOOM_STEP: f32 = 1.25;

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
    pub(crate) document: Document,
    /// What §12.6.4's actions and §8.11's layer switches have changed since it opened.
    pub(crate) view: ViewState,
    /// §12.4.2's labelling ranges, read once when the document opened.
    ///
    /// Once rather than per page turn: the tree is a handful of ranges and reading it costs one
    /// walk, where doing it per page would put a number-tree walk on every arrow key.
    pub(crate) labels: PageLabels,
    /// §12.3.3's outline, read once for the same reason.
    pub(crate) outline: Outline,
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
    /// invented.** This was a single interpretation for four hundred sessions, with a comment
    /// saying a cache "would also need a bound and an eviction rule, and both should be written
    /// after somebody measures what a display list costs to hold". `/PageLayout` supplies both
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
    /// **And it moves by keyboard as well since the two-hundred-and-ninety-seventh session.**
    /// This comment said "this program has no key that moves between fields, so focus moves by
    /// pointer alone"; `Command::Focused` is that key's message and `pdf_model::tab_order` is
    /// Table 31's `/Tabs`, all five values.
    pub(crate) focus: Option<ObjectId>,
    /// §12.7.6.4's import, waiting for the host to supply the file.
    pub(crate) importing: Option<ImportData>,
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
    /// **The page is here since Table 29's continuous layouts were obeyed**, and it is on *both*
    /// ends since the session after the one that found a drag stopping at a row boundary. Until
    /// an arrangement could show several pages the page a selection belonged to was always the
    /// current one — so a page turn ended a selection and that was the whole rule. A scroll that
    /// crosses a row boundary changes the current page without a person having done anything to
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
    /// Which of §12.5.6.14's popup windows a person has opened or closed, by annotation.
    ///
    /// **The file states only the first frame.** Table 186's `/Open` is "[a] flag specifying
    /// whether the popup annotation shall *initially* be displayed open", so what is here is
    /// every window whose state has since been changed — absent means the file's answer still
    /// stands. That is the same division `ViewState` makes for §12.6.4's actions and the edit log
    /// makes for a field's value: the document says what it says, and what a person did is a log
    /// beside it (`CLAUDE.md`'s rule 1).
    pub(crate) popups: BTreeMap<ObjectId, bool>,
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
    pub(crate) fonts: pdf_model::FontCache,
    /// How many objects lost to a damaged object stream this document has already been told about.
    ///
    /// §7.5.7's losses are discovered when an object inside such a stream is first asked for,
    /// which is *after* the document opened — nothing expands an object stream until something
    /// needs one, and `CLAUDE.md`'s startup rule is why. So the sentence cannot be said once at
    /// open and be true; it is said whenever the count grows, and this is what stops it being
    /// said twice. Host state deliberately: putting it in the page's report would make
    /// `pdf_model::interpret` depend on which pages had been read before it. ADR 0366.
    pub(crate) losses_said: usize,
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
/// **Both ends name a page since the six-hundred-and-ninth session**, and that is the whole of
/// what a selection crossing a page boundary needed in this crate. A range used to be into *one*
/// page's readback, and that half was right: the standard offers no offset a selection could be a
/// range of. ISO 32000-2 §9.4.1 says of the text matrix, the text line matrix and the text
/// rendering matrix that they
///
/// > may be specified only within a text object and shall not persist from one text object to the
/// > next
///
/// so text has no continuous position even between two text objects of *one* page, let alone
/// across pages, and §12.4.2's page indices are the only sequence there is. **What was wrong was
/// the conclusion**: this is a pair of `(page, offset)` rather than one number, and a pair
/// composes across a boundary where a number could not exist. Table 29's continuous arrangements
/// put several pages on the screen at once, so a drag from one to the next is an ordinary gesture
/// rather than an exotic one, and refusing it left the second page's half of a paragraph
/// unselectable.
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
}

/// §7.11.4's file Annex O's `ef` parameter named, and the fragment that belongs to it.
///
/// Two fields rather than two state fields on [`Open`], because they are one instruction: Table
/// Annex O.3 says the processor "shall open the embedded file … identified by name" and then that
/// "[a]ny remaining parameters after this parameter apply to the selected embedded file", so a name
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
    /// §12.5.6.10's markup, over quadrilaterals in **default user space**.
    Markup {
        /// The pages it was added to, in page order, each with the quadrilaterals of its own
        /// part of the selection: `Page::id` and Table 182's `/QuadPoints`, one entry per run of
        /// a line.
        ///
        /// **A list because a selection crosses page boundaries** (the six-hundred-and-ninth
        /// session), and the standard rather than convenience is what makes it a list. ISO 32000-2
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
    /// # Errors
    ///
    /// Whatever `pdf_syntax` says about the file, including §7.6.4.1's
    /// [`pdf_syntax::SyntaxError::PasswordRequired`], which the caller turns into a prompt
    /// rather than into a failure.
    pub(crate) fn new(
        bytes: Vec<u8>,
        password: Option<&crate::Secret>,
    ) -> Result<Self, pdf_syntax::SyntaxError> {
        Ok(Self::around(Document::open_with_password(
            bytes,
            pdf_syntax::Limits::DEFAULT,
            // The one call in this crate that reads a password, which is what
            // `Secret::reveal` is named to make visible at its call site. §7.6.4.1's default
            // user password is the empty string, so a document opened without one asks with it.
            password.map_or("", crate::Secret::reveal),
        )?))
    }

    /// Everything the viewer derives from a document, read once.
    ///
    /// Separate from [`Self::new`] because §12.6.4.4's embedded go-to produces a `Document` that
    /// was never a file: it comes out of §7.11.4's embedded file stream inside the document
    /// already open, and everything below it is the same.
    pub(crate) fn around(document: Document) -> Self {
        let pages = Pages::new(&document);
        let page_count = pages.len();
        let labels = PageLabels::read(&document);
        let outline = Outline::read(&document, &pages);
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
            outline,
            page_count,
            page_index,
            layout,
            zoom: INITIAL_ZOOM,
            pending_views: open_view.into_iter().collect(),
            scroll: (0.0, 0.0),
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
            log: Vec::new(),
            cursor: 0,
            saved_at: 0,
            selection: None,
            pending_selection: None,
            searching: None,
            opening_file: None,
            highlights: Vec::new(),
            popups: BTreeMap::new(),
            readbacks: crate::readback::Readbacks::default(),
            fonts: pdf_model::FontCache::new(),
            losses_said: 0,
        }
    }

    /// Drops everything derived from what the page draws, because the view state moved.
    ///
    /// The one place that says what "the ink is stale" means, and it means two things now rather
    /// than one: the display list of the page showing, and the readback of every page a search
    /// has read. Both are functions of the document *and* [`Self::view`], and §8.11's layer
    /// switch, §12.7.5's field value, §12.5.5's appearance under the pointer and §6.3.2.2's
    /// delegated widgets each move the second of those two.
    ///
    /// **Every page's readback goes, not the page that changed.** A layer switched or a value
    /// typed is a change to the *state*, and the state is what every cached page was interpreted
    /// against; the precise alternative — forgetting only the page an annotation sits on — needs
    /// an invariant about which page that is, maintained at two call sites, and a search that
    /// returned different results after this change would be a defect rather than a speed-up.
    /// The cost is that a layer switch makes the next search cold again, which is written down
    /// in ADR 0256 as the deliberate half of the trade.
    /// **And it is where [`Self::ink`] moves**, for the same reason it is where the display lists
    /// go: a host holding pixels of a page cannot tell a re-interpretation of unchanged ink from
    /// an interpretation of changed ink, and this is the only place in this crate that knows.
    pub(crate) fn stale(&mut self) {
        self.ink = self.ink.saturating_add(1);
        for on_screen in &mut self.on_screen {
            on_screen.interpreted = None;
        }
        self.readbacks.clear();
    }

    /// The arrangement's entry for a page, where the arrangement shows it.
    pub(crate) fn on(&self, page: usize) -> Option<&OnScreen> {
        self.on_screen
            .iter()
            .find(|on_screen| on_screen.page == page)
    }

    /// The same, mutably.
    pub(crate) fn on_mut(&mut self, page: usize) -> Option<&mut OnScreen> {
        self.on_screen
            .iter_mut()
            .find(|on_screen| on_screen.page == page)
    }

    /// The current page's interpretation, where it has one.
    ///
    /// What was one field until Table 29's arrangements arrived. Every question this crate
    /// answers about "the page being shown" comes through here, so that *which* page that is
    /// stays one decision rather than one per caller.
    pub(crate) fn interpreted(&self) -> Option<&Interpreted> {
        self.on(self.page_index)?.interpreted.as_ref()
    }

    /// The interpretation of any page the arrangement shows.
    pub(crate) fn interpretation(&self, page: usize) -> Option<&Interpreted> {
        self.on(page)?.interpreted.as_ref()
    }

    /// What is selected, as a range of each page's own readback, in page order.
    ///
    /// **This is where a selection stops being two points and becomes text**, and it is one
    /// function so that the text, the shapes, §14.8.2.5's logical order and §12.5.6.10's markup
    /// cannot disagree about what was chosen. The first page runs from the earlier end to *its*
    /// end, the last from *its* beginning to the later end, and a page between the two is
    /// selected whole — which is what dragging across a boundary means and is the only reading a
    /// column offers.
    ///
    /// Empty where nothing is selected. A page whose interpretation this crate does not hold
    /// contributes nothing rather than an empty range: it can only be a page the arrangement is
    /// not showing, and a selection whose pages have left the screen is dropped by `settle`
    /// before it can be asked about.
    pub(crate) fn spans(&self) -> Vec<(usize, (usize, usize))> {
        let Some(chosen) = self.selection else {
            return Vec::new();
        };
        let (from, to) = chosen.ordered();
        (from.page..=to.page)
            .filter_map(|page| {
                let readback = self.interpretation(page)?.text.len();
                let start = if page == from.page {
                    from.offset.min(readback)
                } else {
                    0
                };
                let end = if page == to.page {
                    to.offset.min(readback)
                } else {
                    readback
                };
                (start <= end).then_some((page, (start, end)))
            })
            .collect()
    }

    /// Which page of the arrangement a viewport point is inside, where it is inside one.
    ///
    /// `None` for a point in the gap between two pages, in the margin beside them, or over
    /// nothing at all — a caller that must answer anyway falls back to the current page and says
    /// so, because a drag that has left the page is still a drag inside its text.
    pub(crate) fn placed_at(&self, at: (f32, f32)) -> Option<&OnScreen> {
        self.on_screen.iter().find(|on_screen| {
            at.0 >= on_screen.origin.0
                && at.0 < on_screen.origin.0 + px(on_screen.raster.0)
                && at.1 >= on_screen.origin.1
                && at.1 < on_screen.origin.1 + px(on_screen.raster.1)
        })
    }

    /// How many pages there are now, which §12.7.8.3.3's imported templates may have changed.
    pub(crate) fn recount(&mut self) {
        self.page_count = Pages::new(&self.document)
            .len()
            .saturating_add(self.view.appended_pages().len());
    }

    /// The page at `index`, counting §12.7.8.3.3's imported template pages after the
    /// document's own.
    ///
    /// §12.7.7's template pages are objects of this document that the page *tree* does not
    /// reach — the clause puts them in a name tree precisely so that they are not displayed
    /// until something asks — so showing one means building it without the inheritance a tree
    /// would have given it, which [`Pages::detached`] is. Their position is this program's
    /// choice: §12.7.8.3.3 says a template page is added to the document and states no place,
    /// and after the document's own pages is the only order that leaves every existing page
    /// index meaning what it meant.
    pub(crate) fn page(&self, index: usize) -> Option<Page> {
        let pages = Pages::new(&self.document);
        if let Some(page) = pages.get(index) {
            return Some(page);
        }
        let appended = index.checked_sub(pages.len())?;
        let object = self
            .document
            .get(*self.view.appended_pages().get(appended)?);
        Some(pages.detached(object.as_dict()?))
    }

    /// Turns what a host asked for into what was done, or `None` where nothing was.
    ///
    /// The one place [`crate::Edit`] and [`Done`] meet. A field's value passes through unchanged;
    /// a markup is resolved *here*, against the selection and the page as they are now, because
    /// the log has to record geometry rather than an intention — see [`Done`].
    ///
    /// `None` for a markup with nothing selected, with no page interpreted, or on a page reached
    /// without an identity ([`Pages::detached`]'s, which is §12.7.7's template): each is a
    /// request this crate cannot turn into an annotation, and adding an empty one would be worse
    /// than doing nothing.
    ///
    /// `drag` is [`crate::Edit::FreeText`]'s two corners, **already in default user space**, and
    /// it is a parameter rather than something read here because the map from the viewport is the
    /// *viewer's*: it needs the viewport's size and the display's scale, which are properties of
    /// the window and not of an open document. Everything else a resolution needs — the page, the
    /// selection, the page transform — is this type's.
    pub(crate) fn resolve(
        &self,
        edit: crate::command::Edit,
        drag: Option<[(f32, f32); 2]>,
    ) -> Option<Done> {
        match edit {
            crate::command::Edit::SetField { field, value } => {
                Some(Done::SetField { field, value })
            }
            crate::command::Edit::SetFreeText { annotation, text } => {
                Some(Done::SetFreeText { annotation, text })
            }
            crate::command::Edit::FreeText { colour, .. } => {
                let [from, to] = drag?;
                let page = self.page(self.page_index)?;
                Some(Done::FreeText {
                    page: page.id?,
                    rect: [from.0, from.1, to.0, to.1],
                    colour,
                })
            }
            crate::command::Edit::Markup { kind, colour } => {
                // The *selection's* pages rather than the current one: a mark-up is over what is
                // selected, and Table 29's continuous arrangements let a person scroll on after
                // selecting without having changed what they chose — and select across a row
                // boundary in the first place, which is why this is a page at a time.
                let mut marked: Vec<(ObjectId, Vec<[f32; 8]>)> = Vec::new();
                for (index, range) in self.spans() {
                    let Some(interpreted) = self.interpretation(index) else {
                        continue;
                    };
                    let quads = crate::select::quads_for(&interpreted.placed, range);
                    let page = self.page(index)?;
                    // §12.5.6.10's `/QuadPoints` is in default user space and everything this
                    // crate answers with is in the display list's, so the shapes go back through
                    // the transform that put them there. A page transform is a similarity with a
                    // flip and is always invertible; the `?` is the type system's rather than a
                    // case.
                    let back = pdf_model::content::page_transform(&page).invert()?;
                    let quads = quads
                        .into_iter()
                        .map(|quad| {
                            let mut mapped = [0.0; 8];
                            for (corner, out) in
                                quad.chunks_exact(2).zip(mapped.chunks_exact_mut(2))
                            {
                                let point = back.apply(Point {
                                    x: corner[0],
                                    y: corner[1],
                                });
                                out[0] = point.x;
                                out[1] = point.y;
                            }
                            mapped
                        })
                        .collect::<Vec<[f32; 8]>>();
                    if !quads.is_empty() {
                        marked.push((page.id?, quads));
                    }
                }
                (!marked.is_empty()).then_some(Done::Markup {
                    pages: marked,
                    kind,
                    colour,
                })
            }
        }
    }

    /// Replays the log up to the cursor onto a state that has none of it applied.
    ///
    /// Undo and redo are *replays* rather than inverses, and that is the decision: an inverse
    /// would have to remember what each edit replaced — which for a field with no `/V` is "no
    /// value at all", a state distinct from every value — and would drift from the log the moment
    /// two edits touched one field. Replaying costs one pass over the log per undo, and the log
    /// is what a person did in one sitting.
    pub(crate) fn replay(&mut self) {
        let log = std::mem::take(&mut self.log);
        self.view.clear_all_fields();
        self.view.clear_all_additions();
        self.view.clear_all_free_text();
        for edit in log.iter().take(self.cursor) {
            match edit {
                Done::SetField { field, value } => {
                    self.view.set_field(&self.document, field, value);
                }
                Done::Markup {
                    pages,
                    kind,
                    colour,
                } => {
                    for (page, quads) in pages {
                        self.view
                            .add_markup(&self.document, *page, *kind, *colour, quads);
                    }
                }
                Done::FreeText { page, rect, colour } => {
                    self.view
                        .add_free_text(&self.document, *page, *rect, "", *colour);
                }
                Done::SetFreeText { annotation, text } => {
                    self.view.set_free_text(&self.document, *annotation, text);
                }
            }
        }
        self.log = log;
        // The page's ink depends on the values, so the display list and every readback are stale.
        self.stale();
    }

    /// Whether anything a person did is unsaved.
    ///
    /// **The cursor's distance from the last save, not its distance from zero.** This asked
    /// `cursor > 0` until the five-hundred-and-twenty-fifth session, so a document went on
    /// saying it had unsaved work for as long as it stayed open after being saved —
    /// `Event::Dirty` never came back false and `viewer-ui`'s title kept its mark. The model
    /// holds the other half of the answer in `ViewState::additions` and `ViewState::edits`,
    /// which is what `doc/todo/01`'s fifth sweep found no caller for: the question a host asks
    /// is *what is unwritten*, and a log's length is what was **done**.
    pub(crate) fn dirty(&self) -> bool {
        self.cursor != self.saved_at
    }

    /// Records that everything up to the cursor has been written to a file.
    pub(crate) fn saved(&mut self) {
        self.saved_at = self.cursor;
    }

    /// The text position a point in one page's display-list coordinates selects.
    pub(crate) fn position_at(&self, page: usize, point: (f32, f32)) -> Option<usize> {
        let interpreted = self.interpretation(page)?;
        crate::select::position_at(&interpreted.placed, point)
    }

    /// The page's extent in user space units, after §7.7.3.3's rotation and `/UserUnit`.
    ///
    /// Read from the page rather than from a display list, because the zoom has to be decided
    /// before there is one: fitting a page to a window is what says how large to interpret it
    /// for, and asking the other way round would interpret every page twice.
    ///
    /// For the page already interpreted the display list carries the same extent, and answering
    /// from it saves a walk of the page tree — which is what the magnification, the geometry and
    /// every mapping of a pointer position ask for.
    pub(crate) fn page_size(&self, index: usize) -> Option<Size> {
        if let Some(on_screen) = self.on(index) {
            if let Some(interpreted) = on_screen.interpreted.as_ref() {
                return Some(interpreted.list.page_size);
            }
            return Some(pdf_model::content::displayed_size(&on_screen.object));
        }
        self.page(index)
            .map(|page| pdf_model::content::displayed_size(&page))
    }

    /// Whether one of §12.5.6.14's windows is open now.
    ///
    /// Table 186's `/Open` answers for a window nobody has touched — "shall *initially* be
    /// displayed open" — and [`Self::popups`] answers for every one somebody has.
    pub(crate) fn popup_is_open(&self, popup: &pdf_model::popup::Popup) -> bool {
        self.popups
            .get(&popup.annotation)
            .copied()
            .unwrap_or(popup.open)
    }

    /// The activation §12.5.1 describes, for an annotation that has one of §12.5.6.14's windows.
    ///
    /// ISO 32000-2 §12.5.1:
    ///
    /// > When the user activates the annotation by clicking it, it exhibits its associated
    /// > object, such as by opening a popup window displaying a text note
    ///
    /// The clause says *exhibits*, not *toggles*: what a second click does is not in the standard,
    /// and closing the window is the choice every reader of a sticky note makes. Recorded as a
    /// choice. Returns whether anything changed, which is what tells the caller to repaint.
    pub(crate) fn toggle_popup(&mut self, annotation: ObjectId) -> bool {
        let Some(page) = self.shown_page() else {
            return false;
        };
        let object = self.document.get(annotation);
        let Some(dict) = object.as_dict() else {
            return false;
        };
        // Either end of Table 172's `/Popup`: a click lands on the *markup* annotation, and a
        // host with a close button on the window has the popup itself.
        let popup = pdf_model::popup::popup_of(&self.document, dict).unwrap_or(annotation);
        let Some(state) = pdf_model::popup::popups(&self.document, page, &self.view)
            .iter()
            .find(|window| window.annotation == popup)
            .map(|window| self.popup_is_open(window))
        else {
            return false;
        };
        self.popups.insert(popup, !state);
        true
    }

    /// The current page, without walking the tree for it again.
    pub(crate) fn shown_page(&self) -> Option<&Page> {
        self.placed_page(self.page_index)
    }

    /// Any page the arrangement shows, without walking the tree for it again.
    pub(crate) fn placed_page(&self, page: usize) -> Option<&Page> {
        self.on(page).map(|on_screen| &on_screen.object)
    }

    /// Device pixels per user space unit for the page showing, given the viewport.
    ///
    /// `None` for a page that cannot be read or has no extent — a zero-sized crop box, which
    /// Table 30 does not forbid and a fuzzer produces immediately.
    pub(crate) fn magnification(&self, viewport: (u32, u32), scale: f32) -> Option<f32> {
        let size = self.page_size(self.page_index)?;
        if size.width <= 0.0 || size.height <= 0.0 || scale <= 0.0 {
            return None;
        }
        // A fit is worked out in *device* pixels and never divided back into logical ones: the
        // round trip through `/ scale` and `* scale` is not the identity in `f32`, and this is
        // the one place where a pixel of error becomes a scrollbar.
        let magnification = match self.zoom {
            Zoom::FitPage => fitted(viewport.0, size.width).min(fitted(viewport.1, size.height)),
            Zoom::FitWidth => fitted(viewport.0, size.width),
            Zoom::FitHeight => fitted(viewport.1, size.height),
            Zoom::Scale(zoom) => zoom.clamp(ZOOM_RANGE.0, ZOOM_RANGE.1) * scale,
            // Neither reaches here: `Viewer` resolves a step into the scale it lands on
            // before storing it, because a chain of steps has to compose and a mode cannot.
            Zoom::In | Zoom::Out => return None,
        };
        (magnification.is_finite() && magnification > 0.0).then_some(magnification)
    }

    /// The magnification one step in or out from where this is now.
    pub(crate) fn stepped(current: f32, direction: Zoom) -> f32 {
        let stepped = match direction {
            Zoom::In => current * ZOOM_STEP,
            Zoom::Out => current / ZOOM_STEP,
            Zoom::FitPage | Zoom::FitWidth | Zoom::FitHeight | Zoom::Scale(_) => current,
        };
        stepped.clamp(ZOOM_RANGE.0, ZOOM_RANGE.1)
    }

    /// Where the current page's raster sits in the viewport, in device pixels.
    ///
    /// **The arrangement decides this since Table 29's layouts were obeyed**, and it is one call
    /// rather than an arithmetic here so that a page standing beside or below another is placed
    /// by the same code that places the current one — ADR 0118's rule about a second opinion.
    /// Under `SinglePage` the answer is what it always was: centred where the page is smaller
    /// than the viewport and scrolled where it is larger.
    pub(crate) fn origin(
        &self,
        viewport: (u32, u32),
        scale: f32,
        magnification: f32,
    ) -> (f32, f32) {
        crate::layout::origin_of(self, self.page_index, viewport, scale, magnification)
            .unwrap_or((0.0, 0.0))
    }

    /// Scrolls so that a point of the viewport keeps the point of the page it was over.
    ///
    /// `before` and `after` are magnifications and `at` is a viewport point in device pixels.
    /// The page point under `at` is `at - origin` in the old raster's coordinates; it is that
    /// same fraction of the new raster, so scaling it by the ratio and putting it back under
    /// `at` is the whole of it. **Through `origin` rather than through the scroll**, because a
    /// page smaller than the viewport is *centred* and its scroll is zero — the arithmetic that
    /// reads the scroll alone is right only while there is something to scroll, and a wheel is
    /// pointed at pages of both kinds.
    ///
    /// The scroll it produces may exceed what the new raster permits; `Viewer::settle` clamps it
    /// against the target it is about to ask for, which is where the new raster's size is known.
    pub(crate) fn hold(
        &mut self,
        viewport: (u32, u32),
        scale: f32,
        before: f32,
        after: f32,
        at: Option<(f32, f32)>,
    ) {
        let origin = self.origin(viewport, scale, before);
        let at = at.unwrap_or((px(viewport.0) / 2.0, px(viewport.1) / 2.0));
        let ratio = after / before;
        let hold = |at: f32, origin: f32| ((at - origin) * ratio - at).max(0.0);
        self.scroll = (hold(at.0, origin.0), hold(at.1, origin.1));
    }

    /// Carries out ISO 32000-2 Annex O's fragment identifier, and names what it could not.
    ///
    /// The returned strings are for a person: every parameter this program refuses is refused *by
    /// name*, because a URI that says `#page=5` and opens page one has told its reader something
    /// false about the document. Nothing here fails and nothing here refuses to open the file.
    ///
    /// **In the fragment's own order**, which §O.2 makes a requirement rather than a convenience:
    ///
    /// > Fragment identifiers shall be processed and (if required) executed from left to right as
    /// > they appear in the character string that makes up the fragment identifier.
    ///
    /// and the `comment` parameter is what turns that into behaviour — it names an annotation on
    /// *the page selected so far*, so `page=3&comment=x` and `comment=x&page=3` mean different
    /// things and the annex says so in a NOTE.
    ///
    /// Called immediately after Table 29's `/OpenAction`, which is where §O.2.2 puts it: these
    /// "should be processed immediately after any other document-specified open parameters have
    /// been processed". A page and a view stated by both is the document's opinion overruled by
    /// the URI's, which is the order that makes a fragment identifier useful.
    ///
    /// # Where the coordinates are
    ///
    /// §O.2.2 states one rule for its own three rectangles and the rule has two halves that pull
    /// apart:
    ///
    /// > All coordinate values … shall be expressed in the default user space coordinate system.
    ///
    /// while each parameter's own row measures from "the top left corner of the page". Default
    /// user space has its origin at the *bottom* left (§8.3.2.3), so the two sentences together
    /// say: the *units* are default user space's, and the origin is the page's top-left corner.
    /// That is the reading this takes — `top` counts downward from `display_box`'s upper edge and
    /// `left` rightward from its left one — and it is the only one under which both sentences are
    /// true at once. `display_box` rather than the media box because §12.2's `/ViewArea` decides
    /// what "the page" is on a screen, and it is the crop box for every document that states none.
    ///
    /// `view`, alone among them, is *not* measured that way: its arguments "shall correspond to
    /// those found in 12.3.2.2", which are default user space's own, origin and all.
    pub(crate) fn apply_fragment(&mut self, fragment: &Fragment) -> Vec<String> {
        let mut notes = Vec::new();
        for parameter in &fragment.parameters {
            if let Some(reason) = parameter.unhonoured() {
                notes.push(format!(
                    "this URI's fragment asks for `{}`, which this program does not do: {reason}",
                    parameter.name()
                ));
                continue;
            }
            self.apply_parameter(parameter, &mut notes);
            // "Any remaining parameters after this parameter apply to the selected embedded
            // file." So everything after `ef` is about a document this program has not opened,
            // and applying it to *this* one would take a person somewhere the URI did not name.
            // `Fragment::parse` has already stopped reading there and kept the remainder whole;
            // it travels with the file as `Event::Extracted`'s fragment, for the host that opens
            // those bytes as a document to hand straight back as `Command::Open`'s (ADR 0431).
            // The loop still stops, because a `Fragment` this crate did not parse could carry
            // parameters here and applying them would be the wrong document's instruction.
            if matches!(parameter, Parameter::EmbeddedFile(_)) {
                if let Some(rest) = fragment.after_embedded_file.clone() {
                    notes.push(format!(
                        "and the fragment continues `{rest}` after it, which applies to that \
                         embedded file rather than to this document and travels with it"
                    ));
                    if let Some(file) = self.opening_file.as_mut() {
                        file.fragment = Some(rest);
                    }
                }
                break;
            }
        }
        for unread in &fragment.unread {
            notes.push(format!(
                "this URI's fragment says `{}`, which is {}",
                unread.name,
                unread.reason.as_str()
            ));
        }
        notes
    }

    /// One of Annex O's parameters, applied. Split from [`Self::apply_fragment`] for its length.
    fn apply_parameter(&mut self, parameter: &Parameter, notes: &mut Vec<String>) {
        match parameter {
            // Table Annex O.3: "The first page in the document has a pageNum value of 1", against
            // this crate's own zero-based index.
            Parameter::Page(number) => {
                let index = usize::try_from(number.saturating_sub(1)).unwrap_or(usize::MAX);
                if index < self.page_count {
                    self.go_to(index);
                } else {
                    notes.push(format!(
                        "this URI's fragment asks for page {number} and this document has {}",
                        self.page_count
                    ));
                }
            }
            Parameter::NamedDestination(name) => self.go_to_named(name, notes),
            Parameter::StructureElement(id) => self.go_to_structure_element(id, notes),
            // Table 166's `/NM`, on the page chosen so far — which is what makes this parameter's
            // position in the fragment matter.
            Parameter::Comment(name) => {
                let found = self.page(self.page_index).and_then(|page| {
                    pdf_model::fragment::annotation_named(&self.document, &page, name)
                });
                if let Some(id) = found {
                    // "[T]he PDF processor shall open the document with the specified comment
                    // selected", and the annex says nothing about what selecting one looks like.
                    // This program has one way of indicating an annotation without changing the
                    // page — §12.5.1's focus, which is what the tab key moves and what a host
                    // draws a ring around — so that is what a selected comment is here. A choice,
                    // and written down as one.
                    self.focus = Some(id);
                } else {
                    notes.push(format!(
                        "this URI's fragment names the comment {}, which page {} does not carry",
                        text(name),
                        self.page_index.saturating_add(1)
                    ));
                }
            }
            // Table Annex O.4's `zoom` is Table 149's `/XYZ` in every respect but two: it states
            // a percentage where `/XYZ` states a factor, and it measures its corner from the top
            // left of the page where `/XYZ` measures from default user space's own origin.
            Parameter::Zoom { percent, offset } => {
                let corner = offset.map(|(left, top)| self.in_default_user_space(left, top));
                self.pending_views.push(pdf_model::destination::View::Xyz {
                    left: corner.map(|(left, _)| left),
                    top: corner.map(|(_, top)| top),
                    // Table 149 makes a zoom of zero the same as none, and this is the same
                    // magnification stated as a percentage, so zero says "leave it alone" here
                    // too rather than magnifying a page to nothing.
                    zoom: Some(percent / 100.0).filter(|zoom| *zoom > 0.0),
                });
            }
            Parameter::View(view) => self.pending_views.push(*view),
            // "Open the document with the specified window view rectangle", which is what Table
            // 149's `/FitR` already means — "magnified just enough to fit the rectangle" — so the
            // rectangle is converted to that clause's corners and nothing new is invented.
            Parameter::ViewRect {
                left,
                top,
                width,
                height,
            } => {
                let (x, y) = self.in_default_user_space(*left, *top);
                self.pending_views.push(pdf_model::destination::View::FitR {
                    rect: [x, y - height, x + width, y],
                });
            }
            // Table Annex O.4: "Open the document and search for one or more words, selecting the
            // first matching word in the document."
            //
            // **Started here and finished by the host**, which is the same division every other
            // unit of work in this crate takes: `Event::NeedsRender` is a page this crate has
            // interpreted and not drawn, and this is a search it has planned and not run. The
            // alternative would be reading up to 1023 pages before `Command::Open` returns —
            // 5.84 s on the standard's own file, on the launch path, which `CLAUDE.md`'s startup
            // rules forbid outright.
            //
            // "In the document", so it begins at the first page and does not wrap: a match before
            // where the fragment's other parameters left the view is still the first one in the
            // document, and stopping at the end is what makes "first" mean anything.
            Parameter::Search(words) => {
                let needles: Vec<String> = words
                    .iter()
                    // Lossy for the reason [`text`] is: the annex gives its byte strings no
                    // character encoding, and a needle is compared against a readback that is
                    // already Unicode. A word whose bytes are not UTF-8 searches for what a
                    // person would see if they typed it.
                    .map(|word| String::from_utf8_lossy(word).into_owned())
                    .collect();
                let spelt = needles.join(" ");
                match crate::search::Searching::new(
                    needles,
                    crate::command::FindDirection::Forward,
                    (0, 0),
                    self.page_count,
                    false,
                ) {
                    Some(searching) => {
                        notes.push(format!(
                            "this URI's fragment asks for a search for `{spelt}`, which is \
                             running: {} page(s) to read",
                            searching.remaining
                        ));
                        self.searching = Some(searching);
                    }
                    None => notes.push(format!(
                        "this URI's fragment asks for a search for `{spelt}`, which names nothing \
                         to look for in a document of {} page(s)",
                        self.page_count
                    )),
                }
            }
            // Table Annex O.3, and the annex's one `shall` on a processor that is not about where
            // the window points:
            //
            // > When used as part of a PDF open parameter, the PDF processor shall open the
            // > embedded file contained within the EmbeddedFiles name tree identified by name .
            //
            // §7.11.4's bytes are inside the document, so nothing is fetched; what *opening* one
            // means is the host's, and the annex says so itself two sentences later —
            // "[s]ecurity should be strongly considered when opening an embedded file … a PDF
            // processor may choose to prompt the user or even prevent opening of the file", which
            // is a `should` and a `may` addressed to whoever faces the person. So this records the
            // name and `Viewer::open` hands the bytes over as `Event::Extracted`, where
            // `Command::Extract` and §12.5.6.15's annotation already put them (ADR 0310).
            //
            // Lossy for the reason [`text`] is, and for the same comparison `annotation_named`
            // makes: the annex gives its byte string no character encoding and §7.7.4's tree key
            // states one, so the match is against §7.9.2's *decoded* text string as UTF-8.
            Parameter::EmbeddedFile(name) => {
                self.opening_file = Some(OpeningFile {
                    name: String::from_utf8_lossy(name).into_owned(),
                    // Filled by [`Self::apply_fragment`], which is the only place the *rest* of
                    // the fragment is in scope.
                    fragment: None,
                });
            }
            // Table Annex O.4's `highlight` and `fdf`, each in a method of its own for the
            // reason `nameddest` and `structelem` are: the reading is longer than the arm.
            Parameter::Highlight {
                left,
                right,
                top,
                bottom,
            } => self.highlight(*left, *right, *top, *bottom),
            Parameter::Fdf(uri) => self.import_from(uri, notes),
        }
    }

    /// Annex O's `highlight`, kept until a host asks where it is on the screen.
    ///
    /// Table Annex O.4: "Open the document with the specified rectangle highlighted. Each argument
    /// shall be an integer or floating point value representing the rectangle measured from the top
    /// left corner of the page. The nature of the highlighting is implementation-dependent."
    ///
    /// **Not the selection**, and the annex is what says so: the two neighbouring parameters that
    /// point a person at something both say *selected* — `comment`'s "with the specified comment
    /// selected", `search`'s "selecting the first matching word" — and this one says *highlighted*,
    /// with a nature left to the processor that the other two are not given. So it is a shape of
    /// its own, kept here and answered by `Query::Highlight`; what it then looks like is the
    /// host's, which is exactly what "implementation-dependent" leaves open and what this boundary
    /// already does with every other quadrilateral it hands over (ADR 0357).
    ///
    /// The corners go through the same conversion `zoom` and `viewrect` take, because the clause
    /// states one rule for all three: default user space's units with the page's top-left corner
    /// as the origin.
    fn highlight(&mut self, left: f32, right: f32, top: f32, bottom: f32) {
        let (x0, y0) = self.in_default_user_space(left, top);
        let (x1, y1) = self.in_default_user_space(right, bottom);
        // Normalised, because the annex names four edges and states no order between them: a URI
        // that writes its bottom above its top has still named a rectangle.
        self.highlights.push(Highlighted {
            page: self.page_index,
            rect: [x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)],
        });
    }

    /// Annex O's `fdf`: §12.7.8's form data, asked of the host that owns the filesystem.
    ///
    /// Table Annex O.4: "Open the document and then import the data from the specified FDF or XFDF
    /// file. The URI shall be either a relative or absolute URI to an FDF or XFDF file."
    ///
    /// **The same request §12.7.6.4's action makes**, and it is made the same way: this crate has
    /// no filesystem (`doc/ui-boundary.md`'s rule 2), so the name crosses as `Event::NeedsFile`
    /// with the purpose the bytes are wanted for and a host resolves it — against the document's
    /// own URI for a relative one, and by whatever policy it has for an absolute one. That division
    /// is the annex's too: §O.1 defines these as a *URI's* parameters, and resolving a URI
    /// reference is the business of whoever holds the base.
    ///
    /// The format is the file name's, read once for both clauses by
    /// [`pdf_model::action::data_format`]. ISO 19444-1's XFDF is the same data in XML and would
    /// need an XML parser, which is a dependency rather than a clause — so it is declined by name
    /// here exactly as `crate::interact::request_file` declines it for the action.
    fn import_from(&mut self, uri: &[u8], notes: &mut Vec<String>) {
        // Lossy for the reason [`text`] is: the annex states no character encoding for the
        // argument, and what a host resolves is a name a person typed.
        let file = String::from_utf8_lossy(uri).into_owned();
        let format = pdf_model::action::data_format(&file);
        if format == pdf_model::action::DataFormat::Fdf {
            notes.push(format!(
                "this URI's fragment asks for the form data in {file}, which the host is being \
                 asked for"
            ));
            self.importing = Some(ImportData { file, format });
        } else {
            notes.push(format!(
                "this URI's fragment asks for the form data in {file}, which is not §12.7.8's \
                 FDF, and no other data format is read"
            ));
        }
    }

    /// Annex O's `nameddest`, through §12.3.2.4's lookup.
    ///
    /// Both of the tables that clause defines are asked, in the order it introduces them, which is
    /// what `Destination::read` does for a name or a string alike.
    fn go_to_named(&mut self, name: &[u8], notes: &mut Vec<String>) {
        let destination = pdf_model::destination::Destination::read(
            &self.document,
            &pdf_syntax::Object::String(name.into()),
        );
        let Some(destination) = destination else {
            notes.push(format!(
                "this URI's fragment names the destination {}, which this document does not \
                 define",
                text(name)
            ));
            return;
        };
        let index = {
            let pages = Pages::new(&self.document);
            destination.page_index(&self.document, &pages)
        };
        if let Some(index) = index.filter(|index| *index < self.page_count) {
            self.go_to(index);
        } else {
            // A destination naming a page in another file (§12.3.2.2's remote form) or naming
            // nothing. Its view still stands: it says how to look at whatever page is showing.
            notes.push(format!(
                "this URI's fragment names the destination {}, which does not name a page of \
                 this document",
                text(name)
            ));
        }
        self.pending_views.push(destination.view);
    }

    /// Annex O's `structelem`: §14.7.2's `/IDTree` for the element, §12.3.2.3's algorithm for its
    /// page.
    fn go_to_structure_element(&mut self, id: &[u8], notes: &mut Vec<String>) {
        let index = pdf_model::structure::Tree::of(&self.document)
            .and_then(|tree| tree.element_by_id(&self.document, id))
            .and_then(|element| {
                let pages = Pages::new(&self.document);
                pdf_model::destination::structure_element_page(&self.document, &element, &pages)
            });
        if let Some(index) = index.filter(|index| *index < self.page_count) {
            self.go_to(index);
            return;
        }
        // The annex states this outcome rather than leaving it open: "[i]f no content is
        // contained within the hierarchy of the structure element or structID does not match a
        // structure element, the first page in the document shall be identified." So page one is
        // the answer and not a failure — said out loud because a person who typed an identifier
        // wants to know that it matched nothing.
        self.go_to(0);
        notes.push(format!(
            "this URI's fragment names the structure element {}, which identifies no page here, \
             so the first page is shown",
            text(id)
        ));
    }

    /// Shows a page named by a fragment, with the state a page turn resets.
    fn go_to(&mut self, index: usize) {
        if index == self.page_index {
            return;
        }
        self.page_index = index;
        self.scroll = (0.0, 0.0);
        self.shown_for = 0.0;
    }

    /// Annex O's corner — measured from the page's top left — as a point in default user space.
    ///
    /// The page's own upper-left corner is `display_box`'s left edge and its *top* edge, and the
    /// axis runs downward from there while default user space's runs up.
    fn in_default_user_space(&self, left: f32, top: f32) -> (f32, f32) {
        let box_ = self
            .page(self.page_index)
            .map_or([0.0, 0.0, 0.0, 0.0], |page| page.display_box);
        (box_[0] + left, box_[3] - top)
    }

    /// Applies §12.3.2.1's other two items — where the window sits, and how large.
    ///
    /// Table 149's eight forms, and every one of them is two decisions: a **magnification**,
    /// which becomes a [`Zoom`], and a **point of the page put at the window's top-left corner**,
    /// which becomes a scroll. The clause states them in default user space and this works in
    /// device pixels of a raster, so each coordinate goes through the page's own transform
    /// (§7.7.3.3's rotation, the crop box's origin, `/UserUnit`) and then through the
    /// magnification, with the y axis flipped once on the way — a raster's rows count down from
    /// the top and PDF's coordinates count up from the bottom.
    ///
    /// §12.3.2.2, of Table 149's own null:
    ///
    /// > A null value for any of the parameters left , top , or zoom specifies that the current
    /// > value of that parameter shall be retained unchanged.
    ///
    /// So an absent coordinate leaves the scroll where it is and an absent zoom leaves the
    /// magnification, which is why each is applied separately rather than as a whole position.
    ///
    /// `bounds` is what the page's contents cover, in the display list's own space, and is only
    /// consulted by the three `/FitB` forms — "the smallest rectangle enclosing all of its
    /// contents", which no page dictionary states and only a display list can answer. **It is
    /// bounded by the page**, which is the same clause's parenthesis:
    ///
    /// > (If any side of the bounding box lies outside the page's crop box, the corresponding
    /// > side of the crop box shall be used instead; see 14.11.2, "Page boundaries" for further
    /// > discussion of the crop box.)
    ///
    /// Returns whether anything changed, which is what decides a redraw.
    pub(crate) fn apply_view(
        &mut self,
        view: pdf_model::destination::View,
        viewport: (u32, u32),
        scale: f32,
        bounds: Option<Rect>,
    ) -> bool {
        use pdf_model::destination::View;

        let Some(size) = self.page_size(self.page_index) else {
            return false;
        };
        let before = (self.zoom, self.scroll);
        // The box each form fits: the page for five of them, its contents for the `/FitB`
        // family. A `/FitB` on a page whose contents cover nothing falls back to the page,
        // because the alternative is a magnification of infinity.
        let content = content_box(bounds, size);
        let (fit_width, fit_height) = match view {
            View::FitB | View::FitBH { .. } | View::FitBV { .. } => content
                .map_or((size.width, size.height), |box_| {
                    (box_.max.x - box_.min.x, box_.max.y - box_.min.y)
                }),
            _ => (size.width, size.height),
        };

        match view {
            View::Xyz { zoom, .. } => {
                if let Some(zoom) = zoom.filter(|zoom| zoom.is_finite() && *zoom > 0.0) {
                    self.zoom = Zoom::Scale(zoom);
                }
            }
            View::Fit | View::FitB => {
                self.zoom = Zoom::Scale(
                    fitted(viewport.0, fit_width).min(fitted(viewport.1, fit_height)) / scale,
                );
            }
            View::FitH { .. } | View::FitBH { .. } => {
                self.zoom = Zoom::Scale(fitted(viewport.0, fit_width) / scale);
            }
            View::FitV { .. } | View::FitBV { .. } => {
                self.zoom = Zoom::Scale(fitted(viewport.1, fit_height) / scale);
            }
            View::FitR { rect } => {
                let (width, height) = ((rect[2] - rect[0]).abs(), (rect[3] - rect[1]).abs());
                // Table 149 says "magnified just enough to fit *the rectangle*", so a rectangle
                // with no extent in one direction states no magnification in it and the other
                // one decides alone. Neither: the file has stated a point, and the zoom stands.
                self.zoom = match (width > 0.0, height > 0.0) {
                    (true, true) => Zoom::Scale(
                        fitted(viewport.0, width).min(fitted(viewport.1, height)) / scale,
                    ),
                    (true, false) => Zoom::Scale(fitted(viewport.0, width) / scale),
                    (false, true) => Zoom::Scale(fitted(viewport.1, height) / scale),
                    (false, false) => self.zoom,
                };
            }
        }

        // The magnification the new zoom lands on, which is what turns a user-space coordinate
        // into a device one. Read back rather than remembered: `Zoom::Scale` is clamped.
        let Some(magnification) = self.magnification(viewport, scale) else {
            self.zoom = before.0;
            return false;
        };
        let corner = match view {
            View::Xyz { left, top, .. } => (left, top),
            View::FitH { top } | View::FitBH { top } => (None, top),
            View::FitV { left } | View::FitBV { left } => (left, None),
            // Table 149 puts the rectangle's own corner at the window's.
            View::FitR { rect } => (Some(rect[0].min(rect[2])), Some(rect[1].max(rect[3]))),
            // "Display the page ... with its contents magnified just enough to fit the entire
            // page within the window" — the whole page is in view, so there is nothing to
            // scroll to and the top-left corner is the answer in both directions.
            View::Fit => (Some(0.0), Some(size.height)),
            View::FitB => content.map_or((Some(0.0), Some(size.height)), |box_| {
                (Some(box_.min.x), Some(box_.max.y))
            }),
        };
        self.scroll_to(corner, view, magnification, size.height);
        crate::layout::settle_scroll(self, viewport, scale, magnification);
        (self.zoom, self.scroll) != before
    }

    /// Puts a point of the page at the window's top-left corner.
    ///
    /// The `/FitB` forms already carry display-list coordinates, because that is the space a
    /// content bounding box is measured in; every other form states default user space and goes
    /// through the page's transform first.
    fn scroll_to(
        &mut self,
        corner: (Option<f32>, Option<f32>),
        view: pdf_model::destination::View,
        magnification: f32,
        page_height: f32,
    ) {
        use pdf_model::destination::View;

        let already_page_space = matches!(
            view,
            View::Fit | View::FitB | View::FitBH { .. } | View::FitBV { .. }
        );
        let (x, y) = match (corner.0, corner.1) {
            (None, None) => return,
            (left, top) => {
                let point = (left.unwrap_or(0.0), top.unwrap_or(page_height));
                if already_page_space {
                    point
                } else {
                    match self.shown_page() {
                        Some(page) => pdf_model::content::page_space_at(page, point.0, point.1),
                        None => point,
                    }
                }
            }
        };
        if corner.0.is_some() {
            self.scroll.0 = (x * magnification).max(0.0);
        }
        if corner.1.is_some() {
            // The display list's y counts up from the bottom of the page and a raster's rows
            // count down from the top, so the distance scrolled is measured from the *top*.
            self.scroll.1 = ((page_height - y) * magnification).max(0.0);
        }
    }
}

/// The raster a page of this size occupies at this magnification.
///
/// The same rounding `TargetSpec::for_page` does — up, so the raster contains the page — stated
/// here because `apply_view` clamps a scroll before there is a target to ask.
///
/// Shared with [`crate::viewer`] rather than copied into it: every caller that places page
/// coordinates on the screen has to agree with the drawn frame to the pixel, and two copies of
/// this rounding are two chances to disagree (ADR 0118).
///
/// **The product is taken in `f64`, as [`TargetSpec::for_page`] takes it.** Agreeing on the
/// rounding is not enough if the multiplication that precedes it differs: `595.276 × 40` is a
/// number `f32` cannot hold, and one of the two arithmetics can land above an integer while the
/// other lands below it, after which `ceil` puts the chrome one pixel off the frame it is drawn
/// over. The inputs are `f32` and the caller's; the *product* is where the precision has to
/// match, and this is the only place that can make it.
pub(crate) fn raster_extent(size: Size, magnification: f32) -> (u32, u32) {
    let extent = |value: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a float-to-integer cast saturates in Rust, and an extent that saturated \
                      is refused by `pixel_extent` wherever a target is actually built"
        )]
        let pixels = (f64::from(value) * f64::from(magnification))
            .ceil()
            .max(1.0) as u32;
        pixels
    };
    (extent(size.width), extent(size.height))
}

/// The box the `/FitB` forms magnify to fit: what the page's contents cover, cut to the page.
///
/// §12.3.2.2 states the bounding box and then bounds it, in the same paragraph:
///
/// > The page's bounding box is the smallest rectangle enclosing all of its contents. (If any
/// > side of the bounding box lies outside the page's crop box, the corresponding side of the
/// > crop box shall be used instead; see 14.11.2, "Page boundaries" for further discussion of
/// > the crop box.)
///
/// The cut is the part a display list makes necessary. `pdf_model::interpret` puts the box the
/// page is displayed in *at the origin* rather than clipping the content to it, so the list
/// carries every mark the stream made, including the ones off the edge — and `content_bounds`
/// unions all of them. Without this, a `/FitB` on a page whose producer left ink outside the
/// crop box would magnify to fit ink this viewer never draws. In the list's own space the
/// displayed box is `0,0 … width,height`, so the clause's "corresponding side of the crop box"
/// is that rectangle's side.
///
/// `None` where nothing survives the cut, which is the same answer as a page covering nothing:
/// the three forms fall back to the page box, because the alternative is a magnification of
/// infinity.
fn content_box(bounds: Option<Rect>, size: Size) -> Option<Rect> {
    bounds
        .map(|box_| Rect {
            min: Point::new(box_.min.x.max(0.0), box_.min.y.max(0.0)),
            max: Point::new(box_.max.x.min(size.width), box_.max.y.min(size.height)),
        })
        .filter(|box_| box_.max.x > box_.min.x && box_.max.y > box_.min.y)
}

/// How many times a fit steps down before it settles for the extra pixel.
///
/// One step has always been enough — the loop's condition is what decides, and this is the
/// bound that keeps a pathological extent from spinning rather than a number anything derives.
const FIT_STEPS: u32 = 4;

/// The largest magnification at which a page extent still fits `viewport` device pixels.
///
/// Not simply the ratio. [`TargetSpec::for_page`] rounds a raster **up** so that it contains
/// the page, and the nearest `f32` to the exact ratio is above it as often as below — so the
/// plain division produces a raster one pixel larger than the viewport it was computed to fit
/// about half the time, which is a page fitted to the window with a scrollbar down the side.
/// Stepping to the next smaller representable scale until the rounding lands is exact, costs
/// nothing measurable once per render, and needs no epsilon anybody had to choose.
fn fitted(viewport: u32, extent: f32) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a viewport dimension divided by a page dimension is a small ratio, and the \
                  loop below is what makes its rounding acceptable"
    )]
    let mut scale = (f64::from(viewport) / f64::from(extent)) as f32;
    for _ in 0..FIT_STEPS {
        if (f64::from(extent) * f64::from(scale)).ceil() <= f64::from(viewport) {
            break;
        }
        scale = f32::from_bits(scale.to_bits().saturating_sub(1));
    }
    scale
}

/// The interpretation of a page, with its reports already worded.
pub(crate) fn interpret(open: &Open, index: usize) -> Option<(Interpretation, Vec<String>, Page)> {
    let page = open.page(index)?;
    let interpretation =
        pdf_model::content::interpret_with_fonts(&open.document, &page, &open.view, &open.fonts);
    let reports = interpretation
        .unsupported
        .iter()
        .map(crate::report::describe)
        .collect();
    Some((interpretation, reports, page))
}

/// A byte string out of a URI's fragment, worded for a person to read.
///
/// Lossy, and this is the one place that is right: Annex O gives its byte strings no character
/// encoding — which is why `pdf_model::fragment` keeps them as bytes — and a sentence somebody
/// reads is where a guess costs nothing, while a comparison against the document's own bytes is
/// where it would cost everything.
fn text(bytes: &[u8]) -> String {
    format!("`{}`", String::from_utf8_lossy(bytes))
}

#[cfg(test)]
mod tests {
    use super::{content_box, raster_extent};
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
    /// the scroll clamp) is positioned from the first and drawn over the second. ADR 0118 is
    /// the session that cost.
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
