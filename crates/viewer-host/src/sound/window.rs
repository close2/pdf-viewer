//! The sounds a toolkit's window plays itself — GTK's and Qt's, which share
//! [`crate::native_window::WindowState`] — around one [`Player`] made at the first sound (RFC 0015
//! section 5, steps 3 and 4; ADR 3329).
//!
//! What is the window's and not the player's: which sentence holds the title while a sound sounds
//! or after one was refused, whether Escape has a sound to stop, and how long the window's
//! answers timer waits before it looks again. The wake that takes a sentence in sooner is the
//! toolkit's, handed over at the first sound; the timer stays the guarantee, because a wake that
//! finds the window inside a turn of its own is that turn's to answer (ADR 2070's argument).

use std::path::PathBuf;
use std::time::Duration;

use viewer_core::DocumentId;

use super::{Heard, OpenSink, Played, Player, Wake};

/// How often a window looks at the player while a sound sounds or one is being decoded.
///
/// A person hears a sound end before the title says so by at most this; the look is one channel
/// drain, which costs nothing a person sees at four times a second.
pub const LOOK: Duration = Duration::from_millis(250);

/// A window's sounds: none, and no thread, worker or helper, until the first (`CLAUDE.md`'s
/// startup rules).
pub struct Sounds {
    /// What opens the sink at the first sound: [`super::helper::open`] unless a test named a
    /// recording double. Taken by the player when it is made.
    open: Option<OpenSink>,
    /// The media worker, where the window names it; otherwise the one beside this executable.
    worker: Option<PathBuf>,
    /// The player, from the first sound on.
    player: Option<Player>,
    /// The player's last word while it holds — a sound sounding, or one refused — which the
    /// title carries. `None` once a sound ends or is stopped.
    sounding: Option<Heard>,
    /// A sound handed over and not yet answered with [`Heard::Playing`] or [`Heard::Refused`].
    awaiting: bool,
}

impl std::fmt::Debug for Sounds {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Sounds")
            .field("player", &self.player)
            .field("sounding", &self.sounding)
            .field("awaiting", &self.awaiting)
            .finish_non_exhaustive()
    }
}

impl Default for Sounds {
    fn default() -> Self {
        Self::new()
    }
}

impl Sounds {
    /// A window's sounds through the sound helper program (ADR 3317).
    #[must_use]
    pub fn new() -> Self {
        Self {
            open: None,
            worker: None,
            player: None,
            sounding: None,
            awaiting: false,
        }
    }

    /// The same, opening `open` in place of the helper: a test's recording double.
    #[must_use]
    pub fn opening_with(open: OpenSink) -> Self {
        Self {
            open: Some(open),
            ..Self::new()
        }
    }

    /// The same, decoding with the worker `program` rather than the one beside this executable.
    #[must_use]
    pub fn with_worker(self, program: PathBuf) -> Self {
        Self {
            worker: Some(program),
            ..self
        }
    }

    /// Plays `played` in place of whatever sounds. The first sound makes the player, with `wake`
    /// as what wakes the window's loop from the player's thread; later sounds do not call it.
    pub fn play(&mut self, played: Played, wake: impl FnOnce() -> Wake) {
        if self.player.is_none() {
            let open = self
                .open
                .take()
                .unwrap_or_else(|| Box::new(super::helper::open));
            let made = Player::new(open, wake());
            self.player = Some(match self.worker.clone() {
                Some(program) => made.with_worker(program),
                None => made,
            });
        }
        if let Some(running) = self.player.as_mut() {
            running.play(played);
            self.awaiting = true;
        }
    }

    /// What the player has said since the last call, oldest first, with the sentence the title
    /// carries brought up to date.
    pub fn heard(&mut self) -> Vec<Heard> {
        let Some(player) = self.player.as_mut() else {
            return Vec::new();
        };
        let heard = player.heard();
        for said in &heard {
            match said {
                Heard::Ended(_) | Heard::Stopped(_) => self.sounding = None,
                Heard::Playing(_) | Heard::Refused(_) => {
                    self.awaiting = false;
                    self.sounding = Some(said.clone());
                }
            }
        }
        heard
    }

    /// Stops the sound that sounds, and answers whether there was one — Escape's question, which
    /// a window with nothing sounding leaves to the key table.
    pub fn stop(&mut self) -> bool {
        let sounding = matches!(self.sounding, Some(Heard::Playing(_)));
        match (sounding, self.player.as_mut()) {
            (true, Some(player)) => {
                player.stop();
                true
            }
            _ => false,
        }
    }

    /// `document` closed: its sound stops, and a sound of another document sounds on.
    pub fn closed(&mut self, document: DocumentId) {
        if let Some(player) = self.player.as_mut() {
            player.closed(document);
        }
    }

    /// The window is closing: the player's thread ends, which stops the sound, closes the helper
    /// and kills the worker before the window is gone.
    pub fn end(&mut self) {
        self.player = None;
        self.sounding = None;
        self.awaiting = false;
    }

    /// What the title says of the sounds, or nothing: the sentence that holds, and Escape named
    /// while a sound sounds.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        match &self.sounding {
            Some(Heard::Playing(_)) => Some("playing a sound, Escape stops it"),
            Some(said) => Some(said.sentence()),
            None => None,
        }
    }

    /// How long the window's timer waits before it looks at the player again: [`LOOK`] while a
    /// sound is being decoded or sounds, and `None` otherwise, so that a window whose sound has
    /// ended wakes for it never.
    #[must_use]
    pub fn interval(&self) -> Option<Duration> {
        (self.awaiting || matches!(self.sounding, Some(Heard::Playing(_)))).then_some(LOOK)
    }

    /// Whether the first sound has made the player.
    #[must_use]
    pub fn started(&self) -> bool {
        self.player.is_some()
    }
}
