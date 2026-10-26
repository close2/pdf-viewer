//! What the reader does at the keyboard and the pointer, and the prompt's share of it.
//!
//! Methods of [`crate::Host`], which this file continues (ADR 2352).

use std::time::Instant;

use viewer_core::{Command, PageTarget, Zoom};
use viewer_host::Supplied;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, NamedKey};

use crate::exchange::Held;
use crate::{DOCUMENT, Host, WHEEL_NOTCH, typing};

impl Host {
    /// A key press while §7.6.4.1's card has the keyboard.
    ///
    /// Every key is taken, which is what *modal* means here (the flagship's `password_key`, whose
    /// rule this copies): the document behind the card is **not open**, so a key that turned a
    /// page would be turning a page that does not exist — and Escape is the prompt's decline
    /// rather than [`Self::abort`], because what the person is declining is the question, not the
    /// worker. Escape and an empty Enter reach [`viewer_host::password::supplied`] by the same
    /// route, and that one place decides what a decline means.
    pub(crate) fn password_key(&mut self, key: &Key<&str>) {
        match key {
            Key::Named(NamedKey::Escape) => {
                self.password.clear();
                self.password_answered();
            }
            Key::Named(NamedKey::Enter) => self.password_answered(),
            Key::Named(NamedKey::Backspace) => {
                self.password.backspace();
                self.card_changed();
            }
            Key::Named(NamedKey::Space) => {
                self.password.typed(" ");
                self.card_changed();
            }
            Key::Character(text) if !text.is_empty() => {
                self.password.typed(text);
                self.card_changed();
            }
            // A key with no character and no meaning here. Taken anyway, for the reason above.
            _ => {}
        }
    }

    /// The card was answered: open again with what was typed, or say why nothing opened.
    ///
    /// The [`viewer_core::Secret`] moves from the card into [`Command::Open`] and is dropped
    /// with it — no copy of it stays on this side, and the trace's [`super::arguments::brief`] never prints one.
    /// The file is opened again rather than kept: rule 2 makes the filesystem this side's, and a
    /// file that has gone away between the attempts is a fact about this machine, said out loud.
    fn password_answered(&mut self) {
        let typed = self.password.take();
        self.card_changed();
        let secret = match viewer_host::password::supplied(typed) {
            Supplied::Open(secret) => secret,
            Supplied::Cancelled => {
                eprintln!("note: {}", viewer_host::password::CANCELLED);
                if self.decline_the_prompted() {
                    return;
                }
                self.heading = format!("{} — {}", self.name(), viewer_host::password::CANCELLED);
                self.retitle();
                return;
            }
        };
        let bytes = match pdf_syntax::FileBytes::on_disk(self.prompted_for()) {
            Ok(bytes) => bytes,
            Err(problem) => {
                eprintln!(
                    "note: cannot reopen {}: {problem}",
                    self.prompted_for().display()
                );
                if self.decline_the_prompted() {
                    return;
                }
                self.heading = format!("{} — cannot reopen the file", self.name());
                self.retitle();
                return;
            }
        };
        self.offer_the_prompted();
        // The worker survives an open it could not finish (ADR 0597), so the retry goes to the
        // same process: the document's descriptor crosses once more, which since ADR 0812 costs
        // an `open` and a `sendmsg` rather than the file's whole length down a pipe, however
        // many of `ATTEMPTS` it is paid.
        self.dispatch(Command::Open {
            id: DOCUMENT,
            bytes,
            password: Some(secret),
            fragment: None,
        });
    }

    /// A press or a release on the page, as `Command::Pointer` (ADR 1987).
    ///
    /// Held behind a busy worker as any command is, so the two ends arrive in order. **After the
    /// release the keyboard follows the focus**, as after a Tab: a press inside a text field is
    /// how a person says "type here", and the worker's viewer has moved §12.6.3's focus there.
    pub(crate) fn click(&mut self, state: ElementState) {
        let action = match state {
            ElementState::Pressed => viewer_core::PointerAction::Pressed,
            ElementState::Released => viewer_core::PointerAction::Released,
        };
        self.dispatch(Command::Pointer {
            at: self.cursor,
            action,
        });
        if action == viewer_core::PointerAction::Released {
            if self.must_hold() {
                self.hold(Held::Focus);
            } else {
                self.ask_the_focus();
            }
        }
    }

    /// One key, one meaning; anything else is nothing.
    pub(crate) fn key(&mut self, event: &KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        self.press(&event.logical_key);
    }

    /// A pressed key: held while the worker is busy, unless it is one about the wait itself
    /// (ADR 1827), and otherwise what it means now.
    pub(crate) fn press(&mut self, key: &Key) {
        if self.permissions_key(key) {
            return;
        }
        if self.must_hold() && !self.acts_at_once(key) {
            self.hold(Held::Key {
                key: key.clone(),
                modifiers: self.modifiers,
                at: Instant::now(),
            });
            return;
        }
        if self.password.shown {
            self.password_key(&key.as_ref());
            return;
        }
        if self.question.shown {
            self.question_key(&key.as_ref());
            return;
        }
        if let typing::Typed::Taken = self.field_key(&key.as_ref()) {
            return;
        }
        match key {
            Key::Named(NamedKey::ArrowRight | NamedKey::PageDown | NamedKey::Space) => {
                self.dispatch(Command::GoTo(PageTarget::Next));
            }
            Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) => {
                self.dispatch(Command::GoTo(PageTarget::Previous));
            }
            Key::Named(NamedKey::ArrowDown) => self.dispatch(Command::Scroll {
                dx: 0.0,
                dy: WHEEL_NOTCH,
            }),
            Key::Named(NamedKey::ArrowUp) => self.dispatch(Command::Scroll {
                dx: 0.0,
                dy: -WHEEL_NOTCH,
            }),
            Key::Named(NamedKey::Home) => self.dispatch(Command::GoTo(PageTarget::First)),
            Key::Named(NamedKey::End) => self.dispatch(Command::GoTo(PageTarget::Last)),
            Key::Named(NamedKey::Escape) => self.abort(),
            Key::Character(text) => match text.as_str() {
                "+" | "=" => self.dispatch(Command::Zoom {
                    zoom: Zoom::In,
                    at: None,
                }),
                "-" => self.dispatch(Command::Zoom {
                    zoom: Zoom::Out,
                    at: None,
                }),
                "q" => self.leaving = true,
                // The archive key, said rather than ignored: `Command::Archive` does not cross
                // this window's pipe (ADR 1954).
                "A" => eprintln!("note: {}", viewer_host::archiving::NOT_IN_A_CONFINED_WINDOW),
                "m" => {
                    let on = self.measuring.toggle();
                    eprintln!("note: {}", viewer_host::measuring::switched(on));
                }
                _ => {}
            },
            _ => {}
        }
    }
}
