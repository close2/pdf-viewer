//! What a print dialogue opens with, and at what resolution the pages are drawn.
//!
//! **Two things every window with a printer needs and no toolkit supplies.** The first is §12.2's
//! half of Table 147: eight of that table's entries are about printing, and the clause makes every
//! one of them a statement about a dialogue rather than about a page. The second is the resolution
//! — a number RFC 0004 fixes with a budget, and one no clause states at all.
//!
//! It is here rather than in [`viewer_core`] for this crate's standing reason: a print dialogue is
//! chrome, `GtkPrintOperation` against `QPrinter` against an IPP attribute is what a print system
//! is, and which sentence each of them is obeying is not. And it is here rather than in a host
//! because the third copy is where two windows stop agreeing.
//!
//! # What §12.2 asks of a dialogue, in its own words
//!
//! §12.2 introduces the dictionary as one "controlling the way the document shall be presented on
//! the screen or in print", and Table 147 states its eight print entries as *defaults a dialogue
//! opens with* rather than as instructions. [`Defaults`] is those eight gathered, and nothing here
//! decides anything with them: a host hands them to its print system, a person changes what they
//! like, and what comes back is the job. That is the clause's own arrangement — each of the five
//! that are about a dialogue names the value it opens at — and the one entry that goes further
//! says so, which is [`Defaults::scaling_enforced`].
//!
//! ADR 1180.

use pdf_model::page::Boundary;
use pdf_model::viewer_preferences::{Duplex, PrintScaling, ViewerPreferences};
use viewer_core::Sheet;

/// Dots per inch a printed page is drawn at where the print system reports none.
///
/// **A documented choice and not a reading**: no clause in ISO 32000-2 states a resolution for a
/// printed page, because the standard describes marks and not devices. 300 is the field's
/// rasterise-for-print floor and RFC 0004 §3 is where it was chosen, against the alternatives that
/// exist: a screen-grade 150 loses the difference a printer can show, and a device-native 1200
/// quadruples the pixels of every page for a difference contone raster of text does not carry.
pub const DEFAULT_DPI: f32 = 300.0;

/// The narrowest resolution a printed page is drawn at.
///
/// Below this a printed page is visibly a screen picture, which is the one thing Route B may not
/// cost — RFC 0004 §3.
pub const MIN_DPI: f32 = 150.0;

/// The widest.
///
/// **A stated, revisable budget rather than a limit of the arithmetic.** Doubling this doubles
/// each axis and quadruples the pixels of every page in the job, and at 600 an A4 page is already
/// 4960 × 7016. RFC 0004 §3 is where it was chosen and where raising it would be argued.
pub const MAX_DPI: f32 = 600.0;

/// How many points there are to an inch — §8.3.2.3's default user space unit.
///
/// > 1/72 inch
///
/// which is what makes a resolution in dots per inch and a scale in pixels per unit the same
/// number divided by this one.
const POINTS_PER_INCH: f32 = 72.0;

/// The scale one page is drawn at, from whatever the print system said its resolution is.
///
/// `None` is a print system that reported nothing, under which [`DEFAULT_DPI`] stands. Anything
/// outside [`MIN_DPI`]..=[`MAX_DPI`] is clamped to it rather than refused: a printer that reports
/// 1200 is a printer, and a job it would take four times as long to draw is still the job the
/// person asked for at the resolution this program has a budget for.
///
/// A resolution that is not a finite positive number is [`DEFAULT_DPI`], because a print system
/// that answered `NaN` has said nothing rather than something strange.
#[must_use]
pub fn scale(dpi: Option<f32>) -> f32 {
    let dpi = dpi.filter(|dpi| dpi.is_finite() && *dpi > 0.0);
    dpi.unwrap_or(DEFAULT_DPI).clamp(MIN_DPI, MAX_DPI) / POINTS_PER_INCH
}

/// The sheet a page is placed on, for a page printed at its own size at the sheet's corner.
///
/// §12.5.6.22 measures Table 194's percentages from "the origin of the media (e.g., printed
/// page)", and [`Sheet::media`] is that rectangle in default user space. `width` and `height` are
/// the paper's dimensions in points, which is what every print system in reach reports: GTK's
/// `PaperSize` and Qt's `QPageSize` both answer in points, and IPP's media dimensions are
/// hundredths of a millimetre a caller converts.
///
/// **The page's own corner is the origin here**, which is the case §12.5.6.22 calls "the usual
/// case where the PDF page size equals the media size" read one step out: this program places a
/// page on the sheet unscaled and unrotated, so the matrix the clause asks to be cancelled is the
/// identity but for that translation. A host that scales a page onto the sheet — shrink-to-fit,
/// n-up, tiling — owes the rest of that sentence and is not offered this function.
#[must_use]
pub fn sheet(width: f32, height: f32, dpi: Option<f32>) -> Sheet {
    Sheet {
        media: Some([0.0, 0.0, width.max(0.0), height.max(0.0)]),
        scale: scale(dpi),
    }
}

/// The job of a window that is showing what would print without having a printer.
///
/// Table 193's other branch, stated rather than guessed: "[i]f the dimensions of the target media
/// are not known at the time of drawing, drawing shall be done relative to the dimensions
/// specified by the page's MediaBox entry". A sheet size invented here would put a fixed print
/// watermark somewhere no printer chose.
#[must_use]
#[expect(
    clippy::doc_markdown,
    reason = "verbatim quotation: Table 193 spells MediaBox without backticks"
)]
pub fn unknown_sheet(dpi: Option<f32>) -> Sheet {
    Sheet {
        media: None,
        scale: scale(dpi),
    }
}

/// §12.2's Table 147, as a print dialogue's opening state.
///
/// Every field is what the *document* asked for, and a person changes any of them. What this type
/// is for is that all four windows open their dialogue on the same eight answers, read once from
/// the same clause, rather than each reading Table 147 for itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defaults {
    /// `/PrintScaling`: whether the dialogue opens on the processor's own scaling or on none.
    ///
    /// §12.2, Table 147:
    ///
    /// > The page scaling option that shall be selected when a print dialogue is displayed for
    /// > this document.
    ///
    /// [`PrintScaling::AppDefault`] leaves the choice to the host, which is the entry's own
    /// meaning — "the interactive PDF processor's default print scaling".
    pub scaling: PrintScaling,
    /// Table 148's `/Enforce`: whether `/PrintScaling` may be changed at all.
    ///
    /// The one entry of Table 147 that is more than a default, and `pdf_model` has already applied
    /// its condition — it holds only where `/PrintScaling` states a valid value other than
    /// `AppDefault`, which is Table 148's own wording.
    ///
    /// **A document telling a reader what they may not change is a restriction, and
    /// `CLAUDE.md` makes those the reader's**: this is reported rather than obeyed, so a host
    /// shows the dialogue with the scaling the document asked for and says that it asked.
    pub scaling_enforced: bool,
    /// `/Duplex`: the paper handling option the dialogue opens with.
    ///
    /// `None` is Table 147's own "implementation dependent", which is a different answer from a
    /// stated default and leaves the choice to the print system.
    pub duplex: Option<Duplex>,
    /// `/PickTrayByPDFSize`: whether the input tray is chosen from the page size.
    ///
    /// `None` is "implementation dependent" again.
    pub pick_tray_by_page_size: Option<bool>,
    /// `/PrintPageRange`: pairs of first and last page, **one-based as the clause writes them**.
    ///
    /// Kept one-based because §12.2, Table 147's own NOTE says so:
    ///
    /// > Although PrintPageRange uses 1-based page numbering, other features of PDF use
    /// > zero-based page numbering.
    ///
    /// A converted value would be indistinguishable from the other kind at the point of use.
    /// Empty where the document states none.
    pub page_range: Vec<(i64, i64)>,
    /// `/NumCopies`: how many copies the dialogue opens with, where the document says.
    ///
    /// `None` is "implementation dependent". The value is the document's, unclamped: a host that
    /// thinks a number is unreasonable is the party that has a person to ask.
    pub copies: Option<i64>,
    /// `/PrintArea`, **deprecated in PDF 2.0**: the boundary a printed page is rendered to.
    ///
    /// Carried rather than applied, and the deprecation is why: the entry is in the table, a
    /// twenty-year-old document may state it, and what a host does with a deprecated preference is
    /// a host's decision to take with it in hand rather than one this crate takes by dropping it.
    pub area: Boundary,
    /// `/PrintClip`, **deprecated in PDF 2.0**: the boundary printed contents are clipped to.
    pub clip: Boundary,
}

impl Defaults {
    /// Table 147's print half, read off the preferences `viewer_core::Query::Preferences` answered.
    ///
    /// A document with no `/ViewerPreferences` dictionary answers with
    /// [`ViewerPreferences::default`], which is Table 147's own defaults — so this needs no branch
    /// for a document that states nothing, and a host needs none either.
    #[must_use]
    pub fn of(preferences: &ViewerPreferences) -> Self {
        Self {
            scaling: preferences.print_scaling,
            scaling_enforced: preferences.enforce_print_scaling,
            duplex: preferences.duplex,
            pick_tray_by_page_size: preferences.pick_tray_by_pdf_size,
            page_range: preferences.print_page_range.clone(),
            copies: preferences.num_copies,
            area: preferences.print_area,
            clip: preferences.print_clip,
        }
    }

    /// Whether this page — **one-based**, as `/PrintPageRange` counts — is in the document's range.
    ///
    /// `true` for every page where the document states no range, which is Table 147's own answer:
    /// the entry is optional and its absence is not a range of nothing.
    #[must_use]
    pub fn in_page_range(&self, page: i64) -> bool {
        self.page_range.is_empty()
            || self
                .page_range
                .iter()
                .any(|&(first, last)| page >= first && page <= last)
    }

    /// What to say about a document that asked for its scaling not to be changed.
    ///
    /// `None` where it asked for nothing. A sentence rather than a refusal, because Table 148's
    /// `/Enforce` is a document asserting something over the person reading it and `CLAUDE.md`
    /// makes every one of those the reader's to set.
    #[must_use]
    pub fn enforcement_note(&self) -> Option<&'static str> {
        self.scaling_enforced.then_some(
            "this document asks that its print scaling not be changed (§12.2, Table 148's /Enforce)",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_DPI, Defaults, MAX_DPI, MIN_DPI, POINTS_PER_INCH, scale, sheet, unknown_sheet,
    };
    use pdf_model::viewer_preferences::{Duplex, PrintScaling, ViewerPreferences};

    /// The clamp is RFC 0004 §3's budget, and both ends of it hold.
    #[test]
    fn a_reported_resolution_is_clamped_to_the_budget_and_an_unreported_one_is_the_default() {
        assert!((scale(None) - DEFAULT_DPI / POINTS_PER_INCH).abs() < f32::EPSILON);
        assert!((scale(Some(1200.0)) - MAX_DPI / POINTS_PER_INCH).abs() < f32::EPSILON);
        assert!((scale(Some(72.0)) - MIN_DPI / POINTS_PER_INCH).abs() < f32::EPSILON);
        // Inside the budget the printer's own number stands, which is the case the clamp exists
        // to leave alone — without this the two above would pass for a function that answered 300
        // whatever it was told.
        assert!((scale(Some(600.0)) - MAX_DPI / POINTS_PER_INCH).abs() < f32::EPSILON);
        assert!((scale(Some(400.0)) - 400.0 / POINTS_PER_INCH).abs() < f32::EPSILON);
        // A print system that answered with nothing usable has said nothing.
        assert!((scale(Some(f32::NAN)) - DEFAULT_DPI / POINTS_PER_INCH).abs() < f32::EPSILON);
        assert!((scale(Some(-100.0)) - DEFAULT_DPI / POINTS_PER_INCH).abs() < f32::EPSILON);
    }

    /// §12.5.6.22's media is the sheet's own rectangle, with its corner at the origin.
    #[test]
    fn a_sheet_is_the_papers_rectangle_in_default_user_space() {
        let a4 = sheet(595.0, 842.0, Some(300.0));
        assert_eq!(a4.media, Some([0.0, 0.0, 595.0, 842.0]));
        assert!((a4.scale - 300.0 / POINTS_PER_INCH).abs() < f32::EPSILON);
        // Table 193's other branch is an answer and not an absence, so it keeps the resolution.
        let unknown = unknown_sheet(Some(300.0));
        assert_eq!(unknown.media, None);
        assert!((unknown.scale - a4.scale).abs() < f32::EPSILON);
    }

    /// Table 147's print half reaches a dialogue entry by entry, and its absence is its defaults.
    #[test]
    fn table_147s_print_entries_are_what_a_dialogue_opens_with() {
        let silent = Defaults::of(&ViewerPreferences::default());
        assert_eq!(silent.scaling, PrintScaling::AppDefault);
        assert!(!silent.scaling_enforced);
        assert_eq!(
            silent.duplex, None,
            "Table 147's own implementation dependent"
        );
        assert_eq!(silent.pick_tray_by_page_size, None);
        assert!(silent.page_range.is_empty());
        assert_eq!(silent.copies, None);
        // A document stating no range has not stated a range of nothing, so every page is in it.
        assert!(silent.in_page_range(1) && silent.in_page_range(9999));
        assert_eq!(silent.enforcement_note(), None);

        let stated = Defaults::of(&ViewerPreferences {
            print_scaling: PrintScaling::NoScaling,
            enforce_print_scaling: true,
            duplex: Some(Duplex::FlipLongEdge),
            pick_tray_by_pdf_size: Some(true),
            print_page_range: vec![(2, 4), (9, 9)],
            num_copies: Some(3),
            ..ViewerPreferences::default()
        });
        assert_eq!(stated.scaling, PrintScaling::NoScaling);
        assert!(stated.scaling_enforced);
        assert_eq!(stated.duplex, Some(Duplex::FlipLongEdge));
        assert_eq!(stated.pick_tray_by_page_size, Some(true));
        assert_eq!(stated.copies, Some(3));
        assert!(stated.enforcement_note().is_some());
        // One-based, inclusive at both ends, and the gap between the pairs is a gap.
        for (page, inside) in [(1, false), (2, true), (4, true), (5, false), (9, true)] {
            assert_eq!(
                stated.in_page_range(page),
                inside,
                "page {page} against {:?}",
                stated.page_range
            );
        }
    }
}
