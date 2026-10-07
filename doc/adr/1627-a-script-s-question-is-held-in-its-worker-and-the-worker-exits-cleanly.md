# 1627 — A script's question is held in its worker until it is answered, and the worker exits cleanly

Status: accepted and **built** in `pdf-script`, `pdf-script-worker`, `pdf-model` and the script
profile of `pdf-sandbox`; the windows' half is ADR 1628's, built by round 1396. Session 1395. Builds
on RFC 0008 sections 4.2 and 6.8, ADR 1609 (the deadline per trigger), ADR 1620 section 6 (the
`SIGSYS` at a clean exit, handed over).
Context: Adobe's *JavaScript for Acrobat API Reference*, "app methods" (`alert`, `response`), cited
by name, never quoted.
Code: `crates/pdf-script/src/question.rs`, `engine/members.rs` (`alert`, `response`, `put`),
`engine/mod.rs` (the wait off the wall budget), `wire.rs` (`encode_question`, `encode_answer`);
`crates/pdf-script-worker/src/worker.rs` (`Wire`), `wire.rs` (`FRAME_QUESTION`, `FRAME_ANSWER`),
`client.rs` (`take_question`, `answer`, `question_withdrawn`, `ANSWER_WAIT`, the queue);
`crates/pdf-model/src/view/scripts.rs` (`ScriptRunner::waiting`, `take_resumed`, `Resumed`,
`ViewState::apply_resumed`); `crates/pdf-sandbox/src/lockdown_linux.rs` (`PERMITTED_SCRIPT`).
Tests: `crates/pdf-script/tests/census_members.rs`; `crates/pdf-script-worker/tests/end_to_end.rs`
(`a_question_holds_its_script_in_the_worker_and_the_answer_resumes_it`,
`triggers_wait_behind_a_question_and_a_question_nobody_answers_is_withdrawn`,
`a_worker_whose_input_ends_exits_zero`).

## 1. The realm asks, and is answered in the call

`app.alert` (positional or one object of named arguments) and `app.response` make a
`pdf_script::Question` — Adobe's icons, button sets and return numbers, the shapes ADR 1628's
`ScriptQuestion` carries, variant for variant — and hand it to the realm's `Asker`, which answers in
the call. **One question per run**: a second is answered as a closed dialogue answers without being
put, and counted in the run's notes (RFC 0008 section 6.8). A question nobody can answer — no asker,
or a face without a dialogue answering `Unanswerable` — is answered the same way and said; it never
throws, so a script that alerts and then works goes on working. An alert's check box is not drawn:
its `bAfterValue` is its `bInitialValue`. A password typed into a response is never repeated in a
report. The time a question waits is not the script's: the engine's wall budget leaves it out.

## 2. The window's thread never waits, so the worker holds the script

ADR 1628 section 2 rules it: a script runs inside `Viewer::handle` on a window's own loop, and that
loop draws the question. So in `pdf-script-worker` the asker is the wire: the worker writes a
`FRAME_QUESTION` in place of the run's outcome and blocks, the script held mid-call, until a
`FRAME_ANSWER` arrives; its reply to that is the run's outcome. The host's `run` returns at once,
**having changed nothing**, with the sentence that it waits; `ScriptRunner::waiting` says so, and
every later trigger of the document queues behind it in order (at most 256), answered the same way.
`ScriptWorker::take_question` hands the question over once, `answer` sends it, the script finishes,
the queue runs (at most a second per call, the rest at the next poll), and each finished run waits
in `ScriptRunner::take_resumed`. `ViewState::apply_resumed` applies each as its trigger's outcome
arriving late: its edits, its sentences, and its event where the trigger can still take it — a
commit's keystroke or validation that refuses puts back what the field showed before the typing
(unless the person is typing in it again), a commit's rewrite is written, a calculation's value set,
a format's text displayed; `/CO` is walked again where a value moved. All or nothing, as every run
is: a held run that then throws applies nothing.

**The deadline is restarted, not paused**: a run's exchange ends when its question comes back, and
the exchange carrying the answer has the full `DEADLINE` from the moment it is sent. A person
reading is no exchange's time.

**A question nobody answers** is answered by the runner after `ANSWER_WAIT`, two minutes, at its
next call — `take_question`, `take_resumed`, `question_withdrawn`, a run — with the closed-dialogue
answer (Cancel, No or OK, and `null`); the run's report says so first, `question_withdrawn` turns
true once so a window can drop its card, and a later `answer` is kept nowhere.

For round 1396: `viewer_host::scripting`'s `Workers::runner_that_asks` hands `(worker, desk)`, the
desk forwarding `take_question` and `answer` with a one-to-one conversion, `Warning` forwarding
`waiting` and `take_resumed`; `Command::AnswerScript` calls `answer` and then
`ViewState::apply_resumed`, marking the page stale when it answers true.

## 3. The question crosses as two frame kinds of the worker's own

`FRAME_QUESTION` (4, worker to host) and `FRAME_ANSWER` (5, host to worker) carry
`pdf_script::wire`'s encodings, version 4. `confined-transport`'s resource port was not used: its
worker side reads with `recvmsg`, which `Profile::Script` does not admit, and its broker answers
inside the exchange, which is the waiting this decision rules out.

## 4. A clean exit is admitted, not avoided

Returning from `main` runs the standard library's teardown, which disables the alternate signal
stack it installed for reporting an overflowed stack: `sigaltstack`, off the script profile, so a
worker whose host closed its end died by `SIGSYS` (exit 159) and dumped core. Two ways out were
priced. `libc::_exit` after a flush needs `unsafe`, which the worker crate forbids, and a safe
wrapper (`rustix::runtime`) is an undocumented experimental module. **`sigaltstack` is admitted**:
it changes where this process's own handler runs and reaches nothing outside it, and the decoder's
profile already admits it. Without it the new test reads exit 159; with it, 0.
