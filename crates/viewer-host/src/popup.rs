//! ISO 32000-2 §12.5.6.14: what a popup window says, and where a host puts it.
//!
//! The clause's first sentence is why a window is a *host's* rather than a page's:
//!
//! > A popup annotation ( PDF 1.3 ) displays text in a popup window for entry and editing. It
//! > shall not appear alone but is associated with a markup annotation, its parent annotation,
//! > and shall be used for editing the parent's text. It shall have no appearance stream or
//! > associated actions of its own
//!
//! No appearance stream, so nothing this program rasterises can draw one; `pdf_model::popup`
//! reads the entries and [`viewer_core::Query::Popups`] places them, and what is left is the
//! window furniture — which is a platform's. `viewer-ui` draws its own, `viewer-gtk` places
//! GTK widgets and `viewer-qt` places Qt ones, and all three of them need the same three
//! strings and the same rectangle.
//!
//! **That is the whole of what is here.** The title bar's label and its timestamp, the body, and
//! the upright box the window occupies — one reading of §12.5.6.2 and Table 166, made once. The
//! *look* is the toolkit's — a title bar's height, its font, its corners — and this crate has no
//! widget in it, with two exceptions: the window's [`PAPER`] and its [`EDGE`].

use viewer_core::PopupWindow;

/// The ground of a popup window's body, under its text.
///
/// **Opaque, and not the page's white**, and both halves are the point. The clause gives a popup
/// no appearance stream, so its window is drawn by this program and covers the page under its
/// rectangle; a body the page showed through puts the note's words over the page's, and a body
/// the colour of paper has no extent anybody can see. A note's colour is a choice — the clause
/// states none — and it is the one `quorra` drew first; it is shared so that the three windows
/// show one window rather than three (ADR 1466).
pub const PAPER: pdf_render::Color = pdf_render::Color {
    r: 1.0,
    g: 0.99,
    b: 0.90,
    a: 1.0,
};

/// The one-pixel line round a popup window, which is where its rectangle ends (ADR 1466).
pub const EDGE: pdf_render::Color = pdf_render::Color {
    r: 0.78,
    g: 0.78,
    b: 0.80,
    a: 1.0,
};

/// One of §12.5.6.14's windows, as a host is about to put it on the screen.
///
/// The strings are borrowed from the answer, so building this list costs no allocation at all —
/// which matters because a host asks [`viewer_core::Query::Popups`] on every repaint, exactly as
/// it asks for the selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window<'a> {
    /// The popup annotation, which is what [`viewer_core::Command::Activate`] closes.
    pub annotation: pdf_syntax::ObjectId,
    /// [`viewer_core::PopupWindow::note`]: the text note a person retypes this window's text into
    /// with [`viewer_core::Edit::SetNoteText`], and `None` for a window a host offers no keyboard
    /// to (ADR 1726).
    pub note: Option<pdf_syntax::ObjectId>,
    /// §12.5.6.2's `/T`, which goes at the left of the title bar.
    ///
    /// > The text label that shall be displayed in the title bar of the annotation's popup window
    /// > when open and active. This entry shall identify the user who added the annotation.
    ///
    /// **Empty where the file states none, and that is a choice rather than a fallback.** The
    /// entry is optional and names *a person*; a window headed `Untitled` or `Annotation` would
    /// be this program asserting an author the document does not claim, which principle 5's last
    /// clause is about. An untitled window still has a title bar, because Table 166's `/M` and
    /// the frame belong there whatever `/T` says.
    pub title: &'a str,
    /// Table 166's `/M`, as a person reads it, at the right of the title bar.
    ///
    /// [`crate::stamp`]'s answer, which is §7.9.4's date where the string parses as one and the
    /// file's own characters where it does not — the table makes displaying it in *any* format a
    /// `shall`, so a string this program cannot parse is still shown.
    ///
    /// `None` where the annotation states no `/M`.
    pub modified: Option<&'a str>,
    /// Table 166's `/Contents`: the text in the window.
    ///
    /// Empty where the annotation states none, which is a window with a title bar and nothing in
    /// it. The clause permits it and a good third of the corpus's popups are like that
    /// (`pdf-model`'s `popup` module records the census).
    pub text: &'a str,
    /// The title bar's colour, from Table 166's `/C` — "[t]he title bar of the annotation's popup
    /// window".
    ///
    /// `None` where the file states none *and* for the empty array the table gives the meaning
    /// "0 No colour; transparent", which `pdf_model::popup` has already folded together. A host
    /// puts its own platform colour there.
    pub colour: Option<pdf_render::Color>,
    /// Where the window goes, in device pixels of the viewport: `(left, top, width, height)`.
    ///
    /// [`crate::bounds`] of [`viewer_core::PopupWindow::quad`], because a window is upright in
    /// every toolkit and §7.7.3.3's `/Rotate` can turn the rectangle the file states.
    pub place: (f32, f32, f32, f32),
    /// §12.5.6.2's thread, under [`Self::text`]: the replies this window shows instead of their
    /// own.
    ///
    /// Table 172 makes it a `shall` that they are not displayed "individually but together in the
    /// form of threaded comments", so a host drawing only [`Self::text`] has dropped them.
    /// `pdf_model::popup::Comment::depth` is how far each is indented.
    pub replies: &'a [pdf_model::popup::Comment],
    /// Table 172's `/RC` with its formatting, which a host draws in place of [`Self::text`] where
    /// it is `Some` — "[a] rich text string … that shall be displayed in the popup window when the
    /// annotation is opened". The runs say what each character is set in; [`size`], [`rise`],
    /// [`family`] and [`not_drawn`] are the readings of them the three windows share (ADR 1642).
    pub rich: Option<&'a pdf_model::popup::RichNote>,
}

/// Every window in an answer that has somewhere to go, in the order the page listed them.
///
/// **A window with no area is left out**, and it is the one refusal here. Table 166 makes `/Rect`
/// required and §12.5.6.14 gives the popup no appearance stream to fall back on, so a rectangle
/// whose corners coincide describes a window a person could not see and a widget a toolkit would
/// still put in its layout. Leaving it out is not silence: the annotation is still on the page and
/// its parent still opens it, and there is nothing here that a host could draw instead.
///
/// The list is short by nature — a page states as many windows as it has open comments — so this
/// allocates one vector per repaint and borrows every string in it.
#[must_use]
pub fn windows(popups: &[PopupWindow]) -> Vec<Window<'_>> {
    popups
        .iter()
        .filter_map(|popup| {
            let place = crate::bounds(popup.quad);
            (place.2 > 0.0 && place.3 > 0.0).then_some(Window {
                annotation: popup.annotation,
                note: popup.note,
                title: popup.title.as_deref().unwrap_or_default(),
                modified: popup.modified.as_deref(),
                text: popup.text.as_deref().unwrap_or_default(),
                colour: popup.colour,
                place,
                replies: &popup.replies,
                rich: popup.rich.as_ref(),
            })
        })
        .collect()
}

/// §12.5.6.2's thread as one block of plain text, for a host that places a single label.
///
/// ISO 32000-2 §12.5.6.2, Table 172, the `/RT` value `R`:
///
/// > Interactive PDF processors shall not display replies to an annotation individually but
/// > together in the form of threaded comments.
///
/// [`Window::replies`] is the structured answer and is what a host laying out its own widgets uses
/// — `viewer-gtk` gives each reply a box and `viewer-ui` indents it as it draws. A host that
/// crosses a bridge carrying strings has one label to fill, and this is the same thread flattened
/// in one place rather than in each of them: each reply's author on its own line, its text under
/// that, and two spaces of indent per `/IRT` hop.
///
/// Empty for a window nobody replied to, which is almost every window, so a caller can append it
/// unconditionally and add no label where there is nothing to say.
#[must_use]
pub fn thread(window: &Window<'_>) -> String {
    let mut out = String::new();
    for reply in window.replies {
        // Four levels, which is the deepest chain measured over ISO 32000-2's own PDF; past that
        // the indent would cost more width than the depth is worth.
        let indent = "  ".repeat(reply.depth.min(4));
        if let Some(who) = reply.title.as_deref().filter(|who| !who.is_empty()) {
            out.push_str(&indent);
            out.push_str(who);
            out.push('\n');
        }
        for line in reply
            .text
            .as_deref()
            .unwrap_or_default()
            .split(['\r', '\n'])
        {
            out.push_str(&indent);
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// The smallest a run is drawn, as a multiple of the window's base size.
///
/// A choice, and the window's rather than the note's: §12.5.6.4 lets the processor choose "a font
/// and size" for a note's window, and a size a person cannot read, or one larger than the
/// document's own rectangle, would be this program showing a window that says nothing. Chapter 27
/// states no bound of its own; these keep `sub` of `sub` legible and a `72pt` heading inside the
/// window (ADR 1642).
pub const SMALLEST_RUN: f32 = 0.5;

/// The largest a run is drawn, as a multiple of the window's base size — [`SMALLEST_RUN`]'s
/// choice, from the other end.
pub const LARGEST_RUN: f32 = 3.0;

/// The size a rich run is drawn at, in the host's own unit.
///
/// `base` is the window's text size and `per_point` how many of the same unit one point is: a
/// toolkit that sizes text in points passes its font's point size and `1.0`, and a host that
/// draws in pixels passes its pixel size and its pixels per point. The run's relative sizes are
/// multiples of `base` and its absolute ones are points, which is `pdf_model::popup::Measure`'s
/// split; the result is held between [`SMALLEST_RUN`] and [`LARGEST_RUN`] of `base`.
#[must_use]
pub fn size(run: &pdf_model::popup::RichRun, base: f32, per_point: f32) -> f32 {
    let wanted = run.size.per_base.mul_add(base, run.size.points * per_point);
    wanted.clamp(base * SMALLEST_RUN, base * LARGEST_RUN)
}

/// How far a rich run's baseline is raised, in the unit [`size`] answers in — negative is
/// lowered — held within the run's own size so that a `vertical-align` of a page's height does
/// not take the run out of its line.
#[must_use]
pub fn rise(run: &pdf_model::popup::RichRun, base: f32, per_point: f32) -> f32 {
    let size = size(run, base, per_point);
    run.rise
        .per_base
        .mul_add(base, run.rise.points * per_point)
        .clamp(-size, size)
}

/// The extra advance after each of a run's characters — chapter 27's `letter-spacing` (page
/// 1204) — in the unit [`size`] answers in, or `None` where it is a share of a space and `space`
/// is not known.
///
/// `space` is the width of a space in the face the run is set in, as a share of its em: a host
/// that chose the face knows it, and a toolkit that chooses its own face from a family name does
/// not, so that host passes `None` and says the spacing under the note ([`toolkit_unapplied`]).
/// Held within the run's own size either way, as [`rise`] is, so that a spacing of a page's width
/// does not put one letter in the window and the rest outside it (ADR 1654).
#[must_use]
pub fn letter_spacing(
    run: &pdf_model::popup::RichRun,
    base: f32,
    per_point: f32,
    space: Option<f32>,
) -> Option<f32> {
    let size = size(run, base, per_point);
    let wanted = match run.letter_spacing {
        pdf_model::popup::RichSpacing::Length(length) => {
            length.per_base.mul_add(base, length.points * per_point)
        }
        pdf_model::popup::RichSpacing::OfSpace(share) => space? * size * share,
    };
    Some(wanted.clamp(-size, size))
}

/// Whether a rich paragraph reads right to left: UAX #9's rules P2 and P3 over its runs' characters
/// as one paragraph, whichever run holds the first strong one.
///
/// A paragraph that states no `text-align` starts at its own start edge — the right for one of
/// these — in all three windows (ADR 1654): Qt's unaligned block follows its text, and a window
/// that places a label by hand asks this.
#[must_use]
pub fn right_to_left(paragraph: &pdf_model::popup::RichParagraph) -> bool {
    let text: String = paragraph.runs.iter().map(|run| run.text.as_str()).collect();
    pdf_font::shaping::Paragraphs::new(&text)
        .is_some_and(|levels| levels.paragraph_level(0) % 2 == 1)
}

/// Whether a run states chapter 27's `xfa-font-horizontal-scale` or `xfa-font-vertical-scale`
/// (page 1202) as anything but its whole size.
#[must_use]
pub fn scaled(run: &pdf_model::popup::RichRun) -> bool {
    (run.horizontal_scale - 1.0).abs() > f32::EPSILON
        || (run.vertical_scale - 1.0).abs() > f32::EPSILON
}

/// How the text after a tab stands at its stop, once the paragraph's direction has turned
/// chapter 27's `after` and `before` into a side: the four alignments Pango's tab array, Qt's tab
/// positions and `quorra`'s own layout each set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabSide {
    /// The text's left edge at the stop.
    Left,
    /// The text centred on the stop.
    Centre,
    /// The text's right edge at the stop.
    Right,
    /// The text's first full stop at the stop, its right edge where it has none.
    Decimal,
}

/// One of a paragraph's tab stops, placed: how the text after it stands, how far it is from the
/// paragraph's left margin in the unit [`size`] answers in, and what fills the room before it.
#[derive(Debug, Clone, PartialEq)]
pub struct TabStop {
    /// How the text after the tab stands at the stop.
    pub side: TabSide,
    /// The stop's distance from the left margin.
    pub at: f32,
    /// The stop's leader, where it states one; a default stop is blank (page 1205).
    pub leader: Option<Leader>,
}

/// A tab leader, placed: chapter 2's *Tab Leader Pattern* (pages 63 to 65) in the unit [`size`]
/// answers in (ADR 1679).
#[derive(Debug, Clone, PartialEq)]
pub struct Leader {
    /// What is repeated across the room before the stop.
    pub pattern: LeaderPattern,
    /// `leaderPatternWidth`, the least width of one repetition; zero where none is stated, and
    /// then a repetition is the pattern's own width.
    pub width: f32,
}

/// What a leader repeats.
#[derive(Debug, Clone, PartialEq)]
pub enum LeaderPattern {
    /// The run's full stop.
    Dots,
    /// A rule in the text's colour, centred on the baseline.
    Rule {
        /// Solid, dashed or dotted.
        style: pdf_model::popup::RichRuleStyle,
        /// Its thickness; `None` takes the window's underline thickness.
        thickness: Option<f32>,
    },
    /// These characters, in the run's face.
    Content(String),
}

/// The most default stops a paragraph is given: a window's width at the smallest interval a
/// person can tell from a space is far below it, and a `tab-interval` of a hair is not a reason to
/// hand a toolkit a million stops.
const MOST_DEFAULT_STOPS: usize = 256;

/// The stops a paragraph's tabs advance to, nearest the left margin first, as far as `room`.
///
/// ISO 32000-2 §12.7.4.3 brings chapter 27 of XFA 3.3 in for a rich text string's formatting, and
/// its *Tab Stops* (pages 1205 to 1207) is the rule: the stops `tab-stops` states, then the
/// default ones at every multiple of `tab-interval` beyond the stated ones, each default stop
/// aligned `after` — its left edge where the paragraph reads left to right. *Beyond* is read in
/// the direction the text flows, which chapter 2's *Tab Stops* makes leftward in a paragraph read
/// right to left (page 61): there the default stops lie left of the leftmost stated one, as a
/// field's layout places them (ADR 1679). `base` and `per_point` are [`size`]'s; a paragraph
/// stating neither property has no stop, and then a tab advances by nothing ([`advances`]), which
/// is the chapter's own reading where nothing sets one.
#[must_use]
pub fn tab_stops(
    paragraph: &pdf_model::popup::RichParagraph,
    base: f32,
    per_point: f32,
    room: f32,
) -> Vec<TabStop> {
    use pdf_model::popup::RichTabAlign;
    let right_to_left = right_to_left(paragraph);
    let side = |align: RichTabAlign| match (align, right_to_left) {
        (RichTabAlign::Left, _) | (RichTabAlign::After, false) | (RichTabAlign::Before, true) => {
            TabSide::Left
        }
        (RichTabAlign::Right, _) | (RichTabAlign::After, true) | (RichTabAlign::Before, false) => {
            TabSide::Right
        }
        (RichTabAlign::Centre, _) => TabSide::Centre,
        (RichTabAlign::Decimal, _) => TabSide::Decimal,
    };
    let length = |measure: pdf_model::popup::Measure| {
        measure.per_base.mul_add(base, measure.points * per_point)
    };
    let mut stops: Vec<TabStop> = paragraph
        .tab_stops
        .iter()
        .map(|stop| TabStop {
            side: side(stop.align),
            at: length(stop.at),
            leader: stop.leader.as_ref().map(|leader| Leader {
                pattern: match &leader.pattern {
                    pdf_model::popup::RichLeaderPattern::Dots => LeaderPattern::Dots,
                    pdf_model::popup::RichLeaderPattern::Rule { style, thickness } => {
                        LeaderPattern::Rule {
                            style: *style,
                            thickness: thickness.map(length).filter(|t| t.is_finite()),
                        }
                    }
                    pdf_model::popup::RichLeaderPattern::Content(content) => {
                        LeaderPattern::Content(content.clone())
                    }
                },
                width: leader
                    .width
                    .map(length)
                    .filter(|width| width.is_finite())
                    .unwrap_or_default()
                    .max(0.0),
            }),
        })
        .filter(|stop| stop.at.is_finite() && stop.at >= 0.0)
        .collect();
    stops.sort_by(|one, other| one.at.total_cmp(&other.at));
    if let Some(interval) = paragraph
        .tab_interval
        .map(length)
        .filter(|interval| interval.is_finite() && *interval > 0.0)
    {
        let default = |at| TabStop {
            side: side(RichTabAlign::After),
            at,
            leader: None,
        };
        if right_to_left {
            // Leftward from the leftmost stated stop, or from the far edge where none is stated;
            // a default stop is never at the margin itself, which no tab can reach leftward.
            let first = stops.first().map_or(room, |stop| stop.at);
            let below: Vec<TabStop> = (1..=MOST_DEFAULT_STOPS)
                .map_while(|step| {
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "a count held at MOST_DEFAULT_STOPS is exactly representable"
                    )]
                    let at = step as f32 * interval;
                    (at < first - f32::EPSILON).then(|| default(at))
                })
                .collect();
            stops.splice(0..0, below);
        } else {
            let last = stops.last().map_or(0.0, |stop| stop.at);
            let mut at = ((last / interval).floor() + 1.0) * interval;
            let mut added = 0;
            while at <= room && added < MOST_DEFAULT_STOPS {
                stops.push(default(at));
                at += interval;
                added = added.saturating_add(1);
            }
        }
    }
    stops
}

/// A right-to-left paragraph's stops as a toolkit that measures a stop from a line's start edge
/// is handed them: each one's distance from `edge`, where the line starts, measured like the
/// stops from the paragraph's left margin and in their unit — nearest that edge first, and only
/// those left of it, since a tab there reaches leftward and none reaches a stop at or beyond the
/// line's start (ADR 1690).
///
/// Chapter 2's *Tab Stops* places a stop from the left margin and has a right-to-left
/// paragraph's tab reach the next one on its left (page 61), which is how [`tab_stops`] places
/// them. Pango and Qt both measure a stop in a block read right to left from its start edge, the
/// right — measured: a stop at 100 px in a line 400 px wide stood at 300 px from its left in both,
/// and at 200 px in a line 300 px wide — so the same stop is a different number in every width,
/// and a window hands these again whenever the toolkit gives the note another one. Each stop keeps
/// its side; what a side is called in a block read right to left is the toolkit's (Pango names
/// the edge the text starts from, Qt the left or right edge as drawn).
#[must_use]
pub fn from_start_edge(stops: &[TabStop], edge: f32) -> Vec<TabStop> {
    stops
        .iter()
        .rev()
        .filter(|stop| stop.at < edge - f32::EPSILON)
        .map(|stop| TabStop {
            at: edge - stop.at,
            ..stop.clone()
        })
        .collect()
}

/// Whether a paragraph's tabs advance at all: chapter 27 sets no stop where neither
/// `tab-interval` nor `tab-stops` states one, so a toolkit — which puts its own default stops
/// every so many spaces — is handed the text without its tab characters there.
#[must_use]
pub fn advances(paragraph: &pdf_model::popup::RichParagraph) -> bool {
    !paragraph.tab_stops.is_empty()
        || paragraph
            .tab_interval
            .is_some_and(|interval| interval.per_base != 0.0 || interval.points != 0.0)
}

/// Whether a paragraph holds a tab.
#[must_use]
pub fn tabbed(paragraph: &pdf_model::popup::RichParagraph) -> bool {
    paragraph.runs.iter().any(|run| run.text.contains('\t'))
}

/// Which of a paragraph's stops a tab that starts at `from` reaches: the nearest beyond it in the
/// direction the paragraph reads — the first right of it, or in a paragraph read right to left the
/// first left of it — by its place in `positions`, the stops' distances from the left margin,
/// nearest it first.
///
/// Chapter 2's *Tab Stops* has a tab advance to the next stop (page 61), and it is the rule both
/// toolkits lay a tab out by, which is why a window that paints a leader over a toolkit's line
/// asks it of the tab's own extent rather than counting tabs: a run longer than the room before a
/// stop carries its tab on to the one after. `None` where no stop lies beyond, the toolkit's own
/// default advance, which [`toolkit_unapplied`] says.
#[must_use]
pub fn reached_stop(positions: &[f32], from: f32, right_to_left: bool) -> Option<usize> {
    if right_to_left {
        positions.iter().rposition(|at| *at < from - REACH_SLACK)
    } else {
        positions.iter().position(|at| *at > from + REACH_SLACK)
    }
}

/// How far a tab must start short of a stop for the stop to be the one it reaches: Pango takes a
/// stop at least one of its units (a thousand-and-twenty-fourth of a pixel) past the line's
/// width, and a stop a window rounded to a whole pixel is within half of one of where it is placed
/// here.
const REACH_SLACK: f32 = 0.01;

/// The most cycles one leader draws: a window's width over the narrowest glyph is far below it,
/// and a pattern of a hair's width is not a reason to draw a million of them.
const MOST_CYCLES: usize = 4096;

/// The share of a dashed rule's cycle a dash takes, and its cycle in thicknesses; a dotted rule's
/// cycle is a square dot and its own width of gap. Chapter 2 states neither, and these are a
/// field appearance's choices (ADR 1660), kept so that one leader is not drawn two ways in one
/// program.
const DASH_CYCLE: f32 = 4.0;
/// A dotted rule's cycle, in thicknesses.
const DOT_CYCLE: f32 = 2.0;

/// Where each whole repetition of a leader starts across the room a tab advanced, `from` to
/// `to`: chapter 2's *Tab Leader Pattern* (pages 63 to 65) as a field's appearance draws it (ADR
/// 1660), in whatever unit the three are in.
///
/// The repetitions are laid on a grid from the paragraph's left margin, `margin`, so that leaders
/// on different lines line up — `leaderAlignment`'s `none` leaves that to the processor — and one
/// the room cannot hold whole is left blank, as the chapter has a processor leave a partial one.
/// `cycle` is the larger of `leaderPatternWidth` and the pattern's own width, which only the
/// window that set the pattern can measure. The three windows draw by this one grid (ADR 1722).
#[must_use]
pub fn leader_cycles(from: f32, to: f32, margin: f32, cycle: f32) -> Vec<f32> {
    if !cycle.is_finite() || cycle <= 0.0 || !from.is_finite() || !to.is_finite() || to <= from {
        return Vec::new();
    }
    let mut index = ((from - margin) / cycle).ceil();
    let mut out = Vec::new();
    while margin + (index + 1.0) * cycle <= to + f32::EPSILON && out.len() < MOST_CYCLES {
        out.push(margin + index * cycle);
        index += 1.0;
    }
    out
}

/// A rule leader's thickness: the one it states, or where it states none the thickness this
/// program draws an underline at, a twentieth of the run's em and never under one pixel — `em`
/// and the answer in the window's pixels (ADR 1679).
#[must_use]
pub fn rule_thickness(stated: Option<f32>, em: f32) -> f32 {
    stated.unwrap_or((em * 0.05).max(1.0)).max(0.0)
}

/// A rule leader across `from` to `to`, as `(left, width)` pieces: one for a solid rule, a dash
/// of two thicknesses every four for a dashed one and a square dot every two for a dotted one, on
/// [`leader_cycles`]' grid from `margin` (ADR 1660's pieces).
#[must_use]
pub fn rule_pieces(
    style: pdf_model::popup::RichRuleStyle,
    thickness: f32,
    (from, to): (f32, f32),
    margin: f32,
) -> Vec<(f32, f32)> {
    use pdf_model::popup::RichRuleStyle;
    if !thickness.is_finite()
        || thickness <= 0.0
        || !from.is_finite()
        || !to.is_finite()
        || to <= from
    {
        return Vec::new();
    }
    match style {
        RichRuleStyle::Solid => vec![(from, to - from)],
        RichRuleStyle::Dashed => leader_cycles(from, to, margin, DASH_CYCLE * thickness)
            .into_iter()
            .map(|x| (x, DASH_CYCLE * thickness * 0.5))
            .collect(),
        RichRuleStyle::Dotted => leader_cycles(from, to, margin, DOT_CYCLE * thickness)
            .into_iter()
            .map(|x| (x, thickness))
            .collect(),
    }
}

/// What a window that sets a note through a toolkit says it did not draw, beside
/// `pdf_model::popup::RichNote::unapplied` (ADRs 1654, 1666, 1690, 1722). `quorra` lays its lines
/// out itself and draws all of these; neither toolkit can draw the first.
///
/// - **A decimal tab in a right-to-left paragraph.** Both toolkits reach a right-to-left
///   paragraph's stops leftward once each is handed as its distance from the line's start edge
///   ([`from_start_edge`]), but both place a decimal stop's text as though what is stored before
///   its full stop stood on the stop's right: a number, which reads left to right inside the
///   paragraph, has its integer digits there instead of on the left (measured: `123.45` at a
///   decimal stop standing 300 px from a line's left edge put its full stop at 306 px in Pango and
///   304 px in Qt, where a stop at 100 px in a paragraph read left to right put it at 98 and
///   100 px; ADR 1690).
/// - **A font scale**, where `scales` is false: Pango's attributes state none per run (its
///   `font_stretch` chooses a face's width, it does not scale one), so `quorra-gtk` passes false;
///   Qt's `QTextCharFormat::setFontStretch` scales, and `quorra-qt` sets it.
/// - **A tab past a paragraph's last stated stop**, where the paragraph states stops and no
///   `tab-interval`: the chapter sets no default stop there and so no advance, and both toolkits
///   put one of their own — Pango repeats the last spacing, Qt its default distance. Whether a tab
///   reaches that far is decided by the toolkit's line, so the sentence names the case rather than
///   a count of it.
#[must_use]
pub fn toolkit_unapplied(note: &pdf_model::popup::RichNote, scales: bool) -> Vec<String> {
    let runs = || {
        note.paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.tag.iter().chain(&paragraph.runs))
    };
    let mut said = Vec::new();
    if note.paragraphs.iter().any(|paragraph| {
        tabbed(paragraph)
            && right_to_left(paragraph)
            && paragraph
                .tab_stops
                .iter()
                .any(|stop| stop.align == pdf_model::popup::RichTabAlign::Decimal)
    }) {
        said.push("a decimal tab in a right-to-left paragraph".to_owned());
    }
    if !scales && runs().any(scaled) {
        said.push("a font scale in a popup window".to_owned());
    }
    if note.paragraphs.iter().any(|paragraph| {
        tabbed(paragraph) && paragraph.tab_interval.is_none() && !paragraph.tab_stops.is_empty()
    }) {
        said.push("a tab past the last stated stop".to_owned());
    }
    said
}

/// The two numbers a toolkit that sizes a face by its height and scales its width sets a run in,
/// for chapter 27's two font scales: the face's size, `size` times the vertical scale, and how
/// wide its glyphs are drawn as a percentage of that face's own, the horizontal scale over the
/// vertical — so that a glyph is `em × vertical` tall and advances `width × em × horizontal`, the
/// reading `quorra` and a field's appearance draw by (ADR 1654).
#[must_use]
pub fn scaled_face(run: &pdf_model::popup::RichRun, size: f32) -> (f32, f32) {
    let vertical = if run.vertical_scale.is_finite() && run.vertical_scale > 0.0 {
        run.vertical_scale
    } else {
        1.0
    };
    let horizontal = if run.horizontal_scale.is_finite() && run.horizontal_scale > 0.0 {
        run.horizontal_scale
    } else {
        1.0
    };
    (size * vertical, horizontal / vertical * 100.0)
}

/// The first family a run's `font-family` names that a toolkit may be handed, or `None` for the
/// window's own face.
///
/// Chapter 27's `font-family` is a search path (page 1201); a toolkit takes one name and finds its
/// own nearest face, so the nearest is what crosses. **A name is handed over only where every
/// character of it is a letter, a digit, a space, a hyphen or a low line**: the name is the
/// document's, and it goes into a Pango markup attribute and into Qt's rich text, where a quote
/// or a semicolon would end the attribute and start one the document wrote. CSS2's generic
/// families cross as themselves, which both toolkits understand.
#[must_use]
pub fn family(run: &pdf_model::popup::RichRun) -> Option<&str> {
    run.families.iter().map(String::as_str).find(|name| {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_')
    })
}

/// The sentence a window says under a rich note about what it did not draw, or `None` where it
/// drew everything the note states.
///
/// `pdf_model::popup::RichNote::unapplied` is the list — chapter 27's properties this program
/// does not carry out — and `also` is what this host adds of its own, such as a face its chrome
/// cannot set. Said rather than dropped, because a formatting a person cannot see and is not told
/// about is trap 5 in an interface.
#[must_use]
pub fn not_drawn(note: &pdf_model::popup::RichNote, also: &[String]) -> Option<String> {
    let all: Vec<&str> = note
        .unapplied
        .iter()
        .chain(also)
        .map(String::as_str)
        .collect();
    (!all.is_empty()).then(|| format!("[formatting not drawn: {}]", all.join(", ")))
}

/// A rich note as Qt's rich text spells it, for a host that hands a toolkit one string.
///
/// `base` is the window's text size in points. Every character of the note is escaped and every
/// attribute is one this function writes — a size and a rise in points, a colour as six hex
/// digits, a family [`family`] has passed — so the string carries no markup the document wrote:
/// no element, no image, no link a label could follow. A `'\n'` is a `br`, a paragraph a `p`
/// with its alignment, a list item its tag and a margin per level.
#[must_use]
pub fn html(note: &pdf_model::popup::RichNote, base: f32) -> String {
    let mut out = String::new();
    html_into(&mut out, note, base, 0.0);
    out
}

/// §12.5.6.2's thread in Qt's rich text, where a reply in it states a rich note — `None` where
/// none does, and [`thread`]'s plain block is then the whole of it.
///
/// [`thread`]'s layout in markup: each reply's author on its own line, dimmed, and its note under
/// that, indented two base sizes per `/IRT` hop up to four. A reply with no `/RC` is its plain
/// text, escaped.
#[must_use]
pub fn thread_html(window: &Window<'_>, base: f32) -> Option<String> {
    use std::fmt::Write as _;
    if window.replies.iter().all(|reply| reply.rich.is_none()) {
        return None;
    }
    let mut out = String::new();
    for reply in window.replies {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a depth held at four is exactly representable"
        )]
        let indent = reply.depth.min(4) as f32 * base * 2.0;
        if let Some(who) = reply.title.as_deref().filter(|who| !who.is_empty()) {
            let _ = write!(
                out,
                "<p style=\"margin-top:0; margin-bottom:0; margin-left:{indent:.1}pt; \
                 color:#777777\">"
            );
            escape_into(&mut out, who);
            out.push_str("</p>");
        }
        if let Some(note) = reply.rich.as_ref() {
            html_into(&mut out, note, base, indent);
        } else {
            let _ = write!(
                out,
                "<p style=\"margin-top:0; margin-bottom:0; margin-left:{indent:.1}pt\">"
            );
            escape_into(&mut out, reply.text.as_deref().unwrap_or_default());
            out.push_str("</p>");
        }
    }
    Some(out)
}

/// Escapes a document's characters for Qt's rich text, a line end as a `br`.
fn escape_into(out: &mut String, text: &str) {
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\r' => {}
            '\n' => out.push_str("<br/>"),
            other => out.push(other),
        }
    }
}

/// [`html`], every paragraph `indent` points further in.
fn html_into(out: &mut String, note: &pdf_model::popup::RichNote, base: f32, indent: f32) {
    use std::fmt::Write as _;
    for paragraph in &note.paragraphs {
        // A paragraph stating no `text-align` takes none, so Qt starts it at its own start edge —
        // the right, for one UAX #9 finds reading right to left — as Pango and `quorra` do (ADR
        // 1654).
        let align = match paragraph.align {
            None => "",
            Some(pdf_model::popup::RichAlign::Left) => " align=\"left\"",
            Some(pdf_model::popup::RichAlign::Centre) => " align=\"center\"",
            Some(pdf_model::popup::RichAlign::Right) => " align=\"right\"",
            Some(pdf_model::popup::RichAlign::Justify) => " align=\"justify\"",
        };
        let indent = f32::from(paragraph.level).mul_add(base * 2.0, indent);
        let _ = write!(
            out,
            "<p{align} style=\"margin-top:0; margin-bottom:0; margin-left:{indent:.1}pt\">"
        );
        if let Some(tag) = &paragraph.tag {
            span(out, tag, base);
            out.push(' ');
        }
        for run in &paragraph.runs {
            span(out, run, base);
        }
        out.push_str("</p>");
    }
}

/// One run as a Qt rich text `span`.
fn span(out: &mut String, run: &pdf_model::popup::RichRun, base: f32) {
    use std::fmt::Write as _;
    let _ = write!(out, "<span style=\"font-size:{:.2}pt", size(run, base, 1.0));
    if let Some(name) = family(run) {
        let _ = write!(out, "; font-family:'{name}'");
    }
    out.push_str(if run.bold {
        "; font-weight:700"
    } else {
        "; font-weight:400"
    });
    if run.italic {
        out.push_str("; font-style:italic");
    }
    if let Some(colour) = run.colour {
        let _ = write!(out, "; color:#{}", hex(colour));
    }
    match (run.underlines > 0, run.line_through) {
        (true, true) => out.push_str("; text-decoration:underline line-through"),
        (true, false) => out.push_str("; text-decoration:underline"),
        (false, true) => out.push_str("; text-decoration:line-through"),
        (false, false) => {}
    }
    if let Some(spacing) = letter_spacing(run, base, 1.0, None).filter(|spacing| *spacing != 0.0) {
        // Qt's rich text reads `letter-spacing` in pixels, its reference pixel being CSS2's, 96 to
        // the inch.
        let _ = write!(out, "; letter-spacing:{:.2}px", spacing * 96.0 / 72.0);
    }
    let raised = rise(run, base, 1.0);
    if raised > 0.0 {
        out.push_str("; vertical-align:super");
    } else if raised < 0.0 {
        out.push_str("; vertical-align:sub");
    }
    out.push_str("\">");
    escape_into(out, &run.text);
    out.push_str("</span>");
}

/// An sRGB colour as six hex digits.
#[must_use]
pub fn hex(colour: pdf_render::Color) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "each component is clamped to 0..=1 and scaled to 0..=255 first"
    )]
    let level = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "{:02x}{:02x}{:02x}",
        level(colour.r),
        level(colour.g),
        level(colour.b)
    )
}

/// Table 166's `/M` as [`Window::modified`] should be shown, or `None` where there is none.
///
/// Separate from [`windows`] because it allocates and the rest of a window does not: a host asks
/// for the whole list on every repaint and for the timestamp only where it has room to draw one.
#[must_use]
pub fn modified(window: &Window<'_>) -> Option<String> {
    let written = window.modified?.to_owned();
    crate::stamp(pdf_syntax::Date::parse(&written), Some(&written))
}

#[cfg(test)]
mod tests {
    use super::{modified, windows};
    use viewer_core::PopupWindow;

    /// A run as the tests below state one: the window's own size and nothing else.
    fn run(text: &str) -> pdf_model::popup::RichRun {
        pdf_model::popup::RichRun {
            text: text.to_owned(),
            families: Vec::new(),
            size: pdf_model::popup::Measure {
                per_base: 1.0,
                points: 0.0,
            },
            bold: false,
            italic: false,
            colour: None,
            underlines: 0,
            underline_by_word: false,
            line_through: false,
            rise: pdf_model::popup::Measure::default(),
            letter_spacing: pdf_model::popup::RichSpacing::default(),
            horizontal_scale: 1.0,
            vertical_scale: 1.0,
        }
    }

    /// A letter spacing in points is points whatever the base and pixels where the host draws
    /// in them; one given as a share of a space is resolved only where the face is known, and
    /// either is held within the run's size.
    #[test]
    fn a_runs_letter_spacing_is_resolved_where_its_unit_can_be() {
        let mut spaced = run("x");
        spaced.letter_spacing = pdf_model::popup::RichSpacing::Length(pdf_model::popup::Measure {
            per_base: 0.0,
            points: 3.0,
        });
        assert_eq!(super::letter_spacing(&spaced, 10.0, 2.0, None), Some(6.0));
        spaced.letter_spacing = pdf_model::popup::RichSpacing::OfSpace(0.5);
        assert_eq!(super::letter_spacing(&spaced, 10.0, 1.0, None), None);
        let resolved = super::letter_spacing(&spaced, 10.0, 1.0, Some(0.25)).unwrap_or_default();
        assert!((resolved - 1.25).abs() < 1e-5, "{resolved}");
        spaced.letter_spacing = pdf_model::popup::RichSpacing::Length(pdf_model::popup::Measure {
            per_base: 0.0,
            points: 400.0,
        });
        assert_eq!(
            super::letter_spacing(&spaced, 10.0, 1.0, None),
            Some(10.0),
            "held at the size"
        );
        let note = pdf_model::popup::RichNote {
            paragraphs: vec![pdf_model::popup::RichParagraph {
                align: None,
                level: 0,
                tag: None,
                runs: vec![spaced],
                tab_interval: None,
                tab_stops: Vec::new(),
            }],
            unapplied: Vec::new(),
        };
        assert!(super::html(&note, 10.0).contains("letter-spacing:13.33px"));
        assert!(super::toolkit_unapplied(&note, false).is_empty());
    }

    /// A toolkit's label states no font scale, so a note with one says so; one without says nothing.
    #[test]
    fn a_toolkit_window_says_a_font_scale_it_cannot_set() {
        let mut wide = run("x");
        wide.horizontal_scale = 2.0;
        let note = |runs| pdf_model::popup::RichNote {
            paragraphs: vec![pdf_model::popup::RichParagraph {
                align: None,
                level: 0,
                tag: None,
                runs,
                tab_interval: None,
                tab_stops: Vec::new(),
            }],
            unapplied: Vec::new(),
        };
        assert_eq!(
            super::toolkit_unapplied(&note(vec![wide]), false),
            vec!["a font scale in a popup window".to_owned()]
        );
        assert!(super::toolkit_unapplied(&note(vec![run("x")]), false).is_empty());
    }

    /// A toolkit that sets a font scale says none, and the face it is set in is the run's size
    /// under the vertical scale and drawn the horizontal over the vertical wide.
    #[test]
    fn a_toolkit_that_scales_is_handed_a_size_and_a_stretch() {
        let mut wide = run("x");
        wide.horizontal_scale = 2.0;
        wide.vertical_scale = 0.5;
        let (points, stretch) = super::scaled_face(&wide, 20.0);
        assert!((points - 10.0).abs() < 1e-5, "{points}");
        assert!((stretch - 400.0).abs() < 1e-3, "{stretch}");
        let note = pdf_model::popup::RichNote {
            paragraphs: vec![pdf_model::popup::RichParagraph {
                align: None,
                level: 0,
                tag: None,
                runs: vec![wide],
                tab_interval: None,
                tab_stops: Vec::new(),
            }],
            unapplied: Vec::new(),
        };
        assert!(super::toolkit_unapplied(&note, true).is_empty());
    }

    /// Chapter 27's *Tab Stops*: the stated stops in order of position, then the default ones at
    /// every multiple of `tab-interval` past the last stated, aligned at their start edge; a
    /// paragraph stating neither has no stop and its tabs advance by nothing.
    #[test]
    fn a_paragraphs_tab_stops_are_the_stated_then_every_interval() {
        use pdf_model::popup::{Measure, RichTabAlign, RichTabStop};
        let points = |points| Measure {
            per_base: 0.0,
            points,
        };
        let mut paragraph = pdf_model::popup::RichParagraph {
            align: None,
            level: 0,
            tag: None,
            runs: vec![run("a\tb")],
            tab_interval: None,
            tab_stops: Vec::new(),
        };
        assert!(!super::advances(&paragraph));
        assert!(super::tab_stops(&paragraph, 10.0, 1.0, 500.0).is_empty());
        paragraph.tab_stops = vec![
            RichTabStop {
                align: RichTabAlign::Decimal,
                at: points(100.0),
                leader: None,
            },
            RichTabStop {
                align: RichTabAlign::Right,
                at: Measure {
                    per_base: 3.0,
                    points: 0.0,
                },
                leader: None,
            },
        ];
        paragraph.tab_interval = Some(points(72.0));
        assert!(super::advances(&paragraph));
        let stops = super::tab_stops(&paragraph, 10.0, 2.0, 500.0);
        let placed: Vec<(super::TabSide, f32)> =
            stops.iter().map(|stop| (stop.side, stop.at)).collect();
        assert_eq!(
            placed,
            vec![
                (super::TabSide::Right, 30.0),
                (super::TabSide::Decimal, 200.0),
                (super::TabSide::Left, 288.0),
                (super::TabSide::Left, 432.0),
            ]
        );
        let note = pdf_model::popup::RichNote {
            paragraphs: vec![paragraph],
            unapplied: Vec::new(),
        };
        assert!(super::toolkit_unapplied(&note, true).is_empty());
        let mut unbounded = note.clone();
        if let Some(paragraph) = unbounded.paragraphs.first_mut() {
            paragraph.tab_interval = None;
        }
        assert_eq!(
            super::toolkit_unapplied(&unbounded, true),
            vec!["a tab past the last stated stop".to_owned()]
        );
    }

    /// Chapter 2's *Tab Stops* has a tab in a paragraph read right to left reach leftward (page
    /// 61), so the default stops lie left of the leftmost stated one, every `tab-interval` from
    /// the margin and never at it, aligned at their start edge — the right in such a paragraph; a
    /// stated stop keeps its leader and a default one is blank (ADR 1679).
    #[test]
    fn a_right_to_left_paragraphs_default_stops_lie_left_of_its_stated_ones() {
        use pdf_model::popup::{Measure, RichLeader, RichLeaderPattern, RichTabAlign, RichTabStop};
        let points = |points| Measure {
            per_base: 0.0,
            points,
        };
        let dots = RichLeader {
            pattern: RichLeaderPattern::Dots,
            width: Some(points(6.0)),
        };
        let paragraph = pdf_model::popup::RichParagraph {
            align: None,
            level: 0,
            tag: None,
            runs: vec![run("\u{5d0}\t\u{5d1}")],
            tab_interval: Some(points(50.0)),
            tab_stops: vec![RichTabStop {
                align: RichTabAlign::Before,
                at: points(180.0),
                leader: Some(dots),
            }],
        };
        let stops = super::tab_stops(&paragraph, 10.0, 1.0, 400.0);
        let placed: Vec<(super::TabSide, f32, bool)> = stops
            .iter()
            .map(|stop| (stop.side, stop.at, stop.leader.is_some()))
            .collect();
        assert_eq!(
            placed,
            vec![
                (super::TabSide::Right, 50.0, false),
                (super::TabSide::Right, 100.0, false),
                (super::TabSide::Right, 150.0, false),
                (super::TabSide::Left, 180.0, true),
            ]
        );
        assert_eq!(
            stops.last().and_then(|stop| stop.leader.as_ref()),
            Some(&super::Leader {
                pattern: super::LeaderPattern::Dots,
                width: 6.0,
            })
        );
        let note = pdf_model::popup::RichNote {
            paragraphs: vec![paragraph],
            unapplied: Vec::new(),
        };
        // Every stop above is reached leftward by both toolkits once handed from the line's
        // start edge (ADR 1690), and both paint the leader over their line (ADR 1722), so nothing
        // is said; the C ABI hands the leader over for its caller to paint (ADR 1726).
        assert!(super::toolkit_unapplied(&note, true).is_empty());
    }

    /// A tab reaches the nearest stop beyond where it starts, in the direction its paragraph
    /// reads (chapter 2's *Tab Stops*, page 61): from 120 that is 150 rightward and 100
    /// leftward, a tab starting on a stop goes on to the next, and none lies beyond the last.
    #[test]
    fn a_tab_reaches_the_nearest_stop_beyond_it_in_its_paragraphs_direction() {
        let stops = [50.0, 100.0, 150.0];
        assert_eq!(super::reached_stop(&stops, 120.0, false), Some(2));
        assert_eq!(super::reached_stop(&stops, 120.0, true), Some(1));
        assert_eq!(super::reached_stop(&stops, 100.0, false), Some(2));
        assert_eq!(super::reached_stop(&stops, 100.0, true), Some(0));
        assert_eq!(super::reached_stop(&stops, 150.0, false), None);
        assert_eq!(super::reached_stop(&stops, 50.0, true), None);
    }

    /// A leader's repetitions lie on a grid from the paragraph's left margin and a partial one is
    /// left blank (chapter 2's *Tab Leader Pattern*): a 6-unit cycle from a margin at 10 across
    /// 25 to 50 starts at 28, 34 and 40, and 46 would end past 50.
    #[test]
    fn a_leaders_cycles_lie_on_the_margins_grid_and_a_partial_one_is_blank() {
        assert_eq!(
            super::leader_cycles(25.0, 50.0, 10.0, 6.0),
            vec![28.0, 34.0, 40.0]
        );
        assert!(super::leader_cycles(25.0, 30.0, 10.0, 6.0).is_empty());
        assert!(super::leader_cycles(25.0, 50.0, 10.0, 0.0).is_empty());
        assert!(super::leader_cycles(50.0, 25.0, 10.0, 6.0).is_empty());
    }

    /// A rule leader is one piece solid, a dash of two thicknesses every four dashed and a square
    /// dot every two dotted, on the same grid; with no thickness stated it is a twentieth of the
    /// em, never under a pixel.
    #[test]
    fn a_rule_leader_is_drawn_in_its_styles_pieces() {
        use pdf_model::popup::RichRuleStyle;
        assert_eq!(
            super::rule_pieces(RichRuleStyle::Solid, 2.0, (10.0, 30.0), 0.0),
            vec![(10.0, 20.0)]
        );
        assert_eq!(
            super::rule_pieces(RichRuleStyle::Dashed, 2.0, (10.0, 34.0), 0.0),
            vec![(16.0, 4.0), (24.0, 4.0)]
        );
        assert_eq!(
            super::rule_pieces(RichRuleStyle::Dotted, 2.0, (10.0, 20.0), 0.0),
            vec![(12.0, 2.0), (16.0, 2.0)]
        );
        assert!(super::rule_pieces(RichRuleStyle::Solid, 0.0, (10.0, 30.0), 0.0).is_empty());
        assert!((super::rule_thickness(None, 40.0) - 2.0).abs() < f32::EPSILON);
        assert!((super::rule_thickness(None, 10.0) - 1.0).abs() < f32::EPSILON);
        assert!((super::rule_thickness(Some(3.0), 10.0) - 3.0).abs() < f32::EPSILON);
    }

    /// A right-to-left paragraph's stops reach a toolkit as distances from the line's start edge,
    /// nearest it first, and a stop at or right of that edge is reached by no tab (ADR 1690):
    /// stops at 50, 100, 150 and 180 points in a line whose right edge is 160 points from the left
    /// margin stand 10, 60 and 110 points from it.
    #[test]
    fn a_right_to_left_paragraphs_stops_are_handed_from_the_lines_start_edge() {
        let stop = |side, at| super::TabStop {
            side,
            at,
            leader: None,
        };
        let stops = [
            stop(super::TabSide::Right, 50.0),
            stop(super::TabSide::Centre, 100.0),
            stop(super::TabSide::Left, 150.0),
            stop(super::TabSide::Right, 180.0),
        ];
        assert_eq!(
            super::from_start_edge(&stops, 160.0),
            vec![
                stop(super::TabSide::Left, 10.0),
                stop(super::TabSide::Centre, 60.0),
                stop(super::TabSide::Right, 110.0),
            ]
        );
        assert_eq!(super::from_start_edge(&stops, 50.0), Vec::new());
    }

    /// A decimal stop in a right-to-left paragraph is the one leftward tab neither toolkit places
    /// (ADR 1690), and a decimal stop in a paragraph read left to right is not said.
    #[test]
    fn a_decimal_tab_is_said_only_where_the_paragraph_reads_right_to_left() {
        use pdf_model::popup::{Measure, RichTabAlign, RichTabStop};
        let paragraph = |text: &str| pdf_model::popup::RichParagraph {
            align: None,
            level: 0,
            tag: None,
            runs: vec![run(text)],
            tab_interval: Some(Measure {
                per_base: 0.0,
                points: 50.0,
            }),
            tab_stops: vec![RichTabStop {
                align: RichTabAlign::Decimal,
                at: Measure {
                    per_base: 0.0,
                    points: 120.0,
                },
                leader: None,
            }],
        };
        let said = |text: &str| {
            super::toolkit_unapplied(
                &pdf_model::popup::RichNote {
                    paragraphs: vec![paragraph(text)],
                    unapplied: Vec::new(),
                },
                true,
            )
        };
        assert_eq!(
            said("\u{5d0}\t12.5"),
            vec!["a decimal tab in a right-to-left paragraph".to_owned()]
        );
        assert_eq!(said("a\t12.5"), Vec::<String>::new());
    }

    /// The note's characters and family names are the document's, so nothing of them reaches Qt's
    /// rich text as markup: every character is escaped and a family that could end its attribute
    /// is not handed over.
    #[test]
    fn a_rich_note_reaches_qt_as_escaped_text_in_spans_this_program_wrote() {
        let mut hostile = run("<img src=\"/etc/passwd\"> & more\nnext");
        hostile.families = vec!["x'; color:red".to_owned(), "Times New Roman".to_owned()];
        hostile.bold = true;
        let note = pdf_model::popup::RichNote {
            paragraphs: vec![pdf_model::popup::RichParagraph {
                align: Some(pdf_model::popup::RichAlign::Centre),
                level: 0,
                tag: None,
                runs: vec![hostile],
                tab_interval: None,
                tab_stops: Vec::new(),
            }],
            unapplied: Vec::new(),
        };
        let html = super::html(&note, 10.0);
        assert!(!html.contains("<img"), "{html}");
        assert!(html.contains("&lt;img src=&quot;/etc/passwd&quot;&gt; &amp; more<br/>next"));
        assert!(html.contains("font-family:'Times New Roman'"), "{html}");
        assert!(!html.contains("color:red"), "{html}");
        assert!(html.contains("align=\"center\""));
        assert!(html.contains("font-weight:700"));
    }

    /// A size is the base's multiple plus its points, held inside the window's two bounds.
    #[test]
    fn a_runs_size_is_relative_where_stated_so_and_bounded() {
        let mut absolute = run("x");
        absolute.size = pdf_model::popup::Measure {
            per_base: 0.0,
            points: 14.0,
        };
        assert!((super::size(&absolute, 10.0, 1.0) - 14.0).abs() < 1e-4);
        assert!(
            (super::size(&absolute, 10.0, 2.0) - 28.0).abs() < 1e-4,
            "points in pixels"
        );
        absolute.size.points = 500.0;
        assert!(
            (super::size(&absolute, 10.0, 1.0) - 30.0).abs() < 1e-4,
            "held at the largest"
        );
        let mut relative = run("x");
        relative.size.per_base = 0.66;
        assert!((super::size(&relative, 20.0, 1.0) - 13.2).abs() < 1e-4);
    }

    /// One window whose `/Rect` is an upright box `wide` by `tall` at the origin.
    fn window(wide: f32, tall: f32) -> PopupWindow {
        PopupWindow {
            annotation: pdf_syntax::ObjectId::new(1, 0),
            parent: None,
            note: None,
            quad: [0.0, 0.0, wide, 0.0, wide, tall, 0.0, tall],
            title: Some("A Reader".to_owned()),
            text: Some("a note".to_owned()),
            modified: Some("D:20240102030405Z".to_owned()),
            subject: None,
            created: None,
            colour: None,
            rich: None,
            replies: Vec::new(),
        }
    }

    #[test]
    fn a_window_with_area_is_placed_where_the_answer_put_it() {
        let answer = [window(120.0, 60.0)];
        let placed = windows(&answer);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].place, (0.0, 0.0, 120.0, 60.0));
        assert_eq!(placed[0].title, "A Reader");
        assert_eq!(placed[0].text, "a note");
    }

    /// Table 166 requires `/Rect`; a rectangle with no area is a window a person cannot see.
    #[test]
    fn a_window_with_no_area_is_not_placed() {
        assert!(windows(&[window(0.0, 60.0)]).is_empty());
        assert!(windows(&[window(120.0, 0.0)]).is_empty());
    }

    /// §12.5.6.2's `/T` is optional and names a person, so nothing is invented for it.
    #[test]
    fn an_untitled_window_is_headed_with_nothing() {
        let bare = PopupWindow {
            title: None,
            text: None,
            ..window(120.0, 60.0)
        };
        let placed = windows(std::slice::from_ref(&bare));
        assert_eq!(placed[0].title, "");
        assert_eq!(placed[0].text, "");
    }

    /// §7.9.4 where it parses, and Table 166's "any format" where it does not.
    #[test]
    fn the_timestamp_is_the_date_where_there_is_one_and_the_string_where_there_is_not() {
        let answer = [window(120.0, 60.0)];
        let placed = windows(&answer);
        assert_eq!(modified(&placed[0]).as_deref(), Some("2024-01-02 03:04"));
        let odd = PopupWindow {
            modified: Some("last Tuesday".to_owned()),
            ..window(120.0, 60.0)
        };
        let placed = windows(std::slice::from_ref(&odd));
        assert_eq!(modified(&placed[0]).as_deref(), Some("last Tuesday"));
        let none = PopupWindow {
            modified: None,
            ..window(120.0, 60.0)
        };
        let placed = windows(std::slice::from_ref(&none));
        assert_eq!(modified(&placed[0]), None);
    }
}
