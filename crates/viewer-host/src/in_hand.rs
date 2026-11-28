//! Which annotation's clip a player holds, followed from the player's own state, and the
//! [`Command::InHand`]s that tell the core (ADR 3889).
//!
//! ISO 32000-2 Table 344's `/Snapshot` is displayed "in place of the current artwork"
//! (§13.7.2.3.5) from the activation that invoked its view to the annotation's deactivation. Every
//! deactivation reaches a player and nothing else: the reader's stop (Table 336's `XD`, Escape or
//! the controller bar), the clip's end, a later clip in its place, and the stop a `PC` or `PI`
//! condition asks of the window. So each player says when it takes a clip associated with an
//! annotation and when the clip leaves its hands, and the core draws the snapshot for as long as
//! one does. The sound player and the picture player each keep one of these, since a clip with
//! sound and pictures is in both their hands.

use pdf_syntax::ObjectId;
use viewer_core::{Command, DocumentId};

/// The annotation whose clip is held now, and the commands owed to the core.
#[derive(Debug, Default)]
pub(crate) struct Holding {
    /// The document and annotation of the clip in hand now.
    held: Option<(DocumentId, ObjectId)>,
    /// What the core has not yet been told.
    owed: Vec<Command>,
}

impl Holding {
    /// The commands owed to the core since the last call.
    pub(crate) fn owed(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.owed)
    }

    /// Holds `now` in place of what is held, owing the core a command for each change: the clip
    /// that left first, then the one taken.
    pub(crate) fn hold(&mut self, now: Option<(DocumentId, ObjectId)>) {
        if self.held == now {
            return;
        }
        if let Some((document, annotation)) = self.held {
            self.owed.push(Command::InHand {
                document,
                annotation,
                in_hand: false,
            });
        }
        if let Some((document, annotation)) = now {
            self.owed.push(Command::InHand {
                document,
                annotation,
                in_hand: true,
            });
        }
        self.held = now;
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, DocumentId, Holding, ObjectId};

    /// The commands owed, as the annotation's number and whether it is in hand.
    fn owed(holding: &mut Holding) -> Vec<(u32, bool)> {
        holding
            .owed()
            .into_iter()
            .map(|command| match command {
                Command::InHand {
                    annotation,
                    in_hand,
                    ..
                } => (annotation.number, in_hand),
                other => panic!("only InHand is owed: {other:?}"),
            })
            .collect()
    }

    fn clip(number: u32) -> (DocumentId, ObjectId) {
        (DocumentId(1), ObjectId::new(number, 0))
    }

    /// A clip taken is said once and left once; another in its place leaves the first before it
    /// takes the second; the same clip said twice owes nothing.
    #[test]
    fn each_change_of_clip_in_hand_owes_the_core_one_command_for_each_side() {
        let mut holding = Holding::default();
        holding.hold(Some(clip(7)));
        holding.hold(Some(clip(7)));
        assert_eq!(owed(&mut holding), vec![(7, true)]);
        holding.hold(Some(clip(9)));
        assert_eq!(owed(&mut holding), vec![(7, false), (9, true)]);
        holding.hold(None);
        holding.hold(None);
        assert_eq!(owed(&mut holding), vec![(9, false)]);
    }
}
