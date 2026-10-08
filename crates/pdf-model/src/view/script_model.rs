//! What crosses from a view state to a script runner and back: where a script runs, the event it
//! is handed, the fields a document's realm holds, and the edits a script makes.
//!
//! RFC 0008 section 6.1 makes the runner's surface data rather than a callback, so that the engine
//! can sit in a confined process of its own (section 6.2) and a run can be recorded and replayed.
//! Section 6.4 says what a script's change is: an edit in the same log a person's typing lands in,
//! never a change to `pdf_syntax::Document` (ADRs 1602, 1603). The field properties and their
//! spellings are Adobe's *JavaScript for Acrobat API Reference*, "Field properties", read at the
//! commit `crates/pdf-script`'s root names — documented choices under principle 5, never
//! derivations, since ISO 32000-2 hands the object model to ISO 21757-1 and the owner settled the
//! reference as the working source (`doc/todo/56`).

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use super::ViewState;
use crate::action::{PageTrigger, Trigger as AnnotationTrigger};
use crate::aform::Trigger;

/// Where a script runs: which of §12.6.3's tables fired it, or the document's open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptSite {
    /// Table 199: a field's keystroke, format, validate or calculate.
    Field(Trigger),
    /// Table 197: one of an annotation's ten events.
    Annotation(AnnotationTrigger),
    /// Table 198: a page opened or closed.
    Page(PageTrigger),
    /// Table 29's `/OpenAction`, where it is an ECMAScript action.
    OpenAction,
    /// One entry of Table 32's `/JavaScript` name tree, which §12.6.4.17 has executed "[w]hen the
    /// document is opened".
    Library,
    /// Table 200: the document as a whole about to close, or around a save or a print.
    Document(DocumentTrigger),
    /// The expression an earlier script handed `app.setInterval` or `app.setTimeOut`, run when
    /// its period has elapsed on the host's ticks (ADR 1702).
    Timer,
}

/// One of Table 200's five events of the document as a whole, each an entry of the catalog's
/// `/AA`.
///
/// Every row is a moment rather than a question: `/WC` is "(Optional; PDF 1.4) An ECMAScript
/// action that shall be performed before closing a document", and the other four say the same of
/// a save and a print, before and after. So a script these fire is told of the operation and has no
/// say in it — `event.rc` is read back, reported where it is false, and never obeyed (ADR 1614).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentTrigger {
    /// `/WC`, "will close".
    WillClose,
    /// `/WS`, "will save".
    WillSave,
    /// `/DS`, "did save".
    DidSave,
    /// `/WP`, "will print".
    WillPrint,
    /// `/DP`, "did print".
    DidPrint,
}

impl DocumentTrigger {
    /// The five, in Table 200's order.
    pub const ALL: [Self; 5] = [
        Self::WillClose,
        Self::WillSave,
        Self::DidSave,
        Self::WillPrint,
        Self::DidPrint,
    ];

    /// The key of the catalog's additional-actions dictionary that states this trigger's action.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::WillClose => "WC",
            Self::WillSave => "WS",
            Self::DidSave => "DS",
            Self::WillPrint => "WP",
            Self::DidPrint => "DP",
        }
    }

    /// The operation the trigger is a moment of: `close`, `save` or `print`.
    #[must_use]
    pub fn operation(self) -> &'static str {
        match self {
            Self::WillClose => "close",
            Self::WillSave | Self::DidSave => "save",
            Self::WillPrint | Self::DidPrint => "print",
        }
    }

    /// Whether the trigger comes before its operation rather than after it.
    #[must_use]
    pub fn before(self) -> bool {
        matches!(self, Self::WillClose | Self::WillSave | Self::WillPrint)
    }

    /// Adobe's `event.name` at this trigger, which the reference's "Event type/name combinations"
    /// page pairs with the type `Doc` (ADR 1614).
    #[must_use]
    pub fn adobe(self) -> &'static str {
        match self {
            Self::WillClose => "WillClose",
            Self::WillSave => "WillSave",
            Self::DidSave => "DidSave",
            Self::WillPrint => "WillPrint",
            Self::DidPrint => "DidPrint",
        }
    }
}

/// How a person committed what they typed into a field: Adobe's `event.commitKey`.
///
/// The reference's "event properties" page numbers four ways a field loses the keyboard; the three
/// here commit the value, and its `0` — a value not committed, an Escape — is what every event
/// that is not a commit carries, so it is `None` where this is carried rather than a fourth variant
/// (ADR 1626). A documented choice under principle 5: the object model is ISO 21757-1's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitKey {
    /// `1`: a click outside the field.
    Click,
    /// `2`: the Enter key.
    Enter,
    /// `3`: a Tab to another field.
    Tab,
}

impl CommitKey {
    /// The three, in the reference's order.
    pub const ALL: [Self; 3] = [Self::Click, Self::Enter, Self::Tab];

    /// The reference's number for this way of committing.
    #[must_use]
    pub fn number(self) -> u8 {
        match self {
            Self::Click => 1,
            Self::Enter => 2,
            Self::Tab => 3,
        }
    }
}

/// One of a push-button's three captions, which Table 192 keeps in its widget's `/MK`.
///
/// Adobe's `nFace` numbers them 0, 1 and 2 on the "Field methods" page's `buttonSetCaption`; the
/// entries are the standard's: `/CA` "[t]he widget annotation's normal caption", `/AC` its
/// "alternate (down) caption", `/RC` its "rollover caption".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// `nFace` 0, Table 192's `/CA`.
    Normal,
    /// `nFace` 1, Table 192's `/AC`.
    Down,
    /// `nFace` 2, Table 192's `/RC`.
    Rollover,
}

impl Face {
    /// The three, in `nFace` order.
    pub const ALL: [Self; 3] = [Self::Normal, Self::Down, Self::Rollover];

    /// Adobe's `nFace` for this caption.
    #[must_use]
    pub fn number(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Down => 1,
            Self::Rollover => 2,
        }
    }

    /// The face `nFace` names, `None` for any other number.
    #[must_use]
    pub fn from_number(number: f64) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|face| (f64::from(face.number()) - number).abs() < f64::EPSILON)
    }

    /// Table 192's key for this caption.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Normal => "CA",
            Self::Down => "AC",
            Self::Rollover => "RC",
        }
    }

    /// This face's place in [`WidgetState::captions`].
    #[must_use]
    pub fn index(self) -> usize {
        usize::from(self.number())
    }
}

/// One of the six glyphs Adobe's `Field.style` names for a check box or a radio button.
///
/// The "Field properties" page names them — check, cross, diamond, circle, star, square — and
/// draws none. ISO 32000-2 draws a toggling button's caption from Table 192's `/CA` in the font
/// Table 228's `/DA` selects, and Annex D's Table D.6 is `ZapfDingbats`' built-in encoding, so each
/// style is the code of the glyph of that shape in Table D.6 — the solid one where the table has
/// several: a documented choice, since neither source pairs a name with a code (ADR 1665).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// `style.ch`: Table D.6's `a20`, ✔, at octal 064.
    Check,
    /// `style.cr`: `a24`, ✘, at octal 070.
    Cross,
    /// `style.di`: `a78`, ◆, at octal 165.
    Diamond,
    /// `style.ci`: `a71`, ●, at octal 154.
    Circle,
    /// `style.st`: `a35`, ★, at octal 110.
    Star,
    /// `style.sq`: `a73`, ■, at octal 156.
    Square,
}

impl Glyph {
    /// The six, in the order the reference's table lists them.
    pub const ALL: [Self; 6] = [
        Self::Check,
        Self::Cross,
        Self::Diamond,
        Self::Circle,
        Self::Star,
        Self::Square,
    ];

    /// The style's name as the reference's table writes it, which is the value of its constant.
    #[must_use]
    pub fn adobe(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Cross => "cross",
            Self::Diamond => "diamond",
            Self::Circle => "circle",
            Self::Star => "star",
            Self::Square => "square",
        }
    }

    /// The style a name of [`Self::adobe`]'s names.
    #[must_use]
    pub fn from_adobe(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|glyph| glyph.adobe() == name)
    }

    /// The caption that draws it: the one-byte code Table D.6 gives the glyph, as the character
    /// of that code, which is what a `/CA` text string of one byte holds.
    #[must_use]
    pub fn caption(self) -> char {
        match self {
            Self::Check => '\u{34}',
            Self::Cross => '\u{38}',
            Self::Diamond => '\u{75}',
            Self::Circle => '\u{6c}',
            Self::Star => '\u{48}',
            Self::Square => '\u{6e}',
        }
    }

    /// The style a widget's normal caption draws, where it is one of the six.
    #[must_use]
    pub fn of_caption(caption: &str) -> Option<Self> {
        let mut characters = caption.chars();
        let first = characters.next()?;
        if characters.next().is_some() {
            return None;
        }
        Self::ALL.into_iter().find(|glyph| glyph.caption() == first)
    }
}

/// What a document's realm is told of the document as a whole, beside its fields: Table 349's
/// information dictionary and §8.11's groups (ADR 1626).
///
/// Told at the first event a runner is handed and again whenever a group's state has changed since
/// — the information dictionary is the file's and does not change while it is open.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocumentState {
    /// Every entry of the trailer's `/Info` a script reads as `this.info`, in the dictionary's
    /// order.
    pub info: Vec<InfoEntry>,
    /// Every optional content group Table 98's `/OCGs` lists, in that order.
    pub layers: Vec<Layer>,
    /// Every annotation a script's `getAnnots` answers, in page order and then in the order of
    /// each page's `/Annots` (ADR 1700).
    pub annotations: Vec<AnnotationState>,
    /// Every page in page order, as `getPageLabel`, `getPageBox` and `getPageRotation` read it
    /// (ADR 1724); at most [`MAX_PAGES`].
    pub pages: Vec<PageState>,
}

/// One page as a document's realm holds it: what Adobe's `getPageLabel`, `getPageBox` and
/// `getPageRotation` read, each from the entry ISO 32000-2 states for it (ADR 1724).
#[derive(Debug, Clone, PartialEq)]
pub struct PageState {
    /// §12.4.2's label for the page, `None` where the document's `/PageLabels` labels it not.
    pub label: Option<String>,
    /// Table 31's five boundaries in [`PageState::BOXES`] order — `/MediaBox`, `/CropBox`,
    /// `/BleedBox`, `/TrimBox`, `/ArtBox` — each after §14.11.2.1's defaults and its intersection
    /// with the media box, as `[x0, y0, x1, y1]` in default user space.
    pub boxes: [[f64; 4]; 5],
    /// Table 31's `/Rotate`, inherited and normalised to 0, 90, 180 or 270.
    pub rotate: u16,
    /// The page's words as `getPageNumWords` counts them and `getPageNthWord` answers them, in
    /// the order the content stream shows them: `None` where they were not read — no script of
    /// the document has spelled either member, or the page lies past what the reading reached
    /// (ADR 1762).
    pub words: Option<Vec<String>>,
}

impl PageState {
    /// The boundaries [`Self::boxes`] holds, in its order.
    pub const BOXES: [crate::page::Boundary; 5] = [
        crate::page::Boundary::Media,
        crate::page::Boundary::Crop,
        crate::page::Boundary::Bleed,
        crate::page::Boundary::Trim,
        crate::page::Boundary::Art,
    ];

    /// The page as Table 31 states it: its five boundaries and its rotation, with §12.4.2's label.
    #[must_use]
    pub fn of(page: &crate::page::Page, label: Option<String>) -> Self {
        Self {
            label,
            boxes: Self::BOXES.map(|boundary| page.boundary(boundary).map(f64::from)),
            rotate: page.rotate,
            words: None,
        }
    }
}

/// Most pages a realm is told of, and most of one field's `/Opt` entries: half the wire's count, as
/// for the annotations (ADR 1700). A page past it answers `getPageBox` and its two siblings with a
/// refusal naming the bound (ADR 1724), and an option past it is no item `numItems` counts (ADR
/// 1725).
pub const MAX_PAGES: usize = 1 << 15;

/// One entry of Table 349's document information dictionary, as `this.info` reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoEntry {
    /// The entry's key, as the file spells it.
    pub key: String,
    /// The entry's value as text: §7.9.2.2's text string, or a name's characters for `/Trapped`.
    pub text: String,
    /// For a value that is §7.9.4's date, the moment it names in milliseconds since
    /// 1970-01-01T00:00:00Z — Adobe's "Doc properties" page answers `CreationDate` and `ModDate`
    /// with a `Date`.
    pub moment: Option<i64>,
}

/// One annotation as a document's realm holds it: what Adobe's `Annotation` object reads, each
/// property from the entry ISO 32000-2 §12.5.2 or §12.5.6.2 states for it (ADR 1700).
///
/// The property names are Adobe's *JavaScript for Acrobat API Reference*, "Annotation
/// properties", read at the commit `crates/pdf-script`'s root names — documented choices under
/// principle 5. Every value is what the reader is shown now: the file's entry, with what a person,
/// an action or a script changed since laid over it.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationState {
    /// The annotation dictionary's object number, which names it in a [`ScriptEdit::Annotation`].
    pub number: u32,
    /// Its generation number.
    pub generation: u16,
    /// The zero-based page whose `/Annots` lists it: Adobe's `page`.
    pub page: u32,
    /// Table 166's `/Subtype`, one of the seventeen Adobe's "Annotation types" lists: `type`.
    pub kind: String,
    /// Table 166's `/Rect`, normalised as §7.9.5 reads a rectangle: `rect`.
    pub rect: [f64; 4],
    /// Table 166's `/NM`, where the file states one: `name`.
    pub name: Option<String>,
    /// Table 166's `/Contents`, or what a person retyped: `contents`. Empty where neither says
    /// anything.
    pub contents: String,
    /// Table 172's `/T`, the markup annotation's author, where the file states one: `author`.
    pub author: Option<String>,
    /// Table 166's `/M` as §7.9.4's date, in milliseconds since 1970-01-01T00:00:00Z, where it
    /// parses as one: `modDate`.
    pub modified: Option<i64>,
    /// Table 167's `Hidden` bit, as the reader sees it now: `hidden`.
    pub hidden: bool,
    /// Table 167's `ReadOnly` bit: `readOnly`.
    pub read_only: bool,
    /// Where §12.5.3's other flags let it go, `Hidden` set aside: what `getAnnots`'s three flag
    /// filters read beside [`Self::hidden`] (ADR 1721).
    pub reach: AnnotationReach,
    /// Whether the annotation's popup window opens with the page — Table 186's `/Open`, or Table
    /// 175's on a text annotation, which is the whole statement for one with no popup — or `None`
    /// for the three subtypes Adobe's `popupOpen` is not a property of and for an annotation that
    /// has no window to open (ADR 1700).
    pub popup_open: Option<bool>,
}

/// Where Table 167's flags let an annotation go, with its `Hidden` bit set aside — the bit a hide
/// action and a script change, which suppresses it on paper, on a screen and for a pointer alike,
/// and which [`AnnotationState::hidden`] carries beside this (ADR 1721).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AnnotationReach {
    /// A printed page: bit 3's three sentences, as `crate::annotation` reads them for paper.
    /// What `ANFB_ShouldPrint` keeps.
    pub printed: bool,
    /// A screen, with the pointer away from the annotation or on it: `NoView`, as `ToggleNoView`
    /// inverts it under the pointer. What `ANFB_ShouldView` keeps.
    pub viewed: bool,
    /// A pointer: `NoView` and `ReadOnly`, as `crate::annotation` reads them for interaction.
    /// What `ANFB_ShouldEdit` keeps.
    pub interactive: bool,
}

/// What a script set on one annotation: the three of Adobe's `Annotation` properties a reader's
/// own edit already reaches (ADR 1700).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotationChange {
    /// `hidden`: Table 167's `Hidden` bit, set or cleared as §12.6.4.11's hide sets or clears it.
    Hidden(bool),
    /// `popupOpen`: whether the annotation's window opens with the page.
    PopupOpen(bool),
    /// `contents`: Table 166's `/Contents`, as a person's retyping of a free text annotation or a
    /// text note.
    Contents(String),
}

/// One optional content group as a document's realm holds it: what Adobe's `OCG` object reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    /// The group dictionary's object number, which names it in a [`ScriptEdit::Layer`].
    pub number: u32,
    /// Its generation number.
    pub generation: u16,
    /// Table 96's `/Name`, which a layer panel shows.
    pub name: String,
    /// Whether the group is on now.
    pub on: bool,
    /// Whether the default configuration turns it on when the document opens: Adobe's
    /// `initState`.
    pub initially_on: bool,
    /// Whether Table 99's `/Locked` names it, so that no person's switch changes it.
    pub locked: bool,
    /// Table 96's `/Intent`, each name's characters in the order the entry lists them: what
    /// `OCG.getIntent` answers (ADR 1762). Table 96 makes the default `View`, so a group stating
    /// none, or stating a value that is neither a name nor an array of names, holds `["View"]`.
    pub intent: Vec<String>,
}

impl ScriptSite {
    /// Adobe's `event.type` and `event.name` for this site.
    ///
    /// The reference's "event object" page lists each pair; a widget's events are `Field`'s, a
    /// link's mouse-up is `Link`'s, and every other annotation's are `Screen`'s, whose names the
    /// reference gives for its page and visibility events too. A documented choice (ADR 1602).
    #[must_use]
    pub fn event_names(self, widget: bool) -> (&'static str, &'static str) {
        match self {
            Self::Field(trigger) => (
                "Field",
                match trigger {
                    Trigger::Keystroke => "Keystroke",
                    Trigger::Format => "Format",
                    Trigger::Validate => "Validate",
                    Trigger::Calculate => "Calculate",
                },
            ),
            Self::Annotation(trigger) => {
                let name = match trigger {
                    AnnotationTrigger::Enter => "Mouse Enter",
                    AnnotationTrigger::Exit => "Mouse Exit",
                    AnnotationTrigger::Down => "Mouse Down",
                    AnnotationTrigger::Up => "Mouse Up",
                    AnnotationTrigger::Focus => "Focus",
                    AnnotationTrigger::Blur => "Blur",
                    AnnotationTrigger::PageOpen => "Open",
                    AnnotationTrigger::PageClose => "Close",
                    AnnotationTrigger::PageVisible => "InView",
                    AnnotationTrigger::PageInvisible => "OutView",
                };
                let kind = if widget {
                    "Field"
                } else if trigger == AnnotationTrigger::Up {
                    "Link"
                } else {
                    "Screen"
                };
                (kind, name)
            }
            Self::Page(PageTrigger::Open) => ("Page", "Open"),
            Self::Page(PageTrigger::Close) => ("Page", "Close"),
            Self::OpenAction | Self::Library => ("Doc", "Open"),
            Self::Document(trigger) => ("Doc", trigger.adobe()),
            // The reference's "Event type/name combinations" page lists no event for a timer's
            // expression, so the pair is a documented choice: `App`, the type of the one event the
            // application raises, and a name the list does not hold, so that a script branching on
            // a listed event takes no branch (ADR 1702).
            Self::Timer => ("App", "Timer"),
        }
    }
}

/// Adobe's `Field.type`, from §12.7.4.1's `/FT` and the button flags of Table 229.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// `Tx`.
    Text,
    /// `Btn` with Table 229 bit 17, `Pushbutton`.
    PushButton,
    /// `Btn` with neither button bit.
    CheckBox,
    /// `Btn` with Table 229 bit 16, `Radio`.
    RadioButton,
    /// `Ch` with Table 233 bit 18, `Combo`.
    ComboBox,
    /// `Ch` without it.
    ListBox,
    /// `Sig`.
    Signature,
}

impl FieldType {
    /// The reference's spelling.
    #[must_use]
    pub fn adobe(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::PushButton => "button",
            Self::CheckBox => "checkbox",
            Self::RadioButton => "radiobutton",
            Self::ComboBox => "combobox",
            Self::ListBox => "listbox",
            Self::Signature => "signature",
        }
    }
}

/// Adobe's `display` constants, over Table 167's `Hidden`, `Print` and `NoView` bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    /// `display.visible`, 0: shown and printed.
    Visible,
    /// `display.hidden`, 1: Table 167's `Hidden`.
    Hidden,
    /// `display.noPrint`, 2: shown, `Print` clear.
    NoPrint,
    /// `display.noView`, 3: Table 167's `NoView`, printed.
    NoView,
}

impl Display {
    /// The constant's number.
    #[must_use]
    pub fn number(self) -> u8 {
        match self {
            Self::Visible => 0,
            Self::Hidden => 1,
            Self::NoPrint => 2,
            Self::NoView => 3,
        }
    }

    /// The constant a number names, `None` for any other.
    #[must_use]
    pub fn from_number(number: f64) -> Option<Self> {
        [Self::Visible, Self::Hidden, Self::NoPrint, Self::NoView]
            .into_iter()
            .find(|display| (f64::from(display.number()) - number).abs() < f64::EPSILON)
    }

    /// Whether a screen shows the field: §12.5.3's `Hidden` and `NoView` each say it does not.
    #[must_use]
    pub fn on_screen(self) -> bool {
        matches!(self, Self::Visible | Self::NoPrint)
    }

    /// The display Table 167's flags describe.
    fn of_flags(flags: i64) -> Self {
        const HIDDEN: i64 = 1 << 1;
        const PRINT: i64 = 1 << 2;
        const NO_VIEW: i64 = 1 << 5;
        if flags & HIDDEN != 0 {
            Self::Hidden
        } else if flags & NO_VIEW != 0 {
            Self::NoView
        } else if flags & PRINT == 0 {
            Self::NoPrint
        } else {
            Self::Visible
        }
    }
}

/// An Adobe colour array: `["T"]`, `["G", g]`, `["RGB", r, g, b]`, `["CMYK", c, m, y, k]`.
///
/// The four shapes are §8.6.4's three device families and the transparent "no colour" Table 192's
/// `/BG` and `/BC` express by an empty array.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Colour {
    /// No colour: an empty array.
    Transparent,
    /// `DeviceGray`.
    Gray(f64),
    /// `DeviceRGB`.
    Rgb([f64; 3]),
    /// `DeviceCMYK`.
    Cmyk([f64; 4]),
}

impl Colour {
    /// The colour a PDF array of components states: Table 192's reading, by component count.
    fn of_components(components: &[f64]) -> Option<Self> {
        Some(match components {
            [] => Self::Transparent,
            [gray] => Self::Gray(*gray),
            [red, green, blue] => Self::Rgb([*red, *green, *blue]),
            [cyan, magenta, yellow, black] => Self::Cmyk([*cyan, *magenta, *yellow, *black]),
            _ => return None,
        })
    }
}

/// Adobe's `border` constants, over Table 168's `/S`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    /// `S`, `border.s`.
    Solid,
    /// `D`, `border.d`.
    Dashed,
    /// `B`, `border.b`.
    Beveled,
    /// `I`, `border.i`.
    Inset,
    /// `U`, `border.u`.
    Underline,
}

impl BorderStyle {
    /// The reference's spelling, which is also what the `border` constants hold.
    #[must_use]
    pub fn adobe(self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Dashed => "dashed",
            Self::Beveled => "beveled",
            Self::Inset => "inset",
            Self::Underline => "underline",
        }
    }

    /// The style the reference spells `text`, `None` for any other.
    #[must_use]
    pub fn from_adobe(text: &str) -> Option<Self> {
        [
            Self::Solid,
            Self::Dashed,
            Self::Beveled,
            Self::Inset,
            Self::Underline,
        ]
        .into_iter()
        .find(|style| style.adobe() == text)
    }
}

/// Adobe's `Field.alignment`, over Table 228's `/Q`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// `/Q 0`.
    Left,
    /// `/Q 1`.
    Center,
    /// `/Q 2`.
    Right,
}

impl Alignment {
    /// The reference's spelling.
    #[must_use]
    pub fn adobe(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }

    /// The alignment the reference spells `text`, `None` for any other.
    #[must_use]
    pub fn from_adobe(text: &str) -> Option<Self> {
        [Self::Left, Self::Center, Self::Right]
            .into_iter()
            .find(|alignment| alignment.adobe() == text)
    }
}

/// One of the text field flags of Table 231 a script reads and writes by Adobe's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFlag {
    /// `multiline`: Table 231 bit 13, `Multiline`.
    Multiline,
    /// `password`: bit 14, `Password`.
    Password,
    /// `doNotScroll`: bit 24, `DoNotScroll`.
    DoNotScroll,
    /// `comb`: bit 25, `Comb`.
    Comb,
}

impl TextFlag {
    /// The four, in the order the reference's "Field properties" page lists them.
    pub const ALL: [Self; 4] = [
        Self::Multiline,
        Self::Password,
        Self::DoNotScroll,
        Self::Comb,
    ];

    /// The flag's bit in Table 227's `/Ff`, where Table 231 numbers it.
    #[must_use]
    pub fn bit(self) -> u32 {
        match self {
            Self::Multiline => 1 << 12,
            Self::Password => 1 << 13,
            Self::DoNotScroll => 1 << 23,
            Self::Comb => 1 << 24,
        }
    }

    /// The reference's spelling.
    #[must_use]
    pub fn adobe(self) -> &'static str {
        match self {
            Self::Multiline => "multiline",
            Self::Password => "password",
            Self::DoNotScroll => "doNotScroll",
            Self::Comb => "comb",
        }
    }
}

/// One field as a document's realm holds it: what `Field`'s properties read.
///
/// Split the way Adobe's *JavaScript for Acrobat API Reference*, "Field versus widget attributes",
/// splits `Field`'s members, and the way ISO 32000-2 splits the dictionaries: §12.7.4.2 gives a
/// field one value however many widgets show it, and §12.5.6.19 gives each widget its own
/// presentation. So the members here are the field's, one per field, and [`Self::widgets`] holds
/// what each widget shows (ADR 1664).
#[derive(Debug, Clone, PartialEq)]
pub struct FieldState {
    /// §12.7.4.2's fully qualified name.
    pub name: String,
    /// `Field.type`.
    pub kind: FieldType,
    /// `Field.value`, as the text a script reads (the view's own reading of Table 226's `/V`).
    pub value: String,
    /// Table 227's `/Ff`, inherited.
    pub flags: u32,
    /// `Field.charLimit`: Table 230's `/MaxLen`.
    pub char_limit: Option<u32>,
    /// `Field.page`: the zero-based page Table 166's `/P` names, where the first widget states one.
    pub page: Option<u32>,
    /// Each widget's own members, in the order the view state's field table lists the widgets —
    /// the order of §12.7.4.1's `/Kids` — which is the index `getField("name.N")` counts from
    /// zero. Empty only for a state no view state built.
    pub widgets: Vec<WidgetState>,
    /// The field's inherited `/Opt`, each entry in both of its forms: Table 234's options for a
    /// choice field, what `numItems` counts and `getItemAt` reads, and Table 230's export values
    /// for a check box or a radio button, one text string per widget, which `exportValues` reads
    /// (ADR 1725). Empty where the field states none.
    pub options: Vec<crate::form::Choice>,
    /// The zero-based indices into [`Self::options`] of the items selected now, ascending, read as
    /// §12.7.5.4 reads `/V` and `/I`: what `currentValueIndices` answers (ADR 1725). Empty for a
    /// field that is not a choice field and for one with nothing selected.
    pub selected: Vec<u32>,
}

impl FieldState {
    /// A property a script set, over this state: a field-level member on the field, a
    /// widget-level one on `widget`, or on every widget where it names none (ADR 1664).
    pub fn apply(&mut self, widget: Option<u32>, property: &Property) {
        match property {
            Property::ReadOnly(flag) => self.flags = set_bit(self.flags, READ_ONLY, *flag),
            Property::Required(flag) => self.flags = set_bit(self.flags, REQUIRED, *flag),
            Property::CharLimit(limit) => self.char_limit = Some(*limit),
            Property::TextFlag(flag, on) => self.flags = set_bit(self.flags, flag.bit(), *on),
            Property::Options(options) => self.options.clone_from(options),
            _ => {
                for (index, held) in self.widgets.iter_mut().enumerate() {
                    if widget.is_none_or(|widget| u32::try_from(index).is_ok_and(|i| i == widget)) {
                        held.apply(property);
                    }
                }
            }
        }
    }

    /// The widget a member reads: the one `widget` names, or the first where it names none — the
    /// reference's rule for a `Field` that stands for every widget of its field.
    #[must_use]
    pub fn widget(&self, widget: Option<u32>) -> Option<&WidgetState> {
        self.widgets
            .get(widget.map_or(Some(0), |index| usize::try_from(index).ok())?)
    }
}

/// One widget of a field as its realm holds it: the members Adobe's reference makes a widget's.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetState {
    /// `Field.display`, from the widget's Table 167 flags and what a hide or a script set.
    pub display: Display,
    /// `Field.textColor`: the colour operator of Table 228's `/DA`, read up from the widget.
    pub text_color: Option<Colour>,
    /// `Field.fillColor`: Table 192's `/BG`.
    pub fill_color: Option<Colour>,
    /// `Field.strokeColor`: Table 192's `/BC`.
    pub stroke_color: Option<Colour>,
    /// `Field.borderStyle`: Table 168's `/S`.
    pub border_style: BorderStyle,
    /// `Field.alignment`: Table 228's `/Q`, read up from the widget.
    pub alignment: Alignment,
    /// `Field.lineWidth`: Table 168's `/W` of the widget's `/BS`, in points; Table 166's `/Border`
    /// third element where the widget states no `/BS` (§12.5.4), and 1 — both tables' default —
    /// where it states neither.
    pub line_width: f64,
    /// `Field.textSize`: the size operand of the `Tf` in Table 228's `/DA`, read up from the
    /// widget; 0 is §12.7.4.3's auto-size. `None` where the `/DA` holds no `Tf`.
    pub text_size: Option<f64>,
    /// `Field.textFont`: the font the `/DA`'s `Tf` names — the `/BaseFont` of the resource it
    /// names in the form's `/DR` where that resolves, the resource name otherwise; empty where
    /// the `/DA` holds no `Tf` (ADR 1762).
    pub text_font: String,
    /// `Field.rect`: Table 166's `/Rect`.
    pub rect: [f64; 4],
    /// `Field.buttonGetCaption`'s three captions, in [`Face`] order: Table 192's `/CA`, `/AC` and
    /// `/RC` of the widget's `/MK`, each empty where it states none.
    pub captions: [String; 3],
    /// The name §12.7.5.2.3's on state is selected by, for a check box's or a radio button's
    /// widget: what `Field.isBoxChecked` compares the field's value with and `checkThisBox`
    /// sets it to. `None` for any other widget, and where the file names no single on state
    /// (ADR 1689).
    pub on_state: Option<String>,
}

impl WidgetState {
    /// A widget-level property, over this widget's state; a field-level one changes nothing here.
    pub fn apply(&mut self, property: &Property) {
        match property {
            Property::Display(display) => self.display = *display,
            Property::TextColor(colour) => self.text_color = Some(*colour),
            Property::FillColor(colour) => self.fill_color = Some(*colour),
            Property::StrokeColor(colour) => self.stroke_color = Some(*colour),
            Property::BorderStyle(style) => self.border_style = *style,
            Property::Alignment(alignment) => self.alignment = *alignment,
            Property::LineWidth(width) => self.line_width = *width,
            Property::TextSize(size) => self.text_size = Some(*size),
            Property::TextFont(font) => self.text_font.clone_from(&font.base),
            Property::Caption(face, caption) => {
                if let Some(slot) = self.captions.get_mut(face.index()) {
                    slot.clone_from(caption);
                }
            }
            Property::Style(glyph) => {
                if let Some(slot) = self.captions.get_mut(Face::Normal.index()) {
                    *slot = glyph.caption().to_string();
                }
            }
            Property::ReadOnly(_)
            | Property::Required(_)
            | Property::CharLimit(_)
            | Property::TextFlag(..)
            | Property::Options(_) => {}
        }
    }
}

impl Default for WidgetState {
    /// A widget stating nothing: visible, no colours, a solid border, left-aligned, no rectangle.
    fn default() -> Self {
        Self {
            display: Display::Visible,
            text_color: None,
            fill_color: None,
            stroke_color: None,
            border_style: BorderStyle::Solid,
            alignment: Alignment::Left,
            line_width: 1.0,
            text_size: None,
            text_font: String::new(),
            rect: [0.0; 4],
            captions: Default::default(),
            on_state: None,
        }
    }
}

/// A property a script set on a field.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// `display` (and the older `hidden`).
    Display(Display),
    /// `readonly`: Table 227 bit 1.
    ReadOnly(bool),
    /// `required`: Table 227 bit 2.
    Required(bool),
    /// `textColor`.
    TextColor(Colour),
    /// `fillColor`.
    FillColor(Colour),
    /// `strokeColor`.
    StrokeColor(Colour),
    /// `borderStyle`.
    BorderStyle(BorderStyle),
    /// `alignment`.
    Alignment(Alignment),
    /// `lineWidth`: Table 168's `/W`, in points (ADR 1762).
    LineWidth(f64),
    /// `textSize`: the `Tf` size of Table 228's `/DA`, 0 being §12.7.4.3's auto-size (ADR 1762).
    TextSize(f64),
    /// `textFont`: the `Tf` font of Table 228's `/DA`, a resource of the form's `/DR` (ADR 1762).
    TextFont(FontName),
    /// `charLimit`.
    CharLimit(u32),
    /// One of Table 231's text field flags set or cleared.
    TextFlag(TextFlag, bool),
    /// `buttonSetCaption(cCaption, nFace)`: one of Table 192's three captions (ADR 1626).
    Caption(Face, String),
    /// `style`: a check box's or a radio button's glyph, Table 192's `/CA` (ADR 1665).
    Style(Glyph),
    /// `setItems`, `insertItemAt`, `deleteItemAt` or `clearItems`: Table 234's `/Opt` as the
    /// script left it, whole, in the array's order — each entry its export value where it states
    /// one and the text a person sees (ADR 1737).
    Options(Vec<crate::form::Choice>),
}

impl Property {
    /// Whether this property and `other` set the same thing, so that the later replaces the
    /// earlier: the same member, and for a caption the same face.
    #[must_use]
    pub fn replaces(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Caption(mine, _), Self::Caption(theirs, _)) => mine == theirs,
            // A style is the normal caption written by another name, so either replaces the
            // other.
            (Self::Style(_), Self::Caption(face, _)) | (Self::Caption(face, _), Self::Style(_)) => {
                *face == Face::Normal
            }
            _ => self.member() == other.member(),
        }
    }

    /// The member a script wrote, as the reference spells it.
    #[must_use]
    pub fn member(&self) -> &'static str {
        match self {
            Self::Display(_) => "display",
            Self::ReadOnly(_) => "readonly",
            Self::Required(_) => "required",
            Self::TextColor(_) => "textColor",
            Self::FillColor(_) => "fillColor",
            Self::StrokeColor(_) => "strokeColor",
            Self::BorderStyle(_) => "borderStyle",
            Self::Alignment(_) => "alignment",
            Self::LineWidth(_) => "lineWidth",
            Self::TextSize(_) => "textSize",
            Self::TextFont(_) => "textFont",
            Self::CharLimit(_) => "charLimit",
            Self::TextFlag(flag, _) => flag.adobe(),
            Self::Caption(..) => "buttonSetCaption",
            Self::Style(_) => "style",
            Self::Options(_) => "setItems",
        }
    }

    /// Whether the member is one Adobe's reference makes a widget's rather than the field's ("Field
    /// versus widget attributes"): set through a `Field` of one widget it changes that widget, and
    /// through a `Field` of the whole field every widget (ADR 1664).
    #[must_use]
    pub fn is_widget_level(&self) -> bool {
        match self {
            Self::Display(_)
            | Self::TextColor(_)
            | Self::FillColor(_)
            | Self::StrokeColor(_)
            | Self::BorderStyle(_)
            | Self::Alignment(_)
            | Self::LineWidth(_)
            | Self::TextSize(_)
            | Self::TextFont(_)
            | Self::Caption(..)
            | Self::Style(_) => true,
            Self::ReadOnly(_)
            | Self::Required(_)
            | Self::CharLimit(_)
            | Self::TextFlag(..)
            | Self::Options(_) => false,
        }
    }
}

/// One change a script made, which the view state applies as an edit a person could have made
/// (RFC 0008 section 6.4).
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptEdit {
    /// `field.value = …` on a field other than the event's own, or `event.value` written where the
    /// trigger takes no value back.
    Value {
        /// The field.
        field: String,
        /// The text it takes.
        value: String,
    },
    /// A property set.
    Property {
        /// The field.
        field: String,
        /// The one widget it was set on, counted from zero in the field table's order, or `None`
        /// for every widget — always `None` for a member that is the field's
        /// ([`Property::is_widget_level`], ADR 1664).
        widget: Option<u32>,
        /// What was set.
        property: Property,
    },
    /// `this.resetForm(…)`: §12.7.6.3's reset over these fields, every field where empty.
    Reset {
        /// The names, each with its descendants.
        fields: Vec<String>,
    },
    /// `this.calculateNow()`: Table 224's `/CO` walked once more after the script.
    Calculate,
    /// `field.setFocus()`: the keyboard focus asked for on this field, which a host carries out —
    /// a view state holds the request and has no focus of its own ([`ViewState::take_focus_request`]).
    Focus {
        /// The field.
        field: String,
        /// The one widget asked for, counted from zero in the field table's order — the index
        /// `getField("name.N")` names — or `None` through a `Field` of every widget, whose focus
        /// is its first widget's (ADR 1688).
        widget: Option<u32>,
    },
    /// `this.pageNum = n`: the zero-based page a script turned to, which a host carries out as a
    /// person's page turn — a view state holds the request and shows no page of its own
    /// ([`ViewState::take_page_request`], ADR 1640).
    GoTo {
        /// The zero-based page.
        page: u32,
    },
    /// An `OCG` object's `state` set: §8.11's group switched as a person's layer switch would
    /// switch it ([`ViewState::set_group`], ADR 1626).
    Layer {
        /// The group dictionary's object number.
        number: u32,
        /// Its generation number.
        generation: u16,
        /// Whether it is to be on.
        on: bool,
    },
    /// An `Annotation` object's property set: the change a person's edit of that annotation would
    /// make (ADR 1700).
    Annotation {
        /// The annotation dictionary's object number.
        number: u32,
        /// Its generation number.
        generation: u16,
        /// What was set.
        change: AnnotationChange,
    },
    /// `app.setInterval(cExpr, nMilliseconds)` or `app.setTimeOut(…)`: an expression the view state
    /// holds and runs when its period has elapsed on the host's ticks, once or until cleared
    /// ([`ViewState::timer_due`], ADR 1702).
    Timer {
        /// The realm's number for it, which the interval or timeout object the script holds names.
        id: u32,
        /// The expression, as text.
        script: String,
        /// The period, in milliseconds.
        period: u32,
        /// Whether it runs every period (`setInterval`) rather than once (`setTimeOut`).
        repeat: bool,
    },
    /// `app.clearInterval(o)` or `app.clearTimeOut(o)`: the timer the object names stops.
    ClearTimer {
        /// The realm's number for it.
        id: u32,
    },
    /// `app.beep(nType)`: a sound the host plays, or says it cannot ([`ViewState::take_beeps`]).
    Beep {
        /// Which of the reference's five.
        sound: Sound,
    },
    /// `field.currentValueIndices = …`: §12.7.5.4's items selected by their indices into `/Opt`,
    /// committed as a person's choice is ([`super::Entered::Chosen`], ADR 1725).
    Choose {
        /// The field.
        field: String,
        /// The zero-based indices, ascending.
        indices: Vec<u32>,
    },
    /// `this.gotoNamedDest(cName)`: §12.3.2.4's named destination, which the view state resolves
    /// and holds for the host as [`ViewChange::Destination`] (ADRs 1724, 1751).
    Destination {
        /// The name, as the script spelled it.
        name: String,
    },
    /// `this.calculate = …`: whether Table 224's `/CO` is walked at all, until a script says
    /// otherwise (ADR 1724).
    Calculation {
        /// Whether calculations are performed.
        on: bool,
    },
    /// `this.zoom`, `this.zoomType`, `this.layout` set or `this.scroll(nX, nY)` called: a change to
    /// the window's view of the document, which a host carries out as it carries a focus request
    /// — a view state holds the request and has no view of its own
    /// ([`ViewState::take_view_requests`], ADR 1736).
    View {
        /// What the script changed.
        change: ViewChange,
    },
    /// `console.show`, `console.hide` or `console.clear`: a request to the host's console, which
    /// shows what scripts log (ADR 1762).
    Console(ConsoleCommand),
}

/// What a script asked of the host's console (ADR 1762): Adobe's "console methods", each a
/// documented choice under principle 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleCommand {
    /// `console.show`: the console is shown.
    Show,
    /// `console.hide`: the console is closed.
    Hide,
    /// `console.clear`: what the console shows is cleared.
    Clear,
}

/// A request to the host's console as a view state holds it until a host takes it (ADR 1762).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsoleRequest {
    /// What the script asked.
    pub command: ConsoleCommand,
    /// How many of [`ViewState::script_reports`]'s sentences stood when the request was applied:
    /// a host whose console shows those sentences clears the ones before this index, and the
    /// lines the same run logged after its `clear` come after it.
    pub at: usize,
}

/// What the host's keyboard held at the event a script runs for: Adobe's `event.shift`,
/// `event.modifier` and `event.keyDown`, which a host tells the view state of
/// ([`ViewState::set_keys`], ADR 1762).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Keys {
    /// Whether a shift key is down.
    pub shift: bool,
    /// Whether the platform's modifier key is down: Control here, as on Adobe's Microsoft Windows
    /// platform, since the reference names none for this platform (ADR 1762).
    pub modifier: bool,
    /// Whether an arrow key made a list box's or a combo box's pop-up selection; a host sets it
    /// only with that keystroke, and it is read only at a choice field's keystroke.
    pub arrows: bool,
}

/// A font a script named for `Field.textFont`, as the form's `/DR` holds it (ADR 1762).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontName {
    /// What `textFont` reads back: the resource's `/BaseFont`, or the resource name.
    pub base: String,
    /// The resource name a `/DA`'s `Tf` names it by: a key of `/DR`'s `/Font`, as its bytes'
    /// characters. Empty in what a realm sends: the view state finds the resource.
    pub resource: String,
}

impl ScriptEdit {
    /// Whether applying this edit can change what a page draws, so that a host draws it again
    /// (ADR 1762). A value, a property, a reset, a layer, an annotation and a choice each can; a
    /// focus, a page turn, a view change, a destination, a timer and a beep are requests a host
    /// carries out on its own, and `calculate` turning the walk on or off writes no value until a
    /// walk runs. A value written over an equal one is counted where the view state reads it.
    #[must_use]
    pub fn redraws(&self) -> bool {
        match self {
            Self::Property { .. }
            | Self::Reset { .. }
            | Self::Layer { .. }
            | Self::Annotation { .. }
            | Self::Choose { .. }
            | Self::Calculate => true,
            Self::Value { .. }
            | Self::Focus { .. }
            | Self::GoTo { .. }
            | Self::Timer { .. }
            | Self::ClearTimer { .. }
            | Self::Beep { .. }
            | Self::Destination { .. }
            | Self::Calculation { .. }
            | Self::View { .. }
            | Self::Console(_) => false,
        }
    }
}

/// One change a script asked of the window's view (ADR 1736).
///
/// Adobe's "Doc properties" and "Doc methods" pages are the meaning of each, cited and never
/// quoted, a documented choice under principle 5; what each lands on is the host's own view —
/// its magnification, its fitting mode, Table 29's page layout and its scroll.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewChange {
    /// `this.zoom = n`: the magnification, as a percentage, where 100 is one logical pixel per
    /// default user space unit — the reading Table 151's `/XYZ` magnification of 1 already has
    /// in the host.
    Zoom(f64),
    /// `this.zoomType = …`: one of the fitting modes a host draws.
    ZoomType(ZoomType),
    /// `this.layout = …`: Table 29's arrangement of the pages in the window.
    Layout(crate::viewer_preferences::PageLayout),
    /// `this.scroll(nX, nY)`: this point of this page brought to the middle of the window.
    Scroll {
        /// The zero-based page, the script's `this.pageNum` when it called.
        page: u32,
        /// The point's horizontal coordinate, in the page's default user space.
        x: f64,
        /// Its vertical coordinate, in the page's default user space.
        y: f64,
    },
    /// `this.gotoNamedDest(cName)`: the page and Table 149's view of the destination §12.3.2.4's
    /// name maps to, which a host shows as it shows a link's (ADR 1751).
    ///
    /// The view state makes this from [`ScriptEdit::Destination`], which is what a realm sends; a
    /// realm that sends this itself asks for a page and a view of it, as a link would.
    Destination {
        /// The zero-based page the destination names.
        page: u32,
        /// Where on it, and how large.
        view: crate::destination::View,
    },
}

/// Adobe's zoom types, the values "Doc properties" lists for `zoomType` (ADR 1736).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoomType {
    /// `NoVary`: a fixed magnification, which stays as it is when the window changes.
    #[default]
    NoVary,
    /// `FitPage`: the whole page, as large as fits.
    FitPage,
    /// `FitWidth`: the page's width, as large as fits.
    FitWidth,
    /// `FitHeight`: the page's height, as large as fits.
    FitHeight,
    /// `FitVisibleWidth`: the width of what the page draws, as large as fits — §12.3.2.2's
    /// `/FitBH`, which a host already applies.
    FitVisibleWidth,
    /// `Preferred`: the reader's own preferred magnification, which a document's realm is not
    /// told.
    Preferred,
    /// `ReflowWidth`: a reflowed page fitted to the window, which this program does not draw.
    ReflowWidth,
}

impl ZoomType {
    /// Every zoom type, in the reference's order.
    pub const ALL: [Self; 7] = [
        Self::NoVary,
        Self::FitPage,
        Self::FitWidth,
        Self::FitHeight,
        Self::FitVisibleWidth,
        Self::Preferred,
        Self::ReflowWidth,
    ];

    /// The reference's spelling, the string `zoomType` reads and the `zoomtype` constants hold.
    #[must_use]
    pub fn adobe(self) -> &'static str {
        match self {
            Self::NoVary => "NoVary",
            Self::FitPage => "FitPage",
            Self::FitWidth => "FitWidth",
            Self::FitHeight => "FitHeight",
            Self::FitVisibleWidth => "FitVisibleWidth",
            Self::Preferred => "Preferred",
            Self::ReflowWidth => "ReflowWidth",
        }
    }

    /// The reference's `zoomtype` constant's name for it: `zoomtype.fitW` is `"FitWidth"`.
    #[must_use]
    pub fn constant(self) -> &'static str {
        match self {
            Self::NoVary => "none",
            Self::FitPage => "fitP",
            Self::FitWidth => "fitW",
            Self::FitHeight => "fitH",
            Self::FitVisibleWidth => "fitV",
            Self::Preferred => "pref",
            Self::ReflowWidth => "refW",
        }
    }

    /// The zoom type the reference spells `name`, or `None`.
    #[must_use]
    pub fn from_adobe(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|zoom| zoom.adobe() == name)
    }

    /// Whether a host carries this zoom type out: every one but the reader's own preference and
    /// reflow, neither of which this program has to give.
    #[must_use]
    pub fn is_drawn(self) -> bool {
        !matches!(self, Self::Preferred | Self::ReflowWidth)
    }
}

/// The window's view of the document as a host told the view state of it: what a script reads as
/// `this.zoom`, `this.zoomType` and `this.layout` (ADR 1736).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WindowView {
    /// How large the page is drawn, as a percentage where 100 is one logical pixel per default
    /// user space unit; `None` where no host has said, which a script reads as `undefined`.
    pub zoom: Option<f64>,
    /// How the magnification is chosen: a fitting mode, or `NoVary` for a fixed one.
    pub zoom_type: ZoomType,
    /// Table 29's arrangement of the pages, as it now stands.
    pub layout: crate::viewer_preferences::PageLayout,
}

/// The field properties a script set, kept beside the edit log by field name.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Overrides {
    /// Each property set on the whole field, the latest a script set.
    pub(super) set: Vec<Property>,
    /// Each widget-level property set on one widget, by the widget's index, the latest a script
    /// set — applied after [`Self::set`], which a later set on the whole field clears it from.
    pub(super) widgets: BTreeMap<u32, Vec<Property>>,
}

impl Overrides {
    /// Records a property set on `widget`, or on the whole field, replacing an earlier one of the
    /// same member there; a set on the whole field replaces every widget's of the member too.
    pub(super) fn record(&mut self, widget: Option<u32>, property: Property) {
        let Some(widget) = widget else {
            for held in self.widgets.values_mut() {
                held.retain(|held| !held.replaces(&property));
            }
            self.widgets.retain(|_, held| !held.is_empty());
            self.set.retain(|held| !held.replaces(&property));
            self.set.push(property);
            return;
        };
        let held = self.widgets.entry(widget).or_default();
        held.retain(|held| !held.replaces(&property));
        held.push(property);
    }

    /// Whether a script set the field read-only, or writable, if it said either.
    pub(super) fn read_only(&self) -> Option<bool> {
        self.set.iter().find_map(|property| match property {
            Property::ReadOnly(flag) => Some(*flag),
            _ => None,
        })
    }
}

/// `app.beep`'s sound type: Adobe's "app methods" page numbers five, 0 to 4, with 4 the default.
///
/// What a host plays is its own: a toolkit with one system sound plays it for all five, which the
/// reference allows of two of its three platforms (ADR 1702).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sound {
    /// 0, an error.
    Error,
    /// 1, a warning.
    Warning,
    /// 2, a question.
    Question,
    /// 3, a status.
    Status,
    /// 4, the default.
    #[default]
    Default,
}

impl Sound {
    /// The five, in the reference's order: each one's place is its number.
    pub const ALL: [Self; 5] = [
        Self::Error,
        Self::Warning,
        Self::Question,
        Self::Status,
        Self::Default,
    ];

    /// The reference's word for the sound, as a host says it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Question => "question",
            Self::Status => "status",
            Self::Default => "default",
        }
    }
}

/// Table 227's flag bits a field's state carries as `readonly` and `required`.
const READ_ONLY: u32 = 1;
/// Table 227 bit 2.
const REQUIRED: u32 = 1 << 1;
/// Table 229 bit 16.
const RADIO: i64 = 1 << 15;
/// Table 229 bit 17.
const PUSHBUTTON: i64 = 1 << 16;
/// Table 233 bit 18.
const COMBO: i64 = 1 << 17;

impl ViewState {
    /// One field as its realm holds it, read from its first widget, with every override this state
    /// keeps applied — a hide's, and a script's (ADR 1603).
    ///
    /// `None` for a name whose first widget has no field type anywhere up its chain, which
    /// §12.7.4.2 makes a widget rather than a field.
    pub(super) fn field_state(
        &self,
        document: &Document,
        name: &str,
        widgets: &[ObjectId],
        pages: &BTreeMap<ObjectId, usize>,
    ) -> Option<FieldState> {
        let first = widgets.first().copied()?;
        let object = document.get(first);
        let stored = object.as_dict()?;
        // Table 234's `/Opt` as a script left it, so the options and what is selected among them
        // are the list the page draws (ADR 1737).
        let widget =
            &*crate::appearance::with_rewritten_options(document, stored, self.scripted(first));
        let field = crate::appearance::Field::read(document, widget, self.annotation(first).value);
        let kind = match field.kind? {
            crate::appearance::FieldKind::Text => FieldType::Text,
            crate::appearance::FieldKind::Button { .. } if field.flags & PUSHBUTTON != 0 => {
                FieldType::PushButton
            }
            crate::appearance::FieldKind::Button { .. } if field.flags & RADIO != 0 => {
                FieldType::RadioButton
            }
            crate::appearance::FieldKind::Button { .. } => FieldType::CheckBox,
            crate::appearance::FieldKind::Choice { .. } if field.flags & COMBO != 0 => {
                FieldType::ComboBox
            }
            crate::appearance::FieldKind::Choice { .. } => FieldType::ListBox,
            crate::appearance::FieldKind::Signature => FieldType::Signature,
        };
        let form = document
            .catalog()
            .ok()
            .and_then(|catalog| document.get_key(&catalog, "AcroForm").as_dict().cloned());
        let char_limit = inherited(document, &field.ancestry, form.as_ref(), "MaxLen")
            .as_integer()
            .and_then(|limit| u32::try_from(limit).ok());
        let mut state = FieldState {
            name: name.to_owned(),
            kind,
            value: self.text_of(document, first).unwrap_or_default(),
            flags: u32::try_from(field.flags & 0xFFFF_FFFF).unwrap_or(0),
            char_limit,
            page: widget
                .get("P")
                .and_then(Object::as_reference)
                .and_then(|page| pages.get(&page))
                .and_then(|index| u32::try_from(*index).ok()),
            widgets: widgets
                .iter()
                .filter_map(|widget| self.widget_state(document, *widget, form.as_ref()))
                .collect(),
            options: Vec::new(),
            selected: Vec::new(),
        };
        // Table 230's and Table 234's `/Opt` are each the field's, inherited, so one reading
        // serves both kinds of field; only a choice field selects among them (ADR 1725).
        let mut options = crate::form::options(document, &field);
        options.truncate(MAX_PAGES);
        if matches!(kind, FieldType::ComboBox | FieldType::ListBox) {
            state.selected = crate::form::selected(document, &field, &options)
                .into_iter()
                .filter_map(|index| u32::try_from(index).ok())
                .collect();
        }
        state.options = options;
        if let Some(overrides) = self.scripting.overrides.get(name) {
            for property in &overrides.set {
                state.apply(None, property);
            }
            for (widget, properties) in &overrides.widgets {
                for property in properties {
                    state.apply(Some(*widget), property);
                }
            }
        }
        Some(state)
    }

    /// One widget as its field's realm holds it, read from the widget and up its field's chain,
    /// with a hide's override applied (ADR 1664).
    fn widget_state(
        &self,
        document: &Document,
        id: ObjectId,
        form: Option<&Dictionary>,
    ) -> Option<WidgetState> {
        let object = document.get(id);
        let widget = object.as_dict()?;
        let field = crate::appearance::Field::read(document, widget, self.annotation(id).value);
        let characteristics = document.get_key(widget, "MK");
        let characteristic = |key: &str| {
            characteristics
                .as_dict()
                .and_then(|mk| colour_of(&document.get_key(mk, key)))
        };
        let style = document.get_key(widget, "BS");
        let border_style = style
            .as_dict()
            .and_then(|bs| document.get_key(bs, "S").as_name().cloned())
            .map_or(BorderStyle::Solid, |name| match name.as_bytes() {
                b"D" => BorderStyle::Dashed,
                b"B" => BorderStyle::Beveled,
                b"I" => BorderStyle::Inset,
                b"U" => BorderStyle::Underline,
                _ => BorderStyle::Solid,
            });
        let annotation_flags = document.get_key(widget, "F").as_integer().unwrap_or(0);
        let mut display = Display::of_flags(annotation_flags);
        match self.annotation_hidden(id) {
            Some(true) => display = Display::Hidden,
            Some(false) if display == Display::Hidden => display = Display::Visible,
            _ => {}
        }
        // Table 228's `/DA` and `/Q` are inheritable and Table 224 states the form's default for
        // each, so each is read up this widget's own chain.
        let tf = tf_of(document, &field.ancestry, form);
        Some(WidgetState {
            display,
            text_color: match inherited(document, &field.ancestry, form, "DA") {
                Object::String(bytes) => text_colour(&bytes),
                _ => None,
            },
            fill_color: characteristic("BG"),
            stroke_color: characteristic("BC"),
            border_style,
            alignment: match inherited(document, &field.ancestry, form, "Q").as_integer() {
                Some(1) => Alignment::Center,
                Some(2) => Alignment::Right,
                _ => Alignment::Left,
            },
            line_width: line_width_of(document, widget),
            text_size: tf.as_ref().and_then(|(_, size)| *size),
            text_font: tf
                .as_ref()
                .and_then(|(font, _)| font.as_ref())
                .map(|name| base_font_of(document, form, name))
                .unwrap_or_default(),
            rect: rect_of(document, widget),
            captions: captions_of(document, &characteristics),
            on_state: match field.kind {
                Some(crate::appearance::FieldKind::Button { toggling: true }) => {
                    crate::appearance::on_state(document, widget)
                        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                }
                _ => None,
            },
        })
    }
}

/// An inheritable entry of a field, read up `ancestry` — the widget first — and then from the
/// interactive form dictionary, which Table 224 gives a default for `/DA` and `/Q`; `/MaxLen` is
/// Table 230's and inherits with the rest of a field's entries.
fn inherited(
    document: &Document,
    ancestry: &[Dictionary],
    form: Option<&Dictionary>,
    key: &str,
) -> Object {
    ancestry
        .iter()
        .map(|dictionary| document.get_key(dictionary, key))
        .find(|value| !matches!(value, Object::Null))
        .or_else(|| {
            form.map(|form| document.get_key(form, key))
                .filter(|value| !matches!(value, Object::Null))
        })
        .unwrap_or(Object::Null)
}

impl ViewState {
    /// The document as a whole as its realm is told of it: the information dictionary, and every
    /// group a person's layer switch can change in the state it is in now (ADR 1626).
    ///
    /// Table 349's entries are read as `this.info` reads them: every key whose value is a text
    /// string — "the value associated with any such key shall be a text string", the clause says of
    /// every key but the two dates — with §7.9.4's date read for the moment it names, and
    /// `/Trapped`'s name as its characters. The groups are Table 98's `/OCGs` in their order, each
    /// with its state now and the state the default configuration opens it in; a group the
    /// configuration's `/Intent` does not cover has "no effect on visibility" (§8.11.2.3), no switch
    /// a person could flip, and is not listed.
    pub(super) fn document_state(&self, document: &Document) -> DocumentState {
        let mut state = DocumentState::default();
        if let Some(info) = document.get_key(document.trailer(), "Info").as_dict() {
            for (key, value) in info.iter() {
                let key = String::from_utf8_lossy(key.as_bytes()).into_owned();
                let text = match document.resolve(value) {
                    Object::String(bytes) => pdf_syntax::text_string(&bytes),
                    Object::Name(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
                    _ => continue,
                };
                let moment = matches!(key.as_str(), "CreationDate" | "ModDate")
                    .then(|| pdf_syntax::Date::parse(&text))
                    .flatten()
                    .map(|date| {
                        date.instant()
                            .saturating_mul(60)
                            .saturating_add(i64::from(date.second))
                            .saturating_mul(1000)
                    });
                state.info.push(InfoEntry { key, text, moment });
            }
        }
        state.pages = page_states(document);
        for (index, page) in state.pages.iter_mut().enumerate() {
            page.words = self.words_of(index);
        }
        let Some(content) = self.optional_content.as_ref() else {
            return state;
        };
        let opened = crate::optional_content::OptionalContent::read(document);
        let properties = document
            .catalog()
            .ok()
            .map(|catalog| document.get_key(&catalog, "OCProperties"));
        let groups = properties
            .as_ref()
            .and_then(Object::as_dict)
            .map(|properties| document.get_key(properties, "OCGs"));
        for group in groups
            .as_ref()
            .and_then(Object::as_array)
            .into_iter()
            .flatten()
            .filter_map(Object::as_reference)
            .take(MAX_LAYERS)
        {
            let Some(on) = content.state(group) else {
                continue;
            };
            state.layers.push(Layer {
                number: group.number,
                generation: group.generation,
                name: content.name(document, group).unwrap_or_default(),
                on,
                initially_on: opened
                    .as_ref()
                    .and_then(|opened| opened.state(group))
                    .unwrap_or(on),
                locked: content.is_locked(group),
                intent: intent_of(document, group),
            });
        }
        state
    }
}

/// Every page in page order, at most [`MAX_PAGES`], each with §12.4.2's label, Table 31's five
/// boundaries and its `/Rotate` (ADR 1724).
///
/// A page the tree lists and [`crate::page::Pages::get`] cannot build is told as the empty page
/// with no label, so that the realm's page numbers stay the document's; such a page is already
/// reported by everything that draws it.
fn page_states(document: &Document) -> Vec<PageState> {
    let pages = crate::page::Pages::new(document);
    let labels = crate::page_label::PageLabels::read(document);
    (0..pages.len().min(MAX_PAGES))
        .map(|index| {
            let label = labels.label(index);
            pages.get(index).map_or_else(
                || PageState {
                    label: label.clone(),
                    boxes: [[0.0; 4]; 5],
                    rotate: 0,
                    words: None,
                },
                |page| PageState::of(&page, label.clone()),
            )
        })
        .collect()
}

/// Table 192's three captions of a widget's `/MK`, in [`Face`] order, each empty where it states
/// none.
fn captions_of(document: &Document, characteristics: &Object) -> [String; 3] {
    Face::ALL.map(|face| {
        characteristics
            .as_dict()
            .and_then(|mk| match document.get_key(mk, face.key()) {
                Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
                _ => None,
            })
            .unwrap_or_default()
    })
}

/// Most groups a realm is told of.
///
/// Table 98's `/OCGs` is the document's, so its length is too; a document with more layers than
/// this is not one any panel lists, and the list is copied at every change of a group's state.
const MAX_LAYERS: usize = 4096;

/// `flags` with `bit` set or cleared.
fn set_bit(flags: u32, bit: u32, on: bool) -> u32 {
    if on { flags | bit } else { flags & !bit }
}

/// A colour array's components as Table 192 states them, `None` where it is not an array of
/// numbers of a length the table gives a meaning.
fn colour_of(object: &Object) -> Option<Colour> {
    let Object::Array(items) = object else {
        return None;
    };
    let components: Option<Vec<f64>> = items.iter().map(Object::as_number).collect();
    Colour::of_components(&components?)
}

/// The colour the last colour operator of a `/DA` string sets.
///
/// Table 228's `/DA` holds text-object operators; the non-stroking colour among them — §8.6.8's
/// `g`, `rg` or `k` — is the colour the field's text is drawn in. A `/DA` with none states no
/// colour, which §8.6.4's initial black makes black, and this answers `None` so that a script
/// reading it gets what the reference answers for an unset colour.
fn text_colour(bytes: &[u8]) -> Option<Colour> {
    let text = String::from_utf8_lossy(bytes);
    let mut numbers: Vec<f64> = Vec::new();
    let mut colour = None;
    for token in text.split_ascii_whitespace() {
        let take = |count: usize, numbers: &[f64]| -> Option<Vec<f64>> {
            numbers
                .len()
                .checked_sub(count)
                .and_then(|start| numbers.get(start..))
                .map(<[f64]>::to_vec)
        };
        match token {
            "g" => colour = take(1, &numbers).and_then(|c| Colour::of_components(&c)),
            "rg" => colour = take(3, &numbers).and_then(|c| Colour::of_components(&c)),
            "k" => colour = take(4, &numbers).and_then(|c| Colour::of_components(&c)),
            _ => {}
        }
        match token.parse::<f64>() {
            Ok(number) if number.is_finite() => numbers.push(number),
            _ => numbers.clear(),
        }
    }
    colour
}

/// Table 168's `/W` of a widget's `/BS`, or the third element of Table 166's `/Border`, or 1.
///
/// §12.5.4: "If neither the Border nor the BS entry is present, the border shall be drawn as a
/// solid line with a width of 1 point"; and Table 166's note on `/Border` makes `/BS` the one that
/// counts where both are present: "If an annotation dictionary includes the BS entry, then the
/// Border entry is ignored." A negative or non-finite width is no width and reads as the default.
fn line_width_of(document: &Document, widget: &Dictionary) -> f64 {
    let finite = |width: f64| (width.is_finite() && width >= 0.0).then_some(width);
    if let Some(style) = document.get_key(widget, "BS").as_dict() {
        return document
            .get_key(style, "W")
            .as_number()
            .and_then(finite)
            .unwrap_or(1.0);
    }
    if let Object::Array(border) = document.get_key(widget, "Border")
        && let Some(width) = border
            .get(2)
            .and_then(|item| document.resolve(item).as_number())
    {
        return finite(width).unwrap_or(1.0);
    }
    1.0
}

/// The font name and size of the `Tf` in Table 228's `/DA`, read up the field's chain and then
/// from the form, where the string holds one.
fn tf_of(
    document: &Document,
    ancestry: &[Dictionary],
    form: Option<&Dictionary>,
) -> Option<(Option<pdf_syntax::Name>, Option<f64>)> {
    let Object::String(bytes) = inherited(document, ancestry, form, "DA") else {
        return None;
    };
    let parsed = crate::variable_text::DefaultAppearance::parse(&bytes);
    parsed.font.as_ref()?;
    Some((parsed.font, parsed.size.map(f64::from)))
}

/// What `textFont` reads for a `/DA` font resource: the `/BaseFont` of the font the form's `/DR`
/// gives that name, or the name's own characters where it gives none.
fn base_font_of(document: &Document, form: Option<&Dictionary>, name: &pdf_syntax::Name) -> String {
    form.and_then(|form| document.get_key(form, "DR").as_dict().cloned())
        .and_then(|resources| document.get_key(&resources, "Font").as_dict().cloned())
        .and_then(|fonts| fonts.get_by_name(name).map(|font| document.resolve(font)))
        .and_then(|font| {
            font.as_dict()
                .map(|font| document.get_key(font, "BaseFont"))
        })
        .and_then(|base| base.as_name().cloned())
        .map_or_else(
            || String::from_utf8_lossy(name.as_bytes()).into_owned(),
            |base| String::from_utf8_lossy(base.as_bytes()).into_owned(),
        )
}

/// Table 96's `/Intent` of a group, as `OCG.getIntent` answers it.
///
/// Table 96: "A single name or an array of names that represent the intended use of the graphics
/// in the group." Its default is `View`, so a group stating none — or a value of another kind —
/// answers that.
fn intent_of(document: &Document, group: ObjectId) -> Vec<String> {
    let text = |name: &pdf_syntax::Name| String::from_utf8_lossy(name.as_bytes()).into_owned();
    let stated = document
        .get(group)
        .as_dict()
        .map(|dictionary| document.get_key(dictionary, "Intent"));
    let names: Vec<String> = match stated {
        Some(Object::Name(name)) => vec![text(&name)],
        Some(Object::Array(items)) => items
            .iter()
            .filter_map(|item| document.resolve(item).as_name().map(text))
            .collect(),
        _ => Vec::new(),
    };
    if names.is_empty() {
        vec!["View".to_owned()]
    } else {
        names
    }
}

/// A widget's Table 166 `/Rect`, or the empty rectangle where it states none readably.
fn rect_of(document: &Document, widget: &Dictionary) -> [f64; 4] {
    let Object::Array(items) = document.get_key(widget, "Rect") else {
        return [0.0; 4];
    };
    let mut rect = [0.0; 4];
    for (slot, item) in rect.iter_mut().zip(items.iter()) {
        *slot = document.resolve(item).as_number().unwrap_or_default();
    }
    rect
}

#[cfg(test)]
mod tests {
    use super::{Colour, Display, text_colour};

    #[test]
    fn a_default_appearance_string_states_its_last_colour() {
        assert_eq!(text_colour(b"/Helv 10 Tf 0 g"), Some(Colour::Gray(0.0)));
        assert_eq!(
            text_colour(b"0 g /Helv 0 Tf 1 0 0 rg"),
            Some(Colour::Rgb([1.0, 0.0, 0.0]))
        );
        assert_eq!(text_colour(b"/Helv 10 Tf"), None);
    }

    #[test]
    fn the_flags_say_which_display_constant_a_widget_has() {
        assert_eq!(Display::of_flags(4), Display::Visible);
        assert_eq!(Display::of_flags(0), Display::NoPrint);
        assert_eq!(Display::of_flags(2 | 4), Display::Hidden);
        assert_eq!(Display::of_flags(32 | 4), Display::NoView);
    }
}
