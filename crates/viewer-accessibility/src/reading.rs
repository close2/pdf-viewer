//! What a host publishes to an assistive technology, gathered once for all three of them.
//!
//! # Why this is here rather than in each host
//!
//! Publishing §14.7's tree is six questions asked of [`viewer_core::Viewer`] and one decision
//! about *when* to ask them, and none of it is a toolkit's. `viewer-ui` held the whole of it as
//! a hundred and fifty lines of `access.rs` while the two native hosts published nothing at all;
//! the moment they do, that becomes three copies of a decision — and `viewer_host`'s own founding
//! sentence is that the third copy of a function is where two hosts stop agreeing (ADR 0246).
//!
//! It is in **this** crate rather than in `viewer-host` because of what it would drag: nothing may
//! depend on `viewer-accessibility` that does not want `accesskit_unix` and its executor, and
//! `viewer-ffi` — which draws nothing and cross-compiles to Windows and macOS — depends on
//! `viewer-host`. So the crate that already owns the adapter owns the gathering too, and the two
//! native hosts take it directly. ADR 0623.
//!
//! # What is a host's, and it is exactly three things
//!
//! What the window is called, how large the viewport is, and what the document is called. Every
//! other fact in a [`Reading`] is an answer `viewer_core` gives, which is what makes the tree three
//! windows publish the same tree.

use viewer_core::{Answer, Query, Viewer};

/// The three facts that decide whether the tree already published is still the right one.
///
/// **A page turn and a resize are the only two things that change the answer**, which is what makes
/// asking the six questions below a page-turn cost rather than a frame cost. A scroll does not: the
/// structure of a page is the same structure wherever it sits, and the quadrilaterals move with it
/// — which is why [`Reading`] is republished on a scroll too, and why *this* comparison is not what
/// decides that. It decides whether the expensive half is asked for.
///
/// The page is the *current* one rather than the list of pages on the screen, and that is
/// deliberate: under Table 29's continuous arrangements the set of pages showing changes exactly
/// when the current page does, because `viewer_core::layout` derives one from the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Showing {
    /// The page a person is on, zero-based.
    pub page: usize,
    /// The viewport's width in device pixels.
    pub width: u32,
    /// Its height.
    pub height: u32,
}

impl Showing {
    /// What the viewer is showing now, for a host to compare against what it last published.
    ///
    /// `None` where there is no document open, which is a window with nothing to say rather than a
    /// document with nothing in it — the second is a [`Reading`] with pages in it and no elements.
    #[must_use]
    pub fn of(viewer: &Viewer, width: u32, height: u32) -> Option<Self> {
        match viewer.query(Query::CurrentPage) {
            Answer::Page { index, .. } => Some(Self {
                page: index,
                width,
                height,
            }),
            _ => None,
        }
    }
}

/// One page of a [`Reading`], owning what the borrowed [`crate::PageView`] points at.
///
/// Owned rather than borrowed because the answers come from six separate queries and each borrows
/// the viewer: a host cannot hold `Query::Reports`' slice while asking `Query::Readback`, and the
/// bridge it publishes through is a field of the same struct the viewer is. Copying one screen's
/// worth of notes is measured in tens of bytes; `doc/todo/31` has what the *tree* costs, which is
/// the part that is not a copy.
#[derive(Debug, Clone)]
struct PageReading {
    /// Which page, zero-based.
    page: usize,
    /// §12.4.2's label, where the document states one.
    label: Option<String>,
    /// Where the page sits in the viewport, `[x0, y0, x1, y1]` in device pixels.
    bounds: [f32; 4],
    /// §14.7's elements for it, in §14.8.2.5's order.
    nodes: Vec<viewer_core::AccessibilityNode>,
    /// What the page could not draw.
    reports: Vec<String>,
    /// §9.10.2's count of codes nothing could name.
    readback: pdf_model::content::Shortfall,
}

/// Everything a host publishes for one screen, gathered from the viewer in one place.
///
/// Built by [`Reading::of`] and handed to [`crate::Bridge::speak`]. It owns its strings so that it
/// can outlive the borrows the six queries take, which is what lets a host with the viewer and the
/// bridge in one struct do both in one function.
#[derive(Debug, Clone)]
pub struct Reading {
    /// What the window's title bar says.
    window: String,
    /// What the document is called.
    document: String,
    /// How many pages it has.
    pages: usize,
    /// The viewport in device pixels, which is the space every quadrilateral is in.
    viewport: (f32, f32),
    /// The pages Table 29's arrangement is showing, in page order.
    shown: Vec<PageReading>,
}

impl Reading {
    /// Asks the viewer everything an assistive technology is owed about this screen.
    ///
    /// Six questions, and the *population* is the structure answer: it carries one entry per page
    /// the arrangement is showing, so a page that reported nothing and read back perfectly still
    /// gets a node saying it is there. The other five are looked up against it.
    ///
    /// **Not on the launch path.** A host calls this after its first frame is on the screen and
    /// then only when [`Showing`] changes — `CLAUDE.md`'s startup section forbids building this on
    /// the way to page one, and `doc/todo/31` has what it costs on a thousand-page document.
    #[must_use]
    pub fn of(viewer: &Viewer, window: &str, document: &str, viewport: (f32, f32)) -> Self {
        let pages = match viewer.query(Query::CurrentPage) {
            Answer::Page { of, .. } => of,
            _ => 0,
        };
        let structures = match viewer.query(Query::AccessibilityTree) {
            Answer::Accessibility(pages) => pages,
            _ => Vec::new(),
        };
        let reports: Vec<(usize, Vec<String>)> = match viewer.query(Query::Reports) {
            Answer::Reports(pages) => pages
                .iter()
                .map(|page| (page.page, page.notes.to_vec()))
                .collect(),
            _ => Vec::new(),
        };
        // The other half of what a person who cannot see the page is owed, and it is a *count*
        // rather than a note: §9.10.2's own "there is no way to determine what the character code
        // represents" is the standard's answer and not a refusal of ours, so it may not join the
        // list above (ADR 0422).
        let readbacks: Vec<(usize, pdf_model::content::Shortfall)> =
            match viewer.query(Query::Readback) {
                Answer::Readback(pages) => pages
                    .iter()
                    .map(|page| (page.page, page.shortfall))
                    .collect(),
                _ => Vec::new(),
            };
        let shown = structures
            .into_iter()
            .map(|structure| PageReading {
                page: structure.page,
                label: match viewer.query(Query::PageLabel(structure.page)) {
                    Answer::Label(label) => Some(label),
                    _ => None,
                },
                bounds: place(viewer, structure.page, viewport),
                reports: reports
                    .iter()
                    .find(|(page, _)| *page == structure.page)
                    .map_or_else(Vec::new, |(_, notes)| notes.clone()),
                readback: readbacks
                    .iter()
                    .find(|(page, _)| *page == structure.page)
                    .map_or_else(pdf_model::content::Shortfall::default, |(_, count)| *count),
                nodes: structure.nodes,
            })
            .collect();
        Self {
            window: window.to_owned(),
            document: document.to_owned(),
            pages,
            viewport,
            shown,
        }
    }

    /// How many pages this screen is publishing, which is what a host's trace line says.
    #[must_use]
    pub fn pages_shown(&self) -> usize {
        self.shown.len()
    }

    /// How many §14.7 elements crossed, over every page on the screen.
    #[must_use]
    pub fn elements(&self) -> usize {
        self.shown.iter().map(|page| page.nodes.len()).sum()
    }

    /// How many sentences this screen's pages could not draw.
    #[must_use]
    pub fn reports(&self) -> usize {
        self.shown.iter().map(|page| page.reports.len()).sum()
    }

    /// How many character codes §9.10.2 could name nothing for, over every page on the screen.
    ///
    /// **The half of this answer that reached no native host at all**, which is why it is counted
    /// here rather than left inside the tree: `tools/state.sh windows` named `Query::Readback` as a
    /// debt of its own, landing and falling with the tree it is published inside.
    #[must_use]
    pub fn unreadable(&self) -> usize {
        self.shown
            .iter()
            .map(|page| page.readback.unnamed.total())
            .sum()
    }

    /// The borrowed view [`crate::tree::build`] takes, assembled from what this owns.
    ///
    /// Called with a closure rather than returned, because [`crate::DocumentView`] borrows a slice
    /// of [`crate::PageView`] that has to live somewhere and this is the only place that can own it.
    pub(crate) fn with_view<T>(&self, take: impl FnOnce(&crate::DocumentView<'_>) -> T) -> T {
        let shown: Vec<crate::PageView<'_>> = self
            .shown
            .iter()
            .map(|page| crate::PageView {
                page: page.page,
                label: page.label.as_deref(),
                bounds: page.bounds,
                nodes: &page.nodes,
                reports: &page.reports,
                readback: page.readback,
            })
            .collect();
        take(&crate::DocumentView {
            window: &self.window,
            document: &self.document,
            pages: self.pages,
            viewport: self.viewport,
            shown: &shown,
        })
    }
}

/// Where one page sits in the viewport, in the device pixels every quadrilateral is in.
///
/// The whole viewport where the viewer will not say, which is the `SinglePage` case a page with no
/// geometry can only be: two page nodes claiming the viewport would be two nodes claiming one
/// place, and under a column the geometry is always answered.
#[expect(
    clippy::cast_precision_loss,
    reason = "a raster's extent in device pixels; f32 is exact to 2^24 and no display is"
)]
fn place(viewer: &Viewer, page: usize, viewport: (f32, f32)) -> [f32; 4] {
    match viewer.query(Query::PageGeometry(page)) {
        Answer::Geometry(geometry) => [
            geometry.origin.0,
            geometry.origin.1,
            geometry.origin.0 + geometry.width as f32,
            geometry.origin.1 + geometry.height as f32,
        ],
        _ => [0.0, 0.0, viewport.0, viewport.1],
    }
}

/// Whether a command can have changed what §14.7's tree says, so that it must be published again.
///
/// **[`Showing`] is not enough on its own, and this is the other half of the same decision.** That
/// comparison asks whether the *page* changed; an edit changes neither the page nor the viewport,
/// so a check box a person ticked went on being announced as unticked until the
/// five-hundred-and-ninetieth session — and after ADR 0425 that included one an assistive
/// technology had clicked itself. `viewer-ui` has forgotten what it published on these commands
/// ever since; it is here rather than there because the two native hosts publish now too, and the
/// third copy of a rule is where two hosts stop agreeing about it (ADR 0623).
///
/// `Event::Dirty` looked like the condition and is not: it fires when the flag *changes*, so only
/// the first edit of a session raises it.
///
/// **A released pointer is on this list and it was not on `viewer-ui`'s**, which is what driving
/// the two native hosts over a real bus showed: those hosts delegate §12.7's widgets, so an
/// assistive technology's `Action.DoAction` arrives as a press and a release on the page rather
/// than as an [`viewer_core::Edit`] — and §12.5.1's activation, §12.6.3's triggers and
/// §12.7.5.2's toggling all follow from a click. The *release* rather than the motion is what
/// keeps it cheap: a drag emits hundreds of `Dragged` and one `Released`, so this costs one tree
/// per click and per completed selection rather than one per pointer sample.
#[must_use]
pub fn republishes(command: &viewer_core::Command) -> bool {
    matches!(
        command,
        viewer_core::Command::Edit(_)
            | viewer_core::Command::Undo
            | viewer_core::Command::Redo
            | viewer_core::Command::SetGroup { .. }
            | viewer_core::Command::Pointer {
                action: viewer_core::PointerAction::Released,
                ..
            }
    )
}
