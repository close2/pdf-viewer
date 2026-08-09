//! The numbers a C caller switches on, and the names it prints for a number it does not know.
//!
//! **This module is where `viewer-core`'s "nothing is `#[non_exhaustive]`" rule is translated
//! into the only thing C offers.** On the Rust side a new [`viewer_core::Event`] fails to compile
//! in every consumer; here it becomes a `uint32_t` an old caller has no arm for. Three things
//! make that survivable, and all three live here: every kind has a **name**, every kind has a
//! **count** a caller can check against its header at startup, and the `match` in
//! [`EventKind::of`] is exhaustive over `viewer_core::Event`, so a variant added to that crate
//! fails to compile *in this file* and cannot reach a caller unnamed.

use viewer_core::{Event, PageTarget, Zoom};

/// What a C caller is told an event is.
///
/// The numbers are the ABI. A kind added later takes the next one and never reuses an old one;
/// [`EventKind::COUNT`] moves with it and is what `pdfv_abi_check` compares against the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum EventKind {
    /// [`viewer_core::Event::Opened`].
    Opened = 0,
    /// [`viewer_core::Event::OpenFailed`].
    OpenFailed = 1,
    /// [`viewer_core::Event::PasswordRequired`].
    PasswordRequired = 2,
    /// [`viewer_core::Event::Closed`].
    Closed = 3,
    /// [`viewer_core::Event::PageChanged`].
    PageChanged = 4,
    /// [`viewer_core::Event::NeedsRender`].
    NeedsRender = 5,
    /// [`viewer_core::Event::Damage`].
    Damage = 6,
    /// [`viewer_core::Event::OpenUri`].
    OpenUri = 7,
    /// [`viewer_core::Event::NeedsFile`].
    NeedsFile = 8,
    /// [`viewer_core::Event::Transition`].
    Transition = 9,
    /// [`viewer_core::Event::Dirty`].
    Dirty = 10,
    /// [`viewer_core::Event::Saved`].
    Saved = 11,
    /// [`viewer_core::Event::Extracted`].
    Extracted = 12,
    /// [`viewer_core::Event::Refused`].
    Refused = 13,
    /// [`viewer_core::Event::Reported`].
    Reported = 14,
    /// [`viewer_core::Event::Searched`].
    Searched = 15,
}

impl EventKind {
    /// How many kinds this build has.
    ///
    /// **The number a C caller checks its header against**, which is the whole of what this ABI
    /// can offer in place of a build failure. It is written out rather than counted by a macro so
    /// that adding a variant is a line a person writes beside the variant, in the same commit.
    pub const COUNT: u32 = 16;

    /// Which kind an event is.
    ///
    /// Exhaustive over [`viewer_core::Event`] with no catch-all arm, deliberately: a message
    /// added to that crate has to fail to compile here, because this is the last place in the
    /// program where a compiler can still say so.
    #[must_use]
    pub const fn of(event: &Event) -> Self {
        match event {
            Event::Opened { .. } => Self::Opened,
            Event::OpenFailed { .. } => Self::OpenFailed,
            Event::PasswordRequired { .. } => Self::PasswordRequired,
            Event::Closed(_) => Self::Closed,
            Event::PageChanged { .. } => Self::PageChanged,
            Event::NeedsRender(_) => Self::NeedsRender,
            Event::Damage(_) => Self::Damage,
            Event::OpenUri { .. } => Self::OpenUri,
            Event::NeedsFile { .. } => Self::NeedsFile,
            Event::Transition { .. } => Self::Transition,
            Event::Dirty { .. } => Self::Dirty,
            Event::Saved { .. } => Self::Saved,
            Event::Extracted { .. } => Self::Extracted,
            Event::Refused { .. } => Self::Refused,
            Event::Reported { .. } => Self::Reported,
            Event::Searched { .. } => Self::Searched,
        }
    }

    /// The name, as `viewer-core` spells the variant, NUL-terminated for `const char *`.
    ///
    /// What a caller prints for a kind it has no arm for. It is a *name* rather than a sentence
    /// because it is the one thing that is true of every event of the kind; the sentence is
    /// `pdfv_events_describe`, which reads the event itself.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Opened => "Opened\0",
            Self::OpenFailed => "OpenFailed\0",
            Self::PasswordRequired => "PasswordRequired\0",
            Self::Closed => "Closed\0",
            Self::PageChanged => "PageChanged\0",
            Self::NeedsRender => "NeedsRender\0",
            Self::Damage => "Damage\0",
            Self::OpenUri => "OpenUri\0",
            Self::NeedsFile => "NeedsFile\0",
            Self::Transition => "Transition\0",
            Self::Dirty => "Dirty\0",
            Self::Saved => "Saved\0",
            Self::Extracted => "Extracted\0",
            Self::Refused => "Refused\0",
            Self::Reported => "Reported\0",
            Self::Searched => "Searched\0",
        }
    }

    /// The name for a number, or `None` for one this build does not define.
    ///
    /// The lookup a caller makes when it has a kind it does not recognise — which is exactly the
    /// case where it *cannot* have got the number from this enumeration, so the conversion has to
    /// be fallible and has to be refusable.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Opened,
            1 => Self::OpenFailed,
            2 => Self::PasswordRequired,
            3 => Self::Closed,
            4 => Self::PageChanged,
            5 => Self::NeedsRender,
            6 => Self::Damage,
            7 => Self::OpenUri,
            8 => Self::NeedsFile,
            9 => Self::Transition,
            10 => Self::Dirty,
            11 => Self::Saved,
            12 => Self::Extracted,
            13 => Self::Refused,
            14 => Self::Reported,
            15 => Self::Searched,
            _ => return None,
        })
    }

    /// The number, as C sees it.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

/// Which of [`viewer_core::PageTarget`]'s six a `pdfv_go_to_page` call means.
///
/// Two arguments in C where Rust has one enum, because [`PageTarget::Index`] and
/// [`PageTarget::Relative`] carry a number and the other four carry nothing. The number is
/// ignored for those four rather than being required to be zero: a caller writing
/// `pdfv_go_to_page(v, PDFV_PAGE_NEXT, 0, &events)` and one writing `-1` mean the same thing, and
/// refusing one of them would be this boundary inventing a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum PageTargetKind {
    /// A zero-based index, which the argument carries.
    Index = 0,
    /// The first page.
    First = 1,
    /// The last page.
    Last = 2,
    /// The next page, or nowhere if this is the last.
    Next = 3,
    /// The previous page.
    Previous = 4,
    /// A signed number of pages from here, which the argument carries.
    Relative = 5,
}

impl PageTargetKind {
    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Index,
            1 => Self::First,
            2 => Self::Last,
            3 => Self::Next,
            4 => Self::Previous,
            5 => Self::Relative,
            _ => return None,
        })
    }

    /// The `viewer-core` target, given the argument the caller passed beside the kind.
    ///
    /// `None` where the argument does not fit: a page index is a `usize` and this boundary takes
    /// an `int64_t`, so a negative index is a caller's mistake rather than a page.
    #[must_use]
    pub fn target(self, argument: i64) -> Option<PageTarget> {
        Some(match self {
            Self::Index => PageTarget::Index(usize::try_from(argument).ok()?),
            Self::First => PageTarget::First,
            Self::Last => PageTarget::Last,
            Self::Next => PageTarget::Next,
            Self::Previous => PageTarget::Previous,
            Self::Relative => PageTarget::Relative(isize::try_from(argument).ok()?),
        })
    }
}

/// Which of [`viewer_core::Zoom`]'s six a `pdfv_zoom` call means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum ZoomKind {
    /// The whole page, as large as fits.
    FitPage = 0,
    /// The page's width.
    FitWidth = 1,
    /// The page's height.
    FitHeight = 2,
    /// A fixed magnification, which the argument carries.
    Scale = 3,
    /// One step larger.
    In = 4,
    /// One step smaller.
    Out = 5,
}

impl ZoomKind {
    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::FitPage,
            1 => Self::FitWidth,
            2 => Self::FitHeight,
            3 => Self::Scale,
            4 => Self::In,
            5 => Self::Out,
            _ => return None,
        })
    }

    /// The `viewer-core` zoom, given the scale the caller passed beside the kind.
    #[must_use]
    pub const fn zoom(self, scale: f32) -> Zoom {
        match self {
            Self::FitPage => Zoom::FitPage,
            Self::FitWidth => Zoom::FitWidth,
            Self::FitHeight => Zoom::FitHeight,
            Self::Scale => Zoom::Scale(scale),
            Self::In => Zoom::In,
            Self::Out => Zoom::Out,
        }
    }
}

/// The pixel layout of a raster this ABI hands over.
///
/// **One variant, and the reason it is a number at all is ADR 0247's first amendment.**
/// `pdf_render::RasterFormat` stopped being `#[non_exhaustive]` in the four-hundred-and-eleventh
/// session precisely so that a second layout would fail to compile in every consumer — and this
/// crate is the consumer that could not have failed. [`Self::of`] is exhaustive over that enum,
/// so a second layout stops the build *here*, where a person has to decide what number C gets for
/// it, rather than reaching a caller that would blit it as RGBA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum PixelFormat {
    /// Four bytes per pixel: red, green, blue, alpha, straight alpha, no row padding.
    Rgba8 = 0,
}

impl PixelFormat {
    /// Which layout a raster is in.
    #[must_use]
    pub const fn of(format: pdf_render::RasterFormat) -> Self {
        match format {
            pdf_render::RasterFormat::Rgba8 => Self::Rgba8,
        }
    }

    /// The number, as C sees it.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

#[cfg(test)]
mod tests {
    use super::{EventKind, PageTargetKind, PixelFormat, ZoomKind};

    /// The count the header states is the count this enumeration has.
    ///
    /// **The one assertion this whole design rests on.** `pdfv_abi_check` compares a caller's
    /// `PDFV_EVENT_KIND_COUNT` against `EventKind::COUNT`, so a kind added without moving the
    /// constant would leave every C caller believing it was up to date. `from_code` walking off
    /// the end is what catches it.
    #[test]
    fn the_kind_count_is_the_number_of_kinds() {
        for code in 0..EventKind::COUNT {
            assert!(
                EventKind::from_code(code).is_some(),
                "{code} is below the count and names nothing"
            );
        }
        assert!(
            EventKind::from_code(EventKind::COUNT).is_none(),
            "the count is one past the last kind"
        );
    }

    /// Every kind's number round-trips, and every name is NUL-terminated exactly once.
    #[test]
    fn every_kind_answers_to_its_own_number_and_carries_a_name() {
        for code in 0..EventKind::COUNT {
            let kind = EventKind::from_code(code).expect("checked above");
            assert_eq!(kind.code(), code);
            let name = kind.name();
            assert_eq!(name.matches('\0').count(), 1, "{name:?}");
            assert!(name.ends_with('\0'), "{name:?}");
        }
    }

    /// The other three enumerations round-trip too, which is what a header can be checked against.
    #[test]
    fn the_argument_enumerations_answer_to_their_own_numbers() {
        for code in 0..6 {
            assert_eq!(
                PageTargetKind::from_code(code).map(|kind| kind as u32),
                Some(code)
            );
            assert_eq!(
                ZoomKind::from_code(code).map(|kind| kind as u32),
                Some(code)
            );
        }
        assert!(PageTargetKind::from_code(6).is_none());
        assert!(ZoomKind::from_code(6).is_none());
        assert_eq!(PixelFormat::of(pdf_render::RasterFormat::Rgba8).code(), 0);
    }

    /// A page index that does not fit `usize` is refused rather than wrapped.
    #[test]
    fn a_negative_page_index_is_not_a_page() {
        assert!(PageTargetKind::Index.target(-1).is_none());
        assert!(PageTargetKind::Index.target(3).is_some());
        // A relative target is signed by design, so the same number is fine there.
        assert!(PageTargetKind::Relative.target(-1).is_some());
        // And the four that carry nothing ignore the argument rather than refusing it.
        assert!(PageTargetKind::Next.target(-99).is_some());
    }
}
