//! Table 172's `/RC` as GTK draws it: a paragraph as a wrapped label, a run as a Pango span, and the tab stops and leaders between them (ADRs 1642, 1722).

use std::cell::Cell;

use gtk4::glib;
use gtk4::prelude::*;

use super::POPUP_PADDING;
use super::notes::{NoteText, label_size};

/// Table 172's `/RC` as GTK draws it: one wrapped label per paragraph, each run a Pango `span`
/// carrying what the note states about it (ADR 1642).
///
/// A label per paragraph because alignment is a label's, and chapter 27's `text-align` is a
/// paragraph's. The base size is the label's own font's, which is the size §12.5.6.4 lets the
/// processor choose; `viewer_host::popup` turns each run's relative and absolute sizes into points
/// against it, and says under the note what was not drawn. The labels come back beside the box,
/// each with the paragraph it sets.
pub(super) fn rich_note(note: &pdf_model::popup::RichNote) -> (gtk4::Box, Vec<NoteText>) {
    let body = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    body.set_vexpand(true);
    body.set_margin_start(POPUP_PADDING);
    body.set_margin_end(POPUP_PADDING);
    body.set_margin_top(POPUP_PADDING);
    let probe = gtk4::Label::new(None);
    let base = label_size(&probe);
    let context = probe.pango_context();
    let mut texts = Vec::with_capacity(note.paragraphs.len());
    for (number, paragraph) in note.paragraphs.iter().enumerate() {
        let advances = viewer_host::popup::advances(paragraph);
        let mut markup = String::new();
        // A list tag is one left-to-right label at the paragraph's start edge, outside its order:
        // UAX #9's LRI and PDI isolate it, so a paragraph read right to left has it at the right
        // and reads it as it is written (ADR 1666).
        if let Some(tag) = &paragraph.tag {
            markup.push('\u{2066}');
            markup.push_str(&pango_span(tag, base, &context, true));
            markup.push_str("\u{2069} ");
        }
        for run in &paragraph.runs {
            markup.push_str(&pango_span(run, base, &context, advances));
        }
        let label = gtk4::Label::new(None);
        label.set_markup(&markup);
        texts.push(NoteText {
            label: label.clone(),
            paragraph: Some(number),
        });
        let right_to_left = viewer_host::popup::right_to_left(paragraph);
        let stops = viewer_host::popup::tab_stops(paragraph, base, 1.0, POPUP_TAB_ROOM);
        if !stops.is_empty() && !right_to_left {
            label.set_tabs(Some(&pango_tabs(&stops, false)));
        }
        label.set_wrap(true);
        label.set_wrap_mode(pango::WrapMode::WordChar);
        // A paragraph stating no alignment starts at its own start edge, the right for one that
        // reads right to left, as the other two windows start it (ADR 1654).
        let (xalign, justify) = match paragraph.align {
            None if right_to_left => (1.0, gtk4::Justification::Right),
            None | Some(pdf_model::popup::RichAlign::Left) => (0.0, gtk4::Justification::Left),
            Some(pdf_model::popup::RichAlign::Centre) => (0.5, gtk4::Justification::Center),
            Some(pdf_model::popup::RichAlign::Right) => (1.0, gtk4::Justification::Right),
            Some(pdf_model::popup::RichAlign::Justify) => (0.0, gtk4::Justification::Fill),
        };
        label.set_xalign(xalign);
        label.set_justify(justify);
        // The indent is taken from the paragraph's start edge, the right for one read right to
        // left, where its tag stands.
        let indent = POPUP_PADDING
            .saturating_mul(3)
            .saturating_mul(i32::from(paragraph.level));
        let tabbed = viewer_host::popup::tabbed(paragraph);
        let leaders = (tabbed && stops.iter().any(|stop| stop.leader.is_some())).then(|| Leaders {
            runs: tab_runs(paragraph),
            stops: stops.clone(),
            right_to_left,
            base,
            context: context.clone(),
        });
        if (right_to_left && tabbed && !stops.is_empty()) || leaders.is_some() {
            let measured = measured_label(&label, stops, right_to_left, leaders);
            if right_to_left {
                measured.set_margin_end(indent);
            } else {
                measured.set_margin_start(indent);
            }
            body.append(&measured);
        } else if right_to_left {
            label.set_margin_end(indent);
            body.append(&label);
        } else {
            label.set_margin_start(indent);
            body.append(&label);
        }
    }
    // Pango states no glyph scale per run, so a font scale is said rather than dropped, with what
    // Pango's tab array cannot say (ADRs 1654, 1666).
    let also = viewer_host::popup::toolkit_unapplied(note, false);
    if let Some(sentence) = viewer_host::popup::not_drawn(note, &also) {
        let said = gtk4::Label::new(Some(&sentence));
        said.set_xalign(0.0);
        said.set_wrap(true);
        said.add_css_class("dim-label");
        body.append(&said);
    }
    (body, texts)
}

/// How far, in points, a popup paragraph's default tab stops are placed: past any window a screen
/// shows, so that a line never reaches the end of them.
const POPUP_TAB_ROOM: f32 = 4096.0;

/// A tabbed paragraph's label under a drawing area of its own size, for the two things Pango's
/// tab array cannot do by itself: a right-to-left paragraph's tabs, which reach leftward to stops
/// placed from its left margin (ADR 1690), and the leaders before its stops, which the drawing
/// area paints over the laid-out text ([`paint_leaders`], ADR 1722).
///
/// Pango measures a stop in a line read right to left from the line's start edge, the right,
/// which is not known until GTK allocates the label, so the stops are handed again as distances
/// from that edge whenever the label's width changes (`viewer_host::popup::from_start_edge`).
///
/// **A label states no allocation of its own, and this crate forbids the `unsafe` a subclass
/// would take**, so the width is read from a drawing area laid over the label — the title bar's
/// composition turned round. The label is the overlay's main child, which is allocated exactly the
/// overlay's size, and the drawing area an overlay child that fills it, draws nothing, takes no
/// pointer and reports that size through `resize`; the other way round, `GtkOverlay` places an
/// overlay child at its preferred size, which for a wrapped label is its height at its narrowest,
/// and the note's next line was drawn through the paragraph (measured: a one-line paragraph
/// allocated 35 px of an 18 px overlay). The indent goes on the overlay, so the two widths are one.
///
/// Two more things the handler does are measured rather than assumed. `resize` is emitted on
/// every allocation, the ones a change of the tabs causes among them, so the stops are handed only
/// when the width differs from the last handed. And a label's resize queued from inside an
/// allocation is dropped: the tabs drawn are the new ones, but a paragraph they wrap onto a second
/// line kept one line's height until something else measured it again (measured: two lines laid out
/// in 18 px). So the label is measured again from the main loop, once, after each change.
fn measured_label(
    label: &gtk4::Label,
    stops: Vec<viewer_host::popup::TabStop>,
    right_to_left: bool,
    leaders: Option<Leaders>,
) -> gtk4::Overlay {
    let measured = gtk4::Overlay::new();
    let ruler = gtk4::DrawingArea::new();
    ruler.set_can_target(false);
    measured.set_child(Some(label));
    measured.add_overlay(&ruler);
    if let Some(leaders) = leaders {
        let painted = label.clone();
        ruler.set_draw_func(move |_, cr, _, _| paint_leaders(&painted, &leaders, cr));
    }
    if !right_to_left {
        return measured;
    }
    let handed = Cell::new(None);
    let label = label.clone();
    ruler.connect_resize(move |_, width, _| {
        if handed.replace(Some(width)) != Some(width) {
            // Logical pixels to points, 96 to the inch, as `pango_tabs` takes them back.
            #[expect(
                clippy::cast_precision_loss,
                reason = "a widget's width in pixels is far inside f32's exact integer range"
            )]
            let edge = width as f32 * 72.0 / 96.0;
            let leftward = viewer_host::popup::from_start_edge(&stops, edge);
            label.set_tabs(Some(&pango_tabs(&leftward, true)));
            let again = label.clone();
            glib::idle_add_local_once(move || again.queue_resize());
        }
    });
    measured
}

/// What [`paint_leaders`] needs of one paragraph, kept beside its label.
struct Leaders {
    /// The run each of the paragraph's tabs is set in, in the order its label's text holds them:
    /// a leader's dots and content are that run's glyphs, and a rule its colour.
    runs: Vec<pdf_model::popup::RichRun>,
    /// The paragraph's stops from its left margin, in points, nearest it first.
    stops: Vec<viewer_host::popup::TabStop>,
    /// Whether the paragraph reads right to left, which is the direction a tab reaches in.
    right_to_left: bool,
    /// The label's own size in points, which a run's relative size is of.
    base: f32,
    /// The label's context, which measures a share of a space for a run's letter spacing.
    context: pango::Context,
}

/// The run each of a paragraph's tabs is set in, in reading order: the run whose text holds it.
fn tab_runs(paragraph: &pdf_model::popup::RichParagraph) -> Vec<pdf_model::popup::RichRun> {
    paragraph
        .runs
        .iter()
        .flat_map(|run| std::iter::repeat_n(run, run.text.matches('\t').count()))
        .cloned()
        .collect()
}

/// Paints each tab's leader over the label's laid-out text: chapter 2's *Tab Leader Pattern*
/// (pages 63 to 65), as `quorra` draws it (ADR 1679), on `viewer_host::popup`'s grid (ADR 1722).
///
/// Pango's tab array holds a position, an alignment and a decimal point and no fill, so the
/// leader is composed rather than asked for: the label's own `pango::Layout` gives each tab's
/// extent (`index_to_pos`) and its line's baseline, the stop it reached is the nearest beyond its
/// start in the paragraph's direction (`reached_stop`), and the dots or content are a layout of
/// the tab's run's own span, appended once per whole repetition on the grid from the label's left
/// edge, which is the paragraph's left margin. A rule is a rectangle in the run's colour centred
/// on the baseline. Drawn through a render node because the drawing area hands a cairo context
/// and `gtk4` binds no Pango-on-cairo call; the node draws the layout as GTK's own text nodes do.
fn paint_leaders(label: &gtk4::Label, leaders: &Leaders, cr: &gtk4::cairo::Context) {
    use viewer_host::popup::LeaderPattern;
    // Points to the label's logical pixels, 96 to the inch, as `pango_tabs` hands the stops.
    const PIXELS: f32 = 96.0 / 72.0;
    #[expect(
        clippy::cast_precision_loss,
        reason = "Pango's scale of 1024 is exact in f32"
    )]
    let scale = pango::SCALE as f32;
    #[expect(
        clippy::cast_precision_loss,
        reason = "a position inside a popup window in Pango units is far inside f32's exact \
                  integer range"
    )]
    let pixels = |units: i32| units as f32 / scale;
    let layout = label.layout();
    let (left, top) = label.layout_offsets();
    #[expect(
        clippy::cast_precision_loss,
        reason = "a label's offset in pixels is far inside f32's exact integer range"
    )]
    let (left, top) = (left as f32, top as f32);
    let positions: Vec<f32> = leaders.stops.iter().map(|stop| stop.at * PIXELS).collect();
    let ink = label.color();
    let snapshot = gtk4::Snapshot::new();
    let text = layout.text();
    for (run, (index, _)) in leaders.runs.iter().zip(text.match_indices('\t')) {
        let Ok(index) = i32::try_from(index) else {
            break;
        };
        let extent = layout.index_to_pos(index);
        let (one, other) = (
            pixels(extent.x()),
            pixels(extent.x().saturating_add(extent.width())),
        );
        let (from, to) = (one.min(other), one.max(other));
        let start = if leaders.right_to_left { to } else { from };
        let Some(leader) =
            viewer_host::popup::reached_stop(&positions, start, leaders.right_to_left)
                .and_then(|reached| leaders.stops.get(reached))
                .and_then(|stop| stop.leader.as_ref())
        else {
            continue;
        };
        let (line, _) = layout.index_to_line_x(index, false);
        let mut lines = layout.iter();
        for _ in 0..line {
            if !lines.next_line() {
                break;
            }
        }
        let baseline = top + pixels(lines.baseline());
        let colour = run.colour.map_or(ink, |colour| {
            gtk4::gdk::RGBA::new(colour.r, colour.g, colour.b, 1.0)
        });
        match &leader.pattern {
            LeaderPattern::Dots | LeaderPattern::Content(_) => {
                let mut shown = run.clone();
                shown.text = match &leader.pattern {
                    LeaderPattern::Content(content) => content.clone(),
                    _ => ".".to_owned(),
                };
                shown.underlines = 0;
                shown.line_through = false;
                let piece = label.create_pango_layout(None);
                piece.set_markup(&pango_span(&shown, leaders.base, &leaders.context, true));
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a few glyphs' width in pixels is far inside f32's exact integer range"
                )]
                let inherent = piece.pixel_extents().1.width() as f32;
                let cycle = inherent.max(leader.width * PIXELS);
                let raised = baseline - pixels(piece.baseline());
                for at in viewer_host::popup::leader_cycles(from, to, 0.0, cycle) {
                    snapshot.save();
                    snapshot.translate(&gtk4::graphene::Point::new(left + at, raised));
                    snapshot.append_layout(&piece, &colour);
                    snapshot.restore();
                }
            }
            LeaderPattern::Rule { style, thickness } => {
                let em = viewer_host::popup::size(run, leaders.base, 1.0) * PIXELS;
                let thick = viewer_host::popup::rule_thickness(thickness.map(|at| at * PIXELS), em);
                let rise = viewer_host::popup::rise(run, leaders.base, 1.0) * PIXELS;
                let y = baseline - rise - thick * 0.5;
                for (x, width) in viewer_host::popup::rule_pieces(*style, thick, (from, to), 0.0) {
                    snapshot.append_color(
                        &colour,
                        &gtk4::graphene::Rect::new(left + x, y, width, thick),
                    );
                }
            }
        }
    }
    if let Some(node) = snapshot.to_node() {
        node.draw(cr);
    }
}

/// A paragraph's tab stops as Pango's tab array takes them, in the label's logical pixels — the
/// unit step 53 measured Pango's letter spacing to be in, 96 to the inch (ADRs 1654, 1666).
///
/// `from_right` is for a paragraph read right to left, whose stops are already distances from the
/// line's start edge: Pango names an alignment by that edge, so a stop whose text stands with its
/// left edge at it is `Right` there, and one whose text ends at it `Left` (measured: in a layout
/// 400 px wide read right to left, a `Left` stop at 100 px put the text after the tab from 300 px
/// leftward, a `Right` one from 300 px rightward; ADR 1690).
pub(super) fn pango_tabs(
    stops: &[viewer_host::popup::TabStop],
    from_right: bool,
) -> pango::TabArray {
    let count = i32::try_from(stops.len()).unwrap_or(i32::MAX);
    let mut tabs = pango::TabArray::new(count, true);
    for (index, stop) in (0..count).zip(stops) {
        let align = match (stop.side, from_right) {
            (viewer_host::popup::TabSide::Left, false)
            | (viewer_host::popup::TabSide::Right, true) => pango::TabAlign::Left,
            (viewer_host::popup::TabSide::Centre, _) => pango::TabAlign::Center,
            (viewer_host::popup::TabSide::Right, false)
            | (viewer_host::popup::TabSide::Left, true) => pango::TabAlign::Right,
            (viewer_host::popup::TabSide::Decimal, _) => pango::TabAlign::Decimal,
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a stop is bounded by POPUP_TAB_ROOM, far inside i32's range in pixels"
        )]
        let at = (stop.at * 96.0 / 72.0).round() as i32;
        tabs.set_tab(index, align, at);
        if stop.side == viewer_host::popup::TabSide::Decimal {
            tabs.set_decimal_point(index, '.');
        }
    }
    tabs
}

/// The width of a space in the face Pango picks for `run`, as a share of the run's em: what a
/// letter spacing given as a share of a space is a share of (chapter 27, page 1204), which only
/// the toolkit that chose the face can measure.
fn pango_space(run: &pdf_model::popup::RichRun, size: f32, context: &pango::Context) -> f32 {
    let mut description = context.font_description().unwrap_or_default();
    if let Some(name) = viewer_host::popup::family(run) {
        description.set_family(name);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "a size held within three times a label's own is far inside i32's range in \
                  Pango units, and Pango's scale of 1024 is exact in f32"
    )]
    description.set_size((size * pango::SCALE as f32).round() as i32);
    description.set_weight(if run.bold {
        pango::Weight::Bold
    } else {
        pango::Weight::Normal
    });
    description.set_style(if run.italic {
        pango::Style::Italic
    } else {
        pango::Style::Normal
    });
    let layout = pango::Layout::new(context);
    layout.set_font_description(Some(&description));
    layout.set_text(" ");
    #[expect(
        clippy::cast_precision_loss,
        reason = "a space's width in Pango units is far inside f32's exact integer range"
    )]
    let width = layout.size().0 as f32 / pango::SCALE as f32;
    width / (size * 96.0 / 72.0).max(f32::EPSILON)
}

/// What one rich run is set in, as the numbers Pango takes: the label's markup ([`pango_span`])
/// and the editor's tag ([`run_tag`]) are both written from it, so that a run typed into is drawn
/// as the window drew it (ADR 1782).
pub(super) struct RunFace<'a> {
    /// The size, in Pango units of points.
    size: i32,
    /// Whether the run is bold.
    bold: bool,
    /// Whether it is italic.
    italic: bool,
    /// A family `viewer_host::popup::family` has passed, or `None` for the window's own.
    family: Option<&'a str>,
    /// Chapter 27's `color`.
    colour: Option<pdf_render::Color>,
    /// How many underlines: Pango draws one or two.
    underlines: u8,
    /// Whether a line is drawn through it.
    line_through: bool,
    /// How far it is raised, in Pango units of points, where it is.
    rise: Option<i32>,
    /// Chapter 27's `letter-spacing` in Pango units of a logical pixel, where it has one.
    spacing: Option<i32>,
}

/// [`RunFace`] for one run, of `base` points and measured where its spacing is a share of a space
/// in `context`.
pub(super) fn run_face<'a>(
    run: &'a pdf_model::popup::RichRun,
    base: f32,
    context: &pango::Context,
) -> RunFace<'a> {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "a size held within three times a label's own is far inside i32's range in \
                  Pango units, and Pango's scale of 1024 is exact in f32"
    )]
    let units = |points: f32| (points * pango::SCALE as f32).round() as i32;
    let size = viewer_host::popup::size(run, base, 1.0);
    let raised = viewer_host::popup::rise(run, base, 1.0);
    // Chapter 27's `letter-spacing`. Pango takes it in thousand-and-twenty-fourths of the layout's
    // own unit, a logical pixel, where a size's are of a point: eight points measured eight pixels
    // a gap under the drive's step 53, so the points become CSS2's reference pixels, 96 to the
    // inch, first (ADR 1654). A share of a space is of the face Pango picks, measured there.
    let space = matches!(
        run.letter_spacing,
        pdf_model::popup::RichSpacing::OfSpace(share) if share != 0.0
    )
    .then(|| pango_space(run, size, context));
    RunFace {
        size: units(size),
        bold: run.bold,
        italic: run.italic,
        family: viewer_host::popup::family(run),
        colour: run.colour,
        underlines: run.underlines,
        line_through: run.line_through,
        rise: (raised.abs() > f32::EPSILON).then(|| units(raised)),
        spacing: viewer_host::popup::letter_spacing(run, base, 1.0, space)
            .filter(|spacing| spacing.abs() > f32::EPSILON)
            .map(|spacing| units(spacing * 96.0 / 72.0)),
    }
}

/// One rich run as a Pango markup `span`: the characters escaped, and every attribute one this
/// function writes from the run's [`RunFace`] — a family `viewer_host::popup::family` has passed,
/// so nothing the document wrote reaches the markup as markup. Its tab characters are kept only
/// where `tabs` says the paragraph's tabs advance.
fn pango_span(
    run: &pdf_model::popup::RichRun,
    base: f32,
    context: &pango::Context,
    tabs: bool,
) -> String {
    use std::fmt::Write as _;
    let face = run_face(run, base, context);
    let mut out = String::new();
    let _ = write!(
        out,
        "<span size=\"{}\" weight=\"{}\" style=\"{}\"",
        face.size,
        if face.bold { "bold" } else { "normal" },
        if face.italic { "italic" } else { "normal" },
    );
    if let Some(name) = face.family {
        let _ = write!(out, " font_family=\"{name}\"");
    }
    if let Some(colour) = face.colour {
        let _ = write!(out, " foreground=\"#{}\"", viewer_host::popup::hex(colour));
    }
    match face.underlines {
        0 => {}
        1 => out.push_str(" underline=\"single\""),
        _ => out.push_str(" underline=\"double\""),
    }
    if face.line_through {
        out.push_str(" strikethrough=\"true\"");
    }
    if let Some(rise) = face.rise {
        let _ = write!(out, " rise=\"{rise}\"");
    }
    if let Some(spacing) = face.spacing {
        let _ = write!(out, " letter_spacing=\"{spacing}\"");
    }
    out.push('>');
    let text: std::borrow::Cow<'_, str> = if tabs {
        run.text.as_str().into()
    } else {
        run.text.replace('\t', "").into()
    };
    out.push_str(&glib::markup_escape_text(&text));
    out.push_str("</span>");
    out
}

/// One rich run's [`RunFace`] as a text buffer's tag, which the note's editor sets over the
/// characters of `/Contents` the run is (ADR 1782): the span's attributes, property for property.
pub(super) fn run_tag(face: &RunFace<'_>) -> gtk4::TextTag {
    let tag = gtk4::TextTag::new(None);
    tag.set_size(face.size);
    tag.set_weight(if face.bold { 700 } else { 400 });
    tag.set_style(if face.italic {
        pango::Style::Italic
    } else {
        pango::Style::Normal
    });
    if let Some(name) = face.family {
        tag.set_family(Some(name));
    }
    if let Some(colour) = face.colour {
        tag.set_foreground_rgba(Some(&gtk4::gdk::RGBA::new(
            colour.r, colour.g, colour.b, 1.0,
        )));
    }
    match face.underlines {
        0 => {}
        1 => tag.set_underline(pango::Underline::Single),
        _ => tag.set_underline(pango::Underline::Double),
    }
    if face.line_through {
        tag.set_strikethrough(true);
    }
    if let Some(rise) = face.rise {
        tag.set_rise(rise);
    }
    if let Some(spacing) = face.spacing {
        tag.set_letter_spacing(spacing);
    }
    tag
}
