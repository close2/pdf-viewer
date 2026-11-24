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

/// What a window says of a key or a click refused while a synchronous sound holds the reader
/// ([`Sounds::holds_the_reader`]).
pub const HELD: &str = "a sound is playing synchronously (ISO 32000-2 §12.6.4.9, Table 212's \
                        /Synchronous), so nothing but Escape, which cancels it, is taken until it \
                        ends";

/// What the title says while a sound is held at its start (ADR 3351) or paused by a rendition
/// action (ADR 3433).
pub const PAUSED: &str = "a sound is paused, Space plays it and Escape stops it";

/// What the title says while a window without its sound helper has handed a sound to this
/// machine's player (ADR 3417); the terminal has the whole sentence, with the helper's place.
pub const HANDED_ON: &str = "no sound helper here, so a sound is handed to this machine's player";

/// Who plays a sound a window is handed, settled once at its launch, before the core is told
/// whether the window plays a sound itself (ISO 32000-2 §13.2.2's viability is judged for the
/// player a clip reaches; ADRs 3350, 3417).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hearing {
    /// The window plays nothing itself and hands every clip on: the C ABI's window whose host has
    /// not said it draws pictures, which hands each clip to the platform's own player (ADR 3217).
    HandsOn,
    /// The window plays a sound itself, through the helper found beside it.
    Here,
    /// The same, and the window shows a clip's pictures itself as well
    /// (`viewer_core::Command::ShowsVideoHere`): GTK's, Qt's, and the C ABI's where its host draws
    /// pictures (RFC 0015 section 6; ADRs 3545, 3562, 3578).
    HereWithPictures,
    /// The window would play a sound itself and its helper is missing, so every clip is handed to
    /// this machine's player as a window that does not play is, and the sentence saying why is
    /// said beside each sound handed on.
    WithoutHelper(String),
}

impl Hearing {
    /// A window that plays sounds, asked whether its helper is there: [`super::helper::missing`],
    /// one look for a file and no process started.
    #[must_use]
    pub fn of_a_playing_window() -> Self {
        super::helper::missing().map_or(Self::Here, Self::WithoutHelper)
    }

    /// [`Self::of_a_playing_window`] for a window that also shows a clip's pictures itself.
    #[must_use]
    pub fn of_a_showing_window() -> Self {
        match Self::of_a_playing_window() {
            Self::Here => Self::HereWithPictures,
            missing => missing,
        }
    }

    /// What the window tells the core with `viewer_core::Command::PlaysHere`.
    #[must_use]
    pub const fn plays_here(&self) -> bool {
        matches!(self, Self::Here | Self::HereWithPictures)
    }

    /// What the window tells the core with `viewer_core::Command::ShowsVideoHere`.
    #[must_use]
    pub const fn shows_video(&self) -> bool {
        matches!(self, Self::HereWithPictures)
    }

    /// The sentence saying why a sound is handed on, where the window has no helper.
    #[must_use]
    pub fn missing(&self) -> Option<&str> {
        match self {
            Self::WithoutHelper(why) => Some(why),
            Self::HandsOn | Self::Here | Self::HereWithPictures => None,
        }
    }
}

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
    /// Who plays the window's sounds, as its launch settled it.
    hearing: Hearing,
    /// Whether the last sound was handed to this machine's player for want of the helper, which
    /// the title says until the next sound ([`HANDED_ON`]).
    handed_on: bool,
}

impl std::fmt::Debug for Sounds {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Sounds")
            .field("player", &self.player)
            .field("sounding", &self.sounding)
            .field("awaiting", &self.awaiting)
            .field("hearing", &self.hearing)
            .field("handed_on", &self.handed_on)
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
            hearing: Hearing::Here,
            handed_on: false,
        }
    }

    /// The same, played by `hearing`'s player: the answer the window's launch gave the core.
    #[must_use]
    pub fn heard_by(self, hearing: Hearing) -> Self {
        Self { hearing, ..self }
    }

    /// Who plays the window's sounds, as its launch settled it.
    #[must_use]
    pub const fn hearing(&self) -> &Hearing {
        &self.hearing
    }

    /// A sound was handed to this machine's player for want of the helper: the title says so until
    /// the next sound (ADR 3417).
    pub fn handed_on(&mut self) {
        self.handed_on = true;
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

    /// The sound last handed to the player, to be listened to by a clip's pictures (ADR 3555);
    /// `None` before the first.
    #[must_use]
    pub fn listening(&self) -> Option<super::Listening> {
        self.player.as_ref().map(Player::listening)
    }

    /// Plays `played` in place of whatever sounds. The first sound makes the player, with `wake`
    /// as what wakes the window's loop from the player's thread; later sounds do not call it.
    pub fn play(&mut self, played: Played, wake: impl FnOnce() -> Wake) {
        self.handed_on = false;
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
                // Whatever sounds sounds on, and the title that holds still holds.
                Heard::Unchanged(_) => {}
                Heard::Playing(_) | Heard::Holding(_) | Heard::Paused(_) | Heard::Refused(_) => {
                    self.awaiting = false;
                    self.sounding = Some(said.clone());
                }
            }
        }
        heard
    }

    /// The commands the core is owed about the rectangle a sound plays in, since the last call
    /// ([`Player::repainted`], ADR 3407).
    pub fn repainted(&mut self) -> Vec<viewer_core::Command> {
        self.player.as_mut().map_or_default(Player::repainted)
    }

    /// Starts the sound held at its start — Table 291's `/A` false, "the media shall be initially
    /// paused when activated" — or paused by a rendition action, and answers whether there was
    /// one: the window's resume control, which a toolkit gives the space bar while a sound is held
    /// (ADRs 3351, 3433).
    pub fn resume(&mut self) -> bool {
        match (&self.sounding, self.player.as_mut()) {
            (Some(Heard::Paused(_)), Some(player)) => {
                player.resume();
                self.awaiting = true;
                true
            }
            _ => false,
        }
    }

    /// Stops the sound that sounds, and answers whether there was one — Escape's question, which
    /// a window with nothing sounding leaves to the key table.
    pub fn stop(&mut self) -> bool {
        let sounding = self.in_hand();
        match (sounding, self.player.as_mut()) {
            (true, Some(player)) => {
                player.stop();
                true
            }
            _ => false,
        }
    }

    /// §12.6.4.14's stop, pause or resume of the rendition `document` associated with
    /// `annotation` ([`viewer_core::Event::OperateMedia`], ADR 3433): [`Player::operate`], whose
    /// answer [`Self::heard`] takes — the timer [`Self::interval`] sets is running, because a sound
    /// is in hand or awaited. A window with neither has nothing associated, and the clause's "there
    /// is no effect" is answered here, as the sentence to say.
    pub fn operate(
        &mut self,
        document: DocumentId,
        annotation: pdf_syntax::ObjectId,
        operation: viewer_core::MediaOperation,
    ) -> Option<String> {
        let out = self.awaiting || self.in_hand();
        match self.player.as_mut() {
            Some(player) if out => {
                player.operate(document, annotation, operation);
                None
            }
            _ => Some(super::unassociated(annotation, operation)),
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
            Some(Heard::Holding(_)) => Some(
                "playing a sound synchronously, nothing but Escape is taken until it ends, and \
                 Escape cancels it",
            ),
            Some(Heard::Paused(_)) => Some(PAUSED),
            Some(said) => Some(said.sentence()),
            None => self.handed_on.then_some(HANDED_ON),
        }
    }

    /// How long the window's timer waits before it looks at the player again: [`LOOK`] while a
    /// sound is being decoded or sounds, and `None` otherwise, so that a window whose sound has
    /// ended wakes for it never.
    #[must_use]
    pub fn interval(&self) -> Option<Duration> {
        (self.awaiting || self.in_hand()).then_some(LOOK)
    }

    /// Whether a synchronous sound holds the reader: ISO 32000-2 §12.6.4.9's Table 212
    /// `/Synchronous`, "allowing no further user interaction other than cancelling the sound,
    /// until the sound has been completely played". While it does, the window takes Escape — which
    /// [`Self::stop`] answers — and refuses every other key and click with [`HELD`] (ADR 3343).
    #[must_use]
    pub fn holds_the_reader(&self) -> bool {
        matches!(self.sounding, Some(Heard::Holding(_)))
    }

    /// Whether a sound is in hand that Escape stops: sounding, holding the reader, or held at its
    /// start.
    fn in_hand(&self) -> bool {
        matches!(
            self.sounding,
            Some(Heard::Playing(_) | Heard::Holding(_) | Heard::Paused(_))
        )
    }

    /// Whether the first sound has made the player.
    #[must_use]
    pub fn started(&self) -> bool {
        self.player.is_some()
    }
}
