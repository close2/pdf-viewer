# 2249 — Tint and CIE sources are compared by allocation, and priced by population

Status: accepted for integration. Session 1707, 2026-10-23.
Context: ADRs 2223, 2235 (the protocol), 2248 (the decision).

## Identity

ADR 2248's `SolidSource::Space` is equal to another only when both are one `Rc`. A value
comparison would need the identity of a tint transform, and a `Function` has none; comparing
the parsed space field by field would be a second equality nobody else keeps. The allocation
cannot be reused while it is compared, because the memo holds its own `Rc`. A selection of
equal values is a new allocation, seeded by its own selection-time conversion, so the only
thing given up is a hit after a restore — which is what the restore populations below price.

## Proof

Seven controls in `tests/icc_paint_context/tints_and_cie.rs` were written before the change
and run against HEAD `ee365e4f`'s production code: six failed on the stale colour — the
neutral knot [139, 135, 152] where a late intent owed red, the uncompensated [89, 89, 89]
where `/UseBlackPtComp` owed black, green [0, 255, 0] inside a `DeviceGray` group that owed
§10.4.2.2's grey 150, and 204 inside a `CalGray` group that owed 154. The seventh, a late
`ri` over `CalRGB` and `Lab` on the device, passed: that route reads no intent. With every
assertion on the inherited paint removed, the expected values and HEAD's cold selections
agree on all seven, so the oracle is not the candidate's. The candidate passes all thirty
tests of the file. In an isolated export with its own target, four omissions each fail on
assertions (exit 101): no retention fails six, no retention at `cs` fails the initial-tint
control, equality without identity fails the restore control, and a match that ignores the
context fails six. Eliding the move in `sc`/`scn` (a clone instead) passes, as it must.

## Cost

Twelve balanced pairs of `black_generation_order --icc-cost`, pinned to CPU 2 after 60 s of
settling, all 24 sides and 48 guards passed (load at most 8.9, GPU mean at most 2.5%); the
only production difference between the two release binaries is `content/colour.rs`. Paired
medians per 20 000 marks, every sample kept in `scratchpad/r1707/paired1/`:

| Population | Before ms | After ms | Paired ratio | Delta ms | Ratio range |
|---|---:|---:|---:|---:|---:|
| icc-solid-repeat | 7.144 | 6.923 | 0.97210 | -0.200 | 0.69911–1.67531 |
| icc-tint-many | 146.844 | 153.028 | 1.04138 | +6.003 | 0.81009–1.35220 |
| rgb-solid-values | 25.736 | 26.011 | 1.00551 | +0.141 | 0.87387–1.52668 |
| indexed-select | 8.880 | 9.218 | 1.03548 | +0.314 | 0.86336–2.67738 |
| sep-repeat | 4.944 | 5.048 | 1.02271 | +0.111 | 0.88510–2.42479 |
| sep-select | 8.912 | 9.375 | 1.04789 | +0.426 | 0.95714–2.32842 |
| sep-intent | 6.150 | 9.094 | 1.48139 | +2.945 | 1.34497–3.63786 |
| sep-bpc | 11.869 | 15.640 | 1.31910 | +3.770 | 1.20832–3.59374 |
| devicen-select | 9.358 | 9.716 | 1.04063 | +0.377 | 0.98794–2.87702 |
| sampled-select | 9.284 | 10.450 | 1.13342 | +1.211 | 1.07947–2.32568 |
| sampled-cs | 211.430 | 215.760 | 1.01776 | +3.732 | 0.92644–2.62660 |
| cal-repeat | 4.908 | 5.171 | 1.04122 | +0.199 | 0.87069–2.57040 |
| cal-select | 7.403 | 8.112 | 1.07106 | +0.525 | 0.91950–2.60895 |
| lab-select | 7.156 | 7.639 | 1.06508 | +0.466 | 0.91413–2.57064 |
| sep-restore | 7.928 | 9.802 | 1.23110 | +1.807 | 1.10766–6.66490 |
| sampled-restore | 9.465 | 11.330 | 1.19647 | +1.842 | 1.08074–2.95824 |
| cal-restore | 6.801 | 8.152 | 1.19670 | +1.334 | 1.13070–4.36872 |

A repeated tint or calibrated paint costs about 0.1–0.2 ms per 20 000 marks; a selection
0.4–0.5 ms, and 1.2 ms where the tint transform is a sampled table (about 60 ns a mark, the
`Rc` of space and operands); a `cs` per mark 1.8% over a parse that dominates it. A restored
source after another took the slot costs 1.3–1.8 ms (20–23%, about 70–90 ns a mark), as ADR
2235 found for `Indexed`. `sep-intent` and `sep-bpc` (+2.9/+3.8 ms) do work the stale
baseline omitted and are not equal-output: their samples change from the neutral and the
uncompensated colour to the owed ones; every other population's sample is identical across
the two binaries, and the untouched controls stay inside their noise. These are whole
interpretations of generated fixtures, not conversion timings; no performance-neutral claim
is made and no further cache is justified by them.
