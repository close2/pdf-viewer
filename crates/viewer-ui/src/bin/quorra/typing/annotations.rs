//! The two annotations typed into or drawn here: §12.5.6.6's free text and §12.5.6.4's note.

use pdf_syntax::ObjectId;
use viewer_core::{Answer, Command, Edit, Query};
use winit::event::ElementState;
use winit::keyboard::{Key, NamedKey};

use super::{Drawing, FREE_TEXT_INK, Target, Typing, after, before, caret_boundary, spliced};
use crate::app::App;

impl App {
    /// A press on an open popup window whose text is a note a person may retype: the keyboard goes
    /// to the note, and the press goes no further, since the window is over the page and a press
    /// on it is not a press on what is underneath (ADR 1726).
    ///
    /// §12.5.6.14's popup "shall be used for editing the parent's text", and
    /// `viewer_core::PopupWindow::note` names the note where the window's text is one this program
    /// retypes, and the window's focus ring says it has the keyboard. **The caret goes where the
    /// press went**: this host lays a note's lines out itself, so the place is
    /// `chrome::popup_offset` over the layout `chrome::popup_windows` draws — the same lines, so
    /// the caret stands where the next character goes (ADR 1739). A window drawn from Table 172's
    /// `/RC` places it the same way, at the glyphs its runs lay out, on the `/Contents` offsets
    /// their characters are (ADR 1770). A window whose text is not retyped takes the press as
    /// before.
    pub(crate) fn press_on_note(&mut self, at: (f32, f32)) -> bool {
        let Answer::Popups(windows) = self.viewer.query(Query::Popups) else {
            return false;
        };
        // The last window drawn is the one on top.
        let Some(window) = windows
            .iter()
            .rev()
            .find(|window| viewer_host::covers(window.quad, at))
        else {
            return false;
        };
        let Some(note) = window.note else {
            return false;
        };
        let end = window.text.as_deref().map_or(0, str::len);
        let caret = self.note_offset(window, at).unwrap_or(end);
        println!(
            "note: typing into the text note {} {} at {caret} of {end}",
            note.number, note.generation
        );
        self.typing = Some(Typing::at_offset(Target::Note(note), at, caret));
        self.redraw();
        true
    }

    /// One end of the drag that draws §12.5.6.6's rectangle.
    ///
    /// The press records a corner and the release sends `Edit::FreeText` with both, then aims the
    /// keyboard at what it made — which needs no event, because `Query::FreeTextAt` at a point
    /// inside the rectangle answers with the annotation the core just added. Asking rather than
    /// being told is the rule this vocabulary has grown by: a message is added for a question a
    /// host cannot answer for itself, and this is not one.
    ///
    /// **The mode ends with the release, whether or not anything was made.** A drag that never
    /// moved has drawn no box and the core answers it by doing nothing (`add_free_text` refuses a
    /// rectangle with no area); leaving the mode armed after that would be a program that seemed
    /// stuck.
    pub(crate) fn draw_free_text(&mut self, drawing: Drawing, element: ElementState) {
        let at = self.on_page(self.cursor);
        match (drawing, element) {
            (Drawing::Armed, ElementState::Pressed) => self.drawing = Some(Drawing::From(at)),
            (Drawing::From(from), ElementState::Released) => {
                self.drawing = None;
                // The colour of the *text*, which Table 177's `/DA` carries — Table 166's `/C` is
                // an icon's background, a popup's title bar and a link's border, none of which
                // this subtype has. A dark red is this host's choice and no clause's, the same
                // footing the highlight's yellow stands on.
                self.dispatch(Command::Edit(Edit::FreeText {
                    from,
                    to: at,
                    colour: FREE_TEXT_INK,
                }));
                let middle = ((from.0 + at.0) * 0.5, (from.1 + at.1) * 0.5);
                self.typing = self.aim_at_free_text(middle);
                if self.typing.is_none() {
                    println!("note: that rectangle has no area, so there is nothing to type in");
                }
                self.redraw();
            }
            (Drawing::Armed, ElementState::Released)
            | (Drawing::From(_), ElementState::Pressed) => {}
        }
    }

    /// Where a press at `at` puts the caret in a note's window — `chrome::popup_offset`, under the
    /// scale the window is drawn at — or `None` where the window's text has no such place.
    fn note_offset(&self, window: &viewer_core::PopupWindow, at: (f32, f32)) -> Option<usize> {
        let chrome = self.chrome.as_ref()?;
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        viewer_ui::chrome::popup_offset(chrome, window, at, scale)
    }

    /// The offset a line up or down from `caret` stands at in a note's window, at the same place
    /// across, or `None` where there is no line there or no layout to ask.
    fn note_line(
        &self,
        window: &viewer_core::PopupWindow,
        caret: usize,
        down: bool,
    ) -> Option<usize> {
        let chrome = self.chrome.as_ref()?;
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        let ((x, top), (_, bottom)) = viewer_ui::chrome::popup_caret(chrome, window, caret, scale)?;
        // One line's advance from the middle of this one: the caret is `1.25` of the text size
        // tall, and `chrome::popup_windows` sets its lines `1.25` of the text size apart.
        let step = bottom - top;
        let middle = f32::midpoint(top, bottom);
        let to = if down { middle + step } else { middle - step };
        let moved = viewer_ui::chrome::popup_offset(chrome, window, (x, to), scale)?;
        // The nearest line to a point past the first or the last is that line itself.
        (moved != caret).then_some(moved)
    }

    /// One key press while a note's window has the keyboard, the caret where a press or a key put
    /// it ([`App::press_on_note`] says how): a character, a space or a return goes in at the
    /// caret, Backspace and Delete take out the character before or after it, the four arrows,
    /// Home and End move it, and Escape gives the keyboard back to the page. Every other key is
    /// consumed, so that a page does not turn under somebody typing.
    pub(super) fn typed_into_note(
        &mut self,
        typing: Typing,
        note: ObjectId,
        key: &Key<&str>,
    ) -> bool {
        // The window's text as the core shows it — the note's retyping where there is one — while
        // the window is still open on the screen; a window that closed takes the keyboard back.
        let window = match self.viewer.query(Query::Popups) {
            Answer::Popups(windows) => windows.into_iter().find(|window| window.note == Some(note)),
            _ => None,
        };
        let Some(window) = window else {
            self.typing = None;
            return false;
        };
        let current = window.text.clone().unwrap_or_default();
        let current = current.as_str();
        // The note's text is the core's and a script may have retyped it since the last key, so
        // the caret is clamped to the text this press starts from.
        let caret = caret_boundary(current, typing.caret);
        let inserted = |text: &str| {
            (
                Some(spliced(current, caret, caret, text)),
                caret.saturating_add(text.len()),
            )
        };
        let (next, moved) = match *key {
            Key::Named(NamedKey::Escape) => {
                self.typing = None;
                println!("note: the keyboard is back on the page");
                self.redraw();
                return true;
            }
            Key::Character(_) if self.control => return true,
            Key::Named(NamedKey::ArrowLeft) => (None, before(current, caret)),
            Key::Named(NamedKey::ArrowRight) => (None, after(current, caret)),
            Key::Named(NamedKey::ArrowUp) => {
                (None, self.note_line(&window, caret, false).unwrap_or(caret))
            }
            Key::Named(NamedKey::ArrowDown) => {
                (None, self.note_line(&window, caret, true).unwrap_or(caret))
            }
            Key::Named(NamedKey::Home) => (None, 0),
            Key::Named(NamedKey::End) => (None, current.len()),
            Key::Named(NamedKey::Backspace) => {
                let from = before(current, caret);
                (Some(spliced(current, from, caret, "")), from)
            }
            Key::Named(NamedKey::Delete) => (
                Some(spliced(current, caret, after(current, caret), "")),
                caret,
            ),
            Key::Named(NamedKey::Enter) => inserted("\n"),
            Key::Named(NamedKey::Space) => inserted(" "),
            Key::Character(text) if !text.is_empty() => inserted(text),
            _ => return true,
        };
        if let Some(next) = next.filter(|next| next != current) {
            self.dispatch(Command::Edit(Edit::SetNoteText {
                annotation: note,
                text: next,
            }));
        }
        self.typing = Some(Typing {
            caret: moved,
            anchor: moved,
            ..typing
        });
        self.redraw();
        true
    }
}
