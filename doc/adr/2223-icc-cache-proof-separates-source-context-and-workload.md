# 2223 — ICC cache proof separates source, context and workload

Status: accepted with measured correctness cost; final integration pending. Session 1694.

## Decision

A source-aware cache must fail independently when it loses source retention, source-profile identity or current conversion context. Run these deliberate omissions only in an isolated candidate export against unchanged independent tests, retain the patches and test hashes, and require assertion failures rather than accepting compilation failures as calibration. Restore the exact source between cases.

The colour oracle precedes the implementation. Rendering through the same implementation with cold selection is an additional discriminator, not independent authority for the desired colour. In particular, group-alpha arithmetic must account for each intermediate storage boundary; ADR 2220 records the corrected independent staged oracle. Do not loosen its tolerance to accommodate an unexplained difference.

Cost populations remain separate: ordinary direct ICC, existing generic ICC pattern tints, mixed use and ordinary RGB controls. Ordinary repeated paints exercise seeded hits; profile/component restore and intent/BPC changes exercise distinct dependencies. Generic ICC tints still convert directly. More tile instances measure repeated geometry as well as conversion and must not be described as one conversion per tile. Modes requiring newly correct current-state work cannot claim the same output as the stale baseline.

Both cost exports are verified against fixed baseline `0eb4d278`, with identical instrumentation and exactly the three production source files changed in the candidate. Document opening and representative raster samples are outside each timer; content interpretation and lazy resource work remain inside. Build each immutable release binary before the exclusive clock, retain binary/source hashes, use fixed settling and balanced pinned runs, and preserve every sample and quiet-guard refusal. Median cost and absolute deltas must accompany ratios; an aggregate must not hide regressions in a smaller population.

The current focused proof is not the combined corpus, launch or platform verdict. Those integration gates remain separately owed. No extension of this source enum to indirect or calibrated colour families, and no generic ICC tint cache unification, is justified merely by this implementation's availability.

## Calibrated receipt

The isolated candidate passes all thirteen corrected independent controls. Removing ICC retention or complete-context matching fails `late_ri_and_extgstate_intent_change_the_selected_source`; removing source-profile identity fails `nested_saves_restore_profile_and_current_intent`. Each failure is an assertion with exit 101, not a compiler error. The independent test hash stays `98ebd368ca3c4badc55100495bab798e25e10e64b3c6d448c9aa6494fcae057d`; the production source is restored byte-for-byte. `scratchpad/r1694/mutation-receipt.json` retains source hashes and exact commands; the driver exits 0 in 157 s, peak 0.65 GiB.

## Measured cost

Twelve balanced CPU2 pairs completed with all 24 sides and 48 quiet guards passing; exit 0, 343 s hold, 506 s elapsed, peak 0.08 GiB. Source and binary hashes agree before and after. The fixed baseline is `0eb4d278`; instrumentation is identical and the candidate changes exactly three production paths. Before/after release builds passed in 116/102 s, peak 0.88/0.82 GiB. Every sample, including the noisy repeated-solid ratio range, is retained in `scratchpad/r1694/paired-cost/summary.json` and its raw logs.

These are whole interpretations of generated one-channel ICC fixtures, not isolated conversion timings or a corpus-wide performance claim. Each sample contains 20 interpretations and 20,000 primary painting operators. Document opening and representative raster samples are outside the timer. The medians below are reported separately; the median of paired ratios/deltas need not equal a ratio/difference of independent medians.

| Population | Before median ms | After median ms | Paired median ratio | Paired median delta ms/20k | Ratio range |
|---|---:|---:|---:|---:|---:|
| icc-solid-repeat | 5.238 | 5.423 | 1.04711 | +0.241 | 0.72358–1.43466 |
| icc-solid-intent | 6.440 | 8.748 | 1.32563 | +2.133 | 1.21542–1.43478 |
| icc-solid-bpc | 12.941 | 15.264 | 1.18430 | +2.401 | 1.03421–1.35686 |
| icc-tint-repeat | 37.364 | 37.508 | 1.00770 | +0.285 | 0.94321–1.07161 |
| icc-tint-many | 139.812 | 139.225 | 1.00293 | +0.394 | 0.92204–1.05193 |
| icc-tint-values | 49.761 | 49.088 | 0.98159 | -0.915 | 0.87771–1.01724 |
| icc-tint-intent | 39.678 | 39.120 | 0.99797 | -0.081 | 0.90560–1.07918 |
| icc-tint-bpc | 46.395 | 45.660 | 0.98275 | -0.811 | 0.94675–1.12974 |
| rgb-solid-repeat | 4.981 | 5.065 | 1.01111 | +0.055 | 0.86831–1.21644 |
| rgb-solid-values | 10.907 | 10.984 | 1.01928 | +0.207 | 0.92575–1.08123 |
| icc-solid-values | 7.013 | 8.460 | 1.20177 | +1.400 | 1.02964–1.34398 |
| icc-solid-profiles | 8.995 | 10.797 | 1.20865 | +1.872 | 1.16203–1.27580 |
| icc-mixed-repeat | 48.181 | 48.596 | 1.01268 | +0.609 | 0.93131–1.05383 |
| icc-mixed-context | 49.045 | 49.046 | 1.00784 | +0.379 | 0.95864–1.03108 |
| icc-mixed-profile | 48.650 | 48.660 | 0.99992 | -0.004 | 0.97211–1.07498 |

Root accepts the scoped correctness tradeoff provisionally, subject to combined corpus and launch gates. Same-output ordinary component/profile restore costs about 20–21% more in these synthetic cases, or 1.400/1.872 ms per 20,000 marks (about 70/94 ns per mark). Restoring an earlier source can require a fresh conversion after the other source occupied the bounded slot. This cost is retained explicitly; there is no performance-neutral claim and no broader cache expansion.

The intent/BPC populations require work the stale baseline omitted: representative pixels change from [139,135,152] to Saturation [0,167,255] or BPC [116,111,133]. Their additional 2.133/2.401 ms per 20,000 marks is not an equal-output comparison. All other representative samples agree across both artifacts. Generic ICC tints remain direct, and their paired median ratios span 0.98159–1.00770; mixed-use ratios span 0.99992–1.01268. RGB controls are 1.01111/1.01928. Short repeated-solid samples remain noisy (paired range 0.72358–1.43466); no sample was discarded or chosen to improve the verdict.
