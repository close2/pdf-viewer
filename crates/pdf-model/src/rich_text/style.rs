//! The style attributes a rich text string and a default style string state, and what each
//! computes to.
//!
//! XFA 3.3 section 27 names the set: a restricted list of CSS2 properties, a handful of
//! XFA-specific ones, and CSS2 and XHTML as the normative reference for what each value may be
//! (*Summary of Supported XHTML and CSS Attributes*, pages 1187 and 1188). A style attribute is a
//! CSS2 declaration block — `name:value` pairs separated by semicolons — and Table 228's and
//! Table 177's `/DS` is the same grammar with no element around it, so one parser reads both
//! ([`Declarations`]).
//!
//! # Which values are read, and the one place the two texts disagree
//!
//! A length is a number and a unit. Chapter 2's *Measurements* (page 36) makes the inch the unit
//! of a measurement that states none; CSS2 section 4.3.2 makes a unit required after every length
//! but zero, and section 4.2 has a declaration whose value is not valid ignored. Chapter 27 sends
//! a reader to CSS2 for which values a property may take, so **CSS2's rule is the one applied**:
//! `font-size:12` is no size at all rather than twelve inches, and the declaration is dropped
//! (ADR 1634 section 3). The units are chapter 2's four absolute ones — `in`, `cm`, `mm`, `pt` —
//! and its two relative ones, `em` and `%`, wherever a property takes a relative measurement.
//!
//! **A value that is not valid is ignored, and that is not a report.** CSS2 section 4.2 states
//! the rule and chapter 27 states its own for markup: what a processor does not understand it
//! ignores. What *is* reported is a property chapter 27 names, stated with a value this tree
//! does not carry out — [`Unapplied`] — because that is formatting the file states and the page
//! does not show.

use std::fmt::Write as _;

/// A quantity some of whose part follows the size the field's text is set at.
///
/// §12.7.4.3 lets a `/DA` ask for its size to be chosen — "[a] zero value for size means that the
/// font shall be auto-sized" — and every `em` and every percentage of a font size below it then
/// follows that choice. So a computed size is `scale × root + points`: an absolute length sets
/// the second and clears the first, a relative one multiplies both, and [`Self::at`] is the only
/// place the two meet.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Linear {
    /// How many root sizes this is.
    pub(crate) per_root: f32,
    /// How many points it is beside them.
    pub(crate) points: f32,
}

impl Linear {
    /// A length in points, whatever the root.
    pub(crate) const fn points(points: f32) -> Self {
        Self {
            per_root: 0.0,
            points,
        }
    }

    /// The root size itself.
    pub(crate) const ROOT: Self = Self {
        per_root: 1.0,
        points: 0.0,
    };

    /// The quantity, once the root size is known.
    pub(crate) fn at(self, root: f32) -> f32 {
        self.per_root.mul_add(root, self.points)
    }

    /// The same quantity multiplied by a factor.
    pub(crate) fn times(self, factor: f32) -> Self {
        Self {
            per_root: self.per_root * factor,
            points: self.points * factor,
        }
    }

    /// The sum of two quantities.
    pub(crate) fn plus(self, other: Self) -> Self {
        Self {
            per_root: self.per_root + other.per_root,
            points: self.points + other.points,
        }
    }
}

/// A length as a style attribute states it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Length {
    /// An absolute length, already in points.
    Points(f32),
    /// `em`: so many of the current font's size.
    Em(f32),
    /// `%`: a percentage of whatever the property makes it a percentage of.
    Percent(f32),
    /// A bare number, which CSS2 section 10.8.1 admits for `line-height` alone, as a multiple of
    /// the font size.
    Number(f32),
}

/// Reads one length: an optional sign, a number, and one of chapter 2's units.
///
/// One point is exactly 1/72 inch and one inch exactly 2.54 centimetres, which chapter 2's
/// *Units* list states and CSS2 section 4.3.2 agrees with. `None` for anything else, including a
/// non-zero number with no unit (CSS2 section 4.3.2; [`self`]'s module comment says why that rule
/// and not chapter 2's).
pub(crate) fn length(value: &str) -> Option<Length> {
    let value = value.trim();
    let split = value
        .char_indices()
        .find(|(at, character)| {
            !(character.is_ascii_digit()
                || *character == '.'
                || (*at == 0 && "+-".contains(*character)))
        })
        .map_or(value.len(), |(at, _)| at);
    let (number, unit) = value.split_at(split);
    let number: f32 = number
        .parse()
        .ok()
        .filter(|number: &f32| number.is_finite())?;
    let points = |per: f32| Some(Length::Points(number * per));
    match unit.trim().to_ascii_lowercase().as_str() {
        "pt" => points(1.0),
        "in" => points(72.0),
        "cm" => points(72.0 / 2.54),
        "mm" => points(72.0 / 25.4),
        "em" => Some(Length::Em(number)),
        "%" => Some(Length::Percent(number)),
        "" if number.abs() < f32::EPSILON => Some(Length::Points(0.0)),
        "" => Some(Length::Number(number)),
        _ => None,
    }
}

/// A colour, as the two forms chapter 27's *Color* table admits (page 1200).
///
/// `#rrggbb`, with two hexadecimal digits a component and either case, and `rgb(r,g,b)` with three
/// decimal integers from 0 to 255. Both are sRGB values, which the stream this module's caller
/// writes states in `DeviceRGB` — ADR 1634 section 4 is the choice and its cost.
pub(crate) fn colour(value: &str) -> Option<[f32; 3]> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() != 6 || !hex.is_ascii() {
            return None;
        }
        let component = |at: usize| {
            hex.get(at..at.saturating_add(2))
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .map(|byte| f32::from(byte) / 255.0)
        };
        return Some([component(0)?, component(2)?, component(4)?]);
    }
    let inner = value
        .strip_prefix("rgb(")
        .or_else(|| value.strip_prefix("RGB("))?
        .strip_suffix(')')?;
    let mut components = inner.split(',').map(|part| part.trim().parse::<u8>().ok());
    let mut next = || {
        components
            .next()
            .flatten()
            .map(|byte| f32::from(byte) / 255.0)
    };
    let out = [next()?, next()?, next()?];
    components.next().is_none().then_some(out)
}

/// One `name:value` declaration of a style attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Declaration {
    /// The property, folded to lower case: CSS2 section 4.1.3 makes property names
    /// case-insensitive.
    pub(crate) name: String,
    /// The value as written, trimmed.
    pub(crate) value: String,
}

/// A style attribute, or a `/DS` string: CSS2's declaration block without its braces.
///
/// The declarations are split on semicolons outside quotation marks, which matters for the one
/// property whose value may carry one — `font-family`, where chapter 27's *Font* table has a name
/// holding white space quoted with either mark.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Declarations(pub(crate) Vec<Declaration>);

impl Declarations {
    /// Reads a declaration block.
    pub(crate) fn parse(text: &str) -> Self {
        let mut out = Vec::new();
        for part in split_outside_quotes(text, ';') {
            let Some((name, value)) = part.split_once(':') else {
                continue;
            };
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim();
            if name.is_empty() || value.is_empty() {
                continue;
            }
            out.push(Declaration {
                name,
                value: value.to_owned(),
            });
        }
        Self(out)
    }
}

/// Splits on a separator wherever it is not inside a pair of quotation marks of either kind.
fn split_outside_quotes(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut quote: Option<char> = None;
    let mut start = 0_usize;
    for (at, character) in text.char_indices() {
        match quote {
            Some(open) if character == open => quote = None,
            None if character == '"' || character == '\'' => quote = Some(character),
            None if character == separator => {
                parts.push(text.get(start..at).unwrap_or_default());
                start = at.saturating_add(character.len_utf8());
            }
            Some(_) | None => {}
        }
    }
    parts.push(text.get(start..).unwrap_or_default());
    parts
}

/// Table 228's `/Q` and chapter 27's `text-align`, as one value (page 1190).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Align {
    /// `left`, and `/Q 0`.
    Left,
    /// `center`, and `/Q 1`.
    Centre,
    /// `right`, and `/Q 2`.
    Right,
    /// `justify`: every line of a paragraph but its last is set to the whole width.
    Justify,
    /// `justify-all`, chapter 27's extension: the last line too.
    JustifyAll,
}

/// Chapter 27's `text-valign`, the paragraph's place in the box from top to bottom (page 1197).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerticalAlign {
    /// `top`.
    Top,
    /// `middle`.
    Middle,
    /// `bottom`.
    Bottom,
}

/// The four kinds of underline chapter 27's *Underline and Strikethrough* table states (page
/// 1208).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Underline {
    /// No underline.
    #[default]
    None,
    /// `underline`: one continuous line.
    Single,
    /// `word`: one line broken at word boundaries.
    SingleWord,
    /// `double`: two continuous lines.
    Double,
    /// `double word`: two lines broken at word boundaries.
    DoubleWord,
}

impl Underline {
    /// Whether the line skips the spaces between words.
    pub(crate) fn by_word(self) -> bool {
        matches!(self, Self::SingleWord | Self::DoubleWord)
    }

    /// How many lines are drawn.
    pub(crate) fn lines(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Single | Self::SingleWord => 1,
            Self::Double | Self::DoubleWord => 2,
        }
    }
}

/// The character properties in force over one run of text.
///
/// Every one of them is inherited, which is CSS2's rule for each (sections 15 and 16) and what
/// chapter 27 states from the other side: a character formatting attribute on a `p` affects all
/// the text in the paragraph, and on a `span` the text the span encloses.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Character {
    /// `font-family`'s search path, nearest first; empty where nothing names one, which leaves
    /// the face the field's `/DA` names.
    pub(crate) families: Vec<String>,
    /// `font-size`, as the height of the em the run is set in.
    pub(crate) size: Linear,
    /// `font-weight`, on CSS2's scale from 100 to 900, where 400 is normal and 700 bold.
    pub(crate) weight: u16,
    /// `font-style` other than `normal`.
    pub(crate) italic: bool,
    /// `color`, in sRGB; `None` where nothing states one, which leaves the `/DA`'s own colour.
    pub(crate) colour: Option<[f32; 3]>,
    /// `text-decoration`'s underline.
    pub(crate) underline: Underline,
    /// `text-decoration`'s `line-through`.
    pub(crate) line_through: bool,
    /// How far the run's baseline sits above the line's: `vertical-align`, `sub` and `sup`.
    pub(crate) rise: Linear,
    /// `letter-spacing`, added after every glyph.
    pub(crate) letter_spacing: Spacing,
    /// `xfa-font-horizontal-scale`, as a factor (page 1202).
    pub(crate) horizontal_scale: f32,
    /// `xfa-font-vertical-scale`, as a factor.
    pub(crate) vertical_scale: f32,
    /// `xfa-spacerun:yes`, under which a run of spaces is kept (page 1220).
    pub(crate) spacerun: bool,
    /// `font-stretch`, as its place among the nine widths, narrowest first: [`STRETCHES`].
    pub(crate) stretch: u8,
    /// `xfa-tab-count`: how many tab stops the element advances by where it opens (*Tab Stops*,
    /// page 1206). Not inherited — it is an event at the element, which the markup walk takes and
    /// clears before anything inside it is read.
    pub(crate) tab_count: u16,
    /// `kerning-mode`: whether the run is kerned by its face's own pairs (*Kerning*, pages 1203
    /// and 1204; ADR 1682).
    pub(crate) kerning: Kerning,
}

/// Chapter 27's *Kerning* table, its two values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Kerning {
    /// `none`, the default: no kerning is applied.
    #[default]
    None,
    /// `pair`: each two adjacent glyphs are kerned by the pair their face states.
    Pair,
}

/// The nine widths, narrowest first, in chapter 27's spelling (*Font*, page 1201); Table 120's
/// `/FontStretch` names the same nine in the same order, so a width is matched by its place.
pub(crate) const STRETCHES: [&str; 9] = [
    "ultra-condensed",
    "extra-condensed",
    "condensed",
    "semi-condensed",
    "normal",
    "semi-expanded",
    "expanded",
    "extra-expanded",
    "ultra-expanded",
];

/// `normal`'s place among [`STRETCHES`], the width nothing styled is set in.
pub(crate) const NORMAL_STRETCH: u8 = 4;

/// `letter-spacing`'s value, kept in the unit it was stated in until the face is known.
///
/// Chapter 27 makes it a relative measurement (page 1204), and chapter 2's two relative units
/// are measured against the face: an `em` is the current font's size and a `%` a percentage of
/// the width of that font's space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Spacing {
    /// A length, possibly a number of ems.
    Length(Linear),
    /// A percentage of the width of a space in the run's face.
    OfSpace(f32),
}

impl Character {
    /// The style of text nothing has styled: the `/DA`'s face, size and colour.
    pub(crate) fn root() -> Self {
        Self {
            families: Vec::new(),
            size: Linear::ROOT,
            weight: 400,
            italic: false,
            colour: None,
            underline: Underline::None,
            line_through: false,
            rise: Linear::default(),
            letter_spacing: Spacing::Length(Linear::default()),
            horizontal_scale: 1.0,
            vertical_scale: 1.0,
            spacerun: false,
            stretch: NORMAL_STRETCH,
            tab_count: 0,
            kerning: Kerning::None,
        }
    }

    /// Whether the run is set bold: CSS2 section 15.5.1's matching for a family with a regular
    /// and a bold face, where a weight above 500 takes the heavier.
    pub(crate) fn bold(&self) -> bool {
        self.weight > 500
    }
}

/// The paragraph properties in force over one paragraph.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Block {
    /// `text-align`; `None` leaves Table 228's `/Q`.
    pub(crate) align: Option<Align>,
    /// `text-valign`.
    pub(crate) valign: Option<VerticalAlign>,
    /// `margin-top`, `margin-right`, `margin-bottom` and `margin-left`, in that order, which is
    /// CSS2 section 8.3's for the `margin` shorthand.
    pub(crate) margins: [Linear; 4],
    /// `text-indent`, the first line's extra indent.
    pub(crate) indent: Linear,
    /// `line-height`, the distance between baselines; `None` derives it from the tallest thing
    /// on each line, which chapter 27's *Line Spacing* states as the default (page 1191).
    pub(crate) line_height: Option<Linear>,
    /// `tab-interval`: the distance between default tab stops, from the left margin.
    pub(crate) tab_interval: Option<Linear>,
    /// `tab-stops` and `xfa-tab-stops`: the stops at stated positions from the left margin, in
    /// the order stated, each with its alignment and its leader.
    pub(crate) tab_stops: Vec<TabStop>,
}

/// One stated tab stop: chapter 2's `[alignment] [leader] location` (page 63).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TabStop {
    /// How the text after the tab stands at the stop.
    pub(crate) align: TabAlign,
    /// The stop's position from the left margin.
    pub(crate) at: Linear,
    /// What fills the room before the aligned text; `None` for the blank leaders, `space()` and
    /// a rule of style `none`, and for a stop that states none.
    pub(crate) leader: Option<Leader>,
}

/// A tab leader, chapter 2's `leader ( leaderPattern [leaderAlignment [leaderPatternWidth]] )`
/// (*Tab Leader Pattern*, pages 63 to 65).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Leader {
    /// What is repeated.
    pub(crate) pattern: LeaderPattern,
    /// `leaderPatternWidth`, the least repetition width; the effective one is the larger of this
    /// and the pattern's own.
    pub(crate) width: Option<Linear>,
}

/// A leader's pattern: chapter 2's `dots`, `rule` and `use-content` (`space` is no leader).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LeaderPattern {
    /// `dots()`: a row of dots, which the chapter lets an implementation draw graphically or as
    /// text; this tree draws the run's full stop.
    Dots,
    /// `rule(ruleStyle [ruleThickness])`, drawn in the text's colour.
    Rule {
        /// `solid`, `dashed` or `dotted`; `double`, `groove` and `ridge` are read as solid, which
        /// the chapter permits.
        style: RuleStyle,
        /// `ruleThickness`; `None` takes the underline's.
        thickness: Option<Linear>,
    },
    /// `use-content(content)`: the content repeated across the room before the stop, as many
    /// whole times as fit.
    Content(String),
}

/// A rule leader's `ruleStyle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuleStyle {
    /// One unbroken line.
    Solid,
    /// Dashes.
    Dashed,
    /// Dots.
    Dotted,
}

/// How the text after a tab stands at the stop (chapter 2's *Tab Stops* table, page 62, which
/// chapter 27's *Tab Stops* sends `xfa-tab-stops` to).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabAlign {
    /// The text's left edge at the stop.
    Left,
    /// The text centred on the stop.
    Centre,
    /// The text's right edge at the stop.
    Right,
    /// The text's first radix character at the stop, and its right edge where it has none.
    Decimal,
    /// The edge the text starts from at the stop: its left edge where the paragraph reads left
    /// to right and its right edge where it reads right to left — every default stop's alignment
    /// since XFA 2.8 (chapter 27's *Tab Stops*, page 1205).
    After,
    /// The edge the text ends at, the other way round.
    Before,
}

impl Block {
    /// A paragraph nothing has styled.
    pub(crate) fn root() -> Self {
        Self {
            align: None,
            valign: None,
            margins: [Linear::default(); 4],
            indent: Linear::default(),
            line_height: None,
            tab_interval: None,
            tab_stops: Vec::new(),
        }
    }

    /// The properties a nested element inherits from this one.
    ///
    /// CSS2 makes `text-align`, `text-indent` and `line-height` inherited and the margins not
    /// (sections 16.1, 16.2 and 10.8.1, 8.3); `text-valign` is chapter 27's own and is treated the
    /// way its CSS2 namesake for table cells is, as a property of the block that states it, which
    /// a paragraph inherits from the body.
    pub(crate) fn inherited(&self) -> Self {
        Self {
            margins: [Linear::default(); 4],
            ..self.clone()
        }
    }
}

/// A property chapter 27 names that a file stated and this tree does not carry out.
///
/// Collected rather than reported one at a time, so a field's report names every such property
/// once, in a sentence a reader can act on — and so a value no processor is asked to honour, or
/// one this tree applies, never appears in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Unapplied(pub(crate) std::collections::BTreeSet<String>);

impl Unapplied {
    /// Records one.
    pub(crate) fn note(&mut self, what: impl Into<String>) {
        self.0.insert(what.into());
    }

    /// Whether nothing was recorded.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The list as one phrase, for a report.
    pub(crate) fn phrase(&self) -> String {
        let mut out = String::new();
        for (index, item) in self.0.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            let _ = write!(out, "{item}");
        }
        out
    }
}

/// Applies a declaration block to the style an element inherits.
///
/// `parent_size` is the size an `em` and a percentage of `font-size` are measured against, which
/// CSS2 section 15.7 makes the *parent's* font size for that property alone; every other `em`
/// below is the element's own size, once `font-size` has been applied, which is why the size is
/// taken first.
pub(crate) fn apply(
    declarations: &Declarations,
    character: &mut Character,
    block: &mut Block,
    unapplied: &mut Unapplied,
) {
    let parent_size = character.size;
    // The size first, as the block's last valid statement of it — `font-size` or the `font`
    // shorthand — because every other `em` below is measured against it wherever in the block it
    // was stated. Everything else is then taken in order, a later declaration overriding an
    // earlier one, which is CSS2 section 6.4.2's cascade within one block.
    for declaration in &declarations.0 {
        let size = match declaration.name.as_str() {
            "font-size" => font_size(&declaration.value, parent_size),
            "font" => shorthand_size(&declaration.value, parent_size),
            _ => None,
        };
        if let Some(size) = size {
            character.size = size;
        }
    }
    for Declaration { name, value } in &declarations.0 {
        let value = value.as_str();
        if name == "font" {
            font_shorthand(value, parent_size, character, block);
        } else if !character_property(name, value, character, unapplied) {
            block_property(name, value, character, block, unapplied);
        }
    }
}

/// The note a string stating `kerning-mode:pair` carries until a layout kerns it.
pub(crate) const PAIR_KERNING: &str = "kerning-mode:pair";

/// One character property, applied; `false` where the name is not one.
fn character_property(
    name: &str,
    value: &str,
    character: &mut Character,
    unapplied: &mut Unapplied,
) -> bool {
    match name {
        "font-family" => {
            let families = families(value);
            if !families.is_empty() {
                character.families = families;
            }
        }
        "font-weight" => {
            if let Some(weight) = weight(value, character.weight) {
                character.weight = weight;
            }
        }
        "font-style" => match value.to_ascii_lowercase().as_str() {
            "normal" => character.italic = false,
            "italic" | "oblique" => character.italic = true,
            _ => {}
        },
        // *Tab Stops* (page 1206): a non-negative integer, and zero changes nothing.
        "xfa-tab-count" => {
            if let Ok(count) = value.trim().parse::<u16>() {
                character.tab_count = count;
            }
        }
        "font-stretch" => {
            // Chapter 27's *Font* table lists nine widths (page 1201), and Table 120's
            // `/FontStretch` names the same nine: the face a width is set in is chosen by it
            // where `/DR` holds one (`layout`'s `Faces`), and said where none is found.
            let wanted = value.trim().to_ascii_lowercase();
            if let Some(at) = STRETCHES.iter().position(|name| *name == wanted) {
                character.stretch = u8::try_from(at).unwrap_or(NORMAL_STRETCH);
            }
        }
        "color" => {
            if let Some(colour) = colour(value) {
                character.colour = Some(colour);
            }
        }
        "text-decoration" => decoration(value, character),
        "vertical-align" => {
            if value.eq_ignore_ascii_case("baseline") {
                character.rise = Linear::default();
            } else if let Some(rise) = relative(value, character.size, character.size) {
                // Chapter 27's *Baseline Adjustment* measures from the surrounding line's
                // baseline (page 1199), not the parent span's, and forbids a span inside a
                // paragraph that set one from setting another — so the value replaces rather
                // than adds.
                character.rise = rise;
            }
        }
        "letter-spacing" => {
            if value.eq_ignore_ascii_case("normal") {
                character.letter_spacing = Spacing::Length(Linear::default());
            } else {
                match length(value) {
                    Some(Length::Points(points)) => {
                        character.letter_spacing = Spacing::Length(Linear::points(points));
                    }
                    Some(Length::Em(ems)) => {
                        character.letter_spacing = Spacing::Length(character.size.times(ems));
                    }
                    Some(Length::Percent(percent)) => {
                        character.letter_spacing = Spacing::OfSpace(percent / 100.0);
                    }
                    Some(Length::Number(_)) | None => {}
                }
            }
        }
        "xfa-font-horizontal-scale" => {
            if let Some(Length::Percent(percent)) = length(value)
                && percent > 0.0
            {
                character.horizontal_scale = percent / 100.0;
            }
        }
        "xfa-font-vertical-scale" => {
            if let Some(Length::Percent(percent)) = length(value)
                && percent > 0.0
            {
                character.vertical_scale = percent / 100.0;
            }
        }
        "xfa-spacerun" => character.spacerun = value.eq_ignore_ascii_case("yes"),
        "kerning-mode" => {
            // Chapter 27's *Kerning* table: `none`, the default, and `pair` (page 1204). Whether
            // the run's face states pairs to kern by is the layout's question, asked of the face
            // a glyph is set in: the layout takes this note back and says what it could not kern
            // (ADR 1682), and a reader that sets no glyphs — a host's popup window — keeps it.
            if value.eq_ignore_ascii_case("pair") {
                character.kerning = Kerning::Pair;
                unapplied.note(PAIR_KERNING);
            } else if value.eq_ignore_ascii_case("none") {
                character.kerning = Kerning::None;
            }
        }
        _ => return false,
    }
    true
}

/// One paragraph property, applied; a name that is neither kind is ignored.
fn block_property(
    name: &str,
    value: &str,
    character: &Character,
    block: &mut Block,
    unapplied: &mut Unapplied,
) {
    match name {
        "text-align" => {
            block.align = match value.to_ascii_lowercase().as_str() {
                "left" => Some(Align::Left),
                "center" => Some(Align::Centre),
                "right" => Some(Align::Right),
                "justify" => Some(Align::Justify),
                "justify-all" => Some(Align::JustifyAll),
                _ => block.align,
            };
        }
        "text-valign" => {
            block.valign = match value.to_ascii_lowercase().as_str() {
                "top" => Some(VerticalAlign::Top),
                "middle" => Some(VerticalAlign::Middle),
                "bottom" => Some(VerticalAlign::Bottom),
                _ => block.valign,
            };
        }
        "text-indent" => {
            if let Some(indent) = relative(value, character.size, Linear::default()) {
                block.indent = indent;
            }
        }
        "line-height" => {
            if value.eq_ignore_ascii_case("normal") {
                block.line_height = None;
            } else if let Some(height) = line_height(value, character.size) {
                block.line_height = Some(height);
            }
        }
        "margin" => {
            let sides: Vec<Linear> = value
                .split_whitespace()
                .filter_map(|side| margin(side, character.size))
                .collect();
            // CSS2 section 8.3's one to four values, which chapter 27's *Set Margins* states
            // in the same words (page 1195).
            let [top, right, bottom, left] = match sides.as_slice() {
                [all] => [*all; 4],
                [vertical, horizontal] => [*vertical, *horizontal, *vertical, *horizontal],
                [top, horizontal, bottom] => [*top, *horizontal, *bottom, *horizontal],
                [top, right, bottom, left] => [*top, *right, *bottom, *left],
                _ => return,
            };
            block.margins = [top, right, bottom, left];
        }
        "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
            let Some(side) = margin(value, character.size) else {
                return;
            };
            let index = match name {
                "margin-top" => 0,
                "margin-right" => 1,
                "margin-bottom" => 2,
                _ => 3,
            };
            if let Some(slot) = block.margins.get_mut(index) {
                *slot = side;
            }
        }
        // *Tab Stops* (page 1206): a non-zero measurement between default stops.
        "tab-interval" => {
            if let Some(interval) = relative(value, character.size, Linear::default())
                .filter(|interval| interval.per_root > 0.0 || interval.points > 0.0)
            {
                block.tab_interval = Some(interval);
            }
        }
        // The old syntax is alignment and position in pairs; the new one, a superset, puts an
        // optional leader between them (chapter 2's *Tab Leader Pattern*, page 63).
        "tab-stops" | "xfa-tab-stops" => {
            let (stops, page_aligned) = tab_stops(value, character.size);
            block.tab_stops = stops;
            if page_aligned {
                unapplied.note(format!("{name}'s leader alignment to the page"));
            }
        }
        // Chapter 27's flow controls between content regions (pages 1192 to 1197): a field's
        // and a note's appearance is one box, so no paragraph ever flows from one region to
        // another and each of these has nothing to act on. Read, and carried out by there
        // being nothing to do — as is a property CSS2 section 4.2 ignores because it is
        // unknown, or one taken above.
        _ => {}
    }
}

/// A `tab-stops` or `xfa-tab-stops` value: each stop's alignment, position and leader, and
/// whether a leader asks to be aligned to the page.
///
/// The grammar is chapter 2's: `[alignment] [leader] location`, repeated, the alignment `left` by
/// default. `before` and `after` are the edges in the direction the text reads, which for text
/// read left to right are `right` and `left`. A leader's `leaderAlignment` of `page` aligns its
/// cycles as though the leader began at the page's right edge, an edge a field's appearance does
/// not know; its cycles are laid on the margin's grid as `none`'s are, and the alignment is said.
fn tab_stops(value: &str, size: Linear) -> (Vec<TabStop>, bool) {
    let mut stops = Vec::new();
    let mut page_aligned = false;
    let mut leader: Option<Leader> = None;
    let mut align = TabAlign::Left;
    let mut rest = value.trim();
    while !rest.is_empty() {
        // A leader is a function with parentheses that may hold spaces and nest, so it is read
        // to its balancing parenthesis rather than to the next space.
        if rest.starts_with("leader") {
            let mut depth = 0_usize;
            let mut end = rest.len();
            for (at, character) in rest.char_indices() {
                match character {
                    '(' => depth = depth.saturating_add(1),
                    ')' => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            end = at.saturating_add(1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let stated = rest.get(..end).unwrap_or_default();
            let (read, page) = read_leader(stated, size);
            leader = read;
            page_aligned |= page;
            rest = rest.get(end..).unwrap_or_default().trim_start();
            continue;
        }
        let (word, after) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        rest = after.trim_start();
        match word.to_ascii_lowercase().as_str() {
            "left" => align = TabAlign::Left,
            "right" => align = TabAlign::Right,
            "after" => align = TabAlign::After,
            "before" => align = TabAlign::Before,
            "center" | "centre" => align = TabAlign::Centre,
            "decimal" => align = TabAlign::Decimal,
            other => {
                if let Some(at) = relative(other, size, Linear::default()) {
                    stops.push(TabStop {
                        align,
                        at,
                        leader: leader.take(),
                    });
                }
                align = TabAlign::Left;
                leader = None;
            }
        }
    }
    (stops, page_aligned)
}

/// One `leader(…)` function: its pattern, read by chapter 2's tables (pages 63 and 64), then its
/// optional `leaderAlignment` and `leaderPatternWidth`; and whether the alignment is `page`.
///
/// `None` for the blank patterns — `space()`, a rule of style `none`, and content that is one
/// space, which the chapter calls equivalent to `space()` — and for a pattern
/// the grammar does not name, which chapter 27 has a processor ignore (page 1187).
fn read_leader(stated: &str, size: Linear) -> (Option<Leader>, bool) {
    let inner = stated
        .trim()
        .strip_prefix("leader")
        .map(str::trim_start)
        .and_then(|rest| rest.strip_prefix('('))
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or_default()
        .trim();
    // The pattern runs to its own balancing parenthesis, or to the first space for a bare name.
    let mut depth = 0_usize;
    let mut end = inner.len();
    for (at, character) in inner.char_indices() {
        match character {
            '(' => depth = depth.saturating_add(1),
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    end = at.saturating_add(1);
                    break;
                }
            }
            character if character.is_whitespace() && depth == 0 => {
                end = at;
                break;
            }
            _ => {}
        }
    }
    let (pattern, rest) = inner.split_at(end.min(inner.len()));
    let mut extra = rest.split_whitespace();
    let page = extra
        .next()
        .is_some_and(|alignment| alignment.eq_ignore_ascii_case("page"));
    let width = extra
        .next()
        .and_then(|width| relative(width, size, Linear::default()));
    let (name, arguments) = pattern
        .split_once('(')
        .map_or((pattern, ""), |(name, arguments)| {
            (name, arguments.strip_suffix(')').unwrap_or(arguments))
        });
    let pattern = match name.trim().to_ascii_lowercase().as_str() {
        "dots" => LeaderPattern::Dots,
        "rule" => {
            let mut words = arguments.split_whitespace();
            let style = match words.next().map(str::to_ascii_lowercase).as_deref() {
                Some("none") => return (None, page),
                Some("dashed") => RuleStyle::Dashed,
                Some("dotted") => RuleStyle::Dotted,
                // `solid`, and the three the chapter lets a processor render as solid.
                _ => RuleStyle::Solid,
            };
            let thickness = words
                .next()
                .and_then(|thickness| relative(thickness, size, Linear::default()));
            LeaderPattern::Rule { style, thickness }
        }
        "use-content" => {
            let content = arguments
                .trim()
                .trim_matches(|quote| quote == '"' || quote == '\'');
            if content.is_empty() || content == " " {
                return (None, page);
            }
            LeaderPattern::Content(content.to_owned())
        }
        _ => return (None, page),
    };
    (Some(Leader { pattern, width }), page)
}

/// `font-size`, against the parent's size.
fn font_size(value: &str, parent: Linear) -> Option<Linear> {
    match length(value)? {
        Length::Points(points) if points > 0.0 => Some(Linear::points(points)),
        Length::Em(ems) if ems > 0.0 => Some(parent.times(ems)),
        Length::Percent(percent) if percent > 0.0 => Some(parent.times(percent / 100.0)),
        _ => None,
    }
}

/// A length whose `em` is the element's size and whose `%` is `percent_of`.
fn relative(value: &str, size: Linear, percent_of: Linear) -> Option<Linear> {
    match length(value)? {
        Length::Points(points) => Some(Linear::points(points)),
        Length::Em(ems) => Some(size.times(ems)),
        Length::Percent(percent) => Some(percent_of.times(percent / 100.0)),
        Length::Number(_) => None,
    }
}

/// One margin. A percentage is of the containing block's width in CSS2 section 8.3, which this
/// module does not know; it is not read.
fn margin(value: &str, size: Linear) -> Option<Linear> {
    if value.eq_ignore_ascii_case("auto") {
        return Some(Linear::default());
    }
    match length(value)? {
        Length::Points(points) => Some(Linear::points(points)),
        Length::Em(ems) => Some(size.times(ems)),
        Length::Percent(_) | Length::Number(_) => None,
    }
}

/// `line-height`: a length, a number of ems, a percentage of the size, or CSS2 section 10.8.1's
/// bare multiple of it.
fn line_height(value: &str, size: Linear) -> Option<Linear> {
    match length(value)? {
        Length::Points(points) if points > 0.0 => Some(Linear::points(points)),
        Length::Em(ems) | Length::Number(ems) if ems > 0.0 => Some(size.times(ems)),
        Length::Percent(percent) if percent > 0.0 => Some(size.times(percent / 100.0)),
        _ => None,
    }
}

/// `font-weight`: CSS2 section 15.6's keywords and nine numbers.
fn weight(value: &str, inherited: u16) -> Option<u16> {
    match value.to_ascii_lowercase().as_str() {
        "normal" => Some(400),
        "bold" => Some(700),
        // CSS2 section 15.6's relative keywords, one step of the nine against the inherited value.
        "bolder" => Some(inherited.saturating_add(300).min(900)),
        "lighter" => Some(inherited.saturating_sub(300).max(100)),
        number => number
            .parse::<u16>()
            .ok()
            .filter(|weight| (100..=900).contains(weight) && weight % 100 == 0),
    }
}

/// `text-decoration`: chapter 27's decoration styles and `line-through`, in either order.
///
/// CSS2 section 16.3.1 makes the property apply to the text of every descendant and lets no
/// descendant take it away, so a decoration stated here is added to the inherited ones rather
/// than replacing them; `none` adds nothing.
fn decoration(value: &str, character: &mut Character) {
    let words: Vec<String> = value
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect();
    let has = |word: &str| words.iter().any(|stated| stated == word);
    if has("line-through") {
        character.line_through = true;
    }
    let underline = match (has("double"), has("word"), has("underline")) {
        (true, true, _) => Underline::DoubleWord,
        (true, false, _) => Underline::Double,
        (false, true, _) => Underline::SingleWord,
        (false, false, true) => Underline::Single,
        (false, false, false) => Underline::None,
    };
    if underline != Underline::None {
        character.underline = underline;
    }
}

/// `font-family`'s list: names separated by commas, each quoted with either mark or written as
/// words (CSS2 section 15.3), with CSS2's three generic families named as §9.6.2.2's faces.
///
/// **The generic families are a choice.** CSS2 section 15.3.1 leaves the face each generic name
/// maps to to the reader; the standard 14's three families are the faces this binary carries, so
/// `serif`, `sans-serif` and `monospace` draw the same everywhere (ADR 1634 section 2).
pub(crate) fn families(value: &str) -> Vec<String> {
    split_outside_quotes(value, ',')
        .into_iter()
        .filter_map(|name| {
            let name = name.trim();
            let unquoted = name
                .strip_prefix('"')
                .and_then(|inner| inner.strip_suffix('"'))
                .or_else(|| {
                    name.strip_prefix('\'')
                        .and_then(|inner| inner.strip_suffix('\''))
                })
                .map_or_else(
                    || name.split_whitespace().collect::<Vec<_>>().join(" "),
                    str::to_owned,
                );
            let generic = match unquoted.to_ascii_lowercase().as_str() {
                "serif" => Some("Times"),
                "sans-serif" => Some("Helvetica"),
                "monospace" => Some("Courier"),
                _ => None,
            };
            let name = generic.map_or(unquoted, str::to_owned);
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

/// `font`: chapter 27's shorthand, `[style] [weight] size[/line-height] family`, its first two in
/// either order (page 1202).
///
/// What the shorthand omits it sets to its default, which the same page says and CSS2 section
/// 15.8 states for every property it covers.
/// The size a `font` shorthand states, where the shorthand is valid.
fn shorthand_size(value: &str, parent_size: Linear) -> Option<Linear> {
    let mut character = Character::root();
    character.size = parent_size;
    let mut block = Block::root();
    let before = character.families.clone();
    font_shorthand(value, parent_size, &mut character, &mut block);
    (character.families != before).then_some(character.size)
}

fn font_shorthand(value: &str, parent_size: Linear, character: &mut Character, block: &mut Block) {
    let tokens = split_outside_quotes(value, ' ');
    let tokens: Vec<&str> = tokens
        .into_iter()
        .filter(|token| !token.is_empty())
        .collect();
    let mut italic = false;
    let mut weight_value = 400_u16;
    let mut index = 0_usize;
    while let Some(token) = tokens.get(index) {
        let lower = token.to_ascii_lowercase();
        match lower.as_str() {
            "italic" | "oblique" => italic = true,
            "normal" => {}
            other => match weight(other, character.weight) {
                Some(stated) => weight_value = stated,
                None => break,
            },
        }
        index = index.saturating_add(1);
    }
    let Some(size_token) = tokens.get(index) else {
        return;
    };
    let (size_text, line) = size_token
        .split_once('/')
        .map_or((*size_token, None), |(size, line)| (size, Some(line)));
    let Some(size) = font_size(size_text, parent_size) else {
        return;
    };
    let rest = tokens
        .get(index.saturating_add(1)..)
        .unwrap_or_default()
        .join(" ");
    let named = families(&rest);
    if named.is_empty() {
        // CSS2 section 15.8 requires the family; a shorthand without one is not valid and is
        // ignored whole.
        return;
    }
    character.italic = italic;
    character.weight = weight_value;
    character.size = size;
    character.families = named;
    block.line_height = line.and_then(|line| line_height(line, size));
}

#[cfg(test)]
mod tests {
    use super::{
        Align, Block, Character, Declarations, Length, Linear, Spacing, Unapplied, Underline,
        apply, colour, families, length,
    };

    fn styled(text: &str) -> (Character, Block, Unapplied) {
        let mut character = Character::root();
        let mut block = Block::root();
        let mut unapplied = Unapplied::default();
        apply(
            &Declarations::parse(text),
            &mut character,
            &mut block,
            &mut unapplied,
        );
        (character, block, unapplied)
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    /// Chapter 2's units: an inch is 72 points and 2.54 centimetres (page 37).
    #[test]
    fn a_length_is_read_in_each_of_chapter_twos_units() {
        assert_eq!(length("72pt"), Some(Length::Points(72.0)));
        let Some(Length::Points(inch)) = length("1in") else {
            panic!("an inch is a length");
        };
        assert!(near(inch, 72.0));
        let Some(Length::Points(centimetres)) = length("2.54cm") else {
            panic!("a centimetre is a length");
        };
        assert!(near(centimetres, 72.0));
        let Some(Length::Points(millimetres)) = length("25.4mm") else {
            panic!("a millimetre is a length");
        };
        assert!(near(millimetres, 72.0));
        assert_eq!(length("-3pt"), Some(Length::Points(-3.0)));
        assert_eq!(length("+4pt"), Some(Length::Points(4.0)));
        assert_eq!(length("0.25em"), Some(Length::Em(0.25)));
        assert_eq!(length("150%"), Some(Length::Percent(150.0)));
        assert_eq!(length("0"), Some(Length::Points(0.0)));
        assert_eq!(
            length("3px"),
            None,
            "chapter 27 admits neither ex nor px (page 1204)"
        );
    }

    /// CSS2 section 4.3.2: a unit is required after any length but zero.
    #[test]
    fn a_size_with_no_unit_is_no_size_at_all() {
        let (character, _, _) = styled("font-size:12");
        assert_eq!(
            character.size,
            Linear::ROOT,
            "the declaration is ignored, not read as inches"
        );
    }

    /// Chapter 27's *Color* table: `#rrggbb` in either case and `rgb(r,g,b)` (page 1200), the two
    /// spellings its Example 27.19 uses side by side.
    #[test]
    fn a_colour_is_read_in_the_two_forms_the_chapter_admits() {
        assert_eq!(colour("#0000ff"), Some([0.0, 0.0, 1.0]));
        assert_eq!(colour("#00FF00"), Some([0.0, 1.0, 0.0]));
        assert_eq!(colour("rgb(255,0,0)"), Some([1.0, 0.0, 0.0]));
        assert_eq!(colour("rgb( 0 , 255 , 0 )"), Some([0.0, 1.0, 0.0]));
        assert_eq!(
            colour("#00f"),
            None,
            "the table admits two hexadecimal digits a component"
        );
        assert_eq!(colour("rgb(256,0,0)"), None);
        assert_eq!(colour("blue"), None);
    }

    /// Chapter 27's *Font* table: a family name holding white space is quoted with either mark,
    /// and the list is a search path (page 1201).
    #[test]
    fn a_family_list_is_read_with_either_quotation_mark() {
        assert_eq!(
            families("'Courier Std', \"Minion Pro\", Arial"),
            vec!["Courier Std", "Minion Pro", "Arial"]
        );
        assert_eq!(families("sans-serif"), vec!["Helvetica"]);
        assert_eq!(families("serif, monospace"), vec!["Times", "Courier"]);
    }

    /// Chapter 27's `font` shorthand, in the shape its Example 27.20 gives: style, weight, a size
    /// with a line height after the solidus, and a quoted family (page 1202).
    #[test]
    fn the_font_shorthand_sets_style_weight_size_line_height_and_family() {
        let (character, block, _) = styled("font:italic bold 16pt/0.5in \"Minion Pro\"");
        assert!(character.italic);
        assert!(character.bold());
        assert_eq!(character.size, Linear::points(16.0));
        assert_eq!(character.families, vec!["Minion Pro"]);
        assert!(
            block
                .line_height
                .is_some_and(|height| near(height.at(0.0), 36.0))
        );

        // The two may come in either order.
        let (character, _, _) = styled("font:bold italic 10pt Times");
        assert!(character.italic && character.bold());
        // And what the shorthand omits it sets to its default.
        let (character, _, _) = styled("font-weight:bold; font:10pt Times");
        assert!(
            !character.bold(),
            "the shorthand resets the weight it does not state"
        );
        let (character, _, _) = styled("font:10pt Times; font-weight:bold");
        assert!(
            character.bold(),
            "a longhand after the shorthand overrides it"
        );
    }

    /// `font-size` in `em` and `%` is measured against the parent's size (CSS2 section 15.7),
    /// which is what lets it follow §12.7.4.3's auto-size.
    #[test]
    fn a_relative_size_follows_the_root() {
        let (character, _, _) = styled("font-size:150%");
        assert!(near(character.size.at(10.0), 15.0));
        let (character, _, _) = styled("font-size:2em");
        assert!(near(character.size.at(7.0), 14.0));
    }

    /// Chapter 27's *Bold* and numeric weights, on CSS2's scale (page 1201).
    #[test]
    fn weights_are_read_as_keywords_and_hundreds() {
        assert!(styled("font-weight:bold").0.bold());
        assert!(styled("font-weight:700").0.bold());
        assert!(styled("font-weight:600").0.bold());
        assert!(!styled("font-weight:500").0.bold());
        assert!(!styled("font-weight:normal").0.bold());
        assert!(!styled("font-weight:650").0.bold(), "not one of the nine");
    }

    /// Chapter 27's decoration styles, `line-through` before or after (page 1208).
    #[test]
    fn text_decoration_reads_every_style_the_chapter_lists() {
        assert_eq!(
            styled("text-decoration:underline").0.underline,
            Underline::Single
        );
        assert_eq!(
            styled("text-decoration:word").0.underline,
            Underline::SingleWord
        );
        assert_eq!(
            styled("text-decoration:double").0.underline,
            Underline::Double
        );
        assert_eq!(
            styled("text-decoration:double word").0.underline,
            Underline::DoubleWord
        );
        let (character, _, _) = styled("text-decoration:line-through underline");
        assert!(character.line_through && character.underline == Underline::Single);
        let (character, _, _) = styled("text-decoration:underline line-through");
        assert!(character.line_through && character.underline == Underline::Single);
    }

    /// Chapter 27's five alignments, `justify-all` among them (page 1190).
    #[test]
    fn text_align_reads_the_five_alignments() {
        for (value, align) in [
            ("left", Align::Left),
            ("center", Align::Centre),
            ("right", Align::Right),
            ("justify", Align::Justify),
            ("justify-all", Align::JustifyAll),
        ] {
            assert_eq!(styled(&format!("text-align:{value}")).1.align, Some(align));
        }
    }

    /// CSS2 section 8.3's one to four margin values (page 1195).
    #[test]
    fn the_margin_shorthand_takes_one_to_four_values() {
        let sides = |text: &str| styled(text).1.margins.map(|side| side.at(0.0));
        let same = |text: &str, expected: [f32; 4]| {
            let got = sides(text);
            assert!(
                got.iter()
                    .zip(expected)
                    .all(|(got, expected)| near(*got, expected)),
                "{text}: {got:?}"
            );
        };
        same("margin:1pt", [1.0; 4]);
        same("margin:1pt 2pt", [1.0, 2.0, 1.0, 2.0]);
        same("margin:1pt 2pt 3pt", [1.0, 2.0, 3.0, 2.0]);
        same("margin:1pt 2pt 3pt 4pt", [1.0, 2.0, 3.0, 4.0]);
        same("margin-left:0.5in", [0.0, 0.0, 0.0, 36.0]);
    }

    /// Chapter 27's relative letter spacing: an `em` is the font's size and a `%` the width of
    /// its space (page 1204, chapter 2 page 37).
    #[test]
    fn letter_spacing_is_kept_in_its_relative_unit() {
        let (character, _, _) = styled("font-size:10pt; letter-spacing:0.25em");
        assert_eq!(
            character.letter_spacing,
            Spacing::Length(Linear::points(2.5))
        );
        let (character, _, _) = styled("letter-spacing:50%");
        assert_eq!(character.letter_spacing, Spacing::OfSpace(0.5));
    }

    /// Properties chapter 27 names and this tree does not apply are collected, and those it
    /// applies or that have nothing to act on are not.
    #[test]
    fn what_is_not_applied_is_named_and_nothing_else_is() {
        let (character, _, unapplied) = styled(
            "font-stretch:condensed; kerning-mode:pair; orphans:2; widows:1; \
             color:#000000",
        );
        assert_eq!(unapplied.phrase(), "kerning-mode:pair");
        // Pair kerning is asked of the face a run is set in, which the layout knows.
        assert_eq!(character.kerning, super::Kerning::Pair);
        // A width is a style the layout looks a face up by, and says where it finds none.
        assert_eq!(
            super::STRETCHES[usize::from(character.stretch)],
            "condensed"
        );
        let (_, _, unapplied) = styled("font-stretch:normal; kerning-mode:none; xfa-tab-count:0");
        assert!(unapplied.is_empty(), "{unapplied:?}");
    }

    /// A semicolon inside a quoted family name does not end the declaration.
    #[test]
    fn a_quoted_semicolon_belongs_to_its_value() {
        let declarations = Declarations::parse("font-family:'A;B'; color:#000000");
        assert_eq!(declarations.0.len(), 2);
        assert_eq!(declarations.0[0].value, "'A;B'");
    }
}
