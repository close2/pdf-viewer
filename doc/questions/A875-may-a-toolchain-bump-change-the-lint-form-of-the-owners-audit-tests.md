Status: complete
Given: 2026-11-17, in conversation — transcribed by the round
Owes: apply the lint-only edits to the specification-audit test files when bumping the toolchain to 1.100.0 (listing each edited audit file in the round's record, while keeping any edit that changes what an audit test builds, reads, or expects as an owner question) alongside the Cargo 1.100 path-lookup update of ADR 2910 (a toolchain / build round)

> Q875: agreed

Reading: adopts option (a) — a toolchain bump may make purely syntactic lint-form edits to the owner's audit tests where every assertion's operands, inputs, and expected outcome stay identical, with each edited file listed in the round's record; any change to test semantics or expectations remains gated on an owner question.
