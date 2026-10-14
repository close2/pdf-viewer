//! §12.5.6.14's popups placed over the page, and a note's editor.

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::{Answer, Command, Edit, Query};

use viewer_host::trace::Topic;

use super::rich_text::{rich_note, run_face, run_tag};
use super::window::logical;
use super::{Host, POPUP_PADDING, with};

impl Host {
    /// §12.5.6.14's open windows, as real widgets over the page.
    ///
    /// **Rebuilt rather than moved, which is the opposite of [`Host::place_fields`]' rule and is
    /// right for the opposite reason.** A control holds the keyboard and a person's half-typed
    /// value, so rebuilding one costs them their place; a popup holds neither — the clause gives
    /// it "no appearance stream or associated actions of its own", and this host makes it untargetable
    /// — so there is nothing in one to lose. A page states as many windows as it has open comments,
    /// which is few, and the comparison below means a scroll that moves nothing rebuilds nothing.
    pub(super) fn place_popups(&mut self, popups: &[viewer_core::PopupWindow]) {
        if self.popups_shown == popups {
            return;
        }
        for placed in self.popups.drain(..) {
            self.ui.popups.remove(&placed.widget);
        }
        let scale = f64::from(self.scale);
        let placed = viewer_host::popup::windows(popups);
        let editing = self.note_editor.as_ref().map(|editor| editor.note);
        for window in &placed {
            let (x, y, width, height) = window.place;
            // The note being typed into keeps its editor where its window is, so that a keystroke
            // that changes the window's text does not take the keyboard away from it; the editor
            // draws the note as the window now would (ADR 1782).
            if let Some(editor) = self
                .note_editor
                .as_mut()
                .filter(|editor| window.note == Some(editor.note))
            {
                editor.styled.style(window.text, window.rich);
                editor.frame.set_size_request(
                    logical(f64::from(width), scale),
                    logical(f64::from(height), scale),
                );
                self.ui
                    .fixed
                    .move_(&editor.frame, f64::from(x) / scale, f64::from(y) / scale);
                continue;
            }
            let (widget, texts) = popup_window(window);
            widget.set_size_request(
                logical(f64::from(width), scale),
                logical(f64::from(height), scale),
            );
            self.ui
                .popups
                .put(&widget, f64::from(x) / scale, f64::from(y) / scale);
            self.popups.push(PlacedPopup {
                widget,
                note: window.note,
                texts,
            });
        }
        // On what the *answer* held rather than on what was placed, so that a window the page
        // states and this host could not put anywhere is a line rather than a silence — which is
        // trap 5 applied to the one report that would otherwise say nothing about a refusal.
        if !popups.is_empty() || !self.popups_shown.is_empty() {
            self.state.trace.say(
                Topic::Panel,
                format_args!(
                    "{} of {} §12.5.6.14 popup window(s) placed",
                    placed.len(),
                    popups.len()
                ),
            );
        }
        self.popups_shown = popups.to_vec();
        // A window that closed or scrolled away takes its editor with it.
        if editing.is_some() && !placed.iter().any(|window| window.note == editing) {
            self.end_note();
        }
    }

    /// Gives a person the note's text to type into, in its window's place (ADR 1726).
    ///
    /// A `GtkTextView` over the window, holding Table 166's `/Contents` as the window shows it, and
    /// every change sent as `Edit::SetNoteText` — the whole text, as a field's value is sent.
    /// Escape, or the window closing, gives the keyboard back to the page. Where the window draws
    /// the note from Table 172's `/RC`, the editor sets each run's face over the characters of
    /// `/Contents` it is, and says under the text what of the paragraphs it does not draw; where a
    /// retyping makes the window plain — its characters no longer `/Contents`' (ADR 1721) — the
    /// editor is plain too, so a person types into what the window shows (ADR 1782).
    ///
    /// The caret starts at `place`, the byte of `/Contents` the press that opened the editor was
    /// on ([`Host::note_place`]), and at the text's end where the press was on no character.
    pub(super) fn edit_note(&mut self, note: pdf_syntax::ObjectId, place: Option<usize>) {
        if self
            .note_editor
            .as_ref()
            .is_some_and(|editor| editor.note == note)
        {
            return;
        }
        self.end_note();
        let Some(window) = self
            .popups_shown
            .iter()
            .find(|window| window.note == Some(note))
        else {
            return;
        };
        let (x, y, width, height) = viewer_host::bounds(window.quad);
        let text = window.text.clone().unwrap_or_default();
        // Before the handler is connected, so that putting the text in sends no edit.
        let (view, styled) = NoteStyle::editor(&text, window.rich.as_ref());
        let (buffer, said) = (styled.buffer.clone(), styled.said.clone());
        // A text buffer counts its offsets in characters, the place is a byte of the same text.
        let caret = place
            .and_then(|byte| text.get(..byte))
            .and_then(|before| i32::try_from(before.chars().count()).ok());
        buffer
            .place_cursor(&caret.map_or_else(|| buffer.end_iter(), |at| buffer.iter_at_offset(at)));
        let me = self.me.clone();
        buffer.connect_changed(move |buffer| {
            let text = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .to_string();
            with(&me, |host| {
                host.dispatch(Command::Edit(Edit::SetNoteText {
                    annotation: note,
                    text,
                }));
            });
        });
        let keys = gtk4::EventControllerKey::new();
        let me = self.me.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key != gtk4::gdk::Key::Escape {
                return glib::Propagation::Proceed;
            }
            let me = me.clone();
            glib::idle_add_local_once(move || with(&me, Host::end_note));
            glib::Propagation::Stop
        });
        view.add_controller(keys);
        let scroller = gtk4::ScrolledWindow::new();
        scroller.set_child(Some(&view));
        scroller.set_vexpand(true);
        let frame = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        frame.append(&scroller);
        frame.append(&said);
        frame.set_overflow(gtk4::Overflow::Hidden);
        let scale = f64::from(self.scale);
        frame.set_size_request(
            logical(f64::from(width), scale),
            logical(f64::from(height), scale),
        );
        // In the controls' layer rather than the windows', which takes no pointer: the editor is a
        // control a person clicks into, as a field's is, and the keyboard there is a control's.
        self.ui
            .fixed
            .put(&frame, f64::from(x) / scale, f64::from(y) / scale);
        // Where the caret starts, as the byte of `/Contents` it stands before, as `quorra` says it.
        let start = place
            .filter(|byte| text.is_char_boundary(*byte))
            .unwrap_or(text.len());
        self.say(&format!(
            "typing into the text note {} {} at {start} of {}",
            note.number,
            note.generation,
            text.len()
        ));
        self.note_editor = Some(NoteEditor {
            note,
            frame,
            styled,
        });
        // The window under the editor is taken away by the next placement.
        self.popups_shown.clear();
        let popups = match self.state.viewer.query(Query::Popups) {
            Answer::Popups(windows) => windows,
            _ => Vec::new(),
        };
        self.place_popups(&popups);
        let focused = view.clone();
        glib::idle_add_local_once(move || {
            focused.grab_focus();
        });
    }

    /// Tells the core the keyboard's Shift and Control, read off the display's keyboard, where they
    /// changed since it was last told: a script reads them at the event the pointer raises next
    /// (ADRs 1762, 1771).
    pub(super) fn tell_keys(&mut self) {
        self.tell_keys_with(false);
    }

    /// [`Self::tell_keys`], with whether an arrow key is making a choice field's selection — true
    /// only around the edit the arrow made, so that the field's Table 199 `/K` reads Adobe's
    /// `event.keyDown` true for it alone (ADR 1786).
    pub(super) fn tell_keys_with(&mut self, arrows: bool) {
        let Some(held) = gtk4::gdk::Display::default()
            .and_then(|display| display.default_seat())
            .and_then(|seat| seat.keyboard())
            .map(|keyboard| keyboard.modifier_state())
        else {
            return;
        };
        let keys = pdf_model::view::Keys {
            shift: held.contains(gtk4::gdk::ModifierType::SHIFT_MASK),
            modifier: held.contains(gtk4::gdk::ModifierType::CONTROL_MASK),
            arrows,
        };
        if keys != self.keys_told {
            self.keys_told = keys;
            self.dispatch(Command::Keys(keys));
        }
    }

    /// The byte of a note's `/Contents` that a press at `at`, in logical pixels of the page's
    /// overlay, is on — read from the label that set the text under it, whose Pango layout says
    /// which character a point is on: Table 166's `/Contents` where the window is plain, a rich
    /// paragraph's character aligned to `/Contents` by `viewer_host::popup::rich_offsets` where it
    /// is drawn from Table 172's `/RC` (ADR 1770). `None` for a press on no label of the text — the
    /// title bar, a reply, the paper below the last line.
    pub(super) fn note_place(&self, note: pdf_syntax::ObjectId, at: (f64, f64)) -> Option<usize> {
        let window = self
            .popups_shown
            .iter()
            .find(|window| window.note == Some(note))?;
        let contents = window.text.as_deref().unwrap_or_default();
        let placed = self
            .popups
            .iter()
            .find(|placed| placed.note == Some(note))?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a pointer position inside a window is far inside f32's exact integer range"
        )]
        let point = gtk4::graphene::Point::new(at.0 as f32, at.1 as f32);
        placed.texts.iter().find_map(|text| {
            let inside = self.ui.popups.compute_point(&text.label, &point)?;
            if !text
                .label
                .contains(f64::from(inside.x()), f64::from(inside.y()))
            {
                return None;
            }
            let layout = text.label.layout();
            let (left, top) = text.label.layout_offsets();
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a point inside a label, in Pango units, is far inside i32's range"
            )]
            let (_, index, trailing) = layout.xy_to_index(
                ((f64::from(inside.x()) - f64::from(left)) * f64::from(pango::SCALE)) as i32,
                ((f64::from(inside.y()) - f64::from(top)) * f64::from(pango::SCALE)) as i32,
            );
            let shown = layout.text();
            let shown = shown.as_str();
            let at = usize::try_from(index).ok()?;
            // Pango answers the character a point is on and whether it is on its trailing half,
            // which puts the caret after it.
            let byte = match usize::try_from(trailing).ok().filter(|count| *count > 0) {
                Some(count) => shown
                    .get(at..)?
                    .char_indices()
                    .nth(count)
                    .map_or(shown.len(), |(after, _)| at.saturating_add(after)),
                None => at,
            };
            match text.paragraph {
                None => contents.is_char_boundary(byte).then_some(byte),
                Some(paragraph) => {
                    let rich = window.rich.as_ref()?;
                    let runs: String = rich
                        .paragraphs
                        .get(paragraph)?
                        .runs
                        .iter()
                        .map(|run| run.text.as_str())
                        .collect();
                    // A list item's tag stands before its runs in the label and is no part of
                    // `/Contents`.
                    let prefix = shown.strip_suffix(runs.as_str())?.len();
                    let within = byte.saturating_sub(prefix).min(runs.len());
                    let tables = viewer_host::popup::rich_offsets(contents, rich)?;
                    tables.get(paragraph)?.get(within).copied()
                }
            }
        })
    }

    /// Takes the note's editor away and gives the keyboard back to the page.
    fn end_note(&mut self) {
        let Some(editor) = self.note_editor.take() else {
            return;
        };
        self.ui.fixed.remove(&editor.frame);
        self.say("the keyboard is back on the page");
        self.popups_shown.clear();
        let popups = match self.state.viewer.query(Query::Popups) {
            Answer::Popups(windows) => windows,
            _ => Vec::new(),
        };
        self.place_popups(&popups);
        self.ui.page_area.grab_focus();
    }
}

/// A note's window being typed into: which note, and the editor placed where its window is.
pub(super) struct NoteEditor {
    /// The text note `Edit::SetNoteText` names.
    note: pdf_syntax::ObjectId,
    /// The scrolled `GtkTextView` and the sentence under it, in the controls' layer.
    pub(super) frame: gtk4::Box,
    /// The runs' faces over the editor's text.
    styled: NoteStyle,
}

/// The faces a rich note's runs are set in over the characters of `/Contents` its editor holds,
/// and the sentence under it (ADR 1782).
///
/// **Set again whenever the window's answer changes**, from nothing: a run's characters move as a
/// person types, and the window goes plain once they are no longer `/Contents`' (ADR 1721), so the
/// spans are [`viewer_host::popup::run_spans`] of the text as it now stands. A buffer's tags are
/// no text, so setting them sends no edit and is not a step a person undoes. Pango states no glyph
/// scale for a tag either, so a font scale is said here as the label says it.
struct NoteStyle {
    /// The editor's text.
    buffer: gtk4::TextBuffer,
    /// The tags set over it, one per run, taken out of the buffer's table before the next.
    tags: Vec<gtk4::TextTag>,
    /// What the editor does not draw of the note, under its text.
    said: gtk4::Label,
    /// The editor's own size in points, which a run's relative size is of.
    base: f32,
    /// The editor's context, which measures a share of a space for a run's letter spacing.
    context: pango::Context,
}

impl NoteStyle {
    /// A text view holding `text`, a note's `/Contents`, set in the runs of `rich` where the window
    /// draws them, and the sentence that goes under it.
    fn editor(text: &str, rich: Option<&pdf_model::popup::RichNote>) -> (gtk4::TextView, Self) {
        let view = gtk4::TextView::new();
        view.set_wrap_mode(gtk4::WrapMode::WordChar);
        view.set_left_margin(POPUP_PADDING);
        view.set_right_margin(POPUP_PADDING);
        view.set_top_margin(POPUP_PADDING);
        view.set_vexpand(true);
        let buffer = view.buffer();
        buffer.set_text(text);
        let said = gtk4::Label::new(None);
        said.set_xalign(0.0);
        said.set_wrap(true);
        said.add_css_class("dim-label");
        said.set_margin_start(POPUP_PADDING);
        said.set_margin_end(POPUP_PADDING);
        let mut styled = Self {
            buffer,
            tags: Vec::new(),
            said,
            base: label_size(&view),
            context: view.pango_context(),
        };
        styled.style(text, rich);
        (view, styled)
    }

    /// Sets the runs of `rich`, the window's note as the core now answers it, over `text`, its
    /// `/Contents`; nothing where the window is plain or the editor's text is not that text yet.
    fn style(&mut self, text: &str, rich: Option<&pdf_model::popup::RichNote>) {
        let table = self.buffer.tag_table();
        for tag in self.tags.drain(..) {
            table.remove(&tag);
        }
        let (start, end) = self.buffer.bounds();
        let held = self.buffer.text(&start, &end, false);
        let spans = rich
            .filter(|_| held.as_str() == text)
            .and_then(|note| Some((note, viewer_host::popup::run_spans(text, note)?)));
        let Some((note, spans)) = spans else {
            self.said.set_visible(false);
            return;
        };
        // A text buffer counts in characters, a span in bytes of the same text.
        let at = |byte: usize| {
            text.get(..byte)
                .and_then(|before| i32::try_from(before.chars().count()).ok())
        };
        for span in &spans {
            let run = note
                .paragraphs
                .get(span.paragraph)
                .and_then(|paragraph| paragraph.runs.get(span.run));
            let (Some(run), Some(from), Some(to)) = (run, at(span.start), at(span.end)) else {
                continue;
            };
            let tag = run_tag(&run_face(run, self.base, &self.context));
            table.add(&tag);
            self.buffer.apply_tag(
                &tag,
                &self.buffer.iter_at_offset(from),
                &self.buffer.iter_at_offset(to),
            );
            self.tags.push(tag);
        }
        let sentence = viewer_host::popup::editor_not_drawn(note, false);
        self.said.set_text(sentence.as_deref().unwrap_or_default());
        self.said.set_visible(sentence.is_some());
    }
}

/// A widget's own font size in points, which a rich run's relative size is of; ten where the
/// theme states none, a common label size and only a stand-in.
pub(super) fn label_size(widget: &impl IsA<gtk4::Widget>) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a font size in Pango units is far inside f32's exact integer range"
    )]
    widget
        .pango_context()
        .font_description()
        .map(|description| description.size() as f32 / pango::SCALE as f32)
        .filter(|size| *size > 0.0)
        .unwrap_or(10.0)
}

/// A popup window placed over the page, and the labels that set its text (ADR 1770).
pub(super) struct PlacedPopup {
    /// The window.
    widget: gtk4::Overlay,
    /// [`viewer_core::PopupWindow::note`]: the note a press on this window retypes.
    note: Option<pdf_syntax::ObjectId>,
    /// The labels that set the note's own text, in order.
    texts: Vec<NoteText>,
}

/// One label that sets a note window's text, and which of it.
pub(super) struct NoteText {
    /// The label, whose Pango layout says which character a point is on.
    pub(super) label: gtk4::Label,
    /// The rich note's paragraph it sets, or `None` for Table 166's `/Contents` in one label.
    pub(super) paragraph: Option<usize>,
}

/// One of §12.5.6.14's popup windows, as the widgets GTK draws it with.
///
/// The clause gives a popup "no appearance stream", so there is nothing on the page to show and a
/// window is furniture — which is why `viewer-core` answers a rectangle and three strings and why
/// this is a real [`gtk4::Frame`] rather than a rectangle painted on the chrome layer. What is
/// this host's is only the *look*: the border, the fonts and the two style classes below.
/// The three texts and the box are `viewer_host::popup`'s, shared with the other two hosts.
fn popup_window(window: &viewer_host::Window<'_>) -> (gtk4::Overlay, Vec<NoteText>) {
    // The paper and the one-pixel edge are the three windows' one choice (ADR 1466): opaque, so
    // the page's words do not show through the note's, and not the page's white, so the window's
    // extent can be seen. Painted under the content rather than asked of a `GtkFrame`, whose theme
    // draws a line and no ground at all.
    let frame = gtk4::Overlay::new();
    let paper = gtk4::DrawingArea::new();
    paper.set_draw_func(|_, cr, width, height| {
        let (edge, ground) = (viewer_host::popup::EDGE, viewer_host::popup::PAPER);
        cr.set_source_rgb(f64::from(edge.r), f64::from(edge.g), f64::from(edge.b));
        if let Err(error) = cr.paint() {
            eprintln!("note: cannot draw a popup window's edge: {error}");
            return;
        }
        cr.set_source_rgb(
            f64::from(ground.r),
            f64::from(ground.g),
            f64::from(ground.b),
        );
        cr.rectangle(1.0, 1.0, f64::from(width) - 2.0, f64::from(height) - 2.0);
        if let Err(error) = cr.fill() {
            eprintln!("note: cannot draw a popup window's paper: {error}");
        }
    });
    frame.set_child(Some(&paper));
    // §12.5.6.14: a popup has "no appearance stream or associated actions of its own", so there is
    // nothing on it to activate — and a widget over the page that swallowed a press would take the
    // selection, the link and the form control underneath it away from the reader.
    frame.set_can_target(false);
    frame.set_can_focus(false);
    // A window is the rectangle the *document* states, so text that does not fit stops at its edge
    // rather than growing it. The drawn host stops at the same place for the same reason.
    frame.set_overflow(gtk4::Overflow::Hidden);

    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    // Inside the edge, so that the title bar does not paint over it.
    column.set_margin_start(1);
    column.set_margin_end(1);
    column.set_margin_top(1);
    column.set_margin_bottom(1);
    let bar = gtk4::Box::new(gtk4::Orientation::Horizontal, POPUP_PADDING);
    bar.set_margin_start(POPUP_PADDING);
    bar.set_margin_end(POPUP_PADDING);
    // §12.5.6.2's `/T`: "[t]he text label that shall be displayed in the title bar of the
    // annotation's popup window when open and active."
    let title = gtk4::Label::new(Some(window.title));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.set_ellipsize(pango::EllipsizeMode::End);
    title.add_css_class("heading");
    bar.append(&title);
    // Table 166's `/M`, in the one format `viewer_host::stamp` gives every date this program
    // shows, so that a popup and §14.3.3's panel do not spell one clause's answer two ways.
    if let Some(when) = viewer_host::popup::modified(window) {
        let stamp = gtk4::Label::new(Some(&when));
        stamp.add_css_class("dim-label");
        bar.append(&stamp);
    }
    // Table 166's `/C` is "[t]he title bar of the annotation's popup window", so the colour is
    // painted *behind* the bar's two labels rather than applied to their text. A drawing area
    // under an overlay, because GTK4 has no per-widget background short of a style sheet and a
    // provider per window would be a document's colour reaching into this program's CSS.
    let painted = gtk4::Overlay::new();
    if let Some(colour) = window.colour {
        let ground = gtk4::DrawingArea::new();
        ground.set_draw_func(move |_, cr, _, _| {
            cr.set_source_rgb(
                f64::from(colour.r),
                f64::from(colour.g),
                f64::from(colour.b),
            );
            if let Err(error) = cr.paint() {
                eprintln!("note: cannot draw a popup window's title bar: {error}");
            }
        });
        painted.set_child(Some(&ground));
    }
    painted.add_overlay(&bar);
    // **A `GtkOverlay` measures its main child and not its overlays**, and the main child here is
    // a drawing area with no natural size at all — so without this the title bar was allocated no
    // height and the note's first line was drawn through the author's name. Photographed, not
    // reasoned: the first screenshot of this window had the two on top of each other.
    painted.set_measure_overlay(&bar, true);
    column.append(&painted);

    // Table 172's `/RC` where the note states one this program reads, with each run's formatting
    // — "[a] rich text string … that shall be displayed in the popup window when the annotation is
    // opened" — and Table 166's `/Contents` otherwise. Wrapped by Pango either way, which is the
    // whole reason a native host places labels here instead of breaking lines for itself.
    let (body, texts) = popup_body(window);
    column.append(&body);

    // §12.5.6.2's thread, under the note it answers. Table 172: "[i]nteractive PDF processors
    // shall not display replies to an annotation individually but together in the form of
    // threaded comments", so `pdf_model::popup` has folded each reply's own window into this one
    // (ADR 1090) and a host that placed only the label above would show the note and lose the
    // conversation. The indent is what *threaded* means and stops at four levels, which is the
    // deepest chain anything in this tree has measured.
    for reply in window.replies {
        let comment = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        comment.set_margin_start(
            POPUP_PADDING.saturating_mul(
                1i32.saturating_add(i32::try_from(reply.depth.min(4)).unwrap_or(4)),
            ),
        );
        comment.set_margin_end(POPUP_PADDING);
        if let Some(who) = reply.title.as_deref().filter(|who| !who.is_empty()) {
            let author = gtk4::Label::new(Some(who));
            author.set_xalign(0.0);
            author.set_ellipsize(pango::EllipsizeMode::End);
            author.add_css_class("dim-label");
            comment.append(&author);
        }
        if let Some(rich) = reply.rich.as_ref() {
            comment.append(&rich_note(rich).0);
        } else {
            let said = gtk4::Label::new(Some(reply.text.as_deref().unwrap_or_default()));
            said.set_xalign(0.0);
            said.set_yalign(0.0);
            said.set_wrap(true);
            said.set_wrap_mode(pango::WrapMode::WordChar);
            comment.append(&said);
        }
        column.append(&comment);
    }

    // **In a scrolled window that scrolls nothing**, because an overlay child is allocated the
    // height it prefers at its narrowest width (trap 132): a column holding a thread was given the
    // height of every word on a line of its own, and the replies under the note were drawn below
    // the window's edge and clipped away. A scrolled window prefers no height of its own and hands
    // the column the window's width, at which the column is as tall as its lines; what the window's
    // rectangle cannot hold is still cut at its edge, as before.
    let held = gtk4::ScrolledWindow::new();
    held.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::External);
    held.set_propagate_natural_height(false);
    held.set_child(Some(&column));
    frame.add_overlay(&held);
    frame.set_measure_overlay(&held, true);
    (frame, texts)
}

/// The window's text: [`rich_note`] where the note states a rich text string this program reads,
/// and Table 166's `/Contents` in one wrapped label otherwise; with the labels that set it.
fn popup_body(window: &viewer_host::Window<'_>) -> (gtk4::Widget, Vec<NoteText>) {
    if let Some(rich) = window.rich {
        let (body, texts) = rich_note(rich);
        return (body.upcast(), texts);
    }
    let note = gtk4::Label::new(Some(window.text));
    note.set_xalign(0.0);
    note.set_yalign(0.0);
    note.set_wrap(true);
    note.set_wrap_mode(pango::WrapMode::WordChar);
    note.set_vexpand(true);
    note.set_margin_start(POPUP_PADDING);
    note.set_margin_end(POPUP_PADDING);
    note.set_margin_top(POPUP_PADDING);
    let texts = vec![NoteText {
        label: note.clone(),
        paragraph: None,
    }];
    (note.upcast(), texts)
}
