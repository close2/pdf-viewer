//! The numbers a C caller switches on, and the names it prints for a number it does not know.
//!
//! **This module is where `viewer-core`'s "nothing is `#[non_exhaustive]`" rule is translated
//! into the only thing C offers.** On the Rust side a new [`viewer_core::Event`] fails to compile
//! in every consumer; here it becomes a `uint32_t` an old caller has no arm for. Three things
//! make that survivable, and all three live here: every kind has a **name**, every kind has a
//! **count** a caller can check against its header at startup, and the `match` in
//! [`EventKind::of`] is exhaustive over `viewer_core::Event`, so a variant added to that crate
//! fails to compile *in this file* and cannot reach a caller unnamed.

//! # Two kinds of enumeration, and only one of them is counted
//!
//! An enumeration this ABI **takes** — [`PageTargetKind`], [`ZoomKind`], [`PointerKind`] — refuses
//! a number it does not define, with [`crate::Status::WrongKind`]. Nothing else is possible: a
//! caller has asked for something this build has no meaning for.
//!
//! An enumeration this ABI **answers with** — [`EventKind`], [`ControlKind`], [`RowKind`],
//! [`PixelFormat`] — cannot refuse, because the number is already in the caller's hands. Each is
//! produced by a `match` that is exhaustive over the Rust type behind it, so a variant added to
//! `viewer-core`, `pdf-model` or `viewer-host` fails to compile *here*, which is the last place a
//! compiler can still say so; and each has a `from_code`, so a caller that meets a number this
//! build does not define learns that it does not rather than switching on it by accident.
//!
//! **Only [`EventKind`] has a count in `pdfv_abi_check`, and that is deliberate rather than an
//! omission.** An event *arrives*: a caller receives one whether or not it asked, so a kind added
//! later is met by a program that has no arm for it and the check has to happen before the first
//! one turns up. A control kind and a row kind are answers to a question the caller asked, in a
//! call it wrote, and `pdfv_control_kind_name` and `pdfv_row_kind_name` are there for the number it
//! did not expect. Widening `pdfv_abi_check` would change the signature of the one function every
//! compiled caller already calls in `main`, which is precisely the hazard the four shapes were
//! chosen against.

use pdf_model::view::{Markup, WidgetAppearances};
use viewer_core::{
    Event, FocusMove, PageTarget, PointerAction, PresentationMode, Purpose, RestrictionLevel,
    Selection, Zoom,
};
use viewer_host::ControlKind as HostControl;

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

/// Which of §12.5.5's three situations [`viewer_core::Command::Pointer`] reports.
///
/// Taken, so a number outside the four is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum PointerKind {
    /// The pointer moved with no button down.
    Moved = 0,
    /// The button went down.
    Pressed = 1,
    /// The pointer moved with the button held, which extends a selection.
    Dragged = 2,
    /// The button came up, which is what activates a link.
    Released = 3,
}

impl PointerKind {
    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Moved,
            1 => Self::Pressed,
            2 => Self::Dragged,
            3 => Self::Released,
            _ => return None,
        })
    }

    /// What `viewer-core` calls it.
    #[must_use]
    pub const fn action(self) -> PointerAction {
        match self {
            Self::Moved => PointerAction::Moved,
            Self::Pressed => PointerAction::Pressed,
            Self::Dragged => PointerAction::Dragged,
            Self::Released => PointerAction::Released,
        }
    }
}

/// What [`viewer_core::Command::Select`] asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum SelectKind {
    /// Everything the page reads back as.
    All = 0,
    /// Nothing.
    None = 1,
}

impl SelectKind {
    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::All,
            1 => Self::None,
            _ => return None,
        })
    }

    /// What `viewer-core` calls it.
    #[must_use]
    pub const fn selection(self) -> Selection {
        match self {
            Self::All => Selection::All,
            Self::None => Selection::None,
        }
    }
}

/// Which way §12.5.1's tab key moves the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum FocusKind {
    /// The next annotation, wrapping at the end. What the tab key means.
    Next = 0,
    /// The previous one. What shift-tab means.
    Previous = 1,
    /// Nothing focused.
    None = 2,
}

impl FocusKind {
    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Next,
            1 => Self::Previous,
            2 => Self::None,
            _ => return None,
        })
    }

    /// What `viewer-core` calls it.
    #[must_use]
    pub const fn moved(self) -> FocusMove {
        match self {
            Self::Next => FocusMove::Next,
            Self::Previous => FocusMove::Previous,
            Self::None => FocusMove::None,
        }
    }
}

/// How much of what a document asserts over its reader this viewer obeys.
///
/// Two of `CLAUDE.md`'s four levels, and the other two are a *question* rather than a level — see
/// [`viewer_core::RestrictionLevel`], which says why shipping them as numbers nothing answers
/// would be worse than not shipping them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum RestrictKind {
    /// Obey what the document asserts. The default.
    On = 0,
    /// Ignore it and perform the operation.
    Off = 1,
}

impl RestrictKind {
    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::On,
            1 => Self::Off,
            _ => return None,
        })
    }

    /// What `viewer-core` calls it.
    #[must_use]
    pub const fn level(self) -> RestrictionLevel {
        match self {
            Self::On => RestrictionLevel::On,
            Self::Off => RestrictionLevel::Off,
        }
    }
}

/// Whether §12.4.4's presentation is running, as the host has said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum PresentKind {
    /// A document being read. The default.
    Off = 0,
    /// A presentation: §12.4.4.2's nodes are respected and a page turn plays `/Trans`.
    On = 1,
}

impl PresentKind {
    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Off,
            1 => Self::On,
            _ => return None,
        })
    }

    /// What `viewer-core` calls it.
    #[must_use]
    pub const fn mode(self) -> PresentationMode {
        match self {
            Self::Off => PresentationMode::Off,
            Self::On => PresentationMode::On,
        }
    }
}

/// Who draws §12.7's widget appearances — §6.3.2.2's "unless otherwise instructed".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum DelegateKind {
    /// This library draws them, which is what §6.3.2.2 asks of a processor nobody has instructed.
    Drawn = 0,
    /// The caller draws them, so the page is interpreted without them.
    Delegated = 1,
}

impl DelegateKind {
    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Drawn,
            1 => Self::Delegated,
            _ => return None,
        })
    }

    /// What `pdf-model` calls it.
    #[must_use]
    pub const fn appearances(self) -> WidgetAppearances {
        match self {
            Self::Drawn => WidgetAppearances::Drawn,
            Self::Delegated => WidgetAppearances::Delegated,
        }
    }
}

/// Which of §12.5.6.10's four text markup annotations to add over what is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum MarkupKind {
    /// Table 182's `Highlight`.
    Highlight = 0,
    /// Table 182's `Underline`.
    Underline = 1,
    /// Table 182's `StrikeOut`.
    StrikeOut = 2,
    /// Table 182's `Squiggly`.
    Squiggly = 3,
}

impl MarkupKind {
    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Highlight,
            1 => Self::Underline,
            2 => Self::StrikeOut,
            3 => Self::Squiggly,
            _ => return None,
        })
    }

    /// What `pdf-model` calls it.
    #[must_use]
    pub const fn markup(self) -> Markup {
        match self {
            Self::Highlight => Markup::Highlight,
            Self::Underline => Markup::Underline,
            Self::StrikeOut => Markup::StrikeOut,
            Self::Squiggly => Markup::Squiggly,
        }
    }
}

/// What a file the viewer asks for is wanted for.
///
/// Both taken and answered — [`viewer_core::Event::NeedsFile`] states it and
/// [`viewer_core::Command::Supply`] echoes it back — so it has both conversions, and the one from
/// Rust is exhaustive so that a second purpose fails to compile here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum PurposeKind {
    /// §12.7.6.4's import-data action: the file holds §12.7.8's form data.
    ImportData = 0,
}

impl PurposeKind {
    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::ImportData,
            _ => return None,
        })
    }

    /// Which purpose an event states.
    #[must_use]
    pub const fn of(purpose: Purpose) -> Self {
        match purpose {
            Purpose::ImportData => Self::ImportData,
        }
    }

    /// What `viewer-core` calls it.
    #[must_use]
    pub const fn purpose(self) -> Purpose {
        match self {
            Self::ImportData => Purpose::ImportData,
        }
    }

    /// The number, as C sees it.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

/// Which platform control a §12.7 field is, as `viewer_host::ControlKind` decided.
///
/// **Answered, so it is named and counted** — and it comes from `viewer-host` rather than from
/// `pdf_model::form` for the reason ADR 0246 gives: one variant per control a toolkit has for the
/// job, rather than one per §12.7.5 type, because the clause's choice field is two controls and its
/// button field is three. A C caller is a native host, so it takes that decision unchanged instead
/// of splitting the clause's taxonomy a fourth time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum ControlKind {
    /// §12.7.5.3's text field.
    Entry = 0,
    /// §12.7.5.2.3's check box.
    Check = 1,
    /// §12.7.5.2.4's radio button.
    Radio = 2,
    /// §12.7.5.2.2's push button, which holds no value.
    Push = 3,
    /// §12.7.5.4's combo box — Table 233 bit 18 set.
    Combo = 4,
    /// §12.7.5.4's list box — bit 18 clear.
    List = 5,
    /// §12.7.5.5's signature field.
    Signature = 6,
    /// Table 226 makes `/FT` required and this field states none anywhere in its ancestry.
    Unstated = 7,
}

impl ControlKind {
    /// How many kinds this build has.
    ///
    /// Not part of `pdfv_abi_check`, and the module comment says why: a control kind is an answer
    /// to a question the caller asked, where an event kind arrives unbidden.
    pub const COUNT: u32 = 8;

    /// Which kind a control is. Exhaustive over `viewer_host::ControlKind` with no catch-all.
    #[must_use]
    pub const fn of(control: &HostControl) -> Self {
        match control {
            HostControl::Entry { .. } => Self::Entry,
            HostControl::Check { .. } => Self::Check,
            HostControl::Radio { .. } => Self::Radio,
            HostControl::Push => Self::Push,
            HostControl::Combo { .. } => Self::Combo,
            HostControl::List { .. } => Self::List,
            HostControl::Signature => Self::Signature,
            HostControl::Unstated => Self::Unstated,
        }
    }

    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Entry,
            1 => Self::Check,
            2 => Self::Radio,
            3 => Self::Push,
            4 => Self::Combo,
            5 => Self::List,
            6 => Self::Signature,
            7 => Self::Unstated,
            _ => return None,
        })
    }

    /// The name, NUL-terminated for `const char *`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Entry => "Entry\0",
            Self::Check => "Check\0",
            Self::Radio => "Radio\0",
            Self::Push => "Push\0",
            Self::Combo => "Combo\0",
            Self::List => "List\0",
            Self::Signature => "Signature\0",
            Self::Unstated => "Unstated\0",
        }
    }

    /// The number, as C sees it.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

/// What acting on a panel row does, as `viewer_host::RowAction` decided.
///
/// Answered, so it is named. The *payload* is read with a second accessor, because the four
/// actions carry four different things and a union of them would be a struct passed by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum RowKind {
    /// §12.3.3: `pdfv_activate` on the row's object.
    Activate = 0,
    /// §8.11.4.3: `pdfv_set_group` on the row's object.
    Toggle = 1,
    /// §7.11.4: `pdfv_extract` on the row's name.
    Extract = 2,
    /// A row that does nothing — §8.11.4.3's leading text string is a heading, not a layer.
    Inert = 3,
}

impl RowKind {
    /// How many kinds this build has.
    pub const COUNT: u32 = 4;

    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Activate,
            1 => Self::Toggle,
            2 => Self::Extract,
            3 => Self::Inert,
            _ => return None,
        })
    }

    /// The name, NUL-terminated for `const char *`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Activate => "Activate\0",
            Self::Toggle => "Toggle\0",
            Self::Extract => "Extract\0",
            Self::Inert => "Inert\0",
        }
    }

    /// The number, as C sees it.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

/// Which of the strings a field, a widget or an option carries a caller is asking for.
///
/// One `which` argument rather than one function per string, because they are all the same
/// two-call idiom over the same handle and index, and a function apiece would be six symbols
/// saying one thing. A number this build does not define is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum TextKind {
    /// §12.7.4.2's fully qualified name, which `pdfv_set_field_text` addresses.
    Qualified = 0,
    /// The name §14.9.3 says a user interface shall show: Table 226's `/TU`, or the qualified
    /// name where the field states none.
    Shown = 1,
    /// Table 226's `/T`, the partial name.
    Partial = 2,
    /// What is displayed: Table 234's option label, or a widget's `/AP /N` on-state name.
    Label = 3,
    /// What §12.7.6.2 would export: Table 234's export value, or Table 230's `/Opt` entry.
    Export = 4,
}

impl TextKind {
    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Qualified,
            1 => Self::Shown,
            2 => Self::Partial,
            3 => Self::Label,
            4 => Self::Export,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ControlKind, EventKind, FocusKind, MarkupKind, PageTargetKind, PixelFormat, PointerKind,
        RowKind, TextKind, ZoomKind,
    };

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

    /// The enumerations added in the five-hundred-and-eleventh round round-trip too.
    ///
    /// The same property `the_argument_enumerations_answer_to_their_own_numbers` asserts, extended
    /// to the ones that arrived with the pointer, the form and the two panels: a number is the ABI,
    /// so a `from_code` that disagreed with a discriminant would be a caller acting on the wrong
    /// variant with nothing to say so.
    #[test]
    fn the_enumerations_that_came_with_the_form_and_the_pointer_round_trip() {
        for code in 0..4 {
            assert_eq!(
                PointerKind::from_code(code).map(|kind| kind as u32),
                Some(code)
            );
            assert_eq!(
                MarkupKind::from_code(code).map(|kind| kind as u32),
                Some(code)
            );
            assert_eq!(RowKind::from_code(code).map(RowKind::code), Some(code));
        }
        assert!(PointerKind::from_code(4).is_none());
        assert!(MarkupKind::from_code(4).is_none());
        assert!(FocusKind::from_code(3).is_none());
        assert!(TextKind::from_code(5).is_none());
        for code in 0..ControlKind::COUNT {
            let kind = ControlKind::from_code(code).expect("below the count");
            assert_eq!(kind.code(), code);
            assert_eq!(kind.name().matches('\0').count(), 1);
        }
        assert!(ControlKind::from_code(ControlKind::COUNT).is_none());
        for code in 0..RowKind::COUNT {
            let kind = RowKind::from_code(code).expect("below the count");
            assert_eq!(kind.name().matches('\0').count(), 1);
        }
        assert!(RowKind::from_code(RowKind::COUNT).is_none());
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
