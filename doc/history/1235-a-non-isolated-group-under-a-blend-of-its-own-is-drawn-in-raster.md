# 1235 — A non-isolated group under a blend of its own is drawn in raster; a replay defect closed

2026-09-23. ADRs 1307, 1308. Applies `doc/questions/A76`'s independence rule again.

## What moved

No ledger status: §11.4.4 and §11.7.4.3 were `implemented`. Their notes now say `render-raster`
draws §11.4.4's result step under a non-Normal group blend; §11.4.4's test list swaps the deleted
refusal test for the two new ones. `NonIsolatedReason::GroupBlendNotNormal` is deleted. In
`raster-gpu`, `encode_group` plans the elements a second time onto transparency for Table 140's
group alpha, and `composite.wgsl` recovers `αg × C = E(B) − (1 − αg) × B` and composites it by
§11.3.6 under the group's mode. `render-raster`'s refusal test became
`cpu_and_quorra_agree_on_a_non_isolated_group_that_blends` (Hue, pixel from the clause).

## Independence

This round did not open, grep or read anything under `crates/render-cpu/`. The ledger's §11.4.4
note, read when this round edited it and after the construction was measured, says `render-cpu`
also runs the elements again onto transparency: same NOTE, same method, numbers never shared.

## Findings

**The witness compares.** `issue12798_page1_reduced.pdf` left `REFUSED_BEFORE_THE_SCENE`: mean
0.0053, worst tile 0.82 at 100%. Nothing stood behind this refusal (trap 48 was already on the
index). **The crawl meet** (census over `openpreserve` and 5000 `tika-issue-tracker` files, first
rung of `zoom_ladder`): of six first pages painting the mode in a non-Normal implicit group, five
compare (mean 0.08–0.28, worst tile at most 2.61); `PDFBOX-3000-42.pdf` is refused for a §11.6.6
three-CIE-component group, a construction behind this one. 1229 did not record its three names.
The pdf.js corpus: 946 agree, 5 differ, 7 refused, 16 not comparable, ratchets held.

**The replay defect was real, and the comment was right.** A solid fill under a non-Normal blend
wrote two Normal `SolidFill` records inside `fill_through_blend_group` and stayed replayable; a
zoom step drew 2298 of 9216 pixels wrong. `plan_child` now calls `unreplayable()` for every
child layer (ADR 1308), held by `record_replay.rs`'s new test.

**Numbers.** Clause derivation: worst 1.3e-5 over 50 000 nested groups per mode. Device: 64
cases over sixteen modes, worst 1 level.
