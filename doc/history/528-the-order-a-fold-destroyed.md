# 528 — The order a fold destroyed, and the witness that was not the witness

**Finding.** ADR 0355 left "a clip already folded into a soft mask" as the cheapest next step and
declined it on a reason that reads well: §11.6.5's mask *is* a product the standard states, and
`MaskCache::combine` had already multiplied the clip into it, so the set was gone by the time the
mark arrived. The reason is right about the soft mask and wrong about the fold — **the standard
states the two in an order, and the order is what the fold destroyed.** §8.5.4 intersects the
clipping path with the object's *intrinsic* shape and §11.3.7.2 multiplies the mask shape into what
comes out of that, so the source shape is `(fⱼ ∩ C) · fₘ` and never `fⱼ · (C · fₘ)`. The cheap part
is that recovering it needs no third buffer: multiplication by a non-negative value distributes
over a minimum, so `min(M, C)·S = min(M·S, C·S)` — the soft mask's own rows beside the product this
cache already holds — and in eight bits it is *exact* rather than approximate, because a minimum
commutes with a monotone rounding.

**Date.** 2026-08-15.
**ADR.** [0363](../adr/0363-a-set-and-a-value-fold-into-one-buffer-and-only-one-of-them-belongs-there.md).
**Touched.** `crates/render-cpu/src/lib.rs` (`Built::value` and `Built::held`, `combine` keeping the
rows it already assembled, `admitted` handing out the pair), `crates/render-cpu/src/scan.rs`
(`Clip::Both`, `composable`, `intersected` and `is_a_set` taking the value beside the product,
`scaled`, and three unit scenes), `crates/render-cpu/tests/clip_intersection.rs` (a soft mask that
varies down the page, and the identity through the display list under it),
`doc/conformance/ledger.toml` (§8.5.4, §10.7.4, §11.6.5), `doc/todo/11`,
`doc/todo/_scan-conversion.md`, `doc/QUORRA_FEEDBACK.md` (§24a), `doc/adr/0363-*` (new), this file.

## The ladder, and the one the round did not have

```text
  a half-plane at device 2.25 under a coincident clip and a soft mask of 128 of 255
    the mark's own coverage, unmasked and unclipped   192 of 255
    the mark under the soft mask alone                 96
    the product taken as a value, which was drawn      72
    min(M · S, C · S), which is drawn now              96      — the clause's own (fⱼ ∩ C) · fₘ
```

**Every scene written for the round put the clip's edge on the mark's, and on that axis the
composition cannot be told from one that never applies the value at all.** Deleting `scaled` from
`intersected` leaves all three of them green. It is arithmetic rather than luck: where `C = M` the
wrong form `min(M, C·S)` is `min(M, M·S)`, which is `M·S` because `S ≤ 1` — the right answer,
arrived at by the scene rather than by the code. That is trap 2's fifth instance in the axis ADR
0046 names, and the parameter every scene had left at its default was the clip's *coincidence*. A
fourth scene offsets the two edges by half a pixel inside one column, where the mark's coverage
falls below the product and the two forms part: the mark at 2.75 under a clip at 2.25 reads **32**
where the unscaled form reads 64 and the old product reads 24.

Each of the four was confirmed to fail with the fix removed, by two separate removals: the cache
handing out `Clip::Value` as before (the display-list scene, 28 levels) and the composition
declining the pair (all three unit scenes, 24 and 8 levels).

## What it moves in the corpus: nothing, and the population says why

120 commands over the 974 first pages take a clip and a soft mask together; 27 of those are fills
reaching the composition; 14 decline because the clip is already a set under the mark; **13
compose**, over five documents. The two forms part only where the mark's boundary and the clip's
are *both* fractional in one pixel, and at all thirteen the mark is whole where the clip is not. So
the round answers `CLAUDE.md`'s **coverage** question and not its robustness one, which that file
states as a case rather than an excuse.

## Gate deltas — every gate run in both arms, page by page

| | before | after |
|---|---|---|
| corpus | 974 documents: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 61 incomplete, 0 slow | **identical, line for line** |
| oracle | 906 agree / 67 contradicted / 786 ambiguous / 1 our geometry / 2 reference geometry / 13 not comparable / 19 no render, of 1794 | **identical, line for line** |
| ink sweep (`doc/todo/00` step 7) | 786 rows | **0 moved**; 20 at or past −1, 16 incomplete, and the four complete names in the standing order |
| cross-backend, page scale | 930 agree / 24 differ / 2 refused / 18 not comparable | **unchanged, differing list identical** |
| cross-backend, 4× GPU lane | 937 / 9 / 5 / 23 | **identical, line for line** |
| text, dates, XMP, JPEG 2000 | — | every summary line unchanged |

The rasteriser's own ink total is identical to the digit on all five composing documents, which is
the same claim one level below the gates. **Session 520 moved this lane 934/20 → 930/24; this round
moves it in neither direction**, and `doc/QUORRA_FEEDBACK.md` §24a says why the corpus cannot
currently tell either backend's fold from the other's.

## What it costs

`examples/callgrind_rasterise`, `RAYON_NUM_THREADS=1`, instructions for one page: **−0.12%** on a
page of text with no clip-and-mask pair at all (rebuild noise), **+0.27%** on the corpus's heaviest
clip page at 3554 clips, **+0.61%** on `knockout_smask.pdf`, **+0.05%** on `issue21346.pdf` and
**+0.11%** on `issue18529.pdf`. The number worth reading is the second, against the **+54%** ADR
0355's first version cost on that same page before it reused a buffer: what this construction adds
is one slice compare per composed row and a `u16` multiply, into a buffer that was already there.

## The finding worth more than the change: ADR 0355 named the wrong residual

That ADR said of `issue21346.pdf` — ADRs 0279's, 0280's and its own witness — that its mark
"carries a soft mask *and* a clip … so what arrives at `scan::fill` is a `Clip::Value`". **Nothing
on that page arrives at `scan::fill` that way.** Its page one takes the clip-and-mask pair exactly
twice and both consumers are a transparency *group's* blit; the only two marks reaching the
composition are clipping regions with no mask at all. Its similarity is 0.9846 before this round
and after it.

**And the group case is owed, where ADR 0355 recorded it as not owed.** That ADR argued a group's
buffer carries §11.4.5's group alpha rather than one mark's coverage, "so there is no second shape
here". §8.5.4's own third sentence answers it: a transparency group's shape is defined as the union
of its constituent objects' shapes and is influenced by the clipping path in effect when the
group's results are painted onto its backdrop. A group has a shape and the clip at the blit
intersects it. What is true is narrower — *this backend's* group buffer carries alpha, which is
shape times opacity, and the two coincide only where every element's opacity is 1. So what the
construction needs is a shape channel beside a group's raster, which `doc/todo/11` item 4 now
carries with a measurement rather than a guess: with the group's alpha meeting the clip by `min`
and the blit then carrying no mask, the witness's device column 14 of row 89 goes `(240, 245, 249)`
to `(227, 237, 244)` against an interior of `(206, 223, 235)` — **0.306 → 0.571** of the mark,
against departure (1)'s 0.827 and the clause's 1.000. Alpha standing in for shape is why that
prices a direction rather than an answer.

## Two notes about the record and the instruments

- **This round was interrupted before it committed, and a later agent finished it.** What arrived
  in the worktree was the code, the ADR, the ledger and the two todo files, with no history file,
  no commit, and no evidence that any gate had been run. The finishing agent verified the clause
  reading and the closed form against `doc/md/` itself, re-ran every gate in both arms, took the
  callgrind numbers again — the table in ADR 0363 is the finishing agent's own, and the two runs
  agree to within a rebuild's noise — and found the discrimination gap above, which is the one
  thing in this file that was not in the worktree when it arrived. It also folded the two copies of
  the rounding arithmetic into one function, because the exactness argument holds only while both
  sides of the `min` round the same way, and an agreement a comment asks for is one a call
  guarantees. **One inherited number did not reproduce**: the group-blit probe arrived as
  0.306 → 0.429 and came out 0.306 → **0.571** when re-run with the construction written down above
  — the same direction, half again as far — and the number that stands is the one whose patch is
  stated. A probe of a hypothetical is worth exactly its stated construction, which is why the
  documents now carry one.
- **A fresh worktree target directory reads as trap 10a's alarm and is not one.** The first oracle
  run printed a **0.0%** reference-cache hit rate, which that trap makes the tell that the corpus or
  a renderer moved. The cause here is that `pdfref-cache` lives under the *target* directory, and a
  worktree with its own `--target-dir` starts with an empty one; the second run in the same
  directory printed 99.8%. The rate is still the right instrument — it just answers a question about
  the directory before it answers one about the tree.
