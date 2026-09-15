# 1096 — the rule reaches the prose, and the budget is counted

## 1. The `§` rule now reaches the documents
Before: `scan_tree` read **1 078 Rust sources** and the ledger's **883** notes; **no Markdown at
all**. After: `conformance::documents` adds the current instructions — `doc/*.md`, `doc/todo/`,
`doc/habits/`, `doc/questions/Q*.md`: **163 documents, 7 583 signs**. The records and the owner's
`A*.md` are out — a gate whose only remedy is an edit may not be aimed at a file that may not be
edited. `another_document` gained two arms: an ITU-T Recommendation in one word (`X.690`), and a
two-number designation (`ETSI EN 319 142-1`, `319` where the acronym has to be). Both had landed on
ISO 32000-2, which has a §10.1 and a §6.3. **36 sites fixed**: 16 in `Q46`–`Q52`, 20 the gate named
on its first run. `doc/todo/56`'s own table of what the checker catches had **gone stale**: its
`ISO 21757-1:2020` row said "silent pass", caught since session 983. **Not gated, measured**:
the other half reports **620 of 7 134** citations naming no clause in 44 documents, and that is the
visible part only (`documents.rs`).

## 2. The budget is counted
Section 8 of `doc/todo/02` has said forty lines since the loop existed; nothing counted it, and
1086–1091 ran 44, 47, 19, 40, 40, 40. `tools/state.sh records` prints the last twelve beside the
budget, off `tests/records.rs`, which fails any record from 1086 on; the figure lives in the check.
1086 44 → 40 and 1087 47 → 39, restatement only — from 1086 a sentence repeating that the rows had
long been read right and a second naming of the five rows; from 1087 two clause quotations ADR 1101
and the code carry, the seam list's parentheticals and `across`'s `Option`. Every number, verdict,
plant and gate line stands. **It named siblings' records too**, theirs to trim.

## 3 and 4. Two lines in the brief, and one command at the merge
`doc/todo/02` §8's template now carries scratch under `scratchpad/r<round>/` (round 1091 lost a log
that way) and *wait on a pid you hold, never `pgrep -f` of your own command line*. `tools/batch.sh
check` prints six lines from inside the worktree — untracked files of an unexpected extension, a
regular PDF under `doc/`, a `\uXXXX` in the ledger, a record over budget, a submodule staged as a
blob, `fmt --all --check` — and exits non-zero if any bites. **Each was calibrated by a plant**
(trap 13): a `stray.h`, a `doc/zz-plant.pdf`, a ledger escape, a line added to 1090 and to 1091, a
`.gitmodules` entry naming `CLAUDE.md`, an unformatted line of mine. Each was named and removed;
the ledger's under a byte comparison.

## Gates
`-p conformance`: 256 lib tests, `--test conformance` 7/7 and `--test documents` 1/1 green; `--test
records` red only on two siblings' records, which this round does not own. `clippy -p conformance
--all-targets` under `-D warnings` 0; `fmt -p conformance --check` 0; `bash -n` on both scripts 0;
`tools/batch.sh check` exit 1, on those same records and nothing else.
