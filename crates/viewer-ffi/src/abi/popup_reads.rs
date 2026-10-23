//! A page's popups, rich paragraphs and tab stops, handed over as runs and fields.
//!
//! The pointer contract and the one `unsafe` position are stated in [`super`] (ADR 2272).

use super::layouts::{
    PdfvRichLeader, PdfvRichParagraph, PdfvRichRun, PdfvRichTab, PdfvRichTabs,
    QUORRA_RICH_ALIGN_CENTRE, QUORRA_RICH_ALIGN_JUSTIFY, QUORRA_RICH_ALIGN_LEFT,
    QUORRA_RICH_ALIGN_NONE, QUORRA_RICH_ALIGN_RIGHT, QUORRA_RICH_LEADER_CONTENT,
    QUORRA_RICH_LEADER_DOTS, QUORRA_RICH_LEADER_NONE, QUORRA_RICH_LEADER_RULE,
    QUORRA_RICH_RULE_DASHED, QUORRA_RICH_RULE_DOTTED, QUORRA_RICH_RULE_SOLID,
    QUORRA_RICH_TAB_AFTER, QUORRA_RICH_TAB_BEFORE, QUORRA_RICH_TAB_CENTRE, QUORRA_RICH_TAB_DECIMAL,
    QUORRA_RICH_TAB_LEFT, QUORRA_RICH_TAB_RIGHT,
};
use super::shared::{copy_floats, copy_out};
use crate::answers::Popups;
use crate::kinds::NoteKind;
use crate::session::Session;
use crate::status::Status;
use core::ffi::{c_char, c_int};

/// §12.5.6.14's open popup windows on the page being shown, as a handle the caller frees.
///
/// The clause makes a popup "a window … for entry and editing" with "no appearance stream", so it
/// is the one annotation subtype whose picture is *not* the page's: a caller draws it as chrome,
/// in its platform's own window furniture. Only the open ones — Table 186's `/Open` says which
/// start that way and `quorra_activate` on the parent annotation changes it.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popups_read(
    viewer: *const Session,
    popups: *mut *mut Popups,
) -> c_int {
    let (Some(viewer), Some(out)) = (viewer.as_ref(), popups.as_mut()) else {
        return Status::NullArgument.code();
    };
    match viewer.reads().popups() {
        Ok(windows) => {
            *out = Box::into_raw(Box::new(windows));
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases what `quorra_popups_read` produced. Null is a no-op.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popups_free(popups: *mut Popups) {
    if !popups.is_null() {
        drop(Box::from_raw(popups));
    }
}

/// How many windows are open. Zero for a null pointer.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popups_len(popups: *const Popups) -> usize {
    popups.as_ref().map_or(0, Popups::len)
}

/// The popup annotation, and Table 186's `/Parent` where it names one.
///
/// Two objects because they answer two questions: the first is what `quorra_activate` closes the
/// window with, and the second is the markup annotation the note belongs to — which is what a
/// caller highlights when the pointer is over the window. `has_parent` is false for a popup the
/// file left unattached, which Table 186 permits.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_object(
    popups: *const Popups,
    index: usize,
    number: *mut u32,
    generation: *mut u16,
    has_parent: *mut bool,
    parent_number: *mut u32,
    parent_generation: *mut u16,
) -> c_int {
    let (Some(popups), Some(number), Some(generation)) =
        (popups.as_ref(), number.as_mut(), generation.as_mut())
    else {
        return Status::NullArgument.code();
    };
    let (annotation, parent) = match popups.objects(index) {
        Ok(objects) => objects,
        Err(status) => return status.code(),
    };
    *number = annotation.0;
    *generation = annotation.1;
    if let Some(has_parent) = has_parent.as_mut() {
        *has_parent = parent.is_some();
    }
    let (parent_id, parent_generation_number) = parent.unwrap_or((0, 0));
    if let Some(parent_number) = parent_number.as_mut() {
        *parent_number = parent_id;
    }
    if let Some(parent_generation) = parent_generation.as_mut() {
        *parent_generation = parent_generation_number;
    }
    Status::Ok.code()
}

/// The text note a person retypes this window's text into, which [`quorra_set_note_text`](super::quorra_set_note_text) names:
/// §12.5.6.14's popup "shall be used for editing the parent's text" (ADR 1726).
/// [`Status::NoAnswer`] for a window whose text this program does not retype.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_note(
    popups: *const Popups,
    index: usize,
    number: *mut u32,
    generation: *mut u16,
) -> c_int {
    let (Some(popups), Some(number), Some(generation)) =
        (popups.as_ref(), number.as_mut(), generation.as_mut())
    else {
        return Status::NullArgument.code();
    };
    match popups.note(index) {
        Ok(note) => {
            *number = note.0;
            *generation = note.1;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// The window's rectangle on the screen: `[x0, y0, … x3, y3]`, y downwards, eight floats.
///
/// The same form `quorra_quads_get`, `quorra_focused_annotation` and `quorra_field_widget` take, in
/// device pixels of the viewport — one arithmetic in one place.
///
/// # Safety
///
/// See the module documentation. `into` is writable for eight floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_quad(
    popups: *const Popups,
    index: usize,
    into: *mut f32,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    if into.is_null() {
        return Status::NullArgument.code();
    }
    match popups.quad(index) {
        Ok(quad) => {
            core::slice::from_raw_parts_mut(into, quad.len()).copy_from_slice(&quad);
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One of the window's three strings: `QUORRA_NOTE_TITLE`, `QUORRA_NOTE_CONTENTS`, `QUORRA_NOTE_MODIFIED`.
///
/// An empty string for one the annotation does not state, because Table 166 makes none of the
/// three required and a note with no title is a note a caller draws with no title.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_text(
    popups: *const Popups,
    index: usize,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = NoteKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match popups.text(index, which) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// Table 166's `/C`, "\[t]he title bar of the annotation's popup window", as three `DeviceRGB`
/// components.
///
/// `QUORRA_NO_ANSWER` where the annotation states no colour, which is a different thing from black.
///
/// # Safety
///
/// See the module documentation. `into` is writable for three floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_colour(
    popups: *const Popups,
    index: usize,
    into: *mut f32,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    if into.is_null() {
        return Status::NullArgument.code();
    }
    match popups.colour(index) {
        Ok(colour) => {
            core::slice::from_raw_parts_mut(into, colour.len()).copy_from_slice(&colour);
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// How many of §12.5.6.2's replies a window shows under its own text.
///
/// Table 172 makes showing them there a `shall`: replies are not displayed "individually but
/// together in the form of threaded comments". A caller that draws `quorra_popup_text` and stops
/// has left a reviewed document's replies off the screen — they are no longer windows of their
/// own, so this is where they are.
///
/// # Safety
///
/// See the module documentation. `into` is writable for one `size_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_reply_count(
    popups: *const Popups,
    index: usize,
    into: *mut usize,
) -> c_int {
    let (Some(popups), Some(into)) = (popups.as_ref(), into.as_mut()) else {
        return Status::NullArgument.code();
    };
    match popups.reply_count(index) {
        Ok(count) => {
            *into = count;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One reply's annotation and how deep in the thread it sits.
///
/// The object is what `quorra_activate` names; the depth is 1 for a reply to the window's own
/// note, 2 for a reply to that reply, and is what *threaded* means when a caller indents them.
///
/// # Safety
///
/// See the module documentation. Each out-parameter is writable or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_reply_object(
    popups: *const Popups,
    index: usize,
    reply: usize,
    number: *mut u32,
    generation: *mut u16,
    depth: *mut usize,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    let (annotation, at) = match popups.reply(index, reply) {
        Ok(answer) => answer,
        Err(status) => return status.code(),
    };
    if let Some(number) = number.as_mut() {
        *number = annotation.0;
    }
    if let Some(generation) = generation.as_mut() {
        *generation = annotation.1;
    }
    if let Some(depth) = depth.as_mut() {
        *depth = at;
    }
    Status::Ok.code()
}

/// One of a reply's three strings: `QUORRA_NOTE_TITLE`, `QUORRA_NOTE_CONTENTS`, `QUORRA_NOTE_MODIFIED`.
///
/// `quorra_popup_text`'s rule, on the same three entries: an empty string for one the annotation
/// does not state.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_reply_text(
    popups: *const Popups,
    index: usize,
    reply: usize,
    which: u32,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    let Some(which) = NoteKind::from_code(which) else {
        return Status::WrongKind.code();
    };
    match popups.reply_text(index, reply, which) {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// Whether note `note` of a window states Table 172's `/RC` this program reads, and its shape.
///
/// `note` is `0` for the window's own and `reply + 1` for one of §12.5.6.2's replies, so a caller
/// walks `0..=reply_count`. **Runs, not the XHTML** (ADR 1655): the string is the document's, and a
/// caller handed it would parse chapter 27 again with its own cascade and its own escaping — the
/// reading every other window of this program takes is what crosses. `QUORRA_NO_ANSWER` is a note
/// a caller draws from `quorra_popup_text` alone. `unapplied` is how many of the note's properties
/// this program does not carry out, each a phrase `quorra_popup_rich_unapplied` reads and the
/// caller says under the note.
///
/// # Safety
///
/// See the module documentation. Each out-parameter is writable or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_rich(
    popups: *const Popups,
    index: usize,
    note: usize,
    paragraphs: *mut usize,
    unapplied: *mut usize,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    match popups.rich(index, note) {
        Ok(rich) => {
            if let Some(paragraphs) = paragraphs.as_mut() {
                *paragraphs = rich.paragraphs.len();
            }
            if let Some(unapplied) = unapplied.as_mut() {
                *unapplied = rich.unapplied.len();
            }
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One paragraph of a rich note: its alignment, its list level, and how many pieces it holds.
///
/// # Safety
///
/// See the module documentation. `into` is writable for one `quorra_rich_paragraph`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_rich_paragraph(
    popups: *const Popups,
    index: usize,
    note: usize,
    paragraph: usize,
    into: *mut PdfvRichParagraph,
) -> c_int {
    let (Some(popups), Some(into)) = (popups.as_ref(), into.as_mut()) else {
        return Status::NullArgument.code();
    };
    match popups.rich_paragraph(index, note, paragraph) {
        Ok(read) => {
            *into = PdfvRichParagraph {
                align: match read.align {
                    None => QUORRA_RICH_ALIGN_NONE,
                    Some(pdf_model::popup::RichAlign::Left) => QUORRA_RICH_ALIGN_LEFT,
                    Some(pdf_model::popup::RichAlign::Centre) => QUORRA_RICH_ALIGN_CENTRE,
                    Some(pdf_model::popup::RichAlign::Right) => QUORRA_RICH_ALIGN_RIGHT,
                    Some(pdf_model::popup::RichAlign::Justify) => QUORRA_RICH_ALIGN_JUSTIFY,
                },
                level: read.level,
                has_tag: read.tag.is_some(),
                pieces: read
                    .runs
                    .len()
                    .saturating_add(usize::from(read.tag.is_some())),
            };
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One piece of a paragraph — its tag first where it has one, then its runs — as a struct.
///
/// # Safety
///
/// See the module documentation. `into` is writable for one `quorra_rich_run`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_rich_run(
    popups: *const Popups,
    index: usize,
    note: usize,
    paragraph: usize,
    piece: usize,
    into: *mut PdfvRichRun,
) -> c_int {
    let (Some(popups), Some(into)) = (popups.as_ref(), into.as_mut()) else {
        return Status::NullArgument.code();
    };
    let tag = popups
        .rich_paragraph(index, note, paragraph)
        .is_ok_and(|read| read.tag.is_some() && piece == 0);
    match popups.rich_run(index, note, (paragraph, piece)) {
        Ok(run) => {
            let (spacing_per_base, spacing_points, spacing_of_space) = match run.letter_spacing {
                pdf_model::popup::RichSpacing::Length(length) => {
                    (length.per_base, length.points, 0.0)
                }
                pdf_model::popup::RichSpacing::OfSpace(share) => (0.0, 0.0, share),
            };
            *into = PdfvRichRun {
                tag,
                bold: run.bold,
                italic: run.italic,
                has_colour: run.colour.is_some(),
                colour: run
                    .colour
                    .map_or([0.0; 3], |colour| [colour.r, colour.g, colour.b]),
                size_per_base: run.size.per_base,
                size_points: run.size.points,
                rise_per_base: run.rise.per_base,
                rise_points: run.rise.points,
                spacing_per_base,
                spacing_points,
                spacing_of_space,
                horizontal_scale: run.horizontal_scale,
                vertical_scale: run.vertical_scale,
                underlines: run.underlines,
                underline_by_word: run.underline_by_word,
                line_through: run.line_through,
                families: run.families.len(),
            };
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// A piece's characters, two-call idiom; a `'\n'` is XHTML's `br`, where the line ends.
///
/// **The document's characters, never markup**: a caller handing them to a toolkit that reads
/// markup escapes them first, which is what `viewer_host::popup::html` does for Qt.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_rich_text(
    popups: *const Popups,
    index: usize,
    note: usize,
    paragraph: usize,
    piece: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    match popups.rich_run(index, note, (paragraph, piece)) {
        Ok(run) => copy_out(&run.text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// One name of a piece's `font-family` search path, nearest first, two-call idiom.
///
/// The document's own spelling — CSS2's generic families arrive as the face §9.6.2.2 names for
/// each — and a caller matches it against its platform's faces itself, walking the path to the
/// first it has. **A name is not markup either**: it may hold a quotation mark.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_rich_family(
    popups: *const Popups,
    index: usize,
    note: usize,
    paragraph: usize,
    piece: usize,
    family: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    match popups
        .rich_run(index, note, (paragraph, piece))
        .and_then(|run| run.families.get(family).ok_or(Status::OutOfRange))
    {
        Ok(name) => copy_out(name, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// A paragraph's tab stops: how many it states and its `tab-interval` (ADR 1667).
///
/// ISO 32000-2 §12.7.4.3 brings chapter 27 of XFA 3.3 in for a rich text string's formatting, and
/// its *Tab Stops* (pages 1205 to 1207) is what a `'\t'` in a run's characters advances to: the
/// next stated stop past the cursor, then a default one every interval beyond the last stated.
///
/// # Safety
///
/// See the module documentation. `into` is writable for one `quorra_rich_tabs`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_rich_tabs(
    popups: *const Popups,
    index: usize,
    note: usize,
    paragraph: usize,
    into: *mut PdfvRichTabs,
) -> c_int {
    let (Some(popups), Some(into)) = (popups.as_ref(), into.as_mut()) else {
        return Status::NullArgument.code();
    };
    match popups.rich_paragraph(index, note, paragraph) {
        Ok(read) => {
            let interval = read.tab_interval.unwrap_or_default();
            *into = PdfvRichTabs {
                stops: read.tab_stops.len(),
                has_interval: read.tab_interval.is_some(),
                interval_per_base: interval.per_base,
                interval_points: interval.points,
            };
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One of a paragraph's stated tab stops, in the order it states them (ADR 1667).
///
/// # Safety
///
/// See the module documentation. `into` is writable for one `quorra_rich_tab`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_rich_tab(
    popups: *const Popups,
    index: usize,
    note: usize,
    paragraph: usize,
    stop: usize,
    into: *mut PdfvRichTab,
) -> c_int {
    let (Some(popups), Some(into)) = (popups.as_ref(), into.as_mut()) else {
        return Status::NullArgument.code();
    };
    match popups
        .rich_paragraph(index, note, paragraph)
        .and_then(|read| read.tab_stops.get(stop).ok_or(Status::OutOfRange))
    {
        Ok(read) => {
            *into = PdfvRichTab {
                align: match read.align {
                    pdf_model::popup::RichTabAlign::Left => QUORRA_RICH_TAB_LEFT,
                    pdf_model::popup::RichTabAlign::Centre => QUORRA_RICH_TAB_CENTRE,
                    pdf_model::popup::RichTabAlign::Right => QUORRA_RICH_TAB_RIGHT,
                    pdf_model::popup::RichTabAlign::Decimal => QUORRA_RICH_TAB_DECIMAL,
                    pdf_model::popup::RichTabAlign::After => QUORRA_RICH_TAB_AFTER,
                    pdf_model::popup::RichTabAlign::Before => QUORRA_RICH_TAB_BEFORE,
                },
                at_per_base: read.at.per_base,
                at_points: read.at.points,
            };
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// One stated tab stop's leader, which fills the room a tab advanced before the text at the stop
/// (ADR 1726).
///
/// Chapter 2's *Tab Leader Pattern* (pages 63 to 65), which ISO 32000-2 §12.7.4.3 brings in with
/// chapter 27. A caller paints it over its own line from the tab's extent there, as the two
/// toolkit windows do (ADR 1722), on [`quorra_leader_cycles`]' grid; a rule's pieces are
/// [`quorra_rule_pieces`]'.
///
/// # Safety
///
/// See the module documentation. `into` is writable for one `quorra_rich_leader`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_rich_leader(
    popups: *const Popups,
    index: usize,
    note: usize,
    paragraph: usize,
    stop: usize,
    into: *mut PdfvRichLeader,
) -> c_int {
    let (Some(popups), Some(into)) = (popups.as_ref(), into.as_mut()) else {
        return Status::NullArgument.code();
    };
    match popups.rich_stop(index, note, (paragraph, stop)) {
        Ok(read) => {
            *into = abi_leader(read.leader.as_ref());
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// A `use-content` leader's characters, two-call idiom; [`Status::NoAnswer`] for any other
/// pattern (ADR 1726). They are the document's and never markup.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_rich_leader_text(
    popups: *const Popups,
    index: usize,
    note: usize,
    paragraph: usize,
    stop: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    match popups
        .rich_stop(index, note, (paragraph, stop))
        .and_then(|read| leader_content(read.leader.as_ref()))
    {
        Ok(text) => copy_out(text, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// Where each whole repetition of a leader starts across the room a tab advanced, `from` to
/// `to`, on a grid from the paragraph's left margin `margin` so that leaders on different lines
/// line up — the grid all three windows draw by (`viewer_host::popup::leader_cycles`, ADR 1722).
/// `cycle` is the larger of the leader's `leaderPatternWidth` and the pattern's own width, which
/// only the caller that set the pattern can measure. Two-call idiom over `float`s: `needed` is
/// how many starts there are, and `out` takes them when `cap` holds that many (ADR 1726).
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` `float`s, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_leader_cycles(
    from: f32,
    to: f32,
    margin: f32,
    cycle: f32,
    out: *mut f32,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    copy_floats(
        &viewer_host::popup::leader_cycles(from, to, margin, cycle),
        out,
        cap,
        needed,
    )
}

/// A rule leader across `from` to `to`, as `(left, width)` pairs written one after the other: one
/// for a solid rule, a dash of two thicknesses every four for a dashed one and a square dot every
/// two for a dotted one, on [`quorra_leader_cycles`]' grid from `margin`
/// (`viewer_host::popup::rule_pieces`, ADR 1726). `style` is `QUORRA_RICH_RULE_*`, and an unknown
/// one is [`Status::OutOfRange`]. Two-call idiom over `float`s: `needed` is twice the pieces.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` `float`s, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_rule_pieces(
    style: u32,
    thickness: f32,
    from: f32,
    to: f32,
    margin: f32,
    out: *mut f32,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    match rule_pieces(style, thickness, (from, to), margin) {
        Ok(pieces) => copy_floats(&pieces, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// One of the note's properties this program does not carry out, as the phrase a caller says
/// under the note, two-call idiom — a property dropped is never a property silently ignored (ADR
/// 1642). What the caller's own toolkit cannot set besides is the caller's to add.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quorra_popup_rich_unapplied(
    popups: *const Popups,
    index: usize,
    note: usize,
    which: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(popups) = popups.as_ref() else {
        return Status::NullArgument.code();
    };
    match popups
        .rich(index, note)
        .and_then(|rich| rich.unapplied.get(which).ok_or(Status::OutOfRange))
    {
        Ok(phrase) => copy_out(phrase, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// A stop's leader in the shape [`quorra_popup_rich_leader`] writes, `QUORRA_RICH_LEADER_NONE` for
/// a stop that states none.
fn abi_leader(leader: Option<&pdf_model::popup::RichLeader>) -> PdfvRichLeader {
    use pdf_model::popup::{RichLeaderPattern, RichRuleStyle};
    let mut out = PdfvRichLeader {
        pattern: QUORRA_RICH_LEADER_NONE,
        rule_style: QUORRA_RICH_RULE_SOLID,
        has_thickness: false,
        thickness_per_base: 0.0,
        thickness_points: 0.0,
        has_width: false,
        width_per_base: 0.0,
        width_points: 0.0,
    };
    let Some(leader) = leader else {
        return out;
    };
    out.pattern = match &leader.pattern {
        RichLeaderPattern::Dots => QUORRA_RICH_LEADER_DOTS,
        RichLeaderPattern::Rule { style, thickness } => {
            out.rule_style = match style {
                RichRuleStyle::Solid => QUORRA_RICH_RULE_SOLID,
                RichRuleStyle::Dashed => QUORRA_RICH_RULE_DASHED,
                RichRuleStyle::Dotted => QUORRA_RICH_RULE_DOTTED,
            };
            if let Some(thickness) = thickness {
                out.has_thickness = true;
                out.thickness_per_base = thickness.per_base;
                out.thickness_points = thickness.points;
            }
            QUORRA_RICH_LEADER_RULE
        }
        RichLeaderPattern::Content(_) => QUORRA_RICH_LEADER_CONTENT,
    };
    if let Some(width) = leader.width {
        out.has_width = true;
        out.width_per_base = width.per_base;
        out.width_points = width.points;
    }
    out
}

/// A `use-content` leader's characters, and [`Status::NoAnswer`] for a stop with any other.
fn leader_content(leader: Option<&pdf_model::popup::RichLeader>) -> Result<&str, Status> {
    match leader.map(|leader| &leader.pattern) {
        Some(pdf_model::popup::RichLeaderPattern::Content(content)) => Ok(content),
        _ => Err(Status::NoAnswer),
    }
}

/// [`quorra_rule_pieces`]' answer, the pairs written one after the other.
fn rule_pieces(
    style: u32,
    thickness: f32,
    (from, to): (f32, f32),
    margin: f32,
) -> Result<Vec<f32>, Status> {
    use pdf_model::popup::RichRuleStyle;
    let style = match style {
        QUORRA_RICH_RULE_SOLID => RichRuleStyle::Solid,
        QUORRA_RICH_RULE_DASHED => RichRuleStyle::Dashed,
        QUORRA_RICH_RULE_DOTTED => RichRuleStyle::Dotted,
        _ => return Err(Status::OutOfRange),
    };
    Ok(
        viewer_host::popup::rule_pieces(style, thickness, (from, to), margin)
            .into_iter()
            .flat_map(|(left, width)| [left, width])
            .collect(),
    )
}
