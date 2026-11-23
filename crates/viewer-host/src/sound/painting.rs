//! Which screen annotation's rectangle a window's sound plays in, as the player's thread tells it,
//! and the [`Command::PlayingIn`]s that tell the core (ADR 3407).
//!
//! ISO 32000-2 Table 294's `/W` 3 plays a media object in "\[t]he rectangle occupied by the screen
//! annotation", and its `/B` "shall be used if the media object does not entirely cover the
//! rectangle" — all of it, for a sound — so the rectangle is painted while the sound is in hand:
//! from the moment the thread takes it, sounding or held, to its end, its stop or its refusal. A
//! pause leaves it painted, because the rendition is still associated with the annotation
//! (§12.6.4.14, ADR 3433). The thread says each change, since it alone knows when they happen.

use viewer_core::{Command, DocumentId, Screen};

/// The screen painted now, and the commands owed to the core.
#[derive(Debug, Default)]
pub(super) struct Painting {
    /// The screen of the sound in hand now.
    painted: Option<(DocumentId, Screen)>,
    /// What the core has not yet been told.
    owed: Vec<Command>,
}

impl Painting {
    /// The commands owed to the core since the last call.
    pub(super) fn owed(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.owed)
    }

    /// Paints `now` in place of what is painted, owing the core a command for each change.
    pub(super) fn paint(&mut self, now: Option<(DocumentId, Screen)>) {
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
    use super::{Command, DocumentId, Painting, Screen};

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

    /// A screen is painted once, cleared once, and a screen of another document clears the first
    /// document's before it paints its own; the same screen said twice owes nothing.
    #[test]
    fn each_change_of_screen_owes_the_core_one_command() {
        let mut painting = Painting::default();
        painting.paint(Some((DocumentId(1), screen(4))));
        painting.paint(Some((DocumentId(1), screen(4))));
        assert_eq!(owed(&mut painting), [(1, Some(4))]);
        painting.paint(Some((DocumentId(2), screen(7))));
        assert_eq!(owed(&mut painting), [(1, None), (2, Some(7))]);
        painting.paint(Some((DocumentId(2), screen(9))));
        assert_eq!(owed(&mut painting), [(2, Some(9))]);
        painting.paint(None);
        painting.paint(None);
        assert_eq!(owed(&mut painting), [(2, None)]);
    }
}
