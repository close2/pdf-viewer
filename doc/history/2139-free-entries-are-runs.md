# Round 2139 — free cross-reference entries are runs, in-use ones have a budget

A design-and-build round in the `audit` worktree for audit 2136's group C, B03 (principle 3).

## What changed

- ADR 3257. A section's statements are its in-use entries apiece (`Filed`, 24 bytes, the section's
  place in read order in the old tuple's padding) and its free entries as `FreeRun`s; the table keeps
  in-use winners in its map and free numbers as `FreeRuns`. `fill` gives a run and an entry §7.5.6's
  precedence by painting runs newest first and dropping an in-use entry a newer run covers; both
  steps are skipped when they cannot change anything.
- `MAX_XREF_ENTRIES` = 2²³ in-use entries across the chain (ISO 32000-1 Table C.1's 8 388 607 objects
  and object 0). A stream's in-use records are counted from their type bytes before anything is held;
  past the budget `read` returns `SyntaxError::LimitExceeded { limit: "MAX_XREF_ENTRIES" }`, never a
  scan and never a part. `object_stream_pairs` refuses an `/N` past it.
- Not built: stopping the decode at `/Index`'s rows (ADR 3257 §3; the decoders have no bounded
  answer that runs a predictor).

## Numbers

The audit test's file: 110 568 KiB grown before, 12.7 MiB after (its decode). ISO 32000-2 open,
callgrind: 63 169 707 instructions before, 62 820 888 after; a first version cost +9.4% and was
restructured. Peak RSS on ISO 32000-2, `issue16091.pdf` and `bug1978317.pdf`: unchanged (11, 11 and
136 MiB). `bug1978317.pdf` taking 136 MiB and 2.2 s to open is not the table's and is left.

## Ledger and manifest

7.5.4, 7.5.8.2 and 7.5.8.3 stay `implemented`; 7.5.4 lists the precedence test and the audit test.
The manifest loses B03 (1 entry to 0); 2136's `findings.json` marks B03 fixed.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-transform`: fmt exit 0; clippy `-D warnings
--all-targets` exit 0 after one fix in the new test; `pdf-sandbox --bins` exit 0; nextest `--lib
--bins --tests` 4575 run, 4575 passed (manifest difference 0 and 0); `cargo test -p pdf-syntax` 415
passed; rustdoc exit 0 after two private links were fixed; fuzz `check --bins` exit 0, and
`document` 50 000 runs from 300 pdf.js seeds (this worktree has no corpus), exit 0, no crash;
`cargo test -p conformance --no-fail-fast` failed only on this record lacking this paragraph,
and passed once it was written.

**Habit.** A change on the open path is counted under callgrind against the release binary of the
commit before it, by function, before it is called done.
