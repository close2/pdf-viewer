//! Table 172's `/RC` as a popup window shows it: paragraphs of styled runs, which a host sets in
//! its own toolkit.
//!
//! ISO 32000-2 §12.5.6.2, Table 172:
//!
//! > (Optional; PDF 1.5) A rich text string (see Adobe XML Architecture, XML Forms Architecture
//! > (XFA) Specification, version 3.3 ) that shall be displayed in the popup window when the
//! > annotation is opened.
//!
//! The window is a host's to draw (§12.5.6.14 gives a popup "no appearance stream"), so the
//! string is not laid out here the way `crate::rich_text::lay_out` sets a field's: it is read by
//! the same walk, with the same cascade, and handed over as what each run *is* — its characters,
//! its face's family, weight and posture, its size, its colour, its lines and its rise — for
//! Pango, Qt's rich text and `viewer-ui`'s own chrome to set. Where a line breaks, and in which
//! machine face, is the toolkit's; what the producer specified about each character is this
//! module's (ADR 1642).

use crate::rich_text::parts::{Align, ListIndent, Piece, Spacing, TabAlign};
use crate::rich_text::{Character, RichText};

/// A length a host resolves against its own text size: so many of the window's base size, and so
/// many points beside them.
///
/// §12.5.6.4 gives the processor the choice of "a font and size" for a note's window, so the size
/// a run states nothing about is the host's; chapter 27's relative sizes (`em`, `%`, `sub`'s
/// 66%) are multiples of it and its absolute ones (`14pt`) are points whatever it is. This is
/// `crate::rich_text`'s own split, kept across the boundary so that a host that changes its text
/// size moves every relative length with it and no absolute one.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Measure {
    /// How many of the window's base text size this is.
    pub per_base: f32,
    /// How many points it is beside them.
    pub points: f32,
}

impl Measure {
    /// The length, in points, once the window's base text size is known in points.
    #[must_use]
    pub fn at(self, base: f32) -> f32 {
        self.per_base.mul_add(base, self.points)
    }
}

/// Chapter 27's `letter-spacing` (page 1204), in the unit it was stated in.
///
/// Chapter 27 makes it a relative measurement, and a percentage is of the width of a space in the
/// face the run is set in — which is the host's to choose (ADR 1642), so the share crosses as a
/// share and the host that picked the face resolves it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RichSpacing {
    /// A length, resolved as [`Measure::at`] resolves one.
    Length(Measure),
    /// So many of the width of a space in the run's face.
    OfSpace(f32),
}

impl Default for RichSpacing {
    fn default() -> Self {
        Self::Length(Measure::default())
    }
}

/// Chapter 27's `text-align` for one paragraph (page 1190).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RichAlign {
    /// `left`.
    Left,
    /// `center`.
    Centre,
    /// `right`.
    Right,
    /// `justify`, and chapter 27's `justify-all`: a toolkit justifies every line but a
    /// paragraph's last, and the last line of a popup's paragraph is not worth a difference.
    Justify,
}

/// Characters set in one style.
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "four independent properties chapter 27 names — weight, posture, an underline's \
              spacing and a line through — which no one enumeration of states would describe"
)]
pub struct RichRun {
    /// The characters, white space resolved as chapter 27 lays it out; a `'\n'` is XHTML's `br`,
    /// where the line ends whatever the width.
    pub text: String,
    /// `font-family`'s search path, nearest first; empty where nothing names one, which is the
    /// window's own face.
    pub families: Vec<String>,
    /// `font-size`: the height of the em.
    pub size: Measure,
    /// Whether the run is bold: CSS2 section 15.5.1's weight above 500.
    pub bold: bool,
    /// `font-style` other than `normal`.
    pub italic: bool,
    /// `color`, in sRGB; `None` is the window's own text colour.
    pub colour: Option<pdf_render::Color>,
    /// How many underlines `text-decoration` draws: 0, 1 or 2 (page 1208).
    pub underlines: u8,
    /// Whether the underline skips the spaces between words.
    pub underline_by_word: bool,
    /// `text-decoration`'s `line-through`.
    pub line_through: bool,
    /// How far the baseline sits above the line's: `vertical-align`, `sub` and `sup`.
    pub rise: Measure,
    /// `letter-spacing`, added after every character.
    pub letter_spacing: RichSpacing,
    /// `xfa-font-horizontal-scale`, as a factor: the glyphs' width over the face's (page 1202).
    pub horizontal_scale: f32,
    /// `xfa-font-vertical-scale`, as a factor: the glyphs' height over the em's.
    pub vertical_scale: f32,
}

/// How the text after a tab stands at its stop (chapter 2's *Tab Stops* table, page 62, which
/// chapter 27's *Tab Stops* sends `xfa-tab-stops` to).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RichTabAlign {
    /// The text's left edge at the stop.
    Left,
    /// The text centred on the stop.
    Centre,
    /// The text's right edge at the stop.
    Right,
    /// The text's first full stop at the stop, and its right edge where it has none.
    Decimal,
    /// The edge the text starts from: its left in a paragraph read left to right, its right in
    /// one read right to left — every default stop's alignment (page 1205).
    After,
    /// The edge the text ends at, the other way round.
    Before,
}

/// One tab stop a paragraph states: `tab-stops` or `xfa-tab-stops` (page 1205).
#[derive(Debug, Clone, PartialEq)]
pub struct RichTabStop {
    /// How the text after the tab stands at it.
    pub align: RichTabAlign,
    /// Its distance from the paragraph's left margin.
    pub at: Measure,
    /// What fills the room before the text at the stop; `None` for a blank leader and for a stop
    /// that states none (chapter 2's *Tab Leader Pattern*, pages 63 to 65; ADR 1679).
    pub leader: Option<RichLeader>,
}

/// A tab leader as a host draws it: what is repeated across the room before a stop, and the least
/// width of one repetition.
#[derive(Debug, Clone, PartialEq)]
pub struct RichLeader {
    /// What is repeated.
    pub pattern: RichLeaderPattern,
    /// `leaderPatternWidth`, the least repetition width; the cycle is the larger of this and the
    /// pattern's own width.
    pub width: Option<Measure>,
}

/// A leader's pattern: chapter 2's `dots`, `rule` and `use-content`.
#[derive(Debug, Clone, PartialEq)]
pub enum RichLeaderPattern {
    /// `dots()`, drawn as the run's own full stop, as a field's appearance draws it.
    Dots,
    /// `rule(ruleStyle [ruleThickness])`, in the text's colour.
    Rule {
        /// How the rule is broken.
        style: RichRuleStyle,
        /// `ruleThickness`; `None` takes the underline's.
        thickness: Option<Measure>,
    },
    /// `use-content(content)`: the characters repeated as many whole times as fit.
    Content(String),
}

/// A rule leader's `ruleStyle`, `double`, `groove` and `ridge` read as solid, which the chapter
/// permits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RichRuleStyle {
    /// One unbroken line.
    Solid,
    /// Dashes.
    Dashed,
    /// Dots.
    Dotted,
}

/// One paragraph of a note: chapter 27's `p`, or a list item.
#[derive(Debug, Clone, PartialEq)]
pub struct RichParagraph {
    /// `text-align`, where the string states one; `None` is the window's own.
    pub align: Option<RichAlign>,
    /// How many lists the paragraph is nested in, each one list indent (page 1218).
    pub level: u16,
    /// A list item's tag — its number with its suffix, or its bullet — in the item's own style.
    /// Generated by the list rather than stated by the string, so it is not among the runs.
    pub tag: Option<RichRun>,
    /// What the paragraph says. A `'\t'` in a run's text is chapter 27's `xfa-tab-count`, one
    /// per stop it advances by: white space resolution leaves no tab character of the string's.
    pub runs: Vec<RichRun>,
    /// `tab-interval`: the distance between default stops, from the left margin; `None` where the
    /// paragraph states none, and then no default stop is set (*Tab Stops*, page 1205).
    pub tab_interval: Option<Measure>,
    /// The stops stated at positions, in the order stated.
    pub tab_stops: Vec<RichTabStop>,
}

/// Table 172's `/RC`, read: what a host draws in the window in place of the plain text.
#[derive(Debug, Clone, PartialEq)]
pub struct RichNote {
    /// The paragraphs, in order.
    pub paragraphs: Vec<RichParagraph>,
    /// What chapter 27 names, the string states and this program does not carry out — each
    /// one a phrase a host says under the note, so that a property dropped is never a property
    /// silently ignored.
    pub unapplied: Vec<String>,
}

/// The note a window shows from Table 172's `/RC`, where it states one this program can read and
/// its characters are the note's.
///
/// `source` is the dictionary §12.5.6.2's group attributes come from — "Contents (or RC and DS )"
/// are among them — and `contents` its Table 166 `/Contents`. **`/Contents` wins where the two
/// disagree**: Table 166 makes it the "[t]ext that shall be displayed for the annotation" and
/// §12.5.6.2's NOTE 1 expects the two to be textually equivalent, so the window shows the plain
/// text of a file that broke that expectation — the rule a free text annotation's appearance
/// already follows (ADR 1635), read through the same function so the two cannot come apart.
/// `None` is a window drawn from [`super::Popup::text`] alone: no `/RC`, an `/RC` that is not a
/// rich text string, or one that disagrees.
pub(super) fn note(
    document: &pdf_syntax::Document,
    source: &pdf_syntax::Dictionary,
    contents: Option<&str>,
) -> Option<RichNote> {
    if document.get_key(source, "RC").is_null() {
        return None;
    }
    let chosen =
        crate::rich_text::for_free_text(document, source, contents, None, &Character::root())?;
    if chosen.disagrees.is_some() {
        return None;
    }
    Some(handed_over(&chosen.rich))
}

/// A read string as the runs a host sets.
fn handed_over(rich: &RichText) -> RichNote {
    let paragraphs = rich
        .paragraphs
        .iter()
        .map(|paragraph| {
            let mut runs: Vec<RichRun> = Vec::new();
            for piece in &paragraph.pieces {
                match piece {
                    Piece::Text(text, style) => runs.push(run(text.clone(), style)),
                    Piece::Break => match runs.last_mut() {
                        Some(last) => last.text.push('\n'),
                        None => runs.push(run("\n".to_owned(), &paragraph.strut)),
                    },
                    // One tab character per stop, which each host sets at the paragraph's stops
                    // (ADR 1666).
                    Piece::Tab(count) => {
                        let tabs = "\t".repeat(usize::from(*count));
                        match runs.last_mut() {
                            Some(last) => last.text.push_str(&tabs),
                            None => runs.push(run(tabs, &paragraph.strut)),
                        }
                    }
                }
            }
            RichParagraph {
                align: paragraph.block.align.map(|align| match align {
                    Align::Left => RichAlign::Left,
                    Align::Centre => RichAlign::Centre,
                    Align::Right => RichAlign::Right,
                    Align::Justify | Align::JustifyAll => RichAlign::Justify,
                }),
                level: match paragraph.list {
                    ListIndent::Levels(levels) => levels,
                    ListIndent::None | ListIndent::Minimal => 0,
                },
                tag: paragraph
                    .tag
                    .as_ref()
                    .map(|tag| run(tag.text.clone(), &tag.style)),
                runs,
                tab_interval: paragraph.block.tab_interval.map(|interval| Measure {
                    per_base: interval.per_root,
                    points: interval.points,
                }),
                tab_stops: paragraph
                    .block
                    .tab_stops
                    .iter()
                    .map(|stop| RichTabStop {
                        align: match stop.align {
                            TabAlign::Left => RichTabAlign::Left,
                            TabAlign::Centre => RichTabAlign::Centre,
                            TabAlign::Right => RichTabAlign::Right,
                            TabAlign::Decimal => RichTabAlign::Decimal,
                            TabAlign::After => RichTabAlign::After,
                            TabAlign::Before => RichTabAlign::Before,
                        },
                        at: Measure {
                            per_base: stop.at.per_root,
                            points: stop.at.points,
                        },
                        leader: stop.leader.as_ref().map(leader),
                    })
                    .collect(),
            }
        })
        .collect();
    // A leader crosses with its stop, and a window that cannot draw one says so itself
    // (ADR 1679).
    let unapplied: Vec<String> = rich.unapplied.0.iter().cloned().collect();
    RichNote {
        paragraphs,
        unapplied,
    }
}

/// A stop's leader as a host reads it, every length a [`Measure`] of the window's base size.
fn leader(leader: &crate::rich_text::parts::Leader) -> RichLeader {
    use crate::rich_text::parts::{LeaderPattern, RuleStyle};
    let measure = |linear: crate::rich_text::parts::Linear| Measure {
        per_base: linear.per_root,
        points: linear.points,
    };
    RichLeader {
        pattern: match &leader.pattern {
            LeaderPattern::Dots => RichLeaderPattern::Dots,
            LeaderPattern::Rule { style, thickness } => RichLeaderPattern::Rule {
                style: match style {
                    RuleStyle::Solid => RichRuleStyle::Solid,
                    RuleStyle::Dashed => RichRuleStyle::Dashed,
                    RuleStyle::Dotted => RichRuleStyle::Dotted,
                },
                thickness: thickness.map(measure),
            },
            LeaderPattern::Content(content) => RichLeaderPattern::Content(content.clone()),
        },
        width: leader.width.map(measure),
    }
}

/// One run's style as a host reads it, `letter-spacing` and chapter 27's two font scales among it:
/// a host that cannot set one says so itself (ADR 1654).
fn run(text: String, style: &Character) -> RichRun {
    RichRun {
        text,
        families: style.families.clone(),
        size: Measure {
            per_base: style.size.per_root,
            points: style.size.points,
        },
        bold: style.bold(),
        italic: style.italic,
        colour: style
            .colour
            .map(|[r, g, b]| pdf_render::Color { r, g, b, a: 1.0 }),
        underlines: style.underline.lines(),
        underline_by_word: style.underline.by_word(),
        line_through: style.line_through,
        rise: Measure {
            per_base: style.rise.per_root,
            points: style.rise.points,
        },
        letter_spacing: match style.letter_spacing {
            Spacing::Length(length) => RichSpacing::Length(Measure {
                per_base: length.per_root,
                points: length.points,
            }),
            Spacing::OfSpace(share) => RichSpacing::OfSpace(share),
        },
        horizontal_scale: style.horizontal_scale,
        vertical_scale: style.vertical_scale,
    }
}
