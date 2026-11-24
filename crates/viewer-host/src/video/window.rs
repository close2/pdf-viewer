//! The pictures a toolkit's window shows itself — GTK's, through
//! [`crate::native_window::WindowState`] — around one [`VideoPlayer`] made at the first clip, each
//! picture converted on the processor (RFC 0015 section 6, round 6; ADR 3545).
//!
//! What is the window's and not the player's: the picture it holds up now, converted by
//! [`super::on_the_processor`] — the oracle's own conversion, because a toolkit window that draws
//! with its own widgets has no texture of this program's to keep a picture in — and when it looks
//! again. The toolkit places the picture over the screen annotation that `viewer_core::Query::Screen`
//! answers with, and asks [`Pictures::look`] at [`Pictures::next_look`].

use std::path::PathBuf;
use std::time::Instant;

use pdf_syntax::ObjectId;
use viewer_core::{DocumentId, MediaOperation};

use super::{Seen, Shown, VideoPlayer};
use crate::sound::Wake;

/// What one look at the player found.
#[derive(Debug, Default)]
pub struct Looked {
    /// What the player said since the last look, oldest first.
    pub said: Vec<Seen>,
    /// Whether the picture held up changed — a new one, or none where there was one — so that the
    /// window redraws it.
    pub changed: bool,
}

/// A window's clip pictures: none, and no thread or worker, until the first (`CLAUDE.md`'s
/// startup rules).
#[derive(Debug, Default)]
pub struct Pictures {
    /// The media worker, where the window names it; otherwise the one beside this executable.
    worker: Option<PathBuf>,
    /// The player, from the first clip on.
    player: Option<VideoPlayer>,
    /// The picture held up, converted, while a clip is in hand.
    raster: Option<pdf_render::Raster>,
}

impl Pictures {
    /// The same, decoding with the worker `program` rather than the one beside this executable.
    #[must_use]
    pub fn with_worker(mut self, program: PathBuf) -> Self {
        self.worker = Some(program);
        self
    }

    /// Shows `shown` in place of the clip in hand, its pictures following `sound` — the clip's
    /// sound as [`crate::sound::Sounds::listening`] answers it — where it has one (ADR
    /// 3555). The first clip makes the player, with `wake` as what wakes the window's loop from the
    /// decoding thread; later clips do not call it.
    pub fn play(
        &mut self,
        shown: Shown,
        sound: Option<crate::sound::Listening>,
        wake: impl FnOnce() -> Wake,
    ) {
        let worker = self.worker.clone();
        let player = self.player.get_or_insert_with(|| {
            let player = VideoPlayer::new(wake());
            match worker {
                Some(program) => player.with_worker(program),
                None => player,
            }
        });
        player.play_beside(shown, sound);
    }

    /// Escape: stops the clip in hand and answers whether there was one, so that a window with
    /// none leaves the key to its key table.
    pub fn stop(&mut self) -> bool {
        match self.player.as_mut() {
            Some(player) if player.in_hand() => {
                player.stop();
                true
            }
            _ => false,
        }
    }

    /// Space: shows the clip held at its start (Table 291's `/A` false) or paused, and answers
    /// whether there was one.
    pub fn resume(&mut self) -> bool {
        self.player.as_mut().is_some_and(VideoPlayer::resume)
    }

    /// §12.6.4.14's stop, pause or resume of the clip associated with `annotation` in `document`;
    /// answers whether it acted ([`VideoPlayer::operate`]).
    pub fn operate(
        &mut self,
        document: DocumentId,
        annotation: ObjectId,
        operation: MediaOperation,
    ) -> bool {
        self.player
            .as_mut()
            .is_some_and(|player| player.operate(document, annotation, operation))
    }

    /// `document` closed: its clip stops.
    pub fn closed(&mut self, document: DocumentId) {
        if let Some(player) = self.player.as_mut() {
            player.closed(document);
        }
    }

    /// The window is closing: the decoding thread ends and the worker with it.
    pub fn end(&mut self) {
        self.player = None;
        self.raster = None;
    }

    /// Whether a clip is in hand, showing or paused: what Escape stops.
    #[must_use]
    pub fn in_hand(&self) -> bool {
        self.player.as_ref().is_some_and(VideoPlayer::in_hand)
    }

    /// Takes the newest picture whose time has come and converts it, and what the player said.
    /// Never waits.
    pub fn look(&mut self, now: Instant) -> Looked {
        let Some(player) = self.player.as_mut() else {
            return Looked::default();
        };
        let mut changed = false;
        if let Some(frame) = player.due(now) {
            match super::on_the_processor(&frame) {
                Ok(raster) => {
                    self.raster = Some(raster);
                    changed = true;
                }
                // The planes the worker's own size bound already admitted; refused all the same
                // rather than drawn wrong, and the clip ends with the reason.
                Err(refused) => {
                    player.stop();
                    let mut said = player.seen();
                    said.push(Seen::Refused(format!(
                        "a picture was not shown, because {refused}"
                    )));
                    self.raster = None;
                    return Looked {
                        said,
                        changed: true,
                    };
                }
            }
        }
        if !player.holds_a_picture() && self.raster.take().is_some() {
            changed = true;
        }
        Looked {
            said: player.seen(),
            changed,
        }
    }

    /// When to look again: [`VideoPlayer::next_due`], `None` while nothing waits for a time.
    #[must_use]
    pub fn next_look(&self, now: Instant) -> Option<Instant> {
        self.player.as_ref()?.next_due(now)
    }

    /// [`Self::next_look`] for a window the decoding thread cannot wake
    /// ([`VideoPlayer::next_due_unwoken`]): Qt's, which looks on its timer (ADR 3562).
    #[must_use]
    pub fn next_look_unwoken(&self, now: Instant) -> Option<Instant> {
        self.player.as_ref()?.next_due_unwoken(now)
    }

    /// Where the picture goes inside `quad`, the screen annotation in window pixels, by the clip's
    /// Table 291 `/F` ([`super::placed`]).
    #[must_use]
    pub fn placed(&self, quad: [f32; 8]) -> Option<super::Placed> {
        self.player.as_ref()?.placed(quad)
    }

    /// The picture held up and the document and screen annotation it is drawn over.
    #[must_use]
    pub fn picture(&self) -> Option<(&pdf_render::Raster, DocumentId, ObjectId)> {
        let raster = self.raster.as_ref()?;
        let (document, screen) = self.player.as_ref()?.screen()?;
        Some((raster, document, screen))
    }
}
