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

/// What a window that sets a note through a toolkit's label says it did not draw, beside
/// `pdf_model::popup::RichNote::unapplied`: a font scale, which neither Pango's markup nor Qt's
/// rich text states a property for, and a letter spacing given as a share of a space, which only
/// the face the toolkit picks can resolve (ADR 1654). `quorra` draws both and passes nothing.
#[must_use]
pub fn toolkit_unapplied(note: &pdf_model::popup::RichNote) -> Vec<String> {
    let runs = || {
        note.paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.tag.iter().chain(&paragraph.runs))
    };
    let mut said = Vec::new();
    if runs().any(scaled) {
        said.push("a font scale in a popup window".to_owned());
    }
    if runs().any(|run| matches!(run.letter_spacing, pdf_model::popup::RichSpacing::OfSpace(share) if share != 0.0)) {
        said.push("letter-spacing as a share of a space".to_owned());
    }
    said
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
            }],
            unapplied: Vec::new(),
        };
        assert!(super::html(&note, 10.0).contains("letter-spacing:13.33px"));
        assert!(super::toolkit_unapplied(&note).is_empty());
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
            }],
            unapplied: Vec::new(),
        };
        assert_eq!(
            super::toolkit_unapplied(&note(vec![wide])),
            vec!["a font scale in a popup window".to_owned()]
        );
        assert!(super::toolkit_unapplied(&note(vec![run("x")])).is_empty());
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
