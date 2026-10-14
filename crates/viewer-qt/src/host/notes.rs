//! §12.5.6.14's popup windows and §12.5.6.4's note editor: the furniture Qt places, Table 172's
//! `/RC` as the runs a `QTextDocument` is built from, and the caret between the editor's positions
//! and `/Contents`' bytes (ADRs 1466, 1722, 1782; filed here by ADR 1908).

use viewer_core::{Command, Edit};

use crate::bridge::ffi::{QtNoteSpan, QtPopup, QtRichParagraph, QtRichRun, QtTab};

use super::{Host, level, packed};

impl Host {
    /// §12.5.6.4: the note's text as a person typed it into its window, sent whole as a field's
    /// value is (ADR 1726). The note is the one `QtPopup` named; `viewer-core` refuses and says any
    /// other.
    pub(crate) fn set_note(&mut self, number: u32, generation: u16, text: &str) {
        self.dispatch(Command::Edit(Edit::SetNoteText {
            annotation: pdf_syntax::ObjectId::new(number, generation),
            text: text.to_owned(),
        }));
    }

    /// Where a press on a note's rich window puts the caret in its `/Contents`: `position` is the
    /// place `QTextDocument`'s own layout found under the press, in the document
    /// `rich_paragraphs` built, and the answer is that character's position in the plain editor's
    /// document ([`editor_position`]); -1 where there is none (ADR 1770).
    ///
    /// The document's text is every paragraph's runs as handed over — a list item's tag first,
    /// and a tab taken out where the paragraph's tabs advance by nothing — with one separator
    /// between paragraphs and each XHTML `br` a block separator of its own, every one a single
    /// unit; the runs' own characters are aligned to `/Contents` by
    /// `viewer_host::popup::rich_offsets`, as in the other two windows.
    pub(crate) fn note_place(&self, number: u32, generation: u16, position: i32) -> i32 {
        let note = pdf_syntax::ObjectId::new(number, generation);
        let place = (|| {
            let window = self
                .popups_shown
                .iter()
                .find(|window| window.note == Some(note))?;
            let contents = window.text.as_deref().unwrap_or_default();
            let rich = window.rich.as_ref()?;
            let tables = viewer_host::popup::rich_offsets(contents, rich)?;
            let mut left = usize::try_from(position).ok()?;
            for (paragraph, (stated, handed)) in rich
                .paragraphs
                .iter()
                .zip(rich_paragraphs(rich, 10.0, 1.0))
                .enumerate()
            {
                let texts: Vec<&str> = handed.runs.iter().map(|run| run.text.as_str()).collect();
                let units: usize = texts.iter().map(|text| text.encode_utf16().count()).sum();
                if left > units {
                    // Past this paragraph and the separator after it.
                    left = left.saturating_sub(units.saturating_add(1));
                    continue;
                }
                let skipped = usize::from(handed.tagged);
                let tag_units: usize = texts
                    .iter()
                    .take(skipped)
                    .map(|text| text.encode_utf16().count())
                    .sum();
                let mut within = left.saturating_sub(tag_units);
                // Walk the stated runs' characters, skipping a tab the handed text does not hold,
                // until `within` units of the handed text are behind.
                let advances = viewer_host::popup::advances(stated);
                let mut byte = 0_usize;
                for run in &stated.runs {
                    for character in run.text.chars() {
                        if within == 0 {
                            break;
                        }
                        if character != '\t' || advances {
                            within = within.saturating_sub(character.len_utf16());
                        }
                        byte = byte.saturating_add(character.len_utf8());
                    }
                }
                let offset = *tables.get(paragraph)?.get(byte)?;
                return i32::try_from(editor_position(contents, offset)?).ok();
            }
            None
        })();
        place.unwrap_or(-1)
    }

    /// Says that a note's window took the keyboard, and the byte of `/Contents` its caret stands
    /// before, as `quorra` says it — `position` is the editor's, in its plain text document — or
    /// that the page has it again (ADR 1726).
    pub(crate) fn note_editing(
        &mut self,
        number: u32,
        generation: u16,
        editing: bool,
        position: i32,
    ) {
        if editing {
            let note = pdf_syntax::ObjectId::new(number, generation);
            let contents = self
                .popups_shown
                .iter()
                .find(|window| window.note == Some(note))
                .and_then(|window| window.text.as_deref())
                .unwrap_or_default();
            let at = usize::try_from(position)
                .map_or(contents.len(), |position| contents_byte(contents, position));
            self.say(&format!(
                "typing into the text note {number} {generation} at {at} of {}",
                contents.len()
            ));
        } else {
            self.say("the keyboard is back on the page");
        }
    }

    /// §12.5.6.14's open popup windows, as the furniture Qt places.
    ///
    /// The clause gives a popup "no appearance stream or associated actions of its own", so there
    /// is nothing of it in the page's pixels and a window is the platform's to draw. The three
    /// texts and the box are `viewer_host::popup`'s, so this host and `viewer-gtk` say the same
    /// thing about one clause.
    pub(crate) fn popups(&self, base: f32) -> Vec<QtPopup> {
        // Qt answers -1 for a font sized in pixels, which has no point size to scale by; ten
        // points is then the base, which is a common label size and only a stand-in.
        let base = if base > 0.0 { base } else { 10.0 };
        let placed = viewer_host::popup::windows(&self.popups_shown);
        placed
            .iter()
            .map(|window| {
                let (x, y, width, height) = window.place;
                let colour = window
                    .colour
                    .map(|colour| (level(colour.r), level(colour.g), level(colour.b)));
                QtPopup {
                    x,
                    y,
                    width,
                    height,
                    title: window.title.to_owned(),
                    modified: viewer_host::popup::modified(window).unwrap_or_default(),
                    text: window.text.to_owned(),
                    thread: viewer_host::popup::thread(window),
                    rich: window
                        .rich
                        .map(|note| rich_paragraphs(note, base, width))
                        .unwrap_or_default(),
                    not_drawn: window
                        .rich
                        // The window sets both font scales and measures a space for a spacing
                        // given as a share of one; what Qt's tab positions cannot say is said
                        // (ADRs 1654, 1666).
                        .and_then(|note| {
                            viewer_host::popup::not_drawn(
                                note,
                                &viewer_host::popup::toolkit_unapplied(note, true),
                            )
                        })
                        .unwrap_or_default(),
                    rich_thread: viewer_host::popup::thread_html(window, base).unwrap_or_default(),
                    coloured: colour.is_some(),
                    red: colour.map_or(0, |rgb| rgb.0),
                    green: colour.map_or(0, |rgb| rgb.1),
                    blue: colour.map_or(0, |rgb| rgb.2),
                    paper: packed(viewer_host::popup::PAPER),
                    edge: packed(viewer_host::popup::EDGE),
                    retypes: window.note.is_some(),
                    note_number: window.note.map_or(0, |note| note.number),
                    note_generation: window.note.map_or(0, |note| note.generation),
                    spans: window
                        .rich
                        .map(|note| note_spans(window.text, note))
                        .unwrap_or_default(),
                    editor_not_drawn: window
                        .rich
                        .and_then(|note| viewer_host::popup::editor_not_drawn(note, true))
                        .unwrap_or_default(),
                }
            })
            .collect()
    }
}

/// Table 172's `/RC` as the popup window's `QTextDocument` is built from it, paragraph by paragraph
/// and run by run (ADR 1666).
///
/// `base` is the window's text size in points and `room` its width, which bounds how many default
/// tab stops a paragraph is handed. A list tag is wrapped in a left-to-right isolate, UAX #9's
/// LRI and PDI, so that it reads as one label at the paragraph's start edge and takes no part in
/// its order; a paragraph whose tabs advance by nothing has its tab characters taken out, so that
/// Qt's own default stops are not used where the chapter sets none.
fn rich_paragraphs(
    note: &pdf_model::popup::RichNote,
    base: f32,
    room: f32,
) -> Vec<QtRichParagraph> {
    note.paragraphs
        .iter()
        .map(|paragraph| {
            let advances = viewer_host::popup::advances(paragraph);
            let tag = paragraph.tag.as_ref().map(|tag| {
                let mut run = rich_run(tag, base, true);
                run.text = format!("\u{2066}{}\u{2069} ", run.text);
                run
            });
            let runs = tag
                .into_iter()
                .chain(
                    paragraph
                        .runs
                        .iter()
                        .map(|run| rich_run(run, base, advances)),
                )
                .collect();
            QtRichParagraph {
                align: match paragraph.align {
                    None => 0,
                    Some(pdf_model::popup::RichAlign::Left) => 1,
                    Some(pdf_model::popup::RichAlign::Centre) => 2,
                    Some(pdf_model::popup::RichAlign::Right) => 3,
                    Some(pdf_model::popup::RichAlign::Justify) => 4,
                },
                right_to_left: viewer_host::popup::right_to_left(paragraph),
                indent: f32::from(paragraph.level) * base * 2.0,
                tagged: paragraph.tag.is_some(),
                runs,
                tabs: viewer_host::popup::tab_stops(paragraph, base, 1.0, room)
                    .into_iter()
                    .map(|stop| qt_tab(&stop))
                    .collect(),
            }
        })
        .collect()
}

/// Where each run of a rich note stands in its editor's plain text document, the run counted as
/// [`rich_paragraphs`] hands it over — a list tag first where there is one (ADR 1782).
fn note_spans(contents: &str, note: &pdf_model::popup::RichNote) -> Vec<QtNoteSpan> {
    let Some(spans) = viewer_host::popup::run_spans(contents, note) else {
        return Vec::new();
    };
    spans
        .iter()
        .filter_map(|span| {
            let tagged = note.paragraphs.get(span.paragraph)?.tag.is_some();
            Some(QtNoteSpan {
                start: i32::try_from(editor_position(contents, span.start)?).ok()?,
                end: i32::try_from(editor_position(contents, span.end)?).ok()?,
                paragraph: u32::try_from(span.paragraph).ok()?,
                run: u32::try_from(span.run.saturating_add(usize::from(tagged))).ok()?,
            })
        })
        .collect()
}

/// The position in a plain text document Qt built from `contents` of the character at byte
/// `byte`, or `None` inside a character.
///
/// Qt counts in UTF-16 units and makes each line break one block separator of one unit — a
/// carriage return, a line feed, and the two together (`QTextCursor::insertText`) — so a note
/// whose lines end in `"\r\n"` is one unit shorter per line than its UTF-16 count.
pub(super) fn editor_position(contents: &str, byte: usize) -> Option<usize> {
    let before = contents.get(..byte)?;
    let pairs = before.matches("\r\n").count();
    Some(before.encode_utf16().count().saturating_sub(pairs))
}

/// The byte of `contents` at `position` of the plain text document Qt built from it, the reverse
/// of [`editor_position`]; the end of `contents` past its last position.
pub(super) fn contents_byte(contents: &str, position: usize) -> usize {
    let mut left = position;
    let mut characters = contents.char_indices().peekable();
    while let Some((byte, character)) = characters.next() {
        if left == 0 {
            return byte;
        }
        if character == '\r' && characters.peek().is_some_and(|(_, next)| *next == '\n') {
            characters.next();
            left = left.saturating_sub(1);
        } else {
            left = left.saturating_sub(character.len_utf16());
        }
    }
    contents.len()
}

/// One placed stop as the bridge carries it, its leader with it (ADR 1722).
fn qt_tab(stop: &viewer_host::popup::TabStop) -> QtTab {
    use pdf_model::popup::RichRuleStyle;
    use viewer_host::popup::LeaderPattern;
    let (leader, rule_style, rule_thickness, content) =
        match stop.leader.as_ref().map(|leader| &leader.pattern) {
            None => (0, 0, -1.0, String::new()),
            Some(LeaderPattern::Dots) => (1, 0, -1.0, String::new()),
            Some(LeaderPattern::Rule { style, thickness }) => (
                2,
                match style {
                    RichRuleStyle::Solid => 0,
                    RichRuleStyle::Dashed => 1,
                    RichRuleStyle::Dotted => 2,
                },
                thickness.unwrap_or(-1.0),
                String::new(),
            ),
            Some(LeaderPattern::Content(characters)) => (3, 0, -1.0, characters.clone()),
        };
    QtTab {
        side: match stop.side {
            viewer_host::popup::TabSide::Left => 0,
            viewer_host::popup::TabSide::Centre => 1,
            viewer_host::popup::TabSide::Right => 2,
            viewer_host::popup::TabSide::Decimal => 3,
        },
        at: stop.at,
        leader,
        leader_width: stop.leader.as_ref().map_or(0.0, |leader| leader.width),
        rule_style,
        rule_thickness,
        content,
    }
}

/// [`viewer_host::popup::reached_stop`] as the bridge hands it to `RichNoteView`, whose stops
/// are pixels from the paragraph's left margin: the stop's place, or −1 (ADR 1722).
pub(crate) fn reached_stop(positions: &[f32], from: f32, right_to_left: bool) -> i32 {
    viewer_host::popup::reached_stop(positions, from, right_to_left)
        .and_then(|reached| i32::try_from(reached).ok())
        .unwrap_or(-1)
}

/// [`viewer_host::popup::leader_cycles`], for the grid `RichNoteView` paints a leader on.
pub(crate) fn leader_cycles(from: f32, to: f32, margin: f32, cycle: f32) -> Vec<f32> {
    viewer_host::popup::leader_cycles(from, to, margin, cycle)
}

/// [`viewer_host::popup::rule_pieces`], each piece's left edge and width one after the other.
pub(crate) fn rule_pieces(style: u8, thickness: f32, from: f32, to: f32, margin: f32) -> Vec<f32> {
    use pdf_model::popup::RichRuleStyle;
    let style = match style {
        1 => RichRuleStyle::Dashed,
        2 => RichRuleStyle::Dotted,
        _ => RichRuleStyle::Solid,
    };
    viewer_host::popup::rule_pieces(style, thickness, (from, to), margin)
        .into_iter()
        .flat_map(|(left, width)| [left, width])
        .collect()
}

/// [`viewer_host::popup::rule_thickness`], a thickness below zero being none stated.
pub(crate) fn rule_thickness(stated: f32, em: f32) -> f32 {
    viewer_host::popup::rule_thickness((stated >= 0.0).then_some(stated), em)
}

/// One run as [`QtRichRun`] carries it: its size and stretch from
/// `viewer_host::popup::scaled_face`, its letter spacing under its horizontal scale, a `br` as
/// U+2028, which is Qt's line separator, and its tabs kept only where `tabs` says they advance.
fn rich_run(run: &pdf_model::popup::RichRun, base: f32, tabs: bool) -> QtRichRun {
    let size = viewer_host::popup::size(run, base, 1.0);
    let (points, stretch) = viewer_host::popup::scaled_face(run, size);
    let horizontal = stretch * points / size.max(f32::EPSILON) / 100.0;
    let raised = viewer_host::popup::rise(run, base, 1.0);
    let text = run
        .text
        .chars()
        .filter(|character| tabs || *character != '\t')
        .map(|character| {
            if character == '\n' {
                '\u{2028}'
            } else {
                character
            }
        })
        .collect();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a stretch is held to QFont's own range of 1 to 4000 before it is converted"
    )]
    let stretch = stretch.round().clamp(1.0, 4000.0) as i32;
    QtRichRun {
        text,
        family: viewer_host::popup::family(run)
            .unwrap_or_default()
            .to_owned(),
        points,
        stretch,
        bold: run.bold,
        italic: run.italic,
        coloured: run.colour.is_some(),
        colour: run.colour.map_or(0, packed),
        underlines: run.underlines,
        line_through: run.line_through,
        spacing: viewer_host::popup::letter_spacing(run, base, 1.0, None).unwrap_or_default()
            * horizontal,
        spacing_of_space: match run.letter_spacing {
            pdf_model::popup::RichSpacing::OfSpace(share) => share,
            pdf_model::popup::RichSpacing::Length(_) => 0.0,
        },
        spacing_bound: size * horizontal,
        rise: if raised > 0.0 {
            1
        } else if raised < 0.0 {
            -1
        } else {
            0
        },
    }
}
