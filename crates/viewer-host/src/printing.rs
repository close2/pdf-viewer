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
use viewer_core::{Fidelity, Sheet};

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
/// n-up — states the rest of that sentence through [`placed`] instead.
#[must_use]
pub fn sheet(width: f32, height: f32, dpi: Option<f32>) -> Sheet {
    Sheet {
        media: Some([0.0, 0.0, width.max(0.0), height.max(0.0)]),
        scale: scale(dpi),
        page_scale: 1.0,
    }
}

/// The scale a printed page is drawn at once §7.6.4.2's Table 22 bit 12 has been asked.
///
/// **This is the implementation-dependent algorithm the cell hands to a processor**, written
/// down rather than left unchosen (ADR 1203). Table 22:
///
/// > When this bit is clear (and bit 3 is set), printing shall be limited to a low- level
/// > representation of the appearance, possibly of degraded quality.
///
/// What leaves this program is a raster of the page, so the resolution is the thing that decides
/// whether "a faithful digital copy of the PDF content could be generated" from it — and
/// [`MIN_DPI`] is the low-level representation because it is this module's own floor: the
/// narrowest resolution a printed page is drawn at at all, below which RFC 0004 §3 says a
/// printed page is visibly a screen picture. Choosing the floor rather than a number invented
/// for the occasion is what keeps the degradation a *statement* rather than a second budget.
///
/// The second half of the algorithm is [`destination_refused`], because a job written to a file
/// is a document whatever its resolution.
#[must_use]
pub fn scale_at(fidelity: Fidelity, dpi: Option<f32>) -> f32 {
    match fidelity {
        Fidelity::Faithful => scale(dpi),
        Fidelity::Degraded => MIN_DPI / POINTS_PER_INCH,
    }
}

/// Why a destination that writes a document may not be offered, where it may not.
///
/// `None` is a job Table 22 bit 12 left alone, which is every job where the document withholds
/// nothing or this reader said not to obey it.
///
/// **The destination is the half of bit 12 the resolution cannot answer.** A print-to-file
/// destination produces exactly what the cell's own words describe — "a representation from which
/// a faithful digital copy of the PDF content could be generated" — at any resolution at all, so
/// degrading the pixels and then writing them into a document would obey the bit in one direction
/// and not the other. A host that has such a destination refuses it by name and says why; one
/// that has none has nothing to do here, which is how the windows stay level (ADR 1203).
#[must_use]
pub fn destination_refused(fidelity: Fidelity) -> Option<&'static str> {
    match fidelity {
        Fidelity::Faithful => None,
        Fidelity::Degraded => Some(
            "this document does not permit printing to a file (§7.6.4.2, Table 22 bit 12);              printing to a printer goes ahead at the lowest resolution this program draws at",
        ),
    }
}

/// RFC 0004 §6's scale modes: how large a page is drawn on the sheet it is placed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scaling {
    /// The page at its own size, whether or not it fits. §12.2's `/PrintScaling` `None`.
    ActualSize,
    /// The page at its own size unless it is too large, and then just small enough.
    ///
    /// The default, and the field's: a page that already fits is not touched, so a document
    /// printed on the paper it was written for is unscaled and §12.5.6.22's matrix B stays the
    /// identity.
    #[default]
    ShrinkToFit,
    /// The page as large as fits, enlarging a small one.
    FitToPage,
}

/// How many pages go on one sheet — RFC 0004 §6's 1, 2 and 4.
///
/// **The grid is a convention and no clause states one.** §12.5.6.22 says only what happens to a
/// watermark "[w]hen n -up printing is selected", leaving what n-up *is* to the processor; this
/// splits the sheet's longer axis for two and both axes for four, which keeps each cell's
/// proportions as near the sheet's as the count allows. Written down as a choice (ADR 1204).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PagesPerSheet {
    /// One page per sheet: the cell is the sheet.
    #[default]
    One,
    /// Two, side by side across the sheet's longer axis.
    Two,
    /// Four, two by two.
    Four,
}

impl PagesPerSheet {
    /// How many pages one sheet takes.
    #[must_use]
    pub const fn count(self) -> usize {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Four => 4,
        }
    }

    /// How the sheet is divided, as (across, up) for a sheet of these proportions.
    fn grid(self, paper: (f32, f32)) -> (usize, usize) {
        match self {
            Self::One => (1, 1),
            Self::Two if paper.0 >= paper.1 => (2, 1),
            Self::Two => (1, 2),
            Self::Four => (2, 2),
        }
    }
}

/// Where one page of an n-up sheet goes, in points from the paper's lower-left corner.
///
/// What a host paints into: the raster of that page, scaled to `extent`, with its lower-left
/// corner at `origin`. The cells are filled left to right and top to bottom, which is the reading
/// order every print system in reach uses and which no clause states.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// The page's lower-left corner on the paper, in points.
    pub origin: (f32, f32),
    /// How large the page is drawn there, in points.
    pub extent: (f32, f32),
}

/// The sheet a job of these pages on this paper is going onto — §12.5.6.22's media and its B.
///
/// **The media is the *cell* and not the paper**, which is the clause's own instruction for
/// n-up: the annotations "shall be positioned as if the dimensions of the printed page were
/// limited to a single portion of the page". For one page per sheet the cell is the paper and
/// this is [`sheet`] with a scale mode applied.
///
/// The rectangle is stated in the *page's* own default user space, so the page's origin is at
/// (0, 0) and the cell's corners fall wherever the placement put them — which is how the
/// translation term of the clause's B sentence is carried by the value rather than computed from
/// it. [`Sheet::page_scale`] is what is left of B, and [`Cell`] is the same placement said the
/// other way round, for a host that has to paint it.
///
/// `scale` is pixels per default user space unit at the printer's resolution, from [`scale`] or
/// [`scale_at`]; the raster is multiplied by the page's own scale so that a page drawn at half
/// size costs a quarter of the pixels and still lands on the paper at the printer's resolution.
///
/// A paper or a page with no extent gives the page back at its own size against the paper, which
/// is [`sheet`]'s answer: there is no placement to compute and nothing a print system said that
/// this could honour.
#[must_use]
pub fn placed(
    paper: (f32, f32),
    page: (f32, f32),
    scaling: Scaling,
    per_sheet: PagesPerSheet,
    scale: f32,
) -> Sheet {
    let Some(placement) = geometry(paper, page, scaling, per_sheet) else {
        return Sheet {
            media: Some([0.0, 0.0, paper.0.max(0.0), paper.1.max(0.0)]),
            scale,
            page_scale: 1.0,
        };
    };
    // The cell in the page's own space: its corner is as far from the page's origin as the page
    // sits from the cell's, divided by what the page is scaled by. Which cell of the sheet it is
    // does not enter — every cell of a grid places its page the same way inside itself, so the
    // sheet is one value for the whole job.
    let scale_back = |value: f32| value / placement.page_scale;
    let (left, bottom) = (
        -scale_back(placement.inset.0),
        -scale_back(placement.inset.1),
    );
    Sheet {
        media: Some([
            left,
            bottom,
            left + scale_back(placement.cell.0),
            bottom + scale_back(placement.cell.1),
        ]),
        scale: scale * placement.page_scale,
        page_scale: placement.page_scale,
    }
}

/// Where the `index`th page of a sheet is painted, for the same job [`placed`] described.
///
/// `None` for an index past what this many pages per sheet holds, and for a paper or a page with
/// no extent.
#[must_use]
pub fn cell(
    paper: (f32, f32),
    page: (f32, f32),
    scaling: Scaling,
    per_sheet: PagesPerSheet,
    index: usize,
) -> Option<Cell> {
    if index >= per_sheet.count() {
        return None;
    }
    let placement = geometry(paper, page, scaling, per_sheet)?;
    let (across, _) = per_sheet.grid(paper);
    // `across` is 1 or 2 by construction — [`PagesPerSheet::grid`] returns no other value — so
    // neither of these can divide by zero; `checked_*` says so rather than a comment alone.
    let column = index.checked_rem(across)?;
    let row = index.checked_div(across)?.checked_add(1)?;
    Some(Cell {
        origin: (
            as_f32(column) * placement.cell.0 + placement.inset.0,
            // Filled top to bottom, and the paper's origin is its lower-left corner, so row 0 is
            // the topmost one and its cell's own lower edge is one cell below the paper's top.
            paper.1 - as_f32(row) * placement.cell.1 + placement.inset.1,
        ),
        extent: placement.extent,
    })
}

/// One sheet's arrangement, with the cell's position on the paper left out.
///
/// The three facts every cell of a grid shares: how large a cell is, how far inside it the page
/// sits, and what the page was scaled by to fit. The cell's own corner is the only thing that
/// differs between them, which is why [`placed`] needs none of it and [`cell`] adds it.
struct Placement {
    /// A cell's size in points.
    cell: (f32, f32),
    /// The page's lower-left corner within its cell, in points.
    inset: (f32, f32),
    /// How large the page is drawn, in points.
    extent: (f32, f32),
    /// §12.5.6.22's B, as much of it as a placement without rotation has.
    page_scale: f32,
}

/// The arrangement of one sheet, or `None` where nothing usable was said.
///
/// A paper or a page with no extent is a print system that answered nothing rather than a job
/// with a placement of zero, so the caller falls back to the page at its own size.
fn geometry(
    paper: (f32, f32),
    page: (f32, f32),
    scaling: Scaling,
    per_sheet: PagesPerSheet,
) -> Option<Placement> {
    let usable = |value: f32| value.is_finite() && value > 0.0;
    if !usable(paper.0) || !usable(paper.1) || !usable(page.0) || !usable(page.1) {
        return None;
    }
    let (across, up) = per_sheet.grid(paper);
    let cell = (paper.0 / as_f32(across), paper.1 / as_f32(up));
    let fit = (cell.0 / page.0).min(cell.1 / page.1);
    let page_scale = match scaling {
        Scaling::ActualSize => 1.0,
        Scaling::ShrinkToFit => fit.min(1.0),
        Scaling::FitToPage => fit,
    };
    if !usable(page_scale) {
        return None;
    }
    let extent = (page.0 * page_scale, page.1 * page_scale);
    // Centred in the cell, which is what every print system in reach does and what no clause
    // states. A page larger than its cell insets negatively, which is the page hanging over the
    // edge — the honest arithmetic for `ActualSize` on paper too small for the page.
    Some(Placement {
        cell,
        inset: ((cell.0 - extent.0) / 2.0, (cell.1 - extent.1) / 2.0),
        extent,
        page_scale,
    })
}

/// A small count as a coordinate. Exact: a grid is never more than a handful of cells.
#[expect(
    clippy::cast_precision_loss,
    reason = "a grid's dimension is 1 or 2, and every integer below 2^24 is exact in an f32"
)]
fn as_f32(value: usize) -> f32 {
    value as f32
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
        page_scale: 1.0,
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
        DEFAULT_DPI, Defaults, MAX_DPI, MIN_DPI, POINTS_PER_INCH, PagesPerSheet, Scaling, cell,
        destination_refused, placed, scale, scale_at, sheet, unknown_sheet,
    };
    use pdf_model::viewer_preferences::{Duplex, PrintScaling, ViewerPreferences};
    use viewer_core::Fidelity;

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

    /// §7.6.4.2's Table 22 bit 12, as this module carries it out: a floor and a destination.
    ///
    /// **The pair is what makes it a reading rather than a clamp.** A degraded job that was only
    /// drawn smaller would still write a file a faithful copy could be generated from, which is
    /// the cell's own subject — "a representation from which a faithful digital copy of the PDF
    /// content could be generated" — so the destination is refused beside it (ADR 1203).
    #[test]
    fn a_withheld_bit_12_draws_at_the_floor_and_refuses_a_destination_that_writes_a_document() {
        // Faithful is the unchanged function, at every one of its three answers.
        for dpi in [None, Some(1200.0), Some(400.0)] {
            assert!((scale_at(Fidelity::Faithful, dpi) - scale(dpi)).abs() < f32::EPSILON);
        }
        // Degraded is the floor whatever the printer reported, including a printer that reported
        // *less* than the floor — which the clamp would already have raised.
        for dpi in [None, Some(1200.0), Some(400.0), Some(72.0)] {
            assert!(
                (scale_at(Fidelity::Degraded, dpi) - MIN_DPI / POINTS_PER_INCH).abs()
                    < f32::EPSILON,
                "{dpi:?}"
            );
        }
        assert_eq!(destination_refused(Fidelity::Faithful), None);
        let refusal = destination_refused(Fidelity::Degraded).expect("a sentence");
        assert!(
            refusal.contains("Table 22 bit 12"),
            "the refusal names the position it comes from: {refusal}"
        );
    }

    /// §12.5.6.22's two post-EXAMPLE bullets, as a placement a host can paint and a core can read.
    ///
    /// The n-up bullet fixes both halves: the annotations "shall be printed at the specified size
    /// and shall be positioned as if the dimensions of the printed page were limited to a single
    /// portion of the page". So the media is the *cell* and the factor is the page's, and the two
    /// have to agree — a cell a host painted into and a media the core measured percentages
    /// against that disagreed would put a watermark somewhere no printer chose.
    ///
    /// A4 in points, two up: the sheet's longer axis is halved, so each cell is 595 x 421 and a
    /// 595 x 842 page fits at exactly half size.
    #[test]
    fn an_n_up_sheet_places_each_page_in_its_own_cell_at_the_factor_the_core_cancels() {
        let paper = (595.0, 842.0);
        let page = (595.0, 842.0);
        let job = placed(
            paper,
            page,
            Scaling::ShrinkToFit,
            PagesPerSheet::Two,
            300.0 / POINTS_PER_INCH,
        );
        assert!(
            (job.page_scale - 0.5).abs() < f32::EPSILON,
            "half of each axis"
        );
        // The cell in the page's own space: 595 x 421 of paper at half size is 1190 x 842 of
        // page, and the page sits at its corner because it fills the cell in one axis and the
        // other is exactly its own height.
        let media = job.media.expect("a sheet somebody chose");
        assert!((media[2] - media[0] - 1190.0).abs() < 0.01, "{media:?}");
        assert!((media[3] - media[1] - 842.0).abs() < 0.01, "{media:?}");
        // The raster is the page's own scale times the printer's, so a page drawn at half size
        // costs a quarter of the pixels and still lands at the printer's resolution.
        assert!((job.scale - 150.0 / POINTS_PER_INCH).abs() < f32::EPSILON);

        // The two cells tile the paper's longer axis and neither overlaps the other.
        let first = cell(paper, page, Scaling::ShrinkToFit, PagesPerSheet::Two, 0).expect("a cell");
        let second =
            cell(paper, page, Scaling::ShrinkToFit, PagesPerSheet::Two, 1).expect("a cell");
        assert!((first.extent.0 - 297.5).abs() < 0.01, "{first:?}");
        assert!((first.extent.1 - 421.0).abs() < 0.01, "{first:?}");
        assert_eq!(first.extent, second.extent, "one size for every cell");
        assert!(
            (first.origin.1 - 421.0).abs() < 0.01,
            "the top row: {first:?}"
        );
        assert!(
            (second.origin.1 - 0.0).abs() < 0.01,
            "the bottom row: {second:?}"
        );
        assert!(
            cell(paper, page, Scaling::ShrinkToFit, PagesPerSheet::Two, 2).is_none(),
            "a third page on a sheet that holds two"
        );
    }

    /// One page per sheet at its own size is what [`sheet`] already built, and the three modes
    /// differ only where the page and the paper differ.
    #[test]
    fn a_single_page_placed_at_its_own_size_is_the_clauses_usual_case() {
        let paper = (595.0, 842.0);
        let unscaled = placed(
            paper,
            paper,
            Scaling::ShrinkToFit,
            PagesPerSheet::One,
            300.0 / POINTS_PER_INCH,
        );
        assert_eq!(unscaled.media, sheet(595.0, 842.0, Some(300.0)).media);
        assert!((unscaled.page_scale - 1.0).abs() < f32::EPSILON);

        // A page twice the paper: shrinking halves it, actual size leaves it alone and hanging
        // over the edge, and fitting is shrinking here because the page is the larger.
        let large = (1190.0, 1684.0);
        let modes = [
            (Scaling::ActualSize, 1.0),
            (Scaling::ShrinkToFit, 0.5),
            (Scaling::FitToPage, 0.5),
        ];
        for (mode, expected) in modes {
            let job = placed(paper, large, mode, PagesPerSheet::One, 1.0);
            assert!((job.page_scale - expected).abs() < f32::EPSILON, "{mode:?}");
        }
        // And a page half the paper: only fitting enlarges it, which is the difference between
        // the two modes that are otherwise the same.
        let small = (297.5, 421.0);
        for (mode, expected) in [(Scaling::ShrinkToFit, 1.0), (Scaling::FitToPage, 2.0)] {
            let job = placed(paper, small, mode, PagesPerSheet::One, 1.0);
            assert!((job.page_scale - expected).abs() < f32::EPSILON, "{mode:?}");
        }
        // Nothing usable said is the page at its own size against the paper, rather than a
        // placement of zero — a print system that answered nothing has not asked for anything.
        let nothing = placed(
            (0.0, 842.0),
            paper,
            Scaling::FitToPage,
            PagesPerSheet::Four,
            1.0,
        );
        assert!((nothing.page_scale - 1.0).abs() < f32::EPSILON);
        assert!(
            cell(
                (0.0, 842.0),
                paper,
                Scaling::FitToPage,
                PagesPerSheet::Four,
                0
            )
            .is_none()
        );
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
