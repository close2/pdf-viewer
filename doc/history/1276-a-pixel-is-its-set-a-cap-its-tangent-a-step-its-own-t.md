# 1276 — A pixel is its set, a butt cap its tangent, a step its own `t`

Batch forty-two, raster slot. No ledger row moved (§8.5.3.3, §8.4.3.3 and §8.7.4.5.3 were already
`implemented`; this made raster's side meet them). ADR 1389; feedback section 56 closes 45 and 55.
`crates/render-cpu/` was not opened.

## Built
- **Fill** (`raster/fill/{topology,overlap,exact}.rs`): the integral stands where the fill winds two
  neighbouring values (plain convex subpaths; or no crossing and nesting alternating in
  orientation); other pixels are recomputed from their own edges by bands, the constant read off
  the integral. Stored outlines answer once; the compute lane and ADR 0090's hybrid take only those
  that wind two values, so `compute_lane.rs`'s byte identity holds.
- **Butt cap** (`flatten::Ends`, `stroke::carried`): the end edge and cap squared to the Bézier's
  end tangent where the end chord can carry it.
- **Ramp** (`device/ramp.rs`, `shading.wgsl::ramp_texel`): segments cut at coincident stops, bounds
  carried as exact `f32`s in a second row, rounding only within a segment.

## Measured (closed forms, every pixel, 1×–8×)
Band stated twice 63.75/127.5 steps → ≤ 0.5; overlapping squares (4 cases) 46–92 → ≤ 0.5; star
42–129 → 0.50; opposed apart squares 153 → ≤ 0.5. Cap: thin arc projection 0.20→0, tight arc outer
corner 0.80/0.40 → 0. Stripes: 0/0/3/5 wrong rows → 0/0/0/0.
A row-wide set pass was built first: 670 k of 1.3 M rows flagged, 11.8 s of thread time; dropped.
Cost now: 1× raster 3.76 → 3.98 s, median ratio 1.30 → 1.38; 4× CPU lane 19.06 → 19.29 s.

## Corpus
1×: 959 agree / 2 differ (issue19083, issue2177), exit 0. 4× CPU lane 958/0, 4× GPU lane 957/0
(`issue10572.pdf` left). `issue1905.pdf` left the 1× device-refusal list mid-round with no cause in
raster found (candidate: a sibling's image change); list updated. ink_ladder: issue20232 within
0.04% of the oracle at every rung, issue15150 and issue21068 equal. `raster_golden` untouched: the
CPU oracle did not change on this round's account.

## Left
Per-placement cache for re-tiled Type 3 glyphs (not measured). Curve–line joins still square to
chords. A crossing of two subpaths exactly at a shared corner is not seen (ADR 1389 section 1).
