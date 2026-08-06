//! The state machine itself.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pdf_model::action::Trigger;
use pdf_model::optional_content::{ListMode, OptionalContent, Presented};
use pdf_model::view::Pointer;
use pdf_render::{DisplayList, Point, Rect, TargetSpec, Transform};
use pdf_syntax::{ObjectId, SyntaxError};

use crate::command::{
    Command, PageTarget, PointerAction, Purpose, Rendered, Selection as CommandSelection, Zoom,
};
use crate::event::{Event, RenderRequest};
use crate::interact;
use crate::open::{Frame, Interpreted, Open, Pending};
use crate::query::{Answer, FrameView, Layer, PageGeometry, PopupWindow, Query, Selected};

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
/// The bounds that are not about allocation stay unconditional, and `TargetSpec::for_page`
/// applies them to every caller: a dimension over [`pdf_render::MAX_EXTENT`] is an `f32`
/// precision limit, and a degenerate one is a target that cannot exist.
const MAX_PIXELS: u64 = 1 << 28;

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
    /// Whether the host takes whole-page pixels from this crate — which is what [`MAX_PIXELS`]
    /// bounds, and the only case in which it should be applied.
    ///
    /// True until a host answers [`Rendered::Presented`], because a viewer that has not been
    /// told otherwise must assume it will be asked to hold a raster. A tier-2 host settles it
    /// on its first frame, which it draws at an opening magnification where the budget is not
    /// in question — so the conservative start costs nothing and the tier is never guessed.
    holds_rasters: bool,
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
            holds_rasters: true,
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

    /// Answers a question about the viewer's state without changing any of it.
    #[must_use]
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
            Query::Outline => Answer::Outline(&open.outline),
            Query::Layers => Answer::Layers(layers(open)),
            Query::Attachments => {
                Answer::Attachments(pdf_model::attachment::attachments(&open.document))
            }
            // Read here rather than held on `Open`: see `Query::Articles`. Two of the 974 corpus
            // documents state a `/Threads` entry at all, one of them an empty array and one a
            // null, so a list built at launch would be empty 999 times in a thousand.
            Query::Articles => {
                Answer::Articles(pdf_model::article::Articles::read(&open.document).threads)
            }
            Query::PageLabel(index) => label(open, index).map_or(Answer::None, Answer::Label),
            Query::Thumbnail(index) => pdf_model::Pages::new(&open.document)
                .get(index)
                .and_then(|page| pdf_model::thumbnail::read(&open.document, &page.dict))
                .and_then(Result::ok)
                .map_or(Answer::None, Answer::Thumbnail),
            Query::LinkAt(at) => Answer::Link(
                self.user_space(open, at)
                    .and_then(|(x, y)| interact::link_at(open, x, y))
                    .is_some(),
            ),
            Query::FieldAt(at) => self
                .page_point(open, at)
                .and_then(|(x, y)| {
                    let page = open.shown_page()?;
                    let (x, y) = pdf_model::content::user_space_at(page, x, y)?;
                    pdf_model::view::field_at(&open.document, page, x, y)
                })
                .map_or(Answer::None, |name| {
                    // The value is the *view*'s and the names are the document's, which is why
                    // they are gathered here rather than inside `field_at`: `pdf_model::view`'s
                    // walk knows the widget and this knows what has been typed into it.
                    let value = open.view.field_value(&open.document, &name.qualified);
                    Answer::Field { name, value }
                }),
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
            Query::Popups => Answer::Popups(self.popup_windows(open)),
            Query::Selection => self.selected(open).map_or(Answer::None, Answer::Selected),
            Query::LogicalSelection => {
                Self::logical_selection(open).map_or(Answer::None, Answer::LogicalSelection)
            }
            Query::Frame => open.frame.as_ref().map_or(Answer::None, |frame| {
                Answer::Frame(FrameView {
                    page: frame.page,
                    raster: &frame.raster,
                    origin: open.origin(self.viewport, (frame.target.width, frame.target.height)),
                })
            }),
            Query::AccessibilityTree => Answer::Accessibility(self.accessibility(open)),
            Query::Reports => open
                .interpreted
                .as_ref()
                .map_or(Answer::Reports(&[]), |interpreted| {
                    Answer::Reports(&interpreted.reports)
                }),
        }
    }

    /// Applies a command to the state, leaving what has to be *drawn* to [`Self::settle`].
    ///
    /// The split is what keeps a scheduling decision from being made twice: every command that
    /// changes what should be on the screen ends by having changed only the state, and exactly
    /// one place works out whether that means a new render.
    fn act(&mut self, command: Command, events: &mut Vec<Event>) {
        match command {
            Command::Open {
                id,
                bytes,
                password,
            } => self.open(id, bytes, password.as_deref(), events),
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
            Command::GoTo(target) => {
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
                if let Some(id) = self.focused {
                    self.page_events(id, Some(left), events);
                }
            }
            Command::Activate(object) => self.activate(object, events),
            Command::Tick { millis } => self.tick(millis, events),
            Command::Zoom { zoom, at } => self.set_zoom(zoom, at, events),
            Command::Scroll { dx, dy } => {
                let viewport = self.viewport;
                let Some(open) = self.focused_mut() else {
                    return;
                };
                let raster = open
                    .frame
                    .as_ref()
                    .map(|frame| (frame.target.width, frame.target.height));
                open.scroll = (open.scroll.0 + dx, open.scroll.1 + dy);
                if let Some(raster) = raster {
                    open.clamp_scroll(viewport, raster);
                }
                events.push(damage(viewport));
            }
            Command::Edit(edit) => self.edit(edit, events),
            Command::Undo => self.move_cursor(-1, events),
            Command::Redo => self.move_cursor(1, events),
            Command::Save => self.save(events),
            Command::Extract { name } => self.extract(&name, events),
            Command::Select(selection) => {
                let viewport = self.viewport;
                let Some(open) = self.focused_mut() else {
                    return;
                };
                open.selection = match selection {
                    CommandSelection::All => open
                        .interpreted
                        .as_ref()
                        .map(|interpreted| (0, interpreted.text.len())),
                    CommandSelection::None => None,
                };
                events.push(damage(viewport));
            }
            Command::Focused(move_to) => self.move_focus(move_to, events),
            Command::SetGroup { group, on } => {
                let Some(open) = self.focused_mut() else {
                    return;
                };
                if open.view.set_group(group, on) {
                    // §8.11 decides what is *drawn*, so a switch invalidates the display list
                    // and not merely the pixels.
                    open.interpreted = None;
                }
            }
            Command::Pointer { at, action } => self.pointer(at, action, events),
            Command::Supply { purpose, bytes } => self.supply(purpose, bytes.as_deref(), events),
            Command::RenderReady { token, rendered } => self.rendered(token, rendered, events),
        }
    }

    /// Opens a document, or says why it could not be.
    fn open(
        &mut self,
        id: DocumentId,
        bytes: Vec<u8>,
        password: Option<&str>,
        events: &mut Vec<Event>,
    ) {
        match Open::new(bytes, password) {
            Ok(open) => {
                let pages = open.page_count;
                let notes = crate::notes::about(&open.document);
                self.documents.insert(id, open);
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
                self.announce_page(events);
                // §12.6.3 puts `/PO` "after … the OpenAction entry in the document Catalog",
                // and `Open::around` has already applied that entry's destination — the page it
                // names is `open.page_index` and its view is waiting in `pending_view` — so the
                // first page's events are raised here, in the clause's order. An `/OpenAction`
                // that is an action rather than a destination is still not *performed*; that is
                // §12.6.4's row and not this one's, and it changes nothing about this ordering.
                self.page_events(id, None, events);
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

    /// Records what a worker did with a request, or drops it for being about the past.
    fn rendered(&mut self, token: RenderToken, rendered: Rendered, events: &mut Vec<Event>) {
        let viewport = self.viewport;
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused_mut() else {
            return;
        };
        // A token that is not the one outstanding answers a question that has been asked again
        // since. Dropping it is the whole reason the token exists.
        if open.pending.as_ref().map(|pending| pending.token) != Some(token) {
            return;
        }
        let Some(pending) = open.pending.take() else {
            return;
        };
        match rendered {
            Rendered::Raster(raster) => {
                open.clamp_scroll(viewport, (pending.target.width, pending.target.height));
                open.shown = Some((pending.page, pending.target, pending.revision));
                open.frame = Some(Frame {
                    page: pending.page,
                    target: pending.target,
                    raster,
                });
                events.push(damage(viewport));
            }
            // Tier 2: the host drew it onto its own surface, so there is nothing here to hold
            // and nothing to repaint from — but it *is* on the screen, and saying so is what
            // stops the scheduler asking for it again.
            Rendered::Presented => {
                open.shown = Some((pending.page, pending.target, pending.revision));
                open.frame = None;
                // Said once and remembered: this host draws its own frames at its own size, so
                // nothing here will hold a whole-page raster for it and `MAX_PIXELS` has
                // nothing to bound.
                self.holds_rasters = false;
            }
            // **A refusal is recorded as an answer**, and it has to be: the scheduler's question
            // is "is what is on the screen what should be", and a host that cannot draw this page
            // at this resolution will say so again the next time it is asked. Without this the
            // two of them spin — ask, refuse, ask — for as long as the page is shown. What
            // changes the answer is the question changing: another page, another zoom, another
            // interpretation, all of which move the tuple below.
            Rendered::Failed(reason) => {
                open.shown = Some((pending.page, pending.target, pending.revision));
                open.frame = None;
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
    fn pointer(&mut self, at: (f32, f32), action: PointerAction, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let viewport = self.viewport;
        let point = self.focused().and_then(|open| self.user_space(open, at));
        let on_page = self.focused().and_then(|open| self.page_point(open, at));
        let Some(open) = self.focused_mut() else {
            return;
        };
        // What a *click* activates is the link one, and only that: §12.5.6.5's activation region
        // is a link's, and `open.pressed` below decides whether a release follows one.
        let under = point.and_then(|(x, y)| interact::link_at(open, x, y));
        // §12.6.3's events belong to *any* annotation, and so — since the two-hundred-and-fifty-
        // third session — does §12.5.5's appearance. Asked once per pointer message, which is
        // what a `/Rect` test over a page's annotation array costs: the same shape
        // `Query::FieldAt` already pays at pointer speed.
        let over = point.and_then(|(x, y)| {
            let page = open.shown_page()?;
            let (x, y) = pdf_model::content::user_space_at(page, x, y)?;
            pdf_model::view::annotation_at(&open.document, page, &open.view, x, y)
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
            open.interpreted = None;
        }

        // Table 197's `/E` and `/X`, in the clause's own order: the cursor leaves one region
        // before it enters the next, and a document may act on both.
        let mut raised: Vec<(ObjectId, Trigger)> = Vec::new();
        if open.inside != over {
            raised.extend(open.inside.map(|left| (left, Trigger::Exit)));
            raised.extend(over.map(|entered| (entered, Trigger::Enter)));
            open.inside = over;
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
                let position = on_page.and_then(|point| open.position_at(point));
                open.selection = position.map(|position| (position, position));
                events.push(damage(viewport));
            }
            PointerAction::Dragged => {
                let position = on_page.and_then(|point| open.position_at(point));
                if let (Some((anchor, _)), Some(position)) = (open.selection, position) {
                    open.selection = Some((anchor, position));
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
                let selecting = open
                    .selection
                    .is_some_and(|(anchor, focus)| anchor != focus);
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
                // session**, and that is a clause rather than a tidy-up: the table conditions the
                // event on the *release* being inside the area and on nothing else, while this
                // arm used to return early whenever the release did not activate a link — so a
                // click on a stamp, a widget or a markup annotation raised nothing at all.
                if clicked && let Some(annotation) = over.filter(|over| Some(*over) != under) {
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
                        .is_some_and(|annotation| open.toggle_popup(annotation));
                if toggled {
                    events.push(damage(viewport));
                }
                if selecting || pressed.is_none() || pressed != under {
                    self.raise(id, raised, events);
                    return;
                }
                let Some((x, y)) = point else {
                    self.raise(id, raised, events);
                    return;
                };
                self.raise(id, raised, events);
                let Some(open) = self.focused_mut() else {
                    return;
                };
                let outcome = interact::activate(open, x, y);
                self.apply(id, outcome, events);
                return;
            }
        }
        self.raise(id, raised, events);
    }

    /// Performs §12.6.3's events the pointer just raised, in the order they were raised.
    fn raise(&mut self, id: DocumentId, raised: Vec<(ObjectId, Trigger)>, events: &mut Vec<Event>) {
        for (annotation, event) in raised {
            let Some(open) = self.focused_mut() else {
                return;
            };
            let outcome = interact::trigger(open, annotation, event);
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
            (Purpose::ImportData, Some(bytes)) => interact::import(open, bytes),
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
        };
        self.apply(id, outcome, events);
    }

    /// Turns what a click asked for into events, and does the parts that are this crate's.
    fn apply(&mut self, id: DocumentId, outcome: interact::Outcome, events: &mut Vec<Event>) {
        let page = self.focused().map(|open| open.page_index);
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
        if let Some((purpose, name)) = outcome.needs_file {
            events.push(Event::NeedsFile {
                document: id,
                purpose,
                name,
            });
        }
        for transition in outcome.transitions {
            events.push(Event::Transition {
                document: id,
                transition,
            });
        }

        // §12.6.4.4 replaces the document every other request was about, so a page named by one
        // of them is a page of a document that is no longer open.
        if let Some(replacement) = outcome.replacement {
            let pages = replacement.page_count;
            let notes = crate::notes::about(&replacement.document);
            self.documents.insert(id, *replacement);
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
            self.announce_page(events);
            self.page_events(id, None, events);
            return;
        }

        let Some(open) = self.focused_mut() else {
            return;
        };
        if outcome.redraw {
            open.interpreted = None;
        }
        // Even a destination naming the page already showing states where to look at it, which
        // is what an outline item pointing at a heading half way down a page is for.
        if outcome.view.is_some() {
            open.pending_view = outcome.view;
        }
        if let Some(target) = outcome.target
            && target != open.page_index
        {
            let left = open.page_index;
            open.page_index = target;
            open.scroll = (0.0, 0.0);
            self.announce_page(events);
            self.page_events(id, Some(left), events);
        }
    }

    /// Writes §7.5.6's incremental update, or says why it could not be written.
    fn save(&mut self, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused() else { return };
        match open.view.save(&open.document) {
            Ok(bytes) => events.push(Event::Saved {
                document: id,
                bytes,
            }),
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
    fn extract(&mut self, name: &str, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused() else { return };
        let Some(file) = pdf_model::attachment::attachments(&open.document)
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
        // Trap 5 on the one path where a *stream* can refuse: §7.6.6 makes a crypt filter's
        // absent key the stream's answer rather than the file's, and two of the corpus's
        // twenty-three attachments are exactly that. A silent empty file would be worse than
        // none.
        let Some(bytes) = open.document.decoded_stream_data(&file.stream) else {
            events.push(Event::Reported {
                document: id,
                page: None,
                notes: vec![format!(
                    "the embedded file {name:?} states a filter or an encryption this program \
                     cannot undo"
                )],
            });
            return;
        };
        // Table 45's `/CheckSum` is the one entry of §7.11.4 that can only be answered by
        // somebody holding the decoded bytes, and this is the only place in the program that
        // does. Said and not acted on: the clause calls it "strictly a checksum, and … not used
        // for security purposes", so a mismatch is the producer's mistake and withholding the
        // file would tell a person less than handing it over with a sentence.
        if file.checksum_matches(&bytes) == Some(false) {
            events.push(Event::Reported {
                document: id,
                page: None,
                notes: vec![format!(
                    "the embedded file {name:?} does not match the MD5 checksum the document \
                     states for it (§7.11.4, Table 45)"
                )],
            });
        }
        events.push(Event::Extracted {
            document: id,
            // Table 43's own name for the file where it states one, because that is what a
            // person would call it; the tree's key otherwise, which is all there is.
            name: file.file_name.clone().unwrap_or_else(|| file.name.clone()),
            bytes: bytes.to_vec(),
        });
    }

    /// Adds one edit to the log and applies it.
    ///
    /// A new edit after an undo discards what was undone: the log is one sequence with a cursor,
    /// which is what makes a replay of its prefix the whole of the state.
    fn edit(&mut self, edit: crate::command::Edit, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused_mut() else {
            return;
        };
        // What was *done*, rather than what was asked for: `Edit::Markup` names its target as
        // "what is selected", and a replay after the selection moved would mark up something
        // else. See `open::Done`.
        let Some(done) = open.resolve(edit) else {
            return;
        };
        let before = open.dirty();
        open.log.truncate(open.cursor);
        open.log.push(done);
        open.cursor = open.log.len();
        open.replay();
        if open.dirty() != before {
            events.push(Event::Dirty {
                document: id,
                dirty: open.dirty(),
            });
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
        open.cursor = cursor;
        open.replay();
        if open.dirty() != before {
            events.push(Event::Dirty {
                document: id,
                dirty: open.dirty(),
            });
        }
    }

    /// Every occurrence of `needle` on the page being shown, as shapes in device pixels.
    fn found(&self, open: &Open, needle: &str) -> Vec<Vec<[f32; 8]>> {
        let Some(interpreted) = open.interpreted.as_ref() else {
            return Vec::new();
        };
        crate::select::find(&interpreted.text, needle)
            .into_iter()
            .map(|range| self.device_quads(open, range))
            .collect()
    }

    /// What is selected, with its shapes in device pixels.
    fn selected<'a>(&self, open: &'a Open) -> Option<Selected<'a>> {
        let (anchor, focus) = open.selection?;
        let interpreted = open.interpreted.as_ref()?;
        let (from, to) = (anchor.min(focus), anchor.max(focus));
        let text = interpreted.text.get(from..to).unwrap_or_default();
        Some(Selected {
            text,
            quads: self.device_quads(open, (from, to)),
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
        Some((object, self.device_quad(open, [x0, y0, x1, y1])?))
    }

    /// §12.5.6.14's open popup windows, placed on the screen.
    ///
    /// The state is the file's `/Open` unless a person has said otherwise since, which is what
    /// `Open::popups` holds and why Table 186's word "initially" is load-bearing.
    fn popup_windows(&self, open: &Open) -> Vec<PopupWindow> {
        let Some(page) = open.shown_page() else {
            return Vec::new();
        };
        pdf_model::popup::popups(&open.document, page, &open.view)
            .into_iter()
            .filter(|popup| open.popup_is_open(popup))
            .filter_map(|popup| {
                Some(PopupWindow {
                    annotation: popup.annotation,
                    parent: popup.parent,
                    quad: self.device_quad(open, popup.rect)?,
                    title: popup.title,
                    text: popup.text,
                    modified: popup.modified,
                    colour: popup.colour,
                })
            })
            .collect()
    }

    /// A rectangle in default user space, as the quadrilateral a host draws over the page.
    ///
    /// **One copy of this arithmetic and deliberately not one per caller**: the origin, the
    /// magnification and the y flip are exactly what was wrong for seventy-five sessions (ADR
    /// 0118), and a second opinion about them in a host — or in a second method here — would be
    /// that defect again. `[x0, y0, x1, y1]`, normalised, in; clockwise from the top-left as it
    /// appears on the screen, out.
    fn device_quad(&self, open: &Open, rect: [f32; 4]) -> Option<[f32; 8]> {
        let interpreted = open.interpreted.as_ref()?;
        let magnification = open.magnification(self.viewport, self.scale)?;
        let height = open.page_size(open.page_index).map(|size| size.height)?;
        let raster = crate::open::raster_extent(interpreted.list.page_size, magnification);
        let origin = open.origin(self.viewport, raster);
        let place = |x: f32, y: f32| {
            (
                origin.0 + x * magnification,
                origin.1 + (height - y) * magnification,
            )
        };
        let [x0, y0, x1, y1] = rect;
        // Clockwise from the top-left as it appears on the screen, which is the order
        // `Selected::quads` uses and the order a host strokes a ring in.
        let (a, b, c, d) = (place(x0, y1), place(x1, y1), place(x1, y0), place(x0, y0));
        Some([a.0, a.1, b.0, b.1, c.0, c.1, d.0, d.1])
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
                let Some(page_id) = pdf_model::Pages::new(&open.document)
                    .indices()
                    .into_iter()
                    .find(|(_, index)| *index == open.page_index)
                    .map(|(object, _)| object)
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
        self.raise(id, raised, events);
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
        let (anchor, focus) = open.selection?;
        let interpreted = open.interpreted.as_ref()?;
        let tree = pdf_model::structure::Tree::of(&open.document)?;
        // As `accessibility` does, and for the same reason: Table 355's `/Pg` names a page
        // *object* and what this crate holds is an index, so the page tree is inverted once.
        let page = pdf_model::Pages::new(&open.document)
            .indices()
            .into_iter()
            .find(|(_, index)| *index == interpreted.page)
            .map(|(object, _)| object)?;
        tree.logical_range(
            &open.document,
            page,
            &interpreted.text,
            &interpreted.marked,
            anchor.min(focus)..anchor.max(focus),
        )
    }

    /// §14.7's structure tree for the page being shown, with §14.9's entries applied.
    ///
    /// Built on demand rather than kept, because a screen reader asks when it attaches and on a
    /// page change, and no other consumer asks at all — while a drag asks
    /// [`Query::Selection`] sixty times a second, which is why *that* one's inputs are cached.
    fn accessibility(&self, open: &Open) -> Vec<crate::AccessibilityNode> {
        let Some(interpreted) = open.interpreted.as_ref() else {
            return Vec::new();
        };
        // Table 355's `/Pg` names a page *object*, and what this crate holds is an index — so the
        // page tree is walked once to invert it. Session 141's `Pages::indices` is what makes
        // that one walk rather than one per element; `Pages::index_of` in a loop is the defect
        // ADR 0124 is about.
        let pages = pdf_model::Pages::new(&open.document);
        let Some(page) = pages
            .indices()
            .into_iter()
            .find(|(_, index)| *index == interpreted.page)
            .map(|(object, _)| object)
        else {
            return Vec::new();
        };
        crate::accessibility::nodes(
            &open.document,
            page,
            &interpreted.text,
            &interpreted.marked,
            &interpreted.described,
            interpreted.language.as_deref(),
        )
        .into_iter()
        .map(|(parent, gathered)| {
            crate::accessibility::finish(
                gathered,
                parent,
                &interpreted.text,
                &interpreted.marked,
                &interpreted.described,
                |start, end| self.device_quads(open, (start, end)),
            )
        })
        .collect()
    }

    /// The shapes covering a range of the readback, in device pixels of the viewport.
    ///
    /// The mapping a host would otherwise have to do, and the reason it does not: it would mean
    /// re-deriving the magnification, the centring and the y flip, which is exactly the
    /// arithmetic ADR 0118 found wrong in the one place it existed.
    fn device_quads(&self, open: &Open, range: (usize, usize)) -> Vec<[f32; 8]> {
        let Some(interpreted) = open.interpreted.as_ref() else {
            return Vec::new();
        };
        let Some(magnification) = open.magnification(self.viewport, self.scale) else {
            return Vec::new();
        };
        let Some(height) = open.page_size(open.page_index).map(|size| size.height) else {
            return Vec::new();
        };
        let raster = crate::open::raster_extent(interpreted.list.page_size, magnification);
        let origin = open.origin(self.viewport, raster);
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

    /// Maps a viewport point to the display list's own coordinates.
    ///
    /// The half of [`Self::user_space`] that stops before the page's own transform: the display
    /// list, the text layer and every quadrilateral this crate hands out are in *this* space, and
    /// only §12.5.2's annotation rectangles are in the other one.
    fn page_point(&self, open: &Open, at: (f32, f32)) -> Option<(f32, f32)> {
        let magnification = open.magnification(self.viewport, self.scale)?;
        if magnification <= 0.0 {
            return None;
        }
        let interpreted = open.interpreted.as_ref()?;
        let raster = crate::open::raster_extent(interpreted.list.page_size, magnification);
        let origin = open.origin(self.viewport, raster);
        let height = open.page_size(open.page_index)?.height;
        Some((
            (at.0 - origin.0) / magnification,
            height - (at.1 - origin.1) / magnification,
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
    fn user_space(&self, open: &Open, at: (f32, f32)) -> Option<(f32, f32)> {
        let (x, y) = self.page_point(open, at)?;
        pdf_model::content::user_space_at(open.shown_page()?, x, y)
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
            open.hold(viewport, before, after, at);
        }
        events.push(damage(viewport));
    }

    /// The pixel budget a render request is held to, which is the host's tier and not a
    /// property of the page.
    ///
    /// [`MAX_PIXELS`] bounds a raster this crate hands back, so it binds a host that takes one
    /// and says nothing to a host that draws its own frames at its own size. What remains for
    /// both is `TargetSpec::for_page`'s own refusal of a dimension `f32` cannot resolve.
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
    #[expect(
        clippy::too_many_lines,
        reason = "the whole of what settling a viewer means, in the order it happens: the \
                  magnification, the page, the interpretation, the scheduler's token. Splitting \
                  it would put that order in two places"
    )]
    fn settle(&mut self, events: &mut Vec<Event>) {
        let (viewport, scale) = (self.viewport, self.scale);
        // A viewport with no extent is a window that has not been laid out yet, and rendering
        // into it would be a request for a zero-pixel raster.
        if viewport.0 == 0 || viewport.1 == 0 {
            return;
        }
        let Some(id) = self.focused else { return };
        let token = RenderToken(self.next_token);
        // Read before the document is borrowed: it is a fact about the host, not the page.
        let budget = self.raster_budget();
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        let page = open.page_index;

        // §12.5.3's `NoZoom` makes one annotation's placement a function of the magnification,
        // so the state interpretation reads has to carry it — and **logical** pixels per user
        // unit rather than device ones, because "the same fixed size on the screen" is a size a
        // person sees and a doubled display should draw it sharper rather than smaller.
        //
        // Re-interpreting only where the last interpretation said the page would notice is what
        // keeps the display list's promise: 923 of the 974 corpus documents have no such
        // annotation and pay nothing for this, and the 51 that do pay exactly what the clause
        // asks for.
        let magnification = open
            .magnification(self.viewport, scale)
            .map(|device| device / scale);
        if open.view.set_magnification(magnification)
            && open
                .interpreted
                .as_ref()
                .is_some_and(|interpreted| interpreted.view_dependent)
        {
            open.interpreted = None;
        }

        if open
            .interpreted
            .as_ref()
            .map(|interpreted| interpreted.page)
            != Some(page)
        {
            // Whether this is a *different* page or the same one drawn again, which is the
            // difference the selection turns on below. Asked of `current` rather than of
            // `interpreted`, because the display list is what a re-interpretation throws away:
            // by this line it is already `None` for both cases and could not tell them apart.
            let turned = open
                .current
                .as_ref()
                .is_none_or(|(interpreted, _)| *interpreted != page);
            let Some((interpretation, reports, object)) = crate::open::interpret(open, page) else {
                return;
            };
            if !reports.is_empty() {
                events.push(Event::Reported {
                    document: id,
                    page: Some(page),
                    notes: reports.clone(),
                });
            }
            open.revision = open.revision.saturating_add(1);
            open.interpreted = Some(Interpreted {
                page,
                list: Arc::new(interpretation.display_list),
                reports,
                text: interpretation.text,
                placed: interpretation.text_layer,
                marked: interpretation.marked,
                described: interpretation.described,
                language: interpretation.language,
                view_dependent: interpretation.view_dependent,
            });
            open.current = Some((page, object));
            // A selection is a range of *this page's* readback, so a page turn ends it — and a
            // re-interpretation of the same page does not. The two are different events and this
            // line treated them alike: a field edited, a layer switched or an annotation added
            // rebuilds the display list, and every one of those took a person's selection away
            // from them. The readback of a page is a function of the document and the view state
            // and both are the same page's, so the range still names what it named.
            if turned {
                open.selection = None;
            }
        }
        let Some(interpreted) = open.interpreted.as_ref() else {
            return;
        };
        let list = Arc::clone(&interpreted.list);

        // §12.3.2.1's location and magnification, applied here because this is the first place
        // both of the things they need exist: a viewport, and — for the three `/FitB` forms —
        // a display list to measure the page's contents from. ADR 0162.
        if let Some(view) = open.pending_view.take() {
            let bounds = content_bounds(&list);
            if open.apply_view(view, viewport, scale, bounds) {
                open.shown = None;
            }
        }

        let Some(magnification) = open.magnification(viewport, scale) else {
            return;
        };
        let target = match TargetSpec::for_page(&list, magnification, budget) {
            Ok(target) => target,
            // Named rather than clamped, because a page silently drawn at a scale nobody chose
            // is a page a person has been told something false about.
            Err(error) => {
                events.push(Event::Reported {
                    document: id,
                    page: Some(page),
                    notes: vec![format!("this page cannot be drawn at this size: {error}")],
                });
                return;
            }
        };
        open.clamp_scroll(viewport, (target.width, target.height));

        let showing = open.shown == Some((page, target, open.revision));
        let asked = open.pending.as_ref().is_some_and(|pending| {
            pending.page == page && pending.target == target && pending.revision == open.revision
        });
        if showing || asked {
            return;
        }

        self.next_token = self.next_token.saturating_add(1);
        open.pending = Some(Pending {
            token,
            page,
            target,
            revision: open.revision,
        });
        events.push(Event::NeedsRender(RenderRequest {
            token,
            document: id,
            page,
            list,
            target,
        }));
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
        let Some(open) = self.focused_mut() else {
            return;
        };
        // Milliseconds in, seconds out, because the clause counts in seconds and a host counts
        // in whatever its event loop gives it. `f32` loses exactness above sixteen million
        // milliseconds — four and a half hours on one page — and what is being measured is a
        // duration a person perceives, which `pdf_model::navigation` narrows for the same reason.
        let seconds =
            f32::from(u16::try_from(millis.min(u32::from(u16::MAX))).unwrap_or(u16::MAX)) / 1000.0;
        open.shown_for += seconds;
        let Some((_, page)) = open.current.as_ref() else {
            return;
        };
        let Some(duration) = pdf_model::navigation::display_duration(&open.document, &page.dict)
        else {
            return;
        };
        if open.shown_for < duration || open.page_index.saturating_add(1) >= open.page_count {
            return;
        }
        self.act(Command::GoTo(PageTarget::Next), events);

        // §12.4.4.1: "the transition style that shall be used when moving to this page from
        // another during a presentation". *This* page, so it is the page arrived at whose
        // `/Trans` plays, and it is named here rather than on every page turn because a turn is
        // only part of a presentation when something is driving the clock — which is the one
        // thing this crate can tell from a `Tick`.
        //
        // The page is fetched rather than read from `Open::current`, because `current` is filled
        // during interpretation and interpretation happens in `settle` — after this. Reading it
        // here would name the transition of the page just *left*, which is the same off-by-one
        // §12.4.4.1's own wording rules out. One page-tree walk per automatic advance, which is
        // at most one a second and not the per-item loop ADR 0124 was about.
        let (Some(id), Some(open)) = (self.focused, self.focused()) else {
            return;
        };
        let pages = pdf_model::Pages::new(&open.document);
        let Some(page) = pages.get(open.page_index) else {
            return;
        };
        if let Some(transition) = pdf_model::navigation::transition(&open.document, &page.dict) {
            events.push(Event::Transition {
                document: id,
                transition,
            });
        }
    }

    fn announce_page(&mut self, events: &mut Vec<Event>) {
        let (Some(id), Some(open)) = (self.focused, self.focused()) else {
            return;
        };
        let index = open.page_index;
        let section = open
            .outline
            .section_at(
                &open.document,
                &pdf_model::Pages::new(&open.document),
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
        self.raise(id, raised, events);
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
        self.raise(id, raised, events);
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
        if index != open.page_index {
            return None;
        }
        let page = open.page_size(index)?;
        let magnification = open.magnification(self.viewport, self.scale)?;
        let interpreted = open.interpreted.as_ref()?;
        let raster = crate::open::raster_extent(interpreted.list.page_size, magnification);
        Some(PageGeometry {
            page,
            scale: magnification,
            width: raster.0,
            height: raster.1,
            origin: open.origin(self.viewport, raster),
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
    let entry = open.document.get_key(&page.dict, "Annots");
    let Some(array) = entry.as_array() else {
        return Vec::new();
    };
    array
        .iter()
        .filter_map(pdf_syntax::Object::as_reference)
        .collect()
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
