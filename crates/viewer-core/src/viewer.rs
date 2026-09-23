//! The state machine itself.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pdf_model::action::Trigger;
use pdf_model::measurement;
use pdf_model::optional_content::{ListMode, OptionalContent, Presented};
use pdf_model::view::Pointer;
use pdf_render::{DisplayList, Point, Rect, TargetSpec, Transform};
use pdf_syntax::{FileBytes, ObjectId, SyntaxError};

use crate::command::{
    Command, Find, FindDirection, PageTarget, PointerAction, Printing, Purpose, Rendered,
    Selection as CommandSelection, Viewing, Zoom,
};
use crate::event::{Event, Extraction, Found, RenderRequest};
use crate::interact;
use crate::open::{Chosen, Interpreted, Open, Pending};
use crate::query::{
    Answer, FrameView, Layer, PageGeometry, PageReadback, PageReports, PageStructure, PopupWindow,
    Query, Selected,
};
use crate::readback::ReadbackCache;

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
    /// The same bound one clause over, and the clause itself says why one is needed: "[i]f NA
    /// specifies an action that navigates to another page, the following actions for navigating to
    /// another page take place, and Next should not be present". *Should*, addressed to a
    /// producer — so a file that states both has stated a walk that arrives at a page whose own
    /// `/PresSteps` would walk again, and a reader may not take a producer's advice as a bound.
    stepping: bool,
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
    /// What this reader does with the restrictions a document asserts over it.
    ///
    /// The whole of the policy `CLAUDE.md`'s "a document's restrictions are the reader's to set"
    /// asks for: one value, held here rather than deduced anywhere, set by
    /// [`Command::Restrict`] and asked **once per operation** in [`Self::standing`] — one level
    /// per operation since the one-thousand-one-hundred-and-forty-seventh session, every one of
    /// them `Off` until a host says otherwise. ADR 0212, ADR 1144.
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
            viewport: (width, height),
            scale: if scale > 0.0 { scale } else { 1.0 },
            next_token: 0,
            raising: false,
            stepping: false,
            presenting: crate::PresentationMode::default(),
            holds_rasters: true,
            restrictions: crate::RestrictionPolicy::default(),
            trust: crate::TrustPolicy::default(),
            delegated: pdf_model::view::WidgetAppearances::default(),
            audience: pdf_model::optional_content::Audience::NONE,
            clock: None,
            separations: false,
            beside: None,
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
        self.settle(&mut events);
        events.into_iter()
    }

    /// What one open document's readback cache is holding, or `None` for a document that is not
    /// open.
    ///
    /// An instrument rather than a [`Query`], and the difference is what each is for: a `Query`
    /// is a question a host asks in order to *draw* something, and six consumers plus a wire
    /// protocol match that enum exhaustively. This answers a question about the program's own
    /// memory, which no interface displays and which `--trace=search` and
    /// `viewer-core/examples/find_cost` print. `CLAUDE.md`'s rule about a budget is that it be
    /// legible as well as bounded, and this is the legible half.
    #[must_use]
    pub fn readback_cache(&self, document: DocumentId) -> Option<ReadbackCache> {
        self.documents
            .get(&document)
            .map(|open| open.readbacks.report())
    }

    /// §8.10.4's target documents, parsed once here rather than once per open document.
    ///
    /// The files are the *reader's* policy and not any document's, so one supply is shared by every
    /// tab and a document opened later gets the same one — [`Command::Restrict`]'s rule for every
    /// host-supplied value. The refusals are kept rather than printed: what a host does with them
    /// is the host's, exactly as `viewer_host::trust_anchors` leaves an anchor's to its caller.
    fn supply_references(&mut self, files: &crate::ReferenceFiles) {
        let (supply, refused) = pdf_model::reference::Supply::read(
            files
                .files
                .iter()
                .map(|(name, bytes)| (name.clone(), bytes.clone())),
            files.source.clone(),
        );
        let supply = Arc::new(supply);
        self.reference_refusals = refused;
        for open in self.documents.values_mut() {
            open.references = Arc::clone(&supply);
            // Every list this document has is a function of what could be imported, so the ones
            // already built are no longer the ones this policy would produce.
            open.stale();
        }
        self.references = supply;
    }

    /// The files [`Command::References`] offered that this reader will not import from.
    ///
    /// Named one by one rather than counted, for the reason `pdf_signature::trust::Supply` gives
    /// one crate over: a person who pointed at a directory of six files and got four target
    /// documents would otherwise be reading pages imported under a supply they did not supply.
    ///
    /// A query rather than an [`Event`], because a refusal here is a fact about the *host's*
    /// input and about no document — so the party that sent the command is the party that asks,
    /// and it may ask whenever it likes. Empty until [`Command::References`] has been sent.
    #[must_use]
    pub fn reference_refusals(&self) -> &[pdf_model::reference::Refusal] {
        &self.reference_refusals
    }

    /// Answers a question about the viewer's state without changing any of it.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per variant of this crate's own question enum, and the count is that \
                  enum's. Splitting it would put half the vocabulary in another function and lose \
                  what the exhaustive match is here for: the compiler naming the question nobody \
                  answered"
    )]
    pub fn query(&self, query: Query<'_>) -> Answer<'_> {
        let (Some(id), Some(open)) = (self.focused, self.focused()) else {
            return Answer::None;
        };
        match query {
            Query::PageCount => Answer::Count(open.page_count),
            Query::CurrentPage => Answer::Page {
                document: id,
                index: open.page_index,
                label: label(open, open.page_index),
                of: open.page_count,
            },
            Query::PageGeometry(index) => self
                .geometry(open, index)
                .map_or(Answer::None, Answer::Geometry),
            // The three as this crate holds them, settled: `Viewer::handle` clamps the scroll at
            // the end of every command, so what a host reads here is where the reader is rather
            // than where the last message asked for them to be.
            Query::View => Answer::View(Viewing {
                page: open.page_index,
                zoom: open.zoom,
                scroll: open.scroll,
            }),
            // Cloned rather than lent, since ADR 0247: every consumer cloned it anyway, because a
            // panel outlives the query that filled it. Measured by
            // `viewer-host --example outline_census`, median of five: ISO 32000-2's own outline
            // is 988 items and the whole answer takes **80.7 µs**; the five-page application
            // note's fourteen take **481 ns**. Against ADR 0246's 3.66 ms to build the three
            // panel models those 988 rows go into, the clone is 2% of what a host does with it.
            Query::Outline => Answer::Outline(open.outline.clone()),
            Query::Layers => Answer::Layers(layers(open)),
            // The log's view rather than the file's, since the eight-hundred-and-eighty-fifth
            // session: a file attached this sitting is in the list before anything is saved, and
            // one detached is out of it. `ViewState::attachments` says which home the list is.
            Query::Attachments => Answer::Attachments(attachments_in_view(open)),
            // Read here rather than held on `Open`: see `Query::Articles`. Two of the 974 corpus
            // documents state a `/Threads` entry at all, one of them an empty array and one a
            // null, so a list built at launch would be empty 999 times in a thousand.
            Query::Articles => {
                Answer::Articles(pdf_model::article::Articles::read(&open.document).threads)
            }
            // Read on demand for the same reason: not one of the 974 corpus documents states a
            // `/Collection`, and a read at launch would walk a folder tree that is never there.
            Query::Collection => collection(open),
            Query::PageLabel(index) => label(open, index).map_or(Answer::None, Answer::Label),
            Query::Thumbnail(index) => pdf_model::Pages::new(&open.document)
                .get(index)
                .and_then(|page| pdf_model::thumbnail::read(&open.document, &page.dict))
                .and_then(Result::ok)
                .map_or(Answer::None, Answer::Thumbnail),
            Query::AttachmentPreview(name) => {
                attachment_preview(open, name).map_or(Answer::None, Answer::Thumbnail)
            }
            // §7.6.4.2's bit 3 was asked by `Command::Print`, so this answers only inside the
            // grant that command made: no job, no page. The interpretation is fresh and not kept
            // — a print job asks for each page once, and a thousand-page document's lists would
            // otherwise sit in this crate for as long as the document is open.
            Query::PrintPage(index) => open
                .printing
                .and_then(|sheet| print_page(open, index, sheet))
                .map_or(Answer::None, |page| Answer::PrintPage(Box::new(page))),
            Query::LinkAt(at) => Answer::Link(
                self.user_space(open, at)
                    .and_then(|(page, (x, y))| interact::link_at(open, page, x, y))
                    .is_some(),
            ),
            Query::FieldAt(at) => self
                .user_space(open, at)
                .and_then(|(page, (x, y))| {
                    let object = open.placed_page(page)?;
                    pdf_model::view::field_at(&open.document, object, x, y)
                })
                .map_or(Answer::None, |name| {
                    // The value is the *view*'s and the names are the document's, which is why
                    // they are gathered here rather than inside `field_at`: `pdf_model::view`'s
                    // walk knows the widget and this knows what has been typed into it.
                    // The characters and whether they *are* the characters, from one reading:
                    // a host that asked twice could be told the string is the field's own after
                    // being handed the bullets. ADR 0247.
                    let value = open.view.field_value(&open.document, &name.qualified);
                    Answer::Field { name, value }
                }),
            Query::Fields => Answer::Fields(self.form_fields(open)),
            Query::Caret { at, offset } => self
                .caret(open, at, offset)
                .map_or(Answer::None, |(from, to)| Answer::Caret { from, to }),
            Query::Offset { at, point } => self
                .offset(open, at, point)
                .map_or(Answer::None, Answer::Offset),
            Query::FieldSelection { at, from, to } => self
                .field_selection(open, at, (from, to))
                .map_or(Answer::None, Answer::FieldSelection),
            Query::FreeTextAt { at } => self
                .user_space(open, at)
                .and_then(|(page, (x, y))| {
                    let object = open.placed_page(page)?;
                    open.view.free_text_at(&open.document, object, x, y)
                })
                .map_or(Answer::None, |(annotation, text)| Answer::FreeText {
                    annotation,
                    text,
                }),
            Query::Measure(points) => self
                .measured(open, points)
                .map_or(Answer::None, Answer::Measured),
            Query::Dirty => Answer::Dirty(open.dirty()),
            Query::Properties => Answer::Properties {
                information: pdf_model::metadata::Information::read(&open.document),
                metadata: pdf_model::xmp::Xmp::document(&open.document),
            },
            Query::Opening => {
                Answer::Opening(pdf_model::viewer_preferences::Opening::read(&open.document))
            }
            Query::Preferences => Answer::Preferences(
                pdf_model::viewer_preferences::ViewerPreferences::read(&open.document),
            ),
            Query::Find(needle) => Answer::Found(self.found(open, needle)),
            Query::Focus => self
                .focus_quad(open)
                .map_or(Answer::None, |(object, quad)| Answer::Focus {
                    object,
                    quad,
                }),
            Query::Highlight => Answer::Highlighted(self.highlight_quads(open)),
            Query::Popups => Answer::Popups(self.popup_windows(open)),
            Query::Selection => self.selected(open).map_or(Answer::None, Answer::Selected),
            Query::LogicalSelection => {
                Self::logical_selection(open).map_or(Answer::None, Answer::LogicalSelection)
            }
            // `Answer::None` for a tier-2 host, which hands back no pixels at all, and a list —
            // possibly empty — for a tier-1 host. The two are told apart by the same flag the
            // pixel budget is: a host that has answered `Rendered::Presented` holds its own.
            //
            // **A page the host answered `Rendered::Listed` for is simply not in the list**, and
            // that is the answer rather than an omission: this crate is holding no pixels of it
            // because the host took its display list, and the host knows which pages those are —
            // it said so. What matters is that the question goes on being answered for the
            // page's neighbours, which is what `Rendered::Presented` could not have left true.
            Query::Frame if !self.holds_rasters => Answer::None,
            Query::Frame => Answer::Frame(
                open.on_screen
                    .iter()
                    .filter_map(|on_screen| {
                        Some(FrameView {
                            page: on_screen.page,
                            raster: on_screen.frame.as_ref()?,
                            origin: on_screen.origin,
                        })
                    })
                    .collect(),
            ),
            Query::AccessibilityTree => Answer::Accessibility(self.accessibility(open)),
            // One entry per page the arrangement is showing and this crate has read — the same
            // population `Answer::Frame` and `Query::PageGeometry` answer over, and the same
            // reason: a host given the current page's sentences for a screen holding four would
            // be reassuring a person about three pages nothing looked at.
            Query::Reports => Answer::Reports(
                open.on_screen
                    .iter()
                    .filter_map(|on_screen| {
                        Some(PageReports {
                            page: on_screen.page,
                            notes: &on_screen.interpreted.as_ref()?.reports,
                        })
                    })
                    .collect(),
            ),
            // An **empty list** rather than a tally of zeroes where no page has been interpreted:
            // "nothing was lost" and "nothing has been read yet" are different answers, and a
            // host that showed the first for the second would be reassuring a person about a
            // page it has not looked at. `Answer::None` stays what it is for every question here
            // — no document is focused.
            Query::Readback => Answer::Readback(
                open.on_screen
                    .iter()
                    .filter_map(|on_screen| {
                        Some(PageReadback {
                            page: on_screen.page,
                            shortfall: on_screen.interpreted.as_ref()?.shortfall,
                        })
                    })
                    .collect(),
            ),
        }
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
            // A document's report is a function of the file *and* of whom this reader believes, so
            // the wording already produced is no longer the wording this policy would produce.
            // `Open::about` is a `OnceCell` over exactly that function (ADR 1044), and forgetting
            // it is how "once" stays a property of the value rather than a stale answer.
            Command::Trust(policy) => {
                self.trust = policy;
                for open in self.documents.values_mut() {
                    open.forget_about();
                }
            }
            Command::References(files) => self.supply_references(&files),
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
                source,
                format,
                bytes,
            } => self.respond(document, source, format, &bytes, events),
            Command::RenderReady { token, rendered } => self.rendered(token, rendered, events),
        }
    }

    /// Opens a document, or says why it could not be.
    fn open(
        &mut self,
        id: DocumentId,
        bytes: FileBytes,
        password: Option<&crate::Secret>,
        fragment: Option<&str>,
        events: &mut Vec<Event>,
    ) {
        match Open::new(bytes, password) {
            Ok(mut open) => {
                // Every answer this reader has given, before anything reads the document — the
                // same list a document reached through an action is handed, so that a host naming
                // a file and a link naming one cannot come to hold two different documents.
                self.adopt(&mut open);
                // §12.11.6, and this is the moment the clause names: "Document requirements shall
                // be evaluated before execution of any document ECMAScripts. If requirements
                // cannot be met … then the processing of the document shall not continue."
                // Nothing below has run, so *not continuing* is a document that was read and put
                // down again rather than one half processed. The window's levels rather than the
                // document's, because a departure belongs to a document that is open and this one
                // is not (ADR 1145, ADR 1167).
                let level = self
                    .restrictions
                    .level(pdf_model::restriction::Operation::Process)
                    .level();
                let warned = match Self::verdict_of(
                    id,
                    level,
                    &open.document,
                    pdf_model::restriction::Operation::Process,
                    None,
                    None,
                ) {
                    Standing::Refuse(refused) => {
                        events.push(refused);
                        return;
                    }
                    Standing::Ask(notes) => {
                        open.asking = Some(crate::open::Held::Process {
                            fragment: fragment.map(str::to_owned),
                        });
                        // Held in the map so that `Command::Answer` can name it, and deliberately
                        // not focused: nothing has been processed, no `Event::Opened` has been
                        // raised, and a question about a document is not that document arriving.
                        self.documents.insert(id, open);
                        events.push(Event::Asking {
                            document: id,
                            operation: pdf_model::restriction::Operation::Process,
                            notes,
                        });
                        return;
                    }
                    Standing::Warn(notes) => Some(notes),
                    Standing::Proceed => None,
                };
                self.documents.insert(id, open);
                self.process(id, fragment, events);
                if let Some(notes) = warned {
                    events.push(Event::Warned {
                        document: id,
                        operation: pdf_model::restriction::Operation::Process,
                        notes,
                    });
                }
            }
            // §7.6.4.1's prompt, and the reason this is not an `OpenFailed`: a document that
            // wants a password is not a document this program cannot read.
            Err(SyntaxError::PasswordRequired) => {
                events.push(Event::PasswordRequired { document: id });
            }
            Err(error) => events.push(Event::OpenFailed {
                document: id,
                reason: error.to_string(),
            }),
        }
    }

    /// Hands a document this window has just acquired every answer its reader has already given.
    ///
    /// The one list, asked by [`Self::open`] for a document a host named — the first file, a
    /// second on the command line, a file a person chose — and where an action's replacement is
    /// taken in, for a document §12.6.4.3's or §12.6.4.4's action reached. Every one of these
    /// values is documented as applying to "every open document and to every one opened
    /// afterwards", and a document shown in this window by this reader is one of those whoever
    /// named it. ADR 1263.
    fn adopt(&self, open: &mut Open) {
        // §8.10.4's target documents are the reader's policy rather than any document's, so a
        // document opened after `Command::References` gets the same supply as one opened before
        // it — `Command::Restrict`'s rule for every host-supplied value.
        open.references = Arc::clone(&self.references);
        // §8.11.4.4's two categories about this reader, on the same rule: a host that said who is
        // reading said it about every document it will show. Free where nothing was said, which
        // is every host by default.
        open.view.set_audience(self.audience.clone());
        // Table 166's `/M`, on the same rule: a host with a clock has one for every document it
        // will show. No entry where nothing was said, which is every host by default.
        open.view.set_modification_time(self.clock);
        // §10.8.3's simulation, on the same rule: a reader who asked for it asked about every
        // document this window will show. Off where nothing was said, which is every host by
        // default.
        open.view.set_separation_simulation(self.separations);
        // A document opened *during* a presentation arrives in the mode the host is in: §12.4.4.2's
        // node is a property of the page being shown and NOTE 2's saved groups of the document, so
        // both are taken here rather than only on `Command::Present`.
        if self.presenting == crate::PresentationMode::On {
            crate::presentation::enter(open);
        }
    }

    /// Everything opening a document does once §12.11.6 has let it go on.
    ///
    /// Split out of [`Self::open`] because the *ask* level puts a person between the two: a
    /// document held at [`crate::Event::Asking`] has been read and nothing more, and what a `yes`
    /// releases is this. ADR 1167.
    fn process(&mut self, id: DocumentId, fragment: Option<&str>, events: &mut Vec<Event>) {
        let Some((pages, mut notes)) = self.documents.get_mut(&id).map(|open| {
            // What opening the document has already discovered about §7.5.7's storage — the
            // catalogue and the page tree are read by now, and either may live in an object
            // stream. The rest arrives per page, below.
            //
            // **What the document says about *itself* is not here, and that is principle 2.**
            // `notes::about` answers eight clauses about the file, and §12.8's answer reads and
            // digests the signed byte ranges — on a signed document that is the whole file, and
            // none of it draws a page. `Command::Report` is what asks for it, and `Open::about`
            // is what makes asking twice cost once. ADR 1044.
            (open.page_count, crate::notes::losses(open))
        }) else {
            return;
        };
        // Annex O's open parameters, and this is where the annex puts them: §O.2.2 says they
        // "should be processed immediately after any other document-specified open parameters
        // have been processed", and `Open::around` has just processed Table 29's `/OpenAction`.
        // So the document states where it opens and the URI overrules it, in that order, which is
        // what a fragment identifier is for.
        if let Some(fragment) = fragment
            && let Some(open) = self.documents.get_mut(&id)
        {
            notes.extend(open.apply_fragment(&pdf_model::fragment::Fragment::parse(fragment)));
        }
        self.focused = Some(id);
        events.push(Event::Opened {
            document: id,
            pages,
        });
        if !notes.is_empty() {
            events.push(Event::Reported {
                document: id,
                page: None,
                notes,
            });
        }
        // Annex O's `ef`, if the fragment named one. Before the first page's events
        // because it is the *document* that was asked for a file rather than the view:
        // Table Annex O.3 files this parameter under object identifiers, and the file is
        // out of the bytes already read rather than out of a page nobody has drawn. The
        // same channel `Command::Extract` uses, so a host needed no new message and every
        // one of the six already handles it (ADR 0310). What travels with the bytes since
        // ADR 0431 is the rest of the fragment: "[a]ny remaining parameters after this
        // parameter apply to the selected embedded file", so they go where the file goes.
        if let Some(file) = self
            .documents
            .get_mut(&id)
            .and_then(|open| open.opening_file.take())
        {
            self.extract(&file.name, Extraction::Fragment, file.fragment, events);
        }
        self.announce_page(events);
        // §12.6.3 puts `/PO` "after … the OpenAction entry in the document Catalog",
        // and `Open::around` has already applied that entry's destination — the page it
        // names is `open.page_index` and its view is waiting in `pending_views` — so the
        // first page's events are raised here, in the clause's order. An `/OpenAction`
        // that is an action rather than a destination is still not *performed*; that is
        // §12.6.4's row and not this one's, and it changes nothing about this ordering.
        self.page_events(id, None, events);
        // Annex O's `search`, if the fragment asked for one. The plan is made and no page
        // has been read: this event is what tells a host there is something to pump, and
        // it is the same division `Event::NeedsRender` makes — a unit of work handed over
        // rather than done on the launch path.
        if let Some(remaining) = self
            .documents
            .get(&id)
            .and_then(|open| open.searching.as_ref())
            .map(|searching| searching.remaining)
        {
            events.push(Event::Searched {
                document: id,
                found: None,
                remaining,
                wrapped: false,
            });
        }
        // Annex O's `fdf`, if the fragment named one — "[o]pen the document and then
        // import the data from the specified FDF or XFDF file". **Last of the four
        // things a fragment can start**, which is the annex's own order: the `fdf`
        // parameter "is recommended to be the last parameter so that the document can
        // open directly to the appropriate view", and the view is what everything above
        // has just settled. The name crosses as the document's own words, exactly as
        // §12.7.6.4's does, and a host is what resolves or refuses it (rule 2).
        if let Some(name) = self
            .documents
            .get(&id)
            .and_then(|open| open.importing.as_ref())
            .map(|import| import.file.clone())
        {
            events.push(Event::NeedsFile {
                document: id,
                purpose: Purpose::ImportData,
                name,
            });
        }
    }

    /// Records what a worker did with a request, or drops it for being about the past.
    fn rendered(&mut self, token: RenderToken, rendered: Rendered, events: &mut Vec<Event>) {
        // A token that is not one of those outstanding answers a question that has been asked
        // again since. Dropping it is the whole reason the token exists — and an arrangement has
        // one outstanding request per page on the screen rather than one, so the token is what
        // says *which* page an answer is about.
        //
        // **Of which document, too**: tokens are issued from one counter for every document this
        // viewer holds, and a document opened beside the one showing is asked for its first page
        // and may be given back the front before the answer lands. Looked up in the focused
        // document alone, that answer was dropped, the request stayed outstanding and the tab never
        // drew (ADR 1275). The damage is the window's only where the page is the front's.
        let damage_of = |id: DocumentId, focused: Option<DocumentId>, viewport| {
            (focused == Some(id)).then(|| damage(viewport))
        };
        let (viewport, focused) = (self.viewport, self.focused);
        let Some((id, open, index)) = self.documents.iter_mut().find_map(|(id, open)| {
            open.on_screen
                .iter()
                .position(|on_screen| {
                    on_screen
                        .pending
                        .as_ref()
                        .is_some_and(|pending| pending.token == token)
                })
                .map(|index| (*id, open, index))
        }) else {
            return;
        };
        let Some(pending) = open.on_screen[index].pending.take() else {
            return;
        };
        let on_screen = &mut open.on_screen[index];
        match rendered {
            Rendered::Raster(raster) => {
                on_screen.shown = Some((pending.target, pending.revision));
                on_screen.frame = Some(raster);
                events.extend(damage_of(id, focused, viewport));
            }
            // Tier 2: the host drew it onto its own surface, so there is nothing here to hold
            // and nothing to repaint from — but it *is* on the screen, and saying so is what
            // stops the scheduler asking for it again.
            Rendered::Presented => {
                on_screen.shown = Some((pending.target, pending.revision));
                on_screen.frame = None;
                // Said once and remembered: this host draws its own frames at its own size, so
                // nothing here will hold a whole-page raster for it and `MAX_PIXELS` has
                // nothing to bound.
                self.holds_rasters = false;
            }
            // **The same two lines as above and deliberately not the third.** The host took this
            // page's display list, so there are no pixels here either — but that is a fact about
            // one page and not about the host, and a host that mixes the two arms will hand the
            // next page back as a raster. Concluding `holds_rasters = false` from it would take
            // `MAX_PIXELS` off every request the mixed host makes and silence `Query::Frame`
            // about the pages it *is* holding pixels for. See `Rendered::Listed`.
            //
            // Damaged like a raster rather than silent like a presentation: the host has what it
            // needs to draw the page and has not said it drew it.
            Rendered::Listed => {
                on_screen.shown = Some((pending.target, pending.revision));
                on_screen.frame = None;
                events.extend(damage_of(id, focused, viewport));
            }
            // **A refusal is recorded as an answer**, and it has to be: the scheduler's question
            // is "is what is on the screen what should be", and a host that cannot draw this page
            // at this resolution will say so again the next time it is asked. Without this the
            // two of them spin — ask, refuse, ask — for as long as the page is shown. What
            // changes the answer is the question changing: another page, another zoom, another
            // interpretation, all of which move the tuple below.
            Rendered::Failed(reason) => {
                on_screen.shown = Some((pending.target, pending.revision));
                on_screen.frame = None;
                events.push(Event::Reported {
                    document: id,
                    page: Some(pending.page),
                    notes: vec![reason],
                });
            }
        }
    }

    /// §12.5.5's three appearances, and what a release activates.
    ///
    /// The pointer state is only changed for an annotation that *states* the appearance in
    /// question, because changing it invalidates the page's display list: a cursor crossing a
    /// link whose only appearance stream is `/N` would otherwise re-interpret the page — 2 000 M
    /// instructions — for a picture that cannot differ.
    ///
    /// **Which annotation the pointer is on is `annotation_at`'s answer and not `link_at`'s**,
    /// since the two-hundred-and-fifty-third session. §12.5.5 is written about an annotation —
    /// "[a]n annotation may define as many as three separate appearances" — and §12.5.6.19's
    /// `/H` is an entry of a *widget*; taking the region from the link one meant that neither
    /// reached anything but a link, and `pdf_model` had implemented both for every annotation
    /// (ADR 0123). `over` is also the region §12.6.3's events already use, so this is one
    /// question asked once rather than two answers that disagreed.
    ///
    /// It is `annotation_at` for a second reason, which is a clause rather than a tidy-up: that
    /// function filters by `annotation::interacts`, and §12.5.3's `ReadOnly` says an annotation
    /// "should not respond to mouse clicks or change its appearance in response to mouse
    /// motions". Reading the region through it is what makes that sentence true here.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per pointer action, and each arm is a different clause: §12.6.3's four \
                  triggers, §12.5.1's activation, §12.5.5's appearance and the selection. \
                  Splitting it would separate the state they all read from the state they set"
    )]
    fn pointer(&mut self, at: (f32, f32), action: PointerAction, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let viewport = self.viewport;
        // **Which page of Table 29's arrangement the pointer is over**, and where on it. Under
        // `SinglePage` that is always the page showing and every answer below is what it was;
        // under a column or a spread it is whichever page the point landed on, so that a link on
        // the page beside or below the current one is still a link.
        let point = self.focused().and_then(|open| self.user_space(open, at));
        let on_page = self.focused().and_then(|open| self.page_point(open, at));
        // **Where in the text this pointer message lands, decided against the display list the
        // person is looking at rather than against whatever survives this function.** §12.5.5's
        // appearance state is changed below, and changing it calls `Open::stale`, which throws
        // the interpretation away — so a press on text that happens to lie over an annotation
        // with a down appearance used to ask an interpretation that had just been dropped and
        // get no anchor at all, and the whole drag from it selected nothing. Found by
        // `viewer-core/tests/selection_census.rs` on its first run, on 44 corpus documents; ADR
        // 0424. Asked once here for both arms below, which is also what a drag over a widget
        // costs least.
        let position = on_page.and_then(|(page, point)| self.focused()?.position_at(page, point));
        let Some(open) = self.focused_mut() else {
            return;
        };
        // What a *click* activates is the link one, and only that: §12.5.6.5's activation region
        // is a link's, and `open.pressed` below decides whether a release follows one.
        let under = point.and_then(|(page, (x, y))| interact::link_at(open, page, x, y));
        // §12.6.3's events belong to *any* annotation, and so — since the two-hundred-and-fifty-
        // third session — does §12.5.5's appearance. Asked once per pointer message, which is
        // what a `/Rect` test over a page's annotation array costs: the same shape
        // `Query::FieldAt` already pays at pointer speed.
        // **`point` is already in default user space**, which is what §12.5.2 states an
        // annotation's `/Rect` in, so it is handed straight to `annotation_at`. This arm used to
        // put it through `user_space_at` a *second* time — `Self::user_space` had already applied
        // it — so on every page whose crop box does not start at the origin, and on every page
        // §7.7.3.3 turns, §12.5.5's appearance, §12.6.3's four pointer triggers, the focus a press
        // gives a widget and §12.5.1's popup all hit-tested somewhere the pointer was not. No gate
        // clicks and `Query::LinkAt` next door was right, which is why it survived; trap 12a is
        // the shape of it and this is the third instance.
        let over = point.and_then(|(page, (x, y))| {
            let object = open.placed_page(page)?;
            pdf_model::view::annotation_at(&open.document, object, &open.view, x, y)
        });

        let wanted = match action {
            // A drag is a person choosing text, not looking at an annotation, so it leaves
            // §12.5.5's appearance where the press put it.
            PointerAction::Moved | PointerAction::Dragged => {
                over.map(|annotation| (annotation, Pointer::Over))
            }
            PointerAction::Pressed => over.map(|annotation| (annotation, Pointer::Down)),
            // Back to hovering: the button is up and the cursor is still where it was.
            PointerAction::Released => over.map(|annotation| (annotation, Pointer::Over)),
        };
        let wanted = if action == PointerAction::Dragged {
            open.pointer
        } else {
            wanted
        };
        let wanted = wanted
            .filter(|(annotation, pointer)| interact::has_appearance(open, *annotation, *pointer));
        if open.pointer != wanted {
            open.pointer = wanted;
            open.view.set_pointer(wanted);
            open.stale();
        }

        // Table 197's `/E` and `/X`, in the clause's own order: the cursor leaves one region
        // before it enters the next, and a document may act on both. §12.6.3's fourth constraint
        // needs nothing here because `over` is the topmost annotation rather than a set — "[i]n
        // the case of overlapping or nested annotations, entering a second annotation's active
        // area causes an X event to occur for the first annotation", and a cursor crossing into
        // an annotation nested inside the one it is in changes `over`, so both are raised.
        //
        // **The button decides whether an entry may happen at all.** §12.6.3 states four
        // constraints on the mouse events beside the table that defines them, and the first two
        // are about this pair: "[a]n E (enter) event may occur only when the mouse button is up"
        // and "[a]n X (exit) event may not occur without a preceding E event". A press puts the
        // button down and a drag keeps it there, so neither may enter — and the region the cursor
        // reached that way is therefore *not entered*, which is what keeps the second constraint
        // true when the cursor later leaves it. The entry is postponed rather than dropped: this
        // field stays empty until a message finds the button up over the same annotation, which
        // is the earliest moment the first constraint allows one.
        let button_down = matches!(action, PointerAction::Pressed | PointerAction::Dragged);
        let mut raised: Vec<(ObjectId, Trigger)> = Vec::new();
        if open.inside != over {
            raised.extend(open.inside.map(|left| (left, Trigger::Exit)));
            if button_down {
                open.inside = None;
            } else {
                raised.extend(over.map(|entered| (entered, Trigger::Enter)));
                open.inside = over;
            }
        }

        match action {
            PointerAction::Moved => {}
            PointerAction::Pressed => {
                // `/D`: "a mouse button is pressed inside the annotation's active area".
                raised.extend(over.map(|annotation| (annotation, Trigger::Down)));
                open.pressed = under;
                open.pressed_on = over;
                // Table 197's `/Bl` and `/Fo`, in the clause's own order, which is `/X` and
                // `/E`'s: one thing loses the focus before the next receives it.
                //
                // **How focus is acquired is not in the standard**, which says only what
                // happens when an annotation "receives the input focus" — so a press inside a
                // widget's active area giving it the focus is a choice, and it is the one every
                // pointing interface makes. Widgets only, because Table 197 says so of both
                // entries; a press on a link or a stamp therefore *blurs* whatever held it.
                let wants_focus = over.filter(|annotation| is_widget(&open.document, *annotation));
                if open.focus != wants_focus {
                    raised.extend(open.focus.map(|left| (left, Trigger::Blur)));
                    raised.extend(wants_focus.map(|got| (got, Trigger::Focus)));
                    open.focus = wants_focus;
                }
                // A press starts an empty selection where it landed, so that the first drag
                // has an anchor. An empty selection highlights nothing and is not a selection
                // a person can see.
                open.selection = position
                    .zip(on_page)
                    .map(|(position, (page, _))| Chosen::in_page(page, (position, position)));
                events.push(damage(viewport));
            }
            PointerAction::Dragged => {
                // A drag moves the selection's far end to wherever the pointer is, **on whatever
                // page of Table 29's arrangement that is**. It used to refuse to leave the page
                // the press landed on, because a range was into one page's readback and an offset
                // meant nothing anywhere else; both ends name their own page now, so a sweep down
                // a column selects the first page's tail, the pages between whole, and the last
                // page's head. §12.4.2 still gives no document-wide offset and this still does not
                // invent one — it carries the page beside the offset instead.
                //
                // A pointer over no page at all — the gap between two rows, the margin beside
                // them — leaves the selection where it was rather than dropping its far end
                // somewhere arbitrary, which is what `on_page` answering `None` already meant.
                if let (Some(chosen), Some(position), Some((page, _))) =
                    (open.selection, position, on_page)
                {
                    open.selection = Some(Chosen {
                        from: chosen.from,
                        to: crate::open::Spot {
                            page,
                            offset: position,
                        },
                    });
                    events.push(damage(viewport));
                }
            }
            PointerAction::Released => {
                let pressed = open.pressed.take();
                let pressed_on = open.pressed_on.take();
                // A press dragged off the annotation before release is a press the person
                // changed their mind about; see `PointerAction::Released`. And a press that
                // selected something was a drag rather than a click, so it does not follow the
                // link it happened to start on — which is what every viewer does and what a
                // person dragging across a paragraph of links expects.
                let selecting = open.selection.is_some_and(|chosen| !chosen.empty());
                let clicked = !selecting;
                // Table 197's `/U` — "an action that shall be performed when the mouse button is
                // released inside the annotation's active area" — for anything that is not the
                // link about to be activated. **The exclusion is the precedence rule, not a
                // shortcut**: the table says an annotation's `/A` "takes precedence over" its
                // `/AA /U`, `interact::activate` performs a link's `/A`, and
                // `action::for_annotation` would return the same list again — so routing a link
                // through both would perform its actions twice.
                //
                // **Raised before the link question is asked, since the three-hundred-and-twelfth
                // session**, and that is a clause rather than a tidy-up: this arm used to return
                // early whenever the release did not activate a link — so a click on a stamp, a
                // widget or a markup annotation raised nothing at all.
                //
                // **Table 197's sentence is not the whole condition**, and §12.6.3's third
                // constraint is the rest of it: "[a] U (up) event may not occur without preceding
                // E and D events". So a release inside an annotation's area raises `/U` only
                // where that same annotation has both — `inside` is the one an `/E` was raised
                // for and not withdrawn by an `/X`, and `pressed_on` is the one the `/D` went
                // down on. A press on the page that drags into an annotation and lets go there
                // has neither, and used to raise `/U` all the same.
                let entered = open.inside;
                if clicked
                    && let Some(annotation) = over
                        .filter(|over| Some(*over) != under)
                        .filter(|over| entered == Some(*over) && pressed_on == Some(*over))
                {
                    raised.push((annotation, Trigger::Up));
                }
                // §12.5.1's other half: "[w]hen the user activates the annotation by clicking it,
                // it exhibits its associated object, such as by opening a popup window displaying
                // a text note". A press and a release on one annotation, and the window §12.5.6.14
                // gives it opens — or closes, which the clause does not state and every reader of
                // a sticky note expects (`Open::toggle_popup`).
                let toggled = clicked
                    && pressed_on
                        .filter(|annotation| Some(*annotation) == over)
                        .is_some_and(|annotation| exhibit(id, open, annotation, events));
                if toggled {
                    events.push(damage(viewport));
                }
                if selecting || pressed.is_none() || pressed != under {
                    self.raise(id, raised, point.map(|(_, at)| at), events);
                    return;
                }
                let Some((page, (x, y))) = point else {
                    self.raise(id, raised, point.map(|(_, at)| at), events);
                    return;
                };
                self.raise(id, raised, point.map(|(_, at)| at), events);
                let Some(open) = self.focused_mut() else {
                    return;
                };
                let outcome = interact::activate(open, page, x, y);
                self.apply(id, outcome, events);
                return;
            }
        }
        self.raise(id, raised, point.map(|(_, at)| at), events);
    }

    /// Performs §12.6.3's events the pointer just raised, in the order they were raised.
    ///
    /// `at` is where the pointer was, in the page's default user space, for the events a
    /// pointer raised — and `None` for the ones it did not, which are Table 198's page open and
    /// close and a focus change. Table 240 bit 5 is the one thing that reads it: "the
    /// coordinates of the mouse click that caused the submit-form action", and a submit button
    /// is a widget, so this is the path those coordinates arrive by.
    fn raise(
        &mut self,
        id: DocumentId,
        raised: Vec<(ObjectId, Trigger)>,
        at: Option<(f32, f32)>,
        events: &mut Vec<Event>,
    ) {
        for (annotation, event) in raised {
            let Some(open) = self.focused_mut() else {
                return;
            };
            let outcome = interact::trigger(open, annotation, event, at);
            self.apply(id, outcome, events);
        }
    }

    /// §12.3.3: activates an object a host is showing outside the page.
    fn activate(&mut self, object: ObjectId, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused_mut() else {
            return;
        };
        let outcome = interact::activate_object(open, object);
        self.apply(id, outcome, events);
    }

    /// Takes the bytes a host was asked for, or says that it declined.
    fn supply(&mut self, purpose: Purpose, bytes: Option<&[u8]>, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused_mut() else {
            return;
        };
        let outcome = match (purpose, bytes) {
            (Purpose::ImportData, Some(bytes)) => {
                interact::import(open, bytes, interact::Arrival::Action)
            }
            // §12.6.4.4's suspended walk, resumed against the root that arrived.
            (Purpose::TargetRoot, Some(bytes)) => interact::resume_root(open, bytes),
            // §12.6.4.3's jump, made against the file Table 203's `/F` named.
            (Purpose::RemoteDocument, Some(bytes)) => interact::resume_remote(open, bytes),
            // Trap 5 on the one path where a *host* declines: a click that silently does
            // nothing is indistinguishable from a click on nothing.
            (Purpose::ImportData, None) => {
                let named = open
                    .importing
                    .take()
                    .map_or_else(String::new, |import| format!(" {}", import.file));
                let mut outcome = interact::Outcome::default();
                outcome
                    .notes
                    .push(format!("import-data: declined —{named} was not supplied"));
                outcome
            }
            (Purpose::TargetRoot, None) => interact::decline_root(open),
            (Purpose::RemoteDocument, None) => interact::decline_remote(open),
            // §12.7.8's named page in a second file, and §12.6.4.7's thread in one (ADR 1239).
            (Purpose::NamedPage, Some(bytes)) => interact::supply_named_page(open, bytes),
            (Purpose::NamedPage, None) => interact::decline_named_page(open),
            (Purpose::ThreadDocument, Some(bytes)) => interact::resume_threaded(open, bytes),
            (Purpose::ThreadDocument, None) => interact::decline_threaded(open),
        };
        self.apply(id, outcome, events);
    }

    /// Applies §12.7.8's form data a server answered a submission with — [`Command::Respond`].
    ///
    /// The document in front takes the whole of [`Self::apply`], exactly as an import-data action's
    /// file does. One behind it takes the values and the sentences, and is drawn again when it next
    /// comes to the front; what it cannot take is Table 253's second file, because
    /// [`Command::Supply`] answers the document in front, so that is declined out loud rather than
    /// asked for on behalf of a document nobody is looking at.
    fn respond(
        &mut self,
        document: DocumentId,
        source: String,
        format: pdf_model::action::DataFormat,
        bytes: &[u8],
        events: &mut Vec<Event>,
    ) {
        let in_front = self.focused == Some(document);
        let Some(open) = self.documents.get_mut(&document) else {
            return;
        };
        open.importing = Some(pdf_model::action::ImportData {
            file: source,
            format,
        });
        let mut outcome = interact::import(open, bytes, interact::Arrival::Answer);
        if in_front {
            self.apply(document, outcome, events);
            return;
        }
        if outcome.redraw {
            open.stale();
        }
        if !outcome.needs_file.is_empty() {
            outcome.needs_file.clear();
            outcome
                .notes
                .extend(interact::decline_named_page(open).notes);
        }
        if !outcome.notes.is_empty() {
            events.push(Event::Reported {
                document,
                page: Some(open.page_index),
                notes: outcome.notes,
            });
        }
    }

    /// Turns what a click asked for into events, and does the parts that are this crate's.
    fn apply(&mut self, id: DocumentId, mut outcome: interact::Outcome, events: &mut Vec<Event>) {
        let page = self.focused().map(|open| open.page_index);
        // Table 203's and Table 204's `/NewWindow true`, decided here because this is where the
        // two halves meet: `interact` read the entry, and whether this program has a second place
        // to put a document is the name `Command::Beside` left in reserve. Both outcomes are said
        // out loud — a person who asked for a second window and got one view is owed the
        // difference, and so is one who got a second tab they did not ask this program for
        // (trap 5, ADR 1263).
        let into = if outcome.replacement.is_some() && outcome.beside {
            let reserved = self.beside.take();
            outcome.notes.push(match reserved {
                Some(_) => "this link asks for a new window: the destination opens beside the \
                            document it was reached from"
                    .to_owned(),
                None => "this link asks for a new window; this view has one, so the destination \
                         replaces what was open"
                    .to_owned(),
            });
            reserved
        } else {
            None
        };
        if !outcome.notes.is_empty() {
            events.push(Event::Reported {
                document: id,
                page,
                notes: outcome.notes,
            });
        }
        for uri in outcome.uris {
            events.push(Event::OpenUri { document: id, uri });
        }
        for submission in outcome.submissions {
            events.push(Event::Submit {
                document: id,
                submission: Box::new(submission),
            });
        }
        for (purpose, name) in outcome.needs_file {
            events.push(Event::NeedsFile {
                document: id,
                purpose,
                name,
            });
        }
        for transition in outcome.transitions {
            if let Some(note) = crate::transition::note(&transition) {
                events.push(Event::Reported {
                    document: id,
                    page,
                    notes: vec![note],
                });
            }
            events.push(Event::Transition {
                document: id,
                transition,
            });
        }

        // §12.6.4.4 replaces the document every other request was about, so a page named by one
        // of them is a page of a document that is no longer open — unless the action asked for a
        // window of its own and this host reserved a name for one, in which case the document
        // that was open is still open and still focused.
        if let Some(mut replacement) = outcome.replacement {
            let pages = replacement.page_count;
            let target = into.unwrap_or(id);
            // Every host-supplied value applies to "every open document and to every one opened
            // afterwards", which a document reached through an action is: it is shown in this
            // window, by this reader, under the answers they gave (ADR 1263).
            self.adopt(&mut replacement);
            self.documents.insert(target, *replacement);
            // A document that opens beside the one showing is *focused*, which is
            // `Command::Open`'s own rule and not a second decision: a person who followed a link
            // asking for a new window asked to be reading the destination. A host that wants the
            // source back in front says so with `Command::Focus`, and it knows which document
            // arrived from the name on the event below.
            self.focused = Some(target);
            events.push(Event::Opened {
                document: target,
                pages,
            });
            // What the replacement says about itself waits for `Command::Report`, exactly as the
            // document it replaced did: an embedded go-to is an open, and an open draws a page
            // before it digests a signature (ADR 1044).
            self.announce_page(events);
            self.page_events(target, None, events);
            return;
        }

        let Some(open) = self.focused_mut() else {
            return;
        };
        if outcome.redraw {
            open.stale();
        }
        // Even a destination naming the page already showing states where to look at it, which
        // is what an outline item pointing at a heading half way down a page is for.
        if let Some(view) = outcome.view {
            open.pending_views = vec![view];
        }
        if let Some(target) = outcome.target
            && target != open.page_index
        {
            let left = open.page_index;
            open.page_index = target;
            open.scroll = (0.0, 0.0);
            // §12.4.4.1's clock is the page's, and an action that jumped has changed the page.
            open.shown_for = 0.0;
            self.announce_page(events);
            self.page_events(id, Some(left), events);
            // §12.4.4.2 names this one: "the navigation request was for random access (such as by
            // clicking on a link)", which the clause treats as forward.
            self.arrive(id, true, Turn::Requested, events);
        }
    }

    /// Writes §7.5.6's incremental update, or says why it could not be written.
    fn save(&mut self, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused_mut() else {
            return;
        };
        match open.view.save(&open.document) {
            Ok(written) => {
                // Table 231 bit 14's NOTE, said out loud. `pdf_model::view::ViewState::save`
                // declines to store a password field's value and this is the only channel that
                // can tell a person their typing did not reach the file — a save that quietly
                // dropped it would be exactly the silence trap 5 exists against. Not `Refused`:
                // that one is for what a *document* asserts over its reader, and this is a rule
                // this program keeps on the person's own behalf.
                for field in written.withheld {
                    events.push(Event::Reported {
                        document: id,
                        page: None,
                        notes: vec![format!(
                            "{field} is a password field, so what was typed into it was not \
                             written to the file (ISO 32000-2 Table 231, bit 14)"
                        )],
                    });
                }
                // The same rule one clause over, and the same channel. §12.5.6.6's annotation is
                // its text, so an annotation written without an appearance is one whose note this
                // program could not draw — Table 177 makes `/DA` Required, so the next reader has
                // what it needs and this one says which annotations it left to them.
                for annotation in written.unappeared {
                    events.push(Event::Reported {
                        document: id,
                        page: None,
                        notes: vec![format!(
                            "the free text annotation {} {} was written with what was typed into \
                             it and no appearance stream, because this program could not lay that \
                             text out (ISO 32000-2 §12.5.6.6)",
                            annotation.number, annotation.generation
                        )],
                    });
                }
                // Table 224's `/NeedAppearances` is in the written file, and a flag names
                // nobody. The entry's own row binds a writer "if it has not provided appearance
                // streams for all visible widget annotations present in the document", so a file
                // written with it set has something outstanding in it and this says which field
                // — trap 5's rule on the one path where a *save* falls short of the clause.
                for field in written.unconstructed {
                    events.push(Event::Reported {
                        document: id,
                        page: None,
                        notes: vec![format!(
                            "{field} was written with its value and without the whole of the \
                             appearance stream this program lays out for it, so the file asks \
                             the next reader to construct it (ISO 32000-2 §12.7.4.3, Table 224's \
                             /NeedAppearances)"
                        )],
                    });
                }
                // §7.11.4.1 gives an embedded file more than one home, so a detach that left
                // the tree is not a deletion where another home still names the stream; the
                // objects stay in use, and the person who asked is told which home keeps them.
                for (name, homes) in written.still_reached {
                    events.push(Event::Reported {
                        document: id,
                        page: None,
                        notes: vec![format!(
                            "{name:?} was taken out of the /EmbeddedFiles tree, but its file \
                             specification and stream are left in use because {homes} still \
                             reaches the stream (ISO 32000-2 §7.11.4.1)"
                        )],
                    });
                }
                events.push(Event::Saved {
                    document: id,
                    bytes: written.bytes,
                });
                // What the update wrote is now in a file, so the log up to the cursor owes it
                // nothing. Said out loud for the same reason an edit says it: `Event::Dirty` is
                // the only thing a host has to go on, and a mark that never comes off tells a
                // person their work is unsaved after they saved it.
                let was = open.dirty();
                open.saved();
                if was {
                    events.push(Event::Dirty {
                        document: id,
                        dirty: false,
                    });
                }
            }
            // Trap 5 on the one path where a *file* can refuse to be written: a save that
            // quietly did nothing is a person's work lost without a word.
            Err(error) => events.push(Event::Reported {
                document: id,
                page: None,
                notes: vec![format!("this document cannot be saved: {error}")],
            }),
        }
    }

    /// §7.11.4: hands an embedded file's decoded bytes to the host.
    ///
    /// The list is re-read rather than cached, for the reason [`Query::Attachments`] is answered
    /// the same way: it is a walk of one name tree over a document that cannot change, and
    /// holding a copy of every attachment's stream would hold a copy of every attachment.
    fn extract(
        &mut self,
        name: &str,
        asked: Extraction,
        fragment: Option<String>,
        events: &mut Vec<Event>,
    ) {
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused() else { return };
        // The log's list, so that a file attached this sitting comes out again before it is
        // saved — its stream is the one the save would write, unfiltered.
        let Some(file) = open
            .view
            .attachments(&open.document)
            .into_iter()
            .find(|file| file.name == name)
        else {
            events.push(Event::Reported {
                document: id,
                page: None,
                notes: vec![format!("this document embeds no file called {name:?}")],
            });
            return;
        };
        hand_over(id, asked, &open.document, &file, fragment, events);
    }

    /// Resolves one edit, asks the policy once, and does what the verdict says.
    ///
    /// Four verdicts, four arms, and none of them quiet: *proceed* commits the edit; *refuse*
    /// says why and commits nothing; *warn* commits and then says why; *ask* holds the resolved
    /// edit and says why, for [`Command::Answer`] to settle. A new edit after an undo discards
    /// what was undone: the log is one sequence with a cursor, which is what makes a replay of
    /// its prefix the whole of the state.
    fn edit(&mut self, edit: crate::command::Edit, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let operation = operation_of(&edit);
        let standing = self.standing(id, operation, field_of(&edit), annotation_of(&edit));
        if let Standing::Refuse(refused) = standing {
            events.push(refused);
            return;
        }
        // §12.5.6.6's rectangle and §12.5.6.15's point are measured in the viewport, and the map
        // out of it needs the viewport's size and the display's scale — this type's rather than
        // an open document's. Taken before the mutable borrow, because the two cannot be held at
        // once.
        let (drag, dropped) = match &edit {
            crate::command::Edit::FreeText { from, to, .. } => {
                let Some(open) = self.focused() else { return };
                let (Some((_, from)), Some((_, to))) =
                    (self.user_space(open, *from), self.user_space(open, *to))
                else {
                    return;
                };
                (Some([from, to]), None)
            }
            crate::command::Edit::Attach {
                home: crate::command::AttachHome::Page { at },
                ..
            } => {
                let Some(open) = self.focused() else { return };
                let Some(dropped) = self.user_space(open, *at) else {
                    return;
                };
                (None, Some(dropped))
            }
            _ => (None, None),
        };
        let Some(open) = self.focused_mut() else {
            return;
        };
        // §7.9.6's keys "shall not overlap", and a detach needs something to detach: both are
        // answered before the log is touched, and said, because an entry that did nothing would
        // be an undo step over nothing.
        match &edit {
            crate::command::Edit::Attach { name, .. }
                if open.view.attachment_named(&open.document, name) =>
            {
                events.push(Event::Reported {
                    document: id,
                    page: None,
                    notes: vec![format!(
                        "a file is already embedded under the name {name:?}, and a name tree's \
                         keys shall not overlap (ISO 32000-2 §7.9.6) — nothing was attached"
                    )],
                });
                return;
            }
            // Table 231 bit 21 is what makes a pathname a *value*; without it a file-select
            // control is an ordinary text field and the bytes behind the path belong to nothing.
            // Said rather than applied as text, which would be the wrong value under the right
            // name (trap 5).
            crate::command::Edit::ChooseFile { field, .. }
                if !pdf_model::view::is_file_select(&open.document, field) =>
            {
                events.push(Event::Reported {
                    document: id,
                    page: None,
                    notes: vec![format!(
                        "field {field:?} does not set Table 231 bit 21 FileSelect, so it is not a \
                         file-select control (ISO 32000-2 §12.7.5.3) — no file was chosen for it"
                    )],
                });
                return;
            }
            crate::command::Edit::Detach { name }
                if !open.view.attachment_named(&open.document, name) =>
            {
                events.push(Event::Reported {
                    document: id,
                    page: None,
                    notes: vec![format!(
                        "this document's /EmbeddedFiles tree names no file called {name:?} — \
                         nothing was detached"
                    )],
                });
                return;
            }
            _ => {}
        }
        // What was *done*, rather than what was asked for: `Edit::Markup` names its target as
        // "what is selected", and a replay after the selection moved would mark up something
        // else. See `open::Done`.
        let Some(done) = open.resolve(edit, drag, dropped) else {
            return;
        };
        match standing {
            Standing::Refuse(_) => {}
            Standing::Proceed => commit(id, open, done, events),
            Standing::Warn(notes) => {
                commit(id, open, done, events);
                events.push(Event::Warned {
                    document: id,
                    operation,
                    notes,
                });
            }
            Standing::Ask(notes) => {
                open.asking = Some(crate::open::Held::Edit(done));
                events.push(Event::Asking {
                    document: id,
                    operation,
                    notes,
                });
            }
        }
    }

    /// [`Command::Answer`]: settles the edit [`Event::Asking`] held.
    ///
    /// Addressed by document rather than to the focused one, because the question named a
    /// document and a person may have turned to another window before answering.
    fn answer(&mut self, document: DocumentId, proceed: bool, events: &mut Vec<Event>) {
        let Some(open) = self.documents.get_mut(&document) else {
            return;
        };
        let Some(held) = open.asking.take() else {
            return;
        };
        // §7.6.4.2's bit 12 is the one question whose `no` is an answer and not a cancellation:
        // Table 22 makes bit 3 the permission to print and bit 12 only the permission to print
        // faithfully, so declining there asks for the degraded job the cell describes rather than
        // for no job at all (ADR 1203). Every other `no` forgets what was held.
        let answered_either_way = matches!(held, crate::open::Held::PrintFaithfully(_));
        if !proceed && !answered_either_way {
            // §12.11.6's question is the one whose `no` has something to undo: a document held
            // before it was processed was never opened, no `Event::Opened` named it, and leaving
            // it in the map would be this crate holding a document nobody can see or close.
            if matches!(held, crate::open::Held::Process { .. }) {
                self.documents.remove(&document);
            }
            return;
        }
        match held {
            crate::open::Held::Process { fragment } => {
                self.process(document, fragment.as_deref(), events);
            }
            crate::open::Held::Edit(done) => commit(document, open, done, events),
            // Nothing to commit: a copy changes no document. What the `yes` releases is the text
            // itself, taken at the moment the question was asked.
            crate::open::Held::Copy {
                logical,
                page_order,
            } => events.push(Event::Copied {
                document,
                logical,
                page_order,
            }),
            // Nothing to commit here either: a print operation changes no document. What the
            // `yes` releases is the intent, applied to the sheet the question named.
            // A `yes` to bit 3 releases the printing; bit 12 is a second question and is asked
            // here for the first time, because a person answering one had not been asked the
            // other (ADR 1203).
            crate::open::Held::PrintFaithfully(sheet) => {
                let fidelity = if proceed {
                    crate::Fidelity::Faithful
                } else {
                    crate::Fidelity::Degraded
                };
                Self::begin_printing(document, open, sheet, fidelity, events);
            }
            crate::open::Held::Print(sheet) => {
                let granted = Self::faithfulness(
                    document,
                    open,
                    self.restrictions.under(open.restrictions),
                    sheet,
                    events,
                );
                if let Some(fidelity) = granted {
                    Self::begin_printing(document, open, sheet, fidelity, events);
                }
            }
        }
    }

    /// [`Command::Print`]: §7.6.4.2's bit 3 asked as an operation, and the intent if it is granted.
    ///
    /// The two ends of §8.11.4.5's print operation. [`Printing::Start`] asks the policy and, on a
    /// yes, puts the focused document into print intent; [`Printing::Finish`] takes it back out,
    /// which is the clause's own requirement that the changes "persist only for the duration of
    /// the print operation". Finishing asks nothing: a reader who is allowed to start is not asked
    /// again for permission to stop, and a `Finish` with no job running is a host tidying up.
    fn print(&mut self, printing: Printing, events: &mut Vec<Event>) {
        use pdf_model::restriction::Operation;

        let Some(id) = self.focused else { return };
        let sheet = match printing {
            Printing::Start(sheet) => sheet,
            // A sheet the person chose after the grant. §12.5.6.22's media is the only thing it
            // changes, so the pages that already carry a watermark against the old one are
            // superseded and nothing else is.
            Printing::Paper(sheet) => {
                let Some(open) = self.documents.get_mut(&id) else {
                    return;
                };
                if open.printing.is_none() {
                    return;
                }
                open.printing = Some(sheet);
                open.view.set_paper(paper_of(sheet));
                open.stale();
                events.push(damage(self.viewport));
                return;
            }
            Printing::Finish => {
                let Some(open) = self.documents.get_mut(&id) else {
                    return;
                };
                Self::end_printing(open);
                events.push(damage(self.viewport));
                return;
            }
        };
        match self.standing(id, Operation::Print, None, None) {
            Standing::Refuse(refused) => events.push(refused),
            Standing::Proceed => {
                let policy = self.restrictions;
                let Some(open) = self.documents.get_mut(&id) else {
                    return;
                };
                let under = policy.under(open.restrictions);
                if let Some(fidelity) = Self::faithfulness(id, open, under, sheet, events) {
                    Self::begin_printing(id, open, sheet, fidelity, events);
                }
            }
            Standing::Warn(notes) => {
                let policy = self.restrictions;
                let Some(open) = self.documents.get_mut(&id) else {
                    return;
                };
                let under = policy.under(open.restrictions);
                let granted = Self::faithfulness(id, open, under, sheet, events);
                if let Some(fidelity) = granted {
                    Self::begin_printing(id, open, sheet, fidelity, events);
                }
                events.push(Event::Warned {
                    document: id,
                    operation: Operation::Print,
                    notes,
                });
            }
            Standing::Ask(notes) => {
                let Some(open) = self.documents.get_mut(&id) else {
                    return;
                };
                open.asking = Some(crate::open::Held::Print(sheet));
                events.push(Event::Asking {
                    document: id,
                    operation: Operation::Print,
                    notes,
                });
            }
        }
        events.push(damage(self.viewport));
    }

    /// §7.6.4.2's Table 22 bit 12, asked of a print bit 3 has already let through.
    ///
    /// `None` is the *ask* level: the question is out and the job waits on
    /// [`Command::Answer`], which is the only one of the four verdicts that does not start a
    /// print. The other three all start one, at the fidelity they decided:
    ///
    /// - **`Proceed`** — the document withholds nothing, or this reader said not to obey it.
    /// - **`Refuse`** — the document withholds bit 12 and this reader obeys it, so the job goes
    ///   ahead degraded and [`Event::Refused`] names the *fidelity* that was withheld. Table 22
    ///   is what makes a refusal and a grant arrive together: "[w]hen this bit is clear (and bit
    ///   3 is set), printing shall be limited to a low- level representation of the appearance,
    ///   possibly of degraded quality" — a limit on the job rather than the end of it.
    /// - **`Warn`** — the job is faithful and the document's reasons are said afterwards, which
    ///   is what that level means everywhere else in this crate.
    ///
    /// ADR 1203.
    fn faithfulness(
        id: DocumentId,
        open: &mut Open,
        policy: crate::RestrictionPolicy,
        sheet: crate::Sheet,
        events: &mut Vec<Event>,
    ) -> Option<crate::Fidelity> {
        use pdf_model::restriction::Operation;

        let operation = Operation::PrintFaithfully;
        let standing = Self::verdict_of(
            id,
            policy.level(operation).level(),
            &open.document,
            operation,
            None,
            None,
        );
        match standing {
            Standing::Proceed => Some(crate::Fidelity::Faithful),
            Standing::Refuse(refused) => {
                events.push(refused);
                Some(crate::Fidelity::Degraded)
            }
            Standing::Warn(notes) => {
                events.push(Event::Warned {
                    document: id,
                    operation,
                    notes,
                });
                Some(crate::Fidelity::Faithful)
            }
            Standing::Ask(notes) => {
                open.asking = Some(crate::open::Held::PrintFaithfully(sheet));
                events.push(Event::Asking {
                    document: id,
                    operation,
                    notes,
                });
                None
            }
        }
    }

    /// Puts one document into print intent, and says so.
    ///
    /// Three statements about the *output* rather than about the document, which is why they go
    /// through `ViewState` and nowhere else (`CLAUDE.md`'s rule 1): §8.11.4.5's event, which
    /// re-applies the `Print` usage application dictionaries over the states the reader has
    /// chosen; §12.5.6.22's target media; and, through the first of those, §12.5.3's device.
    /// Every page already interpreted was interpreted for a screen, so all of them are superseded
    /// — which is what `Open::stale` means and why a print operation redraws the window.
    ///
    /// `fidelity` is Table 22 bit 12's answer, decided by [`Self::faithfulness`] before this is
    /// reached and carried on the grant so that a host knows what its dialogue may offer.
    fn begin_printing(
        id: DocumentId,
        open: &mut Open,
        sheet: crate::Sheet,
        fidelity: crate::Fidelity,
        events: &mut Vec<Event>,
    ) {
        open.printing = Some(sheet);
        open.view
            .set_purpose(pdf_model::optional_content::Purpose::Print);
        open.view.set_paper(paper_of(sheet));
        open.stale();
        events.push(Event::Printing {
            document: id,
            pages: open.page_count,
            fidelity,
        });
    }

    /// Takes it back out — §8.11.4.5's "then all groups shall revert to their prior states".
    ///
    /// Idempotent, because a host that sends [`Printing::Finish`] twice, or once with no job
    /// running, has said nothing wrong.
    fn end_printing(open: &mut Open) {
        if open.printing.take().is_none() {
            return;
        }
        open.view
            .set_purpose(pdf_model::optional_content::Purpose::View);
        open.view.set_paper(None);
        open.stale();
    }

    /// [`Command::Copy`]: §7.6.4.2's bit 5 asked as an operation, and the text if it is granted.
    ///
    /// The selection is taken **before** the policy is asked and in both of §14.8.2.5's orders,
    /// for `crate::open::Done`'s reason: under [`crate::RestrictionLevel::Ask`] the answer may
    /// come after the person has dragged somewhere else, and what goes ahead on a `yes` has to be
    /// what they were asked about. Nothing selected is nothing copied and nothing said — a
    /// question about an empty copy would be a question about nothing.
    fn copy(&mut self, events: &mut Vec<Event>) {
        use pdf_model::restriction::Operation;

        let Some(id) = self.focused else { return };
        let Some(open) = self.focused() else { return };
        let Some(selected) = self.selected(open) else {
            return;
        };
        let page_order = selected.text.into_owned();
        if page_order.is_empty() {
            return;
        }
        let logical = Self::logical_selection(open);
        match self.standing(id, Operation::Extract, None, None) {
            Standing::Refuse(refused) => events.push(refused),
            Standing::Proceed => events.push(Event::Copied {
                document: id,
                logical,
                page_order,
            }),
            Standing::Warn(notes) => {
                events.push(Event::Copied {
                    document: id,
                    logical,
                    page_order,
                });
                events.push(Event::Warned {
                    document: id,
                    operation: Operation::Extract,
                    notes,
                });
            }
            Standing::Ask(notes) => {
                let Some(open) = self.documents.get_mut(&id) else {
                    return;
                };
                open.asking = Some(crate::open::Held::Copy {
                    logical,
                    page_order,
                });
                events.push(Event::Asking {
                    document: id,
                    operation: Operation::Extract,
                    notes,
                });
            }
        }
    }

    /// Moves the page under the viewport, clamped to the raster the host is holding.
    ///
    /// A method rather than an arm of [`Self::act`] because the clamp needs the viewport *and*
    /// the focused document at once, and the borrow that gets one has to end before the other.
    fn scroll(&mut self, dx: f32, dy: f32, events: &mut Vec<Event>) {
        let viewport = self.viewport;
        let Some(open) = self.focused_mut() else {
            return;
        };
        open.scroll = (open.scroll.0 + dx, open.scroll.1 + dy);
        // Clamped — and, under a continuous arrangement, carried across the row boundary — in
        // `settle`, which is where the magnification the arrangement is measured in is known.
        events.push(damage(viewport));
    }

    /// The one place a document's restrictions are consulted, and it is asked per **operation**.
    ///
    /// Once per thing a person did, rather than once per widget the value lands on: §12.7.4.1
    /// makes one field's value shared by all of its widgets, and a question asked per widget
    /// would ask a person the same question three times about one keystroke. That is the shape
    /// *ask* and *warn* need, and it is why the check moved out of `ViewState::set_field` — where
    /// it could only ever say "nothing happened" — into the crate that holds the policy.
    ///
    /// [`Standing::Proceed`] means go ahead: either the document asserts nothing against the
    /// operation, or this reader has turned its restrictions off, which `CLAUDE.md` says shall
    /// always be possible. The two are one answer here and two answers to a person, which is
    /// what [`crate::Event::Refused`] carries the operation for.
    ///
    /// **The policy is `pdf_model::restriction::decide`'s, asked here once and matched
    /// exhaustively**, and since the eight-hundred-and-eighty-fifth session all four of its
    /// verdicts have an arm of their own — the two this used to fold into a refusal are
    /// [`Event::Asking`] and [`Event::Warned`], and the notes are worded for each (ADR 0803, ADR
    /// 0814).
    fn standing(
        &self,
        id: DocumentId,
        operation: pdf_model::restriction::Operation,
        field: Option<&str>,
        annotation: Option<ObjectId>,
    ) -> Standing {
        let Some(open) = self.focused() else {
            return Standing::Proceed;
        };
        Self::verdict_of(
            id,
            // The window's levels as this document sees them: `under` is the whole of the
            // layering and is asked here rather than composed by a caller (ADR 1145).
            self.restrictions
                .under(open.restrictions)
                .level(operation)
                .level(),
            &open.document,
            operation,
            field,
            annotation,
        )
    }

    /// `pdf_model::restriction::decide` at a stated level, worded.
    ///
    /// Separate from [`Self::standing`] because §12.11.6 is asked of a document that is not in
    /// this viewer yet: the level is the window's, there is nothing focused to read it from, and
    /// the wording of all four verdicts must still be the one wording (ADR 1167).
    fn verdict_of(
        id: DocumentId,
        level: pdf_model::restriction::Level,
        document: &pdf_syntax::Document,
        operation: pdf_model::restriction::Operation,
        field: Option<&str>,
        annotation: Option<ObjectId>,
    ) -> Standing {
        use crate::notes::{Standing as Worded, restricted};
        use pdf_model::restriction::Verdict;
        match pdf_model::restriction::decide(level, document, operation, field, annotation) {
            Verdict::Proceed => Standing::Proceed,
            Verdict::Refuse(restrictions) => Standing::Refuse(Event::Refused {
                document: id,
                operation,
                notes: restricted(operation, &restrictions, Worded::Refused),
            }),
            Verdict::Warn(restrictions) => {
                Standing::Warn(restricted(operation, &restrictions, Worded::Warned))
            }
            Verdict::Ask(restrictions) => {
                Standing::Ask(restricted(operation, &restrictions, Worded::Asked))
            }
        }
    }

    /// Moves the log's cursor, which is what undo and redo are.
    fn move_cursor(&mut self, by: isize, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused_mut() else {
            return;
        };
        let Some(cursor) = open.cursor.checked_add_signed(by) else {
            return;
        };
        if cursor > open.log.len() || cursor == open.cursor {
            return;
        }
        let before = open.dirty();
        let crossed = open.cursor.min(cursor)..open.cursor.max(cursor);
        let attachments = open
            .log
            .get(crossed)
            .is_some_and(|entries| entries.iter().any(crate::open::Done::moves_attachments));
        open.cursor = cursor;
        open.replay();
        if open.dirty() != before {
            events.push(Event::Dirty {
                document: id,
                dirty: open.dirty(),
            });
        }
        if attachments {
            events.push(Event::AttachmentsChanged { document: id });
        }
    }

    /// [`Command::Report`]: what the focused document says about itself, in words.
    ///
    /// **The one place `notes::about` is called, and it is not the open path.** Its eight clauses
    /// are claims about the *file* — §12.8's signatures above all, whose answer reads and digests
    /// the signed byte ranges — and `CLAUDE.md` principle 2 defers what page one does not need
    /// until something asks. This is the asking; [`crate::open::Open::about`] is the `OnceCell`
    /// that makes a second asking free. ADR 1044.
    ///
    /// Nothing at all where nothing is open, and nothing where the document has nothing to say:
    /// an empty report is not a sentence, and a host that printed one would be telling a reader
    /// something about a file that said nothing.
    fn report_the_document(&self, events: &mut Vec<Event>) {
        let (Some(id), Some(open)) = (self.focused, self.focused()) else {
            return;
        };
        let notes = open.about(&self.trust);
        if !notes.is_empty() {
            events.push(Event::Reported {
                document: id,
                page: None,
                notes: notes.to_vec(),
            });
        }
    }

    /// Annex O's `search`, and a find bar's *next*: one page of a document-wide search.
    ///
    /// The three verbs of [`Find`] fall into two: `Stop` throws the plan away, and the other two
    /// end in exactly one step being taken — `Start` after making a new plan, `Continue` against
    /// the one already there. A `Continue` with nothing in progress does nothing and says
    /// nothing, which is the right answer for a host that pumped once too often.
    fn find(&mut self, find: Find, events: &mut Vec<Event>) {
        let Some(id) = self.focused else {
            return;
        };
        match find {
            Find::Stop => {
                if let Some(open) = self.focused_mut() {
                    open.searching = None;
                }
                return;
            }
            Find::Start { needle, direction } => {
                let Some(open) = self.focused_mut() else {
                    return;
                };
                // Where the *next* one is after. The far end of what is selected in the
                // direction of travel, so a search started while sitting on an occurrence moves
                // off it rather than finding it again — and with nothing selected, the whole
                // page in that direction. `usize::MAX` going backwards is the end of any
                // readback there could be, which is what "before nothing in particular" means.
                // Only where the selection is on the page a search starts from, which is the
                // current one: an offset into another page's readback would name a place in this
                // one that nobody selected.
                let here = open
                    .selection
                    .map(Chosen::ordered)
                    .and_then(|(first, last)| {
                        (first.page <= open.page_index && open.page_index <= last.page)
                            .then_some((first, last))
                    });
                let from = match direction {
                    // The end of what is selected *on this page*: a selection that runs on past
                    // the page the search starts from has its far end in another readback, and
                    // the whole of this one is what lies ahead here.
                    FindDirection::Forward => here.map_or(0, |(_, last)| {
                        if last.page == open.page_index {
                            last.offset
                        } else {
                            usize::MAX
                        }
                    }),
                    FindDirection::Backward => here.map_or(usize::MAX, |(first, _)| {
                        if first.page == open.page_index {
                            first.offset
                        } else {
                            0
                        }
                    }),
                };
                let pages = open.page_count;
                open.searching = crate::search::Searching::new(
                    vec![needle],
                    direction,
                    (open.page_index, from),
                    pages,
                    true,
                );
                if open.searching.is_none() {
                    // An empty string, or a document with no pages: there is no answer to give,
                    // and a host waiting for one has to be told so rather than left pumping.
                    events.push(Event::Searched {
                        document: id,
                        found: None,
                        remaining: 0,
                        wrapped: false,
                    });
                    return;
                }
            }
            Find::Continue => {}
        }
        self.find_step(id, events);
    }

    /// A page's readback, out of the cache where it is there and by interpreting it where it
    /// is not.
    ///
    /// **This is the whole cost of a search step.** `interpret` is the expensive half of this
    /// program — 5.4 ms a page over ISO 32000-2's 1023, of which §7.7.3.2's page tree is 19% and
    /// the content stream 81% — and until the four-hundred-and-twentieth session every step paid
    /// it, including for a page a search had read a moment before. `crate::readback` is the
    /// bound that made keeping it acceptable; a **repeated** full-document sweep of ISO 32000-2
    /// is 5.45 s without this and 7.27 ms with it, medians of seven runs of
    /// `viewer-core/examples/find_cost`, and the *first* sweep is unchanged.
    ///
    /// `None` is a page this program could not interpret at all, which is not the same as a page
    /// with no text on it and is not cached: a failure costs nothing to reproduce and caching it
    /// would put a second meaning into a map whose entries are otherwise readbacks.
    fn readback(&mut self, id: DocumentId, page: usize) -> Option<Arc<str>> {
        if let Some(open) = self.documents.get_mut(&id)
            && let Some(text) = open.readbacks.get(page)
        {
            return Some(text);
        }
        let open = self.documents.get(&id)?;
        let read = crate::open::interpret(open, page)?;
        let text: Arc<str> = Arc::from(read.interpretation.text);
        if let Some(open) = self.documents.get_mut(&id) {
            open.readbacks.put(page, &text);
        }
        Some(text)
    }

    /// Reads the one page the search is on, and says what it found there.
    ///
    /// Split from [`Self::find`] because the borrow is: interpreting a page needs `&Open` and
    /// stepping the plan needs `&mut`, and doing both in one function would mean holding the
    /// first across the second.
    fn find_step(&mut self, id: DocumentId, events: &mut Vec<Event>) {
        let Some(page) = self
            .documents
            .get(&id)
            .and_then(|open| open.searching.as_ref())
            .filter(|searching| !searching.is_done())
            .map(crate::search::Searching::page)
        else {
            return;
        };
        let text = self.readback(id, page);
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        let Some(searching) = open.searching.as_mut() else {
            return;
        };
        // A page this program cannot interpret is stepped over rather than treated as a page with
        // no text on it: the reports for it are the render path's business, and a search that
        // stopped at the first damaged page would be a worse answer than one that did not.
        let found = if let Some(text) = text.as_deref() {
            searching.step(text)
        } else {
            searching.skip();
            None
        };
        let (remaining, wrapped) = (searching.remaining, searching.wrapped);
        let Some(range) = found else {
            if remaining == 0 {
                open.searching = None;
            }
            events.push(Event::Searched {
                document: id,
                found: None,
                remaining,
                wrapped,
            });
            return;
        };
        open.searching = None;
        // "[S]electing the first matching word in the document" — the annex's own verb, and this
        // crate has exactly one thing that means: the range a host draws its selection over. It
        // waits for the page because the turn below ends the selection that was on the old one.
        open.pending_selection = Some(Chosen::in_page(page, range));
        let viewport = self.viewport;
        self.act(Command::GoTo(PageTarget::Index(page)), events);
        events.push(Event::Searched {
            document: id,
            found: Some(Found { page, range }),
            remaining: 0,
            wrapped,
        });
        events.push(damage(viewport));
    }

    /// Every occurrence of `needle` on the page being shown, as shapes in device pixels.
    fn found(&self, open: &Open, needle: &str) -> Vec<Vec<[f32; 8]>> {
        let Some(interpreted) = open.interpreted() else {
            return Vec::new();
        };
        crate::select::find(&interpreted.text, needle)
            .into_iter()
            .map(|range| self.device_quads(open, open.page_index, range))
            .collect()
    }

    /// What is selected, with its shapes in device pixels.
    ///
    /// **One page's selection still borrows that page's readback**, which is not an optimisation
    /// but the identity `selection_census` asserts: `Selection::All` is `Interpretation::text`
    /// byte for byte, and a borrowed slice of it cannot be anything else. A selection that
    /// crosses a page boundary has no such slice to be — the bytes are in two readbacks — so it
    /// is the one case that allocates, and [`Selected::text`] says which of the two a host has.
    ///
    /// The pages are joined by a newline. No clause states anything about it: §12.4.2 has no
    /// notion of a document-wide string and the page break is not a character in either page's
    /// readback, so this is a choice, made because a paragraph that ends at the foot of a page
    /// reads back as a line ending there.
    fn selected<'a>(&self, open: &'a Open) -> Option<Selected<'a>> {
        let spans = open.spans();
        let ((first, range), rest) = spans.split_first()?;
        let mut quads = self.device_quads(open, *first, *range);
        let head = open
            .interpretation(*first)?
            .text
            .get(range.0..range.1)
            .unwrap_or_default();
        if rest.is_empty() {
            return Some(Selected {
                text: std::borrow::Cow::Borrowed(head),
                quads,
            });
        }
        let mut text = String::from(head);
        for (page, range) in rest {
            text.push('\n');
            if let Some(interpreted) = open.interpretation(*page) {
                text.push_str(interpreted.text.get(range.0..range.1).unwrap_or_default());
            }
            quads.extend(self.device_quads(open, *page, *range));
        }
        Some(Selected {
            text: std::borrow::Cow::Owned(text),
            quads,
        })
    }

    /// The focused annotation's `/Rect` on the screen, for a host that draws a focus ring.
    ///
    /// The same mapping [`Self::device_quads`] uses and deliberately not a second copy of it in a
    /// host: the origin, the magnification and the y flip are exactly the arithmetic that was
    /// wrong for seventy-five sessions (ADR 0118), and a ring drawn from a host's own guess at it
    /// would be that defect again one crate over.
    fn focus_quad(&self, open: &Open) -> Option<(ObjectId, [f32; 8])> {
        let object = open.focus?;
        // Which page of the arrangement the focused annotation is on: the current one under
        // `SinglePage`, and under a column whichever placed page carries it — a ring drawn
        // against the wrong page's origin would be a ring in the wrong place.
        let on = open
            .on_screen
            .iter()
            .find(|on_screen| annotations_of(open, &on_screen.object).contains(&object))
            .map_or(open.page_index, |on_screen| on_screen.page);
        let resolved = open.document.get(object);
        let dict = resolved.as_dict()?;
        let rect = open.document.get_key(dict, "Rect");
        let array = rect.as_array()?;
        let mut values = [0.0_f32; 4];
        for (slot, entry) in values.iter_mut().zip(array.iter()) {
            // A rectangle whose numbers are not numbers is not a rectangle, and a page
            // coordinate that does not fit an `f32` is a page nothing can draw.
            #[expect(
                clippy::cast_possible_truncation,
                reason = "see the comment above: the display list is f32 throughout"
            )]
            {
                *slot = open.document.resolve(entry).as_number()? as f32;
            }
        }
        let (x0, x1) = (values[0].min(values[2]), values[0].max(values[2]));
        let (y0, y1) = (values[1].min(values[3]), values[1].max(values[3]));
        Some((object, self.device_quad(open, on, [x0, y0, x1, y1])?))
    }

    /// ISO 32000-2 Annex O's highlighted rectangles that are on the page being shown.
    ///
    /// Table Annex O.4 measures each "from the top left corner of the page", so a rectangle
    /// belongs to one page and answering for another would put it over the wrong ink. The pages
    /// that are not showing are dropped rather than reported: the fragment stated the rectangle
    /// once, and turning to page 7 is not a question about the rectangle on page 3.
    ///
    /// Empty rather than absent where there is nothing, which is [`Answer::Found`]'s convention
    /// for the same kind of answer: a host draws a list, and a list of none draws nothing.
    fn highlight_quads(&self, open: &Open) -> Vec<[f32; 8]> {
        open.highlights
            .iter()
            .filter(|highlighted| open.on(highlighted.page).is_some())
            .filter_map(|highlighted| self.device_quad(open, highlighted.page, highlighted.rect))
            .collect()
    }

    /// §12.5.6.14's open popup windows, placed on the screen.
    ///
    /// The state is the file's `/Open` unless a person has said otherwise since, which is what
    /// `Open::popups` holds and why Table 186's word "initially" is load-bearing.
    fn popup_windows(&self, open: &Open) -> Vec<PopupWindow> {
        // Every page the arrangement shows rather than the current one alone: a window belongs to
        // the page its annotation is on, and a `OneColumn` reader looking at two pages would
        // otherwise see the notes of one of them.
        open.on_screen
            .iter()
            .flat_map(|on_screen| {
                pdf_model::popup::popups(&open.document, &on_screen.object, &open.view)
                    .into_iter()
                    .filter(|popup| open.popup_is_open(popup))
                    .filter_map(|popup| {
                        Some(PopupWindow {
                            annotation: popup.annotation,
                            parent: popup.parent,
                            quad: self.device_quad(open, on_screen.page, popup.rect)?,
                            title: popup.title,
                            text: popup.text,
                            modified: popup.modified,
                            colour: popup.colour,
                            subject: popup.subject,
                            created: popup.created,
                            replies: popup.replies,
                        })
                    })
                    .collect::<Vec<PopupWindow>>()
            })
            .collect()
    }

    /// §12.7's fields with a widget on the page being shown, placed on the screen.
    ///
    /// The description is `pdf_model::form`'s and the arithmetic is [`Self::device_quad`]'s, which
    /// is the same split [`Self::popup_windows`] makes: what a field is belongs to the crate that
    /// reads documents, and where it lands on a screen is this view's. A widget whose rectangle
    /// cannot be placed — no page is interpreted, or the magnification is not yet known — is left
    /// out rather than given a guessed quadrilateral, and a field left with no widget at all is
    /// left out with them, because a control with nowhere to go is not one a host can place.
    fn form_fields(&self, open: &Open) -> Vec<crate::FormField> {
        // Every page the arrangement shows, for [`Self::popup_windows`]'s reason: a host that
        // places real controls over the page places them over every page it is drawing.
        open.on_screen
            .iter()
            .flat_map(|on_screen| {
                pdf_model::form::fields(&open.document, &on_screen.object, &open.view)
                    .into_iter()
                    .filter_map(|field| {
                        let widgets: Vec<crate::FormWidget> = field
                            .widgets
                            .into_iter()
                            .filter_map(|widget| {
                                Some(crate::FormWidget {
                                    annotation: widget.annotation,
                                    quad: self.device_quad(open, on_screen.page, widget.rect)?,
                                    on_state: widget.on_state,
                                    export: widget.export,
                                    on: widget.on,
                                })
                            })
                            .collect();
                        if widgets.is_empty() {
                            return None;
                        }
                        Some(crate::FormField {
                            name: field.name,
                            partial: field.partial,
                            control: field.control,
                            value: field.value,
                            read_only: field.read_only,
                            required: field.required,
                            no_export: field.no_export,
                            widgets,
                        })
                    })
                    .collect::<Vec<crate::FormField>>()
            })
            .collect()
    }

    /// A rectangle in default user space, as the quadrilateral a host draws over the page.
    ///
    /// **One copy of this arithmetic and deliberately not one per caller**: the origin, the
    /// magnification and the y flip are exactly what was wrong for seventy-five sessions (ADR
    /// 0118), and a second opinion about them in a host — or in a second method here — would be
    /// that defect again. `[x0, y0, x1, y1]`, normalised, in; clockwise from the top-left as it
    /// appears on the screen, out — for a page §7.7.3.3 does not turn, where the corner that is
    /// lowest and leftmost in the document is also the one at the bottom left of the screen.
    fn device_quad(&self, open: &Open, page: usize, rect: [f32; 4]) -> Option<[f32; 8]> {
        let [x0, y0, x1, y1] = rect;
        let corners = [(x0, y1), (x1, y1), (x1, y0), (x0, y0)];
        let mut out = [0.0_f32; 8];
        for (corner, place) in corners.iter().zip(out.chunks_exact_mut(2)) {
            let (x, y) = self.device_point(open, page, *corner)?;
            place[0] = x;
            place[1] = y;
        }
        Some(out)
    }

    /// A point in **default user space**, in device pixels of the viewport.
    ///
    /// Two maps composed, and the second is the one a caller cannot be asked to redo: §7.7.3.3's
    /// `/Rotate` and the crop box's own origin, which `content::page_space_at` applies, and then
    /// the centring, the magnification and the y flip, which `on_screen` does. It is exactly
    /// [`Self::user_space`] backwards — that function undoes the second and then the first — and
    /// writing it as anything else is how the two halves come to disagree.
    ///
    /// **The page transform was missing here until the three-hundred-and-seventy-first session**,
    /// so `Query::Focus`'s ring and `Query::Popups`' windows were placed as though every page were
    /// unrotated with its crop box at the origin. Both are true of every corpus document that has
    /// a widget — none of the 974 states a rotated page with one — which is why no gate saw it and
    /// why this is arithmetic rather than a picture (ADR 0211).
    fn device_point(&self, open: &Open, page: usize, at: (f32, f32)) -> Option<(f32, f32)> {
        let object = open.placed_page(page)?;
        let (x, y) = pdf_model::content::page_space_at(object, at.0, at.1);
        self.on_screen(open, page, (x, y))
    }

    /// A point in the **display list's** space, in device pixels of the viewport.
    ///
    /// The inverse of [`Self::page_point`], and the arithmetic every shape this crate hands over
    /// goes through: the text layer's quadrilaterals are already in this space, and a rectangle
    /// the *document* states reaches it through [`Self::device_point`].
    fn on_screen(&self, open: &Open, page: usize, at: (f32, f32)) -> Option<(f32, f32)> {
        let placed = open.on(page)?;
        let magnification = open.magnification(self.viewport, self.scale)?;
        let height = open.page_size(page).map(|size| size.height)?;
        Some((
            placed.origin.0 + at.0 * magnification,
            placed.origin.1 + (height - at.1) * magnification,
        ))
    }

    /// Where the caret sits in the field at a viewport point, in device pixels.
    ///
    /// The model answers in default user space — §12.7.4.3's layout is where the next glyph's
    /// position comes from, and `ViewState::caret_at` is what asks it — and this is the one
    /// mapping onto the screen that every other shape here already goes through.
    fn caret(
        &self,
        open: &Open,
        at: (f32, f32),
        offset: usize,
    ) -> Option<((f32, f32), (f32, f32))> {
        let (page, (x, y)) = self.user_space(open, at)?;
        let object = open.placed_page(page)?;
        let segment = open.view.caret_at(&open.document, object, x, y, offset)?;
        let from = self.device_point(open, page, (segment[0], segment[1]))?;
        let to = self.device_point(open, page, (segment[2], segment[3]))?;
        Some((from, to))
    }

    /// How far into the field at a viewport point another viewport point is, in bytes.
    ///
    /// [`Self::caret`] backwards, and it goes through the same two maps in the same order: the
    /// viewport point becomes a point in default user space, and §12.7.4.3's own layout is what
    /// turns that into an offset. Both points are mapped, because the second is where the pointer
    /// is now and the first is only what names the field.
    fn offset(&self, open: &Open, at: (f32, f32), point: (f32, f32)) -> Option<usize> {
        let (page, (x, y)) = self.user_space(open, at)?;
        // The moving point is measured against the *field's* page: a drag that has left the
        // widget is still a drag inside its value, which is what the second point is for.
        let (_, point) = self.user_space(open, point)?;
        let object = open.placed_page(page)?;
        open.view.offset_at(&open.document, object, (x, y), point)
    }

    /// The shapes covering a range of the value of the field at a viewport point.
    ///
    /// The model answers in default user space, one quadrilateral per line, and this is the same
    /// mapping onto the screen every other shape here goes through — [`Self::device_point`] per
    /// corner rather than [`Self::device_quad`], because the corners have already been turned by
    /// whatever Table 192's `/R` and §12.5.5's placement do and a rectangle could not carry that.
    fn field_selection(
        &self,
        open: &Open,
        at: (f32, f32),
        range: (usize, usize),
    ) -> Option<Vec<[f32; 8]>> {
        let (page, (x, y)) = self.user_space(open, at)?;
        let object = open.placed_page(page)?;
        let quads = open
            .view
            .field_selection(&open.document, object, (x, y), range)?;
        quads
            .into_iter()
            .map(|quad| {
                let mut out = [0.0_f32; 8];
                for (corner, place) in quad.chunks_exact(2).zip(out.chunks_exact_mut(2)) {
                    let (x, y) = self.device_point(open, page, (corner[0], corner[1]))?;
                    place[0] = x;
                    place[1] = y;
                }
                Some(out)
            })
            .collect()
    }

    /// §12.5.1's tab order, applied: the focus moves to the next or previous annotation.
    ///
    /// The order is `pdf_model::tab_order::order`, which is Table 31's `/Tabs` and all five of
    /// its values; what is decided *here* is the two things the clause leaves to an interface —
    /// that the move wraps, and that §12.6.3's `/Bl` and `/Fo` are raised in Table 197's own
    /// order, one thing losing the focus before the next receives it, exactly as a press does.
    ///
    /// **`/Fo` and `/Bl` still fire for widgets only**, because Table 197 says so of both, while
    /// the *focus itself* moves through every annotation the page has, because §12.5.1 says "the
    /// annotations on a page" without qualification and Table 31's `W` exists to name the
    /// narrower order.
    fn move_focus(&mut self, move_to: crate::command::FocusMove, events: &mut Vec<Event>) {
        let (Some(id), Some(open)) = (self.focused, self.focused_mut()) else {
            return;
        };
        let wants = match move_to {
            crate::command::FocusMove::None => None,
            direction => {
                let Some(page) = open.shown_page() else {
                    return;
                };
                let Some(page_id) =
                    page_object(&pdf_model::Pages::new(&open.document), open.page_index)
                else {
                    return;
                };
                let order = pdf_model::tab_order::order(&open.document, page, page_id);
                match direction {
                    crate::command::FocusMove::Previous => {
                        pdf_model::tab_order::previous(&order, open.focus)
                    }
                    _ => pdf_model::tab_order::next(&order, open.focus),
                }
            }
        };
        if open.focus == wants {
            return;
        }
        let mut raised = Vec::new();
        raised.extend(
            open.focus
                .filter(|left| is_widget(&open.document, *left))
                .map(|left| (left, Trigger::Blur)),
        );
        raised.extend(
            wants
                .filter(|got| is_widget(&open.document, *got))
                .map(|got| (got, Trigger::Focus)),
        );
        open.focus = wants;
        let viewport = self.viewport;
        self.raise(id, raised, None, events);
        // A focus ring is chrome the host draws, so what changed is the viewport rather than the
        // page — the same statement a selection makes, and for the same reason.
        events.push(damage(viewport));
    }

    /// The selection in §14.8.2.5's logical content order, where the page states one.
    ///
    /// Walks the structure tree, so it is deliberately not on the path a drag takes — see
    /// [`Query::LogicalSelection`]. The page's object is needed because §14.7's tree is the
    /// document's and its content items name the page they are on.
    fn logical_selection(open: &Open) -> Option<String> {
        let spans = open.spans();
        if spans.is_empty() {
            return None;
        }
        let tree = pdf_model::structure::Tree::of(&open.document)?;
        let pages = pdf_model::Pages::new(&open.document);
        let mut ordered: Vec<String> = Vec::with_capacity(spans.len());
        for (index, range) in spans {
            let interpreted = open.interpretation(index)?;
            // As `accessibility` does, and for the same reason: Table 355's `/Pg` names a page
            // *object* and what this crate holds is an index.
            let page = page_object(&pages, index)?;
            // **A page whose tree does not reach the range takes the whole answer with it**, the
            // same way a range the tree half-reaches always has: a copy that silently dropped one
            // page of a selection crossing a boundary would be worse than one handing back
            // content order for all of it.
            ordered.push(tree.logical_range(
                &open.document,
                page,
                &interpreted.text,
                &interpreted.marked,
                range.0..range.1,
            )?);
        }
        Some(ordered.join("\n"))
    }

    /// §14.7's structure tree for every page the arrangement shows, with §14.9's entries applied.
    ///
    /// **One tree per page rather than one for the screen**, which is [`Query::AccessibilityTree`]'s
    /// own note: §14.7's structure is the document's, the route to a page's elements is
    /// §14.7.5.4's structural parent tree keyed by *that page's* `/StructParents`, and two pages'
    /// answers share their ancestors — so they are answered side by side and joining them is the
    /// platform's question rather than this crate's.
    fn accessibility(&self, open: &Open) -> Vec<PageStructure> {
        open.on_screen
            .iter()
            .map(|on_screen| PageStructure {
                page: on_screen.page,
                nodes: self.structure(open, on_screen),
            })
            .collect()
    }

    /// One page's elements, as [`crate::AccessibilityNode`]s indexed within that page.
    ///
    /// Built on demand rather than kept, because a screen reader asks when it attaches and on a
    /// page change, and no other consumer asks at all — while a drag asks
    /// [`Query::Selection`] sixty times a second, which is why *that* one's inputs are cached.
    ///
    /// **Everything about the page comes from the arrangement's own entry**, and that is the
    /// answer to what a column costs: [`crate::open::OnScreen::object`] is the page ADR 0124
    /// cached to keep `Pages::get`'s tree walk — 3.8 ms on ISO 32000-2's thousandth page — off
    /// the paths a person drives. Asking the page tree once per page on the screen would have put
    /// several of those walks behind one question.
    fn structure(
        &self,
        open: &Open,
        on_screen: &crate::open::OnScreen,
    ) -> Vec<crate::AccessibilityNode> {
        let Some(interpreted) = on_screen.interpreted.as_ref() else {
            return Vec::new();
        };
        // Table 355's `/Pg` names a page *object*, and what this crate holds is an index.
        let Some(page) = on_screen.object.id else {
            return Vec::new();
        };
        let index = on_screen.page;
        let gathered =
            crate::accessibility::nodes(&open.document, page, interpreted.language.as_deref());
        // §14.7.5.3's object references, answered once for the page. Both readings are skipped
        // entirely where no element states one, which is nearly every page: `annotation_rectangles`
        // walks `/Annots` and `form::fields` walks §12.7.4.1's field tree, and neither is worth
        // paying for an answer nothing will look at.
        let referenced = gathered
            .iter()
            .any(|(_, element)| !element.objects.is_empty());
        let referenced = if referenced {
            referenced_objects(open, &on_screen.object)
        } else {
            Referenced::default()
        };
        let page = crate::accessibility::Readback {
            text: &interpreted.text,
            marked: &interpreted.marked,
            described: &interpreted.described,
            places: &referenced.places,
            languages: &referenced.languages,
            controls: &referenced.controls,
        };
        gathered
            .into_iter()
            .map(|(parent, gathered)| {
                crate::accessibility::finish(
                    gathered,
                    parent,
                    &page,
                    |start, end| self.device_quads(open, index, (start, end)),
                    |ranges| self.device_lines(open, index, ranges),
                    |rect| self.device_rect(open, index, rect),
                    |rect| self.device_marks(open, index, rect),
                )
            })
            .collect()
    }

    /// A rectangle in the **display list's** space, in device pixels of the viewport.
    ///
    /// §14.8.3.3's content rectangle is the caller this exists for, and it is one transform
    /// *shorter* than [`Self::device_rect`]: what the interpreter unioned are the commands' own
    /// bounds, which are already past §7.7.3.3's `/Rotate` and the crop box's origin because the
    /// display list is. So this is [`Self::device_quads`]'s arithmetic over two corners — the
    /// magnification, the centring and the y flip — and nothing else.
    ///
    /// The corners are sorted after the flip rather than assumed to keep their order, for
    /// [`Self::device_rect`]'s reason: exchanging the y coordinates exchanges which corner is the
    /// minimum.
    ///
    /// **The page is not applied as a limit here**, and that is the difference from
    /// [`Self::device_rect`]. That one clips because Table 379's rectangle is a *claim* a producer
    /// made and §14.11.2.1 says how much of the page can be looked at; this one is the union of
    /// commands that were actually put in the display list, each already narrowed by its own clip,
    /// and a backend applies the page boundary to those same commands. Clipping twice would be
    /// this crate second-guessing what it drew.
    ///
    /// `None` when the page is not on the screen, which is when its origin and magnification
    /// cannot be read.
    fn device_marks(&self, open: &Open, page: usize, rect: [f32; 4]) -> Option<[f32; 4]> {
        let magnification = open.magnification(self.viewport, self.scale)?;
        let height = open.page_size(page).map(|size| size.height)?;
        let origin = open.on(page).map(|on_screen| on_screen.origin)?;
        let x0 = origin.0 + rect[0] * magnification;
        let x1 = origin.0 + rect[2] * magnification;
        let y0 = origin.1 + (height - rect[1]) * magnification;
        let y1 = origin.1 + (height - rect[3]) * magnification;
        Some([x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)])
    }

    /// A rectangle stated in **default user space**, in device pixels of the viewport.
    ///
    /// Table 379's `/BBox` is the caller this exists for, and it is one transform longer than
    /// [`Self::device_quads`]: a selection's shapes are already in the display list's space, and
    /// this one starts where §12.5.2's annotation rectangles do — before §7.7.3.3's `/Rotate` and
    /// before the crop box's origin. [`pdf_model::content::page_space_at`] is that first step and
    /// [`Self::user_space`]'s inverse; the rest is the magnification, the centring and the y flip
    /// [`Self::device_quads`] applies, which is why it is applied here rather than composed into
    /// the page's transform (trap 12a, ADR 0118).
    ///
    /// The two mapped corners are sorted afterwards rather than assumed to keep their order: a
    /// `/Rotate 180` page maps a lower-left corner onto an upper-right one, and the y flip
    /// exchanges them again.
    ///
    /// # Why the page is the limit
    ///
    /// The result is intersected with the page, and that is a reading of the clause rather than a
    /// tidy-up. §14.8.5.4.3 makes the rectangle one that "completely encloses its **visible**
    /// content", and §14.11.2.1 says what of a page can be seen: the crop box "defines the region
    /// to which the contents of the page shall be clipped (cropped) when displayed or printed".
    /// So a rectangle beyond the page encloses nothing anybody can look at, and the part of it
    /// that is on the page is the whole of what the attribute can be describing.
    ///
    /// It is not hypothetical: **8 of the corpus's 132 stated rectangles reach outside their own
    /// page** (`pdf-model --example element_bounds_census`), and `doc/PDF20_AN001-BPC.pdf` states
    /// `[-32768 -32768 32767 32767]` for a figure — the whole representable plane, which is a
    /// producer writing "somewhere" rather than a bounding box. Handed on unclipped, that is a
    /// node covering fifty thousand pixels in every direction and winning every hit test on the
    /// page.
    ///
    /// `None` where the rectangle does not meet the page at all, because then the document has
    /// said nothing about where on this page the element is. A rectangle that merely touches an
    /// edge crosses as a degenerate one: §7.9.5's own NOTE is that "[r]ectangles can have a width
    /// of zero or height of zero".
    fn device_rect(&self, open: &Open, page: usize, rect: [f32; 4]) -> Option<[f32; 4]> {
        let magnification = open.magnification(self.viewport, self.scale)?;
        let size = open.page_size(page)?;
        let origin = open.on(page)?.origin;
        let object = open.placed_page(page)?;
        let first = pdf_model::content::page_space_at(object, rect[0], rect[1]);
        let second = pdf_model::content::page_space_at(object, rect[2], rect[3]);
        let (x0, x1) = (first.0.min(second.0), first.0.max(second.0));
        let (y0, y1) = (first.1.min(second.1), first.1.max(second.1));
        if x1 < 0.0 || y1 < 0.0 || x0 > size.width || y0 > size.height {
            return None;
        }
        let (x0, x1) = (x0.max(0.0), x1.min(size.width));
        let (y0, y1) = (y0.max(0.0), y1.min(size.height));
        // The y flip exchanges the two edges: the page's top is raster row zero.
        Some([
            origin.0 + x0 * magnification,
            origin.1 + (size.height - y1) * magnification,
            origin.0 + x1 * magnification,
            origin.1 + (size.height - y0) * magnification,
        ])
    }

    /// The shapes covering a range of the readback, in device pixels of the viewport.
    ///
    /// The mapping a host would otherwise have to do, and the reason it does not: it would mean
    /// re-deriving the magnification, the centring and the y flip, which is exactly the
    /// arithmetic ADR 0118 found wrong in the one place it existed.
    fn device_quads(&self, open: &Open, page: usize, range: (usize, usize)) -> Vec<[f32; 8]> {
        let Some(interpreted) = open.interpretation(page) else {
            return Vec::new();
        };
        let Some(magnification) = open.magnification(self.viewport, self.scale) else {
            return Vec::new();
        };
        let Some(height) = open.page_size(page).map(|size| size.height) else {
            return Vec::new();
        };
        let Some(origin) = open.on(page).map(|on_screen| on_screen.origin) else {
            return Vec::new();
        };
        // [`Self::on_screen`]'s formula, inlined over four corners at a time: this is asked sixty
        // times a second during a drag and may answer with hundreds of quadrilaterals, so the
        // magnification, the page height and the origin are read once per call rather than once
        // per corner. A selection's shapes are already in the display list's space, so there is no
        // page transform to compose — which is the whole difference from [`Self::device_point`].
        crate::select::quads_for(&interpreted.placed, range)
            .into_iter()
            .map(|quad| {
                // Two paired iterators rather than an index: the corners are (x, y) pairs and a
                // `chunks_exact(2)` over both sides says so without arithmetic on the index.
                let mut out = [0.0; 8];
                for (corner, place) in quad.chunks_exact(2).zip(out.chunks_exact_mut(2)) {
                    place[0] = origin.0 + corner[0] * magnification;
                    place[1] = origin.1 + (height - corner[1]) * magnification;
                }
                out
            })
            .collect()
    }

    /// The same range of the readback as one line of characters each, for a caret to move through.
    ///
    /// [`Self::device_quads`]'s other half: that answers where a *selection* is and merges the
    /// glyphs into as few shapes as it can, and this keeps every character apart because a caret
    /// stands between two of them. [`crate::accessibility::TextLine`] says what a consumer does
    /// with it; the space is the same one, so a host needs no second mapping.
    ///
    /// Each character's box is the *bounding rectangle* of its quadrilateral rather than the
    /// quadrilateral itself. That is a loss on rotated or sheared text and it is the one every
    /// platform text interface forces: a character's extent is asked for as a rectangle. The
    /// quadrilaterals are still what [`crate::AccessibilityNode::quads`] answers with, so nothing
    /// that wanted the shape has lost it.
    fn device_lines(
        &self,
        open: &Open,
        page: usize,
        ranges: &[(usize, usize)],
    ) -> Vec<crate::accessibility::TextLine> {
        let Some(interpreted) = open.interpretation(page) else {
            return Vec::new();
        };
        let Some(magnification) = open.magnification(self.viewport, self.scale) else {
            return Vec::new();
        };
        let Some(height) = open.page_size(page).map(|size| size.height) else {
            return Vec::new();
        };
        let Some(origin) = open.on(page).map(|on_screen| on_screen.origin) else {
            return Vec::new();
        };
        crate::select::lines_for(&interpreted.placed, ranges)
            .into_iter()
            .filter_map(|line| {
                let mut text = String::new();
                let mut characters: Vec<crate::accessibility::Character> =
                    Vec::with_capacity(line.len());
                let mut written = 0usize;
                for (span, quad) in line {
                    // A span that is not on a character boundary of the readback names no slice,
                    // and a character of no bytes is a caret position that cannot be left. Both
                    // are dropped rather than carried as an empty entry — see
                    // `crate::select::lines_for`.
                    let Some(slice) = interpreted
                        .text
                        .get(span.clone())
                        .filter(|slice| !slice.is_empty())
                    else {
                        continue;
                    };
                    // What the readback holds between the last glyph and this one, which is
                    // usually a space — §9.4.3's `TJ` moves the pen instead of drawing one, so a
                    // word gap has no character code and no box. It belongs to the character
                    // *before* it, which is the convention AccessKit states for word boundaries:
                    // "[t]railing whitespace is typically considered part of the word that
                    // precedes it". A run built without it would say `twowords`.
                    //
                    // **Whitespace only, and that is a guard rather than a tidy-up.** A line is
                    // decided by where the glyphs landed, so two glyphs beside each other on the
                    // screen may be far apart in the readback — with another element's words in
                    // between. Those are not this element's to speak, and the space that has no
                    // glyph is the whole of what is being recovered here.
                    if let Some(between) = interpreted.text.get(written..span.start)
                        && !between.is_empty()
                        && between.chars().all(char::is_whitespace)
                        && let Some(previous) = characters.last_mut()
                    {
                        text.push_str(between);
                        previous.bytes = previous.bytes.saturating_add(between.len());
                    }
                    text.push_str(slice);
                    characters.push(crate::accessibility::Character {
                        bytes: slice.len(),
                        bounds: device_box(quad, origin, height, magnification),
                    });
                    written = span.end;
                }
                (!characters.is_empty())
                    .then_some(crate::accessibility::TextLine { text, characters })
            })
            .collect()
    }

    /// Maps a viewport point to the display list's own coordinates.
    ///
    /// The half of [`Self::user_space`] that stops before the page's own transform: the display
    /// list, the text layer and every quadrilateral this crate hands out are in *this* space, and
    /// only §12.5.2's annotation rectangles are in the other one.
    fn page_point(&self, open: &Open, at: (f32, f32)) -> Option<(usize, (f32, f32))> {
        let magnification = open.magnification(self.viewport, self.scale)?;
        if magnification <= 0.0 {
            return None;
        }
        // Whichever page of Table 29's arrangement the point is inside, and the current one
        // otherwise — which is what a drag past the bottom of a page needs: a point off the
        // raster still names a position in the text, and refusing it would stop a selection
        // extending the moment the pointer left the page.
        let page = open.placed_at(at).map_or(open.page_index, |on| on.page);
        let origin = open.on(page)?.origin;
        let height = open.page_size(page)?.height;
        Some((
            page,
            (
                (at.0 - origin.0) / magnification,
                height - (at.1 - origin.1) / magnification,
            ),
        ))
    }

    /// Maps a viewport point to default user space on the page being shown.
    ///
    /// Through the transform the frame on the screen was drawn with, and not one this function
    /// invents: §12.5.2 states an annotation's rectangle "in default user space", and getting
    /// there means undoing the centring, the magnification, **the y axis**, the crop box's
    /// origin and §7.7.3.3's rotation, in that order. The last two are
    /// [`pdf_model::content::user_space_at`]'s.
    ///
    /// **The y axis is the one that was wrong.** A raster's y points down from its top row and
    /// PDF's points up from the bottom of the page; the flip between them lives in
    /// `TargetSpec::for_page` rather than in the page's own transform, so undoing it is this
    /// function's job. It is undone about the *page's* height rather than the raster's, because
    /// that is what the forward transform translates by — a raster is rounded up to contain the
    /// page and the leftover fraction of a row is at the bottom. ADR 0118.
    fn user_space(&self, open: &Open, at: (f32, f32)) -> Option<(usize, (f32, f32))> {
        let (page, (x, y)) = self.page_point(open, at)?;
        let object = open.placed_page(page)?;
        Some((page, pdf_model::content::user_space_at(object, x, y)?))
    }

    /// §12.9's measurement of a traced path, in the units the page states for it.
    ///
    /// Every point is carried into default user space, which is where Table 265 states a
    /// viewport's `/BBox`, and `pdf_model::measurement::Viewports::traced` applies §12.9.1's own
    /// selection rule to the first of them.
    ///
    /// **A path that leaves its page is refused rather than folded onto one.** `/VP` is an entry
    /// in a *page* dictionary, so two pages of Table 29's continuous arrangement state two
    /// unrelated arrays and a path across the join has no viewport the clause would choose. The
    /// alternative — measuring the part that stayed — would answer a shorter path than the one a
    /// person drew, with nothing on the screen saying so.
    fn measured(&self, open: &Open, points: &[[f32; 2]]) -> Option<measurement::Traced> {
        let mut page = None;
        let mut user = Vec::with_capacity(points.len());
        for point in points {
            let (index, (x, y)) = self.user_space(open, (point[0], point[1]))?;
            if *page.get_or_insert(index) != index {
                return None;
            }
            user.push([x, y]);
        }
        let object = open.placed_page(page?)?;
        measurement::Viewports::read(&open.document, &object.dict).traced(&user)
    }

    /// Resolves a zoom command into the magnification it lands on.
    ///
    /// A step is resolved here rather than stored, because two steps have to compose and "one
    /// larger than fitted" is not a state a mode can hold. Resolving also holds one point of the
    /// viewport still — the pointer's, or its centre where the command names none — which is what
    /// makes zooming feel like magnification rather than like jumping to a corner. ADR 0166.
    fn set_zoom(&mut self, zoom: Zoom, at: Option<(f32, f32)>, events: &mut Vec<Event>) {
        let (viewport, scale) = (self.viewport, self.scale);
        let Some(open) = self.focused_mut() else {
            return;
        };
        let before = open.magnification(viewport, scale);
        match zoom {
            Zoom::In | Zoom::Out => {
                let Some(before) = before else { return };
                open.zoom = Zoom::Scale(Open::stepped(before / scale, zoom));
            }
            Zoom::FitPage | Zoom::FitWidth | Zoom::FitHeight | Zoom::Scale(_) => open.zoom = zoom,
        }
        if let (Some(before), Some(after)) = (before, open.magnification(viewport, scale))
            && before > 0.0
        {
            open.hold(viewport, scale, before, after, at);
        }
        events.push(damage(viewport));
    }

    /// Puts the reader back at a view this crate answered with.
    ///
    /// **Three assignments in one command, and the order is the whole of the implementation.**
    /// The page goes first because a turn is what zeroes the scroll ([`Self::go_to`] says why),
    /// and going second would throw the restored offset away. The magnification goes next because
    /// the scroll is measured in device pixels and the magnification is what makes one — and
    /// through [`Self::set_zoom`] rather than by assignment, so that a `Viewing` built by hand
    /// with [`Zoom::In`] means the step it means everywhere else. The scroll goes last, and it is
    /// the only part written straight down: it is the one component this vocabulary could not
    /// otherwise state, and every other route to it passes through a clamp or an arithmetic.
    ///
    /// [`Self::settle`] clamps what lands here, exactly as it clamps a wheel's delta. A view this
    /// crate answered with was clamped when it was made, so the clamp is a no-op for it; a view a
    /// host composed for itself is held to the same bound as anything else, which is what keeps
    /// the public fields from being a way round the arrangement.
    fn restore(&mut self, view: Viewing, events: &mut Vec<Event>) {
        self.go_to(PageTarget::Index(view.page), Turn::Requested, events);
        self.set_zoom(view.zoom, None, events);
        let viewport = self.viewport;
        let Some(open) = self.focused_mut() else {
            return;
        };
        open.scroll = view.scroll;
        events.push(damage(viewport));
    }

    /// The pixel budget a render request is held to, which is the host's tier and not a
    /// property of the page.
    ///
    /// [`MAX_PIXELS`] bounds a raster this crate hands back, so it binds a host that takes one
    /// and says nothing to a host that draws its own frames at its own size. What remains for
    /// both is `TargetSpec::for_page`'s own refusal of a dimension `f32` cannot resolve.
    ///
    /// A host that answers [`Rendered::Listed`] for some pages is the first kind: the budget is
    /// per request and it has not stopped taking rasters.
    const fn raster_budget(&self) -> u64 {
        if self.holds_rasters {
            MAX_PIXELS
        } else {
            u64::MAX
        }
    }

    /// Works out whether what is on the screen is what should be, and asks for a render if not.
    ///
    /// The one place a render is scheduled. Interpretation happens here too, which is what makes
    /// [`Event::NeedsRender`] self-contained — see the crate documentation on who interprets and
    /// who rasterises.
    ///
    /// **Table 29's `/PageLayout` made this a loop over the pages on the screen rather than a
    /// statement about one page.** The order is what it always was and is the whole of what the
    /// function is: the view state, then the magnification, then where the scroll now stands, then
    /// which pages that shows, then their interpretations, then §12.3.2.1's pending view, then one
    /// render request per page whose pixels are not the pixels it should have.
    fn settle(&mut self, events: &mut Vec<Event>) {
        let (viewport, scale) = (self.viewport, self.scale);
        // A viewport with no extent is a window that has not been laid out yet, and rendering
        // into it would be a request for a zero-pixel raster.
        if viewport.0 == 0 || viewport.1 == 0 {
            return;
        }
        let Some(id) = self.focused else { return };
        let delegated = self.delegated;
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };

        // §12.5.3's `NoZoom` makes one annotation's placement a function of the magnification,
        // so the state interpretation reads has to carry it — and **logical** pixels per user
        // unit rather than device ones, because "the same fixed size on the screen" is a size a
        // person sees and a doubled display should draw it sharper rather than smaller.
        //
        // Re-interpreting only where the last interpretation said the page would notice is what
        // keeps the display list's promise, and **the question is asked of each page rather than
        // of the arrangement**: `Open::reinterpret` decides it per page now, because Table 29's
        // continuous arrangements put two pages on the screen and one annotation was costing the
        // interpretation of both. `tools/state.sh` counts the population; ADR 0775 has the price.
        //
        // `reinterpret` and not `stale`, because the ink did not move: a zoom changes where
        // §12.5.3 puts the annotation, not what the document's state is, and a host holding a
        // picture of the old magnification may go on standing in with it while the new
        // interpretation renders. `stale` here superseded the ink, so a host refused that
        // picture as "of another ink" and every zoom step of such a page froze for the length
        // of the real frame — the owner's "zooming is laggy", traced to 234 such refusals in
        // one gesture.
        //
        // **And §8.11.4.5 reads the same number**: "[w]henever there is a change to a factor that
        // the usage application dictionaries with event type View depend on (such as zoom level),
        // the corresponding dictionaries shall be reapplied". A reapplication that moves an
        // optional content group changes what the page *draws*, so the picture a host is holding
        // is of something else and `stale` is owed rather than `reinterpret` — which is why
        // `set_magnification` answers with `Magnified` rather than a `bool`. Nothing in the corpus
        // reaches it (`examples/oc_usage_census`), so the ordinary zoom keeps the paragraph above.
        let magnification = open
            .magnification(viewport, scale)
            .map(|device| device / scale);
        let magnified = open.view.set_magnification(magnification);
        if magnified.supersedes_ink() {
            open.stale();
        } else if magnified.needs_interpreting() {
            open.reinterpret();
        }

        // §6.3.2.2's "unless otherwise instructed", carried to whichever document is on screen.
        // Unconditional where it changed — unlike the magnification above, which asks whether
        // the page has anything that would notice: a page with no widget on it costs one field
        // walk that answers nothing, and a page with one would be wrong to keep.
        if open.view.set_widget_appearances(delegated) {
            open.stale();
        }

        let Some(magnification) = open.magnification(viewport, scale) else {
            return;
        };
        // Table 29's continuous arrangements move the current page when the scroll crosses a row,
        // and a page becoming current is a page turn however it happened — §12.6.3's `/O` and
        // `/C` are owed for it exactly as they are owed for an arrow key.
        let left = crate::layout::settle_scroll(open, viewport, scale, magnification);
        if let Some(left) = left {
            self.announce_page(events);
            self.page_events(id, Some(left), events);
        }

        self.arrange(id, magnification, events);

        // §12.3.2.1's location and magnification, applied here because this is the first place
        // both of the things they need exist: a viewport, and — for the three `/FitB` forms —
        // a display list to measure the page's contents from. ADR 0162.
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        if !open.pending_views.is_empty()
            && let Some(list) = open
                .interpreted()
                .map(|interpreted| Arc::clone(&interpreted.list))
        {
            let bounds = content_bounds(&list);
            let mut changed = false;
            // In order: §O.2 makes left-to-right normative for a fragment identifier, and a
            // document's own `/OpenAction` states at most one of these.
            for view in std::mem::take(&mut open.pending_views) {
                changed |= open.apply_view(view, viewport, scale, bounds);
            }
            if changed {
                for on_screen in &mut open.on_screen {
                    on_screen.shown = None;
                }
                let Some(magnification) = open.magnification(viewport, scale) else {
                    return;
                };
                self.arrange(id, magnification, events);
            }
        }

        self.schedule(id, events);
    }

    /// Places Table 29's arrangement and interprets whatever of it is not interpreted yet.
    ///
    /// Split from [`Self::settle`] because §12.3.2.1's pending view may change the magnification
    /// after the first pass and the arrangement then has to be made again — the same work, and
    /// one description of it rather than two.
    fn arrange(&mut self, id: DocumentId, magnification: f32, events: &mut Vec<Event>) {
        let (viewport, scale) = (self.viewport, self.scale);
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        let placements = crate::layout::place(open, viewport, scale, magnification);
        // What has scrolled off the screen is forgotten, which is the eviction rule
        // `Open::on_screen` did not have to invent: the view says what is kept.
        open.on_screen.retain(|on_screen| {
            placements
                .iter()
                .any(|placed| placed.page == on_screen.page)
        });
        for placed in &placements {
            if let Some(on_screen) = open.on_mut(placed.page) {
                on_screen.origin = placed.origin;
                on_screen.raster = placed.raster;
                continue;
            }
            // One page-tree walk per page *arriving* on the screen, and never per frame: this is
            // the cache ADR 0124 put in, with the arrangement deciding its contents.
            let Some(object) = open.page(placed.page) else {
                continue;
            };
            open.on_screen.push(crate::open::OnScreen {
                page: placed.page,
                object,
                origin: placed.origin,
                raster: placed.raster,
                interpreted: None,
                replaceable: None,
                revision: 0,
                shown: None,
                frame: None,
                pending: None,
            });
        }
        open.on_screen.sort_by_key(|on_screen| on_screen.page);
        // A selection is a range of each of its pages' readbacks, so it lives exactly as long as
        // **every** page it covers is on the screen — which under `SinglePage` is the page turn
        // that used to end it. Not "as long as one of them is": a selection whose middle this
        // crate no longer holds the readback for could answer with text that has a hole in it,
        // and a hole nothing announces is the plausible-looking picture principle 1 is about.
        if open.selection.is_some_and(|chosen| {
            let (first, last) = chosen.ordered();
            (first.page..=last.page).any(|page| open.on(page).is_none())
        }) {
            open.selection = None;
        }

        for index in 0..open.on_screen.len() {
            if open.on_screen[index].interpreted.is_some() {
                continue;
            }
            let page = open.on_screen[index].page;
            let Some(crate::open::Read {
                interpretation,
                mut reports,
                object,
                replaceable,
            }) = crate::open::interpret(open, page)
            else {
                continue;
            };
            // §7.5.7's losses become known when a page reaches into an object stream, which is
            // here and never at open. The sentence is the document's rather than the page's and
            // is said once — `crate::notes::losses` — so a page carries it only the first time.
            reports.extend(crate::notes::losses(open));
            if !reports.is_empty() {
                events.push(Event::Reported {
                    document: id,
                    page: Some(page),
                    notes: reports.clone(),
                });
            }
            open.revision = open.revision.saturating_add(1);
            // The page a person is looking at was just interpreted, so a search that starts here
            // — which every find bar's does — need not interpret it again. `doc/todo/49`'s fourth
            // item is exactly this: the same page read for a search and then again to draw it was
            // two interpretations. One 2.6 KB copy per page turn against 5.4 ms of the thing it
            // saves.
            open.readbacks
                .put(page, &Arc::from(interpretation.text.as_str()));
            let revision = open.revision;
            let on_screen = &mut open.on_screen[index];
            on_screen.object = object;
            on_screen.revision = revision;
            // A `None` is a read that *used* the replacement this page already holds rather than
            // one that made a new one, so it says nothing about whether to keep it; see
            // `open::Read::replaceable`.
            if replaceable.is_some() {
                on_screen.replaceable = replaceable;
            }
            on_screen.interpreted = Some(Interpreted {
                shortfall: interpretation.shortfall(),
                list: Arc::new(interpretation.display_list),
                reports,
                text: interpretation.text,
                placed: interpretation.text_layer,
                marked: interpretation.marked,
                described: interpretation.described,
                language: interpretation.language,
                view_dependent: interpretation.view_dependent,
            });
        }
        // …unless a search put one there for a page that has only now been interpreted, which is
        // the one range made *after* a turn rather than before it. See `Open::pending_selection`.
        if let Some(chosen) = open.pending_selection
            && open.interpretation(chosen.from.page).is_some()
        {
            open.pending_selection = None;
            open.selection = Some(chosen);
        }
    }

    /// Asks for a render of every page whose pixels are not the pixels it should have.
    fn schedule(&mut self, id: DocumentId, events: &mut Vec<Event>) {
        let (viewport, scale) = (self.viewport, self.scale);
        // Read before the document is borrowed: it is a fact about the host, not the page.
        let budget = self.raster_budget();
        let mut token = self.next_token;
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        let Some(magnification) = open.magnification(viewport, scale) else {
            return;
        };
        let mut requests = Vec::new();
        let mut refusals = Vec::new();
        // Read once, outside the loop, because it is a fact about the document rather than about
        // one of its pages: every request this pass makes is of the same ink.
        let ink = open.ink;
        for on_screen in &mut open.on_screen {
            let Some(interpreted) = on_screen.interpreted.as_ref() else {
                continue;
            };
            let list = Arc::clone(&interpreted.list);
            let target = match TargetSpec::for_page(&list, magnification, budget) {
                Ok(target) => target,
                // Named rather than clamped, because a page silently drawn at a scale nobody chose
                // is a page a person has been told something false about.
                Err(error) => {
                    refusals.push((
                        on_screen.page,
                        format!("this page cannot be drawn at this size: {error}"),
                    ));
                    continue;
                }
            };
            let showing = on_screen.shown == Some((target, on_screen.revision));
            let asked = on_screen.pending.as_ref().is_some_and(|pending| {
                pending.target == target && pending.revision == on_screen.revision
            });
            if showing || asked {
                continue;
            }
            let issued = RenderToken(token);
            token = token.saturating_add(1);
            on_screen.pending = Some(Pending {
                token: issued,
                page: on_screen.page,
                target,
                revision: on_screen.revision,
            });
            requests.push(Event::NeedsRender(RenderRequest {
                token: issued,
                document: id,
                page: on_screen.page,
                list,
                target,
                ink,
            }));
        }
        self.next_token = token;
        for (page, note) in refusals {
            events.push(Event::Reported {
                document: id,
                page: Some(page),
                notes: vec![note],
            });
        }
        events.extend(requests);
    }

    /// §12.4.4: enters or leaves presentation mode, in every open document.
    ///
    /// Nothing at all when the mode is already what the host says it is, which is what lets a host
    /// send it on every frame if that is what its own state is easiest to mirror from.
    fn present(&mut self, mode: crate::PresentationMode) {
        if self.presenting == mode {
            return;
        }
        self.presenting = mode;
        for open in self.documents.values_mut() {
            match mode {
                crate::PresentationMode::On => crate::presentation::enter(open),
                // §12.4.4.2 NOTE 2's restore changes what is *drawn*, so it invalidates the
                // display list rather than only the pixels made from it — the same thing
                // `Command::SetGroup` does, and for the same clause's reason.
                crate::PresentationMode::Off => {
                    if crate::presentation::leave(open) {
                        open.stale();
                    }
                }
            }
        }
    }

    /// Shows another page, or — during a presentation — another *state* of the page being shown.
    ///
    /// §12.4.4.2 puts a decision in front of every page turn a person asks for: "[i]f the user
    /// requests to navigate forward (such as an arrow key press) **and there is a current
    /// navigation node**", what happens is the node's own actions and not a page turn. So a
    /// presentation steps through a page's states first and turns the page when they run out,
    /// which is what NOTE 1's bullet points are for.
    ///
    /// `turn` is why the page is being turned, and it decides two things that are not the same
    /// question: §12.4.4.1's own clock is not a person requesting anything, so it never steps
    /// within a page — it advances the page, which is what `/Dur` says it does.
    fn go_to(&mut self, target: PageTarget, turn: Turn, events: &mut Vec<Event>) {
        let requested = match target {
            PageTarget::Next => Some(true),
            PageTarget::Previous => Some(false),
            _ => None,
        };
        if turn == Turn::Requested
            && self.presenting == crate::PresentationMode::On
            && let Some(forward) = requested
        {
            let stepped = self.focused_mut().and_then(|open| {
                crate::presentation::step(open, forward)
                    .map(|step| (interact::navigate(open, &step.actions), step.onward))
            });
            if let (Some((outcome, onward)), Some(id)) = (stepped, self.focused) {
                // §12.4.4.2: "[i]f NA specifies an action that navigates to another page, the
                // following actions for navigating to another page take place, and Next should
                // not be present." A node whose actions have already moved the page has had its
                // turn, so errata issue #304's page turn is not owed a second one.
                let jumped = outcome.target.is_some();
                self.apply(id, outcome, events);
                if jumped || !onward {
                    return;
                }
            }
        }

        let Some(open) = self.focused_mut() else {
            return;
        };
        let Some(index) = resolve(open, target) else {
            return;
        };
        if index == open.page_index {
            return;
        }
        let left = open.page_index;
        open.page_index = index;
        // A new page starts at its top: carrying a scroll position across a page turn
        // would land a reader in the middle of a page they have not seen.
        open.scroll = (0.0, 0.0);
        // §12.4.4.1's clock is a property of the page, so a turn restarts it — which is
        // also NOTE 1's "[t]he user can advance the page manually before the specified
        // time has expired" doing the right thing without a second rule.
        open.shown_for = 0.0;
        self.announce_page(events);
        let Some(id) = self.focused else {
            return;
        };
        self.page_events(id, Some(left), events);
        // §12.4.4.2 makes random access forward — "[i]f the navigation request was forward, or if
        // the navigation request was for random access (such as by clicking on a link)" — so only
        // a request that names a page *behind* this one is backward.
        let forward = !matches!(target, PageTarget::Previous)
            && !matches!(target, PageTarget::Relative(delta) if delta < 0);
        self.arrive(id, forward, turn, events);
    }

    /// What §12.4.4 says happens on arriving at a page, where a presentation is running.
    ///
    /// The last paragraph of §12.4.4.2, then its step (c): the page's own navigation node becomes
    /// current and one request is performed against it, and then "[a]ny page transitions specified
    /// by the Trans entry of the page dictionary shall be performed".
    ///
    /// **The two are gated differently, and the difference is what a host has said.** The node
    /// walk needs [`crate::PresentationMode::On`], because walking the nodes changes §8.11's
    /// groups and NOTE 2's obligation to put them back is discharged by entering the mode; a host
    /// that only drives the clock has saved nothing to restore. The transition needs either that
    /// or an automatic advance, because §12.4.4.1's `/Dur` running out *is* a presentation running
    /// — which is ADR 0135's reading and the one thing a tick alone does say.
    fn arrive(&mut self, id: DocumentId, forward: bool, turn: Turn, events: &mut Vec<Event>) {
        // A nested arrival is the outer one's business, and that is not only a bound on the
        // cascade: the clause's step (c) is about "the new page", so an `/NA` that jumped again
        // has one transition to perform and not one per page it passed through.
        if self.stepping {
            return;
        }
        self.stepping = true;
        if self.presenting == crate::PresentationMode::On {
            let outcome = self
                .focused_mut()
                .map(|open| {
                    let actions = crate::presentation::arrived(open, forward);
                    interact::navigate(open, &actions)
                })
                .unwrap_or_default();
            self.apply(id, outcome, events);
        }
        if turn == Turn::Automatic || self.presenting == crate::PresentationMode::On {
            self.play_transition(events);
        }
        self.stepping = false;
    }

    /// §12.4.4.1's `/Trans`: names the transition of the page a presentation has arrived at.
    ///
    /// > The transition style that shall be used when moving to this page from another during a
    /// > presentation
    ///
    /// *This* page, so it is the page arrived at whose `/Trans` plays, and the page is fetched
    /// from the tree rather than read from `Open::shown_page`, because the arrangement it reads
    /// is built in `settle` — after this. Reading it here would
    /// name the transition of the page just *left*, which is the same off-by-one §12.4.4.1's own
    /// wording rules out. One page-tree walk per advance.
    fn play_transition(&mut self, events: &mut Vec<Event>) {
        let (Some(id), Some(open)) = (self.focused, self.focused()) else {
            return;
        };
        let pages = pdf_model::Pages::new(&open.document);
        let Some(page) = pages.get(open.page_index) else {
            return;
        };
        if let Some(transition) = pdf_model::navigation::transition(&open.document, &page.dict) {
            // A transition no frame is shaped for is *named* rather than quietly drawn as a cut,
            // which is trap 5 in the one place a viewer is most tempted to be silent: the page
            // that arrives looks right, and only the file knows it asked for an effect. And one
            // drawn at a quantity this program chose says the quantity is ours (ADR 1299).
            // `crate::transition` decides which those are, because it is what draws them — and
            // it is asked the whole transition, because a style is not the whole of what
            // decides a frame.
            if let Some(note) = crate::transition::note(&transition) {
                let index = self.focused().map(|open| open.page_index);
                events.push(Event::Reported {
                    document: id,
                    page: index,
                    notes: vec![note],
                });
            }
            events.push(Event::Transition {
                document: id,
                transition,
            });
        }
    }

    /// Says which page is showing, where there is one.
    /// §12.4.4.1's `/Dur`: advances the page when it has been shown for as long as it asked.
    ///
    /// > the maximum length of time, in seconds, that the page shall be displayed before the
    /// > presentation automatically advances to the next page.
    ///
    /// A *maximum*, which is why the comparison is `>=` and why any page turn resets the clock.
    /// The last page does not advance and does not loop: §12.4.4 says nothing about what follows
    /// the end, and looping is a decision a host can make with a `GoTo` and this crate cannot
    /// unmake.
    fn tick(&mut self, millis: u32, events: &mut Vec<Event>) {
        // Milliseconds in, seconds out, because the clause counts in seconds and a host counts
        // in whatever its event loop gives it. `f32` loses exactness above sixteen million
        // milliseconds — four and a half hours on one page — and what is being measured is a
        // duration a person perceives, which `pdf_model::navigation` narrows for the same reason.
        let seconds =
            f32::from(u16::try_from(millis.min(u32::from(u16::MAX))).unwrap_or(u16::MAX)) / 1000.0;
        let Some(open) = self.focused_mut() else {
            return;
        };
        open.shown_for += seconds;

        // Table 165's `/Dur` first, because it is the inner of the two clocks: "[t]he maximum
        // number of seconds before the interactive PDF processor shall automatically advance
        // forward to the next navigation node".
        //
        // **To a node, and never to a page**, which is where this entry and an arrow key part
        // company. Errata issue #304 makes a *user's* forward request with no `/Next` navigate to
        // the next page (§12.4.4.2 item (b)); Table 165 names the next navigation node as its own
        // destination, so a node with no successor has nothing to advance to and this clock stops
        // there. The page's own `/Dur` below is the one that turns a page.
        if self.presenting == crate::PresentationMode::On {
            let due = self
                .focused_mut()
                .is_some_and(|open| crate::presentation::node_due(open, seconds));
            if due {
                let stepped = self.focused_mut().and_then(|open| {
                    crate::presentation::step(open, true)
                        .map(|step| interact::navigate(open, &step.actions))
                });
                if let (Some(outcome), Some(id)) = (stepped, self.focused) {
                    self.apply(id, outcome, events);
                }
            }
        }

        let Some(open) = self.focused_mut() else {
            return;
        };
        let Some(page) = open.shown_page() else {
            return;
        };
        let Some(duration) = pdf_model::navigation::display_duration(&open.document, &page.dict)
        else {
            return;
        };
        if open.shown_for < duration || open.page_index.saturating_add(1) >= open.page_count {
            return;
        }
        // §12.4.4.1's own advance, which is a page turn and never a step within a page: `/Dur` is
        // "the maximum length of time … that the page shall be displayed before the presentation
        // automatically advances to the **next page**".
        self.go_to(PageTarget::Next, Turn::Automatic, events);
    }

    fn announce_page(&mut self, events: &mut Vec<Event>) {
        let (Some(id), Some(open)) = (self.focused, self.focused()) else {
            return;
        };
        let index = open.page_index;
        // §7.7.3's tree placed once for the life of the document rather than once per turn, and
        // no `Pages` built here at all: both were per-turn costs that scale with the document —
        // 1023 pages of ISO 32000-2 cost about six milliseconds of walk and a further third of
        // one to construct the tree, on the path an arrow key takes. See `Open::page_indices`.
        let section = open
            .outline
            .section_at_with(
                &open.document,
                open.page_indices
                    .get_or_init(|| pdf_model::Pages::new(&open.document).indices()),
                index,
            )
            .map(ToOwned::to_owned);
        events.push(Event::PageChanged {
            document: id,
            index,
            label: label(open, index),
            of: open.page_count,
            section,
        });
    }

    /// §12.6.3's four page-scoped events and Table 198's two, in the order the clause states.
    ///
    /// A page turn raises six things, and §12.6.3 states the order of four of them:
    ///
    /// > The action shall be executed after the O action in the page's additional - actions
    /// > dictionary (see "Table 198 - Entries in a page object's additional - actions
    /// > dictionary") and the OpenAction entry in the document Catalog (see "Table 29 - Entries
    /// > in the catalog dictionary"), if such actions are present.
    ///
    /// and, of `/PC`, that it shall be executed before the page's own `/C`. So leaving a page is
    /// its annotations' `/PC` then the page's `/C`, and
    /// arriving at one is the page's `/O` then its annotations' `/PO` — which is the sequence
    /// below.
    ///
    /// **`/PV` and `/PI` coincide with `/PO` and `/PC` here, and that is derived rather than
    /// conceded.** §12.6.3 says why the pair exists: "[t]he PV and PI entries allow a distinction
    /// between pages that are open and pages that are visible. At any one time, while more than
    /// one page may be visible, depending on the page layout." This viewer shows one page at a
    /// time, so the two sets have one member each and they are the same member. A host that drew
    /// a continuous tower of pages would separate them, and the place to do it is here.
    ///
    /// NOTE 1 is honoured by not consulting Table 167 at all: "[f]or these trigger events, the
    /// values of the flags specified by the annotation's F entry … have no bearing on whether a
    /// given trigger event occurs" — so a hidden annotation's `/PO` is performed.
    ///
    /// **Nothing cascades.** An action performed here may turn the page again; raising the same
    /// six events for *that* turn would let a document with `/PO` pointing at the next page walk
    /// the whole file, and §12.6.2 states no bound. `raising` is that bound, and it is a flag
    /// rather than a depth because one level is the only one the clause describes.
    fn page_events(&mut self, id: DocumentId, left: Option<usize>, events: &mut Vec<Event>) {
        if self.raising {
            return;
        }
        self.raising = true;
        let mut raised: Vec<(ObjectId, Trigger)> = Vec::new();
        let mut closed = None;
        let mut opened = None;
        // A page turned takes the focus with it: the widget that held it is not on the screen
        // any more, so §12.6.3's `/Bl` is due before the page's own events. Raised here rather
        // than in `pointer` because this is where a page stops being shown, whatever caused it.
        if let Some(open) = self.focused_mut()
            && left.is_some_and(|index| index != open.page_index)
            && let Some(lost) = open.focus.take()
        {
            raised.push((lost, Trigger::Blur));
        }
        if let Some(open) = self.focused() {
            let pages = pdf_model::Pages::new(&open.document);
            if let Some(index) = left.filter(|index| *index != open.page_index) {
                closed = pages.get(index).map(|page| page.dict.clone());
                for annotation in annotations_on(open, &pages, index) {
                    raised.push((annotation, Trigger::PageClose));
                    raised.push((annotation, Trigger::PageInvisible));
                }
            }
            opened = pages.get(open.page_index).map(|page| page.dict.clone());
        }
        self.raise(id, raised, None, events);
        for (page, event) in [
            (closed, pdf_model::action::PageTrigger::Close),
            (opened.clone(), pdf_model::action::PageTrigger::Open),
        ] {
            let Some(page) = page else { continue };
            let Some(open) = self.focused_mut() else {
                break;
            };
            let outcome = interact::page_trigger(open, &page, event);
            self.apply(id, outcome, events);
        }
        let mut raised: Vec<(ObjectId, Trigger)> = Vec::new();
        if let Some(open) = self.focused() {
            let pages = pdf_model::Pages::new(&open.document);
            for annotation in annotations_on(open, &pages, open.page_index) {
                raised.push((annotation, Trigger::PageOpen));
                raised.push((annotation, Trigger::PageVisible));
            }
        }
        self.raise(id, raised, None, events);
        self.raising = false;
    }

    /// The focused document, where one is focused and open.
    fn focused(&self) -> Option<&Open> {
        self.documents.get(&self.focused?)
    }

    /// The focused document, mutably.
    fn focused_mut(&mut self) -> Option<&mut Open> {
        self.documents.get_mut(&self.focused?)
    }

    /// Where a page sits on the screen, for the page that is on it.
    fn geometry(&self, open: &Open, index: usize) -> Option<PageGeometry> {
        let on_screen = open.on(index)?;
        // Answered only once the page has been interpreted, as it always was: a geometry taken
        // from an extent nothing has drawn yet is a promise about a frame that may not arrive.
        on_screen.interpreted.as_ref()?;
        Some(PageGeometry {
            page: open.page_size(index)?,
            scale: open.magnification(self.viewport, self.scale)?,
            width: on_screen.raster.0,
            height: on_screen.raster.1,
            origin: on_screen.origin,
        })
    }
}

/// What a page's contents cover, in the display list's own space.
///
/// §12.3.2.2's `/FitB`, `/FitBH` and `/FitBV` are magnified to fit "the smallest rectangle
/// enclosing all of its contents" — the *bounding box* — which no page dictionary states and
/// only the drawing commands can answer. `None` where a command's extent cannot be bounded, in
/// which case the three forms fall back to the page box and say so.
fn content_bounds(list: &DisplayList) -> Option<Rect> {
    let mut union: Option<Rect> = None;
    for command in list.commands() {
        let bounds = pdf_render::marked_bounds(command, Transform::IDENTITY)?;
        union = Some(union.map_or(bounds, |box_| box_.union(bounds)));
    }
    union
}

/// What §14.7.5.3's object references can name on this page: §12.5.2's rectangles and `/Lang`,
/// and §12.7's controls.
///
/// Both are keyed by the annotation, which is what an object reference names, and both are the
/// same readings the rest of this crate uses — `form::fields` with **this view's** state, so a
/// check box a person has just ticked answers `on` in the accessibility tree exactly as it does
/// in [`Answer::Fields`].
///
/// **Keyed by the annotation means answering for the annotation**, which is what
/// [`this_widgets_control`] is for and what this function did not do for a hundred and thirty
/// sessions: the control it stored under each of a radio set's widgets was the *field's*, so every
/// button of the set said it was selected.
///
/// Takes the page the arrangement is already holding rather than an index into the page tree:
/// under a column this is asked once per page on the screen, and `Pages::get` is the walk ADR
/// 0124 cached `OnScreen::object` to avoid.
fn referenced_objects(open: &Open, shown: &pdf_model::Page) -> Referenced {
    let places = pdf_model::structure::annotation_rectangles(&open.document, &shown.dict);
    let languages = pdf_model::structure::annotation_languages(&open.document, &shown.dict);
    let mut controls = BTreeMap::new();
    for field in pdf_model::form::fields(&open.document, shown, &open.view) {
        for widget in &field.widgets {
            controls.insert(
                widget.annotation,
                this_widgets_control(&field.control, widget),
            );
        }
    }
    Referenced {
        places,
        languages,
        controls,
    }
}

/// The three readings [`referenced_objects`] answers with, keyed by the annotation each is about.
///
/// A value rather than a tuple because they are three different facts about one page and a caller
/// reading `.1` would have to remember which; `Default` is the answer for a page whose structure
/// tree names no object at all, which is nearly every page.
#[derive(Default)]
struct Referenced {
    /// §12.5.2's `/Rect` for each annotation the page lists, in default user space.
    places: BTreeMap<ObjectId, [f32; 4]>,
    /// Table 166's `/Lang` for each annotation that states one.
    languages: BTreeMap<ObjectId, String>,
    /// §12.7's control for each widget annotation of a field with a widget on this page.
    controls: BTreeMap<ObjectId, pdf_model::form::Control>,
}

/// The field's control with §12.7.5.2's on state replaced by **this widget's**.
///
/// **§14.7.5.3's object reference names one widget annotation**, so a node built from it is a
/// button rather than a field — and `pdf_model::form::Control::RadioButton`'s `on` is "[w]hether
/// any widget of the set is on", which is the field's answer and the wrong one here.
/// `pdf_model::form::Widget::on` is the right one and sits beside it, saying so in its own doc
/// comment: "[`Control::CheckBox`] carries the same fact for the field as a whole; this is the
/// per-widget answer a radio set needs."
///
/// What that cost until the seven-hundred-and-thirty-fifth session was a clause, measured on a
/// real AT-SPI bus: a screen reader was told **every** button of a set is selected as soon as one
/// of them was. ISO 32000-2 §12.7.5.2.4:
///
/// > Like check boxes, individual radio buttons have two states, on and off.
///
/// and §12.7.5.2.3 makes the exclusion a `shall` where Table 229 bit 26 is clear: "at most one
/// radio button in a field shall be set at a time". A tree that says three are is the sharpest
/// form of trap 5 — the person it misleads is the one for whom the picture is no answer.
///
/// The fallback is the one both native hosts already apply to their own buttons: a widget whose
/// `/AP` names no on state at all cannot be on by name, so the field's answer is what is left.
fn this_widgets_control(
    control: &pdf_model::form::Control,
    widget: &pdf_model::form::Widget,
) -> pdf_model::form::Control {
    match control {
        pdf_model::form::Control::CheckBox { on } => pdf_model::form::Control::CheckBox {
            on: widget.on || (*on && widget.on_state.is_none()),
        },
        pdf_model::form::Control::RadioButton {
            on,
            no_toggle_to_off,
            in_unison,
        } => pdf_model::form::Control::RadioButton {
            on: widget.on || (*on && widget.on_state.is_none()),
            no_toggle_to_off: *no_toggle_to_off,
            in_unison: *in_unison,
        },
        // Every other type states one value for the field and no widget of its own has another.
        other => other.clone(),
    }
}

/// Which object the page at `index` **is**, for the clauses that name a page object.
///
/// Table 355's `/Pg`, Table 358's and §12.5.1's tab order all name a page object, while this
/// crate holds an index — so the two have to be joined, and where that join is made turns out to
/// matter. It used to be made by *inverting* [`pdf_model::Pages::indices`], and that map
/// deliberately holds an intermediate `/Pages` node as well, "answering with the first page
/// beneath it" — so the inverse lookup handed back whichever of the two object numbers came
/// first, and where a node's is lower than page one's it handed back **a node that is not a
/// page**. Every `/Pg` comparison then failed, and §14.7's whole answer for that page was empty:
/// the same silence an untagged page gives, on ten of this project's own tagged documents
/// including ISO 14289-1 — found by ADR 0342's census, which is what it was built to see.
///
/// [`pdf_model::Page::id`] is "which object the page *is*", which is the question, and one
/// descent to the leaf is cheaper than the whole-tree walk the map costs. The map keeps its own
/// direction — object to index — where the node entry is exactly what a destination naming a
/// node needs.
fn page_object(pages: &pdf_model::Pages, index: usize) -> Option<ObjectId> {
    pages.get(index).and_then(|page| page.id)
}

/// Every annotation object on a page, in `/Annots` order.
///
/// By object identity, because a trigger performs what the *dictionary* states and a direct
/// annotation — one the array holds inline rather than by reference — has no identity to raise
/// an event against. §12.5.2 makes `/Annots` "an array of annotation dictionaries", and every
/// corpus document writes them indirectly.
fn annotations_on(open: &Open, pages: &pdf_model::Pages, index: usize) -> Vec<ObjectId> {
    let Some(page) = pages.get(index) else {
        return Vec::new();
    };
    annotations_of(open, &page)
}

/// The same, for a page this crate is already holding.
fn annotations_of(open: &Open, page: &pdf_model::Page) -> Vec<ObjectId> {
    let entry = open.document.get_key(&page.dict, "Annots");
    let Some(array) = entry.as_array() else {
        return Vec::new();
    };
    array
        .iter()
        .filter_map(pdf_syntax::Object::as_reference)
        .collect()
}

/// Which of §7.6.4.1's operations an [`crate::Edit`] is.
///
/// The one place a host's vocabulary is mapped onto the standard's, so that the two enums stay
/// separate: `Edit` says what a person did in a viewer's terms and
/// [`pdf_model::restriction::Operation`] says what a clause restricts, and folding them into one
/// type would make every future edit a claim about which permission covers it.
fn operation_of(edit: &crate::command::Edit) -> pdf_model::restriction::Operation {
    match edit {
        // §12.7.5.3's file-select control is a form field being filled in, whatever the value
        // happens to be: Table 22 bit 9 permits "[f]ill in existing interactive form fields
        // (including signature fields)", and the pathname is the field's own text.
        crate::command::Edit::SetField { .. } | crate::command::Edit::ChooseFile { .. } => {
            pdf_model::restriction::Operation::FillInForm
        }
        // §12.5.6.6's annotation and the text inside it are both annotating, which Table 22's own
        // wording separates from filling in a form: bit 6 is "[a]dd or modify text annotations,
        // fill in interactive form fields", and bit 9 permits filling alone. So an edit to a free
        // text annotation's `/Contents` is Annotate and not FillInForm, whatever it resembles at a
        // keyboard.
        //
        // §12.5.6.15's annotation carries its file, so a file put on a page is an annotation
        // added — bit 6 too — while a file put in §7.7.4's tree is bit 4's residual, as is
        // taking one out. `AttachHome` carries the argument. ADR 0814.
        crate::command::Edit::Markup { .. }
        | crate::command::Edit::FreeText { .. }
        | crate::command::Edit::SetFreeText { .. }
        | crate::command::Edit::Attach {
            home: crate::command::AttachHome::Page { .. },
            ..
        } => pdf_model::restriction::Operation::Annotate,
        crate::command::Edit::Attach {
            home: crate::command::AttachHome::Document,
            ..
        }
        | crate::command::Edit::Detach { .. } => pdf_model::restriction::Operation::Modify,
    }
}

/// Which field an [`crate::Edit`] names, where it names one.
///
/// §12.7.5.5's signature field lock is the one restriction that is about a *field* rather than
/// about the document, so the question `operation_of` answers — which permission covers this
/// verb — is not enough on its own to ask it. `None` for every edit whose subject is an
/// annotation, which §12.7.4.2 gives no name to.
fn field_of(edit: &crate::command::Edit) -> Option<&str> {
    match edit {
        crate::command::Edit::SetField { field, .. }
        | crate::command::Edit::ChooseFile { field, .. } => Some(field),
        crate::command::Edit::Markup { .. }
        | crate::command::Edit::FreeText { .. }
        | crate::command::Edit::SetFreeText { .. }
        | crate::command::Edit::Attach { .. }
        | crate::command::Edit::Detach { .. } => None,
    }
}

/// Which annotation an [`crate::Edit`] names, where it names one.
///
/// [`field_of`]'s counterpart, and it exists for the same shape one clause over: §12.5.3's Table
/// 167 bit 10 restricts **this annotation** rather than the document, so the verb cannot answer it
/// either. `None` for every edit whose subject is a field or an annotation that does not exist yet
/// — a restriction on changing an annotation's contents has nothing to say about creating one, and
/// the flag is read off an object the file already holds.
fn annotation_of(edit: &crate::command::Edit) -> Option<ObjectId> {
    match edit {
        crate::command::Edit::SetFreeText { annotation, .. } => Some(*annotation),
        crate::command::Edit::SetField { .. }
        | crate::command::Edit::ChooseFile { .. }
        | crate::command::Edit::Markup { .. }
        | crate::command::Edit::FreeText { .. }
        | crate::command::Edit::Attach { .. }
        | crate::command::Edit::Detach { .. } => None,
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

/// Adds one resolved edit to a document's log and says what changed.
///
/// The two events an edit can cause, in the order a host wants them: the unsaved mark first,
/// because it is the state, and the attachment list second, because it is a consequence of it.
fn commit(id: DocumentId, open: &mut Open, done: crate::open::Done, events: &mut Vec<Event>) {
    let committed = open.commit(done);
    if committed.dirty_changed {
        events.push(Event::Dirty {
            document: id,
            dirty: open.dirty(),
        });
    }
    if committed.attachments {
        events.push(Event::AttachmentsChanged { document: id });
    }
}

/// One page of a print operation, interpreted for the sheet and targeted at the printer.
///
/// `None` for a page index the document has not got, and for a page whose marks will not fit the
/// target at the job's resolution — [`pdf_render::TargetSpec::for_page`]'s own refusal, which is
/// the same bound every frame is drawn under. A refusal here is silent in this crate and loud in
/// the host: a print job that got no page for page seven says so where the job is, and a report
/// invented here would fire on a condition no clause states (trap 11).
///
/// The view state is the document's own, which [`Viewer::begin_printing`] has already put into
/// print intent — so this function states nothing about §12.5.3 or §8.11.4.5 and could not: the
/// whole of the print reading is in `pdf_model`, asked of a state that says what the output is
/// for.
/// A sheet as §12.5.6.22 reads it: the media, and what B does to the page on it.
///
/// `None` is Table 193's dimensions that "are not known at the time of drawing", under which the
/// page's own media box stands in and there is no B to cancel — so the scale is dropped with the
/// rectangle rather than carried alone. ADR 1204.
fn paper_of(sheet: crate::Sheet) -> Option<pdf_model::view::TargetMedia> {
    sheet.media.map(|media| pdf_model::view::TargetMedia {
        media,
        page_scale: sheet.page_scale,
    })
}

fn print_page(open: &Open, index: usize, sheet: crate::Sheet) -> Option<crate::PrintPage> {
    let read = crate::open::interpret(open, index)?;
    let list = Arc::new(read.interpretation.display_list);
    let mut reports = read.reports;
    let target = match TargetSpec::for_page(&list, sheet.scale, MAX_PIXELS) {
        Ok(target) => Some(target),
        // Trap 5: a page this crate will not rasterise at the printer's resolution is said out
        // loud rather than handed back as an empty sheet. The sentence is the backend's own,
        // which already names the budget and what was asked of it.
        Err(refusal) => {
            reports.push(format!(
                "page {} will not print: {refusal}",
                index.saturating_add(1)
            ));
            None
        }
    };
    Some(crate::PrintPage {
        page: index,
        list,
        target,
        reports,
    })
}

/// The whole viewport, which is what changes when a frame arrives or a page scrolls.
///
/// A tighter rectangle is available for a scroll — everything but the exposed strip is a
/// translation of what is already on screen — and it is not computed, because a host that blits
/// from a held raster pays for the whole viewport either way and one that scrolls by copying is
/// optimising a case nobody has measured.
fn damage(viewport: (u32, u32)) -> Event {
    Event::Damage(Rect::from_corners(
        Point::new(0.0, 0.0),
        Point::new(px(viewport.0), px(viewport.1)),
    ))
}

/// A pixel count as a coordinate.
///
/// Exact for every extent a raster may have: [`pdf_render::MAX_EXTENT`] is 2²⁴, which is the
/// largest integer an `f32` represents exactly. A *viewport* larger than that would round, and
/// no display is.
pub(crate) fn px(value: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "exact below 2^24, which bounds every raster extent; see the doc comment"
    )]
    let value = value as f32;
    value
}

/// §12.3.5's collection, and the document §12.3.5.1's `/D` says to present first.
///
/// The resolution happens here rather than in a host because Table 153's `/D` "identifies an
/// entry in the `EmbeddedFiles` name tree" and that tree is the document's — a panel holding the
/// collection dictionary has no way to tell a `/D` naming a file from a `/D` naming nothing.
/// [`Answer::Collection`] carries both halves for that reason.
fn collection(open: &Open) -> Answer<'static> {
    pdf_model::collection::Collection::read(&open.document).map_or(Answer::None, |collection| {
        let initial = collection.initial_document(&open.document);
        // Table 153's `/Sort` is a `shall` about the order rows stand in, and the values it
        // orders by need both the document and the attachments — so it is resolved once here
        // rather than by each panel that draws the list (ADR 1168).
        let order = pdf_model::collection::sorted_keys(
            &open.document,
            &collection,
            &open.view.attachments(&open.document),
        )
        .unwrap_or_default();
        Answer::Collection {
            collection,
            initial,
            order,
        }
    })
}

/// One glyph's quadrilateral as a rectangle in device pixels of the viewport.
///
/// The mapping is [`Viewer::device_quads`]'s, corner by corner, and the bounding box is taken
/// afterwards rather than before: the transform flips y, so a quadrilateral's lowest corner in the
/// display list's space is its highest on the screen and a box taken first would come out inverted.
fn device_box(quad: [f32; 8], origin: (f32, f32), height: f32, magnification: f32) -> [f32; 4] {
    let mut bounds = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for corner in quad.chunks_exact(2) {
        let (Some(&x), Some(&y)) = (corner.first(), corner.get(1)) else {
            continue;
        };
        let x = origin.0 + x * magnification;
        let y = origin.1 + (height - y) * magnification;
        bounds = [
            bounds[0].min(x),
            bounds[1].min(y),
            bounds[2].max(x),
            bounds[3].max(y),
        ];
    }
    bounds
}

/// §12.4.2's label for a page, where the document states a non-empty one.
fn label(open: &Open, index: usize) -> Option<String> {
    open.labels.label(index).filter(|label| !label.is_empty())
}

/// Turns a [`PageTarget`] into an index, clamped to the document.
fn resolve(open: &Open, target: PageTarget) -> Option<usize> {
    let (current, count) = (open.page_index, open.page_count);
    let last = count.checked_sub(1)?;
    Some(match target {
        PageTarget::Index(index) => index.min(last),
        PageTarget::First => 0,
        PageTarget::Last => last,
        PageTarget::Next => current.saturating_add(1).min(last),
        PageTarget::Previous => current.saturating_sub(1),
        PageTarget::Relative(delta) => current.saturating_add_signed(delta).min(last),
    })
}

/// Whether an annotation is a widget, which is what §12.6.3's Table 197 makes `/Fo` and `/Bl`
/// about — both entries are
///
/// > (Optional; PDF 1.2; widget annotations only)
///
/// and nothing else in the table carries that restriction.
fn is_widget(document: &pdf_syntax::Document, annotation: ObjectId) -> bool {
    let object = document.get(annotation);
    object.as_dict().is_some_and(|dict| {
        document
            .get_key(dict, "Subtype")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Widget")
    })
}

/// §8.11.4.3's `/Order`, turned into what a layer panel shows.
///
/// **Table 99's `/ListMode` is applied here and nowhere else**, because it is the one entry of
/// that table whose answer depends on the window rather than on the file:
///
/// > A name specifying which optional content groups in the Order array shall be displayed to
/// > the user. Valid values shall be: AllPages Display all groups in the Order array.
/// > VisiblePages Display only those groups in the Order array that are referenced by one or
/// > more visible pages.
///
/// This window shows one page at a time, so "one or more visible pages" is the page it is
/// showing — the same derivation §12.6.3's `/PV` and `/PO` took in the two-hundred-and-fourth
/// session, and the same one this row's stale reason denied for eighty-six: "which pages are
/// visible is a question about a window this crate does not have". `pdf_model` supplies the half
/// that is about the file, `groups_referenced_by`, and this supplies the half that is about the
/// window. One corpus document states the entry (`visibility_expressions.pdf`, on a scan of every
/// uncompressed `/ListMode` in all 974), and it states `VisiblePages`.
///
/// A collection whose children all disappear disappears with them: §8.11.4.3 makes a label
/// "non-selectable" and a heading over nothing is not what the clause asked to be displayed.
fn layers(open: &Open) -> Vec<Layer> {
    let Some(content) = open.view.optional_content() else {
        return Vec::new();
    };
    let shown = match content.list_mode() {
        ListMode::AllPages => None,
        ListMode::VisiblePages => open
            .shown_page()
            .map(|page| pdf_model::optional_content::groups_referenced_by(&open.document, page)),
    };
    build_layers(open, content, content.presentation(), shown.as_ref())
}

/// One level of `/Order`, and its children.
///
/// `shown` is `None` where every group is displayed, and the set a page references where
/// `/ListMode` is `VisiblePages`.
fn build_layers(
    open: &Open,
    content: &OptionalContent,
    entries: &[Presented],
    shown: Option<&BTreeSet<ObjectId>>,
) -> Vec<Layer> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Presented::Group(group) => {
                if shown.is_some_and(|shown| !shown.contains(group)) {
                    return None;
                }
                Some(Layer::Group {
                    group: *group,
                    name: content.name(&open.document, *group),
                    // A group `/Order` names but `/OCGs` does not is not a group this document
                    // configured, and Table 99 does not say what it is. It reads as on, which is
                    // what content governed by no group is.
                    on: content.state(*group).unwrap_or(true),
                    locked: content.is_locked(*group),
                })
            }
            Presented::Collection { label, children } => {
                let children = build_layers(open, content, children, shown);
                if shown.is_some() && children.is_empty() {
                    return None;
                }
                Some(Layer::Collection {
                    label: label.clone(),
                    children,
                })
            }
        })
        .collect()
}

/// Hands one embedded file's decoded bytes to the host, saying what §7.11.4 lets a file get wrong.
///
/// Two callers and one set of sentences: [`Command::Extract`], which names a file §7.7.4's tree
/// filed, and a click on §12.5.6.15's file attachment annotation, which names none because an
/// annotation files its file under nothing. Both owe the same three statements, which is why
/// this is one function rather than two that drifted.
fn hand_over(
    id: DocumentId,
    asked: Extraction,
    document: &pdf_syntax::Document,
    file: &pdf_model::attachment::Attachment,
    fragment: Option<String>,
    events: &mut Vec<Event>,
) {
    let name = file.file_name.clone().unwrap_or_else(|| file.name.clone());
    // Trap 5 on the one path where a *stream* can refuse: §7.6.6 makes a crypt filter's absent
    // key the stream's answer rather than the file's, and two of the corpus's twenty-three
    // attachments are exactly that. A silent empty file would be worse than none.
    let Some(bytes) = document.decoded_stream_data(&file.stream) else {
        events.push(Event::Reported {
            document: id,
            page: None,
            notes: vec![format!(
                "the embedded file {name:?} states a filter or an encryption this program cannot \
                 undo"
            )],
        });
        return;
    };
    // Table 45's `/CheckSum` is the one entry of §7.11.4 that can only be answered by somebody
    // holding the decoded bytes, and this is the only place in the program that does. Said and
    // not acted on: the clause calls it "strictly a checksum, and … not used for security
    // purposes", so a mismatch is the producer's mistake and withholding the file would tell a
    // person less than handing it over with a sentence.
    if file.checksum_matches(&bytes) == Some(false) {
        events.push(Event::Reported {
            document: id,
            page: None,
            notes: vec![format!(
                "the embedded file {name:?} does not match the MD5 checksum the document states \
                 for it (§7.11.4, Table 45)"
            )],
        });
    }
    events.push(Event::Extracted {
        document: id,
        asked,
        // Table 43's own name for the file where it states one, because that is what a person
        // would call it; the tree's key otherwise, which is all there is.
        name,
        bytes: bytes.to_vec(),
        // §O.2.1's remaining parameters, for the one caller that has a URI behind it.
        fragment,
    });
}

/// §12.5.1: what an annotation a person just clicked "exhibits", and whether the page changed.
///
/// > When the user activates the annotation by clicking it, it exhibits its associated object,
/// > such as by opening a popup window displaying a text note ("Figure 77 -Open annotation") or
/// > by playing a sound or a movie.
///
/// Two associated objects, and the clause's "such as" is why they are one function rather than
/// two conditions in the pointer arm. §12.5.6.14's window opens or closes (`Open::toggle_popup`,
/// ADR 0191). §12.5.6.15's *file* comes out: "[a] file attachment annotation contains a
/// reference to a file, which typically shall be embedded in the PDF file", and the clause says
/// what activating one does — "activating the annotation extracts the embedded file and gives
/// the user an opportunity to view it or store it in the file system". The bytes cross as
/// [`Event::Extracted`], which is where they cross for [`crate::Command::Extract`] too, and
/// where they land is the host's policy (rule 2).
///
/// **The file is reached here rather than from a document-wide list, and that is a
/// measurement**: a panel listing every page's attachments has to walk every page, which costs
/// 78 to 123 ms cold over three runs on ISO 32000-2's 1023 pages and 13 to 15 ms warm
/// (`pdf-model --example file_attachment_census`) — and every host asks [`Query::Attachments`]
/// when a document opens, which is exactly where `CLAUDE.md` forbids a full page-tree walk. A
/// click has already found its page. ADR 0295.
fn exhibit(id: DocumentId, open: &mut Open, annotation: ObjectId, events: &mut Vec<Event>) -> bool {
    let toggled = open.toggle_popup(annotation);
    for file in attached_files(open, annotation) {
        hand_over(id, Extraction::Asked, &open.document, &file, None, events);
    }
    toggled
}

/// What the attachments panel lists: the document's files, and §14.13.4's files of the page in
/// view.
///
/// §14.13.4 states the second population and states it of a *page*:
///
/// > One or more files may be associated with any PDF page by including a file specification
/// > dictionary (7.11.3, "File specification dictionaries") for each file as one of the members
/// > of the array value of the AF key in the appropriate page dictionary (7.7.3.3, "Page
/// > objects"). The relationship that the associated files have to the page is supplied by the
/// > AFRelationship key in each file specification dictionary.
///
/// **The page in view rather than every page, and that is principle 2 rather than a shortcut.**
/// A panel listing every page's associated files has to walk the whole page tree, which is what
/// `CLAUDE.md` forbids on the launch path and what every host would pay for, since each of them
/// asks [`Query::Attachments`] when a document opens — the same measurement ADR 0295 took for
/// §12.5.6.15's annotations, 78 to 123 ms cold over ISO 32000-2's 1023 pages. The page the reader
/// is on is a page this crate has already built, so the entry costs one dictionary lookup, and
/// the panel follows the reader exactly as `attached_files` follows the pointer.
///
/// `pdf_model::attachment::associated` had no caller anywhere in this tree that handed it a page
/// dictionary, which is the shape §14.13.3's catalog `/AF` and §12.5.6.15's `/FS` were both in
/// before a consumer reached them: a reading that exists, a row that calls it implemented, and a
/// payload no panel could list. ADR 1186.
///
/// One payload named twice is one file, deduplicated by the stream's identity as
/// `attachment::attachments` and [`attached_files`] both do it.
fn attachments_in_view(open: &Open) -> Vec<pdf_model::attachment::Attachment> {
    let mut out = open.view.attachments(&open.document);
    let Some(page) = open.page(open.page_index) else {
        return out;
    };
    for file in pdf_model::attachment::associated(&open.document, &page.dict) {
        if out
            .iter()
            .any(|seen| Arc::ptr_eq(&seen.stream, &file.stream))
        {
            continue;
        }
        out.push(file);
    }
    out
}

/// §12.3.6's preview picture for one attachment: its own first page's §12.3.4 `/Thumb`.
///
/// Three of Table 160's layouts are made of pictures of the attachments — a strip of thumbnails,
/// thumbnails scattered over the view, a large preview beside the metadata — and §12.3.4 is the
/// one place the standard says what a picture of a document's page is:
///
/// > The thumbnail image for a page shall be an image XObject specified by the Thumb entry in the
/// > page object
///
/// So the preview is the attachment's producer's own miniature of its first page, and this
/// program invents nothing where a file states none. `None` covers every such file: one that is
/// not a PDF, one whose first page states no `/Thumb`, one whose bytes this reader cannot decode.
/// A panel says which of its files had no picture rather than drawing a substitute (ADR 1251).
///
/// **Bounded by what already bounds a stream.** The attachment's bytes come out under
/// `pdf_syntax::Limits::max_stream_len` and the nested document parses under the same limits, so
/// a decompression bomb attached to a collection is refused by the bound that refuses one in the
/// containing file — no second number is chosen here (trap 38).
///
/// **One level.** The nested document is asked for a page's `/Thumb` and never for *its*
/// attachments, so a document embedding itself is one open rather than a walk.
fn attachment_preview(open: &Open, name: &str) -> Option<pdf_model::thumbnail::Thumbnail> {
    let file = open
        .view
        .attachments(&open.document)
        .into_iter()
        .find(|file| file.name == name)?;
    let bytes = open.document.decoded_stream_data(&file.stream)?;
    let embedded = pdf_syntax::Document::open(bytes.to_vec()).ok()?;
    let page = pdf_model::Pages::new(&embedded).get(0)?;
    pdf_model::thumbnail::read(&embedded, &page.dict)?.ok()
}

/// The files the annotation under the click carries: §12.5.6.15's, and §14.13.9's.
///
/// Empty for nearly every click, and the question is asked of one annotation the pointer already
/// found, so it costs two dictionary lookups rather than a walk.
///
/// **§12.5.6.15's `/FS` first**, because its clause is the one that states the extraction outright
/// — "activating the annotation extracts the embedded file and gives the user an opportunity to
/// view it or store it in the file system" — and it belongs to that subtype alone.
///
/// **Then §14.13.9's `/AF`, for every subtype**, which is that clause read through the one above
/// it: "[t]o associate files with annotations, the annotation dictionary shall contain an AF entry
/// which represents the associated files for that annotation", and §12.5.1 says what activating an
/// annotation does with what belongs to it — "it exhibits its associated object, such as by
/// opening a popup window displaying a text note … or by playing a sound or a movie". The *such
/// as* is examples rather than a closed list, and an associated file is an associated object in
/// the plainest sense the standard has.
///
/// The entry had no caller anywhere in this tree until the thousand-and-fifty-first session, which
/// is the shape §14.13.3's catalog `/AF` was in before `attachment::attachments` reached it: a
/// reading that exists, a row that calls it implemented, and a payload no host can extract.
///
/// One payload named both ways is one file, deduplicated by the stream's identity exactly as
/// `attachment::attachments` does it — a producer that writes a `/FS` and repeats it in `/AF` has
/// attached one file.
fn attached_files(open: &Open, annotation: ObjectId) -> Vec<pdf_model::attachment::Attachment> {
    let object = open.document.get(annotation);
    let Some(dict) = object.as_dict() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if open
        .document
        .get_key(dict, "Subtype")
        .as_name()
        .map(pdf_syntax::Name::as_bytes)
        == Some(b"FileAttachment")
        && let Some(file) = pdf_model::attachment::of_annotation(&open.document, dict)
    {
        out.push(file);
    }
    for file in pdf_model::attachment::associated(&open.document, dict) {
        if out
            .iter()
            .any(|seen| Arc::ptr_eq(&seen.stream, &file.stream))
        {
            continue;
        }
        out.push(file);
    }
    out
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
