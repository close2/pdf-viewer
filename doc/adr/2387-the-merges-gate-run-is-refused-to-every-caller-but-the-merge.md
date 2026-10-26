# 2387 — The merge's gate run is refused to every caller but the merge

Session 1776, a "host" round of batch one hundred and seventeen, 2026-10-26.
Status: implemented in `tools/batch.sh`'s dispatcher; the merge's invocation is now
`tools/batch.sh gates --merge`. Context: trap 270, ADR 2362's slot. Cites no clause and moves no
ledger row.

## The decision

`tools/batch.sh gates` without `--merge` prints three sentences — that it is the merge's whole gate
run and empties the shared log first, where a round's gate commands are listed, and the merge's own
line — and exits 2 before anything is touched. With `--merge` it is what it was.

## Why a refusal and not `--list`

Trap 270 is a round sent into the command by a brief that called it a list: it truncated the merge's
log, ran four gates and was stopped in the fifth. A `--list` would add a third list of the gates
beside `doc/todo/02` section 2's map and `tools/round.sh <kind>`, which are the ones a round is
meant to read, and would leave the harm where it was: the next brief that writes the command without
the flag runs the whole sequence again. The refusal removes the harm at the one place every such
mistake passes through, and its message is the pointer `--list` would have been. The flag is a
declaration rather than a lock — a round could type it — but it cannot be typed by a brief that
believes the command lists something, which is the incident.

## What changes for the merge

The orchestrator's chain script, which calls `tools/batch.sh gates` today, calls
`tools/batch.sh gates --merge`; `doc/todo/02` section 8 and `doc/orchestrator-handover.md` say so.
Until it does, the merge's gate step exits 2 with the line it should have run.
