# 1877 — A scene of lines is kept across views, and a zoom step only re-encodes it

Session 1520. Status: **accepted** and **built** (the scene half of `doc/todo/69` §5; the device
half — instances kept resident, the view a uniform — is priced in section 5 and not built). For the
project owner's performance target of 2026-10-10. Context: ADRs 0701, 0702, 1829, 1835, 1845 §4,
1857, 1860; raster's ADRs 0082, 0085, 0097; traps 1, 2, 13. Code: `crates/render-raster/src/stroke.rs`
(`thin_line`, `DotAsLine`, `is_view_free_line`), `present.rs` (`Retained::dots`, `Retained::draws`,
`FrameCost::scene_kept`), `scene.rs` (`Encoder::keep_dot`). Test: `render-raster/tests/
lines_across_views.rs`. Clauses: §8.4.3.2, §8.4.3.3, §8.5.3.2, §10.7.4, §10.7.5. No ledger row moves.

## 1. The premise, and the hypothesis

ADR 1845 §4 priced candidate B on the witness: a device zoom step at 1 168–1 196 ms (scene 400,
encode 505, execute 116) against the same frame replayed at 178 ms. Re-taken at the start of this
round (`zoom_frame`, `Cpu`, one exploring run, not a clock figure): a step of 1.25× from the fit was
1 625 ms, scene 713, encode 680, execute 93, transfer 51; the replay 179 ms. The premise held: every
step rebuilt the scene.

The hypothesis — that what kept the scene view-dependent was ADR 0702's rule for widths resolved in
device space — **held in part and missed the larger half.** A probe of `consume_view`'s caller
(`#[track_caller]`, removed) found one site on the witness, `thin_line`, hit 28 684 times — by the
page's **round dots**, not by its segments. Two different questions made a line the view's:

- *which strokes are lines*: `pdf_render::thin_segment` admits a segment whose ink fits its 2 × 2
  block **at this view** (ADR 1829) — the processor's kernel's limit, not the device's, whose line
  is exact at every size (raster's ADR 0097);
- *whether a dot is a line*: §8.5.3.2's dot is the line's disc only where its area reaches a level
  in some pixel; under that, §10.7.4's substitution draws it (ADR 1857). That is a question about the
  pixels the dot lands in, and no build can answer it once for every view.

## 2. Decision

**A segment of two distinct points is a raster line at every view** where it is solid, `Normal`,
undashed, under a path transform that keeps the axes and collapses nothing, with coordinates and a
width raster's scene admits. Its width crosses as the file states it, with §10.7.5's flag, and
raster resolves §8.4.3.2's zero and the adjustment per placement exactly as it resolves a stroke's
(raster's ADR 0085, this tree's ADR 0701). Nothing in the build reads the view for it.

**A round dot is a line where the view would draw it as one, and the scene remembers that it
asked.** The dot is a line where `thin_segment` admits it, as before, or where it is at least a
device pixel across both ways (its disc then covers more than π/4 of a pixel, so §10.7.4 substitutes
nothing). Each dot drawn as a line is kept beside the scene as a `DotAsLine`, and a view-free scene
is kept at another view only where every kept dot is a line there too — the same call a fresh build
makes, so the kept scene is that build. 28 684 dots asked again cost 13–40 ms at the fit.

The processor's population is unchanged. Past the 2 × 2 block the two backends now measure one set
by two constructions — the processor's general stroker, the device's closed form — which is raster
ADR 0082's relaxation; ADR 1835 §2 declined exactly this argument because no rule decided which lane
drew after a zoom, and ADR 1860's lane rule now does.

## 3. What it moved

One clock hold, HEAD `c6fb18fb`'s export against HEAD plus this round's hunks, each in its own
target directory, `zoom_frame` on the witness at the fit (scale 0.594, 1417 × 1001) then 1.25×,
1.5×, 2× and the fit again, three runs alternating, RADV on the Radeon 890M, load 4.5–7.0 (with
ADR 1878 in the after arm):

| frame | lane | HEAD | after |
|---|---|---|---|
| a step, 1.25× | `Cpu` | 969, 978, 996 ms (scene 420–479) | **385, 395, 439 ms** (scene 12.3–12.6, the dots asked) |
| every step, 1.25× to 2× and back | `Cpu` | 912–1 051 ms | **334–467 ms** (scene 0.0–12.6) |
| every step | `Compute` | 970–1 451 ms | **377–535 ms** |
| the fit, first | `Cpu` | 850–1 289 ms (scene 360–635) | 675–1 068 ms (scene 234–621) |

The same frame replayed is 179 ms: the step went from 5.4× the replay to 2.1×. The first frame's
scene is cheaper too — the segments no longer pass the 2 × 2 classification — and uploads 1 487
resources where it uploaded 4 108, the one-segment strokes no view's block admitted now lines.

**The owner's display** (inside the same hold, `quorra` on the witness, ten `plus` notches 300 ms
apart after the first frame, two runs per arm alternating, load 2.4–3.3): the last notch to its
frame **1.020 and 0.425 s → 0.191 and 0.192 s**. After the device's one asked frame (ADR 1860) the
window keeps the device's scene, and notches eight, nine and ten each land on the device 189–209 ms
after their key, scene 0.2 ms, raster culling 3.92–4.16 million of the 4.41 million commands; HEAD
drew nothing between notch six's processor frame and the last notch's device frame, a rebuilt culled
scene of 130–142 ms and a device draw of 269–281 ms.

## 4. Agreement

`lines_across_views.rs` holds both halves byte for byte on the real adapter: a page of lines (every
cap, four directions, widths from 0 to 6, a fifth-of-a-unit segment, an adjusted line, dots of 2 to
7 units) drawn at six views in sequence keeps the first view's scene at each and draws the bytes a
device that built the scene at that view draws; and a dot a step out takes under a level rebuilds
there, with the fresh build's bytes. Planted back, each test fails: the width resolved at the build's
view moves 2 058 of 172 800 bytes at the second view; the dots left unasked keep a scene the step out
must rebuild.

On the witness itself, `zoom_frame`'s digests: the 1.25×, 2× and 4× frames and the return to the
fit, each drawn from the fit's kept scene, are the bytes of a run that drew that view alone. Against
the processor (`heavy_drawing page`, the oracle at four strips), the device differs at the fit by at
most 15 levels, mean 0.1881 → 0.1911, and at 1.25× by at most 15, mean 0.1455 → 0.1477: the device's
frame moved, by the strokes that are lines now and by §8.4.3.2's zero resolved by raster rather than
before it, and stayed inside the bound the lane is held to.

**The corpus moves, and moves toward the oracle.** The six arms from the export against HEAD's
(`/home/AI/arms-1515/`): every arm exit 0, no page on one side only, 51–70 pages moved an arm. The
largest rise in a page's mean difference from the oracle is +0.0082 levels (`issue4402_reduced.pdf`,
1×); the largest fall is −2.50 (`bug1743245.pdf` on `Gpu` at 1×, 2.61 → 0.11), whose grid of long
hairlines the general stroke drew heavier than the set and the line lane draws as the processor does
— looked at, three panels: HEAD's device, this one, the oracle.

## 5. What is left, priced

The step is now raster's encode, transfer and execute. The encode walks every line per view —
ADR 1878 takes the queue's share of it out — and the transfer stages the 60-byte instances again.
Stating a line's instance in page space, with the view a uniform the line shader applies (`line.wgsl`
reads the device endpoints, the pen and the clip rectangle; under a transform that keeps the axes all
three map by the uniform), would let the instance stream stay resident and a step cost the replay:
execute and the medium. It needs the line lane's shader, which round 1519 was changing in this batch,
and the op list's batches, which interleave lines with the page's other marks and are built per view.
