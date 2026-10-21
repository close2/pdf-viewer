//! Reports, viewer preferences and structured readback for a C caller.

use super::Session;
use crate::answers::{Collection, Miniature, Structure};
use crate::events::Events;
use crate::kinds::{
    BoundaryKind, DirectionKind, DuplexKind, LayoutKind, PageModeKind, PreferenceKey,
    PrintScalingKind, ShortfallKind,
};
use crate::status::Status;
use viewer_core::{Answer, Command, Query};

impl Session {
    /// Ask the focused document what it says about *itself*, and be told in words.
    ///
    /// The eight clauses `viewer_core::notes::about` answers come back on
    /// [`viewer_core::Event::Reported`] with no page on them. **A caller asks once the reader has
    /// their page**: §12.8's answer digests the signed part of the file, which `CLAUDE.md`
    /// principle 2 keeps off the launch path, and a second asking answers the same sentences for
    /// nothing. ADR 1044.
    #[must_use]
    pub fn report(&mut self) -> Events {
        self.handle(Command::Report)
    }

    /// How many pages on the screen this viewer has anything to say about.
    ///
    /// **The entry point a C caller could not have deduced**, and it exists for
    /// [`Self::frame_count`]'s reason said again: a C consumer cannot fail to compile, so a
    /// `/PageLayout` putting a second page on the screen has to be something a caller *asks*
    /// about. Zero where no document is focused or no page has been interpreted, and one for the
    /// `SinglePage` every document that says nothing about it opens in.
    #[must_use]
    pub fn reported_pages(&self) -> usize {
        match self.viewer.query(Query::Reports) {
            Answer::Reports(pages) => pages.len(),
            _ => 0,
        }
    }

    /// Which page one of those entries is about, zero-based.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, [`Status::OutOfRange`] for an entry
    /// past [`Self::reported_pages`].
    pub fn reported_page(&self, entry: usize) -> Result<usize, Status> {
        let Answer::Reports(pages) = self.viewer.query(Query::Reports) else {
            return Err(Status::NoAnswer);
        };
        pages
            .get(entry)
            .map(|page| page.page)
            .ok_or(Status::OutOfRange)
    }

    /// Everything one page on the screen could not draw.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, [`Status::OutOfRange`] for an entry
    /// past [`Self::reported_pages`].
    pub fn reports(&self, entry: usize) -> Result<Vec<String>, Status> {
        let Answer::Reports(pages) = self.viewer.query(Query::Reports) else {
            return Err(Status::NoAnswer);
        };
        pages
            .get(entry)
            .map(|page| page.notes.to_vec())
            .ok_or(Status::OutOfRange)
    }

    /// Table 29's `/PageMode` and `/PageLayout`: what the catalogue asks of the window opening it.
    ///
    /// Both, in one call, because §7.7.2 states them as one instruction to a window and a caller
    /// obeys them together — one chooses which panel is open and the other how the pages are laid
    /// out. [`Self::preference`] is Table 147, which is a different table and a separate call for
    /// that reason.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn opening(&self) -> Result<(PageModeKind, LayoutKind), Status> {
        match self.viewer.query(Query::Opening) {
            Answer::Opening(opening) => Ok((
                PageModeKind::of(opening.mode),
                LayoutKind::of(opening.layout),
            )),
            _ => Err(Status::NoAnswer),
        }
    }

    /// One entry of §12.2's Table 147, as a number.
    ///
    /// See [`PreferenceKey`] for why this is one keyed accessor rather than nineteen symbols or a
    /// struct. Every entry Table 147 gives a default answers that default; the three it leaves
    /// genuinely open answer [`Status::NoAnswer`], because "the document says nothing" and "the
    /// document says the default" are different facts and only the first leaves the choice to the
    /// caller.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused or the entry is one Table 147 leaves
    /// open and the document states none; [`Status::WrongKind`] for `/PrintPageRange`, which is a
    /// list and is read with [`Self::preference_ranges`].
    pub fn preference(&self, key: PreferenceKey) -> Result<i64, Status> {
        let Answer::Preferences(preferences) = self.viewer.query(Query::Preferences) else {
            return Err(Status::NoAnswer);
        };
        let boolean = |value: bool| Ok(i64::from(value));
        match key {
            PreferenceKey::HideToolbar => boolean(preferences.hide_toolbar),
            PreferenceKey::HideMenubar => boolean(preferences.hide_menubar),
            PreferenceKey::HideWindowUi => boolean(preferences.hide_window_ui),
            PreferenceKey::FitWindow => boolean(preferences.fit_window),
            PreferenceKey::CenterWindow => boolean(preferences.center_window),
            PreferenceKey::DisplayDocTitle => boolean(preferences.display_doc_title),
            PreferenceKey::EnforcePrintScaling => boolean(preferences.enforce_print_scaling),
            PreferenceKey::NonFullScreenPageMode => Ok(i64::from(
                PageModeKind::of(preferences.non_full_screen_page_mode).code(),
            )),
            PreferenceKey::Direction => {
                Ok(i64::from(DirectionKind::of(preferences.direction) as u32))
            }
            PreferenceKey::ViewArea => {
                Ok(i64::from(BoundaryKind::of(preferences.view_area) as u32))
            }
            PreferenceKey::ViewClip => {
                Ok(i64::from(BoundaryKind::of(preferences.view_clip) as u32))
            }
            PreferenceKey::PrintArea => {
                Ok(i64::from(BoundaryKind::of(preferences.print_area) as u32))
            }
            PreferenceKey::PrintClip => {
                Ok(i64::from(BoundaryKind::of(preferences.print_clip) as u32))
            }
            PreferenceKey::PrintScaling => Ok(i64::from(PrintScalingKind::of(
                preferences.print_scaling,
            ) as u32)),
            PreferenceKey::Duplex => preferences
                .duplex
                .map(|duplex| i64::from(DuplexKind::of(duplex) as u32))
                .ok_or(Status::NoAnswer),
            PreferenceKey::PickTrayByPdfSize => preferences
                .pick_tray_by_pdf_size
                .map(i64::from)
                .ok_or(Status::NoAnswer),
            PreferenceKey::NumCopies => preferences.num_copies.ok_or(Status::NoAnswer),
            // Named and refused rather than absent, which is trap 5 in the small: a key that did
            // not exist would look like a table this build had not read, and this one says which
            // function to ask instead.
            PreferenceKey::PrintPageRange => Err(Status::WrongKind),
        }
    }

    /// Table 147's `/PrintPageRange`: "the page numbers used to initialize the print dialog box".
    ///
    /// The one entry of that table that is a list, which is why it is not [`Self::preference`]'s.
    /// Pairs, because the entry is "an array of integers … \[t]he first and last pages in a
    /// sub-range", and an empty list is a document that states none.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused.
    pub fn preference_ranges(&self) -> Result<Vec<(i64, i64)>, Status> {
        match self.viewer.query(Query::Preferences) {
            Answer::Preferences(preferences) => Ok(preferences.print_page_range),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.9's measurement of a traced path, as one of its parts or as the whole sentence.
    ///
    /// `part` is `0` for the sentence `viewer_host::measuring::said` composes — which is the only
    /// form §12.10's reading crosses in, because every part of that reading is a *statement*
    /// about the map rather than a quantity — and `1` to `6` for Table 265's `/Name`, Table 267's
    /// `/R`, and the strings §12.9.2's algorithm produced from `/D`, `/A`, `/T` and `/S`.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, where no viewport's `/BBox` contains
    /// the first point — §12.9.1 chooses "the viewport of the first point" and no other — where
    /// the path leaves the page it started on, and where the part asked for is one the document
    /// states nothing for. [`Status::OutOfRange`] for a `part` this build does not define.
    pub fn measure(&self, points: &[[f32; 2]], part: u32) -> Result<String, Status> {
        let Answer::Measured { traced, located } = self.viewer.query(Query::Measure(points)) else {
            return Err(Status::NoAnswer);
        };
        let Some(part) = crate::MeasurePart::from_code(part) else {
            return Err(Status::OutOfRange);
        };
        let part = match part {
            crate::MeasurePart::Sentence => {
                return Ok(viewer_host::measuring::said(
                    points.len(),
                    Some(&traced),
                    located.as_ref(),
                ));
            }
            crate::MeasurePart::Viewport => traced.viewport,
            crate::MeasurePart::Ratio => traced.ratio,
            crate::MeasurePart::Length => traced.length,
            crate::MeasurePart::Area => traced.area,
            crate::MeasurePart::Angle => traced.angle,
            crate::MeasurePart::Slope => traced.slope,
        };
        part.ok_or(Status::NoAnswer)
    }

    /// §12.3.6's preview picture for one of §12.3.5's attachments, by its `/EmbeddedFiles` key.
    ///
    /// Three of Table 160's named layouts are built out of pictures of the *attachments* — a strip
    /// of thumbnails, thumbnails scattered over the view, a large preview beside the metadata —
    /// and the standard says what a picture of a document's page is in one place, §12.3.4's
    /// `/Thumb`. So this opens the named attachment and answers its **first page's** `/Thumb`: the
    /// miniature that file's own producer wrote (ADR 1251).
    ///
    /// The same [`Miniature`] a page's thumbnail comes back as, so `quorra_thumbnail_info`,
    /// `quorra_thumbnail_copy` and `quorra_thumbnail_free` serve both and a caller learns one
    /// idiom.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, where the tree holds no such key, and
    /// where the attachment is not a PDF or its first page states no `/Thumb` — which is most
    /// attachments, and is a document being quiet rather than a failure.
    pub fn attachment_preview(&self, name: &str) -> Result<Miniature, Status> {
        match self.viewer.query(Query::AttachmentPreview(name)) {
            Answer::Thumbnail(thumbnail) => Ok(Miniature::new(thumbnail)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// How many pages on the screen have a readback shortfall to report.
    ///
    /// [`Self::reported_pages`]'s counterpart, and one entry per page for the same reason: a
    /// column shows several pages and a caller given one page's counts for four would be silent
    /// about three of them.
    #[must_use]
    pub fn readback_pages(&self) -> usize {
        match self.viewer.query(Query::Readback) {
            Answer::Readback(pages) => pages.len(),
            _ => 0,
        }
    }

    /// Which page one of those entries is about, zero-based.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, [`Status::OutOfRange`] for an entry past
    /// [`Self::readback_pages`].
    pub fn readback_page(&self, entry: usize) -> Result<usize, Status> {
        let Answer::Readback(pages) = self.viewer.query(Query::Readback) else {
            return Err(Status::NoAnswer);
        };
        pages
            .get(entry)
            .map(|page| page.page)
            .ok_or(Status::OutOfRange)
    }

    /// One of §9.10.2's counts for one page on the screen.
    ///
    /// **Not a report, and [`viewer_core::Query::Readback`] states the reading**: §9.10.2's own
    /// closing sentence is "there is no way to determine what the character code represents", so a
    /// code that route ends at is an answer the standard states rather than something this program
    /// failed to do. What a caller does with it is the thing a person needs — say that a search
    /// found nothing on a page whose text cannot be read, or that a copied selection is short.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused, [`Status::OutOfRange`] for an entry past
    /// [`Self::readback_pages`].
    pub fn readback_count(&self, entry: usize, which: ShortfallKind) -> Result<usize, Status> {
        let Answer::Readback(pages) = self.viewer.query(Query::Readback) else {
            return Err(Status::NoAnswer);
        };
        pages
            .get(entry)
            .map(|page| which.of(&page.shortfall))
            .ok_or(Status::OutOfRange)
    }

    /// §14.7's logical structure for every page the arrangement is showing.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused. A page with an **empty** node list is an
    /// untagged page, which is an answer rather than a silence.
    pub fn structure(&self) -> Result<Structure, Status> {
        match self.viewer.query(Query::AccessibilityTree) {
            Answer::Accessibility(pages) => Ok(Structure::new(pages)),
            _ => Err(Status::NoAnswer),
        }
    }

    /// §12.3.5's portable collection, where the catalogue states one.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no document is focused or it states no collection, which is
    /// every document in this project's corpora.
    pub fn collection(&self) -> Result<Collection, Status> {
        match self.viewer.query(Query::Collection) {
            Answer::Collection {
                collection,
                initial,
                order,
            } => Ok(Collection::new(&collection, &initial, order)),
            _ => Err(Status::NoAnswer),
        }
    }
}
