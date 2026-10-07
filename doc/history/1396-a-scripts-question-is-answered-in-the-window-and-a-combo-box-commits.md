# 1396 — A script's question is answered in the window, and a combo box's commit is driven

UI slot of batch sixty-two. ADRs 1628, 1629; no ledger row, no question.

**Premise that did not hold.** At the start neither `app.alert` nor `app.response` existed (the
surface refused both); the window side was built against RFC 0008 section 4.2's shape and joined
to round 1395's runner through the records once ADR 1627 named its API (ADR 1628 section 4).

**Built (ADR 1628).** `viewer_core::ScriptQuestion`/`ScriptAnswer`, `Event::ScriptAsking` and
`Command::AnswerScript`, on the confined wire (event 23, command 40) and the C ABI (kind 23);
`ScriptAsks` and `ScriptRunners::runner_that_asks`, polled after every command, with
`ViewState::apply_resumed` after every command too. The window's thread never waits: the script is
held in its worker. `quorra`'s card, GTK's and Qt's dialogues titled with the document that asks,
the script's buttons (affirming last), a response's entry with its default, a close answered
Cancel/No/OK/null, never Yes; `quorra-confined` refuses by name. `viewer_host::script_asks` holds
the shared words; `Workers` hands the worker as the desk and `Warning` forwards the resumed runs.

**Built (ADR 1629).** `mode_on` presses a toggle until the window's last line about the mode says
on; `38-located` uses it. `45-combo-commit` drove the editable combo box in all four windows and
found two defects, both fixed: GTK gave the keyboard to the composed `GtkBox` (now its entry), and
`quorra-confined` would not type into one. `46-script-alert` and `47-script-response` drive the
question in all four windows.

**Q271.** The measure tool's hover on a projected map is Q271's refusal, unchanged.

**Left.** A question withdrawn by the worker's wait leaves its card up; the late answer is dropped.

**Measured.** The new steps, each window: `45-combo-commit` 6.5–6.7 s; `46-script-alert` 0.5–1.1 s;
`47-script-response` 4.9–5.1 s. Threads under the agent's user at the first heavy run: 196.
The whole drive behind the lock: 170 works / 0 wrong / 3 not offered, 585.6 s over the time
column, 589 s under the bound (939 s with the lock's queue); `38-located` works in all four.

**Gates.** `rustfmt --check --edition 2024`, every touched `.rs` file by name: 0. Clippy pedantic
on the seven crates: 0 warnings of theirs, exit 0 without `-D warnings`; with it, exit 101 on
siblings' in-flight `pdf-script-worker`, `pdf-render`, `pdf-model`. `cargo nextest run` on the
seven: 873 of 874, the one failing `the_two_deferred_producers_reach_the_raster_arm_by_name`, a
deferred image now crossing as a list (siblings' `pdf-model` content edits). `cargo test -p
conformance`: exit 0, 396 passed. The drive: exit 0, 170 works.
