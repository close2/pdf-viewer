//! Where things are on the screen: page space to device space and back, for marks, quads, fields
//! and the focus ring.

use pdf_model::measurement;
use pdf_render::{DisplayList, Rect, Transform};
use pdf_syntax::ObjectId;

use super::{DocumentId, Viewer};
#[cfg(doc)]
use crate::Answer;
use crate::open::Open;
use crate::query::{PageGeometry, PopupWindow, Selected};

impl Viewer {
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
    pub(super) fn selected<'a>(&self, open: &'a Open) -> Option<Selected<'a>> {
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
    /// host: the origin, the magnification and the y flip are exactly the arithmetic that is easy
    /// to get wrong (ADR 0118), and a ring drawn from a host's own guess at it would be that
    /// defect again one crate over.
    pub(super) fn focus_quad(&self, open: &Open) -> Option<(ObjectId, [f32; 8])> {
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
    pub(super) fn highlight_quads(&self, open: &Open) -> Vec<[f32; 8]> {
        open.highlights
            .iter()
            .filter(|highlighted| open.on(highlighted.page).is_some())
            .filter_map(|highlighted| self.device_quad(open, highlighted.page, highlighted.rect))
            .collect()
    }

    /// §12.5.6.14's open popup windows, placed on the screen.
    ///
    /// The state is the file's `/Open` unless a person or a script has said otherwise since,
    /// the later of the two, which `pdf_model::popup::popups` lays over it from the view state
    /// (ADR 1720) and is why Table 186's word "initially" is load-bearing.
    pub(super) fn popup_windows(&self, open: &Open) -> Vec<PopupWindow> {
        // Every page the arrangement shows rather than the current one alone: a window belongs to
        // the page its annotation is on, and a `OneColumn` reader looking at two pages would
        // otherwise see the notes of one of them.
        open.on_screen
            .iter()
            .flat_map(|on_screen| {
                pdf_model::popup::popups(&open.document, &on_screen.object, &open.view)
                    .into_iter()
                    .filter(|popup| popup.open)
                    .filter_map(|popup| {
                        Some(PopupWindow {
                            note: pdf_model::popup::retyped_by(&open.document, &popup),
                            annotation: popup.annotation,
                            parent: popup.parent,
                            quad: self.device_quad(open, on_screen.page, popup.rect)?,
                            title: popup.title,
                            text: popup.text,
                            modified: popup.modified,
                            colour: popup.colour,
                            subject: popup.subject,
                            created: popup.created,
                            rich: popup.rich,
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
    pub(super) fn form_fields(&self, open: &Open) -> Vec<crate::FormField> {
        let mut fields = self.placed_fields(open);
        displayed(open, &mut fields);
        fields
    }

    /// [`Self::form_fields`] before what each field displays is asked.
    fn placed_fields(&self, open: &Open) -> Vec<crate::FormField> {
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
                        // The characters until `displayed` below has asked which of them a
                        // format displays otherwise.
                        let displayed = field.value.as_ref().map(|shown| shown.text.clone());
                        Some(crate::FormField {
                            name: field.name,
                            partial: field.partial,
                            control: field.control,
                            value: field.value,
                            displayed,
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
    /// magnification and the y flip are exactly what is easy to get wrong (ADR 0118), and a second
    /// opinion about them in a host — or in a second method here — would be that defect again.
    /// `[x0, y0, x1, y1]`, normalised, in; clockwise from the top-left as it
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
    /// **The page transform is applied here**, or `Query::Focus`'s ring and `Query::Popups`'
    /// windows would be placed as though every page were unrotated with its crop box at the
    /// origin. Both are true of every corpus document that has a widget — none of the 974 states
    /// a rotated page with one — which is why no gate would see it and why this is arithmetic
    /// rather than a picture (ADR 0211).
    pub(super) fn device_point(
        &self,
        open: &Open,
        page: usize,
        at: (f32, f32),
    ) -> Option<(f32, f32)> {
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
    pub(super) fn caret(
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
    pub(super) fn offset(&self, open: &Open, at: (f32, f32), point: (f32, f32)) -> Option<usize> {
        let (page, (x, y)) = self.user_space(open, at)?;
        // The moving point is measured against the *field's* page: a drag that has left the
        // widget is still a drag inside its value, which is what the second point is for.
        let (_, point) = self.user_space(open, point)?;
        let object = open.placed_page(page)?;
        open.view.offset_at(&open.document, object, (x, y), point)
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
    pub(super) fn device_marks(
        &self,
        open: &Open,
        page: usize,
        rect: [f32; 4],
    ) -> Option<[f32; 4]> {
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
    /// edge crosses as a degenerate one: §7.9.5's own NOTE is that "\[r]ectangles can have a width
    /// of zero or height of zero".
    pub(super) fn device_rect(&self, open: &Open, page: usize, rect: [f32; 4]) -> Option<[f32; 4]> {
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
    pub(super) fn device_quads(
        &self,
        open: &Open,
        page: usize,
        range: (usize, usize),
    ) -> Vec<[f32; 8]> {
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
    ///
    /// A §14.9.4 `/ActualText` section is one character however many codes it showed: its text is
    /// the replacement, once, and its box covers every glyph the replacement stands for (ADR 2086).
    pub(super) fn device_lines(
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
                let mut spoken: Option<std::ops::Range<usize>> = None;
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
                    // §14.9.4: "[t]he ActualText value shall be used as a replacement, not a
                    // description, for the content", and every code inside the section reads back
                    // as the whole replacement (ADR 1515) — so the codes after the first are the
                    // same span again. The replacement is spoken once, and its character's box
                    // grows to cover each glyph it replaced (ADR 2086).
                    if spoken.as_ref() == Some(&span)
                        && let Some(previous) = characters.last_mut()
                    {
                        let more = device_box(quad, origin, height, magnification);
                        previous.bounds = [
                            previous.bounds[0].min(more[0]),
                            previous.bounds[1].min(more[1]),
                            previous.bounds[2].max(more[2]),
                            previous.bounds[3].max(more[3]),
                        ];
                        continue;
                    }
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
                    spoken = Some(span);
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
    pub(super) fn page_point(&self, open: &Open, at: (f32, f32)) -> Option<(usize, (f32, f32))> {
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
    pub(super) fn user_space(&self, open: &Open, at: (f32, f32)) -> Option<(usize, (f32, f32))> {
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
    pub(super) fn measured(
        &self,
        open: &Open,
        points: &[[f32; 2]],
    ) -> Option<(measurement::Traced, Option<crate::Located>)> {
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
        let viewports = measurement::Viewports::read(&open.document, &object.dict);
        let traced = viewports.traced(&user)?;
        let located = user
            .last()
            .and_then(|[x, y]| crate::located::locate(&viewports, (*x, *y)));
        Some((traced, located))
    }

    /// Where a page sits on the screen, for the page that is on it.
    pub(super) fn geometry(&self, open: &Open, index: usize) -> Option<PageGeometry> {
        let on_screen = open.on(index)?;
        // Answered only once the page has been interpreted, as it always was: a geometry taken
        // from an extent nothing has drawn yet is a promise about a frame that may not arrive.
        on_screen.interpreted.as_ref()?;
        self.placed_geometry(open, index)
    }

    /// [`Self::geometry`], and for the page a host is reading on a thread of its own as well
    /// ([`Self::read_out`]): that page's frame is asked for the moment its read is taken in, or
    /// read here if the read is dropped, so the promise is kept, and a window shows what it
    /// retains of the page where the page will be until the frame lands (ADR 2344).
    pub(super) fn shown_geometry(
        &self,
        open: &Open,
        id: DocumentId,
        index: usize,
    ) -> Option<PageGeometry> {
        if self
            .out
            .as_ref()
            .is_some_and(|out| out.document == id && out.pages.contains(&index))
        {
            return self.placed_geometry(open, index);
        }
        self.geometry(open, index)
    }

    /// Where the arrangement places an arranged page, interpreted or not.
    fn placed_geometry(&self, open: &Open, index: usize) -> Option<PageGeometry> {
        let on_screen = open.on(index)?;
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
pub(super) fn content_bounds(list: &DisplayList) -> Option<Rect> {
    let mut union: Option<Rect> = None;
    for command in list.commands() {
        let bounds = pdf_render::marked_bounds(command, Transform::IDENTITY)?;
        union = Some(union.map_or(bounds, |box_| box_.union(bounds)));
    }
    union
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
pub(super) fn page_object(pages: &pdf_model::Pages, index: usize) -> Option<ObjectId> {
    pages.get(index).and_then(|page| page.id)
}

/// Every annotation object on a page, in `/Annots` order.
///
/// By object identity, because a trigger performs what the *dictionary* states and a direct
/// annotation — one the array holds inline rather than by reference — has no identity to raise
/// an event against. §12.5.2 makes `/Annots` "an array of annotation dictionaries", and every
/// corpus document writes them indirectly.
pub(super) fn annotations_on(open: &Open, pages: &pdf_model::Pages, index: usize) -> Vec<ObjectId> {
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

/// What each field displays when nobody is editing it: `pdf_model`'s answer, Table 199's `/F`
/// applied, written over the characters [`Viewer::placed_fields`] put there (ADR 1604).
///
/// **Two costs measured and both taken out**, in `examples/fields_cost.rs`. Asked one field at a
/// time, `ViewState::displayed_value` walks §12.7.4.1's field tree once per field, which on every
/// repaint of a toolkit window took `prefilled_f1040.pdf`'s 116 fields from 0.6–0.9 ms to 24.5 ms,
/// none of them formatted; so [`states_a_format`] asks first whether a field states a format at
/// all — one without displays its characters, which is that method's own answer — and the fields
/// that do are asked together, through `ViewState::displayed_values`, which walks the tree once.
/// With both, those 116 fields answer in 0.80 ms against 0.65 without the column, and a page of 120
/// fields each stating `AFNumber_Format` in 0.72 ms against 0.35. A password field
/// displays its echo, which is the characters this answer already carries.
pub(super) fn displayed(open: &Open, fields: &mut [crate::FormField]) {
    let formatted: Vec<usize> = fields
        .iter()
        .enumerate()
        .filter(|(_, field)| {
            field.value.as_ref().is_some_and(|shown| !shown.obscured)
                && states_a_format(&open.document, &field.widgets)
        })
        .map(|(index, _)| index)
        .collect();
    if formatted.is_empty() {
        return;
    }
    let answers = open.view.displayed_values(
        &open.document,
        formatted
            .iter()
            .map(|&index| fields[index].name.qualified.as_str()),
    );
    for (index, answer) in formatted.into_iter().zip(answers) {
        if let Some(answer) = answer {
            fields[index].displayed = Some(answer);
        }
    }
}

/// Each field's value as an accessibility node carries it: what the field displays — Table 199's
/// `/F` applied, `$12.50` where it holds `12.5` — and its characters while it is being typed into,
/// which `ViewState::displayed_values` answers as typed (ADR 1617).
///
/// **The displayed value, because it is the value the node's glyphs are of.** The node's
/// `value_lines` are where §12.7.4.3's layout placed each character of the appearance the page
/// draws, and an appearance is drawn through the format; a value of `12.5` beside the positions of
/// `$12.50` would tell a screen reader where characters are that the text does not hold. A
/// password field keeps its echo, which [`displayed`]'s own rule is too. Asked only of fields that
/// state a format, for [`displayed`]'s cost.
pub(super) fn as_displayed(open: &Open, fields: &mut [pdf_model::form::FormField]) {
    let formatted: Vec<usize> = fields
        .iter()
        .enumerate()
        .filter(|(_, field)| {
            field.value.as_ref().is_some_and(|shown| !shown.obscured)
                && field.widgets.first().is_some_and(|widget| {
                    let object = open.document.get(widget.annotation);
                    object.as_dict().is_some_and(|dictionary| {
                        pdf_model::aform::site::of_widget(
                            &open.document,
                            dictionary,
                            pdf_model::aform::Trigger::Format,
                        ) != pdf_model::aform::site::Site::Absent
                    })
                })
        })
        .map(|(index, _)| index)
        .collect();
    if formatted.is_empty() {
        return;
    }
    let answers = open.view.displayed_values(
        &open.document,
        formatted
            .iter()
            .map(|&index| fields[index].name.qualified.as_str()),
    );
    for (index, answer) in formatted.into_iter().zip(answers) {
        if let (Some(answer), Some(shown)) = (answer, fields[index].value.as_mut()) {
            shown.text = answer;
        }
    }
}

/// Whether a field states Table 199's `/F` at all, read from its first widget up its `/Parent`
/// chain as `pdf_model::aform::site::of_widget` reads it — the cheap question [`displayed`] asks
/// before the dear one.
fn states_a_format(document: &pdf_syntax::Document, widgets: &[crate::FormWidget]) -> bool {
    let Some(first) = widgets.first() else {
        return false;
    };
    let object = document.get(first.annotation);
    object.as_dict().is_some_and(|widget| {
        pdf_model::aform::site::of_widget(document, widget, pdf_model::aform::Trigger::Format)
            != pdf_model::aform::site::Site::Absent
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
