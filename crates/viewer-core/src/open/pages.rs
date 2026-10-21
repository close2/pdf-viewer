//! The pages of an open document: which exists, how large, and which is showing.
//!
//! Counting §12.7.8.3.3's imported template pages after the document's own, and the one question
//! that places a point of the screen on a byte of a page's readback.

use pdf_model::{Page, Pages};
use pdf_render::Size;

use super::Open;

impl Open {
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
    ///
    /// **And on §12.2's print pair of boundaries while a print operation stands.** Table 147
    /// states `/PrintArea` and `/PrintClip` beside `/ViewArea` and `/ViewClip` — "[t]he name of
    /// the page boundary representing the area of a page that shall be rendered when printing
    /// the document" against the same sentence about the screen — so which pair a page carries
    /// is decided by what the output is for, and this is the one place in this crate where a
    /// page is built. The purpose is the one `begin_printing` stated, which is why nothing here
    /// consults `printing` instead: `ViewState` is where an operation says what it is producing
    /// and `Page::render_for_printing` is where that reaches a rectangle. ADR 1227.
    pub(crate) fn page(&self, index: usize) -> Option<Page> {
        let pages = Pages::new(&self.document);
        let mut page = if let Some(page) = pages.get(index) {
            page
        } else {
            let appended = index.checked_sub(pages.len())?;
            // Table 253's `/F` divides the two: a template page of this document is an object of
            // it, and one of a second file crossed as a copy that names nothing of that file, so
            // both are read as §12.7.7's detached page and neither needs a `/Parent` (ADR 1239).
            let held;
            let dict = match self.view.appended_pages().get(appended)? {
                pdf_model::view::AppendedPage::Named(id) => {
                    held = self.document.get(*id);
                    held.as_dict()?
                }
                pdf_model::view::AppendedPage::Carried(page) => page,
            };
            pages.detached(dict)
        };
        if self.view.purpose() == pdf_model::optional_content::Purpose::Print {
            page.render_for_printing();
        }
        Some(page)
    }

    /// The text position a point in one page's display-list coordinates selects.
    pub(crate) fn position_at(&self, page: usize, point: (f32, f32)) -> Option<usize> {
        let interpreted = self.interpretation(page)?;
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
        if let Some(on_screen) = self.on(index) {
            if let Some(interpreted) = on_screen.interpreted.as_ref() {
                return Some(interpreted.list.page_size);
            }
            return Some(pdf_model::content::displayed_size(&on_screen.object));
        }
        self.page(index)
            .map(|page| pdf_model::content::displayed_size(&page))
    }

    /// The current page, without walking the tree for it again.
    pub(crate) fn shown_page(&self) -> Option<&Page> {
        self.placed_page(self.page_index)
    }

    /// Any page the arrangement shows, without walking the tree for it again.
    pub(crate) fn placed_page(&self, page: usize) -> Option<&Page> {
        self.on(page).map(|on_screen| &on_screen.object)
    }

    /// How many pages there are now, which §12.7.8.3.3's imported templates may have changed.
    pub(crate) fn recount(&mut self) {
        self.page_count = Pages::new(&self.document)
            .len()
            .saturating_add(self.view.appended_pages().len());
    }
}
