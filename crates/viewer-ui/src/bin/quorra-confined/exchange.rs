//! A command the worker is still performing past `WARN`, and what the window does meanwhile.
//!
//! **Every exchange with the worker blocks the thread that makes it for as long as the document
//! takes** — `viewer_confined::Confined` says so of itself, and means it: a page that crosses as
//! pixels is drawn inside the command that asked for it, so on the owner's drawing the window's own
//! thread sat in one `Command::Resize` for the whole of the page's rasterisation and answered no
//! key, no repaint and no Escape until it came back. The other three windows hold their viewer in
//! process and draw on a thread of their own, so their loop was never inside a draw; this one's
//! viewer is a process, and its loop was inside every one.
//!
//! **The shape is ADR 1811's, one level further in** (ADR 1827). Each command goes to the worker
//! on a thread of its own — the [`viewer_host::Loading`] the open already uses — and the window
//! waits for it no longer than `viewer_host::drawing::WARN`. Every command a launch gate or a drive
//! sends is answered inside that, and is then handled exactly where and as it was before. A command
//! still out at `WARN` is left to its thread: the title says what the worker is doing, the window
//! answers its loop, and `about_to_wait` takes the answer in on the tick that finds it ready.
//!
//! **What the reader does meanwhile is held, in order, and done once the worker answers.** The
//! worker is one viewer performing one command at a time (`viewer-core`'s rule 4), so a key cannot
//! reach it sooner; what the window can do is take the key at once, say so, and not lose it. Three
//! things are not held, because each is about the wait itself: Escape, which ends the worker with a
//! kill it cannot decline (ADR 0241); `q`, which leaves; and the window's close. What a landing
//! causes — the events of the command that came back, and the commands they ask for — goes in front
//! of what the reader did while it was out, because at the commit before this module that is the
//! order it happened in.
//!
//! **Why a key is held as a key and not as the command it would become**: what a key means can
//! depend on a question to the worker — a character typed into a field is sent as the field's
//! whole value, read back first (`typing`'s rule) — and a question asked of a worker that is busy
//! has no answer. A key is therefore decided when it is done, under the modifiers that were held
//! when it was pressed.

use std::time::{Duration, Instant};

use viewer_confined::{Confined, ConfinedError};
use viewer_core::{Command, Event};
use viewer_host::Loading;
use viewer_host::drawing::WARN;
use viewer_host::trace::Topic;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::{Host, brief};

/// A command out with the worker on a thread of its own: the worker comes back with what it
/// answered.
pub(crate) type Exchanging = Loading<(Confined, Result<Vec<Event>, ConfinedError>)>;

/// What the window took while the worker was busy, to be done once it answers.
#[derive(Debug)]
pub(crate) enum Held {
    /// A command for the worker: a wheel's scroll, a resize, what a landing asked for.
    Command {
        /// The command.
        command: Command,
        /// When the window took it.
        at: Instant,
    },
    /// A key, decided when it is done, under the modifiers held when it was pressed.
    Key {
        /// The key.
        key: Key,
        /// Shift and Control as they were: Shift turns §12.5.1's Tab round.
        modifiers: ModifiersState,
        /// When the window took it.
        at: Instant,
    },
    /// A press that is one of §12.9's points, where the pointer was.
    Point {
        /// The pointer, in the window's device pixels.
        cursor: (f32, f32),
        /// When the window took it.
        at: Instant,
    },
    /// The question a Tab asks after its move — which field the keyboard is in now — owed
    /// because the move went out with the worker.
    Focus,
}

/// What the title says about a command the worker has outlasted `WARN` on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Waiting {
    /// The worker is reading a page's contents, with the share read so far.
    Reading(String),
    /// The worker has read what it needed and is drawing.
    Drawing(String),
}

impl Waiting {
    /// What the title shows.
    fn said(&self) -> &str {
        match self {
            Self::Reading(said) | Self::Drawing(said) => said,
        }
    }

    /// Whether `other` is the same kind of wait, which is said once on the way in.
    fn same_kind(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Reading(_), Self::Reading(_)) | (Self::Drawing(_), Self::Drawing(_))
        )
    }
}

impl Host {
    /// Whether the worker is performing something the reader's next act must wait behind: the
    /// open, or a command still out past `WARN`.
    pub(crate) fn busy(&self) -> bool {
        self.opening.is_some() || self.exchanging.is_some()
    }

    /// Whether what the reader does now must be held: the worker is busy, or something taken
    /// while it was is still owed and would otherwise be overtaken.
    pub(crate) fn must_hold(&self) -> bool {
        self.busy() || !self.held.is_empty()
    }

    /// Takes something the reader did while the worker was busy, and says so at once — the line
    /// that tells a trace the loop ran.
    pub(crate) fn hold(&mut self, held: Held) {
        if self.trace.on(Topic::Events) {
            match &held {
                Held::Command { command, .. } => self.trace.say(
                    Topic::Events,
                    format_args!("-> held behind the worker: {}", brief(command)),
                ),
                Held::Key { key, .. } => self.trace.say(
                    Topic::Events,
                    format_args!("-> held behind the worker: key {}", brief(key)),
                ),
                Held::Point { .. } => self.trace.say(
                    Topic::Events,
                    format_args!("-> held behind the worker: a point"),
                ),
                Held::Focus => {}
            }
        }
        // A resize states the window's size rather than a step, so only the newest one is owed.
        if let Held::Command {
            command: Command::Resize { .. },
            ..
        } = held
        {
            self.held.retain(|earlier| {
                !matches!(
                    earlier,
                    Held::Command {
                        command: Command::Resize { .. },
                        ..
                    }
                )
            });
        }
        self.held.push_back(held);
    }

    /// Whether a key pressed while the worker is busy acts at once rather than waiting: the
    /// three that are about the wait itself.
    ///
    /// Escape is the abort and `q` the quit only on the page — the card's Escape declines the
    /// prompt, and a field's Escape and `q` are the field's — so with either up they are held
    /// like any other key.
    pub(crate) fn acts_at_once(&self, key: &Key) -> bool {
        if self.password.shown || self.typing.is_some() {
            return false;
        }
        match key {
            Key::Named(NamedKey::Escape) => true,
            Key::Character(text) => text.as_str() == "q",
            _ => false,
        }
    }

    /// Sends one command to the worker on a thread of its own, and waits for it no longer than
    /// `WARN`.
    pub(crate) fn send(&mut self, command: Command) {
        if self.trace.on(Topic::Events) {
            match self.held_for.take() {
                Some(held) => self.trace.say(
                    Topic::Events,
                    format_args!("-> {} (held {:.2} s)", brief(&command), held.as_secs_f64()),
                ),
                None => self
                    .trace
                    .say(Topic::Events, format_args!("-> {}", brief(&command))),
            }
        }
        let Some(mut confined) = self.confined.take() else {
            return;
        };
        self.sent = Some(Instant::now());
        let exchanging: Exchanging = Loading::start_reporting(move |progress| {
            // The worker relays how far a page has got inside the exchange, whatever the command
            // (ADR 1811); this thread is the one waiting on it now.
            confined.report_progress_to(progress);
            let answered = confined.handle(&command);
            (confined, answered)
        });
        if exchanging.wait() {
            self.answered(exchanging);
            return;
        }
        self.trace.say(
            Topic::Events,
            format_args!("the worker is still performing it past the warning"),
        );
        self.exchanging = Some(exchanging);
        self.mind_the_exchange();
    }

    /// Takes in what the worker answered: its events, then the frame the view now shows.
    pub(crate) fn answered(&mut self, exchanging: Exchanging) {
        if self.waiting.take().is_some() {
            self.retitle();
        }
        let Ok((confined, answered)) = exchanging.join() else {
            self.stop("the thread performing a command in the worker panicked".to_owned());
            return;
        };
        if self.stopped.is_some() {
            // The reader ended this worker while the command was out. Dropping the handle reaps
            // the process, which `stop` explains.
            drop(confined);
            return;
        }
        self.confined = Some(confined);
        self.ahead_of_the_held(|host| match answered {
            Ok(events) => {
                for event in events {
                    host.event(event);
                }
                host.pull_frame();
            }
            Err(problem) => host.died(&problem),
        });
    }

    /// Runs `work` with what the reader did meanwhile set aside, so that whatever `work` asks of
    /// the worker goes in front of it — a landing's own consequences, an open's first size, a
    /// restart's view.
    pub(crate) fn ahead_of_the_held(&mut self, work: impl FnOnce(&mut Self)) {
        let later = std::mem::take(&mut self.held);
        work(self);
        if self.stopped.is_some() {
            self.held.clear();
            return;
        }
        self.held.extend(later);
    }

    /// Does what was held, in order, until the worker is busy again or `WARN` of this thread has
    /// gone; what is left waits for the next turn of the loop.
    pub(crate) fn release_held(&mut self) {
        let began = Instant::now();
        let mut owed = std::mem::take(&mut self.held);
        while !self.busy() && self.stopped.is_none() && began.elapsed() < WARN {
            let Some(held) = owed.pop_front() else {
                break;
            };
            match held {
                Held::Command { command, at } => {
                    self.held_for = Some(at.elapsed());
                    self.send(command);
                }
                Held::Key { key, modifiers, at } => {
                    self.held_for = Some(at.elapsed());
                    let live = std::mem::replace(&mut self.modifiers, modifiers);
                    self.press(&key);
                    self.modifiers = live;
                }
                Held::Point { cursor, at } => {
                    self.held_for = Some(at.elapsed());
                    let live = std::mem::replace(&mut self.cursor, cursor);
                    self.measure_at();
                    self.cursor = live;
                }
                Held::Focus => self.ask_the_focus(),
            }
            // A held act that sent nothing leaves no annotation behind for the next one.
            self.held_for = None;
        }
        if self.stopped.is_some() {
            self.held.clear();
            return;
        }
        // What the acts just done held in their turn comes before what was still owed, which is
        // the order they would have run in had the worker never been busy.
        self.held.extend(owed);
    }

    /// Looks at the command still out, and takes its answer in if it has come.
    pub(crate) fn mind_the_exchange(&mut self) {
        let Some(exchanging) = self.exchanging.as_ref() else {
            return;
        };
        if exchanging.ended() {
            if let Some(exchanging) = self.exchanging.take() {
                self.answered(exchanging);
            }
            return;
        }
        let Some(reading) = exchanging.showing() else {
            return;
        };
        let waiting = match reading.fraction() {
            Some(fraction) if fraction < 1.0 => {
                let sentence = viewer_host::loading::reading(reading);
                Waiting::Reading(match viewer_host::loading::bar(reading, 20) {
                    Some(bar) => format!("{bar} {sentence}"),
                    None => sentence,
                })
            }
            // The unit is the worker's command, which may be a whole view rather than one page.
            _ => Waiting::Drawing(viewer_host::status::still_drawing(None)),
        };
        if self.waiting.as_ref() == Some(&waiting) {
            return;
        }
        if !self
            .waiting
            .as_ref()
            .is_some_and(|was| was.same_kind(&waiting))
        {
            let said = match &waiting {
                Waiting::Reading(_) => viewer_host::loading::reading(reading),
                Waiting::Drawing(said) => said.clone(),
            };
            println!("note: {said}");
        }
        self.waiting = Some(waiting);
        self.retitle();
    }

    /// What the title says about the command still out, or nothing.
    pub(crate) fn waiting_said(&self) -> Option<&str> {
        self.waiting.as_ref().map(Waiting::said)
    }

    /// How long the loop may sleep before it must look at the worker again, where it must.
    pub(crate) fn exchange_tick(&self) -> Option<Duration> {
        if !self.busy() && !self.held.is_empty() {
            return Some(viewer_host::drawing::POLL);
        }
        self.exchanging
            .is_some()
            .then_some(viewer_host::loading::TICK)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::mpsc::channel;
    use std::time::Instant;

    use viewer_core::{Command, PageTarget};
    use winit::keyboard::{Key, NamedKey};

    use super::Held;
    use crate::Host;

    /// A host with no window and no worker, as `tests::a_host` builds it.
    fn a_host() -> Host {
        Host::new(PathBuf::from("held.pdf"), 0, true, false, Instant::now())
    }

    /// An open that has not finished until the sender is dropped: the worker busy, as on the
    /// owner's drawing, without a worker.
    fn opening(host: &mut Host) -> std::sync::mpsc::Sender<()> {
        let (tell, told) = channel::<()>();
        host.opening = Some(viewer_host::Loading::start(move || {
            let _ = told.recv();
            Err("no worker in a test".to_owned())
        }));
        tell
    }

    /// A key pressed while the worker is busy is taken and kept, in order — the behaviour this
    /// replaces is a window whose loop was inside the exchange and took nothing.
    #[test]
    fn a_key_pressed_while_the_worker_is_busy_is_held_in_order() {
        let mut host = a_host();
        let _open = opening(&mut host);
        host.press(&Key::Named(NamedKey::ArrowRight));
        host.press(&Key::Named(NamedKey::ArrowLeft));
        host.dispatch(Command::GoTo(PageTarget::Last));
        let held: Vec<String> = host.held.iter().map(|held| format!("{held:?}")).collect();
        assert_eq!(held.len(), 3, "all three were taken: {held:?}");
        assert!(held[0].contains("ArrowRight") && held[1].contains("ArrowLeft"));
        assert!(held[2].contains("Last"));
        assert!(host.stopped.is_none());
    }

    /// Escape while the worker is busy is the abort at once, and what was held goes with the
    /// worker: nothing would be left to do it.
    #[test]
    fn escape_while_the_worker_is_busy_aborts_at_once() {
        let mut host = a_host();
        let _open = opening(&mut host);
        host.press(&Key::Named(NamedKey::ArrowRight));
        host.press(&Key::Named(NamedKey::Escape));
        assert!(
            host.stopped
                .as_deref()
                .is_some_and(|said| said.contains("aborted")),
            "the abort did not wait, was {:?}",
            host.stopped
        );
        assert!(host.held.is_empty(), "nothing is owed to an ended worker");
    }

    /// `q` while the worker is busy leaves at once.
    #[test]
    fn q_while_the_worker_is_busy_leaves_at_once() {
        let mut host = a_host();
        let _open = opening(&mut host);
        host.press(&Key::Character("q".into()));
        assert!(host.leaving, "the quit did not wait");
        assert!(host.held.is_empty());
    }

    /// A resize states a size, so only the newest is owed, and it keeps its place behind keys
    /// taken before it.
    #[test]
    fn only_the_newest_resize_is_held() {
        let mut host = a_host();
        let _open = opening(&mut host);
        for width in [100, 200, 300] {
            host.press(&Key::Named(NamedKey::ArrowRight));
            host.dispatch(Command::Resize {
                width,
                height: 10,
                scale: 1.0,
            });
        }
        let resizes: Vec<&Held> = host
            .held
            .iter()
            .filter(|held| {
                matches!(
                    held,
                    Held::Command {
                        command: Command::Resize { .. },
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(resizes.len(), 1, "{:?}", host.held);
        assert!(format!("{:?}", resizes[0]).contains("width: 300"));
        assert_eq!(host.held.len(), 4, "three keys and one size");
    }

    /// What a landing asks for goes in front of what the reader did meanwhile.
    #[test]
    fn a_landings_own_commands_go_in_front_of_the_held() {
        let mut host = a_host();
        let _open = opening(&mut host);
        host.press(&Key::Named(NamedKey::ArrowRight));
        host.ahead_of_the_held(|host| host.dispatch(Command::GoTo(PageTarget::First)));
        let held: Vec<String> = host.held.iter().map(|held| format!("{held:?}")).collect();
        assert!(held[0].contains("First"), "{held:?}");
        assert!(held[1].contains("ArrowRight"), "{held:?}");
    }
}
