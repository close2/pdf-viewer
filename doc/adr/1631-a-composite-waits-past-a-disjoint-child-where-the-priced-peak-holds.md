# 1631 — A composite waits past the next child where their rectangles stand apart and the priced peak holds both

Status: accepted. Session 1397. Builds the second lever ADR 1618 section 3 priced and left
("[c]onsecutive composites whose `onto` rectangles stand apart"), under a rule that keeps the
frame budget's arithmetic where it was. Supersedes nothing; amends ADR 1618 section 2 in one
respect: a child no longer always draws the composite before it in a pass of its own.
Context: ISO 32000-2 §11.3.6, §11.4.4; `CLAUDE.md` principles 2 and 4; ADRs 1618, 1630;
raster's ADRs 0020, 0036 and 0038.
Code: `raster/crates/raster-gpu/src/layers.rs` (`Prices`, which `peak_layer_bytes` is now
written with), `compose.rs` (`render_plan`'s list of waiting composites), `compose/child.rs`
(`Composite::waits`, `Executor::may_wait`, `Executor::wait_ceiling`, `composite_pass` of several),
`compose/draw.rs` (`draw_pass` of several), `device/record.rs`.
Tests: `raster/crates/raster-gpu/tests/a_composite_waits_beside_a_disjoint_child.rs` (new, two
tests: the first watched failing with the disjointness test removed, on a closed-form pixel, the
second with the budget test removed, on the texture count); the corpus gate's per-page digests
on six arms.

## 1. Priced against the frame budget first

ADR 1618 left this lever on the budget: while the later child renders, the earlier child's
texture and its backdrop copy are still alive, which `peak_layer_bytes` — the figure the frame is
refused against — never priced. So the first question is whether the peak moves. The census and
the price, from the encoded frames (scratch, removed), at `frame_budget`'s 1600 × 1000 window:

| page | adjacent child pairs | disjoint | priced peak | if every disjoint pair waited | if every disjoint run waited |
|---|---:|---:|---:|---:|---:|
| `bug1721218_reduced.pdf`, each group render | 16 | 11 | 578 736 B | 578 736 B | 578 736 B |
| `personwithdog.pdf` | 6 | 6 | 1 944 256 B | 1 945 152 B | 1 950 400 B |
| `22060_A1_01_Plans.pdf` | 2 | 1 | 3 831 200 B | 3 831 200 B | 3 831 200 B |

On the page the lever is for, **the peak holds**: the plan's heaviest child is so much heavier
than the small ones in a row that two of them alive at once cost less than it. On
`personwithdog.pdf` it does not — by 896 bytes for pairs, 6 144 for runs — and a rule that waits
wherever the rectangles stand apart would be a frame refused against a figure smaller than what it
holds.

## 2. The rule: wait only where the price already covers it

So a composite waits past the next child only where the bytes the wait keeps alive fit under what
the frame was already priced at — and then the peak cannot move on any page, by construction.
`may_wait` lets the waiting composites stay pending past child `next` when all of these hold:

- `next` is isolated and has no group alpha, so its rectangle is its own bounds and known before
  it renders; a seeded child copies its parent's whole accumulator, which must hold every
  composite before it.
- `next`'s rectangle in its parent meets no waiting composite's `onto`. Each composite's draw
  writes only its own `onto` (its scissor), so the accumulator under `next` is the same before
  and after those draws, and `next`'s backdrop copy, taken before them, is the copy it would have
  taken after.
- A recorded pass has written the accumulator, so the copy is a read of real texels; a composite
  whose pass is to clear the accumulator does not wait.
- No waiting composite holds a group alpha, which no price counts.
- The waiting composites' children and copies, plus `next`'s own price (`Prices::child`: its
  subtree, or itself beside its copy), fit under the plan's heaviest child as priced
  (`Prices::heaviest_child`). That is the term `peak_layer_bytes` adds to the plan's own texture,
  so the frame's live bytes never exceed what it was priced at. A seeded plan's region is its
  parent's rather than the one it was priced at, so nothing waits in one.

The composites then draw together, in their children's order and each under its own scissor, at
the head of the next run's pass or in one pass of their own. **Why the bytes are the separate
passes'**: their rectangles are pairwise apart, so no pixel is written by two of them; each reads
its own copy, child, mask and group alpha and never the accumulator; and ADR 1618 section 2's
argument carries the rest — draws in one pass are blended in recording order, as consecutive
passes are.

`Prices` is `peak_layer_bytes`'s arithmetic kept for the frame, so the budget check and the wait
rule read one computation rather than two that could drift.

## 3. Counted

The turn's passes on the scratch exports of section 1:

| | HEAD | ADR 1630 | this change |
|---|---:|---:|---:|
| `bug1721218_reduced.pdf` | 193 | 136 | **114** |
| `personwithdog.pdf` | 26 | 18 | **14** |
| `22060_A1_01_Plans.pdf` | 26 | 18 | **17** |

On `bug1721218_reduced.pdf` every one of the 22 disjoint pairs a turn waits, and no pass on any
other page moves.

## 4. Measured

The sitting of ADR 1630 section 5: exports of HEAD, of ADR 1630 alone and of both, eight runs of
five rounds a tree interleaved, pinned, load 1.0 to 3.2, minima, ms:

| `bug1721218_reduced.pdf` | HEAD | ADR 1630 | this change | the pair, this less ADR 1630 | this less HEAD |
|---|---:|---:|---:|---:|---:|
| `frame_budget` turn | 151.68–153.85 | 149.77–151.95 | **149.10–151.18** | +1.41 to −2.85 | −0.57 to −3.49 |
| `frame_budget` seventh | 150.66–152.01 | 148.62–150.76 | **147.23–151.15** | +2.34 to −3.53 | −0.49 to −4.78 |
| `frame_budget` step | 83.05–84.49 | 82.73–84.10 | **81.61–83.04** | −0.32 to −2.49 | −0.67 to −2.67 |
| `zoom_frame` 1× frame | 77.6–79.5 | 77.1–78.4 | **75.1–77.5** | +0.4 to −2.6 | −0.2 to −3.5 |
| `zoom_frame` 1.25× step | 75.4–77.7 | 74.5–75.2 | **73.8–75.1** | +0.5 to −1.4 | −0.8 to −2.8 |

Per interleaved pair (trap 121) the wait is quicker in six of eight turns and seventh frames and in
all eight steps, and the two changes together are quicker than HEAD in every pair of every row.
The CPU backend read 47.6 ms and 56.9 in the same sitting, so `zoom_frame`'s 1× frame is 1.58× it
where HEAD's is 1.63×, and the step 1.30× where it is 1.33× — `doc/QUORRA_FEEDBACK.md` section 70.
**Callgrind** on the turn, as ADR 1630 section 5 took it, HEAD against both changes:
`CommandEncoder::finish` 17 793 917 → 15 171 349 instructions, the HAL's `begin_render_pass`
2 464 981 → 1 664 066 beside 1 539 683 for the transfers, and `Queue::submit` 4 607 776 →
3 572 344. Of those, ADR 1630's transfers alone account for `finish` 17 002 589 and `submit`
4 080 209: the 22 passes the wait removes are worth 1.8 M instructions of `finish`, about 83 000
each, inside the call where `wgpu`-core records every pass into the HAL's command buffers.

**Corpus.** The run of ADR 1630 section 5, which was of both changes together: 0 pages moved
on any of the six arms, verdicts equal arm for arm.
