# 2121 — The chooser, a script's question and a print reach a C caller as records it answers

Session 1642, batch one hundred's slot 4, a host round. Status: **accepted** and **built**.
Context: ADRs 2088 section 4 (the three acts declined aloud), 1966 and 1985 (the chooser), 1628 and
1643 (a script's question and its withdrawal), 1180 (the print), 2060 decision 5 (Adobe's numbers).

## 1. What was declined

`quorra_window` said three sentences where the toolkit windows open a dialogue: the archive chooser
("cannot offer the choice … yet"), `app.alert` and `app.response` (said, then answered as a dialogue
closed unanswered), and a print grant (said, then ended at once with `Printing::Finish`). And the
print key was handed back as `QUORRA_ACT_PRINT` with no entry point a caller could start a print
through, so the grant was never reached from C at all.

## 2. Decisions

1. **Four owed kinds, each a record of what the toolkit's dialogue opens on** (ADR 2088 decision 2):
   `QUORRA_OWED_ARCHIVE` (13) — the chooser's title and sentence, and the target and profile it
   starts on as indices into `quorra_archive_target`'s and `quorra_archive_profile`'s lists, which
   are `Choosing`'s own; `SCRIPT` (14) — the document, the dialogue's title, and the question through
   `quorra_owed_script_question` and `_script_text` in the event accessors' numbers and shape;
   `SCRIPT_WITHDRAWN` (15) — the dialogue is to close; `PRINT` (16) — the pages and whether bit 12
   limits the quality. `QUORRA_OWED_KIND_COUNT` is 17, so a caller compiled at 13 says so at startup.
2. **Each is answered once**: `quorra_window_archive(target, profile)`, either negative a chooser
   closed with `NOT_CHOSEN` said, past a list `QUORRA_OUT_OF_RANGE` with the chooser still out;
   `quorra_window_answer_script(document, how, button, text)` in `quorra_answer_script`'s numbers;
   `quorra_window_print_finish`, `Printing::Finish` as the toolkits end a print. A second answer is
   `QUORRA_NO_ANSWER`, the guarantee `quorra_window_answer` already gives.
3. **The window carries out the print key**, as `viewer-qt` and `quorra` do: `Printing::Start` on
   the unknown sheet, and the same key ends a print that stands. `quorra_window_press` now answers
   `QUORRA_ACT_NOTHING` for it; `quorra_key_press`, the vocabulary layer's, still answers
   `QUORRA_ACT_PRINT`. The pages on the screen are what would print until it ends, `viewer-qt`'s
   preview; paper, cells and spooling stay RFC 0010's N4.
4. **A question's withdrawal is looked for on the window's own timer**: `quorra_window_interval`
   wakes the caller `script_asks::wake_after` after the question went up, and the tick sends a
   `Command::Tick` of no time, `viewer-gtk`'s one timeout as an interval, looked for again a wait
   later where the runner has not withdrawn it yet.
5. **The decoding of a script's answer and the writing of a question's numbers are one function
   each** in `abi.rs`, shared by the vocabulary layer's entry points and the window's.

## 3. Costs

Nine entry points, four kinds. The print key's answer from `quorra_window_press` changes. The
decision "start on the unknown sheet, or end what stands" is now written in three windows, and is a
candidate for `viewer-host` on habit 131's rule.
