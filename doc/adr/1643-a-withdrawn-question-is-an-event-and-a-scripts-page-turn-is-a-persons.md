# 1643 — A withdrawn question is an event, and a script's page turn is a person's

Status: accepted and **built**. Session 1403.
Builds on ADR 1627 (the worker holds a script on a question for `ANSWER_WAIT`), ADR 1628 (a
question goes out as an event and comes back as a command; section 4 named the card left up), ADR
1641 (the worker withdraws only a question a host took, `question_withdrawn` asked before
`take_question`), ADR 1640 (`ScriptEdit::GoTo` and `ViewState::take_page_request`), ADR 1615 (a
script's `setFocus`, carried out by the viewer).
Code: `crates/viewer-core/src/event.rs` (`Event::ScriptQuestionWithdrawn`), `scripting.rs`
(`ScriptAsks::withdrawn`), `viewer.rs` (`put_script_questions`, `carry_out_page_requests`,
`MAX_PAGE_TURNS`); `crates/viewer-host/src/policy.rs` (`Desk::withdrawn`,
`SCRIPT_ANSWER_WAIT_VARIABLE`, `script_answer_wait`), `script_asks.rs` (`withdrawn`, `wake_after`);
`crates/viewer-confined/src/protocol.rs` (event 24); `crates/viewer-ffi` (kind 24,
`QUORRA_EVENT_KIND_COUNT` 25); the four windows' arms.
Tests: `crates/viewer-core/tests/script_question.rs::a_withdrawn_question_is_said_once_and_before_the_next_question`,
`tests/script_page_turn.rs`; drive steps `48-script-withdrawn` and `49-script-goto`.

## 1. The event

`Event::ScriptQuestionWithdrawn { document }`: the question `Event::ScriptAsking` put for that
document is no longer asked — its wait ran out and the runner answered it as a closed dialogue
answers. The viewer asks `ScriptAsks::withdrawn` after every command, *before* it takes the next
question, so a host reading in order drops one card before it puts the next. The default answers
`false`: a runner whose questions are never withdrawn. The confined wire carries it as event 24;
the C ABI names kind 24 and sends it to no caller, for kind 23's reason (its sessions run no
script).

## 2. What wakes the viewer

The viewer has no clock (rule 3), so a withdrawal is seen only after a command, and a person who
leaves a card alone sends none. **A host that puts a script's question arms one wake at
`viewer_host::script_asks::wake_after`** — the runner's wait and a tenth of a second, since the
runner withdraws a wait that has *passed* and the window's clock started after the runner's — and
sends `Command::Tick { millis: 0 }`. A tick of no time advances no presentation and is a command, so
the viewer asks. No new command was added for it: `Tick` already means "time has passed", and
that is the whole of what the host knows. `quorra` wakes its event loop with `WaitUntil`, GTK arms a
`timeout_add_local_once`, and Qt a `QTimer` inside the dialogue's own `exec`, which closes the
dialogue when `poll_script_question` answers that the question is gone; GTK marks its dialogue
answered before closing it, so the close does not answer twice. `quorra-confined` answers every
question as it arrives and has no card to drop; its arm says so.

## 3. The wait a reader can set, and the drive

`PDF_VIEWER_SCRIPT_ANSWER_WAIT_MS` sets the wait in every window, `ANSWER_WAIT` otherwise — the
channel `PDF_VIEWER_MACHINE_FONTS` already is, for the same reason (ADR 0875) — so the drive watches
a question withdrawn at four seconds rather than two minutes.

## 4. A script's page turn

`ScriptEdit::GoTo` is held by the view state and taken by the viewer after every command, before a
focus request (a field's focus names its own page and wins), and carried out as `go_to` with
`Turn::Requested`, so Table 198's `/C` and `/O` run as for a person's turn. Only the document in
front turns; a request in a document behind stays held until a command finds it in front. A page's
`/O` may turn again, so a command makes at most `MAX_PAGE_TURNS` (four), `MAX_FOCUS_HOPS`'s bound
for the same ring.
