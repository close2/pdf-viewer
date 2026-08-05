# Five pages quorra newly refuses, and a budget whose own comment says they cannot exist

Status: **open, and it is failing a gate right now.**
Priority: 12 — a defect: five documents that drew do not draw.
Corpus: 5 of 974.
Clauses: none directly. This is a resource budget in a dependency.
Code: `crates/render-quorra/tests/corpus.rs`'s `REFUSED` ratchet; `quorra-gpu`'s
`DEFAULT_MAX_FRAME_BYTES`.

## What fails

```sh
cargo test --release -p render-quorra --test corpus -- --ignored --nocapture
```

```
assertion `left == right` failed: the pages quorra refuses have changed
  left: ["bug1703683_page2_reduced.pdf", "bug1721218_reduced.pdf", "issue12810.pdf",
         "issue14497.pdf", "issue1905.pdf", "issue9418.pdf"]
 right: ["bug1721218_reduced.pdf"]
```

One refusal is expected and argued — `bug1721218_reduced.pdf`'s coverage outgrows a 16384 × 16384
scratch image, which the handover has recorded for sessions. **Five are new**, and all five give
the same reason:

```
frame refused: frame needs 616862585 scene-derived bytes, over the stated budget of 268435456
```

(`bug1703683_page2_reduced.pdf`; the others need 280 762 806, 312 400 361 and so on.)

## What is known, and what is not

**Known.** The budget is `quorra_gpu::DEFAULT_MAX_FRAME_BYTES`, 256 MiB, and it is *unchanged*
between the two quorra revisions this tree has used — the constant and its comment are identical
at `7d5dafb` and at `7599081`. So the budget did not move; **what the frame is charged did**.

**Known.** The failure predates the three-hundred-and-eleventh session's work and is not its. Run
with session 311's `pdf-render` changes stashed, the assertion is identical, name for name.

**Known.** The last change to `Cargo.lock` is `cdf81a1`, session 310's "The coverage lane is chosen
per frame, from the magnification", which moves quorra from `7d5dafb` to `7599081` for
`set_coverage`. That is the only candidate in range and it is the obvious one — a per-frame
coverage lane is exactly the kind of change that adds scene-derived allocations.

**Not known, and it is the first thing to establish**: that the bump is the cause. It has not been
bisected, because `QuorraPresenter::set_coverage` does not exist at `7d5dafb`, so downgrading the
lock alone does not compile — the check is a worktree at `9e8e6e1` (the commit before the bump)
with `doc/pdf.js` linked into it, running the same gate.

## Why it matters more than five documents

`quorra-gpu`'s own comment on the constant says:

> 256 MiB of instance data is roughly eight million rectangle commands — beyond any real page by
> orders of magnitude, while still refusing runaway input long before an allocator does.

Five real pages out of 974 exceed it, one of them by 2.3×. Either the accounting now charges
something it should not, or the premise is wrong and the number wants revisiting — and which of
those it is decides whether the fix is upstream or a `quorra_gpu::Options` this tree sets. That
question belongs in `doc/QUORRA_FEEDBACK.md` with the measurement attached, which is how every
other finding about this dependency has gone back.

**Do not raise the budget to make the gate pass.** The constant exists because a GPU buffer sized
from document-derived arithmetic is a decompression bomb under another name (principle 3), and a
ratchet moved to accommodate a regression is the failure mode `CLAUDE.md` trap 5 is about.

## Steps

1. Bisect it, as above, and record which revision changed the charge.
2. Measure what the five pages actually contain — `render-gpu`'s `frame_split` example prints
   where a frame's cost goes — so the report says *what* is being charged rather than that
   something is.
3. Take it to `doc/QUORRA_FEEDBACK.md` if the accounting is wrong, or argue a per-host budget here
   if the premise is.
4. The gate goes green when the five draw, not when the list is edited.
