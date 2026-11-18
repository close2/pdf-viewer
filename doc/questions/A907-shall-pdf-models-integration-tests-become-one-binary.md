Status: complete
Given: 2026-11-18, in conversation — transcribed by the round
Owes: group pdf-model's integration tests into a few binaries by subject (keeping --test corpus and its tier-2 kin as binaries of their own so doc/todo/02 and tools/state.sh do not change) and measure directory size, clean build, and single-test-edit rebuild the same way ADR 3036 did (option 2, a build / model round)

> Q907: agreed

Reading: adopts option 2 — consolidate `pdf-model`'s 350 integration-test files into a few binaries grouped by subject (such as content, images, fonts, forms, saving), keeping the tier-2 corpus walk binaries (`--test corpus` and its kin) standalone, and measure the result as ADR 3036 measured `render-cpu`.
