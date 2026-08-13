# 485 — A step back, and what it tightened

**Finding.** The owner asked whether the tree had drifted into patching without seeing the whole
picture. It had not — the evidence is behavioural and is in ADR 0320 — and the review's real yield
was structural: the gates measure a raster and the work has moved off the raster, three files had
grown past their shape, and three process rules the project had already *learned* were written
down nowhere a round reads. The round bound the rules, split the files, and wrote the instrument
question into a standing todo. **And the newest rule paid for itself before the round ended**: the
three splits ran in parallel worktrees, all three worktrees were cut from a base seven commits
stale — the exact trap the 455–484 block summary records — and it was caught before merge only
because a fast-forward refused. Each split was rebased onto the true head and re-verified there;
the merged result then ran the whole gate sequence on `main`, which is what `doc/todo/02` §2 now
requires of every merge.

**Date.** 2026-08-14.
**ADR.** [0320](../adr/0320-a-step-back-and-what-it-tightened.md).
**Touched.** `doc/todo/02-every-round.md` §2 (the merge gate), `doc/todo/01-ledger-partial-rows.md`
(a sweep round commits one prose sweep as a program), `doc/environment.md` (a commit on `main`
keeps its body), `doc/todo/51-signatures-and-public-keys.md` (no zero-witness family before a
witnessed one), `doc/todo/05-an-instrument-for-the-interactive-surface.md` (new) with its
`doc/todo/README.md` index line, `doc/adr/0320-*` (new), `Cargo.toml`/`Cargo.lock`/`deny.toml`
(`hayro-jpeg2000` → `1dc833f7`, `doc/todo/24`'s step 0) and `doc/todo/24-image-sampling-intent.md`
(step 0 recorded done; the follow-up edits are unblocked), this file — plus the three split
commits, each its own commit with its own body:

- `content.rs becomes a directory that keeps its name` — `pdf-model`'s interpreter into fourteen
  submodules under `content/`, the root file keeping its name and every cited path with it; one
  ledger `test =` line repointed (§10.5's moved test).
- `A crate root that had kept what its siblings gave away` — `pdf-font`'s root to a 52-line front
  door over ten modules; 39 ledger rows, `doc/HANDOVER.md` trap 1 and `doc/todo/21`'s code list
  repointed.
- `The window's one file becomes the shape the other hosts have` — the winit host into fourteen
  modules beside a 301-line entry file; one true duplication replaced (`App::supply` was
  §12.7.6.4's policy line for line, and `viewer_host::resolve_import` states it for all three
  hosts), three near-neighbours checked and deliberately not merged.

## What the splits are and are not

Pure motion, each verified in its own way: a line-coverage audit for `content.rs` (every original
line accounted for exactly once), a block-by-block byte comparison for `pdf-font`, and for the
window the seven binary tests under their new paths plus an unchanged release build. The corpus
gate's 109 per-document lines are identical before and after on the same base; the workspace's
1763 tests pass on every rebased branch and on the merge. The only non-move edits are visibility
keywords, module docs, `use` lines, and the repointed citations named above.

Two things found and deliberately left, each flagged where it sits: `pdf-font`'s
`mod truncation_tests` doc comment was mis-attached before the round (written for the encoding
tests, parked above the truncation tests) and moved with the item it is attached to — reattaching
it is a correction, not a move; and four `[`Interpreter::…`]` intra-doc links in
`content/report.rs` no longer resolve from their new module — `cargo doc` is not a gate, and
rewording moved text was the thing purity forbade.

## What the next round should know

- **The stale-base trap now has a mechanical tell**: a workspace `nextest` total of 1723 against
  `main`'s 1763 was the number that separated every stale worktree from the true head. The block
  summary's advice stands — a worktree round's first command is
  `git rev-list --count HEAD..main` — and the merge gate is what catches whoever forgets.
- **`doc/todo/24` is fully unblocked.** The rev moved, the JPEG 2000 gate is unchanged to the
  line (including `issue19517.pdf`'s refusal — nothing asks for a reduced level yet), and the
  four listed edits are ready to take in order.
- **`doc/todo/05` is a design round, not a feature round.** It ends in an ADR or it has not
  ended.
- No verb was added anywhere in this round, so `doc/todo/01`'s sweeps were not owed and were not
  run; the ink sweep was not owed either, since no change can reach a raster — asserted by the
  corpus and oracle gates coming back identical, which is the check rather than the claim.
