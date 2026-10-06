# 1588 — An owner's answer is seen the day it lands

Session 1376. Status: **accepted**. Extends ADR 1440's printer of what a merge cannot carry; reads
the main checkout as ADR 1487 requires, and never writes it.
Context: `doc/questions/README.md` (an `A` file is the owner's, landed through the owner's own
session); `tools/conformance/tests/questions.rs`; `tools/main-checkout.py`; `tools/batch.sh`.
Code: `tools/main-checkout.py` (`answers`, `--answers`), `tools/batch.sh` (`check_batch`),
`tools/conformance/tests/owner_section.rs`, `doc/environment.md` *After a merge*.

## 1. What was missed

Five answers landed in the owner's checkout on 2026-10-05 at 21:27 and were read a day later. Every
list a round or a merge reads of open questions is drawn from tracked files —
`tests/questions.rs` reads the worktree's index, and a worktree holds only what is committed —
and an `A` file stays untracked (or a modified tracked file) until the owner commits it. The
printer already counted those files, as a number on the seventh line, which said that something was
there and not what: nineteen of them, the newest a day old, read the same as nineteen a month old.

## 2. The decision

`tools/main-checkout.py` prints, **first**, every `A` file `git status` reports in the main
checkout's `doc/questions/` — untracked or modified — newest first, each with the date it landed;
then the questions open once those are counted (every `Q` on the disk with no `A` beside it), and
the ones a list of tracked files alone still calls open. `--answers` prints those two lines and
nothing else, and `tools/batch.sh check`, which every merge runs, prints them. They are never a
finding: an uncommitted answer is the owner's, and nothing a round does clears it.

**The date is the file's modification time**, because that is when it landed on the disk, which
is the fact a merge missed. An answer's `Given:` line is the owner's word about the conversation,
and a modified tracked answer (A03, A51) keeps its first `Given:` while its edit is newer.

## 3. What it costs

One `git status` and one directory listing, read-only (`--no-optional-locks`), on a section that
already ran both. `tests/owner_section.rs` holds the two lines first in the printer's order, the
section's entries for both, and the check's call by name.
