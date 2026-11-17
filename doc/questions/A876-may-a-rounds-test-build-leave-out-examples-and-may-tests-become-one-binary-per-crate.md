Status: complete
Given: 2026-11-17, in conversation — transcribed by the round
Owes: (1) pass --lib --bins --tests in a round's cargo nextest runs of touched crates and state so in doc/todo/02 tier 1 (keeping full example linking in the post-merge train); (2) run a one-round pilot merging render-cpu's 44 integration tests into one binary (with or after ADR 2910's Cargo 1.100 layout update), measuring build size, build time, and run time before deciding on pdf-model (build / infrastructure rounds)

> Q876: agreed with your recommendation

Reading: adopts both recommendations:
1. **Leave out examples in per-round `nextest` builds:** run `--lib --bins --tests` for touched crates in a round (`doc/todo/02` tier 1), relying on `clippy --all-targets` in the round for compile checking and the post-merge train's workspace run for linking examples.
2. **Pilot single-binary integration tests on `render-cpu`:** merge `render-cpu`'s 44 integration-test files into one binary in a single pilot round (with or after ADR 2910's layout update), measure directory size, build time, and run time against today, and report before deciding on `pdf-model`.
