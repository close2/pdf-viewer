# 1702 — A script's timer is counted in the window's ticks, and its sound is the host's

Status: accepted and **built**. Session 1433. Builds on ADR 1689 section 4 (the ranking, and the
shape it named for a timer), ADR 1640 (a script's request is an edit a host carries out), ADR 0473
(`viewer_host::Clock`), ADR 1628 (a host's act is an event out), RFC 0008 section 4.2 and A193
answer 10 (timers admitted, bounded to the document).
Context: Adobe's *JavaScript for Acrobat API Reference*, "app methods" (`beep`, `setInterval`,
`setTimeOut`, `clearInterval`, `clearTimeOut`) and "Event type/name combinations", at
`adobe/dc-acrobat-sdk-docs` commit `ab3b42a7`, cited and never quoted.
Code: `crates/pdf-script/src/engine/members.rs` (`set_timer`, `clear_timer`, `beep`),
`engine/mod.rs` (`Table::next_timer`, `timer_key`), `wire.rs` (site 6, edits 7–9, version 8 shared
with ADR 1700), `surface.rs` (five members fewer); `crates/pdf-model/src/view/script_timers.rs`,
`script_model.rs` (`ScriptSite::Timer`, `ScriptEdit::Timer`/`ClearTimer`/`Beep`, `Sound`),
`scripts.rs` (the three edits applied); `crates/viewer-core/src/viewer.rs` (`run_timers`,
`ask_for_sounds`, `Query::TimerDue`), `event.rs` (`Event::Beep`); `crates/viewer-host/src/script_timers.rs`
(`Ticker`, `played`, `unplayed`); the four windows; `viewer-confined`'s wire; `viewer-ffi`.
Tests: `crates/pdf-script/tests/timers.rs`, `crates/viewer-core/tests/script_timers.rs`,
`pdf-model`'s `script_timers` unit tests, `viewer-host`'s `script_timers` unit tests, the confined
wire's query list, `tools/drive-windows.sh` steps 60 and 61.

## 1. The premise, as the tree stood

The brief named four senders of `Command::Tick` — `quorra`'s presentation and first frame, Qt's
pump twice. **Two more existed**: `viewer-gtk`'s presentation clock and its question wake
(`host.rs`, ADR 1643's tick of no time), and `viewer-ffi`'s `quorra_tick`. Every one of them is a
presentation's clock or a question's wake; none ran while a person simply read. The script side had
no timer table at all: the five members were refusers.

## 2. The timer is the view state's, counted in ticks

A realm records `ScriptEdit::Timer { id, script, period, repeat }` and `ClearTimer { id }`, all or
nothing as every edit; the view state holds the table (`script_timers.rs`), counts each timer down
in the milliseconds a `Tick` carries, and runs a due expression as `ScriptSite::Timer` through the
runner, its edits applied as any script's. Choices, each documented under principle 5: a period is
at least 16 ms; a late tick runs a timer once, never once per missed period; a timer lasts until it
is cleared or its document closes (the reference's garbage-collection warning describes its engine,
not a rule); at most 64 per document; the interval object is an ordinary object holding its number
under a realm symbol, so a script's own properties on it — the reference's example keeps a count —
cannot meet it; `clear` of anything else throws a `TypeError`, which the reference's Stop button
guards with `try`; a timer's run is `event.type` `App`, `event.name` `Timer` — the reference lists
no timer event, so the name is one no listed branch matches — with the document as `this` and
`event.target`.

**A tick no longer turns a page outside a presentation.** The core advanced §12.4.4.1's `/Dur` on
every tick, which was safe only while presentations alone ticked; the clause makes it presentation
timing ("how to display that page in presentation mode"), so `Viewer::tick` runs the timers and
then returns unless `Command::Present(On)` is in force. All three windows send that command when a
presentation starts, and four `headless` tests that ticked without it now send it.

## 3. A window asks, and ticks only while a timer is held

`Query::TimerDue` answers the soonest timer's milliseconds over every open document, `None` where
none is held. A question rather than an event because it is a fact about now that any command
moves, `Query::Dirty`'s shape. `viewer_host::script_timers::Ticker` is the shared arithmetic: it
starts counting when a timer is first held, forgets when none is, and a tick carries the whole
milliseconds that really passed, the fraction kept for the next — the drive's first GTK run told the
core 299 of a 300 ms period and then 0 for ever, 184 173 ticks in ten seconds — and a wait is
rounded up to a whole millisecond. **One clock per window**: while a presentation's `Clock` ticks the core, the ticker
stands down, or the core would be told the same time twice; while §12.6.4.15's one effect is
drawn the timers wait it out, at most Table 164's `/D`, in all three windows alike. GTK arms a `glib` one-shot in
`pump_presentation`; Qt's `presentation_wait` answers the timer's wait when nothing presents, so its
existing `QTimer` serves; `quorra` sends the tick in `about_to_wait` and waits until the next. A
window whose documents set no timer arms nothing, and the open scripts run after the first present,
so nothing reaches the launch path (`CLAUDE.md` principle 2).

## 4. A sound is an event, refused by name where there is none

`app.beep(nType)` records `ScriptEdit::Beep` with one of the reference's five sounds (a number it
does not list is the default, as an absent one is); the view state holds them and the viewer sends
`Event::Beep` after the command. `gdk::Display::beep` and `QApplication::beep` play one system sound
for all five, which the reference allows of two of its three platforms; winit has no sound, so
`quorra` prints that none was played. The C ABI names `QUORRA_EVENT_BEEP` and sends it to no caller:
a session supplies no runner.

## 5. Not done here

A leader painted over the toolkits' text (ADR 1690 section 4) was not reached.
