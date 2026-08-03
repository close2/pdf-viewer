//! One open document, and everything the viewer derives from it.
//!
//! Split from [`crate::Viewer`] because the viewer's own job is the *set* of them and the one
//! viewport they share, while everything here is per document: which page is showing, how large
//! it is drawn, and the pixels that showed it last.

use std::sync::Arc;

use pdf_model::content::{Interpretation, Placed};
use pdf_model::outline::Outline;
use pdf_model::page_label::PageLabels;
use pdf_model::view::ViewState;
use pdf_model::{Page, Pages};
use pdf_render::{DisplayList, Raster, Rect, Size, TargetSpec};
use pdf_syntax::Document;

use crate::command::Zoom;
use crate::viewer::{RenderToken, px};
use pdf_model::action::ImportData;
use pdf_model::view::Pointer;
use pdf_syntax::ObjectId;

/// The zoom a document opens at, where its `/OpenAction` states none.
///
/// Where one does, [`Self::pending_view`] carries it and `apply_view` replaces this on the first
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
    pub(crate) page_index: usize,
    /// How large the page is drawn.
    pub(crate) zoom: Zoom,
    /// §12.3.2.1's other two items, waiting for a viewport and a display list to be applied to.
    ///
    /// A destination states a page, a location and a magnification; the page is a property of the
    /// document and is applied at once, while the other two need to know how large the window is
    /// and — for `/FitB`, `/FitBH` and `/FitBV` — what the page's contents cover. Both are known
    /// in `Viewer::settle` and nowhere earlier, so the view waits here until then. ADR 0162.
    pub(crate) pending_view: Option<pdf_model::destination::View>,
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
    /// The page being shown, kept rather than looked up again.
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
    pub(crate) current: Option<(usize, Page)>,
    /// How long the page showing has been shown, in seconds — §12.4.4.1's `/Dur` clock.
    ///
    /// Accumulated from [`crate::Command::Tick`] and reset by every page change, because the
    /// clause makes the duration a property of *this* page rather than of the presentation.
    pub(crate) shown_for: f32,
    /// The page that was interpreted last, and its drawing commands.
    ///
    /// One page rather than a cache of them. A display list is the expensive artefact and
    /// keeping several would make page-turning free, but it would also need a bound and an
    /// eviction rule, and both should be written after somebody measures what a display list
    /// costs to hold rather than before. What this *does* buy is the case it was kept for: a
    /// zoom or a scroll re-rasterises without re-interpreting.
    pub(crate) interpreted: Option<Interpreted>,
    /// The page, resolution and revision the host last drew, whichever tier it is.
    ///
    /// Separate from [`Self::frame`] because a tier-2 host hands back no pixels: it draws onto
    /// its own surface and says so, and without this the scheduler would see nothing on the
    /// screen and ask for the same frame again, for ever.
    pub(crate) shown: Option<(usize, TargetSpec, u64)>,
    /// The pixels a tier-1 host handed back, and what they are of.
    pub(crate) frame: Option<Frame>,
    /// The render that is outstanding, if one is.
    pub(crate) pending: Option<Pending>,
    /// Which annotation the pointer is interacting with, and how (§12.5.5).
    ///
    /// Kept beside [`Self::view`] rather than read back out of it because the question asked
    /// here is "has this changed", and the answer decides whether the page has to be
    /// interpreted again. Set only for an annotation that *has* the appearance in question:
    /// re-interpreting a page because the cursor crossed a link with one appearance stream
    /// would put 2 000 M instructions on a mouse move.
    pub(crate) pointer: Option<(ObjectId, Pointer)>,
    /// The annotation a press went down on, if the button is still down.
    pub(crate) pressed: Option<ObjectId>,
    /// Which annotation the pointer is inside, for §12.6.3's `/E` and `/X`.
    ///
    /// Separate from `pointer` above, which is §12.5.5's *appearance* state and is filtered to
    /// annotations whose picture can change. Table 197's enter and exit are about the cursor
    /// crossing an activation region and say nothing about appearances, so an annotation with
    /// one `/AP` and an `/AA /E` still raises the event.
    pub(crate) inside: Option<ObjectId>,
    /// §12.7.6.4's import, waiting for the host to supply the file.
    pub(crate) importing: Option<ImportData>,
    /// Everything a person has changed, in the order they changed it.
    ///
    /// The log `CLAUDE.md`'s rule 1 asks for: the document is immutable, so an edit is an entry
    /// here and never a change to the file. Undo and redo *are* this log — which is why they
    /// belong in the core rather than being reimplemented by every host — and §7.5.6's
    /// incremental update is a pure function of it.
    pub(crate) log: Vec<crate::command::Edit>,
    /// How many entries of [`Self::log`] are in effect.
    ///
    /// Everything before the cursor has been applied; everything after it has been undone and
    /// may be redone. A new edit truncates the tail, which is what a single log with a cursor
    /// means and what every editor does.
    pub(crate) cursor: usize,
    /// What is selected, as byte offsets into the interpreted page's readback.
    ///
    /// Anchor first, then where the pointer is now — in that order rather than sorted, because
    /// a selection dragged backwards is a selection, and which end moves is the difference
    /// between extending it and starting again.
    pub(crate) selection: Option<(usize, usize)>,
}

/// A page, interpreted.
#[derive(Debug)]
pub(crate) struct Interpreted {
    /// Which page, zero-based.
    pub(crate) page: usize,
    /// Its drawing commands, resolution-independent and shared with whatever is drawing them.
    pub(crate) list: Arc<DisplayList>,
    /// What could not be drawn, already worded.
    pub(crate) reports: Vec<String>,
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

/// The pixels showing now.
#[derive(Debug)]
pub(crate) struct Frame {
    /// Which page they are of.
    pub(crate) page: usize,
    /// What they were drawn into, which is what makes them stale when the zoom changes.
    pub(crate) target: TargetSpec,
    /// Row-major RGBA, no padding.
    pub(crate) raster: Raster,
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
        password: Option<&str>,
    ) -> Result<Self, pdf_syntax::SyntaxError> {
        Ok(Self::around(Document::open_with_password(
            bytes,
            pdf_syntax::Limits::DEFAULT,
            password.unwrap_or_default(),
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
        Self {
            document,
            view,
            labels,
            outline,
            page_count,
            page_index,
            zoom: INITIAL_ZOOM,
            pending_view: open_view,
            scroll: (0.0, 0.0),
            current: None,
            shown_for: 0.0,
            interpreted: None,
            revision: 0,
            shown: None,
            frame: None,
            pending: None,
            pointer: None,
            pressed: None,
            inside: None,
            importing: None,
            log: Vec::new(),
            cursor: 0,
            selection: None,
        }
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
        for edit in log.iter().take(self.cursor) {
            match edit {
                crate::command::Edit::SetField { field, value } => {
                    self.view.set_field(&self.document, field, value.as_deref());
                }
            }
        }
        self.log = log;
        // The page's ink depends on the values, so the display list is stale.
        self.interpreted = None;
    }

    /// Whether anything a person did is unsaved.
    pub(crate) fn dirty(&self) -> bool {
        self.cursor > 0
    }

    /// The text position a point in the display list's coordinates selects.
    pub(crate) fn position_at(&self, point: (f32, f32)) -> Option<usize> {
        let interpreted = self.interpreted.as_ref()?;
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
        if let Some(interpreted) = self.interpreted.as_ref()
            && interpreted.page == index
        {
            return Some(interpreted.list.page_size);
        }
        if let Some((cached, page)) = self.current.as_ref()
            && *cached == index
        {
            return Some(pdf_model::content::displayed_size(page));
        }
        self.page(index)
            .map(|page| pdf_model::content::displayed_size(&page))
    }

    /// The page being shown, without walking the tree for it again.
    pub(crate) fn shown_page(&self) -> Option<&Page> {
        match &self.current {
            Some((index, page)) if *index == self.page_index => Some(page),
            _ => None,
        }
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

    /// Where the raster's top-left corner sits in the viewport, in device pixels.
    ///
    /// Centred where the page is smaller than the viewport and scrolled where it is larger,
    /// which is one expression because the clamp on the scroll makes the second case the only
    /// one that can be non-zero.
    pub(crate) fn origin(&self, viewport: (u32, u32), raster: (u32, u32)) -> (f32, f32) {
        let centre = |viewport: u32, raster: u32, scroll: f32| {
            let slack = f64::from(viewport) - f64::from(raster);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a difference of two pixel counts, both bounded by MAX_EXTENT"
            )]
            let slack = slack as f32;
            if slack > 0.0 { slack / 2.0 } else { -scroll }
        };
        (
            centre(viewport.0, raster.0, self.scroll.0),
            centre(viewport.1, raster.1, self.scroll.1),
        )
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
        before: f32,
        after: f32,
        at: Option<(f32, f32)>,
    ) {
        let Some(size) = self.page_size(self.page_index) else {
            return;
        };
        let origin = self.origin(viewport, raster_extent(size, before));
        let at = at.unwrap_or((px(viewport.0) / 2.0, px(viewport.1) / 2.0));
        let ratio = after / before;
        let hold = |at: f32, origin: f32| ((at - origin) * ratio - at).max(0.0);
        self.scroll = (hold(at.0, origin.0), hold(at.1, origin.1));
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
    /// contents", which no page dictionary states and only a display list can answer.
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
        let content = bounds.filter(|box_| box_.max.x > box_.min.x && box_.max.y > box_.min.y);
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
        self.clamp_scroll(viewport, raster_extent(size, magnification));
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

    /// Holds the scroll inside the page, which is what stops a page being scrolled out of view.
    pub(crate) fn clamp_scroll(&mut self, viewport: (u32, u32), raster: (u32, u32)) {
        let limit = |viewport: u32, raster: u32| {
            let slack = f64::from(raster) - f64::from(viewport);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a difference of two pixel counts, both bounded by MAX_EXTENT"
            )]
            let slack = slack as f32;
            slack.max(0.0)
        };
        self.scroll.0 = self.scroll.0.clamp(0.0, limit(viewport.0, raster.0));
        self.scroll.1 = self.scroll.1.clamp(0.0, limit(viewport.1, raster.1));
    }
}

/// The raster a page of this size occupies at this magnification.
///
/// The same rounding `TargetSpec::for_page` does — up, so the raster contains the page — stated
/// here because `apply_view` clamps a scroll before there is a target to ask.
fn raster_extent(size: Size, magnification: f32) -> (u32, u32) {
    let extent = |value: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a page extent times a magnification, both finite and bounded by the \
                      pixel budget the target spec applies immediately afterwards"
        )]
        let pixels = (value * magnification).ceil().max(1.0) as u32;
        pixels
    };
    (extent(size.width), extent(size.height))
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
    let interpretation = pdf_model::content::interpret_with(&open.document, &page, &open.view);
    let reports = interpretation
        .unsupported
        .iter()
        .map(crate::report::describe)
        .collect();
    Some((interpretation, reports, page))
}
