# 1628 — A script's question is an event out and a command back, and the window's thread never waits

Status: accepted and **built**, with ADR 1627's runner. Session 1396.
Builds on RFC 0008 sections 4.2 and 6.8 (accepted, `doc/questions/A193`), ADR 1616 (the runner a
host supplies), ADR 1145 (one card, one question at a time), ADR 1155 (a face that cannot ask says
so), ADR 1540 (a dialogue is driven by its buttons' `Action`).
Code: `crates/viewer-core/src/script_question.rs`, `scripting.rs` (`ScriptAsks`,
`ScriptRunners::runner_that_asks`), `viewer.rs` (`put_script_questions`, `Command::AnswerScript`),
`crates/viewer-host/src/script_asks.rs`, `crates/viewer-confined/src/protocol.rs` (event 23,
command 40), `crates/viewer-ffi` (kind 23), the four windows' arms, `viewer-ui`'s `QuestionCard`.
Tests: `crates/viewer-core/tests/script_question.rs`, `viewer-host`'s `script_asks` tests, the
confined wire's round trips.

## 1. The premise, as the tree stood

The brief said the script slot builds `app.alert` and `app.response` as an event across the wire
with a bounded wait (ADR 1627). When this round started neither existed — `pdf-script`'s surface
listed both among the calls it refuses — so the window side was built against the shape RFC 0008
section 4.2 describes, an event a face with a dialogue answers and a face without one refuses by
name, and the two halves were joined through the records (section 4).

## 2. The window's thread never waits

Every window here runs `Viewer::handle` on its own event loop's thread, and a script runs inside
it: a commit's `/V`, `/CO` walk and `/F` happen before `handle` returns. A script call that blocked
there until a person answered would block the loop that draws the dialogue; GTK and Qt could nest a
loop, `quorra` — a `winit` application drawing its own card — cannot. So **the question is data
handed out after the command that raised it, and the answer is a command**: the shape of every other
question here (ADR 1145). What waits is the script, in its confined worker, for as long as its
runner's own bound allows. A runner that blocked inside `ScriptRunner::run` on a person would hang
`quorra` until the bound ran out; that is the one shape this decision rules out.

## 3. What crosses

`ScriptQuestion::Alert { message, icon, buttons, title }` and `Response { question, title, default,
label, password }`; `ScriptAnswer::Pressed(AlertButton)`, `Typed(Option<String>)` (`None` is
`null`) and `Unanswerable`. Icon, button set and return value are Adobe's numbers 0–3 and 1–4, read
from the *JavaScript for Acrobat API Reference* (the source A193 names), cited not quoted, and kept
on the script's side of the wire. **A dialogue closed without a press** answers Cancel where Cancel
is offered, No for Yes/No, OK for OK alone, and `null` for a response: the reference says nothing of
a close, and the least committal button is the choice — never Yes.

## 4. The join a runner makes

`viewer_core::ScriptAsks` — `take_question()`, once per question, and `answer(answer)`. A maker
overrides `ScriptRunners::runner_that_asks` to hand a desk beside each runner; the viewer polls each
document's desk after every command and sends `Event::ScriptAsking`, and hands
`Command::AnswerScript` to it. ADR 1627 is the runner's half: the worker suspends a script that
asks, `run` returns at once having changed nothing, later triggers queue behind it, and each run
that finishes once answered waits in the runner; the viewer calls `ViewState::apply_resumed` after
every command, which applies them as their triggers' outcomes arriving late and marks the page
stale where ink changed. `viewer_host::scripting`'s `Workers` hands the worker itself as the desk,
and `Warning` forwards `waiting` and `take_resumed`, saying *ran* of a resumed run rather than of the
held one. A question not answered within the worker's own wait is answered by the runner as a
closed dialogue; **the card stays up** until a person answers it, and that late answer is dropped —
a withdrawal event that took the card down is not built. At most one question per trigger is put
(RFC 0008 section 6.8).

## 5. The windows

`quorra`: the question card, titled with `viewer_host::script_asks::title` (the document, and the
script's own title beside it), the icon's word or the response's label, a typed entry (bullets for a
password), and a keys line — each button's letter, Enter for the affirming one, Escape for the
close. GTK: a `GtkWindow` with the script's buttons (affirming last and default) or an entry and
Cancel/OK; Qt: a `QDialog` the same, opened from a queued slot. Each prints `script_asks::put` and
`answered` (a password never said back). `quorra-confined` is pinned to `off` (ADR 1616), so no
question can come; its arm prints `UNANSWERABLE` and answers `Unanswerable`. The C ABI's sessions
supply no runner; kind 23 is named for a default arm.
