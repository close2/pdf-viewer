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
use pdf_render::{DisplayList, Raster, Size, TargetSpec};
use pdf_syntax::Document;

use crate::command::Zoom;
use crate::viewer::RenderToken;
use pdf_model::action::ImportData;
use pdf_model::view::Pointer;
use pdf_syntax::ObjectId;

/// The zoom a document opens at.
///
/// §12.3.2.1's `/OpenAction` may state a magnification and this does not honour it yet, which
/// is a gap named rather than hidden: `Destination::view` carries `/XYZ`'s zoom and `/Fit`'s
/// family, and turning those into a [`Zoom`] is a page-positioning question that belongs with
/// scrolling to a destination rather than with opening a file.
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
        let page_index = pdf_model::destination::Destination::open_action(&document)
            .and_then(|destination| destination.page_index(&document, &pages))
            .filter(|index| *index < page_count)
            .unwrap_or(0);
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
            scroll: (0.0, 0.0),
            current: None,
            interpreted: None,
            revision: 0,
            shown: None,
            frame: None,
            pending: None,
            pointer: None,
            pressed: None,
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
            Zoom::FitPage | Zoom::FitWidth | Zoom::Scale(_) => current,
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
