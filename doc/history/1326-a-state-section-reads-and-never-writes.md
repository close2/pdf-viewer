# 1326 — A state section reads and never writes

Instruments slot of batch fifty. ADR 1487. No ledger row moved; no question written.

**The ledger section.** `state.sh ledger` ran `--bin ledger`, the generator: it rebuilt the file
from the clause index and wrote it whole in canonical key order. §8.6.6.6's row carries `test`
above `code`, so every run moved a line — the "reordered test lists". The binary now counts by
default, says whether the file is in generated form and from which line it is not (1694 today),
and writes only under `-- --write`. `verify.md`, `lib.rs`, the missing-row message, `todo/02`
name the flag.

**What else wrote.** Every `quick` section under `strace -f` (writes outside the build directory
and `scratchpad/`): `ledger` the ledger; `main-checkout` `index.lock` in the main checkout and
the worktree (`git status`/`diff` refresh the stat cache), now `--no-optional-locks`;
`conformance` only under `temp_dir()`; the rest nothing. `comments` and `governing` ran Python
without `PYTHONDONTWRITEBYTECODE=1`; harmless (no imports), set anyway.
`tests/read_only.rs` holds it by construction (a lexer: redirections, writing commands, git
locks, bytecode, a writing `conformance` bin run without `--write`); it found those three Python
lines first; a planted script calibrates it.

**Owner's section.** `tests/owner_section.rs` derives the kinds `main-checkout.py` prints, in
`main`'s call order, and *After a merge*'s entries; one list in one order, and each sub-line
named. First run: `no Repository:/Base: preamble:` named nowhere; the patches entry now says it.

**New lines.** `state.sh batches`: per batch commit with `Round durations`, the gates figure
and the sum of figures written `<n> s` with their count (8566f66e: 1269 s; 23 570 s over 7).
`todo/02` section 8 item 4 names it and asks for that shape. `state.sh drive`: the newest
`results.tsv` under `scratchpad/` (or `DRIVE_RESULTS`) as works/wrong/not offered/to look at;
no copy under `doc/`.

**Documents.** state-of-play: 1466 (popup, wheel, title), 1469, 1471, 1472. PLAN: the two
tests, `gates-cost`/`batches`. crate-map: the tests, the ledger bin's two modes. HANDOVER: the
measures row names `gates-cost` and `batches`; a `render-raster` row (empty `differs` lists,
ADR 1435's reference first, ADR 1471). Navigation: 14/14 absent, 28/28 undefined, history 0;
overtaken 16 → 17 (ADR 1483 against `oracle.rs` lists, not these four documents).

**todo/65.** No row changed status (1324, 1325 none); `cargo search` unchanged from the note's
2026-10-01 re-check; frontier map, README and `todo_citers` pass.

**Records 1315–1320.** 31–39 lines. 1316 and 1319 say "see the report" for gates, 1318 none.
