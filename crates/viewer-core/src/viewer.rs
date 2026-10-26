//! The state machine itself: one [`Viewer`] whose behaviour is written in one module per concern.
//!
//! This file holds what is shared by all of them — the state, the identities a host holds, the
//! bounds, and `act`, the one place a command is routed. Each module beside it adds an `impl
//! Viewer` for one concern, and what a sibling calls is `pub(super)`:
//!
//! | module | concern |
//! |---|---|
//! | `opening` | a document taken in, page one interpreted, a finished render installed |
//! | `layout` | zoom, scroll and `settle`, the one place a render is scheduled |
//! | `navigation` | page turns, presentation, the clock and the events a page raises |
//! | `pointer` | §12.5.5's appearances, §12.6.3's triggers and a release's activation |
//! | `outcomes` | what an interaction asked of the host and what the host answered |
//! | `editing` | fields and annotations changed, saved and extracted, and the focus |
//! | `printing` | the pages a host prints and the conversion a person asked for |
//! | `search` | find |
//! | `geometry` | page space to device space and back |
//! | `accessibility` | §14.7's elements and the controls none names |
//! | `answers` | `Viewer::query` and what it assembles |
//! | `media` | a clip handed to the host |
//! | `scripts` | what a document's scripts asked of the viewer, carried out |
//! | `anticipation` | page one drawn from the first bytes, on another thread |

use std::collections::BTreeMap;
use std::sync::Arc;

#[cfg(doc)]
use crate::Rendered;
use crate::command::{Command, Selection as CommandSelection};
use crate::event::{Event, Extraction};
use crate::interact;
use crate::open::{Chosen, Open};

mod accessibility;
mod answers;
mod anticipation;
mod editing;
mod geometry;
mod layout;
mod media;
mod navigation;
mod opening;
mod outcomes;
mod pointer;
mod printing;
mod scripts;
mod search;

pub use self::anticipation::{Anticipated, Anticipating, Foreseen, ReadOut, Reader};
use self::editing::commit_typed;
use self::layout::damage;
pub(crate) use self::layout::px;
use self::scripts::open_sequence;

/// Pixel budget for one rendered page.
///
/// Page dimensions come from the document and the scale from the viewport, so the product needs
/// a bound: a page claiming absurd dimensions must fail to render rather than ask for all
/// available memory. A page over the budget is *named* rather than quietly drawn smaller — a
/// silent cap is a defect, not safety.
///
/// **It bounds an allocation, so it applies where one happens.** A tier-1 host takes a
/// [`Rendered::Raster`] of the whole page and this is its size limit. A tier-2 host draws the
/// page onto its own surface at *window* size and keeps no pixels of ours (`viewer-ui` does
/// exactly that), so a raster of this size is never built for it — and refusing its render
/// request against this number refused pages that nothing was going to allocate, which is what
/// a person zooming in saw. `Viewer::holds_rasters` is which of the two is being talked to.
///
/// **A host that answers [`Rendered::Listed`] is still the first of the two**, and that is the
/// point of that outcome rather than an accident of it: taking one page's display list says
/// nothing about the next page, so the bound stays on. `viewer-confined`'s worker mixes the two
/// arms page by page under an address-space ceiling, where an unbounded raster is a kill rather
/// than a refusal.
///
/// The bounds that are not about allocation stay unconditional, and `TargetSpec::for_page`
/// applies them to every caller: a dimension over [`pdf_render::MAX_EXTENT`] is an `f32`
/// precision limit, and a degenerate one is a target that cannot exist.
///
/// **Public because a host that confines this crate has to do arithmetic with it.**
/// `viewer-confined` runs a viewer under an address-space ceiling and has to know how much of
/// that ceiling a page's pixels may claim before it can say how large a document it can accept.
/// Copying the number into that crate would be a number that can drift; exporting it is the same
/// bound read from one place, which is what `pdf_sandbox`'s own ceiling was derived from.
pub const MAX_PIXELS: u64 = 1 << 28;

/// Most `setFocus` requests one command carries out: a script that asks for the focus, and the
/// `/Fo` script of the field it reaches asking once more, with room; past it the request is
/// dropped, so a ring of focus scripts costs one command four hops and no more (ADR 1615).
const MAX_FOCUS_HOPS: usize = 4;

/// Most `this.pageNum` turns one command carries out, for [`MAX_FOCUS_HOPS`]'s reason: a page's
/// `/O` script may turn the page again, and a ring of them costs one command four turns (ADR 1643).
const MAX_PAGE_TURNS: usize = 4;

/// Why the page is being turned, which ISO 32000-2 §12.4.4 makes two different questions.
///
/// Not a message and deliberately not one: a host says which page it wants, and whether the
/// request came from a person is something this crate knows without being told — every command is
/// a person's, and the one turn that is not comes from §12.4.4.1's own clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Turn {
    /// A person asked: an arrow key, a link, a panel row, a fragment identifier.
    ///
    /// §12.4.4.2's "the user requests to navigate", which is what a page's navigation nodes
    /// answer before the page does.
    Requested,
    /// §12.4.4.1's `/Dur` ran out, which is a presentation advancing itself and not a request.
    Automatic,
}

/// A host's name for one open document.
///
/// The host's rather than the viewer's, so that a host can open a file and refer to it in the
/// same breath without waiting for an event to come back and tell it what the document is
/// called. Opening twice under one identity replaces the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentId(pub u64);

/// Identifies one render request and the answer to it.
///
/// Opaque, and deliberately not a page number: two renders of one page at two zooms are two
/// requests, and the second must not be satisfied by the first arriving late.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderToken(u64);

/// The viewer: documents, where they are being looked at, and what needs drawing.
///
/// Deliberately not [`Default`]: a viewer with no viewport renders nothing, so a host that
/// constructs one has to say how large its window is before anything can appear, and
/// [`Viewer::new`] taking the size is what makes that impossible to forget.
#[derive(Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "four independent facts about one viewer, each read in one place and none of them a \
              state: whether a trigger is being raised, whether a step is under way, whether this \
              host takes rasters at all, and §10.8.3's answer"
)]
pub struct Viewer {
    /// Every open document.
    documents: BTreeMap<DocumentId, Open>,
    /// Which one commands apply to.
    focused: Option<DocumentId>,
    /// Whether the pointer's button is down: a `Pressed` has arrived and its `Released` has not.
    ///
    /// The pointer's state rather than a document's, and apart from `Open::pressed_on` because a
    /// press may land on no annotation at all. §12.5.5's rollover "shall be used when the user
    /// moves the cursor into the annotation's active area without pressing the mouse button", so
    /// a move with the button down shows no rollover whatever it was pressed on (ADR 1370's
    /// press state, completed).
    button_down: bool,
    /// The viewport in device pixels.
    viewport: (u32, u32),
    /// Device pixels per logical pixel.
    scale: f32,
    /// The next token, which only ever increases.
    next_token: u64,
    /// Whether §12.6.3's page-scoped events are being raised right now.
    ///
    /// The bound on a cascade. See [`Self::page_events`].
    raising: bool,
    /// Whether §12.4.4.2's navigation nodes are being walked right now.
    ///
    /// The same bound one clause over, and the clause itself says why one is needed: "\[i]f NA
    /// specifies an action that navigates to another page, the following actions for navigating to
    /// another page take place, and Next should not be present". *Should*, addressed to a
    /// producer — so a file that states both has stated a walk that arrives at a page whose own
    /// `/PresSteps` would walk again, and a reader may not take a producer's advice as a bound.
    stepping: bool,
    /// Whether the arrangement's unread pages are left for the host to read, for the length of
    /// one [`Self::handle_unread`] (ADR 1871).
    unread: bool,
    /// The page whose read a host is running on a thread of its own, which no `settle` reads in
    /// the meantime ([`Self::read_out`], ADR 2344).
    out: Option<anticipation::Out>,
    /// The next [`Self::out`]'s serial, which only ever increases.
    next_out: u64,
    /// Whether the host says §12.4.4's presentation is running.
    ///
    /// Held here rather than per document for [`Self::delegated`]'s reason: it is a fact about the
    /// *window*, and a program showing a slide show is showing one whatever it has open. What it
    /// decides is §12.4.4.2's NOTE 3 — whether the navigation nodes are respected at all — and it
    /// is a value only a host can supply, because full screen is chrome and chrome is the host's.
    presenting: crate::PresentationMode,
    /// Whether the host takes whole-page pixels from this crate — which is what [`MAX_PIXELS`]
    /// bounds, and the only case in which it should be applied.
    ///
    /// True until a host answers [`Rendered::Presented`], because a viewer that has not been
    /// told otherwise must assume it will be asked to hold a raster. A tier-2 host settles it
    /// on its first frame, which it draws at an opening magnification where the budget is not
    /// in question — so the conservative start costs nothing and the tier is never guessed.
    ///
    /// **[`Rendered::Listed`] deliberately does not move it.** That outcome is one page's, and a
    /// host answering it for one page and [`Rendered::Raster`] for the next is a host this
    /// crate is still holding pixels for.
    holds_rasters: bool,
    /// What the host does with a page whose whole raster would be past [`MAX_PIXELS`]
    /// ([`crate::Command::Oversize`], ADR 1892): refused until a host says it draws such a page
    /// from its list, as much of it as its window shows.
    oversize: crate::Oversize,
    /// What this reader does with the restrictions a document asserts over it.
    ///
    /// The whole of the policy `CLAUDE.md`'s "a document's restrictions are the reader's to set"
    /// asks for: one value, held here rather than deduced anywhere, set by
    /// [`Command::Restrict`] and asked **once per operation** in [`Self::standing`] — one level
    /// per operation, every one of them `Off` until a host says otherwise. ADR 0212, ADR 1144.
    ///
    /// **The window's levels, which one document may depart from.** `Open::restrictions` is that
    /// departure and [`crate::RestrictionPolicy::under`] is where the two meet; this value is what
    /// a document inherits when it opens and what the next one opened goes back to (ADR 1145).
    restrictions: crate::RestrictionPolicy,
    /// Whom this reader believes — §12.8.1's third question, which no state machine over a file
    /// can answer for itself.
    ///
    /// The whole of the policy ADR 1039 made a host's, supplied by [`Command::Trust`] and read
    /// when a document is asked what it says about itself. Defaults to no anchors at all, which is
    /// what every host in this tree supplied before the command existed.
    trust: crate::TrustPolicy,
    /// Whether a document's scripts run — RFC 0008 section 6.3's level, as [`Command::Scripts`]
    /// last supplied it, and [`crate::Scripting::Off`] until a host says otherwise (ADR 1616).
    scripting: crate::Scripting,
    /// What the window's keyboard holds, as [`Command::Keys`] last said: none down until a host
    /// says otherwise (ADR 1771).
    keys: pdf_model::view::Keys,
    /// Who draws §12.7's form widgets, as the host has said (§6.3.2.2).
    ///
    /// Held here rather than per document because it is a fact about the *host*: a program that
    /// places native controls over a page places them over every page of every document it shows.
    /// Pushed into each document's [`pdf_model::view::ViewState`] in [`Self::settle`], which is
    /// the same route [`pdf_model::view::ViewState::set_magnification`] takes and for the same
    /// reason — rule 1 makes that state the only channel into interpretation. ADR 0245.
    delegated: pdf_model::view::WidgetAppearances,
    /// §8.11.4.4's answers about this reader: who they are, and what language this is in.
    ///
    /// The eighth host-supplied policy value, held here for `delegated`'s reason — it is a fact
    /// about the *host* and not about any one file — and pushed into each document's
    /// [`pdf_model::view::ViewState`] by [`Command::Audience`], which is the only channel into
    /// interpretation rule 1 allows. [`pdf_model::optional_content::Audience::NONE`] until a
    /// host says otherwise. ADR 1106.
    audience: pdf_model::optional_content::Audience,
    /// What the host's clock says, for Table 166's `/M` on what a save writes.
    ///
    /// The ninth host-supplied policy value, held here for `audience`'s reason — it is a fact
    /// about the *host's machine* and not about any one file — and pushed into each document's
    /// [`pdf_model::view::ViewState`] by [`Command::Clock`], which is where the writer reads it.
    /// `None` until a host says otherwise, under which no annotation this program writes carries
    /// the entry. ADR 1160.
    clock: Option<pdf_syntax::Date>,
    /// Whether §10.8.3's separation simulation is what this reader has asked for.
    ///
    /// The tenth host-supplied policy value, held here for `clock`'s reason — it is a fact about
    /// the *reader* and not about any one file — and pushed into each document's
    /// [`pdf_model::view::ViewState`] by [`Command::Separations`], which is the only channel
    /// into interpretation rule 1 allows. `false` until a host says otherwise, under which
    /// §10.8.2's alternate space and tint transform are what every colour is drawn through.
    /// ADR 1228.
    separations: bool,
    /// The name the next document opened **beside** the one showing would be called by.
    ///
    /// The eleventh host-supplied policy value, held here for `separations`'s reason — it is a
    /// fact about the *window* rather than about any one file — and set by [`Command::Beside`].
    /// `None` until a host says otherwise, under which Table 203's and Table 204's `/NewWindow
    /// true` is answered by a destination that replaces the document it was reached from, said out
    /// loud. Taken rather than read when one is opened under it, so a host offers a name per tab.
    /// ADR 1263.
    beside: Option<DocumentId>,
    /// A file a document named that asked for §7.6.4.1's password, and the name it was asked under.
    ///
    /// Held here rather than on the source document because the retry is `Command::Open` under
    /// that name, which is the route every document a host names takes: the host answers the
    /// prompt with the file's own bytes and a password, and what the request named is applied to
    /// the file that opens. Taken by the next open of any name, so a jump a person walked away
    /// from never lands on a document opened later (ADRs 1332, 1335).
    locked: Option<Hold>,
    /// §8.10.4's target documents, parsed once for every document this viewer holds.
    ///
    /// Handed to each [`Open`] as it is created, so that a document opened after
    /// [`Command::References`] gets the same supply as one opened before it — which is
    /// [`Command::Restrict`]'s rule for every host-supplied policy value.
    references: Arc<pdf_model::reference::Supply>,
    /// The files [`Command::References`] offered that this reader will not import from.
    ///
    /// Kept rather than raised as an event: a refusal is a fact about the *host's* input and not
    /// about any document, so the party that sent the command is the party that asks. See
    /// [`Self::reference_refusals`].
    reference_refusals: Vec<pdf_model::reference::Refusal>,
}

/// A file held for §7.6.4.1's password, with the two names it is about.
#[derive(Debug)]
struct Hold {
    /// The name the prompt was raised under, which the retry's `Command::Open` names.
    asked: DocumentId,
    /// The document whose request named the file.
    source: DocumentId,
    /// What the file was named for.
    locked: Box<interact::Locked>,
}

impl Viewer {
    /// A viewer with nothing open, looking at a viewport of the given size.
    ///
    /// `width` and `height` are device pixels and `scale` is device pixels per logical pixel;
    /// [`Command::Resize`] changes all three later.
    #[must_use]
    pub fn new(width: u32, height: u32, scale: f32) -> Self {
        Self {
            documents: BTreeMap::new(),
            focused: None,
            button_down: false,
            viewport: (width, height),
            scale: if scale > 0.0 { scale } else { 1.0 },
            next_token: 0,
            raising: false,
            stepping: false,
            unread: false,
            out: None,
            next_out: 0,
            presenting: crate::PresentationMode::default(),
            holds_rasters: true,
            oversize: crate::Oversize::Refused,
            restrictions: crate::RestrictionPolicy::default(),
            trust: crate::TrustPolicy::default(),
            scripting: crate::Scripting::Off,
            keys: pdf_model::view::Keys::default(),
            delegated: pdf_model::view::WidgetAppearances::default(),
            audience: pdf_model::optional_content::Audience::NONE,
            clock: None,
            separations: false,
            beside: None,
            locked: None,
            references: Arc::new(pdf_model::reference::Supply::none()),
            reference_refusals: Vec::new(),
        }
    }

    /// Performs one command and returns everything it caused.
    ///
    /// The events are returned rather than delivered, so that a host decides when they are
    /// looked at and this crate never calls into one. A command that changes nothing produces
    /// nothing — an empty iterator is a normal answer and not a failure.
    pub fn handle(&mut self, command: Command) -> impl Iterator<Item = Event> + use<> {
        let mut events = Vec::new();
        self.act(command, &mut events);
        self.apply_resumed_scripts();
        self.carry_out_page_requests(&mut events);
        self.carry_out_focus_requests(&mut events);
        self.carry_out_view_requests(&mut events);
        self.carry_out_bookmark_requests(&mut events);
        self.say_what_scripts_said(&mut events);
        self.ask_about_scripts(&mut events);
        self.put_script_questions(&mut events);
        self.ask_for_sounds(&mut events);
        self.settle(&mut events);
        events.into_iter()
    }

    /// Applies a command to the state, leaving what has to be *drawn* to [`Self::settle`].
    ///
    /// The split is what keeps a scheduling decision from being made twice: every command that
    /// changes what should be on the screen ends by having changed only the state, and exactly
    /// one place works out whether that means a new render.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per variant of `Command`, and the count is that enum's. Splitting it \
                  would put half the vocabulary in another function and lose the property this \
                  match rests on: the compiler naming the command nobody handled"
    )]
    fn act(&mut self, command: Command, events: &mut Vec<Event>) {
        match command {
            Command::Open {
                id,
                bytes,
                password,
                fragment,
            } => self.open(id, bytes, password.as_ref(), fragment.as_deref(), events),
            Command::Close(id) => {
                // Table 200's `/WC`, "before closing a document", and what it said goes out
                // before the document does (ADR 1614).
                if let Some(open) = self.documents.get_mut(&id) {
                    let page = open.page_index;
                    open.view.run_document_scripts(
                        &open.document,
                        pdf_model::view::DocumentTrigger::WillClose,
                        page,
                    );
                }
                self.say_what_scripts_said(events);
                if self.documents.remove(&id).is_some() {
                    events.push(Event::Closed(id));
                }
                if self.focused == Some(id) {
                    self.focused = self.documents.keys().next().copied();
                    self.announce_page(events);
                }
            }
            Command::Focus(id) => {
                if self.documents.contains_key(&id) && self.focused != Some(id) {
                    self.focused = Some(id);
                    self.announce_page(events);
                }
            }
            Command::Resize {
                width,
                height,
                scale,
            } => {
                // The frame already on screen is *not* dropped. It was drawn for a different
                // viewport and is the wrong size, and showing it until the new one arrives is
                // better than showing a person nothing while they drag a window edge.
                // `Frame::target` is what says it is stale.
                self.viewport = (width, height);
                self.scale = if scale > 0.0 { scale } else { 1.0 };
                events.push(damage(self.viewport));
            }
            Command::GoTo(target) => self.go_to(target, Turn::Requested, events),
            Command::Present(mode) => self.present(mode),
            Command::Activate(object) => self.activate(object, events),
            Command::Tick { millis } => self.tick(millis, events),
            Command::Zoom { zoom, at } => self.set_zoom(zoom, at, events),
            Command::Scroll { dx, dy } => self.scroll(dx, dy, events),
            Command::View(view) => self.restore(view, events),
            // Two scopes in one message (ADR 1145). The window's levels are what every document
            // inherits and what one opened afterwards gets; an override is what the focused
            // document departs from them in, for as long as it is open. A document-scoped policy
            // with nothing focused has nothing to attach to and does nothing, which is the answer
            // every other document-scoped command gives.
            Command::Restrict(scope) => match scope {
                crate::RestrictionScope::Window(policy) => self.restrictions = policy,
                crate::RestrictionScope::Document(departures) => {
                    if let Some(open) = self.focused_mut() {
                        open.restrictions = departures;
                    }
                }
            },
            Command::Copy => self.copy(events),
            Command::Print(printing) => self.print(printing, events),
            Command::Archive(asked) => self.archive(asked, events),
            // A document's report is a function of the file *and* of whom this reader believes, so
            // the wording already produced is no longer the wording this policy would produce.
            // `Open::about` is a `OnceLock` over exactly that function (ADR 1044), and forgetting
            // it is how "once" stays a property of the value rather than a stale answer.
            Command::Trust(policy) => {
                self.trust = policy;
                for open in self.documents.values_mut() {
                    open.forget_about();
                }
            }
            Command::References(files) => self.supply_references(&files),
            // A script reads the keys at its event, in whichever document raises it (ADR 1771).
            Command::Keys(keys) => {
                self.keys = keys;
                for open in self.documents.values_mut() {
                    open.view.set_keys(keys);
                }
            }
            // RFC 0008 section 6.3's level, applied to every open document and to every one
            // opened afterwards — `Command::Restrict`'s rule, for its reason (ADR 1616).
            Command::Scripts(scripting) => {
                self.scripting = scripting;
                let ids: Vec<DocumentId> = self.documents.keys().copied().collect();
                for id in ids {
                    self.supply_scripts(id);
                }
            }
            // RFC 0008 section 4.2's `app.alert` and `app.response`: the answer goes to the script
            // waiting on it, and what the script did with it is ink, so the page is drawn again
            // (ADR 1628).
            Command::AnswerScript { document, answer } => {
                if let Some(open) = self.documents.get_mut(&document)
                    && let Some(asks) = open.consent.asks.clone()
                {
                    asks.answer(answer);
                }
            }
            Command::AnswerScripts { document, proceed } => {
                let asking = self.documents.get_mut(&document).is_some_and(|open| {
                    open.consent.answered.is_none() && open.consent.withheld.is_some()
                });
                if asking {
                    if let Some(open) = self.documents.get_mut(&document) {
                        open.consent.answered = Some(proceed);
                    }
                    self.supply_scripts(document);
                }
            }
            // §8.11.4.4's two categories about this reader, applied to every open document and
            // to every one opened afterwards — `Command::Restrict`'s rule, for its reason.
            // A document whose groups move is drawing something else, so its ink is superseded.
            Command::Audience(audience) => {
                self.audience = audience;
                for open in self.documents.values_mut() {
                    if open.view.set_audience(self.audience.clone()) {
                        open.stale();
                    }
                }
            }
            // Table 166's `/M`, applied to every open document and to every one opened
            // afterwards — `Command::Restrict`'s rule, for its reason. Nothing is redrawn: the
            // entry decides what a save writes and no mark on any page.
            Command::Clock(at) => {
                self.clock = at;
                for open in self.documents.values_mut() {
                    open.view.set_modification_time(at);
                }
            }
            // §10.8.3's simulation, applied to every open document and to every one opened
            // afterwards — `Command::Restrict`'s rule, for its reason. Every page already
            // interpreted was interpreted for the other answer, so all of them are superseded.
            // A page refused at the old answer is asked for at the next settle, which follows
            // every command: the refusal it was said under is a refusal this host no longer makes.
            Command::Oversize(oversize) => self.oversize = oversize,
            Command::Separations(simulate) => {
                self.separations = simulate;
                for open in self.documents.values_mut() {
                    if open.view.set_separation_simulation(simulate) {
                        open.stale();
                    }
                }
            }
            // A name and nothing else: nothing is opened, nothing is drawn and no document is
            // touched, because what this carries is what the window *could* do rather than
            // something it has been asked to do (ADR 1263).
            Command::Beside(name) => self.beside = name,
            Command::Answer { document, proceed } => self.answer(document, proceed, events),
            // Table 29's arrangement, as the person reading has now chosen it. The scroll is
            // measured from the current page's row and a row is what has just changed, so it
            // starts again at that page's top — the same reset a page turn makes, and for the
            // same reason: carrying a distance across a change of arrangement would land a reader
            // somewhere neither they nor the document named.
            Command::Layout(layout) => {
                let viewport = self.viewport;
                let Some(open) = self.focused_mut() else {
                    return;
                };
                if open.layout != layout {
                    open.layout = layout;
                    open.scroll = (0.0, 0.0);
                }
                events.push(damage(viewport));
            }
            // Recorded here and applied in `settle`, where the magnification is: both are facts
            // about the window that have to reach every open document's view state, and a
            // document opened after this command has to get it too.
            Command::Delegate(appearances) => self.delegated = appearances,
            Command::Edit(edit) => self.edit(edit, events),
            Command::Undo => self.move_cursor(-1, events),
            Command::Redo => self.move_cursor(1, events),
            Command::Save => self.save(events),
            Command::Extract { name } => self.extract(&name, Extraction::Asked, None, events),
            Command::Select(selection) => {
                let viewport = self.viewport;
                let Some(open) = self.focused_mut() else {
                    return;
                };
                open.selection = match selection {
                    // The *current* page's text, which is what "all" has always meant here — and
                    // still means, now that a **drag** may cross a page boundary and this does
                    // not. Two instruments rest on it: `selection_census` asserts that this
                    // answer is `pdf_model::Interpretation::text` byte for byte, and
                    // `pdf-retrieve`'s default answer is the same identity (ADR 0257). A selection
                    // over several pages joins their readbacks with a line break that no page
                    // states, so a command that quietly selected them would put a character into
                    // that string which neither page has. ADR 0444.
                    CommandSelection::All => open.interpreted().map(|interpreted| {
                        Chosen::in_page(open.page_index, (0, interpreted.text.len()))
                    }),
                    CommandSelection::None => None,
                };
                events.push(damage(viewport));
            }
            Command::Report => self.report_the_document(events),
            Command::Find(find) => self.find(find, events),
            Command::Focused(move_to) => self.move_focus(move_to, events),
            Command::CommitField { field } => {
                if let (Some(id), Some(open)) = (self.focused, self.focused_mut()) {
                    commit_typed(id, open, &field, events);
                }
            }
            Command::Presented => {
                if let Some(open) = self.focused_mut() {
                    open_sequence(open);
                }
            }
            Command::SetGroup { group, on } => {
                let Some(open) = self.focused_mut() else {
                    return;
                };
                if open.view.set_group(group, on) {
                    // §8.11 decides what is *drawn*, so a switch invalidates the display list
                    // and not merely the pixels.
                    open.stale();
                }
            }
            Command::Pointer { at, action } => self.pointer(at, action, events),
            Command::Supply { purpose, bytes } => self.supply(purpose, bytes.as_deref(), events),
            Command::Respond {
                document,
                answers,
                source,
                format,
                bytes,
            } => self.respond(document, answers, source, format, &bytes, events),
            Command::RenderReady { token, rendered } => self.rendered(token, rendered, events),
            Command::Declined {
                document,
                page,
                declined,
            } => self.declined(document, page, declined, events),
        }
    }

    /// The focused document, where one is focused and open.
    fn focused(&self) -> Option<&Open> {
        self.documents.get(&self.focused?)
    }

    /// The focused document, mutably.
    fn focused_mut(&mut self) -> Option<&mut Open> {
        self.documents.get_mut(&self.focused?)
    }
}

/// What the policy said about an edit, with the words a host will need already chosen.
///
/// `Verdict` one crate down, with the sentences attached: the reading is `pdf-model`'s and the
/// wording is this crate's, and the seam between them is here so that `Viewer::edit` matches one
/// value.
#[derive(Debug)]
enum Standing {
    /// Nothing withholds it, or the reader is not obeying.
    Proceed,
    /// The event to send instead of doing anything.
    Refuse(Event),
    /// Do it, then say these.
    Warn(Vec<String>),
    /// Hold it and ask these.
    Ask(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::{Command, DocumentId, Viewer};

    /// A document of one page and nothing else.
    fn one_page() -> Vec<u8> {
        let objects: [&[u8]; 3] = [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        ];
        let mut out = b"%PDF-2.0\n".to_vec();
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
        for offset in offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        out
    }

    /// A document of two pages, each a different size and each with a mark on it, so that a turn
    /// moves the target as well as the list.
    fn two_pages() -> Vec<u8> {
        let objects: [&[u8]; 6] = [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 5 0 R >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 6 0 R >>",
            b"<< /Length 14 >>\nstream\n0 0 50 50 re f\nendstream",
            b"<< /Length 16 >>\nstream\n10 10 80 80 re f\nendstream",
        ];
        let mut out = b"%PDF-2.0\n".to_vec();
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
        for offset in offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        out
    }

    /// A viewer showing page one of [`two_pages`], drawn.
    fn showing_page_one() -> Viewer {
        let mut viewer = Viewer::new(300, 200, 1.0);
        viewer
            .handle(Command::Open {
                id: DocumentId(1),
                bytes: two_pages().into(),
                password: None,
                fragment: None,
            })
            .for_each(drop);
        viewer
    }

    /// What a run of events asks to be drawn: page, target and how many commands, in order.
    fn asked(events: &[crate::Event]) -> Vec<(usize, pdf_render::TargetSpec, usize)> {
        events
            .iter()
            .filter_map(|event| match event {
                crate::Event::NeedsRender(request) => {
                    Some((request.page, request.target, request.list.command_count()))
                }
                _ => None,
            })
            .collect()
    }

    /// A turn whose read is left to the host asks for nothing until the host has read the page,
    /// and then asks for exactly what the turn handled whole asks for — the page, its target and
    /// its list — and the target the core foretells is the one it asks at (ADR 1871).
    #[test]
    fn a_turn_read_by_the_host_asks_for_what_a_turn_handled_whole_asks_for() {
        let mut whole = showing_page_one();
        let expected: Vec<crate::Event> = whole
            .handle(Command::GoTo(crate::PageTarget::Next))
            .collect();
        let expected = asked(&expected);
        assert_eq!(expected.len(), 1, "the whole turn asks for page two once");
        assert!(
            expected
                .iter()
                .all(|&(page, _, commands)| page == 1 && commands > 0),
            "and page two's mark is in its list"
        );

        let mut viewer = showing_page_one();
        let unread: Vec<crate::Event> = viewer
            .handle_unread(Command::GoTo(crate::PageTarget::Next))
            .collect();
        assert!(
            asked(&unread).is_empty(),
            "nothing is asked of a page unread"
        );
        let (read, foreseen) = {
            let anticipating = viewer
                .anticipating()
                .expect("the page the turn brought on is handed out unread");
            let foreseen = anticipating.foresee((300, 200), 1.0);
            let parts = pdf_model::progress::Parts::new();
            (anticipating.reader().read(&parts), foreseen)
        };
        let mut events: Vec<crate::Event> = viewer.anticipated(read, None).collect();
        events.extend(viewer.read_the_rest());
        assert_eq!(asked(&events), expected);
        assert_eq!(
            foreseen.map(|foreseen| foreseen.target),
            expected.first().map(|&(_, target, _)| target),
            "the target foretold is the target asked at"
        );
    }

    /// A turn's read handed out to a host's thread leaves the viewer answering meanwhile: a
    /// command handled before the read comes back neither reads the page nor asks for it, and the
    /// read taken in then asks for exactly what the turn handled whole asks for (ADR 2344).
    #[test]
    fn a_read_out_is_left_unread_by_the_commands_handled_while_it_runs() {
        let mut whole = showing_page_one();
        let expected: Vec<crate::Event> = whole
            .handle(Command::GoTo(crate::PageTarget::Next))
            .collect();
        let expected = asked(&expected);

        let mut viewer = showing_page_one();
        viewer
            .handle_unread(Command::GoTo(crate::PageTarget::Next))
            .for_each(drop);
        let read_out = viewer
            .read_out((300, 200), 1.0)
            .expect("the page the turn brought on is handed out");
        assert_eq!(read_out.page(), 1);
        assert_eq!(
            read_out.foreseen().map(|foreseen| foreseen.target),
            expected.first().map(|&(_, target, _)| target),
            "the target foretold is the target asked at"
        );
        let meanwhile: Vec<crate::Event> = viewer.read_the_rest().collect();
        assert!(
            asked(&meanwhile).is_empty(),
            "a settle while the read is out neither reads the page nor asks for it"
        );
        assert!(
            matches!(
                viewer.query(crate::Query::PageGeometry(1)),
                crate::Answer::Geometry(_)
            ),
            "and the page is placed meanwhile, for a window to show what it retains of it"
        );
        let read = std::thread::scope(|scope| {
            scope
                .spawn(|| read_out.read(&pdf_model::progress::Parts::new()))
                .join()
                .expect("the read on its own thread ends")
        });
        let mut events: Vec<crate::Event> = viewer.anticipated(read, None).collect();
        events.extend(viewer.read_the_rest());
        assert_eq!(asked(&events), expected);
    }

    /// A read out that comes back to a view changed while it ran is dropped, and the page is then
    /// read by the viewer under the view as it stands — the same page a turn handled whole after
    /// the change asks for (ADR 2344). Planted: with the view's check taken out of
    /// `Viewer::anticipated`, the stale read is installed and the page is not read again.
    #[test]
    fn a_read_out_under_a_view_changed_since_is_dropped_and_read_again() {
        let mut viewer = showing_page_one();
        viewer
            .handle_unread(Command::GoTo(crate::PageTarget::Next))
            .for_each(drop);
        let read_out = viewer
            .read_out((300, 200), 1.0)
            .expect("the page the turn brought on is handed out");
        let read = read_out.read(&pdf_model::progress::Parts::new());
        viewer
            .handle(Command::Delegate(
                pdf_model::view::WidgetAppearances::Delegated,
            ))
            .for_each(drop);
        let revision = viewer
            .documents
            .get(&DocumentId(1))
            .map(|open| open.revision);
        let mut events: Vec<crate::Event> = viewer.anticipated(read, None).collect();
        events.extend(viewer.read_the_rest());
        assert_eq!(
            asked(&events)
                .iter()
                .map(|&(page, ..)| page)
                .collect::<Vec<_>>(),
            [1],
            "page two is read again and asked for once"
        );
        assert_eq!(
            viewer
                .documents
                .get(&DocumentId(1))
                .map(|open| open.revision),
            revision.map(|revision| revision.saturating_add(1)),
            "by one interpretation under the changed view, not by the stale read"
        );
    }

    /// A page interpreted ahead under a host's delegated widgets is the page the first `Resize`
    /// draws, not one it interprets again (ADR 1539).
    ///
    /// **Written against the defect**: `anticipate` interpreted under the view's own default, the
    /// first `settle` then applied `Command::Delegate`'s answer, found it changed and dropped the
    /// interpretation — so `quorra-gtk` and `quorra-qt`, which delegate, interpreted page one twice.
    /// The revision is what counts interpretations.
    #[test]
    fn a_delegating_host_keeps_the_page_it_interpreted_ahead() {
        let mut viewer = Viewer::new(0, 0, 1.0);
        for command in [
            Command::Delegate(pdf_model::view::WidgetAppearances::Delegated),
            Command::Open {
                id: DocumentId(1),
                bytes: one_page().into(),
                password: None,
                fragment: None,
            },
        ] {
            viewer.handle(command).for_each(drop);
        }
        viewer.anticipate().for_each(drop);
        let revision = |viewer: &Viewer| {
            viewer
                .documents
                .get(&DocumentId(1))
                .map(|open| open.revision)
        };
        let ahead = revision(&viewer);
        viewer
            .handle(Command::Resize {
                width: 100,
                height: 100,
                scale: 1.0,
            })
            .for_each(drop);
        assert_eq!(
            revision(&viewer),
            ahead,
            "the first resize interpreted nothing the anticipation had not"
        );
        assert!(
            ahead.is_some_and(|revision| revision > 0),
            "the page was interpreted ahead"
        );
    }

    /// Every document a host names gets this reader's answers, the second as well as the first.
    ///
    /// **Written against the defect `Open::around` had**, which lost §8.10.4's target documents,
    /// §8.11.4.4's audience, Table 166's clock, §10.8.3's simulation and §12.4.4's presentation on
    /// the action path (ADR 1263). A window now opens a second document by name — a second path on
    /// its command line, a file a person chose — and each of those values is documented as
    /// applying "to every one opened afterwards", so a document opened *after* the answers were
    /// given is the case this asserts, beside the first.
    #[test]
    fn a_second_document_a_host_names_gets_every_answer_the_first_did() {
        let mut viewer = Viewer::new(100, 100, 1.0);
        let audience = pdf_model::optional_content::Audience {
            language: Some("de".to_owned()),
            ..pdf_model::optional_content::Audience::NONE
        };
        let clock = pdf_syntax::Date::parse("D:20260922120000Z");
        assert!(clock.is_some(), "a §7.9.4 date this test can write");
        for command in [
            Command::Audience(audience.clone()),
            Command::Clock(clock),
            Command::Separations(true),
            Command::Present(crate::PresentationMode::On),
        ] {
            viewer.handle(command).for_each(drop);
        }
        for id in [DocumentId(1), DocumentId(2)] {
            viewer
                .handle(Command::Open {
                    id,
                    bytes: one_page().into(),
                    password: None,
                    fragment: None,
                })
                .for_each(drop);
        }
        for id in [DocumentId(1), DocumentId(2)] {
            let Some(open) = viewer.documents.get(&id) else {
                panic!("{id:?} did not open");
            };
            assert!(
                std::sync::Arc::ptr_eq(&open.references, &viewer.references),
                "{id:?}: §8.10.4's supply is the reader's"
            );
            assert_eq!(open.view.audience(), &audience, "{id:?}: §8.11.4.4");
            assert_eq!(open.view.modification_time(), clock, "{id:?}: Table 166");
            assert!(open.view.separation_simulation(), "{id:?}: §10.8.3");
        }
        assert_eq!(
            viewer.focused,
            Some(DocumentId(2)),
            "the document named last is the one in front, which a host's tab strip follows"
        );
    }

    /// A server's answer to a submission says it is one, and §12.7.6.4's import keeps its word.
    ///
    /// Table 246's `/Status` is "[a] status string that shall be displayed indicating the result
    /// of an action, typically a submit-form action", so the sentence in front of it names the
    /// action whose result it is. The two routes share one import (§12.7.6.2's "incorporating it
    /// into the interactive form"), and what differs is only the word a person reads.
    #[test]
    fn an_answer_to_a_submission_is_worded_as_one_and_an_import_as_an_import() {
        let fdf: &[u8] = b"%FDF-1.2\n1 0 obj\n<< /FDF << /Status (Thank you) /Fields \
            [ << /T (Text1) /V (Ada) >> ] >> >>\nendobj\n\
            trailer\n<< /Root 1 0 R >>\n%%EOF\n";
        let notes = |events: Vec<crate::Event>| -> Vec<String> {
            events
                .into_iter()
                .filter_map(|event| match event {
                    crate::Event::Reported { notes, .. } => Some(notes),
                    _ => None,
                })
                .flatten()
                .collect()
        };
        let mut viewer = Viewer::new(100, 100, 1.0);
        viewer
            .handle(Command::Open {
                id: DocumentId(1),
                bytes: one_page().into(),
                password: None,
                fragment: None,
            })
            .for_each(drop);

        let answered = notes(
            viewer
                .handle(Command::Respond {
                    document: DocumentId(1),
                    answers: crate::Answered::Submission,
                    source: "http://127.0.0.1/submit".to_owned(),
                    format: pdf_model::action::DataFormat::Fdf,
                    bytes: fdf.to_vec(),
                })
                .collect(),
        );
        assert!(
            answered.contains(&"submit-form answer: status — Thank you".to_owned()),
            "{answered:?}"
        );
        assert!(
            answered.contains(
                &"submit-form answer: 1 field(s) imported from http://127.0.0.1/submit, into 0 \
                  widget(s)"
                    .to_owned()
            ),
            "{answered:?}"
        );
        assert!(
            answered.iter().all(|note| !note.starts_with("import-data")),
            "no sentence about a submission's answer calls it an import-data action: {answered:?}"
        );

        // The same bytes by §12.7.6.4's route: the action named the file and the host supplied it.
        let Some(open) = viewer.documents.get_mut(&DocumentId(1)) else {
            panic!("the document opened");
        };
        open.importing = Some(pdf_model::action::ImportData {
            file: "answers.fdf".to_owned(),
            format: pdf_model::action::DataFormat::Fdf,
        });
        let imported = notes(
            viewer
                .handle(Command::Supply {
                    purpose: crate::Purpose::ImportData,
                    bytes: Some(fdf.to_vec()),
                })
                .collect(),
        );
        assert!(
            imported.contains(&"import-data: status — Thank you".to_owned()),
            "{imported:?}"
        );
        assert!(
            imported
                .contains(&"import-data: 1 field(s) from answers.fdf, into 0 widget(s)".to_owned()),
            "{imported:?}"
        );
    }

    /// A page drawn for a tab behind the front is that tab's answer, and it is kept.
    ///
    /// The shape a window's command line makes: the second document opens and asks for its first
    /// page, the first is given the front back, and the drawing lands afterwards. Looked up in the
    /// focused document alone, the answer was dropped and the second tab never drew (ADR 1275).
    #[test]
    fn an_answer_for_a_document_behind_the_front_is_kept() {
        let mut viewer = Viewer::new(100, 100, 1.0);
        let mut asked = None;
        for id in [DocumentId(1), DocumentId(2)] {
            let events: Vec<crate::Event> = viewer
                .handle(Command::Open {
                    id,
                    bytes: one_page().into(),
                    password: None,
                    fragment: None,
                })
                .collect();
            asked = events.into_iter().find_map(|event| match event {
                crate::Event::NeedsRender(request) if request.document == id => Some(request.token),
                _ => None,
            });
        }
        let Some(token) = asked else {
            panic!("the second document asked for its first page");
        };
        viewer.handle(Command::Focus(DocumentId(1))).for_each(drop);
        let events: Vec<crate::Event> = viewer
            .handle(Command::RenderReady {
                token,
                rendered: crate::Rendered::Presented,
            })
            .collect();
        assert!(
            events.is_empty(),
            "nothing is damaged in the front for a page of the tab behind it: {events:?}"
        );
        let behind = &viewer.documents[&DocumentId(2)].on_screen;
        assert!(
            behind
                .iter()
                .all(|page| page.pending.is_none() && page.shown.is_some()),
            "the second document's page is recorded as drawn rather than still outstanding"
        );
    }
}
