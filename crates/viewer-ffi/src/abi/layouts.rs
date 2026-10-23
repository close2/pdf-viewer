//! The structs a C caller passes by value or reads back, and the constants that name their fields' values.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

/// The revision of everything in this header that a caller compiles against.
///
/// **What it protects is the structs passed by value**, and nothing else needs it: a function
/// added later is a symbol an old caller never looks up, and a status or a kind added later is a
/// number an old caller has a `default:` arm for — but a field added to [`PdfvGeometry`],
/// [`PdfvFrame`] or [`PdfvViewing`] changes a size the caller has already compiled, and no
/// diagnostic anywhere would catch it. This number moves whenever one of those does.
///
/// **A struct *added* does not move it**, which is the same argument as the one for a function: a
/// caller compiled before [`PdfvViewing`] existed passes nothing of that shape and looks up neither
/// of the two symbols that take one.
pub const QUORRA_ABI_VERSION: u32 = 2;

/// The level `quorra_restrict_document_operation` takes to mean *no level at all*: this document
/// follows the window's policy for that operation.
///
/// **Not a fifth `QUORRA_RESTRICT_*`**, and the distinction is the decision rather than a
/// spelling. `CLAUDE.md` names four levels; this is the absence of one, valid in that entry point
/// and nowhere else, so every other entry point that takes a level goes on refusing it as a
/// number it does not define (ADR 1145).
pub const QUORRA_RESTRICT_INHERIT: u32 = 4;

/// A rich paragraph that states no `text-align`, which a caller starts at its own start edge
/// (`quorra_rich_paragraph::align`, ADR 1655).
pub const QUORRA_RICH_ALIGN_NONE: u32 = 0;
/// Chapter 27's `text-align: left`.
pub const QUORRA_RICH_ALIGN_LEFT: u32 = 1;
/// `center`.
pub const QUORRA_RICH_ALIGN_CENTRE: u32 = 2;
/// `right`.
pub const QUORRA_RICH_ALIGN_RIGHT: u32 = 3;
/// `justify` and `justify-all`.
pub const QUORRA_RICH_ALIGN_JUSTIFY: u32 = 4;

/// A tab stop's text standing with its left edge at the stop (`quorra_rich_tab::align`, ADR 1667).
pub const QUORRA_RICH_TAB_LEFT: u32 = 0;
/// Centred on the stop.
pub const QUORRA_RICH_TAB_CENTRE: u32 = 1;
/// Its right edge at the stop.
pub const QUORRA_RICH_TAB_RIGHT: u32 = 2;
/// Its first full stop at the stop, its right edge where it has none.
pub const QUORRA_RICH_TAB_DECIMAL: u32 = 3;
/// The edge the text starts from: the left in a paragraph read left to right, the right in one
/// read right to left — every default stop's alignment.
pub const QUORRA_RICH_TAB_AFTER: u32 = 4;
/// The edge the text ends at, the other way round.
pub const QUORRA_RICH_TAB_BEFORE: u32 = 5;

/// A stop with no leader, or a blank one: nothing fills the room before it
/// (`quorra_rich_leader::pattern`, ADR 1726).
pub const QUORRA_RICH_LEADER_NONE: u32 = 0;
/// Chapter 2's `dots()`: the run's own full stop, repeated.
pub const QUORRA_RICH_LEADER_DOTS: u32 = 1;
/// `rule(ruleStyle [ruleThickness])`, in the text's colour.
pub const QUORRA_RICH_LEADER_RULE: u32 = 2;
/// `use-content(content)`: the characters `quorra_popup_rich_leader_text` answers, repeated.
pub const QUORRA_RICH_LEADER_CONTENT: u32 = 3;

/// A rule leader drawn unbroken (`quorra_rich_leader::rule_style`); `double`, `groove` and
/// `ridge` are read as solid, which the chapter permits.
pub const QUORRA_RICH_RULE_SOLID: u32 = 0;
/// A dash of two thicknesses every four.
pub const QUORRA_RICH_RULE_DASHED: u32 = 1;
/// A square dot every two thicknesses.
pub const QUORRA_RICH_RULE_DOTTED: u32 = 2;

/// A paragraph's tab stops, as `quorra_popup_rich_tabs` answers them (ADR 1667): how many it
/// states, and its `tab-interval` where it states one.
///
/// A struct added rather than one widened, so [`QUORRA_ABI_VERSION`] does not move (ADR 0737).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PdfvRichTabs {
    /// How many stops `quorra_popup_rich_tab` reads, in the order the paragraph states them.
    pub stops: usize,
    /// Whether the paragraph states a `tab-interval`; where it does not, no default stop is set
    /// and a tab past every stated stop advances by nothing.
    pub has_interval: bool,
    /// The interval: so many of the caller's text size.
    pub interval_per_base: f32,
    /// And so many points beside them.
    pub interval_points: f32,
}

/// One stated tab stop: how the text after it stands, and its distance from the paragraph's left
/// margin, resolved as a run's lengths are (ADR 1667).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PdfvRichTab {
    /// `QUORRA_RICH_TAB_*`.
    pub align: u32,
    /// The position: so many of the caller's text size.
    pub at_per_base: f32,
    /// And so many points beside them.
    pub at_points: f32,
}

/// A stated stop's leader: chapter 2's *Tab Leader Pattern* (pages 63 to 65), as the two toolkit
/// windows receive it from `viewer_host::popup::tab_stops` and in the units a stop's position
/// crosses in (ADR 1726).
///
/// A struct added rather than [`PdfvRichTab`] widened, so [`QUORRA_ABI_VERSION`] does not move
/// (ADR 0737).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PdfvRichLeader {
    /// `QUORRA_RICH_LEADER_*`.
    pub pattern: u32,
    /// `QUORRA_RICH_RULE_*`, for a rule; solid otherwise.
    pub rule_style: u32,
    /// Whether a rule states its `ruleThickness`; where it does not, the caller's own underline
    /// thickness is the rule's.
    pub has_thickness: bool,
    /// The thickness: so many of the caller's text size.
    pub thickness_per_base: f32,
    /// And so many points beside them.
    pub thickness_points: f32,
    /// Whether the leader states `leaderPatternWidth`; where it does not, one repetition is the
    /// pattern's own width as the caller sets it.
    pub has_width: bool,
    /// The least width of one repetition: so many of the caller's text size.
    pub width_per_base: f32,
    /// And so many points beside them.
    pub width_points: f32,
}

/// One paragraph of Table 172's `/RC`, as `quorra_popup_rich_paragraph` answers it (ADR 1655).
///
/// Written into a caller's struct rather than passed by value, so [`QUORRA_ABI_VERSION`] does not
/// move: a struct added is a shape no older caller hands over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct PdfvRichParagraph {
    /// `QUORRA_RICH_ALIGN_*`.
    pub align: u32,
    /// How many lists the paragraph is nested in, each one list indent.
    pub level: u16,
    /// Whether piece 0 is the item's tag — its number or bullet — rather than the first run.
    pub has_tag: bool,
    /// How many pieces `quorra_popup_rich_run` reads: the tag where there is one, then the runs.
    pub pieces: usize,
}

/// One run of a rich paragraph, its characters and its families read beside it (ADR 1655).
///
/// **Lengths cross as a caller resolves them**: so many of the caller's own text size and so many
/// points beside them, §12.5.6.4 leaving the window's size to the processor. A letter spacing is
/// `spacing_per_base × base + spacing_points × points + spacing_of_space × (width of a space in the
/// face the caller set the run in) × em`, at most one of the three being other than zero — chapter
/// 27's percentage being of a space in a face only the caller picks.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PdfvRichRun {
    /// Whether this piece is a list item's tag, generated by the list rather than stated.
    pub tag: bool,
    /// CSS2's weight above 500.
    pub bold: bool,
    /// `font-style` other than `normal`.
    pub italic: bool,
    /// Whether [`Self::colour`] is stated; the window's own text colour where not.
    pub has_colour: bool,
    /// `color`, sRGB, 0 to 1.
    pub colour: [f32; 3],
    /// The em: so many of the caller's text size.
    pub size_per_base: f32,
    /// And so many points beside them.
    pub size_points: f32,
    /// How far the baseline is raised, the same two ways; negative is lowered.
    pub rise_per_base: f32,
    /// The rise's points.
    pub rise_points: f32,
    /// `letter-spacing` as a share of the caller's text size.
    pub spacing_per_base: f32,
    /// `letter-spacing` in points.
    pub spacing_points: f32,
    /// `letter-spacing` as a share of a space in the run's face.
    pub spacing_of_space: f32,
    /// `xfa-font-horizontal-scale`, a factor.
    pub horizontal_scale: f32,
    /// `xfa-font-vertical-scale`, a factor.
    pub vertical_scale: f32,
    /// How many underlines: 0, 1 or 2.
    pub underlines: u8,
    /// Whether the underline skips the spaces between words.
    pub underline_by_word: bool,
    /// `line-through`.
    pub line_through: bool,
    /// How many names `font-family`'s search path holds, nearest first.
    pub families: usize,
}

/// Where a page sits on the screen and how large it is drawn.
///
/// Passed by value, which is why [`QUORRA_ABI_VERSION`] exists.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PdfvGeometry {
    /// The page's extent in PDF user space units after §7.7.3.3's `/Rotate`.
    pub page_width: f32,
    /// The page's height in the same units.
    pub page_height: f32,
    /// Device pixels per user space unit: the zoom and the display's scale together.
    pub scale: f32,
    /// The rasterised page's width in device pixels.
    pub width: u32,
    /// Its height.
    pub height: u32,
    /// Where the raster's top-left corner sits in the viewport, in device pixels.
    pub origin_x: f32,
    /// The same, vertically.
    pub origin_y: f32,
}

/// Where the reader is looking: the page, the magnification and the scroll.
///
/// Passed by value, which is why [`QUORRA_ABI_VERSION`] exists. A caller reads one from
/// `quorra_view`, keeps it, and hands it back to `quorra_set_view` — the two directions this value is
/// meant to travel in, and a C caller composing one from numbers of its own is guessing where the
/// viewer's clamp would have left the reader.
///
/// **Named `quorra_viewing` in the header and not `quorra_view`, for [`PdfvFrame`]'s reason**: C puts
/// a struct tag and a function in one namespace, so a `typedef struct quorra_view` beside an
/// `int32_t quorra_view(…)` is a redeclaration error. That note was written about
/// `quorra_frame_info`; this is the second time the same fact has decided a name here.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PdfvViewing {
    /// Which page, zero-based.
    pub page: usize,
    /// How large the page is drawn: one of `QUORRA_ZOOM_*`.
    pub zoom: u32,
    /// Logical pixels per user space unit for `QUORRA_ZOOM_SCALE`, and zero for the others.
    pub scale: f32,
    /// How far the page is scrolled under the viewport, in device pixels; positive moves the
    /// content up and left, which is the sense `quorra_scroll`'s delta has.
    pub scroll_x: f32,
    /// The same, vertically.
    pub scroll_y: f32,
}

/// What the viewer is holding, without the pixels.
///
/// The first half of C's two-call idiom: ask this, size a buffer, then `quorra_frame_copy`.
///
/// **Named `quorra_frame` and not `quorra_frame_info`, because C has one namespace for both.** A
/// `typedef struct quorra_frame_info` beside an `int32_t quorra_frame_info(…)` is a redeclaration
/// error, which is the first thing `c/open_a_page.c` found and is a class of mistake no amount of
/// reading the Rust would have shown.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PdfvFrame {
    /// Which page these pixels are of, zero-based.
    pub page: usize,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The pixel layout: `QUORRA_FORMAT_RGBA8`, and nothing else in this build.
    pub format: u32,
    /// How many bytes `quorra_frame_copy` writes.
    pub bytes: usize,
    /// Where the raster's top-left corner sits in the viewport, in device pixels.
    pub origin_x: f32,
    /// The same, vertically.
    pub origin_y: f32,
}
