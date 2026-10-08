# 1771 — A page is interpreted again after its scripts only where they changed what it draws, and a script reads the window's keys and asks of its console

Session 1464. Status: **accepted** and **built**. The host half of ADR 1762's three answers; takes back the cost
ADR 1752 section 3 named.
Code: `crates/viewer-core/src/viewer.rs` (`raise`, `page_scripts`, `Command::Keys`,
`ask_for_sounds`), `crates/viewer-core/src/interact.rs` (`activate`), `command.rs` (`Keys`),
`event.rs` (`Console`), `viewer_host::script_timers::console`, the wire (command kind 41, event
kind 27), `viewer-ffi` (`EventKind::Console`), each window's keys (`quorra`'s `ModifiersChanged`,
GTK's `tell_keys`, Qt's `MainWindow::keys`), `tools/drive-windows.sh` (`drive-triggers.pdf`'s
`Quiet`, step 68's count; `drive-keys.pdf`, step 71).
Tests: `crates/viewer-core/tests/script_triggers.rs`
(`a_script_that_changes_nothing_drawn_does_not_interpret_the_page_again`,
`a_scripts_console_requests_reach_the_host_in_order`, `a_script_reads_the_keys_the_host_last_sent`).

## Decision

**The runner's two answers are acted on apart** (`pdf_model::view::ScriptsRan`, ADR 1762): `handed`
decides whether the chain's `JavaScript` refusals are left unsaid (ADR 1752's `handed_over`), and
`changed` decides whether `Open::stale` throws the page's interpretation away. A widget's six
pointer and focus events, a link's click and a turn's page events all follow it. A `/E` that logs
or asks a question changes nothing a page draws, so a cursor crossing it no longer re-interprets the
page it is on.

The timers' site keeps the count: `run_timers` answers how many it handed over, and a page with a
due timer is interpreted again whatever its script did, until that site answers the same way.

## Consequences

Drive step 68 enters a field whose `/E` only logs three times before its sequence: each entry logs,
and `quorra`'s trace shows no render coming back ready after them, where the triggered field's `/E`,
which writes `Seen`, is followed by one — the count's control, since a pointer message is not traced
by name and its render shows only as `render ready`. The toolkits' traces count a command's events
without naming them, so the count is `quorra`'s; the decision is the core's, which every window
shares.

## The keys and the console

- **`Command::Keys` carries Shift and Control** (Control is the reference's modifier here, ADR
  1762), sent by a window as they change and before the pointer message whose event a script
  reads them at; the core holds them for every open document and every one opened after. No
  window reports an arrow key's choice selection, so `keyDown` reads false at every event.
- **`Event::Console` carries `console.show`, `hide` and `clear`**, after the command whose script
  asked. Every window's console is its log, where each line a script logs is said as it is logged:
  so a request is said by name — shown, it is; hidden, it cannot be; cleared, what was said stays
  said — rather than carried out in a pane none of them has (trap 5).
- Drive step 71: the open action's three requests are said in each window, and a release in a
  field with Shift and Control held logs both true, one without both false.
