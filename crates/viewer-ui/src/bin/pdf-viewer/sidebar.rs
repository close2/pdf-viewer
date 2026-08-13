//! The panel this program draws for itself, and where the pointer is with respect to it.
//!
//! `viewer_ui::chrome` knows how to draw a sidebar and a modal card; what is here is everything
//! that decides *what* it draws and *what a click on it means* — which is a host's, because the
//! three lists come from three queries and the fourth thing on the screen is `/NOTICE`. The
//! pointer lives here too: whether it is over the panel or over the page is the first question
//! every gesture asks, and answering it twice in two modules is how the two would come apart.

use viewer_core::{Answer, Command, Layer, PageTarget, PointerAction, Query, Zoom};
use viewer_ui::chrome::{Content, Hit};

use crate::app::{App, at};
use crate::typing::Typing;

/// How far a touchpad must be dragged under Ctrl for one zoom step.
///
/// A choice, not a derivation: a notch of a mouse wheel is one step by construction and a
/// touchpad reports a stream of pixels instead, so something has to say how many of them a notch
/// is worth. Fifty is about a finger's width on this machine's touchpad and gives roughly the
/// same number of steps per gesture as the wheel does per flick.
const WHEEL_ZOOM_PIXELS: f32 = 50.0;

impl App {
    /// The panel's own display list for this frame, or `None` when there is nothing to draw.
    ///
    /// Rebuilt per frame rather than kept: it is a few hundred glyph fills against the page's
    /// tens of thousands, and a cache would be one more thing that can disagree with the scroll
    /// position.
    pub(crate) fn panel_list(&self, height: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        if !self.panel.shown {
            return None;
        }
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        let layers = self.layers();
        Some(
            self.panel
                .draw(chrome, self.content(&layers), height, scale),
        )
    }

    /// The About card's display list for this frame, or `None` when it is not shown.
    pub(crate) fn about_list(&self, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        if !self.about.shown {
            return None;
        }
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        Some(self.about.draw(chrome, crate::NOTICE, width, height, scale))
    }

    /// §8.11.4.3's `/Order`, asked for fresh.
    ///
    /// Unlike the outline and the attachments this is *not* cached: a click on a layer's switch
    /// changes it, so a copy taken when the document opened would be the one thing on the panel
    /// that lies.
    pub(crate) fn layers(&self) -> Vec<Layer> {
        match self.viewer.query(Query::Layers) {
            Answer::Layers(layers) => layers,
            _ => Vec::new(),
        }
    }

    /// The three lists, gathered for one call into the sidebar.
    fn content<'a>(&'a self, layers: &'a [Layer]) -> Content<'a> {
        Content {
            outline: &self.outline,
            layers,
            attachments: &self.attachments,
            articles: &self.articles,
            collection: self.collection.as_ref().map(|(collection, initial)| {
                viewer_ui::chrome::Presentation {
                    collection,
                    initial,
                }
            }),
            information: &self.information,
            metadata: self.metadata.as_ref(),
            pages: &self.pages,
        }
    }

    /// Builds §12.3.4's page list, once, the first time its tab is shown.
    ///
    /// Called from `present`, which is the one place that runs before the panel is drawn and
    /// holds `&mut self`. A document with no thumbnails at all still gets a list — the rows are
    /// its pages, and §12.3.4's NOTE is why a page without one is still a page.
    pub(crate) fn ensure_pages(&mut self) {
        if !self.panel.shows_pages() || !self.pages.is_empty() {
            return;
        }
        let Answer::Count(count) = self.viewer.query(Query::PageCount) else {
            return;
        };
        self.pages = (0..count)
            .map(|index| {
                let label = match self.viewer.query(Query::PageLabel(index)) {
                    Answer::Label(label) => label,
                    _ => format!("Page {}", index.saturating_add(1)),
                };
                let thumbnail = match self.viewer.query(Query::Thumbnail(index)) {
                    Answer::Thumbnail(thumbnail) => Some(thumbnail.image),
                    _ => None,
                };
                viewer_ui::chrome::Page { label, thumbnail }
            })
            .collect();
    }

    /// What the pointer moving does: the panel's highlight, or the page's §12.5.5 appearance.
    ///
    /// Only one of the two, and never both: a hover highlight in the panel and a rollover
    /// appearance on the page are both answers to "what is under the pointer", and answering
    /// both would leave an annotation lit up behind a panel.
    pub(crate) fn pointer_moved(&mut self) {
        if self.about.shown {
            return;
        }
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        let layers = self.layers();
        // The struct is written out here rather than built by `content`: `self.panel` is
        // borrowed mutably, and only a *field* borrow of the other three is disjoint from it.
        let moved = self.panel.hover(
            at(self.cursor),
            Content {
                outline: &self.outline,
                layers: &layers,
                attachments: &self.attachments,
                articles: &self.articles,
                collection: self.collection.as_ref().map(|(collection, initial)| {
                    viewer_ui::chrome::Presentation {
                        collection,
                        initial,
                    }
                }),
                information: &self.information,
                metadata: self.metadata.as_ref(),
                pages: &self.pages,
            },
            scale,
        );
        drop(layers);
        if moved {
            self.redraw();
        }
        if self.over_panel() {
            if let Some(state) = self.state.as_ref() {
                state.window.set_cursor(winit::window::CursorIcon::Default);
            }
            return;
        }
        let point = self.on_page(self.cursor);
        // **A drag that began inside a field belongs to that field's value**, and the page's own
        // selection is not asked to extend: two highlights over one gesture would say the person
        // had swept the page as well. The anchor stays where the press put it and the caret
        // follows the pointer, which is what makes the pair a selection — `Query::Offset` is asked
        // with the field's point and the pointer's, because a drag that leaves the widget's
        // rectangle is still a drag inside its value (ADR 0225).
        if self.dragging
            && let Some(typing) = self.typing
        {
            if let Answer::Offset(offset) = self.viewer.query(Query::Offset {
                at: typing.at,
                point,
            }) && offset != typing.caret
            {
                self.typing = Some(Typing {
                    caret: offset,
                    ..typing
                });
                self.redraw();
            }
            return;
        }
        self.dispatch(Command::Pointer {
            at: point,
            action: if self.dragging {
                PointerAction::Dragged
            } else {
                PointerAction::Moved
            },
        });
        // §12.5.6.5's activation region, asked at pointer speed — which is why it is a query
        // rather than a command with an event coming back.
        if let (Answer::Link(over), Some(state)) =
            (self.viewer.query(Query::LinkAt(point)), self.state.as_ref())
        {
            state.window.set_cursor(if over {
                winit::window::CursorIcon::Pointer
            } else {
                winit::window::CursorIcon::Default
            });
        }
    }

    /// What a click inside the panel does.
    pub(crate) fn click_panel(&mut self) {
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        // The outline and the attachments are fields rather than queries for exactly this:
        // `Sidebar::click` produces a command for the viewer, and an `Answer` borrowing it would
        // still be alive. The layers are queried and the answer is *owned*, so the borrow ends
        // before the command goes out.
        let layers = self.layers();
        let hit = self.panel.click(
            at(self.cursor),
            Content {
                outline: &self.outline,
                layers: &layers,
                attachments: &self.attachments,
                articles: &self.articles,
                collection: self.collection.as_ref().map(|(collection, initial)| {
                    viewer_ui::chrome::Presentation {
                        collection,
                        initial,
                    }
                }),
                information: &self.information,
                metadata: self.metadata.as_ref(),
                pages: &self.pages,
            },
            scale,
        );
        drop(layers);
        match hit {
            Some(Hit::Activate(object)) => self.dispatch(Command::Activate(object)),
            Some(Hit::Extract(name)) => self.dispatch(Command::Extract { name }),
            // §8.11.2.2: switching a group re-decides what the page draws, so this goes to the
            // core and comes back as a render rather than as a repaint of the panel.
            Some(Hit::SetGroup { group, on }) => self.dispatch(Command::SetGroup { group, on }),
            // §12.3.4: a click on a page's miniature shows that page. A page index rather than
            // a destination — the thumbnail *is* the page, so there is nothing to resolve.
            Some(Hit::GoTo(page)) => self.dispatch(Command::GoTo(PageTarget::Index(page))),
            Some(Hit::Redraw) => self.redraw(),
            Some(Hit::Nothing) | None => {}
        }
    }

    /// A wheel notch: the About card, the panel's list, or the page — and under Ctrl, a zoom.
    pub(crate) fn wheel(&mut self, delta: winit::event::MouseScrollDelta) {
        // A line is not a pixel and winit reports whichever the device produced. Sixteen logical
        // pixels a line is about one row of this program's own text, which is what a line means
        // on a list; a touchpad reports pixels and needs no conversion.
        let by = match delta {
            winit::event::MouseScrollDelta::LineDelta(_, lines) => -lines * 16.0,
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a scroll delta in pixels, which is tens"
            )]
            winit::event::MouseScrollDelta::PixelDelta(position) => -(position.y as f32),
        };
        if self.about.shown {
            let Some((_, height, scale)) = self.window() else {
                return;
            };
            self.about.scroll(by / scale, crate::NOTICE, height, scale);
            self.redraw();
            return;
        }
        // Ctrl is a magnification of the *page*, and the sidebar has no scale to change — so a
        // notch over the sidebar still zooms the page, with **no anchor**: there is no point of
        // the page under the pointer to hold, and `None` is the core's word for that. A step per
        // notch, and a step per `WHEEL_ZOOM_PIXELS` of a touchpad — the sixteen-pixels-a-line
        // conversion above is a distance on a list and says nothing about a magnification.
        if self.control {
            let whole = match delta {
                winit::event::MouseScrollDelta::LineDelta(_, lines) => {
                    self.pinch = 0.0;
                    lines.trunc()
                }
                winit::event::MouseScrollDelta::PixelDelta(position) => {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a scroll delta in pixels, which is tens"
                    )]
                    let pixels = position.y as f32;
                    self.pinch += pixels;
                    let whole = (self.pinch / WHEEL_ZOOM_PIXELS).trunc();
                    self.pinch -= whole * WHEEL_ZOOM_PIXELS;
                    whole
                }
            };
            // `ZOOM_RANGE` spans 0.02 to 64, which is thirty-six steps of 1.25 end to end, so a
            // bound of sixty-four cannot hide a magnification anybody could have reached — it is
            // there because a `f32` cast saturates and a device reporting nonsense would
            // otherwise be a loop of two billion commands.
            #[expect(
                clippy::cast_possible_truncation,
                reason = "clamped to ±64 on the line above"
            )]
            let steps = whole.clamp(-64.0, 64.0) as i32;
            let zoom = if steps > 0 { Zoom::In } else { Zoom::Out };
            let at = (!self.over_panel()).then(|| self.on_page(self.cursor));
            for _ in 0..steps.unsigned_abs() {
                self.dispatch(Command::Zoom { zoom, at });
            }
            return;
        }
        if self.over_panel() {
            let Some((_, height, scale)) = self.window() else {
                return;
            };
            let layers = self.layers();
            self.panel.scroll(
                by / scale,
                Content {
                    outline: &self.outline,
                    layers: &layers,
                    attachments: &self.attachments,
                    articles: &self.articles,
                    collection: self.collection.as_ref().map(|(collection, initial)| {
                        viewer_ui::chrome::Presentation {
                            collection,
                            initial,
                        }
                    }),
                    information: &self.information,
                    metadata: self.metadata.as_ref(),
                    pages: &self.pages,
                },
                height,
                scale,
            );
            drop(layers);
            self.redraw();
        } else {
            self.dispatch(Command::Scroll { dx: 0.0, dy: by });
        }
    }
}
