# 1641 — A withdrawn question is an event, and an answer is only ever the taken question's

Status: accepted; the runner's half **built** in `pdf-script-worker`, the windows' half round
1403's. Session 1402. Builds on ADR 1627 section 2 (`ANSWER_WAIT`, `question_withdrawn`) and ADR
1628 section 4, whose last sentences this closes: a question the worker's wait withdrew left its
card up, and the late answer was "dropped".
Code: `crates/pdf-script-worker/src/client.rs` (`ScriptWorker::answer`, `question_withdrawn`,
`question_deadline`, `expire`).
Tests: `crates/pdf-script-worker/tests/end_to_end.rs`
(`a_late_answer_is_never_handed_to_the_question_asked_after_it`,
`triggers_wait_behind_a_question_and_a_question_nobody_answers_is_withdrawn`).

## 1. The late answer was not dropped; it could reach the next question

ADR 1627's runner answers an expired question at its next call and then runs the queue behind it,
and a queued script may ask a question of its own. `answer` resumed whatever waited, without
checking the wait first and without asking whether the host had taken it — so a press on the first
card, arriving after two minutes and before the host's next poll, was handed to the second question,
which nobody had read. The premise "the late answer is dropped" held only for an empty queue.

## 2. The rule

- `answer` runs the expiry first, and resumes only a question a host has **taken**. An answer that
  arrives after its question's wait is kept nowhere, whatever has been asked since.
- `question_withdrawn` is true once per withdrawal of a **taken** question; a question no host took
  was never on a card, and saying it was withdrawn would be noise.
- `question_deadline` answers when the taken question is withdrawn if nobody answers it, so that a
  window with nothing else to do can wake then and drop the card as the wait runs out; the runner
  keeps no thread of its own, so without it the card falls at the person's next action.

In a window, `handle` and the poll after it run on one thread and a press is a later command, so a
press is never handed over between a withdrawal and the event that says so — provided the poll asks
`question_withdrawn` before `take_question`, which puts the withdrawn card down before the next
card is put up.

## 3. The event, and the windows' half

`Event::ScriptQuestionWithdrawn { document: DocumentId }`, sent from the same poll as
`Event::ScriptAsking` and before it; one question per document is up at a time (ADR 1145), so the
document names the card. `ScriptAsks::withdrawn` is forwarded by `viewer_host::scripting`'s
`Workers` to `ScriptWorker::question_withdrawn`; round 1403 built that half, and a window's wake
when the wait would run out, in ADR 1643. `question_deadline` is the runner's own clock for that
wake, where a host would rather read it than start its own. What the runner did in the person's
place is already said: the resumed run's report opens with the sentence that its question was not
answered within the wait, and `ViewState::apply_resumed` reports it.
