# 2351 — The half float nearest a translucent source is priced, and deferred

Status: deferred — measured against HEAD's six arms on two lanes at 1x and not built, because it
moves three hundred page-lanes for a fall that 26 rises on each lane cancel in part; what would
reopen it is below. Session 1758, 2026-10-26, the pixels slot of batch one hundred and fifteen,
under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clause: ISO
32000-2 §10.7, which leaves the conversion of a colour to the device's bits to the device; no
ledger row moves. Context: ADRs 2310 §3, 2311, 2324, 2350; traps 251, 256, 257. Code read:
`raster-gpu/src/shaders/{rect,coverage,line,shading}.wgsl` (`fs_main`).

## 1. The residue it is aimed at

ADR 2324 hands an opaque source over as its level, which survives RADV's export of a fragment's
colour as a half float rounded toward zero (ADR 2310 §3), and leaves every other source to the
blend so that the pixel is rounded once. That once is after the export: a translucent source, an
antialiased edge, or a mark under a soft mask reaches the blend up to an eighth of a level low on
RADV, where llvmpipe exports the `f32`. ADR 2350 meets it on the shading lane: handing a translucent
ramp over unrounded took `issue14165.pdf`'s partly covered last row and column one level low on
RADV (0.0002 to 0.0011 of 255 on the `cpu` lane at 1x) and held it on llvmpipe.

## 2. The construction priced

Hand every source that is not opaque over as the half float nearest it — one add and one mask on
the `f32`'s bits, `(bits + 0x1000) & 0xffffe000`, in integers for trap 256's reason — so an export
that rounds toward zero keeps it exactly, and leave the blend to round once. Built in a scratch
copy of the tree, never in this one, twice:

- **On the shading lane alone**, with ADR 2350's rule underneath it: `issue14165.pdf` back to
  0.0002, `issue7821.pdf` 0.3062 to 0.2296 on the `cpu` arm at 1x (ADR 2350's construction alone
  gives 0.2456), and the arm's sum of page means 49.8050 to 49.6825 against ADR 2350's 49.7206 —
  36 page-lanes moved, 21 fell, 12 held to four decimals and 3 rose (`alphatrans.pdf` 0.0498 to
  0.0506, `issue18032.pdf` and `personwithdog.pdf` by 0.0001). On llvmpipe the extra rounding cost
  `issue7821.pdf` its gain in part, 0.0538 to 0.0664 on the page drawn alone.
- **On every solid lane as well** (`rect`, `coverage`, `line`): the `cpu` arm at 1x moved 306
  page-lanes, 107 fell, 173 held and 26 rose, the sum 49.8050 to 49.6357; the `gpu` arm 301 moved,
  101 fell, 172 held, 28 rose, 50.6750 to 50.5084. The largest rises are edge pages — `comments.pdf`
  0.3048 to 0.3095, `alphatrans.pdf` 0.0498 to 0.0514, `issue14117.pdf` 0.1237 to 0.1246 — and
  raster's own suite went red at `non_isolated_blended_groups`, a `Hue` group four levels off the
  clause on llvmpipe, where a half float's rounding of a near-grey backdrop moves its hue.

## 3. Decision

Not built. The second form misses record 1717's bar 26 and 28 times, its rises are not ties, and
none of them was isolated to its construction here, so nothing yet says whether each is the
device's export or the oracle's rounding — and a red fixture of raster's own is a third reason
before any page. The first form is the better shading lane on RADV and the worse on
llvmpipe, and it rests on one adapter's export mode, which `wgpu` does not let a shader ask about.
ADR 2350 is built without it, and its one RADV rise is named there.

## 4. Revisit when

- the arms are taken on a second adapter whose export is not RADV's, so that a construction stated
  for one export mode can be weighed on two;
- or `wgpu` exposes the colour export's format or rounding to a shader or a pipeline, so that the
  half float is asked for only where the export rounds toward zero;
- or a pixels round isolates the 26 rising edge pages to a construction of the oracle's, which
  would make the rises an oracle finding rather than a cost.
