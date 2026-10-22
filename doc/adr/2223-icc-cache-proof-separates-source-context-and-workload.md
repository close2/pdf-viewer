# 2223 — ICC cache proof separates source, context and workload

Status: proposed; calibrated omissions passed; final cost receipts pending. Session 1694.

## Decision

A source-aware cache must fail independently when it loses source retention, source-profile identity or current conversion context. Run these deliberate omissions only in an isolated candidate export against unchanged independent tests, retain the patches and test hashes, and require assertion failures rather than accepting compilation failures as calibration. Restore the exact source between cases.

The colour oracle precedes the implementation. Rendering through the same implementation with cold selection is an additional discriminator, not independent authority for the desired colour. In particular, group-alpha arithmetic must account for each intermediate storage boundary; ADR 2220 records the corrected independent staged oracle. Do not loosen its tolerance to accommodate an unexplained difference.

Cost populations remain separate: ordinary direct ICC, existing generic ICC pattern tints, mixed use and ordinary RGB controls. Ordinary repeated paints exercise seeded hits; profile/component restore and intent/BPC changes exercise distinct dependencies. Generic ICC tints still convert directly. More tile instances measure repeated geometry as well as conversion and must not be described as one conversion per tile. Modes requiring newly correct current-state work cannot claim the same output as the stale baseline.

Both cost exports are verified against fixed baseline `0eb4d278`, with identical instrumentation and exactly the three production source files changed in the candidate. Document opening and representative raster samples are outside each timer; content interpretation and lazy resource work remain inside. Build each immutable release binary before the exclusive clock, retain binary/source hashes, use fixed settling and balanced pinned runs, and preserve every sample and quiet-guard refusal. Median cost and absolute deltas must accompany ratios; an aggregate must not hide regressions in a smaller population.

The current focused proof is not the combined corpus, launch or platform verdict. Those integration gates remain separately owed. No extension of this source enum to indirect or calibrated colour families, and no generic ICC tint cache unification, is justified merely by this implementation's availability.

## Calibrated receipt

The isolated candidate passes all thirteen corrected independent controls. Removing ICC retention or complete-context matching fails `late_ri_and_extgstate_intent_change_the_selected_source`; removing source-profile identity fails `nested_saves_restore_profile_and_current_intent`. Each failure is an assertion with exit 101, not a compiler error. The independent test hash stays `98ebd368ca3c4badc55100495bab798e25e10e64b3c6d448c9aa6494fcae057d`; the production source is restored byte-for-byte. `scratchpad/r1694/mutation-receipt.json` retains source hashes and exact commands; the driver exits 0 in 157 s, peak 0.65 GiB.
