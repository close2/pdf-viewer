# 1311 — An opaque band is reduced off its column sums, and a matte is tabulated

Date: 2026-10-01. Branch `batch-1309-1314`, shared worktree. ADR:
[1457](../adr/1457-an-opaque-band-is-reduced-off-its-column-sums-a-matte-is-tabulated-and-a-frame-without-restart-intervals-is-not-cut.md).

## What was asked, and what the measurement said instead

The performance slot: the photographs ADR 1433 did not reach, and its 16 ms reduction. The census
held: 92 of 168 large baseline frames have no `DRI`. But the largest no-`DRI` page's 172 ms was
not a codec at all — `issue13931.pdf`'s soft mask states a `/Matte`, and §11.6.5.2's inversion
ran per sample with two allocations a pixel. The "three serial walks" were two.

## What was built

- The reduction (`pdf_render`'s oracle and raster's mirror together): an opaque band of source
  rows is summed down its columns once and each cell read off the sums by a reciprocal; the
  equality with the premultiplied arithmetic is proved in `paint.rs` and tested at every floor's
  edge. Photograph turn 46.6–48.3 → 42.3–43.7 ms pinned (transfer 16.4–17.0 → 12.2–12.8).
- The matte inverted by a 65 536-entry table per component, rows on the pool: `issue13931` turn
  178.8–181.2 → 64.3–66.6 ms.
- One walk of the first scan serves the `DNL` and the restart intervals.
- ITU-T T.81 fetched (the W3C's copy; ITU's needs a login), `doc/md/T.81.md`, entered in
  `doc/third-party-data.md`; restart.rs's citation corrected to sections E.2.4 and F.2.1.3.1.

## What was tried and not kept

Cutting a no-`DRI` frame at MCU rows an entropy pass finds, DC differences re-coded from zero:
byte-identical on every admitted corpus frame (125 of 125), 2–3 ms saved on a page of one photograph,
1.5–10 ms lost on pages whose other images already decode on the pool. Not merged.
A reduction asked per cell rather than per band: a third slower on soft-masked pages.

## Worth keeping

Profile the page before choosing the lever the brief names: the biggest no-`DRI` photograph's
cost was a mask's arithmetic, not its Huffman pass.
