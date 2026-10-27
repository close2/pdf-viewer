//! One key press while a field has the keyboard.

use viewer_core::{Command, Edit, Entered};
use winit::keyboard::{Key, NamedKey};

use super::{Aim, Typing, after, before, caret_boundary, spliced, stepped};
use crate::app::App;

impl App {
    /// One key press, while a field has the keyboard. Answers whether it was consumed.
    ///
    /// **Nothing is buffered here.** Every press re-asks the core what the field says and sends
    /// back that value with one character added or one removed, so §12.7.5.3's `DoNotScroll`
    /// truncating a value is a thing the host *reads* rather than a thing it has to predict
    /// (ADR 0197). It costs a query per keystroke, which is a walk of one page's annotations.
    pub(crate) fn typed(&mut self, key: &Key<&str>) -> bool {
        match self.typing.map(|typing| (typing, typing.note())) {
            Some((typing, Some(note))) => self.typed_into_note(typing, note, key),
            Some((_, None)) => self.typed_into_value(key),
            None => false,
        }
    }

    /// [`App::typed`] for a field or a free text annotation, whose caret the core places.
    fn typed_into_value(&mut self, key: &Key<&str>) -> bool {
        let Some(typing) = self.typing else {
            return false;
        };
        let Some((current, aim)) = self.aimed(typing) else {
            // The field or the annotation went away — a page turned under the pointer — so the
            // keyboard goes back.
            self.typing = None;
            return false;
        };
        // The caret is clamped to the value *this* press starts from, because the last one may
        // have been truncated by §12.7.5.3's `DoNotScroll` — the same reason nothing is buffered.
        let caret = caret_boundary(&current, typing.caret);
        let anchor = caret_boundary(&current, typing.anchor);
        let (low, high) = (caret.min(anchor), caret.max(anchor));
        // **Shift holds the anchor still and moves only the caret**, which is this host's
        // convention and no clause's — the standard states neither a cursor nor a selection inside
        // a value (ADR 0225). Without it a move collapses the selection, which is why every arm
        // below says where *both* ends go.
        let held = |to: usize| if self.shift { (to, anchor) } else { (to, to) };
        let (next, moved, anchored) = match *key {
            Key::Named(NamedKey::Escape) => {
                self.typing = None;
                // §12.7.5.3's file-select control: "the field's text represents the pathname of a
                // file whose contents shall be submitted as the field's value". Escape is where
                // this window finishes with a field, so the pathname becomes a file *here* rather
                // than on every keystroke — a filesystem read per character would be this host
                // opening a file sixty times a second (ADR 1216).
                self.commit_chosen_file(&aim, &current);
                println!("note: the keyboard is back on the page");
                // The caret goes with the keyboard, and the window is what has to be told: this
                // press changes nothing about the *document*, so no command is sent and nothing
                // else would ask for the frame that takes the caret off the screen.
                self.redraw();
                return true;
            }
            // Ctrl + C, X and V, which needed no message at all — see `clipped`.
            Key::Character(text) if self.control => match self.clipped(text, &current, (low, high))
            {
                Some(outcome) => outcome,
                // Every other key with Control held is consumed rather than sent to the page: a
                // field has the keyboard, and a magnification while somebody is typing is the
                // surprise this whole state exists to prevent.
                None => return true,
            },
            // Moving the caret changes nothing about the document, so these send no edit at all
            // and only ask for the frame that redraws the caret. A move that is not extending the
            // selection lands on the *edge* of it rather than one character further, which is what
            // a person means by pressing Left with something selected.
            Key::Named(NamedKey::ArrowLeft) => {
                let (to, anchor) = stepped(&current, (caret, anchor), self.shift, false);
                (None, to, anchor)
            }
            Key::Named(NamedKey::ArrowRight) => {
                let (to, anchor) = stepped(&current, (caret, anchor), self.shift, true);
                (None, to, anchor)
            }
            Key::Named(NamedKey::Home) => {
                let (to, anchor) = held(0);
                (None, to, anchor)
            }
            Key::Named(NamedKey::End) => {
                let (to, anchor) = held(current.len());
                (None, to, anchor)
            }
            // With something selected, Backspace and Delete take out what is selected and nothing
            // more — the same statement typing a character makes, and the reason both are one
            // splice rather than two behaviours.
            Key::Named(NamedKey::Backspace) => {
                let from = if low < high {
                    low
                } else {
                    before(&current, caret)
                };
                (Some(spliced(&current, from, high, "")), from, from)
            }
            Key::Named(NamedKey::Delete) => {
                let to = if low < high {
                    high
                } else {
                    after(&current, caret)
                };
                (Some(spliced(&current, low, to, "")), low, low)
            }
            // Table 231 bit 13 clear restricts the text "to a single line", so Enter there is no
            // character: it commits what was typed, the way a tab or a press elsewhere does, and
            // the keyboard goes back to the page because the page now shows the field's format,
            // where a caret placed in the value would stand between the wrong characters (ADR 1592).
            Key::Named(NamedKey::Enter) if self.single_line(typing.at) => {
                return self.commit_on_enter(aim);
            }
            Key::Named(NamedKey::Enter) => {
                // §12.7.5.3's Multiline decides whether a return is a character or the end of
                // typing, and the core is what knows: a value with a newline in it lays out on two
                // lines only where Table 231 bit 13 is set, and `variable_text::wrap` is where
                // that is read. So the host offers the newline and the field decides what to keep.
                let to = low.saturating_add(1);
                (Some(spliced(&current, low, high, "\n")), to, to)
            }
            Key::Character(text) if !text.is_empty() => {
                let to = low.saturating_add(text.len());
                (Some(spliced(&current, low, high, text)), to, to)
            }
            Key::Named(NamedKey::Space) => {
                let to = low.saturating_add(1);
                (Some(spliced(&current, low, high, " ")), to, to)
            }
            _ => return false,
        };
        self.typing = Some(Typing {
            caret: moved,
            anchor: anchored,
            ..typing
        });
        // A caret that moved is chrome and not a page: `Query::Caret` answers from state this host
        // holds, so nothing has to be interpreted again and the window only repaints. A keystroke
        // that leaves the value as it was is the same case, and there are two of them — Backspace
        // at the start and Delete at the end — where sending the edit anyway would put an entry in
        // the log, mark the document unsaved and re-interpret the page for a picture that cannot
        // differ.
        let Some(next) = next.filter(|next| *next != current) else {
            self.redraw();
            return true;
        };
        // Through `dispatch`, not through `Viewer::handle` directly: the events an edit raises
        // are what asks for the next frame, and a host that counted them instead of pumping them
        // would type into a page that never redraws. (It did, for one run.)
        self.dispatch(Command::Edit(match aim {
            Aim::Field(field) => Edit::SetField {
                field,
                value: Entered::Text(next),
            },
            Aim::FreeText(annotation) => Edit::SetFreeText {
                annotation,
                text: next,
            },
        }));
        // And the field decides how much of that it took, so where the caret ended up is read
        // back rather than assumed — a value §12.7.5.3 truncated is shorter than what was sent.
        if let Some((taken, _)) = self.aimed(typing) {
            self.typing = Some(Typing {
                caret: caret_boundary(&taken, moved),
                anchor: caret_boundary(&taken, anchored),
                ..typing
            });
        }
        true
    }
}
