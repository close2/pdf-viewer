//! What a viewer answers a caller reads, the same through either handle (ADR 2104).
//!
//! A `quorra_viewer *` owns its [`Viewer`] and a `quorra_window *` holds one inside the window's
//! state, and ADR 2088 refused lending the second out as the first: a `Session` owns its viewer by
//! value. So what a caller *reads* — the panels' rows, the form on the page, the shapes the find bar
//! and the selection draw — is answered by [`Reads`], a borrow of whichever viewer the handle holds,
//! and the two families of entry points hand back the same owning handles from the same code.

use viewer_core::{Answer, Query, Viewer};

use crate::status::Status;
use crate::{Form, Matches, Miniature, Outline, Panel, Popups, Quads};

/// A borrow of one viewer, answering what a caller reads of it.
#[derive(Debug, Clone, Copy)]
pub struct Reads<'a> {
    /// The viewer whose answers these are.
    viewer: &'a Viewer,
}

impl<'a> Reads<'a> {
    /// The view of `viewer`.
    #[must_use]
    pub const fn of(viewer: &'a Viewer) -> Self {
        Self { viewer }
    }

    /// How many pages the focused document has.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn page_count(&self) -> Result<usize, Status> {
        match self.viewer.query(Query::PageCount) {
            Answer::Count(count) => Ok(count),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.3.3's outline, flattened into rows with a depth on each.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused or it states no outline.
    pub fn outline(&self) -> Result<Outline, Status> {
        match self.viewer.query(Query::Outline) {
            Answer::Outline(outline) => Ok(Outline::of(&outline)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// The shapes covering what is selected, in device pixels of the viewport.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn selection_quads(&self) -> Result<Quads, Status> {
        match self.viewer.query(Query::Selection) {
            Answer::Selected(selected) => Ok(Quads::new(selected.quads)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// Annex O's highlighted rectangles on the page being shown.
    ///
    /// Table Annex O.4's `highlight`, which a URI's fragment states and this program cannot draw
    /// for a caller: "\[t]he nature of the highlighting is implementation-dependent", so the shapes
    /// cross and the colour is the caller's. Empty where the fragment named none, which is why
    /// this cannot fail for an open document.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn highlight_quads(&self) -> Result<Quads, Status> {
        match self.viewer.query(Query::Highlight) {
            Answer::Highlighted(quads) => Ok(Quads::new(quads)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// Which annotation holds §12.5.1's focus, and where it is on the screen.
    ///
    /// Named for the *annotation* rather than for the query, because [`crate::Session::focus`] is already
    /// [`viewer_core::Command::Focus`] — a document being made the one commands apply to, which is a different
    /// thing entirely and was here first.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where nothing is focused or it states no usable `/Rect`.
    pub fn focused_annotation(&self) -> Result<((u32, u16), [f32; 8]), Status> {
        match self.viewer.query(Query::Focus) {
            Answer::Focus { object, quad } => Ok(((object.number, object.generation), quad)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.7's fields with a widget on the page being shown.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused. A page with no widget on it answers an
    /// empty form, which is an answer rather than a refusal.
    pub fn fields(&self) -> Result<Form, Status> {
        match self.viewer.query(Query::Fields) {
            Answer::Fields(fields) => Ok(Form::of(&fields)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §8.11.4.3's `/Order`, flattened.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused or it states no optional content.
    pub fn layers(&self) -> Result<Panel, Status> {
        match self.viewer.query(Query::Layers) {
            Answer::Layers(layers) => Ok(Panel::of_layers(&layers)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §7.11.4's embedded files, listed rather than extracted.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused or it states no `/EmbeddedFiles` tree.
    pub fn attachments(&self) -> Result<Panel, Status> {
        match self.viewer.query(Query::Attachments) {
            Answer::Attachments(attachments) => Ok(Panel::of_attachments(&attachments)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// Every occurrence of a string on the page being shown, as shapes to draw over it.
    ///
    /// **This page, and [`crate::Session::find_start`] is the document.** The two are one search asked
    /// twice: this answers from a readback that already exists, in microseconds, so a find bar may
    /// ask it on every repaint; the other interprets pages nobody is looking at and takes one step
    /// at a time. A caller that had only the second could run a search and not draw a match, which
    /// is what this ABI was until now.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn find(&self, needle: &str) -> Result<Matches, Status> {
        match self.viewer.query(Query::Find(needle)) {
            Answer::Found(occurrences) => Ok(Matches::new(occurrences)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §14.3.3's Table 349 and §14.3.2's metadata stream, as a panel would show them.
    ///
    /// **`viewer_host::property_rows`, unchanged**, which is what makes this two lines rather than
    /// a second reading of two tables: both are shown rather than merged, because Table 349's text
    /// entries each carry a NOTE pointing at an XMP counterpart, §12.2 ranks `dc:title` above
    /// `/Title` and nothing ranks the rest, and §14.3.4 leaves the disagreement "at the discretion
    /// of the PDF processor". A panel that merged them would hide a disagreement rather than
    /// resolve one, and this ABI is not the place that decision changes.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn properties(&self) -> Result<Panel, Status> {
        match self.viewer.query(Query::Properties) {
            Answer::Properties {
                information,
                metadata,
            } => Ok(Panel::of_rows(&viewer_host::property_rows(
                &information,
                metadata.as_ref(),
            ))),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.4.3's article threads, as a panel would list them.
    ///
    /// Each row acts through `quorra_activate` on the thread's object, which is the same message an
    /// outline row sends and for the same reason: the *document* decides what activating a thing
    /// means, and `viewer_core::interact` composes §12.6.4.7's thread action out of it so that
    /// following a thread lands on Table 163's `/R` rather than on the page the first bead sits on.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn articles(&self) -> Result<Panel, Status> {
        match self.viewer.query(Query::Articles) {
            Answer::Articles(threads) => Ok(Panel::of_rows(&viewer_host::article_rows(&threads))),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.4.2's label for one page, where the document states one.
    ///
    /// **A page at a time, and separate from [`Self::thumbnail`] on purpose.** A caller drawing a
    /// page list needs a name per row and a picture only for the rows it is showing; folding the
    /// two into one call would make listing a thousand pages decode a thousand images, which is
    /// exactly the launch-path defect ADR 0564 names.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused or the page states no label, which is
    /// most pages of most documents — the clause makes the index what identifies a page and the
    /// label an addition, so a caller falls back to the number rather than to nothing.
    pub fn page_label(&self, index: usize) -> Result<String, Status> {
        match self.viewer.query(Query::PageLabel(index)) {
            Answer::Label(label) => Ok(label),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.3.4's thumbnail for one page, decoded.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, where the page states no `/Thumb` — the
    /// clause's own NOTE says they "are not required, and can be included for some pages and not
    /// for others" — or where this reader could not decode the one it states, which is reported
    /// through the same channel any other undecodable image is.
    pub fn thumbnail(&self, index: usize) -> Result<Miniature, Status> {
        match self.viewer.query(Query::Thumbnail(index)) {
            Answer::Thumbnail(thumbnail) => Ok(Miniature::new(thumbnail)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.5.6.14's open popup windows on the page being shown.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused. An **empty list** where the page has
    /// none open, which is a different answer and is why the two are told apart.
    pub fn popups(&self) -> Result<Popups, Status> {
        match self.viewer.query(Query::Popups) {
            Answer::Popups(windows) => Ok(Popups::new(&windows)),
            _ => Err(Status::NoAnswer),
        }
    }
}
