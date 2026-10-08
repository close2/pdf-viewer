//! The timers a document's scripts set, run on the host's ticks, and the sounds they ask for
//! (ADR 1702).
//!
//! Adobe's *JavaScript for Acrobat API Reference*, "app methods", cited and never quoted:
//! `setInterval` runs an expression every time its period elapses and `setTimeOut` once after it,
//! each answering an object that `clearInterval` or `clearTimeOut` stops; a timer ends with its
//! document; `beep` asks the system for one of five sounds. None of it is ISO 32000-2's, which
//! hands the object model to another standard, so every rule below is a documented choice under
//! principle 5 rather than a derivation.
//!
//! **A view state has no clock**, and neither has the viewer it sits in (`doc/ui-boundary.md`'s
//! rule 3). So a timer is counted down in the milliseconds a host's `Tick` says passed, and a host
//! asks [`ViewState::timer_due`] how long it may sleep before the next one is owed — which is also
//! what keeps a still window from waking at all: a document with no timer answers `None`, and a
//! host with nothing to tick sends nothing (`CLAUDE.md` principle 2).
//!
//! What is chosen rather than read:
//!
//! - **A period is at least [`MIN_PERIOD`]**, a sixtieth of a second: the reference states no
//!   floor, and a timer of 0 would ask a host to wake without pause. A frame is the finest change a
//!   person sees.
//! - **A late tick runs a timer once, never once per period it missed.** A host that slept through
//!   three periods of an interval runs it once and starts its next period then: the expression is
//!   a script's animation step, and replaying missed steps back to back draws nothing a person
//!   sees.
//! - **A timer runs until it is cleared or its document closes.** The reference warns that an
//!   object not held in a variable may be collected and its clock stop; that is its engine's
//!   collector rather than a rule, and a timer here does not depend on one.
//! - **At most [`MAX_TIMERS`] per document**, each expression held to the runner's script bound; a
//!   timer past either is refused by name in the run's notes.

use std::collections::BTreeMap;

use pdf_syntax::Document;

use super::script_model::{ScriptSite, Sound};
use super::scripts::{MAX_SCRIPT_BYTES, ScriptEvent};
use super::{ScriptsRan, ViewState};

/// Shortest period a timer counts, in milliseconds: a sixtieth of a second.
pub const MIN_PERIOD: u32 = 16;

/// Most timers one document holds at once.
///
/// The census's scripts set one or two — a clock, a marquee and the timeout that stops it — and a
/// document that sets more is a loop; each due timer is a script run per tick.
pub const MAX_TIMERS: usize = 64;

/// Most sounds a view state holds until a host takes them.
const MAX_BEEPS: usize = 16;

/// One timer a script set.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Timer {
    /// The expression, as text.
    script: String,
    /// The period, in milliseconds, at least [`MIN_PERIOD`].
    period: u32,
    /// Milliseconds of ticks until it is next due.
    left: u32,
    /// Whether it runs again after it has run.
    repeat: bool,
}

/// The timers and the sounds scripts asked for, held beside the edit log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Timers {
    /// Each timer by the realm's number for it, which is also the order they were set in.
    held: BTreeMap<u32, Timer>,
    /// Each sound asked for and not yet taken by a host, in order.
    beeps: Vec<Sound>,
}

impl Timers {
    /// Holds a timer, replacing one of the same number; answers the sentence that refuses it where
    /// it is past a bound.
    pub(super) fn set(
        &mut self,
        id: u32,
        script: &str,
        period: u32,
        repeat: bool,
    ) -> Result<(), String> {
        if script.len() > MAX_SCRIPT_BYTES {
            return Err(format!(
                "a script set a timer whose expression is {} bytes, past the {MAX_SCRIPT_BYTES} \
                 a script may be; it was not set (ADR 1702)",
                script.len()
            ));
        }
        if !self.held.contains_key(&id) && self.held.len() >= MAX_TIMERS {
            return Err(format!(
                "a script set a timer while its document held {MAX_TIMERS}, the most one may; it \
                 was not set (ADR 1702)"
            ));
        }
        let period = period.max(MIN_PERIOD);
        self.held.insert(
            id,
            Timer {
                script: script.to_owned(),
                period,
                left: period,
                repeat,
            },
        );
        Ok(())
    }

    /// Stops the timer of that number, where there is one.
    pub(super) fn clear(&mut self, id: u32) {
        self.held.remove(&id);
    }

    /// Holds a sound for a host, dropping it past [`MAX_BEEPS`] untaken.
    pub(super) fn beep(&mut self, sound: Sound) {
        if self.beeps.len() < MAX_BEEPS {
            self.beeps.push(sound);
        }
    }

    /// Counts every timer down by `millis` and answers each one now due, in the order they were
    /// set: an interval starts its next period, a timeout is let go.
    fn elapse(&mut self, millis: u32) -> Vec<String> {
        let mut due = Vec::new();
        self.held.retain(|_, timer| {
            timer.left = timer.left.saturating_sub(millis);
            if timer.left > 0 {
                return true;
            }
            due.push(timer.script.clone());
            timer.left = timer.period;
            timer.repeat
        });
        due
    }
}

impl ViewState {
    /// Milliseconds of ticks until a timer a script set is next due, or `None` where none is set.
    ///
    /// What a host asks after every command to decide whether to tick at all: `None` is a window
    /// that sends no `Tick` and arms no timer, and `Some(0)` is one owed now (ADR 1702).
    #[must_use]
    pub fn timer_due(&self) -> Option<u32> {
        self.scripting
            .timers
            .held
            .values()
            .map(|timer| timer.left)
            .min()
    }

    /// Counts the timers scripts set down by `millis` of a host's ticks and runs every one now due,
    /// at page `page`, the page the host shows: answers how many scripts were handed to the runner
    /// and whether what they edited can have changed what a page draws, as the event and page
    /// runners answer (ADR 1762), so a host draws the page again only where one did.
    ///
    /// Each due expression runs as a script of its own, held to the runner's budget, with what it
    /// edits applied as any script's edits are; an interval due again runs again at a later tick,
    /// never twice in one. With no runner supplied a timer cannot have been set, so this does
    /// nothing.
    pub fn run_timers(&mut self, document: &Document, millis: u32, page: usize) -> ScriptsRan {
        if self.runner.0.is_none() || self.scripting.timers.held.is_empty() {
            return ScriptsRan::default();
        }
        let due = self.scripting.timers.elapse(millis);
        if due.is_empty() {
            return ScriptsRan::default();
        }
        let table = super::widgets_by_field_name(document);
        let (mut handed, mut changed, mut calculate, mut drawn) = (0_usize, false, false, false);
        for script in &due {
            let event = ScriptEvent {
                script,
                page,
                ..ScriptEvent::at(ScriptSite::Timer, "")
            };
            if let Some((result, applied)) = self.run_event(document, &table, event) {
                handed = handed.saturating_add(1);
                changed |= applied.values;
                calculate |= applied.calculate;
                drawn |= applied.drawn;
                self.report_each("a timer's expression", result.report);
            }
        }
        // Table 224's order is walked "when the value of any field changes", and a timer's
        // change is a change, as it is at every other site.
        if changed || calculate {
            self.recalculate_scripts(document, &table, "");
        }
        self.refresh_formatted(document, &table);
        ScriptsRan {
            handed,
            changed: drawn,
        }
    }

    /// Every sound a script's `app.beep` asked for since a host last took them, in order (ADR
    /// 1702).
    ///
    /// The sound is the host's to play, as the focus and the page are the host's to move; a host
    /// with no sound to play says so rather than dropping it.
    pub fn take_beeps(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.scripting.timers.beeps)
    }
}

#[cfg(test)]
mod tests {
    use super::{MIN_PERIOD, Timers};

    /// A timeout runs once and is let go; an interval runs every period and starts its next one
    /// when it runs, never catching up on periods a late tick missed.
    #[test]
    fn a_timeout_runs_once_and_an_interval_every_period() {
        let mut timers = Timers::default();
        timers
            .set(1, "once()", 500, false)
            .expect("under the bounds");
        timers
            .set(2, "again()", 200, true)
            .expect("under the bounds");
        assert_eq!(timers.elapse(100), Vec::<String>::new());
        assert_eq!(timers.elapse(100), vec!["again()".to_owned()]);
        assert_eq!(
            timers.elapse(900),
            vec!["once()".to_owned(), "again()".to_owned()],
            "a late tick runs each due timer once"
        );
        assert_eq!(timers.held.len(), 1, "the timeout is gone");
        timers.clear(2);
        assert!(timers.held.is_empty());
    }

    /// A period of nothing is a sixtieth of a second, so a host is never asked to wake without
    /// pause.
    #[test]
    fn a_period_is_at_least_a_frame() {
        let mut timers = Timers::default();
        timers.set(1, "spin()", 0, true).expect("under the bounds");
        assert_eq!(
            timers.held.get(&1).map(|timer| timer.left),
            Some(MIN_PERIOD)
        );
    }

    /// A document holds at most [`super::MAX_TIMERS`]; the next is refused by name, and setting an
    /// existing number again replaces it.
    #[test]
    fn a_timer_past_the_bound_is_refused_by_name() {
        let mut timers = Timers::default();
        for id in 0..u32::try_from(super::MAX_TIMERS).expect("a small number") {
            timers
                .set(id, "tick()", 100, true)
                .expect("under the bound");
        }
        let refused = timers.set(999, "tick()", 100, true);
        assert!(refused.is_err_and(|sentence| sentence.contains("the most one may")));
        assert!(timers.set(0, "other()", 100, true).is_ok(), "a replacement");
    }
}
