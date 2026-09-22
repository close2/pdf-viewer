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

    /// §7.6.4.1's card, where the document has asked for a password.
    ///
    /// Beside the notices card rather than in a module of its own for the reason the notices card
    /// is here: both are modal chrome this host draws over the page, and both are one call into
    /// `viewer_ui::chrome`.
    pub(crate) fn password_list(&self, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        self.password.draw(chrome, width, height, scale)
    }

    /// `CLAUDE.md`'s question, where the *ask* level has one outstanding.
    ///
    /// Beside the password card for the reason that one is beside the notices card: all three are
    /// modal chrome this host draws over the page, and each is one call into `viewer_ui::chrome`.
    pub(crate) fn question_list(&self, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        self.question.draw(chrome, width, height, scale)
    }

    /// `CLAUDE.md`'s four levels, where a person has opened the menu.
    pub(crate) fn menu_list(&self, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        self.menu.draw(chrome, width, height, scale)
    }

    /// Why there is no document at all, where there is none.
    ///
    /// Beside the password card for the same reason it is: both are chrome this host draws over a
    /// window with no page in it, and `Surface::without_a_page` is the path that gets either of
    /// them onto the screen (ADR 0545).
    pub(crate) fn refusal_list(&self, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        self.refused.draw(chrome, width, height, scale)
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
            collection: self
                .collection
                .as_ref()
                .map(
                    |(collection, initial, order)| viewer_ui::chrome::Presentation {
                        collection,
                        initial,
                        order,
                        previews: &self.previews,
                    },
                ),
            information: &self.information,
            metadata: self.metadata.as_ref(),
            page_count: self.page_count,
            pages: Some(&self.pages),
        }
    }

    /// Fetches §12.3.4's rows the panel is about to draw, and no others.
    ///
    /// Called from `present`, which is the one place that runs before the panel is drawn and holds
    /// `&mut self`.
    ///
    /// **This used to build every row of the list the first time the tab was shown**, which is
    /// `CLAUDE.md` section 2's forbidden thumbnail generation reached by a different road: a
    /// document stating Table 29's `/PageMode /UseThumbs` opens that tab as it opens, so the whole
    /// list was on the *launch path* — measured at **121 ms of a 156 ms first present** on a
    /// thousand-page document, by the trace line below, which is why the line is there. The two
    /// native hosts never had it, because a `GtkListView` and a `QAbstractListModel` bind the rows
    /// they lay out and this host lays out its own.
    ///
    /// A document with no thumbnails at all still gets rows — the rows are its pages, and
    /// §12.3.4's NOTE is why a page without one is still a page.
    pub(crate) fn fill_visible_pages(&mut self) {
        if !self.panel.shows_pages() {
            return;
        }
        let Answer::Count(count) = self.viewer.query(Query::PageCount) else {
            return;
        };
        self.page_count = count;
        let Some((_, height, scale)) = self.window() else {
            return;
        };
        let wanted = self.panel.visible_pages(count, height, scale);
        let began = std::time::Instant::now();
        let mut fetched = 0_usize;
        for index in wanted.clone() {
            if self.pages.get(index).is_some() {
                continue;
            }
            let entry = viewer_host::page_entry(&self.viewer, index);
            fetched = fetched.saturating_add(1);
            drop(self.pages.row(index, || viewer_host::Held {
                label: entry.label,
                picture: entry.thumbnail,
            }));
        }
        if fetched > 0 {
            self.trace.say(
                crate::trace::Topic::Panel,
                format_args!(
                    "§12.3.4: rows {}..{} of {count} on the screen, {fetched} fetched in {:?}, \
                     {} held",
                    wanted.start,
                    wanted.end,
                    began.elapsed(),
                    self.pages.len()
                ),
            );
        }
    }

    /// §12.3.6's preview pictures for the rows the files panel is about to draw, and Table 158's
    /// `/Direction` `N`.
    ///
    /// Beside `fill_visible_pages` and for its reason: a preview is an embedded document opened
    /// and a page's §12.3.4 `/Thumb` decoded, so the panel fetches the rows it is showing and no
    /// others. Three of Table 160's layouts are made of those pictures and the other four are not,
    /// which is why nothing is fetched for a collection presented as a tree, a details view or a
    /// tile view (ADR 1251).
    ///
    /// The window's own region is given to the panel where Table 158 asks for it — "[t]he entire
    /// window region shall be dedicated to the file navigation view" — and taken back the moment
    /// the document states something else, so nothing survives a second document (ADR 1252).
    pub(crate) fn fill_collection_previews(&mut self) {
        let Some((_, height, scale)) = self.window() else {
            return;
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a window's width in pixels, which is thousands"
        )]
        let logical = |pixels: u32| pixels as f32 / scale.max(0.01);
        let dedicated = self
            .collection
            .as_ref()
            .is_some_and(|(collection, _, _)| viewer_host::panel::whole_window(collection));
        self.panel.dedicate(match (dedicated, self.window()) {
            (true, Some((width, _, _))) => Some(logical(width)),
            _ => None,
        });
        if !self.panel.shows_files() {
            return;
        }
        let wanted: Vec<String> = match self.collection.as_ref() {
            Some((collection, initial, order)) => {
                let rows = viewer_host::panel::collection_rows(
                    collection,
                    initial,
                    order,
                    &self.attachments,
                );
                let mut keys = Vec::new();
                preview_keys(&rows, &mut keys);
                keys
            }
            None => return,
        };
        let shown = self.panel.visible_previews(wanted.len(), height, scale);
        // Disjoint field borrows: the picture is fetched through the viewer while the cache is
        // being filled, and only naming the two fields apart makes that one statement.
        let viewer = &self.viewer;
        let previews = &mut self.previews;
        for name in wanted.get(shown).unwrap_or_default() {
            let _ = previews.picture(name, || {
                viewer_host::panel::attachment_preview(viewer, name)
            });
        }
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
                collection: self
                    .collection
                    .as_ref()
                    .map(
                        |(collection, initial, order)| viewer_ui::chrome::Presentation {
                            collection,
                            initial,
                            order,
                            previews: &self.previews,
                        },
                    ),
                information: &self.information,
                metadata: self.metadata.as_ref(),
                page_count: self.page_count,
                pages: Some(&self.pages),
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
                collection: self
                    .collection
                    .as_ref()
                    .map(
                        |(collection, initial, order)| viewer_ui::chrome::Presentation {
                            collection,
                            initial,
                            order,
                            previews: &self.previews,
                        },
                    ),
                information: &self.information,
                metadata: self.metadata.as_ref(),
                page_count: self.page_count,
                pages: Some(&self.pages),
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
        // the page under the pointer to hold, and `None` is the core's word for that. How far a
        // device has to travel for one step is `zoom_steps`'s; the sixteen-pixels-a-line
        // conversion above is a distance on a list and says nothing about a magnification.
        if self.control {
            let steps = zoom_steps(&mut self.zoom_carry, delta);
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
                    collection: self
                        .collection
                        .as_ref()
                        .map(
                            |(collection, initial, order)| viewer_ui::chrome::Presentation {
                                collection,
                                initial,
                                order,
                                previews: &self.previews,
                            },
                        ),
                    information: &self.information,
                    metadata: self.metadata.as_ref(),
                    page_count: self.page_count,
                    pages: Some(&self.pages),
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

/// How many whole zoom steps a Ctrl + wheel delta is worth, given what earlier deltas left
/// unspent in `carry`.
///
/// **A `LineDelta` is not a notch.** `winit`'s X11 backend divides an `XInput2` smooth-scroll
/// valuator by that axis's increment, so a high-resolution wheel or a touchpad in line mode
/// reports a *fraction* of a line per event. Truncating each event on its own therefore spent
/// nothing at all: in the trace of 2026-09-15 the device's quantum was about a thirty-seventh of
/// a line, and 579 Ctrl + wheel events carrying 146.4 lines of travel over seven gestures produced
/// **one** zoom step. So the fraction is carried and spent when it completes a step — which is
/// what the pixel arm has always done, and `carry` is in lines for both because a step is a step
/// however the device measured it (ADR 1118).
///
/// `WHEEL_ZOOM_PIXELS` is what converts the one to the other, so a touchpad's fifty pixels stay
/// one step exactly as before.
fn zoom_steps(carry: &mut f32, delta: winit::event::MouseScrollDelta) -> i32 {
    let lines = match delta {
        winit::event::MouseScrollDelta::LineDelta(_, lines) => lines,
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a scroll delta in pixels, which is tens"
        )]
        winit::event::MouseScrollDelta::PixelDelta(position) => {
            position.y as f32 / WHEEL_ZOOM_PIXELS
        }
    };
    // An accumulator is poisoned permanently by one bad value, which a per-event truncation could
    // not be, so a device reporting a NaN or an infinity is ignored here rather than added in.
    if !lines.is_finite() {
        return 0;
    }
    *carry += lines;
    let whole = carry.trunc();
    // `ZOOM_RANGE` spans 0.02 to 64, which is thirty-six steps of 1.25 end to end, so a bound of
    // sixty-four cannot hide a magnification anybody could have reached — it is there because a
    // `f32` cast saturates and a device reporting nonsense would otherwise be a loop of two
    // billion commands.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to ±64 on the line above"
    )]
    let steps = whole.clamp(-64.0, 64.0) as i32;
    // The whole of what `whole` claimed leaves the carry, clamped or not: a nonsense delta is
    // refused a magnification, not banked for the next event to spend.
    *carry -= whole;
    steps
}

/// The `/EmbeddedFiles` keys of the rows carrying one of §12.3.6's pictures of an attachment.
///
/// In the order the panel draws them, which is what `Sidebar::visible_previews` indexes into:
/// `viewer_host::panel::Picture` says which rows have one, and `RowAction::Extract` carries the
/// key `Query::AttachmentPreview` names a file by.
fn preview_keys(rows: &[viewer_host::PanelRow], out: &mut Vec<String>) {
    for row in rows {
        if row
            .picture
            .is_some_and(viewer_host::panel::Picture::wants_the_files_own_picture)
            && let viewer_host::RowAction::Extract { name } = &row.action
        {
            out.push(name.clone());
        }
        preview_keys(&row.children, out);
    }
}

#[cfg(test)]
mod tests {
    use winit::dpi::PhysicalPosition;
    use winit::event::MouseScrollDelta;

    use super::{WHEEL_ZOOM_PIXELS, zoom_steps};

    /// The quantum the mouse in the trace of 2026-09-15 reported, in lines. Every Ctrl + wheel
    /// delta in that file is a multiple of it.
    const TRACE_QUANTUM: f32 = 0.026_981_818;

    /// The measurement, before and after, on the trace's own device.
    ///
    /// Four lines of travel arrive as 149 events of a thirty-seventh of a line each. The old
    /// arithmetic truncated each event alone; this replays that here so the two numbers stand
    /// beside each other in the tree rather than in a record.
    #[test]
    fn a_high_resolution_wheel_spends_every_line_it_travels() {
        let events = 149;
        let truncated_per_event: i32 = (0..events)
            .map(|_| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the old arithmetic, reproduced: a fraction truncates to zero"
                )]
                let whole = TRACE_QUANTUM.trunc() as i32;
                whole
            })
            .sum();
        assert_eq!(truncated_per_event, 0, "before: 4.0 lines bought nothing");

        let mut carry = 0.0;
        let carried: i32 = (0..events)
            .map(|_| zoom_steps(&mut carry, MouseScrollDelta::LineDelta(0.0, TRACE_QUANTUM)))
            .sum();
        assert_eq!(carried, 4, "after: four lines of travel, four steps");
    }

    /// A whole notch is still one step on the event that carries it — a classic wheel is not
    /// made to wait for a second notch.
    #[test]
    fn a_whole_notch_is_still_one_step() {
        let mut carry = 0.0;
        assert_eq!(
            zoom_steps(&mut carry, MouseScrollDelta::LineDelta(0.0, 1.0)),
            1
        );
        assert_eq!(
            zoom_steps(&mut carry, MouseScrollDelta::LineDelta(0.0, -3.0)),
            -3
        );
    }

    /// `WHEEL_ZOOM_PIXELS` of touchpad travel is one step, as it was before the accumulator
    /// changed units.
    #[test]
    fn a_touchpad_still_takes_fifty_pixels_a_step() {
        let mut carry = 0.0;
        let tenth = f64::from(WHEEL_ZOOM_PIXELS) / 10.0;
        let steps: i32 = (0..25)
            .map(|_| {
                zoom_steps(
                    &mut carry,
                    MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, tenth)),
                )
            })
            .sum();
        assert_eq!(steps, 2, "two and a half notches of travel is two steps");
    }

    /// Reversing cancels rather than banking, and a device reporting nonsense neither runs away
    /// nor poisons the carry for the rest of the session.
    #[test]
    fn a_reversal_cancels_and_nonsense_does_not_poison_the_carry() {
        let mut carry = 0.0;
        assert_eq!(
            zoom_steps(&mut carry, MouseScrollDelta::LineDelta(0.0, 0.9)),
            0
        );
        assert_eq!(
            zoom_steps(&mut carry, MouseScrollDelta::LineDelta(0.0, -0.9)),
            0
        );
        assert_eq!(
            zoom_steps(&mut carry, MouseScrollDelta::LineDelta(0.0, 1.0)),
            1
        );

        assert_eq!(
            zoom_steps(&mut carry, MouseScrollDelta::LineDelta(0.0, 1e9)),
            64
        );
        assert_eq!(
            zoom_steps(&mut carry, MouseScrollDelta::LineDelta(0.0, f32::NAN)),
            0
        );
        assert_eq!(
            zoom_steps(&mut carry, MouseScrollDelta::LineDelta(0.0, f32::INFINITY)),
            0
        );
        assert_eq!(
            zoom_steps(&mut carry, MouseScrollDelta::LineDelta(0.0, 1.0)),
            1,
            "the carry survived the nonsense"
        );
    }
}
