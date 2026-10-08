//! A rich text string as the flat list of styled runs a script holds: Adobe's `Span` objects.
//!
//! Table 228's `/RV` is a rich text string "as described in Adobe XML Architecture, XML Forms
//! Architecture (XFA) Specification, version 3.3", and this tree reads it through [`super::markup`]
//! into paragraphs of styled runs. A script sees the same text as an array of `Span` objects —
//! `event.richValue`, `util.xmlToSpans` — and writes one back through `util.spansToXML`. The object's
//! twelve properties are Adobe's, *JavaScript for Acrobat API Reference*, "Span properties", read at
//! `adobe/dc-acrobat-sdk-docs` commit `ab3b42a7`; every mapping between them and chapter 27's
//! styles is a documented choice under principle 5, stated on the item that makes it.
//!
//! **Paragraphs are carriage returns.** The reference's `Span` has no paragraph of its own and
//! says only that the first span on a line decides its alignment, so a paragraph boundary is a
//! `\r` at the end of the text before it — the character §12.5.6.2 separates a plain text's
//! paragraphs by, and the one [`super::RichText::text`] spells — and [`markup`] starts a new `p`
//! at each. A chapter 27 line break `br` inside a paragraph reads as the same `\r`, so it comes
//! back as a paragraph boundary: the reference's object has no way to say the difference.

use std::fmt::Write as _;

use super::markup::{self, Piece};
use super::style::{Align, Character, NORMAL_STRETCH, STRETCHES, Underline};

/// The size a span reports where the markup states its size relative to a root it does not
/// state: the reference's default text size, 12 points ("Span properties" — textSize).
pub const DEFAULT_SIZE: f32 = 12.0;

/// One run of text in one style: Adobe's `Span` object as plain data.
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the reference's object has four boolean properties, and this is that object as \
              plain data"
)]
pub struct Span {
    /// `alignment`: `left`, `center` or `right` — the reference's three — or chapter 27's
    /// `justify` and `justify-all` where the paragraph states one of those (*Paragraph
    /// Formatting*, page 1190); the reference's values do not include them, and answering what the
    /// markup states keeps a script's round trip from losing it. The paragraph's, on every span
    /// of it.
    pub alignment: String,
    /// `fontFamily`: the search path nearest first, chapter 27's `font-family` with CSS2's generic
    /// names read as the standard 14 faces this tree draws them in (ADR 1634 section 2). Empty
    /// where the markup names none, which leaves the field's `/DA` face.
    pub font_family: Vec<String>,
    /// `fontStretch`: one of chapter 27's nine widths (*Font*, page 1201).
    pub font_stretch: String,
    /// `fontStyle`: `italic` or `normal`.
    pub font_style: String,
    /// `fontWeight`: 100 to 900 in hundreds.
    pub font_weight: u16,
    /// `strikethrough`: `text-decoration`'s `line-through`.
    pub strikethrough: bool,
    /// `subscript`: a baseline lowered by `vertical-align` (*Baseline Adjustment*, page 1199).
    pub subscript: bool,
    /// `superscript`: a baseline raised by it.
    pub superscript: bool,
    /// `text`: the characters, white space already resolved as chapter 27 resolves it.
    pub text: String,
    /// `textColor` as sRGB components from 0 to 1; `None` where the markup states no colour,
    /// which leaves the `/DA`'s own. The reference's default is black.
    pub text_color: Option<[f32; 3]>,
    /// `textSize` in points, an `em` measured against [`DEFAULT_SIZE`] where the markup states no
    /// absolute size above it (the field's `/DA` size is the caller's to supply instead).
    pub text_size: f32,
    /// `underline`: any of chapter 27's four underlines (*Underline and Strikethrough*, page 1208).
    pub underline: bool,
}

impl Default for Span {
    /// The reference's defaults: left-aligned, normal width, style and weight, nothing decorated,
    /// 12 points, no colour of its own.
    fn default() -> Self {
        Self {
            alignment: "left".to_owned(),
            font_family: Vec::new(),
            font_stretch: "normal".to_owned(),
            font_style: "normal".to_owned(),
            font_weight: 400,
            strikethrough: false,
            subscript: false,
            superscript: false,
            text: String::new(),
            text_color: None,
            text_size: DEFAULT_SIZE,
            underline: false,
        }
    }
}

/// A rich text string as spans, one per styled run, under a default style string and with
/// relative sizes measured against `root_size`.
///
/// `None` where the markup is not a rich text string [`super::markup`] reads (not well formed,
/// no element, past its length bound). A tab chapter 27 states as `xfa-tab-count` is that many
/// `\t` characters; a paragraph holding no text is an empty span carrying its `\r`.
#[must_use]
pub fn spans(markup: &str, default_style: Option<&str>, root_size: f32) -> Option<Vec<Span>> {
    let rich = markup::parse(markup, default_style, Character::root())?;
    let mut out: Vec<Span> = Vec::new();
    let count = rich.paragraphs.len();
    for (index, paragraph) in rich.paragraphs.iter().enumerate() {
        let alignment = alignment_name(paragraph.block.align);
        let first = out.len();
        for piece in &paragraph.pieces {
            match piece {
                Piece::Text(text, character) => {
                    out.push(span_of(text, character, alignment, root_size));
                }
                Piece::Break => {
                    if out.len() == first {
                        out.push(span_of("", &paragraph.strut, alignment, root_size));
                    }
                    if let Some(last) = out.last_mut() {
                        last.text.push('\r');
                    }
                }
                Piece::Tab(count) => {
                    if out.len() == first {
                        out.push(span_of("", &paragraph.strut, alignment, root_size));
                    }
                    if let Some(last) = out.last_mut() {
                        last.text
                            .extend(std::iter::repeat_n('\t', usize::from(*count)));
                    }
                }
            }
        }
        let ends_in_break = matches!(paragraph.pieces.last(), Some(Piece::Break));
        if index.saturating_add(1) < count && !ends_in_break {
            if out.len() == first {
                out.push(span_of("", &paragraph.strut, alignment, root_size));
            }
            if let Some(last) = out.last_mut() {
                last.text.push('\r');
            }
        }
    }
    Some(out)
}

/// One run's span.
fn span_of(text: &str, character: &Character, alignment: &str, root_size: f32) -> Span {
    // A rise is measured in root sizes and points, as every length the style computes is.
    let rise = character.rise.at(root_size);
    Span {
        alignment: alignment.to_owned(),
        font_family: character.families.clone(),
        font_stretch: STRETCHES
            .get(usize::from(character.stretch))
            .or_else(|| STRETCHES.get(usize::from(NORMAL_STRETCH)))
            .map_or_else(|| "normal".to_owned(), |name| (*name).to_owned()),
        font_style: if character.italic { "italic" } else { "normal" }.to_owned(),
        font_weight: character.weight,
        strikethrough: character.line_through,
        subscript: rise < 0.0,
        superscript: rise > 0.0,
        text: text.to_owned(),
        text_color: character.colour,
        text_size: character.size.at(root_size),
        underline: character.underline != Underline::None,
    }
}

/// A paragraph's alignment as the span spells it.
fn alignment_name(align: Option<Align>) -> &'static str {
    match align {
        None | Some(Align::Left) => "left",
        Some(Align::Centre) => "center",
        Some(Align::Right) => "right",
        Some(Align::Justify) => "justify",
        Some(Align::JustifyAll) => "justify-all",
    }
}

/// Spans written as a rich text string: the XHTML `body` chapter 27 names, one `p` per paragraph
/// and one `span` per run, each stating every property the span carries.
///
/// The `body` names XHTML's namespace and XFA's and `xfa:spec` the version of chapter 27 written
/// to (*Version Specification*, page 1222), as [`super::written`] does. A paragraph's alignment
/// is its first span's that holds characters of it, the reference's rule ("Span properties" —
/// alignment) — the empty remainder of a span whose text ended the paragraph before is not on the
/// line; a value chapter 27
/// does not name is written as `left`. Every span is written with `xfa-spacerun:yes` (page 1220)
/// so that its characters come back as the script left them — chapter 27 otherwise compresses a
/// run of spaces to one. A superscript or subscript is a baseline moved by a third of the size,
/// a documented choice: the reference states no distance, and [`spans`] reads only the sign.
#[must_use]
pub fn markup(spans: &[Span]) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\"?><body xmlns=\"http://www.w3.org/1999/xhtml\" \
         xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\" xfa:spec=\"3.3\">",
    );
    // Each paragraph as the runs it holds: a span's text split at its carriage returns.
    let mut paragraphs: Vec<Vec<(&Span, &str)>> = vec![Vec::new()];
    for span in spans {
        let mut pieces = span.text.split('\r').peekable();
        while let Some(piece) = pieces.next() {
            if let Some(current) = paragraphs.last_mut() {
                current.push((span, piece));
            }
            if pieces.peek().is_some() {
                paragraphs.push(Vec::new());
            }
        }
    }
    // A text ending in a carriage return ends its paragraph and opens none after it.
    if paragraphs.len() > 1
        && paragraphs
            .last()
            .is_some_and(|last| last.iter().all(|(_, text)| text.is_empty()))
    {
        paragraphs.pop();
    }
    for runs in &paragraphs {
        let first = runs
            .iter()
            .find(|(_, text)| !text.is_empty())
            .or_else(|| runs.first());
        let alignment = first.map_or("left", |(span, _)| match span.alignment.as_str() {
            name @ ("left" | "center" | "right" | "justify" | "justify-all") => name,
            _ => "left",
        });
        let _ = write!(out, "<p style=\"text-align:{alignment}\">");
        let mut wrote = false;
        for (span, text) in runs.iter().filter(|(_, text)| !text.is_empty()) {
            out.push_str("<span style=\"");
            escape(&style_of(span), true, &mut out);
            out.push_str("\">");
            escape(text, false, &mut out);
            out.push_str("</span>");
            wrote = true;
        }
        if !wrote {
            out.push_str("<br/>");
        }
        out.push_str("</p>");
    }
    out.push_str("</body>");
    out
}

/// One span's style attribute, every property it carries.
fn style_of(span: &Span) -> String {
    let mut style = String::new();
    let families: Vec<String> = span
        .font_family
        .iter()
        .map(|family| family.replace(['\'', '"', ';'], ""))
        .filter(|family| !family.is_empty())
        .map(|family| format!("'{family}'"))
        .collect();
    if !families.is_empty() {
        let _ = write!(style, "font-family:{};", families.join(","));
    }
    let size = if span.text_size.is_finite() && span.text_size >= 0.0 {
        span.text_size
    } else {
        DEFAULT_SIZE
    };
    let _ = write!(style, "font-size:{size}pt;");
    let weight = if (100..=900).contains(&span.font_weight) {
        (span.font_weight / 100).saturating_mul(100)
    } else {
        400
    };
    let _ = write!(style, "font-weight:{weight};");
    let italic = span.font_style.eq_ignore_ascii_case("italic")
        || span.font_style.eq_ignore_ascii_case("oblique");
    let _ = write!(
        style,
        "font-style:{};",
        if italic { "italic" } else { "normal" }
    );
    let stretch = span.font_stretch.to_ascii_lowercase();
    if STRETCHES.contains(&stretch.as_str()) {
        let _ = write!(style, "font-stretch:{stretch};");
    }
    if let Some(colour) = span.text_color {
        let byte = |component: f32| {
            // A component from 0 to 1 as its nearest of the 256 steps `#rrggbb` writes; out of
            // range is clamped, and NaN is 0, because `as` saturates.
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped to 0..=255 first, and `as` saturates"
            )]
            let byte = (component.clamp(0.0, 1.0) * 255.0).round() as u8;
            byte
        };
        let _ = write!(
            style,
            "color:#{:02x}{:02x}{:02x};",
            byte(colour[0]),
            byte(colour[1]),
            byte(colour[2])
        );
    }
    let decorations: Vec<&str> = [
        (span.underline, "underline"),
        (span.strikethrough, "line-through"),
    ]
    .into_iter()
    .filter_map(|(on, name)| on.then_some(name))
    .collect();
    if !decorations.is_empty() {
        let _ = write!(style, "text-decoration:{};", decorations.join(" "));
    }
    if span.superscript {
        style.push_str("vertical-align:33%;");
    } else if span.subscript {
        style.push_str("vertical-align:-33%;");
    }
    style.push_str("xfa-spacerun:yes");
    style
}

/// XML's five predefined entities where they are needed: `&`, `<` and `>` always, the quotation
/// mark inside an attribute.
fn escape(text: &str, attribute: bool, out: &mut String) {
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paragraph_of_styled_runs_reads_as_one_span_each() {
        let read = spans(
            "<body xmlns=\"http://www.w3.org/1999/xhtml\"><p style=\"text-align:center\">\
             plain <b>bold</b> <span style=\"color:#ff0000;font-size:10pt\">red</span></p>\
             <p>second</p></body>",
            None,
            DEFAULT_SIZE,
        )
        .expect("a rich text string");
        let texts: Vec<&str> = read.iter().map(|span| span.text.as_str()).collect();
        assert_eq!(
            texts,
            ["plain ", "bold", " ", "red\r", "second"],
            "{read:?}"
        );
        assert_eq!(read[1].font_weight, 700);
        assert_eq!(read[3].text_color, Some([1.0, 0.0, 0.0]));
        assert!((read[3].text_size - 10.0).abs() < f32::EPSILON);
        assert_eq!(read[0].alignment, "center");
        assert_eq!(read[4].alignment, "left");
    }

    #[test]
    fn spans_written_read_back_as_the_same_spans() {
        let written = vec![
            Span {
                alignment: "right".to_owned(),
                text: "The answer is x".to_owned(),
                ..Span::default()
            },
            Span {
                alignment: "right".to_owned(),
                text: "2/3".to_owned(),
                superscript: true,
                ..Span::default()
            },
            Span {
                alignment: "right".to_owned(),
                text: ".  <&>\r".to_owned(),
                font_family: vec!["Times".to_owned(), "Courier".to_owned()],
                ..Span::default()
            },
            Span {
                text: "Did you get it right?".to_owned(),
                underline: true,
                strikethrough: true,
                font_style: "italic".to_owned(),
                font_weight: 900,
                font_stretch: "condensed".to_owned(),
                text_color: Some([1.0, 0.0, 0.0]),
                text_size: 9.5,
                subscript: true,
                ..Span::default()
            },
        ];
        let text = markup(&written);
        let read = spans(&text, None, DEFAULT_SIZE).expect("its own markup reads");
        assert_eq!(read, written, "{text}");
    }

    #[test]
    fn an_empty_paragraph_is_an_empty_span_with_its_return() {
        let written = vec![
            Span {
                text: "one\r\r".to_owned(),
                ..Span::default()
            },
            Span {
                text: "three".to_owned(),
                ..Span::default()
            },
        ];
        let read = spans(&markup(&written), None, DEFAULT_SIZE).expect("reads");
        let texts: Vec<&str> = read.iter().map(|span| span.text.as_str()).collect();
        assert_eq!(texts, ["one\r", "\r", "three"]);
    }

    #[test]
    fn text_that_opens_no_element_reads_as_none() {
        assert_eq!(spans("plain characters", None, DEFAULT_SIZE), None);
    }
}
