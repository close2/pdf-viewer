//! The state machine itself.

use std::collections::BTreeMap;
use std::sync::Arc;

use pdf_model::optional_content::{OptionalContent, Presented};
use pdf_render::{Point, Rect, TargetSpec};
use pdf_syntax::SyntaxError;

use crate::command::{Command, PageTarget, Rendered, Zoom};
use crate::event::{Event, RenderRequest};
use crate::open::{Frame, Interpreted, Open, Pending};
use crate::query::{Answer, FrameView, Layer, PageGeometry, Query};

/// Pixel budget for one rendered page.
///
/// Page dimensions come from the document and the scale from the viewport, so the product needs
/// a bound: a page claiming absurd dimensions must fail to render rather than ask for all
/// available memory. A page over the budget is *named* rather than quietly drawn smaller — a
/// silent cap is a defect, not safety.
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
    pub fn query(&self, query: Query) -> Answer<'_> {
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
            Query::Frame => open.frame.as_ref().map_or(Answer::None, |frame| {
                Answer::Frame(FrameView {
                    page: frame.page,
                    raster: &frame.raster,
                    origin: open.origin(self.viewport, (frame.target.width, frame.target.height)),
                })
            }),
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
                let Some(index) = resolve(target, open.page_index, open.page_count) else {
                    return;
                };
                if index == open.page_index {
                    return;
                }
                open.page_index = index;
                // A new page starts at its top: carrying a scroll position across a page turn
                // would land a reader in the middle of a page they have not seen.
                open.scroll = (0.0, 0.0);
                self.announce_page(events);
            }
            Command::Zoom(zoom) => self.set_zoom(zoom, events),
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
                self.documents.insert(id, open);
                self.focused = Some(id);
                events.push(Event::Opened {
                    document: id,
                    pages,
                });
                self.announce_page(events);
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
                open.frame = Some(Frame {
                    page: pending.page,
                    target: pending.target,
                    raster,
                });
                events.push(damage(viewport));
            }
            // Tier 2: the host drew it onto its own surface, so there is nothing here to hold
            // and nothing to repaint from.
            Rendered::Presented => open.frame = None,
            Rendered::Failed(reason) => events.push(Event::Reported {
                document: id,
                page: pending.page,
                notes: vec![reason],
            }),
        }
    }

    /// Resolves a zoom command into the magnification it lands on.
    ///
    /// A step is resolved here rather than stored, because two steps have to compose and "one
    /// larger than fitted" is not a state a mode can hold. Resolving also keeps the viewport's
    /// centre where it was, which is what makes zooming feel like magnification rather than
    /// like jumping to a corner.
    fn set_zoom(&mut self, zoom: Zoom, events: &mut Vec<Event>) {
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
            Zoom::FitPage | Zoom::FitWidth | Zoom::Scale(_) => open.zoom = zoom,
        }
        // Both scrolls are in device pixels of a raster whose size changed by exactly this
        // ratio, so scaling them about the viewport's centre keeps that point where it was.
        if let (Some(before), Some(after)) = (before, open.magnification(viewport, scale))
            && before > 0.0
        {
            let ratio = after / before;
            let recentre = |scroll: f32, extent: u32| {
                let half = px(extent) / 2.0;
                ((scroll + half) * ratio - half).max(0.0)
            };
            open.scroll = (
                recentre(open.scroll.0, viewport.0),
                recentre(open.scroll.1, viewport.1),
            );
        }
        events.push(damage(viewport));
    }

    /// Works out whether what is on the screen is what should be, and asks for a render if not.
    ///
    /// The one place a render is scheduled. Interpretation happens here too, which is what makes
    /// [`Event::NeedsRender`] self-contained — see the crate documentation on who interprets and
    /// who rasterises.
    fn settle(&mut self, events: &mut Vec<Event>) {
        let (viewport, scale) = (self.viewport, self.scale);
        // A viewport with no extent is a window that has not been laid out yet, and rendering
        // into it would be a request for a zero-pixel raster.
        if viewport.0 == 0 || viewport.1 == 0 {
            return;
        }
        let Some(id) = self.focused else { return };
        let token = RenderToken(self.next_token);
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        let page = open.page_index;

        if open
            .interpreted
            .as_ref()
            .map(|interpreted| interpreted.page)
            != Some(page)
        {
            let Some((interpretation, reports)) = crate::open::interpret(open, page) else {
                return;
            };
            if !reports.is_empty() {
                events.push(Event::Reported {
                    document: id,
                    page,
                    notes: reports.clone(),
                });
            }
            open.interpreted = Some(Interpreted {
                page,
                list: Arc::new(interpretation.display_list),
                reports,
            });
        }
        let Some(interpreted) = open.interpreted.as_ref() else {
            return;
        };
        let list = Arc::clone(&interpreted.list);

        let Some(magnification) = open.magnification(viewport, scale) else {
            return;
        };
        let target = match TargetSpec::for_page(&list, magnification, MAX_PIXELS) {
            Ok(target) => target,
            // Named rather than clamped, because a page silently drawn at a scale nobody chose
            // is a page a person has been told something false about.
            Err(error) => {
                events.push(Event::Reported {
                    document: id,
                    page,
                    notes: vec![format!("this page cannot be drawn at this size: {error}")],
                });
                return;
            }
        };
        open.clamp_scroll(viewport, (target.width, target.height));

        let showing = open
            .frame
            .as_ref()
            .is_some_and(|frame| frame.page == page && frame.target == target);
        let asked = open
            .pending
            .as_ref()
            .is_some_and(|pending| pending.page == page && pending.target == target);
        if showing || asked {
            return;
        }

        self.next_token = self.next_token.saturating_add(1);
        open.pending = Some(Pending {
            token,
            page,
            target,
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
        let target = TargetSpec::for_page(&interpreted.list, magnification, MAX_PIXELS).ok()?;
        Some(PageGeometry {
            page,
            scale: magnification,
            width: target.width,
            height: target.height,
            origin: open.origin(self.viewport, (target.width, target.height)),
        })
    }
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
fn px(value: u32) -> f32 {
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
fn resolve(target: PageTarget, current: usize, count: usize) -> Option<usize> {
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

/// §8.11.4.3's `/Order`, turned into what a layer panel shows.
fn layers(open: &Open) -> Vec<Layer> {
    let Some(content) = open.view.optional_content() else {
        return Vec::new();
    };
    build_layers(open, content, content.presentation())
}

/// One level of `/Order`, and its children.
fn build_layers(open: &Open, content: &OptionalContent, entries: &[Presented]) -> Vec<Layer> {
    entries
        .iter()
        .map(|entry| match entry {
            Presented::Group(group) => Layer::Group {
                group: *group,
                name: content.name(&open.document, *group),
                // A group `/Order` names but `/OCGs` does not is not a group this document
                // configured, and Table 99 does not say what it is. It reads as on, which is
                // what content governed by no group is.
                on: content.state(*group).unwrap_or(true),
                locked: content.is_locked(*group),
            },
            Presented::Collection { label, children } => Layer::Collection {
                label: label.clone(),
                children: build_layers(open, content, children),
            },
        })
        .collect()
}
