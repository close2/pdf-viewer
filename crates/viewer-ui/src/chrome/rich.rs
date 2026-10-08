//! Table 172's `/RC` as this window draws it: run by run, each in its own face, in the order UAX
//! #9 gives the paragraph, with its spacing and its scales.
//!
//! ISO 32000-2 §12.5.6.2, Table 172: a rich text string "that shall be displayed in the popup
//! window when the annotation is opened". `pdf_model::popup::RichNote` is what the string says
//! about each character (ADR 1642); a toolkit window hands it to Pango or Qt, and this one has no
//! toolkit, so the layout is here:
//!
//! - **A face per run**, the one of §9.6.2.2's five families its `font-family` search path reaches
//!   first ([`family`]) — the faces the binary carries, so a note is the same on every machine,
//!   which is the chrome's own rule (ADR 0133). A path that reaches none of them is set in the
//!   family `pdf-font` classifies its first name into, which is what a field's appearance falls
//!   back to on a machine with no fonts; CSS2 section 15.5's matching ends in a face of the
//!   processor's choosing, so that is the property carried out, not a property dropped (ADR 1654).
//! - **One paragraph, one order.** The runs' characters are one paragraph to UAX #9: its levels
//!   are resolved over the whole of it (rules P2 and P3 find its direction from its first strong
//!   character, whichever run holds it), joining is over the whole of it, and each laid-out line is
//!   ordered by rules L1 and L2 glyph by glyph, every glyph keeping its own run's style — so a
//!   Hebrew sentence whose words are coloured two ways reads right to left across both. A
//!   paragraph whose direction is right to left and states no `text-align` begins at the right,
//!   its own start edge, as Pango and Qt place one (ADR 1654).
//! - **Tab stops**, chapter 27's *Tab Stops* (pages 1205 to 1207): a tab advances to the next of
//!   the paragraph's stops past it, stated or every `tab-interval`, and the text after it stands
//!   there as the stop's alignment says (ADR 1666) — the next on the left in a paragraph read
//!   right to left, as chapter 2's *Tab Stops* has it (page 61), and the room before it filled
//!   with the stop's leader (ADR 1679).
//! - **`letter-spacing` and the two font scales** as `pdf_model::rich_text::lay_out` applies them
//!   to a field: the spacing is added after every glyph and the horizontal scale multiplies the
//!   advance with it, which is §9.4.4's `(w0 × Tfs + Tc) × Th` with chapter 27's two in place of
//!   `Tc` and `Th`; the vertical scale stretches the glyph upwards about its baseline.

use pdf_model::popup::{RichAlign, RichNote, RichParagraph, RichRun};
use pdf_render::{Color, DisplayList};

use super::{Chrome, DIMMED, Family, Style, elide, rectangle};

/// How many logical pixels one point is: CSS2's reference pixel, 96 to the inch, which is also
/// what the two toolkits take a point to be on a screen of no stated resolution.
const PIXELS_PER_POINT: f32 = 96.0 / 72.0;

/// The family a run is set in: the first name of its `font-family` search path that is one of
/// §9.6.2.2's families or a metric-compatible name for one, and otherwise the family `pdf-font`
/// classifies the path's first name into; the window's own Helvetica where the run names none.
pub(super) fn family(run: &RichRun) -> Family {
    if let Some(family) = run.families.iter().find_map(|name| fourteen(name)) {
        return family;
    }
    run.families
        .first()
        .map_or(Family::Sans, |name| classified(name))
}

/// One of §9.6.2.2's families, where `name` spells one or a face metric-compatible with one: the
/// names `pdf_font::substitute` answers from the compiled-in faces rather than the machine's, read
/// as `pdf_model::rich_text`'s own family comparison folds them — case, spaces, hyphens and a
/// PostScript name's `MT` or `PS` set aside.
fn fourteen(name: &str) -> Option<Family> {
    let folded: String = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect();
    let stem = ["psmt", "mt", "ps"]
        .iter()
        .find_map(|suffix| folded.strip_suffix(suffix).filter(|stem| !stem.is_empty()))
        .unwrap_or(&folded);
    match stem {
        "helvetica" | "arial" => Some(Family::Sans),
        "times" | "timesroman" | "timesnewroman" => Some(Family::Serif),
        "courier" | "couriernew" => Some(Family::Mono),
        "symbol" => Some(Family::Symbol),
        "zapfdingbats" | "dingbats" => Some(Family::Dingbats),
        _ => None,
    }
}

/// The family `pdf-font` classifies a face name into — §9.8's reading of a font the processor
/// does not have, from the name alone — with nothing read from the machine: the request is built
/// and classified, never looked up.
fn classified(name: &str) -> Family {
    use pdf_syntax::{Dictionary, Name, Object};
    let stem: String = name.chars().filter(|c| !c.is_whitespace()).collect();
    let mut dict = Dictionary::new();
    for (key, value) in [("Type", "Font"), ("Subtype", "Type1"), ("BaseFont", &stem)] {
        dict.insert(
            Name::new(key.as_bytes().to_vec()),
            Object::Name(Name::new(value.as_bytes().to_vec())),
        );
    }
    let request =
        pdf_font::substitute::Request::derive(&pdf_syntax::Document::empty(), &dict, None);
    match request.family {
        pdf_font::substitute::Family::Serif => Family::Serif,
        pdf_font::substitute::Family::Monospace => Family::Mono,
        pdf_font::substitute::Family::Symbol => Family::Symbol,
        pdf_font::substitute::Family::ZapfDingbats => Family::Dingbats,
        _ => Family::Sans,
    }
}

/// One glyph of a paragraph, in the order the string stores it.
struct Item<'a> {
    /// What is drawn: the stored character, or its joined form.
    character: char,
    /// The byte of the paragraph's text its stored character starts at, which is what UAX #9's
    /// levels are indexed by.
    byte: usize,
    /// The bytes of the paragraph's text it draws, a lam-alef ligature's two letters.
    bytes: usize,
    /// The run it belongs to.
    run: &'a RichRun,
    /// Which of the paragraph's [`Setting`]s draws it: its run's.
    setting: usize,
    /// How far it advances the line, in the window's pixels: its width at the run's size, with the
    /// run's letter spacing after it, both under the run's horizontal scale.
    advance: f32,
    /// Whether it is a space, which justification widens and a line does not end on.
    space: bool,
    /// Whether it is XHTML's `br`, where the line ends whatever the width.
    end: bool,
    /// Whether it is chapter 27's tab, which advances to the paragraph's next stop: how far is
    /// decided where its line is laid out ([`lines`]), and it draws no glyph.
    tab: bool,
    /// For a tab, the stop it reached, by its place among the paragraph's stops: the leader drawn
    /// across its advance is that stop's.
    stop: Option<usize>,
}

/// How a run is drawn: its family and style, its em in pixels across and up, and how far its
/// baseline is raised.
struct Setting {
    /// The face.
    face: (Family, Style),
    /// The em, in pixels: its width under the horizontal scale and its height under the vertical.
    em: (f32, f32),
    /// The run's letter spacing, in pixels before the horizontal scale.
    spacing: f32,
    /// How far the baseline is raised, in pixels; negative is lowered.
    rise: f32,
}

impl Setting {
    /// How `run` is drawn in a window whose base text size is `size` pixels.
    fn of(chrome: &Chrome, run: &RichRun, size: f32, per_point: f32) -> Self {
        let face = (
            family(run),
            Style {
                bold: run.bold,
                italic: run.italic,
            },
        );
        let em = viewer_host::popup::size(run, size, per_point);
        let space = chrome.glyph_advance(face.0, face.1, ' ');
        Self {
            face,
            em: (em * run.horizontal_scale, em * run.vertical_scale),
            spacing: viewer_host::popup::letter_spacing(run, size, per_point, Some(space))
                .unwrap_or_default(),
            rise: viewer_host::popup::rise(run, size, per_point),
        }
    }

    /// How far a glyph `advance` ems wide moves the line.
    fn advance(&self, run: &RichRun, advance: f32) -> f32 {
        self.em
            .0
            .mul_add(advance, self.spacing * run.horizontal_scale)
    }
}

/// A paragraph's runs as one text, its glyphs in stored order, joined and measured, and how each
/// run is drawn.
fn glyphs_of<'a>(
    chrome: &Chrome,
    paragraph: &'a RichParagraph,
    size: f32,
    per_point: f32,
) -> (String, Vec<Item<'a>>, Vec<Setting>) {
    let mut text = String::new();
    let mut owners: Vec<(usize, usize, &'a RichRun)> = Vec::new();
    for (index, run) in paragraph.runs.iter().enumerate() {
        for character in run.text.chars() {
            owners.push((text.len(), index, run));
            text.push(character);
        }
    }
    let letters: Vec<char> = text.chars().collect();
    let settings: Vec<Setting> = paragraph
        .runs
        .iter()
        .map(|run| Setting::of(chrome, run, size, per_point))
        .collect();
    let mut out = Vec::with_capacity(letters.len());
    for shaped in pdf_font::shaping::shape(&letters) {
        let Some(&(byte, setting, run)) = owners.get(shaped.source) else {
            continue;
        };
        let final_letter = shaped.joined_with.unwrap_or(shaped.source);
        let bytes = owners
            .get(final_letter)
            .map_or(text.len(), |(at, _, _)| *at)
            .saturating_add(
                letters
                    .get(final_letter)
                    .map_or(0, |character| character.len_utf8()),
            )
            .saturating_sub(byte);
        let stored = letters.get(shaped.source).copied().unwrap_or(' ');
        let end = stored == '\n';
        let tab = stored == '\t';
        let advance = match settings.get(setting) {
            Some(setting) if !end && !tab => setting.advance(
                run,
                chrome.glyph_advance(setting.face.0, setting.face.1, shaped.character),
            ),
            _ => 0.0,
        };
        out.push(Item {
            character: shaped.character,
            byte,
            bytes,
            run,
            setting,
            advance,
            space: stored == ' ',
            end,
            tab,
            stop: None,
        });
    }
    (text, out, settings)
}

/// How far a tab at `position` from the paragraph's left margin advances before `group`, the
/// glyphs after it up to the next tab or the line's end, and which of `stops` it reached: the
/// first past the cursor in the direction the text flows, the group standing there as the stop's
/// side says, and by nothing where no stop lies past it — chapter 27's *Tab Stops* (pages 1205 to
/// 1207), as a field's layout reads it.
///
/// Where the paragraph reads right to left the cursor moves leftward and the next stop is the
/// nearest on its left (chapter 2's *Tab Stops*, page 61): the answer is how far the group's
/// right edge lies left of the cursor.
fn tab_advance(
    stops: &[viewer_host::popup::TabStop],
    (position, right_to_left): (f32, bool),
    group: &[Item<'_>],
) -> (f32, Option<usize>) {
    use viewer_host::popup::TabSide;
    let reached = if right_to_left {
        stops
            .iter()
            .rposition(|stop| stop.at < position - f32::EPSILON)
    } else {
        stops
            .iter()
            .position(|stop| stop.at > position + f32::EPSILON)
    };
    let Some((index, stop)) = reached.and_then(|index| Some((index, stops.get(index)?))) else {
        return (0.0, None);
    };
    let whole: f32 = group.iter().map(|item| item.advance).sum();
    let lead = match stop.side {
        TabSide::Left => 0.0,
        TabSide::Centre => whole * 0.5,
        TabSide::Right => whole,
        TabSide::Decimal => {
            group
                .iter()
                .position(|item| item.character == '.')
                .map_or(whole, |radix| {
                    group
                        .get(..radix)
                        .unwrap_or_default()
                        .iter()
                        .map(|item| item.advance)
                        .sum()
                })
        }
    };
    let advance = if right_to_left {
        position - (stop.at - lead + whole)
    } else {
        stop.at - lead - position
    };
    (advance.max(0.0), Some(index))
}

/// A paragraph's glyphs broken into lines that fit `available`, each a range of them in stored
/// order: at the spaces between words where it can, and by glyph where a word is wider than the
/// line, because the window is the document's rectangle and there is nowhere else for it to go.
///
/// A tab's advance is set here, where its place on the line is known: `stops` from the left
/// margin, which lies `origin` before the line's first glyph in the order the text flows — a
/// list tag's width, which the two toolkit windows set inside the line (ADR 1666), or, in a
/// paragraph read right to left, the line's whole width, its first glyph being at its right.
fn lines(
    items: &mut [Item<'_>],
    available: f32,
    (stops, origin, right_to_left): (&[viewer_host::popup::TabStop], f32, bool),
) -> Vec<std::ops::Range<usize>> {
    let mut lines = Vec::new();
    let (mut from, mut used, mut at) = (0, 0.0_f32, 0);
    while let Some(item) = items.get(at) {
        if item.end {
            lines.push(from..at.saturating_add(1));
            from = at.saturating_add(1);
            used = 0.0;
            at = from;
            continue;
        }
        if item.tab {
            let group_end = (at.saturating_add(1)..items.len())
                .find(|&next| items.get(next).is_none_or(|other| other.end || other.tab))
                .unwrap_or(items.len());
            let group = items
                .get(at.saturating_add(1)..group_end)
                .unwrap_or_default();
            let position = if right_to_left {
                origin - used
            } else {
                origin + used
            };
            let (advance, stop) = tab_advance(stops, (position, right_to_left), group);
            let advance = advance.min((available - used).max(0.0));
            if let Some(tab) = items.get_mut(at) {
                tab.advance = advance;
                tab.stop = stop;
            }
            used += advance;
            at = at.saturating_add(1);
            continue;
        }
        let space = item.space;
        let word_end = (at..items.len())
            .find(|&next| {
                items
                    .get(next)
                    .is_none_or(|other| other.end || other.tab || other.space != space)
            })
            .unwrap_or(items.len());
        let width: f32 = items
            .get(at..word_end)
            .unwrap_or_default()
            .iter()
            .map(|item| item.advance)
            .sum();
        if !space && used + width > available && at > from {
            lines.push(from..at);
            from = at;
            used = 0.0;
        }
        if !space && width > available {
            for (offset, glyph) in items
                .get(at..word_end)
                .unwrap_or_default()
                .iter()
                .enumerate()
            {
                let index = at.saturating_add(offset);
                if used + glyph.advance > available && index > from {
                    lines.push(from..index);
                    from = index;
                    used = 0.0;
                }
                used += glyph.advance;
            }
        } else {
            used += width;
        }
        at = word_end;
    }
    if from < items.len() || lines.is_empty() {
        lines.push(from..items.len());
    }
    lines
}

/// Table 172's `/RC` drawn run by run, as the module's header says: each run in its face, weight,
/// posture, size, colour, spacing and scales, with its underlines, its line through and its rise;
/// a paragraph aligned as it states and ordered as UAX #9 orders it; a list item indented under
/// its tag (ADRs 1642, 1654).
///
/// `box_` is `(left, top, room, bottom)`, the top being where the first line's em begins; the
/// answer is where the next line's baseline would go, which is what the thread below takes.
/// Sizes come from `viewer_host::popup::size`, so this window holds a run between the bounds the
/// two toolkits hold it between.
pub(super) fn draw(
    chrome: &Chrome,
    list: &mut DisplayList,
    note: &RichNote,
    box_: (f32, f32, f32, f32),
    size: f32,
    scale: f32,
) -> f32 {
    lay_out(chrome, (Some(list), None), note, box_, size, scale)
}

/// One glyph [`draw`] drew, where it drew it: what a caret over a rich note stands beside.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Placed {
    /// Which of the note's paragraphs it is in.
    pub(super) paragraph: usize,
    /// The bytes of the paragraph's runs' text, taken together, that it draws — empty for a line
    /// that draws nothing, which still has a place at its start.
    pub(super) stored: std::ops::Range<usize>,
    /// Where its stored text begins and ends across the window, in the window's pixels: the left
    /// edge then the right for a glyph read left to right, the right then the left for one read
    /// right to left, UAX #9's level deciding which.
    pub(super) edges: (f32, f32),
    /// Its line's baseline.
    pub(super) baseline: f32,
    /// Its line's tallest em, which is how far above the baseline the line reaches.
    pub(super) height: f32,
}

/// Every glyph [`draw`] would draw for `note` in `box_`, where it would draw it, and nothing
/// drawn: the one layout read twice, so that a caret stands where the glyph is (ADR 1770).
pub(super) fn placed(
    chrome: &Chrome,
    note: &RichNote,
    box_: (f32, f32, f32, f32),
    size: f32,
    scale: f32,
) -> Vec<Placed> {
    let mut places = Vec::new();
    lay_out(chrome, (None, Some(&mut places)), note, box_, size, scale);
    places
}

/// [`draw`]'s layout, drawing into the list where one is given and saying where each glyph went
/// where `places` is given.
fn lay_out(
    chrome: &Chrome,
    (mut list, mut places): (Option<&mut DisplayList>, Option<&mut Vec<Placed>>),
    note: &RichNote,
    box_: (f32, f32, f32, f32),
    size: f32,
    scale: f32,
) -> f32 {
    let (left, mut top, room, bottom) = box_;
    let per_point = PIXELS_PER_POINT * scale;
    for (number, paragraph) in note.paragraphs.iter().enumerate() {
        let tag = paragraph
            .tag
            .as_ref()
            .map(|tag| (tag, Setting::of(chrome, tag, size, per_point)));
        let indent = f32::from(paragraph.level) * size * 2.0;
        let tag_width = tag.as_ref().map_or(0.0, |(tag, setting)| {
            draw_tag(chrome, None, (tag, setting), (0.0, 0.0)) + size * 0.5
        });
        let right_to_left = viewer_host::popup::right_to_left(paragraph);
        // The indent and the tag are at the paragraph's start edge, the right for one read right
        // to left (ADR 1666).
        let start = if right_to_left {
            left
        } else {
            left + indent + tag_width
        };
        let available = (room - indent - tag_width).max(size);
        let (text, mut items, settings) = glyphs_of(chrome, paragraph, size, per_point);
        let levels = pdf_font::shaping::Paragraphs::new(&text);
        let stops = viewer_host::popup::tab_stops(paragraph, size, per_point, room);
        let (origin, margin) = if right_to_left {
            (available, left)
        } else {
            (tag_width, start - tag_width)
        };
        let broken = lines(&mut items, available, (&stops, origin, right_to_left));
        let final_line = broken.len().saturating_sub(1);
        for (index, range) in broken.iter().enumerate() {
            let line = items.get(range.clone()).unwrap_or_default();
            let line_from = line.first().map_or(0, |item| item.byte);
            let ends_paragraph = index == final_line || line.last().is_some_and(|item| item.end);
            let line = trimmed(line);
            let tallest = line
                .iter()
                .filter_map(|item| settings.get(item.setting))
                .map(|setting| setting.em.1)
                .fold(
                    tag.as_ref().map_or(size, |(_, setting)| setting.em.1),
                    f32::max,
                );
            let baseline = top + tallest;
            if baseline > bottom {
                return baseline;
            }
            let align = paragraph.align.unwrap_or(if right_to_left {
                RichAlign::Right
            } else {
                RichAlign::Left
            });
            let (mut x, widen) = line_start(align, (start, available), line, ends_paragraph);
            if index == 0
                && let Some((tag, setting)) = &tag
            {
                let at = if right_to_left {
                    left + room - indent - (tag_width - size * 0.5)
                } else {
                    left + indent
                };
                draw_tag(chrome, list.as_deref_mut(), (tag, setting), (at, baseline));
            }
            let shown = ordered(line, levels.as_ref());
            if let Some(places) = places.as_deref_mut() {
                let line = Placed {
                    paragraph: number,
                    stored: line_from..line_from,
                    edges: (x, x),
                    baseline,
                    height: tallest,
                };
                place_line(places, line, &shown, widen);
            }
            for (item, _) in shown {
                let advance = item.advance + if item.space { widen } else { 0.0 };
                if let (Some(list), Some(setting)) =
                    (list.as_deref_mut(), settings.get(item.setting))
                {
                    if let Some(stop) = item.stop.and_then(|stop| stops.get(stop)) {
                        draw_leader(
                            chrome,
                            list,
                            (item, setting, stop),
                            ((x, baseline), advance, margin),
                        );
                    }
                    draw_item(
                        chrome,
                        list,
                        (item, setting),
                        levels.as_ref(),
                        (x, baseline),
                        advance,
                    );
                }
                x += advance;
            }
            top = baseline + tallest * 0.25;
        }
    }
    top + size
}

/// A line without the spaces it broke at, at either end, or its `br`, none of which it draws.
fn trimmed<'l, 'a>(line: &'l [Item<'a>]) -> &'l [Item<'a>] {
    let lead = line
        .iter()
        .position(|item| !item.space && !item.end)
        .unwrap_or(line.len());
    let tail = line
        .iter()
        .rposition(|item| !item.space && !item.end)
        .map_or(lead, |at| at.saturating_add(1));
    line.get(lead..tail.max(lead)).unwrap_or_default()
}

/// Where each glyph of a line drawn left to right as `shown` stands, from `start` — the line's
/// paragraph, its first stored byte, where it starts across and its baseline and height — each
/// space `widen` wider, as the drawing below advances them: one place a glyph, at the edges of its
/// stored characters, and one place at the start of a line that draws nothing.
fn place_line(places: &mut Vec<Placed>, start: Placed, shown: &[(&Item<'_>, u8)], widen: f32) {
    let mut x = start.edges.0;
    if shown.is_empty() {
        places.push(start);
        return;
    }
    for (item, level) in shown {
        let advance = item.advance + if item.space { widen } else { 0.0 };
        let edges = if level % 2 == 1 {
            (x + advance, x)
        } else {
            (x, x + advance)
        };
        places.push(Placed {
            stored: item.byte..item.byte.saturating_add(item.bytes),
            edges,
            ..start.clone()
        });
        x += advance;
    }
}

/// Where a line of `line`'s glyphs starts in the room from `start` that is `available` wide, and
/// how much wider each of its spaces is drawn, as `align` places it; a paragraph's last line is
/// not justified.
fn line_start(
    align: RichAlign,
    (start, available): (f32, f32),
    line: &[Item<'_>],
    ends_paragraph: bool,
) -> (f32, f32) {
    let width: f32 = line.iter().map(|item| item.advance).sum();
    let spaces = line.iter().filter(|item| item.space).count();
    match align {
        RichAlign::Centre => (start + (available - width) / 2.0, 0.0),
        RichAlign::Right => (start + available - width, 0.0),
        #[expect(
            clippy::cast_precision_loss,
            reason = "a line's count of spaces is far below f32's exact integer range"
        )]
        RichAlign::Justify if !ends_paragraph && spaces > 0 => {
            (start, (available - width).max(0.0) / spaces as f32)
        }
        RichAlign::Justify | RichAlign::Left => (start, 0.0),
    }
}

/// A tab stop's leader across the `room` a tab advanced from `from`, on the line's baseline: chapter 2's *Tab Leader Pattern*
/// (pages 63 to 65), as a field's appearance draws it (ADR 1660), in this window's pixels.
///
/// The cycles are laid on a grid from the paragraph's left margin, `margin`, so that leaders on
/// different lines line up — `leaderAlignment`'s `none` leaves that to the processor — and a cycle
/// the room cannot hold whole is left blank, as the chapter has a processor leave a partial one. A
/// cycle is the larger of `leaderPatternWidth` and the pattern's own width; dots and content are
/// the tab's run's own glyphs, and a rule is drawn in its colour, centred on the baseline, a
/// dashed or dotted one in pieces (ADR 1679). The grid and the pieces are `viewer_host::popup`'s,
/// which the two toolkit windows paint by over their own lines (ADR 1722).
fn draw_leader(
    chrome: &Chrome,
    list: &mut DisplayList,
    (item, setting, stop): (&Item<'_>, &Setting, &viewer_host::popup::TabStop),
    ((from, baseline), room, margin): ((f32, f32), f32, f32),
) {
    use viewer_host::popup::LeaderPattern;
    let Some(leader) = stop.leader.as_ref() else {
        return;
    };
    let to = from + room;
    let colour = item.run.colour.unwrap_or(Color::BLACK);
    let glyphs = |text: &str| -> Vec<(char, f32)> {
        text.chars()
            .map(|character| {
                let advance = setting.advance(
                    item.run,
                    chrome.glyph_advance(setting.face.0, setting.face.1, character),
                );
                (character, advance)
            })
            .collect()
    };
    match &leader.pattern {
        LeaderPattern::Dots | LeaderPattern::Content(_) => {
            let glyphs = glyphs(match &leader.pattern {
                LeaderPattern::Content(content) => content.as_str(),
                _ => ".",
            });
            let inherent: f32 = glyphs.iter().map(|(_, advance)| advance).sum();
            let cycle = inherent.max(leader.width);
            for at in viewer_host::popup::leader_cycles(from, to, margin, cycle) {
                let mut x = at;
                for (character, advance) in &glyphs {
                    if *character != ' ' {
                        chrome.glyph(
                            list,
                            setting.face,
                            *character,
                            (x, baseline - setting.rise),
                            setting.em,
                            colour,
                        );
                    }
                    x += advance;
                }
            }
        }
        LeaderPattern::Rule { style, thickness } => {
            let thick = viewer_host::popup::rule_thickness(*thickness, setting.em.1);
            let top = baseline - setting.rise - thick * 0.5;
            for (x, width) in viewer_host::popup::rule_pieces(*style, thick, (from, to), margin) {
                rectangle(list, (x, top, width, thick), colour);
            }
        }
    }
}

/// A list item's tag at `at` on its baseline, drawn where `list` is given and only measured where
/// it is not; the answer is its width.
///
/// A tag is chapter 27's generated number or bullet rather than the string's characters, so it is
/// set as one left-to-right label in its own run's style and is no part of the paragraph's order;
/// it stands at the paragraph's start edge, the right for one read right to left (ADR 1666).
fn draw_tag(
    chrome: &Chrome,
    mut list: Option<&mut DisplayList>,
    (tag, setting): (&RichRun, &Setting),
    (left, baseline): (f32, f32),
) -> f32 {
    let colour = tag.colour.unwrap_or(Color::BLACK);
    let mut at = left;
    for character in tag.text.chars() {
        if let Some(list) = list.as_deref_mut() {
            chrome.glyph(
                list,
                setting.face,
                character,
                (at, baseline - setting.rise),
                setting.em,
                colour,
            );
        }
        at += setting.advance(
            tag,
            chrome.glyph_advance(setting.face.0, setting.face.1, character),
        );
    }
    at - left
}

/// A line's glyphs left to right on the screen, each with the level it resolved to: UAX #9's rule
/// L1 over the line's own bytes and rule L2's reversals over its glyphs, the levels resolved over
/// the whole paragraph.
fn ordered<'l, 'a>(
    line: &'l [Item<'a>],
    levels: Option<&pdf_font::shaping::Paragraphs<'_>>,
) -> Vec<(&'l Item<'a>, u8)> {
    let (Some(levels), Some(first), Some(last)) = (levels, line.first(), line.last()) else {
        return line.iter().map(|item| (item, 0)).collect();
    };
    let from = first.byte;
    let by_byte = levels.line_levels(from..last.byte.saturating_add(last.bytes));
    let per_glyph: Vec<u8> = line
        .iter()
        .map(|item| {
            by_byte
                .get(item.byte.saturating_sub(from))
                .copied()
                .unwrap_or(0)
        })
        .collect();
    pdf_font::shaping::visual_order(&per_glyph)
        .into_iter()
        .filter_map(|at| Some((line.get(at)?, per_glyph.get(at).copied().unwrap_or(0))))
        .collect()
}

/// One glyph at its place on the line, with the lines its run states.
///
/// The underline a tenth of an em below the baseline and the line through 0.28 em above it, each
/// a twentieth of an em thick: `pdf_model::rich_text`'s choices for a field's appearance (ADR
/// 1634 section 5), kept so that one note does not underline two ways in one program.
fn draw_item(
    chrome: &Chrome,
    list: &mut DisplayList,
    (item, setting): (&Item<'_>, &Setting),
    levels: Option<&pdf_font::shaping::Paragraphs<'_>>,
    (x, baseline): (f32, f32),
    advance: f32,
) {
    let run = item.run;
    let raised = baseline - setting.rise;
    let colour = run.colour.unwrap_or(Color::BLACK);
    if !item.space && !item.tab {
        // Rule L4: a character at an odd level is drawn as its mirror, where it has one.
        let shown = pdf_font::shaping::displayed(item.character, item.byte, levels);
        chrome.glyph(list, setting.face, shown, (x, raised), setting.em, colour);
    }
    let em = setting.em.1;
    let thick = (em * 0.05).max(1.0);
    if run.underlines > 0 && !(item.space && run.underline_by_word) {
        rectangle(list, (x, raised + em * 0.1, advance, thick), colour);
        if run.underlines > 1 {
            rectangle(
                list,
                (x, raised + em * 0.1 + thick * 2.0, advance, thick),
                colour,
            );
        }
    }
    if run.line_through {
        rectangle(list, (x, raised - em * 0.28, advance, thick), colour);
    }
}

/// What a rich note states and this window did not draw, said under it, and where the next line
/// goes: chapter 27's properties `pdf-model` does not carry out — this window sets every face,
/// order, spacing, scale, tab stop and leader the note states, a right-to-left paragraph's
/// leftward (ADRs 1654, 1666, 1679).
pub(super) fn say_what_was_not_drawn(
    chrome: &Chrome,
    list: &mut DisplayList,
    note: &RichNote,
    (left, line, room, bottom): (f32, f32, f32, f32),
    size: f32,
) -> f32 {
    let Some(sentence) = viewer_host::popup::not_drawn(note, &[]) else {
        return line;
    };
    if line > bottom {
        return line;
    }
    chrome.text(
        list,
        &elide(chrome, &sentence, size * 0.85, Style::default(), room),
        (left, line),
        size * 0.85,
        Style::default(),
        DIMMED,
    );
    line + size * 1.25
}

#[cfg(test)]
mod tests {
    use super::{Family, family};

    fn named(families: &[&str]) -> pdf_model::popup::RichRun {
        pdf_model::popup::RichRun {
            text: String::new(),
            families: families.iter().map(|name| (*name).to_owned()).collect(),
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

    /// A search path is walked to the first family the window has, a metric-compatible name
    /// being that family, and a run naming none is the window's own face.
    #[test]
    fn a_runs_search_path_reaches_the_first_family_the_window_has() {
        assert_eq!(family(&named(&[])), Family::Sans);
        assert_eq!(family(&named(&["Courier"])), Family::Mono);
        assert_eq!(family(&named(&["Courier New"])), Family::Mono);
        assert_eq!(family(&named(&["TimesNewRomanPSMT"])), Family::Serif);
        assert_eq!(family(&named(&["Myriad Pro", "Times"])), Family::Serif);
        assert_eq!(family(&named(&["Arial", "Courier"])), Family::Sans);
        assert_eq!(family(&named(&["ZapfDingbats"])), Family::Dingbats);
    }
}
