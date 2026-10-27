# 2467 — An export seeds its target itself and stamps its sources after the seed, and `round.sh` stops listing CLAUDE.md

Session 1816, an "instruments" round of batch one hundred and twenty-two, 2026-10-27. Status:
implemented — the owner's word of 2026-10-27 that trap 281 is fixed in this batch, and ADR 2462 rule 7.
Context: trap 281, trap 114 (`tar -m`), habit 185 (the seed), ADR 1890 (the export), ADR 2462.
Cites no clause and moves no ledger row.

## 1. What decides it

An export extracts HEAD with today's time so that a rebuild over a re-export is not handed the
previous binary (trap 114), and habit 185 had each round seed the export's target by `cp -a
--reflink=always` from the batch's directory afterwards, by hand. The extraction was then older than
a seed whose artefacts a sibling had just rebuilt from its own edited sources; cargo calls an
artefact newer than its sources fresh, and the export compiled HEAD's crates against a neighbour's
mid-edit `pdf-syntax` (batch 120, rounds 1798 and 1804). Cargo reuses a workspace crate's artefacts
across worktrees of the same layout, so nothing else stood in the way.

## 2. The change

- **`tools/batch.sh export --seed <build directory>`** copies the directory (`debug`, `release`,
  `gates`) into the export's own target by reflink, once: a re-export keeps the target and says so,
  because a second copy would put the batch's older artefacts over the round's newer ones. The copy
  is `--reflink=always`, so a filesystem without clones refuses; `BATCH_SEED_REFLINK=auto` is a test's.
- **Every regular file the export extracted is stamped after the seed**, with the current time or one
  second past the newest file the seed holds if its clock ran ahead. Every workspace crate then
  rebuilds from the export's own sources and every dependency stays fresh. The stamp runs for every
  export, seeded or not; links (the corpora) and `.cargo/` are left alone. The record carries a
  `seed:` line.
- **`tools/round.sh` no longer lists `CLAUDE.md`** among the every-round reads (ADR 2462 rule 7): it is
  in every request's context already.
- `tools/conformance/tests/batch.rs` holds the stamp: a seed built from a sibling's edit of a one-crate
  workspace, its artefacts dated two hours ahead, and the export's first build must print
  `Compiling probe`. Calibrated (trap 13): with the stamp taken out it printed `Finished` alone and
  the test failed.

## 3. What it costs

One `find` over the seed to read its newest time (a second over a build directory), and a `touch` of
the extracted tree. Habit 185's by-hand seeding still works; it just loses trap 281's protection.
