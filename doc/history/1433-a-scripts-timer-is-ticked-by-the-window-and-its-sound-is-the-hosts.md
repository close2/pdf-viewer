# 1433 — A script's timer is ticked by the window, and its sound is the host's

Slot 2 of batch sixty-eight, 2026-10-08, a host round. ADR 1702. No ledger row moved: the members
are Adobe's reference, not a clause; §12.4.4.1's row stays `implemented`, its condition now kept by
the core. No question written.

**Premise.** Held in part. Four senders of `Command::Tick` were named; two more existed, GTK's
presentation clock and question wake, and `quorra_tick`. All were a presentation or a question; the
script side had no timer table. And the core advanced §12.4.4.1's `/Dur` on every tick, safe only
while presentations alone ticked.

**Timers (ADR 1702).** `app.setInterval`, `setTimeOut`, `clearInterval`, `clearTimeOut` record
edits (wire 8, shared with ADR 1700); the view state counts them down in ticks and runs a due one as
`ScriptSite::Timer`. `Query::TimerDue` tells a window how long it may sleep, `None` when no
document holds a timer, so a still window arms nothing. `viewer_host::script_timers::Ticker` keeps
one clock per window. A tick turns a page by `/Dur` only under `Command::Present(On)`. Named hunk
for slot 1: `pdf-script` `members.rs` (`set_timer`, `clear_timer`, `beep`), `mod.rs`
(`Table::next_timer`, `timer_key`), `wire.rs` (site 6, edits 7–9), `surface.rs`, `bridge.rs`
(target); `pdf-model` `script_model.rs`, `scripts.rs` (three arms, `Scripting::timers`), `view.rs`.

**Beep.** `Event::Beep`: GDK and Qt play their system sound; `quorra` says none was played;
`QUORRA_EVENT_KIND_COUNT` 25 → 26, `quorra_timer_due` added. ADR 1690 section 4's leader not reached.

**Gates.** `rustfmt --check` on my 35 `.rs` files: exit 0. `RUSTFLAGS="-D warnings" cargo clippy
--all-targets` on `pdf-model`, `pdf-script` (with and without `engine`), `pdf-script-worker`, and
the seven host crates: exit 0. nextest over those: 1 043 passed, 5 failed (four ABI counts, then
fixed; `growth_past_the_ceiling` once, 17 of 17 alone); after the last edits 1 029 passed, 0
failed; `pdf-model` without `corpus` 1 987 passed. Planted (trap 13): timers not run, 2 of 4
`script_timers` tests fail; the presentation guard lifted, the `/Dur` test fails; restored by
patch. `cargo test -p conformance`: 412 passed, exit 0. Behind the lock, the release build and
`tools/drive-windows.sh`: first run exit 0, 213 works, 1 wrong (GTK's timer: a tick dropped its
sub-millisecond rest, then 184 173 ticks of 0 ms; fixed and tested). Second run exit 0, 214 works,
0 wrong, 0 to look at, 6 not offered, 774 s; step 60's photograph looked at (trap 1). `launch_path`
(`--clock`): exit 101, `open_kinstructions` 3.3 k and 4.0 k past the band on
`xfa_filled_imm1344e.pdf` and `opt_demo.pdf`; the same with my core hunks off (1 175.334 against
1 175.297), so not this change. Duration 5 900 s.
