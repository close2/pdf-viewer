# 2072 — The key table crosses to C as numbers, and a press is acted on where every window acts

Session 1618, RFC 0010 section 6's N1 as the RFC states it, for the .NET host of
`doc/questions/A287`. Status: **accepted** and **built**. Context: ADRs 0526 (one key table for
every window), 1192 (a Control the table does not bind means nothing), 1357 (Space and Enter press
a focused push-button), 2048 (the level-hosts rule), 2060 (the eight commands).

## 1. What was missing

A C caller could send every `Command` but had no way to mean by a key what the three windows mean.
`viewer_host::meaning` answers a `Meaning`, half of whose rows are a `Command` that cannot cross a
C boundary and half a `WindowAct` with no number; and the windows wrap it in two decisions of their
own — `viewer_host::pressed` before it, and the "select some text first" sentence for a markup over
nothing after it. A C# window that rebuilt the table would be the copy `doc/todo/30` warns of.

## 2. Decisions

1. **`QUORRA_KEY_*` and `QUORRA_ACT_*` are this ABI's numbers, written out**, not positions in
   `viewer_host::Key::ALL`: that list is `viewer-host`'s to order, and an insertion in its middle
   would renumber keys under a compiled caller. New numbers are appended. `KeyKind::of` and
   `ActKind::of` match `viewer-host`'s enums exhaustively, so a key or an act added there fails to
   compile in `viewer-ffi`; a test walks `Key::ALL` through both directions.
2. **`quorra_key_press` acts on the press**, in the windows' order (`crate::keyboard::decide`): a
   focused push-button first, then the table, then the selection check before a markup. A
   `Meaning::Send` is sent by the library and its events are the answer, because a `Command` is
   not a thing to put in a header and every window dispatches one identically. What only a window
   can do comes back as a `QUORRA_ACT_*`, with `ScrollBy`'s logical distance in `by`.
3. **A sentence the press owes is `QUORRA_ACT_SAY` and `quorra_key_said`**, held on the session
   until the next press: the two-call idiom asks twice, and acting on a press twice to ask twice
   would be two presses.
4. **The modifier is `command`, named by its role** (RFC 0010 section 5): Control on Windows and
   Linux, Command on macOS. `present` takes `QUORRA_PRESENT_*` and `warned` a flag, because full
   screen and the drawing warning are the window's state, as in every window.
5. **Both counts are answered, neither is in `quorra_abi_check`**, for `QUORRA_CONTROL_KIND_COUNT`'s
   reason: a key is passed and an undefined one is `QUORRA_WRONG_KIND`; an act is an answer, and
   `quorra_act_name` names one a caller has no arm for.
6. **A submission's owed sentences cross on its event** — `quorra_event_submit_owed_count` and
   `quorra_event_submit_owed` — so a caller shows them beside the request, as `Submission::owed`
   asks. They already reached `QUORRA_EVENT_REPORTED` prefixed with the button.

## 3. Consequences

268 entry points (7 new) and 57 new constants. The C program reads both counts, presses five keys
and reads their acts, is told the markup sentence with nothing selected, and Tabs to a push-button
whose Space submits a form with Table 240 bit 11 set and bit 8 clear, reading the owed sentence.
The sentence "select some text first" is now written in four places, three windows and this crate;
it belongs in `viewer-host`, which no slot of this round owned. N2's `quorra_window` (section 3.2)
is not built here.
