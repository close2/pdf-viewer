# 614 — The gate that only ran somewhere else

The pipeline had been red on every push for five runs and a week, and the project owner is who
noticed. Five of seven jobs failed in the last of them; none of the five was a CI configuration
that had drifted, and every one reproduced here in seconds once somebody ran the command CI runs.

Date: 2026-08-20.
ADR: [0450](../adr/0450-the-gate-that-only-ran-somewhere-else.md).

Touched: `.github/workflows/ci.yml`, `crates/viewer-qt/cpp/window.cpp`,
`crates/viewer-accessibility/src/bridge.rs`, `crates/pdf-syntax/src/filter.rs`,
`crates/pdf-render/src/paint.rs`, `tools/round.sh`, `doc/todo/02-every-round.md`, `doc/verify.md`,
`doc/todo/52`, `doc/todo/26`, `doc/conformance/ledger.toml` (§12.5.6.12), the ADR and this file.

## The five, diagnosed from their own output

`gh run view 32338494994 --log-failed`, per job, rather than from the job names — and two of the
five were not what the job name suggested.

- **`check` and `test`** — neither reached `clippy` or a test. `viewer-qt`'s `cpp/window.cpp` asks
  for `QPalette::Accent`, which is Qt 6.6's, and the runner's `qt6-base-dev` is older: the build
  script failed and took the whole workspace's two Linux jobs with it. **A defect here** — a
  version requirement made silently.
- **`build (macOS)` and `build (Windows)`** — `unused variable: wake` in
  `viewer-accessibility/src/bridge.rs`, an error under the workflow's `RUSTFLAGS: -D warnings`.
  **A defect here**, and `doc/verify.md`'s Windows `-p viewer-ui` line reproduces it in fifteen
  seconds; nothing had run it.
- **`nightly`** — Miri, and **not** `zlib-rs`. `crossbeam-epoch` 0.9.20 fails Stacked Borrows
  inside a `rayon` worker, reached from `pdf-render`'s divided reduction. The `--skip flate` the
  job carried names a different dependency and could not cover it.

`deny` passed.

**And a sixth, one run earlier**: a `test` job that ran six hours and one minute and was cancelled
at the platform ceiling. Its log stops inside `apt-get update` on the runner's Azure mirrors — an
infrastructure stall rather than a hanging test, but one that cost six hours and produced a failure
whose shape said nothing about its cause. Every job carries `timeout-minutes` now.

## What the local gate turned out to be weaker at

Three instruments, weak in three different ways, which is the ADR's subject:

- **the lint line, by a flag** — `doc/todo/02` §2 had no `RUSTFLAGS="-D warnings"` where the
  workflow has one and `doc/verify.md` already said it "is not optional" for the cross-target
  checks. It carries the flag now. With it set, this tree's host-target lint run is silent — so
  the gap was real and the warning that fell through it was on another target;
- **the cross-target checks, by not being run** — they are in `doc/verify.md` rather than in §2's
  change → gate map, and `viewer-ui`'s row in that map is satisfied by a host build. The block now
  says which CI job those lines stand in for, and gained the macOS twin of the one that catches it;
- **and beside it**, `tools/state.sh` runs neither of §2's first two lines — right for a script
  whose subject is figures, wrong for a section that said it "runs the same sequence", so a round
  that reached for it had not linted at all;
- **the Miri skip, by being a string in a workflow** — `--skip flate` silently took a third test
  (`an_inflate_never_buys_a_buffer_past_the_bound`, `flate` inside `inflate`) where `doc/todo/52`
  said two, and could not cover a dependency nobody had thought to name. Every declination lives on
  its test now and the workflow line has no `--skip`.

## And why nobody noticed

Nothing in this project ever asked what happened to a push. `tools/round.sh` now does, as its fifth
check — a *report*, so it prints green, red with the command that shows why, or **not asked**, and
a silence is never rendered as green. It found this round's own subject on its first run.
`workflow_dispatch` is on the workflow for the other half, usable one merge from now.

## The spec-driven half — §12.5.6.12

The row was `reported` with **no `code` and no `test`**, the only row in the ledger above `silent`
with no code at all. Its stated reason was that the modal verb parts the icon clauses: Table 184
says a reader *should* provide predefined icons where §12.5.6.4's Table 175 says *shall*.

Read against `doc/md/`, that is false — Table 187's file attachment icons and Table 188's sound
icons carry the identical *should*, word for word, and this tree draws all six of them. The real
distinction has been in the code and in its test since the two-hundred-and-sixty-sixth session:
Table 187's and 188's names are **objects** and Table 184's are **legends**, and drawing a legend
means choosing typography rather than drawing the thing the name names. The row never heard,
because it named nothing to be read beside it — the defect §10.8.3's row records of itself. The
refusal was right; the reason was three hundred sessions stale. `doc/todo/26` had the right
argument and the wrong table number (186 for 184).

## No green run, and why

The branch could not be pushed. The token here is read-only for contents — `git push` answers 403
and so does the refs API — so no branch reaches the remote, no pull request can be opened for one,
and `workflow_dispatch` is offered only for a workflow already on the default branch. There is no
route from this account to a run; the merge that lands this is the first thing that can produce
one, and it owes §2 on `main` and a look at what it triggers.

## Gates

The core four with the flag on the lint line, CI's own `RUSTFLAGS="-D warnings" cargo test
--workspace`, the conformance gate, `cargo deny`, and all six of `doc/verify.md`'s cross-target
checks plus clippy on both foreign targets. What changed in the crates is confined to
`#[cfg(not(target_os = "linux"))]`, `#[cfg_attr(miri, …)]` and C++ behind a `#if`, so no gate that
rasterises anything can see it; the numbers are in the commit body.

**Miri was run here, on the crate that was failing.** `pdf-render`'s library tests: 168 passed, 0
failed, 1 ignored, and the interpreter reaches the end where before it aborted inside
`crossbeam-epoch`. `pdf-syntax`'s run was still going when the round closed, but its declinations
were already visible in its output — `an_inflate_never_buys_a_buffer_past_the_bound ... ignored,
zlib-rs's deallocation, not this tree's` — which is the point of the reason living on the test. All
four tests that decline under Miri still run and pass outside it.

**And a gate caught this round committing the thing it exists to catch.** `git add -A` in a
parallel worktree stages the six submodule symlinks over their gitlinks — invisible in `git status`
afterwards, and `git restore --staged` does not undo it — so the commit shipped a 120000 blob where
`doc/arlington-pdf-model` should be, which is a checkout CI could not build.
`every_declared_submodule_is_still_tracked_as_one` failed with the six paths and the
`update-index --cacheinfo` loop that restores them. A round writing about instruments was saved by
one; `doc/environment.md` now says not to reach for `-A` here.

**And the round's shell moved into a neighbour's worktree.** With no `cd` to any worktree, `pwd`
went from `r614` to `r616`, and the next `git commit --amend` landed on `round-616`, rewriting the
commit that round had just made. Nothing was lost — the amend had nothing staged, so tree, parent,
message and author date are identical and only the committer timestamp and SHA differ, and the
original `5ccf6eab` is in that worktree's reflog — but the repair (`git update-ref refs/heads/round-616
5ccf6eab 31d6a710`) needs a permission this round does not have, so **`round-616`'s tip is
`31d6a710` where its own session left `5ccf6eab`**, and whoever merges should know that. The rule
that comes out of it is in `doc/environment.md`: carry `-C /…/worktrees/rNNN` on every `git`
command in a parallel round, so that no moving cwd can decide which branch you are on.

**And a Miri run cost the round an hour, on a cause it did not find and twice thought it had.**
`-p pdf-render` with `RUSTC_WRAPPER=` cleared is 3 min 54 s end to end, so the round blamed this
machine's global `sccache`. Then `-p pdf-syntax`, cleared the same way, ran past **33 minutes of
CPU** — and its process environment says `MIRI_CWD`, so it is the *runner* rather than a compile,
and says `RUSTC_WRAPPER=…/cargo-miri`, because `cargo-miri` sets that variable to itself and the
clearing was a no-op all along. Wrong in the conclusion and wrong in the mechanism. CI did both
crates in 2 min 39 s when it was last green; **the discrepancy is open** and is somebody's round.
What the `pdf-render` run does settle is the thing this round needed: the declinations work, on the
crate that was failing.
