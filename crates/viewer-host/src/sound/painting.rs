//! Which screen annotation's rectangle a window's sound plays in, followed from what the player
//! says, and the [`Command::PlayingIn`]s that tell the core (ADR 3407).
//!
//! ISO 32000-2 Table 294's `/W` 3 plays a media object in "\[t]he rectangle occupied by the screen
//! annotation", and its `/B` "shall be used if the media object does not entirely cover the
//! rectangle" — all of it, for a sound — so the rectangle is painted while the sound plays. The
//! core paints it, and only the player knows when the sound starts and stops: each sound handed to
//! the player is answered first by exactly one of [`Heard::Playing`], [`Heard::Holding`],
//! [`Heard::Paused`] or [`Heard::Refused`], in the order the sounds were handed over, and a sound
//! that sounds ends with [`Heard::Ended`] or [`Heard::Stopped`]. That order is what this follows.

use std::collections::VecDeque;

use viewer_core::{Command, DocumentId, Screen};

use super::Heard;

/// The screens of the sounds handed over and not yet answered, the one painted now, and the
/// commands owed to the core.
#[derive(Debug, Default)]
pub(super) struct Painting {
    /// Each sound handed to the player and not yet first answered: its document and its screen.
    awaiting: VecDeque<(DocumentId, Option<Screen>)>,
    /// The screen of the sound that sounds, or is held at its start, now.
    painted: Option<(DocumentId, Screen)>,
    /// What the core has not yet been told.
    owed: Vec<Command>,
}

impl Painting {
    /// A sound handed to the player.
    pub(super) fn played(&mut self, document: DocumentId, screen: Option<Screen>) {
        self.awaiting.push_back((document, screen));
    }

    /// What the player said, in the order it said it.
    pub(super) fn heard(&mut self, said: &Heard) {
        match said {
            Heard::Playing(_) | Heard::Holding(_) | Heard::Paused(_) => {
                // A held sound's resumption says `Playing` again with nothing awaiting, and the
                // screen painted at its first answer stays.
                if let Some((document, screen)) = self.awaiting.pop_front() {
                    self.paint(screen.map(|screen| (document, screen)));
                }
            }
            Heard::Refused(_) => {
                self.awaiting.pop_front();
            }
            Heard::Ended(_) | Heard::Stopped(_) => self.paint(None),
        }
    }

    /// The commands owed to the core since the last call.
    pub(super) fn owed(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.owed)
    }

    /// Paints `now` in place of what is painted, owing the core a command for each change.
    fn paint(&mut self, now: Option<(DocumentId, Screen)>) {
        if self.painted == now {
            return;
        }
        if let Some((document, _)) = self.painted
            && now.is_none_or(|(now, _)| now != document)
        {
            self.owed.push(Command::PlayingIn {
                document,
                screen: None,
            });
        }
        if let Some((document, screen)) = now {
            self.owed.push(Command::PlayingIn {
                document,
                screen: Some(screen),
            });
        }
        self.painted = now;
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, DocumentId, Heard, Painting, Screen};

    /// The commands owed, as the document and the painted annotation's number.
    fn owed(painting: &mut Painting) -> Vec<(u64, Option<u32>)> {
        painting
            .owed()
            .into_iter()
            .map(|command| match command {
                Command::PlayingIn { document, screen } => {
                    (document.0, screen.map(|screen| screen.annotation.number))
                }
                other => panic!("only PlayingIn is owed: {other:?}"),
            })
            .collect()
    }

    fn screen(number: u32) -> Screen {
        Screen {
            annotation: pdf_syntax::ObjectId::new(number, 0),
            background: pdf_model::media::Background::default(),
        }
    }

    /// A sound with a screen paints it at its first answer and takes it away at its end; a second
    /// sound, which stops the first, repaints in the player's own order — the first's `Stopped`,
    /// then the second's `Playing` — and a refused sound and one with no screen paint nothing.
    #[test]
    fn a_screen_is_painted_from_a_sounds_first_answer_to_its_end() {
        let said = |text: &str| text.to_owned();
        let mut painting = Painting::default();
        painting.played(DocumentId(1), Some(screen(4)));
        assert!(
            owed(&mut painting).is_empty(),
            "nothing before the player answers"
        );
        painting.heard(&Heard::Playing(said("one")));
        assert_eq!(owed(&mut painting), [(1, Some(4))]);
        painting.played(DocumentId(2), Some(screen(7)));
        painting.heard(&Heard::Stopped(said("one")));
        painting.heard(&Heard::Paused(said("two")));
        painting.heard(&Heard::Playing(said("two, resumed")));
        assert_eq!(owed(&mut painting), [(1, None), (2, Some(7))]);
        painting.played(DocumentId(2), Some(screen(9)));
        painting.heard(&Heard::Refused(said("three")));
        painting.heard(&Heard::Ended(said("two")));
        painting.played(DocumentId(2), None);
        painting.heard(&Heard::Playing(said("four")));
        assert_eq!(owed(&mut painting), [(2, None)]);
    }
}
