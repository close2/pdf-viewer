# 520 — The clip that contains a mark, and the blitter that was not needed

**Finding.** `doc/todo/11`'s item 4 has carried one half of §10.7.4's clipping paragraph since the
four-hundred-and-forty-fourth session: a clip *chain* composes as a set intersection (ADR 0280) and
the finished mask then **multiplied** into the mark's own coverage inside `tiny_skia`'s
`fill_path`. ADR 0280 priced reaching it as "this backend's own blitter … the same project the
conflation-free rasteriser is". **The price was wrong in ADR 0268's direction**: all three steps are
public API on the library this backend already uses — `Mask::fill_path` is the coverage buffer,
`PixmapMut::fill_rect` through it over whole device pixels is the blit, and the composition is a
`min` between them. What made the round worth running twice is the closed form: §8.5.4's effective
shape is the intersection of the object's intrinsic shape with the clipping path, so **`S ∩ C = S`
where `S ⊆ C`** — a clip that contains a mark takes nothing from it — which is testable against
another render of the same geometry and against no renderer's arithmetic at all.

**Date.** 2026-08-14.
**ADR.** [0355](../adr/0355-the-clip-that-contains-a-mark-takes-nothing-from-it.md).
**Touched.** `crates/render-cpu/src/scan.rs` (`Clip` with three variants, `Scratch`, `intersected`,
`reached_pixels`, `is_a_set`, and three unit scenes), `crates/render-cpu/src/lib.rs` (`Admitted`
carries a `Clip` rather than a mask; `MaskCache` owns the scratch and `effective` decides which
variant each of its four cases is), `crates/render-cpu/src/shading.rs` (pass-through),
`crates/render-cpu/tests/clip_intersection.rs` (new — the identity through the display list),
`crates/render-quorra/tests/corpus.rs` (four pages added to `DIFFERS_IN_SHAPE`, with why),
`doc/conformance/ledger.toml` (§10.7.4, §8.5.4), `doc/todo/11`, `doc/todo/_scan-conversion.md`,
`doc/QUORRA_FEEDBACK.md` (§24), `doc/adr/0355-*` (new), this file.

## The three numbers the round is about

```text
  a half-plane whose edge falls at device 2.25, clipped by a half-plane with the same edge
    the mark's own coverage, unclipped     192 of 255
    the product, which was drawn           144            (0.7529² = 0.5669)
    min, which is drawn now                192            — S ∩ C = S
```

`issue21346.pdf`, ADR 0279's and 0280's witness, is now three rungs up the same ladder: its edge
reads **0.041 → 0.163 → 0.306** of the mark against a departure-(1) 0.827 and a clause 1.000, and
its similarity 0.9734 → 0.9781 → **0.9846** against a bound of 0.9900. Only one of that page's two
remaining products came out, and the reason is a finding of its own: its mark carries a soft mask
*and* a clip, `MaskCache::combine` multiplies them into one buffer, and what reaches the draw is a
value with no set left to intersect. `doc/todo/11` carries it as the cheapest next step.

## Where the corpus feels it, which is not where the witness said

A **§12.5.5 widget appearance whose border rule sits on the `/BBox` §8.10.1 step c) clips it by**.
`bug1844576.pdf`, `bug1844583.pdf`, `issue16473.pdf`, `issue18823.pdf`, `multiline.pdf`,
`textfields.pdf`. That is the same finding `render-quorra`'s differing list wrote about
`issue21068.pdf` in the two-hundred-and-seventh session, when a *redundant* clip came off its comb
separators — the general rule now answers what removing one clip answered then.

The two largest movers are 181×54-pixel form pages whose similarity falls (0.8738 → 0.5956 and
0.8050 → 0.6506) while their ink rises 17.6% and 10.6%. **Looked at**: at 4× the two renders are
indistinguishable, and at the page's own scale the difference is the field's border rows going from
half-covered to their own coverage. They move away from poppler, mupdf and ghostscript — all three
of which multiply — and towards the clause, which is principle 5's own case and is reported rather
than chased.

## Gate deltas

| | before | after |
|---|---|---|
| oracle verdicts | 906 / 67 / 786 / 13 / 19 | **unchanged**, 88 per-page lines moved |
| ink sweep (`doc/todo/00` step 7) | tail intact | 241 rows moved, **240 up**; four complete names at or past −1, same order |
| cross-backend, page scale | 934 agree / 20 differ | **930 / 24** |
| cross-backend, 4× GPU lane | 937 / 9 / 5 / 23 | **unchanged** |
| corpus, text, dates, XMP, JPEG 2000 | — | every summary line unchanged |

## What the first version cost, which is the lesson

`tiny-skia` takes a mask only at the pixmap's own size, so the coverage buffer is a band's worth of
bytes. Allocating one per mark cost **+54%** of the rasteriser on the corpus's heaviest clip page
(3554 clips) — 1.7 GB of zeroing on one page. One buffer per band in the `MaskCache`, with only the
pixels a mark can reach cleared, brings it to **+5.54%** there and **+1.21%** on a page of text, and
the oracle's 1794 lines are byte-identical across that difference, which is what says it was a
performance change and nothing else.

## Two notes for the next round

- **The stash stack is shared between worktrees.** `git stash push` in this worktree put an entry
  on the same stack a parallel round was using, and `git stash pop` took *its* work, not mine. It
  was returned with its message intact and nothing was lost, but the safe way to measure a before
  and an after in a worktree is `git diff > patch` and `git apply -R` / `git apply`.
- **A scene comparing a clipped render with an unclipped one cannot demand byte equality of a
  gradient.** `tiny-skia` fuses `SourceOverRgba` when there is no mask and does not when there is,
  so the two differ by a level in a gradient's interior — a difference that predates this round and
  has nothing to do with the composition. The scene allows one level and names the boundary pixel
  separately, where the defect it guards is tens of levels wide.
