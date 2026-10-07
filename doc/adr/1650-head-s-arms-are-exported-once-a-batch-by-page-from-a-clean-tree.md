# 1650 — HEAD's arms are exported once a batch, by page, from a clean tree

Session 1407. Status: **accepted**. Carries out ADR 1638 section 4's first item.
Context: ADRs 1443 (the per-page times file), 1440 and trap 50 (one build directory per path),
1638; trap 15 (a binary carries the tree it was built from), trap 109 (the worker rebuilt inside the
lock); `tools/batch.sh arms`, `tools/conformance/tests/batch.rs`.

## 1. The question

A pixels round compares its pages against HEAD's on six arms of `render-raster`'s corpus gate —
three coverage lanes (`cpu`, `gpu`, `compute`) at scales 1 and 4 — and five of them in nine batches
built and walked HEAD again to get that baseline. ADR 1638 owed a command that exports the arms once
per batch at `open`, keyed by commit. Batch sixty-three's arms were exported by hand
(`/home/AI/export-arms.sh` into `/home/AI/arms-1402/`), and its shape is kept: `<lane>-<scale>x.txt`
per arm, `build.log`, and a `README` whose first line names the commit.

## 2. What the hand export showed

Read before building on it (trap 13), the hand export had three defects, each one an export that
looks complete:

1. **No page's digest.** The gate's `--nocapture` output names only the pages that differ from
   the oracle, are refused or are not comparable; the per-page frame digest is written only to the
   file `PDFVIEWER_RASTER_TIMES` names (ADR 1443), and the hand export set no such file. A round
   could not compare its pages by digest against it.
2. **Not one commit.** Each arm was its own `cargo test`, in the shared worktree, minutes apart.
   By the fifth arm the rounds had begun, and `gpu-4x.txt` opens with `Compiling pdf-model` — a
   sibling's mid-batch edit built into an arm labelled HEAD.
3. **A neighbour's wrapper.** `cpu-4x` passed its test and was recorded `exit 1` because the
   worktree's `tools/bounded.sh` was mid-edit (`line 422: dren: command not found`).

## 3. Decisions

1. **Two files an arm.** `<lane>-<scale>x.txt` is the gate's output as before;
   `<lane>-<scale>x.tsv` is its `PDFVIEWER_RASTER_TIMES` file — name, oracle ms, raster ms, frame
   digest, mean error — which is what a comparison by page reads.
2. **HEAD's only if the tree is.** `arms` refuses a worktree holding any path outside
   `scratchpad/` that `git status` reports, and refuses again if one appears while it builds.
   `open` starts it detached, right after `warm`, before any round is briefed.
3. **One build, copied, and one hold of the lock.** The sandbox worker and the corpus test binary
   are built once, the binary's path asked of Cargo (trap 15), both copied out of the build
   directory and their SHA-256 written to `README`; all six arms run the copies, with
   `PDF_SANDBOX_WORKER` naming the copied worker, under one hold of the heavy-walk lock taken before
   the directory is written. A sibling's edit, rebuild or `--bins` between two arms cannot reach
   them. The copies are deleted at the end.
4. **Keyed by commit, never overwritten by another.** The directory is `/home/AI/arms-<first>/`,
   the first session read off the branch `batch-<first>-<last>`, or the directory given. An export
   complete for this commit (a `done` line, no arm's `exit`, six non-empty `.tsv`) is kept; one for
   this commit that is not complete is redone; one naming another commit is refused.
5. **An arm is complete when its test passed and its page file holds lines.** A green line that
   ran no test is `exit 98`, as `gates()` reads one (ADR 1392).

## 4. What it cost, and what calibrated it

Batch sixty-three's export (`/home/AI/arms-1402/`, from a detached checkout of `0f9c12cf`) held the
lock 1 864 s: the 1x arms 71–87 s each, the 4x arms 473–557 s each at a load average of 15–19, with
59 s queued first. Started at `open`, the hold overlaps the briefs being written and delays at most
a round's first walk; mid-batch it is a half-hour that every sibling's walk queues behind, which is
a reason to export at `open` and not a reason to release the lock between arms. Each 1x arm wrote
968 page lines and each 4x arm 963–964, the gate's "pages compared" less the frames it refused. The
digests discriminate: `cpu-1x` and `gpu-1x` differ on 125 pages, `bug1743245.pdf` (the one page the
gate names as differing) among them. A second `arms`
over the finished directory kept it and exited 0, and `tests/batch.rs` holds the four refusals,
calibrated by planting the defect (a skipped dirty-tree check fails the test).

## 5. Declined

- **Exporting from the shared worktree mid-batch.** It is what the hand export did, and §2 is the
  result. A mid-batch export runs from a clean checkout of HEAD (`git worktree add --detach`, its
  own build directory — trap 50 — with the gitignored data linked as `open` links it) with
  `BATCH_WORKTREE` naming it.
- **Re-running each arm through `cargo test`.** Cargo would rebuild between arms whatever changed,
  which is defect 2.
